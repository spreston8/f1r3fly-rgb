/// Wallet Manager - Orchestration Layer
/// 
/// Coordinates all wallet operations by delegating to specialized operation modules.
use super::shared::*;
use super::shared::rgb::{IssueAssetRequest, IssueAssetResponse};
use crate::api::types::*;
use crate::config::WalletConfig;
use crate::error::WalletError;
use crate::firefly::FireflyClient;

pub struct WalletManager {
    pub config: WalletConfig,
    pub storage: Storage,
    balance_checker: BalanceChecker,
    // Ephemeral runtime creation (matches RGB CLI architecture)
    rgb_runtime_manager: RgbRuntimeManager,
    pub firefly_client: Option<FireflyClient>,
}

impl WalletManager {
    // ============================================================================
    // Constructor
    // ============================================================================

    pub fn new() -> Self {
        // Load configuration from environment
        let config = WalletConfig::from_env();
        
        let storage = Storage::new();
        let rgb_runtime_manager = RgbRuntimeManager::new(
            storage.base_dir().clone(),
            config.bpstd_network,
            config.esplora_url.clone(),
        );
        
        // Initialize Firefly client from config
        let firefly_client = Some(FireflyClient::new(
            &config.firefly_host,
            config.firefly_grpc_port,
            config.firefly_http_port,
        ));
        
        Self {
            config: config.clone(),
            storage,
            balance_checker: BalanceChecker::new(config.esplora_url.clone()),
            rgb_runtime_manager,
            firefly_client,
        }
    }

    /// Create WalletManager with custom storage (for testing)
    pub fn new_with_storage(storage: Storage) -> Self {
        // Load configuration from environment (allows test to set env vars)
        let config = WalletConfig::from_env();
        
        let rgb_runtime_manager = RgbRuntimeManager::new(
            storage.base_dir().clone(),
            config.bpstd_network,
            config.esplora_url.clone(),
        );
        
        // No Firefly client in tests
        let firefly_client = None;
        
        Self {
            config: config.clone(),
            storage,
            balance_checker: BalanceChecker::new(config.esplora_url.clone()),
            rgb_runtime_manager,
            firefly_client,
        }
    }

    // ============================================================================
    // RGB Manager Helper (per-wallet instance)
    // ============================================================================

    /// Get RGB manager for a specific wallet
    /// Each wallet has its own isolated RGB data directory
    pub fn get_rgb_manager(&self, wallet_name: &str) -> Result<RgbManager, WalletError> {
        let rgb_data_dir = self.storage.base_dir().join(wallet_name).join("rgb_data");
        RgbManager::new(rgb_data_dir)
    }

    // ============================================================================
    // Wallet Management (delegates to wallet_ops)
    // ============================================================================

    pub fn create_wallet(&self, name: &str, rgb_mode: RgbMode) -> Result<WalletInfo, WalletError> {
        super::wallet_ops::create_wallet(&self.storage, name, rgb_mode)
    }

    pub fn import_wallet(
        &self,
        name: &str,
        mnemonic: bip39::Mnemonic,
        rgb_mode: RgbMode,
    ) -> Result<WalletInfo, WalletError> {
        super::wallet_ops::import_wallet(&self.storage, name, mnemonic, rgb_mode)
    }

    pub fn list_wallets(&self) -> Result<Vec<WalletMetadata>, WalletError> {
        super::wallet_ops::list_wallets(&self.storage)
    }

    pub fn delete_wallet(&self, name: &str) -> Result<(), WalletError> {
        super::wallet_ops::delete_wallet(&self.storage, name)
    }

    // ============================================================================
    // Address Management (delegates to address_ops)
    // ============================================================================

    pub fn get_addresses(&self, name: &str, count: u32) -> Result<Vec<AddressInfo>, WalletError> {
        super::address_ops::get_addresses(&self.storage, name, count)
    }

    pub fn get_primary_address(&self, name: &str) -> Result<NextAddressInfo, WalletError> {
        super::address_ops::get_primary_address(&self.storage, name)
    }

    // ============================================================================
    // Balance & Sync (delegates to balance_ops and sync_ops)
    // ============================================================================

    pub async fn get_balance(&self, name: &str) -> Result<balance::BalanceInfo, WalletError> {
        // Phase 1: Get Bitcoin balance (async HTTP)
        let mut balance = super::balance_ops::get_bitcoin_balance(
            &self.storage,
            &self.balance_checker,
            name,
        )
        .await?;
        
        // Phase 2: Get RGB balance (spawn_blocking for FileHolder operations)
        let storage = self.storage.clone();
        let rgb_mgr = self.rgb_runtime_manager.clone();
        let name_clone = name.to_string();
        let utxos_clone = balance.utxos.clone();
        
        let rgb_data = tokio::task::spawn_blocking(move || {
            super::balance_ops::get_rgb_balance_sync(&storage, &rgb_mgr, &name_clone, &utxos_clone)
        })
        .await
        .map_err(|e| WalletError::Internal(format!("Get RGB balance task panicked: {}", e)))?
        .map_err(|e| {
            log::error!("Failed to get RGB balance for wallet {}: {:?}", name, e);
            e
        })?;
        
        // Phase 3: Merge RGB data into balance
        balance.known_contracts = rgb_data.known_contracts;
        for utxo in &mut balance.utxos {
            let key = format!("{}:{}", utxo.txid, utxo.vout);
            if let Some(assets) = rgb_data.utxo_assets.get(&key) {
                utxo.bound_assets = assets.clone();
                utxo.is_occupied = !assets.is_empty();
            }
        }

        Ok(balance)
    }

    pub async fn sync_wallet(&self, name: &str) -> Result<SyncResult, WalletError> {
        super::sync_ops::sync_wallet(&self.storage, &self.balance_checker, name).await
    }

    // ============================================================================
    // Bitcoin Validation (F1r3fly state validation against Bitcoin)
    // ============================================================================

    /// Validate F1r3fly allocation against Bitcoin blockchain
    /// This is a critical security method that ensures F1r3fly state matches Bitcoin reality
    pub async fn validate_f1r3fly_allocation(
        &self,
        allocation: &crate::firefly::types::Allocation,
    ) -> Result<bool, WalletError> {
        let validator = super::shared::BitcoinValidator::new(&self.config);
        
        validator.validate_allocation(allocation).await
            .map_err(|e| WalletError::Validation(e.to_string()))
    }

    /// Validate a state transition against Bitcoin
    pub async fn validate_f1r3fly_transition(
        &self,
        from_utxo: &str,
        to_utxo: &str,
        amount: u64,
        bitcoin_txid: &str,
    ) -> Result<bool, WalletError> {
        let validator = super::shared::BitcoinValidator::new(&self.config);
        
        validator.validate_transition(from_utxo, to_utxo, amount, bitcoin_txid).await
            .map_err(|e| WalletError::Validation(e.to_string()))
    }

    /// Check if a Bitcoin transaction is confirmed with sufficient confirmations
    pub async fn is_bitcoin_transaction_confirmed(
        &self,
        txid: &str,
        min_confirmations: Option<u32>,
    ) -> Result<bool, WalletError> {
        let validator = super::shared::BitcoinValidator::new(&self.config);
        
        validator.is_transaction_confirmed(txid, min_confirmations).await
            .map_err(|e| WalletError::Validation(e.to_string()))
    }

    pub async fn create_utxo(
        &self,
        name: &str,
        request: CreateUtxoRequest,
    ) -> Result<CreateUtxoResult, WalletError> {
        super::bitcoin_ops::create_utxo(&self.storage, &self.balance_checker, name, request).await
    }

    pub async fn unlock_utxo(
        &self,
        name: &str,
        request: UnlockUtxoRequest,
    ) -> Result<UnlockUtxoResult, WalletError> {
        super::bitcoin_ops::unlock_utxo(&self.storage, &self.balance_checker, name, request).await
    }

    pub async fn send_bitcoin(
        &self,
        name: &str,
        request: SendBitcoinRequest,
    ) -> Result<SendBitcoinResponse, WalletError> {
        super::bitcoin_ops::send_bitcoin(&self.storage, &self.balance_checker, name, request).await
    }

    // ============================================================================
    // RGB Operations - Async Wrappers (Public API)
    // ============================================================================

    /// Issue RGB asset (using ephemeral runtime like RGB CLI)
    pub async fn issue_asset(
        &self,
        wallet_name: &str,
        request: IssueAssetRequest,
    ) -> Result<IssueAssetResponse, WalletError> {
        // Route based on wallet mode
        let mode = self.storage.get_wallet_rgb_mode(wallet_name)
            .map_err(|e| WalletError::Internal(format!("Failed to get wallet mode: {}", e)))?;
        
        match mode {
            RgbMode::Traditional => {
                let storage = self.storage.clone();
                let rgb_mgr = self.rgb_runtime_manager.clone();
                let wallet_name = wallet_name.to_string();
                
                tokio::task::spawn_blocking(move || {
                    super::rgb_ops_traditional::issue_asset(&storage, &rgb_mgr, &wallet_name, request)
                })
                .await
                .map_err(|e| WalletError::Internal(format!("Issue asset task panicked: {}", e)))?
            }
            RgbMode::F1r3fly => {
                let firefly_client = self.firefly_client.clone()
                    .ok_or_else(|| WalletError::Internal("FireflyClient not initialized".to_string()))?;
                let storage = self.storage.clone();
                let wallet_name = wallet_name.to_string();
                
                tokio::task::spawn_blocking(move || {
                    super::rgb_ops_f1r3fly::issue_asset(&storage, &firefly_client, &wallet_name, request)
                })
                .await
                .map_err(|e| WalletError::Internal(format!("Issue asset task panicked: {}", e)))?
            }
        }
    }

    /// Generate RGB invoice (using ephemeral runtime like RGB CLI)
    pub async fn generate_rgb_invoice(
        &self,
        wallet_name: &str,
        request: GenerateInvoiceRequest,
    ) -> Result<GenerateInvoiceResult, WalletError> {
        // Route based on wallet mode
        let mode = self.storage.get_wallet_rgb_mode(wallet_name)
            .map_err(|e| WalletError::Internal(format!("Failed to get wallet mode: {}", e)))?;
        
        match mode {
            RgbMode::Traditional => {
                // Phase 1: Check if specific UTXO selection is requested (async operation)
                let utxo_info = match &request.utxo_selection {
                    Some(UtxoSelection::Specific { txid, vout }) => {
                        log::debug!("Looking up UTXO info for specific selection: {}:{}", txid, vout);
                        Some(
                            super::rgb_transfer_ops::find_utxo_for_selection(
                                &self.storage,
                                &self.balance_checker,
                                wallet_name,
                                txid,
                                *vout,
                            )
                            .await?,
                        )
                    }
                    _ => None,
                };
                
                // Phase 2: Generate invoice in blocking context (sync RGB operations)
                let storage = self.storage.clone();
                let rgb_mgr = self.rgb_runtime_manager.clone();
                let wallet_name = wallet_name.to_string();
                
                tokio::task::spawn_blocking(move || {
                    super::rgb_ops_traditional::generate_invoice(&storage, &rgb_mgr, &wallet_name, request, utxo_info)
                })
                .await
                .map_err(|e| WalletError::Internal(format!("Generate invoice task panicked: {}", e)))?
            }
            RgbMode::F1r3fly => {
                let firefly_client = self.firefly_client.clone()
                    .ok_or_else(|| WalletError::Internal("FireflyClient not initialized".to_string()))?;
                
                // For F1r3fly mode, we don't need UTXO lookup (state is in RSpace)
                // But we still need to handle the request
                let storage = self.storage.clone();
                let wallet_name = wallet_name.to_string();
                
                tokio::task::spawn_blocking(move || {
                    super::rgb_ops_f1r3fly::generate_invoice(&storage, &firefly_client, &wallet_name, request, None)
                })
                .await
                .map_err(|e| WalletError::Internal(format!("Generate invoice task panicked: {}", e)))?
            }
        }
    }

    /// Send RGB transfer (using ephemeral runtime like RGB CLI)
    pub async fn send_transfer(
        &self,
        wallet_name: &str,
        invoice_str: &str,
        fee_rate_sat_vb: Option<u64>,
    ) -> Result<SendTransferResponse, WalletError> {
        // Route based on wallet mode
        let mode = self.storage.get_wallet_rgb_mode(wallet_name)
            .map_err(|e| WalletError::Internal(format!("Failed to get wallet mode: {}", e)))?;
        
        match mode {
            RgbMode::Traditional => {
                let storage = self.storage.clone();
                let rgb_mgr = self.rgb_runtime_manager.clone();
                let wallet_name = wallet_name.to_string();
                let invoice_str = invoice_str.to_string();
                let public_url = self.config.public_url.clone();
                
                tokio::task::spawn_blocking(move || {
                    super::rgb_ops_traditional::send_transfer(&storage, &rgb_mgr, &wallet_name, &invoice_str, fee_rate_sat_vb, &public_url)
                })
                .await
                .map_err(|e| WalletError::Internal(format!("Send transfer task panicked: {}", e)))?
            }
            RgbMode::F1r3fly => {
                let firefly_client = self.firefly_client.clone()
                    .ok_or_else(|| WalletError::Internal("FireflyClient not initialized".to_string()))?;
                let storage = self.storage.clone();
                let wallet_name = wallet_name.to_string();
                let invoice_str = invoice_str.to_string();
                
                tokio::task::spawn_blocking(move || {
                    super::rgb_ops_f1r3fly::send_transfer(&storage, &firefly_client, &wallet_name, &invoice_str, fee_rate_sat_vb)
                })
                .await
                .map_err(|e| WalletError::Internal(format!("Send transfer task panicked: {}", e)))?
            }
        }
    }

    /// Accept RGB consignment (using ephemeral runtime like RGB CLI)
    pub async fn accept_consignment(
        &self,
        wallet_name: &str,
        consignment_bytes: Vec<u8>,
    ) -> Result<AcceptConsignmentResponse, WalletError> {
        // Route based on wallet mode
        let mode = self.storage.get_wallet_rgb_mode(wallet_name)
            .map_err(|e| WalletError::Internal(format!("Failed to get wallet mode: {}", e)))?;
        
        match mode {
            RgbMode::Traditional => {
                let storage = self.storage.clone();
                let rgb_mgr = self.rgb_runtime_manager.clone();
                let wallet_name = wallet_name.to_string();
                
                tokio::task::spawn_blocking(move || {
                    super::rgb_ops_traditional::accept_consignment(&storage, &rgb_mgr, &wallet_name, consignment_bytes)
                })
                .await
                .map_err(|e| WalletError::Internal(format!("Accept consignment task panicked: {}", e)))?
            }
            RgbMode::F1r3fly => {
                let firefly_client = self.firefly_client.clone()
                    .ok_or_else(|| WalletError::Internal("FireflyClient not initialized".to_string()))?;
                let storage = self.storage.clone();
                let wallet_name = wallet_name.to_string();
                
                tokio::task::spawn_blocking(move || {
                    super::rgb_ops_f1r3fly::accept_consignment(&storage, &firefly_client, &wallet_name, consignment_bytes)
                })
                .await
                .map_err(|e| WalletError::Internal(format!("Accept consignment task panicked: {}", e)))?
            }
        }
    }

    /// Export genesis consignment (using ephemeral runtime like RGB CLI)
    pub async fn export_genesis_consignment(
        &self,
        wallet_name: &str,
        contract_id_str: &str,
    ) -> Result<ExportGenesisResponse, WalletError> {
        // Route based on wallet mode
        let mode = self.storage.get_wallet_rgb_mode(wallet_name)
            .map_err(|e| WalletError::Internal(format!("Failed to get wallet mode: {}", e)))?;
        
        match mode {
            RgbMode::Traditional => {
                let storage = self.storage.clone();
                let rgb_mgr = self.rgb_runtime_manager.clone();
                let wallet_name = wallet_name.to_string();
                let contract_id_str = contract_id_str.to_string();
                let public_url = self.config.public_url.clone();
                
                tokio::task::spawn_blocking(move || {
                    super::rgb_ops_traditional::export_genesis(&storage, &rgb_mgr, &wallet_name, &contract_id_str, &public_url)
                })
                .await
                .map_err(|e| WalletError::Internal(format!("Export genesis task panicked: {}", e)))?
            }
            RgbMode::F1r3fly => {
                let firefly_client = self.firefly_client.clone()
                    .ok_or_else(|| WalletError::Internal("FireflyClient not initialized".to_string()))?;
                let storage = self.storage.clone();
                let wallet_name = wallet_name.to_string();
                let contract_id_str = contract_id_str.to_string();
                
                tokio::task::spawn_blocking(move || {
                    super::rgb_ops_f1r3fly::export_genesis(&storage, &firefly_client, &wallet_name, &contract_id_str)
                })
                .await
                .map_err(|e| WalletError::Internal(format!("Export genesis task panicked: {}", e)))?
            }
        }
    }

    // ============================================================================
    // RGB Sync Operations - Async Wrappers
    // ============================================================================

    /// Sync RGB runtime (using ephemeral runtime like RGB CLI)
    pub async fn sync_rgb_runtime(&self, wallet_name: &str) -> Result<(), WalletError> {
        // Route based on wallet mode
        let mode = self.storage.get_wallet_rgb_mode(wallet_name)
            .map_err(|e| WalletError::Internal(format!("Failed to get wallet mode: {}", e)))?;
        
        match mode {
            RgbMode::Traditional => {
                let storage = self.storage.clone();
                let rgb_mgr = self.rgb_runtime_manager.clone();
                let wallet_name = wallet_name.to_string();
                
                tokio::task::spawn_blocking(move || {
                    super::rgb_ops_traditional::sync_rgb_runtime(&storage, &rgb_mgr, &wallet_name)
                })
                .await
                .map_err(|e| WalletError::Internal(format!("Sync RGB task panicked: {}", e)))?
            }
            RgbMode::F1r3fly => {
                let firefly_client = self.firefly_client.clone()
                    .ok_or_else(|| WalletError::Internal("FireflyClient not initialized".to_string()))?;
                let storage = self.storage.clone();
                let wallet_name = wallet_name.to_string();
                
                tokio::task::spawn_blocking(move || {
                    super::rgb_ops_f1r3fly::sync_rgb_runtime(&storage, &firefly_client, &wallet_name)
                })
                .await
                .map_err(|e| WalletError::Internal(format!("Sync RGB task panicked: {}", e)))?
            }
        }
    }

    /// Sync RGB after state change (using ephemeral runtime like RGB CLI)
    pub async fn sync_rgb_after_state_change(&self, wallet_name: &str) -> Result<(), WalletError> {
        // Route based on wallet mode
        let mode = self.storage.get_wallet_rgb_mode(wallet_name)
            .map_err(|e| WalletError::Internal(format!("Failed to get wallet mode: {}", e)))?;
        
        match mode {
            RgbMode::Traditional => {
                let storage = self.storage.clone();
                let rgb_mgr = self.rgb_runtime_manager.clone();
                let wallet_name = wallet_name.to_string();
                
                tokio::task::spawn_blocking(move || {
                    super::rgb_ops_traditional::sync_rgb_after_state_change(&storage, &rgb_mgr, &wallet_name)
                })
                .await
                .map_err(|e| WalletError::Internal(format!("Sync RGB after state change task panicked: {}", e)))?
            }
            RgbMode::F1r3fly => {
                let firefly_client = self.firefly_client.clone()
                    .ok_or_else(|| WalletError::Internal("FireflyClient not initialized".to_string()))?;
                let storage = self.storage.clone();
                let wallet_name = wallet_name.to_string();
                
                tokio::task::spawn_blocking(move || {
                    super::rgb_ops_f1r3fly::sync_rgb_after_state_change(&storage, &firefly_client, &wallet_name)
                })
                .await
                .map_err(|e| WalletError::Internal(format!("Sync RGB after state change task panicked: {}", e)))?
            }
        }
    }

}
