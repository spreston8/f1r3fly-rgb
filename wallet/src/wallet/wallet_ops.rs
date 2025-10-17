/// Wallet lifecycle operations
///
/// Handles wallet creation, import, and listing.
use super::shared::*;
use crate::api::types::{WalletInfo, WalletMetadata};
use crate::error::WalletError;
use bitcoin::Network;
use chrono::Utc;

/// Create a new wallet with a generated mnemonic
pub fn create_wallet(storage: &Storage, name: &str) -> Result<WalletInfo, WalletError> {
    if storage.wallet_exists(name) {
        return Err(WalletError::WalletExists(name.to_string()));
    }

    let keys = KeyManager::generate()?;

    storage.create_wallet(name)?;

    let metadata = Metadata {
        name: name.to_string(),
        created_at: Utc::now(),
        network: "signet".to_string(),
    };
    storage.save_metadata(name, &metadata)?;
    storage.save_mnemonic(name, &keys.mnemonic)?;
    storage.save_descriptor(name, &keys.descriptor)?;

    let first_address = AddressManager::derive_address(&keys.descriptor, 0, Network::Signet)?;
    let public_address = first_address.clone();

    Ok(WalletInfo {
        name: name.to_string(),
        mnemonic: keys.mnemonic.to_string(),
        first_address: first_address.to_string(),
        public_address: public_address.to_string(),
        descriptor: keys.descriptor,
    })
}

/// Import a wallet from an existing mnemonic
pub fn import_wallet(
    storage: &Storage,
    name: &str,
    mnemonic: bip39::Mnemonic,
) -> Result<WalletInfo, WalletError> {
    if storage.wallet_exists(name) {
        return Err(WalletError::WalletExists(name.to_string()));
    }

    let keys = KeyManager::from_mnemonic(&mnemonic.to_string())?;

    storage.create_wallet(name)?;

    let metadata = Metadata {
        name: name.to_string(),
        created_at: Utc::now(),
        network: "signet".to_string(),
    };
    storage.save_metadata(name, &metadata)?;
    storage.save_mnemonic(name, &keys.mnemonic)?;
    storage.save_descriptor(name, &keys.descriptor)?;

    let first_address = AddressManager::derive_address(&keys.descriptor, 0, Network::Signet)?;
    let public_address = first_address.clone();

    Ok(WalletInfo {
        name: name.to_string(),
        mnemonic: keys.mnemonic.to_string(),
        first_address: first_address.to_string(),
        public_address: public_address.to_string(),
        descriptor: keys.descriptor,
    })
}

/// List all wallets
pub fn list_wallets(storage: &Storage) -> Result<Vec<WalletMetadata>, WalletError> {
    let wallet_names = storage.list_wallets()?;
    let mut wallets = Vec::new();

    for name in wallet_names {
        if let Ok(metadata) = storage.load_metadata(&name) {
            let state = storage.load_state(&name).ok();
            let last_synced = state
                .and_then(|s| s.last_synced_height)
                .map(|h| format!("Height: {}", h));

            wallets.push(WalletMetadata {
                name: metadata.name,
                created_at: metadata.created_at.to_rfc3339(),
                last_synced,
            });
        }
    }

    Ok(wallets)
}

/// Delete a wallet and all its data
pub fn delete_wallet(storage: &Storage, name: &str) -> Result<(), WalletError> {
    if !storage.wallet_exists(name) {
        return Err(WalletError::WalletNotFound(name.to_string()));
    }
    
    log::warn!("Deleting wallet: {}", name);
    storage.delete_wallet(name)?;
    
    Ok(())
}
