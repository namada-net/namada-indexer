use anyhow::{Context, anyhow};
use namada_sdk::chain::BlockHeight as NamadaSdkBlockHeight;
use namada_sdk::hash::Hash;
use namada_sdk::queries::RPC;
use namada_sdk::rpc;
use namada_sdk::state::Key;
use shared::block::{BlockHeight, Epoch};
use shared::checksums::Checksums;
use shared::id::Id;
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

pub async fn get_native_token(client: &HttpClient) -> anyhow::Result<Id> {
    let native_token = RPC
        .shell()
        .native_token(client)
        .await
        .context("Failed to query native token")?;
    Ok(Id::from(native_token))
}

pub async fn get_current_epoch(client: &HttpClient) -> anyhow::Result<Epoch> {
    let epoch = rpc::query_epoch(client)
        .await
        .context("Failed to query Namada's current epoch")?;

    Ok(epoch.0 as Epoch)
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

pub async fn query_tx_code_hash(
    client: &HttpClient,
    tx_code_path: &str,
) -> Option<String> {
    let hash_key = Key::wasm_hash(tx_code_path);
    let (tx_code_res, _) =
        rpc::query_storage_value_bytes(client, &hash_key, None, false)
            .await
            .ok()?;
    if let Some(tx_code_bytes) = tx_code_res {
        let tx_code =
            Hash::try_from(&tx_code_bytes[..]).expect("Invalid code hash");
        Some(tx_code.to_string())
    } else {
        None
    }
}

pub async fn get_validator_namada_address(
    client: &HttpClient,
    tm_addr: &Id,
) -> anyhow::Result<Option<Id>> {
    let validator = RPC
        .vp()
        .pos()
        .validator_by_tm_addr(client, &tm_addr.to_string().to_uppercase())
        .await?;

    Ok(validator.map(Id::from))
}

pub async fn query_checksums(client: &HttpClient) -> Checksums {
    let mut checksums = Checksums::default();
    for code_path in Checksums::code_paths() {
        let code =
            query_tx_code_hash(client, &code_path)
                .await
                .unwrap_or_else(|| {
                    panic!("{} must be defined in namada storage.", code_path)
                });

        checksums.add(code_path, code.to_lowercase());
    }

    checksums
}

pub async fn get_first_block_in_epoch(
    client: &HttpClient,
) -> anyhow::Result<BlockHeight> {
    RPC.shell()
        .first_block_height_of_current_epoch(client)
        .await
        .context("Failed to query native token")
        .map(|height| height.0 as BlockHeight)
}
