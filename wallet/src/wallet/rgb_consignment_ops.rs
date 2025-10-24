/// RGB consignment operations
///
/// Handles RGB consignment acceptance and genesis export.
///
/// Key operations:
/// - accept_consignment: Import genesis or transfer consignments
/// - export_genesis_consignment: Export contract state for cross-device sync
use super::shared::*;
use crate::api::types::{AcceptConsignmentResponse, ExportGenesisResponse};
use crate::error::WalletError;
use std::str::FromStr;

// Import RGB types from the actual RGB crate (not from shared::rgb module)
use ::rgb::{ContractId, WitnessStatus};

/// Accept a consignment (genesis or transfer) by importing it into RGB runtime.
///
/// This function validates and imports RGB consignments, automatically detecting:
/// - Whether it's a genesis (initial contract) or transfer (token movement)
/// - Bitcoin transaction ID for transfers
/// - Transaction confirmation status (pending/confirmed/archived)
///
/// After import, users should sync their wallet to see updated token balances.
pub fn accept_consignment(
    storage: &Storage,
    rgb_runtime_manager: &RgbRuntimeManager,
    wallet_name: &str,
    consignment_bytes: Vec<u8>,
    _sync_fn: impl Fn(&str) -> Result<(), WalletError>,
) -> Result<AcceptConsignmentResponse, WalletError> {
    use std::io::Write;

    log::info!(
        "Accepting consignment for wallet: {}, size: {} bytes",
        wallet_name,
        consignment_bytes.len()
    );

    // 1. Save consignment to temp file
    let temp_dir = storage.base_dir().join("temp_consignments");
    std::fs::create_dir_all(&temp_dir)
        .map_err(|e| WalletError::Internal(format!("Failed to create temp dir: {}", e)))?;

    let temp_filename = format!("accept_{}.rgbc", uuid::Uuid::new_v4());
    let temp_path = temp_dir.join(&temp_filename);

    let mut file = std::fs::File::create(&temp_path)
        .map_err(|e| WalletError::Internal(format!("Failed to create temp file: {}", e)))?;
    file.write_all(&consignment_bytes)
        .map_err(|e| WalletError::Internal(format!("Failed to write consignment: {}", e)))?;
    drop(file);
    log::debug!("Temp consignment file created: {:?}", temp_path);

    // 2. Create ephemeral runtime for consignment import
    log::debug!("Creating ephemeral RGB runtime for consignment import");
    let mut runtime = rgb_runtime_manager.init_runtime_no_sync(wallet_name)?;

    let (contract_id_str, import_type, bitcoin_txid, status) = {
        // 3. Get contract IDs before importing
        let contract_ids_before: std::collections::HashSet<String> = runtime
            .contracts
            .contract_ids()
            .map(|id| id.to_string())
            .collect();
    log::debug!(
        "Found {} existing contract(s) before import",
        contract_ids_before.len()
    );

    // 4. Consume consignment (validates and imports)
    log::info!("Importing consignment file");
    use std::convert::Infallible;
    runtime
        .consume_from_file(
            true, // allow_unknown contracts
            &temp_path,
            |_, _, _| Result::<_, Infallible>::Ok(()),
        )
        .map_err(|e| {
            log::error!("Consignment validation failed: {:?}", e);
            WalletError::Rgb(format!("Validation failed: {:?}", e))
        })?;
    log::info!("Consignment imported successfully");

    // 5. Find new or existing contract(s) that were imported
    let contract_ids_after: std::collections::HashSet<String> = runtime
        .contracts
        .contract_ids()
        .map(|id| id.to_string())
        .collect();

    let new_contracts: Vec<String> = contract_ids_after
        .difference(&contract_ids_before)
        .cloned()
        .collect();
    log::debug!("Found {} new contract(s) after import", new_contracts.len());

    // Determine which contract was imported
    let contract_id_str = if !new_contracts.is_empty() {
        // Case 1: New contract imported
        new_contracts.first().unwrap().clone()
    } else if contract_ids_after.len() == 1 {
        // Case 2: Re-importing into wallet with 1 contract (likely the same one)
        contract_ids_after.iter().next().unwrap().clone()
    } else {
        // Case 3: Can't determine which contract - need to parse the consignment file
        // For now, return an error with helpful message
        return Err(WalletError::Rgb(format!(
            "Contract already exists. Wallet has {} contracts. Cannot determine which was updated.",
            contract_ids_after.len()
        )));
    };
    log::info!("Imported contract: {}", contract_id_str);

    // Parse contract ID for querying
    let contract_id = ContractId::from_str(&contract_id_str)
        .map_err(|e| WalletError::Rgb(format!("Invalid contract ID: {:?}", e)))?;

    // 6. Query imported contract to determine type and extract witness info

    log::debug!("Querying contract witness count");
    let witness_count = runtime.contracts.contract_witness_count(contract_id);
    log::debug!("Contract has {} witness(es)", witness_count);

    let (import_type, bitcoin_txid, status) = if witness_count == 0 {
        // Genesis: no witnesses (no Bitcoin TX)
        ("genesis".to_string(), None, "genesis_imported".to_string())
    } else {
        // Transfer: has witnesses (Bitcoin TXs)
        let witnesses: Vec<_> = runtime.contracts.contract_witnesses(contract_id).collect();

        if let Some(last_witness) = witnesses.last() {
            // For TxoSeal, witness.id IS Txid
            let txid = last_witness.id.to_string();

            // Map witness status to our status string
            let status_str = match last_witness.status {
                WitnessStatus::Genesis => "genesis_imported".to_string(),
                WitnessStatus::Offchain => "offchain".to_string(),
                WitnessStatus::Tentative => "pending".to_string(),
                WitnessStatus::Mined(_) => "confirmed".to_string(),
                WitnessStatus::Archived => "archived".to_string(),
            };

            ("transfer".to_string(), Some(txid), status_str)
        } else {
            // Fallback if witnesses iterator is empty despite count > 0
            ("transfer".to_string(), None, "imported".to_string())
        }
        };

        (contract_id_str, import_type, bitcoin_txid, status)
    };

    // 7. Cleanup temp file
    let _ = std::fs::remove_file(&temp_path);

    // ℹ️  NOTE: We do NOT call update() after accept (RGB CLI doesn't either)
    // The consignment has been imported and saved to disk via FileHolder::drop()
    // The next operation that needs fresh state (like balance query) will call update()
    
    Ok(AcceptConsignmentResponse {
        contract_id: contract_id_str,
        status,
        import_type,
        bitcoin_txid,
    })
    // Runtime drops here → FileHolder::drop() auto-saves to disk
}

/// Export a genesis consignment to share contract knowledge.
///
/// **Two use cases:**
/// 1. **Same wallet, different device**: Sync your contract to another device (same mnemonic).
///    The other device will see the same tokens and UTXOs.
/// 2. **Share with another wallet**: Let someone else know this contract exists (different mnemonic).
///    They learn about the contract but don't own any tokens yet.
///    They can then generate invoices to receive tokens from you.
///
/// **Important:** Genesis consignments share contract definition, not token ownership.
/// No Bitcoin transaction is required.
///
/// To actually transfer token ownership, use `send_transfer()` instead.
pub fn export_genesis_consignment(
    storage: &Storage,
    rgb_runtime_manager: &RgbRuntimeManager,
    wallet_name: &str,
    contract_id_str: &str,
) -> Result<ExportGenesisResponse, WalletError> {
    log::info!(
        "Exporting genesis consignment for wallet: {}, contract: {}",
        wallet_name,
        contract_id_str
    );

    // 1. Parse contract ID
    let contract_id = ContractId::from_str(contract_id_str)
        .map_err(|e| WalletError::Rgb(format!("Invalid contract ID: {:?}", e)))?;

    log::debug!("Parsed contract ID: {}", contract_id);

    // 2. Create consignment directory first
    let consignment_filename = format!("genesis_{}.rgbc", contract_id);
    let exports_dir = storage.base_dir().join("exports");

    std::fs::create_dir_all(&exports_dir)
        .map_err(|e| WalletError::Internal(format!("Failed to create exports directory: {}", e)))?;

    let consignment_path = exports_dir.join(&consignment_filename);
    log::debug!("Export path: {:?}", consignment_path);

    // Remove existing file if present (allow re-export)
    if consignment_path.exists() {
        std::fs::remove_file(&consignment_path).map_err(|e| {
            WalletError::Internal(format!("Failed to remove existing export file: {}", e))
        })?;
        log::debug!("Removed existing export file");
    }

    // 3. Create ephemeral runtime and export
    log::debug!("Creating ephemeral RGB runtime for genesis export");
    let runtime = rgb_runtime_manager.init_runtime_no_sync(wallet_name)?;

    {
        // Verify we have this contract
        if !runtime.contracts.has_contract(contract_id) {
            log::error!("Contract {} not found in wallet", contract_id);
            return Err(WalletError::Rgb(format!(
                "Contract {} not found in wallet",
                contract_id
            )));
        }
        log::debug!("Contract exists in runtime");

        // Get contract state to verify we have allocations
        let state = runtime.contracts.contract_state(contract_id);
        log::debug!("Retrieved contract state");

        // Check if we have any owned states (just for validation)
        let has_allocations = state.owned.values().any(|states| !states.is_empty());

        if !has_allocations {
            log::error!("No allocations found for contract");
            return Err(WalletError::Rgb(
                "No allocations found for contract".to_string(),
            ));
        }
        log::debug!("Contract has allocations");

        // Export complete contract state (no terminals needed)
        // Uses export() instead of consign() - exports all state without requiring destinations
        log::info!(
            "Exporting genesis consignment for contract: {}",
            contract_id
        );
        runtime
            .contracts
            .export_to_file(&consignment_path, contract_id)
            .map_err(|e| {
                log::error!("Genesis export failed: {:?}", e);
                WalletError::Rgb(format!("Failed to export genesis consignment: {:?}", e))
            })?;
        log::info!("Genesis consignment exported successfully");
    }

    // 4. Get file size
    let file_size = std::fs::metadata(&consignment_path)
        .map_err(|e| WalletError::Internal(format!("Failed to read file metadata: {}", e)))?
        .len();

    Ok(ExportGenesisResponse {
        contract_id: contract_id_str.to_string(),
        consignment_filename: consignment_filename.clone(),
        file_size_bytes: file_size,
        download_url: format!("/api/genesis/{}", consignment_filename),
    })
    // Runtime drops here → FileHolder::drop() auto-saves to disk
}
