use std::fmt::{self, Display, Formatter};

use diesel::pg::Pg;
use diesel::sql_types::Nullable;
use diesel::{Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};
use shared::crawler_state::{
    BlockCrawlerState, ChainCrawlerState, CrawlerName, EpochCrawlerState,
    IntervalCrawlerState,
};

use crate::schema::crawler_state;

#[derive(Debug, Clone, Serialize, Deserialize, diesel_derive_enum::DbEnum)]
#[ExistingTypePath = "crate::schema::sql_types::CrawlerName"]
pub enum CrawlerNameDb {
    Chain,
    Governance,
    Parameters,
    Pos,
    Rewards,
    Transactions,
    Cometbft,
}

impl Display for CrawlerNameDb {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Chain => f.write_str("chain"),
            Self::Governance => f.write_str("governance"),
            Self::Parameters => f.write_str("parameters"),
            Self::Pos => f.write_str("pos"),
            Self::Rewards => f.write_str("rewards"),
            Self::Transactions => f.write_str("transactions"),
            Self::Cometbft => f.write_str("cometbft"),
        }
    }
}

impl From<CrawlerName> for CrawlerNameDb {
    fn from(value: CrawlerName) -> Self {
        match value {
            CrawlerName::Chain => Self::Chain,
            CrawlerName::Governance => Self::Governance,
            CrawlerName::Parameters => Self::Parameters,
            CrawlerName::Pos => Self::Pos,
            CrawlerName::Rewards => Self::Rewards,
            CrawlerName::Transactions => Self::Transactions,
            CrawlerName::Cometbft => Self::Cometbft,
        }
    }
}

#[derive(Serialize, Queryable, Selectable, Clone, Debug)]
#[diesel(table_name = crawler_state)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct CrawlerStateDb {
    pub name: CrawlerNameDb,
    pub last_processed_block: Option<i32>,
    pub last_processed_epoch: Option<i32>,
    pub first_block_in_epoch: Option<i32>,
    pub timestamp: chrono::NaiveDateTime,
}

#[derive(Serialize, Clone, Debug)]
pub struct ChainCrawlerStateDb {
    pub last_processed_block: i32,
    pub last_processed_epoch: i32,
    pub first_block_in_epoch: i32,
    pub timestamp: chrono::NaiveDateTime,
}
impl
    Queryable<
        (
            Nullable<diesel::sql_types::Integer>,
            Nullable<diesel::sql_types::Integer>,
            Nullable<diesel::sql_types::Integer>,
            diesel::sql_types::Timestamp,
        ),
        Pg,
    > for ChainCrawlerStateDb
{
    type Row = (Option<i32>, Option<i32>, Option<i32>, chrono::NaiveDateTime);

    fn build(row: Self::Row) -> diesel::deserialize::Result<Self> {
        match row {
            (
                Some(last_processed_block),
                Some(last_processed_epoch),
                Some(first_block_in_epoch),
                timestamp,
            ) => Ok(Self {
                last_processed_block,
                last_processed_epoch,
                first_block_in_epoch,
                timestamp,
            }),
            _ => Err("last_processed_block or last_processed_epoch missing \
                      in the block crawler status"
                .into()),
        }
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct BlockCrawlerStateDb {
    pub last_processed_block: i32,
    pub timestamp: chrono::NaiveDateTime,
}
impl
    Queryable<
        (
            Nullable<diesel::sql_types::Integer>,
            diesel::sql_types::Timestamp,
        ),
        Pg,
    > for BlockCrawlerStateDb
{
    type Row = (Option<i32>, chrono::NaiveDateTime);

    fn build(row: Self::Row) -> diesel::deserialize::Result<Self> {
        match row {
            (Some(last_processed_block), timestamp) => Ok(Self {
                last_processed_block,
                timestamp,
            }),
            _ => Err("last_processed_block missing in the block crawler \
                      status"
                .into()),
        }
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct EpochCrawlerStateDb {
    pub last_processed_epoch: i32,
    pub timestamp: chrono::NaiveDateTime,
}
impl
    Queryable<
        (
            Nullable<diesel::sql_types::Integer>,
            diesel::sql_types::Timestamp,
        ),
        Pg,
    > for EpochCrawlerStateDb
{
    type Row = (Option<i32>, chrono::NaiveDateTime);

    fn build(row: Self::Row) -> diesel::deserialize::Result<Self> {
        match row {
            (Some(last_processed_epoch), timestamp) => Ok(Self {
                last_processed_epoch,
                timestamp,
            }),
            _ => Err("last_processed_epoch missing in the chain crawler \
                      status"
                .into()),
        }
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct CometbftCrawlerStateDb {
    pub last_processed_block: i32,
    pub timestamp: chrono::NaiveDateTime,
}
impl
    Queryable<
        (
            Nullable<diesel::sql_types::Integer>,
            diesel::sql_types::Timestamp,
        ),
        Pg,
    > for CometbftCrawlerStateDb
{
    type Row = (Option<i32>, chrono::NaiveDateTime);

    fn build(row: Self::Row) -> diesel::deserialize::Result<Self> {
        match row {
            (Some(last_processed_block), timestamp) => Ok(Self {
                last_processed_block,
                timestamp,
            }),
            _ => Err("last_processed_block missing in the block crawler \
                      status"
                .into()),
        }
    }
}

#[derive(Serialize, Insertable, Clone)]
#[diesel(table_name = crawler_state)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct CrawlerStateTimestampInsertDb {
    pub name: CrawlerNameDb,
    pub timestamp: chrono::NaiveDateTime,
}

#[derive(Serialize, Insertable, Clone)]
#[diesel(table_name = crawler_state)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ChainStateInsertDb {
    pub name: CrawlerNameDb,
    pub last_processed_block: i32,
    pub last_processed_epoch: i32,
    pub first_block_in_epoch: i32,
    pub timestamp: chrono::NaiveDateTime,
}

#[derive(Serialize, Insertable, Clone)]
#[diesel(table_name = crawler_state)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct BlockStateInsertDb {
    pub name: CrawlerNameDb,
    pub last_processed_block: i32,
    pub timestamp: chrono::NaiveDateTime,
}

#[derive(Serialize, Insertable, Clone)]
#[diesel(table_name = crawler_state)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct EpochStateInsertDb {
    pub name: CrawlerNameDb,
    pub last_processed_epoch: i32,
    pub timestamp: chrono::NaiveDateTime,
}

#[derive(Serialize, Insertable, Clone)]
#[diesel(table_name = crawler_state)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct IntervalStateInsertDb {
    pub name: CrawlerNameDb,
    pub timestamp: chrono::NaiveDateTime,
}

impl From<(CrawlerName, i64)> for CrawlerStateTimestampInsertDb {
    fn from((crawler_name, timestamp): (CrawlerName, i64)) -> Self {
        let timestamp = chrono::DateTime::from_timestamp(timestamp, 0)
            .expect("Invalid timestamp")
            .naive_utc();

        Self {
            name: crawler_name.into(),
            timestamp,
        }
    }
}

impl From<(CrawlerName, ChainCrawlerState)> for ChainStateInsertDb {
    fn from((crawler_name, state): (CrawlerName, ChainCrawlerState)) -> Self {
        let timestamp = chrono::DateTime::from_timestamp(state.timestamp, 0)
            .expect("Invalid timestamp")
            .naive_utc();

        Self {
            name: crawler_name.into(),
            last_processed_block: state.last_processed_block as i32,
            last_processed_epoch: state.last_processed_epoch as i32,
            first_block_in_epoch: state.first_block_in_epoch as i32,
            timestamp,
        }
    }
}

impl From<(CrawlerName, BlockCrawlerState)> for BlockStateInsertDb {
    fn from((crawler_name, state): (CrawlerName, BlockCrawlerState)) -> Self {
        let timestamp = chrono::DateTime::from_timestamp(state.timestamp, 0)
            .expect("Invalid timestamp")
            .naive_utc();

        Self {
            name: crawler_name.into(),
            last_processed_block: state.last_processed_block as i32,
            timestamp,
        }
    }
}

impl From<(CrawlerName, EpochCrawlerState)> for EpochStateInsertDb {
    fn from((crawler_name, state): (CrawlerName, EpochCrawlerState)) -> Self {
        let timestamp = chrono::DateTime::from_timestamp(state.timestamp, 0)
            .expect("Invalid timestamp")
            .naive_utc();

        Self {
            name: crawler_name.into(),
            last_processed_epoch: state.last_processed_epoch as i32,
            timestamp,
        }
    }
}

impl From<(CrawlerName, IntervalCrawlerState)> for IntervalStateInsertDb {
    fn from(
        (crawler_name, state): (CrawlerName, IntervalCrawlerState),
    ) -> Self {
        let timestamp = chrono::DateTime::from_timestamp(state.timestamp, 0)
            .expect("Invalid timestamp")
            .naive_utc();

        Self {
            name: crawler_name.into(),
            timestamp,
        }
    }
}
