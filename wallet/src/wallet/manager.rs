/// Wallet Manager - Orchestration Layer
/// 
/// Coordinates all wallet operations by delegating to specialized operation modules.
use super::shared::*;
use super::shared::rgb::{IssueAssetRequest, IssueAssetResponse};
use crate::api::types::*;
use crate::config::WalletConfig;
use crate::error::WalletError;
use crate::firefly::FireflyClient;
use bpstd::psbt::PsbtConstructor;
use std::str::FromStr;
use std::sync::Arc;
use ::rgb::popls::bp::WalletProvider;
use ::rgb::RgbSealDef;
use commit_verify::{Digest, DigestExt, Sha256};

pub struct WalletManager {
    pub config: WalletConfig,
    pub storage: Storage,
    balance_checker: BalanceChecker,
    // OLD: Ephemeral runtime creation (Phase 1: keep for parallel operation)
    rgb_runtime_manager: RgbRuntimeManager,
    // NEW: Long-lived runtime cache (Phase 1: added, Phase 2: will use)
    pub rgb_runtime_cache: Arc<RgbRuntimeCache>,
    // NEW: Lifecycle manager for background tasks (Phase 1: added, Phase 2: will use)
    pub rgb_lifecycle_manager: Arc<RuntimeLifecycleManager>,
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
        
        // Initialize RGB runtime cache (Phase 1)
        let rgb_runtime_cache = Arc::new(RgbRuntimeCache::new(
            storage.base_dir().clone(),
            config.bpstd_network,
            config.esplora_url.clone(),
        ));
        
        // Initialize lifecycle manager (Phase 1)
        let lifecycle_config = LifecycleConfig::default();
        let rgb_lifecycle_manager = Arc::new(RuntimeLifecycleManager::new(
            rgb_runtime_cache.clone(),
            lifecycle_config,
        ));
        
        // Initialize Firefly client (gRPC port 40401, HTTP port 40403)
        let firefly_client = Some(FireflyClient::new("localhost", 40401));
        
        Self {
            config: config.clone(),
            storage,
            balance_checker: BalanceChecker::new(config.esplora_url.clone()),
            rgb_runtime_manager,
            rgb_runtime_cache,
            rgb_lifecycle_manager,
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
        
        // Initialize RGB runtime cache (Phase 1)
        let rgb_runtime_cache = Arc::new(RgbRuntimeCache::new(
            storage.base_dir().clone(),
            config.bpstd_network,
            config.esplora_url.clone(),
        ));
        
        // Initialize lifecycle manager (Phase 1)
        let lifecycle_config = LifecycleConfig::default();
        let rgb_lifecycle_manager = Arc::new(RuntimeLifecycleManager::new(
            rgb_runtime_cache.clone(),
            lifecycle_config,
        ));
        
        // No Firefly client in tests
        let firefly_client = None;
        
        Self {
            config: config.clone(),
            storage,
            balance_checker: BalanceChecker::new(config.esplora_url.clone()),
            rgb_runtime_manager,
            rgb_runtime_cache,
            rgb_lifecycle_manager,
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

    pub fn create_wallet(&self, name: &str) -> Result<WalletInfo, WalletError> {
        super::wallet_ops::create_wallet(&self.storage, name)
    }

    pub fn import_wallet(
        &self,
        name: &str,
        mnemonic: bip39::Mnemonic,
    ) -> Result<WalletInfo, WalletError> {
        super::wallet_ops::import_wallet(&self.storage, name, mnemonic)
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
        
        // Phase 2: Get RGB balance (spawn_blocking for FileHolder operations) with cached runtime
        let storage = self.storage.clone();
        let rgb_cache = self.rgb_runtime_cache.clone();
        let name_clone = name.to_string();
        let utxos_clone = balance.utxos.clone();
        
        let rgb_data = tokio::task::spawn_blocking(move || {
            super::balance_ops::get_rgb_balance_sync(&storage, &rgb_cache, &name_clone, &utxos_clone)
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
    // Bitcoin Operations (delegates to bitcoin_ops)
    // ============================================================================

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

    /// Issue RGB asset (Phase 2: Using cached runtime)
    pub async fn issue_asset(
        &self,
        wallet_name: &str,
        request: IssueAssetRequest,
    ) -> Result<IssueAssetResponse, WalletError> {
        let storage = self.storage.clone();
        let rgb_cache = self.rgb_runtime_cache.clone();
        let rgb_mgr = self.rgb_runtime_manager.clone();
        let wallet_name = wallet_name.to_string();
        
        tokio::task::spawn_blocking(move || {
            Self::issue_asset_blocking(&storage, &rgb_cache, &rgb_mgr, &wallet_name, request)
        })
        .await
        .map_err(|e| WalletError::Internal(format!("Issue asset task panicked: {}", e)))?
    }

    /// Generate RGB invoice (Phase 2: Using cached runtime)
    pub async fn generate_rgb_invoice(
        &self,
        wallet_name: &str,
        request: GenerateInvoiceRequest,
    ) -> Result<GenerateInvoiceResult, WalletError> {
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
        
        // Phase 2: Generate invoice in blocking context (sync RGB operations with cached runtime)
        let storage = self.storage.clone();
        let rgb_cache = self.rgb_runtime_cache.clone();
        let rgb_mgr = self.rgb_runtime_manager.clone();
        let wallet_name = wallet_name.to_string();
        
        tokio::task::spawn_blocking(move || {
            Self::generate_rgb_invoice_blocking(&storage, &rgb_cache, &rgb_mgr, &wallet_name, request, utxo_info)
        })
        .await
        .map_err(|e| WalletError::Internal(format!("Generate invoice task panicked: {}", e)))?
    }

    /// Send RGB transfer (Phase 2: Using cached runtime)
    pub async fn send_transfer(
        &self,
        wallet_name: &str,
        invoice_str: &str,
        fee_rate_sat_vb: Option<u64>,
    ) -> Result<SendTransferResponse, WalletError> {
        let storage = self.storage.clone();
        let rgb_cache = self.rgb_runtime_cache.clone();
        let rgb_mgr = self.rgb_runtime_manager.clone();
        let wallet_name = wallet_name.to_string();
        let invoice_str = invoice_str.to_string();
        
        tokio::task::spawn_blocking(move || {
            Self::send_transfer_blocking(&storage, &rgb_cache, &rgb_mgr, &wallet_name, &invoice_str, fee_rate_sat_vb)
        })
                .await
        .map_err(|e| WalletError::Internal(format!("Send transfer task panicked: {}", e)))?
    }

    /// Accept RGB consignment (Phase 2: Using cached runtime)
    pub async fn accept_consignment(
        &self,
        wallet_name: &str,
        consignment_bytes: Vec<u8>,
    ) -> Result<AcceptConsignmentResponse, WalletError> {
        let storage = self.storage.clone();
        let rgb_cache = self.rgb_runtime_cache.clone();
        let rgb_mgr = self.rgb_runtime_manager.clone();
        let wallet_name = wallet_name.to_string();
        
        tokio::task::spawn_blocking(move || {
            Self::accept_consignment_blocking(&storage, &rgb_cache, &rgb_mgr, &wallet_name, consignment_bytes)
        })
        .await
        .map_err(|e| WalletError::Internal(format!("Accept consignment task panicked: {}", e)))?
    }

    /// Export genesis consignment (Phase 2: Using cached runtime)
    pub async fn export_genesis_consignment(
        &self,
        wallet_name: &str,
        contract_id_str: &str,
    ) -> Result<ExportGenesisResponse, WalletError> {
        let storage = self.storage.clone();
        let rgb_cache = self.rgb_runtime_cache.clone();
        let rgb_mgr = self.rgb_runtime_manager.clone();
        let wallet_name = wallet_name.to_string();
        let contract_id_str = contract_id_str.to_string();
        
        tokio::task::spawn_blocking(move || {
            Self::export_genesis_blocking(&storage, &rgb_cache, &rgb_mgr, &wallet_name, &contract_id_str)
        })
        .await
        .map_err(|e| WalletError::Internal(format!("Export genesis task panicked: {}", e)))?
    }

    // ============================================================================
    // RGB Sync Operations - Async Wrappers
    // ============================================================================

    /// Sync RGB runtime (Phase 2: Using cached runtime)
    pub async fn sync_rgb_runtime(&self, wallet_name: &str) -> Result<(), WalletError> {
        let storage = self.storage.clone();
        let rgb_cache = self.rgb_runtime_cache.clone();
        let rgb_mgr = self.rgb_runtime_manager.clone();
        let wallet_name = wallet_name.to_string();
        
        tokio::task::spawn_blocking(move || {
            Self::sync_rgb_runtime_blocking(&storage, &rgb_cache, &rgb_mgr, &wallet_name)
        })
                .await
        .map_err(|e| WalletError::Internal(format!("Sync RGB task panicked: {}", e)))?
    }

    /// Sync RGB after state change (Phase 2: Using cached runtime)
    pub async fn sync_rgb_after_state_change(&self, wallet_name: &str) -> Result<(), WalletError> {
        let storage = self.storage.clone();
        let rgb_cache = self.rgb_runtime_cache.clone();
        let rgb_mgr = self.rgb_runtime_manager.clone();
        let wallet_name = wallet_name.to_string();
        
        tokio::task::spawn_blocking(move || {
            Self::sync_rgb_after_state_change_blocking(&storage, &rgb_cache, &rgb_mgr, &wallet_name)
        })
                .await
        .map_err(|e| WalletError::Internal(format!("Sync RGB after state change task panicked: {}", e)))?
    }

    // ============================================================================
    // Internal Blocking Methods (Private)
    // ============================================================================

    fn issue_asset_blocking(
        storage: &Storage,
        rgb_runtime_cache: &std::sync::Arc<super::shared::RgbRuntimeCache>,
        _rgb_runtime_manager: &RgbRuntimeManager,
        wallet_name: &str,
        request: IssueAssetRequest,
    ) -> Result<IssueAssetResponse, WalletError> {
        if !storage.wallet_exists(wallet_name) {
            return Err(WalletError::WalletNotFound(wallet_name.to_string()));
        }
        
        let rgb_data_dir = storage.base_dir().join(wallet_name).join("rgb_data");
        let rgb_manager = RgbManager::new(rgb_data_dir)?;
        
        log::debug!("Issuing asset to local stockpile...");
        let result = rgb_manager.issue_rgb20_asset(request)?;
        log::info!("Asset created successfully: {}", result.contract_id);
        
        // 🔧 CRITICAL: Register genesis seal in wallet descriptor (Phase 2: Using cached runtime)
        // Without this, the wallet knows about the tokens (in stockpile)
        // but can't find them when trying to spend (not in descriptor)
        log::debug!("Registering genesis seal in wallet descriptor: {}", result.genesis_seal);
        
        // Parse genesis outpoint from "txid:vout" string
        let genesis_parts: Vec<&str> = result.genesis_seal.split(':').collect();
        if genesis_parts.len() != 2 {
            return Err(WalletError::InvalidInput(format!(
                "Invalid genesis seal format: {}",
                result.genesis_seal
            )));
        }
        
        let genesis_txid = bpstd::Txid::from_str(genesis_parts[0]).map_err(|e| {
            WalletError::InvalidInput(format!("Invalid genesis txid: {}", e))
        })?;
        let genesis_vout = genesis_parts[1].parse::<u32>().map_err(|e| {
            WalletError::InvalidInput(format!("Invalid genesis vout: {}", e))
        })?;
        let genesis_outpoint = bpstd::Outpoint::new(genesis_txid, bpstd::Vout::from_u32(genesis_vout));
        log::debug!("Parsed genesis outpoint: {}:{}", genesis_txid, genesis_vout);
        
        // Get cached runtime and register genesis seal
        log::debug!("Acquiring cached RGB runtime for genesis seal registration");
        let guard = rgb_runtime_cache.get_or_create(wallet_name)?;
        
        guard.execute(|runtime| {
            // Create noise engine from descriptor's noise seed
            let noise_seed = runtime.wallet.descriptor().noise();
            let mut noise_engine = Sha256::new();
            noise_engine.input_raw(noise_seed.as_ref());
            
            // Get next nonce for seal blinding
            let nonce = runtime.wallet.next_nonce();
            log::debug!("Using nonce {} for genesis seal", nonce);
            
            // Create the genesis seal
            let genesis_seal = bpstd::seals::WTxoSeal::no_fallback(
                genesis_outpoint,
                noise_engine,
                nonce
            );
            log::debug!("Created genesis seal with auth token: {}", genesis_seal.auth_token());
            
            // Register it in the descriptor
            runtime.wallet.register_seal(genesis_seal);
            
            // Verify registration
            let seal_count = runtime.wallet.descriptor().seals().count();
            log::info!("Genesis seal registered in descriptor ({} seal(s) tracked)", seal_count);
            
            if seal_count == 0 {
                log::error!("Genesis seal registration failed - descriptor still has 0 seals");
                return Err(WalletError::Rgb(
                    "Failed to register genesis seal in descriptor".to_string()
                ));
            }
            
            // Sync runtime to update witness status for genesis seal
            log::debug!("Syncing RGB runtime after genesis seal registration");
            runtime.update(1).map_err(|e| {
                log::error!("Failed to sync RGB runtime: {:?}", e);
                WalletError::Rgb(format!("Failed to sync RGB state: {:?}", e))
            })?;
            log::info!("Genesis seal registered and synced in cached runtime");
            
            Ok(())
        })?;
        
        // CRITICAL: Evict cached runtime so next operation loads fresh contract from stockpile
        // The contract was added to stockpile by RgbManager, but cached runtime doesn't auto-reload
        log::debug!("Evicting cached runtime to force reload of newly issued contract");
        rgb_runtime_cache.evict(wallet_name)?;
        log::debug!("Cached runtime evicted - next operation will load fresh state");
        
        log::info!("Asset issuance complete");
        Ok(result)
    }

    fn generate_rgb_invoice_blocking(
        storage: &Storage,
        rgb_runtime_cache: &std::sync::Arc<super::shared::RgbRuntimeCache>,
        _rgb_runtime_manager: &RgbRuntimeManager,
        wallet_name: &str,
        request: GenerateInvoiceRequest,
        utxo_info: Option<crate::api::types::UtxoInfo>,
    ) -> Result<GenerateInvoiceResult, WalletError> {
        super::rgb_transfer_ops::generate_rgb_invoice_sync(
            storage,
            rgb_runtime_cache,
            wallet_name,
            request,
            utxo_info,
        )
    }

    fn send_transfer_blocking(
        storage: &Storage,
        rgb_runtime_cache: &std::sync::Arc<super::shared::RgbRuntimeCache>,
        _rgb_runtime_manager: &RgbRuntimeManager,
        wallet_name: &str,
        invoice_str: &str,
        fee_rate_sat_vb: Option<u64>,
    ) -> Result<SendTransferResponse, WalletError> {
        super::rgb_transfer_ops::send_transfer(
            storage,
            rgb_runtime_cache,
            wallet_name,
            invoice_str,
            fee_rate_sat_vb,
            |wn, conf, msg| {
                super::sync_ops::sync_rgb_internal(storage, rgb_runtime_cache, wn, conf, msg)
            },
        )
    }

    fn accept_consignment_blocking(
        storage: &Storage,
        rgb_runtime_cache: &std::sync::Arc<super::shared::RgbRuntimeCache>,
        _rgb_runtime_manager: &RgbRuntimeManager,
        wallet_name: &str,
        consignment_bytes: Vec<u8>,
    ) -> Result<AcceptConsignmentResponse, WalletError> {
        super::rgb_consignment_ops::accept_consignment(
            storage,
            rgb_runtime_cache,
            wallet_name,
            consignment_bytes,
            |wn| {
                super::sync_ops::sync_rgb_after_state_change(storage, rgb_runtime_cache, wn)
            },
        )
    }

    fn export_genesis_blocking(
        storage: &Storage,
        rgb_runtime_cache: &std::sync::Arc<super::shared::RgbRuntimeCache>,
        _rgb_runtime_manager: &RgbRuntimeManager,
        wallet_name: &str,
        contract_id_str: &str,
    ) -> Result<ExportGenesisResponse, WalletError> {
        super::rgb_consignment_ops::export_genesis_consignment(
            storage,
            rgb_runtime_cache,
            wallet_name,
            contract_id_str,
        )
    }

    fn sync_rgb_runtime_blocking(
        storage: &Storage,
        rgb_runtime_cache: &std::sync::Arc<super::shared::RgbRuntimeCache>,
        _rgb_runtime_manager: &RgbRuntimeManager,
        wallet_name: &str,
    ) -> Result<(), WalletError> {
        super::sync_ops::sync_rgb_runtime(storage, rgb_runtime_cache, wallet_name)
    }

    fn sync_rgb_after_state_change_blocking(
        storage: &Storage,
        rgb_runtime_cache: &std::sync::Arc<super::shared::RgbRuntimeCache>,
        _rgb_runtime_manager: &RgbRuntimeManager,
        wallet_name: &str,
    ) -> Result<(), WalletError> {
        super::sync_ops::sync_rgb_after_state_change(storage, rgb_runtime_cache, wallet_name)
    }

    // ============================================================================
    // Lifecycle Management (Phase 1)
    // ============================================================================

    /// Start the RGB runtime lifecycle manager background tasks
    /// 
    /// This should be called once at server startup to begin:
    /// - Auto-save loop: Periodically saves dirty runtimes
    /// - Idle cleanup loop: Evicts idle runtimes to prevent memory leaks
    pub fn start_lifecycle_manager(self: &Arc<Self>) {
        let manager = self.rgb_lifecycle_manager.clone();
        tokio::spawn(async move {
            manager.start().await;
        });
        log::info!("RGB runtime lifecycle manager started");
    }

    /// Shutdown the lifecycle manager and save all RGB state
    /// 
    /// This should be called during server shutdown to ensure:
    /// - Background tasks are stopped
    /// - All dirty runtimes are saved to disk
    pub async fn shutdown(&self) -> Result<(), WalletError> {
        log::info!("Initiating WalletManager shutdown...");
        self.rgb_lifecycle_manager.shutdown().await?;
        log::info!("WalletManager shutdown complete");
        Ok(())
    }
}
