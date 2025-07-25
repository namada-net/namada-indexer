use bigdecimal::{BigDecimal, ToPrimitive};
use shared::id::Id;
use shared::token::{IbcToken, Token};

use crate::appstate::AppState;
use crate::entity::chain::{CirculatingSupply, Parameters, TokenSupply};
use crate::error::chain::ChainError;
use crate::repository::balance::{BalanceRepo, BalanceRepoTrait};
use crate::repository::chain::{ChainRepository, ChainRepositoryTrait};

#[derive(Clone)]
pub struct ChainService {
    chain_repo: ChainRepository,
}

impl ChainService {
    pub fn new(app_state: AppState) -> Self {
        Self {
            chain_repo: ChainRepository::new(app_state),
        }
    }

    pub async fn find_latest_parameters(
        &self,
    ) -> Result<Parameters, ChainError> {
        let parameters = self
            .chain_repo
            .find_chain_parameters()
            .await
            .map(Parameters::from)
            .map_err(ChainError::Database)?;

        Ok(parameters)
    }

    pub async fn find_last_processed_block(&self) -> Result<i32, ChainError> {
        self.chain_repo
            .find_latest_height()
            .await
            .map_err(ChainError::Database)
    }

    pub async fn find_last_processed_epoch(&self) -> Result<i32, ChainError> {
        self.chain_repo
            .find_latest_epoch()
            .await
            .map_err(ChainError::Database)
    }

    pub async fn find_tokens(&self) -> Result<Vec<Token>, ChainError> {
        let tokens_db = self
            .chain_repo
            .find_tokens()
            .await
            .map_err(ChainError::Database)?;

        let tokens = tokens_db
            .into_iter()
            .map(|(token, ibc_token)| match ibc_token {
                Some(ibc_token) => Token::Ibc(IbcToken {
                    address: Id::Account(ibc_token.address),
                    trace: Some(Id::IbcTrace(ibc_token.ibc_trace)),
                }),
                None => Token::Native(Id::Account(token.address)),
            })
            .collect::<Vec<Token>>();

        Ok(tokens)
    }

    pub async fn get_token_supply(
        &self,
        address: String,
        epoch: Option<i32>,
    ) -> Result<Option<TokenSupply>, ChainError> {
        let maybe_token_supply_db = self
            .chain_repo
            .get_token_supply(address, epoch)
            .await
            .map_err(ChainError::Database)?;

        Ok(maybe_token_supply_db.map(|supply| TokenSupply {
            address: Id::Account(supply.address),
            total_supply: supply.total.to_u64().expect("Should fit in u64"),
            effective_supply: supply
                .effective
                .map(|s| s.to_u64().expect("Should fit in u64")),
        }))
    }

    pub async fn get_circulating_supply(
        &self,
        epoch: Option<i32>,
    ) -> Result<CirculatingSupply, ChainError> {
        // Native token address and addresses to exclude from circulating supply
        let native_token_address =
            "tnam1q9gr66cvu4hrzm0sd5kmlnjje82gs3xlfg3v6nu7";
        let excluded_addresses = vec![
            "tnam1qxdzup2hcvhswcgw5kerd5lfkf04t64y3scgqm5v",
            "tnam1qxt7uxhj9r00mfm4u870e7ghz6j20jrdz58gm5kj",
            "tnam1qyez9fd9nkaxfj4u2f2k0vavr8mm69azcgds45rr",
            "tnam1qqp69rzwsgnqdm0d4qfhw4qa4s6v3tlzm5069f4j",
            "tnam1qrucghh3hw2zq8xtqzdj44nh5nrmnkn0usqng8yq",
        ];

        // Get total supply of native token
        let total_supply_result = self
            .chain_repo
            .get_token_supply(native_token_address.to_string(), epoch)
            .await
            .map_err(ChainError::Database)?;

        let total_supply = total_supply_result.ok_or_else(|| {
            ChainError::Unknown("Native token supply not found".to_string())
        })?;

        // Get balances for all excluded addresses and sum them up
        let balance_repo = BalanceRepo::new(self.chain_repo.app_state.clone());
        let mut total_locked_amount = BigDecimal::from(0);

        for address in excluded_addresses {
            let balances = balance_repo
                .get_address_balances(address.to_string())
                .await
                .map_err(ChainError::Database)?;

            // Find the balance of the native token for this excluded address
            let locked_amount = balances
                .iter()
                .find(|balance| balance.token == native_token_address)
                .map(|balance| balance.raw_amount.clone())
                .unwrap_or_else(|| BigDecimal::from(0));

            total_locked_amount += locked_amount;
        }

        // Calculate circulating supply = effective supply - total locked amount
        let effective_supply_amount =
            total_supply.effective.ok_or_else(|| {
                ChainError::Unknown(
                    "Effective supply not found for native token".to_string(),
                )
            })?;
        let circulating_amount =
            &effective_supply_amount - &total_locked_amount;

        Ok(CirculatingSupply {
            circulating_supply: circulating_amount.to_string(),
        })
    }
}
