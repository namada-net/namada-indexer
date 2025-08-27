use axum::async_trait;
use diesel::{
    ExpressionMethods, JoinOnDsl, QueryDsl, RunQueryDsl, SelectableHelper,
};
use orm::schema::{
    inner_transactions, transaction_history, wrapper_transactions,
};
use orm::transactions::{
    InnerTransactionDb, TransactionHistoryDb, TransactionKindDb, WrapperTransactionDb,
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
    async fn find_recent_inner_txs(
        &self,
        limit: Option<u64>,
        page: i64,
        kinds: Option<Vec<TransactionKindDb>>,
        tokens: Option<Vec<String>>,
    ) -> Result<PaginatedResponseDb<(InnerTransactionDb, i32)>, String>;
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

    async fn find_recent_inner_txs(
        &self,
        limit: Option<u64>,
        page: i64,
        kinds: Option<Vec<TransactionKindDb>>,
        tokens: Option<Vec<String>>,
    ) -> Result<PaginatedResponseDb<(InnerTransactionDb, i32)>, String> {
        let conn = self.app_state.get_db_connection().await;

        conn.interact(move |conn| {
            let mut query = inner_transactions::table
                .inner_join(wrapper_transactions::table.on(inner_transactions::dsl::wrapper_id.eq(wrapper_transactions::dsl::id)))
                .into_boxed();

            // Filter by transaction kind if specified, otherwise return all kinds
            if let Some(kinds) = kinds {
                query = query.filter(inner_transactions::dsl::kind.eq_any(kinds));
            }

            // Filter by token if specified
            if let Some(tokens) = tokens {
                // For regular transfers: check sources and targets tokens
                let _regular_transfer_kinds = vec![
                    TransactionKindDb::TransparentTransfer,
                    TransactionKindDb::ShieldedTransfer,
                    TransactionKindDb::ShieldingTransfer,
                    TransactionKindDb::UnshieldingTransfer,
                    TransactionKindDb::MixedTransfer,
                ];
                
                // For IBC transfers: check IBC token address
                let _ibc_transfer_kinds = vec![
                    TransactionKindDb::IbcTransparentTransfer,
                    TransactionKindDb::IbcShieldingTransfer,
                    TransactionKindDb::IbcUnshieldingTransfer,
                ];

                // Build token filter condition
                let regular_kinds_str = "'transparent_transfer'::transaction_kind,
                                          'shielded_transfer'::transaction_kind,
                                          'shielding_transfer'::transaction_kind,
                                          'unshielding_transfer'::transaction_kind,
                                          'mixed_transfer'::transaction_kind";
                let ibc_kinds_str = "'ibc_transparent_transfer'::transaction_kind,
                                      'ibc_shielding_transfer'::transaction_kind,
                                      'ibc_unshielding_transfer'::transaction_kind";

                let tokens_str = tokens.iter()
                    .map(|t| format!("'{}'", t))
                    .collect::<Vec<_>>()
                    .join(",");

                // Wrap the OR group in extra parentheses to preserve AND/OR precedence with other filters
                let token_filter = diesel::dsl::sql::<diesel::sql_types::Bool>(&format!(
                    "((kind IN ({regular_kinds}) AND ((data::jsonb->'sources'->0->>'token') = ANY(ARRAY[{tokens}]) OR (data::jsonb->'targets'->0->>'token') = ANY(ARRAY[{tokens}])))
                      OR (kind IN ({ibc_kinds}) AND (data::jsonb->0->'Ibc'->'address'->>'Account') = ANY(ARRAY[{tokens}])))",
                    regular_kinds = regular_kinds_str,
                    ibc_kinds = ibc_kinds_str,
                    tokens = tokens_str
                ));

                query = query.filter(token_filter);
            }

            // Apply limit if specified to break early after reaching the requested number of matches 
            if let Some(limit) = limit {
                query = query.limit(limit as i64);
            }

            query
                .order(wrapper_transactions::dsl::block_height.desc())
                .select((inner_transactions::all_columns, wrapper_transactions::dsl::block_height))
                .paginate(page)
                .load_and_count_pages::<(InnerTransactionDb, i32)>(conn)
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
    }
}
