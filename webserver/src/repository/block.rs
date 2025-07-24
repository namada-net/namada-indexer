use axum::async_trait;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl, SelectableHelper};
use orm::blocks::BlockDb;
use orm::schema::blocks;

use crate::appstate::AppState;

#[derive(Clone)]
pub struct BlockRepository {
    pub(crate) app_state: AppState,
}

#[async_trait]
pub trait BlockRepositoryTrait {
    fn new(app_state: AppState) -> Self;

    async fn find_block_by_height(
        &self,
        height: i32,
    ) -> Result<Option<BlockDb>, String>;

    async fn find_block_by_timestamp(
        &self,
        timestamp: i64,
    ) -> Result<Option<BlockDb>, String>;

    async fn find_block_by_hash(
        &self,
        hash: String,
    ) -> Result<Option<BlockDb>, String>;
}

#[async_trait]
impl BlockRepositoryTrait for BlockRepository {
    fn new(app_state: AppState) -> Self {
        Self { app_state }
    }

    async fn find_block_by_height(
        &self,
        height: i32,
    ) -> Result<Option<BlockDb>, String> {
        let conn = self.app_state.get_db_connection().await;

        conn.interact(move |conn| {
            blocks::table
                .filter(blocks::dsl::height.eq(height))
                .select(BlockDb::as_select())
                .first(conn)
                .ok()
        })
        .await
        .map_err(|e| e.to_string())
    }

    /// Gets the last block preceeding the given timestamp
    async fn find_block_by_timestamp(
        &self,
        timestamp: i64,
    ) -> Result<Option<BlockDb>, String> {
        let conn = self.app_state.get_db_connection().await;
        let timestamp = chrono::DateTime::from_timestamp(timestamp, 0)
            .expect("Invalid timestamp")
            .naive_utc();

        conn.interact(move |conn| {
            blocks::table
                .filter(blocks::timestamp.le(timestamp))
                .order(blocks::timestamp.desc())
                .select(BlockDb::as_select())
                .first(conn)
                .ok()
        })
        .await
        .map_err(|e| e.to_string())
    }

    async fn find_block_by_hash(
        &self,
        hash: String,
    ) -> Result<Option<BlockDb>, String> {
        let conn = self.app_state.get_db_connection().await;

        conn.interact(move |conn| {
            blocks::table
                .filter(blocks::hash.eq(&hash))
                .or_filter(blocks::app_hash.eq(&hash))
                .order(blocks::timestamp.desc())
                .select(BlockDb::as_select())
                .first(conn)
                .ok()
        })
        .await
        .map_err(|e| e.to_string())
    }
}
