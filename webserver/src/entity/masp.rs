use orm::masp::{
    MaspPoolAggregateKindDb, MaspPoolAggregateWindowDb, MaspPoolDb,
};
use shared::balance::Amount;
use shared::id::Id;

#[derive(Clone, Debug)]
pub enum MaspPoolAggregateWindow {
    OneDay,
    SevenDays,
    ThirtyDays,
    AllTime,
}

#[derive(Clone, Debug)]
pub enum MaspPoolAggregateKind {
    Inflows,
    Outflows,
}

#[derive(Clone, Debug)]
pub struct MaspPoolAggregate {
    pub token_address: Id,
    pub time_window: MaspPoolAggregateWindow,
    pub kind: MaspPoolAggregateKind,
    pub total_amount: Amount,
}

impl From<MaspPoolDb> for MaspPoolAggregate {
    fn from(value: MaspPoolDb) -> Self {
        MaspPoolAggregate {
            token_address: Id::Account(value.token_address),
            time_window: match value.time_window {
                MaspPoolAggregateWindowDb::OneDay => {
                    MaspPoolAggregateWindow::OneDay
                }
                MaspPoolAggregateWindowDb::SevenDays => {
                    MaspPoolAggregateWindow::SevenDays
                }
                MaspPoolAggregateWindowDb::ThirtyDays => {
                    MaspPoolAggregateWindow::ThirtyDays
                }
                MaspPoolAggregateWindowDb::AllTime => {
                    MaspPoolAggregateWindow::AllTime
                }
            },
            kind: match value.kind {
                MaspPoolAggregateKindDb::Inflows => {
                    MaspPoolAggregateKind::Inflows
                }
                MaspPoolAggregateKindDb::Outflows => {
                    MaspPoolAggregateKind::Outflows
                }
            },
            total_amount: value.total_amount.into(),
        }
    }
}
