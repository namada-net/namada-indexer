use std::fmt::Display;

use bigdecimal::BigDecimal;
use serde::Serialize;

use crate::id::Id;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct IbcToken {
    pub address: Id,
    pub trace: Option<Id>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub enum Token {
    Ibc(IbcToken),
    Native(Id),
}

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Ibc(token) => write!(f, "{}", token.address),
            Token::Native(token) => write!(f, "{}", token),
        }
    }
}

impl Token {
    pub fn new(
        token: &str,
        ibc_trace: Option<String>,
        native_address: &str,
    ) -> Self {
        if !token.eq(native_address) {
            Token::Ibc(IbcToken {
                address: Id::Account(token.to_string()),
                trace: ibc_trace.map(Id::IbcTrace),
            })
        } else {
            Token::Native(Id::Account(token.to_string()))
        }
    }
}

#[derive(Debug)]
pub struct IbcRateLimit {
    /// Address of the token in Namada
    pub address: String,
    /// Epoch of the indexed rate limit
    pub epoch: u32,
    /// Throughput limit of token `address` at epoch `epoch`
    pub throughput_limit: BigDecimal,
}
