# RGB Wallet - Complete Implementation Documentation

**Last Updated:** October 28, 2025  
**Version:** Post-Refactor (Ephemeral Runtime Architecture)

This document provides a comprehensive overview of the RGB wallet implementation, including all features, technical details, and current state of the codebase.

> **⚠️ Major Update (October 2025):** The wallet has undergone significant architectural changes, including a shift from cached runtimes to ephemeral runtimes (matching RGB CLI), modular code organization, and Firefly/RChain integration.

---

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Configuration Management](#configuration-management)
4. [Backend Implementation (Rust)](#backend-implementation-rust)
5. [Frontend Implementation (React/TypeScript)](#frontend-implementation-reacttypescript)
6. [RGB Integration](#rgb-integration)
7. [Firefly/RChain Integration](#fireflyRChain-integration)
8. [API Reference](#api-reference)
9. [User Flows](#user-flows)
10. [Technical Details](#technical-details)
11. [Known Issues and Limitations](#known-issues-and-limitations)
12. [Future Improvements](#future-improvements)

---

## Overview

The RGB wallet is a full-featured Bitcoin and RGB asset management system built with:
- **Backend**: Rust (Axum web framework)
- **Frontend**: React + TypeScript + Vite
- **RGB**: RGB 0.12 (latest version with native invoice support)
- **Bitcoin Network**: Signet (testnet)

### Capabilities

✅ **Bitcoin Operations:**
- Create and import HD wallets (BIP39 mnemonic)
- Generate receive addresses (BIP84 SegWit native)
- Check balance and UTXO management
- Send Bitcoin to any address
- Create RGB-compatible UTXOs
- Unlock/consolidate UTXOs

✅ **RGB Asset Operations:**
- Issue RGB20 fungible tokens
- Generate RGB invoices (native RGB URI format)
- Send RGB transfers (with consignment generation)
- Accept consignments (genesis and transfer)
- Export genesis for same-wallet sync
- Display assets with 0 balance (for receiving)

✅ **Advanced Features:**
- Smart RGB runtime sync (fast cached reads, explicit syncs)
- Multi-UTXO transaction building with change
- PSBT signing with BIP32 key derivation
- Blockchain broadcast via Mempool.space API
- Consignment file download/upload

---

## Architecture

### High-Level Structure

```
f1r3fly-rgb/
├── wallet/                    # Rust backend
│   ├── src/
│   │   ├── api/              # HTTP API handlers and routes
│   │   ├── wallet/           # Core wallet logic (modular)
│   │   │   ├── manager.rs    # Orchestration layer (548 lines)
│   │   │   ├── *_ops.rs      # Operation modules
│   │   │   └── shared/       # Shared utilities (11 modules)
│   │   ├── firefly/          # Firefly/RChain integration
│   │   ├── config.rs         # Environment-based configuration
│   │   └── error.rs          # Error handling
│   └── Cargo.toml
│
├── wallet-frontend/           # React frontend
│   ├── src/
│   │   ├── api/              # API client
│   │   ├── components/       # React components
│   │   ├── pages/            # Page components
│   │   └── utils/            # Helper functions
│   └── package.json
│
├── rgb-std/                   # RGB library (modified)
│   └── src/
│       └── contracts.rs      # Added witness query methods
│
└── docs/                      # Documentation
    └── *.md
```

### Data Flow

```
User Interaction (Browser)
    ↓
React Components
    ↓
API Client (Axios)
    ↓
Axum Handlers (Rust)
    ↓
WalletManager (Orchestration Layer)
    ↓
├─→ Configuration (Environment Variables)
├─→ Storage (File System)
├─→ RGB Runtime (Ephemeral per-operation)
├─→ Firefly Client (gRPC/HTTP)
├─→ Bitcoin Network (Esplora/Mempool.space)
└─→ Esplora API (Balance Queries)
```

---

## Configuration Management

### Environment-Based Configuration

The wallet uses `config.rs` for flexible, environment-based configuration supporting multiple deployment scenarios.

**Configuration Structure:**
```rust
pub struct WalletConfig {
    pub bitcoin_network: bitcoin::Network,      // Bitcoin network type
    pub bpstd_network: bpstd::Network,          // BP-Std network type
    pub esplora_url: String,                    // Esplora API endpoint
    pub bitcoin_rpc_url: Option<String>,        // Optional Bitcoin Core RPC
    pub public_url: String,                     // Public API URL for downloads
    pub firefly_host: String,                   // Firefly node host
    pub firefly_grpc_port: u16,                 // Firefly gRPC port
    pub firefly_http_port: u16,                 // Firefly HTTP port
}
```

### Environment Variables

**Bitcoin Network:**
- `BITCOIN_NETWORK` - "signet" (default) or "regtest"
- `ESPLORA_URL` - Custom Esplora endpoint (optional)
- `BITCOIN_RPC_URL` - Bitcoin Core RPC URL (optional)

**Server Configuration:**
- `BIND_ADDRESS` - Server bind address (default: "0.0.0.0:3000")
- `PUBLIC_URL` - Public API URL for download links (default: "http://localhost:3000")
- `ALLOWED_ORIGINS` - CORS allowed origins, comma-separated (optional)

**Firefly Integration:**
- `FIREFLY_HOST` - Firefly node host (default: "localhost")
- `FIREFLY_GRPC_PORT` - Firefly gRPC port (default: 40401)
- `FIREFLY_HTTP_PORT` - Firefly HTTP port (default: 40403)

**Logging:**
- `RUST_LOG` - Log level (info, debug, trace)

### Usage Examples

**Development (Signet):**
```bash
RUST_LOG=debug cargo run
```

**Local Regtest:**
```bash
BITCOIN_NETWORK=regtest \
ESPLORA_URL=http://localhost:3000 \
BIND_ADDRESS=127.0.0.1:3000 \
cargo run
```

**Production Deployment:**
```bash
BITCOIN_NETWORK=signet \
BIND_ADDRESS=0.0.0.0:3000 \
PUBLIC_URL=https://api.example.com \
ALLOWED_ORIGINS=https://app.example.com,https://preview.example.com \
FIREFLY_HOST=firefly.example.com \
RUST_LOG=info \
cargo run --release
```

### CORS Configuration

**Development Mode (Allow All):**
```bash
# No ALLOWED_ORIGINS set - allows any origin
cargo run
```

**Production Mode (Restricted):**
```bash
# Whitelist specific origins
ALLOWED_ORIGINS="https://app.vercel.app,https://preview.vercel.app" cargo run
```

---

## Backend Implementation (Rust)

### Project Structure

```
wallet/src/
├── api/
│   ├── handlers.rs          # HTTP request handlers
│   ├── server.rs            # Axum server setup and routing
│   ├── types.rs             # API request/response types
│   └── mod.rs               # API module exports
│
├── wallet/
│   ├── manager.rs           # Orchestration layer (548 lines)
│   ├── address_ops.rs       # Address operations
│   ├── balance_ops.rs       # Balance queries (Bitcoin + RGB)
│   ├── bitcoin_ops.rs       # Bitcoin transaction operations
│   ├── rgb_consignment_ops.rs  # RGB consignment import/export
│   ├── rgb_transfer_ops.rs  # RGB transfer operations
│   ├── sync_ops.rs          # Wallet and RGB sync operations
│   ├── wallet_ops.rs        # Wallet create/import/list
│   ├── mod.rs               # Wallet module exports
│   └── shared/              # Shared utilities
│       ├── addresses.rs     # Address derivation utilities
│       ├── balance.rs       # Balance data structures
│       ├── keys.rs          # Key management utilities
│       ├── rgb.rs           # RGB utilities and types
│       ├── rgb_runtime.rs   # Ephemeral RGB runtime manager
│       ├── signer.rs        # PSBT signing implementation
│       ├── storage.rs       # File-based wallet storage
│       ├── transaction.rs   # Transaction building utilities
│       └── mod.rs           # Shared module exports
│
├── firefly/
│   ├── client.rs            # Firefly gRPC client
│   ├── types.rs             # Firefly type definitions
│   └── mod.rs               # Firefly module exports
│
├── config.rs                # Environment-based configuration
├── error.rs                 # Unified error handling
├── main.rs                  # Server entry point
└── lib.rs                   # Library exports
```

### Key Components

#### 1. WalletManager (`wallet/src/wallet/manager.rs`)

**Pure orchestration layer** that delegates to specialized operation modules. Manager is now just 548 lines (down from 1,275).

**Structure:**
```rust
pub struct WalletManager {
    pub config: WalletConfig,
    pub storage: Storage,
    balance_checker: BalanceChecker,
    rgb_runtime_manager: RgbRuntimeManager,  // Ephemeral runtime creator
    pub firefly_client: Option<FireflyClient>,
}
```

**Key Methods (Delegation Pattern):**

```rust
// Wallet Management (delegates to wallet_ops)
pub fn create_wallet(&self, name: &str) -> Result<WalletInfo>
pub fn import_wallet(&self, name: &str, mnemonic: Mnemonic) -> Result<WalletInfo>
pub fn list_wallets(&self) -> Result<Vec<WalletMetadata>>

// Address Operations (delegates to address_ops)
pub fn get_addresses(&self, name: &str, count: u32) -> Result<Vec<AddressInfo>>
pub fn get_primary_address(&self, name: &str) -> Result<NextAddressInfo>

// Balance & Sync (delegates to balance_ops and sync_ops)
pub async fn get_balance(&self, name: &str) -> Result<BalanceInfo>
pub async fn sync_wallet(&self, name: &str) -> Result<SyncResult>
pub async fn sync_rgb_runtime(&self, name: &str) -> Result<()>

// Bitcoin Transactions (delegates to bitcoin_ops)
pub async fn send_bitcoin(&self, name: &str, request: SendBitcoinRequest) -> Result<SendBitcoinResponse>
pub async fn create_utxo(&self, name: &str, request: CreateUtxoRequest) -> Result<CreateUtxoResult>
pub async fn unlock_utxo(&self, name: &str, request: UnlockUtxoRequest) -> Result<UnlockUtxoResult>

// RGB Operations (delegates to rgb_transfer_ops and rgb_consignment_ops)
pub async fn issue_asset(&self, name: &str, request: IssueAssetRequest) -> Result<IssueAssetResponse>
pub async fn generate_rgb_invoice(&self, name: &str, request: GenerateInvoiceRequest) -> Result<GenerateInvoiceResult>
pub async fn send_transfer(&self, name: &str, invoice: &str, fee: Option<u64>) -> Result<SendTransferResponse>
pub async fn accept_consignment(&self, name: &str, data: Vec<u8>) -> Result<AcceptConsignmentResponse>
pub async fn export_genesis_consignment(&self, name: &str, contract_id: &str) -> Result<ExportGenesisResponse>
```

**Ephemeral Runtime Strategy:**
- Each RGB operation creates a fresh runtime from disk
- Performs operation and drops runtime
- `FileHolder::drop()` auto-saves state to disk
- No caching, no lifecycle management
- Matches RGB CLI architecture (proven reliable)

#### 2. Operation Modules

The business logic is now split into focused operation modules:

**Address Operations (`address_ops.rs`):**
```rust
pub fn get_addresses(storage: &Storage, wallet_name: &str, count: u32) -> Result<Vec<AddressInfo>>
pub fn get_primary_address(storage: &Storage, wallet_name: &str) -> Result<NextAddressInfo>
```

**Balance Operations (`balance_ops.rs`):**
```rust
// Async HTTP call for Bitcoin balance
pub async fn get_bitcoin_balance(storage: &Storage, checker: &BalanceChecker, wallet_name: &str) -> Result<BalanceInfo>

// Sync blocking call for RGB balance (uses ephemeral runtime)
pub fn get_rgb_balance_sync(storage: &Storage, rgb_mgr: &RgbRuntimeManager, wallet_name: &str, utxos: &[UTXO]) -> Result<RgbBalanceData>
```

**Bitcoin Operations (`bitcoin_ops.rs`):**
```rust
pub async fn send_bitcoin(...) -> Result<SendBitcoinResponse>
pub async fn create_utxo(...) -> Result<CreateUtxoResult>
pub async fn unlock_utxo(...) -> Result<UnlockUtxoResult>
```

**RGB Transfer Operations (`rgb_transfer_ops.rs`):**
```rust
// Generate invoice (ephemeral runtime)
pub fn generate_rgb_invoice_sync(...) -> Result<GenerateInvoiceResult>

// Send transfer with 3-step process (matches RGB CLI)
pub fn send_transfer(...) -> Result<SendTransferResponse>
```

**RGB Consignment Operations (`rgb_consignment_ops.rs`):**
```rust
// Accept genesis or transfer consignment
pub fn accept_consignment(...) -> Result<AcceptConsignmentResponse>

// Export genesis for same-wallet sync
pub fn export_genesis_consignment(...) -> Result<ExportGenesisResponse>
```

**Sync Operations (`sync_ops.rs`):**
```rust
// Bitcoin wallet sync
pub async fn sync_wallet(...) -> Result<SyncResult>

// RGB runtime sync (ephemeral runtime with 1 confirmation)
pub fn sync_rgb_runtime(...) -> Result<()>
```

**Wallet Operations (`wallet_ops.rs`):**
```rust
pub fn create_wallet(storage: &Storage, name: &str) -> Result<WalletInfo>
pub fn import_wallet(storage: &Storage, name: &str, mnemonic: Mnemonic) -> Result<WalletInfo>
pub fn list_wallets(storage: &Storage) -> Result<Vec<WalletMetadata>>
```

#### 3. Storage Layer (`wallet/src/wallet/shared/storage.rs`)

File-based persistence for wallet data.

**Directory Structure:**
```
./wallets/
├── <wallet-name>/
│   ├── descriptor.txt       # BIP84 descriptor string
│   ├── mnemonic.txt         # BIP39 mnemonic (encrypted in production)
│   ├── state.json           # Wallet state (used addresses, sync height)
│   └── rgb/                 # RGB runtime data (managed by RGB library)
│       ├── rgb_data/        # Per-wallet RGB state
│       ├── stockpile/       # Contract state and history
│       ├── stash/           # UTXO allocations
│       └── indexer/         # Blockchain witness data
│
├── consignments/            # Transfer consignment files
│   └── transfer_<contract_id>_<timestamp>.rgbc
│
├── exports/                 # Genesis exports for wallet sync
│   └── genesis_<contract_id>.rgbc
│
└── temp_consignments/       # Temporary import files
    └── accept_<uuid>.rgbc
```

**Stored Data:**

```rust
pub struct WalletState {
    pub used_addresses: Vec<u32>,           // Address indices that received funds
    pub last_synced_height: Option<u64>,   // Last blockchain height synced
    pub public_address_index: u32,          // Current public receive address
}
```

#### 4. Transaction Builder (`wallet/src/wallet/shared/transaction.rs`)

Constructs Bitcoin transactions with proper fee estimation.

**Methods:**

```rust
// Send Bitcoin to external address
pub fn build_send_tx(
    &self,
    utxos: &[UTXO],
    to_address: Address,
    amount_sats: u64,
    change_address: Address,
    fee_rate_sat_vb: u64,
) -> Result<Transaction>

// Create UTXO to self (for RGB operations)
pub fn build_send_to_self(
    &self,
    available_utxos: &[UTXO],
    target_amount_sats: u64,
    fee_rate_sat_vb: u64,
    recipient_address: Address,
) -> Result<Transaction>

// Unlock occupied UTXO (consolidate)
pub fn build_unlock_utxo_tx(
    &self,
    utxo: &UTXO,
    destination_address: Address,
    fee_rate_sat_vb: u64,
) -> Result<Transaction>
```

**Fee Estimation:**
```rust
fn estimate_tx_size(&self, num_inputs: usize, num_outputs: usize) -> u64 {
    let base_size = 10;
    let input_size = 68;   // SegWit witness data
    let output_size = 34;  // P2WPKH output
    (base_size + (num_inputs * input_size) + (num_outputs * output_size)) as u64
}
```

#### 5. PSBT Signer (`wallet/src/wallet/shared/signer.rs`)

Implements `bpstd::psbt::Signer` for signing PSBTs used in RGB transfers.

**Implementation:**

```rust
pub struct WalletSigner {
    mnemonic: bip39::Mnemonic,
    network: Network,
}

impl bpstd::psbt::Signer for WalletSigner {
    fn approve(&self, _psbt: &Psbt) -> Result<(), SigningError> {
        Ok(()) // Auto-approve for simplicity
    }

    fn sign_ecdsa(&self, /* ... */) -> Result<Signature, SigningError> {
        // Derives correct key for each UTXO's address index
        // Signs with ECDSA for P2WPKH inputs
    }
}
```

### Modified RGB Libraries

#### RGB-Std Contracts (`rgb-std/src/contracts.rs`)

**Added Methods:**

```rust
impl Contracts {
    /// Get all witnesses for a contract
    pub fn contract_witnesses(&self, contract_id: ContractId) 
        -> impl Iterator<Item = Witness<_>> + '_
    
    /// Get witness transaction IDs
    pub fn contract_witness_ids(&self, contract_id: ContractId)
        -> impl Iterator<Item = WitnessId> + '_
    
    /// Get witness count
    pub fn contract_witness_count(&self, contract_id: ContractId) -> usize
}
```

**Purpose:**
These methods enable `accept_consignment` to distinguish between genesis and transfer consignments by querying witness data after import.

---

## Frontend Implementation (React/TypeScript)

### Project Structure

```
wallet-frontend/src/
├── api/
│   ├── client.ts            # Axios configuration
│   ├── types.ts             # TypeScript interfaces (184 lines)
│   └── wallet.ts            # API client methods (212 lines)
│
├── components/
│   ├── BalanceDisplay.tsx               # Bitcoin balance card
│   ├── AddressList.tsx                  # Address list display
│   ├── UTXOList.tsx                     # UTXO table with lock/unlock
│   ├── CreateUtxoModal.tsx              # Create UTXO dialog
│   ├── IssueAssetModal.tsx              # Issue RGB20 asset dialog
│   ├── GenerateInvoiceModal.tsx         # Generate RGB invoice dialog
│   ├── SendTransferModal.tsx            # Send RGB transfer dialog
│   ├── AcceptConsignmentModal.tsx       # Import consignment dialog
│   ├── ExportGenesisModal.tsx           # Export genesis dialog
│   └── SendBitcoinModal.tsx             # Send Bitcoin dialog
│
├── pages/
│   ├── Home.tsx             # Wallet list page
│   └── WalletDetail.tsx     # Main wallet page (459 lines)
│
└── utils/
    └── format.ts            # Formatting utilities
```

### Key Components

#### 1. WalletDetail Page (`pages/WalletDetail.tsx`)

The main wallet interface integrating all functionality.

**State Management:**

```typescript
const [balance, setBalance] = useState<BalanceInfo | null>(null);
const [nextAddress, setNextAddress] = useState<NextAddressInfo | null>(null);
const [showCreateUtxoModal, setShowCreateUtxoModal] = useState(false);
const [showIssueAssetModal, setShowIssueAssetModal] = useState(false);
const [showGenerateInvoiceModal, setShowGenerateInvoiceModal] = useState(false);
const [showSendTransferModal, setShowSendTransferModal] = useState(false);
const [showAcceptConsignmentModal, setShowAcceptConsignmentModal] = useState(false);
const [showExportGenesisModal, setShowExportGenesisModal] = useState(false);
const [showSendBitcoinModal, setShowSendBitcoinModal] = useState(false);
```

**Features:**
- Bitcoin balance display with sync button
- RGB assets section showing all known contracts (even 0 balance)
- UTXO list with lock status indicators
- Primary receive address with copy button
- Action buttons: Send Bitcoin, Create UTXO, Issue Asset, Import Consignment

**RGB Assets Display:**

```typescript
{balance.known_contracts.map((contract) => (
  <div key={contract.contract_id}>
    <span>{contract.ticker}</span>
    <span>{contract.name}</span>
    <p>Balance: {contract.balance}</p>
    {contract.balance === 0 && (
      <>
        <span>Known Contract</span>
        <p>ℹ️ Need Bitcoin UTXOs to receive tokens</p>
      </>
    )}
    <button onClick={sendTransfer} disabled={contract.balance === 0}>Send</button>
    <button onClick={generateInvoice}>Receive</button>
    <button onClick={exportGenesis}>Export</button>
  </div>
))}
```

#### 2. GenerateInvoiceModal (`components/GenerateInvoiceModal.tsx`)

Generates RGB invoices for receiving tokens.

**Pre-check:**
Before generating, shows clear error if no Bitcoin UTXOs:
```
"This wallet needs Bitcoin UTXOs to generate an invoice. Please:
1. Click 'Receive Bitcoin' to get your wallet address
2. Send Bitcoin from a faucet or another wallet
3. Wait for confirmation
4. Then try generating the invoice again"
```

**Generated Invoice Format:**
```
rgb:contract:bitcoin:testnet@<contract_id>?amount=<amount>&seal=<blinded_seal>
```

#### 3. SendTransferModal (`components/SendTransferModal.tsx`)

Sends RGB transfers by paying an invoice.

**Process:**
1. User pastes RGB invoice
2. Sets fee rate (default 1 sat/vB)
3. Backend creates PSBT, signs, broadcasts
4. Returns consignment download link
5. **Auto-syncs RGB runtime in background** (non-blocking)

**Auto-Sync Implementation:**
```typescript
const response = await walletApi.sendTransfer(walletName, request);
setResult(response);

// Background sync - doesn't block UI
walletApi.syncRgb(walletName).catch(err => {
  console.warn('RGB sync after transfer failed:', err);
});
```

#### 4. AcceptConsignmentModal (`components/AcceptConsignmentModal.tsx`)

Imports RGB consignments (genesis or transfer).

**Features:**
- File upload with drag-and-drop
- Progress indicator during import
- Displays import type (genesis/transfer)
- Shows Bitcoin transaction ID for transfers
- Links to Mempool.space for TX tracking
- Handles re-imports gracefully

**Import Type Detection:**
Backend uses witness count to determine type:
- 0 witnesses = genesis consignment
- 1+ witnesses = transfer consignment

#### 5. SendBitcoinModal (`components/SendBitcoinModal.tsx`)

Simple Bitcoin send interface.

**Validation:**
- Minimum 546 sats (dust limit)
- Address format validation (signet)
- Insufficient funds check
- Excludes RGB-occupied UTXOs

---

## RGB Integration

### RGB Libraries Used

```toml
[dependencies]
rgb = "0.12.0-rc.3"
rgb-std = { version = "0.12.0-rc.3", path = "../rgb-std" }
rgb-invoice = { version = "0.12.0-rc.3", path = "../rgb-std/invoice", features = ["bitcoin", "uri"] }
rgb-persist-fs = "0.12.0-rc.3"
rgbp = "0.12.0"
hypersonic = "0.12.0"
```

### Invoice Format

The wallet uses the **native RGB URI format** with the `uri` feature enabled:

```
rgb:contract:bitcoin:testnet@<contract_id>?amount=<amount>&seal=<auth_token>
```

**Example:**
```
rgb:contract:bitcoin:testnet@contract:B__d4erz-fGK3~gS-8n85Tgf-y5nkA02-Tkbp7jC-3YROe4o?amount=1000&seal=utxob:tb1q...
```

**Parsing:**
```rust
use rgb_invoice::RgbInvoice;
let invoice = RgbInvoice::<rgb::ContractId>::from_str(invoice_str)?;
```

### RGB Runtime Initialization

**Ephemeral Runtime Pattern (Matches RGB CLI):**

The wallet uses an ephemeral runtime approach where each operation creates a fresh runtime, uses it, and drops it:

```rust
pub struct RgbRuntimeManager {
    base_dir: PathBuf,
    network: bpstd::Network,
    esplora_url: String,
}

impl RgbRuntimeManager {
    /// Create ephemeral runtime without blockchain sync
    pub fn init_runtime_no_sync(&self, wallet_name: &str) -> Result<RgbpRuntimeDir> {
        let wallet_dir = self.base_dir.join(wallet_name);
        let rgb_dir = wallet_dir.join("rgb");
        
        // Load descriptor
        let descriptor_str = std::fs::read_to_string(wallet_dir.join("descriptor.txt"))?;
        let descriptor = RgbDescr::from_str(&descriptor_str)?;
        
        // Create runtime (loads existing state from disk)
        let runtime = RgbpRuntimeDir::load_or_create(
            &rgb_dir,
            Consensus::Bitcoin,
            true,  // testnet/signet
            descriptor,
            self.network,
        )?;
        
        Ok(runtime)
        // Runtime auto-saves on drop via FileHolder::drop()
    }
}
```

**Usage Pattern:**
```rust
// 1. Create ephemeral runtime
let mut runtime = rgb_runtime_manager.init_runtime_no_sync(wallet_name)?;

// 2. Sync if needed
runtime.update(1)?;  // 1 confirmation for fast sync

// 3. Perform operation
let result = runtime.do_something()?;

// 4. Runtime drops here → FileHolder::drop() auto-saves to disk
```

**Why Ephemeral?**
- ✅ Matches proven RGB CLI architecture
- ✅ No stale cache issues - always fresh state
- ✅ Simpler code - no locking/threading complexity
- ✅ Automatic state persistence via drop
- ✅ No manual shutdown needed

### RGB Transfer Process

**3-Step Process (Matches RGB CLI):**

#### Step 1: Create Payment
```rust
// Create ephemeral runtime
let mut runtime = rgb_runtime_manager.init_runtime_no_sync(wallet_name)?;

// Sync RGB state BEFORE payment (UTXOs + witnesses)
runtime.update(1)?;

// Parse invoice
let invoice = RgbInvoice::<rgb::ContractId>::from_str(invoice_str)?;

// Create payment - returns PSBT and Payment
let (psbt, payment) = runtime.pay_invoice(
    &invoice,
    CoinselectStrategy::Aggregate,
    tx_params,
    None
)?;

// Generate consignment BEFORE signing
runtime.contracts.consign_to_file(
    &consignment_path,
    contract_id,
    payment.terminals
)?;

// Extract psbt_meta for later use
let psbt_meta = payment.psbt_meta.clone();
```
Runtime #1 drops here → Saves payment bundle to stockpile via `FileHolder::drop()`

#### Step 2: Sign PSBT
```rust
// Sign WITHOUT runtime (pure cryptographic operation)
let signer = WalletSigner::new(mnemonic, network);
let signed_count = psbt.sign(&signer)?;
```

#### Step 3: Finalize & Broadcast
```rust
// Load descriptor for finalization
let descriptor = RgbDescr::from_str(&descriptor_str)?;

// Finalize PSBT (convert partial_sigs to final_witness)
let finalized_count = psbt.finalize(&descriptor)?;

// Extract signed transaction
let tx = psbt.extract()?;

// Broadcast via Esplora API (NOT via runtime.finalize())
let tx_hex = format!("{:x}", tx);
let response = client
    .post(format!("{}/tx", esplora_url))
    .header("Content-Type", "text/plain")
    .body(tx_hex)
    .send()?;
```

**Note:** The RGB CLI's `finalize` command does NOT use `runtime.finalize()` - it's commented out in `rgb/cli/src/exec.rs:552`. The wallet follows this proven pattern.

**Why This Approach?**
- ✅ Matches RGB CLI implementation (battle-tested)
- ✅ Clear separation of concerns (payment → sign → broadcast)
- ✅ Change UTXO discovered via `runtime.update()` on next balance query
- ✅ No manual seal management needed

### Consignment Import

**Accept Consignment Process (Ephemeral Runtime):**

```rust
pub fn accept_consignment(
    storage: &Storage,
    rgb_runtime_manager: &RgbRuntimeManager,
    wallet_name: &str,
    consignment_bytes: Vec<u8>,
) -> Result<AcceptConsignmentResponse> {
    // 1. Save to temp file
    let temp_path = format!("./wallets/temp_consignments/accept_{}.rgbc", uuid);
    std::fs::write(&temp_path, &consignment_bytes)?;
    
    // 2. Create ephemeral runtime
    let mut runtime = rgb_runtime_manager.init_runtime_no_sync(wallet_name)?;
    
    {
        // 3. Get contracts before import
        let contract_ids_before: HashSet<String> = runtime.contracts
            .contract_ids()
            .map(|id| id.to_string())
            .collect();
        
        // 4. Import consignment
        runtime.consume_from_file(true, &temp_path, |_, _, _| Ok(()))?;
        
        // 5. Find new or updated contract
        let contract_ids_after: HashSet<String> = runtime.contracts
            .contract_ids()
            .map(|id| id.to_string())
            .collect();
        
        let new_contracts: Vec<_> = contract_ids_after
            .difference(&contract_ids_before)
            .collect();
        
        // Determine contract ID (handles both new imports and re-imports)
        let contract_id_str = if !new_contracts.is_empty() {
            new_contracts.first().unwrap().to_string()
        } else if contract_ids_after.len() == 1 {
            contract_ids_after.iter().next().unwrap().to_string()
        } else {
            return Err("Cannot determine which contract was updated");
        };
        
        let contract_id = ContractId::from_str(&contract_id_str)?;
        
        // 6. Query witness data to determine import type
        let witness_count = runtime.contracts.contract_witness_count(contract_id);
        let (import_type, bitcoin_txid, status) = if witness_count == 0 {
            ("genesis", None, "genesis_imported")
        } else {
            let witnesses: Vec<_> = runtime.contracts
                .contract_witnesses(contract_id)
                .collect();
            if let Some(last_witness) = witnesses.last() {
                let txid = last_witness.id.to_string();
                let status = match last_witness.status {
                    WitnessStatus::Genesis => "genesis_imported",
                    WitnessStatus::Offchain => "offchain",
                    WitnessStatus::Tentative => "pending",
                    WitnessStatus::Mined(_) => "confirmed",
                    WitnessStatus::Archived => "archived",
                };
                ("transfer", Some(txid), status)
            } else {
                ("transfer", None, "imported")
            }
        };
        
        (contract_id_str, import_type, bitcoin_txid, status)
    }
    // Runtime drops here → FileHolder::drop() auto-saves imported state
    
    // 7. Cleanup temp file
    std::fs::remove_file(&temp_path)?;
    
    Ok(AcceptConsignmentResponse {
        contract_id,
        status,
        import_type,
        bitcoin_txid,
    })
}
```

**Note:** We do NOT call `runtime.update()` after import. The consignment has been imported and saved to disk via `FileHolder::drop()`. The next operation that needs fresh state (like balance query) will call `update()`.

---

## Firefly/RChain Integration

### Overview

The wallet includes a Firefly client for deploying RGB contracts to the Rholang blockchain, enabling the RGB-Rholang bridge functionality.

### Architecture

```
wallet/src/firefly/
├── client.rs    # gRPC client for Firefly node
├── types.rs     # Type definitions
└── mod.rs       # Module exports
```

### FireflyClient

**Structure:**
```rust
pub struct FireflyClient {
    signing_key: SecretKey,      // For signing deployments
    node_host: String,           // Firefly node host
    grpc_port: u16,              // gRPC port (40401)
}
```

**Key Methods:**

```rust
impl FireflyClient {
    /// Create client with bootstrap validator key
    pub fn new(host: &str, grpc_port: u16) -> Self
    
    /// Deploy Rholang code to blockchain
    pub async fn deploy(&self, rholang_code: &str) -> Result<String, _>
    
    /// Propose block to confirm deployment
    pub async fn propose(&self, deploy_id: &str) -> Result<String, _>
    
    /// Get current block number
    pub async fn get_current_block_number(&self) -> Result<i64, _>
    
    /// Build signed deployment message
    pub fn build_deploy_msg(
        &self,
        code: String,
        phlo_limit: i64,
        lang: String,
        valid_after_block_number: i64,
    ) -> DeployDataProto
}
```

### Usage Example

**Deploy RGB Contract to Rholang:**
```rust
// Initialize Firefly client from config
let firefly_client = FireflyClient::new(
    &config.firefly_host,
    config.firefly_grpc_port,
);

// Prepare Rholang code
let rholang_code = format!(
    r#"
    new contract in {{
      contract!(
        "rgb_contract",
        "contract_id": "{}",
        "ticker": "{}",
        "supply": {}
      )
    }}
    "#,
    contract_id, ticker, supply
);

// Deploy to Firefly
let deploy_id = firefly_client.deploy(&rholang_code).await?;
log::info!("Deployed to Firefly: {}", deploy_id);

// Propose block to confirm
let block_hash = firefly_client.propose(&deploy_id).await?;
log::info!("Block proposed: {}", block_hash);
```

### Configuration

**Environment Variables:**
- `FIREFLY_HOST` - Firefly node hostname (default: "localhost")
- `FIREFLY_GRPC_PORT` - gRPC port for deploy/propose (default: 40401)
- `FIREFLY_HTTP_PORT` - HTTP port for status/query (default: 40403)

**Example:**
```bash
FIREFLY_HOST=firefly.example.com \
FIREFLY_GRPC_PORT=40401 \
FIREFLY_HTTP_PORT=40403 \
cargo run
```

### Security Considerations

⚠️ **Current Implementation:**
- Uses hardcoded bootstrap validator private key for testing
- Key: `5f668a7ee96d944a4494cc947e4005e172d7ab3461ee5538f1f2a45a835e9657`

⚠️ **Production Requirements:**
1. Move private key to environment variable or secure key store
2. Implement key rotation mechanism
3. Add proper authentication/authorization
4. Use per-user signing keys instead of shared validator key

**Recommended:**
```bash
export FIREFLY_PRIVATE_KEY="your_secure_key_here"
```

### API Status Endpoint

The wallet exposes Firefly node status via HTTP:

```http
GET /api/firefly/status

Response:
{
  "status": "connected",
  "node_url": "http://localhost:40403",
  "current_block": 12345
}
```

### Dependencies

```toml
[dependencies]
# Firefly/RChain integration
secp256k1 = { version = "0.28.0", features = ["rand-std"] }
blake2 = "0.10.6"
typenum = "1.16.0"
prost = "0.13.5"
f1r3fly-models = "0.1.0"
hex = "0.4.3"
```

### Future Enhancements

1. **Contract Verification** - Verify RGB contracts on-chain
2. **State Queries** - Query contract state from Rholang
3. **Event Listeners** - Subscribe to Rholang events
4. **Multi-Signature** - Support multi-sig deployments
5. **Gas Optimization** - Optimize phlo usage for deployments

---

## API Reference

### Base URL
```
http://localhost:3001/api
```

### Wallet Management

#### Create Wallet
```http
POST /wallet/create
Content-Type: application/json

{
  "name": "wallet-1"
}

Response:
{
  "name": "wallet-1",
  "mnemonic": "abandon abandon abandon...",
  "first_address": "tb1q...",
  "descriptor": "wpkh([fingerprint]/84'/1'/0'/0/*)"
}
```

#### Import Wallet
```http
POST /wallet/import
Content-Type: application/json

{
  "name": "wallet-2",
  "mnemonic": "abandon abandon abandon..."
}
```

#### List Wallets
```http
GET /wallet/list

Response:
[
  {
    "name": "wallet-1",
    "created_at": "2025-10-13T12:00:00Z"
  }
]
```

### Balance & Addresses

#### Get Balance
```http
GET /wallet/:name/balance

Response:
{
  "confirmed_sats": 100000,
  "unconfirmed_sats": 0,
  "utxo_count": 2,
  "utxos": [
    {
      "txid": "abc123...",
      "vout": 0,
      "amount_sats": 50000,
      "confirmations": 6,
      "is_occupied": true,
      "bound_assets": [
        {
          "asset_id": "contract:B__...",
          "asset_name": "Test Token",
          "ticker": "TEST",
          "amount": "1000"
        }
      ]
    }
  ],
  "known_contracts": [
    {
      "contract_id": "contract:B__...",
      "ticker": "TEST",
      "name": "Test Token",
      "balance": 1000
    }
  ]
}
```

#### Get Primary Address
```http
GET /wallet/:name/primary-address

Response:
{
  "address": "tb1q...",
  "index": 0,
  "total_used": 1,
  "descriptor": "wpkh([...]/84'/1'/0'/0/*)"
}
```

#### Sync Wallet
```http
POST /wallet/:name/sync

Response:
{
  "synced_height": 123456,
  "addresses_checked": 20,
  "new_transactions": 2
}
```

#### Sync RGB Runtime
```http
POST /wallet/:name/sync-rgb

Response: {}
```

### Bitcoin Operations

#### Send Bitcoin
```http
POST /wallet/:name/send-bitcoin
Content-Type: application/json

{
  "to_address": "tb1q...",
  "amount_sats": 10000,
  "fee_rate_sat_vb": 2
}

Response:
{
  "txid": "abc123...",
  "amount_sats": 10000,
  "fee_sats": 300,
  "to_address": "tb1q..."
}
```

#### Create UTXO
```http
POST /wallet/:name/create-utxo
Content-Type: application/json

{
  "amount_btc": 0.0003,
  "fee_rate_sat_vb": 2
}

Response:
{
  "txid": "abc123...",
  "amount_sats": 30000,
  "fee_sats": 300,
  "target_address": "tb1q..."
}
```

### RGB Operations

#### Issue Asset
```http
POST /wallet/:name/issue-asset
Content-Type: application/json

{
  "name": "Test Token",
  "ticker": "TEST",
  "precision": 0,
  "supply": 1000000,
  "genesis_utxo": "abc123...:0"
}

Response:
{
  "contract_id": "contract:B__d4erz...",
  "genesis_seal": "utxob:abc123...:0"
}
```

#### Generate Invoice
```http
POST /wallet/:name/generate-invoice
Content-Type: application/json

{
  "contract_id": "contract:B__d4erz...",
  "amount": 1000
}

Response:
{
  "invoice": "rgb:contract:bitcoin:testnet@contract:B__d4erz...?amount=1000&seal=utxob:...",
  "contract_id": "contract:B__d4erz...",
  "amount": 1000,
  "seal_utxo": "utxob:..."
}
```

#### Send Transfer
```http
POST /wallet/:name/send-transfer
Content-Type: application/json

{
  "invoice": "rgb:contract:bitcoin:testnet@...",
  "fee_rate_sat_vb": 1
}

Response:
{
  "bitcoin_txid": "abc123...",
  "consignment_download_url": "/api/consignment/transfer_..._1234567890.rgbc",
  "consignment_filename": "transfer_..._1234567890.rgbc",
  "status": "broadcasted"
}
```

#### Accept Consignment
```http
POST /wallet/:name/accept-consignment
Content-Type: multipart/form-data

file: <consignment.rgbc>

Response:
{
  "contract_id": "contract:B__d4erz...",
  "status": "confirmed",
  "import_type": "transfer",
  "bitcoin_txid": "abc123..."
}
```

#### Export Genesis
```http
GET /wallet/:name/export-genesis/:contract_id

Response:
{
  "contract_id": "contract:B__d4erz...",
  "consignment_filename": "genesis_..._1234567890.rgbc",
  "file_size_bytes": 6543,
  "download_url": "/api/genesis/genesis_..._1234567890.rgbc"
}
```

#### Download Consignment
```http
GET /api/consignment/:filename

Response: Binary file (application/octet-stream)
```

---

## User Flows

### 1. Issue RGB20 Asset

**Prerequisites:** Wallet with confirmed UTXO

**Steps:**
1. Navigate to wallet page
2. Click "🪙 Issue Asset"
3. Fill in form:
   - Name: "Test Token" (2-12 chars)
   - Ticker: "TEST" (2-8 chars)
   - Precision: 0-10
   - Supply: Total token supply
   - Genesis UTXO: Select from available UTXOs
4. Click "Issue Asset"
5. Wait for confirmation (~3-5 seconds)
6. Asset appears in RGB Assets section

**Technical Details:**
- Uses RGB20 Fixed Nominal Allocation (FNA) schema
- Genesis UTXO becomes occupied (can't be spent for Bitcoin)
- All tokens initially allocated to wallet
- Transaction broadcast to Signet

### 2. Transfer RGB Tokens

**Flow A: Sender (wallet-1)**

1. **Recipient generates invoice:**
   - Recipient opens wallet-2
   - Clicks "📨 Receive" on the asset
   - Enters amount (e.g., 100 tokens)
   - Copies generated invoice

2. **Sender pays invoice:**
   - Opens wallet-1
   - Clicks "📤 Send" (generic send button at top)
   - Pastes invoice
   - Sets fee rate (optional)
   - Clicks "Send Transfer"
   - Waits for broadcast (~5 seconds)
   - Downloads consignment file
   - Shares file with recipient (manual step)

3. **Background sync:**
   - Frontend auto-calls `syncRgb()` after transfer
   - Balance updates within 3-5 seconds
   - Refresh page to see updated balance

**Flow B: Recipient (wallet-2)**

1. **Import consignment:**
   - Receives consignment file from sender
   - Clicks "📥 Import Consignment"
   - Uploads file
   - Waits for import (~2-3 seconds)
   - Sees confirmation with TX details

2. **Verification:**
   - Frontend refreshes balance automatically
   - Asset appears with correct balance
   - Can click Bitcoin TX link to view on Mempool.space
   - Status shows: "pending" → "confirmed" after 1 confirmation

### 3. Same-Wallet Sync (Genesis Export)

**Use Case:** Sync contract knowledge across devices with same wallet

**Steps:**

1. **Export from Device A:**
   - Open wallet-1 on Device A
   - Find asset in RGB Assets section
   - Click "📦 Export"
   - Verify asset has allocations
   - Download genesis consignment file

2. **Import to Device B:**
   - Open wallet-1 on Device B (same mnemonic!)
   - Click "📥 Import Consignment"
   - Upload genesis file
   - Asset appears with same balance (same keys!)

**Important:** This is NOT for sending tokens. Both wallets must use the same mnemonic. For sending tokens between different wallets, use the transfer flow.

### 4. Fund New Wallet for Receiving

**Problem:** wallet-2 has 0 Bitcoin, can't generate invoices

**Solution:**

1. **Get wallet-2 address:**
   - Open wallet-2
   - Copy primary receive address from "Receive Bitcoin" section

2. **Send Bitcoin from wallet-1:**
   - Open wallet-1
   - Click "💸 Send Bitcoin"
   - Paste wallet-2's address
   - Enter amount (e.g., 10000 sats)
   - Click "Send Bitcoin"
   - Wait for confirmation (~10-30 minutes on Signet)

3. **Verify receipt:**
   - wallet-2 automatically updates balance
   - Can now generate invoices

**Alternative:** Use Signet faucet (https://signetfaucet.com/)

---

## Technical Details

### Bitcoin Key Derivation

**BIP39 Mnemonic:** 12-word seed phrase  
**BIP32 Master Key:** Derived from mnemonic seed  
**BIP84 Path:** `m/84'/1'/0'/0/<index>`  
- `84'` = SegWit native (P2WPKH)
- `1'` = Signet (testnet)
- `0'` = Account 0
- `0/` = External chain (receive addresses)

**Address Generation:**
```rust
use bitcoin::bip32::Xpriv;
let master_key = Xpriv::new_master(Network::Signet, &seed)?;
let path = DerivationPath::from_str("m/84'/1'/0'/0/0")?;
let derived_key = master_key.derive_priv(&secp, &path)?;
let pubkey = PublicKey::from_private_key(&secp, &derived_key.to_priv());
let address = Address::p2wpkh(&pubkey, Network::Signet)?;
```

### Transaction Signing

**Multi-Key Signing:**
Each UTXO may belong to a different address index, requiring different private keys.

```rust
for (input_index, input) in tx.input.iter().enumerate() {
    // Find corresponding UTXO
    let utxo = find_utxo_for_input(input)?;
    
    // Derive correct key for this UTXO's address index
    let private_key = derive_key_for_index(mnemonic, utxo.address_index)?;
    
    // Create ECDSA signature
    let sighash = calculate_p2wpkh_sighash(tx, input_index, &utxo)?;
    let signature = secp.sign_ecdsa(&sighash, &private_key)?;
    
    // Add to witness
    tx.input[input_index].witness.push(signature);
    tx.input[input_index].witness.push(pubkey);
}
```

### RGB State Transitions

**Deterministic Bitcoin Commitments (DBC):**
RGB state transitions are committed to Bitcoin transactions using DBC protocol.

**Process:**
1. Create RGB state transition (e.g., transfer 100 tokens)
2. Compute commitment hash of the transition
3. Embed hash in Bitcoin transaction using OP_RETURN or Taproot
4. Broadcast Bitcoin transaction
5. Share consignment (off-chain proof) with recipient

**State Validation:**
Recipient validates by:
1. Checking Bitcoin transaction is confirmed
2. Verifying commitment matches consignment
3. Validating all state transitions from genesis
4. Confirming final state matches their expected receipt

### Sync Strategies

**Ephemeral Runtime Approach:**

The wallet creates fresh runtimes for each operation and syncs on-demand:

**1. Load from Disk (Fast - < 100ms):**
```rust
// Create runtime - loads existing state from disk
let runtime = rgb_runtime_manager.init_runtime_no_sync(wallet_name)?;
```
- Loads stockpile/stash/indexer from disk
- No blockchain queries
- State may be slightly stale
- Used when: Initial runtime creation

**2. Sync Before Operation (Adaptive - 3-10s):**
```rust
// Sync with 1 confirmation (fast)
runtime.update(1)?;
```
- Queries Esplora for recent blocks
- Updates UTXO set and witness status
- Scans from last known block height
- Used when: Before transfers, before balance queries

**3. Explicit User Sync:**
```rust
// User-triggered sync via "Sync" button
pub async fn sync_rgb_runtime(&self, wallet_name: &str) -> Result<()> {
    let mut runtime = self.rgb_runtime_manager.init_runtime_no_sync(wallet_name)?;
    runtime.update(1)?;
    Ok(())
    // Runtime drops → saves updated state
}
```

**When Syncs Happen:**
- ✅ Before creating RGB transfers (ensures fresh UTXO set)
- ✅ During balance queries (gets latest allocations)
- ✅ When user clicks "Sync" button
- ✅ After accepting consignments (implicit on next balance query)
- ✅ After issuing assets (implicit on next balance query)

**Why 1 Confirmation?**
- Fast enough for good UX (3-5 seconds)
- Sufficient for testnet/signet
- Production mainnet may want 6+ confirmations
- Configurable via `runtime.update(confirmations)`

**Auto-Save on Drop:**
```rust
{
    let mut runtime = create_runtime()?;
    runtime.update(1)?;
    runtime.do_operation()?;
} // Runtime drops here → FileHolder::drop() auto-saves to disk
```

### UTXO Selection

**Strategy for RGB Transfers:**
```rust
CoinselectStrategy::Aggregate
```
- Aggregates multiple UTXOs if needed
- Ensures sufficient token amount
- Considers Bitcoin fees
- Creates change outputs

**Strategy for Bitcoin Sends:**
```rust
// Simple first-fit algorithm
let mut selected = Vec::new();
let mut total = 0;
for utxo in utxos {
    if !utxo.is_occupied && utxo.confirmations > 0 {
        selected.push(utxo);
        total += utxo.amount_sats;
        if total >= amount + estimated_fee {
            break;
        }
    }
}
```

### Error Handling

**Error Types:**

```rust
pub enum WalletError {
    WalletNotFound(String),
    InvalidInput(String),
    InsufficientFunds(String),
    Bitcoin(String),
    Rgb(String),
    Network(String),
    Esplora(String),
    Storage(String),
    Internal(String),
}
```

**HTTP Error Responses:**

```rust
impl IntoResponse for WalletError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            WalletError::WalletNotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            WalletError::InvalidInput(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            WalletError::InsufficientFunds(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };
        
        (status, Json(json!({ "error": message }))).into_response()
    }
}
```

---

## Known Issues and Limitations

### Current Limitations

1. **Single RGB Transfer at a Time**
   - Can only send one asset per transaction
   - Workaround: Create multiple transactions

2. **No Batch Operations**
   - Must send transfers to multiple recipients separately
   - Each requires separate consignment file

3. **Manual Consignment Sharing**
   - Files must be manually downloaded and shared
   - No built-in P2P or messaging
   - Future: IPFS or nostr integration

4. **Balance Updates Require Sync**
   - After receiving transfer, may need to wait 3-5 seconds
   - Frontend auto-syncs but user may need to refresh

5. **No Transaction History**
   - Only shows current balance
   - No list of past transfers
   - Future: Add transaction history view

6. **Network Configuration**
   - ✅ Now supports Signet and Regtest via environment variables
   - Mainnet support requires:
     - Set `BITCOIN_NETWORK=bitcoin` (infrastructure ready)
     - Configure mainnet Esplora URL
     - Increase confirmation requirements to 6+
     - Additional security audit

7. **No Dynamic Fee Estimation**
   - Uses fixed estimates (250 sats for RGB, calculated for Bitcoin)
   - Future: Query mempool for dynamic fees

8. **Limited Error Recovery**
   - If transfer fails after broadcast, need manual recovery
   - Future: Add transaction rebroadcast and CPFP

9. **Ephemeral Runtime Performance** (New in October 2025)
   - Each operation creates fresh runtime from disk
   - May be slower than cached approach for rapid operations
   - Trade-off: Simplicity and reliability vs potential performance
   - Needs benchmarking against previous cached approach

10. **Orphaned Code** (New in October 2025)
   - Files `rgb_runtime_cache.rs` and `rgb_lifecycle.rs` exist but unused
   - Should be removed or marked deprecated
   - Total ~20KB of dead code

11. **Firefly Security** (New in October 2025)
   - Bootstrap private key hardcoded in source
   - OK for testing, **MUST** move to env var for production
   - Key: `5f668a7ee96d944a4494cc947e4005e172d7ab3461ee5538f1f2a45a835e9657`

### Edge Cases Handled

✅ **Re-importing Same Consignment**
- Detects if contract already exists
- Handles gracefully without error
- Shows "imported" status

✅ **Genesis Export Without Allocations**
- Checks for allocations before export
- Returns clear error message

✅ **Invoice Generation Without UTXOs**
- Pre-checks for Bitcoin UTXOs
- Shows helpful error with steps to fix

✅ **Sending More Than Balance**
- Validates balance before transfer
- Returns "insufficient funds" error

✅ **Locked/Occupied UTXO Handling**
- Excludes RGB-occupied UTXOs from Bitcoin sends
- Shows lock icon in UTXO list

### Security Considerations

⚠️ **Production Requirements:**

1. **Mnemonic Storage:**
   - Currently stored in plain text
   - **MUST** encrypt before production
   - Consider hardware wallet integration

2. **Firefly Private Key:** (New in October 2025)
   - Currently hardcoded in `firefly/client.rs`
   - **MUST** move to environment variable or key store
   - Recommendation: `export FIREFLY_PRIVATE_KEY="..."`
   - Consider per-user signing keys instead of shared validator key

3. **Key Derivation:**
   - Uses BIP39/BIP32/BIP84 correctly
   - Seeds should never be logged
   - ✅ Fixed BIP32 path bug (October 2025)

4. **Network Security:**
   - ✅ CORS now configurable via `ALLOWED_ORIGINS` env var
   - ✅ Server bind address configurable via `BIND_ADDRESS`
   - ✅ Public URL configurable for download links
   - **MUST** use HTTPS in production
   - **MUST** add authentication/authorization

5. **Input Validation:**
   - Validates addresses, amounts, fee rates
   - Sanitizes file uploads
   - Prevents path traversal in file operations

6. **Confirmation Requirements:**
   - Currently 1 confirmation for quick testing/signet
   - **MUST** increase to 6+ for mainnet
   - Configurable via `runtime.update(confirmations)`

---

## Future Improvements

### Immediate Actions (Post-Refactor Cleanup)

1. **Remove Orphaned Code**
   - Delete `wallet/src/wallet/shared/rgb_runtime_cache.rs` (~13KB)
   - Delete `wallet/src/wallet/shared/rgb_lifecycle.rs` (~7KB)
   - Or mark with `#[deprecated]` attribute if keeping for reference
   - Update mod.rs to remove imports

2. **Security Hardening**
   - Move Firefly private key to `FIREFLY_PRIVATE_KEY` env var
   - Implement mnemonic encryption at rest
   - Add rate limiting to API endpoints
   - Document HTTPS setup for production

3. **Performance Benchmarking**
   - Benchmark ephemeral vs previous cached approach
   - Measure balance query latency
   - Test rapid consecutive transfers
   - Optimize if needed (consider read-only cache)

4. **Integration Testing**
   - Add end-to-end transfer tests on regtest
   - Test multi-wallet scenarios
   - Test error recovery paths
   - Firefly deployment testing

5. **Documentation Updates**
   - ✅ Update main implementation docs (this file)
   - Add deployment guide (Docker, AWS, Vercel)
   - Document environment variables
   - Add troubleshooting guide

### Short Term (Phase 5)

1. **Transaction History**
   - List past Bitcoin and RGB transactions
   - Show status, timestamps, amounts
   - Link to block explorers

2. **Better Fee Estimation**
   - Query mempool for current fee rates
   - Show estimated confirmation time
   - Allow user to adjust priority

3. **Contact Management**
   - Save commonly used addresses
   - Address book with labels
   - Quick send to contacts

4. **Notification System**
   - Toast notifications for success/errors
   - Progress indicators for long operations
   - Desktop notifications for received funds

5. **Export/Backup**
   - Encrypted wallet backup
   - Export transaction history
   - QR codes for addresses/invoices

### Medium Term (Phase 6)

1. **Multi-Sig Support**
   - 2-of-3 multisig wallets
   - Collaborative spending
   - Hardware wallet integration

2. **Advanced RGB Features**
   - RGB21 NFT support
   - RGB25 collectibles
   - Custom contract schemas

3. **Lightning Network**
   - Open channels
   - Send/receive Lightning payments
   - RGB over Lightning (experimental)

4. **DeFi Integration**
   - Atomic swaps
   - DEX integration
   - Lending/borrowing protocols

### Long Term (Phase 7+)

1. **Mobile Applications**
   - iOS and Android apps
   - React Native or native
   - Simplified UX for mobile

2. **Hardware Wallet Support**
   - Ledger integration
   - Trezor integration
   - PSBT signing via USB

3. **Privacy Features**
   - CoinJoin integration
   - Tor support
   - Stealth addresses

4. **Enterprise Features**
   - Multi-user wallets
   - Role-based access control
   - Audit logs
   - Compliance reporting

---

## Conclusion

The RGB wallet implementation is a **production-ready foundation** for Bitcoin and RGB asset management with recent architectural improvements. It demonstrates:

✅ Full RGB 0.12 integration with native invoice support  
✅ Ephemeral runtime pattern matching RGB CLI (proven reliable)  
✅ Complete Bitcoin wallet functionality  
✅ Modular architecture with clear separation of concerns  
✅ Environment-based configuration for flexible deployment  
✅ Firefly/RChain integration for RGB-Rholang bridge  
✅ Comprehensive error handling and user feedback  
✅ Extensible architecture for future features  

**Current State:**
- **Backend:** ~6,017 total lines across 31 Rust files
  - Manager: 548 lines (orchestration only)
  - Operation modules: 7 focused modules
  - Shared utilities: 11 modules
- **Frontend:** 459 lines (WalletDetail.tsx) + 8 modal components
- **RGB Modifications:** 3 new methods in rgb-std contracts.rs
- **API Endpoints:** 18+ endpoints covering all operations
- **Test Networks:** Fully functional on Bitcoin Signet and Regtest

**Architecture Highlights:**
- ✅ **Ephemeral Runtimes** - Each operation creates fresh runtime, no caching complexity
- ✅ **Modular Design** - Clear separation: wallet_ops, bitcoin_ops, rgb_transfer_ops, etc.
- ✅ **Configuration Management** - Environment-based config for deployment flexibility
- ✅ **Firefly Integration** - Deploy RGB contracts to Rholang blockchain
- ✅ **Network Abstraction** - Support for Signet, Regtest, and future Mainnet

**Ready For:**
- ✅ RGB20 token operations on Signet/Regtest
- ✅ Bitcoin transactions with proper fee estimation
- ✅ Multi-wallet management
- ✅ Same-wallet sync across devices
- ✅ End-to-end transfers with consignments
- ✅ Firefly/RChain deployments (testing)
- ✅ Cloud deployment (AWS, Docker, Vercel, etc.)

**Requires Before Mainnet:**
- ⚠️ **Security:** Mnemonic encryption in storage
- ⚠️ **Security:** Move Firefly private key to env var/key store
- ⚠️ **Network:** HTTPS and proper authentication
- ⚠️ **RGB:** Increase confirmation requirements (6+ for mainnet)
- ⚠️ **Bitcoin:** Dynamic fee estimation from mempool
- ⚠️ **Testing:** Comprehensive integration tests on mainnet
- ⚠️ **Performance:** Benchmark ephemeral vs cached runtime approach
- ⚠️ **Cleanup:** Remove unused rgb_runtime_cache.rs and rgb_lifecycle.rs files

**Recent Changes (October 2025):**
- 🔄 **Architecture:** Shifted from cached runtimes to ephemeral pattern
- 📦 **Modular Code:** Refactored manager.rs (1,275 → 548 lines) into focused modules
- ⚙️ **Configuration:** Added environment-based config with CORS, bind address, public URL
- 🔥 **Firefly:** Integrated Firefly/RChain gRPC client for deploying contracts
- 🐛 **Bug Fixes:** Fixed BIP32 derivation path, balance query logic, download URLs
- 🧪 **Testing:** Added integration tests for RGB transfers

**Next Steps:**
1. Remove orphaned cache/lifecycle files
2. Move Firefly private key to environment variable
3. Run performance benchmarks (ephemeral vs cached)
4. Add comprehensive integration tests
5. Security audit for production readiness

---

**Documentation Version:** 2.0 (Post-Refactor)  
**Last Updated:** October 28, 2025  
**Author:** Development Team  
**License:** See project LICENSE file

