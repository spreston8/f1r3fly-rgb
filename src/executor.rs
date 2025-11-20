// F1r3fly Executor for Light Integration
//
// This module provides a complete, production-ready executor for F1r3fly operations.
// It handles:
// - Direct Rholang execution
// - Persistent contract deployment with insertSigned (Pattern B)
// - Method calls via registry lookup
// - State queries via HTTP API
// - Contract registry tracking

use amplify::confinement::SmallVec;
use amplify::ByteArray;
use chrono::Utc;
use hypersonic::{ContractId, Opid};
use node_cli::connection_manager::F1r3flyConnectionManager;
use node_cli::registry::{generate_insert_signed_signature, public_key_to_uri};
use node_cli::rholang_helpers::convert_rholang_to_json;
use secp256k1::{PublicKey, Secp256k1, SecretKey};
use serde_json::{json, Value};
use std::collections::HashMap;
use strict_types::StrictVal;

use crate::F1r3flyRgbError;

/// Result of a F1r3fly execution (deploy or method call)
///
/// Tracks the outcome of Rholang execution on F1r3node for audit and debugging.
#[derive(Debug, Clone)]
pub struct F1r3flyExecutionResult {
    /// RGB operation ID this execution corresponds to
    pub opid: Opid,
    /// F1r3node deploy ID (transaction identifier)
    pub deploy_id: SmallVec<u8>,
    /// Finalized block hash on F1r3node blockchain
    pub finalized_block_hash: SmallVec<u8>,
    /// Rholang source code that was executed
    pub rholang_source: SmallVec<u8>,
    /// State hash for Bitcoin commitment (32 bytes)
    ///
    /// This hash represents the F1r3fly state at finalization and can be
    /// embedded in Bitcoin transactions via Tapret commitments.
    pub state_hash: [u8; 32],
}

impl F1r3flyExecutionResult {
    /// Get deploy ID as string (assumes UTF-8)
    pub fn deploy_id_string(&self) -> Result<String, std::string::FromUtf8Error> {
        String::from_utf8(self.deploy_id.to_vec())
    }

    /// Get finalized block hash as string (assumes UTF-8)
    pub fn block_hash_string(&self) -> Result<String, std::string::FromUtf8Error> {
        String::from_utf8(self.finalized_block_hash.to_vec())
    }

    /// Get Rholang source as string (assumes UTF-8)
    pub fn rholang_source_string(&self) -> Result<String, std::string::FromUtf8Error> {
        String::from_utf8(self.rholang_source.to_vec())
    }

    /// Get the state commitment for Bitcoin anchoring
    ///
    /// This 32-byte hash represents the F1r3fly state at the time of finalization
    /// and can be embedded in Bitcoin transactions via Tapret commitments.
    ///
    /// # Returns
    /// A commitment suitable for RGB Tapret anchoring
    pub fn state_commitment(&self) -> commit_verify::mpc::Commitment {
        commit_verify::mpc::Commitment::from(self.state_hash)
    }
}

/// Metadata for a deployed contract
///
/// Tracks essential information about contracts deployed with insertSigned.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContractMetadata {
    /// Persistent registry URI (rho:id:...)
    /// This is the globally-accessible address for the contract.
    pub registry_uri: String,

    /// Available methods on this contract
    /// Used for validation before method calls.
    pub methods: Vec<String>,

    /// Original Rholang source code
    /// Stored for verification, debugging, and consignment inclusion.
    pub rholang_source: String,
}

/// F1r3fly Executor - Production implementation
///
/// This executor provides the complete API for F1r3fly operations:
/// - Low-level: Direct Rholang execution
/// - High-level: Contract deployment, method calls, state queries
/// - Built-in: Contract registry, insertSigned wrapper, HTTP queries
///
/// All RGB contracts use this executor for state management.
#[derive(Clone)]
pub struct F1r3flyExecutor {
    /// Connection to F1r3node
    connection: F1r3flyConnectionManager,

    /// Registry of deployed contracts (local cache)
    contracts: HashMap<ContractId, ContractMetadata>,

    /// HTTP client for state queries (reqwest::Client is internally Arc-based)
    http_client: reqwest::Client,

    /// Hash-based derivation index for generating unique contract keys
    /// Each contract deployment increments this counter (if auto_derive is true)
    derivation_index: u32,

    /// Enable automatic key derivation for multi-contract support
    ///
    /// - `true` (default): Each deploy_contract() increments derivation_index,
    ///   producing unique child keys and unique registry URIs. Use for deploying
    ///   multiple independent contracts (BTC, ETH, USDT, etc.)
    ///
    /// - `false`: All deployments use derivation_index = 0, producing the same
    ///   child key and same URI. F1r3node's insertSigned will upgrade the contract
    ///   at that URI if the version is higher. Use for contract upgrades/bug fixes.
    auto_derive: bool,
}

impl F1r3flyExecutor {
    /// Create a new F1r3fly executor from environment configuration
    ///
    /// Requires environment variables:
    /// - FIREFLY_HOST
    /// - FIREFLY_GRPC_PORT
    /// - FIREFLY_HTTP_PORT
    /// - FIREFLY_PRIVATE_KEY
    ///
    /// Defaults to `auto_derive = true` for multi-contract support.
    /// Use `set_auto_derive(false)` for contract upgrades.
    pub fn new() -> Result<Self, F1r3flyRgbError> {
        let connection = F1r3flyConnectionManager::from_env().map_err(|e| {
            F1r3flyRgbError::ConnectionFailed(format!("Failed to create connection: {}", e))
        })?;

        Ok(Self {
            connection,
            contracts: HashMap::new(),
            http_client: reqwest::Client::new(),
            derivation_index: 0,
            auto_derive: true, // Default: enable multi-contract support
        })
    }

    /// Create executor with explicit connection
    ///
    /// This allows wallets and other applications to provide their own connection
    /// configuration instead of relying on environment variables.
    ///
    /// # Arguments
    ///
    /// * `connection` - Pre-configured F1r3flyConnectionManager
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use f1r3fly_rgb::F1r3flyExecutor;
    /// # use node_cli::connection_manager::{F1r3flyConnectionManager, ConnectionConfig};
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = ConnectionConfig::new(
    ///     "localhost".to_string(),
    ///     40401,
    ///     40402,
    ///     "your_private_key_hex".to_string(),
    /// );
    /// let connection = F1r3flyConnectionManager::new(config);
    /// let executor = F1r3flyExecutor::with_connection(connection);
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_connection(connection: F1r3flyConnectionManager) -> Self {
        Self {
            connection,
            contracts: HashMap::new(),
            http_client: reqwest::Client::new(),
            derivation_index: 0,
            auto_derive: true, // Default: enable multi-contract support
        }
    }

    /// Enable or disable automatic key derivation for multi-contract support
    ///
    /// # Arguments
    ///
    /// * `enabled` - If `true`, each `deploy_contract()` call increments the
    ///   derivation index, producing unique contracts. If `false`, all deployments
    ///   use the same derived key, allowing contract upgrades via insertSigned's
    ///   version mechanism.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use f1r3fly_rgb::{F1r3flyExecutor, RholangContractLibrary};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut executor = F1r3flyExecutor::new()?;
    ///
    /// // Deploy multiple independent contracts (default behavior)
    /// let btc_id = executor.deploy_contract(
    ///     RholangContractLibrary::rho20_contract(),
    ///     "BTC", "Bitcoin", 21000000, 8,
    ///     vec!["issue".to_string(), "transfer".to_string()]
    /// ).await?;
    /// let eth_id = executor.deploy_contract(
    ///     RholangContractLibrary::rho20_contract(),
    ///     "ETH", "Ethereum", 100000000, 18,
    ///     vec!["issue".to_string(), "transfer".to_string()]
    /// ).await?;
    /// // btc_id != eth_id
    ///
    /// // Deploy contract upgrades to same URI
    /// executor.set_auto_derive(false);
    /// let v1_id = executor.deploy_contract(
    ///     RholangContractLibrary::rho20_contract(),
    ///     "V1", "Version 1", 1000000, 8,
    ///     vec!["issue".to_string(), "transfer".to_string()]
    /// ).await?;
    /// let v2_id = executor.deploy_contract(
    ///     RholangContractLibrary::rho20_contract(),
    ///     "V2", "Version 2", 1000000, 8,
    ///     vec!["issue".to_string(), "transfer".to_string()]
    /// ).await?;
    /// // v1_id == v2_id (upgraded at same URI)
    /// # Ok(())
    /// # }
    /// ```
    pub fn set_auto_derive(&mut self, enabled: bool) {
        self.auto_derive = enabled;
    }

    /// Get current derivation index
    ///
    /// Returns the current index used for BIP32-style key derivation.
    /// This value increments with each contract deployment when auto_derive is enabled.
    pub fn derivation_index(&self) -> u32 {
        self.derivation_index
    }

    /// Set derivation index
    ///
    /// Used when restoring executor state from persistence.
    /// Sets the derivation index to continue from a previous session.
    ///
    /// # Arguments
    ///
    /// * `index` - The derivation index to set
    pub fn set_derivation_index(&mut self, index: u32) {
        self.derivation_index = index;
    }

    /// Get reference to contracts metadata
    ///
    /// Returns the HashMap of deployed contracts, mapping ContractId to ContractMetadata.
    /// Useful for persisting contract state or querying available contracts.
    pub fn contracts_metadata(&self) -> &HashMap<ContractId, ContractMetadata> {
        &self.contracts
    }

    /// Check if a F1r3fly block is finalized
    ///
    /// Used for consignment validation to verify F1r3fly state is immutable.
    /// A finalized block indicates the state is canonical and won't change.
    ///
    /// # Arguments
    ///
    /// * `block_hash` - The block hash to check
    ///
    /// # Returns
    ///
    /// `true` if the block is finalized, `false` otherwise
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use f1r3fly_rgb::F1r3flyExecutor;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let executor = F1r3flyExecutor::new()?;
    /// let block_hash = "abc123...";
    ///
    /// if executor.is_block_finalized(block_hash).await? {
    ///     println!("Block is finalized and immutable");
    /// } else {
    ///     println!("Block not yet finalized");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn is_block_finalized(&self, block_hash: &str) -> Result<bool, F1r3flyRgbError> {
        // Use max_attempts=1 for immediate check (no retry waiting)
        // wait_for_finalization returns Ok(()) if finalized, Err if not
        match self.connection.wait_for_finalization(block_hash, 1).await {
            Ok(()) => Ok(true),
            Err(_) => Ok(false), // Not finalized (not an error condition)
        }
    }

    /// Low-level: Execute Rholang code directly
    ///
    /// This is the core execution method. Use this for:
    /// - Custom Rholang code
    /// - Manual control over deployment
    /// - Operations outside the standard contract workflow
    ///
    /// For standard contract operations, use deploy_contract() and call_method().
    pub async fn execute(
        &self,
        rholang_source: String,
        opid: Opid,
    ) -> Result<F1r3flyExecutionResult, F1r3flyRgbError> {
        log::info!("🔥 F1r3flyExecutor::execute() - opid: {}", opid);
        log::debug!("   Rholang source: {} bytes", rholang_source.len());

        // Deploy and wait for finalization
        let (deploy_id, block_hash) = self
            .connection
            .deploy_and_wait(&rholang_source, 60, 20)
            .await
            .map_err(|e| {
                log::error!("   ❌ Deployment failed: {}", e);
                F1r3flyRgbError::DeploymentFailed {
                    deploy_id: "unknown".to_string(),
                    reason: format!("Deployment failed: {}", e),
                }
            })?;

        log::info!("   ✅ Deployed! ID: {}, Block: {}", deploy_id, block_hash);

        // Compute state hash from finalized block hash and deploy ID
        // This creates a deterministic 32-byte commitment from F1r3node's finalization
        let state_hash = compute_state_hash(&block_hash, &deploy_id);
        log::debug!("   🔐 State hash: {}", hex::encode(state_hash));

        Ok(F1r3flyExecutionResult {
            opid,
            deploy_id: SmallVec::try_from(deploy_id.as_bytes().to_vec())
                .map_err(|_| F1r3flyRgbError::InvalidResponse("Deploy ID too large".to_string()))?,
            finalized_block_hash: SmallVec::try_from(block_hash.as_bytes().to_vec()).map_err(
                |_| F1r3flyRgbError::InvalidResponse("Block hash too large".to_string()),
            )?,
            rholang_source: SmallVec::try_from(rholang_source.as_bytes().to_vec()).map_err(
                |_| F1r3flyRgbError::InvalidResponse("Rholang source too large".to_string()),
            )?,
            state_hash,
        })
    }

    /// High-level: Deploy a persistent contract with insertSigned (Pattern B)
    ///
    /// This method:
    /// 1. Substitutes template variables in the Rholang contract
    /// 2. Deploys to F1r3node using the connection's signing key
    /// 3. Computes the deterministic registry URI
    /// 4. Caches the contract metadata for future method calls
    ///
    /// # Arguments
    /// - `rholang_template`: The contract template code with {{PLACEHOLDERS}}
    /// - `ticker`: Token ticker symbol
    /// - `name`: Token name
    /// - `total_supply`: Initial total supply
    /// - `precision`: Decimal precision
    /// - `methods`: List of method names available on this contract
    ///
    /// # Returns
    /// - `ContractId`: Unique identifier for this contract (derived from registry URI)
    ///
    /// # Pattern B: Persistent Contracts
    /// Uses the connection's signing key (from FIREFLY_PRIVATE_KEY) for BOTH:
    /// - insertSigned signature generation
    /// - Deploy signing
    /// This ensures signature verification succeeds.
    ///
    /// See: docs/bugs/f1r3fly-integration-challenges.md Section 1
    pub async fn deploy_contract(
        &mut self,
        rholang_template: &str,
        ticker: &str,
        name: &str,
        total_supply: u64,
        precision: u8,
        methods: Vec<String>,
    ) -> Result<ContractId, F1r3flyRgbError> {
        log::info!("🚀 Deploying contract with insertSigned + BIP32 derivation");
        log::debug!("   Token: {} ({})", name, ticker);
        log::debug!("   Methods: {:?}", methods);
        log::debug!("   Derivation index: {}", self.derivation_index);

        // Get the master signing key from connection (used for gRPC deployment + phlo payment)
        let master_key_hex = &self.connection.config().signing_key;

        // Derive child key for this contract's URI
        // Use current derivation_index (0 if auto_derive is false, incremented if true)
        let child_key = derive_child_key_from_master(&master_key_hex, self.derivation_index)?;

        // Increment derivation index for next contract (only if auto_derive is enabled)
        if self.auto_derive {
            self.derivation_index += 1;
            log::debug!(
                "   🔑 Auto-derive enabled: incremented index to {}",
                self.derivation_index
            );
        } else {
            log::debug!(
                "   🔑 Auto-derive disabled: keeping index at {}",
                self.derivation_index
            );
        }

        // 1. Substitute template variables using:
        //    - Master key: Signs gRPC deployment (pays phlo)
        //    - Child key: Generates unique registry URI
        let (rholang_code, timestamp_millis) = substitute_template_variables(
            &rholang_template,
            ticker,
            name,
            total_supply,
            precision,
            &master_key_hex,
            &child_key,
        )?;
        log::debug!(
            "   📜 Generated contract (first 800 chars): {}",
            &rholang_code.chars().take(800).collect::<String>()
        );
        log::debug!(
            "   ⏱️  Deploy timestamp: {} (matches signature timestamp)",
            timestamp_millis
        );

        // 2. Deploy with the EXACT timestamp used in the signature
        // Master key signs the gRPC deployment and pays phlo from its REV vault
        let deploy_id = self
            .connection
            .deploy_with_timestamp(&rholang_code, timestamp_millis)
            .await
            .map_err(|e| {
                log::error!("   ❌ Deployment failed: {}", e);
                F1r3flyRgbError::DeploymentFailed {
                    deploy_id: "unknown".to_string(),
                    reason: format!("Deployment failed: {}", e),
                }
            })?;

        // Wait for deploy to be included in a block
        let block_hash = self
            .connection
            .wait_for_deploy(&deploy_id, 60)
            .await
            .map_err(|e| {
                log::error!("   ❌ Deploy not included in block: {}", e);
                F1r3flyRgbError::DeploymentFailed {
                    deploy_id: deploy_id.clone(),
                    reason: format!("Deploy not included: {}", e),
                }
            })?;

        // Wait for block finalization
        self.connection
            .wait_for_finalization(&block_hash, 20)
            .await
            .map_err(|e| {
                log::error!("   ❌ Block not finalized: {}", e);
                F1r3flyRgbError::DeploymentFailed {
                    deploy_id: deploy_id.clone(),
                    reason: format!("Block not finalized: {}", e),
                }
            })?;

        log::info!("   ✅ Deployed! ID: {}, Block: {}", deploy_id, block_hash);

        // 3. Compute deterministic registry URI from the child key
        let registry_uri = compute_registry_uri_from_child_key(&child_key)?;
        // Derive ContractId from registry URI using Blake2b-256 hash
        // This ensures a deterministic 32-byte ID from the variable-length URI
        let contract_id = derive_contract_id_from_uri(&registry_uri);

        log::info!("   ✅ Contract deployed with URI: {}", registry_uri);
        log::debug!("   ContractId: {:?}", contract_id);

        // 4. Cache metadata for future method calls
        self.contracts.insert(
            contract_id,
            ContractMetadata {
                registry_uri,
                methods,
                rholang_source: rholang_code,
            },
        );

        Ok(contract_id)
    }

    /// High-level: Call a method on a deployed contract via registry lookup
    ///
    /// This method:
    /// 1. Looks up the contract metadata from the local cache
    /// 2. Validates the method exists
    /// 3. Builds registry lookup Rholang
    /// 4. Executes the method call
    ///
    /// # Arguments
    /// - `contract_id`: The contract to call (from deploy_contract)
    /// - `method`: Method name (must be in contract's methods list)
    /// - `params`: Method parameters as (name, value) pairs
    ///
    /// # Pattern B: Method Calls
    /// Uses rho:registry:lookup to find the persistent contract.
    pub async fn call_method(
        &mut self,
        contract_id: ContractId,
        method: &str,
        params: &[(&str, StrictVal)],
    ) -> Result<F1r3flyExecutionResult, F1r3flyRgbError> {
        log::info!(
            "📞 Calling method '{}' on contract {:?}",
            method,
            contract_id
        );

        // Look up contract metadata
        let metadata = self.contracts.get(&contract_id).ok_or_else(|| {
            log::error!("   ❌ Contract not found in registry");
            F1r3flyRgbError::ContractNotFound(format!("Contract {} not found", contract_id))
        })?;

        // Validate method exists
        if !metadata.methods.contains(&method.to_string()) {
            log::error!("   ❌ Invalid method: {}", method);
            log::debug!("   Available methods: {:?}", metadata.methods);
            return Err(F1r3flyRgbError::InvalidMethod(method.to_string()));
        }

        log::debug!("   Registry URI: {}", metadata.registry_uri);
        log::debug!("   Parameters: {} params", params.len());

        // Serialize parameters
        log::info!("   📝 EXECUTOR: call_method - Raw params: {:?}", params);
        let serialized_params = serialize_params(params)?;
        log::info!(
            "   📝 EXECUTOR: call_method - Serialized params: '{}'",
            serialized_params
        );

        // Build parameter list with proper comma handling
        let param_list = if serialized_params.is_empty() {
            String::from("*ret")
        } else {
            format!("{}, *ret", serialized_params)
        };

        // Build registry lookup + method call
        // Registry stores (version, bundle+{{*Rho20Token}}) as a simple 2-tuple
        // To use a bundle, we send on @{{bundle}} which unbundles to the original name
        let call_rholang = format!(
            r#"new rl(`rho:registry:lookup`), contractCh, ret in {{
  rl!(`{}`, *contractCh) |
  for(@(_, contractBundle) <- contractCh) {{
    @{{contractBundle}}!("{}", {})
  }}
}}"#,
            metadata.registry_uri, method, param_list
        );

        log::debug!("   Generated Rholang:\n{}", call_rholang);

        // Generate deterministic opid from operation inputs
        // Hash: contract_id + method + params to create unique operation ID
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(contract_id.as_slice());
        hasher.update(method.as_bytes());
        hasher.update(param_list.as_bytes());
        let opid_bytes: [u8; 32] = hasher.finalize().into();
        let opid = Opid::from(opid_bytes);

        log::debug!("   Generated opid: {}", opid);

        // Execute
        self.execute(call_rholang, opid).await
    }

    /// High-level: Query contract state via HTTP API
    ///
    /// This method:
    /// 1. Looks up the contract metadata
    /// 2. Builds a query Rholang call
    /// 3. Executes via HTTP API (not gRPC - for full JSON response)
    /// 4. Parses the Rholang-typed JSON response
    ///
    /// # Arguments
    /// - `contract_id`: The contract to query
    /// - `query_method`: Method name for the query (e.g., "balanceOf", "totalSupply")
    /// - `params`: Query parameters
    ///
    /// # HTTP vs gRPC
    /// We use HTTP here because gRPC returns simplified strings like
    /// "Complex expression with 2 fields" which can't be parsed.
    /// See: docs/bugs/f1r3fly-integration-challenges.md Section 3
    pub async fn query_state(
        &self,
        contract_id: ContractId,
        query_method: &str,
        params: &[(&str, StrictVal)],
    ) -> Result<Value, F1r3flyRgbError> {
        log::info!(
            "🔍 Querying '{}' on contract {:?}",
            query_method,
            contract_id
        );

        let metadata = self.contracts.get(&contract_id).ok_or_else(|| {
            log::error!("   ❌ Contract not found in registry");
            F1r3flyRgbError::ContractNotFound(format!("Contract {} not found", contract_id))
        })?;

        log::debug!("   Registry URI: {}", metadata.registry_uri);

        // Serialize parameters
        log::info!("   📋 EXECUTOR: query_state - Raw params: {:?}", params);
        let serialized_params = serialize_params(params)?;
        log::info!(
            "   📋 EXECUTOR: query_state - Serialized params: '{}'",
            serialized_params
        );

        // Build parameter list with proper comma handling
        let param_list = if serialized_params.is_empty() {
            String::from("*resultCh")
        } else {
            format!("{}, *resultCh", serialized_params)
        };

        // Build registry lookup + query call with return!() PATTERN
        // CRITICAL: Must use return!() channel to make exploratory deploys wait
        // Registry stores (version, bundle+{*Rho20Token}) as a 2-tuple
        let query_rholang = format!(
            r#"new return, rl(`rho:registry:lookup`), contractCh, resultCh in {{
  rl!(`{}`, *contractCh) |
  for(@(_, contractBundle) <- contractCh) {{
    @{{contractBundle}}!("{}", {}) |
    for(@result <- resultCh) {{
      return!(result)
    }}
  }}
}}"#,
            metadata.registry_uri, query_method, param_list
        );

        log::info!("   📜 Generated query Rholang:\n{}", query_rholang);
        log::debug!("   Using HTTP API (explore-deploy) with return!() pattern");

        // Use HTTP API (not gRPC) for complex responses
        let config = self.connection.config();
        let url = format!(
            "http://{}:{}/api/explore-deploy",
            config.node_host, config.http_port
        );

        let response = self
            .http_client
            .post(&url)
            .body(query_rholang)
            .header("Content-Type", "text/plain")
            .send()
            .await
            .map_err(|e| {
                log::error!("   ❌ HTTP request failed: {}", e);
                F1r3flyRgbError::QueryFailed(format!("HTTP error: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            log::error!("   ❌ HTTP error: {}", status);
            return Err(F1r3flyRgbError::QueryFailed(format!("HTTP {}", status)));
        }

        let json_response: Value = response.json().await.map_err(|e| {
            log::error!("   ❌ JSON parse failed: {}", e);
            F1r3flyRgbError::QueryFailed(format!("Invalid JSON: {}", e))
        })?;

        log::debug!(
            "   📥 Raw JSON response: {}",
            serde_json::to_string_pretty(&json_response)
                .unwrap_or_else(|_| format!("{:?}", json_response))
        );

        // Parse Rholang-typed JSON to plain JSON
        let parsed_result = parse_rholang_result(&json_response);
        log::info!("   📊 Parsed result: {:?}", parsed_result);
        log::debug!("   ✅ Query succeeded");

        parsed_result
    }

    /// Get metadata for a deployed contract
    pub fn get_contract_metadata(&self, contract_id: ContractId) -> Option<&ContractMetadata> {
        self.contracts.get(&contract_id)
    }

    /// Register an existing contract in the local cache
    ///
    /// Use this if you have a contract deployed elsewhere and want to call it.
    pub fn register_contract(&mut self, contract_id: ContractId, metadata: ContractMetadata) {
        self.contracts.insert(contract_id, metadata);
    }
    /// Query contract state by registry URI (without requiring local registration)
    ///
    /// This method allows querying contracts that exist on F1r3fly but aren't
    /// registered in the local executor's contracts HashMap. Useful for consignment
    /// acceptance where Bob needs to query Alice's contract metadata.
    ///
    /// # Arguments
    ///
    /// * `registry_uri` - The rho:id:... URI of the contract on F1r3fly
    /// * `query_method` - Method name to call (e.g., "getMetadata", "balanceOf")
    /// * `params` - Method parameters as (name, value) tuples
    ///
    /// # Returns
    ///
    /// JSON response from the contract query
    ///
    /// # Example
    ///
    /// ```ignore
    /// let metadata = executor.query_by_registry_uri(
    ///     "rho:id:abc123...",
    ///     "getMetadata",
    ///     &[]
    /// ).await?;
    /// ```
    pub async fn query_by_registry_uri(
        &self,
        registry_uri: &str,
        query_method: &str,
        params: &[(&str, StrictVal)],
    ) -> Result<Value, F1r3flyRgbError> {
        log::info!(
            "🔍 Querying '{}' on contract at registry URI: {}",
            query_method,
            registry_uri
        );

        // Serialize parameters
        log::debug!(
            "   📋 EXECUTOR: query_by_registry_uri - Raw params: {:?}",
            params
        );
        let serialized_params = serialize_params(params)?;
        log::debug!(
            "   📋 EXECUTOR: query_by_registry_uri - Serialized params: '{}'",
            serialized_params
        );

        // Build parameter list with proper comma handling
        let param_list = if serialized_params.is_empty() {
            String::from("*resultCh")
        } else {
            format!("{}, *resultCh", serialized_params)
        };

        // Build registry lookup + query call with return!() PATTERN
        let query_rholang = format!(
            r#"new return, rl(`rho:registry:lookup`), contractCh, resultCh in {{
  rl!(`{}`, *contractCh) |
  for(@(_, contractBundle) <- contractCh) {{
    @{{contractBundle}}!("{}", {}) |
    for(@result <- resultCh) {{
      return!(result)
    }}
  }}
}}"#,
            registry_uri, query_method, param_list
        );

        log::debug!("   📜 Generated query Rholang:\n{}", query_rholang);

        // Use HTTP API (not gRPC) for complex responses
        let config = self.connection.config();
        let url = format!(
            "http://{}:{}/api/explore-deploy",
            config.node_host, config.http_port
        );

        let response = self
            .http_client
            .post(&url)
            .json(&json!({ "term": query_rholang }))
            .send()
            .await
            .map_err(|e| F1r3flyRgbError::InvalidResponse(format!("HTTP request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(F1r3flyRgbError::InvalidResponse(format!(
                "Query failed with status {}: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            )));
        }

        let response_json: Value = response.json().await.map_err(|e| {
            F1r3flyRgbError::InvalidResponse(format!("Failed to parse response JSON: {}", e))
        })?;

        log::debug!(
            "   📥 Raw JSON response: {}",
            serde_json::to_string_pretty(&response_json).unwrap_or_default()
        );

        // Extract result using same parsing logic as query_state
        let expr_array = response_json["expr"].as_array().ok_or_else(|| {
            F1r3flyRgbError::InvalidResponse(format!(
                "Response missing 'expr' array: {:?}",
                response_json
            ))
        })?;

        // Check if expr array is empty (contract not found or method returned no data)
        if expr_array.is_empty() {
            log::warn!("   ⚠️  Empty expr array - contract may not exist at registry URI or method returned no data");
            log::warn!("   Registry URI: {}", registry_uri);
            log::warn!("   Method: {}", query_method);

            // Return empty object instead of error for better handling
            return Ok(json!({}));
        }

        let expr = expr_array.first().ok_or_else(|| {
            F1r3flyRgbError::InvalidResponse(format!(
                "Failed to get first element from expr array: {:?}",
                response_json
            ))
        })?;

        log::info!("   📊 Parsed result: {:?}", expr);

        Ok(expr.clone())
    }

    /// Get current child key for test signature generation
    ///
    /// **For testing only** - Exposes the child private key used for contract deployment
    /// so tests can generate valid signatures for the secured issue() method.
    ///
    /// # Security
    /// This method is intended for integration tests only. In production, signatures
    /// should be generated at the wallet layer with proper key management.
    ///
    /// # Returns
    /// The child private key corresponding to the current derivation index
    pub fn get_child_key_for_testing(&self) -> Result<SecretKey, F1r3flyRgbError> {
        derive_child_key_from_master(&self.connection.config().signing_key, self.derivation_index)
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Derive a child private key from master key using simple hash-based derivation
///
/// Uses a cryptographic hash to deterministically derive unique keys for each contract.
/// This is a lightweight alternative to full BIP32 that provides:
/// - Deterministic derivation (same index always yields same key)
/// - Unique keys per contract (different indices yield different keys)
/// - Cryptographically secure (Blake2b-256)
///
/// # Arguments
/// - `master_key_hex`: Hex-encoded master private key
/// - `index`: Derivation index for this contract (0, 1, 2, ...)
///
/// # Returns
/// Derived child private key for this specific contract
///
/// # Security
/// Each contract gets a unique derived key, ensuring:
/// - Unique registry URIs per contract
/// - Single master key funds all deployments
/// - Deterministic key recovery from master + index
fn derive_child_key_from_master(
    master_key_hex: &str,
    index: u32,
) -> Result<SecretKey, F1r3flyRgbError> {
    use blake2::digest::consts::U32;
    use blake2::{Blake2b, Digest};

    // Parse master key bytes
    let master_key_bytes = hex::decode(master_key_hex).map_err(|e| {
        F1r3flyRgbError::InvalidRholangSource(format!("Invalid master key hex: {}", e))
    })?;

    if master_key_bytes.len() != 32 {
        return Err(F1r3flyRgbError::InvalidRholangSource(format!(
            "Master key must be 32 bytes, got {}",
            master_key_bytes.len()
        )));
    }

    // Derive child key: hash(master_key || index || "f1r3fly-rgb-contract")
    // The domain separator ensures these keys are only used for F1r3fly-RGB contracts
    let domain_separator = b"f1r3fly-rgb-contract-v1";
    let index_bytes = index.to_be_bytes();

    let child_key_bytes = Blake2b::<U32>::new()
        .chain_update(&master_key_bytes)
        .chain_update(&index_bytes)
        .chain_update(domain_separator)
        .finalize();

    // Convert to secp256k1 SecretKey
    SecretKey::from_slice(&child_key_bytes)
        .map_err(|e| F1r3flyRgbError::InvalidRholangSource(format!("Invalid derived key: {}", e)))
}

/// Substitute template variables in Rholang contract
///
/// Replaces template placeholders with actual values:
/// - {{TICKER}}, {{NAME}}, {{TOTAL_SUPPLY}}, {{PRECISION}}
/// - {{PUBLIC_KEY}}, {{SIGNATURE}}, {{VERSION}}, {{URI}}
///
/// # Arguments
/// - `template`: The contract template with {{PLACEHOLDERS}}
/// - `ticker`: Token ticker symbol
/// - `name`: Token name
/// - `total_supply`: Initial total supply
/// - `precision`: Decimal precision
/// - `master_key_hex`: Master key for gRPC deployment signing (pays phlo)
/// - `child_key`: BIP32-derived child key for unique URI generation
///
/// # Returns
/// Complete Rholang code ready for deployment + timestamp
///
/// # BIP32 Deployment Flow (Multi-Contract Support)
/// 1. Master key signs gRPC deployment → pays phlo from master's REV vault
/// 2. Child key generates insertSigned signature → proves ownership
/// 3. Child key computes unique URI → enables multiple contracts
/// 4. Deploy contains proof of both keys → registry accepts it
/// 5. Each contract gets unique URI: `rho:id:{hash(child_pubkey)}`
///
/// See: docs/bugs/f1r3fly-integration-challenges.md Section 1
fn substitute_template_variables(
    template: &str,
    ticker: &str,
    name: &str,
    total_supply: u64,
    precision: u8,
    master_key_hex: &str,
    child_key: &SecretKey,
) -> Result<(String, i64), F1r3flyRgbError> {
    // Parse master key for deployerPubKey in signature
    let master_key_bytes = hex::decode(master_key_hex).map_err(|e| {
        F1r3flyRgbError::InvalidRholangSource(format!("Invalid master key hex: {}", e))
    })?;

    let master_secret_key = SecretKey::from_slice(&master_key_bytes).map_err(|e| {
        F1r3flyRgbError::InvalidRholangSource(format!("Invalid secp256k1 master key: {}", e))
    })?;

    // Derive public keys
    let secp = Secp256k1::new();
    let master_public_key = PublicKey::from_secret_key(&secp, &master_secret_key);
    let child_public_key = PublicKey::from_secret_key(&secp, child_key);

    // Generate timestamp and version
    let timestamp = Utc::now();
    let timestamp_millis = timestamp.timestamp_millis();
    let version = timestamp_millis; // Use timestamp as version

    // Generate insertSigned signature with CHILD key
    // Signature proves: "I (child key) authorize deployment signed by (master key) at (timestamp) with (version)"
    // Signature is for: (timestamp_millis, master_pubkey_bytes, version)
    let signature =
        generate_insert_signed_signature(child_key, timestamp, &master_public_key, version);
    let signature_hex = hex::encode(&signature);

    // Use CHILD public key for registry URI (ensures unique URI per contract)
    let child_pubkey_hex = hex::encode(child_public_key.serialize_uncompressed());

    // Deployer public key is the CHILD public key (uncompressed secp256k1, 65 bytes)
    // This key will be stored in the contract and used to verify issue() signatures
    // The wallet's f1r3fly_private_key (which matches this public key) will sign issue calls
    let deployer_pubkey_hex = child_pubkey_hex.clone();

    // Compute deterministic URI from CHILD public key
    let uri = public_key_to_uri(&child_public_key);

    // Replace all template variables
    let rholang = template
        .replace("{{TICKER}}", ticker)
        .replace("{{NAME}}", name)
        .replace("{{TOTAL_SUPPLY}}", &total_supply.to_string())
        .replace("{{PRECISION}}", &precision.to_string())
        .replace("{{PUBLIC_KEY}}", &child_pubkey_hex)
        .replace("{{VERSION}}", &version.to_string())
        .replace("{{SIGNATURE}}", &signature_hex)
        .replace("{{URI}}", &uri)
        .replace("{{DEPLOYER_PUBLIC_KEY}}", &deployer_pubkey_hex);

    // Return both the Rholang code AND the timestamp for deploy_with_timestamp
    Ok((rholang, timestamp_millis))
}

/// Compute deterministic registry URI from child key
///
/// Derives the public key from the child secret key and computes
/// the registry URI that will be assigned by insertSigned.
///
/// # Arguments
/// - `child_key`: BIP32-derived child secret key
///
/// # Returns
/// Deterministic URI: `rho:id:{hash(child_pubkey)}`
///
/// # Algorithm
/// 1. Derive public key from child secret key (secp256k1)
/// 2. Serialize public key in uncompressed format (65 bytes)
/// 3. Hash with Blake2b-256 (32 bytes)
/// 4. Compute CRC14 checksum (2 bytes)
/// 5. Combine hash + CRC (34 bytes total)
/// 6. Encode with zbase32
/// 7. Format as `rho:id:{encoded}`
fn compute_registry_uri_from_child_key(child_key: &SecretKey) -> Result<String, F1r3flyRgbError> {
    // Derive public key from child key
    let secp = Secp256k1::new();
    let public_key = PublicKey::from_secret_key(&secp, child_key);

    // Use existing registry helper to compute URI
    Ok(public_key_to_uri(&public_key))
}

/// Derive a ContractId from a registry URI
///
/// Uses Blake2b-256 to hash the registry URI string, producing a deterministic
/// 32-byte ContractId. This ensures:
/// - Same URI always produces same ContractId
/// - ContractId is independent of URI length
/// - Cryptographically secure mapping
///
/// # Arguments
/// - `registry_uri`: The `rho:id:...` URI string
///
/// # Returns
/// A 32-byte ContractId derived from the URI
fn derive_contract_id_from_uri(registry_uri: &str) -> ContractId {
    use blake2::digest::consts::U32;
    use blake2::{Blake2b, Digest};

    // Hash the URI with Blake2b-256
    let hash = Blake2b::<U32>::new()
        .chain_update(registry_uri.as_bytes())
        .finalize();

    // Convert to 32-byte array
    let mut contract_id_bytes = [0u8; 32];
    contract_id_bytes.copy_from_slice(&hash);

    ContractId::from_byte_array(contract_id_bytes)
}

/// Serialize parameters to Rholang syntax
///
/// Converts method parameters to Rholang-compatible format for method calls.
///
/// # Arguments
/// - `params`: Array of (name, value) pairs to serialize
///
/// # Format
/// Parameters are serialized as positional arguments, e.g.:
/// - `[("amount", 100), ("recipient", "addr")]` → `100, "addr"`
///
/// # Current Implementation
/// Basic support for common RGB types. Will be extended as needed.
fn serialize_params(params: &[(&str, StrictVal)]) -> Result<String, F1r3flyRgbError> {
    if params.is_empty() {
        return Ok(String::new());
    }

    let mut parts = Vec::new();

    for (_name, value) in params {
        // Convert StrictVal to Rholang representation
        let rholang_repr = strict_val_to_rholang(value)?;
        parts.push(rholang_repr);
    }

    Ok(parts.join(", "))
}

/// Convert StrictVal to Rholang syntax
///
/// Maps RGB's StrictVal types to their Rholang equivalents.
fn strict_val_to_rholang(val: &StrictVal) -> Result<String, F1r3flyRgbError> {
    use strict_types::StrictVal::*;

    match val {
        // Numbers
        Number(num) => Ok(num.to_string()),

        // Strings
        String(s) => Ok(format!("\"{}\"", s.escape_default())),

        // Bytes
        Bytes(b) => Ok(format!("\"{}\"hexToBytes()", hex::encode(b))),

        // Lists/Arrays
        List(items) => {
            let rholang_items: Result<Vec<std::string::String>, F1r3flyRgbError> =
                items.iter().map(strict_val_to_rholang).collect();
            Ok(format!("[{}]", rholang_items?.join(", ")))
        }

        // Maps/Objects
        Map(map) => {
            let mut pairs = Vec::new();
            for (key, value) in map.iter() {
                let key_rho = strict_val_to_rholang(key)?;
                let val_rho = strict_val_to_rholang(value)?;
                pairs.push(format!("{}: {}", key_rho, val_rho));
            }
            Ok(format!("{{{}}}", pairs.join(", ")))
        }

        // Union/Enums (use their inner value)
        Union(_tag, inner) => strict_val_to_rholang(inner),

        // Other types: use debug representation as fallback
        _ => {
            log::warn!(
                "Unsupported StrictVal type for Rholang serialization: {:?}",
                val
            );
            Ok(format!("\"{}\"", format!("{:?}", val).escape_default()))
        }
    }
}

/// Parse Rholang-typed JSON response to plain JSON
///
/// F1r3fly HTTP API returns Rholang expressions wrapped in type metadata:
/// - `{"ExprString": {"data": "hello"}}` → `"hello"`
/// - `{"ExprInt": {"data": 42}}` → `42`
/// - `{"ExprMap": {"data": {...}}}` → `{...}` (recursive)
///
/// This function unwraps these type wrappers to provide clean JSON for consumption.
///
/// # Why HTTP Instead of gRPC?
/// gRPC responses are simplified to strings like "Complex expression with 2 fields",
/// losing the actual data. HTTP API returns full JSON structures.
///
/// See: docs/bugs/f1r3fly-integration-challenges.md Section 3
fn parse_rholang_result(json_response: &Value) -> Result<Value, F1r3flyRgbError> {
    log::debug!("🔍 parse_rholang_result: Starting parse");

    // Extract expr array from response
    // F1r3fly HTTP format: {"expr": [<expr1>, <expr2>, ...]}
    let expr_array = json_response
        .get("expr")
        .and_then(|v| v.as_array())
        .ok_or_else(|| {
            log::error!("❌ parse_rholang_result: No 'expr' array in response");
            F1r3flyRgbError::QueryFailed(
                "Invalid response format: missing 'expr' array".to_string(),
            )
        })?;

    log::debug!(
        "🔍 parse_rholang_result: Found expr array with {} elements",
        expr_array.len()
    );

    if expr_array.is_empty() {
        log::error!("❌ parse_rholang_result: expr array is empty");
        return Err(F1r3flyRgbError::QueryFailed(
            "No data returned from query".to_string(),
        ));
    }

    // Get first expression (the return value from the contract)
    let rholang_expr = &expr_array[0];
    log::debug!("🔍 parse_rholang_result: First expr = {:?}", rholang_expr);

    // Check if it's Nil
    if rholang_expr.get("ExprNil").is_some() {
        log::warn!(
            "⚠️ parse_rholang_result: Got ExprNil - contract method returned Nil or didn't execute"
        );
        return Err(F1r3flyRgbError::QueryFailed(
            "Contract method returned Nil. Possible causes: method not found, contract not initialized, or method failed to execute".to_string()
        ));
    }

    // Use existing helper from rust-client to unwrap Rholang types
    let parsed = convert_rholang_to_json(rholang_expr).map_err(|e| {
        log::error!(
            "❌ parse_rholang_result: Failed to convert Rholang to JSON: {}",
            e
        );
        F1r3flyRgbError::QueryFailed(format!("Failed to parse Rholang result: {}", e))
    })?;

    log::debug!("🔍 parse_rholang_result: Parsed = {:?}", parsed);

    // If the result is a tuple (common pattern: [actualValue, unforgeable continuation]),
    // extract just the first element (the actual data)
    if let Some(tuple_obj) = parsed.get("ExprTuple") {
        log::debug!("🔍 parse_rholang_result: Found ExprTuple, extracting first element");
        if let Some(data_array) = tuple_obj.get("data").and_then(|v| v.as_array()) {
            if !data_array.is_empty() {
                // Recursively parse the first element of the tuple
                return convert_rholang_to_json(&data_array[0]).map_err(|e| {
                    log::error!(
                        "❌ parse_rholang_result: Failed to parse tuple element: {}",
                        e
                    );
                    F1r3flyRgbError::QueryFailed(format!("Failed to parse tuple element: {}", e))
                });
            }
        }
    }

    log::debug!("✅ parse_rholang_result: Returning parsed result");
    Ok(parsed)
}

/// Compute a 32-byte state hash for Bitcoin commitment
///
/// Derives a deterministic hash from F1r3node's finalization data:
/// - `block_hash`: F1r3node block hash containing the deploy
/// - `deploy_id`: Unique deploy identifier
///
/// This hash represents the F1r3fly state at finalization and can be embedded
/// in Bitcoin transactions via Tapret commitments.
///
/// # Algorithm
/// Uses Blake2b-256 to hash: `block_hash || deploy_id`
///
/// # Returns
/// A 32-byte array suitable for RGB commitment
fn compute_state_hash(block_hash: &str, deploy_id: &str) -> [u8; 32] {
    use blake2::digest::consts::U32;
    use blake2::{Blake2b, Digest};

    // Hash: Blake2b256(block_hash || deploy_id)
    let hash = Blake2b::<U32>::new()
        .chain_update(block_hash.as_bytes())
        .chain_update(deploy_id.as_bytes())
        .finalize();

    let mut state_hash = [0u8; 32];
    state_hash.copy_from_slice(&hash);
    state_hash
}
