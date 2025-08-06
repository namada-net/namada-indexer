use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProposalStatus {
    Pending,
    VotingPeriod,
    Passed,
    Rejected,
    ExecutedPassed,
    ExecutedRejected,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProposalKind {
    Default,
    DefaultWithWasm,
    PgfSteward,
    PgfFunding,
}

#[derive(Clone, Serialize, Deserialize, Validate)]
pub struct ProposalQueryParams {
    #[validate(range(min = 1, max = 10000))]
    pub page: Option<u64>,
    pub status: Option<Vec<ProposalStatus>>,
    pub kind: Option<ProposalKind>,
    pub pattern: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, Validate)]
pub struct ProposalVotesQueryparams {
    #[validate(range(min = 1, max = 10000))]
    pub page: Option<u64>,
}
