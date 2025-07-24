use std::str::FromStr;

use namada_sdk::address::Address as NamadaAddress;
use namada_sdk::rpc;
use namada_sdk::tendermint_rpc::HttpClient;
use orm::revealed_pk::RevealedPkInsertDb;

use crate::appstate::AppState;
use crate::entity::pk::RevealedPk;
use crate::error::revealed_pk::RevealedPkError;
use crate::repository::revealed_pk::{PkRepoTrait, RevealedPkRepo};

#[derive(Clone)]
pub struct RevealedPkService {
    pub revealed_pk_repo: RevealedPkRepo,
}

impl RevealedPkService {
    pub fn new(app_state: AppState) -> Self {
        Self {
            revealed_pk_repo: RevealedPkRepo::new(app_state),
        }
    }

    pub async fn get_revealed_pk_by_address(
        &self,
        client: &HttpClient,
        address: String,
    ) -> Result<RevealedPk, RevealedPkError> {
        // We look for a revealed public key in the database
        let revealed_pk_db = self
            .revealed_pk_repo
            .get_revealed_pk_by_address(address.clone())
            .await
            .map_err(RevealedPkError::Database)?;

        match revealed_pk_db {
            // If we find a revealed public key in the database, we return it
            Some(revealed_pk_db) => Ok(RevealedPk::from(revealed_pk_db)),
            // If we don't find a revealed public key in the database, we look
            // for it in the storage
            None => {
                let address =
                    NamadaAddress::from_str(&address).expect("Invalid address");

                let public_key = rpc::get_public_key_at(client, &address, 0)
                    .await
                    .map_err(|e| RevealedPkError::Rpc(e.to_string()))?;

                // If we find a public key in the storage, we insert it in the
                // database
                if let Some(public_key) = public_key.clone() {
                    // TODO: maybe better to create it using structs from shared
                    let revealed_pk_db = RevealedPkInsertDb {
                        pk: public_key.to_string(),
                        address: address.to_string(),
                    };

                    self.revealed_pk_repo
                        .insert_revealed_pk(revealed_pk_db)
                        .await
                        .map_err(RevealedPkError::Database)?;
                };

                Ok(RevealedPk {
                    public_key: public_key.map(|pk| pk.to_string()),
                })
            }
        }
    }
}
