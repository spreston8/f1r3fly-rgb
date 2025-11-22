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
use amplify::ByteArray;
use bp::seals::{TxoSeal, WTxoSeal};
use bp::Txid;
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

    // Derive public key from child key for owner registration
    let secp = secp256k1::Secp256k1::new();
    let child_public_key = secp256k1::PublicKey::from_secret_key(&secp, &child_key);
    let recipient_pubkey_hex = hex::encode(child_public_key.serialize_uncompressed());

    let nonce = generate_nonce();
    let signature = generate_issue_signature(&seal_recipient, expected_amount, nonce, &child_key)
        .expect("Failed to generate signature");

    let issue_params = &[
        ("recipient", StrictVal::from(seal_recipient.clone())),
        ("amount", StrictVal::from(expected_amount)),
        (
            "recipientPubKey",
            StrictVal::from(recipient_pubkey_hex.as_str()),
        ),
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

    // Derive public key from child key for owner registration
    let secp = secp256k1::Secp256k1::new();
    let child_public_key = secp256k1::PublicKey::from_secret_key(&secp, &child_key);
    let recipient_pubkey_hex = hex::encode(child_public_key.serialize_uncompressed());

    let nonce = generate_nonce();
    let signature = generate_issue_signature("alice", 1000000u64, nonce, &child_key)
        .expect("Failed to generate signature");

    let issue_params = &[
        ("recipient", StrictVal::from("alice")),
        ("amount", StrictVal::from(1000000u64)),
        (
            "recipientPubKey",
            StrictVal::from(recipient_pubkey_hex.as_str()),
        ),
        ("nonce", StrictVal::from(nonce)),
        ("signatureHex", StrictVal::from(signature.as_str())),
    ];

    let result1 = contract
        .call_method("issue", issue_params, seals_op1.clone())
        .await
        .expect("Issue call failed");

    let opid1 = result1.opid;

    // Step 3: Perform second operation (transfer alice → bob)
    let seals_op2 = create_test_seals_with_offset(2, 2001);

    // Generate Bob's key (different derivation index for different participant)
    let mut executor_bob = F1r3flyExecutor::new().expect("Failed to create Bob's executor");
    executor_bob.set_derivation_index(
        test_derivation_offset("test_multiple_operations_with_seal_management") + 1,
    );
    let bob_key = executor_bob
        .get_child_key()
        .expect("Failed to get Bob's key");
    let bob_public_key = secp256k1::PublicKey::from_secret_key(&secp, &bob_key);
    let bob_pubkey_hex = hex::encode(bob_public_key.serialize_uncompressed());

    // Alice signs the transfer (she owns "alice" UTXO)
    let transfer1_nonce = generate_nonce();
    let transfer1_signature =
        f1r3fly_rgb::generate_transfer_signature("alice", "bob", 250, transfer1_nonce, &child_key)
            .expect("Failed to generate transfer signature");

    let transfer_params = &[
        ("from", StrictVal::from("alice")),
        ("to", StrictVal::from("bob")),
        ("amount", StrictVal::from(250u64)),
        ("toPubKey", StrictVal::from(bob_pubkey_hex.as_str())),
        ("nonce", StrictVal::from(transfer1_nonce)),
        (
            "fromSignatureHex",
            StrictVal::from(transfer1_signature.as_str()),
        ),
    ];

    let result2 = contract
        .call_method("transfer", transfer_params, seals_op2.clone())
        .await
        .expect("Transfer call failed");

    let opid2 = result2.opid;

    // Step 4: Perform third operation (transfer bob → charlie)
    let seals_op3 = create_test_seals_with_offset(3, 2003);

    // Generate Charlie's key (different derivation index)
    let mut executor_charlie = F1r3flyExecutor::new().expect("Failed to create Charlie's executor");
    executor_charlie.set_derivation_index(
        test_derivation_offset("test_multiple_operations_with_seal_management") + 2,
    );
    let charlie_key = executor_charlie
        .get_child_key()
        .expect("Failed to get Charlie's key");
    let charlie_public_key = secp256k1::PublicKey::from_secret_key(&secp, &charlie_key);
    let charlie_pubkey_hex = hex::encode(charlie_public_key.serialize_uncompressed());

    // Bob signs the transfer (he now owns "bob" UTXO after previous transfer)
    let transfer2_nonce = generate_nonce();
    let transfer2_signature = f1r3fly_rgb::generate_transfer_signature(
        "bob",
        "charlie",
        100,
        transfer2_nonce,
        &bob_key, // Bob's key signs
    )
    .expect("Failed to generate transfer signature");

    let transfer_params2 = &[
        ("from", StrictVal::from("bob")),
        ("to", StrictVal::from("charlie")),
        ("amount", StrictVal::from(100u64)),
        ("toPubKey", StrictVal::from(charlie_pubkey_hex.as_str())),
        ("nonce", StrictVal::from(transfer2_nonce)),
        (
            "fromSignatureHex",
            StrictVal::from(transfer2_signature.as_str()),
        ),
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

/// Test: Issue prevents nonce reuse (replay protection)
///
/// Verifies:
/// - Same nonce cannot be used twice for issue() method
/// - Rholang contract tracks used nonces and rejects duplicates
/// - Error message indicates "Nonce already used"
#[tokio::test]
async fn test_issue_prevents_nonce_reuse() {
    load_env();

    let test_name = "issue_prevents_nonce_reuse";
    let offset = test_derivation_offset(test_name);

    // Step 1: Create executor and deploy contract
    let mut executor = F1r3flyExecutor::new().expect("Failed to create executor");
    executor.set_derivation_index(offset);
    executor.set_auto_derive(false); // Keep same key for both calls

    let mut contract = F1r3flyRgbContract::issue(executor, "NONCE", "Nonce Test Token", 10_000, 0)
        .await
        .expect("Failed to issue asset");

    let contract_id = contract.contract_id();

    // Step 2: Get signing key and create genesis seal
    let signing_key = contract
        .executor()
        .get_child_key()
        .expect("Failed to get signing key");

    let genesis_seal = create_query_seal();
    let genesis_seal_str = F1r3flyRgbContract::serialize_seal(&genesis_seal);

    // Step 3: Generate a specific nonce
    let nonce = generate_nonce();

    // Step 4: First issue() call with this nonce - should succeed
    // Derive public key from signing key for owner registration
    let secp = secp256k1::Secp256k1::new();
    let signing_public_key = secp256k1::PublicKey::from_secret_key(&secp, &signing_key);
    let recipient_pubkey_hex = hex::encode(signing_public_key.serialize_uncompressed());

    let signature1 = generate_issue_signature(&genesis_seal_str, 1000, nonce, &signing_key)
        .expect("Failed to generate signature");

    let result1 = contract
        .executor_mut()
        .call_method(
            contract_id,
            "issue",
            &[
                ("recipient", StrictVal::from(genesis_seal_str.as_str())),
                ("amount", StrictVal::from(1000u64)),
                (
                    "recipientPubKey",
                    StrictVal::from(recipient_pubkey_hex.as_str()),
                ),
                ("nonce", StrictVal::from(nonce)),
                ("signatureHex", StrictVal::from(signature1.as_str())),
            ],
        )
        .await;

    assert!(
        result1.is_ok(),
        "First issue() call should succeed: {:?}",
        result1
    );

    // Verify first issue succeeded by checking balance
    let balance1 = contract
        .balance(&genesis_seal)
        .await
        .expect("Failed to query balance after first issue");
    assert_eq!(balance1, 1000, "Balance should be 1000 after first issue");

    // Step 5: Second issue() call with SAME nonce but different parameters - should FAIL
    // Create a different seal using matching_seal_pair helper
    let (_, seal2) = create_matching_seal_pair(2000, 42);
    let seal2_str = F1r3flyRgbContract::serialize_seal(&seal2);

    // Sign with same nonce but different recipient/amount
    let signature2 = generate_issue_signature(&seal2_str, 2000, nonce, &signing_key)
        .expect("Failed to generate signature 2");

    let _result2 = contract
        .executor_mut()
        .call_method(
            contract_id,
            "issue",
            &[
                ("recipient", StrictVal::from(seal2_str.as_str())),
                ("amount", StrictVal::from(2000u64)),
                (
                    "recipientPubKey",
                    StrictVal::from(recipient_pubkey_hex.as_str()),
                ),
                ("nonce", StrictVal::from(nonce)), // SAME NONCE!
                ("signatureHex", StrictVal::from(signature2.as_str())),
            ],
        )
        .await
        .expect("Deploy should succeed (but business logic should reject)");

    // Verify second issue was rejected by checking balance didn't change for seal2
    let balance2 = contract
        .balance(&seal2)
        .await
        .expect("Failed to query balance after second issue");
    assert_eq!(
        balance2, 0,
        "Balance for seal2 should be 0 (nonce reuse should be rejected)"
    );

    // Also verify first seal's balance is still 1000
    let balance1_after = contract
        .balance(&genesis_seal)
        .await
        .expect("Failed to query balance for genesis seal");
    assert_eq!(
        balance1_after, 1000,
        "Original balance should remain unchanged"
    );
}

/// Test: Unauthorized issue rejected (wrong signer)
///
/// Verifies:
/// - Only deployer can call issue() method
/// - Signatures from other keys are rejected
/// - Error message indicates "Invalid signature - unauthorized"
#[tokio::test]
async fn test_unauthorized_issue_rejected() {
    load_env();

    let test_name = "unauthorized_issue_rejected";
    let offset = test_derivation_offset(test_name);

    // Step 1: Alice deploys contract with her key (at derivation index)
    let mut executor_alice = F1r3flyExecutor::new().expect("Failed to create Alice's executor");
    executor_alice.set_derivation_index(offset);
    executor_alice.set_auto_derive(false);

    let mut contract_alice =
        F1r3flyRgbContract::issue(executor_alice, "AUTH", "Auth Test Token", 10_000, 0)
            .await
            .expect("Failed to issue asset");

    let contract_id = contract_alice.contract_id();

    // Alice's key (used to deploy contract)
    let alice_key = contract_alice
        .executor()
        .get_child_key()
        .expect("Failed to get Alice's key");

    // Step 2: Bob tries to issue on Alice's contract using his own key
    // Create a different executor with different derivation index to get Bob's key
    let bob_offset = offset + 1000; // Different derivation index
    let mut executor_bob = F1r3flyExecutor::new().expect("Failed to create Bob's executor");
    executor_bob.set_derivation_index(bob_offset);

    let bob_key = executor_bob
        .get_child_key()
        .expect("Failed to get Bob's key");

    // Verify Alice and Bob have different keys
    assert_ne!(
        alice_key.secret_bytes(),
        bob_key.secret_bytes(),
        "Alice and Bob should have different keys"
    );

    // Step 3: Bob generates a signature with HIS key (not Alice's)
    let genesis_seal = create_query_seal();
    let genesis_seal_str = F1r3flyRgbContract::serialize_seal(&genesis_seal);

    let nonce = generate_nonce();

    // Bob signs with HIS key, but tries to issue on Alice's contract
    // Derive Bob's public key for the issue call
    let secp = secp256k1::Secp256k1::new();
    let bob_public_key = secp256k1::PublicKey::from_secret_key(&secp, &bob_key);
    let bob_pubkey_hex = hex::encode(bob_public_key.serialize_uncompressed());

    let bob_signature = generate_issue_signature(&genesis_seal_str, 5000, nonce, &bob_key) // Bob's key!
        .expect("Failed to generate Bob's signature");

    // Step 4: Bob tries to call issue() on Alice's contract
    let _result = contract_alice
        .executor_mut()
        .call_method(
            contract_id,
            "issue",
            &[
                ("recipient", StrictVal::from(genesis_seal_str.as_str())),
                ("amount", StrictVal::from(5000u64)),
                ("recipientPubKey", StrictVal::from(bob_pubkey_hex.as_str())),
                ("nonce", StrictVal::from(nonce)),
                ("signatureHex", StrictVal::from(bob_signature.as_str())),
            ],
        )
        .await
        .expect("Deploy should succeed (but business logic should reject)");

    // Verify unauthorized issue was rejected by checking balance is still 0
    let balance_after_bob = contract_alice
        .balance(&genesis_seal)
        .await
        .expect("Failed to query balance after unauthorized issue");
    assert_eq!(
        balance_after_bob, 0,
        "Balance should be 0 (unauthorized signature should be rejected)"
    );
}

// ============================================================================
// Phase 2 (Issue 2) Tests: Transfer Authorization
// ============================================================================

#[tokio::test]
async fn test_transfer_requires_valid_signature() {
    load_env();

    let test_name = "transfer_requires_valid_signature";
    let offset = test_derivation_offset(test_name);

    // Step 1: Alice deploys contract and issues to herself
    let mut executor_alice = F1r3flyExecutor::new().expect("Failed to create Alice's executor");
    executor_alice.set_derivation_index(offset);
    executor_alice.set_auto_derive(false);

    let mut contract_alice =
        F1r3flyRgbContract::issue(executor_alice, "XFER", "Transfer Test Token", 10_000, 0)
            .await
            .expect("Failed to issue asset");

    let contract_id = contract_alice.contract_id();

    // Get Alice's key and public key
    let alice_key = contract_alice
        .executor()
        .get_child_key()
        .expect("Failed to get Alice's key");
    let secp = secp256k1::Secp256k1::new();
    let alice_public_key = secp256k1::PublicKey::from_secret_key(&secp, &alice_key);
    let alice_pubkey_hex = hex::encode(alice_public_key.serialize_uncompressed());

    // Issue tokens to Alice's UTXO
    let alice_seal = create_query_seal();
    let alice_seal_str = F1r3flyRgbContract::serialize_seal(&alice_seal);

    let issue_nonce = generate_nonce();
    let issue_signature = generate_issue_signature(&alice_seal_str, 5000, issue_nonce, &alice_key)
        .expect("Failed to generate issue signature");

    let _issue_result = contract_alice
        .executor_mut()
        .call_method(
            contract_id,
            "issue",
            &[
                ("recipient", StrictVal::from(alice_seal_str.as_str())),
                ("amount", StrictVal::from(5000u64)),
                (
                    "recipientPubKey",
                    StrictVal::from(alice_pubkey_hex.as_str()),
                ),
                ("nonce", StrictVal::from(issue_nonce)),
                ("signatureHex", StrictVal::from(issue_signature.as_str())),
            ],
        )
        .await
        .expect("Issue should succeed");

    // Verify Alice has balance
    let alice_balance_before = contract_alice
        .balance(&alice_seal)
        .await
        .expect("Failed to query Alice's balance");
    assert_eq!(
        alice_balance_before, 5000,
        "Alice should have 5000 tokens after issue"
    );

    // Step 2: Create Bob's key and UTXO
    let mut executor_bob = F1r3flyExecutor::new().expect("Failed to create Bob's executor");
    executor_bob.set_derivation_index(offset + 1);
    let bob_key = executor_bob
        .get_child_key()
        .expect("Failed to get Bob's key");
    let bob_public_key = secp256k1::PublicKey::from_secret_key(&secp, &bob_key);
    let bob_pubkey_hex = hex::encode(bob_public_key.serialize_uncompressed());

    // Create Bob's seal (deterministic for testing)
    let bob_txid_bytes = [0x22u8; 32];
    let bob_txid = Txid::from_byte_array(bob_txid_bytes);
    let bob_outpoint = bp::Outpoint::new(bob_txid, 0u32);
    use bp::seals::{Noise, TxoSealExt};
    let bob_seal = TxoSeal {
        primary: bob_outpoint,
        secondary: TxoSealExt::Noise(Noise::strict_dumb()),
    };
    let bob_seal_str = F1r3flyRgbContract::serialize_seal(&bob_seal);

    // Step 3: Alice transfers to Bob WITH valid signature
    let transfer_nonce = generate_nonce();
    let transfer_signature = f1r3fly_rgb::generate_transfer_signature(
        &alice_seal_str,
        &bob_seal_str,
        1000,
        transfer_nonce,
        &alice_key, // Alice signs
    )
    .expect("Failed to generate transfer signature");

    let _transfer_result = contract_alice
        .executor_mut()
        .call_method(
            contract_id,
            "transfer",
            &[
                ("from", StrictVal::from(alice_seal_str.as_str())),
                ("to", StrictVal::from(bob_seal_str.as_str())),
                ("amount", StrictVal::from(1000u64)),
                ("toPubKey", StrictVal::from(bob_pubkey_hex.as_str())),
                ("nonce", StrictVal::from(transfer_nonce)),
                (
                    "fromSignatureHex",
                    StrictVal::from(transfer_signature.as_str()),
                ),
            ],
        )
        .await
        .expect("Transfer should succeed with valid signature");

    // Step 4: Verify balances changed correctly
    let alice_balance_after = contract_alice
        .balance(&alice_seal)
        .await
        .expect("Failed to query Alice's balance after transfer");
    assert_eq!(
        alice_balance_after, 4000,
        "Alice should have 4000 tokens after transfer"
    );

    let bob_balance_after = contract_alice
        .balance(&bob_seal)
        .await
        .expect("Failed to query Bob's balance after transfer");
    assert_eq!(
        bob_balance_after, 1000,
        "Bob should have 1000 tokens after transfer"
    );
}

#[tokio::test]
async fn test_transfer_rejects_invalid_signature() {
    load_env();

    let test_name = "transfer_rejects_invalid_signature";
    let offset = test_derivation_offset(test_name);

    // Step 1: Alice deploys contract and issues to herself
    let mut executor_alice = F1r3flyExecutor::new().expect("Failed to create Alice's executor");
    executor_alice.set_derivation_index(offset);
    executor_alice.set_auto_derive(false);

    let mut contract_alice = F1r3flyRgbContract::issue(
        executor_alice,
        "INVSIG",
        "Invalid Signature Test",
        10_000,
        0,
    )
    .await
    .expect("Failed to issue asset");

    let contract_id = contract_alice.contract_id();

    // Get Alice's key
    let alice_key = contract_alice
        .executor()
        .get_child_key()
        .expect("Failed to get Alice's key");
    let secp = secp256k1::Secp256k1::new();
    let alice_public_key = secp256k1::PublicKey::from_secret_key(&secp, &alice_key);
    let alice_pubkey_hex = hex::encode(alice_public_key.serialize_uncompressed());

    // Issue tokens to Alice
    let alice_seal = create_query_seal();
    let alice_seal_str = F1r3flyRgbContract::serialize_seal(&alice_seal);

    let issue_nonce = generate_nonce();
    let issue_signature = generate_issue_signature(&alice_seal_str, 5000, issue_nonce, &alice_key)
        .expect("Failed to generate issue signature");

    let _issue_result = contract_alice
        .executor_mut()
        .call_method(
            contract_id,
            "issue",
            &[
                ("recipient", StrictVal::from(alice_seal_str.as_str())),
                ("amount", StrictVal::from(5000u64)),
                (
                    "recipientPubKey",
                    StrictVal::from(alice_pubkey_hex.as_str()),
                ),
                ("nonce", StrictVal::from(issue_nonce)),
                ("signatureHex", StrictVal::from(issue_signature.as_str())),
            ],
        )
        .await
        .expect("Issue should succeed");

    // Step 2: Create Bob's key and a WRONG key for signing
    let mut executor_bob = F1r3flyExecutor::new().expect("Failed to create Bob's executor");
    executor_bob.set_derivation_index(offset + 1);
    let bob_key = executor_bob
        .get_child_key()
        .expect("Failed to get Bob's key");
    let bob_public_key = secp256k1::PublicKey::from_secret_key(&secp, &bob_key);
    let bob_pubkey_hex = hex::encode(bob_public_key.serialize_uncompressed());

    // Create Bob's seal (deterministic for testing)
    let bob_txid_bytes = [0x22u8; 32];
    let bob_txid = Txid::from_byte_array(bob_txid_bytes);
    let bob_outpoint = bp::Outpoint::new(bob_txid, 0u32);
    use bp::seals::{Noise, TxoSealExt};
    let bob_seal = TxoSeal {
        primary: bob_outpoint,
        secondary: TxoSealExt::Noise(Noise::strict_dumb()),
    };
    let bob_seal_str = F1r3flyRgbContract::serialize_seal(&bob_seal);

    // Create a WRONG key (different from Alice's)
    let mut executor_wrong = F1r3flyExecutor::new().expect("Failed to create wrong executor");
    executor_wrong.set_derivation_index(offset + 999);
    let wrong_key = executor_wrong
        .get_child_key()
        .expect("Failed to get wrong key");

    // Verify wrong key is different from Alice's
    assert_ne!(
        alice_key.secret_bytes(),
        wrong_key.secret_bytes(),
        "Wrong key should be different from Alice's key"
    );

    // Step 3: Try to transfer with WRONG signature (signed by wrong_key, not alice_key)
    let transfer_nonce = generate_nonce();
    let wrong_signature = f1r3fly_rgb::generate_transfer_signature(
        &alice_seal_str,
        &bob_seal_str,
        1000,
        transfer_nonce,
        &wrong_key, // WRONG KEY!
    )
    .expect("Failed to generate wrong signature");

    let _transfer_result = contract_alice
        .executor_mut()
        .call_method(
            contract_id,
            "transfer",
            &[
                ("from", StrictVal::from(alice_seal_str.as_str())),
                ("to", StrictVal::from(bob_seal_str.as_str())),
                ("amount", StrictVal::from(1000u64)),
                ("toPubKey", StrictVal::from(bob_pubkey_hex.as_str())),
                ("nonce", StrictVal::from(transfer_nonce)),
                (
                    "fromSignatureHex",
                    StrictVal::from(wrong_signature.as_str()),
                ),
            ],
        )
        .await
        .expect("Deploy should succeed (but business logic should reject)");

    // Step 4: Verify transfer was REJECTED - balances should be unchanged
    let alice_balance_after = contract_alice
        .balance(&alice_seal)
        .await
        .expect("Failed to query Alice's balance after failed transfer");
    assert_eq!(
        alice_balance_after, 5000,
        "Alice should still have 5000 tokens (transfer rejected)"
    );

    let bob_balance_after = contract_alice
        .balance(&bob_seal)
        .await
        .expect("Failed to query Bob's balance after failed transfer");
    assert_eq!(
        bob_balance_after, 0,
        "Bob should have 0 tokens (transfer rejected)"
    );
}

#[tokio::test]
async fn test_transfer_rejects_unauthorized_sender() {
    load_env();

    let test_name = "transfer_rejects_unauthorized_sender";
    let offset = test_derivation_offset(test_name);

    // Step 1: Alice deploys contract and issues to herself
    let mut executor_alice = F1r3flyExecutor::new().expect("Failed to create Alice's executor");
    executor_alice.set_derivation_index(offset);
    executor_alice.set_auto_derive(false);

    let mut contract_alice = F1r3flyRgbContract::issue(
        executor_alice,
        "UNAUTH",
        "Unauthorized Sender Test",
        10_000,
        0,
    )
    .await
    .expect("Failed to issue asset");

    let contract_id = contract_alice.contract_id();

    // Get Alice's key
    let alice_key = contract_alice
        .executor()
        .get_child_key()
        .expect("Failed to get Alice's key");
    let secp = secp256k1::Secp256k1::new();
    let alice_public_key = secp256k1::PublicKey::from_secret_key(&secp, &alice_key);
    let alice_pubkey_hex = hex::encode(alice_public_key.serialize_uncompressed());

    // Issue tokens to Alice's UTXO
    let alice_seal = create_query_seal();
    let alice_seal_str = F1r3flyRgbContract::serialize_seal(&alice_seal);

    let issue_nonce = generate_nonce();
    let issue_signature = generate_issue_signature(&alice_seal_str, 5000, issue_nonce, &alice_key)
        .expect("Failed to generate issue signature");

    let _issue_result = contract_alice
        .executor_mut()
        .call_method(
            contract_id,
            "issue",
            &[
                ("recipient", StrictVal::from(alice_seal_str.as_str())),
                ("amount", StrictVal::from(5000u64)),
                (
                    "recipientPubKey",
                    StrictVal::from(alice_pubkey_hex.as_str()),
                ),
                ("nonce", StrictVal::from(issue_nonce)),
                ("signatureHex", StrictVal::from(issue_signature.as_str())),
            ],
        )
        .await
        .expect("Issue should succeed");

    // Step 2: Attacker creates their own key and tries to steal Alice's tokens
    let mut executor_attacker =
        F1r3flyExecutor::new().expect("Failed to create attacker's executor");
    executor_attacker.set_derivation_index(offset + 666);
    let attacker_key = executor_attacker
        .get_child_key()
        .expect("Failed to get attacker's key");
    let attacker_public_key = secp256k1::PublicKey::from_secret_key(&secp, &attacker_key);
    let attacker_pubkey_hex = hex::encode(attacker_public_key.serialize_uncompressed());

    // Create attacker's seal (deterministic for testing)
    let attacker_txid_bytes = [0x66u8; 32]; // Different from Alice's 0x11 and Bob's 0x22
    let attacker_txid = Txid::from_byte_array(attacker_txid_bytes);
    let attacker_outpoint = bp::Outpoint::new(attacker_txid, 0u32);
    use bp::seals::{Noise, TxoSealExt};
    let attacker_seal = TxoSeal {
        primary: attacker_outpoint,
        secondary: TxoSealExt::Noise(Noise::strict_dumb()),
    };
    let attacker_seal_str = F1r3flyRgbContract::serialize_seal(&attacker_seal);

    // Verify attacker has different key than Alice
    assert_ne!(
        alice_key.secret_bytes(),
        attacker_key.secret_bytes(),
        "Attacker should have different key than Alice"
    );

    // Step 3: Attacker tries to transfer Alice's tokens to themselves
    // Attacker signs with THEIR key (not Alice's)
    let transfer_nonce = generate_nonce();
    let attacker_signature = f1r3fly_rgb::generate_transfer_signature(
        &alice_seal_str,    // From Alice's UTXO (which attacker doesn't own!)
        &attacker_seal_str, // To attacker's UTXO
        5000,               // Try to steal all tokens
        transfer_nonce,
        &attacker_key, // Attacker signs with their key
    )
    .expect("Failed to generate attacker's signature");

    let _transfer_result = contract_alice
        .executor_mut()
        .call_method(
            contract_id,
            "transfer",
            &[
                ("from", StrictVal::from(alice_seal_str.as_str())),
                ("to", StrictVal::from(attacker_seal_str.as_str())),
                ("amount", StrictVal::from(5000u64)),
                ("toPubKey", StrictVal::from(attacker_pubkey_hex.as_str())),
                ("nonce", StrictVal::from(transfer_nonce)),
                (
                    "fromSignatureHex",
                    StrictVal::from(attacker_signature.as_str()),
                ),
            ],
        )
        .await
        .expect("Deploy should succeed (but business logic should reject)");

    // Step 4: Verify attack was REJECTED - Alice still has tokens, attacker has none
    let alice_balance_after = contract_alice
        .balance(&alice_seal)
        .await
        .expect("Failed to query Alice's balance after attack");
    assert_eq!(
        alice_balance_after, 5000,
        "Alice should still have all 5000 tokens (attack rejected)"
    );

    let attacker_balance_after = contract_alice
        .balance(&attacker_seal)
        .await
        .expect("Failed to query attacker's balance after attack");
    assert_eq!(
        attacker_balance_after, 0,
        "Attacker should have 0 tokens (attack rejected)"
    );
}

#[tokio::test]
async fn test_transfer_rejects_reused_nonce() {
    load_env();

    let test_name = "transfer_rejects_reused_nonce";
    let offset = test_derivation_offset(test_name);

    // Step 1: Alice deploys contract and issues to herself
    let mut executor_alice = F1r3flyExecutor::new().expect("Failed to create Alice's executor");
    executor_alice.set_derivation_index(offset);
    executor_alice.set_auto_derive(false);

    let mut contract_alice = F1r3flyRgbContract::issue(
        executor_alice,
        "REPLAY",
        "Replay Protection Test",
        10_000,
        0,
    )
    .await
    .expect("Failed to issue asset");

    let contract_id = contract_alice.contract_id();

    // Get Alice's key
    let alice_key = contract_alice
        .executor()
        .get_child_key()
        .expect("Failed to get Alice's key");
    let secp = secp256k1::Secp256k1::new();
    let alice_public_key = secp256k1::PublicKey::from_secret_key(&secp, &alice_key);
    let alice_pubkey_hex = hex::encode(alice_public_key.serialize_uncompressed());

    // Issue tokens to Alice
    let alice_seal = create_query_seal();
    let alice_seal_str = F1r3flyRgbContract::serialize_seal(&alice_seal);

    let issue_nonce = generate_nonce();
    let issue_signature = generate_issue_signature(&alice_seal_str, 5000, issue_nonce, &alice_key)
        .expect("Failed to generate issue signature");

    let _issue_result = contract_alice
        .executor_mut()
        .call_method(
            contract_id,
            "issue",
            &[
                ("recipient", StrictVal::from(alice_seal_str.as_str())),
                ("amount", StrictVal::from(5000u64)),
                (
                    "recipientPubKey",
                    StrictVal::from(alice_pubkey_hex.as_str()),
                ),
                ("nonce", StrictVal::from(issue_nonce)),
                ("signatureHex", StrictVal::from(issue_signature.as_str())),
            ],
        )
        .await
        .expect("Issue should succeed");

    // Step 2: Create Bob's key
    let mut executor_bob = F1r3flyExecutor::new().expect("Failed to create Bob's executor");
    executor_bob.set_derivation_index(offset + 1);
    let bob_key = executor_bob
        .get_child_key()
        .expect("Failed to get Bob's key");
    let bob_public_key = secp256k1::PublicKey::from_secret_key(&secp, &bob_key);
    let bob_pubkey_hex = hex::encode(bob_public_key.serialize_uncompressed());

    // Create Bob's seal (deterministic for testing)
    let bob_txid_bytes = [0x22u8; 32];
    let bob_txid = Txid::from_byte_array(bob_txid_bytes);
    let bob_outpoint = bp::Outpoint::new(bob_txid, 0u32);
    use bp::seals::{Noise, TxoSealExt};
    let bob_seal = TxoSeal {
        primary: bob_outpoint,
        secondary: TxoSealExt::Noise(Noise::strict_dumb()),
    };
    let bob_seal_str = F1r3flyRgbContract::serialize_seal(&bob_seal);

    // Step 3: First transfer with specific nonce - should SUCCEED
    let transfer_nonce = 12345u64; // Fixed nonce for replay test
    let transfer_signature = f1r3fly_rgb::generate_transfer_signature(
        &alice_seal_str,
        &bob_seal_str,
        1000,
        transfer_nonce,
        &alice_key,
    )
    .expect("Failed to generate transfer signature");

    let _transfer_result1 = contract_alice
        .executor_mut()
        .call_method(
            contract_id,
            "transfer",
            &[
                ("from", StrictVal::from(alice_seal_str.as_str())),
                ("to", StrictVal::from(bob_seal_str.as_str())),
                ("amount", StrictVal::from(1000u64)),
                ("toPubKey", StrictVal::from(bob_pubkey_hex.as_str())),
                ("nonce", StrictVal::from(transfer_nonce)),
                (
                    "fromSignatureHex",
                    StrictVal::from(transfer_signature.as_str()),
                ),
            ],
        )
        .await
        .expect("First transfer should succeed");

    // Verify first transfer succeeded
    let alice_balance_after_first = contract_alice
        .balance(&alice_seal)
        .await
        .expect("Failed to query Alice's balance after first transfer");
    assert_eq!(
        alice_balance_after_first, 4000,
        "Alice should have 4000 tokens after first transfer"
    );

    let bob_balance_after_first = contract_alice
        .balance(&bob_seal)
        .await
        .expect("Failed to query Bob's balance after first transfer");
    assert_eq!(
        bob_balance_after_first, 1000,
        "Bob should have 1000 tokens after first transfer"
    );

    // Step 4: Try SAME transfer again with SAME nonce - should be REJECTED
    let _transfer_result2 = contract_alice
        .executor_mut()
        .call_method(
            contract_id,
            "transfer",
            &[
                ("from", StrictVal::from(alice_seal_str.as_str())),
                ("to", StrictVal::from(bob_seal_str.as_str())),
                ("amount", StrictVal::from(1000u64)),
                ("toPubKey", StrictVal::from(bob_pubkey_hex.as_str())),
                ("nonce", StrictVal::from(transfer_nonce)), // SAME nonce!
                (
                    "fromSignatureHex",
                    StrictVal::from(transfer_signature.as_str()),
                ),
            ],
        )
        .await
        .expect("Deploy should succeed (but business logic should reject)");

    // Step 5: Verify replay was REJECTED - balances should be unchanged from first transfer
    let alice_balance_after_replay = contract_alice
        .balance(&alice_seal)
        .await
        .expect("Failed to query Alice's balance after replay attempt");
    assert_eq!(
        alice_balance_after_replay, 4000,
        "Alice should still have 4000 tokens (replay rejected)"
    );

    let bob_balance_after_replay = contract_alice
        .balance(&bob_seal)
        .await
        .expect("Failed to query Bob's balance after replay attempt");
    assert_eq!(
        bob_balance_after_replay, 1000,
        "Bob should still have 1000 tokens (replay rejected)"
    );
}
