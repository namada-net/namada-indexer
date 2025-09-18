use crate::appstate::AppState;
use crate::entity::transaction::{
    InnerTransaction, TransactionHistory, TransactionKind, WrapperTransaction,
};
use crate::error::transaction::TransactionError;
use crate::repository::chain::{ChainRepository, ChainRepositoryTrait};
use crate::repository::transaction::{
    TransactionRepository, TransactionRepositoryTrait,
};

#[derive(Clone)]
pub struct TransactionService {
    pub transaction_repo: TransactionRepository,
    pub chain_repo: ChainRepository,
}

impl TransactionService {
    pub fn new(app_state: AppState) -> Self {
        Self {
            transaction_repo: TransactionRepository::new(app_state.clone()),
            chain_repo: ChainRepository::new(app_state),
        }
    }

    pub async fn get_wrapper_tx(
        &self,
        id: String,
    ) -> Result<Option<WrapperTransaction>, TransactionError> {
        let tokens = self
            .chain_repo
            .find_tokens()
            .await
            .map_err(TransactionError::Database)?;

        let wrapper_tx = self
            .transaction_repo
            .find_wrapper_tx(id)
            .await
            .map_err(TransactionError::Database)?;

        Ok(wrapper_tx
            .map(|wrapper| WrapperTransaction::from_db(wrapper, tokens)))
    }

    pub async fn get_inner_tx(
        &self,
        id: String,
    ) -> Result<Option<InnerTransaction>, TransactionError> {
        let inner_tx = self
            .transaction_repo
            .find_inner_tx(id)
            .await
            .map_err(TransactionError::Database)?;

        Ok(inner_tx.map(InnerTransaction::from))
    }

    pub async fn get_inner_tx_by_wrapper_id(
        &self,
        id: String,
    ) -> Result<Vec<InnerTransaction>, TransactionError> {
        let inner_txs = self
            .transaction_repo
            .find_inners_by_wrapper_tx(id)
            .await
            .map_err(TransactionError::Database)?;

        Ok(inner_txs.into_iter().map(InnerTransaction::from).collect())
    }

    pub async fn get_addresses_history(
        &self,
        addresses: Vec<String>,
        page: u64,
    ) -> Result<(Vec<TransactionHistory>, u64, u64), TransactionError> {
        let (txs, total_pages, total_items) = self
            .transaction_repo
            .find_addresses_history(addresses, page as i64)
            .await
            .map_err(TransactionError::Database)?;

        Ok((
            txs.into_iter()
                .map(|(h, t, bh)| TransactionHistory::from_db(h, t, bh))
                .collect(),
            total_pages as u64,
            total_items as u64,
        ))
    }

    pub async fn get_most_recent_transactions(
        &self,
        offset: u64,
        size: u64,
    ) -> Result<Vec<WrapperTransaction>, TransactionError> {
        let tokens = self
            .chain_repo
            .find_tokens()
            .await
            .map_err(TransactionError::Database)?;

        let txs = self
            .transaction_repo
            .find_most_recent_transactions(offset as i64, size as i32)
            .await
            .map_err(TransactionError::Database)?;

        Ok(txs
            .into_iter()
            .map(|w| WrapperTransaction::from_db(w, tokens.clone()))
            .collect())
    }

    pub async fn get_filtered_most_recent_wrappers(
        &self,
        offset: u64,
        size: u64,
        kinds: Vec<TransactionKind>,
        tokens: Vec<String>,
    ) -> Result<Vec<WrapperTransaction>, TransactionError> {
        let tokens_map = self
            .chain_repo
            .find_tokens()
            .await
            .map_err(TransactionError::Database)?;

        let txs = self
            .transaction_repo
            .find_recent_matching_wrappers(
                offset as i64,
                size as i32,
                kinds,
                tokens,
            )
            .await
            .map_err(TransactionError::Database)?;

        Ok(txs
            .into_iter()
            .map(|w| WrapperTransaction::from_db(w, tokens_map.clone()))
            .collect())
    }
}
