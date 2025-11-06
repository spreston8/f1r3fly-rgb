use bip39::Mnemonic;
use std::fs;
use std::path::PathBuf;

use super::models::{Metadata, WalletState};

#[derive(Clone)]
pub struct Storage {
    base_path: PathBuf,
}

impl Storage {
    /// Create a new storage instance with the default base directory ("./wallets")
    pub fn new() -> Self {
        Self {
            base_path: PathBuf::from("./wallets"),
        }
    }

    /// Create storage with custom base directory (for testing)
    pub fn new_with_base_dir(base_path: PathBuf) -> Self {
        Self { base_path         }
    }

    /// Get the base directory path for wallet storage
    pub fn base_dir(&self) -> &PathBuf {
        &self.base_path
    }

    /// Get the directory path for a specific wallet
    fn wallet_dir(&self, name: &str) -> PathBuf {
        self.base_path.join(name)
    }

    /// Create a new wallet directory structure
    pub fn create_wallet(&self, name: &str) -> Result<(), crate::error::StorageError> {
        fs::create_dir_all(&self.base_path)?;
        let wallet_dir = self.wallet_dir(name);
        fs::create_dir_all(&wallet_dir)?;
        Ok(())
    }

    /// Check if a wallet with the given name exists
    pub fn wallet_exists(&self, name: &str) -> bool {
        self.wallet_dir(name).exists()
    }

    /// Save wallet metadata to disk
    pub fn save_metadata(
        &self,
        name: &str,
        meta: &Metadata,
    ) -> Result<(), crate::error::StorageError> {
        let path = self.wallet_dir(name).join("metadata.json");
        let json = serde_json::to_string_pretty(meta)?;
        fs::write(path, json)?;
        Ok(())
    }

    /// Load wallet metadata from disk
    pub fn load_metadata(&self, name: &str) -> Result<Metadata, crate::error::StorageError> {
        let path = self.wallet_dir(name).join("metadata.json");
        if !path.exists() {
            return Err(crate::error::StorageError::FileNotFound(
                path.display().to_string(),
            ));
        }
        let contents = fs::read_to_string(path)?;
        let meta = serde_json::from_str(&contents)?;
        Ok(meta)
    }

    /// Save wallet mnemonic phrase to disk
    pub fn save_mnemonic(
        &self,
        name: &str,
        mnemonic: &Mnemonic,
    ) -> Result<(), crate::error::StorageError> {
        let path = self.wallet_dir(name).join("mnemonic.txt");
        fs::write(path, mnemonic.to_string())?;
        Ok(())
    }

    /// Load wallet mnemonic phrase from disk
    pub fn load_mnemonic(&self, name: &str) -> Result<Mnemonic, crate::error::StorageError> {
        let path = self.wallet_dir(name).join("mnemonic.txt");
        if !path.exists() {
            return Err(crate::error::StorageError::FileNotFound(
                path.display().to_string(),
            ));
        }
        let contents = fs::read_to_string(path)?;
        let mnemonic = Mnemonic::parse(&contents).map_err(|e| {
            crate::error::StorageError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid mnemonic: {}", e),
            ))
        })?;
        Ok(mnemonic)
    }

    /// Save wallet descriptor to disk
    pub fn save_descriptor(
        &self,
        name: &str,
        descriptor: &str,
    ) -> Result<(), crate::error::StorageError> {
        let path = self.wallet_dir(name).join("descriptor.txt");
        fs::write(path, descriptor)?;
        Ok(())
    }

    /// Load wallet descriptor from disk
    pub fn load_descriptor(&self, name: &str) -> Result<String, crate::error::StorageError> {
        let path = self.wallet_dir(name).join("descriptor.txt");
        if !path.exists() {
            return Err(crate::error::StorageError::FileNotFound(
                path.display().to_string(),
            ));
        }
        let descriptor = fs::read_to_string(path)?;
        Ok(descriptor.trim().to_string())
    }

    /// Save wallet state (address indices, used addresses) to disk
    pub fn save_state(
        &self,
        name: &str,
        state: &WalletState,
    ) -> Result<(), crate::error::StorageError> {
        let path = self.wallet_dir(name).join("state.json");
        let json = serde_json::to_string_pretty(state)?;
        fs::write(path, json)?;
        Ok(())
    }

    /// Load wallet state from disk, or return default state if file doesn't exist
    pub fn load_state(&self, name: &str) -> Result<WalletState, crate::error::StorageError> {
        let path = self.wallet_dir(name).join("state.json");
        if !path.exists() {
            return Ok(WalletState::default());
        }
        let contents = fs::read_to_string(path)?;
        let state = serde_json::from_str(&contents)?;
        Ok(state)
    }

    /// List all wallet names in the storage directory
    pub fn list_wallets(&self) -> Result<Vec<String>, crate::error::StorageError> {
        if !self.base_path.exists() {
            return Ok(Vec::new());
        }

        let mut wallets = Vec::new();
        for entry in fs::read_dir(&self.base_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path.file_name() {
                    if let Some(name_str) = name.to_str() {
                        wallets.push(name_str.to_string());
                    }
                }
            }
        }
        Ok(wallets)
    }

    /// Delete a wallet and all its associated data from disk
    pub fn delete_wallet(&self, name: &str) -> Result<(), crate::error::StorageError> {
        let wallet_dir = self.wallet_dir(name);
        
        if !wallet_dir.exists() {
            return Err(crate::error::StorageError::FileNotFound(
                wallet_dir.display().to_string(),
            ));
        }
        
        log::warn!("Deleting wallet directory: {:?}", wallet_dir);
        std::fs::remove_dir_all(&wallet_dir)?;
        log::info!("Wallet '{}' deleted successfully", name);
        
        Ok(())
    }
}

