pub mod config;
pub mod functions;
pub mod namada;
pub mod utils;

use clap::Parser;
use shared::client::Client;

use crate::config::AppConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = AppConfig::parse();

    let client = Client::new(&config.tendermint_url);

    if config.fix_tx {
        functions::fix::fix(client.as_ref()).await?;
    } else if config.deserialize_tx && config.block_height.is_some() {
        let block_height = config.block_height.unwrap();
        functions::deserialize_block::deserialize_tx(
            client.as_ref(),
            block_height,
        )
        .await?;
    } else if config.query_account && config.address.is_some() {
        let address = config.address.as_ref().unwrap();
        functions::query_account::query_account(client.as_ref(), address)
            .await?;
    } else {
        println!("No action specified.");
    }

    Ok(())
}
