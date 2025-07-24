use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct TokenSupply {
    #[validate(range(min = 0))]
    pub epoch: Option<i32>,
    pub address: String,
}

#[derive(Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CirculatingSupply {
    #[validate(range(min = 0))]
    pub epoch: Option<i32>,
}
