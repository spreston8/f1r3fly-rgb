// Allocation Storage & Query Tests
//
// Tests for RGB allocation (UTXO ownership) storage and retrieval on F1r3fly/RSpace.
// Covers:
// - Allocation storage (UTXO → owner → amount)
// - Allocation retrieval
// - Error handling (5 scenarios):
//   - Valid contract, non-existent allocation
//   - Non-existent contract
//   - Empty UTXO
//   - Malformed UTXO
//   - Very long UTXO

use crate::f1r3fly_test_utils::*;

#[tokio::test]
async fn test_store_and_query_allocation() {
    // Load environment variables from .env file
    dotenv::dotenv().ok();

    // Initialize logging
    let _ = env_logger::builder().is_test(true).try_init();

    // Load test configuration
    let config = FireflyTestConfig::from_env();

    // Check if F1r3fly node is reachable
    check_firefly_node_health(&config).await.expect(
        "F1r3fly test node is not running. Please start your F1r3fly node before running tests.",
    );

    // Create FireflyClient
    let mut client = config.create_client();

    // Step 1: Ensure RGB storage contract is deployed
    let uris = ensure_rgb_storage_deployed(&client)
        .await
        .expect("Failed to ensure RGB storage deployed");

    // Set URIs on the client
    client.set_rgb_uris(uris.clone());

    // Step 2: Store a contract (prerequisite for allocations)
    let contract_id = generate_test_contract_id("test_allocation");
    let metadata = sample_contract_metadata();

    let (_contract_deploy_id, _contract_block_hash) =
        store_test_contract(&client, &contract_id, &metadata)
            .await
            .expect("Failed to store contract metadata");

    // Step 3: Store an allocation
    let (utxo, owner_pubkey, amount) = sample_allocation_data();
    let bitcoin_txid =
        "1111111111111111111111111111111111111111111111111111111111111111".to_string();

    let (_alloc_deploy_id, _alloc_block_hash) = store_test_allocation(
        &client,
        &contract_id,
        &utxo,
        &owner_pubkey,
        amount,
        &bitcoin_txid,
    )
    .await
    .expect("Failed to store allocation");

    // Step 4: Query the allocation
    let allocation_data = client
        .query_allocation(&contract_id, &utxo)
        .await
        .expect("Failed to query allocation");

    // Step 5: Verify all fields are preserved correctly
    assert!(
        allocation_data.success,
        "Query should succeed: {:?}",
        allocation_data.error
    );

    let allocation = allocation_data
        .allocation
        .expect("Allocation should be present");
    assert_eq!(allocation.owner, owner_pubkey, "Owner should match");
    assert_eq!(allocation.amount, amount, "Amount should match");
    assert_eq!(
        allocation.bitcoin_txid, bitcoin_txid,
        "Bitcoin txid should match"
    );
    assert_eq!(allocation.confirmed, true, "Confirmed should be true");
    assert!(allocation.created_at > 0, "Created_at should be set");
}

#[tokio::test]
async fn test_query_nonexistent_allocation() {
    // Load environment variables from .env file
    dotenv::dotenv().ok();

    // Initialize logging
    let _ = env_logger::builder().is_test(true).try_init();

    // Load test configuration
    let config = FireflyTestConfig::from_env();

    // Check if F1r3fly node is reachable
    check_firefly_node_health(&config).await.expect(
        "F1r3fly test node is not running. Please start your F1r3fly node before running tests.",
    );

    // Create FireflyClient
    let mut client = config.create_client();

    // Step 1: Ensure RGB storage contract is deployed
    let uris = ensure_rgb_storage_deployed(&client)
        .await
        .expect("Failed to ensure RGB storage deployed");

    // Set URIs on the client
    client.set_rgb_uris(uris.clone());

    // Step 2: Create a valid contract (makes test realistic)
    let contract_id = generate_test_contract_id("nonexistent_alloc");
    let metadata = sample_contract_metadata();

    store_test_contract(&client, &contract_id, &metadata)
        .await
        .expect("Failed to store contract");

    // Scenario 1: Valid contract, non-existent allocation
    let result1 = client
        .query_allocation(&contract_id, "nonexistent_utxo:0")
        .await
        .expect("Query should not error");

    assert!(
        !result1.success,
        "Query for non-existent allocation should fail"
    );
    assert!(result1.allocation.is_none(), "Allocation should be None");
    assert!(result1.error.is_some(), "Error message should be present");
    let error_msg = result1.error.unwrap();
    assert!(
        error_msg.contains("not found") || error_msg.contains("Not found"),
        "Error should mention 'not found', got: {}",
        error_msg
    );

    // Scenario 2: Non-existent contract, non-existent allocation
    let fake_contract_id = "fake_contract_id_that_does_not_exist";
    let result2 = client
        .query_allocation(fake_contract_id, "nonexistent_utxo:0")
        .await
        .expect("Query should not error");

    assert!(
        !result2.success,
        "Query for non-existent contract should fail"
    );
    assert!(result2.allocation.is_none(), "Allocation should be None");
    assert!(result2.error.is_some(), "Error message should be present");

    // Scenario 3: Empty UTXO string
    let result3 = client
        .query_allocation(&contract_id, "")
        .await
        .expect("Query should not error");

    assert!(!result3.success, "Query with empty UTXO should fail");
    assert!(result3.allocation.is_none(), "Allocation should be None");
    assert!(result3.error.is_some(), "Error message should be present");

    // Scenario 4: Malformed UTXO (no colon separator)
    let result4 = client
        .query_allocation(&contract_id, "malformed_utxo_without_colon")
        .await
        .expect("Query should not error");

    assert!(!result4.success, "Query with malformed UTXO should fail");
    assert!(result4.allocation.is_none(), "Allocation should be None");
    assert!(result4.error.is_some(), "Error message should be present");

    // Scenario 5: Very long UTXO string (edge case)
    let long_utxo = "a".repeat(1000);
    let result5 = client
        .query_allocation(&contract_id, &long_utxo)
        .await
        .expect("Query should not error");

    assert!(!result5.success, "Query with very long UTXO should fail");
    assert!(result5.allocation.is_none(), "Allocation should be None");
    assert!(result5.error.is_some(), "Error message should be present");
}

