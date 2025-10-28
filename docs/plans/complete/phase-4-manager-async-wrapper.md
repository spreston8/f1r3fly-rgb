# Phase 4: Manager Async Wrapper Layer

**Date:** October 17, 2025  
**Status:** Proposed  
**Estimated Effort:** 1-2 hours  
**Risk Level:** Low (refactoring, no logic changes)

---

## Executive Summary

Refactor `WalletManager` to expose a **fully async public API** while internally using `spawn_blocking` for all RGB operations. This consolidates spawn_blocking logic in one place, fixes existing bugs, and creates a stable API that won't need future rework.

**Goal:** All handlers become 3-5 lines, all manager methods are async, all RGB blocking operations are safely wrapped.

---

## Current State Issues

### 1. Inconsistent spawn_blocking Usage

| Handler | Has spawn_blocking? | Status |
|---------|---------------------|--------|
| `issue_asset_handler` | ✅ YES | Working |
| `send_transfer_handler` | ✅ YES | Working |
| `accept_consignment_handler` | ✅ YES | Working |
| `issue_asset_with_firefly_handler` | ❌ NO | **BUG** - calls RGB sync on async thread |
| `sync_rgb_handler` | ❌ NO | **BUG** - calls RGB sync on async thread |
| `generate_invoice_handler` | ❌ NO | **BUG** - generate_rgb_invoice is wrongly async |

### 2. generate_rgb_invoice is Incorrectly Async

```rust
// Currently marked async but contains blocking RGB calls:
pub async fn generate_rgb_invoice(...) -> Result<...> {
    let mut runtime = rgb_runtime_manager.init_runtime_no_sync(wallet_name)?;
    runtime.update(1)?;  // ❌ BLOCKING on async context!
}
```

### 3. Handler Boilerplate

Every handler with spawn_blocking repeats 8-10 lines:
```rust
let manager_clone = Arc::clone(&manager);
let name_clone = name.clone();
let req_clone = req.clone();

tokio::task::spawn_blocking(move || {
    manager_clone.some_method(&name_clone, req_clone)
})
.await
.map_err(|e| WalletError::Internal(format!("Task panicked: {}", e)))??;
```

### 4. No Clear API Boundary

- Some manager methods are async (Bitcoin ops)
- Some are sync (RGB ops)
- Handlers must know which need spawn_blocking
- Easy to make mistakes

---

## Proposed Architecture

### Design Principle

**"Manager = Async API, Internals = Sync + spawn_blocking"**

```
HTTP Handler (async)
    ↓
Manager Public Method (async)
    ↓ tokio::task::spawn_blocking
Manager Private Method (sync)
    ↓
RGB Operations (sync)
```

### Manager API Categories

| Category | Methods | Current | Target | spawn_blocking? |
|----------|---------|---------|--------|-----------------|
| **RGB Operations** | issue_asset, send_transfer, accept_consignment, generate_invoice, export_genesis, sync_rgb | Mixed | async | ✅ YES |
| **Bitcoin Operations** | send_bitcoin, create_utxo, unlock_utxo, get_balance | async | async | Already handled |
| **Fast Operations** | create_wallet, list_wallets, delete_wallet, get_addresses | sync | sync | ❌ NO (< 1ms) |

---

## Implementation Plan

### Step 1: Update manager.rs - Add Async Wrappers

**File:** `wallet/src/wallet/manager.rs`

Add async wrapper methods for all RGB operations:

```rust
impl WalletManager {
    // ========================================
    // PUBLIC ASYNC API (RGB Operations)
    // ========================================
    
    /// Issue RGB asset (async wrapper)
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
    
    /// Send RGB transfer (async wrapper)
    pub async fn send_transfer(
        &self,
        wallet_name: &str,
        invoice: &str,
        fee_rate: Option<u64>,
    ) -> Result<SendTransferResponse, WalletError> {
        let storage = self.storage.clone();
        let rgb_mgr = self.rgb_runtime_manager.clone();
        let wallet_name = wallet_name.to_string();
        let invoice = invoice.to_string();
        
        tokio::task::spawn_blocking(move || {
            Self::send_transfer_blocking(&storage, &rgb_mgr, &wallet_name, &invoice, fee_rate)
        })
        .await
        .map_err(|e| WalletError::Internal(format!("Transfer task panicked: {}", e)))?
    }
    
    /// Accept RGB consignment (async wrapper)
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
    
    /// Generate RGB invoice (async wrapper)
    pub async fn generate_rgb_invoice(
        &self,
        wallet_name: &str,
        request: GenerateInvoiceRequest,
    ) -> Result<GenerateInvoiceResult, WalletError> {
        let storage = self.storage.clone();
        let rgb_mgr = self.rgb_runtime_manager.clone();
        let wallet_name = wallet_name.to_string();
        
        tokio::task::spawn_blocking(move || {
            Self::generate_rgb_invoice_blocking(&storage, &rgb_mgr, &wallet_name, request)
        })
        .await
        .map_err(|e| WalletError::Internal(format!("Generate invoice task panicked: {}", e)))?
    }
    
    /// Export genesis consignment (async wrapper)
    pub async fn export_genesis_consignment(
        &self,
        wallet_name: &str,
        contract_id: &str,
    ) -> Result<ExportGenesisResponse, WalletError> {
        let storage = self.storage.clone();
        let rgb_mgr = self.rgb_runtime_manager.clone();
        let wallet_name = wallet_name.to_string();
        let contract_id = contract_id.to_string();
        
        tokio::task::spawn_blocking(move || {
            Self::export_genesis_blocking(&storage, &rgb_mgr, &wallet_name, &contract_id)
        })
        .await
        .map_err(|e| WalletError::Internal(format!("Export genesis task panicked: {}", e)))?
    }
    
    /// Sync RGB runtime (async wrapper)
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
    
    // Bitcoin operations - already async, keep as-is
    // pub async fn send_bitcoin(...) { ... }
    // pub async fn create_utxo(...) { ... }
    // pub async fn unlock_utxo(...) { ... }
    // pub async fn get_balance(...) { ... }
    
    // Fast operations - stay sync, no spawn_blocking needed
    // pub fn create_wallet(...) { ... }
    // pub fn list_wallets(...) { ... }
    // pub fn delete_wallet(...) { ... }
    // pub fn get_addresses(...) { ... }
    
    // ========================================
    // INTERNAL BLOCKING METHODS
    // ========================================
    
    fn issue_asset_blocking(
        storage: &Storage,
        rgb_runtime_manager: &RgbRuntimeManager,
        wallet_name: &str,
        request: IssueAssetRequest,
    ) -> Result<IssueAssetResponse, WalletError> {
        if !storage.wallet_exists(wallet_name) {
            return Err(WalletError::WalletNotFound(wallet_name.to_string()));
        }
        
        let rgb_data_dir = storage.base_dir().join(wallet_name).join("rgb_data");
        let rgb_manager = RgbManager::new(rgb_data_dir)?;
        let result = rgb_manager.issue_rgb20_asset(request)?;
        
        super::sync_ops::sync_rgb_after_state_change(storage, rgb_runtime_manager, wallet_name)?;
        
        Ok(result)
    }
    
    fn send_transfer_blocking(
        storage: &Storage,
        rgb_runtime_manager: &RgbRuntimeManager,
        wallet_name: &str,
        invoice: &str,
        fee_rate: Option<u64>,
    ) -> Result<SendTransferResponse, WalletError> {
        super::rgb_transfer_ops::send_transfer(
            storage,
            rgb_runtime_manager,
            wallet_name,
            invoice,
            fee_rate,
            |wn, conf, msg| {
                super::sync_ops::sync_rgb_internal(storage, rgb_runtime_manager, wn, conf, msg)
            },
        )
    }
    
    fn accept_consignment_blocking(
        storage: &Storage,
        rgb_runtime_manager: &RgbRuntimeManager,
        wallet_name: &str,
        consignment_bytes: Vec<u8>,
    ) -> Result<AcceptConsignmentResponse, WalletError> {
        super::rgb_consignment_ops::accept_consignment(
            storage,
            rgb_runtime_manager,
            wallet_name,
            consignment_bytes,
            |wn| {
                super::sync_ops::sync_rgb_after_state_change(storage, rgb_runtime_manager, wn)
            },
        )
    }
    
    fn generate_rgb_invoice_blocking(
        storage: &Storage,
        rgb_runtime_manager: &RgbRuntimeManager,
        wallet_name: &str,
        request: GenerateInvoiceRequest,
    ) -> Result<GenerateInvoiceResult, WalletError> {
        super::rgb_transfer_ops::generate_rgb_invoice_sync(
            storage,
            rgb_runtime_manager,
            wallet_name,
            request,
        )
    }
    
    fn export_genesis_blocking(
        storage: &Storage,
        rgb_runtime_manager: &RgbRuntimeManager,
        wallet_name: &str,
        contract_id: &str,
    ) -> Result<ExportGenesisResponse, WalletError> {
        super::rgb_consignment_ops::export_genesis_consignment(
            storage,
            rgb_runtime_manager,
            wallet_name,
            contract_id,
        )
    }
    
    fn sync_rgb_runtime_blocking(
        storage: &Storage,
        rgb_runtime_manager: &RgbRuntimeManager,
        wallet_name: &str,
    ) -> Result<(), WalletError> {
        super::sync_ops::sync_rgb_runtime(storage, rgb_runtime_manager, wallet_name)
    }
}
```

### Step 2: Fix generate_rgb_invoice (Remove Async)

**File:** `wallet/src/wallet/rgb_transfer_ops.rs`

```rust
// Change from:
pub async fn generate_rgb_invoice(...) -> Result<...> { ... }

// To:
pub fn generate_rgb_invoice_sync(
    storage: &Storage,
    rgb_runtime_manager: &RgbRuntimeManager,
    wallet_name: &str,
    request: GenerateInvoiceRequest,
) -> Result<GenerateInvoiceResult, WalletError> {
    // ... existing logic (already sync, just remove async + await)
}
```

### Step 3: Simplify All Handlers

**File:** `wallet/src/api/handlers.rs`

Replace all verbose handlers with clean async calls:

```rust
// issue_asset_handler
pub async fn issue_asset_handler(
    State(manager): State<Arc<WalletManager>>,
    Path(name): Path<String>,
    Json(req): Json<IssueAssetRequest>,
) -> Result<Json<IssueAssetResponse>, crate::error::WalletError> {
    let result = manager.issue_asset(&name, req).await?;
    Ok(Json(result))
}

// send_transfer_handler
pub async fn send_transfer_handler(
    State(manager): State<Arc<WalletManager>>,
    Path(wallet_name): Path<String>,
    Json(request): Json<SendTransferRequest>,
) -> Result<Json<SendTransferResponse>, crate::error::WalletError> {
    let result = manager.send_transfer(&wallet_name, &request.invoice, request.fee_rate_sat_vb).await?;
    Ok(Json(result))
}

// accept_consignment_handler
pub async fn accept_consignment_handler(
    State(manager): State<Arc<WalletManager>>,
    Path(wallet_name): Path<String>,
    body: axum::body::Bytes,
) -> Result<Json<AcceptConsignmentResponse>, crate::error::WalletError> {
    let result = manager.accept_consignment(&wallet_name, body.to_vec()).await?;
    Ok(Json(result))
}

// generate_invoice_handler
pub async fn generate_invoice_handler(
    State(manager): State<Arc<WalletManager>>,
    Path(name): Path<String>,
    Json(req): Json<GenerateInvoiceRequest>,
) -> Result<Json<GenerateInvoiceResponse>, crate::error::WalletError> {
    let result = manager.generate_rgb_invoice(&name, req).await?;
    Ok(Json(GenerateInvoiceResponse {
        invoice: result.invoice,
        contract_id: result.contract_id,
        amount: result.amount,
        seal_utxo: result.seal_utxo,
    }))
}

// sync_rgb_handler
pub async fn sync_rgb_handler(
    State(manager): State<Arc<WalletManager>>,
    Path(name): Path<String>,
) -> Result<Json<()>, crate::error::WalletError> {
    manager.sync_rgb_runtime(&name).await?;
    Ok(Json(()))
}

// export_genesis_handler
pub async fn export_genesis_handler(
    State(manager): State<Arc<WalletManager>>,
    Path((wallet_name, contract_id)): Path<(String, String)>,
) -> Result<Json<ExportGenesisResponse>, crate::error::WalletError> {
    let result = manager.export_genesis_consignment(&wallet_name, &contract_id).await?;
    Ok(Json(result))
}

// issue_asset_with_firefly_handler
pub async fn issue_asset_with_firefly_handler(
    State(manager): State<Arc<WalletManager>>,
    Path(name): Path<String>,
    Json(req): Json<IssueAssetRequest>,
) -> Result<Json<IssueAssetResponseWithFirefly>, crate::error::WalletError> {
    // Get Firefly client
    let firefly_client = manager.firefly_client.as_ref()
        .ok_or_else(|| WalletError::Internal("Firefly client not initialized".into()))?;
    
    // Issue asset with Firefly (already async)
    let rgb_manager = manager.get_rgb_manager(&name)?;
    let result = rgb_manager.issue_rgb20_asset_with_firefly(req, firefly_client).await?;
    
    // Sync RGB runtime (now async!)
    manager.sync_rgb_runtime(&name).await?;
    
    Ok(Json(result))
}
```

---

## Benefits

### 1. Consistency
- ✅ All RGB operations use spawn_blocking (centralized in manager)
- ✅ All manager public methods are async (uniform API)
- ✅ All handlers are clean and simple (3-5 lines)

### 2. Safety
- ✅ No missing spawn_blocking (manager handles it)
- ✅ No blocking on async context (generate_rgb_invoice fixed)
- ✅ No runtime panics (proper thread pool usage)

### 3. Maintainability
- 📉 Handler complexity: ~15 lines → ~5 lines (70% reduction)
- 📦 Centralized logic: spawn_blocking only in manager
- 🧪 Testable: Test async manager methods directly

### 4. Future-Proof
- 🔄 When RGB goes async: Only change manager `_blocking` internals
- 🔒 API stable: Handlers never need to change again
- 📚 Documentation: "All manager methods are async"

---

## Bug Fixes

This plan fixes 3 existing bugs:

1. **issue_asset_with_firefly_handler** - Now calls `manager.sync_rgb_runtime().await` (async)
2. **sync_rgb_handler** - Now calls `manager.sync_rgb_runtime().await` (async)
3. **generate_rgb_invoice** - Now properly wrapped in spawn_blocking

---

## Validation Criteria

After implementation, verify:

- [ ] All handlers compile without warnings
- [ ] All RGB operations complete without panics
- [ ] `cargo test` passes
- [ ] Manual test: Issue asset → Generate invoice → Send transfer → Accept consignment
- [ ] Manual test: Issue asset with Firefly (no panic during sync)
- [ ] Grep for `spawn_blocking` in handlers → should only be in manager.rs

---

## Rollback Plan

If issues arise:
1. Revert manager.rs changes
2. Revert handlers.rs changes
3. Keep rgb_transfer_ops.rs sync (it's correct)

Git will preserve the previous working state.

---

## Conclusion

This refactoring achieves the **final, optimal architecture** for the RGB wallet given the constraints:
- RGB requires FileHolder (blocking)
- Axum requires async handlers (non-blocking)
- Solution: spawn_blocking wrapper layer in manager

After this change, the architecture is **stable and won't need rework** even when RGB eventually adds async support (we'll just update manager internals, not the API).

**Recommendation:** Proceed with implementation.

