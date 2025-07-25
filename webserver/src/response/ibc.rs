use serde::{Deserialize, Serialize};

use crate::entity::ibc::{
    IbcAck, IbcAckStatus, IbcRateLimit, IbcTokenFlow, IbcTokenThroughput,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum IbcAckStatusResponse {
    Success,
    Fail,
    Timeout,
    Unknown,
}

impl From<IbcAckStatus> for IbcAckStatusResponse {
    fn from(value: IbcAckStatus) -> Self {
        match value {
            IbcAckStatus::Success => IbcAckStatusResponse::Success,
            IbcAckStatus::Fail => IbcAckStatusResponse::Fail,
            IbcAckStatus::Timeout => IbcAckStatusResponse::Timeout,
            IbcAckStatus::Unknown => IbcAckStatusResponse::Unknown,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IbcAckResponse {
    pub status: IbcAckStatusResponse,
    pub timeout: Option<i64>,
}

impl From<IbcAck> for IbcAckResponse {
    fn from(value: IbcAck) -> Self {
        Self {
            status: IbcAckStatusResponse::from(value.status),
            timeout: value.timeout,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IbcRateLimitResponse {
    pub token_address: String,
    pub throughput_limit: u64,
}

impl From<IbcRateLimit> for IbcRateLimitResponse {
    fn from(value: IbcRateLimit) -> Self {
        Self {
            token_address: value.token_address.to_string(),
            throughput_limit: value.throughput_limit,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IbcTokenFlowResponse {
    pub token_address: String,
    pub withdraw: u64,
    pub deposit: u64,
}

impl From<IbcTokenFlow> for IbcTokenFlowResponse {
    fn from(value: IbcTokenFlow) -> Self {
        Self {
            token_address: value.token_address.to_string(),
            withdraw: value.withdraw,
            deposit: value.deposit,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IbcTokenThroughputResponse {
    pub throughput: u64,
    pub limit: u64,
}

impl From<IbcTokenThroughput> for IbcTokenThroughputResponse {
    fn from(value: IbcTokenThroughput) -> Self {
        Self {
            throughput: value.throughput,
            limit: value.throughput,
        }
    }
}
