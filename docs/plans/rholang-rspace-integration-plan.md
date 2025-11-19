# Rholang + RSpace++ RGB Integration Plan

**Date**: November 6, 2025  
**Status**: Architecture Design  
**Approach**: Embedded RSpace++ execution with per-contract isolation

---

## Executive Summary

This document details the implementation plan for replacing RGB's AluVM execution with embedded Rholang + RSpace++. Since RSpace++ and Rholang are already ported to Rust in the forked f1r3node, they can be **embedded directly into RGB** rather than accessed via remote gRPC.

**Key Insight**: The `Blake2b256Hash` root from RSpace checkpoints IS the cryptographic commitment to contract state.

---

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Determinism Requirements](#determinism-requirements)
3. [State Isolation Strategy](#state-isolation-strategy)
4. [Verification Model](#verification-model)
5. [Cryptographic Anchors](#cryptographic-anchors)
6. [Implementation Roadmap](#implementation-roadmap)
7. [Code Modifications Required](#code-modifications-required)

---

## Architecture Overview

### Current vs New Execution Flow

**Current (AluVM):**
```
RGB Operation → Ledger::apply_verify() → Codex::verify() → AluVM bytecode → VerifiedOperation
```

**New (Rholang):**
```
RGB Operation → Ledger::apply_verify() → RholangExecutor → RSpace++ → Checkpoint (state hash)
```

### Key Components

```
sonic/src/ledger.rs
    ↓
rholang_executor/ (NEW MODULE)
    ├── executor.rs          ← ContractRholangExecutor
    ├── anchor.rs            ← RGBAnchor structure
    └── verification.rs      ← Re-execution validation
    ↓
f1r3node/rholang/          ← Rholang compiler/interpreter
    ↓
f1r3node/rspace++/         ← RSpace tuple space storage
```

### Storage Structure (Per Contract)

```
./wallets/{wallet_name}/rgb_data/TokenName.{contract_id}.contract/
├── rholang.rho              ← Rholang source code (replaces codex.yaml)
├── rspace_data/             ← Per-contract RSpace instance
│   ├── history/             (LMDB - merkle history)
│   ├── roots/               (LMDB - root hashes)
│   └── cold/                (LMDB - cold storage)
├── anchors.dat              ← RSpace state hash commitments (NEW)
├── meta.toml                ← Contract metadata (unchanged)
├── genesis.dat              ← Genesis operation (unchanged)
├── semantics.dat            ← Type system (unchanged)
├── stash.aora               ← Operations history (unchanged)
├── trace.aora               ← State transitions (unchanged)
├── spent.aura               ← UTXO tracking (unchanged)
├── read.aora                ← Read dependencies (unchanged)
└── valid.aura               ← Validity flags (unchanged)
```

**Key insight**: ~70% of RGB files remain unchanged. Only execution-related files are replaced.

---

## Determinism Requirements

### Problem Identified

**Location**: `f1r3node/rspace++/src/rspace/rspace.rs` lines 987-996

```rust
fn shuffle_with_index<D>(&self, t: Vec<D>) -> Vec<(D, i32)> {
    let mut rng = thread_rng();  // ← NON-DETERMINISTIC!
    indexed_vec.shuffle(&mut rng);
    indexed_vec
}
```

This shuffling is used for fairness in RChain's multi-user blockchain environment, but introduces non-determinism.

### Solution: Remove Shuffling for RGB

**Rationale:**
- RGB operations are sequential (one wallet at a time)
- No competing continuations in typical RGB use cases
- Simplicity ensures determinism
- Can add deterministic seeded RNG later if needed

**Modification Required:**

```rust
// In f1r3node/rspace++/src/rspace/rspace.rs

fn shuffle_with_index<D>(&self, t: Vec<D>) -> Vec<(D, i32)> {
    // RGB modification: No shuffling for deterministic execution
    // Always use insertion order
    t.into_iter()
        .enumerate()
        .map(|(i, d)| (d, i as i32))
        .collect()
}
```

**Alternative (if fairness needed later):**
```rust
fn shuffle_with_index_deterministic<D>(
    &self, 
    t: Vec<D>, 
    seed: [u8; 32]  // Derived from RGB operation hash
) -> Vec<(D, i32)> {
    use rand::SeedableRng;
    use rand::rngs::StdRng;
    
    let mut rng = StdRng::from_seed(seed);
    let mut indexed_vec = t
        .into_iter()
        .enumerate()
        .map(|(i, d)| (d, i as i32))
        .collect::<Vec<_>>();
    indexed_vec.shuffle(&mut rng);
    indexed_vec
}
```

### Verification

After modification:
1. Same Rholang code + same inputs = same state hash
2. All nodes executing the same operation reach identical state
3. Re-execution produces identical results

---

## State Isolation Strategy

### Design Decision: Per-Contract RSpace Instances

Each RGB contract gets its own isolated RSpace++ storage.

**Why:**
- ✅ Matches RGB's existing isolation model
- ✅ No cross-contract communication possible
- ✅ Clean state separation
- ✅ Contract deletion = delete all associated data
- ✅ No namespace conflicts
- ✅ Simpler security model

**Trade-offs:**
- Each contract uses ~100MB disk space (configurable)
- Multiple LMDB instances (acceptable for RGB's use case)

### Implementation

```rust
// NEW FILE: sonic/src/rholang_executor/executor.rs

use rholang::rust::interpreter::rho_runtime::{create_runtime_from_kv_store, RhoRuntimeImpl};
use rspace_plus_plus::rspace::shared::rspace_store_manager::get_or_create_rspace_store;
use std::path::PathBuf;
use std::sync::Arc;

pub struct ContractRholangExecutor {
    contract_id: ContractId,
    runtime: RhoRuntimeImpl,
    data_dir: PathBuf,
}

impl ContractRholangExecutor {
    /// Creates a new Rholang executor for a specific RGB contract
    pub fn new(
        contract_dir: PathBuf,
        contract_id: ContractId,
    ) -> Result<Self, ExecutorError> {
        // Create isolated RSpace storage for this contract
        let rspace_data_dir = contract_dir.join("rspace_data");
        std::fs::create_dir_all(&rspace_data_dir)?;
        
        // Initialize RSpace stores (history, roots, cold)
        let stores = get_or_create_rspace_store(
            &rspace_data_dir.to_string_lossy(),
            100 * 1024 * 1024  // 100MB per contract
        )?;
        
        // Create matcher for pattern matching
        let matcher = Arc::new(Box::new(Matcher::default()));
        
        // Create Rholang runtime with this contract's RSpace
        let runtime = create_runtime_from_kv_store(
            stores,
            Par::default(),
            true,
            &mut vec![],  // No additional system processes
            matcher,
        ).await?;
        
        Ok(Self {
            contract_id,
            runtime,
            data_dir: rspace_data_dir,
        })
    }
    
    /// Executes Rholang code and returns state commitment
    pub async fn execute_for_rgb(
        &mut self,
        rholang_source: &str,
        opid: Opid,
    ) -> Result<RholangExecutionResult, ExecutorError> {
        // 1. Get current state hash (before execution)
        let prev_checkpoint = self.runtime.space.create_checkpoint()?;
        let prev_state_hash = prev_checkpoint.root;
        
        // 2. Execute Rholang code
        let eval_result = self.runtime.evaluate_with_term(rholang_source).await?;
        
        // 3. Check for execution errors
        if !eval_result.errors.is_empty() {
            return Err(ExecutorError::RholangExecutionError(eval_result.errors));
        }
        
        // 4. Get new state hash (after execution)
        let new_checkpoint = self.runtime.space.create_checkpoint()?;
        let new_state_hash = new_checkpoint.root;  // ← THE CRYPTOGRAPHIC COMMITMENT
        
        // 5. Return execution result with state hashes
        Ok(RholangExecutionResult {
            opid,
            prev_state_hash,
            new_state_hash,
            cost: eval_result.cost,
            event_log: new_checkpoint.log,
        })
    }
}
```

---

## Verification Model

### RGB's Trust Model

RGB follows **complete client-side validation**:
1. Never trust the sender
2. Verify everything from genesis
3. Re-execute all operations
4. Only accept if validation passes

### Rholang Verification Strategy

**Recipients MUST re-execute all Rholang operations.**

```rust
// NEW FILE: sonic/src/rholang_executor/verification.rs

pub struct RholangConsignmentVerifier {
    temp_executor: ContractRholangExecutor,
}

impl RholangConsignmentVerifier {
    /// Verifies a consignment by re-executing all operations
    pub async fn verify_consignment(
        consignment: &Consignment,
    ) -> Result<VerificationResult, ValidationError> {
        
        // 1. Create temporary RSpace for verification
        let temp_dir = create_temp_verification_dir();
        let mut executor = ContractRholangExecutor::new(
            temp_dir,
            consignment.contract_id,
        )?;
        
        // 2. Execute EVERY operation from genesis
        for (idx, operation) in consignment.operations.iter().enumerate() {
            // Load Rholang source for this operation
            let rholang_source = consignment.get_rholang_source(operation)?;
            
            // Execute
            let result = executor.execute_for_rgb(
                &rholang_source,
                operation.opid,
            ).await?;
            
            // Get anchor for this operation
            let anchor = consignment.get_anchor(operation.opid)?;
            
            // Verify state hash matches anchor
            if result.new_state_hash != anchor.rspace_commitment {
                return Err(ValidationError::StateHashMismatch {
                    operation_index: idx,
                    expected: anchor.rspace_commitment,
                    got: result.new_state_hash,
                    opid: operation.opid,
                });
            }
            
            // Verify Rholang source hash
            let source_hash = blake2b_hash(rholang_source.as_bytes());
            if source_hash != anchor.rholang_source_hash {
                return Err(ValidationError::RholangSourceMismatch {
                    operation_index: idx,
                    opid: operation.opid,
                });
            }
        }
        
        // 3. All operations verified successfully
        Ok(VerificationResult::Valid {
            contract_id: consignment.contract_id,
            operations_verified: consignment.operations.len(),
            final_state_hash: executor.current_state_hash(),
        })
    }
}
```

**Why re-execute:**
- ✅ Maintains RGB's trust-minimized security model
- ✅ Detects malicious or incorrect state transitions
- ✅ Verifies Rholang logic correctness
- ✅ No "trusted setup" required
- ✅ Consistent with RGB's existing validation flow

**Performance:**
- RGB contracts typically have < 100 operations
- Rholang execution: microseconds per operation
- Total verification time: seconds (acceptable for RGB)

---

## Cryptographic Anchors

### Anchor Structure

**Location**: `anchors.dat` in each contract directory

```rust
// NEW FILE: sonic/src/rholang_executor/anchor.rs

use super::super::hashing::Blake2b256Hash;

/// Cryptographic anchor linking RGB operation to RSpace state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RGBAnchor {
    /// RGB operation ID
    pub rgb_opid: Opid,
    
    /// RSpace state hash BEFORE execution
    pub rspace_prev_hash: Blake2b256Hash,
    
    /// RSpace state hash AFTER execution
    /// This is the PRIMARY cryptographic commitment
    pub rspace_commitment: Blake2b256Hash,
    
    /// Hash of the Rholang source code executed
    pub rholang_source_hash: Blake2b256Hash,
    
    /// Timestamp when anchor was created
    pub timestamp: u64,
}

impl RGBAnchor {
    /// Creates a new anchor from execution result
    pub fn from_execution_result(
        result: &RholangExecutionResult,
        rholang_source: &str,
    ) -> Self {
        Self {
            rgb_opid: result.opid,
            rspace_prev_hash: result.prev_state_hash,
            rspace_commitment: result.new_state_hash,
            rholang_source_hash: blake2b_hash(rholang_source.as_bytes()),
            timestamp: current_timestamp(),
        }
    }
    
    /// Verifies an anchor against an execution result
    pub fn verify(&self, result: &RholangExecutionResult) -> Result<(), AnchorError> {
        if self.rgb_opid != result.opid {
            return Err(AnchorError::OpidMismatch);
        }
        if self.rspace_commitment != result.new_state_hash {
            return Err(AnchorError::StateHashMismatch);
        }
        Ok(())
    }
}

/// Storage for all anchors in a contract
pub struct AnchorStore {
    file_path: PathBuf,
    anchors: Vec<RGBAnchor>,
}

impl AnchorStore {
    /// Loads anchors from anchors.dat
    pub fn load(contract_dir: &Path) -> Result<Self, AnchorError> {
        let file_path = contract_dir.join("anchors.dat");
        
        if !file_path.exists() {
            return Ok(Self {
                file_path,
                anchors: Vec::new(),
            });
        }
        
        let data = std::fs::read(&file_path)?;
        let anchors: Vec<RGBAnchor> = bincode::deserialize(&data)?;
        
        Ok(Self { file_path, anchors })
    }
    
    /// Adds a new anchor and persists to disk
    pub fn add_anchor(&mut self, anchor: RGBAnchor) -> Result<(), AnchorError> {
        self.anchors.push(anchor);
        self.save()
    }
    
    /// Persists anchors to disk
    fn save(&self) -> Result<(), AnchorError> {
        let data = bincode::serialize(&self.anchors)?;
        std::fs::write(&self.file_path, data)?;
        Ok(())
    }
    
    /// Gets anchor by operation ID
    pub fn get_anchor(&self, opid: Opid) -> Option<&RGBAnchor> {
        self.anchors.iter().find(|a| a.rgb_opid == opid)
    }
}
```

### Why Minimal Anchors

**Store only essential data:**
- RGB operation ID (links to existing RGB history)
- RSpace state hashes (cryptographic commitments)
- Rholang source hash (verifies executed code)

**Don't store:**
- ❌ Event logs - RGB already stores complete operation history in `stash.aora`
- ❌ Full event details - Recipients re-execute anyway
- ❌ Intermediate state - Only final state hash matters

**Benefits:**
- ✅ Smaller consignments
- ✅ Faster validation
- ✅ Leverages RGB's existing history mechanism
- ✅ No duplication of data

---

## Implementation Roadmap

### Phase 1: Foundation (Week 1)

**Goal**: Set up basic Rholang execution infrastructure

| Task | File | Deliverable |
|------|------|-------------|
| Create rholang_executor module | `sonic/src/rholang_executor/mod.rs` | Module structure |
| Implement ContractRholangExecutor | `sonic/src/rholang_executor/executor.rs` | Basic execution |
| Implement RGBAnchor structure | `sonic/src/rholang_executor/anchor.rs` | Anchor storage |
| Fix RSpace determinism | `f1r3node/rspace++/src/rspace/rspace.rs` | Remove shuffle |
| Add Cargo dependencies | `sonic/Cargo.toml` | Link to rholang/rspace++ |

**Estimated time**: 40 hours

---

### Phase 2: Ledger Integration (Week 2)

**Goal**: Replace AluVM calls with Rholang execution

| Task | File | Deliverable |
|------|------|-------------|
| Modify `Ledger::apply_verify()` | `sonic/src/ledger.rs` | Route to Rholang |
| Load Rholang source from contract | `sonic/src/rholang_executor/loader.rs` | Source loading |
| Generate anchors post-execution | `sonic/src/rholang_executor/executor.rs` | Anchor creation |
| Persist anchors to disk | `sonic/src/rholang_executor/anchor.rs` | Save to anchors.dat |
| Handle execution errors | `sonic/src/rholang_executor/errors.rs` | Error types |

**Estimated time**: 32 hours

---

### Phase 3: Verification (Week 3)

**Goal**: Implement consignment verification

| Task | File | Deliverable |
|------|------|-------------|
| Implement verification logic | `sonic/src/rholang_executor/verification.rs` | Re-execution |
| Integrate with consignment acceptance | `rgb-std/src/contracts.rs` | Verify on import |
| Add anchor validation | `sonic/src/rholang_executor/anchor.rs` | Verify state hashes |
| Handle verification failures | `sonic/src/rholang_executor/errors.rs` | Validation errors |
| Temporary RSpace cleanup | `sonic/src/rholang_executor/verification.rs` | Cleanup temp dirs |

**Estimated time**: 40 hours

---

### Phase 4: Rho20 Standard (Week 4)

**Goal**: Create Rholang equivalent of RGB20

| Task | File | Deliverable |
|------|------|-------------|
| Design Rho20 Rholang contract | `examples/rho20_template.rho` | Fungible token |
| Implement transfer logic | Rholang code | Transfer function |
| Implement balance queries | Rholang code | Query functions |
| Create issuer generation | `sonic/examples/rho20/main.rs` | `.rholang.issuer` |
| Test asset issuance | Integration test | Issue Rho20 token |

**Estimated time**: 32 hours

---

### Phase 5: Testing & Integration (Week 5)

**Goal**: End-to-end working system

| Task | Deliverable |
|------|-------------|
| Unit tests for executor | Test RSpace state hashing |
| Unit tests for anchors | Test anchor creation/verification |
| Integration test: Issuance | Issue Rho20 asset |
| Integration test: Transfer | Send/receive tokens |
| Integration test: Consignment | Accept & verify consignment |
| Integration test: Re-execution | Verify determinism |
| Performance benchmarks | Measure execution speed |
| Documentation | User guide & API docs |

**Estimated time**: 48 hours

---

### Total Implementation Time

**Estimated**: **192 hours (~5 weeks)**

---

## Code Modifications Required

### File 1: `sonic/src/ledger.rs` (CRITICAL)

**Current** (Line ~558):
```rust
pub fn apply_verify(
    &mut self,
    operation: Operation,
    force: bool,
) -> Result<bool, MultiError<AcceptError, S::Error>> {
    // ...
    let verified = articles
        .codex()
        .verify(self.contract_id(), operation, &self.0.state().raw, articles)  // ← AluVM
        .map_err(AcceptError::from)?;
    
    self.apply_internal(opid, verified, present)?;
    Ok(present)
}
```

**Replace With**:
```rust
pub fn apply_verify(
    &mut self,
    operation: Operation,
    force: bool,
) -> Result<bool, MultiError<AcceptError, S::Error>> {
    // ...
    
    // Check if this is a Rholang contract
    let execution_result = if articles.has_rholang_source() {
        // NEW: Execute via Rholang
        let mut executor = self.get_or_create_rholang_executor()?;
        let rholang_source = articles.rholang_source()?;
        
        let result = executor
            .execute_for_rgb(&rholang_source, operation.opid())
            .await
            .map_err(AcceptError::from)?;
        
        // Create and store anchor
        let anchor = RGBAnchor::from_execution_result(&result, &rholang_source);
        self.store_anchor(anchor)?;
        
        // Convert to VerifiedOperation
        VerifiedOperation::from_rholang_result(result)
    } else {
        // Legacy: Execute via AluVM
        articles
            .codex()
            .verify(self.contract_id(), operation, &self.0.state().raw, articles)
            .map_err(AcceptError::from)?
    };
    
    self.apply_internal(opid, execution_result, present)?;
    Ok(present)
}
```

---

### File 2: `f1r3node/rspace++/src/rspace/rspace.rs` (DETERMINISM FIX)

**Current** (Line 987):
```rust
fn shuffle_with_index<D>(&self, t: Vec<D>) -> Vec<(D, i32)> {
    let mut rng = thread_rng();  // NON-DETERMINISTIC
    let mut indexed_vec = t
        .into_iter()
        .enumerate()
        .map(|(i, d)| (d, i as i32))
        .collect::<Vec<_>>();
    indexed_vec.shuffle(&mut rng);
    indexed_vec
}
```

**Replace With**:
```rust
fn shuffle_with_index<D>(&self, t: Vec<D>) -> Vec<(D, i32)> {
    // RGB modification: No shuffling for deterministic execution
    // Always use insertion order
    t.into_iter()
        .enumerate()
        .map(|(i, d)| (d, i as i32))
        .collect()
}
```

---

### File 3: `rgb-std/src/contracts.rs` (VERIFICATION)

**Add new method**:
```rust
impl<S: Stock> Contracts<S> {
    /// Verifies a consignment with Rholang re-execution
    pub fn accept_rholang_consignment(
        &mut self,
        consignment: Consignment,
    ) -> Result<(), ValidationError> {
        // Verify it's a Rholang contract
        if !consignment.is_rholang_contract() {
            return Err(ValidationError::NotRholangContract);
        }
        
        // Re-execute all operations and verify anchors
        let verification_result = RholangConsignmentVerifier::verify_consignment(&consignment)
            .await?;
        
        match verification_result {
            VerificationResult::Valid { .. } => {
                // Standard RGB import
                self.accept(consignment, resolver, false)
            }
            VerificationResult::Invalid(err) => {
                Err(ValidationError::RholangVerificationFailed(err))
            }
        }
    }
}
```

---

### File 4: `sonic/Cargo.toml` (DEPENDENCIES)

**Add**:
```toml
[dependencies]
# ... existing dependencies ...

# Rholang + RSpace++
rholang = { path = "../f1r3node/rholang" }
rspace_plus_plus = { path = "../f1r3node/rspace++" }
models = { path = "../f1r3node/models" }

# Required for Rholang runtime
tokio = { version = "1.40", features = ["full"] }
```

---

## Success Criteria

### Functional Requirements

- ✅ Issue Rho20 fungible token
- ✅ Transfer tokens between wallets
- ✅ Generate and send consignments
- ✅ Verify received consignments via re-execution
- ✅ Deterministic execution (same input = same state hash)
- ✅ State isolation per contract

### Performance Requirements

- ✅ Asset issuance: < 5 seconds
- ✅ Transfer: < 3 seconds
- ✅ Consignment verification: < 10 seconds (for typical ~10 operations)
- ✅ RSpace overhead: < 200MB per contract

### Security Requirements

- ✅ No cross-contract state leakage
- ✅ Recipients can fully verify all operations
- ✅ Anchors cryptographically bind RGB ↔ RSpace state
- ✅ Malicious state transitions are detected

---

## Future Enhancements

### Phase 6+: Advanced Features

1. **Shared RSpace option**
   - Optional global RSpace for cross-contract reads (not writes)
   - User configurable

2. **AluVM ↔ Rholang bridge**
   - Allow Rholang contracts to interact with legacy AluVM contracts
   - Read-only bridge initially

3. **Optimized verification**
   - Cache verified checkpoints
   - Skip re-execution if trusted checkpoint exists

4. **Multi-node support**
   - Optional f1r3node integration for users who want shared execution
   - Keep embedded execution as default

---

## Appendix: Key Data Structures

### RholangExecutionResult

```rust
pub struct RholangExecutionResult {
    /// RGB operation ID
    pub opid: Opid,
    
    /// RSpace state hash before execution
    pub prev_state_hash: Blake2b256Hash,
    
    /// RSpace state hash after execution (THE COMMITMENT)
    pub new_state_hash: Blake2b256Hash,
    
    /// Execution cost
    pub cost: Cost,
    
    /// Event log (produce/consume events)
    pub event_log: Log,
}
```

### Checkpoint (from RSpace)

```rust
pub struct Checkpoint {
    /// Merkle root of RSpace state
    pub root: Blake2b256Hash,  // ← The cryptographic commitment
    
    /// Event log of operations
    pub log: Log,
}
```

### Contract Directory Structure

```
TokenName.{contract_id}.contract/
├── rholang.rho              # Rholang source
├── rspace_data/             # Per-contract RSpace
│   ├── history/
│   ├── roots/
│   └── cold/
├── anchors.dat              # State hash commitments
└── [existing RGB files]     # Unchanged
```

---

**End of Implementation Plan**

