/// Bitcoin transaction operations
/// 
/// Handles Bitcoin sending, UTXO creation/unlocking, and transaction signing.

use super::shared::*;
use crate::api::types::{
    CreateUtxoRequest, CreateUtxoResult,
    SendBitcoinRequest, SendBitcoinResponse,
    UnlockUtxoRequest, UnlockUtxoResult,
};
use crate::error::WalletError;
use bitcoin::Network;
use std::str::FromStr;

/// Create a UTXO for RGB operations
pub async fn create_utxo(
    storage: &Storage,
    balance_checker: &BalanceChecker,
    wallet_name: &str,
    request: CreateUtxoRequest,
) -> Result<CreateUtxoResult, WalletError> {
    if !storage.wallet_exists(wallet_name) {
        return Err(WalletError::WalletNotFound(wallet_name.to_string()));
    }

    let amount_sats = match request.amount_btc {
        Some(btc) => (btc * 100_000_000.0) as u64,
        None => 30_000,
    };

    let fee_rate = request.fee_rate_sat_vb.unwrap_or(2);

    let balance = get_balance_for_tx(storage, balance_checker, wallet_name).await?;

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

    let recipient_address =
        AddressManager::derive_address(&descriptor, target_index, Network::Signet)?;

    let tx_builder = transaction::TransactionBuilder::new(Network::Signet);

    let tx = tx_builder.build_send_to_self(
        &balance.utxos,
        amount_sats,
        fee_rate,
        recipient_address.clone(),
    )?;

    let mnemonic = storage.load_mnemonic(wallet_name)?;

    // Sign transaction with correct keys for each UTXO's address index
    let signed_tx = sign_transaction_multi_key(&tx, &balance.utxos, &mnemonic)?;

    let txid = transaction::broadcast_transaction(&signed_tx, Network::Signet).await?;

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

    Ok(CreateUtxoResult {
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
    wallet_name: &str,
    request: UnlockUtxoRequest,
) -> Result<UnlockUtxoResult, WalletError> {
    if !storage.wallet_exists(wallet_name) {
        return Err(WalletError::WalletNotFound(wallet_name.to_string()));
    }

    let fee_rate = request.fee_rate_sat_vb.unwrap_or(2);

    let balance = get_balance_for_tx(storage, balance_checker, wallet_name).await?;

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

    let destination_address =
        AddressManager::derive_address(&descriptor, destination_index, Network::Signet)?;

    let tx_builder = transaction::TransactionBuilder::new(Network::Signet);

    let tx =
        tx_builder.build_unlock_utxo_tx(&target_utxo, destination_address.clone(), fee_rate)?;

    let mnemonic = storage.load_mnemonic(wallet_name)?;

    // Sign transaction with correct key for the UTXO's address index
    let signed_tx =
        sign_transaction_multi_key(&tx, &vec![target_utxo.clone()], &mnemonic)?;

    let txid = transaction::broadcast_transaction(&signed_tx, Network::Signet).await?;

    let fee_sats = target_utxo.amount_sats - signed_tx.output[0].value.to_sat();
    let recovered_sats = signed_tx.output[0].value.to_sat();

    storage.save_state(wallet_name, &state)?;

    Ok(UnlockUtxoResult {
        txid,
        recovered_sats,
        fee_sats,
    })
}

/// Send Bitcoin to an address
pub async fn send_bitcoin(
    storage: &Storage,
    balance_checker: &BalanceChecker,
    wallet_name: &str,
    request: SendBitcoinRequest,
) -> Result<SendBitcoinResponse, WalletError> {
    log::info!("Sending Bitcoin from wallet: {} to address: {}, amount: {} sats", 
        wallet_name, request.to_address, request.amount_sats);

    if !storage.wallet_exists(wallet_name) {
        return Err(WalletError::WalletNotFound(wallet_name.to_string()));
    }

    // Parse destination address
    let to_address = bitcoin::Address::from_str(&request.to_address)
        .map_err(|e| {
            WalletError::InvalidInput(format!("Invalid address: {}", e))
        })?
        .require_network(Network::Signet)
        .map_err(|e| {
            WalletError::InvalidInput(format!("Address network mismatch: {}", e))
        })?;

    let fee_rate = request.fee_rate_sat_vb.unwrap_or(2);
    let balance = get_balance_for_tx(storage, balance_checker, wallet_name).await?;

    if balance.utxos.is_empty() {
        return Err(WalletError::InsufficientFunds(
            "No UTXOs available to send Bitcoin".to_string(),
        ));
    }

    // Calculate total available (excluding RGB-occupied UTXOs)
    let available_sats: u64 = balance
        .utxos
        .iter()
        .filter(|u| !u.is_occupied && u.confirmations > 0)
        .map(|u| u.amount_sats)
        .sum();

    let estimated_fee = fee_rate * 150; // Rough estimate for 1 input, 2 outputs
    let total_needed = request.amount_sats + estimated_fee;

    if available_sats < total_needed {
        return Err(WalletError::InsufficientFunds(format!(
            "Insufficient funds. Available: {} sats, needed: {} sats (including ~{} sats fee)",
            available_sats, total_needed, estimated_fee
        )));
    }

    // Select UTXOs (simple first-fit)
    let mut selected_utxos = Vec::new();
    let mut selected_total = 0u64;
    for utxo in balance.utxos.iter() {
        if !utxo.is_occupied && utxo.confirmations > 0 {
            selected_utxos.push(utxo.clone());
            selected_total += utxo.amount_sats;
            if selected_total >= total_needed {
                break;
            }
        }
    }

    if selected_total < total_needed {
        return Err(WalletError::InsufficientFunds(
            "Could not select enough confirmed UTXOs".to_string(),
        ));
    }

    // Create change address using internal_next_index
    let descriptor = storage.load_descriptor(wallet_name)?;
    let mut state = storage.load_state(wallet_name)?;
    let change_index = state.internal_next_index;
    
    log::debug!("Using internal address index {} for Bitcoin change", change_index);
    
    state.used_addresses.push(change_index);
    state.internal_next_index += 1;
    
    let change_address =
        AddressManager::derive_address(&descriptor, change_index, Network::Signet)?;

    // Build transaction
    let tx_builder = transaction::TransactionBuilder::new(Network::Signet);
    let tx = tx_builder.build_send_tx(
        &selected_utxos,
        to_address.clone(),
        request.amount_sats,
        change_address,
        fee_rate,
    )?;

    // Sign transaction
    let mnemonic = storage.load_mnemonic(wallet_name)?;
    let signed_tx = sign_transaction_multi_key(&tx, &selected_utxos, &mnemonic)?;

    // Calculate actual fee
    let total_input: u64 = selected_utxos.iter().map(|u| u.amount_sats).sum();
    let total_output: u64 = signed_tx.output.iter().map(|o| o.value.to_sat()).sum();
    let fee_sats = total_input - total_output;

    // Broadcast
    let txid = transaction::broadcast_transaction(&signed_tx, Network::Signet).await?;
    log::info!("Bitcoin sent - txid: {}", txid);

    storage.save_state(wallet_name, &state)?;

    Ok(SendBitcoinResponse {
        txid,
        amount_sats: request.amount_sats,
        fee_sats,
        to_address: request.to_address,
    })
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Helper to get balance without RGB info (used internally for bitcoin operations)
async fn get_balance_for_tx(
    storage: &Storage,
    balance_checker: &BalanceChecker,
    wallet_name: &str,
) -> Result<balance::BalanceInfo, WalletError> {
    let descriptor = storage.load_descriptor(wallet_name)?;
    const GAP_LIMIT: u32 = 20;
    let addresses_with_indices =
        AddressManager::derive_addresses(&descriptor, 0, GAP_LIMIT, Network::Signet)?;
    balance_checker.calculate_balance(&addresses_with_indices).await
}

/// Derive private key for a specific address index
fn derive_private_key_for_index(
    mnemonic: &bip39::Mnemonic,
    address_index: u32,
) -> Result<bitcoin::PrivateKey, WalletError> {
    let seed = mnemonic.to_seed("");
    let master_key = bitcoin::bip32::Xpriv::new_master(Network::Signet, &seed)
        .map_err(|e| WalletError::Bitcoin(e.to_string()))?;

    let path =
        bitcoin::bip32::DerivationPath::from_str(&format!("m/84'/1'/0'/0/{}", address_index))
            .map_err(|e| WalletError::Bitcoin(e.to_string()))?;

    let derived_key = master_key
        .derive_priv(&bitcoin::secp256k1::Secp256k1::new(), &path)
        .map_err(|e| WalletError::Bitcoin(e.to_string()))?;

    Ok(bitcoin::PrivateKey::new(
        derived_key.private_key,
        Network::Signet,
    ))
}

/// Sign a transaction using the correct private keys for each UTXO
fn sign_transaction_multi_key(
    tx: &bitcoin::Transaction,
    utxos: &[balance::UTXO],
    mnemonic: &bip39::Mnemonic,
) -> Result<bitcoin::Transaction, WalletError> {
    use bitcoin::hashes::Hash;
    use bitcoin::secp256k1::{Message, Secp256k1};
    use bitcoin::sighash::{EcdsaSighashType, SighashCache};
    use bitcoin::PublicKey;

    let mut signed_tx = tx.clone();
    let secp = Secp256k1::new();

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
            bitcoin::Address::p2wpkh(&public_key.try_into().unwrap(), Network::Signet)
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
