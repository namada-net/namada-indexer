use orm::blocks::BlockDb;
use shared::id::Id;

#[derive(Debug, Clone)]
pub struct Block {
    pub height: u64,
    pub hash: Option<Id>,
    pub app_hash: Option<Id>,
    pub timestamp: Option<chrono::NaiveDateTime>,
    pub proposer: Option<Id>,
    pub epoch: Option<u64>,
}

impl Block {
    pub fn from_db(block: BlockDb) -> Self {
        Self {
            height: block.height as u64,
            hash: block.hash.map(Id::Hash),
            app_hash: block.app_hash.map(Id::Hash),
            timestamp: block.timestamp,
            proposer: block.proposer.map(Id::Account),
            epoch: block.epoch.map(|epoch| epoch as u64),
        }
    }
}
