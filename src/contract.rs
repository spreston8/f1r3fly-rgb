//! High-level single contract operations
//!
//! This module provides a unified API for interacting with a single F1r3fly-RGB
//! contract, coordinating the executor, tracker, and metadata.

use crate::{
    BitcoinAnchorTracker, ContractMetadata, F1r3flyExecutionResult, F1r3flyExecutor,
    F1r3flyRgbError, RholangContractLibrary,
};
use amplify::confinement::SmallOrdMap;
use bp::seals::{TxoSeal, WTxoSeal};
use hypersonic::ContractId;
use rgb::Pile;
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
/// use strict_types::StrictVal;
/// use amplify::confinement::SmallOrdMap;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Issue new token
/// let executor = F1r3flyExecutor::new()?;
/// let mut contract = F1r3flyRgbContract::issue(
///     executor,
///     "BTC",
///     "Bitcoin",
///     21000000,
///     8,
/// ).await?;
///
/// // Call method
/// # let seals = SmallOrdMap::new();
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
/// # use strict_types::StrictDumb;
/// # let seal = bp::seals::TxoSeal::strict_dumb();
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
    /// * `contract_id` - Contract ID
    /// * `executor` - F1r3fly executor instance
    /// * `metadata` - Contract metadata (from previous deployment)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use f1r3fly_rgb::{F1r3flyRgbContract, F1r3flyExecutor, ContractMetadata};
    /// # use hypersonic::ContractId;
    /// # fn example(contract_id: ContractId, executor: F1r3flyExecutor, metadata: ContractMetadata) -> Result<(), Box<dyn std::error::Error>> {
    /// let contract = F1r3flyRgbContract::new(contract_id, executor, metadata)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(
        contract_id: ContractId,
        executor: F1r3flyExecutor,
        metadata: ContractMetadata,
    ) -> Result<Self, F1r3flyRgbError> {
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
        let contract_id = executor
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
                ],
            )
            .await?;

        // Get metadata from executor's contract registry
        let metadata = executor
            .get_contract_metadata(contract_id)
            .ok_or_else(|| {
                F1r3flyRgbError::ContractNotFound(format!(
                    "Contract {} not found after deployment",
                    contract_id
                ))
            })?
            .clone();

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
    /// # use amplify::confinement::SmallOrdMap;
    /// # async fn example(mut contract: F1r3flyRgbContract, seals: SmallOrdMap<u16, bp::seals::WTxoSeal>) -> Result<(), Box<dyn std::error::Error>> {
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
        log::debug!(
            "Calling method: {} on contract {}",
            method,
            self.contract_id
        );

        // Execute on F1r3fly
        let result = self
            .executor
            .call_method(self.contract_id, method, params)
            .await?;

        // Track seals in Bitcoin anchor tracker
        // Convert hypersonic::Opid to rgb::Opid using byte array
        let opid_bytes: [u8; 32] = result.state_hash;
        let opid = rgb::Opid::from(opid_bytes);
        self.tracker.add_seals(opid, seals);

        log::debug!(
            "Method call complete, state_hash: {}",
            hex::encode(result.state_hash)
        );

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
        // Serialize seal using its primary outpoint (txid:vout)
        // This is the standard Bitcoin UTXO identifier format
        let seal_id = Self::serialize_seal(seal);

        log::info!("📊 CONTRACT: balance() called");
        log::info!("  Input seal: {:?}", seal);
        log::info!("  Serialized seal_id: {}", seal_id);
        log::info!("  StrictVal type: {:?}", StrictVal::from(seal_id.clone()));

        let result = self
            .executor
            .query_state(
                self.contract_id,
                "balanceOf",
                &[("seal", StrictVal::from(seal_id.clone()))],
            )
            .await?;

        log::info!("  Query result: {:?}", result);

        // Parse balance from JSON result
        // query_state returns serde_json::Value
        result
            .as_u64()
            .or_else(|| {
                result
                    .as_i64()
                    .and_then(|n| if n >= 0 { Some(n as u64) } else { None })
            })
            .ok_or_else(|| {
                F1r3flyRgbError::InvalidStateFormat(format!(
                    "Expected unsigned integer for balance, got: {:?}",
                    result
                ))
            })
    }

    /// Serialize a TxoSeal to a stable string identifier
    ///
    /// Uses the primary outpoint (txid:vout) as the seal identifier.
    /// This format is:
    /// - Deterministic: Same seal always produces the same ID
    /// - Standard: Matches Bitcoin UTXO format
    /// - Human-readable: Easy to debug and log
    ///
    /// # Format
    ///
    /// `<txid_hex>:<vout>`
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use f1r3fly_rgb::F1r3flyRgbContract;
    /// # use bp::seals::TxoSeal;
    /// # fn example(seal: TxoSeal) {
    /// let seal_id = F1r3flyRgbContract::serialize_seal(&seal);
    /// // seal_id = "abc123...def:0"
    /// # }
    /// ```
    pub fn serialize_seal(seal: &TxoSeal) -> String {
        use amplify::ByteArray;

        // Extract primary outpoint (txid + vout)
        let outpoint = seal.primary;

        // Serialize txid as hex (Bitcoin standard format)
        let txid_hex = hex::encode(outpoint.txid.to_byte_array());

        // Get vout as u32
        let vout = outpoint.vout.into_u32();

        // Format as "txid:vout" (standard Bitcoin outpoint format)
        format!("{}:{}", txid_hex, vout)
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
