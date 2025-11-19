# F1r3fly-RGB Wallet Integration Plan

## Executive Summary

This document outlines the integration of RGB's Bitcoin wallet infrastructure with F1r3fly's contract execution layer. The goal is to create a production-ready wallet that:
- Uses RGB's battle-tested Bitcoin UTXO management and PSBT construction
- Embeds F1r3fly state hashes in Bitcoin transactions via Tapret commitments
- Maintains F1r3fly's execution model (contracts run on F1r3node, not client-side)

## Architecture Overview

```
F1r3flyRgbWallet
├── Bitcoin Layer (from RGB)
│   ├── Owner (descriptor, UTXOs, keys, PSBT construction)
│   └── Resolver (Esplora for blockchain queries)
│
├── F1r3fly Layer (existing)
│   ├── F1r3flyExecutor (contract execution on F1r3node)
│   └── BitcoinAnchorTracker (seal/witness storage)
│
└── Integration Layer (new)
    ├── Tapret commitment embedding
    ├── State hash → Bitcoin tx linking
    └── High-level operations (issue, transfer, query)
```

## What We Leverage from RGB

### 1. Owner (Bitcoin Wallet Core)

**Source**: `rgb/src/owner.rs`

**What it provides**:
```rust
pub struct Owner<R, O, K = XpubDerivable, U = MemUtxos> {
    network: Network,
    provider: O,        // Descriptor + UTXO set
    resolver: R,        // Esplora blockchain queries
}

impl PsbtConstructor for Owner {
    fn construct_psbt(
        &mut self,
        closes: impl IntoIterator<Item = Outpoint>,
        beneficiaries: impl IntoIterator<Item = Beneficiary>,
        params: TxParams,
    ) -> Result<(Psbt, PsbtMeta), ConstructionError>;
}
```

**Why we need it**:
- Manages Bitcoin descriptor (BIP32 key derivation)
- Tracks UTXO set (available coins)
- Constructs PSBTs with proper inputs/outputs
- Handles coin selection and fee calculation

### 2. PSBT Tapret Extension (Commitment Embedding)

**Source**: `bp-std/psbt/src/csval/tapret.rs`

**What it provides**:
```rust
impl Output {
    pub fn set_tapret_host(&mut self) -> Result<bool, TapretKeyError>;
    
    pub fn tapret_commit(
        &mut self,
        commitment: mpc::Commitment,
    ) -> Result<TapretProof, TapretKeyError>;
    
    pub fn tapret_commitment(&self) -> Result<TapretCommitment, TapretKeyError>;
    pub fn tapret_proof(&self) -> Option<TapretProof>;
}
```

**Why we need it**:
- Industry-standard Tapret implementation
- Embeds 32-byte commitments in taproot outputs
- Generates proofs for verification

### 3. Resolver (Blockchain Queries)

**Source**: `rgb/src/resolvers/esplora.rs`

**What it provides**:
```rust
pub trait Resolver {
    fn resolve_tx(&self, txid: Txid) -> Result<Tx>;
    fn resolve_pub_witness(&self, txid: Txid) -> Result<WitnessStatus>;
}
```

**Why we need it**:
- Queries Esplora for transaction data
- Checks transaction confirmation status
- Fetches witness data for validation

## What We Keep from F1r3fly-RGB

### 1. F1r3flyExecutor (Contract Execution)

**Source**: `f1r3fly-rgb/src/executor.rs`

**Current capabilities**:
```rust
pub struct F1r3flyExecutor {
    client: F1r3flyApiClient,
    // ...
}

impl F1r3flyExecutor {
    pub async fn deploy_contract(...) -> Result<ContractId>;
    pub async fn call_method(...) -> Result<F1r3flyExecutionResult>;
    pub async fn query_state(...) -> Result<StrictVal>;
}
```

**Enhancements needed**:
```rust
pub struct F1r3flyExecutionResult {
    pub deploy_id: String,
    pub finalized_block_hash: String,
    pub state_hash: [u8; 32],  // ← ADD THIS
}

impl F1r3flyExecutionResult {
    pub fn state_commitment(&self) -> mpc::Commitment {
        mpc::Commitment::from(self.state_hash)
    }
}
```

### 2. BitcoinAnchorTracker (Seal Storage)

**Source**: `f1r3fly-rgb/src/bitcoin_anchor.rs`

**Keep as-is** - this is our `Pile` implementation:
```rust
pub struct BitcoinAnchorTracker<S: RgbSeal = TxoSeal> {
    seals: SmallOrdMap<CellAddr, S::Witness>,
    witnesses: SmallOrdMap<S::WitnessId, (Anchor, WitnessStatus)>,
    // ...
}
```

## Implementation Plan

### Phase 1: Add RGB Owner Integration

**Goal**: Integrate RGB's Bitcoin wallet for UTXO management and PSBT construction

#### Step 1.1: Add Dependencies

**File**: `f1r3fly-rgb/Cargo.toml`

```toml
[dependencies]
# RGB wallet components
rgb = "0.12.0-rc.3"
rgb-std = "0.12.0-rc.3"
rgbdescr = "0.12.0"  # RGB descriptor extensions

# Already have these, ensure correct versions:
bp-std = "0.12.0-rc.3"
bpstd = "0.12.0"
```

#### Step 1.2: Create Owner Wrapper

**File**: `f1r3fly-rgb/src/bitcoin_wallet.rs` (NEW)

```rust
//! Bitcoin wallet integration using RGB's Owner

use rgb::owner::{Owner, SingleHolder};
use rgb::resolvers::Esplora as EsploraResolver;
use rgbdescr::RgbDescr;
use bpstd::{XpubDerivable, Network, Wpkh};
use bpstd::psbt::{PsbtConstructor, Beneficiary, TxParams, Psbt, PsbtMeta, ConstructionError};
use bpstd::{Outpoint, Sats};

use crate::MemUtxos;  // We'll need to implement this

/// Bitcoin wallet for F1r3fly-RGB operations
/// 
/// Wraps RGB's Owner to provide Bitcoin UTXO management and PSBT construction
/// while keeping F1r3fly-specific contract execution separate.
pub struct F1r3flyBitcoinWallet {
    owner: Owner<EsploraResolver, SingleHolder<XpubDerivable, MemUtxos>>,
}

impl F1r3flyBitcoinWallet {
    /// Create a new wallet from a descriptor string and Esplora URL
    pub fn new(descriptor: &str, esplora_url: &str, network: Network) -> Result<Self> {
        let xpub = XpubDerivable::from_str(descriptor)?;
        let noise = xpub.xpub().chain_code().to_byte_array();
        let rgb_descr = RgbDescr::<XpubDerivable>::new_unfunded(Wpkh::from(xpub), noise);
        
        let resolver = EsploraResolver::new(esplora_url)?;
        let holder = SingleHolder::new(rgb_descr, MemUtxos::default());
        let owner = Owner::new(network, holder, resolver);
        
        Ok(Self { owner })
    }
    
    /// Sync UTXO set from blockchain
    pub async fn sync(&mut self) -> Result<()> {
        self.owner.sync().await
    }
    
    /// Build a PSBT with given inputs and outputs
    pub fn construct_psbt(
        &mut self,
        inputs: impl IntoIterator<Item = Outpoint>,
        outputs: impl IntoIterator<Item = (Address, Sats)>,
        fee_rate: f64,
    ) -> Result<(Psbt, PsbtMeta), ConstructionError> {
        let beneficiaries = outputs.into_iter()
            .map(|(addr, sats)| Beneficiary::new(addr, sats));
        
        let params = TxParams {
            fee: None,  // Auto-calculate
            fee_precision: Some(fee_rate),
            ..Default::default()
        };
        
        self.owner.construct_psbt(inputs, beneficiaries, params)
    }
    
    /// Get available UTXOs
    pub fn utxos(&self) -> &MemUtxos {
        self.owner.utxos()
    }
    
    /// Get network
    pub fn network(&self) -> Network {
        self.owner.network()
    }
}
```

#### Step 1.3: Implement MemUtxos

**File**: `f1r3fly-rgb/src/utxo_set.rs` (NEW)

```rust
//! In-memory UTXO set for F1r3fly-RGB wallet

use std::collections::HashMap;
use bpstd::{Outpoint, ScriptPubkey, Sats};
use rgb::owner::UtxoSet;

/// In-memory UTXO set
#[derive(Clone, Debug, Default)]
pub struct MemUtxos {
    utxos: HashMap<Outpoint, (ScriptPubkey, Sats)>,
}

impl UtxoSet for MemUtxos {
    fn has(&self, outpoint: Outpoint) -> bool {
        self.utxos.contains_key(&outpoint)
    }
    
    fn get(&self, outpoint: Outpoint) -> Option<(ScriptPubkey, Sats)> {
        self.utxos.get(&outpoint).cloned()
    }
    
    fn insert(&mut self, outpoint: Outpoint, script: ScriptPubkey, amount: Sats) {
        self.utxos.insert(outpoint, (script, amount));
    }
    
    fn remove(&mut self, outpoint: &Outpoint) -> bool {
        self.utxos.remove(outpoint).is_some()
    }
    
    fn iter(&self) -> impl Iterator<Item = (Outpoint, ScriptPubkey, Sats)> + '_ {
        self.utxos.iter().map(|(op, (script, sats))| (*op, script.clone(), *sats))
    }
}
```

### Phase 2: Add State Hash to F1r3flyExecutor

**Goal**: Make F1r3fly execution results include a 32-byte state hash for Tapret commitments

#### Step 2.1: Update F1r3flyExecutionResult

**File**: `f1r3fly-rgb/src/executor.rs`

```rust
// Add to imports
use sha2::{Sha256, Digest};
use commit_verify::mpc;

// Update struct (around line 30)
pub struct F1r3flyExecutionResult {
    pub deploy_id: String,
    pub finalized_block_hash: String,
    pub state_hash: [u8; 32],  // NEW: Deterministic hash for Tapret commitment
}

impl F1r3flyExecutionResult {
    /// Get MPC commitment for Tapret embedding
    pub fn state_commitment(&self) -> mpc::Commitment {
        mpc::Commitment::from(self.state_hash)
    }
}

// Add helper function (at end of file)
fn compute_state_hash(finalized_block_hash: &str, deploy_id: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"f1r3fly-rgb-state-v1:");
    hasher.update(finalized_block_hash.as_bytes());
    hasher.update(b":");
    hasher.update(deploy_id.as_bytes());
    
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}
```

#### Step 2.2: Update execute() Method

**File**: `f1r3fly-rgb/src/executor.rs` (around line 150)

```rust
async fn execute(&mut self, rholang: String) -> Result<F1r3flyExecutionResult> {
    // ... existing code to execute and wait for finalization ...
    
    // After getting finalized_block_hash (around line 180):
    let state_hash = compute_state_hash(&finalized_block_hash, &deploy_id);
    
    Ok(F1r3flyExecutionResult {
        deploy_id,
        finalized_block_hash,
        state_hash,  // NEW
    })
}
```

### Phase 3: Implement Tapret Commitment Embedding

**Goal**: Embed F1r3fly state hashes in Bitcoin PSBTs using Tapret

#### Step 3.1: Create Tapret Helper Module

**File**: `f1r3fly-rgb/src/tapret.rs` (NEW)

```rust
//! Tapret commitment utilities for F1r3fly state hashes

use bp::dbc::tapret::TapretProof;
use bp::seals::{Anchor, mmb, mpc};
use bpstd::psbt::{Psbt, PropKey};
use commit_verify::{mpc as mpc_cv, ReservedBytes};
use strict_encoding::StrictDumb;

pub type Result<T> = std::result::Result<T, TapretError>;

#[derive(Debug)]
pub enum TapretError {
    NotTaprootOutput,
    InvalidOutputIndex { index: usize, max: usize },
    CommitmentFailed(String),
}

impl std::fmt::Display for TapretError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotTaprootOutput => write!(f, "Output is not a taproot output"),
            Self::InvalidOutputIndex { index, max } => {
                write!(f, "Invalid output index {} (max: {})", index, max)
            }
            Self::CommitmentFailed(msg) => write!(f, "Tapret commitment failed: {}", msg),
        }
    }
}

impl std::error::Error for TapretError {}

/// Embed F1r3fly state hash as Tapret commitment in PSBT output
///
/// # Arguments
/// * `psbt` - Mutable PSBT to modify
/// * `output_index` - Which output gets the commitment (typically 0)
/// * `state_hash` - F1r3fly state hash (32 bytes)
///
/// # Returns
/// TapretProof for later Anchor creation
pub fn embed_tapret_commitment(
    psbt: &mut Psbt,
    output_index: usize,
    state_hash: [u8; 32],
) -> Result<TapretProof> {
    // Get mutable output
    let mut outputs: Vec<_> = psbt.outputs_mut().collect();
    
    if output_index >= outputs.len() {
        return Err(TapretError::InvalidOutputIndex {
            index: output_index,
            max: outputs.len(),
        });
    }
    
    let output = &mut outputs[output_index];
    
    // Check if taproot
    if !output.script.is_p2tr() {
        return Err(TapretError::NotTaprootOutput);
    }
    
    // Mark output as Tapret host
    output.set_tapret_host()
        .map_err(|e| TapretError::CommitmentFailed(format!("set_tapret_host failed: {:?}", e)))?;
    
    // Create commitment from state hash
    let commitment = mpc_cv::Commitment::from(state_hash);
    
    // Embed Tapret commitment
    let proof = output.tapret_commit(commitment)
        .map_err(|e| TapretError::CommitmentFailed(format!("tapret_commit failed: {:?}", e)))?;
    
    log::info!("✅ Tapret commitment embedded");
    log::debug!("   State hash: {}", hex::encode(state_hash));
    log::debug!("   Output index: {}", output_index);
    
    Ok(proof)
}

/// Create RGB Anchor from Tapret proof
///
/// # Arguments
/// * `proof` - Tapret proof from `embed_tapret_commitment`
/// * `mpc_protocol` - Protocol ID (use placeholder for F1r3fly)
/// * `mpc_proof` - MPC merkle proof (use placeholder for F1r3fly)
/// * `mmb_proof` - Multi-message bundle proof (use placeholder for F1r3fly)
///
/// # Returns
/// RGB Anchor for storage in BitcoinAnchorTracker
pub fn create_anchor(
    proof: &TapretProof,
    mpc_protocol: mpc::ProtocolId,
    mpc_proof: mpc::MerkleProof,
    mmb_proof: mmb::BundleProof,
) -> Result<Anchor> {
    Ok(Anchor {
        mmb_proof,
        mpc_protocol,
        mpc_proof,
        dbc_proof: Some(proof.clone()),
        fallback_proof: ReservedBytes::strict_dumb(),
    })
}

/// Create test Anchor with placeholder proofs
///
/// For F1r3fly-RGB, we use simplified proofs since we're not doing
/// full RGB client-side validation. The Tapret proof is real, but
/// the MPC/MMB proofs are placeholders.
pub fn create_test_anchor(proof: &TapretProof) -> Result<Anchor> {
    use strict_encoding::StrictDumb;
    
    create_anchor(
        proof,
        mpc::ProtocolId::strict_dumb(),
        mpc::MerkleProof::strict_dumb(),
        mmb::BundleProof::strict_dumb(),
    )
}
```

### Phase 4: Create High-Level F1r3flyRgbWallet

**Goal**: Combine all pieces into a production-ready wallet

#### Step 4.1: Create Main Wallet Struct

**File**: `f1r3fly-rgb/src/wallet.rs` (NEW)

```rust
//! F1r3fly-RGB Wallet
//!
//! Combines F1r3fly contract execution with Bitcoin UTXO management

use bpstd::{Address, Network, Outpoint, Sats, Txid, Tx};
use rgb::{Opid, WitnessStatus};
use rgb_std::seals::TxoSeal;

use crate::{
    F1r3flyExecutor, 
    BitcoinAnchorTracker,
    bitcoin_wallet::F1r3flyBitcoinWallet,
    tapret::{embed_tapret_commitment, create_test_anchor},
};

/// Production wallet combining F1r3fly execution and Bitcoin
pub struct F1r3flyRgbWallet {
    /// F1r3fly contract executor
    executor: F1r3flyExecutor,
    
    /// Bitcoin wallet (RGB's Owner)
    bitcoin: F1r3flyBitcoinWallet,
    
    /// Seal/witness tracker
    tracker: BitcoinAnchorTracker<TxoSeal>,
    
    /// Active contract ID (if any)
    active_contract: Option<ContractId>,
}

impl F1r3flyRgbWallet {
    /// Create new wallet
    pub fn new(
        f1r3node_url: &str,
        private_key: &str,
        bitcoin_descriptor: &str,
        esplora_url: &str,
        network: Network,
    ) -> Result<Self> {
        let executor = F1r3flyExecutor::new_with_config(
            f1r3node_url,
            private_key,
        )?;
        
        let bitcoin = F1r3flyBitcoinWallet::new(
            bitcoin_descriptor,
            esplora_url,
            network,
        )?;
        
        let tracker = BitcoinAnchorTracker::new();
        
        Ok(Self {
            executor,
            bitcoin,
            tracker,
            active_contract: None,
        })
    }
    
    /// Sync Bitcoin wallet with blockchain
    pub async fn sync_bitcoin(&mut self) -> Result<()> {
        self.bitcoin.sync().await
    }
    
    /// Issue new RGB20-like tokens
    pub async fn issue_asset(
        &mut self,
        ticker: &str,
        name: &str,
        supply: u64,
        precision: u8,
        genesis_seal: TxoSeal,
    ) -> Result<AssetInfo> {
        log::info!("Issuing asset: {} ({})", name, ticker);
        
        // 1. Deploy contract to F1r3fly
        let contract_template = RholangContractLibrary::rho20_contract();
        let contract_id = self.executor.deploy_contract(
            contract_template,
            ticker,
            name,
            supply,
            precision,
            vec!["issue".to_string(), "transfer".to_string()],
        ).await?;
        
        self.active_contract = Some(contract_id.clone());
        log::info!("Contract deployed: {}", contract_id);
        
        // 2. Execute initial issue on F1r3fly
        let result = self.executor.call_method(
            contract_id,
            "issue",
            &[
                ("amount", StrictVal::from(supply)),
                ("seal", StrictVal::from(genesis_seal.to_string())),
            ],
        ).await?;
        
        log::info!("F1r3fly execution complete, state_hash: {}", 
            hex::encode(result.state_hash));
        
        // 3. Build Bitcoin transaction with Tapret commitment
        let (mut psbt, meta) = self.bitcoin.construct_psbt(
            vec![],  // No inputs for genesis (will be funded by wallet)
            vec![(self.get_change_address()?, Sats::from(1000u64))],  // Minimal output
            1.0,  // 1 sat/vB fee rate
        )?;
        
        // 4. Embed F1r3fly state hash as Tapret commitment
        let proof = embed_tapret_commitment(&mut psbt, 0, result.state_hash)?;
        
        // 5. Sign PSBT (would use real signer in production)
        // For now, assume psbt.sign() is implemented
        let signed_count = psbt.sign(&self.create_signer()?)?;
        log::info!("Signed {} inputs", signed_count);
        
        // 6. Finalize and extract transaction
        psbt.finalize(&self.bitcoin.owner.descriptor());
        let tx = psbt.extract()?;
        let txid = tx.txid();
        
        // 7. Broadcast
        self.broadcast_tx(&tx).await?;
        log::info!("Broadcast genesis tx: {}", txid);
        
        // 8. Create Anchor and store in tracker
        let anchor = create_test_anchor(&proof)?;
        let opid = Opid::from(result.state_hash);  // Use state hash as operation ID
        
        self.tracker.add_witness(
            opid,
            txid,
            tx,
            anchor,
            WitnessStatus::Tentative,
        );
        
        // 9. Register genesis seal
        self.tracker.register_seal(genesis_seal, opid, 0)?;
        
        Ok(AssetInfo {
            contract_id,
            genesis_seal,
            genesis_txid: txid,
            total_supply: supply,
        })
    }
    
    /// Transfer tokens
    pub async fn transfer(
        &mut self,
        from_seal: TxoSeal,
        to_seal: TxoSeal,
        amount: u64,
    ) -> Result<TransferInfo> {
        let contract_id = self.active_contract
            .ok_or_else(|| Error::NoActiveContract)?;
        
        log::info!("Transferring {} tokens", amount);
        
        // 1. Execute transfer on F1r3fly
        let result = self.executor.call_method(
            contract_id,
            "transfer",
            &[
                ("from", StrictVal::from(from_seal.to_string())),
                ("to", StrictVal::from(to_seal.to_string())),
                ("amount", StrictVal::from(amount)),
            ],
        ).await?;
        
        // 2. Build Bitcoin transaction spending from_seal
        let from_outpoint = from_seal.to_outpoint()?;
        let (mut psbt, meta) = self.bitcoin.construct_psbt(
            vec![from_outpoint],
            vec![(self.derive_address(&to_seal)?, Sats::from(546u64))],  // Dust limit
            1.0,
        )?;
        
        // 3. Embed Tapret commitment
        let proof = embed_tapret_commitment(&mut psbt, 0, result.state_hash)?;
        
        // 4. Sign, finalize, broadcast (same as issue)
        psbt.sign(&self.create_signer()?)?;
        psbt.finalize(&self.bitcoin.owner.descriptor());
        let tx = psbt.extract()?;
        let txid = tx.txid();
        self.broadcast_tx(&tx).await?;
        
        // 5. Update tracker
        let anchor = create_test_anchor(&proof)?;
        let opid = Opid::from(result.state_hash);
        
        self.tracker.add_witness(opid, txid, tx, anchor, WitnessStatus::Tentative);
        self.tracker.register_seal(to_seal, opid, 0)?;
        
        Ok(TransferInfo {
            txid,
            from_seal,
            to_seal,
            amount,
        })
    }
    
    /// Query balance for a seal
    pub async fn balance(&self, seal: &TxoSeal) -> Result<u64> {
        let contract_id = self.active_contract
            .ok_or_else(|| Error::NoActiveContract)?;
        
        let result = self.executor.query_state(
            contract_id,
            "balanceOf",
            &[("seal", StrictVal::from(seal.to_string()))],
        ).await?;
        
        result.as_u64().ok_or(Error::InvalidBalanceResponse)
    }
    
    // Helper methods...
    fn get_change_address(&self) -> Result<Address> {
        // Get next unused address from descriptor
        todo!("Implement address derivation")
    }
    
    fn derive_address(&self, seal: &TxoSeal) -> Result<Address> {
        // Convert seal to address
        todo!("Implement seal → address conversion")
    }
    
    fn create_signer(&self) -> Result<impl Signer> {
        // Create signer from wallet keys
        todo!("Implement signer creation")
    }
    
    async fn broadcast_tx(&self, tx: &Tx) -> Result<()> {
        // Broadcast via Esplora
        todo!("Implement broadcasting")
    }
}

// Response types
pub struct AssetInfo {
    pub contract_id: ContractId,
    pub genesis_seal: TxoSeal,
    pub genesis_txid: Txid,
    pub total_supply: u64,
}

pub struct TransferInfo {
    pub txid: Txid,
    pub from_seal: TxoSeal,
    pub to_seal: TxoSeal,
    pub amount: u64,
}
```

### Phase 5: Update Test Infrastructure

**Goal**: Update test harness to use new wallet

#### Step 5.1: Simplify Test Harness

**File**: `f1r3fly-rgb/tests/helpers/test_harness.rs`

```rust
// REPLACE existing F1r3flyRgbTestHarness with:

use f1r3fly_rgb::F1r3flyRgbWallet;
use f1r3fly_rgb::BitcoinRegtestHelper;

pub struct F1r3flyRgbTestHarness {
    wallet: F1r3flyRgbWallet,
    bitcoin: BitcoinRegtestHelper,
}

impl F1r3flyRgbTestHarness {
    pub async fn setup() -> Result<Self> {
        let wallet = F1r3flyRgbWallet::new(
            &std::env::var("F1R3NODE_URL")?,
            &std::env::var("FIREFLY_PRIVATE_KEY")?,
            "wpkh([fingerprint]/84'/1'/0'/0/*)",  // Test descriptor
            "http://localhost:3000",  // Esplora mock
            Network::Regtest,
        )?;
        
        let bitcoin = BitcoinRegtestHelper::new("http://localhost:3000").await?;
        
        Ok(Self { wallet, bitcoin })
    }
    
    // All methods now delegate to wallet:
    
    pub async fn issue_tokens(&mut self, ...) -> Result<AssetInfo> {
        self.wallet.issue_asset(...).await
    }
    
    pub async fn transfer_tokens(&mut self, ...) -> Result<TransferInfo> {
        self.wallet.transfer(...).await
    }
    
    pub async fn query_balance(&self, seal: &TxoSeal) -> Result<u64> {
        self.wallet.balance(seal).await
    }
}
```

## Migration Checklist

- [ ] Phase 1: RGB Owner Integration
  - [ ] Add dependencies to Cargo.toml
  - [ ] Create `bitcoin_wallet.rs` with Owner wrapper
  - [ ] Implement `MemUtxos` UTXO set
  - [ ] Test PSBT construction independently

- [ ] Phase 2: State Hash Addition
  - [ ] Update `F1r3flyExecutionResult` with `state_hash`
  - [ ] Implement `compute_state_hash()` function
  - [ ] Update `execute()` to populate state hash
  - [ ] Add `state_commitment()` method
  - [ ] Update all tests to check for state_hash

- [ ] Phase 3: Tapret Embedding
  - [ ] Create `tapret.rs` module
  - [ ] Implement `embed_tapret_commitment()`
  - [ ] Implement `create_anchor()` helpers
  - [ ] Test Tapret embedding with dummy PSBTs

- [ ] Phase 4: Main Wallet
  - [ ] Create `wallet.rs` with `F1r3flyRgbWallet`
  - [ ] Implement `issue_asset()`
  - [ ] Implement `transfer()`
  - [ ] Implement `balance()`
  - [ ] Add helper methods (signing, broadcasting, etc.)

- [ ] Phase 5: Test Updates
  - [ ] Update `test_harness.rs` to use new wallet
  - [ ] Update integration tests
  - [ ] Add E2E test for full lifecycle
  - [ ] Verify all tests pass

## Success Criteria

1. ✅ Can create F1r3fly-RGB wallet with descriptor
2. ✅ Can issue tokens with real Tapret commitments
3. ✅ Can transfer tokens with Bitcoin witness transactions
4. ✅ Can query balances from F1r3fly
5. ✅ State hashes are correctly embedded in Bitcoin
6. ✅ Anchors link F1r3fly state to Bitcoin txids
7. ✅ All existing tests pass with new architecture

## Notes

- This plan maintains backward compatibility with existing `BitcoinAnchorTracker`
- F1r3fly execution layer remains unchanged (contracts still run on F1r3node)
- We're only adding Bitcoin wallet infrastructure, not changing contract logic
- The Tapret commitments are real (using RGB's standard), but MPC/MMB proofs are simplified for F1r3fly's use case

