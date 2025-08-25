use orm::transactions::{
    InnerTransactionDb, TransactionHistoryDb, TransactionHistoryKindDb,
    TransactionKindDb, TransactionResultDb, WrapperTransactionDb,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TransactionResult {
    Applied,
    Rejected,
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum TransactionKind {
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

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WrapperTransaction {
    pub tx_id: String,
    pub fee_payer: String,
    pub fee_token: String,
    pub gas_limit: u64,
    pub gas_used: Option<u64>,
    pub amount_per_gas_unit: Option<f64>,
    pub masp_fee_payment: Option<ShortInnerTransaction>,
    pub block_height: u64,
    pub inner_transactions: Vec<ShortInnerTransaction>,
    pub exit_code: TransactionResult,
    pub atomic: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortInnerTransaction {
    pub tx_id: String,
    pub kind: TransactionKind,
    pub data: Option<String>,
    pub memo: Option<String>,
    pub exit_code: TransactionResult,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InnerTransactionWithHeight {
    pub tx_id: String,
    pub block_height: u64,
    pub kind: TransactionKind,
    pub data: Option<String>,
    pub memo: Option<String>,
    pub exit_code: TransactionResult,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InnerTransaction {
    pub tx_id: String,
    pub wrapper_id: String,
    pub kind: TransactionKind,
    pub data: Option<String>,
    pub memo: Option<String>,
    pub exit_code: TransactionResult,
}

impl InnerTransaction {
    pub fn to_short(&self) -> ShortInnerTransaction {
        ShortInnerTransaction {
            tx_id: self.tx_id.clone(),
            kind: self.kind.clone(),
            data: self.data.clone(),
            memo: self.memo.clone(),
            exit_code: self.exit_code.clone(),
        }
    }
}

impl From<TransactionResultDb> for TransactionResult {
    fn from(value: TransactionResultDb) -> Self {
        match value {
            TransactionResultDb::Applied => TransactionResult::Applied,
            TransactionResultDb::Rejected => TransactionResult::Rejected,
        }
    }
}

impl From<TransactionKindDb> for TransactionKind {
    fn from(value: TransactionKindDb) -> Self {
        match value {
            TransactionKindDb::TransparentTransfer => Self::TransparentTransfer,
            TransactionKindDb::ShieldedTransfer => Self::ShieldedTransfer,
            TransactionKindDb::ShieldingTransfer => Self::ShieldingTransfer,
            TransactionKindDb::UnshieldingTransfer => Self::UnshieldingTransfer,
            TransactionKindDb::MixedTransfer => Self::MixedTransfer,
            TransactionKindDb::Bond => Self::Bond,
            TransactionKindDb::Redelegation => Self::Redelegation,
            TransactionKindDb::Unbond => Self::Unbond,
            TransactionKindDb::Withdraw => Self::Withdraw,
            TransactionKindDb::ClaimRewards => Self::ClaimRewards,
            TransactionKindDb::VoteProposal => Self::VoteProposal,
            TransactionKindDb::InitProposal => Self::InitProposal,
            TransactionKindDb::ChangeMetadata => Self::ChangeMetadata,
            TransactionKindDb::ChangeCommission => Self::ChangeCommission,
            TransactionKindDb::RevealPk => Self::RevealPk,
            TransactionKindDb::Unknown => Self::Unknown,
            TransactionKindDb::IbcMsgTransfer => Self::IbcMsgTransfer,
            TransactionKindDb::ChangeConsensusKey => Self::ChangeConsensusKey,
            TransactionKindDb::IbcTransparentTransfer => {
                Self::IbcTransparentTransfer
            }
            TransactionKindDb::IbcShieldingTransfer => {
                Self::IbcShieldingTransfer
            }
            TransactionKindDb::InitAccount => Self::InitAccount,
            TransactionKindDb::IbcUnshieldingTransfer => {
                Self::IbcUnshieldingTransfer
            }
            TransactionKindDb::BecomeValidator => Self::BecomeValidator,
            TransactionKindDb::ReactivateValidator => Self::ReactivateValidator,
            TransactionKindDb::DeactivateValidator => Self::DeactivateValidator,
            TransactionKindDb::UnjailValidator => Self::UnjailValidator,
        }
    }
}

impl From<TransactionKind> for TransactionKindDb {
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
            TransactionKind::IbcTransparentTransfer => Self::IbcTransparentTransfer,
            TransactionKind::IbcShieldingTransfer => Self::IbcShieldingTransfer,
            TransactionKind::IbcUnshieldingTransfer => Self::IbcUnshieldingTransfer,
            TransactionKind::BecomeValidator => Self::BecomeValidator,
            TransactionKind::DeactivateValidator => Self::DeactivateValidator,
            TransactionKind::ReactivateValidator => Self::ReactivateValidator,
            TransactionKind::UnjailValidator => Self::UnjailValidator,
            TransactionKind::InitAccount => Self::InitAccount,
            TransactionKind::ChangeConsensusKey => Self::ChangeConsensusKey,
            TransactionKind::Unknown => Self::Unknown,
        }
    }
}

impl From<WrapperTransactionDb> for WrapperTransaction {
    fn from(value: WrapperTransactionDb) -> Self {
        Self {
            tx_id: value.id,
            fee_payer: value.fee_payer,
            fee_token: value.fee_token,
            gas_limit: value.gas_limit.parse::<u64>().unwrap_or(0),
            gas_used: value.gas_used.map(|gas| gas as u64),
            amount_per_gas_unit: value
                .amount_per_gas_unit
                .map(|gas| gas.parse::<f64>().ok())
                .unwrap_or(None),
            masp_fee_payment: None,
            block_height: value.block_height as u64,
            inner_transactions: vec![],
            exit_code: TransactionResult::from(value.exit_code),
            atomic: value.atomic,
        }
    }
}

impl From<InnerTransactionDb> for InnerTransaction {
    fn from(value: InnerTransactionDb) -> Self {
        Self {
            tx_id: value.id,
            wrapper_id: value.wrapper_id,
            kind: TransactionKind::from(value.kind),
            data: value.data,
            memo: value.memo,
            exit_code: TransactionResult::from(value.exit_code),
        }
    }
}

impl From<(InnerTransactionDb, i32)> for InnerTransactionWithHeight {
    fn from((inner_tx, block_height): (InnerTransactionDb, i32)) -> Self {
        Self {
            tx_id: inner_tx.id,
            block_height: block_height as u64,
            kind: TransactionKind::from(inner_tx.kind),
            data: inner_tx.data,
            memo: inner_tx.memo,
            exit_code: TransactionResult::from(inner_tx.exit_code),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TrasactionHistoryKind {
    Received,
    Sent,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionHistory {
    pub tx: InnerTransaction,
    pub target: String,
    pub kind: TrasactionHistoryKind,
    pub block_height: i32,
}

impl TransactionHistory {
    pub fn from(
        transaction_history_db: TransactionHistoryDb,
        inner_tx_db: InnerTransactionDb,
        block_height: i32,
    ) -> Self {
        Self {
            tx: InnerTransaction::from(inner_tx_db),
            target: transaction_history_db.target,
            kind: match transaction_history_db.kind {
                TransactionHistoryKindDb::Received => {
                    TrasactionHistoryKind::Received
                }
                TransactionHistoryKindDb::Sent => TrasactionHistoryKind::Sent,
            },
            block_height,
        }
    }
}
