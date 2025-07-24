use anyhow::Context;
use deadpool_diesel::postgres::Object;
use diesel::{ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl};
use orm::crawler_state::{CometbftCrawlerStateDb, CrawlerNameDb};
use orm::schema::crawler_state;
use shared::block::BlockHeight;
use shared::crawler_state::CometbftCrawlerState;
use shared::error::ContextDbInteractError;

pub async fn get_cometbft_crawler(
    conn: &Object,
) -> anyhow::Result<Option<CometbftCrawlerState>> {
    let crawler_state: Option<CometbftCrawlerStateDb> = conn
        .interact(move |conn| {
            crawler_state::table
                .filter(crawler_state::name.eq(CrawlerNameDb::Cometbft))
                .select((
                    crawler_state::dsl::last_processed_block,
                    crawler_state::dsl::timestamp,
                ))
                .first(conn)
                .optional()
        })
        .await
        .context_db_interact_error()?
        .context("Failed to read chain crawler state from the db")?;

    match crawler_state {
        Some(crawler_state) => Ok(Some(CometbftCrawlerState {
            last_processed_block: crawler_state.last_processed_block
                as BlockHeight,
            timestamp: crawler_state.timestamp.and_utc().timestamp(),
        })),
        None => Ok(None),
    }
}
