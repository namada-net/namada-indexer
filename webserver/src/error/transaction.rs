use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use thiserror::Error;

use crate::response::api::ApiErrorResponse;

#[derive(Error, Debug)]
pub enum TransactionError {
    #[error("The tx id must be 32bytes long")]
    InvalidTxId,
    #[error("The tx id {0} does not exist")]
    TxIdNotFound(String),
    #[error("Database error: {0}")]
    Database(String),
    #[error("Rpc error: {0}")]
    Rpc(String),
    #[error("Token parameter can only be used with transfer transaction kinds. Valid transfer kinds are: {0}")]
    InvalidTokenParameter(String),
    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl IntoResponse for TransactionError {
    fn into_response(self) -> Response {
        let status_code = match self {
            TransactionError::InvalidTxId => StatusCode::BAD_REQUEST,
            TransactionError::TxIdNotFound(_) => StatusCode::NOT_FOUND,
            TransactionError::InvalidTokenParameter(_) => StatusCode::BAD_REQUEST,
            TransactionError::Unknown(_)
            | TransactionError::Database(_)
            | TransactionError::Rpc(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        ApiErrorResponse::send(status_code.as_u16(), Some(self.to_string()))
    }
}
