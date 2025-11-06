//! UTXO management operations (create and unlock)

use crate::api::types::{CreateUtxoRequest, CreateUtxoResponse, UnlockUtxoRequest, UnlockUtxoResponse};
use crate::error::WalletError;
use crate::storage::Storage;
use bitcoin::Network;

use crate::wallet::AddressManager;
use crate::bitcoin::{BalanceChecker, balance_checker};
use crate::bitcoin::network::get_network;
use crate::rgb::RgbRuntimeManager;
use super::transaction::{TransactionBuilder, broadcast_transaction};

/// Create a UTXO for RGB operations
pub async fn create_utxo(
    storage: &Storage,
    balance_checker: &BalanceChecker,
    rgb_runtime_manager: &RgbRuntimeManager,
    wallet_name: &str,
    request: CreateUtxoRequest,
) -> Result<CreateUtxoResponse, WalletError> {
    if !storage.wallet_exists(wallet_name) {
        return Err(WalletError::WalletNotFound(wallet_name.to_string()));
    }

    let amount_sats = match request.amount_btc {
        Some(btc) => (btc * 100_000_000.0) as u64,
        None => 30_000,
    };

    let fee_rate = request.fee_rate_sat_vb.unwrap_or(2);
    
    log::info!("Creating UTXO for wallet: {} (amount: {} sats, fee rate: {} sat/vB)", 
        wallet_name, amount_sats, fee_rate);

    let balance = get_balance_for_tx(storage, balance_checker, rgb_runtime_manager, wallet_name).await?;

    if balance.utxos.is_empty() {
        return Err(WalletError::InsufficientFunds(
            "No UTXOs available to create new UTXO".to_string(),
        ));
    }

    let descriptor = storage.load_descriptor(wallet_name)?;
    let mut state = storage.load_state(wallet_name)?;

    // Use public_address_index (0) for RGB UTXO creation
    let target_index = state.public_address_index;
    
    if !state.used_addresses.contains(&target_index) {
        state.used_addresses.push(target_index);
    }

    let network = get_network();
    let recipient_address =
        AddressManager::derive_address(&descriptor, target_index, network)?;

    let tx_builder = TransactionBuilder::new(network);

    let tx = tx_builder.build_send_to_self(
        &balance.utxos,
        amount_sats,
        fee_rate,
        recipient_address.clone(),
    )?;

    let mnemonic = storage.load_mnemonic(wallet_name)?;

    // Sign transaction with correct keys for each UTXO's address index
    let signed_tx = sign_transaction_multi_key(&tx, &balance.utxos, &mnemonic)?;

    let txid = broadcast_transaction(&signed_tx, network).await?;
    
    log::debug!("UTXO transaction broadcast successfully: {}", txid);

    let total_input: u64 = balance
        .utxos
        .iter()
        .filter(|u| {
            signed_tx.input.iter().any(|input| {
                if let Ok(tid) = u.txid.parse::<bitcoin::Txid>() {
                    tid == input.previous_output.txid && u.vout == input.previous_output.vout
                } else {
                    false
                }
            })
        })
        .map(|u| u.amount_sats)
        .sum();

    let fee_sats = total_input
        - signed_tx
            .output
            .iter()
            .map(|o| o.value.to_sat())
            .sum::<u64>();

    storage.save_state(wallet_name, &state)?;
    
    log::info!("UTXO created successfully: txid={}, amount={} sats, fee={} sats, address={}", 
        txid, amount_sats, fee_sats, recipient_address);

    Ok(CreateUtxoResponse {
        txid,
        amount_sats,
        fee_sats,
        target_address: recipient_address.to_string(),
    })
}

/// Unlock (spend) a UTXO back to the wallet
pub async fn unlock_utxo(
    storage: &Storage,
    balance_checker: &BalanceChecker,
    rgb_runtime_manager: &RgbRuntimeManager,
    wallet_name: &str,
    request: UnlockUtxoRequest,
) -> Result<UnlockUtxoResponse, WalletError> {
    if !storage.wallet_exists(wallet_name) {
        return Err(WalletError::WalletNotFound(wallet_name.to_string()));
    }

    let fee_rate = request.fee_rate_sat_vb.unwrap_or(2);

    let balance = get_balance_for_tx(storage, balance_checker, rgb_runtime_manager, wallet_name).await?;

    let target_utxo = balance
        .utxos
        .iter()
        .find(|u| u.txid == request.txid && u.vout == request.vout)
        .ok_or_else(|| {
            WalletError::Internal(format!(
                "UTXO {}:{} not found",
                request.txid, request.vout
            ))
        })?
        .clone();

    let descriptor = storage.load_descriptor(wallet_name)?;
    let mut state = storage.load_state(wallet_name)?;

    // Use internal_next_index for unlock destination
    let destination_index = state.internal_next_index;
    
    log::debug!("Unlocking UTXO to internal address index {}", destination_index);
    
    state.used_addresses.push(destination_index);
    state.internal_next_index += 1;

    let network = get_network();
    let destination_address =
        AddressManager::derive_address(&descriptor, destination_index, network)?;

    let tx_builder = TransactionBuilder::new(network);

    let tx =
        tx_builder.build_unlock_utxo_tx(&target_utxo, destination_address.clone(), fee_rate)?;

    let mnemonic = storage.load_mnemonic(wallet_name)?;

    // Sign transaction with correct key for the UTXO's address index
    let signed_tx =
        sign_transaction_multi_key(&tx, &vec![target_utxo.clone()], &mnemonic)?;

    let txid = broadcast_transaction(&signed_tx, network).await?;

    let fee_sats = target_utxo.amount_sats - signed_tx.output[0].value.to_sat();
    let recovered_sats = signed_tx.output[0].value.to_sat();

    storage.save_state(wallet_name, &state)?;

    Ok(UnlockUtxoResponse {
        txid,
        recovered_sats,
        fee_sats,
    })
}

// ============================================================================
// Helper Functions (shared between UTXO and send operations)
// ============================================================================

/// Helper to get balance for transaction building
/// CRITICAL: Includes RGB state to prevent spending RGB-occupied UTXOs
pub async fn get_balance_for_tx(
    storage: &Storage,
    balance_checker: &BalanceChecker,
    rgb_runtime_manager: &RgbRuntimeManager,
    wallet_name: &str,
) -> Result<balance_checker::BalanceInfo, WalletError> {
    let descriptor = storage.load_descriptor(wallet_name)?;
    const GAP_LIMIT: u32 = 20;
    let network = get_network();
    let addresses_with_indices =
        AddressManager::derive_addresses(&descriptor, 0, GAP_LIMIT, network)?;
    
    // Phase 1: Get Bitcoin balance
    let mut balance = balance_checker.calculate_balance(&addresses_with_indices).await?;
    
    // Phase 2: Get RGB balance to mark occupied UTXOs
    let rgb_mgr_clone = rgb_runtime_manager.clone();
    let wallet_name_clone = wallet_name.to_string();
    
    let rgb_data = tokio::task::spawn_blocking(move || {
        crate::bitcoin::balance::get_rgb_balance_sync(&rgb_mgr_clone, &wallet_name_clone)
    })
    .await
    .map_err(|e| WalletError::Internal(format!("Get RGB balance task panicked: {}", e)))?;
    
    // Phase 3: Merge RGB data to mark occupied UTXOs
    if let Ok(rgb_data) = rgb_data {
        for utxo in &mut balance.utxos {
            let key = format!("{}:{}", utxo.txid, utxo.vout);
            if let Some(assets) = rgb_data.utxo_assets.get(&key) {
                utxo.bound_assets = assets.clone();
                utxo.is_occupied = !assets.is_empty();
            }
        }
        balance.known_contracts = rgb_data.known_contracts;
    } else {
        log::warn!("Failed to get RGB balance for transaction - proceeding with Bitcoin-only balance (RGB UTXOs may be at risk!)");
    }
    
    Ok(balance)
}

/// Derive the private key for a specific BIP84 address index using the wallet's mnemonic
pub fn derive_private_key_for_index(
    mnemonic: &bip39::Mnemonic,
    address_index: u32,
) -> Result<bitcoin::PrivateKey, WalletError> {
    use std::str::FromStr;
    
    let network = get_network();
    let seed = mnemonic.to_seed("");
    let master_key = bitcoin::bip32::Xpriv::new_master(network, &seed)
        .map_err(|e| WalletError::Bitcoin(e.to_string()))?;

    // Use coin type 1 for all test networks (Signet, Regtest, Testnet)
    let coin_type = match network {
        Network::Bitcoin => 0,
        _ => 1,
    };
    // BIP84 path: m/84'/coin_type'/0'/0/index
    let path =
        bitcoin::bip32::DerivationPath::from_str(&format!("m/84'/{}'/0'/0/{}", coin_type, address_index))
            .map_err(|e| WalletError::Bitcoin(e.to_string()))?;

    let derived_key = master_key
        .derive_priv(&bitcoin::secp256k1::Secp256k1::new(), &path)
        .map_err(|e| WalletError::Bitcoin(e.to_string()))?;

    Ok(bitcoin::PrivateKey::new(
        derived_key.private_key,
        network,
    ))
}

/// Sign a transaction with multiple private keys, deriving the correct key for each input based on its UTXO's address index
pub fn sign_transaction_multi_key(
    tx: &bitcoin::Transaction,
    utxos: &[balance_checker::UTXO],
    mnemonic: &bip39::Mnemonic,
) -> Result<bitcoin::Transaction, WalletError> {
    use bitcoin::hashes::Hash;
    use bitcoin::secp256k1::{Message, Secp256k1};
    use bitcoin::sighash::{EcdsaSighashType, SighashCache};
    use bitcoin::PublicKey;

    let mut signed_tx = tx.clone();
    let secp = Secp256k1::new();
    let network = get_network();

    for (input_index, input) in tx.input.iter().enumerate() {
        // Find the UTXO for this input
        let utxo = utxos
            .iter()
            .find(|u| {
                if let Ok(txid) = u.txid.parse::<bitcoin::Txid>() {
                    txid == input.previous_output.txid && u.vout == input.previous_output.vout
                } else {
                    false
                }
            })
            .ok_or_else(|| {
                WalletError::Bitcoin("UTXO not found for input".into())
            })?;

        // Derive the correct private key for this UTXO's address index
        let private_key = derive_private_key_for_index(mnemonic, utxo.address_index)?;
        let public_key = PublicKey::from_private_key(&secp, &private_key);
        let script_pubkey =
            bitcoin::Address::p2wpkh(&public_key.try_into().unwrap(), network)
                .script_pubkey();

        // Create signature for this input
        let mut sighash_cache = SighashCache::new(tx);

        let sighash = sighash_cache
            .p2wpkh_signature_hash(
                input_index,
                &script_pubkey,
                bitcoin::Amount::from_sat(utxo.amount_sats),
                EcdsaSighashType::All,
            )
            .map_err(|e| WalletError::Bitcoin(e.to_string()))?;

        let message = Message::from_digest(sighash.to_byte_array());
        let signature = secp.sign_ecdsa(&message, &private_key.inner);

        let mut sig_with_hashtype = signature.serialize_der().to_vec();
        sig_with_hashtype.push(EcdsaSighashType::All.to_u32() as u8);

        // Add witness data to the input
        signed_tx.input[input_index].witness.push(sig_with_hashtype);
        signed_tx.input[input_index]
            .witness
            .push(public_key.to_bytes());
    }

    Ok(signed_tx)
}

