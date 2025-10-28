# Minimal Fix: Resolve Broadcast Error with spawn_blocking

**Goal:** Fix runtime panic during RGB transfers without major refactor

**Status:** Phase 0 (Planning)

**Date:** October 17, 2025

---

## Current Problem

### **The Error**
```rust
// wallet/src/wallet/rgb_transfer_ops.rs:430-433
let handle = tokio::runtime::Handle::try_current()?;
handle.block_on(tokio::task::spawn_blocking(move || {  // ❌ PANIC HERE
    let client = reqwest::blocking::Client::new();
    // ...
}))
```

**Why It Fails:**
- Handler is async (runs on Tokio worker thread)
- Handler calls `send_transfer()` (sync)
- `send_transfer()` calls `broadcast_tx_hex_blocking()` (sync)
- `broadcast_tx_hex_blocking()` tries to `block_on()` **on a Tokio worker thread**
- Tokio forbids this → Panic: "Cannot drop a runtime in a context where blocking is not allowed"

---

## Solution Strategy

### **Approach: "Sync Core + Spawn Blocking Wrapper"**

**Architecture:**
```
API Handler (async)
    ↓ tokio::task::spawn_blocking
WalletManager (sync)
    ↓ sync call
RGB Transfer Ops (sync)
    ↓ simple blocking HTTP
broadcast_tx_hex (sync with reqwest::blocking)
```

**Key Principle:** Move the async/sync boundary to the **handler level** where we have full control.

**Why This Works:**
- ✅ Keeps RGB operations sync (preserves `fs` feature for file persistence)
- ✅ No runtime nesting conflicts
- ✅ Minimal code changes
- ✅ Follows Tokio best practices for blocking operations

---

## Implementation Plan

### **Phase 1: Add Missing Type Requirements** (5 minutes)

**Problem:** `spawn_blocking` requires `Clone + Send + 'static`

**Changes Required:**

#### 1.1 Add Clone to Storage

```rust
// wallet/src/wallet/shared/storage.rs

#[derive(Clone)]  // ← ADD THIS
pub struct Storage {
    base_path: PathBuf,
}
```

#### 1.2 Add Clone to RgbRuntimeManager

```rust
// wallet/src/wallet/shared/rgb_runtime.rs

#[derive(Clone)]  // ← ADD THIS
pub struct RgbRuntimeManager {
    base_path: PathBuf,
    network: Network,
}
```

**Verification:**
```bash
cd wallet
cargo check
```

**Expected:** Compiles successfully

---

### **Phase 2: Fix Broadcast Function** (10 minutes)

**Goal:** Remove runtime nesting, use simple blocking HTTP

#### 2.1 Replace broadcast_tx_hex_blocking

```rust
// wallet/src/wallet/rgb_transfer_ops.rs

// REMOVE the problematic function:
// fn broadcast_tx_hex_blocking(tx_hex: &str) -> Result<(), WalletError> { ... }

// REPLACE with simple blocking version:
fn broadcast_tx_hex(tx_hex: &str) -> Result<String, WalletError> {
    log::debug!("Broadcasting transaction to mempool.space");
    
    // Simple blocking HTTP client (we're on a blocking thread pool now)
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| WalletError::Network(format!("Failed to create HTTP client: {}", e)))?;
    
    let response = client
        .post("https://mempool.space/signet/api/tx")
        .header("Content-Type", "text/plain")
        .body(tx_hex.to_string())
        .send()
        .map_err(|e| WalletError::Network(format!("Broadcast request failed: {}", e)))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response
            .text()
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(WalletError::Network(format!(
            "Broadcast failed with status {}: {}",
            status, error_text
        )));
    }

    let txid = response
        .text()
        .map_err(|e| WalletError::Network(format!("Failed to read response: {}", e)))?;
    
    log::info!("Transaction broadcast successful: {}", txid.trim());
    Ok(txid.trim().to_string())
}
```

#### 2.2 Update Call Site

```rust
// wallet/src/wallet/rgb_transfer_ops.rs (in send_transfer function)

// CHANGE FROM:
broadcast_tx_hex_blocking(&tx_hex)?;

// TO:
let txid = broadcast_tx_hex(&tx_hex)?;
log::info!("Transfer broadcast successful: {}", txid);

// Update return value to include txid (if SendTransferResponse expects it)
```

**Why This Works:**
- ✅ No runtime nesting
- ✅ Simple blocking call (will run on blocking thread pool via spawn_blocking)
- ✅ Returns txid for better tracking
- ✅ Proper timeout handling (30 seconds)
- ✅ Clear error messages

**Verification:**
```bash
cargo check
```

**Expected:** Compiles successfully

---

### **Phase 3: Wrap Handler Calls in spawn_blocking** (15 minutes)

**Goal:** Move blocking operations off Tokio worker threads

#### 3.1 Send Transfer Handler

```rust
// wallet/src/api/handlers.rs

pub async fn send_transfer_handler(
    State(manager): State<Arc<WalletManager>>,
    Path(name): Path<String>,
    Json(req): Json<SendTransferRequest>,
) -> Result<Json<SendTransferResponse>, WalletError> {
    log::info!("Send transfer initiated for wallet: {}", name);
    
    // Clone Arc for move into spawn_blocking
    let manager_clone = Arc::clone(&manager);
    let name_clone = name.clone();
    let invoice = req.invoice.clone();
    let fee = req.fee_sats;
    
    // Run blocking RGB operations on dedicated thread pool
    let result = tokio::task::spawn_blocking(move || {
        manager_clone.send_transfer(&name_clone, &invoice, fee)
    })
    .await
    .map_err(|e| WalletError::Internal(format!("Transfer task panicked: {}", e)))??;
    
    Ok(Json(result))
}
```

**Pattern Explanation:**
```rust
// Double ?? is needed:
// First ? unwraps JoinError (from spawn_blocking panicking)
// Second ? unwraps WalletError (from the actual operation)
tokio::task::spawn_blocking(|| operation())
    .await  // Returns Result<Result<T, WalletError>, JoinError>
    .map_err(|e| WalletError::Internal(...))?  // Handle JoinError
    ?  // Handle WalletError
```

#### 3.2 Issue Asset Handler

```rust
// wallet/src/api/handlers.rs

pub async fn issue_asset_handler(
    State(manager): State<Arc<WalletManager>>,
    Path(name): Path<String>,
    Json(req): Json<IssueAssetRequest>,
) -> Result<Json<IssueAssetResponse>, WalletError> {
    log::info!("Starting RGB asset issuance: {} ({})", req.name, req.ticker);
    
    let manager_clone = Arc::clone(&manager);
    let name_clone = name.clone();
    
    // Run blocking RGB operations on dedicated thread pool
    let result = tokio::task::spawn_blocking(move || {
        // Validate wallet exists
        if !manager_clone.storage.wallet_exists(&name_clone) {
            return Err(WalletError::WalletNotFound(name_clone));
        }
        
        // Get RGB manager and issue asset
        let rgb_manager = manager_clone.get_rgb_manager(&name_clone)?;
        let result = rgb_manager.issue_rgb20_asset(req)?;
        
        // Sync after issuance
        manager_clone.sync_rgb_after_state_change(&name_clone)?;
        
        Ok(result)
    })
    .await
    .map_err(|e| WalletError::Internal(format!("Issue asset task panicked: {}", e)))??;
    
    log::info!("Asset issuance complete: {}", result.contract_id);
    Ok(Json(result))
}
```

#### 3.3 Accept Consignment Handler

```rust
// wallet/src/api/handlers.rs

pub async fn accept_consignment_handler(
    State(manager): State<Arc<WalletManager>>,
    Path(name): Path<String>,
    Json(req): Json<AcceptConsignmentRequest>,
) -> Result<Json<AcceptConsignmentResponse>, WalletError> {
    let manager_clone = Arc::clone(&manager);
    let name_clone = name.clone();
    let consignment_path = req.consignment_path.clone();
    
    // Run blocking RGB operations on dedicated thread pool
    let result = tokio::task::spawn_blocking(move || {
        manager_clone.accept_consignment(&name_clone, &consignment_path)
    })
    .await
    .map_err(|e| WalletError::Internal(format!("Accept consignment task panicked: {}", e)))??;
    
    Ok(Json(result))
}
```

#### 3.4 Send Bitcoin Handler

```rust
// wallet/src/api/handlers.rs

pub async fn send_bitcoin_handler(
    State(manager): State<Arc<WalletManager>>,
    Path(name): Path<String>,
    Json(req): Json<SendBitcoinRequest>,
) -> Result<Json<SendBitcoinResponse>, WalletError> {
    let manager_clone = Arc::clone(&manager);
    let name_clone = name.clone();
    let address = req.address.clone();
    let amount = req.amount_sats;
    let fee = req.fee_sats;
    
    // Run blocking operation on dedicated thread pool
    let txid = tokio::task::spawn_blocking(move || {
        manager_clone.send_bitcoin(&name_clone, &address, amount, fee)
    })
    .await
    .map_err(|e| WalletError::Internal(format!("Send bitcoin task panicked: {}", e)))??;
    
    Ok(Json(SendBitcoinResponse { txid }))
}
```

#### 3.5 Create UTXO Handler

```rust
// wallet/src/api/handlers.rs

pub async fn create_utxo_handler(
    State(manager): State<Arc<WalletManager>>,
    Path(name): Path<String>,
    Json(req): Json<CreateUtxoRequest>,
) -> Result<Json<CreateUtxoResponse>, WalletError> {
    let manager_clone = Arc::clone(&manager);
    let name_clone = name.clone();
    let amount = req.amount_sats;
    
    // Run blocking operation on dedicated thread pool
    let txid = tokio::task::spawn_blocking(move || {
        manager_clone.create_utxo(&name_clone, amount)
    })
    .await
    .map_err(|e| WalletError::Internal(format!("Create UTXO task panicked: {}", e)))??;
    
    Ok(Json(CreateUtxoResponse { txid }))
}
```

#### 3.6 When NOT to Use spawn_blocking

**Fast operations** (don't need spawn_blocking):
- `create_wallet_handler` - Fast local file writes
- `import_wallet_handler` - Fast local file writes
- `list_wallets_handler` - Fast directory read
- `delete_wallet_handler` - Fast directory removal
- `get_balance_handler` - Fast local file reads (unless we add Esplora queries)

**Rule of Thumb:**
- Operation < 10ms → Keep sync, no spawn_blocking
- Operation > 100ms or does network I/O → Use spawn_blocking

**Verification:**
```bash
cargo build --release
```

**Expected:** Builds successfully

---

### **Phase 4: Documentation** (10 minutes)

#### 4.1 Create Architecture Document

Create `docs/architecture/spawn-blocking-pattern.md`:

```markdown
# Spawn Blocking Pattern for RGB Operations

## Problem
RGB operations are blocking (sync) but Axum handlers are async.
Calling blocking code directly from async handlers blocks Tokio worker threads.

## Solution
Use `tokio::task::spawn_blocking` at the handler level to move blocking operations
to a dedicated thread pool.

## Pattern

```rust
pub async fn handler(
    State(manager): State<Arc<WalletManager>>,
    Path(name): Path<String>,
    Json(req): Json<Request>,
) -> Result<Json<Response>, WalletError> {
    // Clone what you need to move into closure
    let manager_clone = Arc::clone(&manager);
    let name_clone = name.clone();
    let data = req.data.clone();
    
    // Spawn blocking operation
    let result = tokio::task::spawn_blocking(move || {
        manager_clone.blocking_operation(&name_clone, data)
    })
    .await
    .map_err(|e| WalletError::Internal(format!("Task panicked: {}", e)))??;
    
    Ok(Json(result))
}
```

## When to Use
- RGB operations (sync, issue, transfer, accept)
- Long-running operations (>100ms)
- Network I/O (broadcast, Esplora queries)
- Any operation that might block

## When NOT to Use
- Fast operations (<10ms): create_wallet, list_wallets, delete_wallet
- Pure CPU-bound operations that complete quickly
- Already async operations (N/A in our case)

## Why This Approach

### Preserves RGB Features
- ✅ Keeps `fs` feature (file persistence)
- ✅ No RGB library feature conflicts
- ✅ Uses RGB's blocking API as designed

### Tokio Best Practices
- ✅ Worker threads never block
- ✅ Blocking operations on dedicated thread pool
- ✅ Server remains responsive during long operations

### Performance
- ✅ No performance regression for single operations
- ✅ Better throughput for concurrent operations
- ✅ Tokio's blocking pool scales dynamically (up to 512 threads)

## Error Handling

### Double Question Mark (`??`)

```rust
let result = tokio::task::spawn_blocking(|| operation())
    .await  // Returns Result<Result<T, OperationError>, JoinError>
    .map_err(|e| WalletError::Internal(...))?  // Handle JoinError (panic)
    ?;  // Handle OperationError (normal error)
```

- First `?`: Handles task panic (JoinError)
- Second `?`: Handles operation error (WalletError)

### Task Panics

If the spawned task panics:
```rust
.map_err(|e| WalletError::Internal(format!("Task panicked: {}", e)))
```

This converts JoinError to WalletError with context.
```

#### 4.2 Update Main Documentation

Add a note to `docs/rgb-wallet-complete-implementation.md`:

```markdown
## Async/Sync Architecture

### Handler Pattern: spawn_blocking

All long-running RGB operations use `tokio::task::spawn_blocking` to avoid blocking Tokio worker threads:

```rust
pub async fn rgb_operation_handler(...) -> Result<...> {
    let result = tokio::task::spawn_blocking(move || {
        manager.blocking_rgb_operation()
    }).await??;
    Ok(result)
}
```

See `docs/architecture/spawn-blocking-pattern.md` for details.
```

---

## Implementation Checklist

### **Phase 1: Prerequisites**
- [ ] Add `#[derive(Clone)]` to `Storage` in `wallet/src/wallet/shared/storage.rs`
- [ ] Add `#[derive(Clone)]` to `RgbRuntimeManager` in `wallet/src/wallet/shared/rgb_runtime.rs`
- [ ] Run `cargo check` - verify compiles

### **Phase 2: Fix Broadcast**
- [ ] Remove `broadcast_tx_hex_blocking` from `wallet/src/wallet/rgb_transfer_ops.rs`
- [ ] Add new `broadcast_tx_hex` function (simple blocking version)
- [ ] Add timeout to blocking client (30 seconds)
- [ ] Return txid from function
- [ ] Update call site in `send_transfer` function
- [ ] Run `cargo check` - verify compiles

### **Phase 3: Wrap Handlers**
- [ ] Update `send_transfer_handler` with `spawn_blocking`
- [ ] Update `issue_asset_handler` with `spawn_blocking`
- [ ] Update `accept_consignment_handler` with `spawn_blocking`
- [ ] Update `send_bitcoin_handler` with `spawn_blocking`
- [ ] Update `create_utxo_handler` with `spawn_blocking`
- [ ] Run `cargo build --release` - verify builds

### **Phase 4: Documentation**
- [ ] Create `docs/architecture/spawn-blocking-pattern.md`
- [ ] Update `docs/rgb-wallet-complete-implementation.md`
- [ ] Add inline code comments explaining pattern

### **Verification**
- [ ] Clean build: `cargo clean && cargo build --release`
- [ ] Start server: `cargo run --release`
- [ ] Manual test: Create wallet, issue asset, transfer
- [ ] Verify no runtime panics
- [ ] Verify transfer completes successfully
- [ ] Verify server remains responsive during long operations

---

## Expected Outcomes

### **Before Fix**
- ❌ Transfer fails with runtime panic
- ❌ Error: "Cannot drop a runtime in a context where blocking is not allowed"
- ❌ Server becomes unresponsive
- ❌ Other requests queue behind blocking operation

### **After Fix**
- ✅ Transfer completes successfully
- ✅ No runtime panics
- ✅ Server remains responsive during transfers
- ✅ Multiple concurrent transfers work correctly
- ✅ Clear error messages on failure
- ✅ Transaction ID returned from broadcast

---

## Performance Characteristics

### **Blocking Thread Pool**

Tokio's blocking thread pool characteristics:
- **Dynamic scaling:** Threads created on-demand
- **Max threads:** 512 (default)
- **Idle cleanup:** Threads cleaned up after 10 seconds of inactivity
- **Queue:** Unbounded (tasks queue if all threads busy)

### **Expected Behavior**

**Single Transfer:**
- Total time: ~10-15 seconds (same as before)
- Worker thread: Available immediately (non-blocking)
- Blocking thread: Used for duration of operation

**Concurrent Transfers:**
- 3 concurrent transfers: All complete in ~10-15 seconds (parallel)
- Worker threads: All available for other requests
- Blocking threads: 3 threads used from pool

**Server Responsiveness:**
- Other API requests: Instant response (not blocked)
- Frontend queries: Fast response during transfers
- No thread pool exhaustion under normal load

### **No Performance Regression**
- ✅ Same total time for single transfer
- ✅ Better throughput for concurrent transfers
- ✅ Server responsive to other requests during transfer
- ✅ No memory leaks or thread leaks

---

## Rollback Plan

If issues arise, rollback is straightforward:

### **Revert Specific Files**

```bash
# Revert handler changes
git checkout HEAD -- wallet/src/api/handlers.rs

# Revert broadcast function
git checkout HEAD -- wallet/src/wallet/rgb_transfer_ops.rs

# Revert Clone derives (if needed)
git checkout HEAD -- wallet/src/wallet/shared/storage.rs
git checkout HEAD -- wallet/src/wallet/shared/rgb_runtime.rs

# Rebuild
cd wallet
cargo build --release
```

### **Incremental Rollback**

If only some handlers have issues:
1. Comment out `spawn_blocking` wrapper
2. Call manager method directly
3. Accept temporary runtime conflict (until proper fix)

Example:
```rust
// Temporary rollback of single handler
pub async fn send_transfer_handler(...) -> Result<...> {
    // TODO: Fix spawn_blocking issue
    let result = manager.send_transfer(&name, &req.invoice, req.fee_sats)?;
    Ok(Json(result))
}
```

---

## Why This Approach

### **Pros**
- ✅ **Minimal code changes** (3 files, ~50 lines)
- ✅ **No RGB feature conflicts** (keeps `fs` feature)
- ✅ **Preserves file persistence** (critical for production)
- ✅ **Easy to understand** (clear separation at handler level)
- ✅ **Easy to rollback** (isolated changes)
- ✅ **Follows Tokio best practices** (spawn_blocking for blocking ops)
- ✅ **No performance regression** (better concurrency)
- ✅ **No new dependencies** (uses existing Tokio features)

### **Cons**
- ⚠️ Not "pure async" (but RGB isn't async-ready anyway)
- ⚠️ Still uses blocking HTTP for broadcast (but simple and works)
- ⚠️ Clone overhead for Arc (negligible, just pointer increment)

### **Trade-offs**

We choose **pragmatism** over **purity**:

1. **RGB requires `fs` feature** → Cannot use async RGB
2. **File persistence is critical** → Must keep `fs` feature
3. **spawn_blocking is best practice** → Tokio-recommended approach
4. **Handler-level boundary is clearest** → Easy to reason about

### **Comparison to Alternatives**

| Approach | Pros | Cons | Verdict |
|----------|------|------|---------|
| **Full async (Option B)** | Pure async | Loses file persistence, complex | ❌ Not viable |
| **spawn_blocking at handler** | Simple, preserves features | Not "pure" async | ✅ **Chosen** |
| **New async runtime** | Decoupled | Runtime overhead, complexity | ❌ Overkill |
| **Rewrite RGB** | Pure async everywhere | Months of work, risky | ❌ Infeasible |

---

## Future Improvements

### **When RGB Library Adds Full Async Support**

If/when RGB adds async support without `fs` feature conflict:

1. Remove `spawn_blocking` wrappers from handlers
2. Make manager methods async
3. Make RGB operation functions async
4. Use `runtime.update_async().await` instead of `runtime.update()`
5. Make `broadcast_tx_hex` async (use `reqwest::Client`)

**Estimated effort:** 2-3 hours

### **Near-Term Optimizations**

1. **Add request timeouts** to handlers (e.g., 60 seconds max)
2. **Add retry logic** to broadcast (transient network failures)
3. **Add metrics** for operation duration
4. **Add rate limiting** to prevent API abuse

### **Long-Term Architecture**

Once RGB async is stable:
- Migrate to full async architecture (see `docs/plans/async-architecture-migration.md`)
- Remove blocking thread pool dependency
- Optimize for maximum concurrency

---

## Estimated Timeline

- **Phase 1 (Prerequisites):** 5 minutes
- **Phase 2 (Fix Broadcast):** 10 minutes
- **Phase 3 (Wrap Handlers):** 15 minutes
- **Phase 4 (Documentation):** 10 minutes
- **Verification:** 10 minutes

**Total:** ~50 minutes

**With testing and debugging:** 1-2 hours

---

## Success Criteria

### **Must Have**
- ✅ No runtime panics during transfers
- ✅ Transfers complete successfully
- ✅ Server remains responsive during transfers
- ✅ Clean build with no warnings

### **Should Have**
- ✅ Clear error messages on failures
- ✅ Transaction ID returned from broadcast
- ✅ Logging at key points
- ✅ Documentation updated

### **Nice to Have**
- ✅ Concurrent transfer testing
- ✅ Performance benchmarks
- ✅ Load testing results

---

## References

- [Tokio spawn_blocking documentation](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html)
- [Axum handler guide](https://docs.rs/axum/latest/axum/)
- [RGB Runtime fs feature](https://github.com/RGB-WG/rgb)
- [Rust Async Book - Blocking](https://rust-lang.github.io/async-book/08_ecosystem/00_chapter.html)

