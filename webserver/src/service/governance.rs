use orm::governance_proposal::{
    GovernanceProposalKindDb, GovernanceProposalResultDb,
};

use crate::appstate::AppState;
use crate::dto::governance::{ProposalKind, ProposalStatus};
use crate::entity::governance::{Proposal, ProposalVote};
use crate::error::governance::GovernanceError;
use crate::repository::chain::{ChainRepository, ChainRepositoryTrait};
use crate::repository::governance::{GovernanceRepo, GovernanceRepoTrait};

#[derive(Clone)]
pub struct GovernanceService {
    governance_repo: GovernanceRepo,
    chain_repo: ChainRepository,
}

impl GovernanceService {
    pub fn new(app_state: AppState) -> Self {
        Self {
            governance_repo: GovernanceRepo::new(app_state.clone()),
            chain_repo: ChainRepository::new(app_state),
        }
    }

    pub async fn find_governance_proposals(
        &self,
        status: Option<ProposalStatus>,
        kind: Option<ProposalKind>,
        pattern: Option<String>,
        page: u64,
    ) -> Result<(Vec<Proposal>, u64, u64), GovernanceError> {
        let kind = self.map_kind(kind);
        let status = self.map_status(status);

        let (db_proposals, total_pages, total_items) = self
            .governance_repo
            .find_governance_proposals(status, kind, pattern, page as i64)
            .await
            .map_err(GovernanceError::Database)?;

        let chain_state = self
            .chain_repo
            .get_state()
            .await
            .map_err(GovernanceError::Database)?;

        let parameters = self
            .chain_repo
            .find_chain_parameters()
            .await
            .map_err(GovernanceError::Database)?;

        Ok((
            db_proposals
                .into_iter()
                .map(|p| {
                    Proposal::from_db(
                        p,
                        &chain_state,
                        parameters.max_block_time,
                        parameters.min_duration,
                    )
                })
                .collect(),
            total_pages as u64,
            total_items as u64,
        ))
    }

    pub async fn find_proposal_data(
        &self,
        proposal_id: u64,
    ) -> Result<Option<Option<String>>, GovernanceError> {
        let db_proposal = self
            .governance_repo
            .find_governance_proposals_by_id(proposal_id as i32)
            .await
            .map_err(GovernanceError::Database)?;

        Ok(db_proposal.map(|proposal| proposal.data))
    }

    pub async fn find_all_governance_proposals(
        &self,
        status: Option<ProposalStatus>,
        kind: Option<ProposalKind>,
        pattern: Option<String>,
    ) -> Result<Vec<Proposal>, GovernanceError> {
        let kind = self.map_kind(kind);
        let status = self.map_status(status);

        let db_proposals = self
            .governance_repo
            .find_all_governance_proposals(status, kind, pattern)
            .await
            .map_err(GovernanceError::Database)?;

        let chain_state = self
            .chain_repo
            .get_state()
            .await
            .map_err(GovernanceError::Database)?;

        let parameters = self
            .chain_repo
            .find_chain_parameters()
            .await
            .map_err(GovernanceError::Database)?;

        Ok(db_proposals
            .into_iter()
            .map(|p| {
                Proposal::from_db(
                    p,
                    &chain_state,
                    parameters.max_block_time,
                    parameters.min_duration,
                )
            })
            .collect())
    }

    pub async fn find_governance_proposal_by_id(
        &self,
        proposal_id: u64,
    ) -> Result<Option<Proposal>, GovernanceError> {
        let db_proposal = self
            .governance_repo
            .find_governance_proposals_by_id(proposal_id as i32)
            .await
            .map_err(GovernanceError::Database)?;

        let chain_state = self
            .chain_repo
            .get_state()
            .await
            .map_err(GovernanceError::Database)?;

        let parameters = self
            .chain_repo
            .find_chain_parameters()
            .await
            .map_err(GovernanceError::Database)?;

        Ok(db_proposal.map(|p| {
            Proposal::from_db(
                p,
                &chain_state,
                parameters.max_block_time,
                parameters.min_duration,
            )
        }))
    }

    pub async fn find_governance_proposal_votes(
        &self,
        proposal_id: u64,
        page: u64,
    ) -> Result<(Vec<ProposalVote>, u64, u64), GovernanceError> {
        let db_proposal = self
            .governance_repo
            .find_governance_proposals_by_id(proposal_id as i32)
            .await
            .map_err(GovernanceError::Database)?;

        if db_proposal.is_none() {
            return Err(GovernanceError::NotFound(proposal_id));
        }

        let (db_proposal_votes, total_pages, total_items) = self
            .governance_repo
            .find_governance_proposal_votes(proposal_id as i32, page as i64)
            .await
            .map_err(GovernanceError::Database)?;

        Ok((
            db_proposal_votes
                .into_iter()
                .map(ProposalVote::from)
                .collect(),
            total_pages as u64,
            total_items as u64,
        ))
    }

    pub async fn find_governance_proposal_votes_by_address(
        &self,
        proposal_id: u64,
        voter_address: String,
    ) -> Result<Vec<ProposalVote>, GovernanceError> {
        let db_proposal = self
            .governance_repo
            .find_governance_proposals_by_id(proposal_id as i32)
            .await
            .map_err(GovernanceError::Database)?;

        if db_proposal.is_none() {
            return Err(GovernanceError::NotFound(proposal_id));
        }

        let db_proposal_votes = self
            .governance_repo
            .find_governance_proposal_votes_by_address(
                proposal_id as i32,
                voter_address,
            )
            .await
            .map_err(GovernanceError::Database)?;

        Ok(db_proposal_votes
            .into_iter()
            .map(ProposalVote::from)
            .collect())
    }

    pub async fn find_governance_proposal_votes_by_voter(
        &self,
        voter_address: String,
    ) -> Result<Vec<ProposalVote>, GovernanceError> {
        let db_proposal_votes = self
            .governance_repo
            .find_governance_proposal_votes_by_voter(voter_address)
            .await
            .map_err(GovernanceError::Database)?;

        Ok(db_proposal_votes
            .into_iter()
            .map(ProposalVote::from)
            .collect())
    }

    fn map_status(
        &self,
        status: Option<ProposalStatus>,
    ) -> Option<GovernanceProposalResultDb> {
        status.map(|s| match s {
            ProposalStatus::Pending => GovernanceProposalResultDb::Pending,
            ProposalStatus::VotingPeriod => {
                GovernanceProposalResultDb::VotingPeriod
            }
            ProposalStatus::Passed => GovernanceProposalResultDb::Passed,
            ProposalStatus::Rejected => GovernanceProposalResultDb::Rejected,
            ProposalStatus::ExecutedPassed => {
                GovernanceProposalResultDb::ExecutedPassed
            }
            ProposalStatus::ExecutedRejected => {
                GovernanceProposalResultDb::ExecutedRejected
            }
        })
    }

    fn map_kind(
        &self,
        kind: Option<ProposalKind>,
    ) -> Option<GovernanceProposalKindDb> {
        kind.map(|t| match t {
            ProposalKind::Default => GovernanceProposalKindDb::Default,
            ProposalKind::DefaultWithWasm => {
                GovernanceProposalKindDb::DefaultWithWasm
            }
            ProposalKind::PgfSteward => GovernanceProposalKindDb::PgfSteward,
            ProposalKind::PgfFunding => GovernanceProposalKindDb::PgfFunding,
        })
    }
}
