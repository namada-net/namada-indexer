use serde::{Deserialize, Serialize};

use crate::entity::crawler::CrawlersTimestamps;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CrawlersTimestampsResponse {
    pub name: String,
    pub timestamp: i64,
    pub last_processed_block_height: Option<u64>,
}

impl From<CrawlersTimestamps> for CrawlersTimestampsResponse {
    fn from(value: CrawlersTimestamps) -> Self {
        Self {
            name: value.name,
            timestamp: value.timestamp,
            last_processed_block_height: value.last_processed_block_height,
        }
    }
}
