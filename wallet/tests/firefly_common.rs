/// F1r3fly Test Utilities
/// 
/// This module provides shared test infrastructure for F1r3fly integration tests:
/// - F1r3fly node configuration (separate test node recommended)
/// - FireflyClient setup with test config
/// - Rholang contract deployment helpers
/// - RSpace++ state query helpers
/// - Test data fixtures

use wallet::firefly::client::FireflyClient;
use wallet::firefly::types::*;
use std::time::Duration;

/// F1r3fly test configuration
#[derive(Debug, Clone)]
pub struct FireflyTestConfig {
    pub host: String,
    pub grpc_port: u16,
    pub http_port: u16,
}

impl FireflyTestConfig {
    /// Load F1r3fly configuration from environment
    /// 
    /// Environment variables:
    /// - FIREFLY_TEST_HOST (default: localhost)
    /// - FIREFLY_TEST_GRPC_PORT (default: 40401)
    /// - FIREFLY_TEST_HTTP_PORT (default: 40403)
    pub fn from_env() -> Self {
        let host = std::env::var("FIREFLY_TEST_HOST")
            .unwrap_or_else(|_| "localhost".to_string());
        
        let grpc_port = std::env::var("FIREFLY_TEST_GRPC_PORT")
            .unwrap_or_else(|_| "40401".to_string())
            .parse()
            .expect("Invalid FIREFLY_TEST_GRPC_PORT");
        
        let http_port = std::env::var("FIREFLY_TEST_HTTP_PORT")
            .unwrap_or_else(|_| "40403".to_string())
            .parse()
            .expect("Invalid FIREFLY_TEST_HTTP_PORT");
        
        log::info!("🔥 F1r3fly Test Node: {}:{} (gRPC), :{} (HTTP)", 
            host, grpc_port, http_port);
        
        Self {
            host,
            grpc_port,
            http_port,
        }
    }
    
    /// Create a FireflyClient for testing
    pub fn create_client(&self) -> FireflyClient {
        FireflyClient::new(
            &self.host,
            self.grpc_port,
            self.http_port,
        )
    }
}

/// Test helper: Deploy Rholang contract and wait for confirmation
pub async fn deploy_and_wait(
    client: &FireflyClient,
    rholang_code: &str,
    max_attempts: u32,
) -> anyhow::Result<(String, String)> {
    log::info!("📤 Deploying Rholang contract...");
    
    let deploy_id = client.deploy(rholang_code).await
        .map_err(|e| anyhow::anyhow!("Deploy failed: {}", e))?;
    
    log::info!("✓ Deploy ID: {}", deploy_id);
    
    log::info!("⏳ Waiting for deploy to be included in block...");
    let block_hash = client.wait_for_deploy(&deploy_id, max_attempts).await
        .map_err(|e| anyhow::anyhow!("Wait for deploy failed: {}", e))?;
    
    log::info!("✓ Confirmed in block: {}", block_hash);
    
    Ok((deploy_id, block_hash))
}

/// Test helper: Deploy rgb_state_storage.rho contract
pub async fn deploy_rgb_state_storage(
    client: &FireflyClient,
) -> anyhow::Result<(String, String)> {
    let contract_code = include_str!("../rholang/rgb_state_storage.rho");
    deploy_and_wait(client, contract_code, 10).await
}

/// Test helper: Query RSpace++ state with retry logic
pub async fn query_state_with_retry(
    client: &FireflyClient,
    query_code: &str,
    max_retries: u32,
    delay_ms: u64,
) -> anyhow::Result<String> {
    for attempt in 1..=max_retries {
        match client.query_state(query_code).await {
            Ok(result) => {
                log::debug!("✓ Query succeeded on attempt {}", attempt);
                return Ok(result);
            }
            Err(e) if attempt < max_retries => {
                log::debug!("Query attempt {} failed: {}, retrying in {}ms...", 
                    attempt, e, delay_ms);
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Query failed after {} attempts: {}", 
                    max_retries, e));
            }
        }
    }
    unreachable!()
}

/// Test fixture: Sample contract metadata
pub fn sample_contract_metadata() -> ContractMetadata {
    ContractMetadata {
        ticker: "TEST".to_string(),
        name: "Test Token".to_string(),
        precision: 8,
        total_supply: 1_000_000,
        genesis_txid: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
        issuer_pubkey: "test_issuer_pubkey".to_string(),
    }
}

/// Test fixture: Sample allocation (for store_allocation parameters)
pub struct TestAllocation {
    pub contract_id: String,
    pub utxo: String,
    pub owner_pubkey: String,
    pub amount: u64,
    pub bitcoin_txid: String,
}

pub fn sample_allocation(contract_id: &str) -> TestAllocation {
    TestAllocation {
        contract_id: contract_id.to_string(),
        utxo: "0000000000000000000000000000000000000000000000000000000000000000:0".to_string(),
        owner_pubkey: "test_owner_pubkey".to_string(),
        amount: 100_000,
        bitcoin_txid: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
    }
}

/// Test fixture: Sample transition (for future transition storage)
pub struct TestTransition {
    pub contract_id: String,
    pub from_utxo: String,
    pub to_utxo: String,
    pub amount: u64,
    pub bitcoin_txid: String,
}

pub fn sample_transition(contract_id: &str) -> TestTransition {
    TestTransition {
        contract_id: contract_id.to_string(),
        from_utxo: "0000000000000000000000000000000000000000000000000000000000000000:0".to_string(),
        to_utxo: "1111111111111111111111111111111111111111111111111111111111111111:0".to_string(),
        amount: 50_000,
        bitcoin_txid: "1111111111111111111111111111111111111111111111111111111111111111".to_string(),
    }
}

/// Generate unique contract ID for testing
pub fn generate_test_contract_id(test_name: &str) -> String {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    format!("rgb20_test_{}_{}", test_name, timestamp)
}

/// Pretty-print RSpace++ query result for debugging
pub fn print_rspace_result(label: &str, result: &str) {
    log::info!("📊 {}", label);
    
    // Try to parse as JSON for pretty printing
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(result) {
        log::info!("{}", serde_json::to_string_pretty(&json).unwrap_or(result.to_string()));
    } else {
        log::info!("{}", result);
    }
}

/// Test helper: Store contract metadata via FireflyClient
pub async fn store_test_contract(
    client: &FireflyClient,
    contract_id: &str,
    metadata: &ContractMetadata,
) -> anyhow::Result<String> {
    log::info!("💾 Storing contract: {}", contract_id);
    
    let deploy_id = client.store_contract(
        contract_id,
        metadata.clone(),
    ).await
        .map_err(|e| anyhow::anyhow!("Store contract failed: {}", e))?;
    
    log::info!("✓ Contract stored with deploy ID: {}", deploy_id);
    Ok(deploy_id)
}

/// Test helper: Store allocation via FireflyClient
pub async fn store_test_allocation(
    client: &FireflyClient,
    allocation: &TestAllocation,
) -> anyhow::Result<String> {
    log::info!("💾 Storing allocation: {} @ {}", 
        allocation.amount, allocation.utxo);
    
    let deploy_id = client.store_allocation(
        &allocation.contract_id,
        &allocation.utxo,
        &allocation.owner_pubkey,
        allocation.amount,
        &allocation.bitcoin_txid,
    ).await
        .map_err(|e| anyhow::anyhow!("Store allocation failed: {}", e))?;
    
    log::info!("✓ Allocation stored with deploy ID: {}", deploy_id);
    Ok(deploy_id)
}

/// Test helper: Query contract metadata
pub async fn query_test_contract(
    client: &FireflyClient,
    contract_id: &str,
) -> anyhow::Result<ContractData> {
    log::info!("🔍 Querying contract: {}", contract_id);
    
    let result = client.query_contract(contract_id).await
        .map_err(|e| anyhow::anyhow!("Query contract failed: {}", e))?;
    
    if let Some(ref contract) = result.contract {
        log::info!("✓ Contract found: {} ({})", contract.name, contract.ticker);
    } else {
        log::info!("✗ Contract not found");
    }
    
    Ok(result)
}

/// Test helper: Query allocation
pub async fn query_test_allocation(
    client: &FireflyClient,
    contract_id: &str,
    utxo: &str,
) -> anyhow::Result<AllocationData> {
    log::info!("🔍 Querying allocation: {} @ {}", contract_id, utxo);
    
    let result = client.query_allocation(contract_id, utxo).await
        .map_err(|e| anyhow::anyhow!("Query allocation failed: {}", e))?;
    
    if let Some(ref allocation) = result.allocation {
        log::info!("✓ Allocation found: {} tokens", allocation.amount);
    } else {
        log::info!("✗ Allocation not found");
    }
    
    Ok(result)
}

/// Check if F1r3fly node is reachable
pub async fn check_firefly_node_health(config: &FireflyTestConfig) -> anyhow::Result<()> {
    log::info!("🏥 Checking F1r3fly node health...");
    
    // Check F1r3fly node status endpoint
    let url = format!("http://{}:{}/api/status", config.host, config.http_port);
    
    let response = tokio::time::timeout(
        Duration::from_secs(5),
        reqwest::get(&url)
    ).await
        .map_err(|_| anyhow::anyhow!("F1r3fly node health check timeout"))?
        .map_err(|e| anyhow::anyhow!("F1r3fly node unreachable: {}", e))?;
    
    if response.status().is_success() {
        log::info!("✓ F1r3fly node is healthy");
        Ok(())
    } else {
        Err(anyhow::anyhow!("F1r3fly node returned status: {}", response.status()))
    }
}


