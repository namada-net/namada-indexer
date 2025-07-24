use anyhow::Context;
use namada_sdk::rpc::query_native_token;
use namada_sdk::tendermint_rpc::HttpClient;
use shared::block::Block;
use shared::block_result::BlockResult;
use shared::checksums::Checksums;
use shared::id::Id;

use crate::namada::query_tx_code_hash;
use crate::utils::{
    query_raw_block_at_height, query_raw_block_results_at_height,
};

pub async fn deserialize_tx(
    client: &HttpClient,
    block_height: u32,
) -> anyhow::Result<()> {
    let native_token = query_native_token(client).await?;

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

    let tm_block_response = query_raw_block_at_height(client, block_height)
        .await
        .context("context: failed to query block")?;

    let tm_block_results_response =
        query_raw_block_results_at_height(client, block_height)
            .await
            .context("context: failed to query block results")?;
    let block_results = BlockResult::from(tm_block_results_response);

    let proposer_address_namada =
        Some(Id::Account("tnam1proposeraddress1234567890".to_string()));
    let epoch = 1;

    let block = Block::from(
        &tm_block_response,
        &block_results,
        &proposer_address_namada,
        &checksums,
        epoch,
        block_height,
        &native_token,
    );

    println!("Deserialized Block: {:#?}", block);

    Ok(())
}
