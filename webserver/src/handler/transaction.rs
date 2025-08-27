use axum::Json;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum_extra::extract::Query;
use axum_macros::debug_handler;

use crate::dto::transaction::{
    RecentInnerQueryParams, TransactionHistoryQueryParams,
};
use crate::error::api::ApiError;
use crate::error::transaction::TransactionError;
use crate::response::transaction::{
    InnerTransaction, InnerTransactionWithHeight, TransactionHistory, WrapperTransaction,
};
use crate::response::utils::PaginatedResponse;
use crate::state::common::CommonState;

#[debug_handler]
pub async fn get_wrapper_tx(
    _headers: HeaderMap,
    Path(tx_id): Path<String>,
    State(state): State<CommonState>,
) -> Result<Json<Option<WrapperTransaction>>, ApiError> {
    is_valid_hash(&tx_id)?;
    let tx_id = tx_id.to_lowercase();

    let wrapper_tx = state
        .transaction_service
        .get_wrapper_tx(tx_id.clone())
        .await?;

    if wrapper_tx.is_none() {
        return Err(TransactionError::TxIdNotFound(tx_id).into());
    }

    let inner_txs = state
        .transaction_service
        .get_inner_tx_by_wrapper_id(tx_id)
        .await?;

    Ok(Json(wrapper_tx.map(|mut wrapper| {
        wrapper.inner_transactions =
            inner_txs.into_iter().map(|tx| tx.to_short()).collect();
        wrapper
    })))
}

#[debug_handler]
pub async fn get_inner_tx(
    _headers: HeaderMap,
    Path(tx_id): Path<String>,
    State(state): State<CommonState>,
) -> Result<Json<Option<InnerTransaction>>, ApiError> {
    is_valid_hash(&tx_id)?;
    let tx_id = tx_id.to_lowercase();

    let inner_tx = state
        .transaction_service
        .get_inner_tx(tx_id.clone())
        .await?;

    if inner_tx.is_none() {
        return Err(TransactionError::TxIdNotFound(tx_id).into());
    }

    Ok(Json(inner_tx))
}

#[debug_handler]
pub async fn get_transaction_history(
    _headers: HeaderMap,
    Query(query): Query<TransactionHistoryQueryParams>,
    State(state): State<CommonState>,
) -> Result<Json<PaginatedResponse<Vec<TransactionHistory>>>, ApiError> {
    let page = query.page.unwrap_or(1);

    let (transactions, total_pages, total_items) = state
        .transaction_service
        .get_addresses_history(query.addresses, page)
        .await?;

    let response =
        PaginatedResponse::new(transactions, page, total_pages, total_items);

    Ok(Json(response))
}

#[debug_handler]
pub async fn get_recent_inner(
    _headers: HeaderMap,
    Query(query): Query<RecentInnerQueryParams>,
    State(state): State<CommonState>,
) -> Result<Json<PaginatedResponse<Vec<InnerTransactionWithHeight>>>, ApiError> {
    let page = query.page.unwrap_or(1);

    let (transactions, total_pages, total_items) = state
        .transaction_service
        .get_recent_inner(query.limit, page, query.kind, query.token)
        .await?;

    let response =
        PaginatedResponse::new(transactions, page, total_pages, total_items);

    Ok(Json(response))
}

fn is_valid_hash(hash: &str) -> Result<(), TransactionError> {
    if hash.len().eq(&64) {
        Ok(())
    } else {
        Err(TransactionError::InvalidTxId)
    }
}
