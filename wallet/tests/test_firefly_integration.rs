// FireflyClient & Rholang Contract Integration Tests
// Tests for FireflyClient interactions with F1r3fly node and RSpace++ state storage
//
// Prerequisites:
// - F1r3fly node must be running locally
// - Configure via environment variables:
//   - FIREFLY_TEST_HOST (default: localhost)
//   - FIREFLY_TEST_GRPC_PORT (default: 40401)
//   - FIREFLY_TEST_HTTP_PORT (default: 40403)

mod firefly_common;
use firefly_common::*;

// ============================================================================
// Contract Deployment Tests
// ============================================================================

#[tokio::test]
async fn test_deploy_rgb_state_storage_contract() {
    // Initialize logging for test output
    let _ = env_logger::builder().is_test(true).try_init();
    
    // Load test configuration from environment
    let config = FireflyTestConfig::from_env();
    
    // Check if F1r3fly node is reachable
    check_firefly_node_health(&config).await
        .expect("F1r3fly test node is not running. Please start your F1r3fly node before running tests.");
    
    // Create FireflyClient
    let client = config.create_client();
    
    // Deploy RGB state storage contract
    println!("Deploying RGB state storage contract...");
    let (deploy_id, block_hash) = deploy_rgb_state_storage(&client).await
        .expect("Failed to deploy RGB state storage contract");
    
    // Verify deploy ID is returned
    assert!(!deploy_id.is_empty(), "Deploy ID should not be empty");
    println!("✓ Deploy ID: {}", deploy_id);
    
    // Verify block hash is returned (contract was included in a block)
    assert!(!block_hash.is_empty(), "Block hash should not be empty");
    println!("✓ Block hash: {}", block_hash);
    
    // Verify block hash format (should be hex string)
    assert!(block_hash.chars().all(|c| c.is_ascii_hexdigit()), 
        "Block hash should be a valid hex string");
    
    println!("✓ Contract deployment successful!");
}

#[tokio::test]
async fn test_deploy_malformed_rholang() {
    // Test: Deploy malformed Rholang code
    // Verify: Proper error handling and reporting
    todo!("Implement: Handle Rholang syntax errors");
}

// ============================================================================
// Contract Metadata Storage & Query Tests
// ============================================================================

#[tokio::test]
async fn test_store_and_query_contract_metadata() {
    // Test: Store contract metadata via StoreContract, query it back
    // 1. Deploy rgb_state_storage.rho
    // 2. Call StoreContract with metadata
    // 3. Query contract by ID
    // Verify: All fields preserved correctly (ticker, name, precision, etc.)
    todo!("Implement: Store and query contract metadata");
}

#[tokio::test]
async fn test_query_nonexistent_contract() {
    // Test: Query contract that doesn't exist
    // Verify: Returns appropriate error/empty result
    todo!("Implement: Query nonexistent contract");
}

#[tokio::test]
async fn test_search_contract_by_ticker() {
    // Test: Store multiple contracts, search by ticker
    // 1. Store contracts with different tickers
    // 2. Search by specific ticker
    // Verify: Correct contract is returned
    todo!("Implement: Search by ticker");
}

// ============================================================================
// Allocation Storage & Query Tests
// ============================================================================

#[tokio::test]
async fn test_store_and_query_allocation() {
    // Test: Store allocation via StoreAllocation, query it back
    // 1. Store allocation with UTXO, owner, amount
    // 2. Query allocation by contract ID and UTXO
    // Verify: All fields preserved correctly
    todo!("Implement: Store and query allocation");
}

#[tokio::test]
async fn test_query_nonexistent_allocation() {
    // Test: Query allocation that doesn't exist
    // Verify: Handles gracefully
    todo!("Implement: Query nonexistent allocation");
}

// ============================================================================
// Transition Storage & Query Tests
// ============================================================================

#[tokio::test]
async fn test_store_and_query_transition() {
    // Test: Store state transition via RecordTransition, query it back
    // 1. Store initial allocation
    // 2. Record transition (from_utxo -> to_utxo)
    // 3. Query transition history
    // Verify: Transition recorded correctly with all fields
    todo!("Implement: Store and query transition");
}

// ============================================================================
// RSpace++ State Query Tests
// ============================================================================

#[tokio::test]
async fn test_query_rspace_state() {
    // Test: Query RSpace++ state via HTTP API
    // 1. Store data in RSpace++ channel
    // 2. Query state using explore-deploy
    // Verify: Returns expected data format
    todo!("Implement: Query RSpace++ state");
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[tokio::test]
async fn test_firefly_connection_failure() {
    // Test: FireflyClient with invalid host/port
    // Verify: Proper error handling (gRPC and HTTP)
    todo!("Implement: Handle connection failure");
}

#[tokio::test]
async fn test_firefly_timeout() {
    // Test: Long-running query with timeout
    // Verify: Timeout handled gracefully
    todo!("Implement: Handle timeout");
}

#[tokio::test]
async fn test_malformed_firefly_response() {
    // Test: F1r3fly returns malformed JSON
    // Verify: Parse error handled gracefully
    todo!("Implement: Handle malformed responses");
}


