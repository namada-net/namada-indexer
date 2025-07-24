use bigdecimal::ToPrimitive;
use shared::id::Id;
use shared::token::{IbcToken, Token};

use crate::appstate::AppState;
use crate::entity::chain::{Parameters, TokenSupply};
use crate::error::chain::ChainError;
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
                    trace: Id::IbcTrace(ibc_token.ibc_trace),
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
}
