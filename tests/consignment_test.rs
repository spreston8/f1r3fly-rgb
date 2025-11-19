//! F1r3flyConsignment Integration Tests
//!
//! Tests the consignment creation, serialization, and validation with live F1r3node.
//! Focuses on transfer package integrity and cryptographic verification.
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
//! Run with: cargo test --test consignment_test -- --nocapture

use amplify::confinement::SmallOrdMap;
use bp::seals::{Anchor, WTxoSeal};
use bp::{Tx, Vout};
use commit_verify::{Digest, DigestExt, Sha256};
use f1r3fly_rgb::{
    create_tapret_anchor, F1r3flyConsignment, F1r3flyExecutor, F1r3flyRgbContract, StrictVal,
};
use rgb::Opid;
use rgb::Pile;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use strict_types::StrictDumb;

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

/// Create test seals for Bitcoin UTXO binding
fn create_test_seals(count: u16, vout_offset: u32) -> SmallOrdMap<u16, WTxoSeal> {
    let mut seals = SmallOrdMap::new();
    let mut noise = Sha256::new();
    noise.input_raw(b"consignment_test_seals");
    noise.input_raw(&vout_offset.to_le_bytes());

    for i in 0..count {
        let mut seal_noise = noise.clone();
        seal_noise.input_raw(&i.to_le_bytes());

        let vout = vout_offset + (i as u32);
        let seal = WTxoSeal::vout_no_fallback(Vout::from_u32(vout), seal_noise, i as u64);
        let _ = seals.insert(i, seal);
    }

    seals
}

/// Create a dummy anchor for failure tests (no Tapret proof)
fn create_dummy_anchor() -> Anchor {
    Anchor::strict_dumb()
}

#[tokio::test]
async fn test_consignment_happy_path_with_validation() {
    load_env();

    // Step 1: Deploy contract
    let mut executor = F1r3flyExecutor::new().expect("Failed to create F1r3flyExecutor");
    executor.set_derivation_index(test_derivation_offset(
        "test_consignment_happy_path_with_validation",
    ));

    let mut contract =
        F1r3flyRgbContract::issue(executor, "CONS1", "Consignment Test Token 1", 1_000_000, 8)
            .await
            .expect("Failed to deploy contract");

    // Step 2: Create seals and call method with seals
    let seals = create_test_seals(1, 5000);

    let issue_params = &[
        ("recipient", StrictVal::from("test_recipient")),
        ("amount", StrictVal::from(500_000u64)),
    ];

    let issue_result = contract
        .call_method("issue", issue_params, seals.clone())
        .await
        .expect("Issue failed");

    // Step 3: Use state_hash from execution result as Opid (this is how consignment identifies operations)
    let opid = Opid::from(issue_result.state_hash);
    contract.tracker_mut().add_seals(opid, seals.clone());

    // Step 4: Create Tapret anchor with cryptographic proof
    let (anchor, witness_tx) = create_tapret_anchor(issue_result.state_hash)
        .expect("Tapret anchor creation should succeed");
    let txid = witness_tx.txid();
    let block_height = std::num::NonZeroU64::new(1).unwrap();

    // Step 5: Add witness transaction and anchor to tracker
    contract.tracker_mut().add_witness(
        opid,
        txid,
        &witness_tx,
        &anchor,
        rgb::WitnessStatus::Mined(block_height),
    );

    contract.tracker_mut().add_anchor(opid, anchor);

    // Step 6: Create consignment with real Bitcoin TX
    let witness_txs = vec![witness_tx.clone()];
    let consignment = F1r3flyConsignment::new(&contract, issue_result, seals.clone(), witness_txs, false)
        .expect("Failed to create consignment");

    // Step 7: Verify consignment structure
    assert_eq!(consignment.version, 1, "Version should be 1");
    assert_eq!(consignment.seals.len(), 1, "Should have 1 seal");
    assert_eq!(
        consignment.witness_txs.len(),
        1,
        "Should have 1 witness transaction"
    );
    assert!(
        !consignment.f1r3fly_proof.block_hash.is_empty(),
        "Block hash should not be empty"
    );
    assert!(
        !consignment.f1r3fly_proof.deploy_id.is_empty(),
        "Deploy ID should not be empty"
    );

    // Step 8: Serialize consignment
    let serialized = consignment
        .to_bytes()
        .expect("Failed to serialize consignment");
    assert!(
        serialized.len() > 100,
        "Serialized data should be substantial"
    );

    // Step 9: Deserialize consignment
    let deserialized =
        F1r3flyConsignment::from_bytes(&serialized).expect("Failed to deserialize consignment");

    // Step 10: Verify round-trip integrity
    assert_eq!(
        deserialized.contract_id, consignment.contract_id,
        "Contract ID should match"
    );
    assert_eq!(
        deserialized.version, consignment.version,
        "Version should match"
    );
    assert_eq!(
        deserialized.seals.len(),
        consignment.seals.len(),
        "Seal count should match"
    );
    assert_eq!(
        deserialized.witness_txs.len(),
        consignment.witness_txs.len(),
        "Witness TX count should match"
    );
    assert_eq!(
        deserialized.f1r3fly_proof.block_hash, consignment.f1r3fly_proof.block_hash,
        "Block hash should match"
    );

    // Step 11: Validate consignment (full cryptographic verification)
    // With real Tapret anchor and deploy_and_wait, validation MUST succeed
    deserialized
        .validate(contract.executor())
        .await
        .expect("Validation should succeed with real Tapret proof and finalized block");
}

#[tokio::test]
async fn test_consignment_fails_without_witness_transaction() {
    load_env();

    // Step 1: Deploy contract
    let mut executor = F1r3flyExecutor::new().expect("Failed to create F1r3flyExecutor");
    executor.set_derivation_index(test_derivation_offset(
        "test_consignment_fails_without_witness_transaction",
    ));

    let mut contract =
        F1r3flyRgbContract::issue(executor, "CONS2", "Consignment Test Token 2", 1_000_000, 8)
            .await
            .expect("Failed to deploy contract");

    // Step 2: Create seals and call method
    let seals = create_test_seals(1, 6000);

    let issue_params = &[
        ("recipient", StrictVal::from("test_recipient_2")),
        ("amount", StrictVal::from(250_000u64)),
    ];

    let issue_result = contract
        .call_method("issue", issue_params, seals.clone())
        .await
        .expect("Issue failed");

    // Step 3: Use state_hash from execution result as Opid
    let opid = Opid::from(issue_result.state_hash);

    // Add dummy anchor (for testing failure path)
    let anchor = create_dummy_anchor();
    contract.tracker_mut().add_anchor(opid, anchor);

    // Step 4: Try to create consignment WITHOUT witness transaction
    let empty_witness_txs = vec![]; // Empty!

    let consignment_result =
        F1r3flyConsignment::new(&contract, issue_result, seals.clone(), empty_witness_txs, false);

    // Should succeed creation (witness is optional at creation time)
    assert!(
        consignment_result.is_ok(),
        "Consignment creation should succeed even without witness"
    );

    let consignment = consignment_result.unwrap();

    // Step 5: Try to validate - should fail with specific error
    // Empty witness_txs means first() returns None
    let validation_result = consignment.validate(contract.executor()).await;

    match validation_result {
        Ok(_) => panic!("Validation should fail without witness transaction"),
        Err(f1r3fly_rgb::F1r3flyRgbError::InvalidConsignment(_msg)) => {
            // Expected: empty witness_txs causes validation to fail
        }
        Err(other) => panic!("Expected InvalidConsignment error, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_consignment_fails_without_anchor() {
    load_env();

    // Step 1: Deploy contract
    let mut executor = F1r3flyExecutor::new().expect("Failed to create F1r3flyExecutor");
    executor.set_derivation_index(test_derivation_offset(
        "test_consignment_fails_without_anchor",
    ));

    let mut contract =
        F1r3flyRgbContract::issue(executor, "CONS3", "Consignment Test Token 3", 1_000_000, 8)
            .await
            .expect("Failed to deploy contract");

    // Step 2: Create seals and call method
    let seals = create_test_seals(1, 7000);

    let issue_params = &[
        ("recipient", StrictVal::from("test_recipient_3")),
        ("amount", StrictVal::from(750_000u64)),
    ];

    let issue_result = contract
        .call_method("issue", issue_params, seals.clone())
        .await
        .expect("Issue failed");

    // Step 3: Create witness transaction but DON'T add anchor to tracker
    let dummy_tx = Tx::strict_dumb();
    let witness_txs = vec![dummy_tx];

    // Step 4: Try to create consignment WITHOUT anchor in tracker
    let consignment_result =
        F1r3flyConsignment::new(&contract, issue_result, seals.clone(), witness_txs, false);

    // Should fail with specific error about missing anchor
    match consignment_result {
        Ok(_) => panic!("Consignment creation should fail without anchor"),
        Err(f1r3fly_rgb::F1r3flyRgbError::InvalidConsignment(_msg)) => {
            // Expected: tracker.get_anchor() returns None
        }
        Err(other) => panic!("Expected InvalidConsignment error, got: {:?}", other),
    }
}
