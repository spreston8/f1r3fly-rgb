# F1r3fly-RGB Wallet Asset Issuance & Balance Query Test Plan

## Overview

This document outlines the integration test strategy for F1r3fly-RGB wallet asset issuance and balance queries (Phase 2, Days 15-22). These tests focus on **wallet-specific integration** and **state persistence**, not re-testing the underlying `f1r3fly-rgb` library functionality.

**Note**: RGB transfer operations will be covered in a separate test plan for Phase 3.

## What's Already Tested (f1r3fly-rgb library)

The following components are thoroughly tested in the `f1r3fly-rgb/tests` directory:

- ✅ **F1r3flyExecutor**: Deploy contracts, call methods, query state, error handling
- ✅ **F1r3flyRgbContract**: Lifecycle management, seal tracking, balance queries
- ✅ **F1r3flyRgbContracts**: Issue multiple contracts, list, contains, register operations

## What We Need to Test (wallet layer)

Focus on wallet-specific wrappers and state management:

1. **F1r3flyExecutorManager** - Creates executor from wallet keys/config
2. **F1r3flyContractsManager** - State persistence to disk
3. **WalletManager Integration** - Asset issuance + balance queries via manager API
4. **State Persistence** - Critical: survives wallet reload
5. **Multi-Wallet Isolation** - Each wallet has independent RGB state

## Test Structure

```
tests/
└── f1r3fly/
    ├── mod.rs                          # Shared helpers
    ├── asset_issuance_test.rs          # Asset issuance via WalletManager
    ├── balance_queries_test.rs         # Balance queries via WalletManager
    └── wallet_state_persistence_test.rs # State persistence + isolation
```

## Test Dependencies

### Required Services

- **F1r3node**: Running instance for contract deployment/queries
- **Bitcoin Regtest**: For creating/funding UTXOs
- **Esplora**: For UTXO indexing (port 3002)

### Existing Infrastructure

Tests leverage `tests/common/mod.rs` infrastructure:
- `TestBitcoinEnv` - Isolated test environment with auto-cleanup
- `BitcoinRpcClient` - Bitcoin Core interactions
- Regtest connectivity checks
- Wallet funding helpers

## Detailed Test Cases

### 1. `mod.rs` - Shared Test Helpers

```rust
pub use crate::common::{TestBitcoinEnv, BitcoinRpcClient};

/// Setup wallet with funded genesis UTXO for asset issuance
pub fn setup_wallet_with_genesis_utxo(
    env: &TestBitcoinEnv,
    wallet_name: &str,
    password: &str,
) -> Result<(WalletManager, String), Box<dyn std::error::Error>>;

/// Check if F1r3node is available
pub fn check_f1r3node_available() -> bool;
```

**Helper Implementation:**
1. Create wallet via `WalletManager`
2. Fund with Bitcoin from regtest
3. Sync wallet
4. Create UTXO for genesis seal via self-send
5. Return manager + genesis UTXO string (format: "txid:vout")

---

### 2. `asset_issuance_test.rs` - Asset Issuance Tests

#### Test: `test_issue_single_asset`
- **Setup**: Create wallet with genesis UTXO
- **Action**: Issue asset via `manager.issue_asset()`
- **Verify**: 
  - AssetInfo returned with correct metadata
  - State file created with contract metadata

#### Test: `test_issue_multiple_assets`
- **Setup**: Create wallet with 2 genesis UTXOs
- **Action**: Issue 2 different assets
- **Verify**: 
  - Both assets have unique contract IDs
  - `list_assets()` returns both
  - Both tracked in state file

#### Test: `test_list_assets_empty`
- **Setup**: Create wallet (no assets issued)
- **Action**: Call `manager.list_assets()`
- **Verify**: Returns empty vector

#### Test: `test_get_asset_info`
- **Setup**: Issue an asset
- **Action**: Call `manager.get_asset_info(contract_id)`
- **Verify**: 
  - Returns correct metadata (ticker, name, supply, precision)
  - Matches issued parameters
  - Genesis seal information correct

#### Test: `test_issue_asset_invalid_utxo_format`
- **Setup**: Create wallet
- **Action**: Try to issue with invalid UTXO string (e.g., "invalid", "abc", "123:xyz")
- **Verify**: Returns appropriate error

#### Test: `test_issue_asset_utxo_not_owned`
- **Setup**: Create wallet
- **Action**: Try to issue with a UTXO not in wallet (fake txid)
- **Verify**: Returns error indicating UTXO not found

#### Test: `test_get_asset_info_not_found`
- **Setup**: Create wallet with one asset
- **Action**: Try to get info for fake contract ID
- **Verify**: Returns error indicating contract not found

**Total: 7 tests**

---

### 3. `balance_queries_test.rs` - Balance Query Tests

#### Test: `test_balance_after_issuance`
- **Setup**: Issue asset with genesis UTXO
- **Action**: Query balance via `manager.get_rgb_balance()`
- **Verify**: 
  - Genesis UTXO holds full supply
  - Total balance equals issued supply
  - UTXO details correct (txid, vout, amount)

#### Test: `test_balance_empty_wallet`
- **Setup**: Create wallet with no RGB assets
- **Action**: Query balance via `manager.get_rgb_balance()`
- **Verify**: Returns empty vector

#### Test: `test_balance_multiple_assets`
- **Setup**: Issue 2 assets with different genesis UTXOs
- **Action**: Query balance for all assets
- **Verify**: 
  - Returns 2 `AssetBalance` entries
  - Each has correct ticker, name, contract_id
  - Each has correct total balance

#### Test: `test_asset_specific_balance`
- **Setup**: Issue 2 assets
- **Action**: Query balance for specific contract via `manager.get_asset_balance(contract_id)`
- **Verify**: 
  - Returns only that asset's balance
  - Does not include other assets

#### Test: `test_get_occupied_utxos`
- **Setup**: Issue 2 assets with different genesis UTXOs
- **Action**: Query occupied UTXOs via `manager.get_occupied_utxos()`
- **Verify**: 
  - Returns 2 UTXOs
  - Each UTXO has correct contract_id
  - Each UTXO has correct amount

#### Test: `test_balance_query_unknown_contract`
- **Setup**: Issue one asset
- **Action**: Try to query balance for fake contract ID
- **Verify**: Returns error indicating contract not found

**Total: 6 tests**

---

### 4. `wallet_state_persistence_test.rs` - State Persistence & Isolation

#### Test: `test_state_persists_across_wallet_reload` ⭐ **CRITICAL**
**Complete End-to-End Workflow:**

1. **Create & Issue**: Create wallet, issue asset "USD"
2. **Verify Initial**: Query balance, verify genesis UTXO has full supply
3. **Close Wallet**: Drop `WalletManager` instance
4. **Reload Wallet**: Create new `WalletManager`, load same wallet
5. **Verify After Reload**: 
   - `list_assets()` shows "USD"
   - `get_asset_info()` returns correct metadata
   - `get_rgb_balance()` shows same balance
6. **Issue Second Asset**: Issue "EUR" after reload
7. **Verify Both Assets**: 
   - Both assets tracked correctly
   - Both have correct balances
   - State file contains both

**Why Critical**: This test ensures users don't lose their RGB assets when closing/reopening the wallet.

#### Test: `test_multiple_wallets_isolated_rgb_state`
- **Step 1**: Create wallet A, issue asset "USD"
- **Step 2**: Create wallet B, issue asset "EUR"
- **Step 3**: Verify wallet A only has "USD" (not "EUR")
- **Step 4**: Verify wallet B only has "EUR" (not "USD")
- **Step 5**: Drop both managers
- **Step 6**: Reload both wallets
- **Step 7**: Verify state still isolated after reload

#### Test: `test_contracts_manager_state_file_location`
- **Setup**: Create wallet, issue asset
- **Verify**: 
  - `f1r3fly_contracts.json` exists in wallet directory
  - File contains expected structure:
    - `derivation_index`
    - `contracts_metadata`
    - `genesis_utxos`
    - `tracker_state`
  - JSON is valid and parseable

#### Test: `test_genesis_utxo_tracking`
- **Setup**: Issue 2 assets with different genesis UTXOs
- **Action**: Query `contracts_manager.genesis_utxos()`
- **Verify**: 
  - Returns 2 entries
  - Each genesis UTXO maps to correct contract ID
  - `GenesisUtxoInfo` has txid, vout, block_height

**Total: 4 tests**

---

## Test Infrastructure Details

### Setup Helper Implementation

```rust
pub fn setup_wallet_with_genesis_utxo(
    env: &TestBitcoinEnv,
    wallet_name: &str,
    password: &str,
) -> Result<(WalletManager, String), Box<dyn std::error::Error>> {
    // 1. Create wallet
    let mut manager = WalletManager::new(env.config().clone())?;
    manager.create_wallet(wallet_name, password)?;
    manager.load_wallet(wallet_name, password)?;
    
    // 2. Get first address
    let addresses = manager.get_bitcoin_addresses(1)?;
    let address = &addresses[0].address;
    
    // 3. Fund wallet
    let txid = env.fund_address(address, 1.0)?;
    env.wait_for_confirmation(&txid, 1)?;
    
    // 4. Sync wallet
    manager.sync_wallet()?;
    
    // 5. Create UTXO for genesis seal (self-send to create specific UTXO)
    let genesis_txid = manager.create_utxo(0.01, None)?;
    env.wait_for_confirmation(&genesis_txid, 1)?;
    manager.sync_wallet()?;
    
    // 6. Format genesis UTXO as "txid:vout"
    // Assume vout=0 for simplicity, or scan UTXOs to find it
    let genesis_utxo = format!("{}:0", genesis_txid);
    
    Ok((manager, genesis_utxo))
}
```

### F1r3node Availability Check

```rust
pub fn check_f1r3node_available() -> bool {
    use std::net::TcpStream;
    use std::time::Duration;
    
    // Try to connect to F1r3node gRPC port
    let host = std::env::var("FIREFLY_GRPC_HOST").unwrap_or("localhost".to_string());
    let port = std::env::var("FIREFLY_GRPC_PORT").unwrap_or("40401".to_string());
    let addr = format!("{}:{}", host, port);
    
    TcpStream::connect_timeout(
        &addr.parse().unwrap(),
        Duration::from_secs(2),
    ).is_ok()
}
```

## Test Execution Strategy

### Running Tests

```bash
# Run all F1r3fly integration tests
cargo test f1r3fly::

# Run specific test file
cargo test f1r3fly::asset_issuance_test

# Run with output
cargo test f1r3fly:: -- --nocapture

# Run single test
cargo test test_state_persists_across_wallet_reload -- --nocapture
```

### Prerequisites

1. **Start Regtest Environment**:
   ```bash
   ./scripts/start-regtest.sh
   ```

2. **Start F1r3node**:
   - Ensure F1r3node is running and accessible
   - Set environment variables:
     ```bash
     export FIREFLY_GRPC_HOST=localhost
     export FIREFLY_GRPC_PORT=40401
     export FIREFLY_HTTP_PORT=40403
     export FIREFLY_PRIVATE_KEY=<your_key>
     ```

3. **Run Tests**:
   ```bash
   cargo test f1r3fly::
   ```

### Environment Variables

Required for F1r3fly-RGB operations:
- `FIREFLY_GRPC_HOST` - F1r3node gRPC host (default: localhost)
- `FIREFLY_GRPC_PORT` - F1r3node gRPC port (default: 40401)
- `FIREFLY_HTTP_PORT` - F1r3node HTTP port (default: 40403)
- `FIREFLY_PRIVATE_KEY` - F1r3fly private key for signing

Optional:
- `BITCOIN_DATADIR` - Bitcoin Core data directory (default: project_root/.bitcoin)

## Error Handling Test Coverage

### Expected Error Scenarios

1. **Invalid UTXO Format**
   - Input: `"invalid"`, `"abc"`, `"123:xyz"`
   - Expected: `AssetError::InvalidUtxoFormat`

2. **UTXO Not Found**
   - Input: Valid format but UTXO not in wallet
   - Expected: `AssetError::UtxoNotFound`

3. **Contract Not Found**
   - Input: Non-existent contract ID
   - Expected: `AssetError::ContractNotFound`

4. **F1r3node Unreachable** (graceful degradation)
   - Scenario: F1r3node down after issuing assets
   - Expected: Can still list assets from state file
   - Note: This is tested implicitly by state persistence tests

## Test Summary

| Test File | Test Count | Focus |
|-----------|------------|-------|
| `asset_issuance_test.rs` | 7 | Asset creation, metadata, error paths |
| `balance_queries_test.rs` | 6 | Balance calculations, UTXO mapping |
| `wallet_state_persistence_test.rs` | 4 | State persistence, isolation |
| **Total** | **17** | **Complete wallet integration** |

## Success Criteria

All tests must pass to ensure:

1. ✅ Assets can be issued via WalletManager
2. ✅ Asset metadata correctly extracted from F1r3node
3. ✅ Balances accurately calculated from contract state
4. ✅ **State persists across wallet reload** (critical!)
5. ✅ Multiple wallets have isolated RGB state
6. ✅ Error paths handled gracefully
7. ✅ Genesis UTXOs correctly tracked

## Future Enhancements

Phase 3 (RGB Transfers) will add:
- Transfer operations tests
- Balance changes after transfer
- Multi-hop transfer scenarios
- UTXO selection for transfers

These will be addressed in a separate test plan.

