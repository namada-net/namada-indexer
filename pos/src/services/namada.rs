use std::collections::HashSet;

use anyhow::Context;
use futures::{StreamExt, TryStreamExt};
use namada_core::chain::Epoch as NamadaSdkEpoch;
use namada_sdk::address::Address;
use namada_sdk::rpc;
use shared::block::Epoch;
use shared::id::Id;
use shared::validator::{Validator, ValidatorSet, ValidatorState};
use tendermint_rpc::HttpClient;

pub async fn get_validator_set_at_epoch(
    client: &HttpClient,
    epoch: Epoch,
) -> anyhow::Result<ValidatorSet> {
    let namada_epoch = to_epoch(epoch);
    let validator_set = rpc::get_all_validators(client, namada_epoch)
        .await
        .with_context(|| {
            format!(
                "Failed to query Namada's consensus validators at epoch \
                 {epoch}"
            )
        })?;

    let validators = futures::stream::iter(validator_set)
        .map(|address| async move {
            let voting_power_fut = async {
                rpc::get_validator_stake(client, namada_epoch, &address)
                    .await
                    .with_context(|| {
                        format!(
                            "Failed to query the stake of validator {address} \
                             at epoch {namada_epoch}"
                        )
                    })
            };

            let commission_fut = async {
                rpc::query_commission_rate(client, &address, Some(namada_epoch))
                    .await
                    .with_context(|| {
                        format!(
                            "Failed to query commission of validator \
                             {address} at epoch {namada_epoch}"
                        )
                    })
            };

            let validator_state_fut = async {
                rpc::get_validator_state(client, &address, Some(namada_epoch))
                    .await
                    .with_context(|| {
                        format!(
                            "Failed to query validator {address} \
                         state"
                        )
                    })
            };

            let (voting_power, commission_pair, validator_state) =
                futures::try_join!(voting_power_fut, commission_fut, validator_state_fut)?;
            let commission = commission_pair
                .commission_rate
                .expect("Commission rate has to exist")
                .to_string();
            let max_commission = commission_pair
                .max_commission_change_per_epoch
                .expect("Max commission rate change has to exist")
                .to_string();
            let validator_state = validator_state.0.map(ValidatorState::from).unwrap_or(ValidatorState::Unknown);

            anyhow::Ok(Validator {
                address: Id::Account(address.to_string()),
                voting_power: voting_power.to_string_native(),
                max_commission,
                commission,
                name: None,
                email: None,
                description: None,
                website: None,
                discord_handler: None,
                avatar: None,
                state: validator_state
            })
        })
        .buffer_unordered(shared::utils::rpc_concurrency())
        .try_collect::<HashSet<_>>()
        .await?;

    Ok(ValidatorSet { validators, epoch })
}

pub async fn get_validators_state(
    client: &HttpClient,
    validators: Vec<Validator>,
    epoch: Epoch,
) -> anyhow::Result<ValidatorSet> {
    let namada_epoch = to_epoch(epoch);

    let validators = futures::stream::iter(validators)
        .map(|mut validator| async move {
            let validator_address = Address::from(validator.address.clone());
            let validator_state = rpc::get_validator_state(
                client,
                &validator_address,
                Some(namada_epoch),
            )
            .await
            .with_context(|| {
                format!("Failed to query validator {validator_address} state")
            })?;

            let validator_state = validator_state
                .0
                .map(ValidatorState::from)
                .unwrap_or(ValidatorState::Unknown);

            let from_unjailing_state =
                validator.state.eq(&ValidatorState::Unjailing)
                    && !validator_state.eq(&ValidatorState::Jailed);
            let from_deactivating_state =
                validator.state.eq(&ValidatorState::Deactivating)
                    && validator_state.eq(&ValidatorState::Inactive);
            let from_reactivating_state =
                validator.state.eq(&ValidatorState::Reactivating)
                    && !validator_state.eq(&ValidatorState::Inactive);
            let from_concrete_state = ![
                ValidatorState::Deactivating,
                ValidatorState::Reactivating,
                ValidatorState::Unjailing,
            ]
            .contains(&validator.state);

            if from_unjailing_state
                || from_deactivating_state
                || from_reactivating_state
                || from_concrete_state
            {
                validator.state = validator_state;
            }

            anyhow::Ok(validator)
        })
        .buffer_unordered(shared::utils::rpc_concurrency())
        .try_collect::<HashSet<_>>()
        .await?;

    Ok(ValidatorSet { validators, epoch })
}

pub async fn get_current_epoch(client: &HttpClient) -> anyhow::Result<Epoch> {
    let epoch = rpc::query_epoch(client)
        .await
        .context("Failed to query Namada's current epoch")?;

    Ok(epoch.0 as Epoch)
}

fn to_epoch(epoch: u32) -> NamadaSdkEpoch {
    NamadaSdkEpoch::from(epoch as u64)
}
