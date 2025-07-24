use std::collections::HashSet;
use std::str::FromStr;

use anyhow::{Context, anyhow};
use futures::{StreamExt, TryStreamExt};
use namada_core::chain::{
    BlockHeight as NamadaSdkBlockHeight, Epoch as NamadaSdkEpoch,
};
use namada_sdk::address::{Address as NamadaSdkAddress, InternalAddress};
use namada_sdk::collections::HashMap;
use namada_sdk::hash::Hash;
use namada_sdk::ibc::IbcTokenHash;
use namada_sdk::ibc::storage::{ibc_trace_key_prefix, is_ibc_trace_key};
use namada_sdk::proof_of_stake::storage_key;
use namada_sdk::queries::RPC;
use namada_sdk::rpc::{
    bonds_and_unbonds, query_native_token, query_proposal_by_id,
};
use namada_sdk::state::Key;
use namada_sdk::storage::DbKeySeg;
use namada_sdk::token::Amount as NamadaSdkAmount;
use namada_sdk::{rpc, token};
use shared::balance::{Amount, Balance, Balances, TokenSupply};
use shared::block::{BlockHeight, Epoch};
use shared::checksums::Checksums;
use shared::id::Id;
use shared::pos::{
    Bond, BondAddresses, Bonds, Redelegation, Unbond, UnbondAddresses, Unbonds,
};
use shared::proposal::{GovernanceProposal, TallyType};
use shared::token::{IbcRateLimit, IbcToken, Token};
use shared::utils::BalanceChange;
use shared::validator::{Validator, ValidatorSet, ValidatorState};
use shared::vote::{GovernanceVote, ProposalVoteKind};
use subtle_encoding::hex;
use tendermint_rpc::HttpClient;

use super::utils::{
    default_retry, query_storage_bytes, query_storage_prefix,
    query_storage_value,
};

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
    let operation = || async {
        RPC.shell()
            .native_token(client)
            .await
            .context("Failed to query native token")
            .map(Id::from)
    };

    default_retry(operation).await
}

pub async fn query_native_token_total_supply(
    client: &HttpClient,
    native_token: &Id,
) -> anyhow::Result<Amount> {
    let native_token = NamadaSdkAddress::from_str(&native_token.to_string())
        .context("Failed to parse native token address")?;

    let operation = || async {
        rpc::get_token_total_supply(client, &native_token)
            .await
            .map(Amount::from)
            .context("Failed to query total supply of native token")
    };

    default_retry(operation).await
}

pub async fn query_native_token_effective_supply(
    client: &HttpClient,
) -> anyhow::Result<Amount> {
    let operation = || async {
        rpc::get_effective_native_supply(client)
            .await
            .map(Amount::from)
            .context("Failed to query effective supply of native token")
    };

    default_retry(operation).await
}

pub async fn get_first_block_in_epoch(
    client: &HttpClient,
) -> anyhow::Result<BlockHeight> {
    let operation = || async {
        RPC.shell()
            .first_block_height_of_current_epoch(client)
            .await
            .context("Failed to query native token")
            .map(|height| height.0 as BlockHeight)
    };

    default_retry(operation).await
}

pub async fn get_epoch_at_block_height(
    client: &HttpClient,
    block_height: BlockHeight,
) -> anyhow::Result<Epoch> {
    let block_height = to_block_height(block_height);
    let operation = || async {
        let epoch = rpc::query_epoch_at_height(client, block_height)
            .await
            .with_context(|| {
                format!(
                    "Failed to query Namada's epoch at height {block_height}"
                )
            })?
            .ok_or_else(|| {
                anyhow!("No Namada epoch found for height {block_height}")
            })?;

        Ok(epoch.0 as Epoch)
    };

    default_retry(operation).await
}

pub async fn query_balance(
    client: &HttpClient,
    balance_changes: &HashSet<BalanceChange>,
    block_height: BlockHeight,
) -> anyhow::Result<Balances> {
    Ok(futures::stream::iter(balance_changes)
        .filter_map(|balance_change| async move {
            tracing::debug!(
                "Fetching balance change for {} ...",
                balance_change.address
            );

            let owner =
                NamadaSdkAddress::from_str(&balance_change.address.to_string())
                    .context("Failed to parse owner address")
                    .ok()?;

            let token_addr = match &balance_change.token {
                Token::Ibc(IbcToken { address, .. }) => address.clone(),
                Token::Native(addr) => addr.clone(),
            }
            .into();

            let operation = || async {
                rpc::get_token_balance(
                    client,
                    &token_addr,
                    &owner,
                    Some(to_block_height(block_height)),
                )
                .await
                .context("Faile querying balance")
            };
            let amount = default_retry(operation).await.ok()?;

            Some(Balance {
                owner: balance_change.address.clone(),
                token: balance_change.token.clone(),
                amount: Amount::from(amount),
                height: block_height,
            })
        })
        .map(futures::future::ready)
        .buffer_unordered(32)
        .collect::<Vec<_>>()
        .await)
}

pub async fn query_tokens(client: &HttpClient) -> anyhow::Result<Vec<Token>> {
    let ibc_tokens = query_ibc_tokens(client).await?;
    let native_token = query_native_token(client).await?;

    let tokens = ibc_tokens
        .into_iter()
        .map(|ibc_token| {
            Token::Ibc(IbcToken {
                address: ibc_token.address,
                trace: ibc_token.trace,
            })
        })
        .chain(std::iter::once(Token::Native(Id::from(native_token))))
        .collect::<Vec<_>>();

    Ok(tokens)
}

async fn query_ibc_tokens(
    client: &HttpClient,
) -> anyhow::Result<HashSet<IbcToken>> {
    let prefix = ibc_trace_key_prefix(None);

    let mut tokens: HashSet<IbcToken> = HashSet::new();
    let ibc_traces =
        query_storage_prefix::<String>(client, &prefix, None).await?;

    if let Some(ibc_traces) = ibc_traces {
        for (key, ibc_trace) in ibc_traces {
            if let Some((_, hash)) = is_ibc_trace_key(&key) {
                let hash: IbcTokenHash = hash.parse().expect(
                    "Parsing an IBC token hash from storage shouldn't fail",
                );
                let ibc_token_addr =
                    NamadaSdkAddress::Internal(InternalAddress::IbcToken(hash));

                let token = IbcToken {
                    address: Id::from(ibc_token_addr),
                    trace: Some(Id::IbcTrace(ibc_trace.clone())),
                };

                tokens.insert(token);
            }
        }
    }

    Ok(tokens)
}

pub async fn query_all_balances(
    client: &HttpClient,
    height: BlockHeight,
) -> anyhow::Result<Balances> {
    let tokens = query_tokens(client).await?;
    let mut all_balances: Balances = vec![];

    for token in tokens.into_iter() {
        let balances = add_balance(client, token, height).await?;
        all_balances.extend(balances);
    }

    anyhow::Ok(all_balances)
}

async fn add_balance(
    client: &HttpClient,
    token: Token,
    height: BlockHeight,
) -> anyhow::Result<Vec<Balance>> {
    let mut all_balances: Vec<Balance> = vec![];
    let token_addr = match token {
        Token::Ibc(IbcToken { ref address, .. }) => address.clone(),
        Token::Native(ref addr) => addr.clone(),
    };

    let balance_prefix = namada_token::storage_key::balance_prefix(
        &NamadaSdkAddress::from(token_addr),
    );

    let balances =
        query_storage_prefix::<token::Amount>(client, &balance_prefix, None)
            .await?;

    if let Some(balances) = balances {
        for (key, balance) in balances {
            let (o, b) =
                match namada_token::storage_key::is_any_token_balance_key(&key)
                {
                    Some([_, owner]) => (owner.clone(), balance),
                    None => continue,
                };

            all_balances.push(Balance {
                owner: Id::from(o),
                token: token.clone(),
                amount: Amount::from(b),
                height,
            })
        }
    }

    Ok(all_balances)
}

pub async fn query_last_block_height(
    client: &HttpClient,
) -> anyhow::Result<BlockHeight> {
    let operation = || async {
        let height = RPC
            .shell()
            .last_block(client)
            .await
            .context("Failed to query Namada's last committed block")?
            .map(|height| height.height.0 as BlockHeight)
            .unwrap_or_default();

        Ok(height)
    };

    default_retry(operation).await
}

// TODO: this can be improved / optimized(bonds and unbonds can be processed in
// parallel)
pub async fn query_all_bonds_and_unbonds(
    client: &HttpClient,
    source: Option<Id>,
    target: Option<Id>,
) -> anyhow::Result<(Bonds, Unbonds)> {
    type Source = NamadaSdkAddress;
    type Validator = NamadaSdkAddress;
    type WithdrawEpoch = NamadaSdkEpoch;
    type StartEpoch = NamadaSdkEpoch;

    type BondKey = (Source, Validator, StartEpoch);
    type BondsMap = HashMap<BondKey, NamadaSdkAmount>;

    type UnbondKey = (Source, Validator, WithdrawEpoch);
    type UnbondsMap = HashMap<UnbondKey, NamadaSdkAmount>;

    let source = source.map(NamadaSdkAddress::from);
    let target = target.map(NamadaSdkAddress::from);

    let operation = || async {
        bonds_and_unbonds(client, &source, &target)
            .await
            .context("Failed to query bonds and unbonds")
    };

    let bonds_and_unbonds = default_retry(operation).await?;

    let mut bonds: BondsMap = HashMap::new();
    let mut unbonds: UnbondsMap = HashMap::new();

    // This is not super nice but it's fewer iterations that doing map and then
    // reduce
    for (bond_id, details) in bonds_and_unbonds {
        for bd in details.bonds {
            let id = bond_id.clone();
            let key = (id.source, id.validator, bd.start);

            if let Some(record) = bonds.get_mut(&key) {
                *record = record.checked_add(bd.amount).unwrap();
            } else {
                bonds.insert(key, bd.amount);
            }
        }

        for ud in details.unbonds {
            let id = bond_id.clone();
            let key = (id.source, id.validator, ud.withdraw);

            if let Some(record) = unbonds.get_mut(&key) {
                *record = record.checked_add(ud.amount).unwrap();
            } else {
                unbonds.insert(key, ud.amount);
            }
        }
    }

    // Map the types, mostly because we can't add indexer amounts
    let bonds = bonds
        .into_iter()
        .map(|((source, target, start), amount)| Bond {
            source: Id::from(source),
            target: Id::from(target),
            amount: Amount::from(amount),
            start: start.0 as Epoch,
        })
        .collect();

    let unbonds = unbonds
        .into_iter()
        .map(|((source, target, withdraw), amount)| Unbond {
            source: Id::from(source),
            target: Id::from(target),
            amount: Amount::from(amount),
            withdraw_at: withdraw.0 as Epoch,
        })
        .collect();

    Ok((bonds, unbonds))
}

pub async fn query_all_proposals(
    client: &HttpClient,
) -> anyhow::Result<Vec<GovernanceProposal>> {
    let last_proposal_id_key =
        namada_governance::storage::keys::get_counter_key();
    let last_proposal_id =
        query_storage_value::<u64>(client, &last_proposal_id_key, None)
            .await
            .context("Failed to query last proposal id")?
            .unwrap_or_default();

    let mut proposals: Vec<GovernanceProposal> = vec![];

    for id in 0..last_proposal_id {
        let proposal = query_proposal_by_id(client, id)
            .await
            .unwrap()
            .expect("Proposal should be written to storage.");
        let proposal_type = proposal.r#type.clone();

        // Create a governance proposal from the namada proposal, without the
        // data
        let mut governance_proposal = GovernanceProposal::from(proposal);

        // Get the proposal data based on the proposal type
        let proposal_data = match proposal_type {
            namada_governance::ProposalType::DefaultWithWasm(_) => {
                let wasm_code = query_proposal_code(client, id).await?;
                let hex_encoded = String::from_utf8(hex::encode(wasm_code))
                    .unwrap_or_default();
                Some(hex_encoded)
            }
            namada_governance::ProposalType::PGFSteward(data) => {
                Some(serde_json::to_string(&data).unwrap())
            }
            namada_governance::ProposalType::PGFPayment(data) => {
                Some(serde_json::to_string(&data).unwrap())
            }
            namada_governance::ProposalType::Default => None,
        };

        // Add the proposal data to the governance proposal
        governance_proposal.data = proposal_data;

        proposals.push(governance_proposal);
    }

    anyhow::Ok(proposals)
}

pub async fn query_proposal_code(
    client: &HttpClient,
    proposal_id: u64,
) -> anyhow::Result<Vec<u8>> {
    let proposal_code_key =
        namada_governance::storage::keys::get_proposal_code_key(proposal_id);
    let proposal_code =
        query_storage_value::<Vec<u8>>(client, &proposal_code_key, None)
            .await
            .expect("Proposal code should be written to storage.")
            .unwrap_or_default();

    anyhow::Ok(proposal_code)
}

pub async fn query_next_governance_id(
    client: &HttpClient,
    block_height: BlockHeight,
) -> anyhow::Result<u64> {
    // For block_height 0 the next id is always 0
    if block_height <= 1 {
        return Ok(0);
    }
    // For all other block heights we need to subtract 1
    // as namada already saved current block and bumped next proposal id
    let block_height = block_height - 1;

    let proposal_counter_key =
        namada_sdk::governance::storage::keys::get_counter_key();
    query_storage_value::<u64>(
        client,
        &proposal_counter_key,
        Some(block_height),
    )
    .await
    .context("Failed to get the next proposal id")
    .map(|id| id.expect("Next governance id should be written to storage"))
}

pub async fn query_bonds(
    client: &HttpClient,
    addresses: &HashSet<BondAddresses>,
) -> anyhow::Result<Vec<(Id, Id, Option<Bond>)>> {
    let nested_bonds = futures::stream::iter(addresses)
        .filter_map(|BondAddresses { source, target }| async move {
            // TODO: if this is too slow do not use query_all_bonds_and_unbonds
            let (bonds_res, _) = query_all_bonds_and_unbonds(
                client,
                Some(source.clone()),
                Some(target.clone()),
            )
            .await
            .context("Failed to query all bonds and unbonds")
            .ok()?;

            let bonds = if !bonds_res.is_empty() {
                bonds_res
                    .into_iter()
                    .map(|bond| (source.clone(), target.clone(), Some(bond)))
                    .collect::<Vec<_>>()
            } else {
                vec![(source.clone(), target.clone(), None)]
            };

            Some(bonds)
        })
        .map(futures::future::ready)
        .buffer_unordered(32)
        .collect::<Vec<_>>()
        .await;

    let bonds = nested_bonds.iter().flatten().cloned().collect();

    anyhow::Ok(bonds)
}

pub async fn query_unbonds(
    client: &HttpClient,
    addresses: HashSet<UnbondAddresses>,
) -> anyhow::Result<Unbonds> {
    let nested_unbonds = futures::stream::iter(addresses)
        .filter_map(|UnbondAddresses { source, validator }| {
            let source = NamadaSdkAddress::from_str(&source.to_string())
                .expect("Failed to parse source address");
            let validator = NamadaSdkAddress::from_str(&validator.to_string())
                .expect("Failed to parse validator address");

            async move {
                let operation = || async {
                    RPC.vp()
                        .pos()
                        .unbond_with_slashing(client, &source, &validator)
                        .await
                        .context("Failed to query unbond amount")
                };
                let unbonds = default_retry(operation).await.ok()?;

                let mut unbonds_map: HashMap<(Id, Id, Epoch), Amount> =
                    HashMap::new();

                for ((_, withdraw_epoch), amount) in unbonds {
                    let record = unbonds_map.get_mut(&(
                        Id::from(source.clone()),
                        Id::from(validator.clone()),
                        withdraw_epoch.0 as Epoch,
                    ));

                    // We have  to merge the unbonds with the same withdraw
                    // epoch into one otherwise we can't
                    // insert them into the db
                    match record {
                        Some(r) => {
                            *r = r.checked_add(&Amount::from(amount)).unwrap();
                        }
                        None => {
                            unbonds_map.insert(
                                (
                                    Id::from(source.clone()),
                                    Id::from(validator.clone()),
                                    withdraw_epoch.0 as Epoch,
                                ),
                                Amount::from(amount),
                            );
                        }
                    }
                }

                let unbonds: Vec<Unbond> = unbonds_map
                    .into_iter()
                    .map(|((source, target, start), amount)| Unbond {
                        source,
                        target,
                        amount,
                        withdraw_at: start,
                    })
                    .collect();

                Some(unbonds)
            }
        })
        .map(futures::future::ready)
        .buffer_unordered(32)
        .collect::<Vec<_>>()
        .await;

    let unbonds = nested_unbonds.iter().flatten().cloned().collect();

    anyhow::Ok(unbonds)
}

pub async fn query_redelegations(
    client: &HttpClient,
    addresses: &HashSet<BondAddresses>,
) -> anyhow::Result<Vec<Redelegation>> {
    futures::stream::iter(addresses)
        // We filter out address pairs that have no redelegations
        .filter_map(|BondAddresses { source, target }| async move {
            let end_epoch = rpc::query_incoming_redelegations(
                client,
                &NamadaSdkAddress::from(target.clone()),
                &NamadaSdkAddress::from(source.clone()),
            )
            .await
            .context("Failed to query incoming redelegations");

            end_epoch.transpose().map(|epoch| {
                epoch.map(|e| Redelegation {
                    delegator: source.clone(),
                    validator: target.clone(),
                    end_epoch: e.0 as Epoch,
                })
            })
        })
        .map(futures::future::ready)
        .buffer_unordered(20)
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<anyhow::Result<Vec<_>>>()
}

pub async fn get_current_epoch(client: &HttpClient) -> anyhow::Result<Epoch> {
    let operation = || async {
        rpc::query_epoch(client)
            .await
            .context("Failed to query Namada's current epoch")
            .map(|epoch| epoch.0 as Epoch)
    };

    default_retry(operation).await
}

pub async fn get_all_consensus_validators_addresses_at(
    client: &HttpClient,
    epoch: u32,
    native_token: Id,
) -> anyhow::Result<HashSet<BalanceChange>> {
    let operation = || async {
        let validators =
            rpc::get_all_consensus_validators(client, (epoch as u64).into())
                .await
                .context("Failed to query Namada's current epoch")?
                .into_iter()
                .map(|validator| {
                    BalanceChange::new(
                        Id::from(validator.address),
                        Token::Native(native_token.clone()),
                    )
                })
                .collect::<HashSet<_>>();

        Ok(validators)
    };

    default_retry(operation).await
}

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

pub async fn is_steward(
    client: &HttpClient,
    address: &Id,
) -> anyhow::Result<bool> {
    let address = NamadaSdkAddress::from(address.clone());
    let operation = || async { Ok(rpc::is_steward(client, &address).await) };

    default_retry(operation).await
}

pub async fn query_tallies(
    client: &HttpClient,
    proposals: Vec<GovernanceProposal>,
) -> anyhow::Result<Vec<(GovernanceProposal, TallyType)>> {
    let proposals = futures::stream::iter(proposals)
        .filter_map(|proposal| async move {
            let is_steward = is_steward(client, &proposal.author).await.ok()?;
            let tally_type = TallyType::from(&proposal.r#type, is_steward);

            Some((proposal, tally_type))
        })
        .map(futures::future::ready)
        .buffer_unordered(32)
        .collect::<Vec<_>>()
        .await;

    anyhow::Ok(proposals)
}

pub async fn query_all_votes(
    client: &HttpClient,
    proposals_ids: Vec<u64>,
) -> anyhow::Result<HashSet<GovernanceVote>> {
    let votes = futures::stream::iter(proposals_ids)
        .filter_map(|proposal_id| async move {
            let operation = || async {
                rpc::query_proposal_votes(client, proposal_id)
                    .await
                    .context("Failed to query proposal votes")
            };
            let votes = default_retry(operation).await.ok()?;

            let votes = votes
                .into_iter()
                .map(|vote| GovernanceVote {
                    proposal_id,
                    vote: ProposalVoteKind::from(vote.data),
                    address: Id::from(vote.delegator),
                })
                .collect::<HashSet<_>>();

            Some(votes)
        })
        .map(futures::future::ready)
        .buffer_unordered(32)
        .collect::<Vec<_>>()
        .await;

    let mut voter_count: HashMap<(Id, u64), u64> = HashMap::new();
    for vote in votes.iter().flatten() {
        *voter_count
            .entry((vote.address.clone(), vote.proposal_id))
            .or_insert(0) += 1;
    }

    let mut seen_voters = HashSet::new();
    anyhow::Ok(
        votes
            .iter()
            .flatten()
            .filter(|&vote| {
                seen_voters.insert((vote.address.clone(), vote.proposal_id))
            })
            .cloned()
            .map(|mut vote| {
                if let Some(count) =
                    voter_count.get(&(vote.address.clone(), vote.proposal_id))
                {
                    if *count > 1_u64 {
                        vote.vote = ProposalVoteKind::Unknown;
                    }
                    vote
                } else {
                    vote
                }
            })
            .collect(),
    )
}

pub async fn get_validator_set_at_epoch(
    client: &HttpClient,
    epoch: Epoch,
) -> anyhow::Result<ValidatorSet> {
    let namada_epoch = NamadaSdkEpoch::from(epoch as u64);
    let operation = || async {
        rpc::get_all_validators(client, namada_epoch)
            .await
            .with_context(|| {
                format!(
                    "Failed to query Namada's consensus validators at epoch \
                     {epoch}"
                )
            })
    };
    let validator_set = default_retry(operation).await?;

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

            let validator_metadata_fut = async {
                rpc::query_metadata(client, &address, Some(namada_epoch))
                    .await
                    .with_context(|| {
                        format!(
                            "Failed to query validator {address} \
                         state"
                        )
                    })
            };

            let (voting_power, validator_metadata, validator_state) =
                futures::try_join!(voting_power_fut, validator_metadata_fut, validator_state_fut)?;
            let commission = validator_metadata.1
                .commission_rate
                .expect("Commission rate has to exist")
                .to_string();
            let max_commission = validator_metadata.1
                .max_commission_change_per_epoch
                .expect("Max commission rate change has to exist")
                .to_string();
            let validator_state = validator_state.0.map(ValidatorState::from).unwrap_or(ValidatorState::Unknown);
            let validator_metadata = validator_metadata.0;

            anyhow::Ok(Validator {
                address: Id::Account(address.to_string()),
                voting_power: voting_power.to_string_native(),
                max_commission,
                commission,
                name: validator_metadata.clone().map(|metadata| metadata.name).unwrap_or(None),
                email: validator_metadata.clone().map(|metadata| Some(metadata.email)).unwrap_or(None),
                description: validator_metadata.clone().map(|metadata| metadata.description).unwrap_or(None),
                website: validator_metadata.clone().map(|metadata| metadata.website).unwrap_or(None),
                discord_handler: validator_metadata.clone().map(|metadata| metadata.discord_handle).unwrap_or(None),
                avatar: validator_metadata.map(|metadata| metadata.avatar).unwrap_or(None),
                state: validator_state
            })
        })
        .buffer_unordered(32)
        .try_collect::<HashSet<_>>()
        .await?;

    Ok(ValidatorSet { validators, epoch })
}

pub async fn get_validator_namada_address(
    client: &HttpClient,
    tm_addr: &Id,
) -> anyhow::Result<Option<Id>> {
    let operation = || async {
        let validator_addr = RPC
            .vp()
            .pos()
            .validator_by_tm_addr(client, &tm_addr.to_string().to_uppercase())
            .await?
            .map(Id::from);

        Ok(validator_addr)
    };

    default_retry(operation).await
}

pub fn query_native_addresses_balance_change(
    native_token: Token,
) -> HashSet<BalanceChange> {
    [
        NamadaSdkAddress::Internal(InternalAddress::Governance),
        NamadaSdkAddress::Internal(InternalAddress::PoS),
        NamadaSdkAddress::Internal(InternalAddress::Masp),
        NamadaSdkAddress::Internal(InternalAddress::Pgf),
        NamadaSdkAddress::Internal(InternalAddress::Ibc),
    ]
    .into_iter()
    .map(|address| BalanceChange::new(Id::from(address), native_token.clone()))
    .collect::<HashSet<_>>()
}

pub async fn query_pipeline_length(client: &HttpClient) -> anyhow::Result<u64> {
    let operation = || async {
        rpc::get_pos_params(client)
            .await
            .with_context(|| "Failed to query pos parameters".to_string())
            .map(|parameters| parameters.pipeline_len)
    };

    default_retry(operation).await
}

pub(super) fn to_block_height(
    block_height: BlockHeight,
) -> NamadaSdkBlockHeight {
    NamadaSdkBlockHeight::from(block_height as u64)
}

pub async fn get_pgf_receipients(
    client: &HttpClient,
    native_token: Id,
) -> HashSet<BalanceChange> {
    let payments = || async {
        rpc::query_pgf_fundings(client)
            .await
            .context("Failed to query PGF fundings")
    };

    default_retry(payments)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|payment| BalanceChange {
            address: Id::Account(payment.detail.target()),
            token: Token::Native(native_token.clone()),
        })
        .collect::<HashSet<_>>()
}

pub async fn get_native_token_supply(
    client: &HttpClient,
    native_token: &Id,
    epoch: u32,
) -> anyhow::Result<TokenSupply> {
    let total_supply_fut =
        query_native_token_total_supply(client, native_token);
    let effective_supply_fut = query_native_token_effective_supply(client);

    let (total_supply, effective_supply) =
        futures::try_join!(total_supply_fut, effective_supply_fut)
            .context("Failed to query native token supplies")?;

    anyhow::Ok(TokenSupply {
        address: native_token.to_string(),
        epoch: epoch as _,
        total: total_supply.into(),
        effective: Some(effective_supply.into()),
    })
}

pub async fn get_token_supply(
    client: &HttpClient,
    token: String,
    epoch: u32,
) -> anyhow::Result<TokenSupply> {
    let address: NamadaSdkAddress =
        token.parse().context("Failed to parse token address")?;

    let operation = || async {
        rpc::get_token_total_supply(client, &address)
            .await
            .map(Amount::from)
            .with_context(|| {
                format!("Failed to query total supply of token {token}")
            })
    };

    let supply = default_retry(operation).await?;

    anyhow::Ok(TokenSupply {
        address: token,
        epoch: epoch as _,
        total: supply.into(),
        effective: None,
    })
}

pub async fn get_throughput_rate_limit(
    client: &HttpClient,
    token: String,
    epoch: u32,
) -> anyhow::Result<IbcRateLimit> {
    let address: NamadaSdkAddress =
        token.parse().context("Failed to parse token address")?;

    let rate_limit = || async {
        rpc::query_ibc_rate_limits(client, &address)
            .await
            .with_context(|| {
                format!(
                    "Failed to query throughput rate limit of token {token}"
                )
            })
    };

    let rate_limit = default_retry(rate_limit).await?;

    Ok(IbcRateLimit {
        epoch,
        address: token,
        throughput_limit: Amount::from(rate_limit.throughput_per_epoch_limit)
            .into(),
    })
}

pub async fn get_rate_limits_for_tokens<I>(
    client: &HttpClient,
    tokens: I,
    epoch: u32,
) -> anyhow::Result<Vec<IbcRateLimit>>
where
    I: IntoIterator<Item = String>,
{
    let mut buffer = vec![];

    let mut stream = futures::stream::iter(tokens)
        .map(|address| get_throughput_rate_limit(client, address, epoch))
        .buffer_unordered(32);

    while let Some(maybe_address) = stream.next().await {
        buffer.push(maybe_address?);
    }

    Ok(buffer)
}

pub async fn query_all_redelegations(
    client: &HttpClient,
    validator_addresses: Vec<Id>,
) -> anyhow::Result<Vec<Redelegation>> {
    let nested_delegations = futures::stream::iter(validator_addresses.clone())
        // Some validators might not have any redelegations
        .filter_map(|validator_address| async move {
            let key = storage_key::validator_incoming_redelegations_key(
                &validator_address.clone().into(),
            );

            query_storage_prefix::<NamadaSdkEpoch>(client, &key, None)
                .await
                .context("Failed to query incoming redelegations")
                .transpose()
                .map(|opt_iter| {
                    opt_iter.map(|iter| {
                        (validator_address, iter.collect::<Vec<_>>())
                    })
                })
        })
        .map(|res| async move {
            let (validator_address, redelegations) = res?;

            redelegations
                .into_iter()
                .map(|r| {
                    let (key, epoch) = r;
                    let delegator = key
                        .segments
                        .last()
                        .context("Can't get delegator address")?;

                    let delegator = match delegator {
                        DbKeySeg::AddressSeg(delegator) => {
                            anyhow::Ok(delegator)
                        }
                        _ => Err(anyhow!("Invalid db key segment")),
                    }?;

                    let redelegation = Redelegation {
                        delegator: Id::from(delegator.clone()),
                        validator: validator_address.clone(),
                        end_epoch: epoch.0 as Epoch,
                    };

                    Ok(redelegation)
                })
                .collect::<anyhow::Result<Vec<_>>>()
        })
        .buffer_unordered(20)
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<anyhow::Result<Vec<Vec<Redelegation>>>>()?;

    Ok(nested_delegations.into_iter().flatten().collect())
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

pub async fn get_validator_addresses_at_epoch(
    client: &HttpClient,
    epoch: Epoch,
) -> anyhow::Result<Vec<Id>> {
    let namada_epoch = to_epoch(epoch);
    let validator_set = rpc::get_all_validators(client, namada_epoch)
        .await
        .with_context(|| {
            format!(
                "Failed to query Namada's consensus validators at epoch \
                 {epoch}"
            )
        })?;

    let validators = validator_set.into_iter().map(Id::from).collect();

    Ok(validators)
}

fn to_epoch(epoch: u32) -> NamadaSdkEpoch {
    NamadaSdkEpoch::from(epoch as u64)
}
