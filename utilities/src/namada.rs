use namada_core::chain::BlockHeight as NamadaSdkBlockHeight;
use namada_sdk::hash::Hash;
use namada_sdk::state::Key;
use namada_sdk::tendermint_rpc::HttpClient;
use shared::block::BlockHeight;

use crate::utils::query_storage_bytes;

pub async fn query_tx_code_hash(
    client: &HttpClient,
    tx_code_path: &str,
) -> Option<String> {
    let storage_key = Key::wasm_hash(tx_code_path);
    let tx_code_res =
        query_storage_bytes(client, &storage_key, None).await.ok()?;
    if let Some(tx_code_bytes) = tx_code_res {
        let tx_code =
            Hash::try_from(&tx_code_bytes[..]).expect("Invalid code hash");
        Some(tx_code.to_string())
    } else {
        None
    }
}

pub(super) fn to_block_height(
    block_height: BlockHeight,
) -> NamadaSdkBlockHeight {
    NamadaSdkBlockHeight::from(block_height as u64)
}
