use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use orm::crawler_state::{CrawlerNameDb, CrawlerStateDb};
use orm::schema::crawler_state;

use crate::appstate::AppState;
use crate::dto::crawler_state::CrawlerNameDto;
use crate::entity::crawler::CrawlersTimestamps;
use crate::error::crawler_state::CrawlerStateError;

#[derive(Clone)]
pub struct CrawlerStateService {
    pub(crate) app_state: AppState,
}

impl CrawlerStateService {
    pub fn new(app_state: AppState) -> Self {
        Self { app_state }
    }

    pub async fn get_timestamps(
        &self,
        names: Vec<CrawlerNameDto>,
    ) -> Result<Vec<CrawlersTimestamps>, CrawlerStateError> {
        let conn = self.app_state.get_db_connection().await;
        let names_db = names
            .iter()
            .map(Self::to_crawler_name_db)
            .collect::<Vec<_>>();

        let crawlers_db: Result<Vec<CrawlerStateDb>, CrawlerStateError> = conn
            .interact(move |conn| {
                let mut query = crawler_state::table.into_boxed();

                if !names_db.is_empty() {
                    query = query.filter(crawler_state::name.eq_any(names_db));
                }

                query
                    .select(crawler_state::all_columns)
                    .get_results(conn)
                    .map_err(|e| CrawlerStateError::Database(e.to_string()))
            })
            .await
            .map_err(|e| CrawlerStateError::Database(e.to_string()))?;

        crawlers_db.map(|crawlers| {
            crawlers
                .into_iter()
                .map(|crawler| CrawlersTimestamps {
                    name: crawler.name.to_string(),
                    timestamp: crawler.timestamp.and_utc().timestamp(),
                    last_processed_block_height: crawler
                        .last_processed_block
                        .map(|h| h as u64),
                })
                .collect::<Vec<CrawlersTimestamps>>()
        })
    }

    fn to_crawler_name_db(value: &CrawlerNameDto) -> CrawlerNameDb {
        match value {
            CrawlerNameDto::Chain => CrawlerNameDb::Chain,
            CrawlerNameDto::Governance => CrawlerNameDb::Governance,
            CrawlerNameDto::Parameters => CrawlerNameDb::Parameters,
            CrawlerNameDto::Pos => CrawlerNameDb::Pos,
            CrawlerNameDto::Rewards => CrawlerNameDb::Rewards,
            CrawlerNameDto::Transactions => CrawlerNameDb::Transactions,
        }
    }
}
