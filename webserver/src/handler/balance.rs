use axum::Json;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum_macros::debug_handler;

use crate::error::api::ApiError;
use crate::response::balance::AddressBalanceResponse;
use crate::response::chain::TokenResponse;
use crate::state::common::CommonState;

#[debug_handler]
pub async fn get_address_balance(
    _headers: HeaderMap,
    Path(address): Path<String>,
    State(state): State<CommonState>,
) -> Result<Json<Vec<AddressBalanceResponse>>, ApiError> {
    let balances = state.balance_service.get_address_balances(address).await?;

    let response = balances
        .into_iter()
        .map(|balance| AddressBalanceResponse {
            token_address: TokenResponse::from(balance.token),
            min_denom_amount: balance.amount.to_string(),
        })
        .collect();

    Ok(Json(response))
}
