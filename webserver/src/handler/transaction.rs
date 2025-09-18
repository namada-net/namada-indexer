use axum::Json;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum_extra::extract::Query;
use axum_macros::debug_handler;
use serde_json;

use crate::dto::transaction::{
    TransactionHistoryQueryParams, TransactionIdParam,
    TransactionMostRecentQueryParams,
};
use crate::entity::transaction::{InnerTransaction, TransactionKind};
use crate::error::api::ApiError;
use crate::error::transaction::TransactionError;
use crate::response::transaction::{
    InnerTransactionResponse, TransactionHistoryResponse,
    WrapperTransactionResponse,
};
use crate::response::utils::PaginatedResponse;
use crate::state::common::CommonState;

#[debug_handler]
pub async fn get_wrapper_tx(
    _headers: HeaderMap,
    Path(tx_id): Path<TransactionIdParam>,
    State(state): State<CommonState>,
) -> Result<Json<Option<WrapperTransactionResponse>>, ApiError> {
    tx_id.is_valid_hash()?;

    let tx_id = tx_id.get();

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

    let response = wrapper_tx
        .map(|wrapper| WrapperTransactionResponse::new(wrapper, inner_txs));

    Ok(Json(response))
}

#[debug_handler]
pub async fn get_inner_tx(
    _headers: HeaderMap,
    Path(tx_id): Path<TransactionIdParam>,
    State(state): State<CommonState>,
) -> Result<Json<Option<InnerTransactionResponse>>, ApiError> {
    tx_id.is_valid_hash()?;

    let tx_id = tx_id.get();

    let inner_tx = state
        .transaction_service
        .get_inner_tx(tx_id.clone())
        .await?;

    let response = inner_tx.map(InnerTransactionResponse::new);

    Ok(Json(response))
}

#[debug_handler]
pub async fn get_transaction_history(
    _headers: HeaderMap,
    Query(query): Query<TransactionHistoryQueryParams>,
    State(state): State<CommonState>,
) -> Result<Json<PaginatedResponse<Vec<TransactionHistoryResponse>>>, ApiError>
{
    let page = query.page.unwrap_or(1);

    let (transactions, total_pages, total_items) = state
        .transaction_service
        .get_addresses_history(query.addresses, page)
        .await?;

    let response = transactions
        .into_iter()
        .map(TransactionHistoryResponse::from)
        .collect();

    Ok(Json(PaginatedResponse::new(
        response,
        page,
        total_pages,
        total_items,
    )))
}

#[debug_handler]
pub async fn get_most_recent_transactions(
    _headers: HeaderMap,
    Query(query): Query<TransactionMostRecentQueryParams>,
    State(state): State<CommonState>,
) -> Result<Json<Vec<WrapperTransactionResponse>>, ApiError> {
    let offset = query.offset.unwrap_or(0);
    let size = query.size.unwrap_or(10);
    let kind = query.kind;
    let token = query.token;

    let filters_present = !kind.is_empty() || !token.is_empty();

    if !filters_present {
        // If no filters are needed, we can just page over the wrapper tx table
        // directly
        let transactions = state
            .transaction_service
            .get_most_recent_transactions(offset, size)
            .await?;

        let inner_txs = transactions
            .iter()
            .map(|tx| {
                state
                    .transaction_service
                    .get_inner_tx_by_wrapper_id(tx.id.to_string())
            })
            .collect::<Vec<_>>();

        let inner_txs = futures::future::join_all(inner_txs).await;

        let response = transactions
            .into_iter()
            .zip(inner_txs.into_iter())
            .map(|(tx, inner_tx_result)| {
                let inner_txs = inner_tx_result.unwrap_or_default();
                WrapperTransactionResponse::new(tx, inner_txs)
            })
            .collect();

        return Ok(Json(response));
    }

    // Filtering by tx kind and/or transfer token:
    // First, search for and return 'size' number of wrappers that have at least
    // one inner tx matching the filters
    let candidates = state
        .transaction_service
        .get_filtered_most_recent_wrappers(
            offset,
            size,
            kind.clone(),
            token.clone(),
        )
        .await?;

    // Filter the corresponding inner txs to include only the matching inners in
    // the response
    let inner_futs = candidates
        .iter()
        .map(|tx| {
            state
                .transaction_service
                .get_inner_tx_by_wrapper_id(tx.id.to_string())
        })
        .collect::<Vec<_>>();

    let inner_results = futures::future::join_all(inner_futs).await;

    let response = candidates
        .into_iter()
        .zip(inner_results.into_iter())
        .filter_map(|(tx, inner_res)| {
            let mut inners = inner_res.unwrap_or_default();

            if !kind.is_empty() {
                inners.retain(|inner_tx| kind.contains(&inner_tx.kind));
            }
            if !token.is_empty() {
                inners.retain(|inner_tx| {
                    filter_inner_tx_by_tokens(inner_tx, &token)
                });
            }

            if inners.is_empty() {
                return None;
            }

            Some(WrapperTransactionResponse::new(tx, inners))
        })
        .collect::<Vec<_>>();

    Ok(Json(response))
}

/// Filter a single inner transaction by token addresses involved (only
/// applicable to transfer types)
fn filter_inner_tx_by_tokens(
    inner_tx: &InnerTransaction,
    token_filter: &[String],
) -> bool {
    if token_filter.is_empty() {
        return true;
    }

    let Some(data) = &inner_tx.data else {
        return false;
    };

    let Ok(json_value) = serde_json::from_str::<serde_json::Value>(data) else {
        return false;
    };

    // Check if a candidate token equals any of the filter tokens
    let token_matches = |candidate: &str| -> bool {
        token_filter
            .iter()
            .any(|filter_token| candidate.eq_ignore_ascii_case(filter_token))
    };

    // IBC transfers: data is an array; get token from [0].Ibc.address.Account
    let is_ibc_kind = matches!(
        inner_tx.kind,
        TransactionKind::IbcTransparentTransfer
            | TransactionKind::IbcShieldingTransfer
            | TransactionKind::IbcUnshieldingTransfer
    );

    if is_ibc_kind {
        if let Some(arr) = json_value.as_array() {
            for item in arr {
                if let Some(ibc_obj) = item.get("Ibc") {
                    if let Some(account) = ibc_obj
                        .get("address")
                        .and_then(|a| a.get("Account"))
                        .and_then(|a| a.as_str())
                    {
                        if token_matches(account) {
                            return true;
                        }
                    }
                }
            }
        }

        // If structure differs unexpectedly, do not match
        return false;
    }

    // Non-IBC transfers: check "sources" array containing entries with a
    // "token" field
    if let Some(obj) = json_value.as_object() {
        if let Some(sources) = obj.get("sources").and_then(|v| v.as_array()) {
            for src in sources {
                if let Some(token_str) = src
                    .as_object()
                    .and_then(|o| o.get("token"))
                    .and_then(|t| t.as_str())
                {
                    if token_matches(token_str) {
                        return true;
                    }
                }
            }
        }
    }

    false
}
