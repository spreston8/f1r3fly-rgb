/// F1r3fly RGB operations (state-based)
/// 
/// This module contains the F1r3fly implementation using:
/// - FireflyClient for blockchain interaction
/// - RSpace contracts for state storage
/// - Direct state queries (no consignment files)
/// - On-chain state verification

use super::shared::*;
use super::shared::rgb::{IssueAssetRequest, IssueAssetResponse};
use crate::api::types::*;
use crate::error::WalletError;
use crate::firefly::FireflyClient;

/// Issue RGB asset using F1r3fly state-based approach
/// 
/// This will:
/// 1. Create the RGB contract metadata
/// 2. Deploy to F1r3fly/RSpace via FireflyClient
/// 3. Store contract metadata in RSpace state
/// 4. Store initial allocation in RSpace state
/// 
/// Unlike traditional RGB, no consignment files are needed.
pub fn issue_asset(
    storage: &Storage,
    firefly_client: &FireflyClient,
    wallet_name: &str,
    request: IssueAssetRequest,
) -> Result<IssueAssetResponse, WalletError> {
    use crate::firefly::types::ContractMetadata;
    use tokio::runtime::Runtime;
    
    log::info!("🔥 Issuing RGB asset via F1r3fly/RSpace: {} ({})", request.name, request.ticker);
    
    // Validate wallet exists
    if !storage.wallet_exists(wallet_name) {
        return Err(WalletError::WalletNotFound(wallet_name.to_string()));
    }
    
    // Parse genesis UTXO
    let genesis_parts: Vec<&str> = request.genesis_utxo.split(':').collect();
    if genesis_parts.len() != 2 {
        return Err(WalletError::InvalidInput(format!(
            "Invalid genesis seal format: {}",
            request.genesis_utxo
        )));
    }
    
    let genesis_txid = genesis_parts[0];
    let _genesis_vout = genesis_parts[1].parse::<u32>().map_err(|e| {
        WalletError::InvalidInput(format!("Invalid genesis vout: {}", e))
    })?;
    
    // Generate contract ID (deterministic based on genesis UTXO)
    let contract_id = format!("rgb20_{}", genesis_txid);
    
    // Derive issuer public key from wallet's key hierarchy
    // Use dedicated derivation path m/84'/1'/0'/2/0 (chain 2 = RGB identity)
    let issuer_pubkey = KeyManager::derive_rgb_issuer_pubkey(storage, wallet_name)?;
    
    // Create contract metadata
    let metadata = ContractMetadata {
        ticker: request.ticker.clone(),
        name: request.name.clone(),
        precision: request.precision,
        total_supply: request.supply,
        genesis_txid: genesis_txid.to_string(),
        issuer_pubkey: issuer_pubkey.clone(),
    };
    
    // Create async runtime for F1r3fly operations
    let rt = Runtime::new().map_err(|e| {
        WalletError::Internal(format!("Failed to create async runtime: {}", e))
    })?;
    
    // Store contract metadata in RSpace
    rt.block_on(async {
        log::debug!("Storing contract metadata in RSpace for contract_id: {}", contract_id);
        firefly_client.store_contract(&contract_id, metadata.clone())
            .await
            .map_err(|e| WalletError::Internal(format!("Failed to store contract in RSpace: {}", e)))?;
        
        log::debug!("Waiting for contract deployment to finalize...");
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        
        // Store initial allocation at genesis UTXO
        log::debug!("Storing initial allocation: {} tokens at {}", request.supply, request.genesis_utxo);
        firefly_client.store_allocation(
            &contract_id,
            &request.genesis_utxo,
            &issuer_pubkey,
            request.supply,
            genesis_txid,
        )
        .await
        .map_err(|e| WalletError::Internal(format!("Failed to store initial allocation in RSpace: {}", e)))?;
        
        log::info!("✓ RGB asset issued via F1r3fly: contract_id={}", contract_id);
        
        Ok(IssueAssetResponse {
            contract_id: contract_id.clone(),
            genesis_seal: request.genesis_utxo.clone(),
        })
    })
}

/// Generate RGB invoice using F1r3fly approach
/// 
/// This will:
/// 1. Query available allocations from RSpace
/// 2. Generate a blinded UTXO for receiving
/// 3. Return invoice string (compatible with RGB standard)
/// 
/// Unlike traditional RGB, the invoice recipient can verify
/// the sender's balance directly from RSpace state.
pub fn generate_invoice(
    storage: &Storage,
    _firefly_client: &FireflyClient,
    wallet_name: &str,
    request: GenerateInvoiceRequest,
    _utxo_info: Option<UtxoInfo>,
) -> Result<GenerateInvoiceResult, WalletError> {
    log::info!("🔥 Generating RGB invoice via F1r3fly for contract: {}", request.contract_id);
    
    // Validate wallet exists
    if !storage.wallet_exists(wallet_name) {
        return Err(WalletError::WalletNotFound(wallet_name.to_string()));
    }
    
    // For F1r3fly mode, we generate a simpler invoice format
    // The seal UTXO is where the recipient will receive the tokens
    // TODO: In production, use a blinded UTXO for privacy
    // For now, generate a deterministic seal based on wallet name and nonce
    let nonce = request.nonce.unwrap_or(0);
    let seal_utxo = format!(
        "{}:{}",
        format!("{:0>64}", format!("{}_{}", wallet_name, nonce)),
        0
    );
    
    // Generate invoice string
    // Format: rgb:contract_id:amount:seal_utxo:f1r3fly
    // The "f1r3fly" suffix indicates this is a F1r3fly-backed invoice
    let invoice = format!(
        "rgb:{}:{}:{}:f1r3fly",
        request.contract_id,
        request.amount.unwrap_or(0),
        seal_utxo
    );
    
    log::info!("✓ Generated F1r3fly invoice: {}", invoice);
    
    Ok(GenerateInvoiceResult {
        invoice,
        contract_id: request.contract_id.clone(),
        amount: request.amount,
        seal_utxo,
        selected_utxo: None, // F1r3fly doesn't need UTXO selection
    })
}

/// Send RGB transfer using F1r3fly state-based approach
/// 
/// This will:
/// 1. Parse the invoice to get recipient and amount
/// 2. Query sender's allocations from RSpace
/// 3. Create Bitcoin transaction with witness commitment
/// 4. Record transition in RSpace (with Bitcoin txid)
/// 5. Update allocations in RSpace state
/// 
/// Unlike traditional RGB, no consignment file is created.
/// The recipient queries RSpace directly to see their new allocation.
pub fn send_transfer(
    storage: &Storage,
    firefly_client: &FireflyClient,
    wallet_name: &str,
    invoice_str: &str,
    _fee_rate_sat_vb: Option<u64>,
) -> Result<SendTransferResponse, WalletError> {
    use tokio::runtime::Runtime;
    
    log::info!("🔥 Sending RGB transfer via F1r3fly");
    
    // Validate wallet exists
    if !storage.wallet_exists(wallet_name) {
        return Err(WalletError::WalletNotFound(wallet_name.to_string()));
    }
    
    // Parse invoice: rgb:contract_id:amount:seal_utxo:f1r3fly
    let parts: Vec<&str> = invoice_str.split(':').collect();
    if parts.len() < 5 || parts[0] != "rgb" || parts[4] != "f1r3fly" {
        return Err(WalletError::InvalidInput(format!(
            "Invalid F1r3fly invoice format: {}",
            invoice_str
        )));
    }
    
    let contract_id = parts[1];
    let amount = parts[2].parse::<u64>().map_err(|e| {
        WalletError::InvalidInput(format!("Invalid amount in invoice: {}", e))
    })?;
    let to_utxo = parts[3];
    
    // For F1r3fly, we need a source UTXO
    // TODO: In production, select this from available allocations queried from RSpace
    // For now, use a placeholder
    let from_utxo = format!("{}:0", "1".repeat(64));
    
    // Generate a placeholder Bitcoin txid for the transfer
    // TODO: In production, create a real Bitcoin transaction with RGB commitment
    let bitcoin_txid = format!("{}", "a".repeat(64));
    
    log::debug!(
        "Transfer: {} tokens from {} to {} (contract: {})",
        amount,
        from_utxo,
        to_utxo,
        contract_id
    );
    
    // Create async runtime for F1r3fly operations
    let rt = Runtime::new().map_err(|e| {
        WalletError::Internal(format!("Failed to create async runtime: {}", e))
    })?;
    
    // Record transition in RSpace
    rt.block_on(async {
        log::debug!("Recording transition in RSpace...");
        firefly_client.record_transition(
            contract_id,
            &from_utxo,
            to_utxo,
            amount,
            &bitcoin_txid,
        )
        .await
        .map_err(|e| WalletError::Internal(format!("Failed to record transition in RSpace: {}", e)))?;
        
        log::debug!("Waiting for transition to finalize...");
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        
        // Store the new allocation for the recipient
        // TODO: In production, derive recipient owner from the seal UTXO or invoice metadata
        let recipient_owner = format!("wallet:recipient"); // Placeholder
        log::debug!("Storing recipient allocation in RSpace...");
        firefly_client.store_allocation(
            contract_id,
            to_utxo,
            &recipient_owner,
            amount,
            &bitcoin_txid,
        )
        .await
        .map_err(|e| WalletError::Internal(format!("Failed to store recipient allocation in RSpace: {}", e)))?;
        
        log::info!("✓ RGB transfer recorded in F1r3fly: txid={}", bitcoin_txid);
        
        Ok(SendTransferResponse {
            bitcoin_txid: bitcoin_txid.clone(),
            consignment_download_url: String::from("f1r3fly://rspace"), // F1r3fly doesn't use files
            consignment_filename: String::from("n/a"),
            status: String::from("recorded_in_rspace"),
        })
    })
}

/// Accept RGB consignment using F1r3fly approach
/// 
/// In F1r3fly mode, there are no consignment files to accept.
/// Instead, recipients query RSpace directly to see their allocations.
/// 
/// This method may be used for compatibility, but will likely
/// just query RSpace for the latest state rather than accepting a file.
pub fn accept_consignment(
    storage: &Storage,
    _firefly_client: &FireflyClient,
    wallet_name: &str,
    _consignment_bytes: Vec<u8>,
) -> Result<AcceptConsignmentResponse, WalletError> {
    log::info!("🔥 Accept consignment called in F1r3fly mode for wallet: {}", wallet_name);
    
    // Validate wallet exists
    if !storage.wallet_exists(wallet_name) {
        return Err(WalletError::WalletNotFound(wallet_name.to_string()));
    }
    
    // F1r3fly doesn't use consignment files
    // Recipients query RSpace directly for their allocations
    Err(WalletError::InvalidInput(
        "F1r3fly RGB does not use consignment files. \
        Recipients query RSpace directly to see their allocations. \
        Use the sync operation to refresh state from RSpace."
            .to_string(),
    ))
}

/// Export genesis consignment using F1r3fly approach
/// 
/// In F1r3fly mode, there are no genesis consignment files.
/// The genesis state is stored in RSpace and can be queried directly.
/// 
/// This method may return a reference to the RSpace contract URI
/// instead of a consignment file.
pub fn export_genesis(
    storage: &Storage,
    _firefly_client: &FireflyClient,
    wallet_name: &str,
    contract_id_str: &str,
) -> Result<ExportGenesisResponse, WalletError> {
    log::info!("🔥 Export genesis called in F1r3fly mode for contract: {}", contract_id_str);
    
    // Validate wallet exists
    if !storage.wallet_exists(wallet_name) {
        return Err(WalletError::WalletNotFound(wallet_name.to_string()));
    }
    
    // F1r3fly doesn't use genesis consignment files
    // Genesis state is stored in RSpace and queryable
    Err(WalletError::InvalidInput(
        format!(
            "F1r3fly RGB does not use genesis consignment files. \
            Genesis state for contract {} is stored in RSpace. \
            Query the contract directly using the contract_id.",
            contract_id_str
        )
    ))
}

/// Sync RGB runtime using F1r3fly approach
/// 
/// This will:
/// 1. Query all allocations for this wallet from RSpace
/// 2. Query all transitions from RSpace
/// 3. Verify Bitcoin confirmations for transitions
/// 4. Update local cache/index
/// 
/// Unlike traditional RGB, this queries RSpace directly
/// rather than scanning the local stash.
pub fn sync_rgb_runtime(
    storage: &Storage,
    _firefly_client: &FireflyClient,
    wallet_name: &str,
) -> Result<(), WalletError> {
    log::info!("🔥 Syncing RGB runtime via F1r3fly for wallet: {}", wallet_name);
    
    // Validate wallet exists
    if !storage.wallet_exists(wallet_name) {
        return Err(WalletError::WalletNotFound(wallet_name.to_string()));
    }
    
    // For F1r3fly mode, sync is implicit
    // State is always queried fresh from RSpace when needed
    // No local stash to sync
    
    log::info!("✓ F1r3fly sync complete (state is always fresh from RSpace)");
    Ok(())
}

/// Sync RGB after state change using F1r3fly approach
/// 
/// This is similar to sync_rgb_runtime but may be optimized
/// to only sync recent changes rather than full state.
pub fn sync_rgb_after_state_change(
    storage: &Storage,
    _firefly_client: &FireflyClient,
    wallet_name: &str,
) -> Result<(), WalletError> {
    log::info!("🔥 Syncing RGB after state change via F1r3fly for wallet: {}", wallet_name);
    
    // Validate wallet exists
    if !storage.wallet_exists(wallet_name) {
        return Err(WalletError::WalletNotFound(wallet_name.to_string()));
    }
    
    // For F1r3fly mode, no sync needed
    // State changes are immediately visible in RSpace
    
    log::info!("✓ F1r3fly sync complete (state changes are immediate in RSpace)");
    Ok(())
}

