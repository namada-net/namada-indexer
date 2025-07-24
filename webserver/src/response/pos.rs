use serde::{Deserialize, Serialize};

use crate::entity::pos::{
    Bond, BondStatus, MergedBond, Reward, Unbond, Validator, ValidatorState,
    ValidatorWithRank, Withdraw,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ValidatorStateResponse {
    Consensus,
    BelowCapacity,
    BelowThreshold,
    Inactive,
    Jailed,
    Deactivating,
    Reactivating,
    Unjailing,
    Unknown,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidatorResponse {
    pub address: String,
    pub voting_power: u64,
    pub max_commission: String,
    pub commission: String,
    pub state: ValidatorStateResponse,
    pub name: Option<String>,
    pub email: Option<String>,
    pub website: Option<String>,
    pub description: Option<String>,
    pub discord_handle: Option<String>,
    pub avatar: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BondStatusResponse {
    Active,
    Inactive,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BondResponse {
    pub min_denom_amount: String,
    pub validator: ValidatorWithRankResponse,
    pub status: BondStatusResponse,
    pub start_epoch: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MergedBondResponse {
    pub min_denom_amount: String,
    pub validator: ValidatorWithRankResponse,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnbondResponse {
    pub min_denom_amount: String,
    pub validator: ValidatorWithRankResponse,
    pub withdraw_epoch: String,
    pub withdraw_time: String,
    pub can_withdraw: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WithdrawResponse {
    pub min_denom_amount: String,
    pub validator: ValidatorWithRankResponse,
    pub withdraw_epoch: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RewardResponse {
    pub min_denom_amount: String,
    pub validator: ValidatorWithRankResponse,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TotalVotingPowerResponse {
    pub total_voting_power: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidatorWithRankResponse {
    #[serde(flatten)]
    pub validator: ValidatorResponse,
    pub rank: Option<u64>,
}

impl From<ValidatorState> for ValidatorStateResponse {
    fn from(value: ValidatorState) -> Self {
        match value {
            ValidatorState::Consensus => Self::Consensus,
            ValidatorState::BelowCapacity => Self::BelowCapacity,
            ValidatorState::BelowThreshold => Self::BelowThreshold,
            ValidatorState::Inactive => Self::Inactive,
            ValidatorState::Jailed => Self::Jailed,
            ValidatorState::Deactivating => Self::Deactivating,
            ValidatorState::Reactivating => Self::Reactivating,
            ValidatorState::Unjailing => Self::Unjailing,
            ValidatorState::Unknown => Self::Unknown,
        }
    }
}

impl From<BondStatus> for BondStatusResponse {
    fn from(value: BondStatus) -> Self {
        match value {
            BondStatus::Active => Self::Active,
            BondStatus::Inactive => Self::Inactive,
        }
    }
}

impl From<Validator> for ValidatorResponse {
    fn from(value: Validator) -> Self {
        ValidatorResponse {
            address: value.address.to_string(),
            voting_power: value.voting_power,
            max_commission: value.max_commission,
            commission: value.commission,
            state: ValidatorStateResponse::from(value.state),
            name: value.name,
            email: value.email,
            website: value.website,
            description: value.description,
            discord_handle: value.discord_handle,
            avatar: value.avatar,
        }
    }
}

impl From<ValidatorWithRank> for ValidatorWithRankResponse {
    fn from(value: ValidatorWithRank) -> Self {
        ValidatorWithRankResponse {
            validator: value.validator.into(),
            rank: value.rank,
        }
    }
}

impl From<Bond> for BondResponse {
    fn from(value: Bond) -> Self {
        BondResponse {
            min_denom_amount: value.min_denom_amount.to_string(),
            validator: ValidatorWithRankResponse::from(value.validator),
            status: BondStatusResponse::from(value.status),
            start_epoch: value.start_epoch,
        }
    }
}

impl From<MergedBond> for MergedBondResponse {
    fn from(value: MergedBond) -> Self {
        MergedBondResponse {
            min_denom_amount: value.min_denom_amount.to_string(),
            validator: ValidatorWithRankResponse::from(value.validator),
        }
    }
}

impl From<Unbond> for UnbondResponse {
    fn from(value: Unbond) -> Self {
        UnbondResponse {
            min_denom_amount: value.min_denom_amount.to_string(),
            validator: ValidatorWithRankResponse::from(value.validator),
            withdraw_epoch: value.withdraw_epoch.to_string(),
            withdraw_time: value.withdraw_time.to_string(),
            can_withdraw: value.can_withdraw,
        }
    }
}

impl From<Withdraw> for WithdrawResponse {
    fn from(value: Withdraw) -> Self {
        WithdrawResponse {
            min_denom_amount: value.min_denom_amount.to_string(),
            validator: ValidatorWithRankResponse::from(value.validator),
            withdraw_epoch: value.withdraw_epoch.to_string(),
        }
    }
}

impl From<Reward> for RewardResponse {
    fn from(value: Reward) -> Self {
        RewardResponse {
            min_denom_amount: value.min_denom_amount.to_string(),
            validator: ValidatorWithRankResponse::from(value.validator),
        }
    }
}
