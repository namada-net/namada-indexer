use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

use bigdecimal::BigDecimal;
use namada_core::masp::MaspTxId;
use namada_core::token::Amount as NamadaAmount;
use namada_events::extend::ReadFromEventAttributes;
use namada_ibc::IbcTxDataHash;
use namada_ibc::apps::transfer::types::packet::PacketData as Ics20PacketData;
use namada_tx::IndexedTx;
use namada_tx::data::TxResult;
use namada_tx::event::MaspTxRef;
use tendermint_rpc::endpoint::block_results::Response as TendermintBlockResultResponse;

use crate::balance::Amount;
use crate::id::Id;
use crate::transaction::{IbcTokenAction, TransactionExitStatus};

#[derive(Debug, Clone)]
pub enum IbcCorePacketKind {
    Send,
    Recv,
    Ack,
    Timeout,
}

#[derive(Debug, Clone)]
pub enum EventKind {
    Applied,
    IbcCore(IbcCorePacketKind),
    FungibleTokenPacket,
    MaspFeePayment,
    MaspTransfer,
    TxWasmName,
    Unknown,
}

impl From<&String> for EventKind {
    fn from(value: &String) -> Self {
        match value.as_str() {
            "tx/applied" => Self::Applied,
            "tx/tx-wasm-name" => Self::TxWasmName,
            "send_packet" => Self::IbcCore(IbcCorePacketKind::Send),
            "recv_packet" => Self::IbcCore(IbcCorePacketKind::Recv),
            "fungible_token_packet" => Self::FungibleTokenPacket,
            "masp/fee-payment" => Self::MaspFeePayment,
            "masp/transfer" => Self::MaspTransfer,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct BlockResult {
    pub height: u64,
    pub begin_events: Vec<Event>,
    pub end_events: Vec<Event>,
}

#[derive(Debug, Clone)]
pub struct Event {
    pub kind: EventKind,
    pub attributes: Option<TxAttributesType>,
}

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
pub struct BatchResults {
    pub batch_errors: BTreeMap<Id, BTreeMap<Id, String>>,
    pub batch_results: BTreeMap<Id, bool>,
}

impl From<TxResult<String>> for BatchResults {
    fn from(value: TxResult<String>) -> Self {
        Self {
            batch_results: value.iter().fold(
                BTreeMap::default(),
                |mut acc, (tx_hash, result)| {
                    let tx_id = Id::from(*tx_hash);
                    let result = if let Ok(result) = result {
                        result.is_accepted()
                    } else {
                        false
                    };
                    acc.insert(tx_id, result);
                    acc
                },
            ),
            batch_errors: value.iter().fold(
                BTreeMap::default(),
                |mut acc, (tx_hash, result)| {
                    let tx_id = Id::from(*tx_hash);
                    let result = if let Ok(result) = result {
                        result
                            .vps_result
                            .errors
                            .iter()
                            .map(|(address, error)| {
                                (Id::from(address.clone()), error.clone())
                            })
                            .collect()
                    } else {
                        BTreeMap::default()
                    };
                    acc.insert(tx_id, result);
                    acc
                },
            ),
        }
    }
}

impl BatchResults {
    fn is_successful(&self, tx_id: &Id) -> bool {
        self.batch_results.get(tx_id).is_some_and(|res| *res)
    }
}

#[derive(Clone)]
pub struct TxApplied {
    pub code: TxEventStatusCode,
    pub gas: u64,
    pub hash: Id,
    pub height: u64,
    pub batch: BatchResults,
    pub info: String,
}

impl fmt::Debug for TxApplied {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            code,
            gas,
            hash,
            height,
            batch,
            info,
        } = self;

        f.debug_struct("TxApplied")
            .field("code", code)
            .field("gas", gas)
            .field("hash", hash)
            .field("height", height)
            .field("batch", batch)
            .field("info", info)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub enum MaspRef {
    MaspSection(MaspTxId),
    IbcData(IbcTxDataHash),
}

#[derive(Debug, Clone)]
pub struct MaspTxData {
    indexed_tx: IndexedTx,
    data: MaspRef,
}

impl From<MaspTxRef> for MaspRef {
    fn from(value: MaspTxRef) -> Self {
        match value {
            MaspTxRef::MaspSection(masp_tx_id) => Self::MaspSection(masp_tx_id),
            MaspTxRef::IbcData(hash) => Self::IbcData(hash),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct IbcPacket {
    pub source_port: String,
    pub dest_port: String,
    pub source_channel: String,
    pub dest_channel: String,
    pub timeout_timestamp: u64,
    // TODO: use height domain type
    pub timeout_height: String,
    pub sequence: String,
    pub data: String,
}

#[derive(Debug, Clone, Default)]
pub struct FungibleTokenPacket {
    pub sender: String,
    pub receiver: String,
    pub denom: String,
    pub memo: String,
    pub amount: BigDecimal,
}

#[derive(Debug, Clone)]
pub enum TxAttributesType {
    TxApplied(TxApplied),
    SendPacket(IbcPacket),
    RecvPacket(IbcPacket),
    FungibleTokenPacket {
        is_ack: bool,
        success: bool,
        packet: FungibleTokenPacket,
    },
    MaspFeePayment(MaspTxData),
    MaspTransfer(MaspTxData),
    WasmName {
        name: String,
        inner_tx_hash: Id,
    },
}

impl TxAttributesType {
    pub fn deserialize(
        event_kind: &EventKind,
        attributes: &BTreeMap<String, String>,
    ) -> Option<Self> {
        match event_kind {
            EventKind::Unknown => None,
            EventKind::FungibleTokenPacket => {
                let (is_ack, success) =
                    if let Some(success) = attributes.get("success") {
                        (false, success == "\u{1}" || success == "true")
                    } else {
                        (
                            true,
                            attributes
                                .get("acknowledgement")
                                .map(|ack| ack.starts_with("result:"))
                                .unwrap_or_default(),
                        )
                    };

                let sender = attributes.get("sender")?.to_owned();
                let receiver = attributes.get("receiver")?.to_owned();
                let denom = attributes.get("denom")?.to_owned();
                let memo = attributes.get("memo")?.to_owned();
                let amount =
                    attributes.get("amount")?.parse::<BigDecimal>().ok()?;

                Some(TxAttributesType::FungibleTokenPacket {
                    is_ack,
                    success,
                    packet: FungibleTokenPacket {
                        sender,
                        receiver,
                        denom,
                        memo,
                        amount,
                    },
                })
            }
            EventKind::IbcCore(
                kind @ (IbcCorePacketKind::Ack | IbcCorePacketKind::Timeout),
            ) => {
                tracing::warn!(?kind, "Received unhandled IBC packet kind");
                None
            }
            EventKind::IbcCore(
                kind @ (IbcCorePacketKind::Send | IbcCorePacketKind::Recv),
            ) => {
                let source_port =
                    attributes.get("packet_src_port").unwrap().to_owned();
                let dest_port =
                    attributes.get("packet_dst_port").unwrap().to_owned();
                let source_channel =
                    attributes.get("packet_src_channel").unwrap().to_owned();
                let dest_channel =
                    attributes.get("packet_dst_channel").unwrap().to_owned();
                let sequence =
                    attributes.get("packet_sequence").unwrap().to_owned();
                let timeout_timestamp = attributes
                    .get("packet_timeout_timestamp")
                    .unwrap_or(&"0".to_string())
                    .parse::<u64>()
                    .unwrap_or_default()
                    .to_owned();
                let timeout_height =
                    attributes.get("packet_timeout_height").unwrap().to_owned();
                let data = attributes.get("packet_data").unwrap().to_owned();

                let constructor = match kind {
                    IbcCorePacketKind::Send => Self::SendPacket,
                    IbcCorePacketKind::Recv => Self::RecvPacket,
                    _ => unreachable!(),
                };

                Some(constructor(IbcPacket {
                    source_port,
                    dest_port,
                    source_channel,
                    dest_channel,
                    timeout_timestamp,
                    timeout_height,
                    sequence,
                    data,
                }))
            }
            EventKind::Applied => Some(Self::TxApplied(TxApplied {
                code: attributes
                    .get("code")
                    .map(|code| TxEventStatusCode::from(code.as_str()))
                    .unwrap()
                    .to_owned(),
                gas: attributes
                    .get("gas_used")
                    .map(|gas| u64::from_str(gas).unwrap())
                    .unwrap()
                    .to_owned(),
                hash: attributes
                    .get("hash")
                    .map(|hash| Id::Hash(hash.to_lowercase()))
                    .unwrap()
                    .to_owned(),
                height: attributes
                    .get("height")
                    .map(|height| u64::from_str(height).unwrap())
                    .unwrap()
                    .to_owned(),
                batch: attributes
                    .get("batch")
                    .map(|batch_result| {
                        let tx_result: TxResult<String> =
                            serde_json::from_str(batch_result).unwrap();
                        BatchResults::from(tx_result)
                    })
                    .unwrap_or_default(),
                info: attributes.get("info").unwrap().to_owned(),
            })),
            EventKind::MaspFeePayment => {
                let data = MaspTxRef::read_from_event_attributes(attributes)
                    .unwrap()
                    .into();
                let indexed_tx =
                    IndexedTx::read_from_event_attributes(attributes).unwrap();

                Some(Self::MaspFeePayment(MaspTxData { indexed_tx, data }))
            }
            EventKind::MaspTransfer => {
                let data = MaspTxRef::read_from_event_attributes(attributes)
                    .unwrap()
                    .into();
                let indexed_tx =
                    IndexedTx::read_from_event_attributes(attributes).unwrap();

                Some(Self::MaspTransfer(MaspTxData { indexed_tx, data }))
            }
            EventKind::TxWasmName => {
                let name = attributes.get("code-name").unwrap().to_string();
                let inner_tx_hash = Id::Hash(
                    attributes.get("inner-tx-hash").unwrap().to_string(),
                );

                Some(Self::WasmName {
                    name,
                    inner_tx_hash,
                })
            }
        }
    }

    pub fn as_fungible_token_packet(
        &self,
    ) -> Option<(
        IbcTokenAction,
        Option<&IbcPacket>,
        Cow<'_, FungibleTokenPacket>,
    )> {
        let (action, packet) = match self {
            Self::SendPacket(packet) => (IbcTokenAction::Withdraw, packet),
            Self::RecvPacket(packet) => (IbcTokenAction::Deposit, packet),
            Self::FungibleTokenPacket {
                is_ack: true,
                success: true,
                packet,
            } => {
                return Some((
                    IbcTokenAction::Withdraw,
                    None,
                    Cow::Borrowed(packet),
                ));
            }
            Self::FungibleTokenPacket {
                is_ack: false,
                success: true,
                packet,
            } => {
                return Some((
                    IbcTokenAction::Deposit,
                    None,
                    Cow::Borrowed(packet),
                ));
            }
            _ => return None,
        };

        let packet_data: Ics20PacketData =
            serde_json::from_str(&packet.data).ok()?;
        let ibc_amount: NamadaAmount =
            packet_data.token.amount.try_into().ok()?;

        let ics20_packet = FungibleTokenPacket {
            memo: packet_data.memo.to_string(),
            sender: packet_data.sender.to_string(),
            receiver: packet_data.receiver.to_string(),
            denom: packet_data.token.denom.to_string(),
            amount: Amount::from(ibc_amount).into(),
        };

        Some((action, Some(packet), Cow::Owned(ics20_packet)))
    }
}

impl From<TendermintBlockResultResponse> for BlockResult {
    fn from(value: TendermintBlockResultResponse) -> Self {
        let begin_events = value
            .begin_block_events
            .unwrap_or_default()
            .iter()
            .map(|event| {
                let kind = EventKind::from(&event.kind);
                let raw_attributes = event.attributes.iter().fold(
                    BTreeMap::default(),
                    |mut acc, attribute| {
                        acc.insert(
                            String::from(attribute.key_str().unwrap()),
                            String::from(attribute.value_str().unwrap()),
                        );
                        acc
                    },
                );
                let attributes =
                    TxAttributesType::deserialize(&kind, &raw_attributes);
                Event { kind, attributes }
            })
            .collect::<Vec<Event>>();
        let end_events = value
            .end_block_events
            .unwrap_or_default()
            .iter()
            .map(|event| {
                let kind = EventKind::from(&event.kind);
                let raw_attributes = event.attributes.iter().fold(
                    BTreeMap::default(),
                    |mut acc, attribute| {
                        acc.insert(
                            String::from(attribute.key_str().unwrap()),
                            String::from(attribute.value_str().unwrap()),
                        );
                        acc
                    },
                );
                let attributes =
                    TxAttributesType::deserialize(&kind, &raw_attributes);
                Event { kind, attributes }
            })
            .collect::<Vec<Event>>();
        Self {
            height: value.height.value(),
            begin_events,
            end_events,
        }
    }
}

impl From<&TendermintBlockResultResponse> for BlockResult {
    fn from(value: &TendermintBlockResultResponse) -> Self {
        BlockResult::from(value.clone())
    }
}

impl BlockResult {
    pub fn is_wrapper_tx_applied(&self, tx_hash: &Id) -> TransactionExitStatus {
        let exit_status = self
            .end_events
            .iter()
            .filter_map(|event| {
                if let Some(TxAttributesType::TxApplied(data)) =
                    &event.attributes
                {
                    Some(data.clone())
                } else {
                    None
                }
            })
            .find(|attributes| attributes.hash.eq(tx_hash))
            .map(|attributes| attributes.clone().code)
            .map(TransactionExitStatus::from);

        exit_status.unwrap_or(TransactionExitStatus::Rejected)
    }

    pub fn gas_used(&self, tx_hash: &Id) -> Option<String> {
        self.end_events
            .iter()
            .filter_map(|event| {
                if let Some(TxAttributesType::TxApplied(data)) =
                    &event.attributes
                {
                    Some(data.clone())
                } else {
                    None
                }
            })
            .find(|attributes| attributes.hash.eq(tx_hash))
            .map(|attributes| attributes.gas.to_string())
    }

    pub fn is_inner_tx_accepted(
        &self,
        wrapper_hash: &Id,
        inner_hash: &Id,
    ) -> TransactionExitStatus {
        let exit_status = self
            .end_events
            .iter()
            .filter_map(|event| {
                if let Some(TxAttributesType::TxApplied(data)) =
                    &event.attributes
                {
                    Some(data.clone())
                } else {
                    None
                }
            })
            .find(|attributes| attributes.hash.eq(wrapper_hash))
            .map(|attributes| attributes.batch.is_successful(inner_hash))
            .map(|successful| match successful {
                true => TransactionExitStatus::Applied,
                false => TransactionExitStatus::Rejected,
            });
        exit_status.unwrap_or(TransactionExitStatus::Rejected)
    }

    pub fn masp_ref(&self, indexed_tx: &IndexedTx) -> Option<(MaspRef, bool)> {
        self.end_events
            .iter()
            .find_map(|event| match event.kind {
                EventKind::MaspFeePayment => {
                    event.attributes.as_ref().map(|attr| match attr {
                        TxAttributesType::MaspFeePayment(data)
                            if &data.indexed_tx == indexed_tx =>
                        {
                            Some((data.data.to_owned(), true))
                        }
                        _ => None,
                    })
                }
                EventKind::MaspTransfer => {
                    event.attributes.as_ref().map(|attr| match attr {
                        TxAttributesType::MaspTransfer(data)
                            if &data.indexed_tx == indexed_tx =>
                        {
                            Some((data.data.to_owned(), false))
                        }
                        _ => None,
                    })
                }
                _ => None,
            })
            .flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestAttribute {
        key: String,
        value: String,
    }

    struct TestEvent {
        kind: String,
        attributes: Vec<TestAttribute>,
    }

    #[test]
    fn ibc_fungible_token_events() {
        let mut events: Vec<_> = example_events()
            .into_iter()
            .filter_map(|test_event| {
                let kind = EventKind::from(&test_event.kind);
                let attributes = test_event
                    .attributes
                    .into_iter()
                    .map(|TestAttribute { key, value }| (key, value))
                    .collect();

                let parsed_attributes =
                    TxAttributesType::deserialize(&kind, &attributes);

                parsed_attributes.as_ref()?;

                Some(Event {
                    kind,
                    attributes: parsed_attributes,
                })
            })
            .collect();

        assert_eq!(events.len(), 1);
        assert!(matches!(
            events.remove(0),
            Event {
                kind: EventKind::FungibleTokenPacket,
                attributes: Some(
                    TxAttributesType::FungibleTokenPacket {
                        is_ack: true,
                        success: true,
                        packet: FungibleTokenPacket {
                            sender,
                            receiver,
                            denom,
                            memo,
                            amount,
                        },
                    },
                ),
            }
            if
                sender == "osmo1m8wg4vxkefhs374qxmmqpyusgz289wmulex5qdwpfx7jnrxzer5s9cv83q"
                    && receiver == "mantra1drlcrf9drkhwayf37dephlxc0uqg5zcqjd7er8"
                    && denom == "transfer/channel-85077/uom"
                    && memo.is_empty()
                    && amount == "19582".parse().unwrap()
        ));
    }

    fn example_events() -> Vec<TestEvent> {
        vec![
            TestEvent {
                kind: "acknowledge_packet".to_owned(),
                attributes: vec![
                    TestAttribute {
                        key: "packet_timeout_height".to_owned(),
                        value: "0-0".to_owned(),
                    },
                    TestAttribute {
                        key: "packet_timeout_timestamp".to_owned(),
                        value: "1739790400335658800".to_owned(),
                    },
                    TestAttribute {
                        key: "packet_sequence".to_owned(),
                        value: "56446".to_owned(),
                    },
                    TestAttribute {
                        key: "packet_src_port".to_owned(),
                        value: "transfer".to_owned(),
                    },
                    TestAttribute {
                        key: "packet_src_channel".to_owned(),
                        value: "channel-85077".to_owned(),
                    },
                    TestAttribute {
                        key: "packet_dst_port".to_owned(),
                        value: "transfer".to_owned(),
                    },
                    TestAttribute {
                        key: "packet_dst_channel".to_owned(),
                        value: "channel-0".to_owned(),
                    },
                    TestAttribute {
                        key: "packet_channel_ordering".to_owned(),
                        value: "ORDER_UNORDERED".to_owned(),
                    },
                    TestAttribute {
                        key: "packet_connection".to_owned(),
                        value: "connection-2766".to_owned(),
                    },
                    TestAttribute {
                        key: "connection_id".to_owned(),
                        value: "connection-2766".to_owned(),
                    },
                    TestAttribute {
                        key: "msg_index".to_owned(),
                        value: "1".to_owned(),
                    },
                ],
            },
            TestEvent {
                kind: "fungible_token_packet".to_owned(),
                attributes: vec![
                    TestAttribute {
                        key: "module".to_owned(),
                        value: "transfer".to_owned(),
                    },
                    TestAttribute {
                        key: "sender".to_owned(),
                        value: "osmo1m8wg4vxkefhs374qxmmqpyusgz289wmulex5qdwpfx7jnrxzer5s9cv83q".to_owned(),
                    },
                    TestAttribute {
                        key: "receiver".to_owned(),
                        value: "mantra1drlcrf9drkhwayf37dephlxc0uqg5zcqjd7er8".to_owned(),
                    },
                    TestAttribute {
                        key: "denom".to_owned(),
                        value: "transfer/channel-85077/uom".to_owned(),
                    },
                    TestAttribute {
                        key: "amount".to_owned(),
                        value: "19582".to_owned(),
                    },
                    TestAttribute {
                        key: "memo".to_owned(),
                        value: "".to_owned(),
                    },
                    TestAttribute {
                        key: "acknowledgement".to_owned(),
                        value: "result:\"\\001\" ".to_owned(),
                    },
                    TestAttribute {
                        key: "msg_index".to_owned(),
                        value: "1".to_owned(),
                    },
                ],
            },
            TestEvent {
                kind: "fungible_token_packet".to_owned(),
                attributes: vec![
                    TestAttribute {
                        key: "success".to_owned(),
                        value: "\u{1}".to_owned(),
                    },
                    TestAttribute {
                        key: "msg_index".to_owned(),
                        value: "1".to_owned(),
                    },
                ],
            },
            TestEvent {
                kind: "ibc_transfer".to_owned(),
                attributes: vec![
                    TestAttribute {
                        key: "sender".to_owned(),
                        value: "noble1dljdlrhg8s9kj2yq2u90q4e6kdxll8njpywgzh".to_owned(),
                    },
                    TestAttribute {
                        key: "receiver".to_owned(),
                        value: "mantra1dljdlrhg8s9kj2yq2u90q4e6kdxll8njzv3yer".to_owned(),
                    },
                    TestAttribute {
                        key: "amount".to_owned(),
                        value: "4995952207".to_owned(),
                    },
                    TestAttribute {
                        key: "denom".to_owned(),
                        value: "uusdc".to_owned(),
                    },
                    TestAttribute {
                        key: "memo".to_owned(),
                        value: "".to_owned(),
                    },
                ],
            },
        ]
    }
}
