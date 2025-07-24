use std::fmt::Display;

use serde::{Deserialize, Serialize};

use crate::entity::governance::{
    Proposal, ProposalStatus, ProposalType, ProposalVote, TallyType, VoteType,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProposalTypeResponse {
    Default,
    DefaultWithWasm,
    PgfSteward,
    PgfFunding,
}

impl Display for ProposalTypeResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProposalTypeResponse::Default => write!(f, "default"),
            ProposalTypeResponse::DefaultWithWasm => {
                write!(f, "default_with_wasm")
            }
            ProposalTypeResponse::PgfSteward => write!(f, "pgf_steward"),
            ProposalTypeResponse::PgfFunding => write!(f, "pgf_funding"),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TallyTypeResponse {
    TwoFifths,
    OneHalfOverOneThird,
    LessOneHalfOverOneThirdNay,
}

impl Display for TallyTypeResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TallyTypeResponse::TwoFifths => write!(f, "two_fifths"),
            TallyTypeResponse::OneHalfOverOneThird => {
                write!(f, "one_half_over_one_third")
            }
            TallyTypeResponse::LessOneHalfOverOneThirdNay => {
                write!(f, "less_one_half_over_one_third_nay")
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VoteTypeResponse {
    Yay,
    Nay,
    Abstain,
    Unknown,
}

impl Display for VoteTypeResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VoteTypeResponse::Yay => write!(f, "yay"),
            VoteTypeResponse::Nay => write!(f, "nay"),
            VoteTypeResponse::Abstain => write!(f, "abstain"),
            VoteTypeResponse::Unknown => write!(f, "unknown"),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProposalStatusResponse {
    Pending,
    Rejected,
    Passed,
    Voting,
    ExecutedPassed,
    ExecutedRejected,
    Unknown,
}

impl Display for ProposalStatusResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProposalStatusResponse::Pending => write!(f, "Pending"),
            ProposalStatusResponse::Rejected => write!(f, "Rejected"),
            ProposalStatusResponse::Passed => write!(f, "Passed"),
            ProposalStatusResponse::Voting => write!(f, "Voting"),
            ProposalStatusResponse::ExecutedPassed => {
                write!(f, "ExecutedPassed")
            }
            ProposalStatusResponse::ExecutedRejected => {
                write!(f, "ExecutedRejected")
            }
            ProposalStatusResponse::Unknown => write!(f, "Unknown"),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProposalResponse {
    pub id: u64,
    pub content: String,
    pub r#type: ProposalTypeResponse,
    pub tally_type: TallyTypeResponse,
    pub data: Option<String>,
    pub author: String,
    pub start_epoch: u64,
    pub end_epoch: u64,
    pub activation_epoch: u64,
    pub start_time: String,
    pub end_time: String,
    pub current_time: String,
    pub activation_time: String,
    pub status: ProposalStatusResponse,
    pub yay_votes: f64,
    pub nay_votes: f64,
    pub abstain_votes: f64,
}

impl From<Proposal> for ProposalResponse {
    fn from(value: Proposal) -> Self {
        Self {
            id: value.id,
            content: value.content,
            r#type: match value.r#type {
                ProposalType::Default => ProposalTypeResponse::Default,
                ProposalType::DefaultWithWasm => {
                    ProposalTypeResponse::DefaultWithWasm
                }
                ProposalType::PgfSteward => ProposalTypeResponse::PgfSteward,
                ProposalType::PgfFunding => ProposalTypeResponse::PgfFunding,
            },
            tally_type: match value.tally_type {
                TallyType::TwoFifths => TallyTypeResponse::TwoFifths,
                TallyType::OneHalfOverOneThird => {
                    TallyTypeResponse::OneHalfOverOneThird
                }
                TallyType::LessOneHalfOverOneThirdNay => {
                    TallyTypeResponse::LessOneHalfOverOneThirdNay
                }
            },
            data: value.data,
            author: value.author.to_string(),
            start_epoch: value.start_epoch,
            end_epoch: value.end_epoch,
            activation_epoch: value.activation_epoch,
            start_time: value.start_time,
            end_time: value.end_time,
            current_time: value.current_time,
            activation_time: value.activation_time,
            status: match value.status {
                ProposalStatus::Pending => ProposalStatusResponse::Pending,
                ProposalStatus::Rejected => ProposalStatusResponse::Rejected,
                ProposalStatus::Passed => ProposalStatusResponse::Passed,
                ProposalStatus::Voting => ProposalStatusResponse::Voting,
                ProposalStatus::ExecutedPassed => {
                    ProposalStatusResponse::ExecutedPassed
                }
                ProposalStatus::ExecutedRejected => {
                    ProposalStatusResponse::ExecutedRejected
                }
                ProposalStatus::Unknown => ProposalStatusResponse::Unknown,
            },
            yay_votes: value.yay_votes,
            nay_votes: value.nay_votes,
            abstain_votes: value.abstain_votes,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProposalVoteResponse {
    pub proposal_id: u64,
    pub vote: VoteTypeResponse,
    pub voter_address: String,
}

impl From<ProposalVote> for ProposalVoteResponse {
    fn from(value: ProposalVote) -> Self {
        Self {
            proposal_id: value.proposal_id,
            vote: match value.vote {
                VoteType::Yay => VoteTypeResponse::Yay,
                VoteType::Nay => VoteTypeResponse::Nay,
                VoteType::Abstain => VoteTypeResponse::Abstain,
                VoteType::Unknown => VoteTypeResponse::Unknown,
            },
            voter_address: value.voter_address.to_string(),
        }
    }
}
