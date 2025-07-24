use std::collections::HashMap;
use std::fs::File;

use namada_sdk::rpc::query_native_token;
use namada_sdk::tendermint_rpc::HttpClient;
use serde::{Deserialize, Serialize};
use shared::checksums::Checksums;
use shared::transaction::TransactionKind;

use crate::namada::query_tx_code_hash;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataLong {
    pub id: String,
    pub name: String,
    pub data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortData {
    pub id: String,
    pub data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Data {
    Long(DataLong),
    Short(ShortData),
}

pub async fn fix(client: &HttpClient) -> anyhow::Result<()> {
    let mut txs = HashMap::new();

    let native_token = query_native_token(client).await?;

    let mut checksums = Checksums::default();
    for code_path in Checksums::code_paths() {
        let code =
            query_tx_code_hash(client, &code_path)
                .await
                .unwrap_or_else(|| {
                    panic!("{} must be defined in namada storage.", code_path)
                });
        checksums.add(code_path, code.to_lowercase());
    }

    let file = File::open("data.csv")?;
    let mut rdr = csv::Reader::from_reader(file);

    for result in rdr.records() {
        let record = result?;
        let deserialized_record = if let Some(d) = record.get(3) {
            serde_json::from_str::<Data>(d).unwrap()
        } else {
            continue;
        };

        let hash = record.get(0).unwrap().to_string();

        let kind_hash = match deserialized_record.clone() {
            Data::Long(data) => data.id,
            Data::Short(data) => data.id,
        };
        let kind = checksums.get_name_by_id(&kind_hash).unwrap_or_else(|| {
            panic!("{} must be defined in namada storage.", kind_hash)
        });

        let data = match deserialized_record {
            Data::Long(data) => {
                let data = data.data.strip_prefix("0x").unwrap();
                subtle_encoding::hex::decode(data).unwrap()
            }
            Data::Short(data) => {
                let data = data.data.strip_prefix("0x").unwrap();
                subtle_encoding::hex::decode(data).unwrap()
            }
        };

        let res = TransactionKind::from(
            &kind_hash,
            &kind,
            &data,
            native_token.clone(),
        );

        if let TransactionKind::Unknown(data) = res {
            panic!("Unknown transaction kind: {} with data: {:?}", kind, data);
        }

        txs.insert(hash, (res, kind));
    }

    for (id, (tx_kind, kind_name)) in txs {
        let kind_name = kind_name.strip_prefix("tx_").unwrap();
        let data = tx_kind.to_json().unwrap();

        let query = format!(
            "UPDATE inner_transactions SET kind = '{}', data = '{}' WHERE id \
             = '{}';",
            kind_name, data, id
        );

        println!("{}", query);
    }

    Ok(())
}
