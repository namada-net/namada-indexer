use anyhow::Context;
use namada_sdk::tendermint_rpc::{Client, HttpClient};
use shared::genesis::{Genesis, GenesisParams, GenesisRequest};
use tendermint_rpc::endpoint::status::Response as TenderminStatusResponse;

pub async fn query_genesis(client: &HttpClient) -> anyhow::Result<Genesis> {
    let genesis_params: GenesisParams =
        client.perform(GenesisRequest).await?.genesis;

    Ok(Genesis::from(genesis_params))
}

pub async fn query_status(
    client: &HttpClient,
) -> anyhow::Result<TenderminStatusResponse> {
    client
        .status()
        .await
        .context("Failed to query CometBFT's status")
}
