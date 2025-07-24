use std::collections::HashSet;
use std::time::Duration;

use anyhow::Context;
use futures::StreamExt;
use namada_sdk::queries::RPC;
use namada_sdk::rpc;
use shared::balance::Amount;
use shared::block::Epoch;
use shared::id::Id;
use shared::rewards::Reward;
use shared::utils::DelegationPair;
use tendermint_rpc::HttpClient;

pub async fn query_delegation_pairs(
    client: &HttpClient,
) -> anyhow::Result<HashSet<DelegationPair>> {
    let data = rpc::bonds_and_unbonds(client, &None, &None)
        .await
        .with_context(|| {
            "Failed to query Namada's bonds and unbonds".to_string()
        })?;

    let pairs =
        data.into_iter()
            .fold(HashSet::new(), |mut acc, (bond_id, _)| {
                acc.insert(DelegationPair {
                    validator_address: Id::from(bond_id.validator.clone()),
                    delegator_address: Id::from(bond_id.source),
                });
                acc.insert(DelegationPair {
                    validator_address: Id::from(bond_id.validator.clone()),
                    delegator_address: Id::from(bond_id.validator),
                });
                acc
            });

    anyhow::Ok(pairs)
}

pub async fn query_rewards(
    client: &HttpClient,
    delegation_pairs: &HashSet<DelegationPair>,
    epoch: Epoch,
) -> anyhow::Result<Vec<Reward>> {
    let mut all_rewards: Vec<Reward> = Vec::new();

    let batches: Vec<(usize, Vec<DelegationPair>)> = delegation_pairs
        .clone()
        .into_iter()
        .collect::<Vec<_>>()
        .chunks(32)
        .enumerate()
        .map(|(i, chunk)| (i, chunk.to_vec()))
        .collect();

    tracing::info!(
        "Got {} batches with a total of {} rewards to query...",
        batches.len(),
        delegation_pairs.len()
    );

    let results = futures::stream::iter(batches)
        .map(|batch| process_batch_with_retries(client, batch, epoch))
        .buffer_unordered(3)
        .collect::<Vec<_>>()
        .await;

    tracing::info!("Done fetching rewards!");

    for result in results {
        match result {
            Ok(mut rewards) => all_rewards.append(&mut rewards),
            Err(err) => return Err(err),
        }
    }

    Ok(all_rewards)
}

pub async fn get_current_epoch(client: &HttpClient) -> anyhow::Result<Epoch> {
    let epoch = rpc::query_epoch(client)
        .await
        .context("Failed to query Namada's current epoch")?;

    Ok(epoch.0 as Epoch)
}

async fn process_batch_with_retries(
    client: &HttpClient,
    batch: (usize, Vec<DelegationPair>),
    epoch: Epoch,
) -> anyhow::Result<Vec<Reward>> {
    let mut retries = 0;

    tracing::info!("Processing batch {}", batch.0);
    loop {
        let result = process_batch(client, batch.1.clone(), epoch).await;

        match result {
            Ok(rewards) => {
                tracing::info!("Batch {} done!", batch.0);
                return Ok(rewards);
            }
            Err(err) => {
                retries += 1;
                tracing::warn!(
                    "Batch reward failed (attempt {}/{}) - Error: {:?}",
                    retries,
                    3,
                    err
                );

                if retries >= 3 {
                    tracing::error!(
                        "Batch reward failed after maximum retries."
                    );
                    return Err(err);
                }
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }
    }
}

async fn process_batch(
    client: &HttpClient,
    batch: Vec<DelegationPair>,
    epoch: Epoch,
) -> anyhow::Result<Vec<Reward>> {
    Ok(futures::stream::iter(batch)
        .filter_map(|delegation| async move {
            tracing::debug!(
                "Fetching rewards {} -> {} ...",
                delegation.validator_address,
                delegation.delegator_address
            );

            let reward = RPC
                .vp()
                .pos()
                .rewards(
                    client,
                    &delegation.validator_address.clone().into(),
                    &Some(delegation.delegator_address.clone().into()),
                    &Some((epoch as u64).into()),
                )
                .await
                .ok()?;

            tracing::debug!(
                "Done fetching reward for {} -> {}!",
                delegation.validator_address,
                delegation.delegator_address
            );

            Some(Reward {
                delegation_pair: delegation.clone(),
                amount: Amount::from(reward),
                epoch: epoch as i32,
            })
        })
        .map(futures::future::ready)
        .buffer_unordered(32)
        .collect::<Vec<_>>()
        .await)
}
