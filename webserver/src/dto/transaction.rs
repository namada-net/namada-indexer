use serde::de::value::StrDeserializer;
use serde::{Deserialize, Serialize};
use subtle_encoding::hex;
use validator::Validate;

use crate::entity::transaction::TransactionKind;
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

#[derive(Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct TransactionMostRecentQueryParams {
    #[validate(range(min = 0, max = 1000000))]
    pub offset: Option<u64>,
    #[validate(range(min = 10, max = 30))]
    pub size: Option<u64>,
    #[serde(default, deserialize_with = "deserialize_kinds_vec")]
    pub kind: Vec<TransactionKind>,
    #[serde(default, deserialize_with = "deserialize_tokens_vec")]
    pub token: Vec<String>,
}

// Parse the comma separated list of tx kinds from the query string into a vec
// of validated tx kinds
#[derive(Deserialize)]
#[serde(untagged)]
enum KindList {
    List(Vec<TransactionKind>),
    Csv(String),
}

fn deserialize_kinds_vec<'de, D>(
    deserializer: D,
) -> Result<Vec<TransactionKind>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt = Option::<KindList>::deserialize(deserializer)?;
    let vec = match opt {
        None => Vec::new(),
        Some(KindList::List(v)) => v,
        Some(KindList::Csv(s)) => s
            .split(',')
            .filter(|p| !p.is_empty())
            .map(|p| p.trim())
            .map(|p| {
                TransactionKind::deserialize(StrDeserializer::<D::Error>::new(
                    &p,
                ))
            })
            .collect::<Result<Vec<_>, _>>()?,
    };
    Ok(vec)
}

// Parse the comma separated list of token addresses from the query string into
// a vec of strings
#[derive(Deserialize)]
#[serde(untagged)]
enum TokenList {
    List(Vec<String>),
    Csv(String),
}

fn deserialize_tokens_vec<'de, D>(
    deserializer: D,
) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt = Option::<TokenList>::deserialize(deserializer)?;
    let vec = match opt {
        None => Vec::new(),
        Some(TokenList::List(v)) => v,
        Some(TokenList::Csv(s)) => s
            .split(',')
            .filter(|p| !p.is_empty())
            .map(|p| p.trim().to_string())
            .collect(),
    };
    Ok(vec)
}
