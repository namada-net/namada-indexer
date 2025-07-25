use serde::{Deserialize, Serialize};

use crate::entity::block::Block;
use crate::entity::transaction::WrapperTransaction;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockResponse {
    pub height: u64,
    pub hash: Option<String>,
    pub app_hash: Option<String>,
    pub timestamp: Option<String>,
    pub proposer: Option<String>,
    pub transactions: Vec<String>,
    pub parent_app_hash: Option<String>,
    pub parent_hash: Option<String>,
    pub epoch: Option<String>,
}

impl BlockResponse {
    pub fn from(
        block: Block,
        prev_block: Option<Block>,
        transactions: Vec<WrapperTransaction>,
    ) -> Self {
        Self {
            height: block.height,
            hash: block.hash.map(|hash| hash.to_string()),
            app_hash: block.app_hash.map(|app_hash| app_hash.to_string()),
            timestamp: block
                .timestamp
                .map(|t| t.and_utc().timestamp().to_string()),
            proposer: block.proposer.map(|proposer| proposer.to_string()),
            transactions: transactions
                .into_iter()
                .map(|wrapper| wrapper.id.to_string())
                .collect(),
            parent_app_hash: prev_block
                .clone()
                .map(|block| block.app_hash)
                .unwrap_or(None)
                .map(|hash| hash.to_string()),
            parent_hash: prev_block
                .map(|block| block.hash)
                .unwrap_or(None)
                .map(|hash| hash.to_string()),
            epoch: block.epoch.map(|e| e.to_string()),
        }
    }
}
