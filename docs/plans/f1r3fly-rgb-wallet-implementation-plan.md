# F1r3fly-RGB Wallet Implementation Plan

## Executive Summary

Implementation plan for `f1r3fly-rgb-wallet` - a command-line wallet that combines F1r3fly contract execution with Bitcoin UTXO management for RGB smart contracts.

**Timeline**: 7-8 weeks  
**Deliverable**: Production-ready CLI wallet matching `rgb_transfer_balance_test.rs` functionality

---

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Configuration](#configuration)
3. [Key Management Strategy](#key-management-strategy)
4. [Project Structure](#project-structure)
5. [Implementation Phases](#implementation-phases)
6. [CLI Command Specification](#cli-command-specification)
7. [Testing Strategy](#testing-strategy)
8. [Dependencies](#dependencies)

---

## Architecture Overview

### High-Level Design

```
f1r3fly-rgb-wallet (CLI)
├─ WalletManager (orchestrator)
├─ Storage (keys + persistence)
├─ Bitcoin Layer (BDK)
│  └─ UTXO management, sync, transactions
├─ F1r3fly Layer (f1r3fly-rgb)
│  └─ F1r3flyRgbContracts, executor, consignments
└─ CLI (clap commands)
```

### Key Design Decisions

1. **CLI First**: Pure command-line interface (no interactive prompts)
2. **Single Wallet Mode**: One active wallet at a time (initially)
3. **BDK for Bitcoin**: Industry-standard Bitcoin wallet library
4. **node_cli for F1r3node**: Same connection pattern as f1r3fly-rgb
5. **JSON Persistence**: Human-readable wallet state
6. **Network Support**: Regtest (primary), Signet, Testnet3, Mainnet

---

## Configuration

### Global Config Structure

```json
{
  "f1r3node": {
    "host": "localhost",
    "grpc_port": 40401,
    "http_port": 40402
  },
  "bitcoin": {
    "network": "regtest",
    "esplora_url": "http://localhost:3002"
  }
}
```

**Location**: `~/.f1r3fly-rgb-wallet/config.json`

### Network-Specific Defaults

**Regtest:**
```json
{
  "f1r3node": {"host": "localhost", "grpc_port": 40401, "http_port": 40402},
  "bitcoin": {"network": "regtest", "esplora_url": "http://localhost:3002"}
}
```

**Signet:**
```json
{
  "f1r3node": {"host": "localhost", "grpc_port": 40401, "http_port": 40402},
  "bitcoin": {"network": "signet", "esplora_url": "https://mempool.space/signet/api"}
}
```

**Testnet:**
```json
{
  "f1r3node": {"host": "localhost", "grpc_port": 40401, "http_port": 40402},
  "bitcoin": {"network": "testnet", "esplora_url": "https://mempool.space/testnet/api"}
}
```

**Mainnet:**
```json
{
  "f1r3node": {"host": "localhost", "grpc_port": 40401, "http_port": 40402},
  "bitcoin": {"network": "mainnet", "esplora_url": "https://mempool.space/api"}
}
```

### Configuration Priority

```
CLI args > Environment vars > config.json > Network defaults
```

### CLI Override Examples

```bash
# Use config.json defaults
f1r3fly-rgb-wallet --wallet my_wallet get-balance

# Override F1r3node host
f1r3fly-rgb-wallet --wallet my_wallet --f1r3node-host 192.168.1.100 get-balance

# Override via environment
F1R3NODE_HOST=192.168.1.100 f1r3fly-rgb-wallet --wallet my_wallet get-balance

# Override network
f1r3fly-rgb-wallet --wallet my_wallet --network signet sync
```

---

## Key Management Strategy

### Dual Key System

Users need **two distinct key systems** derived from a single mnemonic:

1. **Bitcoin Keys (BIP39/BIP32/BIP84)**
   - Purpose: Bitcoin UTXO ownership, transaction signing
   - Derivation: `m/84'/0'/0'/0/*` (BIP84 native segwit)
   - Usage: Receive Bitcoin, create UTXOs, sign witness transactions

2. **F1r3fly Key (Single secp256k1 keypair)**
   - Purpose: F1r3node `insertSigned` authentication
   - Derivation: `m/1337'/0'/0'/0/0` (custom path, single key)
   - Usage: Sign F1r3node operations, authorize contract execution

### Unified Derivation from Single Mnemonic

```
BIP39 Mnemonic (12 or 24 words)
├─ Bitcoin Keys: m/84'/0'/0'/0/*     (BIP84 hierarchical)
└─ F1r3fly Key:  m/1337'/0'/0'/0/0   (Single derived key)
```

**Benefits:**
- ✅ One mnemonic backs up everything
- ✅ Deterministic and recoverable
- ✅ Clean separation (different derivation paths)

### Wallet Data Directory

```
~/.f1r3fly-rgb-wallet/
├── config.json                    # Global config (f1r3node, esplora, network)
└── wallets/
    └── my_wallet/
        ├── wallet.json            # Metadata (name, creation date)
        ├── keys.json              # Encrypted: mnemonic, f1r3fly keys
        ├── descriptor.txt         # Bitcoin descriptor (BDK)
        ├── bitcoin.db             # BDK SQLite database
        ├── f1r3fly_contracts.json # F1r3flyRgbContracts state
        └── bitcoin_tracker.bin    # BitcoinAnchorTracker state
```

### Wallet Keys Structure (`keys.json`)

```json
{
  "encrypted_mnemonic": "...",
  "bitcoin_descriptor": "wpkh([fingerprint/84'/0'/0']xpub.../0/*)",
  "f1r3fly_public_key": "02abc123...",
  "encrypted_f1r3fly_private_key": "..."
}
```

---

## Project Structure

```
f1r3fly-rgb-wallet/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── src/
│   ├── lib.rs                     # Public API
│   ├── main.rs                    # CLI binary entry point
│   │
│   ├── manager.rs                 # WalletManager (orchestrator)
│   ├── config.rs                  # Configuration system
│   ├── error.rs                   # Error types
│   ├── types.rs                   # Shared types
│   │
│   ├── storage/
│   │   ├── mod.rs
│   │   ├── keys.rs                # BIP39/BIP32 key derivation
│   │   ├── models.rs              # WalletInfo, WalletKeys
│   │   └── file_system.rs         # Disk persistence
│   │
│   ├── bitcoin/
│   │   ├── mod.rs
│   │   ├── wallet.rs              # BDK wallet wrapper
│   │   ├── network.rs             # Esplora client
│   │   ├── sync.rs                # Blockchain sync
│   │   ├── balance.rs             # UTXO queries
│   │   ├── utxo.rs                # UTXO creation/management
│   │   └── send.rs                # Bitcoin transactions
│   │
│   ├── f1r3fly/
│   │   ├── mod.rs
│   │   ├── executor.rs            # F1r3flyExecutor wrapper
│   │   ├── contracts.rs           # F1r3flyRgbContracts manager
│   │   ├── asset.rs               # Issue assets
│   │   ├── transfer.rs            # Send transfers
│   │   ├── consignment.rs         # Export/accept consignments
│   │   ├── invoice.rs             # Generate invoices
│   │   └── balance.rs             # Query RGB balances
│   │
│   └── cli/
│       ├── mod.rs
│       ├── args.rs                # Clap argument definitions
│       └── commands/
│           ├── mod.rs
│           ├── wallet.rs          # Wallet management commands
│           ├── bitcoin.rs         # Bitcoin commands
│           ├── rgb.rs             # RGB commands
│           └── config.rs          # Config commands
│
└── tests/
    ├── common.rs                  # Test utilities
    └── f1r3fly_transfer_balance_test.rs  # Full lifecycle test
```

---

## Implementation Phases

### Phase 1: Foundation (Week 1-2)

**Goal**: Basic wallet lifecycle and Bitcoin operations

#### Week 1: Configuration & Key Management

**Tasks:**
- [ ] **Day 1-2: Project Setup**
  - [ ] Create module structure in `src/`
  - [ ] Update `Cargo.toml` with all dependencies
  - [ ] Define error types in `error.rs`
  - [ ] Define config types in `config.rs`
  - [ ] Create `.gitignore` for wallet data

- [ ] **Day 3-4: Key Management (`storage/keys.rs`)**
  - [ ] Implement `generate_mnemonic()` (BIP39 12-word)
  - [ ] Implement `derive_bitcoin_keys(mnemonic, network)` (BIP32 xprv at m/84')
  - [ ] Implement `derive_f1r3fly_key(mnemonic)` (secp256k1 at m/1337'/0'/0'/0/0)
  - [ ] Implement key encryption/decryption (AES-GCM with password)
  - [ ] Write unit tests for all derivation functions

- [ ] **Day 5-6: Storage & Config**
  - [ ] Implement `config.rs`:
    - [ ] `GlobalConfig` struct
    - [ ] `NetworkType` enum (Regtest/Signet/Testnet/Mainnet)
    - [ ] `load_config()` with CLI/env/file priority
    - [ ] `save_config()`
    - [ ] Network-specific defaults
  - [ ] Implement `storage/models.rs`:
    - [ ] `WalletInfo` struct
    - [ ] `WalletKeys` struct (Bitcoin + F1r3fly keys)
    - [ ] `WalletMetadata` struct
  - [ ] Implement `storage/file_system.rs`:
    - [ ] `create_wallet_directory()`
    - [ ] `save_wallet()`
    - [ ] `load_wallet()`
    - [ ] `list_wallets()`
    - [ ] `delete_wallet()`
  - [ ] Write tests for storage operations

- [ ] **Day 7: CLI Foundation**
  - [ ] Setup `cli/args.rs` with `clap`:
    - [ ] Global args: `--wallet`, `--network`, `--f1r3node-host`, etc.
    - [ ] Subcommands structure
  - [ ] Implement `cli/commands/config.rs`:
    - [ ] `config init` command
  - [ ] Implement `cli/commands/wallet.rs`:
    - [ ] `create-wallet <name>` command
    - [ ] `import-wallet <name> --mnemonic "<words>"` command
    - [ ] `list-wallets` command
  - [ ] Implement `main.rs` CLI entry point
  - [ ] Test end-to-end wallet creation

**Success Criteria:**
- ✅ Can create wallet from generated mnemonic
- ✅ Can import wallet from existing mnemonic
- ✅ Can list all wallets
- ✅ Keys properly derived (Bitcoin + F1r3fly)
- ✅ Encrypted keys persisted to disk

**Testing:**
```bash
f1r3fly-rgb-wallet config init --network regtest
f1r3fly-rgb-wallet create-wallet test1
f1r3fly-rgb-wallet list-wallets
```

---

#### Week 2: Bitcoin Layer (BDK Integration)

**Tasks:**
- [ ] **Day 8-9: BDK Wallet Setup**
  - [ ] Implement `bitcoin/wallet.rs`:
    - [ ] `BitcoinWallet` struct wrapping `bdk::Wallet`
    - [ ] `new()` - Initialize BDK wallet from descriptor
    - [ ] Configure SQLite persistence
    - [ ] Network-specific parameters
  - [ ] Implement `bitcoin/network.rs`:
    - [ ] `EsploraClient` wrapper using `bdk_esplora`
    - [ ] Network-specific endpoint configuration
    - [ ] Error handling for network requests

- [ ] **Day 10: Sync Operations**
  - [ ] Implement `bitcoin/sync.rs`:
    - [ ] `sync_wallet()` - Full blockchain sync
    - [ ] Return `SyncResult` (height, new addresses, new txs)
    - [ ] Progress logging

- [ ] **Day 11: Balance Queries**
  - [ ] Implement `bitcoin/balance.rs`:
    - [ ] `get_balance()` - Confirmed + unconfirmed
    - [ ] `list_utxos()` - All wallet UTXOs with details
    - [ ] `mark_rgb_occupied()` - Mark UTXOs holding RGB
    - [ ] `get_addresses()` - List wallet addresses

- [ ] **Day 12-13: UTXO Operations**
  - [ ] Implement `bitcoin/utxo.rs`:
    - [ ] `create_utxo()` - Self-send to create specific UTXO
    - [ ] `unlock_utxo()` - Spend UTXO back to self
    - [ ] Handle fee rate calculation
  - [ ] Implement `bitcoin/send.rs`:
    - [ ] `send_bitcoin()` - Send to external address
    - [ ] PSBT construction with BDK
    - [ ] Transaction signing
    - [ ] Broadcast transaction

- [ ] **Day 14: Manager Integration**
  - [ ] Implement `manager.rs`:
    - [ ] `WalletManager` struct
    - [ ] `new()` - Load config
    - [ ] `create_wallet(name)` - Generate keys, init BDK wallet
    - [ ] `import_wallet(name, mnemonic)` - Import and init
    - [ ] `load_wallet(name)` - Load existing wallet
    - [ ] Delegate to Bitcoin layer methods:
      - [ ] `sync_wallet()`
      - [ ] `get_balance()`
      - [ ] `get_addresses()`
      - [ ] `create_utxo()`
      - [ ] `send_bitcoin()`

- [ ] **Day 14: CLI Bitcoin Commands**
  - [ ] Implement `cli/commands/bitcoin.rs`:
    - [ ] `sync` command
    - [ ] `get-balance` command
    - [ ] `get-addresses` command
    - [ ] `create-utxo` command
    - [ ] `send-bitcoin` command
  - [ ] Wire up to manager in handlers

**Success Criteria:**
- ✅ Can sync wallet with regtest Bitcoin node
- ✅ Can query Bitcoin balance (confirmed/unconfirmed)
- ✅ Can list addresses
- ✅ Can create UTXO via self-send
- ✅ Can send Bitcoin to external address
- ✅ BDK state persists to SQLite

**Testing:**
```bash
# Mine blocks to wallet address for testing
f1r3fly-rgb-wallet --wallet test1 sync
f1r3fly-rgb-wallet --wallet test1 get-balance
f1r3fly-rgb-wallet --wallet test1 get-addresses
f1r3fly-rgb-wallet --wallet test1 create-utxo --amount 0.0003
f1r3fly-rgb-wallet --wallet test1 send-bitcoin --to <addr> --amount 10000
```

---

### Phase 2: F1r3fly-RGB Integration (Week 3-4)

**Goal**: RGB asset issuance and state tracking

#### Week 3: F1r3fly Foundation

**Tasks:**
- [ ] **Day 15-16: F1r3fly Executor**
  - [ ] Implement `f1r3fly/executor.rs`:
    - [ ] `F1r3flyExecutorManager` struct
    - [ ] Initialize `node_cli::Connection` from config
    - [ ] Load F1r3fly keys from wallet
    - [ ] `create_executor()` - Create `F1r3flyExecutor` instance
    - [ ] Handle connection errors gracefully

- [ ] **Day 17: Contracts Management**
  - [ ] Implement `f1r3fly/contracts.rs`:
    - [ ] `F1r3flyContractsManager` struct
    - [ ] Initialize `F1r3flyRgbContracts` from f1r3fly-rgb
    - [ ] Initialize `BitcoinAnchorTracker`
    - [ ] `save_state()` - Persist to `f1r3fly_contracts.json`
    - [ ] `load_state()` - Load from disk
    - [ ] Track deployed contracts

- [ ] **Day 18-19: Asset Issuance**
  - [ ] Implement `f1r3fly/asset.rs`:
    - [ ] `issue_asset()`:
      - [ ] Verify genesis UTXO exists in Bitcoin wallet
      - [ ] Deploy contract via `F1r3flyRgbContracts::issue()`
      - [ ] Register genesis seal in tracker
      - [ ] Persist updated contracts state
      - [ ] Return `AssetInfo` response
    - [ ] `list_assets()` - Query all contracts
    - [ ] `get_asset_info()` - Get contract metadata
  - [ ] Integrate with `manager.rs`

- [ ] **Day 20-21: Balance Queries**
  - [ ] Implement `f1r3fly/balance.rs`:
    - [ ] `get_rgb_balance()`:
      - [ ] Query contract state for each asset
      - [ ] Map Bitcoin UTXOs to RGB seals
      - [ ] Calculate available vs spent tokens
      - [ ] Return per-contract balances
    - [ ] `get_occupied_utxos()` - List UTXOs with RGB
    - [ ] Integration with Bitcoin balance display

**Success Criteria:**
- ✅ F1r3flyExecutor connects to F1r3node
- ✅ Can issue RGB asset with F1r3node deployment
- ✅ Genesis seal properly registered
- ✅ Contracts state persists to JSON
- ✅ Can query RGB balance per contract

---

#### Week 4: RGB CLI Commands

**Tasks:**
- [ ] **Day 22: CLI Integration**
  - [ ] Implement `cli/commands/rgb.rs`:
    - [ ] `issue-asset` command
    - [ ] `list-assets` command
    - [ ] `rgb-balance` command
    - [ ] `get-contract-info` command
  - [ ] Wire up to manager

- [ ] **Day 23-24: Testing & Refinement**
  - [ ] End-to-end issuance test
  - [ ] Test with fresh F1r3node
  - [ ] Test state persistence across wallet reloads
  - [ ] Fix any bugs found

- [ ] **Day 25-26: Bitcoin/RGB Integration**
  - [ ] Mark occupied UTXOs in `get-balance` output
  - [ ] Prevent spending RGB-occupied UTXOs in `send-bitcoin`
  - [ ] Add warnings for UTXO operations

- [ ] **Day 27-28: Documentation**
  - [ ] Update README with issuance examples
  - [ ] Document F1r3fly key system
  - [ ] Add troubleshooting section

**Success Criteria:**
- ✅ `issue-asset` deploys contract to F1r3node
- ✅ `list-assets` shows issued assets
- ✅ `rgb-balance` displays token balances
- ✅ RGB-occupied UTXOs clearly marked
- ✅ State persists across operations

**Testing:**
```bash
# Create UTXO for genesis
f1r3fly-rgb-wallet --wallet test1 create-utxo --amount 0.0003
# Wait for confirmation...

# Issue asset
f1r3fly-rgb-wallet --wallet test1 issue-asset \
  --ticker TEST \
  --name TestToken \
  --supply 1000 \
  --precision 0 \
  --genesis-utxo <txid:vout>

# Check results
f1r3fly-rgb-wallet --wallet test1 list-assets
f1r3fly-rgb-wallet --wallet test1 rgb-balance
```

---

### Phase 3: Transfers & Consignments (Week 5-6)

**Goal**: Complete RGB transfer flow

#### Week 5: Invoice & Transfer

**Tasks:**
- [ ] **Day 29-30: Invoice Generation**
  - [ ] Implement `f1r3fly/invoice.rs`:
    - [ ] `generate_invoice()`:
      - [ ] Create blinded seal for recipient
      - [ ] Select UTXO for auth token (or use specified)
      - [ ] Generate RGB invoice string (Base64-encoded compact format)
      - [ ] Format: `"rgb:<base64_encoded_data>"` containing contract ID, blinded seal, amount
      - [ ] Validate invoice format
    - [ ] `parse_invoice()` - Parse invoice string (decode Base64)
    - [ ] Integration tests
  - [ ] **Note**: Invoice is a string (~110 chars), NOT a file - can be copy/pasted, QR coded

- [ ] **Day 31-33: Transfer Operations**
  - [ ] Implement `f1r3fly/transfer.rs`:
    - [ ] `send_transfer()`:
      - [ ] Parse RGB invoice (decode Base64 string)
      - [ ] Execute F1r3fly contract method via `F1r3flyRgbContract::call_method()`
      - [ ] Build Bitcoin witness transaction (using BDK)
      - [ ] Embed Tapret commitment using `f1r3fly_rgb::create_tapret_anchor()`
      - [ ] Create `F1r3flyConsignment` using existing `F1r3flyConsignment::new()`
      - [ ] Serialize consignment to JSON bytes via `consignment.to_bytes()`
      - [ ] Register change seals in tracker
      - [ ] Broadcast witness transaction
      - [ ] Save consignment JSON to disk (~5-20 KB, much smaller than traditional RGB)
      - [ ] Return transfer response with consignment path
    - [ ] Handle errors at each step
  - [ ] **Note**: `F1r3flyConsignment` already implemented in `f1r3fly-rgb/src/consignment.rs`

- [ ] **Day 34-35: Consignment Operations**
  - [ ] Implement `f1r3fly/consignment.rs`:
    - [ ] `export_genesis()`:
      - [ ] Load contract from manager
      - [ ] Create genesis consignment using `F1r3flyConsignment::new()`
      - [ ] Serialize to JSON via `to_bytes()`
      - [ ] Save to disk as `.json` file
    - [ ] `accept_consignment()`:
      - [ ] Load consignment JSON from file
      - [ ] Deserialize using `F1r3flyConsignment::from_bytes()`
      - [ ] Validate using `F1r3flyConsignment::validate()`:
        - [ ] Check F1r3node block finalization (query remote state)
        - [ ] Verify Tapret proof (Bitcoin anchor validation)
        - [ ] Validate seals (UTXO ownership)
      - [ ] Import into local state (update `f1r3fly_contracts.json`)
      - [ ] Register received seals in tracker
      - [ ] Persist updated state
  - [ ] **Note**: Consignment format is lightweight JSON (~5-20 KB), not traditional RGB files (MBs)

**Success Criteria:**
- ✅ Generate RGB invoice with blinded seal
- ✅ Send transfer with F1r3flyConsignment
- ✅ Consignment cryptographically valid
- ✅ Change seals properly registered
- ✅ Witness transaction broadcasts successfully

---

#### Week 6: CLI & Testing

**Tasks:**
- [ ] **Day 36-37: CLI Transfer Commands**
  - [ ] Implement commands in `cli/commands/rgb.rs`:
    - [ ] `generate-invoice` command
    - [ ] `send-transfer` command
    - [ ] `accept-consignment` command
    - [ ] `export-genesis` command
  - [ ] Wire up to manager handlers

- [ ] **Day 38-40: End-to-End Testing**
  - [ ] Manual test: Full transfer flow
  - [ ] Two wallet test (sender + recipient)
  - [ ] Verify balances update correctly
  - [ ] Test consignment file portability
  - [ ] Test genesis export/import

- [ ] **Day 41-42: Bug Fixes & Refinement**
  - [ ] Fix any transfer issues
  - [ ] Improve error messages
  - [ ] Add progress indicators
  - [ ] Handle edge cases

**Success Criteria:**
- ✅ Full transfer flow works end-to-end
- ✅ Sender balance decreases correctly
- ✅ Recipient balance increases after confirmation
- ✅ Consignments portable between wallets
- ✅ Genesis consignments work

**Testing:**
```bash
# Recipient: Generate invoice
f1r3fly-rgb-wallet --wallet recipient generate-invoice \
  --contract-id <contract_id> \
  --amount 100 \
  > invoice.txt

# Sender: Send transfer
f1r3fly-rgb-wallet --wallet sender send-transfer \
  --invoice $(cat invoice.txt)

# Wait for confirmation...

# Recipient: Accept consignment
f1r3fly-rgb-wallet --wallet recipient accept-consignment \
  --file ~/.f1r3fly-rgb-wallet/wallets/sender/consignments/<file>.json

# Verify balances
f1r3fly-rgb-wallet --wallet sender rgb-balance
f1r3fly-rgb-wallet --wallet recipient rgb-balance
```

---

### Phase 4: Testing & Hardening (Week 7-8)

**Goal**: Production-ready with comprehensive tests

#### Week 7: Integration Testing

**Tasks:**
- [ ] **Day 43-45: Port RGB Transfer Test**
  - [ ] Create `tests/common.rs`:
    - [ ] `TestEnvironment` struct with cleanup
    - [ ] `TestNetworkConfig` for regtest/signet
    - [ ] `wait_for_confirmation()` helper
    - [ ] `mine_regtest_block()` helper
    - [ ] `print_rgb_balance()` helper
  - [ ] Create `tests/f1r3fly_transfer_balance_test.rs`:
    - [ ] Port test structure from `wallet/tests/rgb_transfer_balance_test.rs`
    - [ ] Phase 0: Environment setup
    - [ ] Phase 1: Asset issuance
    - [ ] Phase 2: Transfer execution
    - [ ] Phase 3: Balance verification
    - [ ] Critical test points for balance persistence

- [ ] **Day 46-47: Run Full Test Suite**
  - [ ] Setup regtest environment
  - [ ] Run `f1r3fly_transfer_balance_test.rs`
  - [ ] Debug any failures
  - [ ] Ensure balance persists after sync
  - [ ] Verify all assertions pass

- [ ] **Day 48-49: Edge Case Tests**
  - [ ] Multiple transfers in sequence
  - [ ] Multiple assets per wallet
  - [ ] Change seal tracking accuracy
  - [ ] UTXO occupation logic
  - [ ] State persistence across wallet reloads
  - [ ] Invalid consignment rejection
  - [ ] Network error handling
  - [ ] F1r3node connection failures

**Success Criteria:**
- ✅ `f1r3fly_transfer_balance_test.rs` passes completely
- ✅ All edge case tests pass
- ✅ No balance discrepancies
- ✅ State persists correctly
- ✅ Errors handled gracefully

---

#### Week 8: Documentation & Polish

**Tasks:**
- [ ] **Day 50-51: Documentation**
  - [ ] Update main `README.md`:
    - [ ] Installation instructions
    - [ ] Quick start guide
    - [ ] Full command reference
    - [ ] Configuration guide
  - [ ] Create `docs/user-guide.md`:
    - [ ] Wallet management
    - [ ] Bitcoin operations
    - [ ] RGB operations
    - [ ] Transfer workflows
  - [ ] Create `docs/architecture.md`:
    - [ ] System design
    - [ ] Key management
    - [ ] F1r3fly integration
    - [ ] State persistence
  - [ ] Create `docs/troubleshooting.md`:
    - [ ] Common errors
    - [ ] F1r3node connection issues
    - [ ] Bitcoin sync issues
    - [ ] RGB transfer issues

- [ ] **Day 52-53: Polish & UX**
  - [ ] Improve CLI help messages
  - [ ] Add progress indicators for long operations
  - [ ] Better error messages with suggestions
  - [ ] Add `--verbose` flag for debugging
  - [ ] Colorized output (optional)

- [ ] **Day 54-56: Final Testing**
  - [ ] Fresh regtest environment test
  - [ ] Test on signet (if available)
  - [ ] Performance testing
  - [ ] Memory leak checks
  - [ ] Code cleanup and formatting

**Success Criteria:**
- ✅ Comprehensive documentation
- ✅ Clear error messages
- ✅ Good user experience
- ✅ Production-ready code quality

---

## CLI Command Specification

### Global Arguments (All Commands)

```bash
--wallet <name>                # Required for most commands
--network <net>                # Override: regtest/signet/testnet/mainnet
--f1r3node-host <host>         # Override F1r3node host
--f1r3node-grpc-port <port>    # Override F1r3node gRPC port
--f1r3node-http-port <port>    # Override F1r3node HTTP port
--esplora-url <url>            # Override Esplora URL
--data-dir <path>              # Override wallet data directory
--log-level <level>            # Set log level: debug/info/warn/error
```

### Configuration Commands

```bash
# Initialize config file with network defaults
f1r3fly-rgb-wallet config init [--network <net>]

# Show current config
f1r3fly-rgb-wallet config show

# Edit config value
f1r3fly-rgb-wallet config set <key> <value>
```

### Wallet Management Commands

```bash
# Create new wallet (generates mnemonic)
f1r3fly-rgb-wallet create-wallet <name>

# Import wallet from mnemonic
f1r3fly-rgb-wallet import-wallet <name> --mnemonic "<12 or 24 words>"

# List all wallets
f1r3fly-rgb-wallet list-wallets

# Show wallet info (mnemonic, addresses, keys)
f1r3fly-rgb-wallet --wallet <name> wallet-info [--show-mnemonic]

# Delete wallet
f1r3fly-rgb-wallet delete-wallet <name>
```

### Bitcoin Commands

```bash
# Sync wallet with blockchain
f1r3fly-rgb-wallet --wallet <name> sync

# Get Bitcoin balance
f1r3fly-rgb-wallet --wallet <name> get-balance

# Get addresses
f1r3fly-rgb-wallet --wallet <name> get-addresses [--count <n>]

# Create UTXO via self-send
f1r3fly-rgb-wallet --wallet <name> create-utxo \
  --amount <btc> \
  [--fee-rate <sat/vB>]

# Send Bitcoin to address
f1r3fly-rgb-wallet --wallet <name> send-bitcoin \
  --to <address> \
  --amount <sats> \
  [--fee-rate <sat/vB>]
```

### RGB Asset Commands

```bash
# Issue new RGB asset
f1r3fly-rgb-wallet --wallet <name> issue-asset \
  --ticker <TICKER> \
  --name <Name> \
  --supply <amount> \
  --precision <decimals> \
  --genesis-utxo <txid:vout>

# List all assets
f1r3fly-rgb-wallet --wallet <name> list-assets

# Get RGB balance for all assets
f1r3fly-rgb-wallet --wallet <name> rgb-balance

# Get RGB balance for specific asset
f1r3fly-rgb-wallet --wallet <name> rgb-balance --contract-id <id>

# Get contract info
f1r3fly-rgb-wallet --wallet <name> get-contract-info --contract-id <id>
```

### RGB Transfer Commands

```bash
# Generate invoice (recipient)
f1r3fly-rgb-wallet --wallet <name> generate-invoice \
  --contract-id <id> \
  --amount <tokens> \
  [--utxo <txid:vout>]

# Send transfer (sender)
f1r3fly-rgb-wallet --wallet <name> send-transfer \
  --invoice <rgb_invoice_string> \
  [--fee-rate <sat/vB>]

# Accept consignment (recipient)
f1r3fly-rgb-wallet --wallet <name> accept-consignment \
  --file <path/to/consignment.json>

# Export genesis consignment (issuer)
f1r3fly-rgb-wallet --wallet <name> export-genesis \
  --contract-id <id> \
  --output <path/to/output.json>
```

---

## Testing Strategy

### Unit Tests

- **Key Derivation**: Test BIP39/BIP32 and F1r3fly key derivation
- **Storage**: Test wallet save/load/delete
- **Config**: Test config loading with overrides
- **Bitcoin Layer**: Test BDK integration (with mock blockchain)
- **F1r3fly Layer**: Test executor initialization (with mock F1r3node)

### Integration Tests

- **Regtest Environment**:
  - Bitcoin Core in regtest mode
  - Electrs for Esplora API
  - F1r3node instance
- **Test Coverage**:
  - Full wallet lifecycle
  - Asset issuance
  - Transfer flow
  - Balance persistence
  - State recovery

### Test Environment Setup

```bash
# Start regtest environment
cd /path/to/f1r3fly-rgb-2
./scripts/start-regtest.sh

# Run integration tests
cd f1r3fly-rgb-wallet
cargo test --test f1r3fly_transfer_balance_test -- --ignored --nocapture
```

### Test Success Criteria

- ✅ All unit tests pass
- ✅ `f1r3fly_transfer_balance_test.rs` passes completely
- ✅ Edge case tests pass
- ✅ No memory leaks
- ✅ Graceful error handling

---

## Dependencies

### Cargo.toml

```toml
[package]
name = "f1r3fly-rgb-wallet"
version = "0.1.0"
edition = "2021"
authors = ["F1r3fly RGB Contributors"]
description = "F1r3fly-RGB CLI wallet"
license = "Apache-2.0"

[[bin]]
name = "f1r3fly-rgb-wallet"
path = "src/main.rs"

[dependencies]
# F1r3fly-RGB core library
f1r3fly-rgb = { path = "../f1r3fly-rgb" }

# F1r3node client (same as f1r3fly-rgb)
node_cli = { git = "https://github.com/F1R3FLY-io/rust-client", branch = "rgb" }

# Bitcoin & BDK
bdk = { version = "1.0", features = ["keys-bip39"] }
bdk_esplora = "0.18"
bitcoin = "0.32"
bip39 = "2.0"

# Cryptography
secp256k1 = { version = "0.28", features = ["rand-std"] }
blake2 = "0.10"

# CLI
clap = { version = "4.5", features = ["derive"] }

# Async
tokio = { version = "1.40", features = ["full"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Encryption (for key storage)
aes-gcm = "0.10"

# Utilities
anyhow = "1.0"
thiserror = "1.0"
log = "0.4"
env_logger = "0.11"
hex = "0.4"
chrono = "0.4"

[dev-dependencies]
tempfile = "3.8"
dotenv = "0.15"
```

### Dependency Notes

- **f1r3fly-rgb**: Provides `F1r3flyRgbContracts`, `F1r3flyConsignment`, etc.
- **node_cli**: F1r3node gRPC client (same as used in f1r3fly-rgb)
- **BDK**: Industry-standard Bitcoin wallet library
- **clap**: CLI argument parsing
- **aes-gcm**: Key encryption (password-based)

---

## Success Metrics

### Phase 1 Complete
- ✅ Wallet creation/import works
- ✅ Bitcoin sync works
- ✅ Can query balance
- ✅ Can send Bitcoin

### Phase 2 Complete
- ✅ F1r3node connection works
- ✅ Can issue RGB asset
- ✅ Can query RGB balance
- ✅ State persists

### Phase 3 Complete
- ✅ Invoice generation works
- ✅ Transfer flow complete
- ✅ Consignment validation works
- ✅ Balances update correctly

### Phase 4 Complete
- ✅ Full integration test passes
- ✅ Documentation complete
- ✅ Production-ready

---

## Next Steps

1. **Review this plan** - Ensure all requirements covered
2. **Begin Phase 1, Week 1** - Project setup and key management
3. **Iterate weekly** - Review progress and adjust as needed
4. **Test continuously** - Validate each phase before moving forward

---

## Appendix: Comparison with Traditional RGB Wallet

| Aspect | Traditional RGB | F1r3fly-RGB | Notes |
|--------|----------------|-------------|-------|
| **Contract Storage** | `rgb-std::Stock` (local) | F1r3node (remote) | State on blockchain shard |
| **Seal Tracking** | `rgb-std::Pile` | `BitcoinAnchorTracker` | Same interface |
| **Contract API** | `rgb-std::Contract` | `F1r3flyRgbContract` | Same operations |
| **Transfers** | `rgb-std::Consignment` | `F1r3flyConsignment` | Includes F1r3flyStateProof |
| **Validation** | ALuVM execution | Rholang on F1r3node | Different VM, same result |
| **Bitcoin Layer** | Manual PSBT | BDK | Modern approach |
| **Keys** | Bitcoin only | Bitcoin + F1r3fly | Dual key system |

---

**End of Implementation Plan**

