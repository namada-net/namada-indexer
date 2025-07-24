use anyhow::Context;
use chrono::NaiveDateTime;
use diesel::upsert::excluded;
use diesel::{ExpressionMethods, PgConnection, RunQueryDsl};
use orm::cometbft::CometbftBlock;
use orm::crawler_state::{BlockStateInsertDb, CrawlerNameDb};
use orm::schema::{cometbft_block, crawler_state};
use shared::crawler_state::{BlockCrawlerState, CrawlerName};

pub fn upsert_blocks(
    transaction_conn: &mut PgConnection,
    blocks: Vec<shared::cometbft::CometbftBlock>,
) -> anyhow::Result<()> {
    diesel::insert_into(cometbft_block::table)
        .values::<Vec<CometbftBlock>>(
            blocks
                .into_iter()
                .map(CometbftBlock::from)
                .collect::<Vec<_>>(),
        )
        .on_conflict(cometbft_block::id)
        .do_nothing()
        .execute(transaction_conn)
        .context("Failed to insert block in db")?;

    anyhow::Ok(())
}

pub fn update_timestamp(
    transaction_conn: &mut PgConnection,
    timestamp: NaiveDateTime,
) -> anyhow::Result<()> {
    diesel::update(crawler_state::table)
        .filter(
            crawler_state::name.eq(CrawlerNameDb::from(CrawlerName::Cometbft)),
        )
        .set(crawler_state::timestamp.eq(timestamp))
        .execute(transaction_conn)
        .context("Failed to update crawler timestamp in db")?;

    anyhow::Ok(())
}

pub fn insert_crawler_state(
    transaction_conn: &mut PgConnection,
    crawler_state: BlockCrawlerState,
) -> anyhow::Result<()> {
    diesel::insert_into(crawler_state::table)
        .values::<&BlockStateInsertDb>(
            &(CrawlerName::Cometbft, crawler_state).into(),
        )
        .on_conflict(crawler_state::name)
        .do_update()
        .set((
            crawler_state::timestamp.eq(excluded(crawler_state::timestamp)),
            crawler_state::last_processed_block
                .eq(excluded(crawler_state::last_processed_block)),
        ))
        .execute(transaction_conn)
        .context("Failed to update crawler state in db")?;

    anyhow::Ok(())
}
