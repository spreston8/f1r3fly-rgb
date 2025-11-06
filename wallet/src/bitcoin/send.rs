//! Bitcoin sending operations

use crate::storage::Storage;
use crate::api::types::{SendBitcoinRequest, SendBitcoinResponse};
use crate::error::WalletError;
use super::transaction::{self, TransactionBuilder};
use super::utxo::{get_balance_for_tx, sign_transaction_multi_key};
use crate::bitcoin::balance_checker::BalanceChecker;
use crate::bitcoin::network::get_network;
use crate::wallet::AddressManager;
use crate::rgb::runtime::RgbRuntimeManager;
use std::str::FromStr;

/// Send Bitcoin to an address
pub async fn send_bitcoin(
    storage: &Storage,
    balance_checker: &BalanceChecker,
    rgb_runtime_manager: &RgbRuntimeManager,
    wallet_name: &str,
    request: SendBitcoinRequest,
) -> Result<SendBitcoinResponse, WalletError> {
    log::info!("Sending Bitcoin from wallet: {} to address: {}, amount: {} sats", 
        wallet_name, request.to_address, request.amount_sats);

    if !storage.wallet_exists(wallet_name) {
        return Err(WalletError::WalletNotFound(wallet_name.to_string()));
    }

    // Parse destination address
    let network = get_network();
    let to_address = bitcoin::Address::from_str(&request.to_address)
        .map_err(|e| {
            WalletError::InvalidInput(format!("Invalid address: {}", e))
        })?
        .require_network(network)
        .map_err(|e| {
            WalletError::InvalidInput(format!("Address network mismatch: {}", e))
        })?;

    let fee_rate = request.fee_rate_sat_vb.unwrap_or(2);
    let balance = get_balance_for_tx(storage, balance_checker, rgb_runtime_manager, wallet_name).await?;

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
        AddressManager::derive_address(&descriptor, change_index, network)?;

    // Build transaction
    let tx_builder = TransactionBuilder::new(network);
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
    let txid = transaction::broadcast_transaction(&signed_tx, network).await?;
    log::info!("Bitcoin sent - txid: {}", txid);

    storage.save_state(wallet_name, &state)?;

    Ok(SendBitcoinResponse {
        txid,
        amount_sats: request.amount_sats,
        fee_sats,
        to_address: request.to_address,
    })
}
