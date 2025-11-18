//! F1r3flyExecutor Integration Tests - Category 3
//!
//! Tests the F1r3flyExecutor with live F1r3node instance.
//! Uses Pattern B (persistent contracts with insertSigned) and HTTP queries.
//!
//! ## Test Isolation Strategy
//!
//! Each test uses a unique derivation index offset (derived from hashing the test name)
//! to ensure parallel tests don't interfere with each other's contracts on F1r3node.
//! This prevents state pollution when tests run concurrently.
//!
//! Requirements:
//! - Running f1r3node instance
//! - FIREFLY_* environment variables set
//!
//! Run with: cargo test --test f1r3fly_executor_test -- --nocapture

use f1r3fly_rgb::StrictVal;
use f1r3fly_rgb::{ContractId, F1r3flyExecutor, RholangContractLibrary};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Load environment variables from .env file and initialize logging
fn load_env() {
    use std::path::PathBuf;

    // Initialize logger (only once, subsequent calls are no-ops)
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::Info)
        .try_init();

    // Load from .env
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
///
/// Example:
/// - "test_executor_deploy" → offset 1234567890
/// - "test_method_call" → offset 987654321
fn test_derivation_offset(test_name: &str) -> u32 {
    let mut hasher = DefaultHasher::new();
    test_name.hash(&mut hasher);
    // Use modulo to keep in reasonable range while maintaining uniqueness
    (hasher.finish() % (u32::MAX as u64)) as u32
}

/// Deploy a fresh test contract with test-specific derivation offset
///
/// ## Multi-Contract Support with Hash-Based Key Derivation
///
/// Each test gets a unique derivation index offset to prevent parallel test interference:
/// - Test name is hashed to produce a unique base offset (e.g., 1234567890)
/// - Executor starts at that offset and auto-increments for multiple contracts
/// - Different offset → Different child keys → Different registry URIs → Isolated contracts
///
/// **Key Signing Flow:**
/// 1. Master key signs gRPC deployment → Pays phlo from master's REV vault
/// 2. Child key (derived at test offset) signs `insertSigned` → Unique registry URI
/// 3. F1r3node verifies both signatures → Registers contract at isolated URI
///
/// **Test Isolation:**
/// Each test operates on completely independent contracts on F1r3node, preventing
/// state pollution when tests run in parallel.
async fn deploy_test_contract(executor: &mut F1r3flyExecutor) -> ContractId {
    let template = RholangContractLibrary::rho20_contract();

    executor
        .deploy_contract(
            template,
            "TEST",
            "Test Token",
            1_000_000,
            8,
            vec![
                "issue".to_string(),
                "transfer".to_string(),
                "balanceOf".to_string(),
            ],
        )
        .await
        .expect("Deploy failed")
}

// ============================================================================
// Category 3: F1r3flyExecutor Integration Tests
// ============================================================================

#[tokio::test]
async fn test_executor_deploy_contract() {
    load_env();

    let mut executor = F1r3flyExecutor::new().expect("Failed to create F1r3flyExecutor");
    executor.set_derivation_index(test_derivation_offset("test_executor_deploy_contract"));

    // Get the RHO20 template
    let template = RholangContractLibrary::rho20_contract();

    // Deploy contract with template substitution
    // Uses FIREFLY_PRIVATE_KEY from the executor's connection
    let contract_id = executor
        .deploy_contract(
            template,
            "DEPLOY",
            "Deploy Test Token",
            500_000,
            6,
            vec![
                "issue".to_string(),
                "transfer".to_string(),
                "balanceOf".to_string(),
            ],
        )
        .await
        .expect("Deploy failed");

    // Verify contract ID is not empty
    assert!(
        !contract_id.to_string().is_empty(),
        "Contract ID should not be empty"
    );

    // Debug: Print the contract ID
    println!("📋 Contract ID: {}", contract_id.to_string());

    // Verify contract ID is 32 bytes (checked via raw bytes, not string encoding)
    assert_eq!(
        contract_id.to_byte_array().len(),
        32,
        "Contract ID should be exactly 32 bytes"
    );
}

#[tokio::test]
async fn test_executor_call_method() {
    load_env();

    let mut executor = F1r3flyExecutor::new().expect("Failed to create F1r3flyExecutor");
    executor.set_derivation_index(test_derivation_offset("test_executor_call_method"));

    let contract_id = deploy_test_contract(&mut executor).await;

    // Step 1: Issue tokens to alice
    let alice = "alice_method_test";
    let bob = "bob_method_test";
    let initial_amount = 1000;
    let transfer_amount = 300;

    executor
        .call_method(
            contract_id,
            "issue",
            &[
                ("recipient", StrictVal::from(alice.to_string())),
                ("amount", StrictVal::from(initial_amount)),
            ],
        )
        .await
        .expect("Issue failed");

    // Step 2: Transfer tokens from alice to bob
    let deploy_info = executor
        .call_method(
            contract_id,
            "transfer",
            &[
                ("from", StrictVal::from(alice.to_string())),
                ("to", StrictVal::from(bob.to_string())),
                ("amount", StrictVal::from(transfer_amount)),
            ],
        )
        .await
        .expect("Method call failed");

    // Verify deploy info structure
    assert!(
        !deploy_info.deploy_id.is_empty(),
        "Deploy ID should not be empty"
    );
    assert!(
        !deploy_info.finalized_block_hash.is_empty(),
        "Finalized block hash should not be empty"
    );
    assert!(
        !deploy_info.rholang_source.is_empty(),
        "Rholang source should not be empty"
    );

    // Step 3: Verify alice's balance decreased
    let alice_balance = executor
        .query_state(
            contract_id,
            "balanceOf",
            &[("address", StrictVal::from(alice.to_string()))],
        )
        .await
        .expect("Query alice failed");

    let alice_value = alice_balance
        .as_u64()
        .or_else(|| alice_balance.as_i64().map(|i| i as u64))
        .expect("Alice balance should be a number");

    assert_eq!(
        alice_value,
        initial_amount - transfer_amount,
        "Alice should have {} tokens after transfer, got {}",
        initial_amount - transfer_amount,
        alice_value
    );

    // Step 4: Verify bob's balance increased
    let bob_balance = executor
        .query_state(
            contract_id,
            "balanceOf",
            &[("address", StrictVal::from(bob.to_string()))],
        )
        .await
        .expect("Query bob failed");

    let bob_value = bob_balance
        .as_u64()
        .or_else(|| bob_balance.as_i64().map(|i| i as u64))
        .expect("Bob balance should be a number");

    assert_eq!(
        bob_value, transfer_amount,
        "Bob should have {} tokens after transfer, got {}",
        transfer_amount, bob_value
    );
}

#[tokio::test]
async fn test_executor_query_state() {
    load_env();

    let mut executor = F1r3flyExecutor::new().expect("Failed to create F1r3flyExecutor");
    executor.set_derivation_index(test_derivation_offset("test_executor_query_state"));

    let contract_id = deploy_test_contract(&mut executor).await;

    // Step 1: Issue tokens to alice
    let alice = "alice_address";
    let issue_amount = 1000;

    let issue_params = vec![
        ("recipient", StrictVal::from(alice.to_string())),
        ("amount", StrictVal::from(issue_amount)),
    ];

    let issue_result = executor
        .call_method(contract_id, "issue", &issue_params)
        .await
        .expect("Issue failed");
    println!(
        "📊 DEBUG - Issue result deploy_id: {}",
        hex::encode(&issue_result.deploy_id)
    );
    println!(
        "📊 DEBUG - Issue result block: {}",
        hex::encode(&issue_result.finalized_block_hash)
    );

    // Step 2: Query balance for alice
    // Note: deploy_and_wait() ensures the deploy is finalized before returning
    let balance_params = vec![("address", StrictVal::from(alice.to_string()))];

    let balance = executor
        .query_state(contract_id, "balanceOf", &balance_params)
        .await
        .expect("Query failed");

    // DEBUG: Print the full response structure
    println!("📊 DEBUG - Full balance response: {:#?}", balance);

    // Step 3: Verify exact balance
    assert!(
        balance.is_number(),
        "Balance should be a number, got: {:?}",
        balance
    );

    let balance_value = balance
        .as_u64()
        .or_else(|| balance.as_i64().map(|i| i as u64))
        .expect("Balance should be convertible to u64");

    assert_eq!(
        balance_value, issue_amount,
        "Balance should be {} for alice after issue, got {}",
        issue_amount, balance_value
    );

    // Step 4: Query balance for non-existent address (should be 0)
    let bob = "bob_address";
    let bob_params = vec![("address", StrictVal::from(bob.to_string()))];

    let bob_balance = executor
        .query_state(contract_id, "balanceOf", &bob_params)
        .await
        .expect("Query failed for bob");

    assert!(
        bob_balance.is_number(),
        "Bob's balance should be a number, got: {:?}",
        bob_balance
    );

    let bob_value = bob_balance
        .as_u64()
        .or_else(|| bob_balance.as_i64().map(|i| i as u64))
        .unwrap_or(0);

    assert_eq!(
        bob_value, 0,
        "Balance should be 0 for non-existent address, got {}",
        bob_value
    );
}

#[tokio::test]
async fn test_executor_invalid_method_error() {
    load_env();

    let mut executor = F1r3flyExecutor::new().expect("Failed to create F1r3flyExecutor");
    executor.set_derivation_index(test_derivation_offset("test_executor_invalid_method_error"));

    let contract_id = deploy_test_contract(&mut executor).await;

    // Try to call a method that doesn't exist
    let result = executor
        .call_method(contract_id, "nonexistent_method", &[])
        .await;

    // Verify it returns an error
    assert!(
        result.is_err(),
        "Calling non-existent method should return an error"
    );

    // Verify the error is InvalidMethod
    let error = result.unwrap_err();
    let error_msg = format!("{:?}", error);
    assert!(
        error_msg.contains("InvalidMethod")
            || error_msg.contains("not found")
            || error_msg.contains("Method"),
        "Error should indicate invalid method, got: {}",
        error_msg
    );
}

#[tokio::test]
async fn test_executor_contract_not_found_error() {
    load_env();

    let mut executor = F1r3flyExecutor::new().expect("Failed to create F1r3flyExecutor");
    executor.set_derivation_index(test_derivation_offset(
        "test_executor_contract_not_found_error",
    ));

    // Create a fake contract ID that doesn't exist
    let fake_id = ContractId::from([0u8; 32]);

    // Try to call a method on the non-existent contract
    let result = executor.call_method(fake_id, "transfer", &[]).await;

    // Verify it returns an error
    assert!(
        result.is_err(),
        "Calling method on non-existent contract should return an error"
    );

    // Verify the error is ContractNotFound
    let error = result.unwrap_err();
    let error_msg = format!("{:?}", error);
    assert!(
        error_msg.contains("ContractNotFound")
            || error_msg.contains("not found")
            || error_msg.contains("Contract"),
        "Error should indicate contract not found, got: {}",
        error_msg
    );
}

#[tokio::test]
async fn test_executor_multiple_method_calls() {
    load_env();

    let mut executor = F1r3flyExecutor::new().expect("Failed to create F1r3flyExecutor");
    executor.set_derivation_index(test_derivation_offset(
        "test_executor_multiple_method_calls",
    ));

    let contract_id = deploy_test_contract(&mut executor).await;

    let alice = "alice_multi";
    let bob = "bob_multi";
    let charlie = "charlie_multi";

    // Step 1: Issue tokens to alice
    let issue_amount = 5000;
    let issue_result = executor
        .call_method(
            contract_id,
            "issue",
            &[
                ("recipient", StrictVal::from(alice.to_string())),
                ("amount", StrictVal::from(issue_amount)),
            ],
        )
        .await
        .expect("Issue call failed");

    assert!(
        !issue_result.deploy_id.is_empty(),
        "Issue deploy ID should not be empty"
    );

    // Step 2: Transfer from alice to bob
    let transfer_to_bob = 1500;
    let transfer_result = executor
        .call_method(
            contract_id,
            "transfer",
            &[
                ("from", StrictVal::from(alice.to_string())),
                ("to", StrictVal::from(bob.to_string())),
                ("amount", StrictVal::from(transfer_to_bob)),
            ],
        )
        .await
        .expect("Transfer call failed");

    assert!(
        !transfer_result.deploy_id.is_empty(),
        "Transfer deploy ID should not be empty"
    );

    // Verify deploy IDs are different (different operations)
    assert_ne!(
        issue_result.deploy_id, transfer_result.deploy_id,
        "Different method calls should have different deploy IDs"
    );

    // Step 3: Transfer from bob to charlie
    let transfer_to_charlie = 500;
    executor
        .call_method(
            contract_id,
            "transfer",
            &[
                ("from", StrictVal::from(bob.to_string())),
                ("to", StrictVal::from(charlie.to_string())),
                ("amount", StrictVal::from(transfer_to_charlie)),
            ],
        )
        .await
        .expect("Second transfer failed");

    // Step 4: Verify all balances are correct
    let alice_balance = executor
        .query_state(
            contract_id,
            "balanceOf",
            &[("address", StrictVal::from(alice.to_string()))],
        )
        .await
        .expect("Query alice failed");

    let alice_value = alice_balance
        .as_u64()
        .or_else(|| alice_balance.as_i64().map(|i| i as u64))
        .expect("Alice balance should be a number");

    assert_eq!(
        alice_value,
        issue_amount - transfer_to_bob,
        "Alice balance incorrect after transfers"
    );

    let bob_balance = executor
        .query_state(
            contract_id,
            "balanceOf",
            &[("address", StrictVal::from(bob.to_string()))],
        )
        .await
        .expect("Query bob failed");

    let bob_value = bob_balance
        .as_u64()
        .or_else(|| bob_balance.as_i64().map(|i| i as u64))
        .expect("Bob balance should be a number");

    assert_eq!(
        bob_value,
        transfer_to_bob - transfer_to_charlie,
        "Bob balance incorrect after transfers"
    );

    let charlie_balance = executor
        .query_state(
            contract_id,
            "balanceOf",
            &[("address", StrictVal::from(charlie.to_string()))],
        )
        .await
        .expect("Query charlie failed");

    let charlie_value = charlie_balance
        .as_u64()
        .or_else(|| charlie_balance.as_i64().map(|i| i as u64))
        .expect("Charlie balance should be a number");

    assert_eq!(
        charlie_value, transfer_to_charlie,
        "Charlie balance incorrect after transfers"
    );
}

#[tokio::test]
async fn test_executor_query_after_method_call() {
    load_env();

    let mut executor = F1r3flyExecutor::new().expect("Failed to create F1r3flyExecutor");
    executor.set_derivation_index(test_derivation_offset(
        "test_executor_query_after_method_call",
    ));

    let contract_id = deploy_test_contract(&mut executor).await;

    let alice = "alice_query_test";
    let charlie = "charlie_query_test";
    let initial_amount = 2000;
    let transfer_amount = 750;

    // Step 1: Issue tokens to alice
    executor
        .call_method(
            contract_id,
            "issue",
            &[
                ("recipient", StrictVal::from(alice.to_string())),
                ("amount", StrictVal::from(initial_amount)),
            ],
        )
        .await
        .expect("Issue failed");

    // Step 2: Query alice's initial balance
    let alice_before = executor
        .query_state(
            contract_id,
            "balanceOf",
            &[("address", StrictVal::from(alice.to_string()))],
        )
        .await
        .expect("Query alice before failed");

    let alice_before_value = alice_before
        .as_u64()
        .or_else(|| alice_before.as_i64().map(|i| i as u64))
        .expect("Alice balance before should be a number");

    assert_eq!(
        alice_before_value, initial_amount,
        "Alice should have {} tokens before transfer",
        initial_amount
    );

    // Step 3: Transfer from alice to charlie
    executor
        .call_method(
            contract_id,
            "transfer",
            &[
                ("from", StrictVal::from(alice.to_string())),
                ("to", StrictVal::from(charlie.to_string())),
                ("amount", StrictVal::from(transfer_amount)),
            ],
        )
        .await
        .expect("Transfer call failed");

    // Step 4: Query alice's balance after transfer
    let alice_after = executor
        .query_state(
            contract_id,
            "balanceOf",
            &[("address", StrictVal::from(alice.to_string()))],
        )
        .await
        .expect("Query alice after failed");

    let alice_after_value = alice_after
        .as_u64()
        .or_else(|| alice_after.as_i64().map(|i| i as u64))
        .expect("Alice balance after should be a number");

    assert_eq!(
        alice_after_value,
        initial_amount - transfer_amount,
        "Alice should have {} tokens after transfer, got {}",
        initial_amount - transfer_amount,
        alice_after_value
    );

    // Step 5: Query charlie's balance
    let charlie_balance = executor
        .query_state(
            contract_id,
            "balanceOf",
            &[("address", StrictVal::from(charlie.to_string()))],
        )
        .await
        .expect("Query charlie failed");

    let charlie_value = charlie_balance
        .as_u64()
        .or_else(|| charlie_balance.as_i64().map(|i| i as u64))
        .expect("Charlie balance should be a number");

    assert_eq!(
        charlie_value, transfer_amount,
        "Charlie should have {} tokens after transfer, got {}",
        transfer_amount, charlie_value
    );
}

#[tokio::test]
async fn test_executor_caching_works() {
    load_env();

    let mut executor = F1r3flyExecutor::new().expect("Failed to create F1r3flyExecutor");
    executor.set_derivation_index(test_derivation_offset("test_executor_caching_works"));

    // Disable auto-derive to test contract upgrade behavior
    // This keeps the derivation_index at the test's offset, producing the same URI for both deployments
    executor.set_auto_derive(false);

    // Deploy first contract
    let contract_id_1 = deploy_test_contract(&mut executor).await;

    // Deploy second contract with auto_derive disabled
    // F1r3node's insertSigned will upgrade the contract at the same URI
    // if the version (timestamp) is higher
    let template = RholangContractLibrary::rho20_contract();

    let contract_id_2 = executor
        .deploy_contract(
            template,
            "CASH",
            "Cash Token",
            500_000,
            6,
            vec![
                "issue".to_string(),
                "transfer".to_string(),
                "balanceOf".to_string(),
            ],
        )
        .await
        .expect("Deploy failed");

    // With auto_derive disabled, both contracts have the SAME ID (contract upgrade)
    assert_eq!(
        contract_id_1, contract_id_2,
        "With auto_derive disabled, deployments upgrade the same contract URI"
    );

    // Verify we can call methods on both cached contracts
    let params = &[
        ("recipient", StrictVal::from("alice")),
        ("amount", StrictVal::from(1000u64)),
    ];

    let result_1 = executor.call_method(contract_id_1, "issue", params).await;
    let result_2 = executor.call_method(contract_id_2, "issue", params).await;

    assert!(result_1.is_ok(), "Should be able to call cached contract 1");
    assert!(result_2.is_ok(), "Should be able to call cached contract 2");
}

#[tokio::test]
async fn test_executor_query_metadata_immediately() {
    load_env();

    log::info!(
        "🧪 Testing immediate getMetadata query after deployment (reproducing wallet issue)"
    );

    let mut executor = F1r3flyExecutor::new().expect("Failed to create F1r3flyExecutor");
    executor.set_derivation_index(test_derivation_offset(
        "test_executor_query_metadata_immediately",
    ));

    let template = RholangContractLibrary::rho20_contract();

    log::info!("📝 Deploying contract...");
    let contract_id = executor
        .deploy_contract(
            template,
            "IMMEDIATE",
            "Immediate Query Test",
            500_000,
            6,
            vec!["issue".to_string(), "getMetadata".to_string()],
        )
        .await
        .expect("Deploy failed");

    log::info!("✅ Contract deployed: {}", contract_id);
    log::info!("🔍 Attempting IMMEDIATE getMetadata query...");

    // Query IMMEDIATELY after deployment (like wallet does)
    let result = executor.query_state(contract_id, "getMetadata", &[]).await;

    // Log the result to understand the behavior
    match result {
        Ok(metadata) => {
            log::info!("✅ Immediate query SUCCEEDED!");
            log::info!("📊 Metadata: {:#?}", metadata);

            // Verify metadata structure
            assert!(metadata.is_object(), "Metadata should be an object");

            let ticker = metadata.get("ticker");
            let name = metadata.get("name");
            let supply = metadata.get("supply");
            let decimals = metadata.get("decimals");

            log::info!("  Ticker: {:?}", ticker);
            log::info!("  Name: {:?}", name);
            log::info!("  Supply: {:?}", supply);
            log::info!("  Decimals: {:?}", decimals);

            assert!(ticker.is_some(), "Should have ticker");
            assert!(name.is_some(), "Should have name");
            assert!(supply.is_some(), "Should have supply");
            assert!(decimals.is_some(), "Should have decimals");
        }
        Err(e) => {
            log::error!("❌ Immediate query FAILED: {}", e);
            log::error!("   This reproduces the wallet issue!");

            // For now, we'll panic to see the error clearly
            panic!("Immediate getMetadata query failed: {}", e);
        }
    }
}
