use bigdecimal::BigDecimal;
use orm::ibc::IbcAckStatusDb;
use shared::id::Id;

use crate::appstate::AppState;
use crate::entity::ibc::{
    IbcAck, IbcAckStatus, IbcRateLimit, IbcTokenFlow, IbcTokenThroughput,
};
use crate::error::ibc::IbcError;
use crate::repository::ibc::{IbcRepository, IbcRepositoryTrait};

#[derive(Clone)]
pub struct IbcService {
    pub ibc_repo: IbcRepository,
}

impl IbcService {
    pub fn new(app_state: AppState) -> Self {
        Self {
            ibc_repo: IbcRepository::new(app_state),
        }
    }

    pub async fn get_ack_by_tx_id(
        &self,
        tx_id: String,
    ) -> Result<IbcAck, IbcError> {
        self.ibc_repo
            .find_ibc_ack(tx_id)
            .await
            .map_err(IbcError::Database)
            .map(|ack| match ack {
                Some(ack) => IbcAck {
                    status: match ack.status {
                        IbcAckStatusDb::Unknown => IbcAckStatus::Unknown,
                        IbcAckStatusDb::Timeout => IbcAckStatus::Timeout,
                        IbcAckStatusDb::Fail => IbcAckStatus::Fail,
                        IbcAckStatusDb::Success => IbcAckStatus::Success,
                    },
                    timeout: Some(ack.timeout),
                },
                None => IbcAck {
                    status: IbcAckStatus::Unknown,
                    timeout: None,
                },
            })
    }

    pub async fn get_throughput_limits(
        &self,
        matching_token_address: Option<String>,
        matching_rate_limit: Option<BigDecimal>,
    ) -> Result<Vec<IbcRateLimit>, IbcError> {
        self.ibc_repo
            .get_throughput_limits(matching_token_address, matching_rate_limit)
            .await
            .map_err(IbcError::Database)
            .map(|limits| {
                limits
                    .into_iter()
                    .map(|(token_address, throughput_limit)| IbcRateLimit {
                        token_address: Id::Account(token_address),
                        throughput_limit: throughput_limit
                            .parse()
                            .expect("Should be a valid number"),
                    })
                    .collect()
            })
    }

    pub async fn get_token_flows(
        &self,
        matching_token_address: Option<String>,
    ) -> Result<Vec<IbcTokenFlow>, IbcError> {
        self.ibc_repo
            .get_token_flows(matching_token_address)
            .await
            .map_err(IbcError::Database)
            .map(|flows| {
                flows
                    .into_iter()
                    .map(|(token_address, withdraw, deposit)| IbcTokenFlow {
                        token_address: Id::Account(token_address),
                        withdraw: withdraw
                            .parse()
                            .expect("Should be a valid number"),
                        deposit: deposit
                            .parse()
                            .expect("Should be a valid number"),
                    })
                    .collect()
            })
    }

    pub async fn get_token_throughput(
        &self,
        token: String,
    ) -> Result<IbcTokenThroughput, IbcError> {
        self.ibc_repo
            .get_token_throughput(token)
            .await
            .map_err(IbcError::Database)
            .map(|(throughput, limit)| IbcTokenThroughput {
                throughput: throughput
                    .parse()
                    .expect("Should be a valid number"),
                limit: limit.parse().expect("Should be a valid number"),
            })
    }
}
