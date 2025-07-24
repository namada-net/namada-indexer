use axum::Json;
use axum::extract::State;
use axum::http::HeaderMap;
use axum_extra::extract::Query;
use strum::VariantArray;

use crate::dto::crawler_state::{CrawlerNameDto, CrawlerStateQueryParams};
use crate::error::api::ApiError;
use crate::response::crawler_state::CrawlersTimestampsResponse;
use crate::state::common::CommonState;

pub async fn get_crawlers_timestamps(
    _headers: HeaderMap,
    Query(query): Query<CrawlerStateQueryParams>,
    State(state): State<CommonState>,
) -> Result<Json<Vec<CrawlersTimestampsResponse>>, ApiError> {
    let crawler_names = query.crawler_names.unwrap_or(vec![]);

    let timestamps = state
        .crawler_state_service
        .get_timestamps(crawler_names.clone())
        .await?;

    let variants = if crawler_names.is_empty() {
        CrawlerNameDto::VARIANTS.to_vec()
    } else {
        CrawlerNameDto::VARIANTS
            .iter()
            .filter(|variant| crawler_names.contains(variant))
            .cloned()
            .collect::<Vec<_>>()
    };

    let timestamps_with_defaults = variants
        .into_iter()
        .map(|variant| {
            timestamps
                .clone()
                .into_iter()
                .find(|timestamp| timestamp.name == variant.to_string())
                .map_or_else(
                    || CrawlersTimestampsResponse {
                        name: variant.to_string(),
                        timestamp: 0,
                        last_processed_block_height: None,
                    },
                    CrawlersTimestampsResponse::from,
                )
        })
        .collect::<Vec<_>>();

    Ok(Json(timestamps_with_defaults))
}
