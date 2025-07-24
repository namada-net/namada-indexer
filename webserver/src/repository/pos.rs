use axum::async_trait;
use bigdecimal::BigDecimal;
use diesel::dsl::{sql, sum};
use diesel::sql_types::Integer;
use diesel::{
    BoolExpressionMethods, ExpressionMethods, JoinOnDsl,
    NullableExpressionMethods, QueryDsl, RunQueryDsl, SelectableHelper,
};
use orm::bond::BondDb;
use orm::crawler_state::{CrawlerNameDb, EpochCrawlerStateDb};
use orm::helpers::OrderByDb;
use orm::pos_rewards::PoSRewardDb;
use orm::schema::{
    bonds, crawler_state, pos_rewards, redelegation, unbonds, validators,
};
use orm::unbond::UnbondDb;
use orm::validators::{
    ValidatorDb, ValidatorSortByDb, ValidatorStateDb, validator_sort_by,
};

use super::utils::{Paginate, PaginatedResponseDb};
use crate::appstate::AppState;

#[derive(Clone)]
pub struct PosRepository {
    pub(crate) app_state: AppState,
}

#[async_trait]
pub trait PosRepositoryTrait {
    fn new(app_state: AppState) -> Self;

    async fn find_validators(
        &self,
        page: i64,
        states: Vec<ValidatorStateDb>,
        sort_by: Option<(ValidatorSortByDb, OrderByDb)>,
    ) -> Result<PaginatedResponseDb<ValidatorDb>, String>;

    async fn find_all_validators(
        &self,
        states: Vec<ValidatorStateDb>,
    ) -> Result<Vec<ValidatorDb>, String>;

    async fn get_validators_rank(&self) -> Result<Vec<i32>, String>;

    async fn find_validator_by_id(
        &self,
        id: i32,
    ) -> Result<Option<ValidatorDb>, String>;

    async fn find_validator_by_address(
        &self,
        address: String,
    ) -> Result<Option<ValidatorDb>, String>;

    async fn find_merged_bonds_by_address(
        &self,
        address: String,
        page: i64,
    ) -> Result<
        PaginatedResponseDb<(
            String,
            ValidatorDb,
            Option<i32>,
            Option<BigDecimal>,
        )>,
        String,
    >;

    async fn find_bonds_by_address(
        &self,
        address: String,
        page: i64,
        active_at: Option<i32>,
    ) -> Result<PaginatedResponseDb<(ValidatorDb, BondDb)>, String>;

    async fn find_unbonds_by_address(
        &self,
        address: String,
        page: i64,
        active_at: Option<i32>,
    ) -> Result<PaginatedResponseDb<(ValidatorDb, UnbondDb)>, String>;

    async fn find_merged_unbonds_by_address(
        &self,
        address: String,
        current_epoch: i32,
        page: i64,
    ) -> Result<
        PaginatedResponseDb<(String, ValidatorDb, Option<BigDecimal>, i32)>,
        String,
    >;

    async fn find_withdraws_by_address(
        &self,
        address: String,
        current_epoch: i32,
        page: i64,
    ) -> Result<PaginatedResponseDb<(ValidatorDb, UnbondDb)>, String>;

    async fn find_rewards_by_address(
        &self,
        address: String,
    ) -> Result<Vec<(PoSRewardDb, ValidatorDb)>, String>;

    async fn get_total_voting_power(&self) -> Result<Option<i64>, String>;

    async fn get_state(&self) -> Result<EpochCrawlerStateDb, String>;
}

#[async_trait]
impl PosRepositoryTrait for PosRepository {
    fn new(app_state: AppState) -> Self {
        Self { app_state }
    }

    async fn find_validators(
        &self,
        page: i64,
        states: Vec<ValidatorStateDb>,
        sort_by: Option<(ValidatorSortByDb, OrderByDb)>,
    ) -> Result<PaginatedResponseDb<ValidatorDb>, String> {
        let conn = self.app_state.get_db_connection().await;

        conn.interact(move |conn| {
            let mut boxed = validators::table
                .filter(validators::dsl::state.eq_any(states))
                .into_boxed();

            if let Some((sort_by, order_by)) = sort_by {
                boxed = boxed.order(validator_sort_by(sort_by, order_by));
            }

            boxed
                .select(ValidatorDb::as_select())
                .paginate(page)
                .load_and_count_pages(conn)
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
    }

    async fn find_all_validators(
        &self,
        states: Vec<ValidatorStateDb>,
    ) -> Result<Vec<ValidatorDb>, String> {
        let conn = self.app_state.get_db_connection().await;

        conn.interact(move |conn| {
            validators::table
                .filter(validators::dsl::state.eq_any(states))
                .select(ValidatorDb::as_select())
                .load(conn)
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
    }

    /// Returns the ids of validators in consensus in descending order of voting
    /// power
    async fn get_validators_rank(&self) -> Result<Vec<i32>, String> {
        let conn = self.app_state.get_db_connection().await;

        conn.interact(move |conn| {
            validators::table
                .filter(validators::dsl::state.eq(ValidatorStateDb::Consensus))
                .order(validators::dsl::voting_power.desc())
                .select(validators::id)
                .load(conn)
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
    }

    async fn find_validator_by_id(
        &self,
        validator_id: i32,
    ) -> Result<Option<ValidatorDb>, String> {
        let conn = self.app_state.get_db_connection().await;

        conn.interact(move |conn| {
            validators::table
                .filter(validators::dsl::id.eq(validator_id))
                .select(ValidatorDb::as_select())
                .first(conn)
                .ok()
        })
        .await
        .map_err(|e| e.to_string())
    }

    async fn find_validator_by_address(
        &self,
        address: String,
    ) -> Result<Option<ValidatorDb>, String> {
        let conn = self.app_state.get_db_connection().await;

        conn.interact(move |conn| {
            validators::table
                .filter(validators::dsl::namada_address.eq(address))
                .select(ValidatorDb::as_select())
                .first(conn)
                .ok()
        })
        .await
        .map_err(|e| e.to_string())
    }

    async fn find_bonds_by_address(
        &self,
        address: String,
        page: i64,
        active_at: Option<i32>,
    ) -> Result<PaginatedResponseDb<(ValidatorDb, BondDb)>, String> {
        let conn = self.app_state.get_db_connection().await;

        conn.interact(move |conn| {
            let mut query =
                validators::table.inner_join(bonds::table).into_boxed();

            if let Some(at) = active_at {
                query = query.filter(bonds::dsl::start.le(at));
            }

            query
                .filter(bonds::dsl::address.eq(address))
                .select((validators::all_columns, bonds::all_columns))
                .paginate(page)
                .load_and_count_pages::<(ValidatorDb, BondDb)>(conn)
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
    }

    async fn find_merged_bonds_by_address(
        &self,
        address: String,
        page: i64,
    ) -> Result<
        PaginatedResponseDb<(
            String,
            ValidatorDb,
            Option<i32>,
            Option<BigDecimal>,
        )>,
        String,
    > {
        let conn = self.app_state.get_db_connection().await;

        conn.interact(move |conn| {

            validators::table
                .left_outer_join(
                    redelegation::table.on(
                        validators::id.eq(redelegation::validator_id)
                        .and(redelegation::delegator.eq(address.clone()))
                    )
                )
                .inner_join(bonds::table)
                .filter(bonds::address.eq(address.clone()))
                .group_by((bonds::address, validators::id, redelegation::end_epoch))
                .select((
                    bonds::address,
                    validators::all_columns,
                    redelegation::end_epoch.nullable(),
                    sum(bonds::raw_amount),
                ))
                .paginate(page)
                // TODO: this is ok for now, create mixed aggragate later
                .load_and_count_pages::<(String, ValidatorDb, Option<i32>, Option<BigDecimal>)>(
                    conn,
                )
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
    }

    async fn find_unbonds_by_address(
        &self,
        address: String,
        page: i64,
        active_at: Option<i32>,
    ) -> Result<PaginatedResponseDb<(ValidatorDb, UnbondDb)>, String> {
        let conn = self.app_state.get_db_connection().await;

        conn.interact(move |conn| {
            let mut query =
                validators::table.inner_join(unbonds::table).into_boxed();

            if let Some(at) = active_at {
                query = query.filter(unbonds::dsl::withdraw_epoch.lt(at));
            }

            query
                .filter(unbonds::dsl::address.eq(address))
                .select((validators::all_columns, unbonds::all_columns))
                .paginate(page)
                .load_and_count_pages::<(ValidatorDb, UnbondDb)>(conn)
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
    }

    async fn find_merged_unbonds_by_address(
        &self,
        address: String,
        current_epoch: i32,
        page: i64,
    ) -> Result<
        PaginatedResponseDb<(String, ValidatorDb, Option<BigDecimal>, i32)>,
        String,
    > {
        let conn = self.app_state.get_db_connection().await;

        conn.interact(move |conn| {
                validators::table
                .inner_join(unbonds::table)
                .filter(unbonds::dsl::address.eq(address))
                .group_by((unbonds::address, validators::id, sql::<Integer>(&format!("CASE WHEN withdraw_epoch <= {} THEN 0 ELSE withdraw_epoch END", current_epoch))))
                .select((
                        unbonds::address,
                        validators::all_columns,
                        sum(unbonds::raw_amount),
                        sql::<Integer>(&format!("CASE WHEN MIN(withdraw_epoch) <= {} THEN 0 ELSE MAX(withdraw_epoch) END AS withdraw_epoch", current_epoch))))
                .paginate(page)
                .load_and_count_pages::<(String, ValidatorDb, Option<BigDecimal>, i32)>(conn)

        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
    }

    async fn find_withdraws_by_address(
        &self,
        address: String,
        current_epoch: i32,
        page: i64,
    ) -> Result<PaginatedResponseDb<(ValidatorDb, UnbondDb)>, String> {
        let conn = self.app_state.get_db_connection().await;

        conn.interact(move |conn| {
            validators::table
                .inner_join(unbonds::table)
                .filter(
                    unbonds::dsl::address
                        .eq(address)
                        .and(unbonds::dsl::withdraw_epoch.le(current_epoch)),
                )
                .select((validators::all_columns, unbonds::all_columns))
                .paginate(page)
                .load_and_count_pages::<(ValidatorDb, UnbondDb)>(conn)
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
    }

    async fn find_rewards_by_address(
        &self,
        address: String,
    ) -> Result<Vec<(PoSRewardDb, ValidatorDb)>, String> {
        let conn = self.app_state.get_db_connection().await;

        conn.interact(move |conn| {
            pos_rewards::table
                .inner_join(validators::table)
                .filter(pos_rewards::dsl::owner.eq(address))
                .select((PoSRewardDb::as_select(), ValidatorDb::as_select()))
                .get_results(conn)
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
    }

    async fn get_total_voting_power(&self) -> Result<Option<i64>, String> {
        let conn = self.app_state.get_db_connection().await;

        conn.interact(move |conn| {
            validators::table
                .select(sum(validators::dsl::voting_power))
                .first(conn)
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
    }

    async fn get_state(&self) -> Result<EpochCrawlerStateDb, String> {
        let conn = self.app_state.get_db_connection().await;

        conn.interact(move |conn| {
            crawler_state::table
                .filter(crawler_state::dsl::name.eq(CrawlerNameDb::Pos))
                .select((
                    crawler_state::dsl::last_processed_epoch,
                    crawler_state::dsl::timestamp,
                ))
                .first(conn)
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
    }
}
