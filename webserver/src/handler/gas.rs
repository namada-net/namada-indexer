use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum_macros::debug_handler;

use crate::dto::gas::GasEstimateQuery;
use crate::error::api::ApiError;
use crate::response::gas::{GasEstimateResponse, GasPriceResponse};
use crate::state::common::CommonState;

#[debug_handler]
pub async fn get_gas_price_by_token(
    _headers: HeaderMap,
    Path(token): Path<String>,
    State(state): State<CommonState>,
) -> Result<Json<Vec<GasPriceResponse>>, ApiError> {
    let gas_prices = state.gas_service.get_gas_price_by_token(token).await?;

    let response = gas_prices
        .into_iter()
        .map(|gas_price| gas_price.into())
        .collect();

    Ok(Json(response))
}

#[debug_handler]
pub async fn get_all_gas_prices(
    _headers: HeaderMap,
    State(state): State<CommonState>,
) -> Result<Json<Vec<GasPriceResponse>>, ApiError> {
    let gas_prices = state.gas_service.get_all_gas_prices().await?;

    let response = gas_prices
        .into_iter()
        .map(|gas_price| gas_price.into())
        .collect();

    Ok(Json(response))
}

#[debug_handler]
pub async fn get_gas_estimate(
    _headers: HeaderMap,
    Query(query): Query<GasEstimateQuery>,
    State(state): State<CommonState>,
) -> Result<Json<GasEstimateResponse>, ApiError> {
    query.is_valid()?;

    let gas = state
        .gas_service
        .estimate_gas(
            query.bond.unwrap_or(0),
            query.redelegate.unwrap_or(0),
            query.claim_rewards.unwrap_or(0),
            query.unbond.unwrap_or(0),
            query.transparent_transfer.unwrap_or(0),
            query.shielded_transfer.unwrap_or(0),
            query.shielding_transfer.unwrap_or(0),
            query.unshielding_transfer.unwrap_or(0),
            query.vote.unwrap_or(0),
            query.ibc_unshielding_transfer.unwrap_or(0),
            query.ibc_shielding_transfer.unwrap_or(0),
            query.ibc_transparent_transfer.unwrap_or(0),
            query.withdraw.unwrap_or(0),
            query.reveal_pk.unwrap_or(0),
            query.signatures.unwrap_or(2),
            query.tx_size.unwrap_or(0),
        )
        .await?;

    let response = GasEstimateResponse {
        min: gas.min,
        max: gas.max,
        avg: gas.avg,
        total_estimates: gas.total_estimates,
    };

    Ok(Json(response))
}
