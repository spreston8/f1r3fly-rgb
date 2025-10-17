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
use crate::api::types::{GenerateInvoiceRequest, GenerateInvoiceResult, SendTransferResponse};
use crate::error::WalletError;
use bitcoin::Network;
use std::str::FromStr;

// Import RGB types from the actual RGB crates (not from shared::rgb module)
use ::rgb::ContractId;
use ::rgb_invoice::RgbInvoice;

/// Generate an RGB invoice for receiving assets
pub async fn generate_rgb_invoice(
    storage: &Storage,
    rgb_runtime_manager: &RgbRuntimeManager,
    wallet_name: &str,
    request: GenerateInvoiceRequest,
    _sync_fn: impl Fn(&str, u32, &str) -> Result<(), WalletError>,
) -> Result<GenerateInvoiceResult, WalletError> {
    // Verify wallet exists
    if !storage.wallet_exists(wallet_name) {
        return Err(WalletError::WalletNotFound(wallet_name.to_string()));
    }

    // Parse contract ID
    let contract_id = ContractId::from_str(&request.contract_id)
        .map_err(|e| WalletError::InvalidInput(format!("Invalid contract ID: {}", e)))?;

    // Load state to get public address index
    let state = storage.load_state(wallet_name)?;
    let public_index = state.public_address_index;
    
    log::debug!("Using public address index {} for RGB invoice", public_index);

    log::debug!("Initializing RGB runtime");
    // Initialize RGB Runtime (try without sync first for speed)
    let mut runtime = rgb_runtime_manager.init_runtime_no_sync(wallet_name)?;
    log::debug!("RGB runtime initialized");

    // Generate auth token (blinded seal) from public address UTXO
    let nonce = 0u64; // Default nonce
    log::debug!("Generating auth token for invoice at index {}", public_index);
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
    log::info!("Auth token generated successfully for public address");

    // Use native RGB invoice API with uri feature
    use hypersonic::Consensus;
    use rgb_invoice::{RgbBeneficiary, RgbInvoice};
    use strict_types::StrictVal;

    // Create beneficiary from auth token
    let beneficiary = RgbBeneficiary::Token(auth);

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

    // Extract seal outpoint for display (auth token encodes the UTXO)
    let seal_outpoint = format!("{}", auth);

    Ok(GenerateInvoiceResult {
        invoice: invoice_str,
        contract_id: request.contract_id,
        amount: request.amount,
        seal_utxo: seal_outpoint,
    })
}

/// Send RGB asset transfer
pub fn send_transfer(
    storage: &Storage,
    rgb_runtime_manager: &RgbRuntimeManager,
    wallet_name: &str,
    invoice_str: &str,
    fee_rate_sat_vb: Option<u64>,
    sync_fn: impl Fn(&str, u32, &str) -> Result<(), WalletError>,
) -> Result<SendTransferResponse, WalletError> {
    log::info!("Initiating RGB transfer for wallet: {}", wallet_name);
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

    // Initialize RGB runtime with fresh state
    let mut runtime = rgb_runtime_manager.init_runtime_no_sync(wallet_name)?;

    // Set fee rate (default 1 sat/vB if not provided)
    let fee_sats = fee_rate_sat_vb.unwrap_or(1) * 250; // Rough estimate for typical RGB tx size
    let tx_params = TxParams::with(Sats::from(fee_sats));
    log::debug!("Transaction fee: {} sats", fee_sats);

    // Use aggregate coinselect strategy (same as RGB CLI default)
    let strategy = CoinselectStrategy::Aggregate;

    // Pay invoice - this returns PSBT and Payment
    // Note: pay_invoice internally handles DBC commit
    log::debug!("Creating payment from invoice");
    let (mut psbt, payment) = runtime
        .pay_invoice(&invoice, strategy, tx_params, None)
        .map_err(|e| {
            log::error!("Failed to create payment: {:?}", e);
            WalletError::Rgb(format!("Failed to create payment: {:?}", e))
        })?;
    log::debug!("Payment created successfully");

    // Extract contract ID from invoice scope
    let contract_id = invoice.scope;
    log::debug!("Contract ID: {}", contract_id);

    // Generate consignment BEFORE signing
    log::debug!("Creating consignment file");
    let consignment_dir = storage.base_dir().join("consignments");
    std::fs::create_dir_all(&consignment_dir)
        .map_err(|e| WalletError::Internal(format!("Failed to create consignments dir: {}", e)))?;

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let consignment_filename = format!("transfer_{}_{}.rgbc", contract_id, timestamp);
    let consignment_path = consignment_dir.join(&consignment_filename);

    runtime
        .contracts
        .consign_to_file(&consignment_path, contract_id, payment.terminals)
        .map_err(|e| {
            log::error!("Failed to create consignment: {:?}", e);
            WalletError::Rgb(format!("Failed to create consignment: {:?}", e))
        })?;
    log::info!("Consignment created: {}", consignment_filename);

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

    // Finalize the PSBT with wallet descriptor
    log::debug!("Finalizing PSBT");
    let finalized_count = psbt.finalize(runtime.wallet.descriptor());
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
    broadcast_tx_hex(&tx_hex)?;
    log::info!("Transaction broadcasted successfully");

    // Note: Frontend will call sync-rgb endpoint after transfer to update balance

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
fn broadcast_tx_hex(tx_hex: &str) -> Result<(), WalletError> {
    let client = reqwest::blocking::Client::new();
    let response = client
        .post("https://mempool.space/signet/api/tx")
        .body(tx_hex.to_string())
        .send()
        .map_err(|e| WalletError::Network(e.to_string()))?;

    if !response.status().is_success() {
        let error_text = response
            .text()
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(WalletError::Network(format!(
            "Broadcast failed: {}",
            error_text
        )));
    }

    Ok(())
}
