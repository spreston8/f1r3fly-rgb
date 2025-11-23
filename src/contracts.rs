//! Multi-contract collection management
//!
//! Provides a unified interface for managing multiple F1r3fly-RGB contracts,
//! similar to RGB's `Contracts` type.

use crate::{F1r3flyExecutor, F1r3flyRgbContract, F1r3flyRgbError, RholangContractLibrary};
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

        // Deploy contract using the shared executor (preserves derivation_index)
        let contract_id = self
            .executor
            .deploy_contract(
                RholangContractLibrary::rho20_contract(),
                ticker,
                name,
                supply,
                precision,
                vec![
                    "issue".to_string(),
                    "transfer".to_string(),
                    "balanceOf".to_string(),
                    "getMetadata".to_string(),
                    "claim".to_string(),
                    "ownerOf".to_string(),
                ],
            )
            .await?;

        // Get metadata from executor
        let metadata = self
            .executor
            .get_contract_metadata(contract_id)
            .ok_or_else(|| {
                F1r3flyRgbError::ContractNotFound(format!(
                    "Contract {} not found after deployment",
                    contract_id
                ))
            })?
            .clone();

        // Create contract instance with cloned executor for independent operations
        let contract = F1r3flyRgbContract::new(contract_id, self.executor.clone(), metadata)?;

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
    ///     println!("Found contract: {}", contract.metadata().registry_uri);
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
    /// # use hypersonic::ContractId;
    /// # fn example(mut contracts: F1r3flyRgbContracts, contract_id: ContractId, executor: F1r3flyExecutor, metadata: ContractMetadata) -> Result<(), Box<dyn std::error::Error>> {
    /// let contract = F1r3flyRgbContract::new(contract_id, executor, metadata)?;
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

    /// Register a loaded contract
    ///
    /// Used during state restoration to add contracts back to the collection.
    ///
    /// # Arguments
    ///
    /// * `id` - Contract ID
    /// * `contract` - Contract instance
    pub fn register_loaded_contract(&mut self, id: ContractId, contract: F1r3flyRgbContract) {
        self.contracts.insert(id, contract);
    }

    /// Get reference to the shared executor
    ///
    /// Returns a reference to the F1r3flyExecutor used by all contracts.
    /// Useful for querying executor state like derivation index or contracts metadata.
    pub fn executor(&self) -> &F1r3flyExecutor {
        &self.executor
    }

    /// Get mutable reference to the shared executor
    ///
    /// Returns a mutable reference to the F1r3flyExecutor.
    /// Useful for restoring state like setting derivation index when loading from disk.
    pub fn executor_mut(&mut self) -> &mut F1r3flyExecutor {
        &mut self.executor
    }
}
