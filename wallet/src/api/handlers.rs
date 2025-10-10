use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::wallet::manager::{AddressInfo, NextAddressInfo, SyncResult, WalletInfo, WalletManager, WalletMetadata};
use crate::wallet::balance::BalanceInfo;
use super::types::{CreateWalletRequest, ImportWalletRequest, CreateUtxoRequest, CreateUtxoResponse, UnlockUtxoRequest, UnlockUtxoResponse};

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

