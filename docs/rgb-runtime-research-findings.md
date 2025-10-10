# RGB Runtime Research Findings

This document contains technical research findings from exploring the RGB codebase to understand how to integrate RGB runtime for UTXO occupation detection.

**Status**: ✅ Research Complete - Ready for Implementation  
**Date**: October 10, 2025  
**Confidence Level**: 9.5/10

---

## Table of Contents
1. [RGB Runtime Initialization](#rgb-runtime-initialization)
2. [UTXO Occupation Detection](#utxo-occupation-detection)
3. [Asset Metadata Extraction](#asset-metadata-extraction)
4. [Data Structures](#data-structures)
5. [Implementation Strategy](#implementation-strategy)
6. [Open Research Items](#open-research-items)

---

## RGB Runtime Initialization

### Key Components Discovery

From analyzing `/rgb/cli/src/args.rs` (lines 150-170):

```rust
pub fn runtime(&self, opts: &WalletOpts) -> RgbpRuntimeDir<MultiResolver> {
    let resolver = self.resolver(&opts.resolver);
    let path = self.wallet_dir(opts.wallet.as_deref());
    let hodler = FileHolder::load(path).unwrap();
    let wallet = Owner::with_components(self.network, hodler, resolver);
    let mut runtime = RgbpRuntimeDir::from(
        RgbWallet::with_components(wallet, self.contracts())
    );
    runtime
}

pub fn contracts(&self) -> Contracts<StockpileDir<TxoSeal>> {
    let stockpile = StockpileDir::load(self.data_dir(), Consensus::Bitcoin, true)
        .expect("Invalid contracts directory");
    Contracts::load(stockpile)
}
```

### Required Libraries

```toml
[dependencies]
rgb-runtime = { path = "../rgb", version = "0.12.0-rc.3" }
rgb-std = { path = "../rgb-std", version = "0.12.0-rc.3" }
rgb-persist-fs = { path = "../rgb-std/persistence/fs", version = "0.12.0-rc.3" }
bpstd = { path = "../bp-std", version = "0.12.0-rc.3" }
```

### Initialization Pattern

**For Backend Wallet (without full RGB wallet):**

We only need the **Contracts** component (not the full RGB wallet with keys):

```rust
use rgb_persist_fs::StockpileDir;
use rgb::{Contracts, Consensus};
use bpstd::seals::TxoSeal;
use bpstd::Network;

pub struct RgbManager {
    data_dir: PathBuf,
    network: Network,
}

impl RgbManager {
    pub fn new(data_dir: PathBuf, network: Network) -> Result<Self, RgbError> {
        // Ensure RGB data directory exists
        fs::create_dir_all(&data_dir)?;
        Ok(Self { data_dir, network })
    }
    
    fn load_contracts(&self) -> Result<Contracts<StockpileDir<TxoSeal>>, RgbError> {
        let stockpile = StockpileDir::load(&self.data_dir, Consensus::Bitcoin, true)?;
        Ok(Contracts::load(stockpile))
    }
}
```

**Note:** We don't need `Owner`, `FileHolder`, or `Resolver` for simple occupation checking - those are needed for creating/accepting RGB transfers.

---

## UTXO Occupation Detection

### Method Discovery

From `/rgb-std/src/popls/bp.rs` (line 478):

```rust
pub fn wallet_contract_state(&self, contract_id: ContractId) -> ContractState<Outpoint> {
    self.contracts
        .contract_state(contract_id)
        .clone()
        .filter_map(|seal| {
            if self.wallet.has_utxo(seal.primary) {
                Some(seal.primary)
            } else {
                None
            }
        })
}
```

### Data Structure for Owned State

From `/rgb-std/src/contract.rs`:

```rust
pub struct OwnedState<Seal> {
    /// Operation output defining this element of owned state
    pub addr: CellAddr,
    
    /// State assignment (contains seal and data)
    pub assignment: Assignment<Seal>,
    
    /// Confirmation status
    pub status: WitnessStatus,
}

pub struct Assignment<Seal> {
    pub seal: Seal,           // The UTXO outpoint
    pub data: StateAtom,      // The token amount/data
}

pub struct ContractState<Seal> {
    pub immutable: BTreeMap<StateName, Vec<ImmutableState>>,
    pub owned: BTreeMap<StateName, Vec<OwnedState<Seal>>>,  // ← RGB allocations!
    pub aggregated: BTreeMap<StateName, StrictVal>,
}
```

### Algorithm for Checking Occupation

```rust
use bpstd::Outpoint;

impl RgbManager {
    pub fn check_utxo_occupied(
        &self,
        txid: bitcoin::Txid,
        vout: u32,
    ) -> Result<bool, RgbError> {
        let contracts = self.load_contracts()?;
        
        // Convert bitcoin types to bpstd types
        let outpoint = Outpoint::new(
            txid,
            bpstd::Vout::from_u32(vout)
        );
        
        // Check all contracts for this outpoint
        for contract_id in contracts.contract_ids() {
            let state = contracts.contract_state(contract_id);
            
            // Check all owned state (allocations)
            for (_state_name, owned_states) in state.owned {
                for owned_state in owned_states {
                    // Compare the seal (UTXO) with our target outpoint
                    if owned_state.assignment.seal == outpoint {
                        return Ok(true);  // Found RGB allocation!
                    }
                }
            }
        }
        
        Ok(false)  // No RGB allocations found
    }
}
```

---

## Asset Metadata Extraction

### Contract Information

From `/rgb/cli/src/exec.rs` (lines 272-329):

```rust
for contract_id in contract_ids {
    let state = runtime.wallet_contract_state(contract_id);
    let articles = runtime.contracts.contract_articles(contract_id);
    
    // Access contract metadata
    let contract_name = articles.issue().meta.name;
    let issuer = articles.issue().meta.issuer;
    let timestamp = articles.issue().meta.timestamp;
    
    // Access owned state
    for (state_name, states) in &state.owned {
        for state in states {
            let seal = state.assignment.seal;      // The UTXO
            let data = state.assignment.data;      // The token amount
            let status = state.status;             // Confirmation status
        }
    }
}
```

### Available Metadata

From `/rgb/src/info.rs`:

```rust
pub struct ContractInfo {
    pub id: ContractId,
    pub name: ContractName,
    pub issuer: Identity,
    pub timestamp: DateTime<Utc>,
    pub codex: CodexInfo,
    pub consensus: Consensus,
    pub testnet: bool,
}
```

### Current Implementation for get_bound_assets

```rust
pub struct BoundAsset {
    pub asset_id: String,        // Contract ID (✅ KNOWN)
    pub asset_name: String,       // Contract name (✅ KNOWN)
    pub ticker: String,           // ⚠️ RESEARCH NEEDED
    pub amount: u64,              // ⚠️ RESEARCH NEEDED
}

impl RgbManager {
    pub fn get_bound_assets(
        &self,
        txid: bitcoin::Txid,
        vout: u32,
    ) -> Result<Vec<BoundAsset>, RgbError> {
        let contracts = self.load_contracts()?;
        let outpoint = Outpoint::new(txid, bpstd::Vout::from_u32(vout));
        let mut assets = Vec::new();
        
        for contract_id in contracts.contract_ids() {
            let state = contracts.contract_state(contract_id);
            let articles = contracts.contract_articles(contract_id);
            
            for (state_name, owned_states) in state.owned {
                for owned_state in owned_states {
                    if owned_state.assignment.seal == outpoint {
                        assets.push(BoundAsset {
                            asset_id: contract_id.to_string(),
                            asset_name: articles.issue().meta.name.to_string(),
                            ticker: "???".to_string(),      // TODO
                            amount: 0,                      // TODO
                        });
                    }
                }
            }
        }
        
        Ok(assets)
    }
}
```

---

## Data Structures

### Core Types Reference

```rust
// From rgb-std
use rgb::{ContractId, ContractName, CodexId};
use hypersonic::{StateName, StateAtom};
use strict_types::StrictVal;

// From bp-std
use bpstd::{Outpoint, Txid, Vout};
use bpstd::seals::TxoSeal;
use bpstd::Network;

// From rgb-persist-fs
use rgb_persist_fs::StockpileDir;
```

### Contract State Hierarchy

```
Contracts (collection of all contracts)
  ├─ ContractId → Contract
  │   ├─ Articles (metadata)
  │   │   └─ Issue Meta (name, issuer, timestamp)
  │   └─ ContractState
  │       ├─ immutable: BTreeMap<StateName, Vec<ImmutableState>>
  │       ├─ owned: BTreeMap<StateName, Vec<OwnedState<Seal>>>  ← Token allocations
  │       └─ aggregated: BTreeMap<StateName, StrictVal>
  │
  └─ OwnedState
      ├─ addr: CellAddr (operation address)
      ├─ assignment: Assignment<Seal>
      │   ├─ seal: Outpoint (txid:vout)
      │   └─ data: StateAtom (token amount)
      └─ status: WitnessStatus (confirmation depth)
```

---

## Implementation Strategy

### Phase 3A: Basic Occupation Detection (READY ✅)

**Confidence: 9/10**

1. Add RGB dependencies to `wallet/Cargo.toml`
2. Create `wallet/src/wallet/rgb.rs` module
3. Implement `RgbManager` with:
   - `new()` - Initialize with data directory
   - `check_utxo_occupied()` - Boolean check
4. Integrate into `balance.rs`:
   - After fetching UTXOs from Esplora
   - Call RGB manager for each UTXO
   - Set `is_occupied` field
5. Update frontend to display occupied/unoccupied tabs

### Phase 3B: Asset Metadata (PARTIAL ⚠️)

**Confidence: 7/10**

1. Extend `get_bound_assets()` to return:
   - ✅ Contract ID
   - ✅ Contract Name
   - ⚠️ Ticker (needs research)
   - ⚠️ Amount (needs research)
2. Add `bound_assets` field to UTXO struct
3. Update frontend to display asset badges

### Phase 3C: Unlock Feature (READY ✅)

**Confidence: 8/10**

No RGB-specific logic needed - just regular Bitcoin transaction that ignores RGB state.

---

## Open Research Items

### 1. Ticker Symbol Extraction ✅

**Status**: RESEARCH COMPLETE

**Discovery**: Ticker is stored in **global/immutable state** of the contract!

**Evidence from `/rgb/examples/DemoToken.yaml`**:
```yaml
global:
  - name: ticker
    verified: DEMO
  - name: name
    verified: Demo Token
  - name: precision
    verified: centiMilli
  - name: issued
    verified: 10000
```

**How to Access** (from `/rgb/cli/src/exec.rs` lines 294-305):
```rust
for (name, map) in &state.immutable {
    for state in map {
        let state_name = name.as_str();  // e.g., "ticker", "name", "precision"
        let value = state.data.verified.to_string();  // Convert StrictVal to String
        println!("{}\t{}", state_name, value);
    }
}
```

**Implementation**:
```rust
// Get ticker from global state
let state = contracts.contract_state(contract_id);

let ticker = state.immutable
    .get(&StateName::from_str("ticker").unwrap())
    .and_then(|states| states.first())
    .map(|s| s.data.verified.to_string())
    .unwrap_or_else(|| "N/A".to_string());

let name = state.immutable
    .get(&StateName::from_str("name").unwrap())
    .and_then(|states| states.first())
    .map(|s| s.data.verified.to_string())
    .unwrap_or_else(|| articles.issue().meta.name.to_string());

let precision = state.immutable
    .get(&StateName::from_str("precision").unwrap())
    .and_then(|states| states.first())
    .map(|s| s.data.verified.to_string())
    .unwrap_or_else(|| "0".to_string());
```

---

### 2. Token Amount Parsing ✅

**Status**: RESEARCH COMPLETE

**Discovery**: Amount is stored directly in `assignment.data` as `StrictVal`!

**Evidence from `/rgb/cli/src/exec.rs` line 330**:
```rust
for (name, map) in &state.owned {
    for state in map {
        print!("\t{:<16}", name.as_str());          // State name (e.g., "balance")
        print!("\t{:<12}", state.status.to_string());
        print!("\t{:<32}", state.assignment.data.to_string());  // ← Amount as string!
        print!("\t{:<46}", state.addr);
        println!("\t{}", state.assignment.seal);     // The UTXO outpoint
    }
}
```

**Key Insight**: `StrictVal` implements `Display` trait, so we can call `.to_string()` directly!

**Implementation**:
```rust
for (state_name, owned_states) in state.owned {
    for owned_state in owned_states {
        if owned_state.assignment.seal == target_outpoint {
            // Get amount as string
            let amount_str = owned_state.assignment.data.to_string();
            
            // Try to parse as u64 (for fungible tokens)
            let amount = amount_str.parse::<u64>().unwrap_or(0);
            
            // Or keep as string for display
            let amount_display = amount_str;
        }
    }
}
```

**Fallback Strategy**:
- If parsing to u64 fails, display as string
- For NFTs or complex state, show the full `StrictVal` representation
- Log warning if parse fails but continue gracefully

**Data Structure**:
```rust
pub struct BoundAsset {
    pub asset_id: String,          // Contract ID
    pub asset_name: String,         // Contract name (or from global state)
    pub ticker: String,             // From global state "ticker"
    pub amount: String,             // StrictVal.to_string() - keep as string!
}
```

**Rationale for String Amount**:
- RGB supports various data types (u64, u128, custom types)
- Precision varies per contract (see "precision" in global state)
- Safest to display as string and let UI format
- Frontend can parse if needed for calculations

---

### 3. Performance Optimization

**Concern**: Loading contracts on every UTXO check might be slow

**Solution**: Cache contracts collection in `RgbManager`:
```rust
pub struct RgbManager {
    data_dir: PathBuf,
    network: Network,
    contracts: Arc<RwLock<Option<Contracts<StockpileDir<TxoSeal>>>>>,
}
```

**Refresh strategy**: Invalidate cache on:
- Manual sync request
- New RGB transfer accepted
- Periodic interval (e.g., every 5 minutes)

---

## Next Steps

1. ✅ Complete research on ticker extraction - **DONE**
2. ✅ Complete research on amount parsing - **DONE**
3. ✅ Update this document with findings - **DONE**
4. 🚀 **READY**: Begin Phase 3A implementation
5. ⏭️ Test with real RGB assets on Signet
6. ⏭️ Iterate based on test results

---

## Implementation Summary

### ✅ Complete Understanding Achieved

**What we now know:**

1. **Ticker**: Stored in `state.immutable.get("ticker")` as `StrictVal`
2. **Amount**: Stored in `owned_state.assignment.data` as `StrictVal`
3. **Conversion**: Both use `.to_string()` for display
4. **Parsing**: Can attempt `.parse::<u64>()` for numeric amounts, fallback to string

**Updated BoundAsset Structure:**
```rust
pub struct BoundAsset {
    pub asset_id: String,        // contract_id.to_string()
    pub asset_name: String,       // From articles or global "name"
    pub ticker: String,           // From global "ticker"
    pub amount: String,           // assignment.data.to_string()
}
```

### 🎯 Ready to Proceed

All research objectives complete. Can now implement Phase 3A with high confidence.

---

## References

**Key Source Files**:
- `/rgb/cli/src/args.rs` - Runtime initialization
- `/rgb/cli/src/exec.rs` - Contract state access patterns
- `/rgb-std/src/popls/bp.rs` - Wallet state methods
- `/rgb-std/src/contract.rs` - Data structure definitions
- `/rgb/src/info.rs` - Contract metadata structures

**RGB Libraries**:
- `rgb-runtime` (0.12.0-rc.3) - Main runtime
- `rgb-std` (0.12.0-rc.3) - Standard library
- `rgb-persist-fs` (0.12.0-rc.3) - Filesystem persistence
- `bp-std` (0.12.0-rc.3) - Bitcoin protocol types

---

## Confidence Levels

| Component | Confidence | Status |
|-----------|-----------|--------|
| Runtime Initialization | 9/10 ✅ | Algorithm clear |
| UTXO Occupation Check | 9/10 ✅ | Implementation ready |
| Contract ID Extraction | 10/10 ✅ | Trivial |
| Contract Name Extraction | 9/10 ✅ | Clear from articles |
| Ticker Extraction | 9/10 ✅ | **RESEARCH COMPLETE** |
| Amount Parsing | 9/10 ✅ | **RESEARCH COMPLETE** |
| Error Handling | 8/10 ✅ | Standard patterns |
| Performance | 7/10 ⚠️ | Caching needed |

**Overall Confidence: 9.5/10** (Very high confidence, ready to implement!)

