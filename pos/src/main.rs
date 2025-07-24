use std::convert::identity;
use std::sync::Arc;

use chrono::{NaiveDateTime, Utc};
use clap::Parser;
use deadpool_diesel::postgres::Object;
use namada_sdk::time::DateTimeUtc;
use orm::crawler_state::EpochStateInsertDb;
use orm::migrations::CustomMigrationSource;
use orm::validators::ValidatorInsertDb;
use pos::app_state::AppState;
use pos::config::AppConfig;
use pos::repository::{self};
use pos::services::{
    namada as namada_service, tendermint as tendermint_service,
};
use shared::client::Client;
use shared::crawler;
use shared::crawler_state::{CrawlerName, EpochCrawlerState};
use shared::error::{AsDbError, AsRpcError, ContextDbInteractError, MainError};
use tendermint_rpc::HttpClient;

#[tokio::main]
async fn main() -> Result<(), MainError> {
    let config = AppConfig::parse();

    config.log.init();

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

    // We always start from the current epoch
    let next_epoch = namada_service::get_current_epoch(client.as_ref())
        .await
        .into_rpc_error()?;

    crawler::crawl(
        move |epoch| crawling_fn(epoch, conn.clone(), Arc::new(client.get())),
        next_epoch,
        None,
    )
    .await
}

async fn crawling_fn(
    epoch_to_process: u32,
    conn: Arc<Object>,
    client: Arc<HttpClient>,
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

    let validators_set =
        namada_service::get_validator_set_at_epoch(&client, epoch_to_process)
            .await
            .into_rpc_error()?;

    tracing::info!(
        epoch = epoch_to_process,
        validators = validators_set.validators.len(),
        "Queried validators successfully...",
    );

    let timestamp = DateTimeUtc::now().0.timestamp();
    let crawler_state = EpochCrawlerState {
        last_processed_epoch: epoch_to_process,
        timestamp,
    };
    let crawler_state: EpochStateInsertDb =
        (CrawlerName::Pos, crawler_state).into();

    conn.interact(move |conn| {
        conn.build_transaction()
            .read_write()
            .run(|transaction_conn| {
                let validators_dbo = &validators_set
                    .validators
                    .into_iter()
                    .map(ValidatorInsertDb::from_validator)
                    .collect::<Vec<_>>();

                repository::pos::upsert_validators(
                    transaction_conn,
                    validators_dbo,
                )?;

                repository::crawler_state::upsert_crawler_state(
                    transaction_conn,
                    crawler_state,
                )?;

                anyhow::Ok(())
            })
    })
    .await
    .context_db_interact_error()
    .and_then(identity)
    .into_db_error()?;

    tracing::info!(epoch = epoch_to_process, "Updated validators in database");

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
