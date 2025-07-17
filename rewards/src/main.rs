use std::convert::identity;
use std::sync::Arc;
use std::time::Duration;

use chrono::NaiveDateTime;
use clap::Parser;
use deadpool_diesel::postgres::Object;
use namada_sdk::time::{DateTimeUtc, Utc};
use orm::migrations::CustomMigrationSource;
use rewards::config::AppConfig;
use rewards::repository;
use rewards::services::{
    namada as namada_service, tendermint as tendermint_service,
};
use rewards::state::AppState;
use shared::client::Client;
use shared::crawler;
use shared::crawler_state::{CrawlerName, IntervalCrawlerState};
use shared::error::{AsDbError, AsRpcError, ContextDbInteractError, MainError};
use tendermint_rpc::HttpClient;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), MainError> {
    let config = AppConfig::parse();

    config.log.init();

    tracing::info!("version: {}", env!("VERGEN_GIT_SHA").to_string());

    let client = Client::new(&config.tendermint_url);

    let chain_id = tendermint_service::query_status(client.as_ref())
        .await
        .into_rpc_error()?
        .node_info
        .network
        .to_string();

    tracing::info!("Network chain id: {}", chain_id);

    let app_state = AppState::new(config.database_url).into_db_error()?;

    let conn = Arc::new(app_state.get_db_connection().await.into_db_error()?);

    // Run migrations
    CustomMigrationSource::new(chain_id)
        .run_migrations(&conn)
        .await
        .expect("Should be able to run migrations");

    tracing::debug!("Querying epoch...");

    let mut epoch = config.backfill_from;

    if epoch.is_none() {
        loop {
            epoch = Some(
                namada_service::get_current_epoch(client.as_ref())
                    .await
                    .into_rpc_error()?,
            );

            if epoch.unwrap_or(0) < 2 {
                tracing::info!("Waiting for first epoch to happen...");
                sleep(Duration::from_secs(config.sleep_for)).await;
            } else {
                break;
            }
        }
    }

    crawler::crawl(
        move |epoch| crawling_fn(conn.clone(), Arc::new(client.get()), epoch),
        epoch.unwrap_or(0),
        None,
    )
    .await
}

async fn crawling_fn(
    conn: Arc<Object>,
    client: Arc<HttpClient>,
    epoch_to_process: u32,
) -> Result<(), MainError> {
    let should_process = can_process(epoch_to_process, client.clone()).await?;

    if !should_process {
        let timestamp = Utc::now().naive_utc();
        update_crawler_timestamp(&conn, timestamp).await?;

        tracing::trace!(
            epoch = epoch_to_process,
            "Epoch does not exist yet, waiting...",
        );

        return Err(MainError::NoAction);
    }

    tracing::info!("Starting to update proposals...");

    // TODO: change this by querying all the pairs in the database
    let delegations_pairs = namada_service::query_delegation_pairs(&client)
        .await
        .into_rpc_error()?;

    tracing::info!(
        epoch = epoch_to_process,
        delegations = delegations_pairs.len(),
        "Querying rewards..."
    );

    let rewards = namada_service::query_rewards(
        &client,
        &delegations_pairs,
        epoch_to_process,
    )
    .await
    .into_rpc_error()?;

    let non_zero_rewards = rewards
        .iter()
        .filter(|reward| !reward.amount.is_zero())
        .cloned()
        .collect::<Vec<_>>();

    let timestamp = DateTimeUtc::now().0.timestamp();
    let crawler_state = IntervalCrawlerState { timestamp };

    tracing::info!(
        epoch = epoch_to_process,
        delegations = delegations_pairs.len(),
        rewards = rewards.len(),
        non_zero_rewards = non_zero_rewards.len(),
        "Queried rewards successfully",
    );

    conn.interact(move |conn| {
        conn.build_transaction().read_write().run(
            |transaction_conn: &mut diesel::pg::PgConnection| {
                repository::pos_rewards::upsert_rewards(
                    transaction_conn,
                    non_zero_rewards,
                    epoch_to_process as i32,
                )?;

                repository::crawler_state::upsert_crawler_state(
                    transaction_conn,
                    (CrawlerName::Rewards, crawler_state).into(),
                )?;

                Ok(())
            },
        )
    })
    .await
    .context_db_interact_error()
    .and_then(identity)
    .into_db_error()?;

    tracing::info!(
        epoch = epoch_to_process,
        "Inserted rewards into database; waiting for next epoch"
    );

    Ok(())
}

async fn can_process(
    epoch: u32,
    client: Arc<HttpClient>,
) -> Result<bool, MainError> {
    let current_epoch = namada_service::get_current_epoch(&client.clone())
        .await
        .map_err(|e| {
            tracing::error!(
                "Failed to query Namada's last committed block: {}",
                e
            );
            MainError::RpcError
        })?;

    Ok(current_epoch >= epoch)
}

async fn update_crawler_timestamp(
    conn: &Object,
    timestamp: NaiveDateTime,
) -> Result<(), MainError> {
    conn.interact(move |transaction_conn| {
        repository::crawler_state::update_timestamp(
            transaction_conn,
            timestamp,
        )?;

        anyhow::Ok(())
    })
    .await
    .context_db_interact_error()
    .and_then(identity)
    .into_db_error()
}
