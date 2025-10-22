/// Axum HTTP handlers for Esplora API endpoints

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use std::sync::Arc;

use crate::rpc_client::BitcoinRpcClient;
use crate::types::*;

/// Shared application state
pub type AppState = Arc<BitcoinRpcClient>;

/// Custom error type for handlers
pub enum ApiError {
    NotFound(String),
    BadRequest(String),
    Internal(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        (status, message).into_response()
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        ApiError::Internal(err.to_string())
    }
}

/// GET /blocks/tip/height
/// Returns the current blockchain height as plain text
pub async fn get_tip_height(
    State(rpc): State<AppState>,
) -> Result<String, ApiError> {
    let height = rpc.get_block_count()?;
    Ok(height.to_string())
}

/// GET /blocks/tip/hash
/// Returns the current blockchain tip hash as plain text
pub async fn get_tip_hash(
    State(rpc): State<AppState>,
) -> Result<String, ApiError> {
    let hash = rpc.get_best_block_hash()?;
    Ok(hash)
}

/// POST /tx
/// Broadcasts a raw transaction (hex string in body)
/// Returns the txid as plain text
pub async fn broadcast_transaction(
    State(rpc): State<AppState>,
    body: String,
) -> Result<String, ApiError> {
    let txid = rpc.send_raw_transaction(&body)
        .map_err(|e| ApiError::BadRequest(format!("Failed to broadcast: {}", e)))?;
    Ok(txid)
}

/// GET /tx/{txid}
/// Returns transaction details in Esplora JSON format
pub async fn get_transaction(
    State(rpc): State<AppState>,
    Path(txid): Path<String>,
) -> Result<Json<TxResponse>, ApiError> {
    let tx = rpc.get_transaction(&txid)
        .map_err(|_| ApiError::NotFound(format!("Transaction not found: {}", txid)))?;
    Ok(Json(tx))
}

/// GET /tx/{txid}/status
/// Returns transaction confirmation status
pub async fn get_transaction_status(
    State(rpc): State<AppState>,
    Path(txid): Path<String>,
) -> Result<Json<TxStatusResponse>, ApiError> {
    let status = rpc.get_transaction_status(&txid)
        .map_err(|_| ApiError::NotFound(format!("Transaction not found: {}", txid)))?;
    Ok(Json(status))
}

/// GET /tx/{txid}/raw
/// Returns raw transaction hex
pub async fn get_transaction_raw(
    State(rpc): State<AppState>,
    Path(txid): Path<String>,
) -> Result<String, ApiError> {
    let hex = rpc.get_raw_transaction_hex(&txid)
        .map_err(|_| ApiError::NotFound(format!("Transaction not found: {}", txid)))?;
    Ok(hex)
}

/// GET /address/{address}/utxo
/// Returns all UTXOs for an address
pub async fn get_address_utxos(
    State(rpc): State<AppState>,
    Path(address): Path<String>,
) -> Result<Json<Vec<UtxoResponse>>, ApiError> {
    let utxos = rpc.get_address_utxos(&address)
        .map_err(|e| ApiError::BadRequest(format!("Invalid address: {}", e)))?;
    Ok(Json(utxos))
}

/// GET /tx/{txid}/outspend/{index}
/// Returns spending status of a transaction output
pub async fn get_output_spend_status(
    State(rpc): State<AppState>,
    Path((txid, index)): Path<(String, u32)>,
) -> Result<Json<OutputSpendStatus>, ApiError> {
    let status = rpc.get_output_spend_status(&txid, index)
        .map_err(|_| ApiError::NotFound(format!("Output not found: {}:{}", txid, index)))?;
    Ok(Json(status))
}

// ============================================================================
// REGTEST HELPER ENDPOINTS (not part of standard Esplora API)
// ============================================================================

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct MineBlocksRequest {
    pub count: u64,
    #[serde(default)]
    pub address: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MineBlocksResponse {
    pub block_hashes: Vec<String>,
    pub new_height: u64,
}

/// POST /regtest/mine
/// Mine blocks in Regtest (helper endpoint for testing)
pub async fn mine_blocks(
    State(rpc): State<AppState>,
    Json(req): Json<MineBlocksRequest>,
) -> Result<Json<MineBlocksResponse>, ApiError> {
    log::info!("Mining {} blocks", req.count);
    
    let block_hashes = rpc.generate_blocks(req.count, req.address)?;
    let new_height = rpc.get_block_count()?;
    
    log::info!("Mined {} blocks, new height: {}", block_hashes.len(), new_height);
    
    Ok(Json(MineBlocksResponse {
        block_hashes,
        new_height,
    }))
}

/// GET /health
/// Health check endpoint
pub async fn health_check() -> &'static str {
    "OK"
}

