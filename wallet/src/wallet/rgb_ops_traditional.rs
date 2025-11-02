/// Traditional RGB operations (consignment-based)
/// 
/// This module contains the original RGB implementation using:
/// - RGB runtime for validation and signing
/// - Local stash for state storage
/// - Consignment files for state transfer
/// - Manual file sharing (email/IPFS/upload)

use super::shared::*;
use super::shared::rgb::{IssueAssetRequest, IssueAssetResponse};
use crate::api::types::*;
use crate::error::WalletError;
use std::str::FromStr;

/// Issue RGB asset using traditional consignment-based approach
pub fn issue_asset(
    storage: &Storage,
    rgb_runtime_manager: &RgbRuntimeManager,
    wallet_name: &str,
    request: IssueAssetRequest,
) -> Result<IssueAssetResponse, WalletError> {
    if !storage.wallet_exists(wallet_name) {
        return Err(WalletError::WalletNotFound(wallet_name.to_string()));
    }
    
    use super::shared::rgb::{RgbAssignment, RgbCreateParams, RgbIssuer};
    use strict_types::TypeName;
    use chrono::Utc;
    
    log::info!("Issuing RGB asset through ephemeral runtime (Traditional mode)");
    
    // Parse genesis outpoint first
    let genesis_parts: Vec<&str> = request.genesis_utxo.split(':').collect();
    if genesis_parts.len() != 2 {
        return Err(WalletError::InvalidInput(format!(
            "Invalid genesis seal format: {}",
            request.genesis_utxo
        )));
    }
    
    let genesis_txid = bpstd::Txid::from_str(genesis_parts[0]).map_err(|e| {
        WalletError::InvalidInput(format!("Invalid genesis txid: {}", e))
    })?;
    let genesis_vout = genesis_parts[1].parse::<u32>().map_err(|e| {
        WalletError::InvalidInput(format!("Invalid genesis vout: {}", e))
    })?;
    let genesis_outpoint = bpstd::Outpoint::new(genesis_txid, bpstd::Vout::from_u32(genesis_vout));
    
    // Create ephemeral runtime (loads from disk, like RGB CLI)
    log::debug!("Creating ephemeral RGB runtime for issuance");
    let mut runtime = rgb_runtime_manager.init_runtime_no_sync(wallet_name)?;
    
    // ℹ️  NOTE: We do NOT call runtime.update() before issuance (RGB CLI doesn't either)
    // The genesis UTXO will be discovered later during payment via update()
    
    {
        // 1. Load and import RGB20 issuer if needed
        let rgb_data_dir = storage.base_dir().join(wallet_name).join("rgb_data");
        let issuer_path = rgb_data_dir.join("RGB20-FNA.issuer");
        
        // Ensure the issuer file exists (create if missing)
        if !issuer_path.exists() {
            log::debug!("Creating RGB20 issuer file at: {}", issuer_path.display());
            std::fs::write(&issuer_path, super::shared::rgb::RGB20_ISSUER_BYTES).map_err(|e| {
                WalletError::Rgb(format!("Failed to create RGB20 issuer file: {}", e))
            })?;
        }
        
        let issuer = super::shared::rgb::RGB20_ISSUER.get_or_init(|| {
            RgbIssuer::load(&issuer_path, |_, _, _| -> Result<_, std::convert::Infallible> { Ok(()) })
                .expect("Failed to load RGB20 issuer")
        });
        
        let codex_id = issuer.codex_id();
        log::debug!("RGB20 issuer codex ID: {}", codex_id);
        
        // Import issuer if not already in contracts
        if !runtime.contracts.has_issuer(codex_id) {
            log::debug!("Importing RGB20 issuer into runtime contracts");
            runtime.contracts.import_issuer(issuer.clone()).map_err(|e| {
                WalletError::Rgb(format!("Failed to import RGB20 issuer: {:?}", e))
            })?;
        }
        
        // 2. Create contract params
        log::debug!("Creating contract params for: {} ({})", request.name, request.ticker);
        let type_name = TypeName::try_from(request.name.clone()).map_err(|e| {
            WalletError::InvalidInput(format!("Invalid asset name: {:?}", e))
        })?;
        
        let mut params = RgbCreateParams::new_bitcoin_testnet(codex_id, type_name);
        
        // 3. Add global state
        params = params
            .with_global_verified("ticker", request.ticker.as_str())
            .with_global_verified("name", request.name.as_str())
            .with_global_verified("precision", super::shared::rgb::map_precision(request.precision))
            .with_global_verified("issued", request.supply);
        
        // 4. Add owned state (initial allocation at genesis UTXO)
        params.push_owned_unlocked(
            "balance",
            RgbAssignment::new_internal(genesis_outpoint, request.supply),
        );
        
        params.timestamp = Some(Utc::now());
        
        // 5. Issue through runtime - this keeps contracts in sync!
        log::debug!("Issuing contract through runtime.issue() - contracts stay in sync");
        let contract_id = runtime.issue(params).map_err(|e| {
            WalletError::Rgb(format!("Failed to issue contract: {:?}", e))
        })?;
        
        log::info!("✓ Contract issued through runtime: {}", contract_id);
        
        // ℹ️  NOTE: Unlike our previous approach, we do NOT manually add the genesis UTXO here.
        // RGB CLI just calls runtime.issue() and lets the runtime drop.
        // The genesis outpoint is recorded internally by RGB as part of the contract state.
        // When a payment is later created, runtime.update() is called first, which:
        // 1. Scans the blockchain for UTXOs at derived addresses
        // 2. Uses the recorded genesis outpoint to locate the genesis UTXO
        // 3. Populates the UTXO set so pay_invoice() can spend it
        //
        // Manually adding it here was interfering with RGB's internal UTXO tracking.
        
        log::info!("✓ Asset issuance complete - contract state saved");
        
        Ok(IssueAssetResponse {
            contract_id: contract_id.to_string(),
            genesis_seal: request.genesis_utxo.clone(),
        })
    }
    // Runtime drops here → FileHolder::drop() auto-saves to disk (with genesis UTXO!)
}

/// Generate RGB invoice using traditional approach
pub fn generate_invoice(
    storage: &Storage,
    rgb_runtime_manager: &RgbRuntimeManager,
    wallet_name: &str,
    request: GenerateInvoiceRequest,
    utxo_info: Option<UtxoInfo>,
) -> Result<GenerateInvoiceResult, WalletError> {
    super::rgb_transfer_ops::generate_rgb_invoice_sync(
        storage,
        rgb_runtime_manager,
        wallet_name,
        request,
        utxo_info,
    )
}

/// Send RGB transfer using traditional consignment approach
pub fn send_transfer(
    storage: &Storage,
    rgb_runtime_manager: &RgbRuntimeManager,
    wallet_name: &str,
    invoice_str: &str,
    fee_rate_sat_vb: Option<u64>,
    public_url: &str,
) -> Result<SendTransferResponse, WalletError> {
    super::rgb_transfer_ops::send_transfer(
        storage,
        rgb_runtime_manager,
        wallet_name,
        invoice_str,
        fee_rate_sat_vb,
        public_url,
        |wn, conf, msg| {
            super::sync_ops::sync_rgb_internal(storage, rgb_runtime_manager, wn, conf, msg)
        },
    )
}

/// Accept RGB consignment using traditional approach
pub fn accept_consignment(
    storage: &Storage,
    rgb_runtime_manager: &RgbRuntimeManager,
    wallet_name: &str,
    consignment_bytes: Vec<u8>,
) -> Result<AcceptConsignmentResponse, WalletError> {
    super::rgb_consignment_ops::accept_consignment(
        storage,
        rgb_runtime_manager,
        wallet_name,
        consignment_bytes,
        |wn| {
            super::sync_ops::sync_rgb_after_state_change(storage, rgb_runtime_manager, wn)
        },
    )
}

/// Export genesis consignment
pub fn export_genesis(
    storage: &Storage,
    rgb_runtime_manager: &RgbRuntimeManager,
    wallet_name: &str,
    contract_id_str: &str,
    public_url: &str,
) -> Result<ExportGenesisResponse, WalletError> {
    super::rgb_consignment_ops::export_genesis_consignment(
        storage,
        rgb_runtime_manager,
        wallet_name,
        contract_id_str,
        public_url,
    )
}

/// Sync RGB runtime
pub fn sync_rgb_runtime(
    storage: &Storage,
    rgb_runtime_manager: &RgbRuntimeManager,
    wallet_name: &str,
) -> Result<(), WalletError> {
    super::sync_ops::sync_rgb_runtime(storage, rgb_runtime_manager, wallet_name)
}

/// Sync RGB after state change
pub fn sync_rgb_after_state_change(
    storage: &Storage,
    rgb_runtime_manager: &RgbRuntimeManager,
    wallet_name: &str,
) -> Result<(), WalletError> {
    super::sync_ops::sync_rgb_after_state_change(storage, rgb_runtime_manager, wallet_name)
}

