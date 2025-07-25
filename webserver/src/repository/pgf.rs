use axum::async_trait;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl, SelectableHelper};
use orm::pgf::PublicGoodFundingPaymentDb;
use orm::schema::public_good_funding;

use super::utils::{Paginate, PaginatedResponseDb};
use crate::appstate::AppState;

#[derive(Clone)]
pub struct PgfRepo {
    pub(crate) app_state: AppState,
}

#[async_trait]
pub trait PgfRepoTrait {
    fn new(app_state: AppState) -> Self;

    async fn get_pgf_continuous_payments(
        &self,
        page: i64,
    ) -> Result<PaginatedResponseDb<PublicGoodFundingPaymentDb>, String>;

    async fn find_pgf_payments_by_proposal_id(
        &self,
        proposal_id: i32,
    ) -> Result<Vec<PublicGoodFundingPaymentDb>, String>;
}

#[async_trait]
impl PgfRepoTrait for PgfRepo {
    fn new(app_state: AppState) -> Self {
        Self { app_state }
    }

    async fn get_pgf_continuous_payments(
        &self,
        page: i64,
    ) -> Result<PaginatedResponseDb<PublicGoodFundingPaymentDb>, String> {
        let conn = self.app_state.get_db_connection().await;

        conn.interact(move |conn| {
            public_good_funding::table
                .select(PublicGoodFundingPaymentDb::as_select())
                .order(public_good_funding::columns::proposal_id.desc())
                .paginate(page)
                .load_and_count_pages(conn)
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
    }

    async fn find_pgf_payments_by_proposal_id(
        &self,
        proposal_id: i32,
    ) -> Result<Vec<PublicGoodFundingPaymentDb>, String> {
        let conn = self.app_state.get_db_connection().await;

        conn.interact(move |conn| {
            public_good_funding::table
                .filter(public_good_funding::dsl::proposal_id.eq(proposal_id))
                .select(PublicGoodFundingPaymentDb::as_select())
                .get_results(conn)
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
    }
}
