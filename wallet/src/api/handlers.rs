use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::wallet::manager::{AddressInfo, NextAddressInfo, SyncResult, WalletInfo, WalletManager, WalletMetadata};
use crate::wallet::balance::BalanceInfo;
use crate::wallet::rgb::{IssueAssetRequest, IssueAssetResponse};
use super::types::{CreateWalletRequest, ImportWalletRequest, CreateUtxoRequest, CreateUtxoResponse, UnlockUtxoRequest, UnlockUtxoResponse, SendBitcoinRequest, SendBitcoinResponse, GenerateInvoiceRequest, GenerateInvoiceResponse, SendTransferRequest, SendTransferResponse, AcceptConsignmentResponse, ExportGenesisResponse};

pub async fn create_wallet_handler(
    State(manager): State<Arc<WalletManager>>,
    Json(req): Json<CreateWalletRequest>,
) -> Result<Json<WalletInfo>, crate::error::WalletError> {
    let wallet_info = manager.create_wallet(&req.name)?;
    Ok(Json(wallet_info))
}

pub async fn import_wallet_handler(
    State(manager): State<Arc<WalletManager>>,
    Json(req): Json<ImportWalletRequest>,
) -> Result<Json<WalletInfo>, crate::error::WalletError> {
    let wallet_info = manager.import_wallet(&req.name, &req.mnemonic)?;
    Ok(Json(wallet_info))
}

pub async fn list_wallets_handler(
    State(manager): State<Arc<WalletManager>>,
) -> Result<Json<Vec<WalletMetadata>>, crate::error::WalletError> {
    let wallets = manager.list_wallets()?;
    Ok(Json(wallets))
}

#[derive(Debug, Deserialize)]
pub struct AddressQuery {
    #[serde(default = "default_address_count")]
    pub count: u32,
}

fn default_address_count() -> u32 {
    10
}

pub async fn get_addresses_handler(
    State(manager): State<Arc<WalletManager>>,
    Path(name): Path<String>,
    Query(query): Query<AddressQuery>,
) -> Result<Json<Vec<AddressInfo>>, crate::error::WalletError> {
    let addresses = manager.get_addresses(&name, query.count)?;
    Ok(Json(addresses))
}

pub async fn get_primary_address_handler(
    State(manager): State<Arc<WalletManager>>,
    Path(name): Path<String>,
) -> Result<Json<NextAddressInfo>, crate::error::WalletError> {
    let primary_address = manager.get_primary_address(&name)?;
    Ok(Json(primary_address))
}

pub async fn get_balance_handler(
    State(manager): State<Arc<WalletManager>>,
    Path(name): Path<String>,
) -> Result<Json<BalanceInfo>, crate::error::WalletError> {
    let balance = manager.get_balance(&name).await?;
    Ok(Json(balance))
}

pub async fn sync_wallet_handler(
    State(manager): State<Arc<WalletManager>>,
    Path(name): Path<String>,
) -> Result<Json<SyncResult>, crate::error::WalletError> {
    let result = manager.sync_wallet(&name).await?;
    Ok(Json(result))
}

pub async fn sync_rgb_handler(
    State(manager): State<Arc<WalletManager>>,
    Path(name): Path<String>,
) -> Result<Json<()>, crate::error::WalletError> {
    manager.sync_rgb_runtime(&name)?;
    Ok(Json(()))
}

pub async fn create_utxo_handler(
    State(manager): State<Arc<WalletManager>>,
    Path(name): Path<String>,
    Json(req): Json<CreateUtxoRequest>,
) -> Result<Json<CreateUtxoResponse>, crate::error::WalletError> {
    let manager_req = crate::wallet::manager::CreateUtxoRequest {
        amount_btc: req.amount_btc,
        fee_rate_sat_vb: req.fee_rate_sat_vb,
    };
    
    let result = manager.create_utxo(&name, manager_req).await?;
    
    Ok(Json(CreateUtxoResponse {
        txid: result.txid,
        amount_sats: result.amount_sats,
        fee_sats: result.fee_sats,
        target_address: result.target_address,
    }))
}

pub async fn unlock_utxo_handler(
    State(manager): State<Arc<WalletManager>>,
    Path(name): Path<String>,
    Json(req): Json<UnlockUtxoRequest>,
) -> Result<Json<UnlockUtxoResponse>, crate::error::WalletError> {
    let manager_req = crate::wallet::manager::UnlockUtxoRequest {
        txid: req.txid,
        vout: req.vout,
        fee_rate_sat_vb: req.fee_rate_sat_vb,
    };
    
    let result = manager.unlock_utxo(&name, manager_req).await?;
    
    Ok(Json(UnlockUtxoResponse {
        txid: result.txid,
        recovered_sats: result.recovered_sats,
        fee_sats: result.fee_sats,
    }))
}

pub async fn send_bitcoin_handler(
    State(manager): State<Arc<WalletManager>>,
    Path(name): Path<String>,
    Json(req): Json<SendBitcoinRequest>,
) -> Result<Json<SendBitcoinResponse>, crate::error::WalletError> {
    let manager_req = crate::wallet::manager::SendBitcoinRequest {
        to_address: req.to_address,
        amount_sats: req.amount_sats,
        fee_rate_sat_vb: req.fee_rate_sat_vb,
    };
    
    let result = manager.send_bitcoin(&name, manager_req).await?;
    
    Ok(Json(SendBitcoinResponse {
        txid: result.txid,
        amount_sats: result.amount_sats,
        fee_sats: result.fee_sats,
        to_address: result.to_address,
    }))
}

pub async fn issue_asset_handler(
    State(manager): State<Arc<WalletManager>>,
    Path(name): Path<String>,
    Json(req): Json<IssueAssetRequest>,
) -> Result<Json<IssueAssetResponse>, crate::error::WalletError> {
    // Validate wallet exists
    if !manager.storage.wallet_exists(&name) {
        return Err(crate::error::WalletError::WalletNotFound(name));
    }
    
    // Issue asset via RGB manager
    let result = manager.rgb_manager.issue_rgb20_asset(req)?;
    
    Ok(Json(result))
}

pub async fn generate_invoice_handler(
    State(manager): State<Arc<WalletManager>>,
    Path(name): Path<String>,
    Json(req): Json<GenerateInvoiceRequest>,
) -> Result<Json<GenerateInvoiceResponse>, crate::error::WalletError> {
    // Generate invoice
    let result = manager.generate_rgb_invoice(&name, 
        crate::wallet::manager::GenerateInvoiceRequest {
            contract_id: req.contract_id.clone(),
            amount: req.amount,
        }
    ).await?;
    
    Ok(Json(GenerateInvoiceResponse {
        invoice: result.invoice,
        contract_id: result.contract_id,
        amount: result.amount,
        seal_utxo: result.seal_utxo,
    }))
}

pub async fn send_transfer_handler(
    State(manager): State<Arc<WalletManager>>,
    Path(wallet_name): Path<String>,
    Json(request): Json<SendTransferRequest>,
) -> Result<Json<SendTransferResponse>, crate::error::WalletError> {
    eprintln!("📨 send_transfer_handler called for wallet: {}", wallet_name);
    
    let result = manager.send_transfer(
        &wallet_name,
        &request.invoice,
        request.fee_rate_sat_vb,
    ).map_err(|e| {
        eprintln!("❌ send_transfer failed: {:?}", e);
        e
    })?;
    
    eprintln!("✅ send_transfer_handler succeeded");
    Ok(Json(SendTransferResponse {
        bitcoin_txid: result.bitcoin_txid,
        consignment_download_url: result.consignment_download_url,
        consignment_filename: result.consignment_filename,
        status: result.status,
    }))
}

pub async fn download_consignment_handler(
    State(manager): State<Arc<WalletManager>>,
    Path(filename): Path<String>,
) -> Result<axum::response::Response, crate::error::WalletError> {
    use axum::response::IntoResponse;
    use axum::http::{header, StatusCode};

    let consignment_path = manager.storage.base_dir()
        .join("consignments")
        .join(&filename);

    if !consignment_path.exists() {
        return Err(crate::error::WalletError::Rgb(format!("Consignment file not found: {}", filename)));
    }

    let file_contents = std::fs::read(&consignment_path)
        .map_err(|e| crate::error::WalletError::Internal(format!("Failed to read file: {}", e)))?;

    let response = (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/octet-stream"),
            (header::CONTENT_DISPOSITION, &format!("attachment; filename=\"{}\"", filename)),
        ],
        file_contents,
    ).into_response();

    Ok(response)
}

pub async fn accept_consignment_handler(
    State(manager): State<Arc<WalletManager>>,
    Path(wallet_name): Path<String>,
    body: axum::body::Bytes,
) -> Result<Json<AcceptConsignmentResponse>, crate::error::WalletError> {
    if !manager.storage.wallet_exists(&wallet_name) {
        return Err(crate::error::WalletError::WalletNotFound(wallet_name));
    }

    let result = manager.accept_consignment(&wallet_name, body.to_vec())?;
    Ok(Json(result))
}

pub async fn export_genesis_handler(
    State(manager): State<Arc<WalletManager>>,
    Path((wallet_name, contract_id)): Path<(String, String)>,
) -> Result<Json<ExportGenesisResponse>, crate::error::WalletError> {
    if !manager.storage.wallet_exists(&wallet_name) {
        return Err(crate::error::WalletError::WalletNotFound(wallet_name));
    }

    let result = manager.export_genesis_consignment(&wallet_name, &contract_id)?;
    Ok(Json(result))
}

pub async fn download_genesis_handler(
    State(manager): State<Arc<WalletManager>>,
    Path(filename): Path<String>,
) -> Result<axum::response::Response, crate::error::WalletError> {
    use axum::response::IntoResponse;
    use axum::http::{header, StatusCode};

    let genesis_path = manager.storage.base_dir()
        .join("exports")
        .join(&filename);

    if !genesis_path.exists() {
        return Err(crate::error::WalletError::Rgb(
            format!("Genesis file not found: {}", filename)
        ));
    }

    let file_contents = std::fs::read(&genesis_path)
        .map_err(|e| crate::error::WalletError::Internal(
            format!("Failed to read genesis file: {}", e)
        ))?;

    let response = (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/octet-stream"),
            (header::CONTENT_DISPOSITION, &format!("attachment; filename=\"{}\"", filename)),
        ],
        file_contents,
    ).into_response();

    Ok(response)
}
