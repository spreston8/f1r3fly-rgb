use super::addresses::AddressManager;
use super::balance::BalanceChecker;
use super::keys::KeyManager;
use super::storage::{Metadata, Storage};
use bitcoin::Network;
use chrono::Utc;

pub struct WalletManager {
    storage: Storage,
    balance_checker: BalanceChecker,
}

impl WalletManager {
    pub fn new() -> Self {
        Self {
            storage: Storage::new(),
            balance_checker: BalanceChecker::new(),
        }
    }

    pub fn create_wallet(&self, name: &str) -> Result<WalletInfo, crate::error::WalletError> {
        if self.storage.wallet_exists(name) {
            return Err(crate::error::WalletError::WalletExists(name.to_string()));
        }

        let keys = KeyManager::generate()?;

        self.storage.create_wallet(name)?;

        let metadata = Metadata {
            name: name.to_string(),
            created_at: Utc::now(),
            network: "signet".to_string(),
        };
        self.storage.save_metadata(name, &metadata)?;
        self.storage.save_mnemonic(name, &keys.mnemonic)?;
        self.storage.save_descriptor(name, &keys.descriptor)?;

        let first_address = AddressManager::derive_address(&keys.descriptor, 0, Network::Signet)?;

        Ok(WalletInfo {
            name: name.to_string(),
            mnemonic: keys.mnemonic.to_string(),
            first_address: first_address.to_string(),
            descriptor: keys.descriptor,
        })
    }

    pub fn import_wallet(
        &self,
        name: &str,
        mnemonic: &str,
    ) -> Result<WalletInfo, crate::error::WalletError> {
        if self.storage.wallet_exists(name) {
            return Err(crate::error::WalletError::WalletExists(name.to_string()));
        }

        let keys = KeyManager::from_mnemonic(mnemonic)?;

        self.storage.create_wallet(name)?;

        let metadata = Metadata {
            name: name.to_string(),
            created_at: Utc::now(),
            network: "signet".to_string(),
        };
        self.storage.save_metadata(name, &metadata)?;
        self.storage.save_mnemonic(name, &keys.mnemonic)?;
        self.storage.save_descriptor(name, &keys.descriptor)?;

        let first_address = AddressManager::derive_address(&keys.descriptor, 0, Network::Signet)?;

        Ok(WalletInfo {
            name: name.to_string(),
            mnemonic: keys.mnemonic.to_string(),
            first_address: first_address.to_string(),
            descriptor: keys.descriptor,
        })
    }

    pub fn list_wallets(&self) -> Result<Vec<WalletMetadata>, crate::error::WalletError> {
        let wallet_names = self.storage.list_wallets()?;
        let mut wallets = Vec::new();

        for name in wallet_names {
            if let Ok(metadata) = self.storage.load_metadata(&name) {
                let state = self.storage.load_state(&name).ok();
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

    pub fn get_addresses(
        &self,
        name: &str,
        count: u32,
    ) -> Result<Vec<AddressInfo>, crate::error::WalletError> {
        if !self.storage.wallet_exists(name) {
            return Err(crate::error::WalletError::WalletNotFound(name.to_string()));
        }

        let descriptor = self.storage.load_descriptor(name)?;
        let state = self.storage.load_state(name)?;

        let addresses =
            AddressManager::derive_addresses(&descriptor, 0, count, Network::Signet)?;

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

    pub fn get_primary_address(
        &self,
        name: &str,
    ) -> Result<NextAddressInfo, crate::error::WalletError> {
        if !self.storage.wallet_exists(name) {
            return Err(crate::error::WalletError::WalletNotFound(name.to_string()));
        }

        let descriptor = self.storage.load_descriptor(name)?;
        let state = self.storage.load_state(name)?;

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

    pub async fn get_balance(
        &self,
        name: &str,
    ) -> Result<super::balance::BalanceInfo, crate::error::WalletError> {
        if !self.storage.wallet_exists(name) {
            return Err(crate::error::WalletError::WalletNotFound(name.to_string()));
        }

        let descriptor = self.storage.load_descriptor(name)?;

        const GAP_LIMIT: u32 = 20;
        let addresses =
            AddressManager::derive_addresses(&descriptor, 0, GAP_LIMIT, Network::Signet)?;

        let address_list: Vec<_> = addresses.into_iter().map(|(_, addr)| addr).collect();

        let balance = self.balance_checker.calculate_balance(&address_list).await?;

        Ok(balance)
    }

    pub async fn sync_wallet(
        &self,
        name: &str,
    ) -> Result<SyncResult, crate::error::WalletError> {
        if !self.storage.wallet_exists(name) {
            return Err(crate::error::WalletError::WalletNotFound(name.to_string()));
        }

        let descriptor = self.storage.load_descriptor(name)?;
        let mut state = self.storage.load_state(name)?;

        let tip_height = self.balance_checker.get_tip_height().await?;

        const GAP_LIMIT: u32 = 20;
        let addresses =
            AddressManager::derive_addresses(&descriptor, 0, GAP_LIMIT, Network::Signet)?;

        let mut new_transactions = 0;

        for (index, address) in addresses {
            let utxos = self.balance_checker.get_address_utxos(&address).await?;
            if !utxos.is_empty() && !state.used_addresses.contains(&index) {
                state.used_addresses.push(index);
                new_transactions += utxos.len();
            }
        }

        state.last_synced_height = Some(tip_height);
        self.storage.save_state(name, &state)?;

        Ok(SyncResult {
            synced_height: tip_height,
            addresses_checked: GAP_LIMIT,
            new_transactions,
        })
    }
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletInfo {
    pub name: String,
    pub mnemonic: String,
    pub first_address: String,
    pub descriptor: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletMetadata {
    pub name: String,
    pub created_at: String,
    pub last_synced: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressInfo {
    pub index: u32,
    pub address: String,
    pub used: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub synced_height: u64,
    pub addresses_checked: u32,
    pub new_transactions: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NextAddressInfo {
    pub address: String,
    pub index: u32,
    pub total_used: usize,
    pub descriptor: String,
}

