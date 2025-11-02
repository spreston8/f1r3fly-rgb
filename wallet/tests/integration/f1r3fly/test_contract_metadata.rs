// Contract Deployment & Metadata Storage Tests
//
// Tests for RGB contract metadata storage and retrieval on F1r3fly/RSpace.
// Covers:
// - Contract deployment via insertSigned
// - Metadata storage in TreeHashMap
// - Metadata retrieval via exploratory deploys
// - Secondary index (ticker search)
// - Error handling for non-existent tickers

use crate::f1r3fly_test_utils::*;

#[tokio::test]
async fn test_deploy_and_store_contract_metadata() {
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

    // Step 1: Ensure RGB storage contract is deployed (uses shared fixture)
    // This deploys once and caches URIs for all subsequent tests
    let uris = ensure_rgb_storage_deployed(&client)
        .await
        .expect("Failed to ensure RGB storage deployed");

    // Set URIs on the client so it can call RGB storage methods
    client.set_rgb_uris(uris.clone());

    // Step 2: Store contract metadata
    // This calls the registered RGBStorage contract:
    // 1. Registry lookup: rl!(`rho:id:...`, *rgbCh)
    // 2. Contract call: @RGBStorage!("storeContract", contractId, metadata, *ack)
    // 3. Contract stores data in its internal treeHashMap
    let contract_id = generate_test_contract_id("test_store_query");
    let metadata = sample_contract_metadata();

    let (_store_deploy_id, _store_block_hash) =
        store_test_contract(&client, &contract_id, &metadata)
            .await
            .expect("Failed to store contract metadata");

    // Step 3: Query contract metadata using exploratory deploy
    let contract_data = client
        .query_contract(&contract_id, None)
        .await
        .expect("Failed to query contract metadata");

    // Step 4: Verify all fields are preserved correctly
    assert!(
        contract_data.success,
        "Query should succeed: {:?}",
        contract_data.error
    );

    let contract = contract_data.contract.expect("Contract should be present");
    assert_eq!(contract.ticker, metadata.ticker, "Ticker should match");
    assert_eq!(contract.name, metadata.name, "Name should match");
    assert_eq!(
        contract.precision, metadata.precision,
        "Precision should match"
    );
    assert_eq!(
        contract.total_supply, metadata.total_supply,
        "Total supply should match"
    );
    assert_eq!(
        contract.genesis_txid, metadata.genesis_txid,
        "Genesis txid should match"
    );
    assert_eq!(
        contract.issuer, metadata.issuer_pubkey,
        "Issuer should match"
    );
}

#[tokio::test]
async fn test_search_contract_by_ticker() {
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

    // Step 2: Store multiple contracts with different tickers
    let btc_id = generate_test_contract_id("btc");
    let btc_metadata = sample_contract_with_ticker("BTC", "Bitcoin", 21_000_000);

    let eth_id = generate_test_contract_id("eth");
    let eth_metadata = sample_contract_with_ticker("ETH", "Ethereum", 120_000_000);

    let usdt_id = generate_test_contract_id("usdt");
    let usdt_metadata = sample_contract_with_ticker("USDT", "Tether USD", 1_000_000_000);

    // Store all three contracts
    store_test_contract(&client, &btc_id, &btc_metadata)
        .await
        .expect("Failed to store BTC contract");

    store_test_contract(&client, &eth_id, &eth_metadata)
        .await
        .expect("Failed to store ETH contract");

    store_test_contract(&client, &usdt_id, &usdt_metadata)
        .await
        .expect("Failed to store USDT contract");

    // Step 3: Search for ETH contract by ticker
    let eth_result = client
        .search_contract_by_ticker("ETH")
        .await
        .expect("Failed to search for ETH");

    // Step 4: Verify correct contract is returned
    assert!(
        eth_result.success,
        "Search should succeed: {:?}",
        eth_result.error
    );

    let eth_contract = eth_result.contract.expect("ETH contract should be present");
    assert_eq!(eth_contract.ticker, "ETH", "Ticker should be ETH");
    assert_eq!(eth_contract.name, "Ethereum", "Name should be Ethereum");
    assert_eq!(
        eth_contract.total_supply, 120_000_000,
        "Total supply should match"
    );

    // Step 5: Search for BTC contract by ticker
    let btc_result = client
        .search_contract_by_ticker("BTC")
        .await
        .expect("Failed to search for BTC");

    assert!(btc_result.success, "Search should succeed");
    let btc_contract = btc_result.contract.expect("BTC contract should be present");
    assert_eq!(btc_contract.ticker, "BTC", "Ticker should be BTC");
    assert_eq!(btc_contract.name, "Bitcoin", "Name should be Bitcoin");

    // Step 6: Search for non-existent ticker
    let nonexistent_result = client
        .search_contract_by_ticker("XYZ")
        .await
        .expect("Failed to search for XYZ");

    assert!(
        !nonexistent_result.success,
        "Search for non-existent ticker should fail"
    );
    assert!(
        nonexistent_result.contract.is_none(),
        "Non-existent ticker should return None"
    );
    assert!(
        nonexistent_result.error.is_some(),
        "Error message should be present"
    );
}

