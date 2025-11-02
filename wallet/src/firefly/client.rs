use node_cli::f1r3fly_api::F1r3flyApi;
use crate::firefly::types::*;
use crate::firefly::helpers::convert_rholang_to_json;

/// RGB storage contract URIs
/// These are obtained from deploying rgb_state_storage.rho
#[derive(Debug, Clone)]
pub struct RgbStorageUris {
    pub store_contract: String,
    pub get_contract: String,
    pub search_by_ticker: String,
    pub store_allocation: String,
    pub get_allocation: String,
    pub record_transition: String,
    pub get_transition: String,
}

/// FireflyClient wraps the F1r3flyApi from rust-client and adds RGB-specific methods
/// 
/// Note: We store the connection parameters and create F1r3flyApi instances on-demand
/// to avoid lifetime issues without leaking memory.
#[derive(Clone)]
pub struct FireflyClient {
    pub node_host: String,
    pub grpc_port: u16,
    pub http_port: u16,
    signing_key: String,
    rgb_uris: Option<RgbStorageUris>,
}

impl FireflyClient {
    pub fn new(host: &str, grpc_port: u16, http_port: u16) -> Self {
        // Load private key from environment, fallback to default bootstrap key
        let signing_key = std::env::var("FIREFLY_PRIVATE_KEY")
            .unwrap_or_else(|_| {
                log::warn!("FIREFLY_PRIVATE_KEY not set, using default bootstrap key");
                "5f668a7ee96d944a4494cc947e4005e172d7ab3461ee5538f1f2a45a835e9657".to_string()
            });
        
        Self {
            node_host: host.to_string(),
            grpc_port,
            http_port,
            signing_key,
            rgb_uris: None,
        }
    }
    
    /// Set RGB storage contract URIs
    /// Call this after deploying rgb_state_storage.rho to enable RGB operations
    pub fn set_rgb_uris(&mut self, uris: RgbStorageUris) {
        self.rgb_uris = Some(uris);
    }
    
    /// Get RGB storage URIs (returns error if not set)
    fn get_rgb_uris(&self) -> Result<&RgbStorageUris, Box<dyn std::error::Error>> {
        self.rgb_uris.as_ref()
            .ok_or_else(|| "RGB storage URIs not set. Call set_rgb_uris() first.".into())
    }
    
    /// Create an F1r3flyApi instance for this operation
    /// This is cheap since F1r3flyApi just holds references and a SecretKey
    pub fn api(&self) -> F1r3flyApi {
        F1r3flyApi::new(&self.signing_key, &self.node_host, self.grpc_port)
    }

    // ============================================================================
    // Core F1r3fly Operations (delegated to F1r3flyApi)
    // ============================================================================

    /// Deploy Rholang code (writes to blockchain)
    /// Delegates to F1r3flyApi which handles VABN and Block 50 issues automatically
    pub async fn deploy(&self, rholang_code: &str) -> Result<String, Box<dyn std::error::Error>> {
        // Use 500,000 phlo - enough for complex contracts like RGB storage with insertSigned
        // 50,000 is too low (OutOfPhlogistonsError), 5 billion causes contracts not to execute
        self.api().deploy_with_phlo_limit(rholang_code, 500_000, "rholang").await
    }
    
    /// Deploy Rholang code with a specific timestamp (for insertSigned compatibility)
    /// This is CRITICAL for insertSigned to work - the deploy timestamp must match the signature timestamp
    /// 
    /// The Scala version of insertSigned verifies: blake2b256((timestamp, deployerPubKey, version))
    /// where timestamp comes from rho:deploy:data, so we MUST deploy with the same timestamp we signed
    pub async fn deploy_with_timestamp(&self, rholang_code: &str, timestamp_millis: i64) -> Result<String, Box<dyn std::error::Error>> {
        // Use custom phlo limit and pass the timestamp through
        self.api().deploy_with_timestamp_and_phlo_limit(rholang_code, "rholang", Some(timestamp_millis), 500_000).await
    }

    /// Wait for a deploy to be included in a block
    /// Returns the block hash once the deploy is found
    /// Uses rust-client's get_deploy_block_hash pattern
    pub async fn wait_for_deploy(
        &self,
        deploy_id: &str,
        max_attempts: u32,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let check_interval_sec = 1;
        let api = self.api();
        
        for attempt in 1..=max_attempts {
            // Use rust-client's get_deploy_block_hash method
            let result = api.get_deploy_block_hash(deploy_id, self.http_port).await
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                    Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
                })?;
            
            match result {
                Some(block_hash) => {
                    log::debug!("Deploy {} found in block {} after {} attempts", 
                        deploy_id, block_hash, attempt);
                    return Ok(block_hash);
                }
                None => {
                    if attempt >= max_attempts {
                        return Err(Box::new(std::io::Error::new(
                            std::io::ErrorKind::TimedOut,
                            format!("Deploy not included in block after {} attempts", max_attempts)
                        )));
                    }
                    tokio::time::sleep(tokio::time::Duration::from_secs(check_interval_sec)).await;
                }
            }
        }

        Err(Box::new(std::io::Error::new(std::io::ErrorKind::TimedOut, "Deploy wait timeout")))
    }

    /// Wait for a specific block to be finalized
    /// Uses rust-client's is_finalized method via gRPC
    pub async fn wait_for_block_finalization(
        &self,
        target_block_hash: &str,
        max_attempts: u32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Use rust-client's is_finalized method (gRPC-based)
        // Default retry delay of 5 seconds (same as rust-client deploy_and_wait)
        let retry_delay_sec = 5;
        let api = self.api();
        
        match api.is_finalized(target_block_hash, max_attempts, retry_delay_sec).await? {
            true => Ok(()),
            false => Err(format!(
                "Block {} not finalized after {} attempts",
                target_block_hash, max_attempts
            )
            .into()),
        }
    }

    /// Deploy Rholang code and wait for it to be finalized
    /// This is the recommended method for deploying RGB state that needs to be queried
    /// Follows the rust-client deploy_and_wait pattern:
    /// 1. Deploy code
    /// 2. Wait for deploy to be included in a block
    /// 3. Wait for block to be finalized
    pub async fn deploy_and_wait_for_finalization(
        &self,
        rholang_code: &str,
        max_block_wait_attempts: u32,
        max_finalization_attempts: u32,
    ) -> Result<(String, String), Box<dyn std::error::Error + Send + Sync>> {
        // Step 1: Deploy the code
        let deploy_id = self.deploy(rholang_code).await
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
            })?;
        
        // Step 2: Wait for deploy to be included in a block
        let block_hash = self.wait_for_deploy(&deploy_id, max_block_wait_attempts).await?;
        
        // Step 3: Wait for block to be finalized
        self.wait_for_block_finalization(&block_hash, max_finalization_attempts).await
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
            })?;
        
        Ok((deploy_id, block_hash))
    }

    // ============================================================================
    // RGB State Storage Operations (RGB-specific, kept in wallet)
    // ============================================================================

    /// Query RSpace state using exploratory deploy (read-only)
    pub async fn query_state(&self, query_code: &str) -> Result<String, Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        
        // Use /api/explore-deploy endpoint with query_code as JSON string
        let response = client
            .post(&format!("http://{}:{}/api/explore-deploy", &self.node_host, self.http_port))
            .header("Content-Type", "application/json")
            .json(&query_code)
            .send()
            .await?;
        
        if response.status().is_success() {
            let result: serde_json::Value = response.json().await?;
            
            // Extract the expr array from the response
            if let Some(expr_array) = result.get("expr") {
                if let Some(expr_list) = expr_array.as_array() {
                    if !expr_list.is_empty() {
                        // Return the first expression result
                        return Ok(serde_json::to_string(&expr_list[0])?);
                    }
                }
            }
            
            // If no expr data, return empty object
            Ok("{}".to_string())
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            Err(format!("Query failed with status {}: {}", status, error_text).into())
        }
    }

    /// Store RGB contract metadata using the RGB Storage Contract
    /// Calls the "storeContract" method on the registered RGBStorage contract
    /// 
    /// Note: This requires the RGB storage contract to be deployed first
    /// and set_rgb_uris() to be called with the contract URIs
    /// Store RGB contract metadata using the RGB Storage Contract
    /// 
    /// Implementation:
    /// 1. Look up the registered RGBStorage contract via registry
    /// 2. Call the "storeContract" method on the contract
    /// 3. The contract stores data in its internal treeHashMap (persists in closure)
    /// 
    /// This works because:
    /// - The contract is registered in the system registry
    /// - State lives in the contract's closure (accessible from exploratory deploys)
    pub async fn store_contract(
        &self,
        contract_id: &str,
        metadata: ContractMetadata,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let uris = self.get_rgb_uris()?;
        
        // Generate Rholang code
        let rholang_code = format!(
            r#"new rl(`rho:registry:lookup`), rgbCh, ack in {{
  rl!(`{}`, *rgbCh) |
  for(@(_, RGBStorage) <- rgbCh) {{
    @RGBStorage!("storeContract", "{}", {{
      "schema": "RGB20",
      "ticker": "{}",
      "name": "{}",
      "precision": {},
      "total_supply": {},
      "genesis_txid": "{}",
      "issuer": "{}"
    }}, *ack)
  }}
}}"#,
            uris.store_contract,  // The URI of the registered RGBStorage contract
            contract_id,
            metadata.ticker,
            metadata.name,
            metadata.precision,
            metadata.total_supply,
            metadata.genesis_txid,
            metadata.issuer_pubkey
        );
        
        self.deploy(&rholang_code).await
    }

    /// Query RGB contract metadata using the RGB Storage Contract
    /// Calls the "getContract" method on the registered RGBStorage contract
    /// 
    /// Note: This requires the RGB storage contract to be deployed first
    /// and set_rgb_uris() to be called with the contract URIs
    pub async fn query_contract(
        &self,
        contract_id: &str,
        _at_block_hash: Option<&str>,  // Not needed with insertSigned pattern
    ) -> Result<ContractData, Box<dyn std::error::Error>> {
        let uris = self.get_rgb_uris()?;
        
        // Generate Rholang query code
        let query_code = format!(
            r#"new return, rl(`rho:registry:lookup`), rgbCh in {{
  rl!(`{}`, *rgbCh) |
  for(@(_, rgbStorage) <- rgbCh) {{
    @rgbStorage!("getContract", "{}", *return)
  }}
}}"#,
            uris.get_contract,
            contract_id
        );
        
        // Use HTTP API directly to get the full JSON response
        // This preserves complex Rholang data structures that rust-client's simplified parser loses
        let http_client = reqwest::Client::new();
        let response = http_client
            .post(format!("http://{}:{}/api/explore-deploy", self.node_host, self.http_port))
            .body(query_code)
            .header("Content-Type", "text/plain")
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Ok(ContractData {
                success: false,
                contract: None,
                error: Some(format!("HTTP error: {}", response.status())),
            });
        }
        
        // Parse the full JSON response
        let json_response: serde_json::Value = response.json().await?;
        
        log::debug!("Raw JSON response: {}", serde_json::to_string_pretty(&json_response)?);
        
        // Extract the expr data from the response
        // Response format: {"expr": [{"ExprMap": {...}}], "block": {...}}
        if let Some(expr_array) = json_response.get("expr").and_then(|v| v.as_array()) {
            if expr_array.is_empty() {
                return Ok(ContractData {
                    success: false,
                    contract: None,
                    error: Some("No data returned from contract".to_string()),
                });
            }
            
            // The contract returns a Rholang map, we need to convert it to plain JSON
            let rholang_expr = &expr_array[0];
            log::debug!("Rholang expression: {}", serde_json::to_string_pretty(rholang_expr)?);
            
            // Convert Rholang ExprMap to plain JSON
            let plain_json = convert_rholang_to_json(rholang_expr)?;
            log::debug!("Converted to plain JSON: {}", serde_json::to_string_pretty(&plain_json)?);
            
            // Try to parse as ContractData
            match serde_json::from_value::<ContractData>(plain_json.clone()) {
                Ok(contract_data) => Ok(contract_data),
                Err(e) => {
                    Ok(ContractData {
                        success: false,
                        contract: None,
                        error: Some(format!("Failed to parse contract data: {}. Raw: {}", e, plain_json)),
                    })
                }
            }
        } else {
            Ok(ContractData {
                success: false,
                contract: None,
                error: Some("Invalid response format".to_string()),
            })
        }
    }

    /// Search for a contract by ticker symbol
    /// Search for RGB contract by ticker using the RGB Storage Contract
    /// 
    /// Uses secondary index pattern: ticker → contract_id → metadata
    pub async fn search_contract_by_ticker(
        &self,
        ticker: &str,
    ) -> Result<ContractData, Box<dyn std::error::Error>> {
        let uris = self.get_rgb_uris()?;
        
        // Generate Rholang query code
        let query_code = format!(
            r#"new return, rl(`rho:registry:lookup`), rgbCh in {{
  rl!(`{}`, *rgbCh) |
  for(@(_, rgbStorage) <- rgbCh) {{
    @rgbStorage!("searchByTicker", "{}", *return)
  }}
}}"#,
            uris.search_by_ticker,
            ticker
        );
        
        // Use HTTP API directly to get the full JSON response
        let http_client = reqwest::Client::new();
        let response = http_client
            .post(format!("http://{}:{}/api/explore-deploy", self.node_host, self.http_port))
            .body(query_code)
            .header("Content-Type", "text/plain")
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Ok(ContractData {
                success: false,
                contract: None,
                error: Some(format!("HTTP error: {}", response.status())),
            });
        }
        
        // Parse the full JSON response
        let json_response: serde_json::Value = response.json().await?;
        
        log::debug!("Raw JSON response: {}", serde_json::to_string_pretty(&json_response)?);
        
        // Extract the expr data from the response
        if let Some(expr_array) = json_response.get("expr").and_then(|v| v.as_array()) {
            if expr_array.is_empty() {
                return Ok(ContractData {
                    success: false,
                    contract: None,
                    error: Some("No data returned from contract".to_string()),
                });
            }
            
            // Convert Rholang ExprMap to plain JSON
            let rholang_expr = &expr_array[0];
            log::debug!("Rholang expression: {}", serde_json::to_string_pretty(rholang_expr)?);
            
            let plain_json = convert_rholang_to_json(rholang_expr)?;
            log::debug!("Converted to plain JSON: {}", serde_json::to_string_pretty(&plain_json)?);
            
            // Try to parse as ContractData
            match serde_json::from_value::<ContractData>(plain_json.clone()) {
                Ok(contract_data) => Ok(contract_data),
                Err(e) => {
                    Ok(ContractData {
                        success: false,
                        contract: None,
                        error: Some(format!("Failed to parse contract data: {}. Raw: {}", e, plain_json)),
                    })
                }
            }
        } else {
            Ok(ContractData {
                success: false,
                contract: None,
                error: Some("Invalid response format".to_string()),
            })
        }
    }

    /// Store RGB allocation using the RGB Storage Contract
    /// 
    /// Stores allocation data in the RGBStorage contract's allocationMapCh
    /// Key format: "{contract_id}:{utxo}"
    pub async fn store_allocation(
        &self,
        contract_id: &str,
        utxo: &str,
        owner_pubkey: &str,
        amount: u64,
        bitcoin_txid: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let uris = self.get_rgb_uris()?;
        
        // Generate composite key: contract_id:utxo
        let key = format!("{}:{}", contract_id, utxo);
        
        // Get current timestamp
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        
        // Generate Rholang code
        let rholang_code = format!(
            r#"new rl(`rho:registry:lookup`), rgbCh, ack in {{
  rl!(`{}`, *rgbCh) |
  for(@(_, RGBStorage) <- rgbCh) {{
    @RGBStorage!("storeAllocation", "{}", {{
      "owner": "{}",
      "amount": {},
      "bitcoin_txid": "{}",
      "confirmed": true,
      "created_at": {}
    }}, *ack)
  }}
}}"#,
            uris.store_allocation,
            key,
            owner_pubkey,
            amount,
            bitcoin_txid,
            timestamp
        );

        self.deploy(&rholang_code).await
    }

    /// Query RGB allocation using the RGB Storage Contract
    /// 
    /// Queries allocation data from the RGBStorage contract's allocationMapCh
    /// Key format: "{contract_id}:{utxo}"
    pub async fn query_allocation(
        &self,
        contract_id: &str,
        utxo: &str,
    ) -> Result<AllocationData, Box<dyn std::error::Error>> {
        let uris = self.get_rgb_uris()?;
        
        // Generate composite key: contract_id:utxo
        let key = format!("{}:{}", contract_id, utxo);
        
        // Generate Rholang query code
        let query_code = format!(
            r#"new return, rl(`rho:registry:lookup`), rgbCh in {{
  rl!(`{}`, *rgbCh) |
  for(@(_, rgbStorage) <- rgbCh) {{
    @rgbStorage!("getAllocation", "{}", *return)
  }}
}}"#,
            uris.get_allocation,
            key
        );
        
        // Use HTTP API directly to get the full JSON response
        let http_client = reqwest::Client::new();
        let response = http_client
            .post(format!("http://{}:{}/api/explore-deploy", self.node_host, self.http_port))
            .body(query_code)
            .header("Content-Type", "text/plain")
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Ok(AllocationData {
                success: false,
                allocation: None,
                error: Some(format!("HTTP error: {}", response.status())),
            });
        }
        
        // Parse the full JSON response
        let json_response: serde_json::Value = response.json().await?;
        
        log::debug!("Raw JSON response: {}", serde_json::to_string_pretty(&json_response)?);
        
        // Extract the expr data from the response
        if let Some(expr_array) = json_response.get("expr").and_then(|v| v.as_array()) {
            if expr_array.is_empty() {
                return Ok(AllocationData {
                    success: false,
                    allocation: None,
                    error: Some("No data returned from contract".to_string()),
                });
            }
            
            // Convert Rholang ExprMap to plain JSON
            let rholang_expr = &expr_array[0];
            log::debug!("Rholang expression: {}", serde_json::to_string_pretty(rholang_expr)?);
            
            let plain_json = convert_rholang_to_json(rholang_expr)?;
            log::debug!("Converted to plain JSON: {}", serde_json::to_string_pretty(&plain_json)?);
            
            // Try to parse as AllocationData
            match serde_json::from_value::<AllocationData>(plain_json.clone()) {
                Ok(allocation_data) => Ok(allocation_data),
                Err(e) => {
                    Ok(AllocationData {
                        success: false,
                        allocation: None,
                        error: Some(format!("Failed to parse allocation data: {}. Raw: {}", e, plain_json)),
                    })
                }
            }
        } else {
            Ok(AllocationData {
                success: false,
                allocation: None,
                error: Some("Invalid response format".to_string()),
            })
        }
    }

    /// Store RGB transition using the RGB Storage Contract
    /// 
    /// Stores transition data in the RGBStorage contract's transitionMapCh
    /// Key format: "{contract_id}:{from_utxo}:{to_utxo}"
    pub async fn record_transition(
        &self,
        contract_id: &str,
        from_utxo: &str,
        to_utxo: &str,
        amount: u64,
        bitcoin_txid: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let uris = self.get_rgb_uris()?;
        
        // Generate composite key: contract_id:from_utxo:to_utxo
        let key = format!("{}:{}:{}", contract_id, from_utxo, to_utxo);
        
        // Get current timestamp
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        
        // Generate Rholang code
        let rholang_code = format!(
            r#"new rl(`rho:registry:lookup`), rgbCh, ack in {{
  rl!(`{}`, *rgbCh) |
  for(@(_, RGBStorage) <- rgbCh) {{
    @RGBStorage!("recordTransition", "{}", {{
      "from": "{}",
      "to": "{}",
      "amount": {},
      "bitcoin_txid": "{}",
      "timestamp": {},
      "validated": false
    }}, *ack)
  }}
}}"#,
            uris.record_transition,
            key,
            from_utxo,
            to_utxo,
            amount,
            bitcoin_txid,
            timestamp
        );

        self.deploy(&rholang_code).await
    }

    /// Query RGB transition using the RGB Storage Contract
    /// 
    /// Queries transition data from the RGBStorage contract's transitionMapCh
    /// Key format: "{contract_id}:{from_utxo}:{to_utxo}"
    pub async fn query_transition(
        &self,
        contract_id: &str,
        from_utxo: &str,
        to_utxo: &str,
    ) -> Result<TransitionData, Box<dyn std::error::Error>> {
        let uris = self.get_rgb_uris()?;
        
        // Generate composite key: contract_id:from_utxo:to_utxo
        let key = format!("{}:{}:{}", contract_id, from_utxo, to_utxo);
        
        // Generate Rholang query code
        let query_code = format!(
            r#"new return, rl(`rho:registry:lookup`), rgbCh in {{
  rl!(`{}`, *rgbCh) |
  for(@(_, rgbStorage) <- rgbCh) {{
    @rgbStorage!("getTransition", "{}", *return)
  }}
}}"#,
            uris.get_transition,
            key
        );
        
        // Use HTTP API directly to get the full JSON response
        let http_client = reqwest::Client::new();
        let response = http_client
            .post(format!("http://{}:{}/api/explore-deploy", self.node_host, self.http_port))
            .body(query_code)
            .header("Content-Type", "text/plain")
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Ok(TransitionData {
                success: false,
                transition: None,
                error: Some(format!("HTTP error: {}", response.status())),
            });
        }
        
        // Parse the full JSON response
        let json_response: serde_json::Value = response.json().await?;
        
        log::debug!("Raw JSON response: {}", serde_json::to_string_pretty(&json_response)?);
        
        // Extract the expr data from the response
        if let Some(expr_array) = json_response.get("expr").and_then(|v| v.as_array()) {
            if expr_array.is_empty() {
                return Ok(TransitionData {
                    success: false,
                    transition: None,
                    error: Some("No data returned from contract".to_string()),
                });
            }
            
            // Convert Rholang ExprMap to plain JSON
            let rholang_expr = &expr_array[0];
            log::debug!("Rholang expression: {}", serde_json::to_string_pretty(rholang_expr)?);
            
            let plain_json = convert_rholang_to_json(rholang_expr)?;
            log::debug!("Converted to plain JSON: {}", serde_json::to_string_pretty(&plain_json)?);
            
            // Try to parse as TransitionData
            match serde_json::from_value::<TransitionData>(plain_json.clone()) {
                Ok(transition_data) => Ok(transition_data),
                Err(e) => {
                    Ok(TransitionData {
                        success: false,
                        transition: None,
                        error: Some(format!("Failed to parse transition data: {}. Raw: {}", e, plain_json)),
                    })
                }
            }
        } else {
            Ok(TransitionData {
                success: false,
                transition: None,
                error: Some("Invalid response format".to_string()),
            })
        }
    }
}
