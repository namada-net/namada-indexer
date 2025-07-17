use std::convert::identity;
use std::sync::Arc;

use chrono::NaiveDateTime;
use clap::Parser;
use deadpool_diesel::postgres::Object;
use namada_sdk::state::EPOCH_SWITCH_BLOCKS_DELAY;
use namada_sdk::time::{DateTimeUtc, Utc};
use orm::gas::GasPriceDb;
use orm::migrations::CustomMigrationSource;
use orm::parameters::ParametersInsertDb;
use parameters::app_state::AppState;
use parameters::config::AppConfig;
use parameters::repository;
use parameters::services::{
    namada as namada_service, tendermint as tendermint_service,
};
use shared::client::Client;
use shared::crawler;
use shared::crawler_state::{CrawlerName, IntervalCrawlerState};
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

    let current_epoch = namada_service::get_current_epoch(client.as_ref())
        .await
        .into_rpc_error()?;

    crawler::crawl(
        move |epoch| crawling_fn(epoch, conn.clone(), Arc::new(client.get())),
        current_epoch,
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

        tracing::trace!("New epoch does not exist yet, waiting...",);

        return Err(MainError::NoAction);
    }

    tracing::debug!("Querying parameters...");

    let parameters = namada_service::get_parameters(&client)
        .await
        .into_rpc_error()?;

    let genesis = tendermint_service::query_genesis(&client)
        .await
        .into_rpc_error()?;

    let checksums = namada_service::query_checksums(&client).await;

    let gas_price = namada_service::get_gas_price(&client).await;

    let timestamp = DateTimeUtc::now().0.timestamp();
    let crawler_state = IntervalCrawlerState { timestamp };

    tracing::info!("Queried parameters successfully",);

    conn.interact(move |conn| {
        conn.build_transaction()
            .read_write()
            .run(|transaction_conn| {
                repository::parameters::upsert_chain_parameters(
                    transaction_conn,
                    ParametersInsertDb::from((
                        parameters,
                        genesis,
                        checksums,
                        EPOCH_SWITCH_BLOCKS_DELAY,
                    )),
                )?;

                repository::parameters::upsert_gas_price(
                    transaction_conn,
                    gas_price
                        .iter()
                        .cloned()
                        .map(GasPriceDb::from)
                        .collect::<Vec<GasPriceDb>>(),
                )?;

                repository::crawler_state::upsert_crawler_state(
                    transaction_conn,
                    (CrawlerName::Parameters, crawler_state).into(),
                )?;

                anyhow::Ok(())
            })
    })
    .await
    .context_db_interact_error()
    .and_then(identity)
    .into_db_error()?;

    tracing::info!("Inserted parameters into database");

    Ok(())
}

async fn can_process(
    epoch: u32,
    client: Arc<HttpClient>,
) -> Result<bool, MainError> {
    let current_epoch = namada_service::get_current_epoch(&client.clone())
        .await
        .map_err(|e| {
            tracing::error!("Failed to query Namada's current epoch: {}", e);
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
