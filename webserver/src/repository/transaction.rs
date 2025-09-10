use async_trait::async_trait;
use diesel::{
    ExpressionMethods, JoinOnDsl, QueryDsl, RunQueryDsl, SelectableHelper,
};
use orm::schema::{
    inner_transactions, transaction_history, wrapper_transactions,
};
use orm::transactions::{
    InnerTransactionDb, TransactionHistoryDb, WrapperTransactionDb,
};

use super::utils::{Paginate, PaginatedResponseDb};
use crate::appstate::AppState;

#[derive(Clone)]
pub struct TransactionRepository {
    pub(crate) app_state: AppState,
}

#[async_trait]
pub trait TransactionRepositoryTrait {
    fn new(app_state: AppState) -> Self;

    async fn find_wrapper_tx(
        &self,
        id: String,
    ) -> Result<Option<WrapperTransactionDb>, String>;
    async fn find_inners_by_wrapper_tx(
        &self,
        wrapper_id: String,
    ) -> Result<Vec<InnerTransactionDb>, String>;
    async fn find_inner_tx(
        &self,
        id: String,
    ) -> Result<Option<InnerTransactionDb>, String>;
    async fn find_addresses_history(
        &self,
        addresses: Vec<String>,
        page: i64,
    ) -> Result<
        PaginatedResponseDb<(TransactionHistoryDb, InnerTransactionDb, i32)>,
        String,
    >;
    async fn find_txs_by_block_height(
        &self,
        block_height: i32,
    ) -> Result<Vec<WrapperTransactionDb>, String>;
    async fn find_most_recent_transactions(
        &self,
        offset: i64,
        size: i32,
    ) -> Result<Vec<WrapperTransactionDb>, String>;
}

#[async_trait]
impl TransactionRepositoryTrait for TransactionRepository {
    fn new(app_state: AppState) -> Self {
        Self { app_state }
    }

    async fn find_wrapper_tx(
        &self,
        id: String,
    ) -> Result<Option<WrapperTransactionDb>, String> {
        let conn = self.app_state.get_db_connection().await;

        conn.interact(move |conn| {
            wrapper_transactions::table
                .find(id)
                .select(WrapperTransactionDb::as_select())
                .first(conn)
                .ok()
        })
        .await
        .map_err(|e| e.to_string())
    }

    async fn find_inners_by_wrapper_tx(
        &self,
        wrapper_id: String,
    ) -> Result<Vec<InnerTransactionDb>, String> {
        let conn = self.app_state.get_db_connection().await;

        conn.interact(move |conn| {
            inner_transactions::table
                .filter(inner_transactions::dsl::wrapper_id.eq(wrapper_id))
                .select(InnerTransactionDb::as_select())
                .get_results(conn)
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
    }

    async fn find_inner_tx(
        &self,
        id: String,
    ) -> Result<Option<InnerTransactionDb>, String> {
        let conn = self.app_state.get_db_connection().await;

        conn.interact(move |conn| {
            inner_transactions::table
                .find(id)
                .select(InnerTransactionDb::as_select())
                .first(conn)
                .ok()
        })
        .await
        .map_err(|e| e.to_string())
    }

    async fn find_addresses_history(
        &self,
        addresses: Vec<String>,
        page: i64,
    ) -> Result<
        PaginatedResponseDb<(TransactionHistoryDb, InnerTransactionDb, i32)>,
        String,
    > {
        let conn = self.app_state.get_db_connection().await;

        conn.interact(move |conn| {
            transaction_history::table
                .filter(transaction_history::dsl::target.eq_any(addresses))
                .inner_join(inner_transactions::table.on(transaction_history::dsl::inner_tx_id.eq(inner_transactions::dsl::id)))
                .inner_join(wrapper_transactions::table.on(inner_transactions::dsl::wrapper_id.eq(wrapper_transactions::dsl::id)))
                .order(wrapper_transactions::dsl::block_height.desc())
                .select((transaction_history::all_columns, inner_transactions::all_columns, wrapper_transactions::dsl::block_height))
                .paginate(page)
                .load_and_count_pages::<(TransactionHistoryDb, InnerTransactionDb, i32)>(conn)
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
    }

    async fn find_txs_by_block_height(
        &self,
        block_height: i32,
    ) -> Result<Vec<WrapperTransactionDb>, String> {
        let conn = self.app_state.get_db_connection().await;

        conn.interact(move |conn| {
            wrapper_transactions::table
                .filter(
                    wrapper_transactions::dsl::block_height.eq(block_height),
                )
                .select(WrapperTransactionDb::as_select())
                .get_results(conn)
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
    }

    async fn find_most_recent_transactions(
        &self,
        offset: i64,
        size: i32,
    ) -> Result<Vec<WrapperTransactionDb>, String> {
        let conn = self.app_state.get_db_connection().await;

        conn.interact(move |conn| {
            wrapper_transactions::table
                .order(wrapper_transactions::dsl::block_height.desc())
                .offset(offset)
                .limit(size as i64)
                .select(WrapperTransactionDb::as_select())
                .get_results(conn)
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
    }
}
