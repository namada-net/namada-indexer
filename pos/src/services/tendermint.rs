use anyhow::Context;
use tendermint_rpc::endpoint::status::Response as TenderminStatusResponse;
use tendermint_rpc::{Client, HttpClient};

pub async fn query_status(
    client: &HttpClient,
) -> anyhow::Result<TenderminStatusResponse> {
    client
        .status()
        .await
        .context("Failed to query CometBFT's status")
}
