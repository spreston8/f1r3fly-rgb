# RGB Asset Transfer Implementation Plan

**Status**: 📋 Planning Complete - Ready for Review  
**Target**: Full RGB transfer functionality (invoice → send → receive)  
**Approach**: Native RGB Runtime integration (no CLI)  
**Date**: October 11, 2025

---

## Table of Contents
1. [Overview](#overview)
2. [Prerequisites](#prerequisites)
3. [Phase 1: RGB Runtime Foundation](#phase-1-rgb-runtime-foundation)
4. [Phase 2: Invoice Generation (Recipient)](#phase-2-invoice-generation-recipient)
5. [Phase 3: Send Transfer (Sender)](#phase-3-send-transfer-sender)
6. [Phase 4: Consignment Import/Export](#phase-4-consignment-importexport)
   - [Phase 4A: Genesis Export/Import (Same Wallet Sync)](#phase-4a-genesis-exportimport-same-wallet-sync)
   - [Phase 4B: Accept Consignment (Transfer & Genesis Import)](#phase-4b-accept-consignment-transfer--genesis-import)
7. [Phase 5: Frontend Integration](#phase-5-frontend-integration)
8. [Phase 6: Testing & Polish](#phase-6-testing--polish)
9. [Timeline & Effort](#timeline--effort)

---

## Overview

### Goal
Enable complete RGB asset management and transfers:
- ✅ **Same Wallet Sync** - Export/import genesis consignment across devices (no Bitcoin TX)
- ✅ **Transfer Between Wallets** - Full send/receive flow with consignments
  - Recipient generates invoice (with blinded seal)
  - Sender sends payment (creates PSBT + consignment)
  - Recipient accepts consignment (validates + imports)

### Approach
Use **RGB Runtime** (`RgbWallet` + `RgbpRuntimeDir`) for native library integration instead of CLI spawning.

### Key Deliverables
1. Genesis export/import for same-wallet device sync
2. Backend APIs for invoice/send/receive transfers
3. Frontend UI for both genesis sync and transfer flows
4. Manual consignment file sharing (download/upload)
5. End-to-end tested transfer cycle

---

## Prerequisites

### ✅ Already Have
- RGB data directory (`./wallets/rgb_data/`)
- RGB occupation detection (Phase 3 complete)
- Asset issuance working
- PSBT signing logic
- Esplora integration

### ⚠️ Need to Build
- RGB Runtime initialization per wallet
- FileHolder integration (descriptor + UTXO persistence)
- MemUtxos population from Esplora
- MultiResolver wrapper
- Transfer API endpoints

---

## Phase 1: RGB Runtime Foundation

**Goal**: Initialize RGB Runtime for any wallet on-demand

**Duration**: 3-4 days

**Confidence**: 7.5/10

---

### Step 1.1: Add RGB Runtime Dependencies

**File**: `wallet/Cargo.toml`

**Changes**:
```toml
[dependencies]
# Existing RGB deps (already added for issuance)
rgb = { package = "rgb-runtime", version = "0.12.0-rc.3", path = "../rgb", features = ["fs"] }
rgb-std = { version = "0.12.0-rc.3", path = "../rgb-std" }
rgb-persist-fs = { version = "0.12.0-rc.3", path = "../rgb-std/persistence/fs" }
bpstd = { package = "bp-std", version = "0.12.0-rc.3", path = "../bp-std" }

# NEW: RGB Runtime specific
rgbp = { version = "0.12.0-rc.3", path = "../rgb" }  # Exposes Owner, FileHolder
rgb-descriptors = { version = "0.12.0-rc.3", path = "../rgb/descriptors" }  # For RgbDescr
toml = "0.8"  # For TOML serialization (descriptor.toml, utxo.toml)
```

**Verification**:
```bash
cd wallet && cargo check
```

---

### Step 1.2: Create RGB Runtime Module

**File**: `wallet/src/wallet/rgb_runtime.rs` (NEW)

**Purpose**: Manage RGB Runtime initialization and lifecycle

**Implementation**:
```rust
use std::path::PathBuf;
use std::str::FromStr;
use bpstd::{Network, XpubDerivable, Wpkh, Outpoint, Vout, Sats};
use rgbp::Owner;
use rgbp::owner::file::FileHolder;
use rgbp::resolvers::MultiResolver;
use rgbp::descriptors::RgbDescr;
use rgb::{Contracts, Consensus};
use rgb_persist_fs::StockpileDir;
use bpstd::seals::TxoSeal;
use rgb::popls::bp::RgbWallet;
use rgb::runtime::file::RgbpRuntimeDir;

pub struct RgbRuntimeManager {
    base_path: PathBuf,
    network: Network,
}

impl RgbRuntimeManager {
    pub fn new(base_path: PathBuf, network: Network) -> Self {
        Self { base_path, network }
    }
    
    /// Initialize RGB Runtime for a specific wallet
    pub fn init_runtime(
        &self,
        wallet_name: &str,
    ) -> Result<RgbpRuntimeDir<MultiResolver>, crate::error::WalletError> {
        // 1. Create resolver
        let resolver = self.create_resolver()?;
        
        // 2. Ensure FileHolder exists (create if needed)
        let rgb_wallet_path = self.base_path
            .join(wallet_name)
            .join("rgb_wallet");
        
        let hodler = if rgb_wallet_path.exists() {
            FileHolder::load(rgb_wallet_path)
                .map_err(|e| crate::error::WalletError::Rgb(e.to_string()))?
        } else {
            self.create_file_holder(wallet_name)?
        };
        
        // 3. Create Owner
        let owner = Owner::with_components(self.network, hodler, resolver);
        
        // 4. Load Contracts (shared RGB data)
        let contracts = self.load_contracts()?;
        
        // 5. Create RgbWallet
        let rgb_wallet = RgbWallet::with_components(owner, contracts);
        
        // 6. Wrap in RgbRuntime
        let mut runtime = RgbpRuntimeDir::from(rgb_wallet);
        
        // 7. Sync wallet with blockchain
        runtime.update(32)  // 32 confirmations
            .map_err(|e| crate::error::WalletError::Rgb(format!("Sync failed: {:?}", e)))?;
        
        Ok(runtime)
    }
    
    fn create_resolver(&self) -> Result<MultiResolver, crate::error::WalletError> {
        MultiResolver::new_esplora("https://mempool.space/signet/api")
            .map_err(|e| crate::error::WalletError::Network(e.to_string()))
    }
    
    fn create_file_holder(
        &self,
        wallet_name: &str,
    ) -> Result<FileHolder, crate::error::WalletError> {
        // Load our descriptor
        let descriptor_path = self.base_path
            .join(wallet_name)
            .join("descriptor.txt");
        let descriptor_str = std::fs::read_to_string(&descriptor_path)
            .map_err(|e| crate::error::WalletError::Storage(e.to_string()))?;
        
        // Convert to RgbDescr
        let rgb_descr = self.descriptor_to_rgb(&descriptor_str)?;
        
        // Create FileHolder directory
        let rgb_wallet_path = self.base_path
            .join(wallet_name)
            .join("rgb_wallet");
        
        FileHolder::create(rgb_wallet_path, rgb_descr)
            .map_err(|e| crate::error::WalletError::Rgb(e.to_string()))
    }
    
    fn descriptor_to_rgb(
        &self,
        descriptor: &str,
    ) -> Result<RgbDescr, crate::error::WalletError> {
        let xpub = XpubDerivable::from_str(descriptor)
            .map_err(|e| crate::error::WalletError::InvalidDescriptor(e.to_string()))?;
        
        let noise = xpub.xpub().chain_code().to_byte_array();
        
        Ok(RgbDescr::new_unfunded(Wpkh::from(xpub), noise))
    }
    
    fn load_contracts(&self) -> Result<Contracts<StockpileDir<TxoSeal>>, crate::error::WalletError> {
        let rgb_data_dir = self.base_path.join("rgb_data");
        let stockpile = StockpileDir::load(&rgb_data_dir, Consensus::Bitcoin, true)
            .map_err(|e| crate::error::WalletError::Rgb(format!("Failed to load stockpile: {:?}", e)))?;
        
        Ok(Contracts::load(stockpile))
    }
}
```

**Test**:
- Create wallet
- Initialize runtime
- Verify `./wallets/{name}/rgb_wallet/` created with `descriptor.toml` and `utxo.toml`

---

### Step 1.3: Integrate into WalletManager

**File**: `wallet/src/wallet/manager.rs`

**Changes**:
```rust
use super::rgb_runtime::RgbRuntimeManager;

pub struct WalletManager {
    pub storage: Storage,
    balance_checker: BalanceChecker,
    pub rgb_manager: RgbManager,
    rgb_runtime_manager: RgbRuntimeManager,  // NEW
}

impl WalletManager {
    pub fn new() -> Self {
        let storage = Storage::new();
        let rgb_data_dir = storage.base_dir().join("rgb_data");
        let rgb_manager = RgbManager::new(rgb_data_dir.clone(), bpstd::Network::Signet)
            .expect("Failed to initialize RGB manager");
        
        let rgb_runtime_manager = RgbRuntimeManager::new(
            storage.base_dir().clone(),
            bpstd::Network::Signet,
        );
        
        Self {
            storage,
            balance_checker: BalanceChecker::new(),
            rgb_manager,
            rgb_runtime_manager,
        }
    }
    
    // Helper method for other modules
    pub(crate) fn get_runtime(
        &self,
        wallet_name: &str,
    ) -> Result<RgbpRuntimeDir<MultiResolver>, crate::error::WalletError> {
        self.rgb_runtime_manager.init_runtime(wallet_name)
    }
}
```

---

### Step 1.4: Handle MemUtxos Synchronization

**Challenge**: RGB's FileHolder auto-saves UTXOs to `utxo.toml`, but it needs to be populated from Esplora.

**Solution**: Sync UTXOs before RGB operations

**File**: `wallet/src/wallet/rgb_runtime.rs`

**Add method**:
```rust
impl RgbRuntimeManager {
    /// Sync UTXOs from Esplora to RGB's MemUtxos
    pub async fn sync_utxos(
        &self,
        wallet_name: &str,
    ) -> Result<(), crate::error::WalletError> {
        // This is handled by runtime.update() during initialization
        // FileHolder auto-saves on drop, so UTXOs persist to utxo.toml
        Ok(())
    }
}
```

**Note**: RGB's `runtime.update()` handles UTXO synchronization internally via the resolver.

---

### Phase 1 Deliverables

- ✅ RGB Runtime dependencies added
- ✅ `RgbRuntimeManager` module created
- ✅ Runtime initialization working
- ✅ `FileHolder` created per wallet
- ✅ `descriptor.toml` and `utxo.toml` persisted
- ✅ Contracts loaded from shared `rgb_data/`

**Test Checklist**:
- [ ] Create new wallet → RGB Runtime initializes
- [ ] Import existing wallet → RGB Runtime initializes
- [ ] Verify `./wallets/{name}/rgb_wallet/descriptor.toml` exists
- [ ] Verify `./wallets/{name}/rgb_wallet/utxo.toml` exists
- [ ] Run `cargo test` (if tests added)

---

## Phase 2: Invoice Generation (Recipient) ✅ **COMPLETE**

**Goal**: Recipient can generate RGB invoice to receive assets

**Duration**: 2 days

**Confidence**: 8/10

**Status**: ✅ Implemented and tested

---

### Step 2.1: Backend API - Generate Invoice

**File**: `wallet/src/api/types.rs`

**Add structs**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateInvoiceRequest {
    pub contract_id: String,  // "rgb:abc123..."
    pub amount: u64,           // Token amount to receive
    pub nonce: Option<u8>,     // For multiple invoices (default: 0)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateInvoiceResponse {
    pub invoice: String,       // Full invoice string
    pub amount: u64,
    pub contract_id: String,
    pub seal_outpoint: String, // The UTXO seal used (txid:vout)
}
```

**File**: `wallet/src/wallet/manager.rs`

**Add method**:
```rust
impl WalletManager {
    pub fn generate_invoice(
        &self,
        wallet_name: &str,
        request: GenerateInvoiceRequest,
    ) -> Result<GenerateInvoiceResponse, crate::error::WalletError> {
        // 1. Initialize runtime
        let mut runtime = self.get_runtime(wallet_name)?;
        
        // 2. Parse contract ID
        use rgb::ContractId;
        let contract_id = ContractId::from_str(&request.contract_id)
            .map_err(|e| crate::error::WalletError::InvalidInput(e.to_string()))?;
        
        // 3. Get auth token (seal from existing UTXO)
        let nonce = request.nonce.unwrap_or(0);
        let auth = runtime.auth_token(nonce)
            .ok_or_else(|| crate::error::WalletError::Rgb(
                "No unspent outputs available for seal".to_string()
            ))?;
        
        // 4. Create beneficiary
        use rgb::invoice::RgbBeneficiary;
        let beneficiary = RgbBeneficiary::Token(auth);
        
        // 5. Create invoice
        use rgb::invoice::RgbInvoice;
        use rgb::{CallScope, Consensus};
        use strict_types::StrictVal;
        
        let invoice = RgbInvoice::new(
            CallScope::ContractId(contract_id),
            Consensus::Bitcoin,
            true,  // testnet/signet
            beneficiary,
            Some(StrictVal::num(request.amount)),
        );
        
        // 6. Get seal outpoint for display
        let seal_outpoint = format!("{}:{}", auth.txid(), auth.vout());
        
        Ok(GenerateInvoiceResponse {
            invoice: invoice.to_string(),
            amount: request.amount,
            contract_id: request.contract_id,
            seal_outpoint,
        })
    }
}
```

**File**: `wallet/src/api/handlers.rs`

**Add handler**:
```rust
pub async fn generate_invoice_handler(
    State(manager): State<Arc<WalletManager>>,
    Path(name): Path<String>,
    Json(req): Json<GenerateInvoiceRequest>,
) -> Result<Json<GenerateInvoiceResponse>, crate::error::WalletError> {
    if !manager.storage.wallet_exists(&name) {
        return Err(crate::error::WalletError::WalletNotFound(name));
    }
    
    let result = manager.generate_invoice(&name, req)?;
    Ok(Json(result))
}
```

**File**: `wallet/src/api/server.rs`

**Add route**:
```rust
.route("/api/wallet/:name/generate-invoice", post(handlers::generate_invoice_handler))
```

---

### Step 2.2: Frontend UI - Generate Invoice

**File**: `wallet-frontend/src/components/GenerateInvoiceModal.tsx` (NEW)

**Implementation**:
```tsx
import { useState } from 'react';
import { walletApi } from '../api/wallet';
import type { GenerateInvoiceRequest, GenerateInvoiceResponse } from '../api/types';
import { copyToClipboard } from '../utils/format';

interface GenerateInvoiceModalProps {
  walletName: string;
  contractId: string;    // Pre-filled from asset selection
  isOpen: boolean;
  onClose: () => void;
}

export default function GenerateInvoiceModal({
  walletName,
  contractId,
  isOpen,
  onClose,
}: GenerateInvoiceModalProps) {
  const [amount, setAmount] = useState('');
  const [invoice, setInvoice] = useState<GenerateInvoiceResponse | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  const handleGenerate = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setIsLoading(true);

    try {
      const request: GenerateInvoiceRequest = {
        contract_id: contractId,
        amount: parseInt(amount),
        nonce: 0,
      };

      const response = await walletApi.generateInvoice(walletName, request);
      setInvoice(response);
    } catch (err: any) {
      setError(err.response?.data?.error || err.message || 'Failed to generate invoice');
    } finally {
      setIsLoading(false);
    }
  };

  const handleCopy = async () => {
    if (invoice) {
      await copyToClipboard(invoice.invoice);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center p-4 z-50">
      <div className="bg-white dark:bg-gray-800 rounded-lg p-6 max-w-2xl w-full max-h-[90vh] overflow-y-auto">
        <h2 className="text-2xl font-bold text-gray-900 dark:text-white mb-4">
          Generate Invoice
        </h2>

        {!invoice ? (
          <form onSubmit={handleGenerate}>
            <div className="mb-4">
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                Amount to Receive
              </label>
              <input
                type="number"
                value={amount}
                onChange={(e) => setAmount(e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md 
                         bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                placeholder="Enter amount"
                required
                min="1"
              />
            </div>

            {error && (
              <div className="mb-4 p-3 bg-red-50 dark:bg-red-900/30 border border-red-200 
                            dark:border-red-700 rounded-md text-red-800 dark:text-red-300">
                {error}
              </div>
            )}

            <div className="flex justify-end space-x-3">
              <button
                type="button"
                onClick={onClose}
                className="px-4 py-2 text-gray-700 dark:text-gray-300 hover:bg-gray-100 
                         dark:hover:bg-gray-700 rounded-md transition-colors"
                disabled={isLoading}
              >
                Cancel
              </button>
              <button
                type="submit"
                className="px-4 py-2 bg-blue-600 hover:bg-blue-700 dark:bg-blue-500 
                         dark:hover:bg-blue-600 text-white rounded-md transition-colors 
                         disabled:opacity-50"
                disabled={isLoading}
              >
                {isLoading ? 'Generating...' : 'Generate Invoice'}
              </button>
            </div>
          </form>
        ) : (
          <div>
            <div className="mb-4 p-4 bg-green-50 dark:bg-green-900/30 border border-green-200 
                          dark:border-green-700 rounded-md">
              <h3 className="text-lg font-semibold text-green-800 dark:text-green-300 mb-2">
                ✅ Invoice Generated
              </h3>
              <p className="text-sm text-green-700 dark:text-green-400">
                Share this invoice with the sender
              </p>
            </div>

            <div className="mb-4">
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                Invoice String
              </label>
              <div className="relative">
                <textarea
                  value={invoice.invoice}
                  readOnly
                  rows={4}
                  className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md 
                           bg-gray-50 dark:bg-gray-700 text-gray-900 dark:text-white font-mono text-sm"
                />
                <button
                  onClick={handleCopy}
                  className="absolute top-2 right-2 px-3 py-1 bg-blue-600 hover:bg-blue-700 
                           dark:bg-blue-500 dark:hover:bg-blue-600 text-white text-sm rounded 
                           transition-colors"
                >
                  {copied ? '✓ Copied' : 'Copy'}
                </button>
              </div>
            </div>

            <div className="mb-4 text-sm text-gray-600 dark:text-gray-400 space-y-1">
              <p><strong>Amount:</strong> {invoice.amount} tokens</p>
              <p><strong>Seal UTXO:</strong> {invoice.seal_outpoint}</p>
            </div>

            <div className="flex justify-end">
              <button
                onClick={onClose}
                className="px-4 py-2 bg-gray-600 hover:bg-gray-700 dark:bg-gray-500 
                         dark:hover:bg-gray-600 text-white rounded-md transition-colors"
              >
                Done
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
```

---

### Step 2.3: Frontend Integration

**File**: `wallet-frontend/src/pages/WalletDetail.tsx`

**Add state and button**:
```tsx
const [showGenerateInvoiceModal, setShowGenerateInvoiceModal] = useState(false);
const [selectedContractId, setSelectedContractId] = useState<string | null>(null);

// In the assets display section:
<button
  onClick={() => {
    setSelectedContractId(asset.contract_id);
    setShowGenerateInvoiceModal(true);
  }}
  className="px-3 py-1 bg-green-600 hover:bg-green-700 text-white text-sm rounded"
>
  Receive
</button>

// At the bottom:
{selectedContractId && (
  <GenerateInvoiceModal
    walletName={name || ''}
    contractId={selectedContractId}
    isOpen={showGenerateInvoiceModal}
    onClose={() => {
      setShowGenerateInvoiceModal(false);
      setSelectedContractId(null);
    }}
  />
)}
```

---

### Phase 2 Deliverables

- ✅ Backend API: `POST /api/wallet/:name/generate-invoice`
- ✅ Backend uses Bitlight-compatible format
- ✅ Frontend: `GenerateInvoiceModal` component
- ✅ Frontend: RGB Assets section with "Receive" buttons
- ✅ Invoice string displayed with copy button
- ✅ Seal UTXO shown for reference
- ✅ Both backend and frontend compile successfully

**Test Checklist**:
- [ ] Generate invoice for existing asset
- [ ] Copy invoice string
- [ ] Verify invoice format (starts with `contract:tb@`)
- [ ] Error handling (no UTXOs available)
- [ ] Amount is required (Bitlight-style UX)

---

## Phase 3: Send Transfer (Sender)

**Goal**: Sender can send RGB assets using an invoice

**Duration**: 2-3 days *(Updated after deep research)*

**Confidence**: 8.5/10 *(Updated after deep research)*

**Status**: ✅ Research Complete - Ready for Implementation

**Key Research Findings** *(See `rgb-runtime-research-findings.md` Phase 3 section for details)*:
- ✅ `pay_invoice()` already includes DBC commit (no separate `complete()` call needed)
- ✅ Consignment must be generated **BEFORE** signing (not after broadcasting)
- ✅ Use `Signer` trait for PSBT signing
- ✅ Broadcasting is trivial: `format!("{:x}", tx)`

---

### Step 3.1: Backend API - Send Transfer

**File**: `wallet/src/api/types.rs`

**Add structs**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendTransferRequest {
    pub invoice: String,             // Full invoice string
    pub fee_rate_sat_vb: Option<u64>, // Default: 2
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendTransferResponse {
    pub bitcoin_txid: String,
    pub consignment_download_url: String,  // Frontend serves this file
    pub consignment_filename: String,
    pub pending_status: String,  // "pending" or "broadcasting"
}
```

**File**: `wallet/src/wallet/manager.rs`

**Add method** *(Corrected workflow based on research)*:
```rust
impl WalletManager {
    pub fn send_transfer(
        &self,
        wallet_name: &str,
        request: SendTransferRequest,
    ) -> Result<SendTransferResponse, crate::error::WalletError> {
        // 1. Parse invoice
        use rgb::invoice::RgbInvoice;
        let invoice = RgbInvoice::from_str(&request.invoice)
            .map_err(|e| crate::error::WalletError::InvalidInput(format!("Invalid invoice: {:?}", e)))?;
        
        // 2. Initialize runtime
        let mut runtime = self.get_runtime(wallet_name)?;
        
        // 3. Create payment (PSBT + RGB state)
        // NOTE: pay_invoice() internally calls complete() which does DBC commit
        // The returned PSBT is ready for signing!
        use bpstd::psbt::TxParams;
        use rgb::CoinselectStrategy;
        use bpstd::Sats;
        
        let params = TxParams::with(request.fee_rate_sat_vb.unwrap_or(2));
        let (mut psbt, payment) = runtime.pay_invoice(
            &invoice,
            CoinselectStrategy::Accumulative,
            params,
            Some(Sats::from_sats(1000)),  // Min locked amount
        ).map_err(|e| crate::error::WalletError::Rgb(format!("Payment failed: {:?}", e)))?;
        
        // 4. **GENERATE CONSIGNMENT FIRST** (before signing!)
        // This is the correct RGB workflow - consignment doesn't depend on signatures
        let consignment_filename = format!("transfer_{}_{}.rgb", 
            invoice.scope, chrono::Utc::now().timestamp());
        let consignment_path = self.storage.base_dir()
            .join("consignments")
            .join(&consignment_filename);
        
        std::fs::create_dir_all(consignment_path.parent().unwrap())?;
        
        runtime.contracts.consign_to_file(
            &consignment_path,
            invoice.scope,           // contract_id from invoice
            payment.terminals,       // from Payment struct
        ).map_err(|e| crate::error::WalletError::Rgb(format!("Consignment failed: {:?}", e)))?;
        
        // 5. Sign PSBT using Signer trait
        let signer = self.create_signer(wallet_name)?;
        psbt.sign(&signer)
            .map_err(|e| crate::error::WalletError::Bitcoin(format!("Signing failed: {:?}", e)))?;
        
        // 6. Finalize PSBT
        psbt.finalize(runtime.wallet.descriptor());
        
        // 7. Extract signed transaction
        let tx = psbt.extract()
            .map_err(|e| crate::error::WalletError::Rgb(format!("Extraction failed: {:?}", e)))?;
        let txid = tx.txid();
        
        // 8. Broadcast transaction (simple hex formatting)
        let tx_hex = format!("{:x}", tx);
        self.broadcast_tx_hex(&tx_hex)?;
        
        Ok(SendTransferResponse {
            bitcoin_txid: txid.to_string(),
            consignment_download_url: format!("/api/consignment/{}", consignment_filename),
            consignment_filename,
            pending_status: "broadcasted".to_string(),
        })
    }
    
    // Helper: Create WalletSigner implementing Signer trait
    fn create_signer(&self, wallet_name: &str) -> Result<WalletSigner, crate::error::WalletError> {
        let mnemonic = self.storage.load_mnemonic(wallet_name)?;
        let descriptor = self.storage.load_descriptor(wallet_name)?;
        Ok(WalletSigner::new(mnemonic, descriptor))
    }
    
    // Helper: Broadcast transaction via Esplora
    fn broadcast_tx_hex(&self, tx_hex: &str) -> Result<(), crate::error::WalletError> {
        use reqwest::blocking::Client;
        
        let client = Client::new();
        let response = client
            .post("https://mempool.space/signet/api/tx")
            .body(tx_hex.to_string())
            .send()
            .map_err(|e| crate::error::WalletError::Network(e.to_string()))?;
        
        if !response.status().is_success() {
            let error_text = response.text().unwrap_or_else(|_| "Unknown error".to_string());
            return Err(crate::error::WalletError::Network(format!("Broadcast failed: {}", error_text)));
        }
        
        Ok(())
    }
}
```

**File**: `wallet/src/api/handlers.rs`

**Add handlers**:
```rust
pub async fn send_transfer_handler(
    State(manager): State<Arc<WalletManager>>,
    Path(name): Path<String>,
    Json(req): Json<SendTransferRequest>,
) -> Result<Json<SendTransferResponse>, crate::error::WalletError> {
    if !manager.storage.wallet_exists(&name) {
        return Err(crate::error::WalletError::WalletNotFound(name));
    }
    
    let result = manager.send_transfer(&name, req)?;
    Ok(Json(result))
}

pub async fn download_consignment_handler(
    State(manager): State<Arc<WalletManager>>,
    Path(filename): Path<String>,
) -> Result<impl IntoResponse, crate::error::WalletError> {
    let file_path = manager.storage.base_dir()
        .join("temp_consignments")
        .join(&filename);
    
    if !file_path.exists() {
        return Err(crate::error::WalletError::NotFound("Consignment file not found".to_string()));
    }
    
    let contents = std::fs::read(&file_path)
        .map_err(|e| crate::error::WalletError::Storage(e.to_string()))?;
    
    Ok((
        [(axum::http::header::CONTENT_TYPE, "application/octet-stream")],
        [(axum::http::header::CONTENT_DISPOSITION, 
          format!("attachment; filename=\"{}\"", filename))],
        contents,
    ))
}
```

**File**: `wallet/src/api/server.rs`

**Add routes**:
```rust
.route("/api/wallet/:name/send-transfer", post(handlers::send_transfer_handler))
.route("/api/consignment/:filename", get(handlers::download_consignment_handler))
```

---

### Step 3.2: WalletSigner Implementation (NEW - Based on Research)

**File**: `wallet/src/wallet/signer.rs` (NEW)

**Purpose**: Implement `Signer` and `Sign` traits for PSBT signing

```rust
use bpstd::psbt::{Signer, Rejected};
use bc::secp256k1::{Secp256k1, Message, ecdsa};
use bc::{Sighash, LegacyPk};
use derive::{Sign, KeyOrigin, Xpriv};

#[derive(Clone)]
pub struct WalletSigner {
    mnemonic: bip39::Mnemonic,
    descriptor: String,
}

impl WalletSigner {
    pub fn new(mnemonic: bip39::Mnemonic, descriptor: String) -> Self {
        Self { mnemonic, descriptor }
    }
    
    fn derive_key_from_origin(&self, origin: &KeyOrigin) -> Option<bc::PrivateKey> {
        // Extract derivation path from KeyOrigin
        // Use existing derive_private_key_for_index logic
        // Return the private key for this path
        todo!("Implement key derivation from KeyOrigin")
    }
}

impl Signer for WalletSigner {
    type Sign<'s> = Self where Self: 's;
    
    fn approve(&self, _psbt: &bpstd::Psbt) -> Result<Self::Sign<'_>, Rejected> {
        // No user interaction needed in backend
        Ok(self.clone())
    }
}

impl Sign for WalletSigner {
    fn sign_ecdsa(
        &self,
        sighash: Sighash,
        _pk: LegacyPk,
        origin: Option<&KeyOrigin>,
    ) -> Option<ecdsa::Signature> {
        let private_key = self.derive_key_from_origin(origin?)?;
        
        let secp = Secp256k1::new();
        let message = Message::from_digest(sighash.to_byte_array());
        Some(secp.sign_ecdsa(&message, &private_key.inner))
    }
    
    fn sign_bip340(
        &self,
        _sighash: bc::TapSighash,
        _pk: bc::XOnlyPk,
        _leaf_hash: Option<bc::TapLeafHash>,
    ) -> Option<bc::secp256k1::schnorr::Signature> {
        // Not needed for P2WPKH (our descriptor type)
        None
    }
}
```

---

### Step 3.3: Frontend UI - Send Transfer

**File**: `wallet-frontend/src/components/SendTransferModal.tsx` (NEW)

*(Implementation similar to GenerateInvoiceModal, with invoice paste input and consignment download)*

---

### Phase 3 Deliverables

- ✅ Backend API: `POST /api/wallet/:name/send-transfer`
- ✅ Backend API: `GET /api/consignment/:filename`
- ✅ **NEW**: `WalletSigner` implementing `Signer` trait
- ✅ **CORRECTED**: Consignment generation before signing
- ✅ **SIMPLIFIED**: Broadcasting with hex formatting
- ✅ Frontend: `SendTransferModal` component
- ✅ File download link

**Test Checklist**:
- [ ] Parse valid invoice
- [ ] Create PSBT (with DBC commit already done)
- [ ] Generate consignment (before signing)
- [ ] Sign PSBT (using WalletSigner)
- [ ] Finalize PSBT
- [ ] Extract transaction
- [ ] Broadcast TX (hex format)
- [ ] Download consignment file

**Key Workflow** *(Corrected)*:
```
1. pay_invoice()        → PSBT + Payment (DBC committed)
2. consign_to_file()    → Generate consignment
3. psbt.sign()          → Sign with WalletSigner
4. psbt.finalize()      → Finalize inputs
5. psbt.extract()       → Get signed transaction
6. format!("{:x}", tx)  → Hex for broadcast
7. POST to Esplora      → Broadcast
```

---

## Phase 4: Consignment Import/Export

**Goal**: Enable both genesis export/import (same wallet sync) and transfer consignment acceptance (different wallets)

**Duration**: 3-4 days (Phase 4A: 1-2 days, Phase 4B: 1-2 days)

**Confidence**: 8.5/10

---

## Understanding Two Types of Consignments

### Genesis Consignment (Phase 4A)
**Purpose:** Sync contract state across devices with the same wallet  
**Use Case:** Computer A issues asset → Computer B imports to see it  
**Bitcoin TX:** ❌ None required (no transfer)  
**Recipients:** Same wallet, different devices

### Transfer Consignment (Phase 4B)
**Purpose:** Transfer asset ownership to different wallet  
**Use Case:** Wallet A sends tokens → Wallet B receives tokens  
**Bitcoin TX:** ✅ Required (moves tokens on-chain)  
**Recipients:** Different wallets, different keys

| Aspect | Genesis Consignment | Transfer Consignment |
|--------|---------------------|---------------------|
| **Purpose** | Share contract knowledge | Transfer ownership |
| **Wallets** | Same (same mnemonic) | Different (different keys) |
| **Bitcoin TX** | ❌ No | ✅ Yes |
| **State Transitions** | Genesis only | Full history + new transfer |
| **Endpoints** | Original genesis seal | New blinded seals |
| **After Import** | Existing UTXO shows tokens | New tokens on new UTXO |
| **Asset Movement** | No movement (already own) | Tokens change owners |
| **Invoice Required** | ❌ No | ✅ Yes |

---

## Phase 4A: Genesis Export/Import (Same Wallet Sync)

**Goal**: Export/import genesis consignment to sync contract state across devices

**Duration**: 1-2 days

**Confidence**: 9/10

---

### Step 4A.1: Backend API - Export Genesis Consignment

**File**: `wallet/src/api/types.rs`

**Add structs**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportGenesisRequest {
    pub contract_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportGenesisResponse {
    pub contract_id: String,
    pub consignment_path: String,
    pub file_size_bytes: u64,
}
```

**File**: `wallet/src/wallet/manager.rs`

**Add method**:
```rust
impl WalletManager {
    pub fn export_genesis_consignment(
        &self,
        wallet_name: &str,
        contract_id: &str,
    ) -> Result<ExportGenesisResponse, crate::error::WalletError> {
        // 1. Parse contract ID
        use rgb::ContractId;
        let contract_id = ContractId::from_str(contract_id)
            .map_err(|e| WalletError::InvalidInput(format!("Invalid contract ID: {}", e)))?;
        
        // 2. Load contracts
        let contracts = self.rgb_manager.load_contracts()?;
        
        // 3. Verify we have this contract
        if !contracts.has_contract(contract_id) {
            return Err(WalletError::Rgb("Contract not found".to_string()));
        }
        
        // 4. Get contract state to find genesis seals
        let state = contracts.contract_state(contract_id);
        
        // Find all seals for this contract (include all current allocations)
        let mut seals = Vec::new();
        for (_state_name, owned_states) in state.owned {
            for owned_state in owned_states {
                seals.push(owned_state.assignment.seal);
            }
        }
        
        if seals.is_empty() {
            return Err(WalletError::Rgb("No allocations found for contract".to_string()));
        }
        
        // 5. Create consignment for all state we know
        let consignment_filename = format!("genesis_{}.rgb", contract_id);
        let consignment_path = self.storage.base_dir()
            .join("exports")
            .join(&consignment_filename);
        
        std::fs::create_dir_all(consignment_path.parent().unwrap())?;
        
        // Export consignment including all seals we own
        contracts.consign_to_file(
            &consignment_path,
            contract_id,
            seals,
        ).map_err(|e| WalletError::Rgb(format!("Export failed: {:?}", e)))?;
        
        let file_size = std::fs::metadata(&consignment_path)?.len();
        
        Ok(ExportGenesisResponse {
            contract_id: contract_id.to_string(),
            consignment_path: consignment_path.display().to_string(),
            file_size_bytes: file_size,
        })
    }
}
```

**File**: `wallet/src/api/handlers.rs`

**Add handler**:
```rust
pub async fn export_genesis_handler(
    State(manager): State<Arc<WalletManager>>,
    Path((name, contract_id)): Path<(String, String)>,
) -> Result<Json<ExportGenesisResponse>, crate::error::WalletError> {
    if !manager.storage.wallet_exists(&name) {
        return Err(crate::error::WalletError::WalletNotFound(name));
    }
    
    let result = manager.export_genesis_consignment(&name, &contract_id)?;
    Ok(Json(result))
}

pub async fn download_genesis_handler(
    State(manager): State<Arc<WalletManager>>,
    Path(filename): Path<String>,
) -> Result<impl IntoResponse, crate::error::WalletError> {
    let file_path = manager.storage.base_dir()
        .join("exports")
        .join(&filename);
    
    if !file_path.exists() {
        return Err(crate::error::WalletError::NotFound("Genesis file not found".to_string()));
    }
    
    let contents = std::fs::read(&file_path)
        .map_err(|e| crate::error::WalletError::Storage(e.to_string()))?;
    
    Ok((
        [(axum::http::header::CONTENT_TYPE, "application/octet-stream")],
        [(axum::http::header::CONTENT_DISPOSITION, 
          format!("attachment; filename=\"{}\"", filename))],
        contents,
    ))
}
```

**File**: `wallet/src/api/server.rs`

**Add routes**:
```rust
.route("/api/wallet/:name/export-genesis/:contract_id", get(handlers::export_genesis_handler))
.route("/api/genesis/:filename", get(handlers::download_genesis_handler))
```

---

### Step 4A.2: Frontend UI - Export Genesis

**File**: `wallet-frontend/src/components/ExportGenesisModal.tsx` (NEW)

```tsx
import { useState } from 'react';
import { walletApi } from '../api/wallet';

interface ExportGenesisModalProps {
  walletName: string;
  contractId: string;
  assetName: string;
  isOpen: boolean;
  onClose: () => void;
}

export default function ExportGenesisModal({
  walletName,
  contractId,
  assetName,
  isOpen,
  onClose,
}: ExportGenesisModalProps) {
  const [isExporting, setIsExporting] = useState(false);
  const [exportResult, setExportResult] = useState<any | null>(null);
  const [error, setError] = useState<string | null>(null);

  const handleExport = async () => {
    setError(null);
    setIsExporting(true);

    try {
      const result = await walletApi.exportGenesis(walletName, contractId);
      setExportResult(result);
    } catch (err: any) {
      setError(err.response?.data?.error || err.message || 'Export failed');
    } finally {
      setIsExporting(false);
    }
  };

  const handleDownload = () => {
    const filename = `genesis_${contractId}.rgb`;
    window.location.href = `/api/genesis/${filename}`;
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center p-4 z-50">
      <div className="bg-white dark:bg-gray-800 rounded-lg p-6 max-w-2xl w-full">
        <h2 className="text-2xl font-bold text-gray-900 dark:text-white mb-4">
          Export Contract State
        </h2>

        <div className="mb-4 p-4 bg-blue-50 dark:bg-blue-900/30 border border-blue-200 dark:border-blue-700 rounded-md">
          <p className="text-sm text-blue-800 dark:text-blue-300">
            <strong>Use this to sync your wallet across devices.</strong><br />
            Export the genesis consignment and import it on another device with the same wallet mnemonic.
            No Bitcoin transaction is required.
          </p>
        </div>

        <div className="mb-4">
          <p className="text-sm text-gray-600 dark:text-gray-400">
            <strong>Asset:</strong> {assetName}<br />
            <strong>Contract ID:</strong> {contractId}
          </p>
        </div>

        {!exportResult ? (
          <>
            {error && (
              <div className="mb-4 p-3 bg-red-50 dark:bg-red-900/30 border border-red-200 
                            dark:border-red-700 rounded-md text-red-800 dark:text-red-300">
                {error}
              </div>
            )}

            <div className="flex justify-end space-x-3">
              <button
                onClick={onClose}
                className="px-4 py-2 text-gray-700 dark:text-gray-300 hover:bg-gray-100 
                         dark:hover:bg-gray-700 rounded-md transition-colors"
                disabled={isExporting}
              >
                Cancel
              </button>
              <button
                onClick={handleExport}
                className="px-4 py-2 bg-blue-600 hover:bg-blue-700 dark:bg-blue-500 
                         dark:hover:bg-blue-600 text-white rounded-md transition-colors 
                         disabled:opacity-50"
                disabled={isExporting}
              >
                {isExporting ? 'Exporting...' : 'Export Genesis Consignment'}
              </button>
            </div>
          </>
        ) : (
          <>
            <div className="mb-4 p-4 bg-green-50 dark:bg-green-900/30 border border-green-200 
                          dark:border-green-700 rounded-md">
              <h3 className="text-lg font-semibold text-green-800 dark:text-green-300 mb-2">
                ✅ Export Successful
              </h3>
              <p className="text-sm text-green-700 dark:text-green-400">
                Genesis consignment exported. Download and transfer to your other device.
              </p>
            </div>

            <div className="mb-4">
              <p className="text-sm text-gray-600 dark:text-gray-400">
                <strong>File Size:</strong> {(exportResult.file_size_bytes / 1024).toFixed(2)} KB
              </p>
            </div>

            <div className="flex justify-end space-x-3">
              <button
                onClick={onClose}
                className="px-4 py-2 text-gray-700 dark:text-gray-300 hover:bg-gray-100 
                         dark:hover:bg-gray-700 rounded-md transition-colors"
              >
                Close
              </button>
              <button
                onClick={handleDownload}
                className="px-4 py-2 bg-green-600 hover:bg-green-700 dark:bg-green-500 
                         dark:hover:bg-green-600 text-white rounded-md transition-colors"
              >
                📥 Download File
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
```

---

### Step 4A.3: Add Export Button to Asset List

**File**: `wallet-frontend/src/pages/WalletDetail.tsx`

**Add state and button**:
```tsx
const [showExportGenesisModal, setShowExportGenesisModal] = useState(false);
const [selectedAssetForExport, setSelectedAssetForExport] = useState<any | null>(null);

// In the RGB assets display section, add "Export" button:
<button
  onClick={() => {
    setSelectedAssetForExport(asset);
    setShowExportGenesisModal(true);
  }}
  className="px-3 py-1 bg-purple-600 hover:bg-purple-700 text-white text-sm rounded"
>
  Export
</button>

// At the bottom:
{selectedAssetForExport && (
  <ExportGenesisModal
    walletName={name || ''}
    contractId={selectedAssetForExport.asset_id}
    assetName={selectedAssetForExport.asset_name}
    isOpen={showExportGenesisModal}
    onClose={() => {
      setShowExportGenesisModal(false);
      setSelectedAssetForExport(null);
    }}
  />
)}
```

---

### Phase 4A Deliverables

- ✅ Backend: `export_genesis_consignment()` method
- ✅ Backend API: `GET /api/wallet/:name/export-genesis/:contract_id`
- ✅ Backend API: `GET /api/genesis/:filename` (download)
- ✅ Frontend: `ExportGenesisModal` component
- ✅ Frontend: Export button in asset list
- ✅ File download handling

**Test Checklist**:
- [ ] Issue asset on Computer A
- [ ] Export genesis consignment
- [ ] Download `.rgb` file
- [ ] Transfer file to Computer B (USB/network)
- [ ] Verify file size is reasonable (KB range)

---

## Phase 4B: Accept Consignment (Transfer & Genesis Import)

**Goal**: Accept both genesis and transfer consignments with automatic detection

**Duration**: 1-2 days

**Confidence**: 9/10

---

### Step 4B.1: Backend API - Enhanced Accept Consignment

**File**: `wallet/src/api/types.rs`

**Add structs**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcceptConsignmentResponse {
    pub contract_id: String,
    pub bitcoin_txid: String,
    pub status: String,  // "genesis_imported", "pending", or "confirmed"
    pub import_type: String,  // "genesis" or "transfer"
}
```

**File**: `wallet/src/wallet/manager.rs`

**Add enhanced method**:
```rust
impl WalletManager {
    pub fn accept_consignment(
        &self,
        wallet_name: &str,
        consignment_bytes: Vec<u8>,
    ) -> Result<AcceptConsignmentResponse, crate::error::WalletError> {
        // 1. Save consignment to temp file
        let temp_path = self.storage.base_dir()
            .join("temp_consignments")
            .join(format!("accept_{}.rgb", uuid::Uuid::new_v4()));
        
        std::fs::write(&temp_path, &consignment_bytes)?;
        
        // 2. Initialize runtime
        let mut runtime = self.get_runtime(wallet_name)?;
        
        // 3. Consume consignment (validates and imports)
        use std::convert::Infallible;
        runtime.consume_from_file(
            true,  // allow_unknown contracts
            &temp_path,
            |_, _, _| Result::<_, Infallible>::Ok(()),
        ).map_err(|e| crate::error::WalletError::Rgb(format!("Validation failed: {:?}", e)))?;
        
        // 4. Parse consignment for metadata
        use rgb::Consignment;
        let consignment = Consignment::load(&temp_path)
            .map_err(|e| crate::error::WalletError::Rgb(format!("Parse failed: {:?}", e)))?;
        
        let contract_id = consignment.contract_id();
        let bitcoin_txid = consignment.anchoring_txid();
        
        // 5. Determine if this was a genesis import or transfer
        // Genesis: no state transitions (only genesis operation)
        // Transfer: has state transitions
        let is_genesis = consignment.state_transitions().is_empty();
        
        // 6. Check Bitcoin TX status (if not genesis-only import)
        let status = if !is_genesis {
        let tx_status = self.check_tx_status(&bitcoin_txid)?;
            if tx_status.confirmed { "confirmed" } else { "pending" }
        } else {
            "genesis_imported"
        };
        
        // 7. Cleanup temp file
        let _ = std::fs::remove_file(&temp_path);
        
        Ok(AcceptConsignmentResponse {
            contract_id: contract_id.to_string(),
            bitcoin_txid: bitcoin_txid.to_string(),
            status: status.to_string(),
            import_type: if is_genesis { "genesis" } else { "transfer" }.to_string(),
        })
    }
}
```

**File**: `wallet/src/api/handlers.rs`

**Add handler**:
```rust
pub async fn accept_consignment_handler(
    State(manager): State<Arc<WalletManager>>,
    Path(name): Path<String>,
    body: axum::extract::Bytes,
) -> Result<Json<AcceptConsignmentResponse>, crate::error::WalletError> {
    if !manager.storage.wallet_exists(&name) {
        return Err(crate::error::WalletError::WalletNotFound(name));
    }
    
    let result = manager.accept_consignment(&name, body.to_vec())?;
    Ok(Json(result))
}
```

**File**: `wallet/src/api/server.rs`

**Add route**:
```rust
.route("/api/wallet/:name/accept-consignment", post(handlers::accept_consignment_handler))
```

---

### Step 4B.2: Frontend UI - Accept Consignment (Enhanced)

**File**: `wallet-frontend/src/components/AcceptConsignmentModal.tsx` (NEW)

```tsx
import { useState } from 'react';
import { walletApi } from '../api/wallet';

interface AcceptConsignmentModalProps {
  walletName: string;
  isOpen: boolean;
  onClose: () => void;
  onSuccess: () => void;
}

export default function AcceptConsignmentModal({
  walletName,
  isOpen,
  onClose,
  onSuccess,
}: AcceptConsignmentModalProps) {
  const [file, setFile] = useState<File | null>(null);
  const [isImporting, setIsImporting] = useState(false);
  const [result, setResult] = useState<any | null>(null);
  const [error, setError] = useState<string | null>(null);

  const handleFileSelect = (e: React.ChangeEvent<HTMLInputElement>) => {
    if (e.target.files && e.target.files[0]) {
      setFile(e.target.files[0]);
      setError(null);
    }
  };

  const handleImport = async () => {
    if (!file) return;

    setError(null);
    setIsImporting(true);

    try {
      const arrayBuffer = await file.arrayBuffer();
      const bytes = new Uint8Array(arrayBuffer);
      
      const importResult = await walletApi.acceptConsignment(walletName, bytes);
      setResult(importResult);
      
      // Refresh wallet data after successful import
      setTimeout(() => {
        onSuccess();
      }, 2000);
    } catch (err: any) {
      setError(err.response?.data?.error || err.message || 'Import failed');
    } finally {
      setIsImporting(false);
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center p-4 z-50">
      <div className="bg-white dark:bg-gray-800 rounded-lg p-6 max-w-2xl w-full">
        <h2 className="text-2xl font-bold text-gray-900 dark:text-white mb-4">
          Import Consignment
        </h2>

        {!result ? (
          <>
            <div className="mb-4 p-4 bg-blue-50 dark:bg-blue-900/30 border border-blue-200 
                          dark:border-blue-700 rounded-md">
              <p className="text-sm text-blue-800 dark:text-blue-300">
                <strong>Two types of consignments:</strong><br />
                • <strong>Genesis:</strong> Sync contract state from another device (same wallet)<br />
                • <strong>Transfer:</strong> Receive tokens from another wallet (different sender)
              </p>
            </div>

            <div className="mb-4">
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                Select Consignment File (.rgb)
              </label>
              <input
                type="file"
                accept=".rgb,.consignment"
                onChange={handleFileSelect}
                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md 
                         bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
              />
              {file && (
                <p className="mt-2 text-sm text-gray-600 dark:text-gray-400">
                  Selected: {file.name} ({(file.size / 1024).toFixed(2)} KB)
                </p>
              )}
            </div>

            {error && (
              <div className="mb-4 p-3 bg-red-50 dark:bg-red-900/30 border border-red-200 
                            dark:border-red-700 rounded-md text-red-800 dark:text-red-300">
                {error}
              </div>
            )}

            <div className="flex justify-end space-x-3">
              <button
                onClick={onClose}
                className="px-4 py-2 text-gray-700 dark:text-gray-300 hover:bg-gray-100 
                         dark:hover:bg-gray-700 rounded-md transition-colors"
                disabled={isImporting}
              >
                Cancel
              </button>
              <button
                onClick={handleImport}
                className="px-4 py-2 bg-blue-600 hover:bg-blue-700 dark:bg-blue-500 
                         dark:hover:bg-blue-600 text-white rounded-md transition-colors 
                         disabled:opacity-50"
                disabled={!file || isImporting}
              >
                {isImporting ? 'Importing...' : 'Import Consignment'}
              </button>
            </div>
          </>
        ) : (
          <>
            <div className="mb-4 p-4 bg-green-50 dark:bg-green-900/30 border border-green-200 
                          dark:border-green-700 rounded-md">
              <h3 className="text-lg font-semibold text-green-800 dark:text-green-300 mb-2">
                ✅ Import Successful
              </h3>
              <p className="text-sm text-green-700 dark:text-green-400">
                {result.import_type === 'genesis' 
                  ? 'Genesis consignment imported. Contract state synchronized.'
                  : 'Transfer consignment accepted. Assets will appear after confirmation.'}
              </p>
            </div>

            <div className="mb-4 space-y-2">
              <p className="text-sm text-gray-600 dark:text-gray-400">
                <strong>Type:</strong> {result.import_type === 'genesis' ? 'Genesis (Same Wallet Sync)' : 'Transfer (Received Tokens)'}
              </p>
              <p className="text-sm text-gray-600 dark:text-gray-400">
                <strong>Contract ID:</strong> {result.contract_id}
              </p>
              {result.import_type === 'transfer' && (
                <>
                  <p className="text-sm text-gray-600 dark:text-gray-400">
                    <strong>Bitcoin TX:</strong> {result.bitcoin_txid}
                  </p>
                  <p className="text-sm text-gray-600 dark:text-gray-400">
                    <strong>Status:</strong> {result.status === 'confirmed' ? '✅ Confirmed' : '⏳ Pending'}
                  </p>
                </>
              )}
            </div>

            <div className="flex justify-end">
              <button
                onClick={() => {
                  onClose();
                  setResult(null);
                  setFile(null);
                }}
                className="px-4 py-2 bg-gray-600 hover:bg-gray-700 dark:bg-gray-500 
                         dark:hover:bg-gray-600 text-white rounded-md transition-colors"
              >
                Done
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
```

---

### Step 4B.3: Add Import Button to Wallet Page

**File**: `wallet-frontend/src/pages/WalletDetail.tsx`

**Add state and button**:
```tsx
const [showAcceptConsignmentModal, setShowAcceptConsignmentModal] = useState(false);

// Add button near the top of the wallet detail page:
<button
  onClick={() => setShowAcceptConsignmentModal(true)}
  className="px-4 py-2 bg-green-600 hover:bg-green-700 text-white rounded-md"
>
  📥 Import Consignment
</button>

// At the bottom:
<AcceptConsignmentModal
  walletName={name || ''}
  isOpen={showAcceptConsignmentModal}
  onClose={() => setShowAcceptConsignmentModal(false)}
  onSuccess={() => {
    // Refresh wallet data
    loadWalletData();
  }}
/>
```

---

### Phase 4B Deliverables

- ✅ Backend: Enhanced `accept_consignment()` with type detection
- ✅ Backend API: `POST /api/wallet/:name/accept-consignment`
- ✅ Frontend: `AcceptConsignmentModal` component
- ✅ Frontend: Import button on wallet page
- ✅ Automatic detection of genesis vs transfer
- ✅ Different UI for each consignment type
- ✅ File upload handling
- ✅ Status display (genesis_imported/pending/confirmed)

**Test Checklist**:
- [ ] Import genesis consignment on Computer B
- [ ] Verify asset appears after genesis import
- [ ] Import transfer consignment (if testing transfers)
- [ ] Verify correct type detection (genesis vs transfer)
- [ ] Error handling for invalid files
- [ ] Verify Bitcoin TX status check for transfers

---

## Phase 4 Complete Deliverables

### Backend APIs
- ✅ `GET /api/wallet/:name/export-genesis/:contract_id` - Export genesis consignment
- ✅ `GET /api/genesis/:filename` - Download genesis file
- ✅ `POST /api/wallet/:name/accept-consignment` - Import any consignment

### Frontend Components
- ✅ `ExportGenesisModal` - Export genesis consignment
- ✅ `AcceptConsignmentModal` - Import consignment (auto-detects type)
- ✅ Export button in asset list
- ✅ Import button on wallet page

### Functionality
- ✅ Genesis export for same-wallet sync
- ✅ Genesis import (no Bitcoin TX required)
- ✅ Transfer import (with Bitcoin TX validation)
- ✅ Automatic type detection
- ✅ File download/upload handling
- ✅ Status tracking

### User Flows

**Same Wallet Sync (Genesis):**
1. Computer A: Issue asset → Asset appears
2. Computer A: Click "Export" → Download `genesis_XXX.rgb`
3. Transfer file to Computer B (USB/network/cloud)
4. Computer B: Click "Import Consignment" → Upload file
5. Computer B: Asset appears (same UTXO as Computer A)

**Transfer Between Wallets:**
1. Wallet A: Generate invoice (Phase 2)
2. Wallet B: Send transfer (Phase 3) → Download transfer consignment
3. Transfer file to Wallet A
4. Wallet A: Click "Import Consignment" → Upload file
5. Wallet A: Tokens appear after Bitcoin TX confirms

---

## Complete Test Checklist

**Genesis Export/Import:**
- [ ] Export genesis from wallet that issued asset
- [ ] Download `.rgb` file
- [ ] Transfer file to different device
- [ ] Import on device with same wallet mnemonic
- [ ] Verify asset appears with correct amount
- [ ] Verify UTXO shows as occupied
- [ ] Verify no Bitcoin transaction required

**Transfer Accept:**
- [ ] Receive transfer consignment from sender
- [ ] Import transfer consignment
- [ ] Verify Bitcoin TX is checked
- [ ] Verify status shows pending/confirmed
- [ ] Verify balance updates after confirmation
- [ ] Verify different UTXO than sender

**Error Handling:**
- [ ] Invalid file format
- [ ] Corrupted consignment
- [ ] Wrong wallet (transfer to different wallet)
- [ ] Network errors during TX check
- [ ] File too large

---

## Phase 5: Frontend Integration

**Goal**: Complete UI for transfer workflow

**Duration**: 2 days

**Confidence**: 9/10

---

### Step 5.1: Assets Page Updates

**Add buttons**:
- "Receive" → Opens GenerateInvoiceModal
- "Send" → Opens SendTransferModal
- "Accept Transfer" → Opens AcceptConsignmentModal

---

### Step 5.2: Activity/History Display

**Show**:
- Transfer type (sent/received)
- Amount
- Bitcoin TX link
- Status (pending/confirmed)
- Timestamp

---

### Step 5.3: Error Handling

**Display user-friendly errors**:
- No UTXOs available (can't generate invoice)
- Insufficient balance (can't send)
- Invalid invoice format
- Invalid consignment file
- Network errors

---

### Phase 5 Deliverables

- ✅ Complete transfer UI flow
- ✅ Transfer history display
- ✅ Error messages
- ✅ Loading states
- ✅ Success confirmations

---

## Phase 6: Testing & Polish

**Goal**: End-to-end tested and production-ready

**Duration**: 2-3 days

**Confidence**: 8/10

---

### Step 6.1: Integration Testing

**Test complete flow**:
1. Wallet A: Generate invoice
2. Wallet B: Send transfer using invoice
3. Wallet B: Download consignment
4. Wallet A: Upload and accept consignment
5. Both: Verify balance updates

---

### Step 6.2: Error Handling

**Test edge cases**:
- Invalid invoice
- Expired invoice
- Insufficient balance
- No UTXOs for seal
- Network failures
- Corrupted consignment

---

### Step 6.3: Documentation

**Add to Docs page**:
- How to generate invoice
- How to send transfer
- How to accept consignment
- Manual file sharing instructions

---

### Phase 6 Deliverables

- ✅ End-to-end transfer tested
- ✅ All edge cases handled
- ✅ User documentation complete
- ✅ Production-ready code

---

## Timeline & Effort

| Phase | Duration | Complexity | Confidence |
|-------|----------|------------|------------|
| **Phase 1: RGB Runtime Foundation** | 3-4 days | High | 7.5/10 |
| **Phase 2: Invoice Generation** | ✅ 2 days | Medium | 8/10 |
| **Phase 3: Send Transfer** | **2-3 days** *(was 3-4)* | **Medium** *(was High)* | **8.5/10** *(was 7/10)* |
| **Phase 4A: Genesis Export/Import** | 1-2 days | Low-Medium | 9/10 |
| **Phase 4B: Transfer Accept** | 1-2 days | Low | 9/10 |
| **Phase 5: Frontend Integration** | 2 days | Medium | 9/10 |
| **Phase 6: Testing & Polish** | 2-3 days | Medium | 8/10 |
| **TOTAL** | **13-20 days** *(was 14-21)* | — | **8.2/10** *(was 8/10)* |

**Phase 3 Updates** *(After Deep Research - Oct 12, 2025)*:
- ✅ Complexity reduced (Medium vs High) - simpler workflow discovered
- ✅ Duration reduced (2-3 vs 3-4 days) - no `complete()` step, no type conversions
- ✅ Confidence increased (8.5/10 vs 7/10) - complete RGB CLI source analysis

---

## Dependencies & Risks

### Dependencies
- ✅ RGB libraries (local submodules)
- ✅ Esplora API (mempool.space)
- ✅ Existing wallet infrastructure
- ✅ PSBT signing logic

### Risks
1. **MemUtxos sync complexity** (Medium) - Mitigated by runtime.update()
2. **PSBT signing integration** (Low) - Already have logic
3. **Consignment size** (Low) - Files are small (KB range)
4. **Manual file sharing UX** (Medium) - Acceptable for MVP

---

## Success Criteria

- ✅ Recipient can generate invoice
- ✅ Sender can send transfer using invoice
- ✅ Recipient can accept and validate consignment
- ✅ Balance updates correctly after transfer
- ✅ Bitcoin TX visible on Mempool explorer
- ✅ Complete transfer in < 5 minutes (excluding Bitcoin confirmations)

---

## Future Enhancements (Out of Scope)

1. **Relay Server** - Auto-share consignments (1-2 days)
2. **QR Codes** - For invoice sharing
3. **Transfer History** - Detailed activity log
4. **Multiple Networks** - Testnet, Mainnet support
5. **Batch Transfers** - Send to multiple recipients

---

## Approval Checklist

Before starting implementation, confirm:
- [ ] Architecture approved
- [ ] Phase breakdown approved
- [ ] Timeline acceptable
- [ ] Manual file sharing acceptable
- [ ] Dependencies available
- [ ] Tests planned

---

**Status**: 📋 Ready for review and approval

**Next Step**: User approval → Begin Phase 1 implementation

