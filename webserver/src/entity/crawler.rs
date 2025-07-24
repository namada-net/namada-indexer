#[derive(Clone, Debug)]
pub struct CrawlersTimestamps {
    pub name: String,
    pub timestamp: i64,
    pub last_processed_block_height: Option<u64>,
}
