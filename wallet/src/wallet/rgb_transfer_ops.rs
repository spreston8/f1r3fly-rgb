/// RGB transfer operations
///
/// Handles RGB invoice generation, asset transfer, PSBT signing, and transaction broadcast.
///
/// Key operations:
/// - generate_rgb_invoice: Creates RGB invoices with blinded seals
/// - send_transfer: Executes RGB transfers (payment + consignment + PSBT signing)
/// - populate_psbt_bip32_derivations: Adds derivation paths to PSBT for signing
/// - Helper functions for signing and broadcasting
use super::shared::*;
use crate::api::types::{GenerateInvoiceRequest, GenerateInvoiceResult, SendTransferResponse, UtxoInfo};
use crate::error::WalletError;
use bitcoin::Network;
use std::str::FromStr;

// Import RGB types from the actual RGB crates (not from shared::rgb module)
use ::rgb::ContractId;
use ::rgb_invoice::{RgbInvoice, RgbBeneficiary};
use ::rgb_invoice::bp::{WitnessOut, AddressPayload};

/// Helper: Find which address index a specific UTXO belongs to (async, called by manager)
pub async fn find_utxo_for_selection(
    storage: &Storage,
    balance_checker: &BalanceChecker,
    wallet_name: &str,
    target_txid: &str,
    target_vout: u32,
) -> Result<UtxoInfo, WalletError> {
    log::debug!("Finding UTXO info for: {}:{}", target_txid, target_vout);
    
    // Get wallet balance which includes all UTXOs with their address indices
    let descriptor = storage.load_descriptor(wallet_name)?;
    const GAP_LIMIT: u32 = 20;
    let addresses_with_indices =
        AddressManager::derive_addresses(&descriptor, 0, GAP_LIMIT, Network::Signet)?;

    let balance = balance_checker
        .calculate_balance(&addresses_with_indices)
        .await?;

    // Find the UTXO
    for utxo in &balance.utxos {
        if utxo.txid == target_txid && utxo.vout == target_vout {
            log::debug!("Found UTXO at address index {} with {} sats", utxo.address_index, utxo.amount_sats);
            return Ok(UtxoInfo {
                txid: utxo.txid.clone(),
                vout: utxo.vout,
                amount_sats: utxo.amount_sats,
                address: utxo.address.clone(),
                confirmations: utxo.confirmations,
            });
        }
    }

    Err(WalletError::InvalidInput(format!(
        "UTXO {}:{} not found in wallet",
        target_txid, target_vout
    )))
}

/// Generate an RGB invoice for receiving assets (sync version)
/// If utxo_info is provided, creates a WitnessOut-based invoice for that specific UTXO.
/// Otherwise, creates an AuthToken-based invoice using the public address.
pub fn generate_rgb_invoice_sync(
    storage: &Storage,
    rgb_runtime_cache: &std::sync::Arc<super::shared::RgbRuntimeCache>,
    wallet_name: &str,
    request: GenerateInvoiceRequest,
    utxo_info: Option<UtxoInfo>,
) -> Result<GenerateInvoiceResult, WalletError> {
    // Verify wallet exists
    if !storage.wallet_exists(wallet_name) {
        return Err(WalletError::WalletNotFound(wallet_name.to_string()));
    }

    // Parse contract ID
    let contract_id = ContractId::from_str(&request.contract_id)
        .map_err(|e| WalletError::InvalidInput(format!("Invalid contract ID: {}", e)))?;

    log::debug!("Acquiring cached RGB runtime for invoice generation");
    // Get cached RGB runtime (Phase 2)
    let guard = rgb_runtime_cache.get_or_create(wallet_name)?;
    log::debug!("RGB runtime acquired");

    // Use native RGB invoice API
    use hypersonic::Consensus;
    use rgb_invoice::RgbBeneficiary;
    use strict_types::StrictVal;

    // Create beneficiary using cached runtime (Phase 2)
    let (beneficiary, seal_info, selected_utxo) = guard.execute(|runtime| {
        // Create beneficiary: WitnessOut (specific UTXO) or AuthToken (automatic)
        if let Some(utxo) = utxo_info.clone() {
        log::info!("Creating WitnessOut-based invoice for specific UTXO: {}:{}", utxo.txid, utxo.vout);
        
        // Parse the address to get script pubkey, then convert to AddressPayload
        let address = bitcoin::Address::from_str(&utxo.address)
            .map_err(|e| WalletError::InvalidInput(format!("Invalid address: {}", e)))?
            .assume_checked();
        
        // Convert bitcoin script_pubkey to bpstd ScriptPubkey, then to AddressPayload
        let script_pubkey = address.script_pubkey();
        let script_vec = script_pubkey.to_bytes();
        // Convert Vec<u8> to ScriptBytes, then to ScriptPubkey
        let script_bytes = bpstd::ScriptBytes::try_from(script_vec)
            .map_err(|e| WalletError::InvalidInput(format!("Invalid script bytes: {:?}", e)))?;
        let bpstd_script = bpstd::ScriptPubkey::from(script_bytes);
        let payload = AddressPayload::from_script(&bpstd_script)
            .map_err(|e| WalletError::InvalidInput(format!("Invalid script for address: {}", e)))?;
        
        // Use nonce from request or default to 0
        let nonce = request.nonce.unwrap_or(0u64);
        log::debug!("Creating WitnessOut with nonce: {}", nonce);
        
        // Create WitnessOut
        let witness_out = WitnessOut::new(payload, nonce);
        let seal_display = witness_out.to_string();
        log::debug!("WitnessOut created: {}", seal_display);
        
        Ok((RgbBeneficiary::WitnessOut(witness_out), seal_display, Some(utxo)))
    } else {
        log::info!("Creating AuthToken-based invoice (automatic UTXO selection)");
        
        // Load state to get public address index
        let state = storage.load_state(wallet_name)?;
        let public_index = state.public_address_index;
        log::debug!("Using public address index {} for RGB invoice", public_index);
        
        // Generate auth token (blinded seal) from public address UTXO
        let nonce = request.nonce.unwrap_or(0u64);
        log::debug!("Generating auth token with nonce: {}", nonce);
        let auth = match runtime.auth_token(Some(nonce)) {
            Some(token) => {
                log::debug!("Auth token generated without sync");
                token
            }
            None => {
                log::info!("Auth token unavailable, syncing RGB runtime");
                // UTXOs exist but RGB runtime doesn't know about them yet
                // Quick sync to register the UTXOs with RGB runtime
                log::info!("Syncing RGB runtime (1 confirmation)");
                runtime.update(1).map_err(|e| {
                    log::error!("RGB sync failed during invoice generation: {:?}", e);
                    WalletError::Rgb(format!("RGB sync failed: {:?}", e))
                })?;
                log::debug!("RGB sync completed");

                // Try again after sync
                log::debug!("Retrying auth token generation after sync");
                runtime.auth_token(Some(nonce)).ok_or_else(|| {
                    log::error!("Failed to create auth token even after sync");
                    WalletError::Rgb(
                        "No UTXO available at your wallet address. Use the 'Create UTXO' button to prepare for receiving RGB tokens."
                            .to_string(),
                    )
                })?
            }
        };
        
        let seal_display = auth.to_string();
        log::info!("Auth token generated successfully");
        
        // Return None for auto mode since we don't know which UTXO was selected
        // (RGB's auth_token() doesn't expose the underlying UTXO selection)
        Ok((RgbBeneficiary::Token(auth), seal_display, None))
        }
    })?;

    // Create amount as StrictVal if provided
    let amount_val = request.amount.map(StrictVal::num);

    // Create invoice using RGB native API
    let invoice = RgbInvoice::new(
        contract_id,
        Consensus::Bitcoin,
        true, // testnet = true for signet
        beneficiary,
        amount_val,
    );

    // Serialize to URI string using native Display implementation
    let invoice_str = invoice.to_string();

    log::info!("Invoice generated successfully: {}", seal_info);

    Ok(GenerateInvoiceResult {
        invoice: invoice_str,
        contract_id: request.contract_id,
        amount: request.amount,
        seal_utxo: seal_info,
        selected_utxo,
    })
}

/// Send RGB asset transfer (Phase 2: Using long-lived runtime cache)
pub fn send_transfer(
    storage: &Storage,
    rgb_runtime_cache: &std::sync::Arc<super::shared::RgbRuntimeCache>,
    wallet_name: &str,
    invoice_str: &str,
    fee_rate_sat_vb: Option<u64>,
    sync_fn: impl Fn(&str, u32, &str) -> Result<(), WalletError>,
) -> Result<SendTransferResponse, WalletError> {
    log::info!("Initiating RGB transfer for wallet: {} (using cached runtime)", wallet_name);
    log::debug!("Invoice: {}", invoice_str);

    use bpstd::psbt::{PsbtConstructor, TxParams};
    use bpstd::Sats;
    use rgbp::CoinselectStrategy;

    // Parse invoice using native RGB uri feature
    log::debug!("Parsing RGB invoice");
    let invoice = RgbInvoice::<ContractId>::from_str(invoice_str).map_err(|e| {
        log::error!("Invalid invoice format: {}", e);
        WalletError::InvalidInput(format!("Invalid invoice: {}", e))
    })?;
    log::debug!("Invoice parsed successfully");

    // Sync RGB state to ensure we have fresh token balances
    // This prevents "StateInsufficient" errors from stale cache
    sync_fn(wallet_name, 1, "Syncing RGB state for transfer")?;

    // Get cached runtime for this wallet
    log::debug!("Acquiring cached RGB runtime");
    let guard = rgb_runtime_cache.get_or_create(wallet_name)?;

    // Set fee rate (default 1 sat/vB if not provided)
    let fee_sats = fee_rate_sat_vb.unwrap_or(1) * 250; // Rough estimate for typical RGB tx size
    let tx_params = TxParams::with(Sats::from(fee_sats));
    log::debug!("Transaction fee: {} sats", fee_sats);

    // Use aggregate coinselect strategy (same as RGB CLI default)
    let strategy = CoinselectStrategy::Aggregate;

    // Determine giveaway amount for WitnessOut invoices
    // WitnessOut invoices create a new UTXO at the recipient's address,
    // which requires sending Bitcoin sats as "giveaway" to fund the UTXO.
    // AuthToken invoices use existing UTXOs, so no giveaway is needed.
    let giveaway = match &invoice.auth {
        RgbBeneficiary::WitnessOut(_) => {
            let amount = Sats::from(5000u64); // 5000 sats default (safe for dust limit + fees)
            log::info!(
                "WitnessOut invoice detected - providing giveaway: {} sats to create recipient UTXO",
                5000
            );
            Some(amount)
        }
        RgbBeneficiary::Token(_) => {
            log::debug!("AuthToken invoice - no giveaway needed (uses existing UTXO)");
            None
        }
    };

    // Extract contract ID from invoice scope
    let contract_id = invoice.scope;
    log::debug!("Contract ID: {}", contract_id);

    // Prepare consignment directory and filename
    let consignment_dir = storage.base_dir().join("consignments");
    std::fs::create_dir_all(&consignment_dir)
        .map_err(|e| WalletError::Internal(format!("Failed to create consignments dir: {}", e)))?;

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let consignment_filename = format!("transfer_{}_{}.rgbc", contract_id, timestamp);
    let consignment_path = consignment_dir.join(&consignment_filename);

    // Execute RGB operations within cached runtime
    // Phase 2: Long-lived runtime automatically maintains seals across operations
    // No manual seal registration needed - the runtime's in-memory pile preserves seal state
    log::debug!("Executing RGB transfer operations on cached runtime");
    let (psbt, consignment_filename_result, descriptor) = guard.execute(|runtime| {
        // Pay invoice - this returns PSBT and Payment
        // Note: pay_invoice internally handles DBC commit and seal creation
        log::debug!("Creating payment from invoice");
        let (psbt, payment) = runtime
            .pay_invoice(&invoice, strategy, tx_params, giveaway)
            .map_err(|e| {
                log::error!("Failed to create payment: {:?}", e);
                WalletError::Rgb(format!("Failed to create payment: {:?}", e))
            })?;
        log::debug!("Payment created successfully");

        // Generate consignment BEFORE signing
        log::debug!("Creating consignment file");
        runtime
            .contracts
            .consign_to_file(&consignment_path, contract_id, payment.terminals)
            .map_err(|e| {
                log::error!("Failed to create consignment: {:?}", e);
                WalletError::Rgb(format!("Failed to create consignment: {:?}", e))
            })?;
        log::info!("Consignment created: {}", consignment_filename);

        // Sync witness state - tracks all seals (including new change seals) for confirmations
        // The long-lived runtime maintains seal state in memory, so update() just refreshes witness status
        log::debug!("Syncing RGB witness state");
        runtime.update(1).map_err(|e| {
            log::error!("Failed to sync RGB witness state: {:?}", e);
            WalletError::Rgb(format!("Failed to sync RGB state: {:?}", e))
        })?;
        log::info!("RGB witness state synced");

        // Return psbt, consignment filename, and descriptor for finalization outside the guard
        // (finalization happens after signing, which is done outside the guard)
        let descriptor = runtime.wallet.descriptor().clone();
        Ok((psbt, consignment_filename.clone(), descriptor))
    })?;
    let consignment_filename = consignment_filename_result;
    let mut psbt = psbt;

    // Debug: Inspect PSBT before signing
    log::debug!("Inspecting PSBT before signing");
    let inputs_vec: Vec<_> = psbt.inputs().collect();
    log::debug!("PSBT has {} input(s)", inputs_vec.len());
    for (i, input) in inputs_vec.iter().enumerate() {
        log::debug!(
            "  Input {}: witness_utxo={}, bip32_derivations={}",
            i,
            input.witness_utxo.is_some(),
            input.bip32_derivation.len()
        );
        if let Some(ref utxo) = input.witness_utxo {
            log::debug!(
                "    UTXO amount: {} sats, scriptPubKey: {:x}",
                utxo.value,
                utxo.script_pubkey
            );
        }
        for (pubkey, origin) in &input.bip32_derivation {
            log::debug!(
                "    PubKey: {:x}, Derivation: {}",
                pubkey,
                origin.as_derivation()
            );
        }
    }

    // Populate BIP32 derivation paths for our inputs (required for signing)
    log::debug!("Populating BIP32 derivation paths");
    populate_psbt_bip32_derivations(storage, wallet_name, &mut psbt)?;
    log::debug!("BIP32 derivations populated");

    // Debug: Inspect PSBT after populating derivations
    for (i, input) in psbt.inputs().enumerate() {
        log::debug!(
            "Input {} has {} BIP32 derivation(s)",
            i,
            input.bip32_derivation.len()
        );
        for (pk, origin) in &input.bip32_derivation {
            log::debug!("  Derivation entry: pk={:x}, path={}", pk, origin.as_derivation());
        }
    }

    // Sign the PSBT using our wallet signer
    log::debug!("Signing PSBT with {} input(s)", psbt.inputs().count());
    let signer = create_signer(storage, wallet_name)?;
    let signed_count = psbt.sign(&signer).map_err(|e| {
        log::error!("PSBT signing failed: {:?}", e);
        WalletError::Rgb(format!("Failed to approve signing: {:?}", e))
    })?;
    log::info!("Signed {} PSBT input(s)", signed_count);

    if signed_count == 0 {
        log::error!("No PSBT inputs were signed");
        return Err(WalletError::Rgb("Failed to sign any inputs".into()));
    }

    // Finalize the PSBT with wallet descriptor (from cached runtime)
    log::debug!("Finalizing PSBT");
    let finalized_count = psbt.finalize(&descriptor);
    log::debug!("Finalized {} input(s)", finalized_count);

    if finalized_count == 0 {
        log::error!("No PSBT inputs were finalized");
        return Err(WalletError::Rgb("Failed to finalize any inputs".into()));
    }

    // Extract the signed transaction
    log::debug!("Extracting transaction from PSBT");
    let bpstd_tx = psbt.extract().map_err(|e| {
        log::error!("{} non-finalized inputs remain", e.0);
        WalletError::Rgb(format!(
            "Failed to extract transaction: {} non-finalized inputs remain",
            e.0
        ))
    })?;

    // Convert bpstd::Tx to hex string using :x format specifier
    // bpstd::Tx implements Display with :x formatting
    let tx_hex = format!("{:x}", bpstd_tx);

    // Get txid from bpstd::Tx
    let txid = bpstd_tx.txid().to_string();
    log::info!("Transaction extracted - txid: {}", txid);

    // Broadcast transaction
    log::info!("Broadcasting transaction to network");
    let broadcast_txid = broadcast_tx_hex(&tx_hex)?;
    log::info!("Transaction broadcast successful: {}", broadcast_txid);
    
    // Verify broadcast txid matches extracted txid
    if broadcast_txid != txid {
        log::warn!(
            "Broadcast txid ({}) differs from extracted txid ({})",
            broadcast_txid, txid
        );
    }

    // Phase 2: Cached runtime automatically saves state when marked dirty
    // The guard.execute() call already synced witness state via runtime.update(1)
    // Seals remain in the long-lived runtime's pile for future operations
    
    // Return response
    Ok(SendTransferResponse {
        bitcoin_txid: txid,
        consignment_download_url: format!("/api/consignment/{}", consignment_filename),
        consignment_filename,
        status: "broadcasted".to_string(),
    })
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Populate BIP32 derivation paths for PSBT inputs
/// This is required for signing - the signer needs to know which key to use for each input
fn populate_psbt_bip32_derivations(
    storage: &Storage,
    wallet_name: &str,
    psbt: &mut bpstd::Psbt,
) -> Result<(), WalletError> {
    use bitcoin::bip32::{ChildNumber, DerivationPath, Xpriv, Xpub};
    use bitcoin::secp256k1::Secp256k1;
    use std::str::FromStr;

    // Load mnemonic and derive keys
    let mnemonic = storage.load_mnemonic(wallet_name)?;
    let seed = mnemonic.to_seed("");
    let secp = Secp256k1::new();
    let master_key = Xpriv::new_master(Network::Signet, &seed)
        .map_err(|e| WalletError::Bitcoin(e.to_string()))?;

    let fingerprint = master_key.fingerprint(&secp);
    log::debug!("Master fingerprint: {:08x}", fingerprint);

    // Derive to account level (m/84'/1'/0')
    let account_path =
        DerivationPath::from_str("m/84'/1'/0'").map_err(|e| WalletError::Bitcoin(e.to_string()))?;
    let account_key = master_key
        .derive_priv(&secp, &account_path)
        .map_err(|e| WalletError::Bitcoin(e.to_string()))?;

    // Convert fingerprint to bytes for bpstd
    let fingerprint_bytes = fingerprint.to_bytes();

    // Collect derivation info for each input first
    let mut derivation_info = Vec::new();

    for (input_idx, input) in psbt.inputs().enumerate() {
        let witness_utxo = match &input.witness_utxo {
            Some(utxo) => utxo,
            None => {
                log::warn!("Input {} has no witness_utxo, skipping", input_idx);
                continue;
            }
        };

        let script_pubkey = &witness_utxo.script_pubkey;
        log::debug!(
            "Input {}: Searching for derivation of {:x}",
            input_idx,
            script_pubkey
        );

        // Search through derivation indices to find matching address
        let mut found = false;
        'outer: for chain in [0u32, 1u32] {
            for index in 0..1000u32 {
                // Derive key at m/84'/1'/0'/{chain}/{index}
                let external_child = ChildNumber::from_normal_idx(chain)
                    .map_err(|e| WalletError::Bitcoin(e.to_string()))?;
                let child_number = ChildNumber::from_normal_idx(index)
                    .map_err(|e| WalletError::Bitcoin(e.to_string()))?;

                let derived_key = account_key
                    .derive_priv(&secp, &[external_child, child_number])
                    .map_err(|e| WalletError::Bitcoin(e.to_string()))?;

                // Get public key from derived private key
                let xpub = Xpub::from_priv(&secp, &derived_key);
                let pubkey = bitcoin::PublicKey::new(xpub.public_key);

                // Convert to P2WPKH script
                let compressed = bitcoin::key::CompressedPublicKey::try_from(pubkey)
                    .map_err(|e| WalletError::Bitcoin(e.to_string()))?;
                let address = bitcoin::Address::p2wpkh(&compressed, Network::Signet);
                let derived_script = address.script_pubkey();

                // Check if this matches the input's scriptPubKey
                let input_script_bytes = script_pubkey.as_slice();
                let input_script = bitcoin::ScriptBuf::from_bytes(input_script_bytes.to_vec());

                if derived_script == input_script {
                    log::debug!("Found match at m/84'/1'/0'/{}/{}", chain, index);

                    // Store derivation info to apply later
                    let pubkey_bytes = pubkey.to_bytes();
                    derivation_info.push((input_idx, pubkey_bytes, chain, index));

                    found = true;
                    break 'outer;
                }
            }
        }

        if !found {
            log::error!("No matching derivation found for input {}", input_idx);
            return Err(WalletError::Bitcoin(format!(
                "Could not find derivation path for input {}",
                input_idx
            )));
        }
    }

    // Now apply derivation info to mutable inputs
    let mut inputs_mut_vec: Vec<_> = psbt.inputs_mut().collect();
    for (input_idx, pubkey_bytes, chain, index) in derivation_info {
        let input = &mut inputs_mut_vec[input_idx];

        // Build full derivation path from master
        let full_path_str = format!("84h/1h/0h/{}/{}", chain, index);
        let bpstd_path = bpstd::DerivationPath::from_str(&full_path_str)
            .map_err(|e| WalletError::Bitcoin(format!("Invalid path: {:?}", e)))?;

        // Create bpstd types
        let bpstd_pk = bpstd::LegacyPk::from_bytes(&pubkey_bytes)
            .map_err(|e| WalletError::Bitcoin(format!("Invalid pubkey: {:?}", e)))?;

        // Construct KeyOrigin
        let key_origin = bpstd::KeyOrigin::new(fingerprint_bytes.into(), bpstd_path);
        input.bip32_derivation.insert(bpstd_pk, key_origin);
    }

    Ok(())
}

/// Create a wallet signer for PSBT signing
fn create_signer(storage: &Storage, wallet_name: &str) -> Result<WalletSigner, WalletError> {
    let mnemonic = storage.load_mnemonic(wallet_name)?;
    Ok(WalletSigner::new(mnemonic, Network::Signet))
}

/// Broadcast transaction hex to mempool.space
/// Simple blocking HTTP call - will be run on blocking thread pool via spawn_blocking
fn broadcast_tx_hex(tx_hex: &str) -> Result<String, WalletError> {
    log::debug!("Broadcasting transaction to mempool.space");
    
    // Simple blocking HTTP client with timeout
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| WalletError::Network(format!("Failed to create HTTP client: {}", e)))?;
    
    let response = client
        .post("https://mempool.space/signet/api/tx")
        .header("Content-Type", "text/plain")
        .body(tx_hex.to_string())
        .send()
        .map_err(|e| WalletError::Network(format!("Broadcast request failed: {}", e)))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response
            .text()
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(WalletError::Network(format!(
            "Broadcast failed with status {}: {}",
            status, error_text
        )));
    }

    let txid = response
        .text()
        .map_err(|e| WalletError::Network(format!("Failed to read response: {}", e)))?;
    
    log::info!("Transaction broadcast successful: {}", txid.trim());
    Ok(txid.trim().to_string())
}
