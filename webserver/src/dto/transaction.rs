use serde::{Deserialize, Serialize};
use serde::de::value::StrDeserializer;
use validator::Validate;
use crate::response::transaction::TransactionKind;

#[derive(Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct TransactionHistoryQueryParams {
    #[validate(range(min = 1, max = 10000))]
    pub page: Option<u64>,
    #[validate(length(min = 1, max = 10))]
    pub addresses: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct RecentInnerQueryParams {
    #[validate(range(min = 1, max = 10000))]
    pub page: Option<u64>,
    #[validate(range(min = 1, max = 100))]
    pub number: Option<u64>,
    #[serde(default, deserialize_with = "deserialize_kinds_opt")]
    pub kind: Option<Vec<TransactionKind>>,
}

// Parse the comma separated list of tx kinds from the query string into a vec of validated tx kinds
#[derive(Deserialize)]
#[serde(untagged)]
enum KindList {
    List(Vec<TransactionKind>),
    Csv(String),
}

fn deserialize_kinds_opt<'de, D>(deserializer: D) -> Result<Option<Vec<TransactionKind>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt = Option::<KindList>::deserialize(deserializer)?;
    match opt {
        None => Ok(None),
        Some(KindList::List(v)) => Ok(Some(v)),
        Some(KindList::Csv(s)) => {
            let kinds: Result<Vec<_>, D::Error> = s
                .split(',')
                .filter(|p| !p.is_empty())
                .map(|p| p.trim())
                .map(|p| TransactionKind::deserialize(StrDeserializer::<D::Error>::new(p)))
                .collect();
            kinds.map(Some)
        }
    }
}
