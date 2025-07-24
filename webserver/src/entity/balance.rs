use shared::balance::Amount;
use shared::id::Id;
use shared::token::Token;

#[derive(Debug, Clone)]
pub struct Balance {
    pub owner: Id,
    pub token: Token,
    pub amount: Amount,
}
