use diesel::{Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};
use shared::transaction::{
    InnerTransaction, TransactionExitStatus, TransactionHistoryKind,
    TransactionKind, TransactionTarget, WrapperTransaction,
};

use crate::schema::{
    inner_transactions, transaction_history, wrapper_transactions,
};

#[derive(Debug, Clone, Serialize, Deserialize, diesel_derive_enum::DbEnum)]
#[ExistingTypePath = "crate::schema::sql_types::TransactionKind"]
pub enum TransactionKindDb {
    TransparentTransfer,
    ShieldedTransfer,
    ShieldingTransfer,
    UnshieldingTransfer,
    MixedTransfer,
    IbcMsgTransfer,
    IbcTransparentTransfer,
    IbcShieldingTransfer,
    IbcUnshieldingTransfer,
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
    BecomeValidator,
    ReactivateValidator,
    DeactivateValidator,
    UnjailValidator,
    ChangeConsensusKey,
    InitAccount,
    Unknown,
}

impl From<TransactionKind> for TransactionKindDb {
    fn from(value: TransactionKind) -> Self {
        match value {
            TransactionKind::TransparentTransfer(_) => {
                Self::TransparentTransfer
            }
            TransactionKind::ShieldedTransfer(_) => Self::ShieldedTransfer,
            TransactionKind::UnshieldingTransfer(_) => {
                Self::UnshieldingTransfer
            }
            TransactionKind::InitAccount(_) => Self::InitAccount,
            TransactionKind::ChangeConsensusKey(_) => Self::ChangeConsensusKey,
            TransactionKind::ShieldingTransfer(_) => Self::ShieldingTransfer,
            TransactionKind::MixedTransfer(_) => Self::MixedTransfer,
            TransactionKind::IbcMsg(_) => Self::IbcMsgTransfer,
            TransactionKind::IbcTrasparentTransfer(_) => {
                Self::IbcTransparentTransfer
            }
            TransactionKind::IbcShieldingTransfer(_) => {
                Self::IbcShieldingTransfer
            }
            TransactionKind::IbcUnshieldingTransfer(_) => {
                Self::IbcUnshieldingTransfer
            }
            TransactionKind::Bond(_) => Self::Bond,
            TransactionKind::Redelegation(_) => Self::Redelegation,
            TransactionKind::Unbond(_) => Self::Unbond,
            TransactionKind::Withdraw(_) => Self::Withdraw,
            TransactionKind::ClaimRewards(_) => Self::ClaimRewards,
            TransactionKind::ProposalVote(_) => Self::VoteProposal,
            TransactionKind::InitProposal(_) => Self::InitProposal,
            TransactionKind::MetadataChange(_) => Self::ChangeMetadata,
            TransactionKind::CommissionChange(_) => Self::ChangeCommission,
            TransactionKind::DeactivateValidator(_) => {
                Self::DeactivateValidator
            }
            TransactionKind::ReactivateValidator(_) => {
                Self::ReactivateValidator
            }
            TransactionKind::RevealPk(_) => Self::RevealPk,
            TransactionKind::BecomeValidator(_) => Self::BecomeValidator,
            TransactionKind::UnjailValidator(_) => Self::UnjailValidator,
            TransactionKind::Unknown(_) => TransactionKindDb::Unknown,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, diesel_derive_enum::DbEnum)]
#[ExistingTypePath = "crate::schema::sql_types::TransactionResult"]
pub enum TransactionResultDb {
    Applied,
    Rejected,
}

impl From<TransactionExitStatus> for TransactionResultDb {
    fn from(value: TransactionExitStatus) -> Self {
        match value {
            TransactionExitStatus::Applied => TransactionResultDb::Applied,
            TransactionExitStatus::Rejected => TransactionResultDb::Rejected,
        }
    }
}

#[derive(Serialize, Queryable, Selectable, Insertable, Clone)]
#[diesel(table_name = inner_transactions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct InnerTransactionInsertDb {
    pub id: String,
    pub wrapper_id: String,
    pub kind: TransactionKindDb,
    pub data: Option<String>,
    pub memo: Option<String>,
    pub exit_code: TransactionResultDb,
}

pub type InnerTransactionDb = InnerTransactionInsertDb;

impl InnerTransactionInsertDb {
    pub fn from(tx: InnerTransaction) -> Self {
        Self {
            id: tx.tx_id.to_string(),
            wrapper_id: tx.wrapper_id.to_string(),
            kind: TransactionKindDb::from(tx.kind),
            data: tx.data,
            memo: tx.memo,
            exit_code: TransactionResultDb::from(tx.exit_code),
        }
    }
}

#[derive(Serialize, Queryable, Selectable, Insertable, Clone)]
#[diesel(table_name = wrapper_transactions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct WrapperTransactionDb {
    pub id: String,
    pub fee_payer: String,
    pub fee_token: String,
    pub gas_limit: String,
    pub gas_used: Option<i32>,
    pub amount_per_gas_unit: Option<String>,
    pub masp_fee_payment: Option<String>,
    pub block_height: i32,
    pub exit_code: TransactionResultDb,
    pub atomic: bool,
}

pub type WrapperTransactionInsertDb = WrapperTransactionDb;

impl WrapperTransactionInsertDb {
    pub fn from(tx: WrapperTransaction) -> Self {
        Self {
            id: tx.tx_id.to_string(),
            fee_payer: tx.fee.gas_payer.to_string(),
            fee_token: tx.fee.gas_token.to_string(),
            gas_limit: tx.fee.gas,
            gas_used: tx.fee.gas_used.map(|gas| gas as i32),
            amount_per_gas_unit: Some(tx.fee.amount_per_gas_unit),
            masp_fee_payment: tx.fee.masp_fee_payment.map(|id| id.to_string()),
            block_height: tx.block_height as i32,
            exit_code: TransactionResultDb::from(tx.exit_code),
            atomic: tx.atomic,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, diesel_derive_enum::DbEnum)]
#[ExistingTypePath = "crate::schema::sql_types::HistoryKind"]
pub enum TransactionHistoryKindDb {
    Received,
    Sent,
}

impl From<TransactionHistoryKind> for TransactionHistoryKindDb {
    fn from(value: TransactionHistoryKind) -> Self {
        match value {
            TransactionHistoryKind::Received => Self::Received,
            TransactionHistoryKind::Sent => Self::Sent,
        }
    }
}

#[derive(Serialize, Queryable, Selectable, Clone)]
#[diesel(table_name = transaction_history)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct TransactionHistoryDb {
    pub id: i32,
    pub inner_tx_id: String,
    pub target: String,
    pub kind: TransactionHistoryKindDb,
}

#[derive(Serialize, Insertable, Clone)]
#[diesel(table_name = transaction_history)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct TransactionHistoryInsertDb {
    pub inner_tx_id: String,
    pub target: String,
    pub kind: TransactionHistoryKindDb,
}

impl TransactionHistoryInsertDb {
    pub fn from(target: TransactionTarget) -> Self {
        Self {
            inner_tx_id: target.inner_tx.to_string(),
            target: target.address,
            kind: TransactionHistoryKindDb::from(target.kind),
        }
    }
}
