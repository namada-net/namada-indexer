use std::collections::BTreeMap;

use anyhow::Context;
use namada_core::chain::Epoch as NamadaEpoch;
use namada_parameters::EpochDuration;
use namada_sdk::address::Address as NamadaAddress;
use namada_sdk::arith::checked;
use namada_sdk::dec::Dec;
use namada_sdk::hash::Hash;
use namada_sdk::proof_of_stake::storage_key as pos_storage_key;
use namada_sdk::queries::RPC;
use namada_sdk::rpc::{
    self, get_token_total_supply, get_total_staked_tokens, query_storage_value,
};
use namada_sdk::state::Key;
use namada_sdk::token::Amount as NamadaSdkAmount;
use shared::balance::Amount;
use shared::block::Epoch;
use shared::checksums::Checksums;
use shared::gas::GasPrice;
use shared::parameters::Parameters;
use tendermint_rpc::HttpClient;

async fn query_tx_code_hash(
    client: &HttpClient,
    tx_code_path: &str,
) -> anyhow::Result<Option<String>> {
    let hash_key = Key::wasm_hash(tx_code_path);
    let (tx_code_res, _) =
        rpc::query_storage_value_bytes(client, &hash_key, None, false).await?;
    if let Some(tx_code_bytes) = tx_code_res {
        let tx_code =
            Hash::try_from(&tx_code_bytes[..]).expect("Invalid code hash");
        Ok(Some(tx_code.to_string()))
    } else {
        Ok(None)
    }
}

pub async fn query_checksums(client: &HttpClient) -> anyhow::Result<Checksums> {
    let mut checksums = Checksums::default();
    for code_path in Checksums::code_paths() {
        let code = query_tx_code_hash(client, &code_path)
            .await
            .with_context(|| format!("Failed to query {} hash", code_path))?
            .with_context(|| {
                format!("{} must be defined in namada storage.", code_path)
            })?;

        checksums.add_with_ext(code_path, code.to_lowercase());
    }

    Ok(checksums)
}

pub async fn get_parameters(client: &HttpClient) -> anyhow::Result<Parameters> {
    let pos_parameters = rpc::get_pos_params(client)
        .await
        .with_context(|| "Failed to query pos parameters".to_string())?;

    let epochs_per_year_key =
        namada_parameters::storage::get_epochs_per_year_key();
    let epochs_per_year: u64 =
        rpc::query_storage_value(client, &epochs_per_year_key)
            .await
            .with_context(|| {
                "Failed to query epochs_per_year parameter".to_string()
            })?;

    let epoch_duration_key =
        namada_parameters::storage::get_epoch_duration_storage_key();
    let epoch_duration: EpochDuration =
        rpc::query_storage_value(client, &epoch_duration_key)
            .await
            .with_context(|| {
                "Failed to query epochs_per_year parameter".to_string()
            })?;

    let native_token_address = RPC
        .shell()
        .native_token(client)
        .await
        .context("Failed to query native token")?;

    let max_block_time = RPC.shell().max_block_time(client).await?;

    let maybe_apr = rpc::get_staking_rewards_rate(client).await;

    // ATM namada throws an error when epoch is 0
    // this is a workaround to set APR to 0 in this case
    let apr = match maybe_apr {
        Ok(apr) => Ok(apr.staking_rewards_rate.to_string()),
        Err(e) => {
            let epoch = rpc::query_epoch(client).await?;
            if epoch.0 == 0 {
                Ok("0".to_string())
            } else {
                Err(e)
            }
        }
    }?;

    Ok(Parameters {
        unbonding_length: pos_parameters.unbonding_len,
        pipeline_length: pos_parameters.pipeline_len,
        epochs_per_year,
        min_num_of_blocks: epoch_duration.min_num_of_blocks,
        min_duration: epoch_duration.min_duration.0,
        max_block_time: max_block_time.0,
        apr,
        native_token_address: native_token_address.to_string(),
        cubic_slashing_window_length: pos_parameters
            .cubic_slashing_window_length,
        duplicate_vote_min_slash_rate: pos_parameters
            .duplicate_vote_min_slash_rate
            .to_string(),
        light_client_attack_min_slash_rate: pos_parameters
            .light_client_attack_min_slash_rate
            .to_string(),
    })
}

pub async fn get_gas_price(client: &HttpClient) -> Vec<GasPrice> {
    let min_gas_price_key = namada_parameters::storage::get_gas_cost_key();
    let gas_cost_table = query_storage_value::<
        HttpClient,
        BTreeMap<NamadaAddress, NamadaSdkAmount>,
    >(client, &min_gas_price_key)
    .await
    .expect("Gas cost table should be defined.");

    let mut gas_table: Vec<GasPrice> = Vec::new();

    for (token, gas_cost) in gas_cost_table {
        gas_table.push(GasPrice {
            token: token.to_string(),
            amount: Amount::from(gas_cost),
        })
    }

    gas_table
}

pub async fn get_current_epoch(client: &HttpClient) -> anyhow::Result<Epoch> {
    let epoch = rpc::query_epoch(client)
        .await
        .context("Failed to query Namada's current epoch")?;

    Ok(epoch.0 as Epoch)
}

async fn _calc_apr(
    client: &HttpClient,
    epoch: NamadaEpoch,
    native_token_address: &NamadaAddress,
    epochs_per_year: u64,
) -> anyhow::Result<String> {
    let bonded_tokens = get_total_staked_tokens(client, epoch)
        .await
        .expect("Bonded tokens should be valid");

    let total_supply = get_token_total_supply(client, native_token_address)
        .await
        .expect("Total supply should be written to storage.");

    let pos_inflation_key = pos_storage_key::last_pos_inflation_amount_key();
    let inflation_amount: NamadaSdkAmount =
        query_storage_value(client, &pos_inflation_key)
            .await
            .expect("Inflation amount should be written to storage.");

    // Total supply of native token
    let s_nam = Dec::try_from(total_supply).unwrap();

    // Stored inflation amount per epoch
    let i_pos_last = Dec::try_from(inflation_amount).unwrap();

    // Inflation rate per year
    let i_rate_pos = checked!(i_pos_last / s_nam * epochs_per_year).unwrap();

    // Total bonded tokens
    let l_pos = Dec::try_from(bonded_tokens).unwrap();

    // Annual provision
    let a_prov = i_rate_pos
        .checked_mul(s_nam)
        .expect("Annual provision should be valid");

    // Nominal APR
    let apr_nom = a_prov
        .checked_div(l_pos)
        .expect("Nominal APR should be valid");

    Ok(apr_nom.to_string())
}
