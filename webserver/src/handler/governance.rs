use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum_macros::debug_handler;

use crate::dto::governance::{ProposalQueryParams, ProposalVotesQueryparams};
use crate::error::api::ApiError;
use crate::error::governance::GovernanceError;
use crate::response::governance::{ProposalResponse, ProposalVoteResponse};
use crate::response::utils::PaginatedResponse;
use crate::state::common::CommonState;

#[debug_handler]
pub async fn get_governance_proposals(
    _headers: HeaderMap,
    Query(query): Query<ProposalQueryParams>,
    State(state): State<CommonState>,
) -> Result<Json<PaginatedResponse<Vec<ProposalResponse>>>, ApiError> {
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

    let response = proposals
        .into_iter()
        .map(ProposalResponse::from)
        .collect::<Vec<_>>();

    Ok(Json(PaginatedResponse::new(
        response,
        page,
        total_pages,
        total_items,
    )))
}

#[debug_handler]
pub async fn get_governance_proposal_by_id(
    _headers: HeaderMap,
    Path(proposal_id): Path<u64>,
    State(state): State<CommonState>,
) -> Result<Json<ProposalResponse>, ApiError> {
    let proposal = state
        .gov_service
        .find_governance_proposal_by_id(proposal_id)
        .await?;

    if let Some(proposal) = proposal {
        Ok(Json(ProposalResponse::from(proposal)))
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
) -> Result<Json<PaginatedResponse<Vec<ProposalVoteResponse>>>, ApiError> {
    let page = query.page.unwrap_or(1);
    let (proposal_votes, total_pages, total_votes) = state
        .gov_service
        .find_governance_proposal_votes(proposal_id, page)
        .await?;

    let response = proposal_votes
        .into_iter()
        .map(ProposalVoteResponse::from)
        .collect::<Vec<_>>();

    Ok(Json(PaginatedResponse::new(
        response,
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
) -> Result<Json<Vec<ProposalVoteResponse>>, ApiError> {
    let proposal_votes = state
        .gov_service
        .find_governance_proposal_votes_by_address(proposal_id, address)
        .await?;

    let response = proposal_votes
        .into_iter()
        .map(ProposalVoteResponse::from)
        .collect::<Vec<_>>();

    Ok(Json(response))
}

#[debug_handler]
pub async fn get_governance_proposal_votes_by_voter(
    _headers: HeaderMap,
    Path(address): Path<String>,
    State(state): State<CommonState>,
) -> Result<Json<Vec<ProposalVoteResponse>>, ApiError> {
    let proposal_votes = state
        .gov_service
        .find_governance_proposal_votes_by_voter(address)
        .await?;

    let response = proposal_votes
        .into_iter()
        .map(ProposalVoteResponse::from)
        .collect::<Vec<_>>();

    Ok(Json(response))
}
