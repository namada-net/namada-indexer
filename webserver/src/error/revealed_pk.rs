use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use thiserror::Error;

use crate::response::api::ApiErrorResponse;

#[derive(Error, Debug)]
pub enum RevealedPkError {
    #[error("{0} is not a valid address")]
    InvalidAddress(String),
    #[error("Revealed public key {0} not found")]
    NotFound(u64),
    #[error("Database error: {0}")]
    Database(String),
    #[error("Rpc error: {0}")]
    Rpc(String),
    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl IntoResponse for RevealedPkError {
    fn into_response(self) -> Response {
        let status_code = match self {
            RevealedPkError::InvalidAddress(_) => StatusCode::BAD_REQUEST,
            RevealedPkError::NotFound(_) => StatusCode::NOT_FOUND,
            RevealedPkError::Unknown(_)
            | RevealedPkError::Database(_)
            | RevealedPkError::Rpc(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        ApiErrorResponse::send(status_code.as_u16(), Some(self.to_string()))
    }
}
