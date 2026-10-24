use async_trait::async_trait;
use diesel::dsl::max;
use diesel::{
    ExpressionMethods, JoinOnDsl, OptionalExtension, QueryDsl, RunQueryDsl,
    SelectableHelper,
};
use orm::crawler_state::{ChainCrawlerStateDb, CrawlerNameDb};
use orm::parameters::ParametersDb;
use orm::schema::{
    chain_parameters, crawler_state, ibc_token, token, token_supplies_per_epoch,
};
use orm::token::{IbcTokenDb, TokenDb};
use orm::token_supplies_per_epoch::TokenSuppliesDb;

use crate::appstate::AppState;

#[derive(Clone)]
pub struct ChainRepository {
    pub(crate) app_state: AppState,
}

#[async_trait]
pub trait ChainRepositoryTrait {
    fn new(app_state: AppState) -> Self;

    async fn find_latest_height(&self) -> Result<i32, String>;

    async fn find_latest_epoch(&self) -> Result<i32, String>;

    async fn find_chain_parameters(&self) -> Result<ParametersDb, String>;

    async fn get_state(&self) -> Result<ChainCrawlerStateDb, String>;

    async fn find_tokens(
        &self,
    ) -> Result<Vec<(TokenDb, Option<IbcTokenDb>)>, String>;

    async fn get_token_supply(
        &self,
        address: String,
        epoch: Option<i32>,
    ) -> Result<Option<TokenSuppliesDb>, String>;
}

#[async_trait]
impl ChainRepositoryTrait for ChainRepository {
    fn new(app_state: AppState) -> Self {
        Self { app_state }
    }

    async fn find_latest_height(&self) -> Result<i32, String> {
        let conn = self.app_state.get_db_connection().await;

        conn.interact(move |conn| {
            crawler_state::dsl::crawler_state
                .filter(crawler_state::dsl::name.eq(CrawlerNameDb::Chain))
                .select(max(crawler_state::dsl::last_processed_block))
                .first::<Option<i32>>(conn)
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
        .and_then(|x| x.ok_or("No block processed".to_string()))
    }

    async fn find_latest_epoch(&self) -> Result<i32, String> {
        let conn = self.app_state.get_db_connection().await;

        conn.interact(move |conn| {
            crawler_state::dsl::crawler_state
                .filter(crawler_state::dsl::name.eq(CrawlerNameDb::Chain))
                .select(max(crawler_state::dsl::last_processed_epoch))
                .first::<Option<i32>>(conn)
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
        .and_then(|x| x.ok_or("No epoch processed".to_string()))
    }

    async fn get_state(&self) -> Result<ChainCrawlerStateDb, String> {
        let conn = self.app_state.get_db_connection().await;

        conn.interact(move |conn| {
            crawler_state::table
                .filter(crawler_state::dsl::name.eq(CrawlerNameDb::Chain))
                .select((
                    crawler_state::dsl::last_processed_block,
                    crawler_state::dsl::last_processed_epoch,
                    crawler_state::dsl::first_block_in_epoch,
                    crawler_state::dsl::timestamp,
                ))
                .first(conn)
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
    }

    async fn find_chain_parameters(&self) -> Result<ParametersDb, String> {
        let conn = self.app_state.get_db_connection().await;

        conn.interact(move |conn| {
            chain_parameters::table
                .select(ParametersDb::as_select())
                .first(conn)
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
    }

    async fn find_tokens(
        &self,
    ) -> Result<Vec<(TokenDb, Option<IbcTokenDb>)>, String> {
        let conn = self.app_state.get_db_connection().await;

        conn.interact(move |conn| {
            token::table
                .left_join(
                    ibc_token::table.on(token::address.eq(ibc_token::address)),
                )
                .select((
                    TokenDb::as_select(),
                    Option::<IbcTokenDb>::as_select(),
                ))
                .load::<(TokenDb, Option<IbcTokenDb>)>(conn)
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
    }

    async fn get_token_supply(
        &self,
        address: String,
        epoch: Option<i32>,
    ) -> Result<Option<TokenSuppliesDb>, String> {
        let conn = self.app_state.get_db_connection().await;

        conn.interact(move |conn| {
            let mut query = token_supplies_per_epoch::table
                .filter(token_supplies_per_epoch::dsl::address.eq(&address))
                .into_boxed();

            if let Some(epoch) = epoch {
                query = query
                    .filter(token_supplies_per_epoch::dsl::epoch.eq(epoch));
            } else {
                query =
                    query.order(token_supplies_per_epoch::dsl::epoch.desc());
            }

            query
                .select(TokenSuppliesDb::as_select())
                .first(conn)
                .optional()
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
    }
}
