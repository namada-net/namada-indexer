use shared::id::Id;

#[derive(Clone, Debug)]
pub enum IbcAckStatus {
    Success,
    Fail,
    Timeout,
    Unknown,
}

#[derive(Clone, Debug)]
pub struct IbcAck {
    pub status: IbcAckStatus,
    pub timeout: Option<i64>,
}

#[derive(Clone, Debug)]
pub struct IbcRateLimit {
    pub token_address: Id,
    pub throughput_limit: u64,
}

#[derive(Clone, Debug)]
pub struct IbcTokenFlow {
    pub token_address: Id,
    pub withdraw: u64,
    pub deposit: u64,
}

#[derive(Clone, Debug)]
pub struct IbcTokenThroughput {
    pub throughput: u64,
    pub limit: u64,
}
