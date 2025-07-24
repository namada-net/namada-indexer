use crate::appstate::AppState;
use crate::entity::block::Block;
use crate::entity::transaction::WrapperTransaction;
use crate::error::block::BlockError;
use crate::repository::block::{BlockRepository, BlockRepositoryTrait};
use crate::repository::chain::{ChainRepository, ChainRepositoryTrait};
use crate::repository::transaction::{
    TransactionRepository, TransactionRepositoryTrait,
};

#[derive(Clone)]
pub struct BlockService {
    block_repo: BlockRepository,
    transaction_repo: TransactionRepository,
    chain_repo: ChainRepository,
}

impl BlockService {
    pub fn new(app_state: AppState) -> Self {
        Self {
            block_repo: BlockRepository::new(app_state.clone()),
            transaction_repo: TransactionRepository::new(app_state.clone()),
            chain_repo: ChainRepository::new(app_state),
        }
    }

    pub async fn get_block_by_height(
        &self,
        height: i32,
    ) -> Result<(Block, Option<Block>, Vec<WrapperTransaction>), BlockError>
    {
        let block = self
            .block_repo
            .find_block_by_height(height)
            .await
            .map_err(BlockError::Database)?
            .map(Block::from_db);

        let block = block.ok_or(BlockError::NotFound(
            "height".to_string(),
            height.to_string(),
        ))?;

        let prev_block = if let Some(block_height) = height.checked_sub(1) {
            self.block_repo
                .find_block_by_height(block_height)
                .await
                .map_err(BlockError::Database)?
                .map(Block::from_db)
        } else {
            None
        };

        let tokens = self
            .chain_repo
            .find_tokens()
            .await
            .map_err(BlockError::Database)?;

        let transactions = self
            .transaction_repo
            .find_txs_by_block_height(height)
            .await
            .map_err(BlockError::Database)?
            .into_iter()
            .map(|transaction| {
                WrapperTransaction::from_db(transaction, tokens.clone())
            })
            .collect();

        Ok((block, prev_block, transactions))
    }

    pub async fn get_block_by_timestamp(
        &self,
        timestamp: i64,
    ) -> Result<(Block, Option<Block>, Vec<WrapperTransaction>), BlockError>
    {
        let block = self
            .block_repo
            .find_block_by_timestamp(timestamp)
            .await
            .map_err(BlockError::Database)?
            .map(Block::from_db);

        let block = block.ok_or(BlockError::NotFound(
            "timestamp".to_string(),
            timestamp.to_string(),
        ))?;

        let block_height = block.height as i32;

        let prev_block = if let Some(block_height) = block_height.checked_sub(1)
        {
            self.block_repo
                .find_block_by_height(block_height)
                .await
                .map_err(BlockError::Database)?
                .map(Block::from_db)
        } else {
            None
        };

        let tokens = self
            .chain_repo
            .find_tokens()
            .await
            .map_err(BlockError::Database)?;

        let transactions = self
            .transaction_repo
            .find_txs_by_block_height(block_height)
            .await
            .map_err(BlockError::Database)?
            .into_iter()
            .map(|transaction| {
                WrapperTransaction::from_db(transaction, tokens.clone())
            })
            .collect();

        Ok((block, prev_block, transactions))
    }
}
