use orm::pgf::{PaymentKindDb, PaymentRecurrenceDb};
use serde::{Deserialize, Serialize};

use crate::entity::pgf::{PaymentKind, PaymentRecurrence, PgfPayment};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PaymentRecurrenceResponse {
    Retro,
    Continuous,
}

impl From<PaymentRecurrenceDb> for PaymentRecurrenceResponse {
    fn from(value: PaymentRecurrenceDb) -> Self {
        match value {
            PaymentRecurrenceDb::Continuous => Self::Continuous,
            PaymentRecurrenceDb::Retro => Self::Retro,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PaymentKindResponse {
    Native,
    Ibc,
}

impl From<PaymentKindDb> for PaymentKindResponse {
    fn from(value: PaymentKindDb) -> Self {
        match value {
            PaymentKindDb::Ibc => Self::Ibc,
            PaymentKindDb::Native => Self::Native,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PgfPaymentResponse {
    pub recurrence: PaymentRecurrenceResponse,
    pub proposal_id: u64,
    pub kind: PaymentKindResponse,
    pub receipient: String,
    pub amount: String,
}

impl From<PgfPayment> for PgfPaymentResponse {
    fn from(value: PgfPayment) -> Self {
        Self {
            recurrence: match value.recurrence {
                PaymentRecurrence::Continuous => {
                    PaymentRecurrenceResponse::Continuous
                }
                PaymentRecurrence::Retro => PaymentRecurrenceResponse::Retro,
            },
            proposal_id: value.proposal_id,
            kind: match value.kind {
                PaymentKind::Ibc => PaymentKindResponse::Ibc,
                PaymentKind::Native => PaymentKindResponse::Native,
            },
            receipient: value.receipient.to_string(),
            amount: value.amount.to_string(),
        }
    }
}
