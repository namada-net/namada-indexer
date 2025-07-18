use std::collections::{BTreeMap, HashSet};

use namada_ibc::IbcMessage;
use namada_ibc::core::channel::types::msgs::{MsgRecvPacket, PacketMsg};
use namada_ibc::core::handler::types::msgs::MsgEnvelope;
use namada_sdk::address::Address;
use namada_sdk::borsh::BorshDeserialize;
use namada_sdk::token::Transfer;
use subtle_encoding::hex;
use tendermint_rpc::endpoint::block::Response as TendermintBlockResponse;

use crate::block_result::BlockResult;
use crate::checksums::Checksums;
use crate::header::BlockHeader;
use crate::id::Id;
use crate::masp::{MaspEntry, MaspEntryDirection};
use crate::pos::{BondAddresses, UnbondAddresses};
use crate::proposal::{GovernanceProposal, GovernanceProposalKind};
use crate::public_key::PublicKey;
use crate::token::{IbcToken, Token};
use crate::transaction::{
    InnerTransaction, Transaction, TransactionKind, TransactionTarget,
    WrapperTransaction,
};
use crate::utils::{BalanceChange, MASP_ADDRESS, ibc_ack_to_balance_info};
use crate::validator::{
    Validator, ValidatorMetadataChange, ValidatorState, ValidatorStateChange,
};
use crate::vote::GovernanceVote;

pub type Epoch = u32;
pub type BlockHeight = u32;

#[derive(Debug, Clone, Default, Copy)]
pub enum TxEventStatusCode {
    Ok,
    #[default]
    Fail,
}

impl From<&str> for TxEventStatusCode {
    fn from(value: &str) -> Self {
        match value {
            "0" => Self::Ok,
            _ => Self::Fail,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Block {
    pub hash: Id,
    pub header: BlockHeader,
    pub transactions: Vec<(WrapperTransaction, Vec<InnerTransaction>)>,
    pub epoch: Epoch,
}

impl Block {
    pub fn from(
        block_response: &TendermintBlockResponse,
        block_results: &BlockResult,
        proposer_address_namada: &Option<Id>,
        checksums: &Checksums,
        epoch: Epoch,
        block_height: BlockHeight,
        native_token: &Address,
    ) -> Self {
        let transactions = block_response
            .block
            .data
            .iter()
            .enumerate()
            .filter_map(|(index, tx_raw_bytes)| {
                Transaction::deserialize(
                    tx_raw_bytes,
                    index,
                    block_height,
                    checksums.clone(),
                    block_results,
                    native_token,
                )
                .map_err(|reason| {
                    tracing::info!("Couldn't deserialize tx due to {}", reason);
                })
                .ok()
            })
            .collect::<Vec<(WrapperTransaction, Vec<InnerTransaction>)>>();

        Block {
            hash: Id::from(block_response.block_id.hash),
            header: BlockHeader {
                height: block_response.block.header.height.value()
                    as BlockHeight,
                proposer_address_tm: block_response
                    .block
                    .header
                    .proposer_address
                    .to_string()
                    .to_lowercase(),
                proposer_address_namada: proposer_address_namada
                    .as_ref()
                    .map(Id::to_string),
                timestamp: block_response.block.header.time.unix_timestamp(),
                app_hash: Id::from(&block_response.block.header.app_hash),
            },
            transactions,
            epoch,
        }
    }

    pub fn inner_txs(&self) -> Vec<InnerTransaction> {
        self.transactions
            .iter()
            .flat_map(|(_, inner_txs)| inner_txs.clone())
            .collect()
    }

    pub fn wrapper_txs(&self) -> Vec<WrapperTransaction> {
        self.transactions
            .iter()
            .map(|(wrapper_tx, _)| wrapper_tx.clone())
            .collect()
    }

    pub fn sources(&self) -> HashSet<TransactionTarget> {
        self.inner_txs()
            .into_iter()
            .flat_map(|tx| match tx.kind {
                TransactionKind::TransparentTransfer(transfer)
                | TransactionKind::MixedTransfer(transfer)
                | TransactionKind::ShieldedTransfer(transfer)
                | TransactionKind::UnshieldingTransfer(transfer)
                | TransactionKind::ShieldingTransfer(transfer) => {
                    if let Some(data) = transfer {
                        let sources = data
                            .sources
                            .0
                            .keys()
                            .map(|account| {
                                TransactionTarget::sent(
                                    tx.tx_id.clone(),
                                    account.owner(),
                                )
                            })
                            .collect::<Vec<_>>();
                        let targets = data
                            .targets
                            .0
                            .keys()
                            .map(|account| {
                                TransactionTarget::received(
                                    tx.tx_id.clone(),
                                    account.owner(),
                                )
                            })
                            .collect::<Vec<_>>();
                        [sources, targets].concat()
                    } else {
                        vec![]
                    }
                }
                TransactionKind::IbcTrasparentTransfer((_, transfer))
                | TransactionKind::IbcShieldingTransfer((_, transfer))
                | TransactionKind::IbcUnshieldingTransfer((_, transfer)) => {
                    let sources = transfer
                        .sources
                        .0
                        .keys()
                        .map(|account| {
                            TransactionTarget::sent(
                                tx.tx_id.clone(),
                                account.owner(),
                            )
                        })
                        .collect::<Vec<_>>();
                    let targets = transfer
                        .targets
                        .0
                        .keys()
                        .map(|account| {
                            TransactionTarget::received(
                                tx.tx_id.clone(),
                                account.owner(),
                            )
                        })
                        .collect::<Vec<_>>();
                    [sources, targets].concat()
                }
                TransactionKind::Bond(bond) => {
                    if let Some(data) = bond {
                        let source =
                            data.source.unwrap_or(data.validator.clone());
                        vec![TransactionTarget::sent(
                            tx.tx_id.clone(),
                            source.to_string(),
                        )]
                    } else {
                        vec![]
                    }
                }
                TransactionKind::Redelegation(redelegation) => {
                    if let Some(data) = redelegation {
                        vec![TransactionTarget::sent(
                            tx.tx_id.clone(),
                            data.owner.to_string(),
                        )]
                    } else {
                        vec![]
                    }
                }
                TransactionKind::Unbond(unbond) => {
                    if let Some(data) = unbond {
                        let source =
                            data.source.unwrap_or(data.validator.clone());
                        vec![TransactionTarget::sent(
                            tx.tx_id.clone(),
                            source.to_string(),
                        )]
                    } else {
                        vec![]
                    }
                }
                TransactionKind::Withdraw(withdraw) => {
                    if let Some(data) = withdraw {
                        let source =
                            data.source.unwrap_or(data.validator.clone());
                        vec![TransactionTarget::sent(
                            tx.tx_id.clone(),
                            source.to_string(),
                        )]
                    } else {
                        vec![]
                    }
                }
                TransactionKind::ClaimRewards(claim_rewards) => {
                    if let Some(data) = claim_rewards {
                        let source =
                            data.source.unwrap_or(data.validator.clone());
                        vec![TransactionTarget::sent(
                            tx.tx_id.clone(),
                            source.to_string(),
                        )]
                    } else {
                        vec![]
                    }
                }
                TransactionKind::ProposalVote(vote_proposal_data) => {
                    if let Some(data) = vote_proposal_data {
                        vec![TransactionTarget::sent(
                            tx.tx_id,
                            data.voter.to_string(),
                        )]
                    } else {
                        vec![]
                    }
                }
                TransactionKind::InitProposal(init_proposal_data) => {
                    if let Some(data) = init_proposal_data {
                        vec![TransactionTarget::sent(
                            tx.tx_id,
                            data.author.to_string(),
                        )]
                    } else {
                        vec![]
                    }
                }
                TransactionKind::MetadataChange(meta_data_change) => {
                    if let Some(data) = meta_data_change {
                        vec![TransactionTarget::sent(
                            tx.tx_id,
                            data.validator.to_string(),
                        )]
                    } else {
                        vec![]
                    }
                }
                TransactionKind::CommissionChange(commission_change) => {
                    if let Some(data) = commission_change {
                        vec![TransactionTarget::sent(
                            tx.tx_id,
                            data.validator.to_string(),
                        )]
                    } else {
                        vec![]
                    }
                }
                TransactionKind::RevealPk(reveal_pk_data) => {
                    if let Some(data) = reveal_pk_data {
                        let source = Address::from(&data.public_key);
                        vec![TransactionTarget::sent(
                            tx.tx_id,
                            source.to_string(),
                        )]
                    } else {
                        vec![]
                    }
                }
                TransactionKind::BecomeValidator(become_validator) => {
                    if let Some(data) = become_validator {
                        vec![TransactionTarget::sent(
                            tx.tx_id,
                            data.address.to_string(),
                        )]
                    } else {
                        vec![]
                    }
                }
                TransactionKind::ReactivateValidator(address) => {
                    if let Some(data) = address {
                        vec![TransactionTarget::sent(
                            tx.tx_id,
                            data.to_string(),
                        )]
                    } else {
                        vec![]
                    }
                }
                TransactionKind::DeactivateValidator(address) => {
                    if let Some(data) = address {
                        vec![TransactionTarget::sent(
                            tx.tx_id,
                            data.to_string(),
                        )]
                    } else {
                        vec![]
                    }
                }
                TransactionKind::UnjailValidator(address) => {
                    if let Some(data) = address {
                        vec![TransactionTarget::sent(
                            tx.tx_id,
                            data.to_string(),
                        )]
                    } else {
                        vec![]
                    }
                }
                TransactionKind::ChangeConsensusKey(data) => {
                    if let Some(data) = data {
                        let change_consensus_key_data = data.clone();

                        vec![TransactionTarget::sent(
                            tx.tx_id,
                            change_consensus_key_data.validator.to_string(),
                        )]
                    } else {
                        vec![]
                    }
                }
                TransactionKind::IbcMsg(_)
                | TransactionKind::InitAccount(_)
                | TransactionKind::Unknown(_) => vec![],
            })
            .collect::<HashSet<_>>()
    }

    pub fn masp_entries(&self) -> Vec<MaspEntry> {
        self.transactions
            .iter()
            .fold(vec![], |mut acc, (wrapper_tx, inner_txs)| {
                // Extract successful inner txs
                for inner_tx in inner_txs {
                    if inner_tx.was_successful(wrapper_tx) {
                        acc.push(inner_tx)
                    }
                }

                acc
            })
            .iter()
            .flat_map(|tx| match &tx.kind {
                TransactionKind::ShieldingTransfer(Some(transfer_data)) => {
                    transfer_data
                        .targets
                        .0
                        .iter()
                        .map(|(account, amount)| MaspEntry {
                            token_address: account.token(),
                            timestamp: self.header.timestamp,
                            raw_amount: amount.amount().into(),
                            direction: MaspEntryDirection::In,
                            inner_tx_id: tx.tx_id.clone(),
                        })
                        .collect()
                }
                TransactionKind::UnshieldingTransfer(Some(transfer_data)) => {
                    transfer_data
                        .sources
                        .0
                        .iter()
                        .map(|(account, amount)| MaspEntry {
                            token_address: account.token(),
                            timestamp: self.header.timestamp,
                            raw_amount: amount.amount().into(),
                            direction: MaspEntryDirection::Out,
                            inner_tx_id: tx.tx_id.clone(),
                        })
                        .collect()
                }
                TransactionKind::MixedTransfer(Some(transfer_data)) => {
                    transfer_data
                        .sources
                        .0
                        .iter()
                        .map(|(source, denominated_amount)| {
                            (
                                source,
                                denominated_amount,
                                MaspEntryDirection::Out,
                            )
                        })
                        .chain(transfer_data.targets.0.iter().map(
                            |(target, denominated_amount)| {
                                (
                                    target,
                                    denominated_amount,
                                    MaspEntryDirection::In,
                                )
                            },
                        ))
                        .filter_map(|(transfer, denominated_amount, dir)| {
                            if transfer.owner() == MASP_ADDRESS.to_string() {
                                Some(MaspEntry {
                                    token_address: transfer.token(),
                                    timestamp: self.header.timestamp,
                                    raw_amount: denominated_amount
                                        .amount()
                                        .into(),
                                    direction: dir,
                                    inner_tx_id: tx.tx_id.clone(),
                                })
                            } else {
                                None
                            }
                        })
                        .collect()
                }
                TransactionKind::IbcShieldingTransfer((_, transfer_data)) => {
                    transfer_data
                        .targets
                        .0
                        .iter()
                        .map(|(account, amount)| MaspEntry {
                            token_address: account.token(),
                            timestamp: self.header.timestamp,
                            raw_amount: amount.amount().into(),
                            direction: MaspEntryDirection::In,
                            inner_tx_id: tx.tx_id.clone(),
                        })
                        .collect()
                }
                TransactionKind::IbcUnshieldingTransfer((_, transfer_data)) => {
                    transfer_data
                        .sources
                        .0
                        .iter()
                        .map(|(account, amount)| MaspEntry {
                            token_address: account.token(),
                            timestamp: self.header.timestamp,
                            raw_amount: amount.amount().into(),
                            direction: MaspEntryDirection::Out,
                            inner_tx_id: tx.tx_id.clone(),
                        })
                        .collect()
                } // we could improve this by looking at mixed transfers too
                _ => vec![],
            })
            .collect()
    }

    pub fn governance_proposal(
        &self,
        mut next_proposal_id: u64,
    ) -> Vec<GovernanceProposal> {
        self.transactions
            .iter()
            .fold(vec![], |mut acc, (wrapper_tx, inner_txs)| {
                // Extract successful inner txs
                for inner_tx in inner_txs {
                    if inner_tx.was_successful(wrapper_tx) {
                        acc.push(inner_tx)
                    }
                }

                acc
            })
            .iter()
            .filter_map(|tx| match &tx.kind {
                TransactionKind::InitProposal(Some(init_proposal_data)) => {
                    let proposal_content_bytes = tx
                        .get_section_data_by_id(Id::from(
                            init_proposal_data.content,
                        ))
                        .unwrap_or_default();

                    let proposal_content =
                        BTreeMap::<String, String>::try_from_slice(
                            &proposal_content_bytes,
                        )
                        .unwrap_or_default();

                    let proposal_content_serialized =
                        serde_json::to_string_pretty(&proposal_content)
                            .unwrap_or_default();

                    let proposal_data = match init_proposal_data.r#type.clone()
                    {
                        namada_governance::ProposalType::DefaultWithWasm(
                            hash,
                        ) => {
                            let wasm_code =
                                tx.get_section_data_by_id(Id::from(hash));
                            if let Some(wasm_code) = wasm_code {
                                let hex_encoded =
                                    String::from_utf8(hex::encode(wasm_code))
                                        .unwrap_or_default();
                                Some(hex_encoded)
                            } else {
                                None
                            }
                        }
                        namada_governance::ProposalType::PGFSteward(data) => {
                            Some(serde_json::to_string(&data).unwrap())
                        }
                        namada_governance::ProposalType::PGFPayment(data) => {
                            Some(serde_json::to_string(&data).unwrap())
                        }
                        namada_governance::ProposalType::Default => None,
                    };

                    let current_id = next_proposal_id;
                    next_proposal_id += 1;

                    Some(GovernanceProposal {
                        id: current_id,
                        author: Id::from(init_proposal_data.author.to_owned()),
                        r#type: GovernanceProposalKind::from(
                            init_proposal_data.r#type.to_owned(),
                        ),
                        data: proposal_data,
                        voting_start_epoch: Epoch::from(
                            init_proposal_data.voting_start_epoch.0 as u32,
                        ),
                        voting_end_epoch: Epoch::from(
                            init_proposal_data.voting_end_epoch.0 as u32,
                        ),
                        activation_epoch: Epoch::from(
                            init_proposal_data.activation_epoch.0 as u32,
                        ),
                        content: proposal_content_serialized,
                    })
                }
                _ => None,
            })
            .collect()
    }

    pub fn pos_rewards(&self) -> HashSet<(Id, Id)> {
        self.transactions
            .iter()
            .fold(vec![], |mut acc, (wrapper_tx, inner_txs)| {
                // Extract successful inner txs
                for inner_tx in inner_txs {
                    if inner_tx.was_successful(wrapper_tx) {
                        acc.push(inner_tx)
                    }
                }

                acc
            })
            .iter()
            .filter_map(|tx| match &tx.kind {
                TransactionKind::ClaimRewards(Some(data)) => {
                    let validator = data.validator.to_owned();
                    let source = data
                        .source
                        .to_owned()
                        .unwrap_or_else(|| validator.clone());

                    Some((Id::from(source), Id::from(validator)))
                }
                _ => None,
            })
            .collect()
    }

    pub fn governance_votes(&self) -> HashSet<GovernanceVote> {
        self.transactions
            .iter()
            .fold(vec![], |mut acc, (wrapper_tx, inner_txs)| {
                // Extract successful inner txs
                for inner_tx in inner_txs {
                    if inner_tx.was_successful(wrapper_tx) {
                        acc.push(inner_tx)
                    }
                }

                acc
            })
            .iter()
            .filter_map(|tx| match &tx.kind {
                TransactionKind::ProposalVote(Some(vote_proposal_data)) => {
                    Some(GovernanceVote {
                        proposal_id: vote_proposal_data.id,
                        vote: vote_proposal_data.vote.to_owned().into(),
                        address: Id::from(vote_proposal_data.voter.to_owned()),
                    })
                }
                _ => None,
            })
            .collect()
    }

    pub fn ibc_tokens(&self) -> HashSet<IbcToken> {
        self.transactions
            .iter()
            .fold(vec![], |mut acc, (wrapper_tx, inner_txs)| {
                // Extract successful inner txs
                for inner_tx in inner_txs {
                    if inner_tx.was_successful(wrapper_tx) {
                        acc.push(inner_tx)
                    }
                }

                acc
            })
            .iter()
            .filter_map(|tx| match &tx.kind {
                TransactionKind::IbcTrasparentTransfer((
                    Token::Ibc(ibc_token),
                    _,
                ))
                | TransactionKind::IbcShieldingTransfer((
                    Token::Ibc(ibc_token),
                    _,
                ))
                | TransactionKind::IbcUnshieldingTransfer((
                    Token::Ibc(ibc_token),
                    _,
                )) => Some(ibc_token.to_owned()),
                _ => None,
            })
            .collect()
    }

    // TODO: move this and process_inner_tx_for_balance to a separate module
    pub fn addresses_with_balance_change(
        &self,
        native_token: &Id,
    ) -> HashSet<BalanceChange> {
        self.transactions
            .iter()
            .flat_map(|(wrapper_tx, inners_txs)| {
                let mut balance_changes: Vec<BalanceChange> = inners_txs
                    .iter()
                    .filter_map(|tx| {
                        if tx.was_successful(wrapper_tx) {
                            self.process_inner_tx_for_balance(tx, native_token)
                        } else {
                            None
                        }
                    })
                    .flatten()
                    .collect();

                // Push the balance change of the gas payer
                balance_changes.push(BalanceChange::new(
                    wrapper_tx.fee.gas_payer.clone(),
                    Token::Native(wrapper_tx.fee.gas_token.clone()),
                ));
                // If the token is not the native one also push the balanche
                // change of the block proposer (the balance change for the
                // native token is pushed by default)
                if &wrapper_tx.fee.gas_token != native_token {
                    if let Some(block_proposer) =
                        &self.header.proposer_address_namada
                    {
                        balance_changes.push(BalanceChange::new(
                            Id::Account(block_proposer.to_owned()),
                            Token::Native(wrapper_tx.fee.gas_token.clone()),
                        ));
                    }
                }

                balance_changes
            })
            .collect()
    }

    fn process_inner_tx_for_balance(
        &self,
        tx: &InnerTransaction,
        native_token: &Id,
    ) -> Option<Vec<BalanceChange>> {
        let change = match &tx.kind {
            TransactionKind::IbcMsg(Some(msg)) => {
                let balance = ibc_ack_to_balance_info(
                    msg.0.clone(),
                    native_token.clone(),
                )
                // TODO: as this function does not return Result, we just ok()
                // it for now
                .ok()??;

                vec![balance]
            }
            TransactionKind::IbcMsg(None) => Default::default(),

            // Shielded transfers don't move any transparent balance
            TransactionKind::ShieldedTransfer(_) => Default::default(),
            TransactionKind::ShieldingTransfer(data)
            | TransactionKind::UnshieldingTransfer(data)
            | TransactionKind::MixedTransfer(data)
            | TransactionKind::TransparentTransfer(data) => {
                let data = data.as_ref()?;

                [&data.sources, &data.targets]
                    .iter()
                    .flat_map(|transfer_changes| {
                        transfer_changes.0.keys().map(|account| {
                            BalanceChange::new(
                                Id::Account(account.owner()),
                                Token::new(
                                    &account.token(),
                                    None,
                                    &native_token.to_string(),
                                ),
                            )
                        })
                    })
                    .collect()
            }
            TransactionKind::IbcTrasparentTransfer((token, data))
            | TransactionKind::IbcShieldingTransfer((token, data))
            | TransactionKind::IbcUnshieldingTransfer((token, data)) => {
                [&data.sources, &data.targets]
                    .iter()
                    .flat_map(|transfer_changes| {
                        transfer_changes.0.keys().map(|account| {
                            BalanceChange::new(
                                Id::Account(account.owner()),
                                token.to_owned(),
                            )
                        })
                    })
                    .collect()
            }
            TransactionKind::Bond(data) => {
                let data = data.as_ref()?;

                let bond_data = data.clone();
                let address = bond_data.source.unwrap_or(bond_data.validator);
                let source = Id::from(address);

                vec![BalanceChange::new(
                    source,
                    Token::Native(native_token.clone()),
                )]
            }
            TransactionKind::Withdraw(data) => {
                let data = data.as_ref()?;

                let withdraw_data = data.clone();
                let address =
                    withdraw_data.source.unwrap_or(withdraw_data.validator);
                let source = Id::from(address);

                vec![BalanceChange::new(
                    source,
                    Token::Native(native_token.clone()),
                )]
            }
            TransactionKind::ClaimRewards(data) => {
                let data = data.as_ref()?;

                let claim_rewards_data = data.clone();
                let address = claim_rewards_data
                    .source
                    .unwrap_or(claim_rewards_data.validator);
                let source = Id::from(address);

                vec![BalanceChange::new(
                    source,
                    Token::Native(native_token.clone()),
                )]
            }
            TransactionKind::InitProposal(data) => {
                let data = data.as_ref()?;

                let init_proposal_data = data.clone();
                let author = Id::from(init_proposal_data.author);

                vec![BalanceChange::new(
                    author,
                    Token::Native(native_token.clone()),
                )]
            }
            TransactionKind::Redelegation(_)
            | TransactionKind::ChangeConsensusKey(_)
            | TransactionKind::InitAccount(_)
            | TransactionKind::CommissionChange(_)
            | TransactionKind::RevealPk(_)
            | TransactionKind::DeactivateValidator(_)
            | TransactionKind::Unknown(_)
            | TransactionKind::UnjailValidator(_)
            | TransactionKind::MetadataChange(_)
            | TransactionKind::ReactivateValidator(_)
            | TransactionKind::Unbond(_)
            | TransactionKind::BecomeValidator(_)
            | TransactionKind::ProposalVote(_) => Default::default(),
        };

        Some(change)
    }

    pub fn ibc_msg_recv_packet(
        msg: IbcMessage<Transfer>,
    ) -> Option<MsgRecvPacket> {
        // Early return if the message is not an Envelope
        let IbcMessage::Envelope(e) = msg else {
            return None;
        };

        // Early return if the envelope is not a Packet
        let MsgEnvelope::Packet(packet_msg) = *e else {
            return None;
        };

        // Early return if it's not a Recv message
        let PacketMsg::Recv(recv_msg) = packet_msg else {
            return None;
        };

        Some(recv_msg)
    }

    pub fn new_validators(&self) -> HashSet<Validator> {
        self.transactions
            .iter()
            .fold(vec![], |mut acc, (wrapper_tx, inner_txs)| {
                // Extract successful inner txs
                for inner_tx in inner_txs {
                    if inner_tx.was_successful(wrapper_tx) {
                        acc.push(inner_tx)
                    }
                }

                acc
            })
            .iter()
            .filter_map(|tx| match &tx.kind {
                TransactionKind::BecomeValidator(Some(data)) => {
                    let data = data.to_owned();
                    Some(Validator {
                        address: Id::from(data.address),
                        voting_power: "0".to_string(),
                        max_commission: data
                            .max_commission_rate_change
                            .to_string(),
                        commission: data.commission_rate.to_string(),
                        name: data.name,
                        email: Some(data.email),
                        description: data.description,
                        website: data.website,
                        discord_handler: data.discord_handle,
                        avatar: data.avatar,
                        state: ValidatorState::Inactive,
                    })
                }
                _ => None,
            })
            .collect()
    }

    pub fn update_validators_state(&self) -> HashSet<ValidatorStateChange> {
        self.transactions
            .iter()
            .fold(vec![], |mut acc, (wrapper_tx, inner_txs)| {
                // Extract successful inner txs
                for inner_tx in inner_txs {
                    if inner_tx.was_successful(wrapper_tx) {
                        acc.push(inner_tx)
                    }
                }

                acc
            })
            .iter()
            .filter_map(|tx| match &tx.kind {
                TransactionKind::DeactivateValidator(Some(data)) => {
                    Some(ValidatorStateChange {
                        address: Id::from(data.to_owned()),
                        state: ValidatorState::Deactivating,
                    })
                }
                TransactionKind::ReactivateValidator(Some(data)) => {
                    Some(ValidatorStateChange {
                        address: Id::from(data.to_owned()),
                        state: ValidatorState::Reactivating,
                    })
                }
                _ => None,
            })
            .collect()
    }

    pub fn bond_addresses(&self) -> HashSet<BondAddresses> {
        self.transactions
            .iter()
            .fold(vec![], |mut acc, (wrapper_tx, inner_txs)| {
                // Extract successful inner txs
                for inner_tx in inner_txs {
                    if inner_tx.was_successful(wrapper_tx) {
                        acc.push(inner_tx)
                    }
                }

                acc
            })
            .iter()
            .filter_map(|tx| match &tx.kind {
                TransactionKind::Bond(Some(bond_data)) => {
                    let bond_data = bond_data.to_owned();
                    let source_address = bond_data
                        .source
                        .unwrap_or_else(|| bond_data.validator.clone())
                        .to_owned();
                    let target_address = bond_data.validator;

                    Some(vec![BondAddresses {
                        source: Id::from(source_address),
                        target: Id::from(target_address),
                    }])
                }
                TransactionKind::Unbond(Some(unbond_data)) => {
                    let unbond_data = unbond_data.to_owned();
                    let source_address = unbond_data
                        .source
                        .unwrap_or_else(|| unbond_data.validator.clone());
                    let validator_address = unbond_data.validator;

                    Some(vec![BondAddresses {
                        source: Id::from(source_address),
                        target: Id::from(validator_address),
                    }])
                }
                TransactionKind::Redelegation(Some(redelegation_data)) => {
                    let namada_tx::data::pos::Redelegation {
                        src_validator,
                        dest_validator,
                        owner,
                        amount: _,
                    } = redelegation_data.to_owned();

                    Some(vec![
                        BondAddresses {
                            source: Id::from(owner.clone()),
                            target: Id::from(src_validator),
                        },
                        BondAddresses {
                            source: Id::from(owner),
                            target: Id::from(dest_validator),
                        },
                    ])
                }
                _ => None,
            })
            .flatten()
            .collect()
    }

    pub fn unbond_addresses(&self) -> HashSet<UnbondAddresses> {
        self.transactions
            .iter()
            .fold(vec![], |mut acc, (wrapper_tx, inner_txs)| {
                // Extract successful inner txs
                for inner_tx in inner_txs {
                    if inner_tx.was_successful(wrapper_tx) {
                        acc.push(inner_tx)
                    }
                }

                acc
            })
            .iter()
            .filter_map(|tx| match &tx.kind {
                TransactionKind::Unbond(Some(data)) => {
                    let unbond_data = data.to_owned();

                    let source_address = unbond_data
                        .source
                        .unwrap_or_else(|| unbond_data.validator.clone());
                    let validator_address = unbond_data.validator;

                    Some(UnbondAddresses {
                        source: Id::from(source_address),
                        validator: Id::from(validator_address),
                    })
                }
                _ => None,
            })
            .collect()
    }

    pub fn withdraw_addresses(&self) -> HashSet<UnbondAddresses> {
        self.transactions
            .iter()
            .fold(vec![], |mut acc, (wrapper_tx, inner_txs)| {
                // Extract successful inner txs
                for inner_tx in inner_txs {
                    if inner_tx.was_successful(wrapper_tx) {
                        acc.push(inner_tx)
                    }
                }

                acc
            })
            .iter()
            .filter_map(|tx| match &tx.kind {
                TransactionKind::Withdraw(Some(data)) => {
                    let withdraw_data = data.to_owned();

                    let source_address = withdraw_data
                        .source
                        .unwrap_or_else(|| withdraw_data.validator.clone());
                    let validator_address = withdraw_data.validator;

                    Some(UnbondAddresses {
                        source: Id::from(source_address),
                        validator: Id::from(validator_address),
                    })
                }
                _ => None,
            })
            .collect()
    }

    pub fn validator_metadata(&self) -> Vec<ValidatorMetadataChange> {
        self.transactions
            .iter()
            .fold(vec![], |mut acc, (wrapper_tx, inner_txs)| {
                // Extract successful inner txs
                for inner_tx in inner_txs {
                    if inner_tx.was_successful(wrapper_tx) {
                        acc.push(inner_tx)
                    }
                }

                acc
            })
            .iter()
            .filter_map(|tx| match &tx.kind {
                TransactionKind::MetadataChange(Some(data)) => {
                    let namada_tx::data::pos::MetaDataChange {
                        validator,
                        email,
                        description,
                        website,
                        discord_handle,
                        avatar,
                        name,
                        commission_rate,
                    } = data.to_owned();

                    Some(ValidatorMetadataChange {
                        address: Id::from(validator),
                        commission: commission_rate.map(|c| c.to_string()),
                        name,
                        email,
                        description,
                        website,
                        discord_handler: discord_handle,
                        avatar,
                    })
                }
                TransactionKind::CommissionChange(data) => {
                    let commission_change = data.clone().unwrap();

                    let source_address = commission_change.validator;

                    Some(ValidatorMetadataChange {
                        address: Id::from(source_address),
                        commission: Some(
                            commission_change.new_rate.to_string(),
                        ),
                        name: None,
                        email: None,
                        description: None,
                        website: None,
                        discord_handler: None,
                        avatar: None,
                    })
                }
                _ => None,
            })
            .collect()
    }

    pub fn revealed_pks(&self) -> Vec<(PublicKey, Id)> {
        self.transactions
            .iter()
            .fold(vec![], |mut acc, (wrapper_tx, inner_txs)| {
                // Extract successful inner txs
                for inner_tx in inner_txs {
                    if inner_tx.was_successful(wrapper_tx) {
                        acc.push(inner_tx)
                    }
                }

                acc
            })
            .iter()
            .filter_map(|tx| match &tx.kind {
                TransactionKind::RevealPk(Some(reveal_pk_data)) => Some((
                    PublicKey::from(reveal_pk_data.public_key.to_owned()),
                    Id::from(reveal_pk_data.public_key.to_owned()),
                )),
                _ => None,
            })
            .collect()
    }
}
