use shared::balance::Amount;
use shared::id::Id;

#[derive(Debug, Clone)]
pub enum PaymentRecurrence {
    Continuous,
    Retro,
}

#[derive(Debug, Clone)]

pub enum PaymentKind {
    Ibc,
    Native,
}

#[derive(Debug, Clone)]

pub enum PgfAction {
    Add,
    Remove,
}

#[derive(Debug, Clone)]
pub struct PgfPayment {
    pub proposal_id: u64,
    pub recurrence: PaymentRecurrence,
    pub kind: PaymentKind,
    pub receipient: Id,
    pub amount: Amount,
}
