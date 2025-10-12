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
6. [Phase 4: Accept Consignment (Recipient)](#phase-4-accept-consignment-recipient)
7. [Phase 5: Frontend Integration](#phase-5-frontend-integration)
8. [Phase 6: Testing & Polish](#phase-6-testing--polish)
9. [Timeline & Effort](#timeline--effort)

---

## Overview

### Goal
Enable complete RGB asset transfers between wallets:
- ✅ **Recipient** generates invoice (with blinded seal)
- ✅ **Sender** sends payment (creates PSBT + consignment)
- ✅ **Recipient** accepts consignment (validates + imports)

### Approach
Use **RGB Runtime** (`RgbWallet` + `RgbpRuntimeDir`) for native library integration instead of CLI spawning.

### Key Deliverables
1. Backend APIs for invoice/send/receive
2. Frontend UI for transfer flow
3. Manual consignment file sharing (download/upload)
4. End-to-end tested transfer cycle

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

**Duration**: 3-4 days

**Confidence**: 7/10

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

**Add method**:
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
        use bpstd::psbt::TxParams;
        use rgb::popls::bp::CoinselectStrategy;
        use bpstd::Sats;
        
        let params = TxParams::with(request.fee_rate_sat_vb.unwrap_or(2));
        let (mut psbt, payment) = runtime.pay_invoice(
            &invoice,
            CoinselectStrategy::Accumulative,
            params,
            Some(Sats::from_sats(1000)),  // Min locked amount
        ).map_err(|e| crate::error::WalletError::Rgb(format!("Payment failed: {:?}", e)))?;
        
        // 4. Sign PSBT (using our existing logic)
        let mnemonic = self.storage.load_mnemonic(wallet_name)?;
        let xprv = self.derive_xprv_from_mnemonic(&mnemonic)?;
        self.sign_psbt(&mut psbt, &xprv)?;
        
        // 5. Finalize PSBT
        psbt.finalize(runtime.wallet.descriptor())
            .map_err(|e| crate::error::WalletError::Rgb(format!("Finalization failed: {:?}", e)))?;
        
        // 6. Extract and broadcast Bitcoin TX
        let tx = psbt.extract()
            .map_err(|e| crate::error::WalletError::Rgb(format!("Extraction failed: {:?}", e)))?;
        
        let txid = tx.txid();
        self.broadcast_tx(&tx)?;
        
        // 7. Generate consignment
        let consignment_filename = format!("consignment_{}_{}.consignment", 
            invoice.scope, txid);
        let consignment_path = self.storage.base_dir()
            .join("temp_consignments")
            .join(&consignment_filename);
        
        std::fs::create_dir_all(consignment_path.parent().unwrap())?;
        
        runtime.contracts.consign_to_file(
            &consignment_path,
            invoice.scope,
            payment.terminals,
        ).map_err(|e| crate::error::WalletError::Rgb(format!("Consignment failed: {:?}", e)))?;
        
        Ok(SendTransferResponse {
            bitcoin_txid: txid.to_string(),
            consignment_download_url: format!("/api/consignment/{}", consignment_filename),
            consignment_filename,
            pending_status: "pending".to_string(),
        })
    }
    
    // Helper: Sign PSBT (reuse existing logic from transaction.rs)
    fn sign_psbt(
        &self,
        psbt: &mut bpstd::Psbt,
        xprv: &bitcoin::bip32::Xpriv,
    ) -> Result<(), crate::error::WalletError> {
        // Use existing signing logic
        todo!("Implement PSBT signing")
    }
    
    fn broadcast_tx(
        &self,
        tx: &bpstd::Tx,
    ) -> Result<(), crate::error::WalletError> {
        // Use existing Esplora broadcast
        todo!("Implement TX broadcast")
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

### Step 3.2: Frontend UI - Send Transfer

**File**: `wallet-frontend/src/components/SendTransferModal.tsx` (NEW)

*(Implementation similar to GenerateInvoiceModal, with invoice paste input and consignment download)*

---

### Phase 3 Deliverables

- ✅ Backend API: `POST /api/wallet/:name/send-transfer`
- ✅ Backend API: `GET /api/consignment/:filename`
- ✅ Frontend: `SendTransferModal` component
- ✅ PSBT signing integrated
- ✅ Bitcoin TX broadcast
- ✅ Consignment file generation
- ✅ File download link

**Test Checklist**:
- [ ] Parse valid invoice
- [ ] Create PSBT
- [ ] Sign PSBT
- [ ] Broadcast TX
- [ ] Generate consignment
- [ ] Download consignment file

---

## Phase 4: Accept Consignment (Recipient)

**Goal**: Recipient can upload and validate consignment

**Duration**: 1-2 days

**Confidence**: 9/10

---

### Step 4.1: Backend API - Accept Consignment

**File**: `wallet/src/api/types.rs`

**Add structs**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcceptConsignmentResponse {
    pub contract_id: String,
    pub bitcoin_txid: String,
    pub status: String,  // "pending" or "confirmed"
}
```

**File**: `wallet/src/wallet/manager.rs`

**Add method**:
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
            .join(format!("accept_{}.consignment", uuid::Uuid::new_v4()));
        
        std::fs::write(&temp_path, &consignment_bytes)?;
        
        // 2. Initialize runtime
        let mut runtime = self.get_runtime(wallet_name)?;
        
        // 3. Consume consignment
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
        
        // 5. Check Bitcoin TX status
        let tx_status = self.check_tx_status(&bitcoin_txid)?;
        
        // 6. Cleanup temp file
        let _ = std::fs::remove_file(&temp_path);
        
        Ok(AcceptConsignmentResponse {
            contract_id: contract_id.to_string(),
            bitcoin_txid: bitcoin_txid.to_string(),
            status: if tx_status.confirmed { "confirmed" } else { "pending" },
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

### Step 4.2: Frontend UI - Accept Consignment

**File**: `wallet-frontend/src/components/AcceptConsignmentModal.tsx` (NEW)

*(Implementation: file upload, validation status, success/error display)*

---

### Phase 4 Deliverables

- ✅ Backend API: `POST /api/wallet/:name/accept-consignment`
- ✅ Frontend: `AcceptConsignmentModal` component
- ✅ File upload handling
- ✅ Consignment validation
- ✅ Status display (pending/confirmed)

**Test Checklist**:
- [ ] Upload valid consignment
- [ ] Validate consignment
- [ ] Check Bitcoin TX status
- [ ] Update wallet balance
- [ ] Error handling (invalid file)

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
| **Phase 2: Invoice Generation** | 2 days | Medium | 8/10 |
| **Phase 3: Send Transfer** | 3-4 days | High | 7/10 |
| **Phase 4: Accept Consignment** | 1-2 days | Low | 9/10 |
| **Phase 5: Frontend Integration** | 2 days | Medium | 9/10 |
| **Phase 6: Testing & Polish** | 2-3 days | Medium | 8/10 |
| **TOTAL** | **13-19 days** | — | **8/10** |

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

