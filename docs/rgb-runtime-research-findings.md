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

## RGB20 Asset Issuance

### Reference Implementation (rgb-wallet)

From `/rgb-wallet/backend/src/wallet/rgb.rs` (lines 379-494):

**CLI Approach:**
```rust
pub fn issue_token(
    &self,
    wallet_name: &str,
    token_name: &str,
    ticker: &str,
    supply: u64,
    precision: &str,
    seal_utxo: &str,  // "txid:vout" format
) -> Result<(String, String), Box<dyn std::error::Error>> {
    // 1. Generate YAML contract file
    let yaml_content = self.generate_token_yaml(token_name, ticker, supply, precision, seal_utxo)?;
    fs::write(&contract_path, yaml_content)?;
    
    // 2. Issue via RGB CLI
    let issue_result = Command::new(&self.rgb_binary_path)
        .args([
            "--network", "signet",
            "-d", &data_path,
            "issue", "-w", wallet_name, &contract_path
        ])
        .output()?;
    
    // 3. Extract contract ID from stdout
    let contract_id = self.extract_contract_id(&issue_output)?;
    
    // 4. Create backup (for sharing/importing)
    Command::new(&self.rgb_binary_path)
        .args([
            "--network", "signet",
            "-d", &data_path,
            "backup", &contract_id, &backup_path
        ])
        .output()?;
    
    Ok((contract_id, backup_filename))
}
```

**YAML Template Structure:**
```yaml
consensus: bitcoin
testnet: true
issuer:
  codexId: 7C15w3W1-L0T~zXw-Aeh5~kV-Zquz729-HXQFKQW-_5lX9O8  # RGB20-FNA schema ID
  version: 0
  checksum: AYkSrg
name: TokenName
method: issue
timestamp: "2024-10-10T10:32:00+00:00"

global:
  - name: ticker
    verified: TICK
  - name: name
    verified: Token Name
  - name: precision
    verified: centiMilli  # or: indivisible, deci, centi, milli, micro, nano, etc.
  - name: issued
    verified: 1000000

owned:
  - name: balance
    seal: txid:vout  # Genesis UTXO
    data: 1000000    # Initial allocation
```

---

### Native Runtime Approach (Discovered)

From research in `/rgb/examples/` and `/rgb-std/src/contract.rs`:

**1. Load RGB20 Issuer (Schema)**
```rust
use rgb::Issuer;
use std::convert::Infallible;

// RGB20-FNA.issuer is the schema file (located at /rgb/examples/RGB20-FNA.issuer)
let issuer = Issuer::load(
    "path/to/RGB20-FNA.issuer",
    |_, _, _| -> Result<_, Infallible> { Ok(()) }
)?;

// The codex_id from the issuer matches the YAML: 7C15w3W1-L0T~zXw-Aeh5~kV-Zquz729-HXQFKQW-_5lX9O8
let codex_id = issuer.codex_id();
```

**2. Create Contract Parameters**
```rust
use rgb::{CreateParams, Assignment, Outpoint};
use strict_encoding::vname;
use chrono::Utc;

// Initialize params for Signet testnet
let mut params = CreateParams::new_bitcoin_testnet(
    issuer.codex_id(),
    "TokenName"  // Contract name
);

// Add global state (immutable metadata)
params = params
    .with_global_verified("ticker", "TICK")
    .with_global_verified("name", "Token Name")
    .with_global_verified("precision", "centiMilli")  // String, not enum!
    .with_global_verified("issued", 1_000_000u64);

// Add owned state (initial allocation)
let genesis_outpoint = Outpoint::from_str("txid:vout")?;
params.push_owned_unlocked(
    "balance",  // State name for RGB20 fungible tokens
    Assignment::new_internal(genesis_outpoint, 1_000_000u64)
);

// Optional: Set timestamp
params.timestamp = Some(Utc::now());
```

**3. Issue Contract**
```rust
use rgb_runtime::{Contracts, Runtime};

// Load contracts (already done in RgbManager)
let mut contracts = self.load_contracts()?;

// Issue the contract (RGB runtime handles Bitcoin TX, anchoring, stash persistence)
let contract_id = contracts.issue(params.transform(noise_engine))?;

// Contract is now issued and stored in local stash!
// The genesis UTXO is now "occupied" with the asset
```

---

### Key Differences: CLI vs Native

| Aspect | CLI Approach (rgb-wallet) | Native Approach (Our Implementation) |
|--------|---------------------------|--------------------------------------|
| **Schema Loading** | Automatic (via `import` command) | Manual (`Issuer::load()`) |
| **Contract Definition** | YAML file | Rust `CreateParams` struct |
| **Transaction Handling** | RGB CLI creates internally | RGB runtime creates internally |
| **Output Parsing** | Parse stdout text | Direct Rust types |
| **Error Handling** | Exit codes + stderr | Rust `Result` types |
| **Backup** | Separate `backup` command | Call `consignment` methods |
| **Flexibility** | Limited to YAML schema | Full programmatic control |
| **Performance** | Process spawn overhead | Direct library calls |

---

### Precision Values Reference

From RGB20 schema (observed in examples):

| String Value | Decimal Places | Example |
|--------------|----------------|---------|
| `indivisible` | 0 | Whole units only (NFTs, shares) |
| `deci` | 1 | 0.1 |
| `centi` | 2 | 0.01 (like USD cents) |
| `milli` | 3 | 0.001 |
| `deciMilli` | 4 | 0.0001 |
| `centiMilli` | 5 | 0.00001 |
| `micro` | 6 | 0.000001 |
| `deciMicro` | 7 | 0.0000001 |
| `centiMicro` | 8 | 0.00000001 (like Bitcoin sats) |
| `nano` | 9 | 0.000000001 |
| `deciNano` | 10 | 0.0000000001 |

**Note:** These are **string values**, not enums. Pass as `StrictVal` strings.

---

### Implementation Path for Our Wallet

**Approach:** Use **native RGB runtime** (not CLI) for better integration, error handling, and performance.

**Steps:**
1. ✅ Copy `/rgb/examples/RGB20-FNA.issuer` to our wallet data directory
2. ✅ Load issuer once at initialization (cache in `RgbManager`)
3. ✅ Convert form inputs to `CreateParams`
4. ✅ Call `contracts.issue()` 
5. ✅ Return contract ID to frontend
6. ✅ UTXO becomes "occupied" automatically (Phase 3 detects it)

**Confidence: 9/10** - Native approach is cleaner and better integrated than spawning CLI process.

---

## References

**Key Source Files**:
- `/rgb/cli/src/args.rs` - Runtime initialization
- `/rgb/cli/src/exec.rs` - Contract state access patterns
- `/rgb-std/src/popls/bp.rs` - Wallet state methods
- `/rgb-std/src/contract.rs` - Data structure definitions
- `/rgb/src/info.rs` - Contract metadata structures
- `/rgb/examples/DemoToken.yaml` - Working RGB20 example
- `/rgb/examples/RGB20-FNA.issuer` - RGB20 schema file
- `/rgb-wallet/backend/src/wallet/rgb.rs` - CLI-based reference implementation

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
| **RGB20 Issuance** | **9/10 ✅** | **RESEARCH COMPLETE** |
| Error Handling | 8/10 ✅ | Standard patterns |
| Performance | 7/10 ⚠️ | Caching needed |

**Overall Confidence: 9.5/10** (Very high confidence, ready to implement!)

---

## RGB Asset Transfers (Full Runtime Approach)

### Research Date: October 11, 2025

### Discovery Source
Deep-dive analysis of `/rgb/cli/src/args.rs`, `/rgb/src/runtime.rs`, `/rgb/src/owner.rs`, and actual CLI transfer commands.

---

### RGB Runtime Architecture

#### Component Hierarchy

```
RgbRuntime (RgbpRuntimeDir)
    │
    ├─> RgbWallet
    │   ├─> Owner (WalletProvider)
    │   │   ├─> FileHolder (OwnerProvider)
    │   │   │   ├─> RgbDescr (descriptor)
    │   │   │   └─> MemUtxos (UTXO set)
    │   │   └─> MultiResolver (blockchain resolver)
    │   │
    │   └─> Contracts<StockpileDir>
    │       ├─> Issuers (schema collection)
    │       └─> Contracts (asset collection)
    │
    └─> Transfer Methods
        ├─> pay_invoice()
        ├─> consume_from_file()
        └─> update() (sync)
```

---

### 1. FileHolder Component

**Location**: `/rgb/src/owner.rs` lines 596-678

**Purpose**: Persist wallet descriptor and UTXO set to disk

**File Structure**:
```
./wallets/{name}/rgb_wallet/
  ├── descriptor.toml    # RgbDescr (WPKH descriptor + noise)
  └── utxo.toml          # MemUtxos (tracked UTXOs with derivation)
```

**Key Methods**:
```rust
FileHolder::create(path: PathBuf, descriptor: RgbDescr) -> io::Result<Self>
FileHolder::load(path: PathBuf) -> io::Result<Self>
FileHolder::save(&self) -> io::Result<()>  // Auto-save on drop
```

**RgbDescr Structure**:
```rust
pub struct RgbDescr<K = XpubDerivable> {
    pub wpkh: Option<Wpkh<K>>,      // P2WPKH descriptor (our mode)
    pub tapret: Option<Tapret<K>>,  // Taproot descriptor (RGB-specific)
    pub noise: [u8; 32],            // Chain code for blinding
}
```

**Conversion from our descriptor**:
```rust
// Our format: [c0a1b2c3/84h/1h/0h]tpubDC.../<0;1>/*
// To RGB format:
let xpub = XpubDerivable::from_str(descriptor)?;
let noise = xpub.xpub().chain_code().to_byte_array();
let rgb_descr = RgbDescr::new_unfunded(Wpkh::from(xpub), noise);
```

**MemUtxos Structure**:
```rust
pub struct MemUtxos {
    utxos: BTreeMap<Outpoint, UtxoInfo>,
}

pub struct UtxoInfo {
    pub derivation: Vec<ChildNumber>,  // e.g., [0, 5] for m/0/5
    pub amount: Sats,
    pub status: UtxoStatus,  // Confirmed, Mempool, Spent
}
```

**Challenge**: We fetch UTXOs on-demand from Esplora, but RGB needs them tracked in `MemUtxos`.

**Solution**: Populate `MemUtxos` from our Esplora data before RGB operations.

---

### 2. Owner Component

**Location**: `/rgb/src/owner.rs` lines 176-586

**Purpose**: Combines descriptor + UTXOs + blockchain resolver

**Structure**:
```rust
pub struct Owner<R, O, K, U>
where
    R: Resolver,              // MultiResolver (Esplora/Electrum)
    O: OwnerProvider,         // FileHolder
    K: DeriveSet,             // XpubDerivable
    U: UtxoSet,               // MemUtxos
{
    network: Network,
    provider: O,
    resolver: R,
    _phantom: PhantomData<(K, U)>,
}
```

**Creation**:
```rust
Owner::with_components(network, hodler, resolver)
```

**What it provides**:
- Address derivation: `next_address()`
- Key derivation for signing
- UTXO tracking: `has_utxo(outpoint)`
- Descriptor access: `descriptor()`

---

### 3. MultiResolver Component

**Location**: `/rgb/cli/src/args.rs` lines 172-180

**Purpose**: Abstract blockchain data access

**Creation**:
```rust
use rgbp::resolvers::MultiResolver;

let resolver = MultiResolver::new_esplora("https://mempool.space/signet/api")?;
// Or:
let resolver = MultiResolver::new_electrum("ssl://electrum.blockstream.info:60002")?;
```

**What it provides**:
- Transaction fetching
- UTXO confirmation checking
- Block height queries

**Integration with our wallet**: We already have Esplora integration - just need to wrap it in `MultiResolver`.

---

### 4. RgbWallet Component

**Location**: `rgb-std/src/popls/bp.rs`

**Purpose**: High-level RGB operations

**Structure**:
```rust
pub struct RgbWallet<W, Sp, S, C>
where
    W: WalletProvider,        // Owner
    Sp: Stockpile,            // StockpileDir
    S: KeyedCollection<CodexId, Issuer>,
    C: KeyedCollection<ContractId, Contract>,
{
    pub wallet: W,
    pub contracts: Contracts<Sp, S, C>,
}
```

**Key Methods for Transfers**:

**1. Generate Invoice (Recipient)**:
```rust
// Get seal from existing UTXO
let auth = runtime.auth_token(nonce)
    .ok_or("No unspent outputs available")?;
let beneficiary = RgbBeneficiary::Token(auth);

// Or witness-out based seal
let wout = runtime.wout(nonce);
let beneficiary = RgbBeneficiary::WitnessOut(wout);

// Create invoice
let invoice = RgbInvoice::new(
    CallScope::ContractId(contract_id),
    Consensus::Bitcoin,
    true,  // testnet
    beneficiary,
    Some(StrictVal::num(amount)),
);

// Invoice string: "contract:bitcoin:rgb:abc123.../balance/100@wout:..."
```

**2. Send Payment (Sender)**:
```rust
// Parse invoice
let invoice = RgbInvoice::from_str(invoice_str)?;

// Create payment (PSBT + RGB state)
let (mut psbt, payment) = runtime.pay_invoice(
    &invoice,
    CoinselectStrategy::Accumulative,  // or Smallest
    TxParams::with(fee_rate_sat_vb),
    sats_giveaway,  // Bitcoin to give with RGB (for witness-out)
)?;

// payment.terminals contains RGB state transitions
// payment.bundle contains prefab data for payjoin (optional)
```

**3. Generate Consignment (Sender)**:
```rust
// After broadcasting Bitcoin TX
runtime.contracts.consign_to_file(
    &consignment_path,
    contract_id,  // From invoice.scope
    payment.terminals,  // From pay_invoice result
)?;
```

**4. Accept Consignment (Recipient)**:
```rust
runtime.consume_from_file(
    true,  // allow_unknown: import new contracts
    &consignment_path,
    |hash, identity, sig| {
        // Signature validator (can be no-op for testing)
        Result::<_, Infallible>::Ok(())
    },
)?;
```

---

### 5. RgbRuntime (RgbpRuntimeDir)

**Location**: `/rgb/src/runtime.rs` lines 71-412

**Purpose**: Convenience wrapper around `RgbWallet`

**Type Alias**:
```rust
pub type RgbpRuntimeDir<R> = RgbRuntime<
    Owner<R, FileHolder>,
    StockpileDir<TxoSeal>
>;
```

**Initialization (from CLI)**:
```rust
// 1. Create resolver
let resolver = MultiResolver::new_esplora(url)?;

// 2. Load FileHolder
let hodler = FileHolder::load(wallet_path)?;

// 3. Create Owner
let owner = Owner::with_components(network, hodler, resolver);

// 4. Load Contracts
let stockpile = StockpileDir::load(data_dir, Consensus::Bitcoin, true)?;
let contracts = Contracts::load(stockpile);

// 5. Create RgbWallet
let rgb_wallet = RgbWallet::with_components(owner, contracts);

// 6. Wrap in RgbRuntime
let mut runtime = RgbpRuntimeDir::from(rgb_wallet);

// 7. Sync with blockchain
runtime.update(min_confirmations)?;
```

**Additional Methods**:
- `update(min_confirmations)` - Sync wallet with blockchain
- `compose_psbt()` - Low-level PSBT construction
- `color_psbt()` - Add RGB commitments to PSBT

---

### Complete Transfer Flow (Native APIs)

#### Recipient: Generate Invoice

```rust
// 1. Initialize runtime
let mut runtime = init_rgb_runtime(wallet_name)?;

// 2. Get seal (from existing UTXO)
let auth = runtime.auth_token(0)
    .ok_or("No UTXOs available")?;

// 3. Create invoice
let invoice = RgbInvoice::new(
    CallScope::ContractId(contract_id),
    Consensus::Bitcoin,
    true,
    RgbBeneficiary::Token(auth),
    Some(StrictVal::num(amount)),
);

// 4. Return invoice string
Ok(invoice.to_string())
```

#### Sender: Send Payment

```rust
// 1. Parse invoice
let invoice = RgbInvoice::from_str(invoice_str)?;

// 2. Initialize runtime
let mut runtime = init_rgb_runtime(wallet_name)?;

// 3. Create payment
let params = TxParams::with(fee_rate);
let (mut psbt, payment) = runtime.pay_invoice(
    &invoice,
    CoinselectStrategy::Accumulative,
    params,
    Some(Sats::from_sats(1000)),  // Min locked amount
)?;

// 4. Sign PSBT (using our existing signing logic)
let signed_psbt = sign_psbt_with_keys(&mut psbt, &xprv, &descriptor)?;

// 5. Finalize PSBT
signed_psbt.finalize(runtime.wallet.descriptor())?;

// 6. Extract and broadcast Bitcoin TX
let tx = signed_psbt.extract()?;
broadcast_tx(&tx)?;

// 7. Generate consignment
let consignment_path = format!("consignment_{}_{}.consignment", 
    invoice.scope, tx.txid());
runtime.contracts.consign_to_file(
    &consignment_path,
    invoice.scope,
    payment.terminals,
)?;

Ok(SendTransferResponse {
    bitcoin_txid: tx.txid().to_string(),
    consignment_path,
})
```

#### Recipient: Accept Consignment

```rust
// 1. Initialize runtime
let mut runtime = init_rgb_runtime(wallet_name)?;

// 2. Validate and import consignment
runtime.consume_from_file(
    true,  // allow_unknown contracts
    &consignment_file_path,
    |_, _, _| Result::<_, Infallible>::Ok(()),
)?;

// 3. Parse consignment for metadata
let consignment = Consignment::load(&consignment_file_path)?;
let contract_id = consignment.contract_id();
let bitcoin_txid = consignment.anchoring_txid();

// 4. Check Bitcoin TX status
let tx_status = check_tx_status(&bitcoin_txid)?;

Ok(AcceptConsignmentResponse {
    contract_id: contract_id.to_string(),
    bitcoin_txid: bitcoin_txid.to_string(),
    status: if tx_status.confirmed { "confirmed" } else { "pending" },
})
```

---

### Mapping to Our Existing Wallet

| RGB Component | Our Equivalent | Integration Strategy |
|---------------|----------------|----------------------|
| **Descriptor (RgbDescr)** | `descriptor.txt` (string) | Convert string → `RgbDescr` |
| **UTXO Tracking (MemUtxos)** | Esplora API (on-demand) | Populate `MemUtxos` from Esplora before ops |
| **Mnemonic/Keys** | `mnemonic.txt` + BIP32 | Load for PSBT signing (already have) |
| **RGB Data (StockpileDir)** | `./wallets/rgb_data/` | Already correct structure ✅ |
| **Network** | `bpstd::Network::Signet` | Already compatible ✅ |
| **Blockchain API** | Direct HTTP (Esplora) | Wrap in `MultiResolver` |
| **Wallet Path** | `./wallets/{name}/` | Add `rgb_wallet/` subdirectory |

---

### Key Implementation Challenges

#### Challenge 1: Descriptor Conversion

**Problem**: We store descriptor as plain string, RGB needs `RgbDescr` struct.

**Solution**:
```rust
fn descriptor_string_to_rgb(descriptor: &str) -> Result<RgbDescr, Error> {
    let xpub = XpubDerivable::from_str(descriptor)?;
    let noise = xpub.xpub().chain_code().to_byte_array();
    Ok(RgbDescr::new_unfunded(Wpkh::from(xpub), noise))
}
```

**Difficulty**: Low (straightforward API)

---

#### Challenge 2: MemUtxos Population

**Problem**: RGB needs in-memory UTXO tracking with derivation paths.

**Solution**:
```rust
async fn populate_mem_utxos(
    descriptor: &str,
    wallet_name: &str,
) -> Result<MemUtxos, Error> {
    let mut utxos = BTreeMap::new();
    
    // Fetch from Esplora
    let addresses = derive_addresses(descriptor, 0..20)?;
    
    for (idx, address) in addresses.iter().enumerate() {
        let esplora_utxos = fetch_utxos_from_esplora(address).await?;
        
        for utxo in esplora_utxos {
            let outpoint = Outpoint::new(utxo.txid, Vout::from_u32(utxo.vout));
            let info = UtxoInfo {
                derivation: vec![
                    ChildNumber::from_normal_idx(0).unwrap(),
                    ChildNumber::from_normal_idx(idx as u32).unwrap(),
                ],
                amount: Sats::from_sats(utxo.value),
                status: if utxo.status.confirmed {
                    UtxoStatus::Confirmed
                } else {
                    UtxoStatus::Mempool
                },
            };
            utxos.insert(outpoint, info);
        }
    }
    
    Ok(MemUtxos { utxos })
}
```

**Difficulty**: Medium (requires tracking derivation paths)

---

#### Challenge 3: PSBT Signing Integration

**Problem**: `pay_invoice()` returns unsigned PSBT, we need to sign with our keys.

**Solution**: We already have PSBT signing! Just reuse it.

```rust
// From existing wallet/src/wallet/transaction.rs
fn sign_psbt(
    &self,
    mut psbt: Psbt,
    wallet_name: &str,
) -> Result<Psbt, Error> {
    let mnemonic = self.storage.load_mnemonic(wallet_name)?;
    let xprv = derive_xprv_from_mnemonic(&mnemonic)?;
    
    // Sign each input (existing logic)
    for (idx, input) in psbt.inputs.iter().enumerate() {
        let signature = create_signature(&xprv, &psbt, idx)?;
        psbt.inputs[idx].partial_sigs.insert(pubkey, signature);
    }
    
    Ok(psbt)
}
```

**Difficulty**: Low (reuse existing code)

---

#### Challenge 4: FileHolder Persistence

**Problem**: RGB expects `descriptor.toml` and `utxo.toml` files.

**Solution**: Create/update these files during RGB operations.

```rust
fn ensure_rgb_wallet_files(wallet_name: &str) -> Result<(), Error> {
    let rgb_wallet_path = format!("./wallets/{}/rgb_wallet/", wallet_name);
    fs::create_dir_all(&rgb_wallet_path)?;
    
    // Load our descriptor
    let descriptor_str = load_descriptor(wallet_name)?;
    let rgb_descr = descriptor_string_to_rgb(&descriptor_str)?;
    
    // Populate UTXOs
    let mem_utxos = populate_mem_utxos(&descriptor_str, wallet_name).await?;
    
    // Create FileHolder (auto-saves to TOML)
    FileHolder::create(PathBuf::from(rgb_wallet_path), rgb_descr)?;
    
    Ok(())
}
```

**Difficulty**: Medium (file format conversion)

---

### Confidence Levels for Transfers

| Component | Confidence | Notes |
|-----------|-----------|-------|
| **Invoice Generation** | 8/10 ✅ | API clear, need UTXO tracking |
| **Payment Creation** | 7/10 ⚠️ | `pay_invoice()` API clear, but many params |
| **PSBT Signing** | 9/10 ✅ | Already have signing logic |
| **Consignment Generation** | 9/10 ✅ | Simple `consign_to_file()` call |
| **Consignment Validation** | 9/10 ✅ | Simple `consume_from_file()` call |
| **FileHolder Integration** | 7/10 ⚠️ | Need to manage TOML persistence |
| **MemUtxos Population** | 7/10 ⚠️ | Derivation path tracking needed |
| **MultiResolver Wrapper** | 9/10 ✅ | Just wrap our Esplora |
| **Overall Transfer Flow** | 8/10 ✅ | High confidence, some integration work |

---

### Open Questions

1. **AuthToken vs WitnessOut seals**: Which to use by default?
   - **Answer**: Start with `AuthToken` (simpler, uses existing UTXOs)

2. **Coinselect strategy**: Which strategy for RGB payments?
   - **Answer**: Start with `Accumulative` (CLI default)

3. **Sats giveaway**: How much Bitcoin to give with RGB?
   - **Answer**: ~1000 sats (dust limit + buffer)

4. **PSBT finalization**: Who finalizes the PSBT?
   - **Answer**: We do, after signing (before extraction)

5. **Consignment size**: How big are `.consignment` files?
   - **Answer**: Varies (KB to MB), depends on contract history

---

### Next Steps

1. ✅ Complete transfer flow research - **DONE**
2. ✅ Document RGB Runtime architecture - **DONE**
3. ✅ Map to existing wallet components - **DONE**
4. 🚀 Create detailed implementation plan
5. ⏭️ Begin implementation in phases

---

### Updated Confidence Levels

| Component | Confidence | Status |
|-----------|-----------|--------|
| Runtime Initialization | 9/10 ✅ | Algorithm clear |
| UTXO Occupation Check | 9/10 ✅ | Implementation ready |
| Contract ID Extraction | 10/10 ✅ | Trivial |
| Contract Name Extraction | 9/10 ✅ | Clear from articles |
| Ticker Extraction | 9/10 ✅ | **RESEARCH COMPLETE** |
| Amount Parsing | 9/10 ✅ | **RESEARCH COMPLETE** |
| RGB20 Issuance | 9/10 ✅ | **RESEARCH COMPLETE** |
| **Invoice Generation** | **8/10 ✅** | **RESEARCH COMPLETE** |
| **Send Payment** | **8/10 ✅** | **RESEARCH COMPLETE** |
| **Accept Consignment** | **9/10 ✅** | **RESEARCH COMPLETE** |
| Error Handling | 8/10 ✅ | Standard patterns |
| Performance | 7/10 ⚠️ | Caching needed |

**Overall Confidence: 8.5/10** (High confidence for full transfer implementation!)

---

## Phase 3 Send Transfer - Deep Research Analysis

### Research Date: October 12, 2025

### Critical Discoveries from RGB Source Code Analysis

---

#### Discovery 1: `pay_invoice()` Already Includes DBC Commit! ✅

**Source**: `/rgb/src/runtime.rs` lines 150-214

**Finding**: The PSBT returned from `pay_invoice()` is **already DBC-committed**. No need to call `runtime.complete()` separately!

**Evidence**:
```rust
// pay_invoice internally calls transfer()
pub fn pay_invoice(...) -> Result<(Psbt, Payment), ...> {
    let request = self.fulfill(invoice, strategy, giveaway)?;
    let script = OpRequestSet::with(request.clone());
    let (psbt, mut payment) = self.transfer(script, params)?;  // ← Calls transfer
    payment.terminals.insert(terminal);
    Ok((psbt, payment))
}

// transfer() internally calls complete()
pub fn transfer(...) -> Result<(Psbt, Payment), ...> {
    let payment = self.exec(script, params)?;
    let psbt = self.complete(payment.uncomit_psbt.clone(), &payment.bundle)?;  // ← DBC COMMIT HERE!
    Ok((psbt, payment))
}

// complete() does the DBC commitment
pub fn complete(&mut self, mut psbt: Psbt, bundle: &PrefabBundle) -> Result<Psbt, TransferError> {
    let (mpc, dbc) = psbt.dbc_commit()?;  // ← Deterministic Bitcoin Commitment
    let tx = psbt.to_unsigned_tx();
    let prevouts = psbt.inputs().map(|inp| inp.previous_outpoint).collect();
    self.include(bundle, &tx.into(), mpc, dbc, &prevouts)?;
    Ok(psbt)
}
```

**Implication**: The workflow is simpler than initially thought. The PSBT is ready for signing immediately after `pay_invoice()`.

**Payment Struct Contents**:
```rust
pub struct Payment {
    pub uncomit_psbt: Psbt,           // Pre-commit version (for RBF)
    pub psbt_meta: PsbtMeta,          // Change output info
    pub bundle: PrefabBundle,         // RGB operations
    pub terminals: BTreeSet<AuthToken>, // For consignment
}
```

---

#### Discovery 2: Correct Workflow Order - Consignment BEFORE Signing! ✅

**Source**: `/rgb/cli/src/exec.rs` lines 397-440

**Finding**: RGB CLI generates consignment **BEFORE** signing the PSBT, not after!

**Evidence from RGB CLI**:
```rust
// 1. Generate payment
let (mut psbt, payment) = runtime.pay_invoice(invoice, strategy, params, sats)?;

// 2. Save PSBT to file (for external signing)
psbt.encode(ver, &mut psbt_file)?;

// 3. CREATE CONSIGNMENT IMMEDIATELY (before signing!)
runtime.contracts.consign_to_file(
    consignment_path,
    invoice.scope,      // contract_id
    payment.terminals   // from Payment struct
)?;

// 4. Sign PSBT externally (user does this with hardware wallet, etc.)
// 5. Finalize and extract (separate command)
// 6. Broadcast
```

**Rationale**: 
- Consignment contains RGB state history and proofs
- Bitcoin transaction details are finalized at PSBT creation
- Signatures don't affect RGB state validity
- Recipient can validate consignment while sender signs offline

**Correct Flow**:
```
pay_invoice() → save PSBT → generate consignment → sign PSBT → finalize → extract → broadcast
```

**Previous Plan Was Wrong**:
```
pay_invoice() → sign PSBT → broadcast → generate consignment  // ❌ INCORRECT ORDER
```

---

#### Discovery 3: PSBT Signing with `bpstd` Signer Trait ✅

**Source**: `/bp-std/psbt/src/sign.rs` lines 70-113

**Finding**: `bpstd::Psbt` has a clean signing API using the `Signer` trait.

**Signing API**:
```rust
impl Psbt {
    pub fn sign(&mut self, signer: &impl Signer) -> Result<usize, SignError> {
        let satisfier = signer.approve(self)?;
        let tx = self.to_unsigned_tx();
        let prevouts = self.inputs.iter().map(Input::prev_txout).cloned().collect();
        let mut sig_hasher = SighashCache::new(Tx::from(tx), prevouts)?;
        
        for input in &mut self.inputs {
            sig_count += input.sign(&satisfier, &mut sig_hasher)?;
        }
        Ok(sig_count)
    }
}
```

**Signer Trait Requirements**:
```rust
pub trait Signer {
    type Sign<'s>: Sign where Self: 's;
    fn approve(&self, psbt: &Psbt) -> Result<Self::Sign<'_>, Rejected>;
}

pub trait Sign {
    fn sign_ecdsa(
        &self,
        sighash: Sighash,
        pk: LegacyPk,
        origin: Option<&KeyOrigin>,
    ) -> Option<ecdsa::Signature>;
    
    fn sign_bip340(
        &self,
        sighash: TapSighash,
        pk: XOnlyPk,
        leaf_hash: Option<TapLeafHash>,
    ) -> Option<bip340::Signature>;
}
```

**Implementation Strategy**:
```rust
struct WalletSigner {
    mnemonic: bip39::Mnemonic,
    descriptor: String,
}

impl Signer for WalletSigner {
    type Sign<'s> = Self where Self: 's;
    
    fn approve(&self, _psbt: &Psbt) -> Result<Self::Sign<'_>, Rejected> {
        // No user interaction needed in our backend
        Ok(self.clone())
    }
}

impl Sign for WalletSigner {
    fn sign_ecdsa(
        &self,
        sighash: Sighash,
        pk: LegacyPk,
        origin: Option<&KeyOrigin>,
    ) -> Option<ecdsa::Signature> {
        // Derive private key from origin path
        let private_key = self.derive_key_from_origin(origin?)?;
        
        // Sign sighash
        let secp = Secp256k1::new();
        let message = Message::from_digest(sighash.to_byte_array());
        Some(secp.sign_ecdsa(&message, &private_key.inner))
    }
    
    fn sign_bip340(...) -> Option<bip340::Signature> {
        // Not needed for P2WPKH (our descriptor type)
        None
    }
}
```

**Key Insight**: We can reuse our existing key derivation logic (`derive_private_key_for_index`) but adapt it to work with `KeyOrigin` from PSBT.

---

#### Discovery 4: Type Conversions and Broadcasting ✅

**Source**: `/bp-esplora-client/src/blocking.rs` lines 270-293

**Finding**: Broadcasting `bpstd::Tx` is trivial - just format as hex!

**Broadcasting**:
```rust
// bpstd::Tx implements Display with :x formatting
pub fn broadcast(&self, transaction: &Tx) -> Result<(), Error> {
    let mut request = minreq::post(format!("{}/tx", self.url))
        .with_body(format!("{transaction:x}").as_bytes().to_vec());
    // ... send request
}
```

**Our Implementation**:
```rust
fn broadcast_tx(&self, tx: &bpstd::Tx) -> Result<String, WalletError> {
    let tx_hex = format!("{:x}", tx);
    
    // Can use our existing HTTP client
    let client = reqwest::Client::new();
    let response = client
        .post("https://mempool.space/signet/api/tx")
        .body(tx_hex)
        .send()
        .await?;
    
    let txid = response.text().await?;
    Ok(txid)
}
```

**No Conversion Needed**: Everything uses `bpstd` types natively! No need to convert between `bpstd::Tx` and `bitcoin::Transaction`.

---

#### Discovery 5: Corrected Send Transfer Implementation ✅

**Complete Workflow**:

```rust
pub fn send_transfer(
    &self,
    wallet_name: &str,
    request: SendTransferRequest,
) -> Result<SendTransferResponse, WalletError> {
    // 1. Parse invoice
    let invoice = RgbInvoice::from_str(&request.invoice)
        .map_err(|e| WalletError::InvalidInput(format!("Invalid invoice: {:?}", e)))?;
    
    // 2. Initialize runtime
    let mut runtime = self.get_runtime(wallet_name)?;
    
    // 3. Create payment (ALREADY includes complete/DBC commit!)
    let params = TxParams::with(request.fee_rate_sat_vb.unwrap_or(2));
    let (mut psbt, payment) = runtime.pay_invoice(
        &invoice,
        CoinselectStrategy::Accumulative,
        params,
        Some(Sats::from_sats(1000)),  // Min locked amount
    ).map_err(|e| WalletError::Rgb(format!("Payment failed: {:?}", e)))?;
    
    // 4. **CREATE CONSIGNMENT FIRST** (before signing!)
    let consignment_filename = format!("transfer_{}_{}.rgb", 
        invoice.scope, Utc::now().timestamp());
    let consignment_path = self.storage.base_dir()
        .join("consignments")
        .join(&consignment_filename);
    
    std::fs::create_dir_all(consignment_path.parent().unwrap())?;
    
    runtime.contracts.consign_to_file(
        &consignment_path,
        invoice.scope,           // contract_id (from invoice)
        payment.terminals        // from Payment struct
    ).map_err(|e| WalletError::Rgb(format!("Consignment failed: {:?}", e)))?;
    
    // 5. Sign PSBT (using custom signer)
    let signer = self.create_signer(wallet_name)?;
    psbt.sign(&signer)
        .map_err(|e| WalletError::Bitcoin(format!("Signing failed: {:?}", e)))?;
    
    // 6. Finalize PSBT
    psbt.finalize(runtime.wallet.descriptor());
    
    // 7. Extract signed transaction
    let tx = psbt.extract()
        .map_err(|e| WalletError::Rgb(format!("Extraction failed: {:?}", e)))?;
    let txid = tx.txid();
    
    // 8. Broadcast transaction
    let tx_hex = format!("{:x}", tx);
    self.broadcast_tx_hex(&tx_hex)?;
    
    Ok(SendTransferResponse {
        bitcoin_txid: txid.to_string(),
        consignment_download_url: format!("/api/consignment/{}", consignment_filename),
        consignment_filename,
        status: "broadcasted".to_string(),
    })
}
```

---

### Updated Risk Assessment

| Risk | Before Research | After Deep Research | Resolution |
|------|----------------|---------------------|------------|
| Missing `complete` step | HIGH ❌ | ✅ **RESOLVED** | Already handled by `pay_invoice` |
| Wrong workflow order | HIGH ❌ | ✅ **RESOLVED** | Consignment before signing confirmed |
| PSBT signing complexity | HIGH ❌ | MEDIUM ⚠️ | Use `Signer` trait, implement custom signer |
| Type conversions | MEDIUM ⚠️ | ✅ **RESOLVED** | `format!("{:x}", tx)` for hex |
| Consignment API usage | LOW ✅ | ✅ **CONFIRMED** | `consign_to_file(path, contract_id, terminals)` |

---

### Updated Confidence Levels

| Component | Before | After | Notes |
|-----------|--------|-------|-------|
| Workflow Understanding | 6/10 | **9.5/10** ✅ | Complete analysis of RGB CLI source |
| PSBT Signing | 5/10 | **8/10** ⚠️ | Clear API, need custom implementation |
| Type Conversions | 6/10 | **10/10** ✅ | No conversions needed |
| Consignment Generation | 8/10 | **9.5/10** ✅ | Confirmed with RGB CLI code |
| Broadcasting | 7/10 | **10/10** ✅ | Trivial hex formatting |
| **Overall Phase 3** | **6.5/10** | **8.5/10** ✅ | **High confidence, ready for implementation** |

---

### Implementation Complexity Reduction

**Original Estimate**: 3-4 days (High complexity)

**Updated Estimate**: 2-3 days (Medium complexity)

**Reasons for Reduction**:
1. ✅ No need to implement `complete` step (already done)
2. ✅ Simpler workflow (no post-broadcast consignment generation)
3. ✅ No type conversions needed
4. ✅ Clear signing API with `Signer` trait
5. ⚠️ Only challenge: Custom `Signer` implementation (1 day)

---

### Remaining Implementation Tasks

#### Task 1: Custom Signer Implementation (Medium - 1 day)
- Create `WalletSigner` struct
- Implement `Signer` trait
- Implement `Sign` trait with ECDSA signing
- Adapt `derive_private_key_for_index` to work with `KeyOrigin`

#### Task 2: Send Transfer Method (Easy - 0.5 day)
- Implement corrected workflow
- Generate consignment before signing
- Use custom signer for PSBT
- Finalize and extract transaction
- Broadcast with hex formatting

#### Task 3: API Endpoints & Handlers (Easy - 0.5 day)
- `POST /api/wallet/:name/send-transfer`
- `GET /api/consignment/:filename`
- Request/Response types

#### Task 4: Frontend UI (Medium - 1 day)
- `SendTransferModal.tsx`
- Invoice paste input
- Consignment download link
- Error handling

#### Task 5: Testing & Polish (Easy - 0.5 day)
- End-to-end test flow
- Error handling
- Edge cases

**Total**: 2-3 days

---

### Open Questions (Minimal)

1. **KeyOrigin to derivation index mapping**: How to extract the final index from PSBT's `KeyOrigin`?
   - **Status**: Need to examine `KeyOrigin` structure
   - **Difficulty**: Low (likely documented in `bp-std`)

2. **Multiple inputs signing**: Does the signer need to handle all inputs automatically?
   - **Status**: Yes, `psbt.sign()` iterates all inputs
   - **Difficulty**: None (handled by framework)

---

### Next Steps

1. ✅ Complete Phase 3 deep research - **DONE**
2. ✅ Update documentation with findings - **IN PROGRESS**
3. ⏭️ **Await user confirmation to proceed**
4. ⏭️ Implement custom `WalletSigner`
5. ⏭️ Implement send transfer method
6. ⏭️ Test end-to-end flow

---

### References for Phase 3 Implementation

**Key Source Files**:
- `/rgb/src/runtime.rs` - `pay_invoice()`, `transfer()`, `complete()` methods
- `/rgb/cli/src/exec.rs` - Complete send transfer workflow (lines 397-440)
- `/bp-std/psbt/src/sign.rs` - `Signer` and `Sign` trait definitions
- `/bp-std/src/signers.rs` - `TestnetSigner` reference implementation
- `/bp-esplora-client/src/blocking.rs` - Broadcasting with hex format

---

**Phase 3 Status**: 📋 Research Complete - **Awaiting User Confirmation to Proceed** (Confidence: 8.5/10)

---

## Phase 4B Accept Consignment - Deep Research Analysis

### Research Date: October 12, 2025

### Problem Statement

Initial implementation of `accept_consignment` had non-production-ready aspects:
1. Could not detect genesis vs transfer consignments
2. Could not extract Bitcoin transaction ID from transfer consignments
3. Could not determine transaction status (pending vs confirmed)

The challenge was that `Consignment` struct has private fields and is consumed during `consume_from_file()`, making direct parsing impossible.

---

### Discovery 1: Consignment Structure is Opaque ✅

**Source**: `/rgb-std/src/consignment.rs` lines 40-107

**Finding**: The `Consignment<Seal>` struct is intentionally opaque:

```rust
pub struct Consignment<Seal: RgbSeal> {
    header: ConsignmentHeader<Seal>,           // PRIVATE
    operation_seals: LargeVec<OperationSeals<Seal>>,  // PRIVATE
}

impl<Seal: RgbSeal> Consignment<Seal> {
    pub fn articles(...) -> Result<Articles, SemanticError>  // Only metadata
    pub(crate) fn into_operations(self) -> InMemOps<Seal>   // Consumes self!
}
```

**Key Insight**: After `consume_from_file()` is called, the consignment is consumed (moved) and stored internally. We cannot access it anymore.

**Implication**: We must query the **imported contract state** instead of parsing the consignment file directly.

---

### Discovery 2: Post-Import Contract Querying ✅

**Source**: `/rgb-std/src/contract.rs` lines 437-452

**Finding**: After import, we can query contract state through the `Contract` API:

```rust
pub fn witness_ids(&self) -> impl Iterator<Item = <P::Seal as RgbSeal>::WitnessId>
pub fn witnesses(&self) -> impl Iterator<Item = Witness<P::Seal>>
pub fn operations(&self) -> impl Iterator<Item = (Opid, Operation, OpRels<P::Seal>)>
pub fn trace(&self) -> impl Iterator<Item = (Opid, Transition)>
```

**Key Structure - Witness**:
```rust
pub struct Witness<Seal: RgbSeal> {
    pub id: Seal::WitnessId,           // For TxoSeal, this IS Txid!
    pub published: Seal::Published,     // Block height/anchor data
    pub client: Seal::Client,           // DBC commitment data
    pub status: WitnessStatus,          // Mining status
    pub opids: HashSet<Opid>,          // Operation IDs
}
```

**Critical Discovery**: For Bitcoin operations (`TxoSeal`), the `WitnessId` type **IS** `bitcoin::Txid`!

---

### Discovery 3: Genesis vs Transfer Detection ✅

**Method**: Check the number of witnesses after import

**Logic**:
- **Genesis Consignment**: Contains only the genesis operation (no Bitcoin TX witness)
  - `witness_ids().count() == 0`
- **Transfer Consignment**: Contains state transitions with Bitcoin TX witnesses
  - `witness_ids().count() >= 1`

**Implementation**:
```rust
let witness_count = runtime.contracts
    .with_contract(contract_id, |contract| {
        contract.witness_ids().count()
    });

let import_type = if witness_count == 0 {
    "genesis"
} else {
    "transfer"
};
```

**Confidence**: 9/10 ✅ (Straightforward witness count check)

---

### Discovery 4: Bitcoin TX ID Extraction ✅

**Source**: Type analysis of `TxoSeal` and `WitnessId`

**Finding**: For Bitcoin-based RGB (`TxoSeal`), the witness ID **directly maps** to `Txid`:

```rust
// From rgb-std/src/stl.rs and rgb-std/src/popls/bp.rs
use bp::seals::TxoSeal;

// For TxoSeal:
type WitnessId = bitcoin::Txid;  // Direct type alias!
```

**Implementation**:
```rust
let witnesses: Vec<_> = runtime.contracts
    .with_contract(contract_id, |contract| {
        contract.witnesses().collect()
    });

// Get the last (most recent) witness
if let Some(last_witness) = witnesses.last() {
    let txid: Txid = last_witness.id;  // Direct access!
    let bitcoin_txid = Some(txid.to_string());
}
```

**Confidence**: 10/10 ✅ (Direct type mapping, no conversion needed)

---

### Discovery 5: Transaction Status Detection ✅

**Source**: `/rgb-std/src/pile.rs` lines 41-139

**Finding**: `WitnessStatus` enum provides confirmation status:

```rust
pub enum WitnessStatus {
    Genesis,                    // Contract genesis (no TX)
    Tentative,                  // Unconfirmed (in mempool)
    Mined(NonZeroU64),          // Confirmed at block height
    Archived,                   // Orphaned/replaced
}
```

**Status Mapping**:
```rust
let status = match witness.status {
    WitnessStatus::Genesis => "genesis_imported",
    WitnessStatus::Tentative => "pending",
    WitnessStatus::Mined(_) => "confirmed",
    WitnessStatus::Archived => "archived",
};
```

**Confidence**: 9/10 ✅ (Clear enum mapping)

---

### Complete Production-Ready Implementation ✅

**Algorithm**:

```rust
pub fn accept_consignment(
    &self,
    wallet_name: &str,
    consignment_bytes: Vec<u8>,
) -> Result<AcceptConsignmentResponse, WalletError> {
    // 1. Save consignment to temp file
    let temp_path = save_to_temp_file(&consignment_bytes)?;
    
    // 2. Initialize runtime
    let mut runtime = self.get_runtime(wallet_name)?;
    
    // 3. Get contract IDs BEFORE import
    let contract_ids_before: HashSet<String> = runtime.contracts
        .contract_ids()
        .map(|id| id.to_string())
        .collect();
    
    // 4. Import consignment (validates and stores)
    runtime.consume_from_file(
        true,  // allow_unknown contracts
        &temp_path,
        |_, _, _| Result::<_, Infallible>::Ok(()),
    )?;
    
    // 5. Find newly imported contract
    let contract_ids_after: HashSet<String> = runtime.contracts
        .contract_ids()
        .map(|id| id.to_string())
        .collect();
    
    let new_contract_id = contract_ids_after
        .difference(&contract_ids_before)
        .next()
        .ok_or("No new contract imported")?
        .clone();
    
    // 6. Query imported contract state
    let (import_type, bitcoin_txid, status) = runtime.contracts
        .with_contract(contract_id, |contract| {
            let witnesses: Vec<_> = contract.witnesses().collect();
            
            if witnesses.is_empty() {
                // Genesis: no witnesses
                ("genesis", None, "genesis_imported")
            } else {
                // Transfer: extract last witness
                let last_witness = witnesses.last().unwrap();
                let txid = last_witness.id.to_string();
                let status = match last_witness.status {
                    WitnessStatus::Tentative => "pending",
                    WitnessStatus::Mined(_) => "confirmed",
                    _ => "imported",
                };
                ("transfer", Some(txid), status)
            }
        });
    
    // 7. Cleanup temp file
    let _ = std::fs::remove_file(&temp_path);
    
    Ok(AcceptConsignmentResponse {
        contract_id: new_contract_id,
        status: status.to_string(),
        import_type: import_type.to_string(),
        bitcoin_txid,
    })
}
```

---

### Updated Risk Assessment

| Risk | Before Research | After Research | Resolution |
|------|----------------|----------------|------------|
| Cannot parse consignment | HIGH ❌ | ✅ **RESOLVED** | Query contract post-import |
| Cannot detect genesis/transfer | HIGH ❌ | ✅ **RESOLVED** | Check witness count |
| Cannot extract TX ID | HIGH ❌ | ✅ **RESOLVED** | `witness.id` IS Txid |
| Cannot determine status | MEDIUM ⚠️ | ✅ **RESOLVED** | Map `WitnessStatus` enum |

---

### Updated Confidence Levels

| Component | Before | After | Notes |
|-----------|--------|-------|-------|
| Genesis Detection | 3/10 | **9/10** ✅ | Witness count check |
| TX ID Extraction | 2/10 | **10/10** ✅ | Direct type mapping |
| Status Detection | 4/10 | **9/10** ✅ | Clear enum mapping |
| **Overall Phase 4B** | **3/10** | **9/10** ✅ | **Production-ready** |

---

### Implementation Complexity

**Original Estimate**: Unknown (blocked by API limitations)

**Updated Estimate**: 1-2 hours (Simple contract querying)

**Reasons for Simplicity**:
1. ✅ No need to parse consignment structure
2. ✅ Direct witness querying API available
3. ✅ Type mappings are 1:1 (no conversions)
4. ✅ Clear enum values for status

---

### Key Takeaways

1. **RGB API Design**: The consignment is consumed during import for a reason - it's transformed into queryable contract state.

2. **Witness = Bitcoin TX**: For Bitcoin RGB, witnesses directly correspond to Bitcoin transactions.

3. **Genesis vs Transfer**: The distinguishing factor is the presence of witnesses (Bitcoin TXs).

4. **Query After Import**: Always query the contract state after `consume_from_file()` rather than trying to parse the consignment.

---

### References for Phase 4B Implementation

**Key Source Files**:
- `/rgb-std/src/consignment.rs` - Consignment structure (opaque by design)
- `/rgb-std/src/contract.rs` - Contract query methods (`witnesses()`, `witness_ids()`)
- `/rgb-std/src/pile.rs` - `Witness` struct and `WitnessStatus` enum
- `/rgb-std/src/popls/bp.rs` - `TxoSeal` type for Bitcoin
- `/rgb-std/src/stl.rs` - Type definitions and mappings

---

**Phase 4B Status**: 📋 Research Complete - **Ready for Production Implementation** (Confidence: 9/10)

---

### Implementation Notes (Post-Implementation)

**Initial API Limitation**: During initial implementation, we discovered that the `with_contract` method in `Contracts` struct was **private**, preventing access to `Contract::witnesses()` method.

**Resolution**: ✅ **RESOLVED** - Added public witness query methods to RGB source code.

**RGB Source Code Modifications** (`rgb-std/src/contracts.rs`):

Added three new public methods:
1. `contract_witnesses(contract_id)` - Get all witnesses for a contract
2. `contract_witness_ids(contract_id)` - Get all witness IDs (Bitcoin TXIDs)
3. `contract_witness_count(contract_id)` - Get witness count (for genesis detection)

**Final Implementation**: The `accept_consignment` function now fully supports:
✅ Validates and imports consignments (both genesis and transfer)
✅ Detects newly imported contracts
✅ Returns contract ID
✅ Distinguishes genesis from transfer (witness count check)
✅ Extracts Bitcoin TX ID (from witness.id)
✅ Determines confirmation status (from witness.status enum)

**User Experience**: Complete. Users now see:
- ✅ Import type: 🎁 Genesis or 💸 Transfer
- ✅ Transaction status: ⏳ Pending or ✅ Confirmed
- ✅ Bitcoin TX link to mempool explorer (for transfers)
- ✅ Contextual success messages

**Status Mappings**:
```rust
WitnessStatus::Genesis => "genesis_imported"
WitnessStatus::Offchain => "offchain"
WitnessStatus::Tentative => "pending"
WitnessStatus::Mined(_) => "confirmed"
WitnessStatus::Archived => "archived"
```

**Frontend Enhancements**:
- Type badges with colors (blue for genesis, purple for transfer)
- Status badges with colors (green for confirmed, yellow for pending)
- Clickable Bitcoin TX links to mempool.space
- Contextual success messages based on import type

---

