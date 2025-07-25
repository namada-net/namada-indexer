use serde::{Deserialize, Serialize};

use super::chain::TokenResponse;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddressBalanceResponse {
    pub token_address: TokenResponse,
    pub min_denom_amount: String,
}
