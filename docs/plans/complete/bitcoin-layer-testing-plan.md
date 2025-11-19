# Bitcoin Layer Testing Plan

## Overview

Comprehensive testing plan for the Bitcoin layer of `f1r3fly-rgb-wallet`, covering wallet initialization, network sync, balance queries, UTXO operations, and manager integration.

**Test File**: `tests/bitcoin_integration_test.rs`  
**Common Utilities**: `tests/common/mod.rs`  
**Environment**: Bitcoin Core regtest (via `scripts/start-regtest.sh`)  
**Total Tests**: ~31 comprehensive tests  
**Execution**: Parallel (preferred) or sequential if state pollution detected

---

## Test Environment Requirements

### Prerequisites

1. **Regtest Environment Running**:
   ```bash
   ./scripts/start-regtest.sh
   ```
   - Bitcoin Core on port 18443
   - Electrs on port 3002
   - Esplora on port 5001

2. **Funded Test Address**:
   - Address: `bcrt1q6rz28mcfaxtmd6v789l9rrlrusdprr9pz3cppk`
   - Funded with 10 BTC (from start-regtest.sh)

3. **Test Execution**:
   ```bash
   # Parallel (preferred)
   cargo test --test bitcoin_integration_test
   
   # Sequential (if state pollution)
   cargo test --test bitcoin_integration_test -- --test-threads=1
   
   # With output
   cargo test --test bitcoin_integration_test -- --nocapture
   ```

---

## Test Architecture

### Directory Structure

```
tests/
├── common/
│   ├── mod.rs              # Test utilities and environment
│   └── bitcoin_rpc.rs      # Optional: Bitcoin RPC wrapper
└── bitcoin_integration_test.rs  # All Bitcoin layer tests
```

---

## Common Test Utilities (`tests/common/mod.rs`)

### Core Test Environment

```rust
pub struct TestBitcoinEnv {
    /// Temporary directory (auto-cleanup)
    _temp_dir: TempDir,
    
    /// Wallets directory path
    wallets_dir: PathBuf,
    
    /// Global config (regtest)
    config: GlobalConfig,
    
    /// Unique test wallet name (includes test name + UUID)
    test_wallet_name: String,
    
    /// Bitcoin RPC client (for mining blocks, sending funds)
    bitcoin_rpc: Option<BitcoinRpcClient>,
}
```

### Required Helper Functions

#### 1. Environment Setup

```rust
impl TestBitcoinEnv {
    /// Create new test environment with unique wallet name
    pub fn new(test_name: &str) -> Self;
    
    /// Verify regtest is running, panic if not
    fn check_regtest_running() -> Result<(), String>;
    
    /// Get regtest config pointing to localhost:3002
    fn regtest_config() -> GlobalConfig;
}
```

#### 2. Wallet Management

```rust
impl TestBitcoinEnv {
    /// Create and fund a wallet from regtest
    pub fn create_funded_wallet(
        &self,
        name: &str,
        btc_amount: f64,
    ) -> Result<(WalletKeys, WalletMetadata), Box<dyn Error>>;
    
    /// Create wallet without funding
    pub fn create_wallet(
        &self,
        name: &str,
    ) -> Result<(WalletKeys, WalletMetadata), Box<dyn Error>>;
    
    /// Get wallet directory path
    pub fn wallet_dir(&self, name: &str) -> PathBuf;
}
```

#### 3. Bitcoin Operations

```rust
impl TestBitcoinEnv {
    /// Mine N blocks to default mining address
    pub fn mine_blocks(&self, count: u32) -> Result<Vec<String>, String>;
    
    /// Send BTC from regtest mining wallet to address
    pub fn fund_address(&self, address: &str, btc_amount: f64) -> Result<String, String>;
    
    /// Wait for transaction to be confirmed
    pub fn wait_for_confirmation(
        &self,
        txid: &str,
        confirmations: u32,
        timeout_secs: u64,
    ) -> Result<(), String>;
    
    /// Get current blockchain height
    pub fn get_blockchain_height(&self) -> Result<u32, String>;
    
    /// Generate new regtest address for testing
    pub fn get_new_test_address(&self) -> Result<String, String>;
}
```

#### 4. Cleanup & Utilities

```rust
impl TestBitcoinEnv {
    /// Get unique wallet name for this test
    pub fn unique_wallet_name(&self) -> &str;
    
    /// Check if wallet exists
    pub fn wallet_exists(&self, name: &str) -> bool;
}

impl Drop for TestBitcoinEnv {
    /// Cleanup temporary directories
    /// Note: Bitcoin regtest state persists (by design)
    fn drop(&mut self);
}
```

#### 5. Bitcoin RPC Client Helper

```rust
pub struct BitcoinRpcClient {
    datadir: String,
}

impl BitcoinRpcClient {
    pub fn new() -> Self;
    
    /// Execute bitcoin-cli command
    fn execute_cli(&self, args: &[&str]) -> Result<String, String>;
    
    /// Mine blocks: bitcoin-cli generatetoaddress <count> <address>
    pub fn mine_to_address(&self, count: u32, address: &str) -> Result<Vec<String>, String>;
    
    /// Send to address: bitcoin-cli -rpcwallet=mining_wallet sendtoaddress <addr> <amount>
    pub fn send_to_address(&self, address: &str, amount_btc: f64) -> Result<String, String>;
    
    /// Get transaction: bitcoin-cli gettransaction <txid>
    pub fn get_transaction(&self, txid: &str) -> Result<serde_json::Value, String>;
    
    /// Get new address: bitcoin-cli -rpcwallet=mining_wallet getnewaddress
    pub fn get_new_address(&self) -> Result<String, String>;
}
```

---

## Test Modules

### Module 1: Wallet Creation & Persistence

**File Section**: `mod test_wallet_persistence`

#### Test 1.1: `test_bitcoin_wallet_initialization_with_sqlite_persistence`

**Purpose**: Verify BitcoinWallet initializes with SQLite and persists state

**Steps**:
1. Create TestBitcoinEnv with unique wallet name
2. Generate mnemonic and derive keys
3. Initialize BitcoinWallet with descriptor and wallet directory
4. Verify SQLite database file created at `{wallet_dir}/bitcoin.db`
5. Get a new address to trigger persistence
6. Drop wallet instance
7. Create new BitcoinWallet instance pointing to same DB
8. Verify wallet loads existing state (address index preserved)
9. Verify addresses match

**Assertions**:
- SQLite DB file exists
- Wallet can be reloaded from DB
- Address derivation index persists

---

#### Test 1.2: `test_wallet_network_specific_addresses`

**Purpose**: Verify wallet produces correct regtest addresses

**Steps**:
1. Create TestBitcoinEnv
2. Create BitcoinWallet for regtest
3. Get 5 external addresses
4. Get 5 internal (change) addresses
5. Verify all start with `bcrt1` (regtest prefix)
6. Verify external and internal addresses are different
7. Verify addresses are valid P2WPKH

**Assertions**:
- All addresses have `bcrt1` prefix
- External != Internal addresses
- Addresses parse as valid Bitcoin addresses

---

#### Test 1.3: `test_multiple_wallets_independent_state`

**Purpose**: Verify multiple wallets don't interfere with each other

**Steps**:
1. Create TestBitcoinEnv
2. Create wallet1 and wallet2 with different names
3. Get address from wallet1
4. Get address from wallet2
5. Verify different DB files created
6. Verify addresses are different
7. Fund wallet1 only
8. Sync both wallets
9. Verify only wallet1 has balance

**Assertions**:
- Separate `bitcoin.db` files exist
- Wallets have different addresses
- Balances are independent

---

### Module 2: Network & Sync Operations

**File Section**: `mod test_network_sync`

#### Test 2.1: `test_esplora_client_connection_and_height_query`

**Purpose**: Verify EsploraClient connects to regtest Esplora

**Steps**:
1. Create TestBitcoinEnv
2. Create EsploraClient pointing to `http://localhost:3002`
3. Call `get_height()`
4. Call `get_tip_hash()`
5. Use bitcoin-cli to get blockchain height
6. Compare heights (should match or be within 1 block)
7. Verify `is_available()` returns true

**Assertions**:
- Esplora client connects successfully
- Height matches bitcoin-cli
- Tip hash is valid

---

#### Test 2.2: `test_wallet_sync_with_empty_wallet`

**Purpose**: Verify syncing empty wallet works

**Steps**:
1. Create TestBitcoinEnv with new wallet
2. Create BitcoinWallet
3. Call sync_wallet()
4. Verify SyncResult returned
5. Verify height matches blockchain
6. Verify new_transactions = 0
7. Verify balance = 0

**Assertions**:
- Sync completes without error
- Height is current
- No transactions found

---

#### Test 2.3: `test_wallet_sync_detects_received_funds`

**Purpose**: Verify sync detects incoming transactions

**Steps**:
1. Create TestBitcoinEnv with new wallet
2. Create BitcoinWallet and get address
3. Record initial sync state (height, tx count)
4. Send 0.1 BTC to wallet address from regtest
5. Mine 1 block
6. Wait for Electrs to index (2-3 second delay)
7. Sync wallet again
8. Verify SyncResult shows new_transactions > 0
9. Verify balance > 0

**Assertions**:
- new_transactions = 1
- Balance equals sent amount (minus potential dust)
- Height increased by 1

---

#### Test 2.4: `test_wallet_sync_idempotent`

**Purpose**: Verify repeated syncs don't duplicate data

**Steps**:
1. Create funded wallet
2. Sync wallet
3. Record SyncResult (height, tx count)
4. Sync wallet again immediately (no new blocks)
5. Verify SyncResult unchanged

**Assertions**:
- Height same
- Transaction count same
- Balance unchanged

---

#### Test 2.5: `test_sync_updates_wallet_after_spending`

**Purpose**: Verify sync detects spent UTXOs

**Steps**:
1. Create funded wallet
2. Sync and record UTXO count
3. Send Bitcoin to external address
4. Mine 1 block
5. Sync wallet
6. Verify UTXO count decreased
7. Verify balance decreased by amount + fee

**Assertions**:
- UTXO count reflects spend
- Balance correct after spend

---

### Module 3: Balance & UTXO Queries

**File Section**: `mod test_balance_queries`

#### Test 3.1: `test_get_balance_confirmed_vs_unconfirmed`

**Purpose**: Verify balance correctly distinguishes confirmed/unconfirmed

**Steps**:
1. Create TestBitcoinEnv with new wallet
2. Create BitcoinWallet and get address
3. Sync wallet (balance should be 0)
4. Send 0.5 BTC to address (don't mine yet)
5. Sync wallet
6. Get balance - verify unconfirmed > 0, confirmed = 0
7. Mine 1 block
8. Sync wallet
9. Get balance - verify confirmed > 0, unconfirmed = 0

**Assertions**:
- Unconfirmed balance detected before mining
- Confirmed balance appears after mining
- Total = confirmed + unconfirmed

---

#### Test 3.2: `test_list_utxos_with_details`

**Purpose**: Verify list_utxos returns accurate UTXO information

**Steps**:
1. Create funded wallet with 2 separate transactions (0.1 BTC each)
2. Mine 1 block to confirm
3. Sync wallet
4. Create empty RGB-occupied set
5. Call list_utxos()
6. Verify UTXO count = 2
7. For each UTXO, verify:
   - Valid outpoint (txid:vout)
   - Amount = 0.1 BTC
   - is_confirmed = true
   - confirmation_height is set
   - is_rgb_occupied = false
8. Verify UTXOs sorted by amount descending

**Assertions**:
- UTXO count matches transactions
- All fields populated correctly
- Sorted order correct

---

#### Test 3.3: `test_get_addresses_external_and_internal`

**Purpose**: Verify get_addresses returns correct address information

**Steps**:
1. Create wallet
2. Get first receive address and use it
3. Get first change address via internal keychain
4. Call get_addresses(count=None)
5. Verify returns both external and internal addresses
6. Verify addresses have correct prefixes (bcrt1)
7. Verify used addresses marked as is_used=true
8. Verify unused addresses marked as is_used=false

**Assertions**:
- Both keychains represented
- Usage flags correct
- Address format valid

---

#### Test 3.4: `test_get_new_address_increments_index`

**Purpose**: Verify address derivation increments properly

**Steps**:
1. Create wallet
2. Get first address via get_new_address()
3. Get second address via get_new_address()
4. Verify addresses different
5. Verify both valid bcrt1 addresses
6. Get addresses list
7. Verify both addresses in list with correct indices

**Assertions**:
- Each call returns unique address
- Addresses sequential in derivation

---

#### Test 3.5: `test_rgb_occupied_utxo_marking`

**Purpose**: Verify RGB UTXO tracking works correctly

**Steps**:
1. Create funded wallet with 2 UTXOs
2. Sync wallet
3. Create empty RGB-occupied set
4. List UTXOs - verify all is_rgb_occupied=false
5. Mark first UTXO as RGB-occupied
6. List UTXOs with RGB set
7. Verify first UTXO has is_rgb_occupied=true
8. Verify second UTXO has is_rgb_occupied=false
9. Unmark first UTXO
10. List UTXOs - verify all is_rgb_occupied=false

**Assertions**:
- Marking works
- Unmarking works
- Other UTXOs unaffected

---

#### Test 3.6: `test_list_utxos_excludes_spent`

**Purpose**: Verify spent UTXOs not listed

**Steps**:
1. Create funded wallet with 2 UTXOs
2. Sync and list UTXOs (count=2)
3. Spend one UTXO
4. Mine 1 block
5. Sync wallet
6. List UTXOs
7. Verify count = 1 (or 2 if change UTXO created)
8. Verify spent UTXO not in list

**Assertions**:
- Spent UTXOs excluded
- Remaining UTXOs correct

---

### Module 4: UTXO Operations

**File Section**: `mod test_utxo_operations`

#### Test 4.1: `test_create_utxo_self_send_with_specific_amount`

**Purpose**: Verify create_utxo creates exact UTXO amount

**Steps**:
1. Create funded wallet with sufficient balance (1 BTC)
2. Sync wallet
3. Record initial UTXO count and balance
4. Create UTXO of 10,000 sats with medium fee rate
5. Verify UtxoOperationResult returned with txid
6. Mine 1 block
7. Sync wallet
8. List UTXOs
9. Find newly created UTXO
10. Verify amount = 10,000 sats exactly
11. Verify total balance decreased by (10,000 + fee)

**Assertions**:
- UTXO created with exact amount
- Transaction broadcasts successfully
- Fee deducted from source

---

#### Test 4.2: `test_create_utxo_with_different_fee_rates`

**Purpose**: Verify different fee rates work correctly

**Steps**:
1. Create funded wallet
2. Create UTXO with low_priority() fee rate
3. Record fee from result
4. Mine block, sync
5. Create UTXO with medium_priority() fee rate
6. Record fee from result
7. Mine block, sync
8. Create UTXO with high_priority() fee rate
9. Record fee from result
10. Verify high_fee > medium_fee > low_fee

**Assertions**:
- All fee rates work
- Fees scale correctly
- All transactions confirm

---

#### Test 4.3: `test_create_utxo_marks_rgb_occupied_when_requested`

**Purpose**: Verify RGB marking during UTXO creation

**Steps**:
1. Create funded wallet
2. Create empty RGB-occupied set
3. Create UTXO with mark_rgb=true, passing RGB set
4. Verify outpoint in RGB-occupied set
5. Mine block, sync
6. List UTXOs with RGB set
7. Verify new UTXO has is_rgb_occupied=true

**Assertions**:
- RGB set updated immediately
- UTXO marked correctly after confirmation

---

#### Test 4.4: `test_unlock_utxo_spends_back_to_self`

**Purpose**: Verify unlock_utxo works correctly

**Steps**:
1. Create funded wallet
2. Create specific UTXO of 50,000 sats
3. Mine block, sync
4. Record UTXO details (outpoint, amount)
5. Unlock the UTXO with medium fee rate
6. Verify UtxoOperationResult returned
7. Mine block, sync
8. List UTXOs
9. Verify original UTXO gone (spent)
10. Verify new UTXO exists with amount ≈ 50,000 - fee

**Assertions**:
- Original UTXO spent
- New UTXO created
- Amount preserved minus fee

---

#### Test 4.5: `test_unlock_utxo_removes_rgb_occupied_flag`

**Purpose**: Verify RGB flag removed when unlocking

**Steps**:
1. Create funded wallet
2. Create RGB-occupied set
3. Create UTXO with mark_rgb=true
4. Mine block, sync
5. Verify UTXO in RGB set
6. Unlock UTXO, passing RGB set
7. Verify original outpoint removed from RGB set
8. Mine block, sync
9. List UTXOs
10. Verify new UTXO not marked as RGB-occupied

**Assertions**:
- Old UTXO removed from RGB set
- New UTXO not in RGB set

---

#### Test 4.6: `test_get_recommended_fee_rates_from_esplora`

**Purpose**: Verify fee rate estimation works

**Steps**:
1. Create EsploraClient for regtest
2. Call get_recommended_fee_rates()
3. Verify returns (low, medium, high)
4. Verify low < medium < high
5. Verify all > 0
6. Verify reasonable values (e.g., < 1000 sat/vB)

**Assertions**:
- Fee rates returned
- Rates ordered correctly
- Values reasonable

---

### Module 5: Send Bitcoin Operations

**File Section**: `mod test_send_operations`

#### Test 5.1: `test_send_bitcoin_to_external_address`

**Purpose**: Verify send_bitcoin works end-to-end

**Steps**:
1. Create funded wallet with 1 BTC
2. Sync wallet
3. Generate recipient address via bitcoin-cli
4. Record initial balance
5. Send 50,000 sats to recipient
6. Verify transaction broadcasts (txid returned)
7. Mine 1 block
8. Sync wallet
9. Verify balance decreased by ~50,000 + fee
10. Use bitcoin-cli to verify recipient received funds

**Assertions**:
- Transaction broadcasts
- Balance decreases correctly
- Recipient receives funds

---

#### Test 5.2: `test_send_bitcoin_insufficient_funds_error`

**Purpose**: Verify error handling for insufficient funds

**Steps**:
1. Create wallet with only 1,000 sats
2. Sync wallet
3. Attempt to send 10,000 sats
4. Verify function returns error
5. Verify error is BuildFailed or InsufficientFunds variant
6. Verify no transaction broadcast

**Assertions**:
- Returns appropriate error
- No funds lost

---

#### Test 5.3: `test_send_bitcoin_with_custom_fee_rate`

**Purpose**: Verify custom fee rates respected

**Steps**:
1. Create funded wallet
2. Sync wallet
3. Create high_priority() fee rate
4. Send 100,000 sats with high fee rate
5. Record fee from transaction
6. Mine block, sync
7. Verify fee paid is higher than minimum
8. Verify transaction confirms

**Assertions**:
- Custom fee rate used
- Transaction confirms

---

#### Test 5.4: `test_send_bitcoin_creates_change_output`

**Purpose**: Verify change handling works

**Steps**:
1. Create wallet with single 1 BTC UTXO
2. Sync wallet
3. Send 0.1 BTC to external address
4. Mine block, sync
5. List UTXOs
6. Verify change UTXO created with ~0.9 BTC

**Assertions**:
- Change UTXO created
- Change amount correct

---

### Module 6: Manager Integration

**File Section**: `mod test_manager_integration`

#### Test 6.1: `test_manager_create_wallet_end_to_end`

**Purpose**: Verify WalletManager.create_wallet() works completely

**Steps**:
1. Create TestBitcoinEnv
2. Create regtest config
3. Create WalletManager with config
4. Call manager.create_wallet("test_wallet", "password")
5. Verify mnemonic string returned (12 words)
6. Verify wallet files created in wallet directory
7. Verify keys.json, wallet.json, descriptor.txt exist
8. Verify bitcoin.db created
9. Verify manager.is_wallet_loaded() = true
10. Get new address from manager
11. Verify valid bcrt1 address

**Assertions**:
- Mnemonic returned
- All files created
- Wallet loaded in manager
- Can perform operations

---

#### Test 6.2: `test_manager_import_wallet_from_mnemonic`

**Purpose**: Verify importing from mnemonic works

**Steps**:
1. Create TestBitcoinEnv
2. Generate known test mnemonic
3. Create WalletManager
4. Import wallet with mnemonic and password
5. Verify wallet created
6. Load wallet in new manager instance
7. Verify keys derived match expected from mnemonic

**Assertions**:
- Import succeeds
- Keys deterministic from mnemonic

---

#### Test 6.3: `test_manager_load_wallet_and_sync`

**Purpose**: Verify wallet can be loaded and synced

**Steps**:
1. Create wallet via manager
2. Get address and fund it
3. Mine block
4. Drop manager
5. Create new WalletManager instance
6. Load same wallet with password
7. Sync wallet
8. Verify balance detected

**Assertions**:
- Wallet loads successfully
- Sync works after reload
- Balance correct

---

#### Test 6.4: `test_manager_sync_wallet_updates_balance`

**Purpose**: Verify manager sync updates balance correctly

**Steps**:
1. Create wallet via manager
2. Get address from manager
3. Fund address from regtest (0.5 BTC)
4. Mine 1 block
5. Call manager.sync_wallet()
6. Verify SyncResult shows new transactions
7. Call manager.get_balance()
8. Verify balance = 0.5 BTC

**Assertions**:
- Sync detects funds
- Balance query works
- Amount correct

---

#### Test 6.5: `test_manager_get_addresses_and_new_address`

**Purpose**: Verify address operations through manager

**Steps**:
1. Create wallet via manager
2. Call manager.get_addresses(Some(5))
3. Verify returns 5 addresses
4. Call manager.get_new_address()
5. Verify returns new address
6. Call manager.get_new_address() again
7. Verify returns different address

**Assertions**:
- get_addresses works
- get_new_address increments
- All addresses valid

---

#### Test 6.6: `test_manager_create_utxo_full_flow`

**Purpose**: Verify UTXO creation through manager

**Steps**:
1. Create wallet via manager
2. Fund wallet with 1 BTC
3. Sync wallet
4. Create UTXO of 25,000 sats with mark_rgb=true
5. Verify UtxoOperationResult returned
6. Mine block
7. Sync wallet
8. Verify manager.rgb_occupied() contains outpoint

**Assertions**:
- UTXO created
- RGB tracking works through manager

---

#### Test 6.7: `test_manager_send_bitcoin_full_flow`

**Purpose**: Verify sending Bitcoin through manager

**Steps**:
1. Create wallet via manager
2. Fund with 1 BTC
3. Sync
4. Generate recipient address
5. Call manager.send_bitcoin(recipient, 100000, fee_rate)
6. Verify txid returned
7. Mine block
8. Sync
9. Verify balance decreased
10. Verify recipient received funds

**Assertions**:
- Send succeeds
- Balance updates
- Recipient receives

---

#### Test 6.8: `test_manager_multiple_wallets_isolated`

**Purpose**: Verify multiple wallets don't interfere

**Steps**:
1. Create manager
2. Create wallet1
3. Create wallet2 (new manager instance)
4. Fund wallet1
5. Sync wallet1
6. Verify wallet1 has balance
7. Load wallet2
8. Sync wallet2
9. Verify wallet2 balance = 0
10. Verify wallet1 still has correct balance

**Assertions**:
- Wallets independent
- No state pollution

---

## Test Execution & Quality Standards

### Parallel Execution Strategy

**Goal**: Tests should run in parallel when possible

**Approach**:
1. Each test uses **unique wallet names** (test_name + UUID)
2. Each test uses **separate temp directories** (via `TempDir`)
3. Bitcoin regtest state is **shared but read-mostly** (mining is coordinated)
4. Tests **avoid conflicting operations** on shared regtest state

**If State Pollution Occurs**:
```bash
# Fall back to sequential
cargo test --test bitcoin_integration_test -- --test-threads=1
```

### Panic on Failure

All tests use standard Rust assertions that panic on failure:
- `assert!()` - Basic assertions
- `assert_eq!()` - Equality checks
- `assert!(result.is_ok())` - Result checking
- `.expect("message")` - Unwrap with context

**Never use**:
- `if result.is_err() { return }` (silent failures)
- Logging errors without panicking
- Swallowing errors

### Robustness Standards

1. **Pre-flight Checks**:
   - Verify regtest running before any test
   - Panic with clear message if not running

2. **Timeouts**:
   - All wait operations have timeouts (e.g., 30 seconds)
   - Panic if timeout exceeded with clear reason

3. **Cleanup**:
   - Use `TempDir` for automatic cleanup
   - No manual cleanup required in tests
   - Bitcoin regtest state persists (intentional, shared resource)

4. **Error Messages**:
   - Use `.expect("Clear message about what failed")`
   - Include context: wallet name, operation, expected vs actual

5. **Determinism**:
   - Tests should be deterministic (same result every run)
   - Use fixed amounts, not random values
   - Use known test mnemonics where needed

6. **Independence**:
   - Each test should work in isolation
   - No dependencies between tests
   - Unique names prevent collisions

---

## Implementation Checklist

### Phase 1: Common Utilities
- [ ] Create `tests/common/mod.rs`
- [ ] Implement `TestBitcoinEnv` struct
- [ ] Implement `BitcoinRpcClient` helper
- [ ] Implement all helper functions
- [ ] Add regtest running check
- [ ] Test utilities in isolation

### Phase 2: Module 1 - Wallet Tests
- [ ] Test 1.1: SQLite persistence
- [ ] Test 1.2: Network-specific addresses
- [ ] Test 1.3: Multiple wallets isolation

### Phase 3: Module 2 - Network/Sync Tests
- [ ] Test 2.1: Esplora connection
- [ ] Test 2.2: Empty wallet sync
- [ ] Test 2.3: Detect received funds
- [ ] Test 2.4: Sync idempotent
- [ ] Test 2.5: Sync after spending

### Phase 4: Module 3 - Balance Queries
- [ ] Test 3.1: Confirmed vs unconfirmed
- [ ] Test 3.2: List UTXOs details
- [ ] Test 3.3: Get addresses
- [ ] Test 3.4: New address increments
- [ ] Test 3.5: RGB UTXO marking
- [ ] Test 3.6: Exclude spent UTXOs

### Phase 5: Module 4 - UTXO Operations
- [ ] Test 4.1: Create UTXO specific amount
- [ ] Test 4.2: Different fee rates
- [ ] Test 4.3: RGB marking on create
- [ ] Test 4.4: Unlock UTXO
- [ ] Test 4.5: RGB flag removal
- [ ] Test 4.6: Fee rate estimation

### Phase 6: Module 5 - Send Operations
- [ ] Test 5.1: Send to external
- [ ] Test 5.2: Insufficient funds error
- [ ] Test 5.3: Custom fee rate
- [ ] Test 5.4: Change output

### Phase 7: Module 6 - Manager Integration
- [ ] Test 6.1: Create wallet
- [ ] Test 6.2: Import wallet
- [ ] Test 6.3: Load and sync
- [ ] Test 6.4: Sync updates balance
- [ ] Test 6.5: Address operations
- [ ] Test 6.6: Create UTXO via manager
- [ ] Test 6.7: Send via manager
- [ ] Test 6.8: Multiple wallets

### Phase 8: Final Validation
- [ ] Run all tests in parallel
- [ ] Verify no failures
- [ ] Run all tests sequentially
- [ ] Verify no failures
- [ ] Test with fresh regtest environment
- [ ] Document any known limitations

---

## Success Criteria

✅ All 31 tests pass consistently  
✅ Tests run in parallel without conflicts  
✅ Tests panic immediately on any failure  
✅ Clear error messages for all failure modes  
✅ Proper cleanup of test resources  
✅ No manual intervention required  
✅ Tests work with fresh regtest environment  
✅ Coverage of all Bitcoin layer functionality  
✅ Manager integration fully tested  
✅ RGB UTXO tracking verified

---

## Notes

1. **Regtest State**: The regtest blockchain state persists between tests. This is intentional and allows tests to build on each other if needed. Each test should use unique wallets to avoid conflicts.

2. **Timing**: Tests that mine blocks should wait 2-3 seconds after mining to allow Electrs to index before syncing wallets.

3. **Amounts**: Use realistic amounts (not dust) to avoid issues. Minimum recommended: 0.0001 BTC (10,000 sats).

4. **Addresses**: Always verify address formats match network (bcrt1 for regtest).

5. **Fees**: Fee estimates on regtest may be minimal or fallback to defaults. Tests should verify fees > 0 but not assert exact amounts.

---

**End of Bitcoin Layer Testing Plan**

