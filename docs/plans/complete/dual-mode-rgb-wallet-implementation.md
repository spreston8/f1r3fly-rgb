# Dual-Mode RGB Wallet Implementation Plan

**Version:** 1.0  
**Date:** November 1, 2025  
**Status:** In Progress

---

## **Architecture: Separate Wallets Per Mode**

### **Core Principle:**
- Each wallet has an **immutable RGB mode** set at creation time
- Traditional wallets use consignment-based RGB operations
- F1r3fly wallets use F1r3fly state-based RGB operations
- **No mode switching** - create separate wallets for each mode
- Clean separation, no state migration complexity

---

## **Current State Analysis**

### **What We Have:**
```
✅ Traditional RGB Operations (wallet/src/wallet/)
   ├── manager.rs - Orchestrates RGB operations
   ├── rgb_transfer_ops.rs - Send/receive transfers
   ├── rgb_consignment_ops.rs - Accept/export consignments
   └── Uses: RGB runtime + local stash

✅ F1r3fly Storage Layer (wallet/src/firefly/)
   ├── client.rs - FireflyClient with all RGB methods
   ├── types.rs - F1r3fly data structures
   ├── registry.rs - Registry operations
   └── Uses: F1r3fly node + Bitcoin validation

✅ Integration Tests (wallet/tests/integration/f1r3fly/)
   ├── test_contract_metadata.rs
   ├── test_allocations.rs
   └── test_transitions.rs

❌ Missing: Connection between WalletManager and FireflyClient
❌ Missing: RGB mode flag in wallet metadata
❌ Missing: F1r3fly-backed RGB operations
```

### **Storage Structure:**
```
./wallets/
├── alice/                    # Traditional wallet
│   ├── metadata.json         # { name, created_at, network }
│   ├── mnemonic.txt
│   ├── descriptor.txt
│   ├── state.json
│   └── rgb_data/             # Local RGB stash
│       ├── stash.dat
│       └── contracts/
│
└── bob/                      # F1r3fly wallet (future)
    ├── metadata.json         # { name, created_at, network, rgb_mode: "f1r3fly" }
    ├── mnemonic.txt
    ├── descriptor.txt
    ├── state.json
    └── rgb_data/             # Still exists for RGB runtime (security)
        └── contracts/        # But queries go to F1r3fly, not local stash
```

---

## **Implementation Plan**

### **Phase 1: Add RGB Mode Infrastructure** (2-3 hours)

#### **Step 1.1: Create RgbMode enum**
**File:** `wallet/src/wallet/shared/rgb_mode.rs` (NEW)
```rust
use serde::{Deserialize, Serialize};

/// RGB operation mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RgbMode {
    /// Traditional consignment-based RGB operations
    Traditional,
    /// F1r3fly state-based RGB operations
    F1r3fly,
}

impl Default for RgbMode {
    fn default() -> Self {
        RgbMode::Traditional
    }
}

impl std::fmt::Display for RgbMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RgbMode::Traditional => write!(f, "traditional"),
            RgbMode::F1r3fly => write!(f, "f1r3fly"),
        }
    }
}
```

#### **Step 1.2: Update storage metadata**
**File:** `wallet/src/wallet/shared/storage.rs`
```rust
// Update Metadata struct
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub network: String,
    #[serde(default)]  // For backward compatibility
    pub rgb_mode: RgbMode,
}

// Add helper method
impl Storage {
    pub fn get_wallet_rgb_mode(&self, name: &str) -> Result<RgbMode, crate::error::StorageError> {
        let metadata = self.load_metadata(name)?;
        Ok(metadata.rgb_mode)
    }
}
```

#### **Step 1.3: Update API types**
**File:** `wallet/src/api/types.rs`
```rust
#[derive(Debug, Deserialize)]
pub struct CreateWalletRequest {
    pub name: String,
    pub rgb_mode: Option<RgbMode>,  // Optional, defaults to Traditional
}

#[derive(Debug, Serialize)]
pub struct WalletInfo {
    pub name: String,
    pub mnemonic: String,
    pub first_address: String,
    pub public_address: String,
    pub descriptor: String,
    pub rgb_mode: RgbMode,  // Show mode in response
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletMetadata {
    pub name: String,
    pub created_at: String,
    pub last_synced: Option<String>,
    pub rgb_mode: RgbMode,  // Show mode in list
}
```

#### **Step 1.4: Update wallet creation**
**File:** `wallet/src/wallet/wallet_ops.rs`
```rust
pub fn create_wallet(
    storage: &Storage,
    name: &str,
    rgb_mode: RgbMode,  // NEW parameter
) -> Result<WalletInfo, WalletError> {
    if storage.wallet_exists(name) {
        return Err(WalletError::WalletExists(name.to_string()));
    }

    let keys = KeyManager::generate()?;
    storage.create_wallet(name)?;

    let metadata = Metadata {
        name: name.to_string(),
        created_at: Utc::now(),
        network: "signet".to_string(),
        rgb_mode,  // Store mode
    };
    storage.save_metadata(name, &metadata)?;
    storage.save_mnemonic(name, &keys.mnemonic)?;
    storage.save_descriptor(name, &keys.descriptor)?;

    let first_address = AddressManager::derive_address(&keys.descriptor, 0, keys.network)?;

    Ok(WalletInfo {
        name: name.to_string(),
        mnemonic: keys.mnemonic.to_string(),
        first_address: first_address.to_string(),
        public_address: first_address.to_string(),
        descriptor: keys.descriptor,
        rgb_mode,  // Return mode
    })
}

// Also update import_wallet() with same changes
```

#### **Step 1.5: Update WalletManager**
**File:** `wallet/src/wallet/manager.rs`
```rust
impl WalletManager {
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
}
```

#### **Step 1.6: Update API handler**
**File:** `wallet/src/api/handlers.rs`
```rust
pub async fn create_wallet_handler(
    State(manager): State<Arc<WalletManager>>,
    Json(req): Json<CreateWalletRequest>,
) -> Result<Json<WalletInfo>, crate::error::WalletError> {
    let rgb_mode = req.rgb_mode.unwrap_or(RgbMode::Traditional);
    let wallet_info = manager.create_wallet(&req.name, rgb_mode)?;
    Ok(Json(wallet_info))
}

pub async fn import_wallet_handler(
    State(manager): State<Arc<WalletManager>>,
    Json(req): Json<ImportWalletRequest>,
) -> Result<Json<WalletInfo>, crate::error::WalletError> {
    let mnemonic = bip39::Mnemonic::parse(&req.mnemonic)
        .map_err(|e| crate::error::WalletError::InvalidInput(format!("Invalid mnemonic: {}", e)))?;

    let rgb_mode = req.rgb_mode.unwrap_or(RgbMode::Traditional);
    let wallet_info = manager.import_wallet(&req.name, mnemonic, rgb_mode)?;
    Ok(Json(wallet_info))
}

// Update list_wallets_handler to include rgb_mode in response
```

#### **Step 1.7: Update shared/mod.rs**
**File:** `wallet/src/wallet/shared/mod.rs`
```rust
pub mod rgb_mode;  // Add this line

pub use rgb_mode::RgbMode;  // Re-export for convenience
```

**Deliverable:** Can create wallets with RGB mode flag, mode persists, shows in API responses

---

### **Phase 2: Refactor Traditional RGB Operations** (2-3 hours)

#### **Step 2.1: Extract Traditional RGB methods**
**File:** `wallet/src/wallet/rgb_ops_traditional.rs` (NEW)
```rust
/// Traditional RGB operations (consignment-based)
/// 
/// This module contains the original RGB implementation using:
/// - RGB runtime for validation and signing
/// - Local stash for state storage
/// - Consignment files for state transfer

use super::shared::*;
use crate::error::WalletError;
use crate::api::types::*;

// Move these methods from manager.rs:
// - issue_asset_blocking() → issue_asset()
// - generate_rgb_invoice_blocking() → generate_invoice()
// - send_transfer_blocking() → send_transfer()
// - accept_consignment_blocking() → accept_consignment()
// - export_genesis_blocking() → export_genesis()
// - sync_rgb_runtime_blocking() → sync_rgb_runtime()
// - sync_rgb_after_state_change_blocking() → sync_rgb_after_state_change()

// Keep all existing logic unchanged - just move and rename
```

#### **Step 2.2: Update WalletManager to route based on mode**
**File:** `wallet/src/wallet/manager.rs`
```rust
impl WalletManager {
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
                // Delegate to traditional implementation
                let storage = self.storage.clone();
                let rgb_mgr = self.rgb_runtime_manager.clone();
                let wallet_name = wallet_name.to_string();
                
                tokio::task::spawn_blocking(move || {
                    rgb_ops_traditional::issue_asset(&storage, &rgb_mgr, &wallet_name, request)
                })
                .await
                .map_err(|e| WalletError::Internal(format!("Issue asset task panicked: {}", e)))?
            }
            RgbMode::F1r3fly => {
                // Not implemented yet - return error
                Err(WalletError::InvalidOperation(
                    "F1r3fly mode not yet implemented. Use Traditional mode for now.".to_string()
                ))
            }
        }
    }
    
    // Repeat for all RGB methods:
    // - generate_rgb_invoice()
    // - send_transfer()
    // - accept_consignment()
    // - export_genesis_consignment()
    // - sync_rgb_runtime()
    // - sync_rgb_after_state_change()
}
```

#### **Step 2.3: Update mod.rs**
**File:** `wallet/src/wallet/mod.rs`
```rust
mod rgb_ops_traditional;  // Add this line
```

**Deliverable:** Traditional RGB operations still work, code is cleaner, mode routing in place

---

### **Phase 3: Implement F1r3fly RGB Operations** (1-2 days)

#### **Step 3.1: Create F1r3fly operations module**
**File:** `wallet/src/wallet/rgb_ops_f1r3fly.rs` (NEW)
```rust
/// F1r3fly RGB operations (state-based)
/// 
/// This module contains F1r3fly-backed RGB implementation using:
/// - RGB runtime for validation and signing (security)
/// - F1r3fly node for state storage (coordination)
/// - Bitcoin blockchain for final validation (trust anchor)

use super::shared::*;
use crate::error::WalletError;
use crate::firefly::FireflyClient;
use crate::firefly::types::*;
use crate::api::types::*;

/// Issue RGB asset with F1r3fly state storage
pub async fn issue_asset(
    storage: &Storage,
    rgb_mgr: &RgbRuntimeManager,
    firefly_client: &FireflyClient,
    wallet_name: &str,
    request: IssueAssetRequest,
) -> Result<IssueAssetResponse, WalletError> {
    // Step 1: Issue via RGB runtime (for security/validity)
    // Use traditional issue_asset to create contract
    let storage_clone = storage.clone();
    let rgb_mgr_clone = rgb_mgr.clone();
    let wallet_name_clone = wallet_name.to_string();
    let request_clone = request.clone();
    
    let response = tokio::task::spawn_blocking(move || {
        rgb_ops_traditional::issue_asset(&storage_clone, &rgb_mgr_clone, &wallet_name_clone, request_clone)
    })
    .await
    .map_err(|e| WalletError::Internal(format!("Issue asset task panicked: {}", e)))??;
    
    // Step 2: Store contract metadata on F1r3fly
    let metadata = ContractMetadata {
        ticker: request.ticker.clone(),
        name: request.name.clone(),
        precision: request.precision,
        total_supply: request.supply,
        genesis_txid: "pending".to_string(),  // Will be filled after Bitcoin TX
        issuer_pubkey: get_wallet_pubkey(storage, wallet_name)?,
    };
    
    let deploy_id = firefly_client
        .store_contract(&response.contract_id, metadata)
        .await
        .map_err(|e| WalletError::Internal(format!("Failed to store contract on F1r3fly: {}", e)))?;
    
    // Wait for F1r3fly finalization
    let block_hash = firefly_client
        .wait_for_deploy(&deploy_id, 60)
        .await
        .map_err(|e| WalletError::Internal(format!("Failed to wait for deploy: {}", e)))?;
    
    firefly_client
        .wait_for_block_finalization(&block_hash, 24)
        .await
        .map_err(|e| WalletError::Internal(format!("Failed to wait for finalization: {}", e)))?;
    
    // Step 3: Store initial allocation on F1r3fly
    let owner_pubkey = get_wallet_pubkey(storage, wallet_name)?;
    
    let deploy_id = firefly_client
        .store_allocation(
            &response.contract_id,
            &response.genesis_seal,
            &owner_pubkey,
            request.supply,
            "pending_bitcoin_txid",  // Will be updated after Bitcoin confirmation
        )
        .await
        .map_err(|e| WalletError::Internal(format!("Failed to store allocation: {}", e)))?;
    
    // Wait for F1r3fly finalization
    let block_hash = firefly_client
        .wait_for_deploy(&deploy_id, 60)
        .await
        .map_err(|e| WalletError::Internal(format!("Failed to wait for deploy: {}", e)))?;
    
    firefly_client
        .wait_for_block_finalization(&block_hash, 24)
        .await
        .map_err(|e| WalletError::Internal(format!("Failed to wait for finalization: {}", e)))?;
    
    log::info!("✓ Asset issued and stored on F1r3fly: {}", response.contract_id);
    
    Ok(response)
}

/// Generate RGB invoice (F1r3fly mode queries F1r3fly for allocations)
pub async fn generate_invoice(
    storage: &Storage,
    rgb_mgr: &RgbRuntimeManager,
    _firefly_client: &FireflyClient,
    wallet_name: &str,
    request: GenerateInvoiceRequest,
    utxo_info: Option<UtxoInfo>,
) -> Result<GenerateInvoiceResult, WalletError> {
    // For now, delegate to traditional (invoice generation doesn't need F1r3fly)
    // In the future, could query F1r3fly to verify contract exists
    let storage_clone = storage.clone();
    let rgb_mgr_clone = rgb_mgr.clone();
    let wallet_name_clone = wallet_name.to_string();
    
    tokio::task::spawn_blocking(move || {
        rgb_ops_traditional::generate_invoice(&storage_clone, &rgb_mgr_clone, &wallet_name_clone, request, utxo_info)
    })
    .await
    .map_err(|e| WalletError::Internal(format!("Generate invoice task panicked: {}", e)))?
}

/// Send RGB transfer with F1r3fly state update
pub async fn send_transfer(
    storage: &Storage,
    rgb_mgr: &RgbRuntimeManager,
    _firefly_client: &FireflyClient,
    wallet_name: &str,
    invoice_str: &str,
    fee_rate_sat_vb: Option<u64>,
    public_url: &str,
) -> Result<SendTransferResponse, WalletError> {
    // Step 1: Send transfer via traditional method (creates Bitcoin TX)
    let storage_clone = storage.clone();
    let rgb_mgr_clone = rgb_mgr.clone();
    let wallet_name_clone = wallet_name.to_string();
    let invoice_clone = invoice_str.to_string();
    let public_url_clone = public_url.to_string();
    
    let response = tokio::task::spawn_blocking(move || {
        rgb_ops_traditional::send_transfer(
            &storage_clone,
            &rgb_mgr_clone,
            &wallet_name_clone,
            &invoice_clone,
            fee_rate_sat_vb,
            &public_url_clone,
        )
    })
    .await
    .map_err(|e| WalletError::Internal(format!("Send transfer task panicked: {}", e)))??;
    
    // Step 2: Record transition on F1r3fly
    // TODO: Extract from_utxo, to_utxo, amount from invoice/response
    // firefly_client.record_transition(...).await?;
    
    log::info!("✓ Transfer sent and recorded on F1r3fly: {}", response.bitcoin_txid);
    
    Ok(response)
}

/// Accept consignment (F1r3fly mode validates against F1r3fly state)
pub async fn accept_consignment(
    storage: &Storage,
    rgb_mgr: &RgbRuntimeManager,
    _firefly_client: &FireflyClient,
    wallet_name: &str,
    consignment_bytes: Vec<u8>,
) -> Result<AcceptConsignmentResponse, WalletError> {
    // For Phase 1, delegate to traditional
    // In Phase 2, add F1r3fly notification listening
    let storage_clone = storage.clone();
    let rgb_mgr_clone = rgb_mgr.clone();
    let wallet_name_clone = wallet_name.to_string();
    
    tokio::task::spawn_blocking(move || {
        rgb_ops_traditional::accept_consignment(&storage_clone, &rgb_mgr_clone, &wallet_name_clone, consignment_bytes)
    })
    .await
    .map_err(|e| WalletError::Internal(format!("Accept consignment task panicked: {}", e)))?
}

/// Export genesis consignment
pub async fn export_genesis(
    storage: &Storage,
    rgb_mgr: &RgbRuntimeManager,
    _firefly_client: &FireflyClient,
    wallet_name: &str,
    contract_id_str: &str,
    public_url: &str,
) -> Result<ExportGenesisResponse, WalletError> {
    // Delegate to traditional
    let storage_clone = storage.clone();
    let rgb_mgr_clone = rgb_mgr.clone();
    let wallet_name_clone = wallet_name.to_string();
    let contract_id_clone = contract_id_str.to_string();
    let public_url_clone = public_url.to_string();
    
    tokio::task::spawn_blocking(move || {
        rgb_ops_traditional::export_genesis(&storage_clone, &rgb_mgr_clone, &wallet_name_clone, &contract_id_clone, &public_url_clone)
    })
    .await
    .map_err(|e| WalletError::Internal(format!("Export genesis task panicked: {}", e)))?
}

// Helper function
fn get_wallet_pubkey(storage: &Storage, wallet_name: &str) -> Result<String, WalletError> {
    let mnemonic = storage.load_mnemonic(wallet_name)
        .map_err(|e| WalletError::Internal(format!("Failed to load mnemonic: {}", e)))?;
    let keys = KeyManager::from_mnemonic(&mnemonic.to_string())?;
    // Derive public key from descriptor
    // For now, return placeholder
    Ok("TODO_derive_pubkey_from_descriptor".to_string())
}
```

#### **Step 3.2: Update WalletManager to use F1r3fly ops**
**File:** `wallet/src/wallet/manager.rs`
```rust
impl WalletManager {
    pub async fn issue_asset(
        &self,
        wallet_name: &str,
        request: IssueAssetRequest,
    ) -> Result<IssueAssetResponse, WalletError> {
        let mode = self.storage.get_wallet_rgb_mode(wallet_name)
            .map_err(|e| WalletError::Internal(format!("Failed to get wallet mode: {}", e)))?;
        
        match mode {
            RgbMode::Traditional => {
                let storage = self.storage.clone();
                let rgb_mgr = self.rgb_runtime_manager.clone();
                let wallet_name = wallet_name.to_string();
                
                tokio::task::spawn_blocking(move || {
                    rgb_ops_traditional::issue_asset(&storage, &rgb_mgr, &wallet_name, request)
                })
                .await
                .map_err(|e| WalletError::Internal(format!("Issue asset task panicked: {}", e)))?
            }
            RgbMode::F1r3fly => {
                let firefly_client = self.firefly_client.as_ref()
                    .ok_or_else(|| WalletError::Internal("F1r3fly client not initialized".to_string()))?;
                
                rgb_ops_f1r3fly::issue_asset(
                    &self.storage,
                    &self.rgb_runtime_manager,
                    firefly_client,
                    wallet_name,
                    request,
                ).await
            }
        }
    }
    
    // Repeat for other RGB methods:
    // - generate_rgb_invoice()
    // - send_transfer()
    // - accept_consignment()
    // - export_genesis_consignment()
}
```

#### **Step 3.3: Update mod.rs**
**File:** `wallet/src/wallet/mod.rs`
```rust
mod rgb_ops_f1r3fly;  // Add this line
```

**Deliverable:** F1r3fly wallets can issue assets and store state on F1r3fly

---

### **Phase 4: Testing & Validation** (1 day)

#### **Step 4.1: Unit tests for mode infrastructure**
**File:** `wallet/tests/rgb_mode_test.rs` (NEW)
```rust
use wallet::wallet::shared::{Storage, RgbMode};
use wallet::wallet::wallet_ops::create_wallet;
use tempfile::tempdir;

#[test]
fn test_create_traditional_wallet() {
    let temp = tempdir().unwrap();
    let storage = Storage::new_with_base_dir(temp.path().to_path_buf());
    let wallet = create_wallet(&storage, "alice", RgbMode::Traditional).unwrap();
    assert_eq!(wallet.rgb_mode, RgbMode::Traditional);
}

#[test]
fn test_create_f1r3fly_wallet() {
    let temp = tempdir().unwrap();
    let storage = Storage::new_with_base_dir(temp.path().to_path_buf());
    let wallet = create_wallet(&storage, "bob", RgbMode::F1r3fly).unwrap();
    assert_eq!(wallet.rgb_mode, RgbMode::F1r3fly);
}

#[test]
fn test_mode_persists_across_restarts() {
    let temp = tempdir().unwrap();
    let storage = Storage::new_with_base_dir(temp.path().to_path_buf());
    create_wallet(&storage, "alice", RgbMode::F1r3fly).unwrap();
    
    // Reload
    let mode = storage.get_wallet_rgb_mode("alice").unwrap();
    assert_eq!(mode, RgbMode::F1r3fly);
}

#[test]
fn test_default_mode_is_traditional() {
    let temp = tempdir().unwrap();
    let storage = Storage::new_with_base_dir(temp.path().to_path_buf());
    let wallet = create_wallet(&storage, "charlie", RgbMode::default()).unwrap();
    assert_eq!(wallet.rgb_mode, RgbMode::Traditional);
}
```

#### **Step 4.2: Integration tests for F1r3fly operations**
**File:** `wallet/tests/integration/f1r3fly_wallet_test.rs` (NEW)
```rust
use wallet::wallet::WalletManager;
use wallet::wallet::shared::RgbMode;
use wallet::wallet::shared::rgb::IssueAssetRequest;

#[tokio::test]
async fn test_f1r3fly_wallet_issue_asset() {
    // Setup
    let manager = WalletManager::new();
    
    // Create F1r3fly wallet
    let wallet = manager.create_wallet("bob", RgbMode::F1r3fly).unwrap();
    assert_eq!(wallet.rgb_mode, RgbMode::F1r3fly);
    
    // Issue asset
    let request = IssueAssetRequest {
        ticker: "TEST".to_string(),
        name: "Test Token".to_string(),
        precision: 8,
        supply: 1_000_000,
        genesis_utxo: "0000000000000000000000000000000000000000000000000000000000000000:0".to_string(),
    };
    
    let response = manager.issue_asset("bob", request).await.unwrap();
    
    // Verify: Asset stored on F1r3fly
    let firefly_client = manager.firefly_client.as_ref().unwrap();
    let contract = firefly_client.query_contract(&response.contract_id, None).await.unwrap();
    assert!(contract.success);
    assert_eq!(contract.contract.unwrap().ticker, "TEST");
}

#[tokio::test]
async fn test_traditional_wallet_still_works() {
    let manager = WalletManager::new();
    
    // Create Traditional wallet
    let wallet = manager.create_wallet("alice", RgbMode::Traditional).unwrap();
    assert_eq!(wallet.rgb_mode, RgbMode::Traditional);
    
    // Traditional operations should still work
    // (Test basic operations)
}
```

**Deliverable:** All tests pass, both modes work correctly

---

### **Phase 5: Documentation & Frontend Support** (1 day)

#### **Step 5.1: API documentation**
Update API docs to show mode parameter:
```
POST /wallets
{
  "name": "alice",
  "rgb_mode": "traditional"  // or "f1r3fly"
}

Response:
{
  "name": "alice",
  "mnemonic": "...",
  "first_address": "...",
  "public_address": "...",
  "descriptor": "...",
  "rgb_mode": "traditional"
}

GET /wallets
Response:
[
  {
    "name": "alice",
    "created_at": "2025-11-01T...",
    "last_synced": "Height: 12345",
    "rgb_mode": "traditional"
  },
  {
    "name": "bob",
    "created_at": "2025-11-01T...",
    "last_synced": null,
    "rgb_mode": "f1r3fly"
  }
]
```

#### **Step 5.2: Frontend integration guide**
```javascript
// Create Traditional wallet
POST /wallets
{ "name": "alice", "rgb_mode": "traditional" }

// Create F1r3fly wallet
POST /wallets
{ "name": "bob", "rgb_mode": "f1r3fly" }

// Issue asset (mode is automatic based on wallet)
POST /wallets/alice/rgb/issue
{ "ticker": "TEST", ... }  // Uses Traditional

POST /wallets/bob/rgb/issue
{ "ticker": "TEST", ... }  // Uses F1r3fly
```

**Deliverable:** Clear documentation for frontend integration

---

## **File Structure (After Implementation)**

```
wallet/src/
├── wallet/
│   ├── manager.rs                    # Orchestration + mode routing
│   ├── rgb_ops_traditional.rs        # Traditional RGB operations (NEW)
│   ├── rgb_ops_f1r3fly.rs           # F1r3fly RGB operations (NEW)
│   ├── wallet_ops.rs                 # Updated with rgb_mode parameter
│   └── shared/
│       ├── rgb_mode.rs               # RgbMode enum (NEW)
│       ├── storage.rs                # Updated with rgb_mode in Metadata
│       └── ...
├── firefly/
│   ├── client.rs                     # Unchanged (already complete)
│   ├── types.rs                      # Unchanged
│   └── registry.rs                   # Unchanged
└── api/
    ├── handlers.rs                   # Updated to accept rgb_mode
    └── types.rs                      # Updated with rgb_mode fields

wallet/tests/
├── rgb_mode_test.rs                  # Mode infrastructure tests (NEW)
└── integration/
    ├── f1r3fly_wallet_test.rs       # F1r3fly wallet tests (NEW)
    └── f1r3fly/                      # Existing F1r3fly storage tests
        ├── test_contract_metadata.rs
        ├── test_allocations.rs
        └── test_transitions.rs
```

---

## **Timeline Estimate**

| Phase | Description | Time | Complexity |
|-------|-------------|------|------------|
| 1 | RGB Mode Infrastructure | 2-3 hours | Low |
| 2 | Refactor Traditional RGB | 2-3 hours | Low |
| 3 | Implement F1r3fly RGB | 1-2 days | Medium |
| 4 | Testing & Validation | 1 day | Medium |
| 5 | Documentation | 1 day | Low |
| **Total** | | **3-5 days** | |

---

## **Success Criteria**

### **Phase 1 Complete:**
- ✅ Can create wallet with `rgb_mode: "traditional"`
- ✅ Can create wallet with `rgb_mode: "f1r3fly"`
- ✅ Mode persists in metadata.json
- ✅ Mode shows in API responses

### **Phase 2 Complete:**
- ✅ Traditional RGB operations still work
- ✅ Code is cleaner (extracted to separate module)
- ✅ Mode routing in place (Traditional works, F1r3fly returns error)

### **Phase 3 Complete:**
- ✅ F1r3fly wallet can issue asset
- ✅ Asset metadata stored on F1r3fly
- ✅ Initial allocation stored on F1r3fly
- ✅ Can query contract from F1r3fly
- ✅ Can query allocation from F1r3fly

### **Phase 4 Complete:**
- ✅ All unit tests pass
- ✅ All integration tests pass
- ✅ Both modes work end-to-end

### **Phase 5 Complete:**
- ✅ API documentation updated
- ✅ Frontend integration guide written
- ✅ Ready for demo

---

## **Frontend Demo Flow (After Implementation)**

```
┌─────────────────────────────────────────────────────────────┐
│                    RGB Wallet Demo                          │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Step 1: Create Wallets                                    │
│  ┌──────────────────────┐    ┌──────────────────────┐     │
│  │ Create Wallet        │    │ Create Wallet        │     │
│  │ Name: Alice          │    │ Name: Bob            │     │
│  │ Mode: Traditional    │    │ Mode: F1r3fly        │     │
│  │ [Create]             │    │ [Create]             │     │
│  └──────────────────────┘    └──────────────────────┘     │
│                                                             │
│  Step 2: Fund Wallets (Bitcoin)                            │
│  Alice: 0.01 BTC         Bob: 0.01 BTC                     │
│                                                             │
│  Step 3: Issue Assets                                      │
│  Alice issues TEST token → Takes 30s, generates consignment│
│  Bob issues TEST token   → Takes 2s, stored on F1r3fly ✨  │
│                                                             │
│  Step 4: Compare                                           │
│  ┌──────────────────────┐    ┌──────────────────────┐     │
│  │ Alice (Traditional)  │    │ Bob (F1r3fly)        │     │
│  │ Balance: 1000 TEST   │    │ Balance: 1000 TEST   │     │
│  │ Storage: Local stash │    │ Storage: F1r3fly ✨  │     │
│  │ Transfer: Consignment│    │ Transfer: Instant ✨ │     │
│  └──────────────────────┘    └──────────────────────┘     │
└─────────────────────────────────────────────────────────────┘
```

---

## **Key Decisions Summary**

1. **Separate Wallets:** Each wallet has immutable mode (no switching)
2. **Hybrid F1r3fly Mode:** RGB runtime (security) + F1r3fly (coordination)
3. **Backward Compatible:** Existing wallets default to Traditional
4. **Clean Separation:** `rgb_ops_traditional.rs` vs `rgb_ops_f1r3fly.rs`
5. **Mode Routing:** `WalletManager` routes based on wallet mode
6. **No Migration:** Create new wallet for different mode (Phase 1)

---

## **Progress Tracking**

### **Phase 1: RGB Mode Infrastructure**
- [ ] Step 1.1: Create RgbMode enum
- [ ] Step 1.2: Update storage metadata
- [ ] Step 1.3: Update API types
- [ ] Step 1.4: Update wallet creation
- [ ] Step 1.5: Update WalletManager
- [ ] Step 1.6: Update API handler
- [ ] Step 1.7: Update shared/mod.rs

### **Phase 2: Refactor Traditional RGB**
- [ ] Step 2.1: Extract Traditional RGB methods
- [ ] Step 2.2: Update WalletManager routing
- [ ] Step 2.3: Update mod.rs

### **Phase 3: Implement F1r3fly RGB**
- [ ] Step 3.1: Create F1r3fly operations module
- [ ] Step 3.2: Update WalletManager F1r3fly routing
- [ ] Step 3.3: Update mod.rs

### **Phase 4: Testing & Validation**
- [ ] Step 4.1: Unit tests for mode infrastructure
- [ ] Step 4.2: Integration tests for F1r3fly operations

### **Phase 5: Documentation**
- [ ] Step 5.1: API documentation
- [ ] Step 5.2: Frontend integration guide

---

## **Notes**

- After each step, verify code compiles before proceeding
- Test traditional RGB operations continue to work after refactoring
- F1r3fly client must be initialized in WalletManager for F1r3fly mode to work
- Ensure RGB storage contract is deployed before testing F1r3fly operations

