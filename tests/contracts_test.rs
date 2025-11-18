//! F1r3flyRgbContracts Integration Tests
//!
//! Tests the high-level multi-contract collection API with live F1r3node instance.
//! Focuses on contract management, collection operations, and state isolation.
//!
//! ## Test Isolation Strategy
//!
//! Each test uses a unique derivation index offset (derived from hashing the test name)
//! to ensure parallel tests don't interfere with each other's contracts on F1r3node.
//! This prevents state pollution when tests run concurrently.
//!
//! ## Multi-Contract Support
//!
//! Each `F1r3flyRgbContracts::issue()` call creates an **independent contract**:
//! - Master key (`FIREFLY_PRIVATE_KEY`) pays phlo for all deployments
//! - Shared executor preserves `derivation_index` across multiple `issue()` calls
//! - Each deployment increments `derivation_index` → Unique child key → Unique URI
//! - Collection properly manages multiple independent contracts
//!
//! Tests verify true multi-contract functionality with independent state.
//!
//! Requirements:
//! - Running f1r3node instance
//! - FIREFLY_* environment variables set
//!
//! Run with: cargo test --test contracts_test -- --nocapture

use f1r3fly_rgb::{F1r3flyExecutor, F1r3flyRgbContract, F1r3flyRgbContracts, Pile};
use hypersonic::ContractId;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Load environment variables from .env file
fn load_env() {
    use std::path::PathBuf;

    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push(".env");

    dotenv::from_path(&path).ok();
}

/// Generate a unique derivation index offset from test name
///
/// Uses hash-based derivation to ensure parallel tests don't collide on F1r3node:
/// - Each test name produces a unique offset in a large space (0 to u32::MAX)
/// - Tests can deploy multiple contracts (offset, offset+1, offset+2, ...)
/// - Extremely low probability of collision across different test names
fn test_derivation_offset(test_name: &str) -> u32 {
    let mut hasher = DefaultHasher::new();
    test_name.hash(&mut hasher);
    (hasher.finish() % (u32::MAX as u64)) as u32
}

// ============================================================================
// Test 1: Issue Multiple Contracts and Verify Collection State
// ============================================================================

#[tokio::test]
async fn test_contracts_issue_multiple_with_verification() {
    load_env();

    let mut executor = F1r3flyExecutor::new().expect("Failed to create F1r3flyExecutor");
    executor.set_derivation_index(test_derivation_offset(
        "test_contracts_issue_multiple_with_verification",
    ));

    let mut contracts = F1r3flyRgbContracts::new(executor);

    // Issue multiple tokens with unique tickers for test isolation
    let btc_id = contracts
        .issue("MBTC", "Multi Bitcoin", 21000000, 8)
        .await
        .expect("BTC issue failed");

    let eth_id = contracts
        .issue("METH", "Multi Ethereum", 100000000, 18)
        .await
        .expect("ETH issue failed");

    let usdt_id = contracts
        .issue("MUSDT", "Multi Tether", 1000000000, 6)
        .await
        .expect("USDT issue failed");

    // Verify contract IDs are unique
    assert_ne!(btc_id, eth_id, "BTC and ETH should have different IDs");
    assert_ne!(eth_id, usdt_id, "ETH and USDT should have different IDs");
    assert_ne!(btc_id, usdt_id, "BTC and USDT should have different IDs");

    // Verify collection count
    assert_eq!(contracts.count(), 3, "Should have exactly 3 contracts");

    // Verify all contracts are retrievable
    assert!(
        contracts.get(&btc_id).is_some(),
        "BTC contract should exist"
    );
    assert!(
        contracts.get(&eth_id).is_some(),
        "ETH contract should exist"
    );
    assert!(
        contracts.get(&usdt_id).is_some(),
        "USDT contract should exist"
    );

    // Verify contract metadata for each
    let btc = contracts.get(&btc_id).unwrap();
    assert!(
        !btc.metadata().registry_uri.is_empty(),
        "BTC should have registry URI"
    );

    let eth = contracts.get(&eth_id).unwrap();
    assert!(
        !eth.metadata().registry_uri.is_empty(),
        "ETH should have registry URI"
    );

    let usdt = contracts.get(&usdt_id).unwrap();
    assert!(
        !usdt.metadata().registry_uri.is_empty(),
        "USDT should have registry URI"
    );
}

// ============================================================================
// Test 2: Contract Lifecycle - Issue, Retrieve, and Mutate
// ============================================================================

#[tokio::test]
async fn test_contracts_get_and_mutate() {
    load_env();

    let mut executor = F1r3flyExecutor::new().expect("Failed to create F1r3flyExecutor");
    executor.set_derivation_index(test_derivation_offset("test_contracts_get_and_mutate"));

    let mut contracts = F1r3flyRgbContracts::new(executor);

    // Issue contract
    let id = contracts
        .issue("LTEST", "Lifecycle Test", 1000000, 8)
        .await
        .expect("Issue failed");

    // Test immutable access
    {
        let contract = contracts.get(&id).expect("Contract not found");
        assert_eq!(contract.contract_id(), id);
        assert!(!contract.metadata().registry_uri.is_empty());
        assert_eq!(contract.metadata().methods.len(), 4);
    }

    // Test mutable access
    {
        let contract_mut = contracts.get_mut(&id).expect("Contract not found");

        // Verify we can access mutable executor
        let _executor_mut = contract_mut.executor_mut();

        // Verify we can access mutable tracker
        let tracker_mut = contract_mut.tracker_mut();
        assert_eq!(
            tracker_mut.witness_ids().count(),
            0,
            "New contract should have no witnesses"
        );
    }

    // Verify immutable access still works after mutable borrow
    let contract = contracts.get(&id).expect("Contract should still exist");
    assert_eq!(contract.contract_id(), id);
}

// ============================================================================
// Test 3: List and Contains Operations with Edge Cases
// ============================================================================

#[tokio::test]
async fn test_contracts_list_and_contains_edge_cases() {
    load_env();

    let mut executor = F1r3flyExecutor::new().expect("Failed to create F1r3flyExecutor");
    executor.set_derivation_index(test_derivation_offset(
        "test_contracts_list_and_contains_edge_cases",
    ));

    let mut contracts = F1r3flyRgbContracts::new(executor);

    // Verify empty collection
    assert_eq!(contracts.count(), 0, "New collection should be empty");
    assert_eq!(
        contracts.list().len(),
        0,
        "Empty collection should have empty list"
    );

    // Issue contracts
    let id1 = contracts
        .issue("LIST1", "List Token 1", 1000, 2)
        .await
        .expect("Issue 1 failed");

    let id2 = contracts
        .issue("LIST2", "List Token 2", 2000, 4)
        .await
        .expect("Issue 2 failed");

    // Verify list contains both
    let list = contracts.list();
    assert_eq!(list.len(), 2, "List should contain 2 contracts");
    assert!(list.contains(&id1), "List should contain contract 1");
    assert!(list.contains(&id2), "List should contain contract 2");

    // Test contains method
    assert!(
        contracts.contains(&id1),
        "Collection should contain contract 1"
    );
    assert!(
        contracts.contains(&id2),
        "Collection should contain contract 2"
    );

    // Test non-existent contract
    let fake_id = ContractId::from([0u8; 32]);
    assert!(
        !contracts.contains(&fake_id),
        "Should not contain fake contract"
    );
    assert!(
        contracts.get(&fake_id).is_none(),
        "Get should return None for fake contract"
    );

    // Verify count matches list length
    assert_eq!(
        contracts.count(),
        list.len(),
        "Count should match list length"
    );
}

// ============================================================================
// Test 4: Register Existing Contract
// ============================================================================

#[tokio::test]
async fn test_contracts_register_existing_contract() {
    load_env();

    let mut executor = F1r3flyExecutor::new().expect("Failed to create F1r3flyExecutor");
    executor.set_derivation_index(test_derivation_offset(
        "test_contracts_register_existing_contract",
    ));

    // Create separate contract instance
    let standalone_contract = F1r3flyRgbContract::issue(
        executor.clone(),
        "REGTEST",
        "Register Test Token",
        5000000,
        6,
    )
    .await
    .expect("Failed to issue standalone contract");

    let contract_id = standalone_contract.contract_id();

    // Create contracts collection
    let mut contracts = F1r3flyRgbContracts::new(executor);

    // Verify contract doesn't exist yet
    assert!(
        !contracts.contains(&contract_id),
        "Contract should not exist before registration"
    );
    assert_eq!(contracts.count(), 0, "Collection should be empty");

    // Register the contract
    let registered_id = contracts.register(standalone_contract);

    // Verify registration
    assert_eq!(
        registered_id, contract_id,
        "Registered ID should match original"
    );
    assert!(
        contracts.contains(&contract_id),
        "Contract should exist after registration"
    );
    assert_eq!(contracts.count(), 1, "Collection should have 1 contract");

    // Verify we can retrieve and use the registered contract
    let retrieved = contracts
        .get(&contract_id)
        .expect("Should retrieve registered contract");
    assert_eq!(retrieved.contract_id(), contract_id);
    assert!(!retrieved.metadata().registry_uri.is_empty());

    // Register another contract
    let second_contract = F1r3flyRgbContract::issue(
        contracts.get(&contract_id).unwrap().executor().clone(),
        "REGTEST2",
        "Register Test 2",
        3000000,
        4,
    )
    .await
    .expect("Failed to issue second contract");

    let second_id = contracts.register(second_contract);

    // Verify both contracts exist
    assert_eq!(
        contracts.count(),
        2,
        "Should have 2 contracts after second registration"
    );
    assert!(
        contracts.contains(&contract_id),
        "First contract should still exist"
    );
    assert!(
        contracts.contains(&second_id),
        "Second contract should exist"
    );
}

// ============================================================================
// Test 5: Concurrent Operations and State Isolation
// ============================================================================

#[tokio::test]
async fn test_contracts_concurrent_operations_state_isolation() {
    load_env();

    let mut executor = F1r3flyExecutor::new().expect("Failed to create F1r3flyExecutor");
    executor.set_derivation_index(test_derivation_offset(
        "test_contracts_concurrent_operations_state_isolation",
    ));

    let mut contracts = F1r3flyRgbContracts::new(executor);

    // Issue multiple contracts rapidly
    let id1 = contracts
        .issue("CONC1", "Concurrent 1", 1000000, 8)
        .await
        .expect("Concurrent issue 1 failed");

    let id2 = contracts
        .issue("CONC2", "Concurrent 2", 2000000, 8)
        .await
        .expect("Concurrent issue 2 failed");

    let id3 = contracts
        .issue("CONC3", "Concurrent 3", 3000000, 8)
        .await
        .expect("Concurrent issue 3 failed");

    // Verify all contracts are independent
    assert_ne!(id1, id2, "Contract 1 and 2 should be different");
    assert_ne!(id2, id3, "Contract 2 and 3 should be different");
    assert_ne!(id1, id3, "Contract 1 and 3 should be different");

    // Verify all are accessible
    let c1 = contracts.get(&id1).expect("Contract 1 should exist");
    let c2 = contracts.get(&id2).expect("Contract 2 should exist");
    let c3 = contracts.get(&id3).expect("Contract 3 should exist");

    // Verify each has unique metadata
    assert_ne!(
        c1.metadata().registry_uri,
        c2.metadata().registry_uri,
        "Contracts should have different registry URIs"
    );
    assert_ne!(
        c2.metadata().registry_uri,
        c3.metadata().registry_uri,
        "Contracts should have different registry URIs"
    );

    // Verify list is complete and correct
    let list = contracts.list();
    assert_eq!(list.len(), 3, "Should have 3 contracts");
    assert!(list.contains(&id1), "List should contain contract 1");
    assert!(list.contains(&id2), "List should contain contract 2");
    assert!(list.contains(&id3), "List should contain contract 3");

    // Verify count matches
    assert_eq!(contracts.count(), 3, "Count should be 3");

    // Test retrieving non-existent contract between valid ones
    let fake_id = ContractId::from([0xFFu8; 32]);
    assert!(
        !contracts.contains(&fake_id),
        "Should not contain fake contract"
    );
    assert!(
        contracts.get(&fake_id).is_none(),
        "Get fake should return None"
    );

    // Verify collection is still intact after failed get
    assert_eq!(
        contracts.count(),
        3,
        "Count should still be 3 after failed get"
    );
    assert!(
        contracts.get(&id1).is_some(),
        "Contract 1 should still exist"
    );
    assert!(
        contracts.get(&id2).is_some(),
        "Contract 2 should still exist"
    );
    assert!(
        contracts.get(&id3).is_some(),
        "Contract 3 should still exist"
    );
}
