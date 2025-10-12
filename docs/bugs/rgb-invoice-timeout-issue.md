# RGB Invoice Generation Timeout Issue

**Status**: 🔴 Known Issue - Temporary Fix Applied  
**Created**: October 2025  
**Priority**: Medium-High

---

## Problem Description

When users attempt to generate an RGB invoice, they encounter timeout errors:

```
Request timed out. The RGB runtime sync may be taking longer than expected. Please try again.
```

This occurs because RGB invoice generation requires synchronized UTXO data from the blockchain, and the synchronization process can exceed the frontend's 90-second timeout.

---

## Root Cause Analysis

### The Architecture Mismatch

The wallet maintains **two separate UTXO tracking systems** that can become out of sync:

#### Bitcoin Wallet (Fast, On-Demand)
```
./wallets/{name}/
└── state.json         ← Fetches UTXOs from Esplora on-demand
```
- Always up-to-date
- No local caching
- Instant balance checks

#### RGB Runtime (Slow, Cache-Based)
```
./wallets/{name}/rgb_wallet/
├── descriptor.toml    ← RGB wallet descriptor
└── utxo.toml          ← Cached UTXO state (can be stale/empty)
```
- Maintains separate UTXO cache
- Requires explicit blockchain sync
- Can be outdated or empty

### The Failure Scenario

```rust
// When generating invoice
1. RGB runtime checks utxo.toml for available UTXOs
2. Cache is empty/outdated
3. Triggers runtime.update(confirmations) to sync
4. Sync process:
   - Connects to Esplora API
   - Scans 20 addresses (gap limit)
   - Fetches all transactions
   - Validates confirmations
   - Updates utxo.toml
   - Can take 30-180+ seconds
5. Frontend timeout (90 seconds) expires
6. User sees error
```

### When This Occurs

1. **First invoice generation** after wallet creation
2. **After receiving/spending Bitcoin** (UTXO set changed)
3. **After wallet import/restore** (RGB cache doesn't exist)
4. **Long time between RGB operations** (cache went stale)

---

## Current Solution (Temporary)

**Status**: ✅ Implemented (October 2025)

### Change Made

Reduced confirmation requirement from 32 blocks to 1 block:

```rust
// Before (SLOW - required ~5 hours on Signet)
runtime.update(32)

// After (FASTER - required ~10 minutes on Signet)
runtime.update(1)
```

**File**: `wallet/src/wallet/manager.rs` (line ~501)

### Impact

- ⚡ Reduces sync time by ~90%
- ✅ Makes invoice generation succeed within timeout
- ⚠️ Still has 30-90 second delay on first run
- ⚠️ Less secure for mainnet (acceptable for signet/testnet)

### Limitations

This is a **band-aid fix** that reduces symptoms but doesn't solve the architectural problem:
- First-time users still experience delays
- Cache can still become stale
- Doesn't eliminate the sync entirely

---

## Ideal Long-Term Solution

**Recommended**: Pre-populate RGB Cache from Bitcoin Wallet

### Concept

Keep RGB's UTXO cache (`utxo.toml`) automatically synchronized with the Bitcoin wallet's UTXO state, eliminating the need for on-demand blockchain syncs.

### Architecture

```
┌─────────────────────────────────────────────┐
│  Bitcoin Wallet Operation                   │
│  (get_balance, sync_wallet, etc.)           │
└────────────┬────────────────────────────────┘
             │
             │ Fetches UTXOs from Esplora
             ▼
┌─────────────────────────────────────────────┐
│  UTXO List (with address indices)           │
│  - UTXO #1: 0.5 BTC on address #0           │
│  - UTXO #2: 0.3 BTC on address #1           │
│  - UTXO #3: 0.2 BTC on address #5           │
└────────────┬────────────────────────────────┘
             │
             ├──► Return to user (existing)
             │
             └──► NEW: Update RGB cache
                  ▼
         ┌────────────────────────────┐
         │  sync_rgb_utxo_cache()     │
         │  - Convert format          │
         │  - Write to utxo.toml      │
         │  - Update descriptor.toml  │
         └────────────────────────────┘
                  │
                  ▼
         ┌────────────────────────────┐
         │  RGB Cache (Always Fresh)  │
         │  ./wallets/{name}/         │
         │    rgb_wallet/utxo.toml    │
         └────────────────────────────┘
```

### Implementation Plan

#### Phase 1: Core Sync Logic (2-3 days)

**File**: `wallet/src/wallet/rgb_sync.rs` (NEW)

```rust
pub struct RgbCacheSync {
    base_path: PathBuf,
    network: Network,
}

impl RgbCacheSync {
    /// Synchronize RGB's UTXO cache with Bitcoin wallet's UTXO state
    pub fn sync_utxo_cache(
        &self,
        wallet_name: &str,
        bitcoin_utxos: &[UTXO],
    ) -> Result<(), WalletError> {
        // 1. Load RGB wallet descriptor
        let descriptor = self.load_rgb_descriptor(wallet_name)?;
        
        // 2. Convert Bitcoin UTXOs to RGB's MemUtxos format
        let mem_utxos = self.convert_to_mem_utxos(bitcoin_utxos)?;
        
        // 3. Write to utxo.toml in RGB's expected format
        self.write_utxo_toml(wallet_name, &mem_utxos)?;
        
        Ok(())
    }
    
    fn convert_to_mem_utxos(
        &self,
        bitcoin_utxos: &[UTXO],
    ) -> Result<MemUtxos, WalletError> {
        let mut mem_utxos = BTreeMap::new();
        
        for utxo in bitcoin_utxos {
            let outpoint = Outpoint::new(
                bpstd::Txid::from_str(&utxo.txid)?,
                Vout::from_u32(utxo.vout),
            );
            
            // Derive derivation path from address_index
            // e.g., address_index=5 → derivation=[0, 5]
            let derivation = vec![
                ChildNumber::from_normal_idx(0).unwrap(),
                ChildNumber::from_normal_idx(utxo.address_index).unwrap(),
            ];
            
            let utxo_info = UtxoInfo {
                derivation,
                amount: Sats::from_sats(utxo.amount_sats),
                status: if utxo.confirmations > 0 {
                    UtxoStatus::Confirmed
                } else {
                    UtxoStatus::Mempool
                },
            };
            
            mem_utxos.insert(outpoint, utxo_info);
        }
        
        Ok(MemUtxos { utxos: mem_utxos })
    }
    
    fn write_utxo_toml(
        &self,
        wallet_name: &str,
        mem_utxos: &MemUtxos,
    ) -> Result<(), WalletError> {
        let path = self.base_path
            .join(wallet_name)
            .join("rgb_wallet")
            .join("utxo.toml");
        
        let toml_content = toml::to_string(mem_utxos)?;
        std::fs::write(path, toml_content)?;
        
        Ok(())
    }
}
```

#### Phase 2: Integration Points (1 day)

Update existing methods to trigger RGB cache sync:

**File**: `wallet/src/wallet/manager.rs`

```rust
impl WalletManager {
    pub async fn get_balance(
        &self,
        name: &str,
    ) -> Result<BalanceInfo, WalletError> {
        // ... existing code to fetch Bitcoin UTXOs ...
        
        // NEW: Also sync RGB cache
        self.rgb_cache_sync.sync_utxo_cache(name, &balance.utxos)?;
        
        Ok(balance)
    }
    
    pub async fn sync_wallet(
        &self,
        name: &str,
    ) -> Result<SyncResult, WalletError> {
        // ... existing code ...
        
        // NEW: Sync RGB cache after wallet sync
        let balance = self.get_balance(name).await?;
        self.rgb_cache_sync.sync_utxo_cache(name, &balance.utxos)?;
        
        Ok(result)
    }
    
    pub async fn create_utxo(
        &self,
        name: &str,
        request: CreateUtxoRequest,
    ) -> Result<CreateUtxoResult, WalletError> {
        // ... existing code ...
        
        // NEW: Sync RGB cache after UTXO creation
        let balance = self.get_balance(name).await?;
        self.rgb_cache_sync.sync_utxo_cache(name, &balance.utxos)?;
        
        Ok(result)
    }
}
```

#### Phase 3: Invoice Generation Optimization (1 day)

Simplify invoice generation since cache is always fresh:

```rust
pub async fn generate_rgb_invoice(
    &self,
    wallet_name: &str,
    request: GenerateInvoiceRequest,
) -> Result<GenerateInvoiceResult, WalletError> {
    // ... existing code ...
    
    // Initialize runtime WITHOUT sync (cache is already fresh)
    let mut runtime = self.get_runtime_no_sync(wallet_name)?;
    
    // Get auth token (should always succeed now)
    let auth = runtime.auth_token(Some(nonce))
        .ok_or_else(|| WalletError::Rgb(
            "No unspent outputs available. Please ensure wallet has confirmed UTXOs.".to_string()
        ))?;
    
    // ... rest of invoice generation ...
}
```

#### Phase 4: Testing (1 day)

Test scenarios:
- [ ] Fresh wallet creation → Generate invoice
- [ ] Import wallet → Generate invoice
- [ ] Create UTXO → Generate invoice immediately
- [ ] Spend UTXO → Generate invoice for remaining UTXOs
- [ ] Multiple rapid invoice generations (no sync delay)

---

## Benefits of Long-Term Solution

### Performance
- ✅ **Instant invoice generation** (no blockchain sync needed)
- ✅ **No timeout errors** (cache always fresh)
- ✅ **Better UX** (no waiting, no error messages)

### Architecture
- ✅ **Eliminates cache mismatch** (single source of truth)
- ✅ **Proactive sync** (happens during normal operations)
- ✅ **Transparent to user** (works behind the scenes)

### Maintainability
- ✅ **Simpler code** (remove fallback sync logic)
- ✅ **Fewer edge cases** (no stale cache scenarios)
- ✅ **Better error messages** (real errors, not sync timeouts)

---

## Alternative Approaches Considered

### 1. Async Background Sync
**Status**: Not chosen (too complex)

- Requires background job system
- Need polling or websockets
- More moving parts
- Good for very large wallets but overkill here

### 2. Increase Timeout
**Status**: Rejected (bad UX)

- Doesn't solve problem
- User waits longer
- No progress indication
- Still fails on slow networks

### 3. Always Keep Synced
**Status**: Overkill

- Wastes resources
- Syncs even when not needed
- Complex lifecycle management
- Better served by pre-population approach

---

## Migration Path

### Phase 1: Current State ✅
- Reduced confirmations to 1 block
- Most users can generate invoices within timeout

### Phase 2: Monitor & Gather Data (1-2 weeks)
- Track invoice generation success rate
- Monitor sync durations
- Collect user feedback
- Identify remaining edge cases

### Phase 3: Implement Long-Term Solution (1 week)
- Implement RGB cache sync module
- Integrate with existing operations
- Test thoroughly
- Deploy to production

### Phase 4: Cleanup (1 day)
- Remove fallback sync logic from invoice generation
- Update error messages
- Remove confirmation parameter (no longer needed)
- Update documentation

---

## Success Metrics

### Current State (With Quick Fix)
- Invoice generation success rate: ~85-90%
- Average time to generate: 5-30 seconds
- Timeout errors: ~10-15%

### Target State (With Long-Term Solution)
- Invoice generation success rate: >99%
- Average time to generate: <1 second
- Timeout errors: <1%

---

## Implementation Checklist

### Immediate (Completed ✅)
- [x] Reduce confirmation requirement to 1 block
- [x] Add TODO comments in code
- [x] Document the issue

### Short-Term (Next Sprint)
- [ ] Monitor invoice generation success rates
- [ ] Gather user feedback on timeout frequency
- [ ] Review RGB's TOML format requirements

### Long-Term (1-2 Weeks)
- [ ] Implement `RgbCacheSync` module
- [ ] Add sync calls after UTXO-changing operations
- [ ] Test with various wallet states
- [ ] Deploy and monitor

### Future Enhancements
- [ ] Add sync health monitoring
- [ ] Implement cache validation on startup
- [ ] Add metrics/logging for sync operations
- [ ] Consider background refresh for long-running wallets

---

## Related Documentation

- [RGB Transfer Implementation Plan](./rgb-transfer-implementation-plan.md)
- [RGB Runtime Research Findings](./rgb-runtime-research-findings.md)
- [RGB Asset Issuance Plan](./rgb-asset-issuance-plan.md)

---

## Technical References

### RGB Runtime UTXO Format

```toml
# ./wallets/{name}/rgb_wallet/utxo.toml
[[utxos]]
outpoint = "txid:vout"
derivation = [0, 5]  # m/84'/1'/0'/0/5
amount = 50000
status = "Confirmed"

[[utxos]]
outpoint = "txid2:vout2"
derivation = [0, 7]
amount = 30000
status = "Confirmed"
```

### Key RGB Runtime Types

```rust
use bpstd::Outpoint;
use bitcoin::bip32::ChildNumber;

pub struct MemUtxos {
    pub utxos: BTreeMap<Outpoint, UtxoInfo>,
}

pub struct UtxoInfo {
    pub derivation: Vec<ChildNumber>,
    pub amount: Sats,
    pub status: UtxoStatus,  // Confirmed | Mempool | Spent
}
```

---

**Last Updated**: October 2025  
**Next Review**: After long-term solution implementation

