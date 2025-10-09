use bigdecimal::BigDecimal;
use namada_sdk::ibc::core::channel::types::acknowledgement::AcknowledgementStatus;
use namada_sdk::ibc::core::channel::types::msgs::PacketMsg;
use namada_sdk::ibc::core::handler::types::msgs::MsgEnvelope;
use shared::block_result::{BlockResult, TxAttributesType};
use shared::gas::GasEstimation;
use shared::transaction::{
    IbcAck, IbcAckStatus, IbcSequence, IbcTokenAction, InnerTransaction,
    TransactionKind, WrapperTransaction, ibc_denom_received, ibc_denom_sent,
};

pub fn get_ibc_token_flows(
    block_results: &BlockResult,
) -> impl Iterator<Item = (IbcTokenAction, String, BigDecimal)> + use<'_> {
    block_results.end_events.iter().filter_map(|event| {
        let (action, original_packet, fungible_token_packet) =
            event.attributes.as_ref()?.as_fungible_token_packet()?;

        let denom = match &action {
            IbcTokenAction::Withdraw => {
                ibc_denom_sent(&fungible_token_packet.denom)
            }
            IbcTokenAction::Deposit => {
                let packet = original_packet?;

                ibc_denom_received(
                    &fungible_token_packet.denom,
                    &packet.source_port,
                    &packet.source_channel,
                    &packet.dest_port,
                    &packet.dest_channel,
                )
                .inspect_err(|err| {
                    tracing::debug!(?err, "Failed to parse received IBC denom");
                })
                .ok()?
            }
        };

        Some((action, denom, fungible_token_packet.amount.clone()))
    })
}

pub fn get_ibc_packets(
    block_results: &BlockResult,
    txs: &[(WrapperTransaction, Vec<InnerTransaction>)],
) -> Vec<IbcSequence> {
    let mut ibc_txs: Vec<_> = txs.iter().rev().fold(
        Default::default(),
        |mut acc, (wrapper_tx, inner_txs)| {
            // Extract successful ibc transactions from each batch
            for inner_tx in inner_txs {
                if inner_tx.is_ibc() && inner_tx.was_successful(wrapper_tx) {
                    acc.push(inner_tx.tx_id.to_owned())
                }
            }
            acc
        },
    );

    block_results
        .end_events
        .iter()
        .filter_map(|event| {
            if let Some(attributes) = &event.attributes {
                match attributes {
                    TxAttributesType::SendPacket(packet) => Some(IbcSequence {
                        sequence_number: packet.sequence.clone(),
                        source_port: packet.source_port.clone(),
                        dest_port: packet.dest_port.clone(),
                        source_channel: packet.source_channel.clone(),
                        dest_channel: packet.dest_channel.clone(),
                        timeout: packet.timeout_timestamp,
                        tx_id: ibc_txs
                            .pop()
                            .expect("Ibc ack should have a corresponding tx."),
                    }),
                    _ => None,
                }
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
}

pub fn get_ibc_ack_packet(inner_txs: &[InnerTransaction]) -> Vec<IbcAck> {
    inner_txs.iter().filter_map(|tx| match tx.kind.clone() {
        TransactionKind::IbcMsg(Some(ibc_message)) => match ibc_message.0 {
            namada_sdk::ibc::IbcMessage::Envelope(msg_envelope) => {
                match *msg_envelope {
                    MsgEnvelope::Packet(packet_msg) => match packet_msg {
                        PacketMsg::Recv(_) => None,
                        PacketMsg::Ack(msg) => {
                            let ack = match serde_json::from_slice::<
                                AcknowledgementStatus,
                            >(
                                msg.acknowledgement.as_bytes()
                            ) {
                                Ok(status) => IbcAck {
                                    sequence_number: msg.packet.seq_on_a.to_string(),
                                    source_port: msg.packet.port_id_on_a.to_string(),
                                    dest_port: msg.packet.port_id_on_b.to_string(),
                                    source_channel: msg.packet.chan_id_on_a.to_string(),
                                    dest_channel: msg.packet.chan_id_on_b.to_string(),
                                    status: match status {
                                        AcknowledgementStatus::Success(_) => IbcAckStatus::Success,
                                        AcknowledgementStatus::Error(_) => IbcAckStatus::Fail,
                                    },
                                },
                                Err(_) => IbcAck {
                                    sequence_number: msg.packet.seq_on_a.to_string(),
                                    source_port: msg.packet.port_id_on_a.to_string(),
                                    dest_port: msg.packet.port_id_on_b.to_string(),
                                    source_channel: msg.packet.chan_id_on_a.to_string(),
                                    dest_channel: msg.packet.chan_id_on_b.to_string(),
                                    status: IbcAckStatus::Unknown,
                                },
                            };
                            Some(ack)
                        }
                        PacketMsg::Timeout(msg) => Some(IbcAck {
                            sequence_number: msg.packet.seq_on_a.to_string(),
                            source_port: msg.packet.port_id_on_a.to_string(),
                            dest_port: msg.packet.port_id_on_b.to_string(),
                            source_channel: msg.packet.chan_id_on_a.to_string(),
                            dest_channel: msg.packet.chan_id_on_b.to_string(),
                            status: IbcAckStatus::Timeout,
                        }),
                        PacketMsg::TimeoutOnClose(msg) => Some(IbcAck {
                            sequence_number: msg.packet.seq_on_a.to_string(),
                            source_port: msg.packet.port_id_on_a.to_string(),
                            dest_port: msg.packet.port_id_on_b.to_string(),
                            source_channel: msg.packet.chan_id_on_a.to_string(),
                            dest_channel: msg.packet.chan_id_on_b.to_string(),
                            status: IbcAckStatus::Timeout,
                        }),
                    },
                    _ => None,
                }
            },
            _ => None
        },
        _ => None,
    }).collect()
}

pub fn get_gas_estimates(
    txs: &[(WrapperTransaction, Vec<InnerTransaction>)],
) -> Vec<GasEstimation> {
    txs.iter()
        .filter(|(wrapper_tx, inner_txs)| {
            inner_txs
                .iter()
                // We can only index gas if all the inner transactions of the
                // batch were successfully executed, otherwise we'd end up
                // inserting in the db a gas value which is not guaranteed to be
                // enough for such a batch
                .all(|inner_tx| inner_tx.was_successful(wrapper_tx))
        })
        .map(|(wrapper_tx, inner_txs)| {
            let mut gas_estimate = GasEstimation::new(wrapper_tx.tx_id.clone());
            gas_estimate.signatures = wrapper_tx.total_signatures;
            gas_estimate.size = wrapper_tx.size;

            inner_txs.iter().for_each(|tx| match tx.kind {
                TransactionKind::TransparentTransfer(_) => {
                    gas_estimate.increase_transparent_transfer();
                }
                TransactionKind::MixedTransfer(_) => {
                    let notes = tx.notes;
                    gas_estimate.increase_mixed_transfer(notes)
                }
                TransactionKind::IbcTrasparentTransfer(_) => {
                    gas_estimate.increase_ibc_transparent_transfer()
                }
                TransactionKind::Bond(_) => gas_estimate.increase_bond(),
                TransactionKind::Redelegation(_) => {
                    gas_estimate.increase_redelegation()
                }
                TransactionKind::Unbond(_) => gas_estimate.increase_unbond(),
                TransactionKind::Withdraw(_) => {
                    gas_estimate.increase_withdraw()
                }
                TransactionKind::ClaimRewards(_) => {
                    gas_estimate.increase_claim_rewards()
                }
                TransactionKind::ProposalVote(_) => {
                    gas_estimate.increase_vote()
                }
                TransactionKind::RevealPk(_) => {
                    gas_estimate.increase_reveal_pk()
                }
                TransactionKind::ShieldedTransfer(_) => {
                    let notes = tx.notes;
                    gas_estimate.increase_shielded_transfer(notes);
                }
                TransactionKind::ShieldingTransfer(_) => {
                    let notes = tx.notes;
                    gas_estimate.increase_shielding_transfer(notes)
                }
                TransactionKind::UnshieldingTransfer(_) => {
                    let notes = tx.notes;
                    gas_estimate.increase_unshielding_transfer(notes)
                }
                TransactionKind::IbcShieldingTransfer(_) => {
                    let notes = tx.notes;
                    gas_estimate.increase_ibc_shielding_transfer(notes)
                }
                TransactionKind::IbcUnshieldingTransfer(_) => {
                    let notes = tx.notes;
                    gas_estimate.increase_ibc_unshielding_transfer(notes)
                }
                TransactionKind::ChangeConsensusKey(_)
                | TransactionKind::IbcMsg(_)
                | TransactionKind::InitAccount(_)
                | TransactionKind::InitProposal(_)
                | TransactionKind::MetadataChange(_)
                | TransactionKind::CommissionChange(_)
                | TransactionKind::BecomeValidator(_)
                | TransactionKind::ReactivateValidator(_)
                | TransactionKind::DeactivateValidator(_)
                | TransactionKind::UnjailValidator(_)
                | TransactionKind::Unknown(_) => (),
            });
            gas_estimate
        })
        .collect()
}
