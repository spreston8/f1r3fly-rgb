use crate::api::types::{
    GenerateInvoiceRequest, GenerateInvoiceResponse, SendTransferResponse, UtxoInfo,
};
use crate::bitcoin::network::get_network;
use crate::bitcoin::signer::WalletSigner;
use crate::bitcoin::BalanceChecker;
use crate::error::WalletError;
use crate::rgb::RgbRuntimeManager;
/// RGB transfer operations
///
/// Handles RGB invoice generation, asset transfer, PSBT signing, and transaction broadcast.
///
/// Key operations:
/// - generate_rgb_invoice: Creates RGB invoices with blinded seals
/// - send_transfer: Executes RGB transfers (payment + consignment + PSBT signing)
/// - populate_psbt_bip32_derivations: Adds derivation paths to PSBT for signing
/// - Helper functions for signing and broadcasting
use crate::storage::Storage;
use crate::wallet::AddressManager;
use std::str::FromStr;

use ::rgb::ContractId;
use ::rgb_invoice::bp::{AddressPayload, WitnessOut};
use ::rgb_invoice::{RgbBeneficiary, RgbInvoice};

/// Helper: Find which address index a specific UTXO belongs to (async, called by manager)
pub async fn find_utxo_for_selection(
    storage: &Storage,
    balance_checker: &BalanceChecker,
    wallet_name: &str,
    target_txid: &str,
    target_vout: u32,
) -> Result<UtxoInfo, WalletError> {
    // Get wallet balance which includes all UTXOs with their address indices
    let descriptor = storage.load_descriptor(wallet_name)?;
    const GAP_LIMIT: u32 = 20;
    let network = get_network();
    let addresses_with_indices =
        AddressManager::derive_addresses(&descriptor, 0, GAP_LIMIT, network)?;

    let balance = balance_checker
        .calculate_balance(&addresses_with_indices)
        .await?;

    // Find the UTXO
    for utxo in &balance.utxos {
        if utxo.txid == target_txid && utxo.vout == target_vout {
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
    rgb_runtime_manager: &RgbRuntimeManager,
    wallet_name: &str,
    request: GenerateInvoiceRequest,
    utxo_info: Option<UtxoInfo>,
) -> Result<GenerateInvoiceResponse, WalletError> {
    // Verify wallet exists
    if !storage.wallet_exists(wallet_name) {
        return Err(WalletError::WalletNotFound(wallet_name.to_string()));
    }

    // Parse contract ID
    let contract_id = ContractId::from_str(&request.contract_id)
        .map_err(|e| WalletError::InvalidInput(format!("Invalid contract ID: {}", e)))?;

    // Create ephemeral RGB runtime
    let mut runtime = rgb_runtime_manager.init_runtime_no_sync(wallet_name)?;

    // Use native RGB invoice API
    use hypersonic::Consensus;
    use rgb_invoice::RgbBeneficiary;
    use strict_types::StrictVal;

    // Create beneficiary using ephemeral runtime
    let (beneficiary, seal_info, selected_utxo) = {
        // Create beneficiary: WitnessOut (specific UTXO) or AuthToken (automatic)
        if let Some(utxo) = utxo_info.clone() {
            log::info!(
                "Creating WitnessOut-based invoice for specific UTXO: {}:{}",
                utxo.txid,
                utxo.vout
            );

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
            let payload = AddressPayload::from_script(&bpstd_script).map_err(|e| {
                WalletError::InvalidInput(format!("Invalid script for address: {}", e))
            })?;

            // Use nonce from request or default to 0
            let nonce = request.nonce.unwrap_or(0u64);

            // Create WitnessOut
            let witness_out = WitnessOut::new(payload, nonce);
            let seal_display = witness_out.to_string();

            (
                RgbBeneficiary::WitnessOut(witness_out),
                seal_display,
                Some(utxo),
            )
        } else {
            log::info!("Creating AuthToken-based invoice (automatic UTXO selection)");

            // Generate auth token (blinded seal) from public address UTXO
            let nonce = request.nonce.unwrap_or(0u64);
            let auth = match runtime.auth_token(Some(nonce)) {
                Some(token) => token,
                None => {
                    // UTXOs exist but RGB runtime doesn't know about them yet
                    // Quick sync to register the UTXOs with RGB runtime
                    log::info!("Syncing RGB runtime (1 confirmation)");
                    runtime.update(1).map_err(|e| {
                        log::error!("RGB sync failed during invoice generation: {:?}", e);
                        WalletError::Rgb(format!("RGB sync failed: {:?}", e))
                    })?;

                    // Try again after sync
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
            (RgbBeneficiary::Token(auth), seal_display, None)
        }
    };

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

    Ok(GenerateInvoiceResponse {
        invoice: invoice_str,
        contract_id: request.contract_id,
        amount: request.amount,
        seal_utxo: seal_info,
        selected_utxo,
    })
    // Runtime drops here → FileHolder::drop() auto-saves to disk
}

/// Send RGB asset transfer
pub fn send_transfer(
    storage: &Storage,
    rgb_runtime_manager: &RgbRuntimeManager,
    wallet_name: &str,
    invoice_str: &str,
    fee_rate_sat_vb: Option<u64>,
    public_url: &str,
    _sync_fn: impl Fn(&str, u32, &str) -> Result<(), WalletError>,
) -> Result<SendTransferResponse, WalletError> {
    log::info!(
        "Initiating RGB transfer for wallet: {} (using ephemeral runtime)",
        wallet_name
    );

    use bpstd::psbt::TxParams;
    use bpstd::Sats;
    use rgbp::CoinselectStrategy;

    // Parse invoice using native RGB uri feature
    let invoice = RgbInvoice::<ContractId>::from_str(invoice_str).map_err(|e| {
        log::error!("Invalid invoice format: {}", e);
        WalletError::InvalidInput(format!("Invalid invoice: {}", e))
    })?;

    // Create ephemeral runtime
    let mut runtime = rgb_runtime_manager.init_runtime_no_sync(wallet_name)?;

    // Set fee rate (default 1 sat/vB if not provided)
    let fee_sats = fee_rate_sat_vb.unwrap_or(1) * 250; // Rough estimate for typical RGB tx size
    let tx_params = TxParams::with(Sats::from(fee_sats));

    // Use aggregate coinselect strategy
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
        RgbBeneficiary::Token(_) => None,
    };

    // Extract contract ID from invoice scope
    let contract_id = invoice.scope;

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

    // ═══════════════════════════════════════════════════════════════════════════════
    // STEP 1: Create Payment
    // ═══════════════════════════════════════════════════════════════════════════════
    let (psbt, _psbt_meta, txid_str) = {
        // Sync RGB state before creating payment
        // This updates both wallet UTXOs and witness confirmations
        runtime.update(1).map_err(|e| {
            log::error!("Failed to sync RGB state: {:?}", e);
            WalletError::Rgb(format!("Failed to sync RGB state: {:?}", e))
        })?;

        // Pay invoice - this returns PSBT and Payment
        // RGB internally handles DBC commit and includes bundle in contract
        let (psbt, payment) = runtime
            .pay_invoice(&invoice, strategy, tx_params, giveaway)
            .map_err(|e| {
                log::error!("Failed to create payment: {:?}", e);
                WalletError::Rgb(format!("Failed to create payment: {:?}", e))
            })?;

        // Extract psbt_meta for later use with finalize()
        let psbt_meta = payment.psbt_meta.clone();

        // Generate consignment BEFORE signing
        runtime
            .contracts
            .consign_to_file(&consignment_path, contract_id, payment.terminals)
            .map_err(|e| {
                log::error!("Failed to create consignment: {:?}", e);
                WalletError::Rgb(format!("Failed to create consignment: {:?}", e))
            })?;
        log::info!("Consignment created: {}", consignment_filename);

        // Extract txid for response
        let tx = psbt.to_unsigned_tx();
        let txid_str = format!("{}", tx.txid());

        (psbt, psbt_meta, txid_str)
    }; // Runtime #1 drops here → Saves payment bundle to stockpile

    let mut psbt = psbt;

    // Populate BIP32 derivation paths for our inputs (required for signing)
    populate_psbt_bip32_derivations(storage, wallet_name, &mut psbt)?;

    // ═══════════════════════════════════════════════════════════════════════════════
    // STEP 2: Sign PSBT (WITHOUT runtime - just cryptographic signing)
    // ═══════════════════════════════════════════════════════════════════════════════
    let signer = create_signer(storage, wallet_name)?;
    let signed_count = psbt.sign(&signer).map_err(|e| {
        log::error!("PSBT signing failed: {:?}", e);
        WalletError::Rgb(format!("Failed to approve signing: {:?}", e))
    })?;

    if signed_count == 0 {
        log::error!("No PSBT inputs were signed");
        return Err(WalletError::Rgb("Failed to sign any inputs".into()));
    }
    log::info!("Signed {} PSBT input(s)", signed_count);

    // ═══════════════════════════════════════════════════════════════════════════════
    // STEP 3: Finalize & Broadcast
    // ═══════════════════════════════════════════════════════════════════════════════

    // Load descriptor for finalization
    let descriptor_str = storage.load_descriptor(wallet_name)?;
    use bpstd::{Wpkh, XpubDerivable};
    use rgb_descriptors::RgbDescr;
    use std::str::FromStr;
    let xpub = XpubDerivable::from_str(&descriptor_str)
        .map_err(|e| WalletError::InvalidDescriptor(e.to_string()))?;
    let noise = xpub.xpub().chain_code().to_byte_array();
    let rgb_descr = RgbDescr::<XpubDerivable>::new_unfunded(Wpkh::from(xpub), noise);

    // Finalize PSBT (convert partial_sigs to final_witness)
    let finalized_count = psbt.finalize(&rgb_descr);
    log::info!("Finalized {} input(s)", finalized_count);

    if !psbt.is_finalized() {
        return Err(WalletError::Rgb("PSBT not fully finalized".into()));
    }

    // Extract signed transaction
    let tx = psbt.extract().map_err(|e| {
        WalletError::Rgb(format!(
            "Failed to extract transaction: {} unfinalized inputs",
            e.0
        ))
    })?;

    // Broadcast via Esplora
    let tx_hex = format!("{:x}", tx);

    use crate::config::WalletConfig;
    let config = WalletConfig::from_env();
    let base_url = config.esplora_url;

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| WalletError::Network(format!("Failed to create HTTP client: {}", e)))?;

    let response = client
        .post(format!("{}/tx", base_url))
        .header("Content-Type", "text/plain")
        .body(tx_hex)
        .send()
        .map_err(|e| WalletError::Network(format!("Broadcast failed: {}", e)))?;

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

    log::info!("Transaction broadcast successful: {}", txid_str);

    // Return response
    Ok(SendTransferResponse {
        bitcoin_txid: txid_str,
        consignment_download_url: format!(
            "{}/api/consignment/{}",
            public_url, consignment_filename
        ),
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
    let network = get_network();
    let master_key =
        Xpriv::new_master(network, &seed).map_err(|e| WalletError::Bitcoin(e.to_string()))?;

    let fingerprint = master_key.fingerprint(&secp);

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
                let address = bitcoin::Address::p2wpkh(&compressed, network);
                let derived_script = address.script_pubkey();

                // Check if this matches the input's scriptPubKey
                let input_script_bytes = script_pubkey.as_slice();
                let input_script = bitcoin::ScriptBuf::from_bytes(input_script_bytes.to_vec());

                if derived_script == input_script {
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
    let network = get_network();
    Ok(WalletSigner::new(mnemonic, network))
}
