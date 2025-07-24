use serde::{Deserialize, Serialize};
use subtle_encoding::hex;
use validator::Validate;

use crate::error::transaction::TransactionError;

#[derive(Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct TransactionHistoryQueryParams {
    #[validate(range(min = 1, max = 10000))]
    pub page: Option<u64>,
    #[validate(length(min = 1, max = 10))]
    pub addresses: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionIdParam(String);

impl TransactionIdParam {
    pub fn is_valid_hash(&self) -> Result<(), TransactionError> {
        let is_valid_lenght = self.0.len().eq(&64);
        let is_valid_hex = hex::decode(self.0.as_bytes()).is_ok();
        if is_valid_lenght && is_valid_hex {
            Ok(())
        } else {
            Err(TransactionError::InvalidTxId)
        }
    }

    pub fn get(&self) -> String {
        self.0.to_lowercase()
    }
}
