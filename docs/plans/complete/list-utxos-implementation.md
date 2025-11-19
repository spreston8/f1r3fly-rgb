# List UTXOs Command Implementation Plan

## Executive Summary

Implementation plan for the `list-utxos` command in `f1r3fly-rgb-wallet`. This command is critical for:
- Test automation (automatic genesis UTXO selection)
- User visibility into UTXO state and RGB occupation
- Safe UTXO management to prevent accidental RGB token loss
- Smooth asset issuance and transfer workflows

**Timeline**: 2-3 days  
**Priority**: High (blocks RGB testing automation)  
**Phase**: Phase 2, Week 4 (Day 22 enhancement)

---

## Command Specification

### Basic Usage

```bash
# List all UTXOs
f1r3fly-rgb-wallet --wallet <name> list-utxos --password <pass>

# Filter options
f1r3fly-rgb-wallet --wallet <name> list-utxos \
  --available-only      # Only UTXOs not occupied by RGB
  --rgb-only           # Only UTXOs with RGB seals
  --confirmed-only     # Only confirmed UTXOs (default for safety)
  --min-amount <btc>   # Minimum amount filter

# Output format
f1r3fly-rgb-wallet --wallet <name> list-utxos \
  --format table       # Human-readable (default)
  --format json        # Machine-readable
  --format compact     # Script-friendly: "txid:vout amount status"
```

### Output Examples

#### Table Format (Default)
```
UTXO List for wallet: test1
=========================================

Outpoint              | Amount (BTC) | Confirmations | Status        | RGB Assets
--------------------- | ------------ | ------------- | ------------- | ----------------
abc123...def:0        | 0.00030000   | 6             | Available     | -
abc123...def:1        | 50.00000000  | 101           | Available     | -
def456...abc:0        | 0.00030000   | 3             | RGB-Occupied  | TEST (1000)
ghi789...xyz:1        | 0.00025000   | 1             | RGB-Occupied  | TEST (change)

Total UTXOs: 4
Available: 2 (50.00030000 BTC)
RGB-Occupied: 2 (0.00055000 BTC)
```

#### Compact Format (Script-Friendly)
```
abc123...def:0 0.00030000 available
abc123...def:1 50.00000000 available
def456...abc:0 0.00030000 rgb-occupied TEST:1000
ghi789...xyz:1 0.00025000 rgb-occupied TEST:change
```

#### JSON Format
```json
{
  "wallet": "test1",
  "total_utxos": 4,
  "available_count": 2,
  "rgb_occupied_count": 2,
  "total_available_btc": 50.00030000,
  "total_rgb_occupied_btc": 0.00055000,
  "utxos": [
    {
      "outpoint": "abc123...def:0",
      "txid": "abc123...def456...",
      "vout": 0,
      "amount_btc": 0.00030000,
      "amount_sats": 30000,
      "confirmations": 6,
      "status": "available",
      "rgb_assets": []
    },
    {
      "outpoint": "def456...abc:0",
      "txid": "def456...abc123...",
      "vout": 0,
      "amount_btc": 0.00030000,
      "amount_sats": 30000,
      "confirmations": 3,
      "status": "rgb_occupied",
      "rgb_assets": [
        {
          "contract_id": "rgb:2a3b4c...",
          "ticker": "TEST",
          "amount": 1000,
          "seal_type": "genesis"
        }
      ]
    }
  ]
}
```

---

## Implementation Steps

### Step 1: Define Data Structures (src/types.rs)

**Tasks:**
- [ ] Define `UtxoInfo` struct with all UTXO details
- [ ] Define `UtxoStatus` enum (Available, RgbOccupied, Unconfirmed)
- [ ] Define `RgbSealInfo` struct for RGB asset metadata
- [ ] Define `SealType` enum (Genesis, Transfer, Change)
- [ ] Define `UtxoFilter` struct for filtering options
- [ ] Define `OutputFormat` enum (Table, Json, Compact)
- [ ] Implement serialization for JSON output

**Code Structure:**
```rust
// src/types.rs

#[derive(Debug, Clone, Serialize)]
pub struct UtxoInfo {
    pub outpoint: String,        // "txid:vout"
    pub txid: String,
    pub vout: u32,
    pub amount_sats: u64,
    pub amount_btc: f64,
    pub confirmations: u32,
    pub status: UtxoStatus,
    pub rgb_assets: Vec<RgbSealInfo>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum UtxoStatus {
    Available,
    RgbOccupied,
    Unconfirmed,
}

#[derive(Debug, Clone, Serialize)]
pub struct RgbSealInfo {
    pub contract_id: String,
    pub ticker: String,
    pub amount: Option<u64>,
    pub seal_type: SealType,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SealType {
    Genesis,
    Transfer,
    Change,
}

#[derive(Debug, Clone, Default)]
pub struct UtxoFilter {
    pub available_only: bool,
    pub rgb_only: bool,
    pub confirmed_only: bool,
    pub min_amount_sats: Option<u64>,
}

#[derive(Debug, Clone)]
pub enum OutputFormat {
    Table,
    Json,
    Compact,
}
```

**Success Criteria:**
- ✅ All structs compile
- ✅ Serialization works (test with sample data)
- ✅ Documentation comments added

---

### Step 2: Implement Bitcoin Layer (src/bitcoin/balance.rs)

**Tasks:**
- [ ] Add `list_all_utxos()` method to `BitcoinWallet`
- [ ] Query BDK for all UTXOs with confirmation counts
- [ ] Convert BDK UTXO format to our `UtxoInfo` (without RGB data)
- [ ] Handle edge cases (empty wallet, unconfirmed UTXOs)
- [ ] Add unit tests

**Implementation Details:**
```rust
// src/bitcoin/balance.rs

impl BitcoinWallet {
    /// Get all UTXOs from BDK wallet (Bitcoin-only info)
    pub fn list_all_utxos(&self) -> Result<Vec<UtxoInfo>> {
        // 1. Get UTXOs from BDK wallet.list_unspent()
        // 2. Get current blockchain height
        // 3. Calculate confirmations for each UTXO
        // 4. Convert to UtxoInfo (status: Available or Unconfirmed)
        // 5. Sort by confirmations (highest first)
    }
}
```

**Success Criteria:**
- ✅ Returns all wallet UTXOs
- ✅ Confirmation counts accurate
- ✅ Handles unconfirmed UTXOs
- ✅ Proper error handling

---

### Step 3: Implement RGB Integration (src/f1r3fly/balance.rs)

**Tasks:**
- [ ] Add `get_rgb_seal_info()` method to query RGB seals on UTXOs
- [ ] Query `F1r3flyRgbContracts` for seal assignments
- [ ] Query `BitcoinAnchorTracker` for UTXO seal mappings
- [ ] Build `RgbSealInfo` for each occupied UTXO
- [ ] Handle multiple seals on same UTXO (if possible)

**Implementation Details:**
```rust
// src/f1r3fly/balance.rs

pub fn get_rgb_seal_info(
    contracts: &F1r3flyRgbContracts,
    tracker: &BitcoinAnchorTracker,
    outpoint: &OutPoint,
) -> Result<Vec<RgbSealInfo>> {
    // 1. Query tracker for seals at this outpoint
    // 2. For each seal, get contract details from F1r3flyRgbContracts
    // 3. Get ticker, amount, seal type
    // 4. Build RgbSealInfo structs
    // 5. Return list (may be empty for non-RGB UTXOs)
}
```

**Success Criteria:**
- ✅ Identifies RGB-occupied UTXOs
- ✅ Extracts correct contract ID, ticker, amount
- ✅ Distinguishes genesis vs transfer vs change seals
- ✅ Returns empty list for non-RGB UTXOs

---

### Step 4: Implement Manager Orchestration (src/manager.rs)

**Tasks:**
- [ ] Add `list_utxos()` method to `WalletManager`
- [ ] Orchestrate Bitcoin + RGB data merging
- [ ] Apply filters (available-only, rgb-only, confirmed-only, min-amount)
- [ ] Mark UTXOs as Available or RgbOccupied based on seal presence
- [ ] Return filtered and enriched UTXO list

**Implementation Details:**
```rust
// src/manager.rs

impl WalletManager {
    pub fn list_utxos(&self, filter: UtxoFilter) -> Result<Vec<UtxoInfo>> {
        // 1. Get Bitcoin UTXOs from bitcoin wallet
        let mut utxos = self.bitcoin_wallet.list_all_utxos()?;
        
        // 2. If F1r3fly contracts loaded, enrich with RGB data
        if let Some(ref contracts) = self.f1r3fly_contracts {
            for utxo in &mut utxos {
                let outpoint = parse_outpoint(&utxo.outpoint)?;
                let rgb_seals = get_rgb_seal_info(
                    contracts,
                    &self.tracker,
                    &outpoint,
                )?;
                
                if !rgb_seals.is_empty() {
                    utxo.status = UtxoStatus::RgbOccupied;
                    utxo.rgb_assets = rgb_seals;
                }
            }
        }
        
        // 3. Apply filters
        let filtered = apply_filters(utxos, filter);
        
        Ok(filtered)
    }
}

fn apply_filters(utxos: Vec<UtxoInfo>, filter: UtxoFilter) -> Vec<UtxoInfo> {
    utxos.into_iter()
        .filter(|u| {
            // available_only filter
            if filter.available_only && u.status != UtxoStatus::Available {
                return false;
            }
            // rgb_only filter
            if filter.rgb_only && u.status != UtxoStatus::RgbOccupied {
                return false;
            }
            // confirmed_only filter
            if filter.confirmed_only && u.confirmations == 0 {
                return false;
            }
            // min_amount filter
            if let Some(min) = filter.min_amount_sats {
                if u.amount_sats < min {
                    return false;
                }
            }
            true
        })
        .collect()
}
```

**Success Criteria:**
- ✅ Merges Bitcoin + RGB data correctly
- ✅ All filters work as expected
- ✅ No panic on missing RGB state
- ✅ Proper error propagation

---

### Step 5: Implement CLI Command (src/cli/commands/bitcoin.rs)

**Tasks:**
- [ ] Add `list-utxos` subcommand to CLI args
- [ ] Parse all flags (--available-only, --rgb-only, --format, etc.)
- [ ] Implement `handle_list_utxos()` function
- [ ] Implement table formatter (`print_utxos_table()`)
- [ ] Implement JSON formatter (`print_utxos_json()`)
- [ ] Implement compact formatter (`print_utxos_compact()`)
- [ ] Add summary statistics at bottom

**Implementation Details:**
```rust
// src/cli/commands/bitcoin.rs

pub fn handle_list_utxos(
    manager: &WalletManager,
    filter: UtxoFilter,
    format: OutputFormat,
) -> Result<()> {
    let utxos = manager.list_utxos(filter)?;
    
    match format {
        OutputFormat::Table => print_utxos_table(&utxos),
        OutputFormat::Json => print_utxos_json(&utxos),
        OutputFormat::Compact => print_utxos_compact(&utxos),
    }
    
    Ok(())
}

fn print_utxos_table(utxos: &[UtxoInfo]) {
    // 1. Print header
    // 2. Print each UTXO as table row
    // 3. Print summary statistics
}

fn print_utxos_json(utxos: &[UtxoInfo]) {
    // Build JSON response with metadata + utxos array
    // Use serde_json to serialize
}

fn print_utxos_compact(utxos: &[UtxoInfo]) {
    // One line per UTXO: "txid:vout amount status [rgb_info]"
}
```

**Success Criteria:**
- ✅ All output formats work correctly
- ✅ Table formatting is clean and aligned
- ✅ JSON is valid and complete
- ✅ Compact format is script-parseable
- ✅ Summary statistics accurate

---

### Step 6: Wire Up CLI Args (src/cli/args.rs)

**Tasks:**
- [ ] Add `ListUtxos` variant to command enum
- [ ] Define all flags with clap derives
- [ ] Add help text and examples
- [ ] Wire up to `handle_list_utxos()` in main command handler

**Implementation Details:**
```rust
// src/cli/args.rs

#[derive(Debug, Subcommand)]
pub enum Commands {
    // ... existing commands ...
    
    /// List all UTXOs with status and RGB occupation info
    ListUtxos {
        /// Only show available (non-RGB) UTXOs
        #[arg(long)]
        available_only: bool,
        
        /// Only show RGB-occupied UTXOs
        #[arg(long)]
        rgb_only: bool,
        
        /// Only show confirmed UTXOs
        #[arg(long, default_value = "true")]
        confirmed_only: bool,
        
        /// Minimum amount in BTC
        #[arg(long)]
        min_amount: Option<f64>,
        
        /// Output format: table, json, compact
        #[arg(long, default_value = "table")]
        format: String,
    },
}
```

**Success Criteria:**
- ✅ Command appears in help output
- ✅ All flags work correctly
- ✅ Validation for format flag
- ✅ Proper error messages for invalid input

---

### Step 7: Update get-balance Command (Enhancement)

**Tasks:**
- [ ] Enhance `get-balance` output to show UTXO summary
- [ ] Add line showing total UTXOs, available, RGB-occupied
- [ ] Add note to use `list-utxos` for details

**Implementation Details:**
```rust
// src/cli/commands/bitcoin.rs (enhance existing function)

pub fn handle_get_balance(...) {
    // ... existing balance display ...
    
    // Add UTXO summary
    let all_utxos = manager.list_utxos(UtxoFilter::default())?;
    let available = all_utxos.iter().filter(|u| u.status == UtxoStatus::Available).count();
    let rgb_occupied = all_utxos.iter().filter(|u| u.status == UtxoStatus::RgbOccupied).count();
    
    println!("\nUTXO Summary:");
    println!("  Total UTXOs:    {}", all_utxos.len());
    println!("  Available:      {}", available);
    println!("  RGB-Occupied:   {}", rgb_occupied);
    println!("\nUse 'list-utxos' for detailed UTXO information");
}
```

**Success Criteria:**
- ✅ Summary shows correct counts
- ✅ Doesn't break existing balance display
- ✅ Helpful pointer to list-utxos command

---

### Step 8: Update CLI Spec Documentation

**Tasks:**
- [ ] Add `list-utxos` to CLI command spec in implementation plan
- [ ] Add to README examples
- [ ] Update help text to cross-reference related commands

**Location:**
- `docs/plans/f1r3fly-rgb-wallet-implementation-plan.md` line ~760
- `f1r3fly-rgb-wallet/README.md`

**Content to Add:**
```bash
# List UTXOs with details
f1r3fly-rgb-wallet --wallet <name> list-utxos \
  [--available-only] \
  [--rgb-only] \
  [--confirmed-only] \
  [--min-amount <btc>] \
  [--format table|json|compact]
```

**Success Criteria:**
- ✅ Command documented in plan
- ✅ Examples in README
- ✅ Clear usage instructions

---

## Testing Plan

### Unit Tests

**Location**: `src/bitcoin/balance.rs`, `src/manager.rs`

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_list_utxos_empty_wallet() {
        // Create empty wallet, should return empty list
    }
    
    #[test]
    fn test_list_utxos_bitcoin_only() {
        // Mock wallet with UTXOs, no RGB
        // Should return all as Available
    }
    
    #[test]
    fn test_list_utxos_rgb_occupied() {
        // Mock wallet with RGB-occupied UTXO
        // Should mark as RgbOccupied with correct asset info
    }
    
    #[test]
    fn test_filter_available_only() {
        // Apply filter, should exclude RGB-occupied
    }
    
    #[test]
    fn test_filter_rgb_only() {
        // Apply filter, should exclude Available
    }
    
    #[test]
    fn test_filter_confirmed_only() {
        // Apply filter, should exclude unconfirmed
    }
    
    #[test]
    fn test_filter_min_amount() {
        // Apply filter, should exclude below threshold
    }
}
```

### Integration Tests (test_cli.sh)

**Add Test 8a: List UTXOs (after Test 8)**

```bash
# Test 8a: List UTXOs
echo "======================================"
echo "Test 8a: List UTXOs"
echo "======================================"
if [ "$REGTEST_RUNNING" = true ]; then
    echo "All UTXOs (table format):"
    cargo run --bin f1r3fly-rgb-wallet -- \
        --data-dir "$TEMP_DIR" \
        --wallet "$WALLET_NAME" \
        list-utxos --password "$PASSWORD" 2>&1 | grep -v "warning:"
    
    echo ""
    echo "Available UTXOs only (compact format):"
    cargo run --bin f1r3fly-rgb-wallet -- \
        --data-dir "$TEMP_DIR" \
        --wallet "$WALLET_NAME" \
        list-utxos \
        --available-only \
        --format compact \
        --password "$PASSWORD" 2>&1 | grep -v "warning:"
    
    echo ""
    echo "JSON output test:"
    cargo run --bin f1r3fly-rgb-wallet -- \
        --data-dir "$TEMP_DIR" \
        --wallet "$WALLET_NAME" \
        list-utxos \
        --format json \
        --password "$PASSWORD" 2>&1 | grep -v "warning:" | jq '.total_utxos'
else
    echo "⚠ Skipping (regtest not running)"
fi
echo ""
```

**Add Helper Function for Genesis UTXO Selection**

```bash
# Helper: Get first available confirmed UTXO for RGB operations
get_available_utxo() {
    cargo run --bin f1r3fly-rgb-wallet -- \
        --data-dir "$TEMP_DIR" \
        --wallet "$WALLET_NAME" \
        list-utxos \
        --format compact \
        --available-only \
        --confirmed-only \
        --password "$PASSWORD" 2>&1 | \
        grep -v "warning:" | \
        head -1 | \
        awk '{print $1}'  # Extract txid:vout
}
```

**Update Test 10: Issue Asset (Use Helper)**

```bash
# Test 10: Issue RGB Asset (improved)
echo "======================================"
echo "Test 10: Issue RGB Asset"
echo "======================================"
if [ "$F1R3NODE_RUNNING" = true ] && [ "$REGTEST_RUNNING" = true ]; then
    # Automatically select genesis UTXO
    GENESIS_UTXO=$(get_available_utxo)
    
    if [ -z "$GENESIS_UTXO" ]; then
        echo "ERROR: No available UTXO for genesis"
        echo "Available UTXOs:"
        cargo run --bin f1r3fly-rgb-wallet -- \
            --data-dir "$TEMP_DIR" \
            --wallet "$WALLET_NAME" \
            list-utxos --available-only --password "$PASSWORD"
        exit 1
    fi
    
    echo "✓ Selected genesis UTXO: $GENESIS_UTXO"
    echo ""
    
    ISSUE_OUTPUT=$(cargo run --bin f1r3fly-rgb-wallet -- \
        --data-dir "$TEMP_DIR" \
        --wallet "$WALLET_NAME" \
        issue-asset \
        --ticker TEST \
        --name TestToken \
        --supply 1000 \
        --precision 0 \
        --genesis-utxo "$GENESIS_UTXO" \
        --password "$PASSWORD" 2>&1)
    
    echo "$ISSUE_OUTPUT" | grep -v "warning:"
    
    # Extract contract ID for future tests
    CONTRACT_ID=$(echo "$ISSUE_OUTPUT" | grep -i "contract" | grep -o 'rgb:[a-zA-Z0-9]*')
    echo ""
    echo "✓ CONTRACT_ID stored: $CONTRACT_ID"
else
    echo "⚠ Skipping (F1r3node or regtest not running)"
fi
echo ""
```

**Add Test 12a: Verify RGB-Occupied Status**

```bash
# Test 12a: Verify UTXO marked as RGB-occupied
echo "======================================"
echo "Test 12a: Verify RGB-Occupied UTXO"
echo "======================================"
if [ "$F1R3NODE_RUNNING" = true ] && [ "$REGTEST_RUNNING" = true ]; then
    echo "RGB-occupied UTXOs:"
    cargo run --bin f1r3fly-rgb-wallet -- \
        --data-dir "$TEMP_DIR" \
        --wallet "$WALLET_NAME" \
        list-utxos \
        --rgb-only \
        --password "$PASSWORD" 2>&1 | grep -v "warning:"
    
    echo ""
    echo "Verify genesis UTXO is marked:"
    cargo run --bin f1r3fly-rgb-wallet -- \
        --data-dir "$TEMP_DIR" \
        --wallet "$WALLET_NAME" \
        list-utxos \
        --format compact \
        --rgb-only \
        --password "$PASSWORD" 2>&1 | \
        grep -v "warning:" | \
        grep "$GENESIS_UTXO"
    
    if [ $? -eq 0 ]; then
        echo "✓ Genesis UTXO correctly marked as RGB-occupied"
    else
        echo "✗ ERROR: Genesis UTXO not marked as RGB-occupied"
        exit 1
    fi
else
    echo "⚠ Skipping (F1r3node or regtest not running)"
fi
echo ""
```

**Update Test Summary (lines 244-282)**

```bash
echo "======================================"
echo "Test Summary"
echo "======================================"
echo ""
echo "Bitcoin Layer Tests:"
echo "  ✓ Test 1: Wallet creation - SUCCESS"
echo "  ✓ Test 2: List wallets - SUCCESS"
if [ "$REGTEST_RUNNING" = true ]; then
    echo "  ✓ Test 3: Address extraction - SUCCESS"
    echo "  ✓ Test 4: Initial sync - SUCCESS"
    echo "  ✓ Test 5: Mine blocks to fund wallet - SUCCESS"
    echo "  ✓ Test 6: Sync wallet (after funding) - SUCCESS"
    echo "  ✓ Test 7: Get balance (funded) - SUCCESS"
    echo "  ✓ Test 8: Create UTXO - SUCCESS"
    echo "  ✓ Test 8a: List UTXOs - SUCCESS"
    echo "  ✓ Test 9: Send Bitcoin - SUCCESS"
    echo ""
    echo "  All Bitcoin tests passed! (10/10)"
else
    echo "  ⚠ Tests 3-9: SKIPPED (regtest not running)"
fi
echo ""
echo "RGB Asset Tests:"
if [ "$F1R3NODE_RUNNING" = true ] && [ "$REGTEST_RUNNING" = true ]; then
    echo "  ✓ Test 10: Issue asset (with auto-selection) - SUCCESS"
    echo "  ✓ Test 11: List assets - SUCCESS"
    echo "  ✓ Test 12: RGB balance - SUCCESS"
    echo "  ✓ Test 12a: Verify RGB-occupied UTXO - SUCCESS"
    echo ""
    echo "  All RGB tests passed! (4/4)"
else
    echo "  ⚠ RGB tests SKIPPED"
fi
```

---

## Success Criteria

### Functional Requirements
- ✅ `list-utxos` command works without RGB (Bitcoin-only wallets)
- ✅ `list-utxos` shows RGB occupation status when contracts exist
- ✅ All filters work correctly (available-only, rgb-only, confirmed-only, min-amount)
- ✅ All output formats work (table, json, compact)
- ✅ Table format is readable and well-formatted
- ✅ JSON format is valid and parseable
- ✅ Compact format is script-friendly (automation ready)

### Integration Requirements
- ✅ `get-balance` shows UTXO summary
- ✅ `test_cli.sh` uses `list-utxos` for genesis UTXO selection
- ✅ `test_cli.sh` verifies RGB occupation after issuance
- ✅ RGB issuance test fully automated (no manual UTXO input)

### Code Quality
- ✅ Unit tests pass
- ✅ Integration tests pass
- ✅ No clippy warnings
- ✅ Proper error handling
- ✅ Documentation complete

### User Experience
- ✅ Clear help text
- ✅ Intuitive flag names
- ✅ Useful error messages
- ✅ Cross-references to related commands

---

## Timeline

**Day 1 (4-5 hours):**
- Step 1: Data structures
- Step 2: Bitcoin layer implementation
- Unit tests for Bitcoin layer

**Day 2 (4-5 hours):**
- Step 3: RGB integration
- Step 4: Manager orchestration
- Unit tests for RGB integration and filtering

**Day 3 (3-4 hours):**
- Step 5: CLI implementation (all formatters)
- Step 6: CLI args wiring
- Step 7: Enhance get-balance
- Step 8: Documentation updates

**Day 3 (continued):**
- Integration testing with test_cli.sh
- Bug fixes and refinement

**Total**: ~12-14 hours over 2-3 days

---

## Files to Create/Modify

### New Files
- None (all code goes in existing files)

### Modified Files
1. `src/types.rs` - Add UTXO-related types
2. `src/bitcoin/balance.rs` - Add `list_all_utxos()`
3. `src/f1r3fly/balance.rs` - Add `get_rgb_seal_info()`
4. `src/manager.rs` - Add `list_utxos()` orchestration
5. `src/cli/args.rs` - Add `ListUtxos` command variant
6. `src/cli/commands/bitcoin.rs` - Add `handle_list_utxos()` and formatters
7. `src/cli/commands/bitcoin.rs` - Enhance `handle_get_balance()`
8. `test_cli.sh` - Add Test 8a, helper function, update Tests 10, 12a
9. `docs/plans/f1r3fly-rgb-wallet-implementation-plan.md` - Add command to spec
10. `f1r3fly-rgb-wallet/README.md` - Add usage examples

---

## Risk Mitigation

### Analysis Based on Traditional RGB Wallet Implementation

All risk assessments have been validated against the working traditional RGB wallet in `wallet/` directory.

---

### **Issue 1: RGB seal type tracking**

**Risk Level**: ✅ **LOW**

**Current Implementation Evidence:**
- `wallet/src/rgb/asset.rs` (lines 103-155): `get_bound_assets()` shows the system **does NOT track seal types** (genesis/transfer/change) explicitly
- Traditional RGB uses `BoundAsset` struct with only: `asset_id`, `asset_name`, `ticker`, `amount` (no `seal_type` field)
- The RGB-std library tracks seals internally via `owned_state.assignment.seal.primary` (TxoSeal with outpoint)
- **Seal type distinction is NOT needed** for basic UTXO listing - only "RGB-occupied" vs "available" matters

**Revised Mitigation:**
- Start with simple boolean `is_rgb_occupied` flag (already proven pattern in existing code)
- **Do NOT implement `SealType` enum initially** - it's unnecessary complexity
- Can determine "genesis" heuristically later if needed (check if outpoint matches any `genesis_utxo` in metadata)
- Change seals are auto-tracked by `BitcoinAnchorTracker`, don't need explicit labeling

**Implementation Change:**
```rust
// SIMPLIFIED - Remove SealType enum from Step 1
#[derive(Debug, Clone, Serialize)]
pub struct RgbSealInfo {
    pub contract_id: String,
    pub ticker: String,
    pub amount: Option<u64>,
    // NO seal_type field needed
}
```

---

### **Issue 2: Multiple RGB assets per UTXO**

**Risk Level**: ✅ **LOW** (Confirmed Supported)

**Current Implementation Evidence:**
- `wallet/src/rgb/asset.rs` (lines 113-154): `get_bound_assets()` returns `Vec<BoundAsset>` - explicitly supports **multiple assets per UTXO**
- `wallet/src/bitcoin/balance.rs` (line 145): Uses `HashMap<String, Vec<BoundAsset>>` - UTXO can have multiple RGB assets
- `wallet/src/bitcoin/balance_checker.rs` (line 183): `UTXO.bound_assets: Vec<BoundAsset>` - API already handles this
- RGB protocol **DOES allow multiple contracts on one UTXO** (each as separate state assignment)

**Example Scenario:**
```
UTXO abc123:0 can hold:
  - Asset A (TEST): 1000 tokens
  - Asset B (USDT): 500 tokens  
  - Asset C (change seal with 0 balance)
```

**Mitigation:**
- Keep `Vec<RgbSealInfo>` design - it's architecturally correct
- Implementation must aggregate multiple assets when displaying
- Table format: Show comma-separated list or multiple rows
- No changes needed - original design is correct

---

### **Issue 3: Performance with large wallets**

**Risk Level**: ⚠️ **MODERATE** (F1r3fly-specific concern)

**Current Implementation Evidence:**

**Traditional RGB (In-Memory):**
```rust
// wallet/src/rgb/asset.rs lines 85-97
for contract_id in contracts.contract_ids() {
    let state = contracts.contract_state(contract_id);
    for (_state_name, owned_states) in state.owned {
        for owned_state in owned_states {
            if seal_outpoint == target_outpoint {
                return Ok(true);  // Early exit optimization
            }
        }
    }
}
```
- Uses in-memory RGB-std `Contracts` state (fast)
- O(contracts × UTXOs) but state already loaded

**F1r3fly-RGB (Remote Queries):**
- `f1r3fly-rgb-wallet/src/f1r3fly/balance.rs` (lines 305-358): `get_occupied_utxos()` iterates **all contracts × all UTXOs**
- Each UTXO queries F1r3node via gRPC: `contract.balance(&seal).await` 
- Network latency per UTXO makes this slower than traditional RGB

**Caching Infrastructure:**
- `wallet/src/rgb/cache.rs`: Entire file dedicated to **RGB runtime caching** for performance
- Lines 56-68: "Fast path" uses read lock for cache hits
- Lines 233-254: Maintains cache statistics for monitoring
- Lines 260-327: LRU eviction with idle timeout (prevents unbounded memory)

**Mitigation Strategy:**
- **Phase 1**: Simple O(contracts × UTXOs) iteration (proven acceptable in existing wallet for typical sizes)
- **Phase 2**: Build `HashSet<OutPoint>` cache of occupied UTXOs on first query
- **Phase 3**: Invalidate cache only on RGB state changes (issue/transfer/accept)
- **F1r3fly optimization**: Batch gRPC queries if possible, or cache balance results per-contract

**Performance Notes:**
- Traditional wallet handles this fine without caching (state in memory)
- F1r3fly needs more aggressive caching due to network round trips
- Consider background pre-population of occupied UTXO set on wallet load

---

### **Issue 4: Unconfirmed RGB UTXOs**

**Risk Level**: ✅ **LOW** (Already Handled by BDK)

**Current Implementation Evidence:**
- `f1r3fly-rgb-wallet/src/bitcoin/balance.rs` (lines 173-179): Already tracks confirmation status separately:
  ```rust
  let is_confirmed = utxo.chain_position.is_confirmed();
  let confirmation_height = match utxo.chain_position {
      ChainPosition::Confirmed { anchor, .. } => Some(anchor.block_id.height),
      _ => None,
  };
  ```
- Traditional wallet uses `is_confirmed` boolean flag
- BDK provides confirmation data per-UTXO natively (no ambiguity)

**Mitigation:**
- Use BDK's built-in confirmation tracking (already implemented)
- Display both properties **independently**:
  - Confirmation status: `confirmations: 0` (unconfirmed) vs `confirmations: 6` (confirmed)
  - RGB occupation: `Available` vs `RgbOccupied`
- These are orthogonal properties - don't conflate them
- Default filter: `--confirmed-only` for safety (exclude unconfirmed UTXOs by default)
- Add warning in output for unconfirmed RGB UTXOs (don't hide them, just mark clearly)

**Display Example:**
```
Outpoint              | Amount (BTC) | Confirmations | Status        | RGB Assets
--------------------- | ------------ | ------------- | ------------- | ----------------
abc123...def:0        | 0.00030000   | 0             | ⚠️ Unconfirmed| TEST (1000)
def456...abc:0        | 0.00030000   | 6             | RGB-Occupied  | TEST (1000)
```

---

## Key Insights from Traditional RGB Wallet

Based on analysis of `wallet/` implementation:

1. **Caching exists but not for UTXO listing** - The `RgbRuntimeCache` caches runtime instances, not UTXO queries
2. **Simple data structures work** - `BoundAsset` has only 4 fields, no complex metadata or seal type tracking
3. **Multiple assets per UTXO is standard** - Code explicitly handles `Vec<BoundAsset>` per UTXO
4. **Performance is acceptable** - No complaints in existing wallet despite O(n×m) complexity for in-memory queries
5. **F1r3fly implementation may be slower** - Queries F1r3node over gRPC per-UTXO (`contract.balance(&seal)`) vs in-memory RGB-std state
6. **Seal type tracking unnecessary** - Traditional wallet doesn't distinguish genesis/transfer/change seals in display
7. **Confirmation tracking separate** - BDK handles this natively; treat as orthogonal to RGB occupation

**Main Difference:**
- **Traditional RGB**: Queries in-memory `Contracts` state (fast, O(n×m) acceptable)
- **F1r3fly-RGB**: Queries F1r3node over gRPC for each UTXO (slower, caching MORE important)

---

## Future Enhancements

### Phase 3 Additions (Transfer Support)
- Add filter for change UTXOs vs transfer UTXOs
- Show transfer history per UTXO
- Display received but not yet accepted consignments

### Phase 4 Additions (Polish)
- Add `--watch` flag for real-time UTXO monitoring
- Colorized output (RGB-occupied in yellow/red)
- Export to CSV format
- UTXO graph visualization (ASCII art)

---

## Appendix: Example Test Output

```bash
$ f1r3fly-rgb-wallet --wallet test1 list-utxos --password test

UTXO List for wallet: test1
=========================================

Outpoint              | Amount (BTC) | Confirmations | Status        | RGB Assets
--------------------- | ------------ | ------------- | ------------- | ----------------
a1b2c3d4...e5f6:0     | 0.00030000   | 6             | Available     | -
a1b2c3d4...e5f6:1     | 50.00000000  | 101           | Available     | -
f7e8d9c0...b1a2:0     | 0.00030000   | 3             | RGB-Occupied  | TEST (1000)

Total UTXOs: 3
Available: 2 (50.00030000 BTC)
RGB-Occupied: 1 (0.00030000 BTC)


$ f1r3fly-rgb-wallet --wallet test1 list-utxos --rgb-only --format compact

f7e8d9c0...b1a2:0 0.00030000 rgb-occupied TEST:1000


$ f1r3fly-rgb-wallet --wallet test1 list-utxos --available-only --format json | jq
{
  "wallet": "test1",
  "total_utxos": 2,
  "available_count": 2,
  "rgb_occupied_count": 0,
  "total_available_btc": 50.00030000,
  "total_rgb_occupied_btc": 0.0,
  "utxos": [
    {
      "outpoint": "a1b2c3d4...e5f6:0",
      "txid": "a1b2c3d4e5f67890...",
      "vout": 0,
      "amount_btc": 0.00030000,
      "amount_sats": 30000,
      "confirmations": 6,
      "status": "available",
      "rgb_assets": []
    },
    {
      "outpoint": "a1b2c3d4...e5f6:1",
      "txid": "a1b2c3d4e5f67890...",
      "vout": 1,
      "amount_btc": 50.00000000,
      "amount_sats": 5000000000,
      "confirmations": 101,
      "status": "available",
      "rgb_assets": []
    }
  ]
}
```

---

**End of Implementation Plan**

