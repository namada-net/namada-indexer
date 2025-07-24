use anyhow::{Context, anyhow};
use namada_sdk::chain::BlockHeight as NamadaSdkBlockHeight;
use namada_sdk::queries::RPC;
use namada_sdk::rpc;
use shared::block::{BlockHeight, Epoch};
use tendermint_rpc::HttpClient;

pub async fn get_last_block(
    client: &HttpClient,
) -> anyhow::Result<BlockHeight> {
    let last_block = RPC
        .shell()
        .last_block(client)
        .await
        .context("Failed to query Namada's last committed block")?;

    last_block
        .ok_or(anyhow::anyhow!("No last block found"))
        .map(|b| BlockHeight::from(b.height.0 as u32))
}

pub async fn get_epoch_at_block_height(
    client: &HttpClient,
    block_height: BlockHeight,
) -> anyhow::Result<Epoch> {
    let block_height = NamadaSdkBlockHeight::from(block_height as u64);
    let epoch = rpc::query_epoch_at_height(client, block_height)
        .await
        .with_context(|| {
            format!("Failed to query Namada's epoch at height {block_height}")
        })?
        .ok_or_else(|| {
            anyhow!("No Namada epoch found for height {block_height}")
        })?;
    Ok(epoch.0 as Epoch)
}
