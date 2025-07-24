use serde::{Deserialize, Serialize};

use super::chain::TokenResponse;
use crate::entity::gas::GasPrice;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GasPriceResponse {
    pub token: TokenResponse,
    pub min_denom_amount: String,
}

impl From<GasPrice> for GasPriceResponse {
    fn from(gas_price: GasPrice) -> Self {
        Self {
            token: gas_price.token.into(),
            min_denom_amount: gas_price.min_denom_amount,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GasEstimateResponse {
    pub min: u64,
    pub max: u64,
    pub avg: u64,
    pub total_estimates: u64,
}
