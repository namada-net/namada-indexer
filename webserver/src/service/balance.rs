use shared::balance::Amount;
use shared::id::Id;
use shared::token::{IbcToken, Token};

use crate::appstate::AppState;
use crate::entity::balance::Balance;
use crate::error::balance::BalanceError;
use crate::repository::balance::{BalanceRepo, BalanceRepoTrait};

#[derive(Clone)]
pub struct BalanceService {
    pub balance_repo: BalanceRepo,
}

impl BalanceService {
    pub fn new(app_state: AppState) -> Self {
        Self {
            balance_repo: BalanceRepo::new(app_state),
        }
    }

    pub async fn get_address_balances(
        &self,
        address: String,
    ) -> Result<Vec<Balance>, BalanceError> {
        let balances = self
            .balance_repo
            .get_address_balances(address.clone())
            .await
            .map_err(BalanceError::Database)?;

        let tokens = self
            .balance_repo
            .get_all_token()
            .await
            .map_err(BalanceError::Database)?;

        let denominated_balances = tokens
            .into_iter()
            .map(|(token, ibc_token)| Balance {
                owner: Id::Account(address.clone()),
                token: match ibc_token {
                    Some(ibc_token) => Token::Ibc(IbcToken {
                        address: Id::Account(ibc_token.address),
                        trace: Id::IbcTrace(ibc_token.ibc_trace),
                    }),
                    None => Token::Native(Id::Account(token.address.clone())),
                },
                amount: balances
                    .iter()
                    .find(|&balance| balance.token.eq(&token.address))
                    .cloned()
                    .map(|balance| Amount::from(balance.raw_amount))
                    .unwrap_or_else(Amount::zero),
            })
            .collect();

        Ok(denominated_balances)
    }
}
