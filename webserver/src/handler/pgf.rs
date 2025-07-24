use axum::Json;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum_extra::extract::Query;
use axum_macros::debug_handler;

use crate::dto::pgf::PgfQueryParams;
use crate::error::api::ApiError;
use crate::response::pgf::PgfPaymentResponse;
use crate::response::utils::PaginatedResponse;
use crate::state::common::CommonState;

#[debug_handler]
pub async fn get_pgf_continuous_payments(
    _headers: HeaderMap,
    Query(query): Query<PgfQueryParams>,
    State(state): State<CommonState>,
) -> Result<Json<PaginatedResponse<Vec<PgfPaymentResponse>>>, ApiError> {
    let page = query.page.unwrap_or(1);

    let (pgf_payments, total_pages, total_items) =
        state.pgf_service.get_all_pgf_payments(page).await?;

    let response = pgf_payments
        .into_iter()
        .map(|payment| payment.into())
        .collect();

    Ok(Json(PaginatedResponse::new(
        response,
        page,
        total_pages,
        total_items,
    )))
}

#[debug_handler]
pub async fn get_pgf_payment_by_proposal_id(
    _headers: HeaderMap,
    Path(proposal_id): Path<u64>,
    State(state): State<CommonState>,
) -> Result<Json<Option<PgfPaymentResponse>>, ApiError> {
    let pgf_payment = state
        .pgf_service
        .find_pfg_payment_by_proposal_id(proposal_id)
        .await?;

    let response = pgf_payment.map(|payment| payment.into());

    Ok(Json(response))
}
