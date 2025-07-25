use axum::Json;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum_extra::extract::Query;
use axum_macros::debug_handler;
use bigdecimal::BigDecimal;

use crate::dto::ibc::{
    IbcRateLimit as IbcRateLimitDto, IbcTokenFlow as IbcTokenFlowDto,
};
use crate::error::api::ApiError;
use crate::response::ibc::{
    IbcAckResponse, IbcRateLimitResponse, IbcTokenFlowResponse,
    IbcTokenThroughputResponse,
};
use crate::state::common::CommonState;

#[debug_handler]
pub async fn get_ibc_status(
    _headers: HeaderMap,
    Path(tx_id): Path<String>,
    State(state): State<CommonState>,
) -> Result<Json<IbcAckResponse>, ApiError> {
    let ibc_ack_status = state.ibc_service.get_ack_by_tx_id(tx_id).await?;

    let response = IbcAckResponse::from(ibc_ack_status);

    Ok(Json(response))
}

#[debug_handler]
pub async fn get_ibc_rate_limits(
    Query(query): Query<IbcRateLimitDto>,
    State(state): State<CommonState>,
) -> Result<Json<Vec<IbcRateLimitResponse>>, ApiError> {
    let rate_limits = state
        .ibc_service
        .get_throughput_limits(
            query.token_address,
            query.throughput_limit.map(BigDecimal::from),
        )
        .await?;

    let response = rate_limits
        .into_iter()
        .map(IbcRateLimitResponse::from)
        .collect();

    Ok(Json(response))
}

#[debug_handler]
pub async fn get_ibc_token_flows(
    Query(query): Query<IbcTokenFlowDto>,
    State(state): State<CommonState>,
) -> Result<Json<Vec<IbcTokenFlowResponse>>, ApiError> {
    let token_flows = state
        .ibc_service
        .get_token_flows(query.token_address)
        .await?;

    let response = token_flows
        .into_iter()
        .map(IbcTokenFlowResponse::from)
        .collect();

    Ok(Json(response))
}

#[debug_handler]
pub async fn get_ibc_token_throughput(
    //_headers: HeaderMap,
    Path(token): Path<String>,
    State(state): State<CommonState>,
) -> Result<Json<IbcTokenThroughputResponse>, ApiError> {
    let throughput = state.ibc_service.get_token_throughput(token).await?;

    let response = IbcTokenThroughputResponse::from(throughput);

    Ok(Json(response))
}
