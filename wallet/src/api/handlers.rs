use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{
    api::{self, types::*},
    wallet::{
        shared::{
            balance::BalanceInfo,
            rgb::{IssueAssetRequest, IssueAssetResponse},
        },
        WalletManager,
    },
};

use super::types::{
    AcceptConsignmentResponse, CreateUtxoRequest, CreateUtxoResponse, CreateWalletRequest,
    DeleteWalletResponse, ExportGenesisResponse, GenerateInvoiceRequest, GenerateInvoiceResponse,
    ImportWalletRequest, SendBitcoinRequest, SendBitcoinResponse, SendTransferRequest,
    SendTransferResponse, UnlockUtxoRequest, UnlockUtxoResponse,
};

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
    // Parse mnemonic string
    let mnemonic = bip39::Mnemonic::parse(&req.mnemonic)
        .map_err(|e| crate::error::WalletError::InvalidInput(format!("Invalid mnemonic: {}", e)))?;

    let wallet_info = manager.import_wallet(&req.name, mnemonic)?;
    Ok(Json(wallet_info))
}

pub async fn list_wallets_handler(
    State(manager): State<Arc<WalletManager>>,
) -> Result<Json<Vec<WalletMetadata>>, crate::error::WalletError> {
    let wallets = manager.list_wallets()?;
    Ok(Json(wallets))
}

pub async fn delete_wallet_handler(
    State(manager): State<Arc<WalletManager>>,
    Path(name): Path<String>,
) -> Result<Json<DeleteWalletResponse>, crate::error::WalletError> {
    manager.delete_wallet(&name)?;

    Ok(Json(DeleteWalletResponse {
        wallet_name: name,
        status: "deleted".to_string(),
    }))
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
    manager.sync_rgb_runtime(&name).await?;
    Ok(Json(()))
}

pub async fn create_utxo_handler(
    State(manager): State<Arc<WalletManager>>,
    Path(name): Path<String>,
    Json(req): Json<CreateUtxoRequest>,
) -> Result<Json<CreateUtxoResponse>, crate::error::WalletError> {
    let manager_req = api::types::CreateUtxoRequest {
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
    let manager_req = api::types::UnlockUtxoRequest {
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
    let manager_req = api::types::SendBitcoinRequest {
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
    log::info!("Starting RGB asset issuance: {} ({})", req.name, req.ticker);
    let result = manager.issue_asset(&name, req).await?;
    Ok(Json(result))
}

pub async fn generate_invoice_handler(
    State(manager): State<Arc<WalletManager>>,
    Path(name): Path<String>,
    Json(req): Json<GenerateInvoiceRequest>,
) -> Result<Json<GenerateInvoiceResponse>, crate::error::WalletError> {
    // Generate invoice (pass the full request with utxo_selection and nonce)
    let result = manager.generate_rgb_invoice(&name, req).await?;

    Ok(Json(GenerateInvoiceResponse {
        invoice: result.invoice,
        contract_id: result.contract_id,
        amount: result.amount,
        seal_utxo: result.seal_utxo,
        selected_utxo: result.selected_utxo,
    }))
}

pub async fn send_transfer_handler(
    State(manager): State<Arc<WalletManager>>,
    Path(wallet_name): Path<String>,
    Json(request): Json<SendTransferRequest>,
) -> Result<Json<SendTransferResponse>, crate::error::WalletError> {
    log::info!("Send transfer initiated for wallet: {}", wallet_name);

    let result = manager
        .send_transfer(&wallet_name, &request.invoice, request.fee_rate_sat_vb)
        .await?;

    log::info!("Send transfer succeeded for wallet: {}", wallet_name);
    Ok(Json(result))
}

pub async fn download_consignment_handler(
    State(manager): State<Arc<WalletManager>>,
    Path(filename): Path<String>,
) -> Result<axum::response::Response, crate::error::WalletError> {
    use axum::http::{header, StatusCode};
    use axum::response::IntoResponse;

    let consignment_path = manager
        .storage
        .base_dir()
        .join("consignments")
        .join(&filename);

    if !consignment_path.exists() {
        return Err(crate::error::WalletError::Rgb(format!(
            "Consignment file not found: {}",
            filename
        )));
    }

    let file_contents = std::fs::read(&consignment_path)
        .map_err(|e| crate::error::WalletError::Internal(format!("Failed to read file: {}", e)))?;

    let response = (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/octet-stream"),
            (
                header::CONTENT_DISPOSITION,
                &format!("attachment; filename=\"{}\"", filename),
            ),
        ],
        file_contents,
    )
        .into_response();

    Ok(response)
}

pub async fn accept_consignment_handler(
    State(manager): State<Arc<WalletManager>>,
    Path(wallet_name): Path<String>,
    body: axum::body::Bytes,
) -> Result<Json<AcceptConsignmentResponse>, crate::error::WalletError> {
    let result = manager
        .accept_consignment(&wallet_name, body.to_vec())
        .await?;
    Ok(Json(result))
}

pub async fn export_genesis_handler(
    State(manager): State<Arc<WalletManager>>,
    Path((wallet_name, contract_id)): Path<(String, String)>,
) -> Result<Json<ExportGenesisResponse>, crate::error::WalletError> {
    let result = manager
        .export_genesis_consignment(&wallet_name, &contract_id)
        .await?;
    Ok(Json(result))
}

pub async fn download_genesis_handler(
    State(manager): State<Arc<WalletManager>>,
    Path(filename): Path<String>,
) -> Result<axum::response::Response, crate::error::WalletError> {
    use axum::http::{header, StatusCode};
    use axum::response::IntoResponse;

    let genesis_path = manager.storage.base_dir().join("exports").join(&filename);

    if !genesis_path.exists() {
        return Err(crate::error::WalletError::Rgb(format!(
            "Genesis file not found: {}",
            filename
        )));
    }

    let file_contents = std::fs::read(&genesis_path).map_err(|e| {
        crate::error::WalletError::Internal(format!("Failed to read genesis file: {}", e))
    })?;

    let response = (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/octet-stream"),
            (
                header::CONTENT_DISPOSITION,
                &format!("attachment; filename=\"{}\"", filename),
            ),
        ],
        file_contents,
    )
        .into_response();

    Ok(response)
}
