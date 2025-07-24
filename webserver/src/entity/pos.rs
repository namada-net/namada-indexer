use bigdecimal::BigDecimal;
use orm::bond::BondDb;
use orm::crawler_state::{ChainCrawlerStateDb, EpochCrawlerStateDb};
use orm::pos_rewards::PoSRewardDb;
use orm::unbond::UnbondDb;
use orm::validators::{ValidatorDb, ValidatorStateDb};
use serde::{Deserialize, Serialize};
use shared::balance::Amount;
use shared::crawler_state::ChainCrawlerState;
use shared::id::Id;

use crate::response::utils::{epoch_progress, time_between_epochs};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ValidatorState {
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

impl From<ValidatorStateDb> for ValidatorState {
    fn from(value: ValidatorStateDb) -> Self {
        match value {
            ValidatorStateDb::Consensus => Self::Consensus,
            ValidatorStateDb::BelowCapacity => Self::BelowCapacity,
            ValidatorStateDb::BelowThreshold => Self::BelowThreshold,
            ValidatorStateDb::Inactive => Self::Inactive,
            ValidatorStateDb::Jailed => Self::Jailed,
            ValidatorStateDb::Deactivating => Self::Deactivating,
            ValidatorStateDb::Reactivating => Self::Reactivating,
            ValidatorStateDb::Unjailing => Self::Unjailing,
            ValidatorStateDb::Unknown => Self::Unknown,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Validator {
    pub id: String,
    pub address: Id,
    pub voting_power: u64,
    pub max_commission: String,
    pub commission: String,
    pub state: ValidatorState,
    pub name: Option<String>,
    pub email: Option<String>,
    pub website: Option<String>,
    pub description: Option<String>,
    pub discord_handle: Option<String>,
    pub avatar: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BondStatus {
    Active,
    Inactive,
}

#[derive(Clone, Debug)]
pub struct Bond {
    pub min_denom_amount: Amount,
    pub validator: ValidatorWithRank,
    pub status: BondStatus,
    pub start_epoch: u64,
}

#[derive(Clone, Debug)]
pub struct MergedBond {
    pub min_denom_amount: Amount,
    pub validator: ValidatorWithRank,
    pub redelegation: Option<MergedBondRedelegation>,
}

#[derive(Clone, Debug)]
pub struct Unbond {
    pub min_denom_amount: Amount,
    pub validator: ValidatorWithRank,
    pub withdraw_epoch: u64,
    pub withdraw_time: u64,
    pub can_withdraw: bool,
}

#[derive(Clone, Debug)]
pub struct Withdraw {
    pub min_denom_amount: Amount,
    pub validator: ValidatorWithRank,
    pub withdraw_epoch: u64,
}

#[derive(Clone, Debug)]
pub struct Reward {
    pub min_denom_amount: Amount,
    pub validator: ValidatorWithRank,
}

impl From<ValidatorDb> for Validator {
    fn from(value: ValidatorDb) -> Self {
        Self {
            id: value.id.to_string(),
            address: Id::Account(value.namada_address),
            voting_power: value.voting_power as u64,
            max_commission: value.max_commission,
            commission: value.commission,
            state: value.state.into(),
            name: value.name,
            email: value.email,
            website: value.website,
            description: value.description,
            discord_handle: value.discord_handle,
            avatar: value.avatar,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ValidatorWithRank {
    pub validator: Validator,
    pub rank: Option<u64>,
}

impl ValidatorWithRank {
    pub fn from(db_validator: ValidatorDb, rank: Option<i32>) -> Self {
        Self {
            validator: Validator::from(db_validator),
            rank: rank.map(|r| r as u64),
        }
    }
}

impl From<(&BondDb, &EpochCrawlerStateDb)> for BondStatus {
    fn from((bond, status): (&BondDb, &EpochCrawlerStateDb)) -> Self {
        if bond.start <= status.last_processed_epoch {
            Self::Active
        } else {
            Self::Inactive
        }
    }
}

impl Bond {
    pub fn from(
        db_bond: BondDb,
        status: BondStatus,
        db_validator: ValidatorDb,
    ) -> Self {
        Self {
            min_denom_amount: db_bond.raw_amount.into(),
            validator: ValidatorWithRank::from(db_validator, None),
            status,
            start_epoch: db_bond.start as u64,
        }
    }
}

#[derive(Clone, Debug)]
pub struct MergedBondRedelegation {
    pub redelegation_end_epoch: i32,
    pub chain_state: ChainCrawlerState,
    pub min_num_of_blocks: i32,
    pub min_duration: i32,
    pub slash_processing_epoch_offset: i32,
}

impl MergedBond {
    pub fn from(
        amount: BigDecimal,
        db_validator: ValidatorDb,
        redelegation: Option<MergedBondRedelegation>,
    ) -> Self {
        Self {
            min_denom_amount: amount.into(),
            validator: ValidatorWithRank::from(db_validator, None),
            redelegation,
        }
    }
}

impl Unbond {
    pub fn from(
        raw_amount: BigDecimal,
        withdraw_epoch: i32,
        db_validator: ValidatorDb,
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

        let to_withdraw = time_between_epochs(
            blocks_per_epoch,
            epoch_progress,
            chain_state.last_processed_epoch,
            withdraw_epoch,
            min_duration,
        );

        let time_now = chain_state.timestamp.and_utc().timestamp();
        let withdraw_time = time_now + i64::from(to_withdraw);

        Self {
            min_denom_amount: raw_amount.into(),
            validator: ValidatorWithRank::from(db_validator, None),
            withdraw_epoch: withdraw_epoch as u64,
            withdraw_time: withdraw_time as u64,
            can_withdraw: chain_state.last_processed_epoch >= withdraw_epoch,
        }
    }
}

impl Withdraw {
    pub fn from(db_unbond: UnbondDb, db_validator: ValidatorDb) -> Self {
        Self {
            min_denom_amount: db_unbond.raw_amount.into(),
            validator: ValidatorWithRank::from(db_validator, None),
            withdraw_epoch: db_unbond.withdraw_epoch as u64,
        }
    }
}

impl Reward {
    pub fn from(db_reward: PoSRewardDb, db_validator: ValidatorDb) -> Self {
        Self {
            min_denom_amount: db_reward.raw_amount.into(),
            validator: ValidatorWithRank::from(db_validator, None),
        }
    }
}
