use axum::Json;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum_extra::extract::Query;
use axum_macros::debug_handler;

use crate::dto::pos::{
    AllValidatorsQueryParams, BondsDto, RewardsDto, UnbondsDto,
    ValidatorQueryParams, ValidatorStateDto, WithdrawsDto,
};
use crate::error::api::ApiError;
use crate::response::pos::{
    BondResponse, MergedBondResponse, RewardResponse, TotalVotingPowerResponse,
    UnbondResponse, ValidatorWithRankResponse, WithdrawResponse,
};
use crate::response::utils::PaginatedResponse;
use crate::state::common::CommonState;

#[debug_handler]
pub async fn get_validators(
    _headers: HeaderMap,
    Query(query): Query<ValidatorQueryParams>,
    State(state): State<CommonState>,
) -> Result<Json<PaginatedResponse<Vec<ValidatorWithRankResponse>>>, ApiError> {
    let page = query.page.unwrap_or(1);
    let states = query.state.unwrap_or_else(ValidatorStateDto::all);
    let (validators, total_pages, total_validators) = state
        .pos_service
        .get_validators(page, states, query.sort_field, query.sort_order)
        .await?;

    let response = validators
        .into_iter()
        .map(ValidatorWithRankResponse::from)
        .collect();

    Ok(Json(PaginatedResponse::new(
        response,
        page,
        total_pages,
        total_validators,
    )))
}

#[debug_handler]
pub async fn get_all_validators(
    _headers: HeaderMap,
    Query(query): Query<AllValidatorsQueryParams>,
    State(state): State<CommonState>,
) -> Result<Json<Vec<ValidatorWithRankResponse>>, ApiError> {
    let states = query.state.unwrap_or_else(ValidatorStateDto::all);
    let validators = state.pos_service.get_all_validators(states).await?;

    let response = validators
        .into_iter()
        .map(ValidatorWithRankResponse::from)
        .collect();

    Ok(Json(response))
}

#[debug_handler]
pub async fn get_bonds(
    _headers: HeaderMap,
    query: Query<BondsDto>,
    Path(address): Path<String>,
    State(state): State<CommonState>,
) -> Result<Json<PaginatedResponse<Vec<BondResponse>>>, ApiError> {
    let page = query.page.unwrap_or(1);

    let (bonds, total_pages, total_bonds) = state
        .pos_service
        .get_bonds_by_address(address, page, query.active_at)
        .await?;

    let response = bonds.into_iter().map(BondResponse::from).collect();

    Ok(Json(PaginatedResponse::new(
        response,
        page,
        total_pages,
        total_bonds,
    )))
}

#[debug_handler]
pub async fn get_merged_bonds(
    _headers: HeaderMap,
    query: Query<BondsDto>,
    Path(address): Path<String>,
    State(state): State<CommonState>,
) -> Result<Json<PaginatedResponse<Vec<MergedBondResponse>>>, ApiError> {
    let page = query.page.unwrap_or(1);

    let (bonds, total_pages, total_bonds) = state
        .pos_service
        .get_merged_bonds_by_address(address, page)
        .await?;

    let response = bonds.into_iter().map(MergedBondResponse::from).collect();

    Ok(Json(PaginatedResponse::new(
        response,
        page,
        total_pages,
        total_bonds,
    )))
}

#[debug_handler]
pub async fn get_unbonds(
    _headers: HeaderMap,
    query: Query<UnbondsDto>,
    Path(address): Path<String>,
    State(state): State<CommonState>,
) -> Result<Json<PaginatedResponse<Vec<UnbondResponse>>>, ApiError> {
    let page = query.page.unwrap_or(1);

    let (unbonds, total_pages, total_unbonds) = state
        .pos_service
        .get_unbonds_by_address(address, page, query.active_at)
        .await?;

    let response = unbonds.into_iter().map(UnbondResponse::from).collect();

    Ok(Json(PaginatedResponse::new(
        response,
        page,
        total_pages,
        total_unbonds,
    )))
}

#[debug_handler]
pub async fn get_merged_unbonds(
    _headers: HeaderMap,
    query: Query<UnbondsDto>,
    Path(address): Path<String>,
    State(state): State<CommonState>,
) -> Result<Json<PaginatedResponse<Vec<UnbondResponse>>>, ApiError> {
    let page = query.page.unwrap_or(1);

    let (unbonds, total_pages, total_unbonds) = state
        .pos_service
        .get_merged_unbonds_by_address(address, page)
        .await?;

    let response = unbonds.into_iter().map(UnbondResponse::from).collect();

    Ok(Json(PaginatedResponse::new(
        response,
        page,
        total_pages,
        total_unbonds,
    )))
}

#[debug_handler]
pub async fn get_withdraws(
    _headers: HeaderMap,
    query: Query<WithdrawsDto>,
    Path(address): Path<String>,
    State(state): State<CommonState>,
) -> Result<Json<PaginatedResponse<Vec<WithdrawResponse>>>, ApiError> {
    let page = query.page.unwrap_or(1);

    let (withdraws, total_pages, total_withdraws) = state
        .pos_service
        .get_withdraws_by_address(address, query.epoch, page)
        .await?;

    let response = withdraws.into_iter().map(WithdrawResponse::from).collect();

    Ok(Json(PaginatedResponse::new(
        response,
        page,
        total_pages,
        total_withdraws,
    )))
}

#[debug_handler]
pub async fn get_rewards(
    _headers: HeaderMap,
    query: Query<RewardsDto>,
    Path(address): Path<String>,
    State(state): State<CommonState>,
) -> Result<Json<Vec<RewardResponse>>, ApiError> {
    let rewards = state
        .pos_service
        .get_rewards_by_address(address, query.epoch)
        .await?;
    let response = rewards.into_iter().map(RewardResponse::from).collect();

    Ok(Json(response))
}

#[debug_handler]
pub async fn get_rewards_by_delegator_and_validator_and_epoch(
    _headers: HeaderMap,
    Path((delegator, validator, epoch)): Path<(String, String, u64)>,
    State(state): State<CommonState>,
) -> Result<Json<Vec<RewardResponse>>, ApiError> {
    let rewards = state
        .pos_service
        .get_rewards_by_delegator_and_validator_and_epoch(
            delegator, validator, epoch,
        )
        .await?;

    let response = rewards.into_iter().map(RewardResponse::from).collect();
    Ok(Json(response))
}

#[debug_handler]
pub async fn get_total_voting_power(
    _headers: HeaderMap,
    State(state): State<CommonState>,
) -> Result<Json<TotalVotingPowerResponse>, ApiError> {
    let total_voting_power = state.pos_service.get_total_voting_power().await?;

    Ok(Json(TotalVotingPowerResponse {
        total_voting_power: total_voting_power.to_string(),
    }))
}
