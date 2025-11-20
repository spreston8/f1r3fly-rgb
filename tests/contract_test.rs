//! F1r3flyRgbContract Integration Tests
//!
//! Tests the high-level F1r3flyRgbContract API with live F1r3node instance.
//! Focuses on seal logic, tracker integration, and end-to-end workflows.
//!
//! Note: Uses dummy seals for fast, focused testing of seal tracking logic.
//! For full Bitcoin integration testing with real UTXOs, see E2E regtest tests.
//!
//! ## Test Isolation Strategy
//!
//! Each test uses a unique derivation index offset (derived from hashing the test name)
//! to ensure parallel tests don't interfere with each other's contracts on F1r3node.
//! This prevents state pollution when tests run concurrently.
//!
//! ## Multi-Contract Support
//!
//! Each test creates an **independent contract** with hash-based key derivation:
//! - Master key (`FIREFLY_PRIVATE_KEY`) pays phlo for all deployments
//! - `auto_derive = true` (default): Each `F1r3flyRgbContract::issue()` increments derivation_index
//! - Child key: `hash(master_key || derivation_index || domain_separator)`
//! - Unique child key → Unique registry URI → Independent contract state
//!
//! Tests run safely in parallel with zero state interference.
//!
//! ## Seal Creation Strategy
//!
//! Tests use high vout offsets (100, 1000, 2000+) for deterministic seal generation.
//! This ensures seals within a test have predictable, non-overlapping vout ranges,
//! which helps with debugging and test clarity (not required for isolation).
//!
//! Requirements:
//! - Running f1r3node instance
//! - FIREFLY_* environment variables set
//!
//! Run with: cargo test --test contract_test -- --nocapture

use amplify::confinement::SmallOrdMap;
use bp::seals::{TxoSeal, WTxoSeal};
use commit_verify::{Digest, DigestExt, Sha256};
use f1r3fly_rgb::{generate_issue_signature, generate_nonce, F1r3flyExecutor, F1r3flyRgbContract};
use rgb::Pile;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use strict_types::{StrictDumb, StrictVal};

/// Load environment variables from .env file
fn load_env() {
    use std::path::PathBuf;

    // Load from f1r3fly-rgb/.env
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

/// Create test seals for Bitcoin UTXO binding
///
/// Uses deterministic dummy seals for focused testing of seal tracking logic.
/// For production E2E tests with real Bitcoin UTXOs, use BitcoinRegtestHelper.
///
/// # Arguments
/// * `count` - Number of seals to create
/// * `vout_offset` - Base vout value for seal creation
fn create_test_seals_with_offset(count: u16, vout_offset: u32) -> SmallOrdMap<u16, WTxoSeal> {
    let mut seals = SmallOrdMap::new();
    let mut noise = Sha256::new();
    noise.input_raw(b"test_seals_for_contract");
    noise.input_raw(&vout_offset.to_le_bytes());

    for i in 0..count {
        let mut seal_noise = noise.clone();
        seal_noise.input_raw(&i.to_le_bytes());

        let vout = vout_offset + (i as u32);
        let seal = WTxoSeal::vout_no_fallback(bp::Vout::from_u32(vout), seal_noise, i as u64);
        let _ = seals.insert(i, seal);
    }

    seals
}

/// Create a test TxoSeal for balance queries
///
/// Uses a deterministic txid and vout for testing
fn create_query_seal() -> TxoSeal {
    use amplify::ByteArray;
    use bp::Txid;

    // Create deterministic txid for testing
    let txid_bytes = [0x11u8; 32];
    let txid = Txid::from_byte_array(txid_bytes);

    // Create seal with vout 0
    let outpoint = bp::Outpoint::new(txid, 0u32);

    // Create seal with no fallback (secondary = Noise - using strict_dumb for simplicity)
    use bp::seals::{Noise, TxoSealExt};

    TxoSeal {
        primary: outpoint,
        secondary: TxoSealExt::Noise(Noise::strict_dumb()),
    }
}

/// Create a matching pair of WTxoSeal and TxoSeal for testing
///
/// Returns (witnessed_seal, query_seal) with the same underlying outpoint
fn create_matching_seal_pair(vout: u32, nonce: u64) -> (WTxoSeal, TxoSeal) {
    use amplify::ByteArray;
    use bp::Txid;
    use commit_verify::{Digest, DigestExt, Sha256};

    // Create deterministic txid and noise
    let txid_bytes = [0xAAu8; 32];
    let txid = Txid::from_byte_array(txid_bytes);

    let mut noise = Sha256::new();
    noise.input_raw(b"matching_seal_for_balance_test");
    noise.input_raw(&vout.to_le_bytes());

    // Create witnessed seal for issue operation
    let witnessed_seal = WTxoSeal::vout_no_fallback(bp::Vout::from_u32(vout), noise, nonce);

    // Create matching regular seal for balance query
    let outpoint = bp::Outpoint::new(txid, vout);
    let query_seal = TxoSeal {
        primary: outpoint,
        secondary: bp::seals::TxoSealExt::Noise(bp::seals::Noise::strict_dumb()),
    };

    (witnessed_seal, query_seal)
}

// ============================================================================
// Test 1: Complete Contract Lifecycle with Seal Tracking
// ============================================================================

#[tokio::test]
async fn test_contract_lifecycle_with_seal_tracking() {
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::Info)
        .try_init();
    load_env();

    // Step 1: Create executor and issue token
    let mut executor = F1r3flyExecutor::new().expect("Failed to create F1r3flyExecutor");
    executor.set_derivation_index(test_derivation_offset(
        "test_contract_lifecycle_with_seal_tracking",
    ));
    executor.set_auto_derive(false); // Disable to keep derivation_index stable for signature verification

    let contract = F1r3flyRgbContract::issue(executor, "SEAL", "Seal Test Token", 1000000, 8)
        .await
        .expect("Failed to issue contract");

    let contract_id = contract.contract_id();

    // Verify metadata
    let methods = &contract.metadata().methods;
    assert_eq!(methods.len(), 4, "Contract should have exactly 4 methods");

    // Verify exact method set (order-independent)
    let mut expected_methods = vec![
        "getMetadata".to_string(),
        "issue".to_string(),
        "transfer".to_string(),
        "balanceOf".to_string(),
    ];
    let mut actual_methods = methods.clone();
    expected_methods.sort();
    actual_methods.sort();
    assert_eq!(
        actual_methods, expected_methods,
        "Contract methods should match exactly"
    );

    // Step 2: Verify tracker is initialized and empty
    let tracker = contract.tracker();

    // Tracker should be empty initially (no operations yet)
    assert_eq!(
        tracker.witness_ids().count(),
        0,
        "Tracker should have no witnesses initially"
    );

    // Step 3: Call transfer method with seals
    let mut contract = contract; // Make mutable for call_method

    // Create seals for transfer operation (use high vout for deterministic uniqueness)
    let seals = create_test_seals_with_offset(2, 100);

    let transfer_params = &[
        ("from", StrictVal::from("alice")),
        ("to", StrictVal::from("bob")),
        ("amount", StrictVal::from(500u64)),
    ];

    let result = contract
        .call_method("transfer", transfer_params, seals.clone())
        .await
        .expect("Transfer method call failed");

    // Verify state hash is non-empty
    assert_ne!(
        result.state_hash, [0u8; 32],
        "State hash should not be all zeros"
    );

    // Step 4: Verify seals were tracked

    // Use the opid from the execution result (not derived from state_hash)
    let opid = result.opid;

    // Check that seals were added for this operation
    let tracked_seals = contract.tracker().seals(opid, 10);
    assert_eq!(
        tracked_seals.len(),
        seals.len(),
        "All seals should be tracked for the operation"
    );

    // Verify seal data matches what we provided
    for (seal_idx, original_seal) in seals.iter() {
        let tracked_seal = tracked_seals
            .get(seal_idx)
            .expect("Seal should exist in tracker");

        // WTxoSeal should match
        assert_eq!(
            tracked_seal, original_seal,
            "Tracked seal should match original"
        );
    }

    // Step 5: Test seal serialization and balance query with actual value verification
    let query_seal = create_query_seal();

    // Test seal serialization format
    let seal_id = F1r3flyRgbContract::serialize_seal(&query_seal);

    // Verify seal serialization format (txid:vout)
    let parts: Vec<&str> = seal_id.split(':').collect();
    assert_eq!(
        parts.len(),
        2,
        "Seal ID should have exactly 2 parts (txid:vout)"
    );
    assert_eq!(parts[0].len(), 64, "Txid should be 64 hex chars (32 bytes)");

    // Verify all chars in txid are valid hex
    assert!(
        parts[0].chars().all(|c| c.is_ascii_hexdigit()),
        "Txid should contain only hex characters"
    );

    // Verify vout is a valid number
    parts[1].parse::<u32>().expect("Vout should be a valid u32");

    // Issue tokens to a specific seal so we can verify the exact balance
    let expected_amount: u64 = 750000;

    // Create matching witnessed/query seal pair for balance verification (use high vout for uniqueness)
    let (witnessed_seal, query_seal_for_balance) = create_matching_seal_pair(1000, 42);

    // Use the seal ID as the recipient so balanceOf can find it
    let seal_recipient = F1r3flyRgbContract::serialize_seal(&query_seal_for_balance);

    let mut issue_seals = SmallOrdMap::new();
    let _ = issue_seals.insert(0, witnessed_seal);

    // Generate nonce and signature for secured issue() method
    let child_key = contract
        .executor()
        .get_child_key()
        .expect("Failed to get child key for testing");

    let nonce = generate_nonce();
    let signature = generate_issue_signature(&seal_recipient, expected_amount, nonce, &child_key)
        .expect("Failed to generate signature");

    let issue_params = &[
        ("recipient", StrictVal::from(seal_recipient.clone())),
        ("amount", StrictVal::from(expected_amount)),
        ("nonce", StrictVal::from(nonce)),
        ("signatureHex", StrictVal::from(signature.as_str())),
    ];

    let issue_result = contract
        .call_method("issue", issue_params, issue_seals.clone())
        .await
        .expect("Issue call failed");

    // Verify issue operation succeeded
    assert_ne!(
        issue_result.state_hash, [0u8; 32],
        "Issue should generate state hash"
    );

    // Now query the balance for the exact seal we just issued to
    // This tests the full workflow: issue → serialize → query → parse
    let balance_result = contract.balance(&query_seal_for_balance).await;

    // Verify the balance matches what we issued - this is the key assertion!
    match balance_result {
        Ok(balance) => {
            assert_eq!(
                balance, expected_amount,
                "Balance should match issued amount: expected {}, got {}",
                expected_amount, balance
            );
        }
        Err(e) => {
            // If query fails, this is a real error since we just issued to this seal
            panic!(
                "Balance query should succeed after issuing tokens. Error: {:?}",
                e
            );
        }
    }

    // Step 6: Verify accessor methods
    assert_eq!(contract.contract_id(), contract_id);
    assert_eq!(
        contract.metadata().registry_uri,
        contract.metadata().registry_uri
    );

    // Can access executor
    let _ = contract.executor();

    // Can access tracker
    let _ = contract.tracker();
}

// ============================================================================
// Test 2: Multiple Operations with Seal Management
// ============================================================================

#[tokio::test]
async fn test_multiple_operations_with_seal_management() {
    load_env();

    // Step 1: Issue contract
    let mut executor = F1r3flyExecutor::new().expect("Failed to create F1r3flyExecutor");
    executor.set_derivation_index(test_derivation_offset(
        "test_multiple_operations_with_seal_management",
    ));
    executor.set_auto_derive(false); // Disable to keep derivation_index stable for signature verification

    let mut contract = F1r3flyRgbContract::issue(executor, "MULTI", "Multi-Op Token", 5000000, 6)
        .await
        .expect("Failed to issue contract");

    // Step 2: Perform first operation (issue) - use high vout range for deterministic uniqueness
    let seals_op1 = create_test_seals_with_offset(1, 2000);

    // Generate signature for issue() call
    let child_key = contract
        .executor()
        .get_child_key()
        .expect("Failed to get child key");
    let nonce = generate_nonce();
    let signature = generate_issue_signature("alice", 1000000u64, nonce, &child_key)
        .expect("Failed to generate signature");

    let issue_params = &[
        ("recipient", StrictVal::from("alice")),
        ("amount", StrictVal::from(1000000u64)),
        ("nonce", StrictVal::from(nonce)),
        ("signatureHex", StrictVal::from(signature.as_str())),
    ];

    let result1 = contract
        .call_method("issue", issue_params, seals_op1.clone())
        .await
        .expect("Issue call failed");

    let opid1 = result1.opid;

    // Step 3: Perform second operation (transfer)
    let seals_op2 = create_test_seals_with_offset(2, 2001);

    let transfer_params = &[
        ("from", StrictVal::from("alice")),
        ("to", StrictVal::from("bob")),
        ("amount", StrictVal::from(250u64)),
    ];

    let result2 = contract
        .call_method("transfer", transfer_params, seals_op2.clone())
        .await
        .expect("Transfer call failed");

    let opid2 = result2.opid;

    // Step 4: Perform third operation (another transfer)
    let seals_op3 = create_test_seals_with_offset(3, 2003);

    let transfer_params2 = &[
        ("from", StrictVal::from("bob")),
        ("to", StrictVal::from("charlie")),
        ("amount", StrictVal::from(100u64)),
    ];

    let result3 = contract
        .call_method("transfer", transfer_params2, seals_op3.clone())
        .await
        .expect("Second transfer call failed");

    let opid3 = result3.opid;

    // Step 5: Verify all operations have unique state hashes
    assert_ne!(
        result1.state_hash, result2.state_hash,
        "Op 1 and Op 2 should have different state hashes"
    );
    assert_ne!(
        result2.state_hash, result3.state_hash,
        "Op 2 and Op 3 should have different state hashes"
    );
    assert_ne!(
        result1.state_hash, result3.state_hash,
        "Op 1 and Op 3 should have different state hashes"
    );

    // Step 6: Verify tracker contains all operations' seals
    let tracked_seals_1 = contract.tracker().seals(opid1, 10);
    let tracked_seals_2 = contract.tracker().seals(opid2, 10);
    let tracked_seals_3 = contract.tracker().seals(opid3, 10);

    assert_eq!(tracked_seals_1.len(), 1, "Op 1 should have 1 seal");
    assert_eq!(tracked_seals_2.len(), 2, "Op 2 should have 2 seals");
    assert_eq!(tracked_seals_3.len(), 3, "Op 3 should have 3 seals");

    // Step 7: Verify seal data integrity

    // Check that each operation's seals match what we provided
    for (seal_idx, original_seal) in seals_op1.iter() {
        let tracked_seal = tracked_seals_1.get(seal_idx).expect("Seal should exist");
        assert_eq!(
            tracked_seal, original_seal,
            "Op 1 seal {} should match",
            seal_idx
        );
    }

    for (seal_idx, original_seal) in seals_op2.iter() {
        let tracked_seal = tracked_seals_2.get(seal_idx).expect("Seal should exist");
        assert_eq!(
            tracked_seal, original_seal,
            "Op 2 seal {} should match",
            seal_idx
        );
    }

    for (seal_idx, original_seal) in seals_op3.iter() {
        let tracked_seal = tracked_seals_3.get(seal_idx).expect("Seal should exist");
        assert_eq!(
            tracked_seal, original_seal,
            "Op 3 seal {} should match",
            seal_idx
        );
    }

    // Step 8: Verify seal serialization consistency
    let test_seal = create_query_seal();
    let serialized1 = F1r3flyRgbContract::serialize_seal(&test_seal);
    let serialized2 = F1r3flyRgbContract::serialize_seal(&test_seal);

    assert_eq!(
        serialized1, serialized2,
        "Seal serialization should be deterministic"
    );

    // Step 9: Test mutable and immutable accessors
    // Test executor_mut
    let _executor_mut = contract.executor_mut();

    // Test tracker_mut
    let tracker_mut = contract.tracker_mut();
    // Verify we can call Pile trait methods on mutable tracker
    assert_eq!(
        tracker_mut.witness_ids().count(),
        0,
        "No witnesses added yet"
    );
}
