use crate::api::types::*;
use crate::bitcoin::balance_checker;
use crate::bitcoin::{
    send::send_bitcoin,
    utxo::{create_utxo, unlock_utxo},
    BalanceChecker,
};
use crate::config::WalletConfig;
use crate::error::WalletError;
use crate::rgb::{
    asset::{IssueAssetRequest, IssueAssetResponse, RgbManager},
    consignment::{accept_consignment, export_genesis_consignment},
    runtime::RgbRuntimeManager,
    transfer::{find_utxo_for_selection, generate_rgb_invoice_sync, send_transfer},
};
/// Wallet Manager - Orchestration Layer
///
/// Coordinates all wallet operations by delegating to specialized operation modules.
use crate::storage::Storage;
use crate::wallet::{
    addresses::{get_addresses, get_primary_address},
    lifecycle::{create_wallet, delete_wallet, import_wallet, list_wallets},
    sync::{sync_rgb_after_state_change, sync_rgb_internal, sync_rgb_runtime, sync_wallet},
};
use std::str::FromStr;

pub struct WalletManager {
    pub config: WalletConfig,
    pub storage: Storage,
    balance_checker: BalanceChecker,
    // Ephemeral runtime creation
    rgb_runtime_manager: RgbRuntimeManager,
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

        Self {
            config: config.clone(),
            storage,
            balance_checker: BalanceChecker::new(config.esplora_url.clone()),
            rgb_runtime_manager,
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

        Self {
            config: config.clone(),
            storage,
            balance_checker: BalanceChecker::new(config.esplora_url.clone()),
            rgb_runtime_manager,
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
        create_wallet(&self.storage, name)
    }

    pub fn import_wallet(
        &self,
        name: &str,
        mnemonic: bip39::Mnemonic,
    ) -> Result<WalletInfo, WalletError> {
        import_wallet(&self.storage, name, mnemonic)
    }

    pub fn list_wallets(&self) -> Result<Vec<WalletMetadata>, WalletError> {
        list_wallets(&self.storage)
    }

    pub fn delete_wallet(&self, name: &str) -> Result<(), WalletError> {
        delete_wallet(&self.storage, name)
    }

    // ============================================================================
    // Address Management (delegates to address_ops)
    // ============================================================================

    pub fn get_addresses(&self, name: &str, count: u32) -> Result<Vec<AddressInfo>, WalletError> {
        get_addresses(&self.storage, name, count)
    }

    pub fn get_primary_address(&self, name: &str) -> Result<NextAddressInfo, WalletError> {
        get_primary_address(&self.storage, name)
    }

    // ============================================================================
    // Balance & Sync (delegates to balance_ops and sync_ops)
    // ============================================================================

    pub async fn get_balance(
        &self,
        name: &str,
    ) -> Result<balance_checker::BalanceInfo, WalletError> {
        // Phase 1: Get Bitcoin balance (async HTTP)
        let mut balance = crate::bitcoin::balance::get_bitcoin_balance(
            &self.storage,
            &self.balance_checker,
            name,
        )
        .await?;

        // Phase 2: Get RGB balance (spawn_blocking for FileHolder operations)
        let rgb_mgr = self.rgb_runtime_manager.clone();
        let name_clone = name.to_string();

        let rgb_data = tokio::task::spawn_blocking(move || {
            crate::bitcoin::balance::get_rgb_balance_sync(&rgb_mgr, &name_clone)
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
        sync_wallet(&self.storage, &self.balance_checker, name).await
    }

    // ============================================================================
    // Bitcoin Operations (delegates to bitcoin_ops)
    // ============================================================================

    pub async fn create_utxo(
        &self,
        name: &str,
        request: CreateUtxoRequest,
    ) -> Result<CreateUtxoResponse, WalletError> {
        create_utxo(
            &self.storage,
            &self.balance_checker,
            &self.rgb_runtime_manager,
            name,
            request,
        )
        .await
    }

    pub async fn unlock_utxo(
        &self,
        name: &str,
        request: UnlockUtxoRequest,
    ) -> Result<UnlockUtxoResponse, WalletError> {
        unlock_utxo(
            &self.storage,
            &self.balance_checker,
            &self.rgb_runtime_manager,
            name,
            request,
        )
        .await
    }

    pub async fn send_bitcoin(
        &self,
        name: &str,
        request: SendBitcoinRequest,
    ) -> Result<SendBitcoinResponse, WalletError> {
        send_bitcoin(
            &self.storage,
            &self.balance_checker,
            &self.rgb_runtime_manager,
            name,
            request,
        )
        .await
    }

    // ============================================================================
    // RGB Operations - Async Wrappers (Public API)
    // ============================================================================

    /// Issue RGB asset
    pub async fn issue_asset(
        &self,
        wallet_name: &str,
        request: IssueAssetRequest,
    ) -> Result<IssueAssetResponse, WalletError> {
        let storage = self.storage.clone();
        let rgb_mgr = self.rgb_runtime_manager.clone();
        let wallet_name = wallet_name.to_string();

        tokio::task::spawn_blocking(move || {
            Self::issue_asset_blocking(&storage, &rgb_mgr, &wallet_name, request)
        })
        .await
        .map_err(|e| WalletError::Internal(format!("Issue asset task panicked: {}", e)))?
    }

    /// Generate RGB invoice
    pub async fn generate_rgb_invoice(
        &self,
        wallet_name: &str,
        request: GenerateInvoiceRequest,
    ) -> Result<GenerateInvoiceResponse, WalletError> {
        // Phase 1: Check if specific UTXO selection is requested (async operation)
        let utxo_info = match &request.utxo_selection {
            Some(UtxoSelection::Specific { txid, vout }) => {
                Some(
                    find_utxo_for_selection(
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
            Self::generate_rgb_invoice_blocking(
                &storage,
                &rgb_mgr,
                &wallet_name,
                request,
                utxo_info,
            )
        })
        .await
        .map_err(|e| WalletError::Internal(format!("Generate invoice task panicked: {}", e)))?
    }

    /// Send RGB transfer
    pub async fn send_transfer(
        &self,
        wallet_name: &str,
        invoice_str: &str,
        fee_rate_sat_vb: Option<u64>,
    ) -> Result<SendTransferResponse, WalletError> {
        let storage = self.storage.clone();
        let rgb_mgr = self.rgb_runtime_manager.clone();
        let wallet_name = wallet_name.to_string();
        let invoice_str = invoice_str.to_string();
        let public_url = self.config.public_url.clone();

        tokio::task::spawn_blocking(move || {
            Self::send_transfer_blocking(
                &storage,
                &rgb_mgr,
                &wallet_name,
                &invoice_str,
                fee_rate_sat_vb,
                &public_url,
            )
        })
        .await
        .map_err(|e| WalletError::Internal(format!("Send transfer task panicked: {}", e)))?
    }

    /// Accept RGB consignment
    pub async fn accept_consignment(
        &self,
        wallet_name: &str,
        consignment_bytes: Vec<u8>,
    ) -> Result<AcceptConsignmentResponse, WalletError> {
        let storage = self.storage.clone();
        let rgb_mgr = self.rgb_runtime_manager.clone();
        let wallet_name = wallet_name.to_string();

        tokio::task::spawn_blocking(move || {
            Self::accept_consignment_blocking(&storage, &rgb_mgr, &wallet_name, consignment_bytes)
        })
        .await
        .map_err(|e| WalletError::Internal(format!("Accept consignment task panicked: {}", e)))?
    }

    /// Export genesis consignment
    pub async fn export_genesis_consignment(
        &self,
        wallet_name: &str,
        contract_id_str: &str,
    ) -> Result<ExportGenesisResponse, WalletError> {
        let storage = self.storage.clone();
        let rgb_mgr = self.rgb_runtime_manager.clone();
        let wallet_name = wallet_name.to_string();
        let contract_id_str = contract_id_str.to_string();
        let public_url = self.config.public_url.clone();

        tokio::task::spawn_blocking(move || {
            Self::export_genesis_blocking(
                &storage,
                &rgb_mgr,
                &wallet_name,
                &contract_id_str,
                &public_url,
            )
        })
        .await
        .map_err(|e| WalletError::Internal(format!("Export genesis task panicked: {}", e)))?
    }

    // ============================================================================
    // RGB Sync Operations - Async Wrappers
    // ============================================================================

    /// Sync RGB runtime
    pub async fn sync_rgb_runtime(&self, wallet_name: &str) -> Result<(), WalletError> {
        let storage = self.storage.clone();
        let rgb_mgr = self.rgb_runtime_manager.clone();
        let wallet_name = wallet_name.to_string();

        tokio::task::spawn_blocking(move || {
            Self::sync_rgb_runtime_blocking(&storage, &rgb_mgr, &wallet_name)
        })
        .await
        .map_err(|e| WalletError::Internal(format!("Sync RGB task panicked: {}", e)))?
    }

    /// Sync RGB after state change
    pub async fn sync_rgb_after_state_change(&self, wallet_name: &str) -> Result<(), WalletError> {
        let storage = self.storage.clone();
        let rgb_mgr = self.rgb_runtime_manager.clone();
        let wallet_name = wallet_name.to_string();

        tokio::task::spawn_blocking(move || {
            Self::sync_rgb_after_state_change_blocking(&storage, &rgb_mgr, &wallet_name)
        })
        .await
        .map_err(|e| {
            WalletError::Internal(format!("Sync RGB after state change task panicked: {}", e))
        })?
    }

    // ============================================================================
    // Internal Blocking Methods (Private)
    // ============================================================================

    fn issue_asset_blocking(
        storage: &Storage,
        rgb_runtime_manager: &RgbRuntimeManager,
        wallet_name: &str,
        request: IssueAssetRequest,
    ) -> Result<IssueAssetResponse, WalletError> {
        if !storage.wallet_exists(wallet_name) {
            return Err(WalletError::WalletNotFound(wallet_name.to_string()));
        }

        use crate::rgb::asset::{RgbAssignment, RgbCreateParams, RgbIssuer};
        use chrono::Utc;
        use strict_types::TypeName;

        log::info!("Issuing RGB asset");

        // Parse genesis outpoint first
        let genesis_parts: Vec<&str> = request.genesis_utxo.split(':').collect();
        if genesis_parts.len() != 2 {
            return Err(WalletError::InvalidInput(format!(
                "Invalid genesis seal format: {}",
                request.genesis_utxo
            )));
        }

        let genesis_txid = bpstd::Txid::from_str(genesis_parts[0])
            .map_err(|e| WalletError::InvalidInput(format!("Invalid genesis txid: {}", e)))?;
        let genesis_vout = genesis_parts[1]
            .parse::<u32>()
            .map_err(|e| WalletError::InvalidInput(format!("Invalid genesis vout: {}", e)))?;
        let genesis_outpoint =
            bpstd::Outpoint::new(genesis_txid, bpstd::Vout::from_u32(genesis_vout));

        // Create ephemeral runtime (loads from disk)
        let mut runtime = rgb_runtime_manager.init_runtime_no_sync(wallet_name)?;

        {
            // 1. Load and import RGB20 issuer if needed
            let rgb_data_dir = storage.base_dir().join(wallet_name).join("rgb_data");
            let issuer_path = rgb_data_dir.join("RGB20-FNA.issuer");

            // Ensure the issuer file exists (create if missing)
            if !issuer_path.exists() {
                std::fs::write(&issuer_path, crate::rgb::asset::RGB20_ISSUER_BYTES).map_err(
                    |e| WalletError::Rgb(format!("Failed to create RGB20 issuer file: {}", e)),
                )?;
            }

            let issuer = crate::rgb::asset::RGB20_ISSUER.get_or_init(|| {
                RgbIssuer::load(
                    &issuer_path,
                    |_, _, _| -> Result<_, std::convert::Infallible> { Ok(()) },
                )
                .expect("Failed to load RGB20 issuer")
            });

            let codex_id = issuer.codex_id();

            // Import issuer if not already in contracts
            if !runtime.contracts.has_issuer(codex_id) {
                runtime
                    .contracts
                    .import_issuer(issuer.clone())
                    .map_err(|e| {
                        WalletError::Rgb(format!("Failed to import RGB20 issuer: {:?}", e))
                    })?;
            }

            // 2. Create contract params
            let type_name = TypeName::try_from(request.name.clone())
                .map_err(|e| WalletError::InvalidInput(format!("Invalid asset name: {:?}", e)))?;

            let mut params = RgbCreateParams::new_bitcoin_testnet(codex_id, type_name);

            // 3. Add global state
            params = params
                .with_global_verified("ticker", request.ticker.as_str())
                .with_global_verified("name", request.name.as_str())
                .with_global_verified(
                    "precision",
                    crate::rgb::asset::map_precision(request.precision),
                )
                .with_global_verified("issued", request.supply);

            // 4. Add owned state (initial allocation at genesis UTXO)
            params.push_owned_unlocked(
                "balance",
                RgbAssignment::new_internal(genesis_outpoint, request.supply),
            );

            params.timestamp = Some(Utc::now());

            // 5. Issue through runtime - this keeps contracts in sync
            let contract_id = runtime
                .issue(params)
                .map_err(|e| WalletError::Rgb(format!("Failed to issue contract: {:?}", e)))?;

            log::info!("Contract issued: {}", contract_id);

            Ok(IssueAssetResponse {
                contract_id: contract_id.to_string(),
                genesis_seal: request.genesis_utxo.clone(),
            })
        }
        // Runtime drops here → FileHolder::drop() auto-saves to disk (with genesis UTXO!)
    }

    fn generate_rgb_invoice_blocking(
        storage: &Storage,
        rgb_runtime_manager: &RgbRuntimeManager,
        wallet_name: &str,
        request: GenerateInvoiceRequest,
        utxo_info: Option<crate::api::types::UtxoInfo>,
    ) -> Result<GenerateInvoiceResponse, WalletError> {
        generate_rgb_invoice_sync(
            storage,
            rgb_runtime_manager,
            wallet_name,
            request,
            utxo_info,
        )
    }

    fn send_transfer_blocking(
        storage: &Storage,
        rgb_runtime_manager: &RgbRuntimeManager,
        wallet_name: &str,
        invoice_str: &str,
        fee_rate_sat_vb: Option<u64>,
        public_url: &str,
    ) -> Result<SendTransferResponse, WalletError> {
        send_transfer(
            storage,
            rgb_runtime_manager,
            wallet_name,
            invoice_str,
            fee_rate_sat_vb,
            public_url,
            |wn, conf, msg| sync_rgb_internal(storage, rgb_runtime_manager, wn, conf, msg),
        )
    }

    fn accept_consignment_blocking(
        storage: &Storage,
        rgb_runtime_manager: &RgbRuntimeManager,
        wallet_name: &str,
        consignment_bytes: Vec<u8>,
    ) -> Result<AcceptConsignmentResponse, WalletError> {
        accept_consignment(
            storage,
            rgb_runtime_manager,
            wallet_name,
            consignment_bytes,
            |wn| sync_rgb_after_state_change(storage, rgb_runtime_manager, wn),
        )
    }

    fn export_genesis_blocking(
        storage: &Storage,
        rgb_runtime_manager: &RgbRuntimeManager,
        wallet_name: &str,
        contract_id_str: &str,
        public_url: &str,
    ) -> Result<ExportGenesisResponse, WalletError> {
        export_genesis_consignment(
            storage,
            rgb_runtime_manager,
            wallet_name,
            contract_id_str,
            public_url,
        )
    }

    fn sync_rgb_runtime_blocking(
        storage: &Storage,
        rgb_runtime_manager: &RgbRuntimeManager,
        wallet_name: &str,
    ) -> Result<(), WalletError> {
        sync_rgb_runtime(storage, rgb_runtime_manager, wallet_name)
    }

    fn sync_rgb_after_state_change_blocking(
        storage: &Storage,
        rgb_runtime_manager: &RgbRuntimeManager,
        wallet_name: &str,
    ) -> Result<(), WalletError> {
        sync_rgb_after_state_change(storage, rgb_runtime_manager, wallet_name)
    }
}
