use std::collections::{BTreeMap, HashMap};
use std::str::FromStr;

use namada_core::address::Address;
use namada_core::masp::MaspTxId;
use namada_sdk::borsh::BorshSerializeExt;
use namada_sdk::ibc::IbcMessage as NamadaIbcMessage;
use namada_sdk::token::{
    Account as NamadaAccount, DenominatedAmount as NamadaDenominatedAmount,
    Transfer as NamadaTransfer,
};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize};
use subtle_encoding::hex;

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ChainAddress {
    /// On chain account
    ChainAccount(NamadaAccount),
    /// Address of the ibc token receiver
    IbcPfmAccount(String, Address),
    /// External address
    ExternalAccount(String, Address),
}

impl ChainAddress {
    pub fn owner(&self) -> String {
        match self {
            ChainAddress::ChainAccount(account) => account.owner.to_string(),
            ChainAddress::IbcPfmAccount(owner, _) => owner.clone(),
            ChainAddress::ExternalAccount(owner, _) => owner.clone(),
        }
    }

    pub fn token(&self) -> String {
        match self {
            ChainAddress::ChainAccount(account) => account.token.to_string(),
            ChainAddress::IbcPfmAccount(_, token) => token.clone().to_string(),
            ChainAddress::ExternalAccount(_, token) => {
                token.clone().to_string()
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct AccountsMap(pub BTreeMap<ChainAddress, NamadaDenominatedAmount>);

impl Serialize for AccountsMap {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_seq(self.0.iter().map(|(k, v)| match k {
            ChainAddress::ChainAccount(account) => HashMap::from([
                ("owner", account.owner.encode()),
                ("token", account.token.encode()),
                ("type", "onChain".to_string()),
                ("amount", v.amount().raw_amount().to_string()),
            ]),
            ChainAddress::IbcPfmAccount(account, token) => HashMap::from([
                ("owner", account.clone()),
                ("token", token.encode()),
                ("type", "pfm".to_string()),
                ("amount", v.amount().raw_amount().to_string()),
            ]),
            ChainAddress::ExternalAccount(account, token) => HashMap::from([
                ("owner", account.clone()),
                ("token", token.encode()),
                ("type", "external".to_string()),
                ("amount", v.amount().raw_amount().to_string()),
            ]),
        }))
    }
}

impl<'de> Deserialize<'de> for AccountsMap {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        <Vec<BTreeMap<String, String>> as Deserialize>::deserialize(
            deserializer,
        )
        .map(|v| {
            AccountsMap(
                v.into_iter()
                    .map(|val| {
                        let kind = val.get("type").expect("Cannot find type");

                        if kind.eq("pfm") {
                            let owner =
                                val.get("owner").expect("Cannot find owner");
                            let token =
                                val.get("token").expect("Cannot find token");
                            let amount =
                                val.get("amount").expect("Cannot find amount");

                            (
                                ChainAddress::IbcPfmAccount(
                                    owner.clone(),
                                    Address::decode(token).expect(
                                        "Cannot parse Address for token",
                                    ),
                                ),
                                NamadaDenominatedAmount::from_str(amount)
                                    .expect("Cannot parse DenominatedAmount"),
                            )
                        } else if kind.eq("onChain") {
                            let owner =
                                val.get("owner").expect("Cannot find owner");
                            let token =
                                val.get("token").expect("Cannot find token");
                            let amount =
                                val.get("amount").expect("Cannot find amount");

                            (
                                ChainAddress::ChainAccount(NamadaAccount {
                                    owner: Address::decode(owner).expect(
                                        "Cannot parse Address for owner",
                                    ),
                                    token: Address::decode(token).expect(
                                        "Cannot parse Address for token",
                                    ),
                                }),
                                NamadaDenominatedAmount::from_str(amount)
                                    .expect("Cannot parse DenominatedAmount"),
                            )
                        } else {
                            let owner =
                                val.get("owner").expect("Cannot find owner");
                            let token =
                                val.get("token").expect("Cannot find token");
                            let amount =
                                val.get("amount").expect("Cannot find amount");

                            (
                                ChainAddress::ExternalAccount(
                                    owner.clone(),
                                    Address::decode(token).expect(
                                        "Cannot parse Address for token",
                                    ),
                                ),
                                NamadaDenominatedAmount::from_str(amount)
                                    .expect("Cannot parse DenominatedAmount"),
                            )
                        }
                    })
                    .collect(),
            )
        })
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct TransferData {
    /// Sources of this transfer
    pub sources: AccountsMap,
    /// Targets of this transfer
    pub targets: AccountsMap,
    /// Hash of tx section that contains the MASP transaction
    pub shielded_section_hash: Option<MaspTxId>,
}

impl From<NamadaTransfer> for TransferData {
    fn from(transfer: NamadaTransfer) -> Self {
        let sources = AccountsMap(
            transfer
                .sources
                .iter()
                .map(|(account, denom)| {
                    (ChainAddress::ChainAccount(account.clone()), *denom)
                })
                .collect(),
        );
        let targets = AccountsMap(
            transfer
                .targets
                .iter()
                .map(|(account, denom)| {
                    (ChainAddress::ChainAccount(account.clone()), *denom)
                })
                .collect(),
        );
        let shielded_section_hash = transfer.shielded_section_hash;

        TransferData {
            sources,
            targets,
            shielded_section_hash,
        }
    }
}

#[derive(Debug, Clone)]
pub struct IbcMessage<Transfer>(pub NamadaIbcMessage<Transfer>);

impl Serialize for IbcMessage<NamadaTransfer> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match &self.0 {
            NamadaIbcMessage::Transfer(ibc_transfer) => {
                let mut state =
                    serializer.serialize_struct("IbcTransfer", 2)?;

                state.serialize_field("message", &ibc_transfer.message)?;
                state.serialize_field("transfer", &ibc_transfer.transfer)?;

                state.end()
            }
            NamadaIbcMessage::NftTransfer(ibc_nft_transfer) => {
                let mut state =
                    serializer.serialize_struct("IbcNftTransfer", 2)?;

                state.serialize_field(
                    "nft_message",
                    &ibc_nft_transfer.message,
                )?;
                state
                    .serialize_field("transfer", &ibc_nft_transfer.transfer)?;

                state.end()
            }
            NamadaIbcMessage::Envelope(data) => {
                let mut state =
                    serializer.serialize_struct("IbcEnvelope", 1)?;

                // todo: implement this bs :(

                state.serialize_field(
                    "data",
                    &String::from_utf8_lossy(&hex::encode(
                        data.serialize_to_vec(),
                    ))
                    .into_owned(),
                )?;

                state.end()
            }
        }
    }
}
