use anyhow::Context;
use namada_ibc::apps::nft_transfer::types::PORT_ID_STR as NFT_PORT_ID_STR;
use namada_ibc::apps::transfer::types::packet::PacketData as FtPacketData;
use namada_ibc::apps::transfer::types::{
    Amount as IbcAmount, PORT_ID_STR as FT_PORT_ID_STR, PrefixedDenom,
    TracePrefix,
};
use namada_ibc::core::channel::types::acknowledgement::AcknowledgementStatus;
use namada_ibc::core::channel::types::msgs::PacketMsg;
use namada_ibc::core::channel::types::packet::Packet;
use namada_ibc::core::handler::types::msgs::MsgEnvelope;
use namada_ibc::core::host::types::identifiers::{ChannelId, PortId};
use namada_sdk::address::Address;
use namada_sdk::token::Transfer;

use crate::id::Id;
use crate::ser::{self, ChainAddress, TransferData};
use crate::token::Token;
use crate::transaction::TransactionKind;

pub(crate) const MASP_ADDRESS: Address =
    Address::Internal(namada_sdk::address::InternalAddress::Masp);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BalanceChange {
    pub address: Id,
    pub token: Token,
}

impl BalanceChange {
    pub fn new(address: Id, token: Token) -> Self {
        Self { address, token }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct GovernanceProposalShort {
    pub id: u64,
    pub voting_start_epoch: u64,
    pub voting_end_epoch: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct DelegationPair {
    pub validator_address: Id,
    pub delegator_address: Id,
}

pub fn transfer_to_tx_kind(data: Transfer) -> TransactionKind {
    let has_shielded_section = data.shielded_section_hash.is_some();
    if has_shielded_section
        && data.sources.is_empty()
        && data.targets.is_empty()
    {
        // For fully shielded transaction we don't explicitly write the masp
        // address in the sources nor targets
        return TransactionKind::ShieldedTransfer(Some(data.into()));
    }

    let (all_sources_are_masp, any_sources_are_masp) = data
        .sources
        .iter()
        .fold((true, false), |(all, any), (acc, _)| {
            let is_masp = acc.owner.eq(&MASP_ADDRESS);
            (all && is_masp, any || is_masp)
        });

    let (all_targets_are_masp, any_targets_are_masp) = data
        .targets
        .iter()
        .fold((true, false), |(all, any), (acc, _)| {
            let is_masp = acc.owner.eq(&MASP_ADDRESS);
            (all && is_masp, any || is_masp)
        });

    match (
        all_sources_are_masp,
        any_sources_are_masp,
        all_targets_are_masp,
        any_targets_are_masp,
        has_shielded_section,
    ) {
        (true, _, true, _, true) => {
            TransactionKind::ShieldedTransfer(Some(data.into()))
        }
        (true, _, _, false, true) => {
            TransactionKind::UnshieldingTransfer(Some(data.into()))
        }
        (_, false, true, _, true) => {
            TransactionKind::ShieldingTransfer(Some(data.into()))
        }
        (_, false, _, false, false) => {
            TransactionKind::TransparentTransfer(Some(data.into()))
        }
        _ => TransactionKind::MixedTransfer(Some(data.into())),
    }
}

fn packet_msg_to_balance_info(
    native_token: Id,
    packet_msg: PacketMsg,
) -> anyhow::Result<Option<BalanceChange>> {
    let extract = |packet: Packet| -> anyhow::Result<BalanceChange> {
        let packet_data = serde_json::from_slice::<FtPacketData>(&packet.data)
            .context("Could not deserialize IBC fungible token packet")?;

        let maybe_ibc_trace = get_namada_ibc_trace_when_sending(
            &packet_data.token.denom,
            &packet.port_id_on_a,
            &packet.chan_id_on_a,
        );
        let (_, token) = get_ibc_token(
            maybe_ibc_trace,
            Address::from(native_token),
            &packet_data.token.denom,
        )?;

        let source = Id::Account(packet_data.sender.to_string());

        Ok(BalanceChange::new(source, token))
    };

    match packet_msg {
        PacketMsg::Ack(msg) => {
            let ack = serde_json::from_slice::<AcknowledgementStatus>(
                msg.acknowledgement.as_bytes(),
            )
            .context("Could not deserialize IBC acknowledgement")?;

            match ack {
                AcknowledgementStatus::Success(_) => Ok(None),
                AcknowledgementStatus::Error(_) => {
                    extract(msg.packet).map(Some)
                }
            }
        }
        PacketMsg::Timeout(msg) => extract(msg.packet).map(Some),
        PacketMsg::TimeoutOnClose(msg) => extract(msg.packet).map(Some),
        _ => Ok(None),
    }
}

pub fn ibc_ack_to_balance_info(
    ibc_data: namada_ibc::IbcMessage<Transfer>,
    native_token: Id,
) -> anyhow::Result<Option<BalanceChange>> {
    let namada_ibc::IbcMessage::Envelope(msg_envelope) = ibc_data else {
        return Ok(None);
    };
    let MsgEnvelope::Packet(packet_msg) = *msg_envelope else {
        return Ok(None);
    };

    packet_msg_to_balance_info(native_token, packet_msg)
}

pub fn transfer_to_ibc_tx_kind(
    ibc_data: namada_ibc::IbcMessage<Transfer>,
    native_token: Address,
) -> anyhow::Result<TransactionKind> {
    match &ibc_data {
        namada_ibc::IbcMessage::Envelope(msg_envelope) => {
            if let MsgEnvelope::Packet(
                namada_ibc::core::channel::types::msgs::PacketMsg::Recv(msg),
            ) = msg_envelope.as_ref()
            {
                // Extract transfer info from the packet
                let (transfer_data, token_id) =
                    match msg.packet.port_id_on_b.as_str() {
                        FT_PORT_ID_STR => {
                            let packet_data =
                                serde_json::from_slice::<FtPacketData>(
                                    &msg.packet.data,
                                )
                                .expect(
                                    "Could not deserialize IBC fungible token \
                                     packet",
                                );

                            let maybe_ibc_trace =
                                get_namada_ibc_trace_when_receiving(
                                    &packet_data.token.denom,
                                    &msg.packet.port_id_on_a,
                                    &msg.packet.chan_id_on_a,
                                    &msg.packet.port_id_on_b,
                                    &msg.packet.chan_id_on_b,
                                );

                            let (token, token_id, denominated_amount) =
                                get_token_and_amount(
                                    maybe_ibc_trace,
                                    packet_data.token.amount,
                                    native_token,
                                    &packet_data.token.denom,
                                )?;

                            (
                                TransferData {
                                    sources: crate::ser::AccountsMap(
                                        [(
                                            ChainAddress::ExternalAccount(
                                                packet_data.sender.to_string(),
                                                token.clone(),
                                            ),
                                            denominated_amount,
                                        )]
                                        .into(),
                                    ),
                                    targets: crate::ser::AccountsMap(
                                        [(
                                            convert_account(
                                                &packet_data,
                                                token.clone(),
                                                false,
                                            )
                                            .expect(
                                                "Should be able to convert \
                                                 receiver",
                                            ),
                                            denominated_amount,
                                        )]
                                        .into(),
                                    ),
                                    shielded_section_hash: None,
                                },
                                token_id,
                            )
                        }
                        NFT_PORT_ID_STR => {
                            // TODO: add support for indexing nfts
                            todo!(
                                "IBC NFTs are not yet supported for indexing \
                                 purposes"
                            )
                        }
                        _ => {
                            tracing::warn!("Found unsupported IBC packet data");
                            return Ok(TransactionKind::IbcMsg(Some(
                                ser::IbcMessage(ibc_data),
                            )));
                        }
                    };

                let is_shielding =
                    namada_sdk::ibc::extract_masp_tx_from_envelope(
                        msg_envelope,
                    )
                    .is_some();
                if is_shielding {
                    Ok(TransactionKind::IbcShieldingTransfer((
                        token_id,
                        transfer_data,
                    )))
                } else {
                    Ok(TransactionKind::IbcTrasparentTransfer((
                        token_id,
                        transfer_data,
                    )))
                }
            } else {
                Ok(TransactionKind::IbcMsg(Some(ser::IbcMessage(ibc_data))))
            }
        }
        namada_ibc::IbcMessage::Transfer(transfer) => {
            let maybe_ibc_trace = get_namada_ibc_trace_when_sending(
                &transfer.message.packet_data.token.denom,
                &transfer.message.port_id_on_a,
                &transfer.message.chan_id_on_a,
            );

            let (token, token_id, denominated_amount) = get_token_and_amount(
                maybe_ibc_trace,
                transfer.message.packet_data.token.amount,
                native_token,
                &transfer.message.packet_data.token.denom,
            )?;

            let transfer_data = TransferData {
                sources: crate::ser::AccountsMap(
                    [(
                        convert_account(
                            &transfer.message.packet_data,
                            token.clone(),
                            true,
                        )
                        .expect("Should be able to convert sender"),
                        denominated_amount,
                    )]
                    .into(),
                ),
                targets: crate::ser::AccountsMap(
                    [(
                        ChainAddress::ExternalAccount(
                            transfer.message.packet_data.receiver.to_string(),
                            token,
                        ),
                        denominated_amount,
                    )]
                    .into(),
                ),
                shielded_section_hash: transfer
                    .transfer
                    .clone()
                    .map(|t| t.shielded_section_hash)
                    .unwrap_or_default(),
            };

            if transfer.transfer.is_some() {
                Ok(TransactionKind::IbcUnshieldingTransfer((
                    token_id,
                    transfer_data,
                )))
            } else {
                Ok(TransactionKind::IbcTrasparentTransfer((
                    token_id,
                    transfer_data,
                )))
            }
        }
        namada_ibc::IbcMessage::NftTransfer(_nft_transfer) => {
            // TODO: add support for indexing nfts
            todo!("IBC NFTs are not yet supported for indexing purposes")
        }
    }
}

fn convert_account(
    packet_data: &FtPacketData,
    token: Address,
    is_sender: bool,
) -> Result<ChainAddress, String> {
    let is_pfm = {
        use serde::Deserialize;

        #[derive(Deserialize)]
        struct PfmMemo {
            #[allow(dead_code)]
            forward: serde_json::Value,
        }

        serde_json::from_str::<PfmMemo>(packet_data.memo.as_ref()).is_ok()
    };

    let address = if is_sender {
        &packet_data.sender
    } else {
        &packet_data.receiver
    };

    Ok(if is_pfm {
        ChainAddress::IbcPfmAccount(address.to_string(), token)
    } else if !address.as_ref().starts_with("tnam1") {
        ChainAddress::ExternalAccount(address.to_string(), token)
    } else {
        ChainAddress::ChainAccount(namada_sdk::token::Account {
            owner: Address::decode(address).map_err(|err| {
                format!(
                    "Ibc {} address is not valid: {err}",
                    if is_sender { "sender" } else { "receiver" }
                )
            })?,
            token,
        })
    })
}

fn get_namada_ibc_trace_when_receiving(
    // NB: we dub the sender `chain A`
    sender_denom: &PrefixedDenom,
    sender_port: &PortId,
    sender_channel: &ChannelId,
    // NB: we dub the receiver `chain B` (i.e. Namada)
    receiver_port: &PortId,
    receiver_channel: &ChannelId,
) -> Option<String> {
    let prefix = TracePrefix::new(sender_port.clone(), sender_channel.clone());

    if !sender_denom.trace_path.starts_with(&prefix) {
        // NOTE: this is a token native to chain A
        Some(format!("{receiver_port}/{receiver_channel}/{sender_denom}"))
    } else {
        // NOTE: this token is not native to chain A. it
        // could be NAM, but also some other token from
        // any other chain that is neither Namada
        // (i.e. chain B) nor chain A.

        let mut denom = sender_denom.clone();

        denom.trace_path.remove_prefix(&prefix);

        if denom.trace_path.is_empty() {
            // NOTE: this token is native to Namada.
            // WE ARE ASSUMING WE HAVE NAM. this could
            // be a mistake, if in the future we enable
            // the ethereum bridge, or mint other kinds
            // of tokens other than NAM.
            return None;
        }

        Some(if sender_denom.trace_path.starts_with(&prefix) {
            denom.to_string()
        } else {
            format!("{receiver_port}/{receiver_channel}/{sender_denom}")
        })
    }
}

fn get_namada_ibc_trace_when_sending(
    // NB: we dub the sender `chain A` (i.e. Namada)
    sender_denom: &PrefixedDenom,
    _sender_port: &PortId,
    _sender_channel: &ChannelId,
) -> Option<String> {
    Some(sender_denom.to_string())
}

fn get_ibc_token(
    maybe_ibc_trace: Option<String>,
    native_token: Address,
    original_denom: &PrefixedDenom,
) -> anyhow::Result<(Address, crate::token::Token)> {
    if let Some(ibc_trace) = maybe_ibc_trace {
        let is_native = original_denom
            .to_string()
            .contains(&native_token.to_string());

        let token_address =
            namada_ibc::trace::convert_to_address(ibc_trace.clone())
                .expect("Failed to convert IBC trace to address");

        if is_native {
            Ok((
                native_token.clone(),
                crate::token::Token::Native(native_token.into()),
            ))
        } else {
            Ok((
                token_address.clone(),
                crate::token::Token::Ibc(crate::token::IbcToken {
                    address: token_address.into(),
                    trace: Some(Id::IbcTrace(ibc_trace)),
                }),
            ))
        }
    } else if !original_denom
        .to_string()
        .contains(&native_token.to_string())
    {
        let namada_address =
            Address::decode(original_denom.base_denom.as_str())
                .context("Failed to decode base denom")?;

        Ok((
            namada_address.clone(),
            crate::token::Token::Ibc(crate::token::IbcToken {
                address: namada_address.into(),
                trace: None,
            }),
        ))
    } else {
        Ok((
            native_token.clone(),
            crate::token::Token::Native(native_token.into()),
        ))
    }
}

fn get_ibc_amount(
    amount: IbcAmount,
    is_ibc_token: bool,
) -> namada_sdk::token::DenominatedAmount {
    let converted_amount = amount
        .try_into()
        .expect("Failed conversion of IBC amount to Namada one");

    if is_ibc_token {
        namada_sdk::token::DenominatedAmount::new(converted_amount, 0.into())
    } else {
        namada_sdk::token::DenominatedAmount::native(converted_amount)
    }
}

fn get_token_and_amount(
    maybe_ibc_trace: Option<String>,
    amount: IbcAmount,
    native_token: Address,
    original_denom: &PrefixedDenom,
) -> anyhow::Result<(
    Address,
    crate::token::Token,
    namada_sdk::token::DenominatedAmount,
)> {
    let (address, token) =
        get_ibc_token(maybe_ibc_trace.clone(), native_token, original_denom)?;
    let is_ibc_token = maybe_ibc_trace.is_some();
    let denominated_amount = get_ibc_amount(amount, is_ibc_token);

    Ok((address, token, denominated_amount))
}

pub fn is_valid_bech32_address(address: &str, prefix: &str) -> bool {
    match bech32::decode(address) {
        Ok((hrp, _)) => hrp.as_str() == prefix,
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {

    use namada_sdk::address::InternalAddress;

    use super::*;

    #[test]
    fn is_valid_bech32_address() {
        let address = "tnam1qq6xmw8crfrprhlmajrq5c28p4dqy0nyjvtvupk5";
        let res = super::is_valid_bech32_address(address, "tnam");
        assert!(res, "Address {address} should be valid");
    }

    fn cmp_print(x: &str, y: &str) -> bool {
        if x == y {
            true
        } else {
            println!("{x} != {y}");
            false
        }
    }

    #[test]
    fn received_ibc_trace_parsing() {
        // native on cosmos, foreign on namada
        let maybe_ibc_trace = get_namada_ibc_trace_when_receiving(
            // sender side
            &"uatom".parse().unwrap(),
            &PortId::transfer(),
            &"channel-1234".parse().unwrap(),
            // receiver side
            &PortId::transfer(),
            &"channel-0".parse().unwrap(),
        );
        assert!(matches!(
            maybe_ibc_trace,
            Some(trace) if cmp_print(&trace, "transfer/channel-0/uatom"),
        ));

        // foreign on cosmos, native on namada
        let maybe_ibc_trace = get_namada_ibc_trace_when_receiving(
            // sender side
            &"transfer/channel-1234/\
              tnam1q9gr66cvu4hrzm0sd5kmlnjje82gs3xlfg3v6nu7"
                .parse()
                .unwrap(),
            &PortId::transfer(),
            &"channel-1234".parse().unwrap(),
            // receiver side
            &PortId::transfer(),
            &"channel-0".parse().unwrap(),
        );
        assert!(maybe_ibc_trace.is_none());

        // foreign on cosmos, foreign on namada
        let maybe_ibc_trace = get_namada_ibc_trace_when_receiving(
            // sender side
            &"transfer/channel-4321/transfer/channel-5/\
              tnam1q9gr66cvu4hrzm0sd5kmlnjje82gs3xlfg3v6nu7"
                .parse()
                .unwrap(),
            &PortId::transfer(),
            &"channel-1234".parse().unwrap(),
            // receiver side
            &PortId::transfer(),
            &"channel-0".parse().unwrap(),
        );
        assert!(matches!(
            maybe_ibc_trace,
            Some(trace) if cmp_print(
                &trace,
                "transfer/channel-0/\
                    transfer/channel-4321/\
                    transfer/channel-5/\
                    tnam1q9gr66cvu4hrzm0sd5kmlnjje82gs3xlfg3v6nu7"
            ),
        ));
        let maybe_ibc_trace = get_namada_ibc_trace_when_receiving(
            // sender side
            &"transfer/channel-1317/transfer/channel-1/uosmo"
                .parse()
                .unwrap(),
            &PortId::transfer(),
            &"channel-1317".parse().unwrap(),
            // receiver side
            &PortId::transfer(),
            &"channel-2".parse().unwrap(),
        );
        assert!(matches!(
            maybe_ibc_trace,
            Some(trace) if cmp_print(
                &trace,
                "transfer/channel-1/uosmo"
            ),
        ));
    }

    #[test]
    fn sent_ibc_trace_parsing() {
        // native on namada, foreign on cosmos
        let maybe_ibc_trace = get_namada_ibc_trace_when_sending(
            &"tnam1q9gr66cvu4hrzm0sd5kmlnjje82gs3xlfg3v6nu7"
                .parse()
                .unwrap(),
            &PortId::transfer(),
            &"channel-0".parse().unwrap(),
        );
        assert!(matches!(
            maybe_ibc_trace,
            Some(trace) if cmp_print(&trace, "tnam1q9gr66cvu4hrzm0sd5kmlnjje82gs3xlfg3v6nu7"),
        ));

        // foreign on namada, native on cosmos
        let maybe_ibc_trace = get_namada_ibc_trace_when_sending(
            &"transfer/channel-0/uatom".parse().unwrap(),
            &PortId::transfer(),
            &"channel-0".parse().unwrap(),
        );
        assert!(matches!(
            maybe_ibc_trace,
            Some(trace) if cmp_print(&trace, "transfer/channel-0/uatom"),
        ));

        // foreign on namada, foreign on cosmos
        let maybe_ibc_trace = get_namada_ibc_trace_when_sending(
            &"transfer/channel-0/transfer/channel-4321/transfer/channel-5/\
              tnam1q9gr66cvu4hrzm0sd5kmlnjje82gs3xlfg3v6nu7"
                .parse()
                .unwrap(),
            &PortId::transfer(),
            &"channel-0".parse().unwrap(),
        );
        assert!(matches!(
            maybe_ibc_trace,
            Some(trace) if cmp_print(
                &trace,
                "transfer/channel-0/\
                    transfer/channel-4321/\
                    transfer/channel-5/\
                    tnam1q9gr66cvu4hrzm0sd5kmlnjje82gs3xlfg3v6nu7"
            ),
        ));
        let maybe_ibc_trace = get_namada_ibc_trace_when_sending(
            &"transfer/channel-1/uosmo".parse().unwrap(),
            &PortId::transfer(),
            &"channel-2".parse().unwrap(),
        );
        assert!(matches!(
            maybe_ibc_trace,
            Some(trace) if cmp_print(&trace, "transfer/channel-1/uosmo"),
        ));
    }

    #[test]
    fn test_get_ibc_token() {
        let maybe_ibc_trace = get_ibc_token(
            None,
            Address::decode("tnam1q9gr66cvu4hrzm0sd5kmlnjje82gs3xlfg3v6nu7")
                .unwrap(),
            &"transer/channel-1/tnam1p5z8ruwyu7ha8urhq2l0dhpk2f5dv3ts7uyf2n75"
                .parse()
                .unwrap(),
        )
        .unwrap();

        assert!(matches!(
            maybe_ibc_trace,
            (Address::Internal(InternalAddress::IbcToken(_)), Token::Ibc(a)) if a.trace.is_none()
        ));
    }
}
