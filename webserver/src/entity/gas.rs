use orm::gas::GasPriceDb;
use orm::token::{IbcTokenDb, TokenDb};
use shared::id::Id;
use shared::token::{IbcToken, Token};

#[derive(Clone, Debug)]
pub struct GasPrice {
    pub token: Token,
    pub min_denom_amount: String,
}

impl GasPrice {
    pub fn from_db(
        gas_price_db: GasPriceDb,
        tokens: Vec<(TokenDb, Option<IbcTokenDb>)>,
    ) -> Self {
        let token = tokens
            .into_iter()
            .find_map(|(token, ibc_token)| {
                if gas_price_db.token == token.address {
                    match ibc_token {
                        Some(ibc_token) => Some(Token::Ibc(IbcToken {
                            address: Id::Account(ibc_token.address),
                            trace: Id::IbcTrace(ibc_token.ibc_trace),
                        })),
                        None => Some(Token::Native(Id::Account(token.address))),
                    }
                } else {
                    None
                }
            })
            .expect("Fee token should be known.");

        Self {
            token,
            min_denom_amount: gas_price_db.amount.to_string(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct GasEstimate {
    pub min: u64,
    pub max: u64,
    pub avg: u64,
    pub total_estimates: u64,
}
