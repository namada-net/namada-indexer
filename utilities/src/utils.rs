use std::time::Duration;

use anyhow::Context;
use namada_sdk::borsh::BorshDeserialize;
use namada_sdk::queries::RPC;
use namada_sdk::storage::{self, PrefixValue};
use namada_sdk::tendermint_rpc::endpoint::block::Response as TendermintBlockResponse;
use namada_sdk::tendermint_rpc::endpoint::block_results::Response as TendermintBlockResultResponse;
use namada_sdk::tendermint_rpc::{Client, HttpClient};
use shared::block::BlockHeight;
use tokio::time::sleep;

/// Query a range of storage values with a matching prefix and decode them with
/// [`BorshDeserialize`]. Returns an iterator of the storage keys paired with
/// their associated values.
pub async fn query_storage_prefix<T>(
    client: &HttpClient,
    key: &storage::Key,
    height: Option<BlockHeight>,
) -> anyhow::Result<Option<impl Iterator<Item = (storage::Key, T)>>>
where
    T: BorshDeserialize,
{
    let operation = || async {
        RPC.shell()
            .storage_prefix(
                client,
                None,
                height.map(super::namada::to_block_height),
                false,
                key,
            )
            .await
            .context("failed to query storage prefix")
    };

    let values = default_retry(operation).await?;

    let decode = |PrefixValue { key, value }: PrefixValue| {
        T::try_from_slice(&value[..]).map(|value| (key, value)).ok()
    };

    Ok(if values.data.is_empty() {
        None
    } else {
        Some(values.data.into_iter().filter_map(decode))
    })
}

pub async fn query_storage_value<T>(
    client: &HttpClient,
    key: &storage::Key,
    height: Option<BlockHeight>,
) -> anyhow::Result<Option<T>>
where
    T: BorshDeserialize,
{
    let operation = || async {
        RPC.shell()
            .storage_value(
                client,
                None,
                height.map(super::namada::to_block_height),
                false,
                key,
            )
            .await
            .context("failed to query storage value")
    };

    let value = default_retry(operation).await?;

    if value.data.is_empty() {
        Ok(None)
    } else {
        let value = BorshDeserialize::try_from_slice(&value.data)
            .context("Failed to deserialize value")?;
        Ok(Some(value))
    }
}

pub async fn query_storage_bytes(
    client: &HttpClient,
    key: &storage::Key,
    height: Option<BlockHeight>,
) -> anyhow::Result<Option<Vec<u8>>> {
    let operation = || async {
        RPC.shell()
            .storage_value(
                client,
                None,
                height.map(super::namada::to_block_height),
                false,
                key,
            )
            .await
            .context("failed to query storage value")
    };

    let value = default_retry(operation).await?;

    if value.data.is_empty() {
        Ok(None)
    } else {
        Ok(Some(value.data))
    }
}

async fn _retry<F, Fut, T>(
    mut operation: F,
    max_retries: u32,
    delay: Duration,
) -> anyhow::Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<T>>,
{
    let mut attempts = 0_u32;

    loop {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(err) if attempts < max_retries => {
                tracing::warn!(
                    "retrying operation after error: {:?} - ({}/{})",
                    err,
                    attempts,
                    max_retries
                );
                attempts += 1;
                sleep(delay * attempts).await;
            }
            Err(err) => return Err(err),
        }
    }
}

pub async fn query_raw_block_at_height(
    client: &HttpClient,
    height: u32,
) -> anyhow::Result<TendermintBlockResponse> {
    client
        .block(height)
        .await
        .context("Failed to query CometBFT's last committed height")
}

// TODO: map return to our type
pub async fn query_raw_block_results_at_height(
    client: &HttpClient,
    height: u32,
) -> anyhow::Result<TendermintBlockResultResponse> {
    client
        .block_results(height)
        .await
        .context("Failed to query CometBFT's block results")
}

pub async fn default_retry<F, Fut, T>(operation: F) -> anyhow::Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<T>>,
{
    _retry(operation, 3, Duration::from_millis(500)).await
}
