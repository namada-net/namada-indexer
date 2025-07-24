use std::str::FromStr;

use anyhow::Context;
use namada_sdk::address::Address;
use namada_sdk::tendermint_rpc::HttpClient;

pub async fn query_account(
    client: &HttpClient,
    account_address: &str,
) -> anyhow::Result<()> {
    let address =
        Address::from_str(account_address).context("Invalid address format")?;
    let account = namada_sdk::rpc::get_account_info(client, &address)
        .await
        .context("Failed to query account")?;

    if let Some(account) = account {
        println!("Account Information: {:#?}", account);
    } else {
        println!("No account found for address: {}", account_address);
    }

    Ok(())
}
