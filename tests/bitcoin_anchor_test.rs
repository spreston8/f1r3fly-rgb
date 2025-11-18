//! Comprehensive tests for BitcoinAnchorTracker
//!
//! Tests cover:
//! - Pile trait lifecycle (new, load, commit)
//! - Witness and seal management
//! - Persistence (automatic and manual)
//! - Edge cases (empty state, missing data, RBF)
//! - Complex scenarios (multiple ops/witnesses)
//!
//! ## Temporary File Cleanup
//!
//! All tests using `tempfile::TempDir` automatically clean up temporary directories
//! on drop, even if the test panics or fails. Cleanup is verified explicitly at the
//! end of each test to ensure proper resource management in all scenarios.

use amplify::confinement::SmallOrdMap;
use amplify::ByteArray;
use bp::seals::{Anchor, Noise, TxoSealExt, WOutpoint};
use bp::{Outpoint, Vout};
use f1r3fly_rgb::{
    AnchorConfig, BitcoinAnchorTracker, Pile, Tx, Txid, TxoSeal, WTxoSeal, WitnessStatus,
};
use rgb::{CellAddr, Opid}; // Import from rgb-std (which re-exports from ultrasonic)
use std::num::NonZero;
use strict_types::StrictDumb;
use tempfile::TempDir;

// ============================================================================
// Test Helpers
// ============================================================================

/// Create a test Opid from a simple counter
fn test_opid(counter: u8) -> Opid {
    let mut bytes = [0u8; 32];
    bytes[0] = counter;
    Opid::from(bytes)
}

/// Create a test Txid from a simple counter
fn test_txid(counter: u8) -> Txid {
    let mut bytes = [0u8; 32];
    bytes[0] = counter;
    Txid::from_byte_array(bytes)
}

/// Create a test WTxoSeal (witnessed seal)
fn test_seal(txid: Txid, vout: u32) -> WTxoSeal {
    WTxoSeal {
        primary: WOutpoint::Extern(Outpoint::new(txid, Vout::from_u32(vout))),
        secondary: TxoSealExt::Noise(Noise::strict_dumb()),
    }
}

/// Create a test witness transaction (using Tx::strict_dumb for simplicity)
fn test_tx() -> Tx {
    Tx::strict_dumb()
}

/// Create a test anchor (using Anchor::strict_dumb for simplicity)
fn test_anchor() -> Anchor {
    Anchor::strict_dumb()
}

// ============================================================================
// Test 1: Pile Trait Lifecycle - New, Load, Commit
// ============================================================================

#[test]
fn test_pile_lifecycle_with_persistence() {
    // TempDir automatically cleans up on drop, even on test failure/panic
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("tracker.json");
    let temp_path = temp_dir.path().to_path_buf();

    // Create with persistence config
    let conf = AnchorConfig {
        persistence_path: Some(db_path.clone()),
    };

    let mut tracker = <BitcoinAnchorTracker<TxoSeal> as Pile>::new(conf.clone()).unwrap();
    assert_eq!(tracker.operation_count(), 0);
    assert_eq!(tracker.witness_count(), 0);
    assert_eq!(tracker.persistence_path(), Some(db_path.as_path()));

    // Add some data
    let opid = test_opid(1);
    let txid = test_txid(1);
    let seal = test_seal(txid, 0);
    let mut seals = SmallOrdMap::new();
    seals.insert(0u16, seal).unwrap();

    tracker.add_seals(opid, seals);
    tracker.add_witness(
        opid,
        txid,
        &test_tx(),
        &test_anchor(),
        WitnessStatus::Tentative,
    );

    // Add anchor after witness (simulates PSBT finalization)
    let anchor = test_anchor();
    tracker.add_anchor(opid, anchor.clone());
    assert!(tracker.has_anchor(&opid));
    assert!(tracker.get_anchor(&opid).is_some());

    // Commit should auto-save (including anchors)
    tracker.commit_transaction();
    assert!(db_path.exists(), "Auto-save failed");

    // Load from disk
    let loaded = BitcoinAnchorTracker::<TxoSeal>::load(conf).unwrap();
    assert_eq!(loaded.operation_count(), 1);
    assert_eq!(loaded.witness_count(), 1);
    assert_eq!(loaded.persistence_path(), Some(db_path.as_path()));
    assert!(loaded.has_witness(txid));

    // Verify anchor persisted
    assert!(loaded.has_anchor(&opid), "Anchor should be persisted");
    assert!(
        loaded.get_anchor(&opid).is_some(),
        "Should retrieve persisted anchor"
    );

    // Verify cleanup: Drop temp_dir explicitly and verify directory is removed
    drop(temp_dir);
    assert!(
        !temp_path.exists(),
        "TempDir should be cleaned up after drop"
    );
}

// ============================================================================
// Test 2: Witness Status Progression (Tentative → Mined → Archived)
// ============================================================================

#[test]
fn test_witness_status_progression() {
    let mut tracker = BitcoinAnchorTracker::<TxoSeal>::new();
    let opid = test_opid(1);
    let txid = test_txid(1);

    // Add witness as tentative
    tracker.add_witness(
        opid,
        txid,
        &test_tx(),
        &test_anchor(),
        WitnessStatus::Tentative,
    );
    assert_eq!(tracker.witness_status(txid), WitnessStatus::Tentative);

    // Update to mined with 1 confirmation
    tracker.update_witness_status(txid, WitnessStatus::Mined(NonZero::new(1).unwrap()));
    assert_eq!(
        tracker.witness_status(txid),
        WitnessStatus::Mined(NonZero::new(1).unwrap())
    );

    // Update to mined with 6 confirmations (considered safe)
    tracker.update_witness_status(txid, WitnessStatus::Mined(NonZero::new(6).unwrap()));
    assert_eq!(
        tracker.witness_status(txid),
        WitnessStatus::Mined(NonZero::new(6).unwrap())
    );

    // Update to archived (spent)
    tracker.update_witness_status(txid, WitnessStatus::Archived);
    assert_eq!(tracker.witness_status(txid), WitnessStatus::Archived);
}

// ============================================================================
// Test 3: RBF (Replace-By-Fee) Scenario
// ============================================================================

#[test]
fn test_rbf_scenario() {
    let mut tracker = BitcoinAnchorTracker::<TxoSeal>::new();
    let opid = test_opid(1);
    let original_txid = test_txid(1);
    let replacement_txid = test_txid(2);

    // Original transaction (low fee, tentative)
    tracker.add_witness(
        opid,
        original_txid,
        &test_tx(),
        &test_anchor(),
        WitnessStatus::Tentative,
    );
    assert!(tracker.has_witness(original_txid));
    assert_eq!(
        tracker.witness_status(original_txid),
        WitnessStatus::Tentative
    );

    // Add anchor for original transaction
    let original_anchor = test_anchor();
    tracker.add_anchor(opid, original_anchor);
    assert!(tracker.has_anchor(&opid));

    // RBF replacement transaction (higher fee, tentative)
    tracker.add_witness(
        opid,
        replacement_txid,
        &test_tx(),
        &test_anchor(),
        WitnessStatus::Tentative,
    );
    assert!(tracker.has_witness(replacement_txid));

    // Remove old anchor and add new one (RBF updates the anchor)
    let removed = tracker.remove_anchor(&opid);
    assert!(removed.is_some(), "Should remove old anchor");

    let replacement_anchor = test_anchor();
    tracker.add_anchor(opid, replacement_anchor);
    assert!(tracker.has_anchor(&opid), "Should have new anchor");

    // Original should be marked as replaced
    tracker.update_witness_status(original_txid, WitnessStatus::Archived);
    assert_eq!(
        tracker.witness_status(original_txid),
        WitnessStatus::Archived
    );

    // Replacement gets mined
    tracker.update_witness_status(
        replacement_txid,
        WitnessStatus::Mined(NonZero::new(1).unwrap()),
    );
    assert_eq!(
        tracker.witness_status(replacement_txid),
        WitnessStatus::Mined(NonZero::new(1).unwrap())
    );

    // Both witnesses should be linked to the same operation
    let witness_ids: Vec<_> = tracker.op_witness_ids(opid).collect();
    assert_eq!(witness_ids.len(), 2);
    assert!(witness_ids.contains(&original_txid));
    assert!(witness_ids.contains(&replacement_txid));
}

// ============================================================================
// Test 4: Multiple Operations with Shared Witness (Batching)
// ============================================================================

#[test]
fn test_batch_operations_shared_witness() {
    let mut tracker = BitcoinAnchorTracker::<TxoSeal>::new();

    // Three operations batched into one Bitcoin transaction
    let opid1 = test_opid(1);
    let opid2 = test_opid(2);
    let opid3 = test_opid(3);
    let batch_txid = test_txid(100);

    // Each operation has its own seals
    let seal1 = test_seal(batch_txid, 0);
    let seal2 = test_seal(batch_txid, 1);
    let seal3 = test_seal(batch_txid, 2);

    let mut seals1 = SmallOrdMap::new();
    seals1.insert(0u16, seal1).unwrap();
    tracker.add_seals(opid1, seals1);

    let mut seals2 = SmallOrdMap::new();
    seals2.insert(0u16, seal2).unwrap();
    tracker.add_seals(opid2, seals2);

    let mut seals3 = SmallOrdMap::new();
    seals3.insert(0u16, seal3).unwrap();
    tracker.add_seals(opid3, seals3);

    // All operations share the same witness transaction
    let tx = test_tx();
    let anchor = test_anchor();
    tracker.add_witness(
        opid1,
        batch_txid,
        &tx,
        &anchor,
        WitnessStatus::Mined(NonZero::new(6).unwrap()),
    );
    tracker.add_witness(
        opid2,
        batch_txid,
        &tx,
        &anchor,
        WitnessStatus::Mined(NonZero::new(6).unwrap()),
    );
    tracker.add_witness(
        opid3,
        batch_txid,
        &tx,
        &anchor,
        WitnessStatus::Mined(NonZero::new(6).unwrap()),
    );

    // Verify all operations link to the same witness
    assert_eq!(
        tracker.op_witness_ids(opid1).collect::<Vec<_>>(),
        vec![batch_txid]
    );
    assert_eq!(
        tracker.op_witness_ids(opid2).collect::<Vec<_>>(),
        vec![batch_txid]
    );
    assert_eq!(
        tracker.op_witness_ids(opid3).collect::<Vec<_>>(),
        vec![batch_txid]
    );

    // Verify witness links back to all operations
    let ops: Vec<_> = tracker.ops_by_witness_id(batch_txid).collect();
    assert_eq!(ops.len(), 3);
    assert!(ops.contains(&opid1));
    assert!(ops.contains(&opid2));
    assert!(ops.contains(&opid3));
}

// ============================================================================
// Test 5: Genesis Operation (No Witnesses)
// ============================================================================

#[test]
fn test_genesis_operation_no_witnesses() {
    let mut tracker = BitcoinAnchorTracker::<TxoSeal>::new();
    let genesis_opid = test_opid(0);

    // Genesis has seals but no witnesses (creates assets from nothing)
    let txid = test_txid(1);
    let seal = test_seal(txid, 0);
    let mut seals = SmallOrdMap::new();
    seals.insert(0u16, seal).unwrap();

    tracker.add_seals(genesis_opid, seals.clone());

    // Should have seals but no witnesses
    assert_eq!(tracker.operation_count(), 1);
    assert_eq!(tracker.witness_count(), 0);
    assert_eq!(tracker.op_witness_ids(genesis_opid).count(), 0);

    // Can retrieve seals
    let retrieved_seals = tracker.seals(genesis_opid, 0);
    assert_eq!(retrieved_seals.len(), 1);
    assert_eq!(retrieved_seals.get(&0), seals.get(&0));
}

// ============================================================================
// Test 6: ExactSizeIterator Contract Compliance
// ============================================================================

#[test]
fn test_exact_size_iterator_contract() {
    let mut tracker = BitcoinAnchorTracker::<TxoSeal>::new();
    let opid = test_opid(1);

    // Add multiple witnesses for one operation
    let txid1 = test_txid(1);
    let txid2 = test_txid(2);
    let txid3 = test_txid(3);

    tracker.add_witness(
        opid,
        txid1,
        &test_tx(),
        &test_anchor(),
        WitnessStatus::Tentative,
    );
    tracker.add_witness(
        opid,
        txid2,
        &test_tx(),
        &test_anchor(),
        WitnessStatus::Tentative,
    );
    tracker.add_witness(
        opid,
        txid3,
        &test_tx(),
        &test_anchor(),
        WitnessStatus::Mined(NonZero::new(1).unwrap()),
    );

    // Test op_witness_ids returns ExactSizeIterator
    let mut iter = tracker.op_witness_ids(opid);
    assert_eq!(iter.len(), 3);
    assert_eq!(iter.size_hint(), (3, Some(3)));

    iter.next();
    assert_eq!(iter.len(), 2);
    assert_eq!(iter.size_hint(), (2, Some(2)));

    // Test ops_by_witness_id returns ExactSizeIterator
    let mut iter = tracker.ops_by_witness_id(txid1);
    assert_eq!(iter.len(), 1);
    assert_eq!(iter.size_hint(), (1, Some(1)));

    iter.next();
    assert_eq!(iter.len(), 0);
    assert_eq!(iter.size_hint(), (0, Some(0)));
}

// ============================================================================
// Test 7: Empty State Queries (Edge Cases)
// ============================================================================

#[test]
fn test_empty_state_queries() {
    let tracker = BitcoinAnchorTracker::<TxoSeal>::new();
    let nonexistent_opid = test_opid(99);
    let nonexistent_txid = test_txid(99);

    // Empty tracker should handle queries gracefully
    assert_eq!(tracker.operation_count(), 0);
    assert_eq!(tracker.witness_count(), 0);
    assert!(!tracker.has_witness(nonexistent_txid));
    assert_eq!(
        tracker.witness_status(nonexistent_txid),
        WitnessStatus::Archived
    );

    // Iterators should be empty
    assert_eq!(tracker.witness_ids().count(), 0);
    assert_eq!(tracker.witnesses().count(), 0);
    assert_eq!(tracker.op_witness_ids(nonexistent_opid).count(), 0);
    assert_eq!(tracker.ops_by_witness_id(nonexistent_txid).count(), 0);

    // Seal queries should return empty/None
    let addr = CellAddr {
        opid: nonexistent_opid,
        pos: 0,
    };
    assert!(tracker.seal(addr).is_none());
    assert_eq!(tracker.seals(nonexistent_opid, 10).len(), 0);
}

// ============================================================================
// Test 8: Persistence Round-Trip (Manual Save/Load)
// ============================================================================

#[test]
fn test_manual_persistence_round_trip() {
    // TempDir automatically cleans up on drop, even on test failure/panic
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("manual_save.json");
    let temp_path = temp_dir.path().to_path_buf();

    // Create tracker and add complex state
    let mut tracker = BitcoinAnchorTracker::<TxoSeal>::new();

    // Multiple operations with multiple outputs
    let opid1 = test_opid(1);
    let opid2 = test_opid(2);

    // Operation 1: 3 outputs
    let mut seals1 = SmallOrdMap::new();
    seals1.insert(0u16, test_seal(test_txid(10), 0)).unwrap();
    seals1.insert(1u16, test_seal(test_txid(10), 1)).unwrap();
    seals1.insert(2u16, test_seal(test_txid(10), 2)).unwrap();
    tracker.add_seals(opid1, seals1);

    // Operation 2: 2 outputs
    let mut seals2 = SmallOrdMap::new();
    seals2.insert(0u16, test_seal(test_txid(20), 0)).unwrap();
    seals2.insert(1u16, test_seal(test_txid(20), 1)).unwrap();
    tracker.add_seals(opid2, seals2);

    // Add witnesses
    tracker.add_witness(
        opid1,
        test_txid(10),
        &test_tx(),
        &test_anchor(),
        WitnessStatus::Mined(NonZero::new(6).unwrap()),
    );
    tracker.add_witness(
        opid2,
        test_txid(20),
        &test_tx(),
        &test_anchor(),
        WitnessStatus::Tentative,
    );

    // Add anchors for each operation
    let anchor1 = test_anchor();
    let anchor2 = test_anchor();
    tracker.add_anchor(opid1, anchor1);
    tracker.add_anchor(opid2, anchor2);
    assert!(tracker.has_anchor(&opid1));
    assert!(tracker.has_anchor(&opid2));

    // Save manually
    tracker.save(&db_path).unwrap();
    assert!(db_path.exists());

    // Load and verify
    let loaded = BitcoinAnchorTracker::<TxoSeal>::load_from_disk(&db_path).unwrap();
    assert_eq!(loaded.operation_count(), 2);
    assert_eq!(loaded.witness_count(), 2);

    // Verify seals preserved
    assert_eq!(loaded.seals(opid1, 10).len(), 3);
    assert_eq!(loaded.seals(opid2, 10).len(), 2);

    // Verify witnesses preserved
    assert!(loaded.has_witness(test_txid(10)));
    assert!(loaded.has_witness(test_txid(20)));
    assert_eq!(
        loaded.witness_status(test_txid(10)),
        WitnessStatus::Mined(NonZero::new(6).unwrap())
    );
    assert_eq!(
        loaded.witness_status(test_txid(20)),
        WitnessStatus::Tentative
    );

    // Verify anchors preserved
    assert!(loaded.has_anchor(&opid1), "Anchor 1 should be persisted");
    assert!(loaded.has_anchor(&opid2), "Anchor 2 should be persisted");
    assert!(
        loaded.get_anchor(&opid1).is_some(),
        "Should retrieve anchor 1"
    );
    assert!(
        loaded.get_anchor(&opid2).is_some(),
        "Should retrieve anchor 2"
    );

    // Verify loaded tracker has auto-persistence enabled
    assert_eq!(loaded.persistence_path(), Some(db_path.as_path()));

    // Verify cleanup: Drop temp_dir explicitly and verify directory is removed
    drop(temp_dir);
    assert!(
        !temp_path.exists(),
        "TempDir should be cleaned up after drop"
    );
}

// ============================================================================
// Test 9: OpRels (Operation Relations) - Full Data Retrieval
// ============================================================================

#[test]
fn test_op_relations_comprehensive() {
    let mut tracker = BitcoinAnchorTracker::<TxoSeal>::new();
    let opid = test_opid(1);

    // Add 5 seals for this operation
    let mut seals = SmallOrdMap::new();
    for i in 0..5u16 {
        seals
            .insert(i, test_seal(test_txid(100), i as u32))
            .unwrap();
    }
    tracker.add_seals(opid, seals);

    // Add 2 witnesses (e.g., original + RBF replacement)
    let txid1 = test_txid(1);
    let txid2 = test_txid(2);
    tracker.add_witness(
        opid,
        txid1,
        &test_tx(),
        &test_anchor(),
        WitnessStatus::Archived,
    );
    tracker.add_witness(
        opid,
        txid2,
        &test_tx(),
        &test_anchor(),
        WitnessStatus::Mined(NonZero::new(6).unwrap()),
    );

    // Get full relations (up to output 3, so should get outputs 0,1,2,3)
    let relations = tracker.op_relations(opid, 3);

    assert_eq!(relations.opid, opid);
    assert_eq!(relations.witness_ids.len(), 2);
    assert!(relations.witness_ids.contains(&txid1));
    assert!(relations.witness_ids.contains(&txid2));
    assert_eq!(relations.defines.len(), 4); // 0, 1, 2, 3

    // Get relations with up_to = 10 (should get all 5 seals)
    let relations_all = tracker.op_relations(opid, 10);
    assert_eq!(relations_all.defines.len(), 5);
}

// ============================================================================
// Test 10: Concurrent Modifications (Stress Test)
// ============================================================================

#[test]
fn test_large_scale_operations() {
    // TempDir automatically cleans up on drop, even on test failure/panic
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("large_dataset.json");
    let temp_path = temp_dir.path().to_path_buf();

    let mut tracker = BitcoinAnchorTracker::<TxoSeal>::new();

    // Add 100 operations with 10 outputs each = 1000 seals
    for op_counter in 0..100u8 {
        let opid = test_opid(op_counter);
        let mut seals = SmallOrdMap::new();

        for output_idx in 0..10u16 {
            let seal = test_seal(test_txid(op_counter), output_idx as u32);
            seals.insert(output_idx, seal).unwrap();
        }

        tracker.add_seals(opid, seals);

        // Every 10 operations share a witness (batching)
        let batch_txid = test_txid(op_counter / 10);
        tracker.add_witness(
            opid,
            batch_txid,
            &test_tx(),
            &test_anchor(),
            WitnessStatus::Mined(NonZero::new(6).unwrap()),
        );
    }

    // Verify scale
    assert_eq!(tracker.operation_count(), 100);
    assert_eq!(tracker.witness_count(), 10); // 100 ops / 10 per batch

    // Verify batch witness has 10 operations
    let first_batch_txid = test_txid(0);
    assert_eq!(tracker.ops_by_witness_id(first_batch_txid).count(), 10);

    // Verify random operation has correct data
    let random_opid = test_opid(50);
    let seals = tracker.seals(random_opid, 100);
    assert_eq!(seals.len(), 10);

    // Test persistence of large dataset
    tracker.save(&db_path).unwrap();

    // Verify file exists and is reasonably sized (not a stub)
    let metadata = std::fs::metadata(&db_path).unwrap();
    assert!(
        metadata.len() > 1000,
        "Persisted data should be substantial"
    );

    // Verify can load back
    let loaded = BitcoinAnchorTracker::<TxoSeal>::load_from_disk(&db_path).unwrap();
    assert_eq!(loaded.operation_count(), 100);
    assert_eq!(loaded.witness_count(), 10);

    // Verify cleanup: Drop temp_dir explicitly and verify directory is removed
    drop(temp_dir);
    assert!(
        !temp_path.exists(),
        "TempDir should be cleaned up after drop"
    );
}
