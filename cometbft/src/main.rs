use std::convert::identity;
use std::sync::Arc;
use std::time::Instant;

use anyhow::Context;
use chrono::{NaiveDateTime, Utc};
use clap::Parser;
use cometbft::app_state::AppState;
use cometbft::config::AppConfig;
use cometbft::repository::cometbft as cometbft_repo;
use cometbft::services::{
    db as db_service, namada as namada_service,
    tendermint as tendermint_service,
};
use deadpool_diesel::postgres::Object;
use futures::future;
use itertools::Itertools;
use shared::client::Client as SharedClient;
use shared::cometbft::CometbftBlock;
use shared::crawler::crawl;
use shared::error::{AsDbError, AsRpcError, ContextDbInteractError, MainError};
use tendermint_rpc::Client;
use tokio::time::sleep;

const CATCH_UP_THRESHOLD: u64 = 1000;

#[tokio::main]
async fn main() -> Result<(), MainError> {
    let config = AppConfig::parse();
    config.log.init();

    let client = SharedClient::new(&config.tendermint_url);

    let app_state = AppState::new(config.database_url).into_db_error()?;
    let conn = Arc::new(app_state.get_db_connection().await.into_db_error()?);

    let latest_block = tendermint_service::query_latest_block(client.as_ref())
        .await
        .into_rpc_error()?;

    let cometbft_state_height = db_service::get_cometbft_crawler(&conn)
        .await
        .into_db_error()?
        .map(|s| s.last_processed_block)
        .unwrap_or_default();

    let from_height = cometbft_state_height as u64 + 1;
    let to_height = latest_block.block.header.height.value();

    if to_height - CATCH_UP_THRESHOLD > from_height {
        tracing::info!(
            "Catching up from height {} to {}",
            from_height,
            to_height
        );
        initial_query(
            &client,
            conn.clone(),
            from_height,
            to_height,
            config.batch_size,
        )
        .await
        .into_rpc_error()?;
    }

    let cometbft_state_height = db_service::get_cometbft_crawler(&conn)
        .await
        .into_db_error()?
        .map(|s| s.last_processed_block)
        .unwrap_or_default();

    crawl(
        move |block_height| {
            crawling_fn(block_height, client.clone(), conn.clone())
        },
        cometbft_state_height,
        None,
    )
    .await
}

async fn crawling_fn(
    block_height: u32,
    client: SharedClient,
    conn: Arc<deadpool_diesel::postgres::Object>,
) -> Result<(), MainError> {
    let should_process = can_process(block_height, &client).await?;

    if !should_process {
        let timestamp = Utc::now().naive_utc();
        update_crawler_timestamp(&conn, timestamp).await?;

        tracing::trace!(
            block = block_height,
            "Block does not exist yet, waiting...",
        );

        return Err(MainError::NoAction);
    }

    let start = Instant::now();

    let (block, block_result, epoch) = tokio::try_join!(
        async {
            tendermint_service::query_raw_block_at_height(
                client.as_ref(),
                block_height,
            )
            .await
            .into_rpc_error()
        },
        async {
            tendermint_service::query_raw_block_results_at_height(
                client.as_ref(),
                block_height,
            )
            .await
            .into_rpc_error()
        },
        async {
            namada_service::get_epoch_at_block_height(
                client.as_ref(),
                block_height,
            )
            .await
            .into_rpc_error()
        }
    )?;

    let first_checkpoint = Instant::now();

    tracing::info!(
        block_height = block_height,
        time_taken = first_checkpoint.duration_since(start).as_secs_f64(),
        "Queried block successfully",
    );

    conn.interact(move |conn| {
        conn.build_transaction()
            .read_write()
            .run(|transaction_conn| {
                cometbft_repo::upsert_blocks(
                    transaction_conn,
                    vec![CometbftBlock {
                        block_height,
                        block,
                        events: block_result,
                        epoch,
                    }],
                )?;

                cometbft_repo::insert_crawler_state(
                    transaction_conn,
                    shared::crawler_state::BlockCrawlerState {
                        last_processed_block: block_height,
                        timestamp: chrono::Utc::now().timestamp(),
                    },
                )?;

                anyhow::Ok(())
            })
    })
    .await
    .context_db_interact_error()
    .and_then(identity)
    .into_db_error()?;

    let second_checkpoint = Instant::now();

    tracing::info!(
        block = block_height,
        time_taken = second_checkpoint
            .duration_since(first_checkpoint)
            .as_secs_f64(),
        "Inserted block into database"
    );

    Ok(())
}

pub async fn initial_query(
    client: &SharedClient,
    conn: Arc<deadpool_diesel::postgres::Object>,
    from_height: u64,
    to_height: u64,
    batch_size: usize,
) -> anyhow::Result<()> {
    let http_client = client.get();

    for (batch_num, height_chunk) in (from_height..=to_height)
        .chunks(batch_size)
        .into_iter()
        .enumerate()
    {
        let start = Instant::now();

        let chunk: Vec<_> = height_chunk.collect();
        let chunk_min_block_height = chunk.first().copied().unwrap_or_default();
        let chunk_max_block_height = chunk.last().copied().unwrap_or_default();

        loop {
            let batch_futures: Vec<_> = chunk
                .iter()
                .map(|&height| {
                    let client = http_client.clone();
                    async move {
                        let height = height as u32;
                        (
                            height,
                            client.block(height).await,
                            client.block_results(height).await,
                            namada_service::get_epoch_at_block_height(
                                &client, height,
                            )
                            .await,
                        )
                    }
                })
                .collect();

            let fetch_results = future::join_all(batch_futures).await;

            let mut successful_blocks = Vec::with_capacity(chunk.len());

            for result in fetch_results {
                match result {
                    (height, Ok(block), Ok(events), Ok(epoch)) => {
                        successful_blocks.push(CometbftBlock {
                            block_height: height,
                            block,
                            events,
                            epoch,
                        })
                    }
                    _ => break,
                }
            }

            let first_checkpoint = Instant::now();

            if successful_blocks.len() != chunk.len() {
                tracing::warn!(
                    "Failed to fetch all blocks in batch {}: expected {}, got \
                     {}",
                    batch_num,
                    chunk.len(),
                    successful_blocks.len()
                );
                sleep(std::time::Duration::from_secs(1)).await;
                continue;
            }

            tracing::info!(
                batch_size = chunk.len(),
                time_taken =
                    first_checkpoint.duration_since(start).as_secs_f64(),
                from_height = chunk_min_block_height,
                to_height = chunk_max_block_height,
                "Queried blocks successfully",
            );

            conn.interact(move |conn| {
                conn.build_transaction()
                    .read_write()
                    .run(|transaction_conn| {
                        cometbft_repo::upsert_blocks(
                            transaction_conn,
                            successful_blocks,
                        )?;

                        cometbft_repo::insert_crawler_state(
                            transaction_conn,
                            shared::crawler_state::BlockCrawlerState {
                                last_processed_block: chunk_max_block_height
                                    as u32,
                                timestamp: chrono::Utc::now().timestamp(),
                            },
                        )?;

                        anyhow::Ok(())
                    })
            })
            .await
            .context_db_interact_error()
            .and_then(identity)
            .into_db_error()?;

            let second_checkpoint = Instant::now();

            tracing::info!(
                batch_size = chunk.len(),
                time_taken = second_checkpoint
                    .duration_since(first_checkpoint)
                    .as_secs_f64(),
                "Inserted blocks into database"
            );

            break;
        }
    }
    Ok(())
}

async fn can_process(
    block_height: u32,
    client: &SharedClient,
) -> Result<bool, MainError> {
    let last_block_height = namada_service::get_last_block(client.as_ref())
        .await
        .map_err(|e| {
            tracing::error!(
                "Failed to query Namada's last committed block: {}",
                e
            );
            MainError::RpcError
        })?;

    Ok(last_block_height >= block_height)
}

async fn update_crawler_timestamp(
    conn: &Object,
    timestamp: NaiveDateTime,
) -> Result<(), MainError> {
    conn.interact(move |transaction_conn| {
        cometbft_repo::update_timestamp(transaction_conn, timestamp)?;

        anyhow::Ok(())
    })
    .await
    .context_db_interact_error()
    .into_db_error()?
    .context("Insert crawler state error")
    .into_db_error()
}
