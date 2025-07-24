use diesel::{Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};
use shared::token::Token;

use crate::schema::{ibc_token, token};

#[derive(Debug, Clone, Serialize, Deserialize, diesel_derive_enum::DbEnum)]
#[ExistingTypePath = "crate::schema::sql_types::TokenType"]
pub enum TokenTypeDb {
    Native,
    Ibc,
}

#[derive(Debug, Clone, Queryable, Selectable, Insertable)]
#[diesel(table_name = token)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct TokenDb {
    pub address: String,
    pub token_type: TokenTypeDb,
}

pub type TokenInsertDb = TokenDb;

impl From<&Token> for TokenDb {
    fn from(token: &Token) -> Self {
        match token {
            Token::Native(token) => TokenDb {
                address: token.to_string(),
                token_type: TokenTypeDb::Native,
            },
            Token::Ibc(token) => TokenDb {
                address: token.address.to_string(),
                token_type: TokenTypeDb::Ibc,
            },
        }
    }
}

#[derive(Debug, Clone, Queryable, Selectable, Insertable)]
#[diesel(table_name = ibc_token)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct IbcTokenDb {
    pub address: String,
    pub ibc_trace: String,
}

pub type IbcTokenInsertDb = IbcTokenDb;

impl IbcTokenDb {
    pub fn from_token(token: &Token) -> Option<Self> {
        match token {
            Token::Ibc(token) => Some(IbcTokenDb {
                address: token.address.to_string(),
                ibc_trace: token.clone().trace.unwrap_or_default().to_string(),
            }),
            Token::Native(_) => None,
        }
    }
}
