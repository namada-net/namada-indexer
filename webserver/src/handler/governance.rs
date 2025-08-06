use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum_extra::extract::Query as QueryExt;
use axum_macros::debug_handler;

use crate::dto::governance::{ProposalQueryParams, ProposalVotesQueryparams};
use crate::error::api::ApiError;
use crate::error::governance::GovernanceError;
use crate::response::governance::{Proposal, ProposalVote};
use crate::response::utils::PaginatedResponse;
use crate::state::common::CommonState;

#[debug_handler]
pub async fn get_governance_proposals(
    _headers: HeaderMap,
    QueryExt(query): QueryExt<ProposalQueryParams>,
    State(state): State<CommonState>,
) -> Result<Json<PaginatedResponse<Vec<Proposal>>>, ApiError> {
    let page = query.page.unwrap_or(1);
    let (proposals, total_pages, total_items) = state
        .gov_service
        .find_governance_proposals(
            query.status,
            query.kind,
            query.pattern,
            page,
        )
        .await?;

    let response =
        PaginatedResponse::new(proposals, page, total_pages, total_items);
    Ok(Json(response))
}

#[debug_handler]
pub async fn get_all_governance_proposals(
    _headers: HeaderMap,
    QueryExt(query): QueryExt<ProposalQueryParams>,
    State(state): State<CommonState>,
) -> Result<Json<Vec<Proposal>>, ApiError> {
    let proposals = state
        .gov_service
        .find_all_governance_proposals(query.status, query.kind, query.pattern)
        .await?;

    Ok(Json(proposals))
}

#[debug_handler]
pub async fn get_governance_proposal_by_id(
    _headers: HeaderMap,
    Path(proposal_id): Path<u64>,
    State(state): State<CommonState>,
) -> Result<Json<Proposal>, ApiError> {
    let proposal = state
        .gov_service
        .find_governance_proposal_by_id(proposal_id)
        .await?;

    if let Some(proposal) = proposal {
        Ok(Json(proposal))
    } else {
        Err(GovernanceError::NotFound(proposal_id).into())
    }
}

#[debug_handler]
pub async fn get_proposal_data_by_proposal_id(
    _headers: HeaderMap,
    Path(proposal_id): Path<u64>,
    State(state): State<CommonState>,
) -> Result<String, ApiError> {
    let proposal = state.gov_service.find_proposal_data(proposal_id).await?;

    if let Some(data) = proposal {
        if let Some(data) = data {
            Ok(data)
        } else {
            Err(GovernanceError::DataNotFound(proposal_id).into())
        }
    } else {
        Err(GovernanceError::NotFound(proposal_id).into())
    }
}

#[debug_handler]
pub async fn get_governance_proposal_votes(
    _headers: HeaderMap,
    Path(proposal_id): Path<u64>,
    Query(query): Query<ProposalVotesQueryparams>,
    State(state): State<CommonState>,
) -> Result<Json<PaginatedResponse<Vec<ProposalVote>>>, ApiError> {
    let page = query.page.unwrap_or(1);
    let (proposal_votes, total_pages, total_votes) = state
        .gov_service
        .find_governance_proposal_votes(proposal_id, page)
        .await?;

    Ok(Json(PaginatedResponse::new(
        proposal_votes,
        page,
        total_pages,
        total_votes,
    )))
}

#[debug_handler]
pub async fn get_governance_proposal_votes_by_address(
    _headers: HeaderMap,
    Path((proposal_id, address)): Path<(u64, String)>,
    State(state): State<CommonState>,
) -> Result<Json<Vec<ProposalVote>>, ApiError> {
    let proposal_votes = state
        .gov_service
        .find_governance_proposal_votes_by_address(proposal_id, address)
        .await?;

    Ok(Json(proposal_votes))
}

#[debug_handler]
pub async fn get_governance_proposal_votes_by_voter(
    _headers: HeaderMap,
    Path(address): Path<String>,
    State(state): State<CommonState>,
) -> Result<Json<Vec<ProposalVote>>, ApiError> {
    let proposal_votes = state
        .gov_service
        .find_governance_proposal_votes_by_voter(address)
        .await?;

    Ok(Json(proposal_votes))
}
