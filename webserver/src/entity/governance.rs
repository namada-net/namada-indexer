use orm::crawler_state::ChainCrawlerStateDb;
use orm::governance_proposal::{
    GovernanceProposalDb, GovernanceProposalKindDb, GovernanceProposalResultDb,
    GovernanceProposalTallyTypeDb,
};
use orm::governance_votes::{GovernanceProposalVoteDb, GovernanceVoteKindDb};
use shared::id::Id;

use crate::response::utils::{epoch_progress, time_between_epochs};

#[derive(Clone, Debug)]
pub enum ProposalType {
    Default,
    DefaultWithWasm,
    PgfSteward,
    PgfFunding,
}

impl From<GovernanceProposalKindDb> for ProposalType {
    fn from(value: GovernanceProposalKindDb) -> Self {
        match value {
            GovernanceProposalKindDb::PgfSteward => Self::PgfSteward,
            GovernanceProposalKindDb::PgfFunding => Self::PgfFunding,
            GovernanceProposalKindDb::Default => Self::Default,
            GovernanceProposalKindDb::DefaultWithWasm => Self::DefaultWithWasm,
        }
    }
}

#[derive(Clone, Debug)]
pub enum TallyType {
    TwoFifths,
    OneHalfOverOneThird,
    LessOneHalfOverOneThirdNay,
}

impl From<GovernanceProposalTallyTypeDb> for TallyType {
    fn from(value: GovernanceProposalTallyTypeDb) -> Self {
        match value {
            GovernanceProposalTallyTypeDb::TwoFifths => Self::TwoFifths,
            GovernanceProposalTallyTypeDb::OneHalfOverOneThird => {
                Self::OneHalfOverOneThird
            }
            GovernanceProposalTallyTypeDb::LessOneHalfOverOneThirdNay => {
                Self::LessOneHalfOverOneThirdNay
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum ProposalStatus {
    Pending,
    Rejected,
    Passed,
    Voting,
    ExecutedPassed,
    ExecutedRejected,
    Unknown,
}

impl From<GovernanceProposalResultDb> for ProposalStatus {
    fn from(value: GovernanceProposalResultDb) -> Self {
        match value {
            GovernanceProposalResultDb::Passed => Self::Passed,
            GovernanceProposalResultDb::Rejected => Self::Rejected,
            GovernanceProposalResultDb::Pending => Self::Pending,
            GovernanceProposalResultDb::Unknown => Self::Unknown,
            GovernanceProposalResultDb::VotingPeriod => Self::Voting,
            GovernanceProposalResultDb::ExecutedPassed => Self::ExecutedPassed,
            GovernanceProposalResultDb::ExecutedRejected => {
                Self::ExecutedRejected
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Proposal {
    pub id: u64,
    pub content: String,
    pub r#type: ProposalType,
    pub tally_type: TallyType,
    pub data: Option<String>,
    pub author: Id,
    pub start_epoch: u64,
    pub end_epoch: u64,
    pub activation_epoch: u64,
    pub start_time: String,
    pub end_time: String,
    pub current_time: String,
    pub activation_time: String,
    pub status: ProposalStatus,
    pub yay_votes: f64,
    pub nay_votes: f64,
    pub abstain_votes: f64,
}

#[derive(Clone, Debug)]
pub enum VoteType {
    Yay,
    Nay,
    Abstain,
    Unknown,
}

impl From<GovernanceVoteKindDb> for VoteType {
    fn from(value: GovernanceVoteKindDb) -> Self {
        match value {
            GovernanceVoteKindDb::Nay => Self::Nay,
            GovernanceVoteKindDb::Yay => Self::Yay,
            GovernanceVoteKindDb::Abstain => Self::Abstain,
            GovernanceVoteKindDb::Unknown => Self::Unknown,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ProposalVote {
    pub proposal_id: u64,
    pub vote: VoteType,
    pub voter_address: Id,
}

impl Proposal {
    pub fn from_db(
        value: GovernanceProposalDb,
        chain_state: &ChainCrawlerStateDb,
        max_block_time: i32,
        min_duration: i32,
    ) -> Self {
        let blocks_per_epoch = min_duration / max_block_time;

        let epoch_progress = epoch_progress(
            chain_state.last_processed_block,
            chain_state.first_block_in_epoch,
            blocks_per_epoch,
        );

        let to_start = time_between_epochs(
            blocks_per_epoch,
            epoch_progress,
            chain_state.last_processed_epoch,
            value.start_epoch,
            min_duration,
        );

        let to_end = time_between_epochs(
            blocks_per_epoch,
            epoch_progress,
            chain_state.last_processed_epoch,
            value.end_epoch,
            min_duration,
        );

        let to_activation = time_between_epochs(
            blocks_per_epoch,
            epoch_progress,
            chain_state.last_processed_epoch,
            value.activation_epoch,
            min_duration,
        );

        let time_now = chain_state.timestamp.and_utc().timestamp();
        let start_time = time_now + i64::from(to_start);
        let end_time = time_now + i64::from(to_end);
        let activation_time = time_now + i64::from(to_activation);

        Self {
            id: value.id as u64,
            content: value.content,
            r#type: ProposalType::from(value.kind),
            tally_type: TallyType::from(value.tally_type),
            data: value.data,
            author: Id::Account(value.author),
            start_epoch: value.start_epoch as u64,
            end_epoch: value.end_epoch as u64,
            activation_epoch: value.activation_epoch as u64,
            start_time: start_time.to_string(),
            end_time: end_time.to_string(),
            current_time: time_now.to_string(),
            activation_time: activation_time.to_string(),
            status: ProposalStatus::from(value.result),
            yay_votes: value
                .yay_votes
                .parse::<f64>()
                .expect("Should be a number"),
            nay_votes: value
                .nay_votes
                .parse::<f64>()
                .expect("Should be a number"),
            abstain_votes: value
                .abstain_votes
                .parse::<f64>()
                .expect("Should be a number"),
        }
    }
}

impl From<GovernanceProposalVoteDb> for ProposalVote {
    fn from(value: GovernanceProposalVoteDb) -> Self {
        Self {
            proposal_id: value.proposal_id as u64,
            vote: VoteType::from(value.kind),
            voter_address: Id::Account(value.voter_address),
        }
    }
}
