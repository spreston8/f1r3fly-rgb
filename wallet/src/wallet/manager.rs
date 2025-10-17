/// Wallet Manager - Orchestration Layer
/// 
/// Coordinates all wallet operations by delegating to specialized operation modules.
use super::shared::*;
use crate::api::types::*;
use crate::error::WalletError;
use crate::firefly::FireflyClient;

pub struct WalletManager {
    pub storage: Storage,
    balance_checker: BalanceChecker,
    rgb_runtime_manager: RgbRuntimeManager,
    pub firefly_client: Option<FireflyClient>,
}

impl WalletManager {
    // ============================================================================
    // Constructor
    // ============================================================================

    pub fn new() -> Self {
        let storage = Storage::new();
        let rgb_runtime_manager =
            RgbRuntimeManager::new(storage.base_dir().clone(), bpstd::Network::Signet);
        
        // Initialize Firefly client (gRPC port 40401, HTTP port 40403)
        let firefly_client = Some(FireflyClient::new("localhost", 40401));
        
        Self {
            storage,
            balance_checker: BalanceChecker::new(),
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
        super::balance_ops::get_balance(
            &self.storage,
            &self.balance_checker,
            &self.rgb_runtime_manager,
            name,
        )
        .await
    }

    pub async fn sync_wallet(&self, name: &str) -> Result<SyncResult, WalletError> {
        super::sync_ops::sync_wallet(&self.storage, &self.balance_checker, name).await
    }

    pub fn sync_rgb_runtime(&self, name: &str) -> Result<(), WalletError> {
        super::sync_ops::sync_rgb_runtime(&self.storage, &self.rgb_runtime_manager, name)
    }

    pub fn sync_rgb_after_state_change(&self, name: &str) -> Result<(), WalletError> {
        super::sync_ops::sync_rgb_after_state_change(
            &self.storage,
            &self.rgb_runtime_manager,
            name,
        )
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
    // RGB Operations (delegates to rgb_transfer_ops and rgb_consignment_ops)
    // ============================================================================

    pub async fn generate_rgb_invoice(
        &self,
        wallet_name: &str,
        request: GenerateInvoiceRequest,
    ) -> Result<GenerateInvoiceResult, WalletError> {
        super::rgb_transfer_ops::generate_rgb_invoice(
            &self.storage,
            &self.rgb_runtime_manager,
            wallet_name,
            request,
            |wn, conf, msg| {
                super::sync_ops::sync_rgb_internal(
                    &self.storage,
                    &self.rgb_runtime_manager,
                    wn,
                    conf,
                    msg,
                )
            },
        )
        .await
    }

    pub fn send_transfer(
        &self,
        wallet_name: &str,
        invoice_str: &str,
        fee_rate_sat_vb: Option<u64>,
    ) -> Result<SendTransferResponse, WalletError> {
        super::rgb_transfer_ops::send_transfer(
            &self.storage,
            &self.rgb_runtime_manager,
            wallet_name,
            invoice_str,
            fee_rate_sat_vb,
            |wn, conf, msg| {
                super::sync_ops::sync_rgb_internal(
                    &self.storage,
                    &self.rgb_runtime_manager,
                    wn,
                    conf,
                    msg,
                )
            },
        )
    }

    pub fn accept_consignment(
        &self,
        wallet_name: &str,
        consignment_bytes: Vec<u8>,
    ) -> Result<AcceptConsignmentResponse, WalletError> {
        super::rgb_consignment_ops::accept_consignment(
            &self.storage,
            &self.rgb_runtime_manager,
            wallet_name,
            consignment_bytes,
            |wn| {
                super::sync_ops::sync_rgb_after_state_change(
                    &self.storage,
                    &self.rgb_runtime_manager,
                    wn,
                )
            },
        )
    }

    pub fn export_genesis_consignment(
        &self,
        wallet_name: &str,
        contract_id_str: &str,
    ) -> Result<ExportGenesisResponse, WalletError> {
        super::rgb_consignment_ops::export_genesis_consignment(
            &self.storage,
            &self.rgb_runtime_manager,
            wallet_name,
            contract_id_str,
        )
    }
}
