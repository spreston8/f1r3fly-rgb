use super::addresses::AddressManager;
use super::balance::BalanceChecker;
use super::keys::KeyManager;
use super::rgb::RgbManager;
use super::rgb_runtime::RgbRuntimeManager;
use super::storage::{Metadata, Storage};
use bitcoin::Network;
use chrono::Utc;
use std::str::FromStr;

pub struct WalletManager {
    pub storage: Storage,
    balance_checker: BalanceChecker,
    pub rgb_manager: RgbManager,
    rgb_runtime_manager: RgbRuntimeManager,
}

impl WalletManager {
    pub fn new() -> Self {
        let storage = Storage::new();
        let rgb_data_dir = storage.base_dir().join("rgb_data");
        let rgb_manager = RgbManager::new(rgb_data_dir, bpstd::Network::Signet)
            .expect("Failed to initialize RGB manager");
        
        let rgb_runtime_manager = RgbRuntimeManager::new(
            storage.base_dir().clone(),
            bpstd::Network::Signet,
        );
        
        Self {
            storage,
            balance_checker: BalanceChecker::new(),
            rgb_manager,
            rgb_runtime_manager,
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

        let mut balance = self.balance_checker.calculate_balance(&address_list).await?;

        // Check each UTXO for RGB assets
        for utxo in &mut balance.utxos {
            let txid = match utxo.txid.parse::<bitcoin::Txid>() {
                Ok(t) => t,
                Err(_) => continue,
            };

            // Check if UTXO is occupied by RGB assets
            if let Ok(is_occupied) = self.rgb_manager.check_utxo_occupied(txid, utxo.vout) {
                utxo.is_occupied = is_occupied;

                // If occupied, get the bound assets
                if is_occupied {
                    if let Ok(assets) = self.rgb_manager.get_bound_assets(txid, utxo.vout) {
                        utxo.bound_assets = assets;
                    }
                }
            }
        }

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

    pub async fn create_utxo(
        &self,
        name: &str,
        request: CreateUtxoRequest,
    ) -> Result<CreateUtxoResult, crate::error::WalletError> {
        if !self.storage.wallet_exists(name) {
            return Err(crate::error::WalletError::WalletNotFound(name.to_string()));
        }

        let amount_sats = match request.amount_btc {
            Some(btc) => (btc * 100_000_000.0) as u64,
            None => 30_000,
        };

        let fee_rate = request.fee_rate_sat_vb.unwrap_or(2);

        let balance = self.get_balance(name).await?;
        
        if balance.utxos.is_empty() {
            return Err(crate::error::WalletError::InsufficientFunds(
                "No UTXOs available to create new UTXO".to_string()
            ));
        }

        let descriptor = self.storage.load_descriptor(name)?;
        let mut state = self.storage.load_state(name)?;

        let mut next_index = 0;
        while state.used_addresses.contains(&next_index) {
            next_index += 1;
        }
        state.used_addresses.push(next_index);

        let recipient_address = AddressManager::derive_address(&descriptor, next_index, Network::Signet)?;

        let tx_builder = super::transaction::TransactionBuilder::new(Network::Signet);
        
        let tx = tx_builder.build_send_to_self(
            &balance.utxos,
            amount_sats,
            fee_rate,
            recipient_address.clone(),
        )?;

        let mnemonic = self.storage.load_mnemonic(name)?;

        let seed = mnemonic.to_seed("");
        let master_key = bitcoin::bip32::Xpriv::new_master(Network::Signet, &seed)
            .map_err(|e| crate::error::WalletError::Bitcoin(e.to_string()))?;

        let path = bitcoin::bip32::DerivationPath::from_str("m/84'/1'/0'/0/0")
            .map_err(|e| crate::error::WalletError::Bitcoin(e.to_string()))?;
        
        let derived_key = master_key.derive_priv(&bitcoin::secp256k1::Secp256k1::new(), &path)
            .map_err(|e| crate::error::WalletError::Bitcoin(e.to_string()))?;

        let private_key = bitcoin::PrivateKey::new(derived_key.private_key, Network::Signet);

        let signed_tx = tx_builder.sign_transaction(tx, &balance.utxos, &private_key)?;

        let txid = super::transaction::broadcast_transaction(&signed_tx, Network::Signet).await?;

        let total_input: u64 = balance.utxos.iter()
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
        
        let fee_sats = total_input - signed_tx.output.iter().map(|o| o.value.to_sat()).sum::<u64>();

        self.storage.save_state(name, &state)?;

        Ok(CreateUtxoResult {
            txid,
            amount_sats,
            fee_sats,
            target_address: recipient_address.to_string(),
        })
    }

    pub async fn unlock_utxo(
        &self,
        name: &str,
        request: UnlockUtxoRequest,
    ) -> Result<UnlockUtxoResult, crate::error::WalletError> {
        if !self.storage.wallet_exists(name) {
            return Err(crate::error::WalletError::WalletNotFound(name.to_string()));
        }

        let fee_rate = request.fee_rate_sat_vb.unwrap_or(2);

        let balance = self.get_balance(name).await?;

        let target_utxo = balance.utxos.iter()
            .find(|u| u.txid == request.txid && u.vout == request.vout)
            .ok_or_else(|| crate::error::WalletError::Internal(
                format!("UTXO {}:{} not found", request.txid, request.vout)
            ))?
            .clone();

        let descriptor = self.storage.load_descriptor(name)?;
        let mut state = self.storage.load_state(name)?;

        let mut next_index = 0;
        while state.used_addresses.contains(&next_index) {
            next_index += 1;
        }
        state.used_addresses.push(next_index);

        let destination_address = AddressManager::derive_address(&descriptor, next_index, Network::Signet)?;

        let tx_builder = super::transaction::TransactionBuilder::new(Network::Signet);
        
        let tx = tx_builder.build_unlock_utxo_tx(
            &target_utxo,
            destination_address.clone(),
            fee_rate,
        )?;

        let mnemonic = self.storage.load_mnemonic(name)?;
        let seed = mnemonic.to_seed("");
        let master_key = bitcoin::bip32::Xpriv::new_master(Network::Signet, &seed)
            .map_err(|e| crate::error::WalletError::Bitcoin(e.to_string()))?;

        let path = bitcoin::bip32::DerivationPath::from_str("m/84'/1'/0'/0/0")
            .map_err(|e| crate::error::WalletError::Bitcoin(e.to_string()))?;
        
        let derived_key = master_key.derive_priv(&bitcoin::secp256k1::Secp256k1::new(), &path)
            .map_err(|e| crate::error::WalletError::Bitcoin(e.to_string()))?;

        let private_key = bitcoin::PrivateKey::new(derived_key.private_key, Network::Signet);

        let signed_tx = tx_builder.sign_transaction(tx, &vec![target_utxo.clone()], &private_key)?;

        let txid = super::transaction::broadcast_transaction(&signed_tx, Network::Signet).await?;

        let fee_sats = target_utxo.amount_sats - signed_tx.output[0].value.to_sat();
        let recovered_sats = signed_tx.output[0].value.to_sat();

        self.storage.save_state(name, &state)?;

        Ok(UnlockUtxoResult {
            txid,
            recovered_sats,
            fee_sats,
        })
    }
    
    /// Helper method to initialize RGB Runtime for a wallet (used by transfer APIs)
    pub(crate) fn get_runtime(
        &self,
        wallet_name: &str,
    ) -> Result<rgbp::RgbpRuntimeDir<rgbp::resolvers::MultiResolver>, crate::error::WalletError> {
        self.rgb_runtime_manager.init_runtime(wallet_name)
    }
    
    pub(crate) fn get_runtime_no_sync(
        &self,
        wallet_name: &str,
    ) -> Result<rgbp::RgbpRuntimeDir<rgbp::resolvers::MultiResolver>, crate::error::WalletError> {
        self.rgb_runtime_manager.init_runtime_no_sync(wallet_name)
    }
    
    /// Generate RGB invoice for receiving assets
    pub async fn generate_rgb_invoice(
        &self,
        wallet_name: &str,
        request: GenerateInvoiceRequest,
    ) -> Result<GenerateInvoiceResult, crate::error::WalletError> {
        // Verify wallet exists
        if !self.storage.wallet_exists(wallet_name) {
            return Err(crate::error::WalletError::WalletNotFound(wallet_name.to_string()));
        }
        
        // Parse contract ID
        use rgb::ContractId;
        let contract_id = ContractId::from_str(&request.contract_id)
            .map_err(|e| crate::error::WalletError::InvalidInput(format!("Invalid contract ID: {}", e)))?;
        
        // Initialize RGB Runtime (try without sync first for speed)
        let mut runtime = self.get_runtime_no_sync(wallet_name)?;
        
        // Generate auth token (blinded seal) from available UTXO
        let nonce = 0u64;  // Default nonce
        let auth = match runtime.auth_token(Some(nonce)) {
            Some(token) => token,
            None => {
                // No UTXOs available, need to sync with blockchain
                // This happens on first run or if UTXOs changed
                runtime.update(32)
                    .map_err(|e| crate::error::WalletError::Rgb(format!("Sync failed: {:?}", e)))?;
                
                // Try again after sync
                runtime.auth_token(Some(nonce))
                    .ok_or_else(|| crate::error::WalletError::Rgb(
                        "No unspent outputs available for seal even after sync. Please create a UTXO first.".to_string()
                    ))?
            }
        };
        
        // Manually serialize to Bitlight-compatible format
        // Format: contract:{network}@{contract_id}/{amount}@at:{auth_token}/
        // Example: contract:tb@EHKyQHds-2Tq9bLb-i37_iA2-MR0MYi6-TR5zbpm-2hXl0zI/1000000000@at:StOTDkHA-.../
        // This format is proven in production (used by Bitlight wallet)
        // 
        // Components:
        // - contract: URI scheme
        // - tb: network (testnet bitcoin, used for both testnet and signet)
        // - contract_id: Baid64-encoded contract identifier
        // - amount: token amount (base units, considering precision)
        // - at: auth token (Baid64-encoded blinded seal)
        let network = "tb"; // Signet uses 'tb' (testnet bitcoin)
        let contract_id_str = contract_id.to_string(); // Baid64 encoded
        let auth_token_str = auth.to_string(); // Baid64 encoded
        
        let invoice_str = if let Some(amount) = request.amount {
            format!("contract:{}@{}/{}@at:{}/", network, contract_id_str, amount, auth_token_str)
        } else {
            // For "any amount" invoices (amount not specified)
            format!("contract:{}@{}@at:{}/", network, contract_id_str, auth_token_str)
        };
        
        // Extract seal outpoint for display (auth token encodes the UTXO)
        let seal_outpoint = format!("{}", auth);
        
        Ok(GenerateInvoiceResult {
            invoice: invoice_str,
            contract_id: request.contract_id,
            amount: request.amount,
            seal_utxo: seal_outpoint,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUtxoRequest {
    pub amount_btc: Option<f64>,
    pub fee_rate_sat_vb: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUtxoResult {
    pub txid: String,
    pub amount_sats: u64,
    pub fee_sats: u64,
    pub target_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnlockUtxoRequest {
    pub txid: String,
    pub vout: u32,
    pub fee_rate_sat_vb: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnlockUtxoResult {
    pub txid: String,
    pub recovered_sats: u64,
    pub fee_sats: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateInvoiceRequest {
    pub contract_id: String,
    pub amount: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateInvoiceResult {
    pub invoice: String,
    pub contract_id: String,
    pub amount: Option<u64>,
    pub seal_utxo: String,
}
