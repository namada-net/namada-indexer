use async_trait::async_trait;
use diesel::dsl::{exists, sql};
use diesel::prelude::*;
use diesel::sql_types::Text;
use diesel::{
    ExpressionMethods, JoinOnDsl, QueryDsl, RunQueryDsl, SelectableHelper,
};
use orm::schema::{
    inner_transactions, transaction_history, wrapper_transactions,
};
use orm::transactions::{
    InnerTransactionDb, TransactionHistoryDb, TransactionKindDb,
    WrapperTransactionDb,
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
        kinds: Vec<TransactionKind>,
        tokens: Vec<String>,
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
        kinds: Vec<TransactionKind>,
        tokens: Vec<String>,
    ) -> Result<Vec<WrapperTransactionDb>, String> {
        let conn = self.app_state.get_db_connection().await;

        conn.interact(move |conn| {
            let mut outer =
                wrapper_transactions::table.into_boxed::<diesel::pg::Pg>();

            // 1) Kind filter using typed enum mapping (apply as its own EXISTS)
            if !kinds.is_empty() {
                fn map_kind(k: &TransactionKind) -> TransactionKindDb {
                    match k {
                        TransactionKind::TransparentTransfer => {
                            TransactionKindDb::TransparentTransfer
                        }
                        TransactionKind::ShieldedTransfer => {
                            TransactionKindDb::ShieldedTransfer
                        }
                        TransactionKind::ShieldingTransfer => {
                            TransactionKindDb::ShieldingTransfer
                        }
                        TransactionKind::UnshieldingTransfer => {
                            TransactionKindDb::UnshieldingTransfer
                        }
                        TransactionKind::MixedTransfer => {
                            TransactionKindDb::MixedTransfer
                        }
                        TransactionKind::Bond => TransactionKindDb::Bond,
                        TransactionKind::Redelegation => {
                            TransactionKindDb::Redelegation
                        }
                        TransactionKind::Unbond => TransactionKindDb::Unbond,
                        TransactionKind::Withdraw => {
                            TransactionKindDb::Withdraw
                        }
                        TransactionKind::ClaimRewards => {
                            TransactionKindDb::ClaimRewards
                        }
                        TransactionKind::VoteProposal => {
                            TransactionKindDb::VoteProposal
                        }
                        TransactionKind::InitProposal => {
                            TransactionKindDb::InitProposal
                        }
                        TransactionKind::ChangeMetadata => {
                            TransactionKindDb::ChangeMetadata
                        }
                        TransactionKind::ChangeCommission => {
                            TransactionKindDb::ChangeCommission
                        }
                        TransactionKind::RevealPk => {
                            TransactionKindDb::RevealPk
                        }
                        TransactionKind::IbcMsgTransfer => {
                            TransactionKindDb::IbcMsgTransfer
                        }
                        TransactionKind::IbcTransparentTransfer => {
                            TransactionKindDb::IbcTransparentTransfer
                        }
                        TransactionKind::IbcShieldingTransfer => {
                            TransactionKindDb::IbcShieldingTransfer
                        }
                        TransactionKind::IbcUnshieldingTransfer => {
                            TransactionKindDb::IbcUnshieldingTransfer
                        }
                        TransactionKind::BecomeValidator => {
                            TransactionKindDb::BecomeValidator
                        }
                        TransactionKind::DeactivateValidator => {
                            TransactionKindDb::DeactivateValidator
                        }
                        TransactionKind::ReactivateValidator => {
                            TransactionKindDb::ReactivateValidator
                        }
                        TransactionKind::UnjailValidator => {
                            TransactionKindDb::UnjailValidator
                        }
                        TransactionKind::ChangeConsensusKey => {
                            TransactionKindDb::ChangeConsensusKey
                        }
                        TransactionKind::InitAccount => {
                            TransactionKindDb::InitAccount
                        }
                        TransactionKind::Unknown => TransactionKindDb::Unknown,
                    }
                }

                let kinds_db: Vec<TransactionKindDb> =
                    kinds.iter().map(map_kind).collect();

                let inner_by_kind = inner_transactions::table
                    .filter(
                        inner_transactions::dsl::wrapper_id
                            .eq(wrapper_transactions::dsl::id),
                    )
                    .filter(inner_transactions::dsl::kind.eq_any(kinds_db));

                outer = outer.filter(exists(inner_by_kind));
            }

            // 2) Token filters via JSON path extraction (apply as its own
            //    EXISTS)
            if !tokens.is_empty() {
                // regular transfer kinds (non-IBC)
                let regular_kinds: Vec<TransactionKindDb> = vec![
                    TransactionKindDb::TransparentTransfer,
                    TransactionKindDb::ShieldedTransfer,
                    TransactionKindDb::ShieldingTransfer,
                    TransactionKindDb::UnshieldingTransfer,
                    TransactionKindDb::MixedTransfer,
                ];

                // IBC transfer kinds
                let ibc_kinds: Vec<TransactionKindDb> = vec![
                    TransactionKindDb::IbcTransparentTransfer,
                    TransactionKindDb::IbcShieldingTransfer,
                    TransactionKindDb::IbcUnshieldingTransfer,
                ];

                // JSON path extracts coerced to non-null text
                let sources_token = sql::<Text>(
                    "COALESCE(inner_transactions.data::jsonb #>> \
                     '{sources,0,token}', '')",
                );
                let targets_token = sql::<Text>(
                    "COALESCE(inner_transactions.data::jsonb #>> \
                     '{targets,0,token}', '')",
                );
                let ibc_account = sql::<Text>(
                    "COALESCE(inner_transactions.data::jsonb #>> \
                     '{0,Ibc,address,Account}', '')",
                );

                // Build `(kind in regular) AND ((sources IN tokens) OR (targets
                // IN tokens))`
                let regular_filter = inner_transactions::dsl::kind
                    .eq_any(regular_kinds.clone())
                    .and(
                        sources_token
                            .eq_any(tokens.clone())
                            .or(targets_token.eq_any(tokens.clone())),
                    );

                // Build `(kind in ibc_kinds) AND (ibc_account IN tokens)`
                let ibc_filter = inner_transactions::dsl::kind
                    .eq_any(ibc_kinds.clone())
                    .and(ibc_account.eq_any(tokens));

                let inner_by_token = inner_transactions::table
                    .filter(
                        inner_transactions::dsl::wrapper_id
                            .eq(wrapper_transactions::dsl::id),
                    )
                    .filter(regular_filter.or(ibc_filter));

                outer = outer.filter(exists(inner_by_token));
            }

            outer
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
