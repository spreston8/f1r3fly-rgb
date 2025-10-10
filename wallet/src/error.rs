use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WalletError {
    #[error("Wallet already exists: {0}")]
    WalletExists(String),

    #[error("Wallet not found: {0}")]
    WalletNotFound(String),

    #[error("Invalid mnemonic: {0}")]
    InvalidMnemonic(String),

    #[error("Invalid descriptor: {0}")]
    InvalidDescriptor(String),

    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("Bitcoin error: {0}")]
    Bitcoin(String),

    #[error("Esplora error: {0}")]
    Esplora(String),

    #[error("Insufficient funds: {0}")]
    InsufficientFunds(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("RGB error: {0}")]
    Rgb(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Wallet directory not found: {0}")]
    DirectoryNotFound(String),

    #[error("File not found: {0}")]
    FileNotFound(String),
}

impl IntoResponse for WalletError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            WalletError::WalletExists(_) => (StatusCode::CONFLICT, self.to_string()),
            WalletError::WalletNotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            WalletError::InvalidMnemonic(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            WalletError::InvalidDescriptor(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            WalletError::InsufficientFunds(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            WalletError::Network(_) => (StatusCode::SERVICE_UNAVAILABLE, self.to_string()),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}

