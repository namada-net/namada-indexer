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
use crate::entity::transaction::TransactionKind;

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

    async fn find_recent_matching_wrappers(
        &self,
        offset: i64,
        size: i32,
        kinds: Option<Vec<TransactionKind>>,
        tokens: Option<Vec<String>>,
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

    async fn find_recent_matching_wrappers(
        &self,
        offset: i64,
        size: i32,
        kinds: Option<Vec<TransactionKind>>,
        tokens: Option<Vec<String>>,
    ) -> Result<Vec<WrapperTransactionDb>, String> {
        let conn = self.app_state.get_db_connection().await;

        conn.interact(move |conn| {
            use diesel::dsl;
            use diesel::prelude::*;

            let mut query = wrapper_transactions::table.into_boxed();

            let mut exists_clauses: Vec<String> = Vec::new();

            // Kind filter
            if let Some(kinds) = kinds {
                if !kinds.is_empty() {
                    fn kind_to_db(k: &TransactionKind) -> &'static str {
                        match k {
                            TransactionKind::TransparentTransfer => {
                                "transparent_transfer"
                            }
                            TransactionKind::ShieldedTransfer => {
                                "shielded_transfer"
                            }
                            TransactionKind::ShieldingTransfer => {
                                "shielding_transfer"
                            }
                            TransactionKind::UnshieldingTransfer => {
                                "unshielding_transfer"
                            }
                            TransactionKind::MixedTransfer => "mixed_transfer",
                            TransactionKind::Bond => "bond",
                            TransactionKind::Redelegation => "redelegation",
                            TransactionKind::Unbond => "unbond",
                            TransactionKind::Withdraw => "withdraw",
                            TransactionKind::ClaimRewards => "claim_rewards",
                            TransactionKind::VoteProposal => "vote_proposal",
                            TransactionKind::InitProposal => "init_proposal",
                            TransactionKind::ChangeMetadata => {
                                "change_metadata"
                            }
                            TransactionKind::ChangeCommission => {
                                "change_commission"
                            }
                            TransactionKind::RevealPk => "reveal_pk",
                            TransactionKind::IbcMsgTransfer => {
                                "ibc_msg_transfer"
                            }
                            TransactionKind::IbcTransparentTransfer => {
                                "ibc_transparent_transfer"
                            }
                            TransactionKind::IbcShieldingTransfer => {
                                "ibc_shielding_transfer"
                            }
                            TransactionKind::IbcUnshieldingTransfer => {
                                "ibc_unshielding_transfer"
                            }
                            TransactionKind::BecomeValidator => {
                                "become_validator"
                            }
                            TransactionKind::DeactivateValidator => {
                                "deactivate_validator"
                            }
                            TransactionKind::ReactivateValidator => {
                                "reactivate_validator"
                            }
                            TransactionKind::UnjailValidator => {
                                "unjail_validator"
                            }
                            TransactionKind::ChangeConsensusKey => {
                                "change_consensus_key"
                            }
                            TransactionKind::InitAccount => "init_account",
                            TransactionKind::Unknown => "unknown",
                        }
                    }

                    let kinds_str = kinds
                        .into_iter()
                        .map(|k| {
                            format!("'{}'::transaction_kind", kind_to_db(&k))
                        })
                        .collect::<Vec<_>>()
                        .join(",");
                    exists_clauses.push(format!(
                        "inner_transactions.kind IN ({})",
                        kinds_str
                    ));
                }
            }

            // Token filters
            if let Some(tokens) = tokens {
                if !tokens.is_empty() {
                    let tokens_str = tokens
                        .into_iter()
                        .map(|t| format!("'{}'", t))
                        .collect::<Vec<_>>()
                        .join(",");

                    // IBC and non-IBC transfers have different 'data' json
                    // structure, so the query will differ slighlty

                    // Non-IBC transfer kinds
                    let regular_kinds_str =
                        "'transparent_transfer'::transaction_kind,
                                              \
                         'shielded_transfer'::transaction_kind,
                                              \
                         'shielding_transfer'::transaction_kind,
                                              \
                         'unshielding_transfer'::transaction_kind,
                                              \
                         'mixed_transfer'::transaction_kind";

                    // IBC transfer kinds
                    let ibc_kinds_str =
                        "'ibc_transparent_transfer'::transaction_kind,
                                          \
                         'ibc_shielding_transfer'::transaction_kind,
                                          \
                         'ibc_unshielding_transfer'::transaction_kind";

                    let token_predicate = format!(
                        "((inner_transactions.kind IN ({regular_kinds}) AND \
                         ((inner_transactions.data::jsonb->'sources'->0->>'\
                         token') = ANY(ARRAY[{tokens}]) OR \
                         (inner_transactions.data::jsonb->'targets'->0->>'\
                         token') = ANY(ARRAY[{tokens}])))
                          OR (inner_transactions.kind IN ({ibc_kinds}) AND \
                         (inner_transactions.data::jsonb->0->'Ibc'->'address'\
                         ->>'Account') = ANY(ARRAY[{tokens}])))",
                        regular_kinds = regular_kinds_str,
                        ibc_kinds = ibc_kinds_str,
                        tokens = tokens_str,
                    );
                    exists_clauses.push(token_predicate);
                }
            }

            if !exists_clauses.is_empty() {
                let exists_sql = format!(
                    "EXISTS (SELECT 1 FROM inner_transactions WHERE \
                     inner_transactions.wrapper_id = wrapper_transactions.id \
                     AND {} )",
                    exists_clauses.join(" AND ")
                );
                let exists_filter =
                    dsl::sql::<diesel::sql_types::Bool>(&exists_sql);
                query = query.filter(exists_filter);
            }

            query
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
