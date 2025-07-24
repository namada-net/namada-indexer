use axum::async_trait;
use diesel::dsl::IntoBoxed;
use diesel::pg::Pg;
use diesel::{
    BoolExpressionMethods, ExpressionMethods, PgTextExpressionMethods,
    QueryDsl, RunQueryDsl, SelectableHelper,
};
use orm::governance_proposal::{
    GovernanceProposalDb, GovernanceProposalKindDb, GovernanceProposalResultDb,
};
use orm::governance_votes::GovernanceProposalVoteDb;
use orm::schema::{governance_proposals, governance_votes};

use crate::appstate::AppState;
use crate::repository::utils::{Paginate, PaginatedResponseDb};

#[derive(Clone)]
pub struct GovernanceRepo {
    pub(crate) app_state: AppState,
}

#[async_trait]
pub trait GovernanceRepoTrait {
    fn new(app_state: AppState) -> Self;

    async fn find_governance_proposals(
        &self,
        status: Option<GovernanceProposalResultDb>,
        kind: Option<GovernanceProposalKindDb>,
        pattern: Option<String>,
        page: i64,
    ) -> Result<PaginatedResponseDb<GovernanceProposalDb>, String>;

    async fn find_all_governance_proposals(
        &self,
        status: Option<GovernanceProposalResultDb>,
        kind: Option<GovernanceProposalKindDb>,
        pattern: Option<String>,
    ) -> Result<Vec<GovernanceProposalDb>, String>;

    async fn find_governance_proposals_by_id(
        &self,
        proposal_id: i32,
    ) -> Result<Option<GovernanceProposalDb>, String>;

    async fn find_governance_proposal_votes(
        &self,
        proposal_id: i32,
        page: i64,
    ) -> Result<PaginatedResponseDb<GovernanceProposalVoteDb>, String>;

    async fn find_governance_proposal_votes_by_address(
        &self,
        proposal_id: i32,
        voter_address: String,
    ) -> Result<Vec<GovernanceProposalVoteDb>, String>;

    async fn find_governance_proposal_votes_by_voter(
        &self,
        voter_address: String,
    ) -> Result<Vec<GovernanceProposalVoteDb>, String>;
}

#[async_trait]
impl GovernanceRepoTrait for GovernanceRepo {
    fn new(app_state: AppState) -> Self {
        Self { app_state }
    }

    async fn find_governance_proposals(
        &self,
        status: Option<GovernanceProposalResultDb>,
        kind: Option<GovernanceProposalKindDb>,
        pattern: Option<String>,
        page: i64,
    ) -> Result<PaginatedResponseDb<GovernanceProposalDb>, String> {
        let conn = self.app_state.get_db_connection().await;
        let query = self.governance_proposals(status, kind, pattern);

        conn.interact(move |conn| {
            query
                .select(GovernanceProposalDb::as_select())
                .order(governance_proposals::dsl::id.desc())
                .paginate(page)
                .load_and_count_pages(conn)
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
    }

    async fn find_all_governance_proposals(
        &self,
        status: Option<GovernanceProposalResultDb>,
        kind: Option<GovernanceProposalKindDb>,
        pattern: Option<String>,
    ) -> Result<Vec<GovernanceProposalDb>, String> {
        let conn = self.app_state.get_db_connection().await;
        let query = self.governance_proposals(status, kind, pattern);

        conn.interact(move |conn| {
            query
                .select(GovernanceProposalDb::as_select())
                .order(governance_proposals::dsl::id.desc())
                .load(conn)
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
    }

    async fn find_governance_proposals_by_id(
        &self,
        proposal_id: i32,
    ) -> Result<Option<GovernanceProposalDb>, String> {
        let conn = self.app_state.get_db_connection().await;

        conn.interact(move |conn| {
            governance_proposals::table
                .find(proposal_id)
                .select(GovernanceProposalDb::as_select())
                .first(conn)
                .ok()
        })
        .await
        .map_err(|e| e.to_string())
    }

    async fn find_governance_proposal_votes(
        &self,
        proposal_id: i32,
        page: i64,
    ) -> Result<PaginatedResponseDb<GovernanceProposalVoteDb>, String> {
        let conn = self.app_state.get_db_connection().await;

        conn.interact(move |conn| {
            governance_votes::table
                .filter(governance_votes::dsl::proposal_id.eq(proposal_id))
                .select(GovernanceProposalVoteDb::as_select())
                .paginate(page)
                .load_and_count_pages(conn)
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
    }

    async fn find_governance_proposal_votes_by_address(
        &self,
        proposal_id: i32,
        voter_address: String,
    ) -> Result<Vec<GovernanceProposalVoteDb>, String> {
        let conn = self.app_state.get_db_connection().await;

        conn.interact(move |conn| {
            governance_votes::table
                .filter(governance_votes::dsl::proposal_id.eq(proposal_id).and(
                    governance_votes::dsl::voter_address.eq(voter_address),
                ))
                .select(GovernanceProposalVoteDb::as_select())
                .get_results(conn)
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
    }

    async fn find_governance_proposal_votes_by_voter(
        &self,
        voter_address: String,
    ) -> Result<Vec<GovernanceProposalVoteDb>, String> {
        let conn = self.app_state.get_db_connection().await;

        conn.interact(move |conn| {
            governance_votes::table
                .filter(governance_votes::dsl::voter_address.eq(voter_address))
                .select(GovernanceProposalVoteDb::as_select())
                .get_results(conn)
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
    }
}

#[allow(clippy::needless_lifetimes)]
impl<'a> GovernanceRepo {
    fn governance_proposals(
        &self,
        status: Option<GovernanceProposalResultDb>,
        kind: Option<GovernanceProposalKindDb>,
        pattern: Option<String>,
    ) -> IntoBoxed<'a, governance_proposals::table, Pg> {
        let mut query = governance_proposals::table.into_boxed();

        if let Some(status) = status {
            query = query
                .filter(governance_proposals::dsl::result.eq(status.clone()))
        }

        if let Some(kind) = kind {
            query = query.filter(governance_proposals::dsl::kind.eq(kind));
        }

        if let Some(pattern) = pattern {
            query = query.filter(
                governance_proposals::dsl::content
                    .ilike(format!("%{}%", pattern)),
            );
        }

        query
    }
}
