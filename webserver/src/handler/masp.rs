use axum::Json;
use axum::extract::State;
use axum::http::HeaderMap;
use axum_extra::extract::Query;
use axum_macros::debug_handler;

use crate::dto::masp::MaspAggregatesQueryParams;
use crate::error::api::ApiError;
use crate::response::masp::MaspPoolAggregateResponse;
use crate::state::common::CommonState;

#[debug_handler]
pub async fn get_masp_aggregates(
    _headers: HeaderMap,
    State(state): State<CommonState>,
    Query(query): Query<MaspAggregatesQueryParams>,
) -> Result<Json<Vec<MaspPoolAggregateResponse>>, ApiError> {
    let masp_aggregates = state
        .masp_service
        .find_all_masp_aggregates(query.token)
        .await?;

    let response = masp_aggregates
        .into_iter()
        .map(MaspPoolAggregateResponse::from)
        .collect();

    Ok(Json(response))
}
