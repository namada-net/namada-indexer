use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ValidatorStateDto {
    Consensus,
    BelowCapacity,
    BelowThreshold,
    Inactive,
    Jailed,
    Unknown,
}

impl ValidatorStateDto {
    pub fn all() -> Vec<Self> {
        [
            Self::Consensus,
            Self::BelowCapacity,
            Self::BelowThreshold,
            Self::Inactive,
            Self::Jailed,
            Self::Unknown,
        ]
        .to_vec()
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OrderByDto {
    Asc,
    Desc,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ValidatorSortFieldDto {
    VotingPower,
    Commission,
    Rank,
}

#[derive(Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ValidatorQueryParams {
    #[validate(range(min = 1, max = 10000))]
    pub page: Option<u64>,
    pub state: Option<Vec<ValidatorStateDto>>,
    pub sort_field: Option<ValidatorSortFieldDto>,
    pub sort_order: Option<OrderByDto>,
}

#[derive(Clone, Serialize, Deserialize, Validate)]
pub struct AllValidatorsQueryParams {
    pub state: Option<Vec<ValidatorStateDto>>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MyValidatorKindDto {
    WithBonds,
    WithUnbonds,
}

#[derive(Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct BondsDto {
    #[validate(range(min = 1, max = 10000))]
    pub page: Option<u64>,
    #[validate(range(min = 0))]
    pub active_at: Option<i32>,
}

#[derive(Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UnbondsDto {
    #[validate(range(min = 1, max = 10000))]
    pub page: Option<u64>,
    #[validate(range(min = 0))]
    pub active_at: Option<i32>,
}

#[derive(Clone, Serialize, Deserialize, Validate)]
pub struct WithdrawsDto {
    #[validate(range(min = 1, max = 10000))]
    pub page: Option<u64>,
    #[validate(range(min = 1, max = 10000))]
    pub epoch: Option<u64>,
}

#[derive(Clone, Serialize, Deserialize, Validate)]
pub struct RewardsDto {
    #[validate(range(min = 1, max = 10000))]
    pub epoch: Option<u64>,
}
