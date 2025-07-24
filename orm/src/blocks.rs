use diesel::{Insertable, Queryable, Selectable};
use shared::block::Block;
use tendermint_rpc::endpoint::block::Response as TendermintBlockResponse;

use crate::schema::blocks;

#[derive(Insertable, Clone, Queryable, Selectable, Debug)]
#[diesel(table_name = blocks)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct BlockDb {
    pub height: i32,
    pub hash: Option<String>,
    pub app_hash: Option<String>,
    pub timestamp: Option<chrono::NaiveDateTime>,
    pub proposer: Option<String>,
    pub epoch: Option<i32>,
}

pub type BlockInsertDb = BlockDb;

impl From<(Block, TendermintBlockResponse)> for BlockInsertDb {
    fn from(
        (block, tm_block_response): (Block, TendermintBlockResponse),
    ) -> Self {
        let timestamp = chrono::DateTime::from_timestamp(
            tm_block_response.block.header.time.unix_timestamp(),
            0,
        )
        .expect("Invalid timestamp")
        .naive_utc();

        Self {
            height: block.header.height as i32,
            hash: Some(block.hash.to_string()),
            app_hash: Some(block.header.app_hash.to_string()),
            timestamp: Some(timestamp),
            proposer: block.header.proposer_address_namada,
            epoch: Some(block.epoch as i32),
        }
    }
}

impl BlockInsertDb {
    pub fn fake(height: i32) -> Self {
        Self {
            height,
            hash: Some(height.to_string()), /* fake hash but ensures
                                             * uniqueness
                                             * with height */
            app_hash: Some("fake_app_hash".to_string()), /* doesn't require
                                                          * uniqueness */
            timestamp: Some(
                chrono::DateTime::from_timestamp(0, 0).unwrap().naive_utc(),
            ),
            proposer: Some("fake_proposer".to_string()),
            epoch: Some(0),
        }
    }
}
