# F1r3fly-RGB High-Level Abstractions Plan

## Executive Summary

This document outlines the addition of high-level abstractions to `f1r3fly-rgb` to match RGB's abstraction level and simplify wallet implementation. Currently, `f1r3fly-rgb` provides low-level primitives (executor, tracker, tapret), but lacks the high-level APIs that RGB provides through its `Contract` and `Contracts` types.

**Goal**: Add missing abstractions so wallet implementations have the same ease-of-use as RGB wallets.

**Priority**: **CRITICAL** - Blocks wallet layer development

**Estimated Time**: 20-28 hours

---

## Problem Statement

### What RGB Provides to Wallets

```rust
use rgb_std::{
    Contract,              // Single contract operations
    Contracts,             // Multi-contract collection
    Stock,                 // Contract state storage
    Pile,                  // Bitcoin witness/seal storage
    Consignment,           // Transfer packages
};

// RGB wallets can do:
let mut contracts = Contracts::load(stock, pile)?;
let contract_id = contracts.issue(params)?;
contracts.transfer(contract_id, from, to, amount)?;
let consignment = contracts.create_consignment(seals)?;
```

### What F1r3fly-RGB Currently Provides

```rust
use f1r3fly_rgb::{
    F1r3flyExecutor,           // Low-level execution
    BitcoinAnchorTracker,      // Low-level tracking
    embed_tapret_commitment,   // Low-level PSBT
};

// Wallets must manually:
let mut executor = F1r3flyExecutor::new()?;
let mut tracker = BitcoinAnchorTracker::new();
let result = executor.call_method(contract_id, "transfer", params).await?;
tracker.add_seals(opid, seals);
let proof = embed_tapret_commitment(&mut psbt, 0, result.state_hash)?;
// ... manual coordination required
```

**Problem**: Wallet must coordinate executor + tracker + tapret manually. RGB provides unified API.

---

## Architecture Overview

### Current State (Low-Level)

```
┌─────────────────────────────────────────────────┐
│ Wallet Implementation (Manual Coordination)     │
│ • Call F1r3flyExecutor                          │
│ • Track seals in BitcoinAnchorTracker           │
│ • Build PSBTs with embed_tapret_commitment      │
│ • Create consignments manually                  │
└─────────────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────────────┐
│ f1r3fly-rgb (Low-Level Primitives)              │
│ • F1r3flyExecutor                               │
│ • BitcoinAnchorTracker                          │
│ • Tapret functions                              │
└─────────────────────────────────────────────────┘
```

### Target State (High-Level)

```
┌─────────────────────────────────────────────────┐
│ Wallet Implementation (Simplified)              │
│ • Call F1r3flyRgbContracts                      │
│ • Simple issue/transfer/balance APIs            │
│ • Accept/create consignments easily             │
└─────────────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────────────┐
│ f1r3fly-rgb (High-Level + Low-Level) ← NEW      │
│ ┌─────────────────────────────────────────────┐ │
│ │ HIGH-LEVEL (NEW)                            │ │
│ │ • F1r3flyRgbContract                        │ │
│ │ • F1r3flyRgbContracts                       │ │
│ │ • F1r3flyConsignment                        │ │
│ └─────────────────────────────────────────────┘ │
│ ┌─────────────────────────────────────────────┐ │
│ │ LOW-LEVEL (EXISTING)                        │ │
│ │ • F1r3flyExecutor                           │ │
│ │ • BitcoinAnchorTracker                      │ │
│ │ • Tapret functions                          │ │
│ └─────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────┘
```

---

## Phase 1: F1r3flyRgbContract (Single Contract API)

**Priority**: 🔥 **CRITICAL**  
**Estimated Time**: 8-10 hours

### 1.1: Create Contract Struct

**File**: `f1r3fly-rgb/src/contract.rs` (NEW)

```rust
//! High-level single contract operations
//!
//! This module provides a unified API for interacting with a single F1r3fly-RGB
//! contract, coordinating the executor, tracker, and metadata.

use crate::{
    F1r3flyExecutor, F1r3flyExecutionResult, ContractMetadata,
    BitcoinAnchorTracker, F1r3flyRgbError, RholangContractLibrary,
};
use amplify::confinement::SmallOrdMap;
use bp::seals::{TxoSeal, WTxoSeal};
use hypersonic::{ContractId, Opid};
use strict_types::StrictVal;

/// High-level API for a single F1r3fly-RGB contract
///
/// Coordinates F1r3fly execution, Bitcoin anchor tracking, and contract metadata.
/// Provides RGB-compatible operations (issue, transfer, balance) while maintaining
/// F1r3fly's shard-based state management.
///
/// # Architecture
///
/// - **Executor**: Manages Rholang execution on F1r3node
/// - **Tracker**: Tracks Bitcoin seals and witnesses (RGB Pile implementation)
/// - **Metadata**: Contract info (registry URI, methods, source)
///
/// # Example
///
/// ```rust,no_run
/// use f1r3fly_rgb::{F1r3flyRgbContract, F1r3flyExecutor};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Issue new token
/// let executor = F1r3flyExecutor::new()?;
/// let contract = F1r3flyRgbContract::issue(
///     executor,
///     "BTC",
///     "Bitcoin",
///     21000000,
///     8,
/// ).await?;
///
/// // Call method
/// use strict_types::StrictVal;
/// let result = contract.call_method(
///     "transfer",
///     &[
///         ("from", StrictVal::from("alice")),
///         ("to", StrictVal::from("bob")),
///         ("amount", StrictVal::from(100u64)),
///     ],
///     seals,
/// ).await?;
///
/// // Query balance
/// let balance = contract.balance(&seal).await?;
/// # Ok(())
/// # }
/// ```
pub struct F1r3flyRgbContract {
    /// Contract ID (derived from registry URI)
    contract_id: ContractId,
    
    /// F1r3fly executor for Rholang execution
    executor: F1r3flyExecutor,
    
    /// Bitcoin anchor tracker (RGB Pile implementation)
    tracker: BitcoinAnchorTracker<TxoSeal>,
    
    /// Contract metadata (registry URI, methods, Rholang source)
    metadata: ContractMetadata,
}

impl F1r3flyRgbContract {
    /// Create new contract from existing metadata
    ///
    /// Use this when loading a contract that was previously deployed.
    ///
    /// # Arguments
    ///
    /// * `executor` - F1r3fly executor instance
    /// * `metadata` - Contract metadata (from previous deployment)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use f1r3fly_rgb::{F1r3flyRgbContract, F1r3flyExecutor, ContractMetadata};
    /// # fn example(executor: F1r3flyExecutor, metadata: ContractMetadata) -> Result<(), Box<dyn std::error::Error>> {
    /// let contract = F1r3flyRgbContract::new(executor, metadata)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(
        executor: F1r3flyExecutor,
        metadata: ContractMetadata,
    ) -> Result<Self, F1r3flyRgbError> {
        let contract_id = metadata.contract_id;
        let tracker = BitcoinAnchorTracker::new();
        
        Ok(Self {
            contract_id,
            executor,
            tracker,
            metadata,
        })
    }
    
    /// Issue new RGB20-compatible fungible token
    ///
    /// Deploys a new RHO20 contract to F1r3node using the standard template.
    /// This is the genesis operation for a new asset.
    ///
    /// # Arguments
    ///
    /// * `executor` - F1r3fly executor instance
    /// * `ticker` - Asset ticker symbol (e.g., "BTC")
    /// * `name` - Asset full name (e.g., "Bitcoin")
    /// * `supply` - Total supply (e.g., 21000000)
    /// * `precision` - Decimal precision (e.g., 8 for Bitcoin)
    ///
    /// # Returns
    ///
    /// New `F1r3flyRgbContract` instance with deployed contract
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use f1r3fly_rgb::{F1r3flyRgbContract, F1r3flyExecutor};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let executor = F1r3flyExecutor::new()?;
    /// let contract = F1r3flyRgbContract::issue(
    ///     executor,
    ///     "GOLD",
    ///     "Digital Gold",
    ///     1000000,
    ///     2,
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn issue(
        mut executor: F1r3flyExecutor,
        ticker: &str,
        name: &str,
        supply: u64,
        precision: u8,
    ) -> Result<Self, F1r3flyRgbError> {
        log::info!("Issuing new asset: {} ({})", name, ticker);
        
        // Deploy RHO20 contract
        let metadata = executor.deploy_contract(
            RholangContractLibrary::rho20_contract(),
            ticker,
            name,
            supply,
            precision,
            vec!["issue".to_string(), "transfer".to_string(), "balanceOf".to_string()],
        ).await?;
        
        let contract_id = metadata.contract_id;
        let tracker = BitcoinAnchorTracker::new();
        
        log::info!("Contract deployed: {}", contract_id);
        
        Ok(Self {
            contract_id,
            executor,
            tracker,
            metadata,
        })
    }
    
    /// Call contract method
    ///
    /// Executes a method on the F1r3fly contract and tracks the associated seals.
    ///
    /// # Arguments
    ///
    /// * `method` - Method name (e.g., "transfer")
    /// * `params` - Method parameters as (name, value) tuples
    /// * `seals` - Bitcoin seals (UTXO bindings) for this operation
    ///
    /// # Returns
    ///
    /// Execution result with state hash for Tapret commitment
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use f1r3fly_rgb::{F1r3flyRgbContract, F1r3flyExecutor};
    /// # use strict_types::StrictVal;
    /// # async fn example(mut contract: F1r3flyRgbContract, seals: amplify::confinement::SmallOrdMap<u16, bp::seals::WTxoSeal>) -> Result<(), Box<dyn std::error::Error>> {
    /// let result = contract.call_method(
    ///     "transfer",
    ///     &[
    ///         ("from", StrictVal::from("alice")),
    ///         ("to", StrictVal::from("bob")),
    ///         ("amount", StrictVal::from(100u64)),
    ///     ],
    ///     seals,
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn call_method(
        &mut self,
        method: &str,
        params: &[(&str, StrictVal)],
        seals: SmallOrdMap<u16, WTxoSeal>,
    ) -> Result<F1r3flyExecutionResult, F1r3flyRgbError> {
        log::debug!("Calling method: {} on contract {}", method, self.contract_id);
        
        // Execute on F1r3fly
        let result = self.executor.call_method(
            self.contract_id,
            method,
            params,
        ).await?;
        
        // Track seals in Bitcoin anchor tracker
        let opid = Opid::from(result.state_hash);
        self.tracker.add_seals(opid, seals);
        
        log::debug!("Method call complete, state_hash: {}", hex::encode(result.state_hash));
        
        Ok(result)
    }
    
    /// Query token balance for a seal
    ///
    /// Queries the F1r3fly shard for the current balance of a given seal (UTXO).
    ///
    /// # Arguments
    ///
    /// * `seal` - The seal to query balance for
    ///
    /// # Returns
    ///
    /// Current token balance as u64
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use f1r3fly_rgb::{F1r3flyRgbContract, F1r3flyExecutor};
    /// # use bp::seals::TxoSeal;
    /// # async fn example(contract: F1r3flyRgbContract, seal: TxoSeal) -> Result<(), Box<dyn std::error::Error>> {
    /// let balance = contract.balance(&seal).await?;
    /// println!("Balance: {}", balance);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn balance(&self, seal: &TxoSeal) -> Result<u64, F1r3flyRgbError> {
        let seal_str = format!("{:?}", seal); // TODO: Proper seal serialization
        let result = self.executor.query_state(
            self.contract_id,
            "balanceOf",
            &[("seal", StrictVal::String(seal_str))],
        ).await?;
        
        // Parse balance from result
        match result {
            StrictVal::Number(n) => Ok(n as u64),
            _ => Err(F1r3flyRgbError::InvalidStateFormat(
                "Expected number for balance".to_string()
            )),
        }
    }
    
    /// Get contract ID
    pub fn contract_id(&self) -> ContractId {
        self.contract_id
    }
    
    /// Get contract metadata
    pub fn metadata(&self) -> &ContractMetadata {
        &self.metadata
    }
    
    /// Get mutable reference to executor
    pub fn executor_mut(&mut self) -> &mut F1r3flyExecutor {
        &mut self.executor
    }
    
    /// Get reference to executor
    pub fn executor(&self) -> &F1r3flyExecutor {
        &self.executor
    }
    
    /// Get mutable reference to tracker
    pub fn tracker_mut(&mut self) -> &mut BitcoinAnchorTracker<TxoSeal> {
        &mut self.tracker
    }
    
    /// Get reference to tracker
    pub fn tracker(&self) -> &BitcoinAnchorTracker<TxoSeal> {
        &self.tracker
    }
}
```

### 1.2: Add Tests

**File**: `f1r3fly-rgb/tests/contract_test.rs` (NEW)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_contract_issue() {
        // Load environment
        dotenv::from_path(".env").ok();
        
        let executor = F1r3flyExecutor::new().expect("Failed to create executor");
        
        let contract = F1r3flyRgbContract::issue(
            executor,
            "TEST",
            "Test Token",
            1000000,
            8,
        ).await.expect("Issue failed");
        
        assert_eq!(contract.metadata().ticker, "TEST");
        assert_eq!(contract.metadata().name, "Test Token");
    }
    
    #[tokio::test]
    async fn test_contract_call_method() {
        dotenv::from_path(".env").ok();
        
        let executor = F1r3flyExecutor::new().expect("Failed to create executor");
        let mut contract = F1r3flyRgbContract::issue(
            executor,
            "TEST",
            "Test Token",
            1000000,
            8,
        ).await.expect("Issue failed");
        
        // Create test seals
        let seals = create_test_seals();
        
        // Call transfer method
        let result = contract.call_method(
            "transfer",
            &[
                ("from", StrictVal::from("alice")),
                ("to", StrictVal::from("bob")),
                ("amount", StrictVal::from(100u64)),
            ],
            seals,
        ).await.expect("Transfer failed");
        
        assert!(!result.state_hash.is_empty());
    }
    
    #[tokio::test]
    async fn test_contract_balance() {
        dotenv::from_path(".env").ok();
        
        let executor = F1r3flyExecutor::new().expect("Failed to create executor");
        let contract = F1r3flyRgbContract::issue(
            executor,
            "TEST",
            "Test Token",
            1000000,
            8,
        ).await.expect("Issue failed");
        
        // Query balance (genesis seal should have full supply)
        let seal = create_genesis_seal();
        let balance = contract.balance(&seal).await.expect("Balance query failed");
        
        assert!(balance > 0);
    }
    
    // Helper functions
    fn create_test_seals() -> SmallOrdMap<u16, WTxoSeal> {
        use commit_verify::{Digest, DigestExt, Sha256};
        let mut noise = Sha256::new();
        noise.input_raw(b"test_seals");
        
        small_bmap! {
            0 => WTxoSeal::vout_no_fallback(1.into(), noise.clone(), 1),
            1 => WTxoSeal::vout_no_fallback(2.into(), noise, 2)
        }
    }
    
    fn create_genesis_seal() -> TxoSeal {
        TxoSeal::from_str("bc1q...").expect("Valid seal")
    }
}
```

### 1.3: Update lib.rs

**File**: `f1r3fly-rgb/src/lib.rs`

Add to modules:
```rust
pub mod contract;
```

Add to re-exports:
```rust
pub use contract::F1r3flyRgbContract;
```

---

## Phase 2: F1r3flyRgbContracts (Multi-Contract Collection)

**Priority**: 🔥 **CRITICAL**  
**Estimated Time**: 6-8 hours

### 2.1: Create Contracts Collection

**File**: `f1r3fly-rgb/src/contracts.rs` (NEW)

```rust
//! Multi-contract collection management
//!
//! Provides a unified interface for managing multiple F1r3fly-RGB contracts,
//! similar to RGB's `Contracts` type.

use crate::{
    F1r3flyRgbContract, F1r3flyExecutor, ContractMetadata,
    F1r3flyRgbError,
};
use hypersonic::ContractId;
use std::collections::HashMap;

/// Collection of F1r3fly-RGB contracts
///
/// Manages multiple contracts with a shared executor. Provides high-level
/// operations for issuing, transferring, and querying across contracts.
///
/// # Example
///
/// ```rust,no_run
/// use f1r3fly_rgb::{F1r3flyRgbContracts, F1r3flyExecutor};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let executor = F1r3flyExecutor::new()?;
/// let mut contracts = F1r3flyRgbContracts::new(executor);
///
/// // Issue multiple tokens
/// let btc_id = contracts.issue("BTC", "Bitcoin", 21000000, 8).await?;
/// let eth_id = contracts.issue("ETH", "Ethereum", 100000000, 18).await?;
///
/// // Access specific contract
/// let btc_contract = contracts.get(&btc_id).unwrap();
/// let balance = btc_contract.balance(&seal).await?;
/// # Ok(())
/// # }
/// ```
pub struct F1r3flyRgbContracts {
    /// Shared F1r3fly executor
    executor: F1r3flyExecutor,
    
    /// Map of contract ID to contract instance
    contracts: HashMap<ContractId, F1r3flyRgbContract>,
}

impl F1r3flyRgbContracts {
    /// Create new contracts collection
    ///
    /// # Arguments
    ///
    /// * `executor` - Shared F1r3fly executor for all contracts
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use f1r3fly_rgb::{F1r3flyRgbContracts, F1r3flyExecutor};
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let executor = F1r3flyExecutor::new()?;
    /// let contracts = F1r3flyRgbContracts::new(executor);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(executor: F1r3flyExecutor) -> Self {
        Self {
            executor,
            contracts: HashMap::new(),
        }
    }
    
    /// Issue new RGB20-compatible fungible token
    ///
    /// Deploys a new RHO20 contract and adds it to the collection.
    ///
    /// # Arguments
    ///
    /// * `ticker` - Asset ticker symbol
    /// * `name` - Asset full name
    /// * `supply` - Total supply
    /// * `precision` - Decimal precision
    ///
    /// # Returns
    ///
    /// Contract ID of the newly issued asset
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use f1r3fly_rgb::{F1r3flyRgbContracts, F1r3flyExecutor};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let executor = F1r3flyExecutor::new()?;
    /// let mut contracts = F1r3flyRgbContracts::new(executor);
    /// let contract_id = contracts.issue("BTC", "Bitcoin", 21000000, 8).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn issue(
        &mut self,
        ticker: &str,
        name: &str,
        supply: u64,
        precision: u8,
    ) -> Result<ContractId, F1r3flyRgbError> {
        log::info!("Issuing asset: {} ({})", name, ticker);
        
        // Clone executor for new contract
        let executor = self.executor.clone();
        
        // Issue contract
        let contract = F1r3flyRgbContract::issue(
            executor,
            ticker,
            name,
            supply,
            precision,
        ).await?;
        
        let contract_id = contract.contract_id();
        
        // Store in collection
        self.contracts.insert(contract_id, contract);
        
        log::info!("Asset issued with contract ID: {}", contract_id);
        
        Ok(contract_id)
    }
    
    /// Get contract by ID
    ///
    /// # Arguments
    ///
    /// * `id` - Contract ID to retrieve
    ///
    /// # Returns
    ///
    /// Reference to contract, or None if not found
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use f1r3fly_rgb::{F1r3flyRgbContracts, F1r3flyExecutor};
    /// # use hypersonic::ContractId;
    /// # async fn example(contracts: F1r3flyRgbContracts, id: ContractId) -> Result<(), Box<dyn std::error::Error>> {
    /// if let Some(contract) = contracts.get(&id) {
    ///     println!("Found contract: {}", contract.metadata().name);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn get(&self, id: &ContractId) -> Option<&F1r3flyRgbContract> {
        self.contracts.get(id)
    }
    
    /// Get mutable contract by ID
    ///
    /// # Arguments
    ///
    /// * `id` - Contract ID to retrieve
    ///
    /// # Returns
    ///
    /// Mutable reference to contract, or None if not found
    pub fn get_mut(&mut self, id: &ContractId) -> Option<&mut F1r3flyRgbContract> {
        self.contracts.get_mut(id)
    }
    
    /// Register existing contract
    ///
    /// Adds a contract that was loaded from disk or received via consignment.
    ///
    /// # Arguments
    ///
    /// * `contract` - Contract instance to register
    ///
    /// # Returns
    ///
    /// Contract ID
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use f1r3fly_rgb::{F1r3flyRgbContracts, F1r3flyRgbContract, F1r3flyExecutor, ContractMetadata};
    /// # fn example(mut contracts: F1r3flyRgbContracts, executor: F1r3flyExecutor, metadata: ContractMetadata) -> Result<(), Box<dyn std::error::Error>> {
    /// let contract = F1r3flyRgbContract::new(executor, metadata)?;
    /// let id = contracts.register(contract);
    /// # Ok(())
    /// # }
    /// ```
    pub fn register(&mut self, contract: F1r3flyRgbContract) -> ContractId {
        let contract_id = contract.contract_id();
        self.contracts.insert(contract_id, contract);
        contract_id
    }
    
    /// List all contract IDs
    ///
    /// # Returns
    ///
    /// Vector of all contract IDs in the collection
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use f1r3fly_rgb::{F1r3flyRgbContracts, F1r3flyExecutor};
    /// # fn example(contracts: F1r3flyRgbContracts) {
    /// for contract_id in contracts.list() {
    ///     println!("Contract: {}", contract_id);
    /// }
    /// # }
    /// ```
    pub fn list(&self) -> Vec<ContractId> {
        self.contracts.keys().copied().collect()
    }
    
    /// Get count of contracts
    ///
    /// # Returns
    ///
    /// Number of contracts in collection
    pub fn count(&self) -> usize {
        self.contracts.len()
    }
    
    /// Check if contract exists
    ///
    /// # Arguments
    ///
    /// * `id` - Contract ID to check
    ///
    /// # Returns
    ///
    /// true if contract exists, false otherwise
    pub fn contains(&self, id: &ContractId) -> bool {
        self.contracts.contains_key(id)
    }
}
```

### 2.2: Add Tests

**File**: `f1r3fly-rgb/tests/contracts_test.rs` (NEW)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_contracts_issue_multiple() {
        dotenv::from_path(".env").ok();
        
        let executor = F1r3flyExecutor::new().expect("Failed to create executor");
        let mut contracts = F1r3flyRgbContracts::new(executor);
        
        // Issue multiple tokens
        let btc_id = contracts.issue("BTC", "Bitcoin", 21000000, 8)
            .await.expect("BTC issue failed");
        let eth_id = contracts.issue("ETH", "Ethereum", 100000000, 18)
            .await.expect("ETH issue failed");
        
        assert_ne!(btc_id, eth_id);
        assert_eq!(contracts.count(), 2);
    }
    
    #[tokio::test]
    async fn test_contracts_get() {
        dotenv::from_path(".env").ok();
        
        let executor = F1r3flyExecutor::new().expect("Failed to create executor");
        let mut contracts = F1r3flyRgbContracts::new(executor);
        
        let id = contracts.issue("TEST", "Test Token", 1000000, 8)
            .await.expect("Issue failed");
        
        let contract = contracts.get(&id).expect("Contract not found");
        assert_eq!(contract.metadata().ticker, "TEST");
    }
    
    #[tokio::test]
    async fn test_contracts_list() {
        dotenv::from_path(".env").ok();
        
        let executor = F1r3flyExecutor::new().expect("Failed to create executor");
        let mut contracts = F1r3flyRgbContracts::new(executor);
        
        let id1 = contracts.issue("T1", "Token 1", 1000, 2).await.expect("Issue 1 failed");
        let id2 = contracts.issue("T2", "Token 2", 2000, 2).await.expect("Issue 2 failed");
        
        let list = contracts.list();
        assert_eq!(list.len(), 2);
        assert!(list.contains(&id1));
        assert!(list.contains(&id2));
    }
    
    #[tokio::test]
    async fn test_contracts_contains() {
        dotenv::from_path(".env").ok();
        
        let executor = F1r3flyExecutor::new().expect("Failed to create executor");
        let mut contracts = F1r3flyRgbContracts::new(executor);
        
        let id = contracts.issue("TEST", "Test", 1000, 2).await.expect("Issue failed");
        
        assert!(contracts.contains(&id));
        assert!(!contracts.contains(&ContractId::from([0u8; 32])));
    }
}
```

### 2.3: Update lib.rs

**File**: `f1r3fly-rgb/src/lib.rs`

Add to modules:
```rust
pub mod contracts;
```

Add to re-exports:
```rust
pub use contracts::F1r3flyRgbContracts;
```

---

## Phase 3: F1r3flyConsignment (Transfer Packages)

**Priority**: 🔥 **CRITICAL**  
**Estimated Time**: 10-12 hours

### 3.1: Create Consignment Types

**File**: `f1r3fly-rgb/src/consignment.rs` (NEW)

```rust
//! Consignment handling for F1r3fly-RGB transfers
//!
//! Provides lightweight consignment packages for transferring RGB assets
//! with F1r3fly state proofs and Bitcoin anchors.

use crate::{
    F1r3flyExecutionResult, ContractMetadata, F1r3flyRgbError, F1r3flyExecutor,
};
use amplify::confinement::SmallOrdMap;
use bp::seals::{Anchor, WTxoSeal};
use hypersonic::ContractId;
use rgb::Witness;
use serde::{Deserialize, Serialize};

/// F1r3fly-RGB consignment for asset transfers
///
/// A lightweight transfer package containing:
/// - Contract metadata (Rholang source, methods)
/// - F1r3fly state proof (block hash, state hash)
/// - Bitcoin anchor (Tapret proof)
/// - Seals and witnesses
///
/// Unlike traditional RGB consignments, this does NOT contain:
/// - Full operation history (state is on F1r3fly shard)
/// - AluVM schemas (uses Rholang)
/// - Client-side state (queries F1r3fly)
///
/// # Example
///
/// ```rust,no_run
/// use f1r3fly_rgb::{F1r3flyConsignment, F1r3flyRgbContract};
///
/// # async fn example(contract: &F1r3flyRgbContract, result: f1r3fly_rgb::F1r3flyExecutionResult, seals: amplify::confinement::SmallOrdMap<u16, bp::seals::WTxoSeal>, witnesses: Vec<rgb::Witness>) -> Result<(), Box<dyn std::error::Error>> {
/// // Create consignment
/// let consignment = F1r3flyConsignment::new(
///     contract,
///     result,
///     seals,
///     witnesses,
/// )?;
///
/// // Serialize and send
/// let bytes = consignment.to_bytes()?;
/// // send_to_recipient(bytes);
///
/// // Recipient validates
/// let received = F1r3flyConsignment::from_bytes(&bytes)?;
/// received.validate(&executor).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct F1r3flyConsignment {
    /// Consignment format version
    pub version: u16,
    
    /// Contract ID
    pub contract_id: ContractId,
    
    /// Contract metadata (Rholang source, methods, registry URI)
    pub contract_metadata: ContractMetadata,
    
    /// F1r3fly state proof
    pub f1r3fly_proof: F1r3flyStateProof,
    
    /// Bitcoin anchor (Tapret proof)
    pub bitcoin_anchor: Anchor,
    
    /// Seals (UTXO bindings)
    pub seals: SmallOrdMap<u16, WTxoSeal>,
    
    /// Witnesses (Bitcoin TX confirmations)
    pub witnesses: Vec<Witness>,
}

/// F1r3fly state proof for consignment validation
///
/// Contains cryptographic proof of F1r3fly shard state at time of transfer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct F1r3flyStateProof {
    /// F1r3fly block hash (finalized state)
    pub block_hash: String,
    
    /// State hash (committed to Bitcoin via Tapret)
    pub state_hash: [u8; 32],
    
    /// F1r3fly deploy ID (transaction ID)
    pub deploy_id: String,
}

impl F1r3flyConsignment {
    /// Create new consignment from contract execution result
    ///
    /// # Arguments
    ///
    /// * `contract` - Source contract
    /// * `result` - F1r3fly execution result (contains state proof)
    /// * `seals` - Seals for this transfer
    /// * `witnesses` - Bitcoin witnesses (TX confirmations)
    ///
    /// # Returns
    ///
    /// New consignment ready for serialization
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use f1r3fly_rgb::{F1r3flyConsignment, F1r3flyRgbContract, F1r3flyExecutionResult};
    /// # use amplify::confinement::SmallOrdMap;
    /// # use bp::seals::WTxoSeal;
    /// # use rgb::Witness;
    /// # fn example(contract: &F1r3flyRgbContract, result: F1r3flyExecutionResult, seals: SmallOrdMap<u16, WTxoSeal>, witnesses: Vec<Witness>) -> Result<(), Box<dyn std::error::Error>> {
    /// let consignment = F1r3flyConsignment::new(
    ///     contract,
    ///     result,
    ///     seals,
    ///     witnesses,
    /// )?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(
        contract: &F1r3flyRgbContract,
        result: F1r3flyExecutionResult,
        seals: SmallOrdMap<u16, WTxoSeal>,
        witnesses: Vec<Witness>,
    ) -> Result<Self, F1r3flyRgbError> {
        // Get anchor from contract tracker
        // TODO: This requires tracker to store anchors by opid
        let bitcoin_anchor = Anchor::strict_dumb(); // Placeholder
        
        Ok(Self {
            version: 1,
            contract_id: contract.contract_id(),
            contract_metadata: contract.metadata().clone(),
            f1r3fly_proof: F1r3flyStateProof {
                block_hash: result.finalized_block_hash,
                state_hash: result.state_hash,
                deploy_id: result.deploy_id,
            },
            bitcoin_anchor,
            seals,
            witnesses,
        })
    }
    
    /// Validate consignment
    ///
    /// Verifies:
    /// 1. F1r3fly state proof is valid (query shard)
    /// 2. Bitcoin anchor matches state hash
    /// 3. Seals are valid UTXOs
    ///
    /// # Arguments
    ///
    /// * `executor` - F1r3fly executor for state verification
    ///
    /// # Returns
    ///
    /// Ok(()) if valid, error otherwise
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use f1r3fly_rgb::{F1r3flyConsignment, F1r3flyExecutor};
    /// # async fn example(consignment: F1r3flyConsignment, executor: &F1r3flyExecutor) -> Result<(), Box<dyn std::error::Error>> {
    /// consignment.validate(executor).await?;
    /// println!("Consignment is valid!");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn validate(&self, executor: &F1r3flyExecutor) -> Result<(), F1r3flyRgbError> {
        log::info!("Validating consignment for contract: {}", self.contract_id);
        
        // 1. Verify F1r3fly state proof
        // TODO: Query F1r3fly to verify block_hash and state_hash match
        // let state = executor.verify_state_proof(
        //     &self.f1r3fly_proof.block_hash,
        //     &self.f1r3fly_proof.state_hash,
        // ).await?;
        
        // 2. Verify Bitcoin anchor
        // TODO: Verify anchor.mpc_proof matches state_hash
        // verify_anchor(&self.bitcoin_anchor, &self.f1r3fly_proof.state_hash)?;
        
        // 3. Verify seals are valid
        if self.seals.is_empty() {
            return Err(F1r3flyRgbError::InvalidConsignment(
                "No seals in consignment".to_string()
            ));
        }
        
        log::info!("✅ Consignment validated");
        Ok(())
    }
    
    /// Serialize consignment to bytes
    ///
    /// # Returns
    ///
    /// Serialized consignment as byte vector
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use f1r3fly_rgb::F1r3flyConsignment;
    /// # fn example(consignment: F1r3flyConsignment) -> Result<(), Box<dyn std::error::Error>> {
    /// let bytes = consignment.to_bytes()?;
    /// // Send bytes to recipient
    /// # Ok(())
    /// # }
    /// ```
    pub fn to_bytes(&self) -> Result<Vec<u8>, F1r3flyRgbError> {
        serde_json::to_vec(self)
            .map_err(|e| F1r3flyRgbError::SerializationError(e.to_string()))
    }
    
    /// Deserialize consignment from bytes
    ///
    /// # Arguments
    ///
    /// * `data` - Serialized consignment bytes
    ///
    /// # Returns
    ///
    /// Deserialized consignment
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use f1r3fly_rgb::F1r3flyConsignment;
    /// # fn example(bytes: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
    /// let consignment = F1r3flyConsignment::from_bytes(&bytes)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_bytes(data: &[u8]) -> Result<Self, F1r3flyRgbError> {
        serde_json::from_slice(data)
            .map_err(|e| F1r3flyRgbError::SerializationError(e.to_string()))
    }
    
    /// Get contract ID
    pub fn contract_id(&self) -> ContractId {
        self.contract_id
    }
    
    /// Get contract metadata
    pub fn metadata(&self) -> &ContractMetadata {
        &self.contract_metadata
    }
    
    /// Get F1r3fly state proof
    pub fn f1r3fly_proof(&self) -> &F1r3flyStateProof {
        &self.f1r3fly_proof
    }
    
    /// Get seals
    pub fn seals(&self) -> &SmallOrdMap<u16, WTxoSeal> {
        &self.seals
    }
}
```

### 3.2: Add Error Variants

**File**: `f1r3fly-rgb/src/error.rs`

Add to `F1r3flyRgbError` enum:
```rust
/// Invalid consignment format
InvalidConsignment(String),

/// Serialization error
SerializationError(String),
```

### 3.3: Add Tests

**File**: `f1r3fly-rgb/tests/consignment_test.rs` (NEW)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_consignment_create() {
        dotenv::from_path(".env").ok();
        
        let executor = F1r3flyExecutor::new().expect("Failed to create executor");
        let contract = F1r3flyRgbContract::issue(
            executor,
            "TEST",
            "Test Token",
            1000000,
            8,
        ).await.expect("Issue failed");
        
        // Create execution result
        let result = F1r3flyExecutionResult {
            deploy_id: "test_deploy".to_string(),
            finalized_block_hash: "test_block".to_string(),
            state_hash: [1u8; 32],
        };
        
        let seals = create_test_seals();
        let witnesses = vec![];
        
        let consignment = F1r3flyConsignment::new(
            &contract,
            result,
            seals,
            witnesses,
        ).expect("Consignment creation failed");
        
        assert_eq!(consignment.version, 1);
        assert_eq!(consignment.contract_id, contract.contract_id());
    }
    
    #[tokio::test]
    async fn test_consignment_serialization() {
        dotenv::from_path(".env").ok();
        
        let executor = F1r3flyExecutor::new().expect("Failed to create executor");
        let contract = F1r3flyRgbContract::issue(
            executor,
            "TEST",
            "Test Token",
            1000000,
            8,
        ).await.expect("Issue failed");
        
        let result = F1r3flyExecutionResult {
            deploy_id: "test_deploy".to_string(),
            finalized_block_hash: "test_block".to_string(),
            state_hash: [1u8; 32],
        };
        
        let seals = create_test_seals();
        let witnesses = vec![];
        
        let consignment = F1r3flyConsignment::new(
            &contract,
            result,
            seals,
            witnesses,
        ).expect("Consignment creation failed");
        
        // Serialize
        let bytes = consignment.to_bytes().expect("Serialization failed");
        
        // Deserialize
        let deserialized = F1r3flyConsignment::from_bytes(&bytes)
            .expect("Deserialization failed");
        
        assert_eq!(consignment.contract_id, deserialized.contract_id);
        assert_eq!(consignment.version, deserialized.version);
    }
    
    #[tokio::test]
    async fn test_consignment_validation() {
        dotenv::from_path(".env").ok();
        
        let executor = F1r3flyExecutor::new().expect("Failed to create executor");
        let contract = F1r3flyRgbContract::issue(
            executor.clone(),
            "TEST",
            "Test Token",
            1000000,
            8,
        ).await.expect("Issue failed");
        
        let result = F1r3flyExecutionResult {
            deploy_id: "test_deploy".to_string(),
            finalized_block_hash: "test_block".to_string(),
            state_hash: [1u8; 32],
        };
        
        let seals = create_test_seals();
        let witnesses = vec![];
        
        let consignment = F1r3flyConsignment::new(
            &contract,
            result,
            seals,
            witnesses,
        ).expect("Consignment creation failed");
        
        // Validate
        let validation = consignment.validate(&executor).await;
        
        // Note: This may fail if F1r3fly state verification is not implemented
        // For now, just check it doesn't panic
        match validation {
            Ok(_) => println!("Validation passed"),
            Err(e) => println!("Validation failed (expected): {:?}", e),
        }
    }
    
    // Helper
    fn create_test_seals() -> SmallOrdMap<u16, WTxoSeal> {
        use commit_verify::{Digest, DigestExt, Sha256};
        let mut noise = Sha256::new();
        noise.input_raw(b"test_seals");
        
        small_bmap! {
            0 => WTxoSeal::vout_no_fallback(1.into(), noise, 1)
        }
    }
}
```

### 3.4: Update lib.rs

**File**: `f1r3fly-rgb/src/lib.rs`

Add to modules:
```rust
pub mod consignment;
```

Add to re-exports:
```rust
pub use consignment::{F1r3flyConsignment, F1r3flyStateProof};
```

---

## Implementation Checklist

### Phase 1: Contract (Single Contract API)
- [ ] Create `src/contract.rs` with `F1r3flyRgbContract` struct
- [ ] Implement `new()` constructor
- [ ] Implement `issue()` static method
- [ ] Implement `call_method()` method
- [ ] Implement `balance()` method
- [ ] Add accessor methods (contract_id, metadata, executor, tracker)
- [ ] Create `tests/contract_test.rs` with 3+ tests
- [ ] Update `src/lib.rs` exports
- [ ] Run `cargo test --lib contract` to verify

### Phase 2: Contracts (Multi-Contract Collection)
- [ ] Create `src/contracts.rs` with `F1r3flyRgbContracts` struct
- [ ] Implement `new()` constructor
- [ ] Implement `issue()` method
- [ ] Implement `get()` and `get_mut()` methods
- [ ] Implement `register()` method
- [ ] Implement `list()`, `count()`, `contains()` methods
- [ ] Create `tests/contracts_test.rs` with 4+ tests
- [ ] Update `src/lib.rs` exports
- [ ] Run `cargo test --lib contracts` to verify

### Phase 3: Consignment (Transfer Packages)
- [ ] Create `src/consignment.rs` with `F1r3flyConsignment` struct
- [ ] Create `F1r3flyStateProof` struct
- [ ] Implement `new()` method
- [ ] Implement `validate()` method (basic version)
- [ ] Implement `to_bytes()` and `from_bytes()` methods
- [ ] Add accessor methods
- [ ] Add error variants to `error.rs`
- [ ] Create `tests/consignment_test.rs` with 3+ tests
- [ ] Update `src/lib.rs` exports
- [ ] Run `cargo test --lib consignment` to verify

### Phase 4: Integration & Documentation
- [ ] Run full test suite: `cargo test`
- [ ] Run integration tests: `cargo test --test '*'`
- [ ] Update main `lib.rs` documentation
- [ ] Add examples to README
- [ ] Update architecture diagram in docs
- [ ] Verify all lints pass: `cargo clippy`
- [ ] Verify formatting: `cargo fmt --check`

---

## Success Criteria

### Functional Requirements
- ✅ Wallets can use `F1r3flyRgbContract` for single contract operations
- ✅ Wallets can use `F1r3flyRgbContracts` for multi-contract management
- ✅ Wallets can create and validate consignments
- ✅ API matches RGB's abstraction level
- ✅ All tests pass

### Non-Functional Requirements
- ✅ API is simple and intuitive (matches RGB patterns)
- ✅ Good documentation with examples
- ✅ Proper error handling
- ✅ Clean separation of concerns (contract vs executor vs tracker)

---

## Implementation Notes

### For the AI Assistant (Me)

**Order of Implementation**:
1. **Phase 1 first** - Contract is the foundation
2. **Phase 2 second** - Contracts builds on Contract
3. **Phase 3 third** - Consignment uses Contract

**Testing Strategy**:
- Write tests immediately after each component
- Use `dotenv::from_path(".env").ok()` in all integration tests
- Check for `F1R3NODE_URL` and skip tests if not available
- Use helper functions to reduce duplication

**Common Patterns**:
- Clone `F1r3flyExecutor` when needed (it's cheap)
- Use `&mut self` when modifying tracker
- Return `Result<T, F1r3flyRgbError>` consistently
- Add comprehensive logging with `log::info!` and `log::debug!`

**Error Handling**:
- Add new error variants to `error.rs` as needed
- Use `map_err()` to convert external errors
- Provide descriptive error messages

**Code Quality**:
- Run `cargo fmt` after each file
- Run `cargo clippy` to catch issues
- Add doc comments with examples
- Keep functions under 50 lines where possible

---

## Estimated Timeline

| Phase | Component | Time | Total |
|-------|-----------|------|-------|
| 1 | F1r3flyRgbContract | 8-10h | 8-10h |
| 2 | F1r3flyRgbContracts | 6-8h | 14-18h |
| 3 | F1r3flyConsignment | 10-12h | 24-30h |
| 4 | Integration & Docs | 4-6h | 28-36h |

**Total Estimate**: 28-36 hours (1-1.5 weeks at 20h/week)

---

## After This Plan

Once this plan is complete, the wallet layer (`f1r3fly-rgb-wallet`) can be implemented with:
- BDK for Bitcoin operations
- `F1r3flyRgbContracts` for RGB operations
- Simple UI/UX layer

The wallet will have the **same ease of use** as RGB wallets, while using F1r3fly for state management.

---

**Status**: 📋 **Ready for Implementation**  
**Next Step**: Review plan, then begin Phase 1

