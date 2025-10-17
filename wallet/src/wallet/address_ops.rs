/// Address management operations
/// 
/// Handles address derivation and queries.

use super::shared::*;
use crate::api::types::{AddressInfo, NextAddressInfo};
use crate::error::WalletError;
use bitcoin::Network;

/// Get multiple derived addresses for a wallet
pub fn get_addresses(
    storage: &Storage,
    wallet_name: &str,
    count: u32,
) -> Result<Vec<AddressInfo>, WalletError> {
    if !storage.wallet_exists(wallet_name) {
        return Err(WalletError::WalletNotFound(wallet_name.to_string()));
    }

    let descriptor = storage.load_descriptor(wallet_name)?;
    let state = storage.load_state(wallet_name)?;

    let addresses = AddressManager::derive_addresses(&descriptor, 0, count, Network::Signet)?;

    let address_infos = addresses
        .into_iter()
        .map(|(index, address)| AddressInfo {
            index,
            address: address.to_string(),
            used: state.used_addresses.contains(&index),
        })
        .collect();

    Ok(address_infos)
}

/// Get the next unused address for a wallet
pub fn get_primary_address(
    storage: &Storage,
    wallet_name: &str,
) -> Result<NextAddressInfo, WalletError> {
    if !storage.wallet_exists(wallet_name) {
        return Err(WalletError::WalletNotFound(wallet_name.to_string()));
    }

    let descriptor = storage.load_descriptor(wallet_name)?;
    let state = storage.load_state(wallet_name)?;

    // Always return Address #0 for consistent development experience
    let primary_index = 0;

    let address = AddressManager::derive_address(&descriptor, primary_index, Network::Signet)?;

    Ok(NextAddressInfo {
        address: address.to_string(),
        index: primary_index,
        total_used: state.used_addresses.len(),
        descriptor,
    })
}

