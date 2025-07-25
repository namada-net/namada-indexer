use serde::{Deserialize, Serialize};

use super::chain::TokenResponse;
use crate::entity::transaction::{
    InnerTransaction, TransactionExitStatus, TransactionHistory,
    TransactionHistoryKind, TransactionKind, WrapperTransaction,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TransactionResultResponse {
    Applied,
    Rejected,
}

impl From<TransactionExitStatus> for TransactionResultResponse {
    fn from(value: TransactionExitStatus) -> Self {
        match value {
            TransactionExitStatus::Applied => {
                TransactionResultResponse::Applied
            }
            TransactionExitStatus::Rejected => {
                TransactionResultResponse::Rejected
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum TransactionKindResponse {
    TransparentTransfer,
    ShieldedTransfer,
    ShieldingTransfer,
    UnshieldingTransfer,
    MixedTransfer,
    Bond,
    Redelegation,
    Unbond,
    Withdraw,
    ClaimRewards,
    VoteProposal,
    InitProposal,
    ChangeMetadata,
    ChangeCommission,
    RevealPk,
    IbcMsgTransfer,
    IbcTransparentTransfer,
    IbcShieldingTransfer,
    IbcUnshieldingTransfer,
    BecomeValidator,
    DeactivateValidator,
    ReactivateValidator,
    UnjailValidator,
    InitAccount,
    ChangeConsensusKey,
    Unknown,
}

impl From<TransactionKind> for TransactionKindResponse {
    fn from(value: TransactionKind) -> Self {
        match value {
            TransactionKind::TransparentTransfer => Self::TransparentTransfer,
            TransactionKind::ShieldedTransfer => Self::ShieldedTransfer,
            TransactionKind::ShieldingTransfer => Self::ShieldingTransfer,
            TransactionKind::UnshieldingTransfer => Self::UnshieldingTransfer,
            TransactionKind::MixedTransfer => Self::MixedTransfer,
            TransactionKind::Bond => Self::Bond,
            TransactionKind::Redelegation => Self::Redelegation,
            TransactionKind::Unbond => Self::Unbond,
            TransactionKind::Withdraw => Self::Withdraw,
            TransactionKind::ClaimRewards => Self::ClaimRewards,
            TransactionKind::VoteProposal => Self::VoteProposal,
            TransactionKind::InitProposal => Self::InitProposal,
            TransactionKind::ChangeMetadata => Self::ChangeMetadata,
            TransactionKind::ChangeCommission => Self::ChangeCommission,
            TransactionKind::RevealPk => Self::RevealPk,
            TransactionKind::IbcMsgTransfer => Self::IbcMsgTransfer,
            TransactionKind::IbcTransparentTransfer => {
                Self::IbcTransparentTransfer
            }
            TransactionKind::IbcShieldingTransfer => Self::IbcShieldingTransfer,
            TransactionKind::IbcUnshieldingTransfer => {
                Self::IbcUnshieldingTransfer
            }
            TransactionKind::BecomeValidator => Self::BecomeValidator,
            TransactionKind::DeactivateValidator => Self::DeactivateValidator,
            TransactionKind::ReactivateValidator => Self::ReactivateValidator,
            TransactionKind::UnjailValidator => Self::UnjailValidator,
            TransactionKind::Unknown => Self::Unknown,
            TransactionKind::ChangeConsensusKey => Self::ChangeConsensusKey,
            TransactionKind::InitAccount => Self::InitAccount,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WrapperTransactionResponse {
    pub id: String,
    pub fee_payer: String,
    pub fee_token: TokenResponse,
    pub gas_limit: String,
    pub gas_used: Option<u64>,
    pub amount_per_gas_unit: Option<f64>,
    pub block_height: u64,
    pub inner_transactions: Vec<ShortInnerTransactionResponse>,
    pub exit_code: TransactionResultResponse,
    pub atomic: bool,
}

impl WrapperTransactionResponse {
    pub fn new(
        wrapper: WrapperTransaction,
        inners: Vec<InnerTransaction>,
    ) -> Self {
        Self {
            id: wrapper.id.to_string(),
            fee_payer: wrapper.fee_payer.to_string(),
            fee_token: wrapper.fee_token.into(),
            gas_limit: wrapper.gas_limit.to_string(),
            gas_used: wrapper.gas_used,
            amount_per_gas_unit: wrapper.amount_per_gas_unit,
            block_height: wrapper.block_height,
            inner_transactions: inners
                .into_iter()
                .map(|inner| ShortInnerTransactionResponse {
                    id: inner.id.to_string(),
                    kind: inner.kind.into(),
                    data: inner.data,
                    memo: inner.memo,
                    exit_code: TransactionResultResponse::from(inner.exit_code),
                })
                .collect(),
            exit_code: TransactionResultResponse::from(wrapper.exit_code),
            atomic: wrapper.atomic,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortInnerTransactionResponse {
    pub id: String,
    pub kind: TransactionKindResponse,
    pub data: Option<String>,
    pub memo: Option<String>,
    pub exit_code: TransactionResultResponse,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InnerTransactionResponse {
    pub id: String,
    pub wrapper_id: String,
    pub kind: TransactionKindResponse,
    pub data: Option<String>,
    pub memo: Option<String>,
    pub exit_code: TransactionResultResponse,
}

impl InnerTransactionResponse {
    pub fn new(inner: InnerTransaction) -> Self {
        Self {
            id: inner.id.to_string(),
            wrapper_id: inner.wrapper_id.to_string(),
            kind: TransactionKindResponse::from(inner.kind),
            data: inner.data,
            memo: inner.memo,
            exit_code: TransactionResultResponse::from(inner.exit_code),
        }
    }
}

impl From<InnerTransaction> for ShortInnerTransactionResponse {
    fn from(value: InnerTransaction) -> Self {
        ShortInnerTransactionResponse {
            id: value.id.to_string(),
            kind: value.kind.into(),
            data: value.data,
            memo: value.memo,
            exit_code: TransactionResultResponse::from(value.exit_code),
        }
    }
}

impl InnerTransactionResponse {
    pub fn to_short(&self) -> ShortInnerTransactionResponse {
        ShortInnerTransactionResponse {
            id: self.id.clone(),
            kind: self.kind.clone(),
            data: self.data.clone(),
            memo: self.memo.clone(),
            exit_code: self.exit_code.clone(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TrasactionHistoryKindResponse {
    Received,
    Sent,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionHistoryResponse {
    pub tx: InnerTransactionResponse,
    pub target: String,
    pub kind: TrasactionHistoryKindResponse,
    pub block_height: u64,
}

impl From<TransactionHistory> for TransactionHistoryResponse {
    fn from(value: TransactionHistory) -> Self {
        Self {
            tx: InnerTransactionResponse::new(value.tx),
            target: value.target.to_string(),
            kind: match value.kind {
                TransactionHistoryKind::Received => {
                    TrasactionHistoryKindResponse::Received
                }
                TransactionHistoryKind::Sent => {
                    TrasactionHistoryKindResponse::Sent
                }
            },
            block_height: value.block_height,
        }
    }
}
