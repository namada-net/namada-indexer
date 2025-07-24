use diesel::{Insertable, Queryable, Selectable};

use crate::schema::cometbft_block;

#[derive(Insertable, Clone, Queryable, Selectable, Debug)]
#[diesel(table_name = cometbft_block)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct CometbftBlock {
    pub id: i32,
    pub encoded_block: String,
    pub encoded_block_result: String,
    pub epoch: i32,
}

impl From<CometbftBlock> for shared::cometbft::CometbftBlock {
    fn from(block: CometbftBlock) -> Self {
        shared::cometbft::CometbftBlock {
            block_height: block.id as u32,
            block: serde_json::from_str(&block.encoded_block).unwrap(),
            events: serde_json::from_str(&block.encoded_block_result).unwrap(),
            epoch: block.epoch as u32,
        }
    }
}

impl From<shared::cometbft::CometbftBlock> for CometbftBlock {
    fn from(block: shared::cometbft::CometbftBlock) -> Self {
        CometbftBlock {
            id: block.block_height as i32,
            encoded_block: serde_json::to_string(&block.block).unwrap(),
            encoded_block_result: serde_json::to_string(&block.events).unwrap(),
            epoch: block.epoch as i32,
        }
    }
}
