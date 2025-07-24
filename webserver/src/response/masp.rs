use serde::{Deserialize, Serialize};

use crate::entity::masp::{
    MaspPoolAggregate, MaspPoolAggregateKind, MaspPoolAggregateWindow,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MaspPoolAggregateWindowResponse {
    OneDay,
    SevenDays,
    ThirtyDays,
    AllTime,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MaspPoolAggregateKindResponse {
    Inflows,
    Outflows,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MaspPoolAggregateResponse {
    pub token_address: String,
    pub time_window: MaspPoolAggregateWindowResponse,
    pub kind: MaspPoolAggregateKindResponse,
    pub total_amount: String,
}

impl From<MaspPoolAggregate> for MaspPoolAggregateResponse {
    fn from(value: MaspPoolAggregate) -> Self {
        Self {
            token_address: value.token_address.to_string(),
            time_window: match value.time_window {
                MaspPoolAggregateWindow::OneDay => {
                    MaspPoolAggregateWindowResponse::OneDay
                }
                MaspPoolAggregateWindow::SevenDays => {
                    MaspPoolAggregateWindowResponse::SevenDays
                }
                MaspPoolAggregateWindow::ThirtyDays => {
                    MaspPoolAggregateWindowResponse::ThirtyDays
                }
                MaspPoolAggregateWindow::AllTime => {
                    MaspPoolAggregateWindowResponse::AllTime
                }
            },
            kind: match value.kind {
                MaspPoolAggregateKind::Inflows => {
                    MaspPoolAggregateKindResponse::Inflows
                }
                MaspPoolAggregateKind::Outflows => {
                    MaspPoolAggregateKindResponse::Outflows
                }
            },
            total_amount: value.total_amount.to_string(),
        }
    }
}
