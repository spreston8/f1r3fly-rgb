//! Tapret commitment utilities for F1r3fly state hashes
//!
//! Per integration plan Phase 3, Step 3.1: This module provides utilities
//! for embedding F1r3fly state hashes in Bitcoin transactions via Tapret commitments.

use bp::dbc::tapret::TapretProof;
use bp::seals::{mmb, mpc, Anchor};
use bpstd::psbt::Psbt;
use commit_verify::{mpc as mpc_cv, ReservedBytes};
use strict_encoding::StrictDumb;

pub type Result<T> = std::result::Result<T, TapretError>;

#[derive(Debug)]
pub enum TapretError {
    NotTaprootOutput,
    InvalidOutputIndex { index: usize, max: usize },
    CommitmentFailed(String),
}

impl std::fmt::Display for TapretError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotTaprootOutput => write!(f, "Output is not a taproot output"),
            Self::InvalidOutputIndex { index, max } => {
                write!(f, "Invalid output index {} (max: {})", index, max)
            }
            Self::CommitmentFailed(msg) => write!(f, "Tapret commitment failed: {}", msg),
        }
    }
}

impl std::error::Error for TapretError {}

/// Create a PSBT with Taproot output for Tapret commitment
///
/// This creates a valid PSBT structure that can receive Tapret commitments.
/// Useful for testing and wallet integration.
pub fn create_test_psbt_with_taproot() -> Psbt {
    use amplify::confinement::Confined;
    use bp::{secp256k1, LockTime, Sats, Tx, TxOut};
    use bpstd::ScriptPubkey;

    // Create valid P2TR script pubkey (34 bytes total)
    // Format: OP_1 (0x51) + OP_PUSHBYTES_32 (0x20) + 32-byte x-only pubkey
    let mut script_bytes = vec![0x51, 0x20]; // OP_1 + OP_PUSHBYTES_32
    script_bytes.extend_from_slice(&[0u8; 32]); // 32-byte x-only pubkey (dummy)

    let script = ScriptPubkey::from_checked(script_bytes);

    let output = TxOut {
        value: Sats::from(100_000u64),
        script_pubkey: script,
    };

    // Create transaction
    let tx = Tx {
        version: bp::TxVer::V2,
        inputs: Confined::try_from(vec![]).unwrap(),
        outputs: Confined::try_from(vec![output]).unwrap(),
        lock_time: LockTime::ZERO,
    };

    // Create PSBT from transaction
    let mut psbt = Psbt::from_tx(tx);

    // Set internal key for Taproot (required for Tapret commitments)
    if let Some(output) = psbt.outputs_mut().next() {
        let secp = secp256k1::Secp256k1::new();
        let secret_key = secp256k1::SecretKey::from_slice(&[1u8; 32]).expect("Valid secret key");
        let public_key = secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
        let (internal_key, _parity) = public_key.x_only_public_key();
        output.tap_internal_key = Some(internal_key.into());
    }

    psbt
}

/// Create an anchor with Tapret proof for a given state hash
///
/// Creates a valid PSBT, embeds Tapret commitment, and returns a proper
/// Anchor with cryptographically valid dbc_proof.
///
/// # Arguments
///
/// * `state_hash` - The F1r3fly state hash to commit
///
/// # Returns
///
/// Tuple of (Anchor, Tx) where:
/// - Anchor contains the Tapret proof for validation
/// - Tx is the witness transaction for the consignment
///
/// # Example
///
/// ```rust,no_run
/// use f1r3fly_rgb::create_tapret_anchor;
///
/// let state_hash = [0x42u8; 32];
/// let (anchor, witness_tx) = create_tapret_anchor(state_hash).expect("Failed to create anchor");
/// ```
pub fn create_tapret_anchor(state_hash: [u8; 32]) -> Result<(Anchor, bp::Tx)> {
    use bp::Tx;

    // Step 1: Create PSBT with Taproot output
    let mut psbt = create_test_psbt_with_taproot();

    // Step 2: Embed Tapret commitment with state hash
    let tapret_proof = embed_tapret_commitment(&mut psbt, 0, state_hash)?;

    // Step 3: Create anchor with Tapret proof
    let anchor = create_anchor(&tapret_proof)?;

    // Step 4: Extract transaction for witness
    // Convert PSBT back to Tx for witness storage
    let tx: Tx = psbt.to_unsigned_tx().into();

    Ok((anchor, tx))
}

/// Embed F1r3fly state hash as Tapret commitment in PSBT output
///
/// Per integration plan Step 3.1, this embeds a 32-byte F1r3fly state hash
/// into a Bitcoin PSBT output using RGB's Tapret commitment mechanism.
///
/// # Arguments
/// * `psbt` - Mutable PSBT to modify
/// * `output_index` - Which output gets the commitment (typically 0)
/// * `state_hash` - F1r3fly state hash (32 bytes)
///
/// # Returns
/// TapretProof for later Anchor creation
///
/// # Errors
/// Returns error if:
/// - Output index is out of bounds
/// - Output is not a taproot output
/// - Tapret commitment fails
pub fn embed_tapret_commitment(
    psbt: &mut Psbt,
    output_index: usize,
    state_hash: [u8; 32],
) -> Result<TapretProof> {
    // Get mutable output
    let mut outputs: Vec<_> = psbt.outputs_mut().collect();

    if output_index >= outputs.len() {
        return Err(TapretError::InvalidOutputIndex {
            index: output_index,
            max: outputs.len(),
        });
    }

    let output = &mut outputs[output_index];

    // Check if taproot
    if !output.script.is_p2tr() {
        return Err(TapretError::NotTaprootOutput);
    }

    // Mark output as Tapret host
    output
        .set_tapret_host()
        .map_err(|e| TapretError::CommitmentFailed(format!("set_tapret_host failed: {:?}", e)))?;

    // Create commitment from state hash
    let commitment = mpc_cv::Commitment::from(state_hash);

    // Embed Tapret commitment
    let proof = output
        .tapret_commit(commitment)
        .map_err(|e| TapretError::CommitmentFailed(format!("tapret_commit failed: {:?}", e)))?;

    log::info!("✅ Tapret commitment embedded");
    log::debug!("   State hash: {}", hex::encode(state_hash));
    log::debug!("   Output index: {}", output_index);

    Ok(proof)
}

/// Extract Tapret commitment from PSBT output
///
/// Reads the embedded F1r3fly state hash from a PSBT output that has a Tapret commitment.
/// Useful for post-embedding verification and auditing.
///
/// # Arguments
/// * `psbt` - PSBT containing the Tapret commitment
/// * `output_index` - Which output to extract from (typically 0)
///
/// # Returns
/// The 32-byte state hash that was embedded
///
/// # Errors
/// Returns error if:
/// - Output index is out of bounds
/// - Output is not a taproot output
/// - No Tapret commitment found
/// - Commitment extraction fails
pub fn extract_tapret_commitment(psbt: &Psbt, output_index: usize) -> Result<[u8; 32]> {
    // Get output
    let outputs: Vec<_> = psbt.outputs().collect();

    if output_index >= outputs.len() {
        return Err(TapretError::InvalidOutputIndex {
            index: output_index,
            max: outputs.len(),
        });
    }

    let output = &outputs[output_index];

    // Check if taproot
    if !output.script.is_p2tr() {
        return Err(TapretError::NotTaprootOutput);
    }

    // Extract Tapret commitment from output
    let commitment = output
        .tapret_commitment()
        .map_err(|e| TapretError::CommitmentFailed(format!("tapret_commitment failed: {:?}", e)))?;

    // Convert commitment to 32-byte array (extract MPC commitment)
    let commitment_bytes = commitment.mpc.to_byte_array();

    log::debug!("✅ Tapret commitment extracted");
    log::debug!("   State hash: {}", hex::encode(commitment_bytes));
    log::debug!("   Output index: {}", output_index);

    Ok(commitment_bytes)
}

/// Verify Tapret commitment in PSBT or finalized transaction
///
/// Validates that a Tapret proof correctly commits to the expected F1r3fly state hash.
/// Performs full cryptographic verification if a finalized transaction is available (post-signing),
/// or commitment value verification for PSBTs (pre-signing).
///
/// # Arguments
/// * `psbt` - PSBT containing the commitment
/// * `output_index` - Which output to verify (typically 0)
/// * `state_hash` - The F1r3fly state hash we expect to be committed
/// * `proof` - The Tapret proof from `embed_tapret_commitment()`
///
/// # Returns
/// `Ok(())` if commitment is valid, error otherwise
///
/// # Example
/// ```no_run
/// use f1r3fly_rgb::tapret::{embed_tapret_commitment, verify_tapret_commitment};
///
/// # fn example(mut psbt: bpstd::psbt::Psbt, state_hash: [u8; 32]) -> Result<(), Box<dyn std::error::Error>> {
/// let proof = embed_tapret_commitment(&mut psbt, 0, state_hash)?;
///
/// // Verify before signing (value check only)
/// verify_tapret_commitment(&psbt, 0, state_hash, &proof)?;
///
/// // After signing, full cryptographic verification happens automatically
/// // wallet.sign_psbt(&mut psbt)?;
/// verify_tapret_commitment(&psbt, 0, state_hash, &proof)?; // Full verification
/// # Ok(())
/// # }
/// ```
pub fn verify_tapret_commitment(
    psbt: &Psbt,
    output_index: usize,
    expected_state_hash: [u8; 32],
    proof: &TapretProof,
) -> Result<()> {
    // Extract commitment from PSBT
    let extracted_hash = extract_tapret_commitment(psbt, output_index)?;

    // Verify extracted hash matches expected
    if extracted_hash != expected_state_hash {
        return Err(TapretError::CommitmentFailed(format!(
            "Commitment mismatch: expected {}, found {}",
            hex::encode(expected_state_hash),
            hex::encode(extracted_hash)
        )));
    }

    // If PSBT can be extracted to transaction, perform full cryptographic verification
    if let Ok(tx) = psbt.extract() {
        verify_tapret_proof_in_tx(&tx, expected_state_hash, proof)?;
        log::info!("✅ Tapret commitment verified (complete)");
    } else {
        log::info!("✅ Tapret commitment verified (partial - PSBT not finalized)");
        log::warn!("   ⚠️  Full verification will occur after PSBT is signed");
    }

    Ok(())
}

/// Verify Tapret proof cryptographically in finalized transaction
///
/// Performs full cryptographic verification: internal key, merkle path, and commitment embedding.
/// Use this after extracting a signed transaction from a PSBT, or when you have a mined transaction.
pub fn verify_tapret_proof_in_tx(
    tx: &bp::Tx,
    state_hash: [u8; 32],
    proof: &TapretProof,
) -> Result<()> {
    let commitment = mpc_cv::Commitment::from(state_hash);

    proof.verify(&commitment, tx).map_err(|e| {
        TapretError::CommitmentFailed(format!("Tapret proof verification failed: {:?}", e))
    })?;

    log::debug!("✅ Tapret proof cryptographically verified");
    log::debug!("   State hash: {}", hex::encode(state_hash));
    log::debug!("   Txid: {}", tx.txid());

    Ok(())
}

/// Create RGB Anchor from Tapret proof
///
/// Creates an Anchor for F1r3fly-RGB's light integration architecture.
/// Uses a real Tapret proof (genuine Bitcoin commitment) with placeholder
/// MPC/MMB proofs since F1r3fly-RGB doesn't require full RGB client-side validation.
///
/// # Architecture Note
/// F1r3fly-RGB uses F1r3node for contract execution and state management,
/// while Bitcoin provides censorship-resistant anchoring via Tapret commitments.
/// This hybrid approach means:
/// - **Tapret proof**: Real (embeds F1r3fly state hash in Bitcoin)
/// - **MPC/MMB proofs**: Placeholders (validation happens on F1r3node, not client-side)
///
/// # Arguments
/// * `proof` - Tapret proof from `embed_tapret_commitment`
///
/// # Returns
/// RGB Anchor suitable for storage in BitcoinAnchorTracker
pub fn create_anchor(proof: &TapretProof) -> Result<Anchor> {
    Ok(Anchor {
        mmb_proof: mmb::BundleProof::strict_dumb(),
        mpc_protocol: mpc::ProtocolId::strict_dumb(),
        mpc_proof: mpc::MerkleProof::strict_dumb(),
        dbc_proof: Some(proof.clone()),
        fallback_proof: ReservedBytes::strict_dumb(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use amplify::confinement::Confined;
    use bp::{secp256k1, LockTime, Sats, Tx, TxOut};
    use bpstd::ScriptPubkey;

    /// Create a test PSBT with a single Taproot output
    fn create_test_psbt_with_taproot() -> Psbt {
        // Create a simple unsigned transaction with one Taproot output
        let tx = create_test_tx_with_taproot();

        // Create PSBT from transaction
        let mut psbt = Psbt::from_tx(tx);

        // Set internal key for Taproot (required for Tapret commitments)
        // Generate a valid dummy x-only pubkey from a test secret key
        if let Some(output) = psbt.outputs_mut().next() {
            let secp = secp256k1::Secp256k1::new();
            let secret_key =
                secp256k1::SecretKey::from_slice(&[1u8; 32]).expect("Valid secret key");
            let public_key = secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
            let (internal_key, _parity) = public_key.x_only_public_key();
            output.tap_internal_key = Some(internal_key.into());
        }

        psbt
    }

    /// Create a test transaction with a Taproot output
    fn create_test_tx_with_taproot() -> Tx {
        // Create valid P2TR script pubkey (34 bytes total)
        // Format: OP_1 (0x51) + OP_PUSHBYTES_32 (0x20) + 32-byte x-only pubkey
        let mut script_bytes = vec![0x51, 0x20]; // OP_1 + OP_PUSHBYTES_32
        script_bytes.extend_from_slice(&[0u8; 32]); // 32-byte x-only pubkey (dummy)

        let script = ScriptPubkey::from_checked(script_bytes);

        let output = TxOut {
            value: Sats::from(10000u64),
            script_pubkey: script,
        };

        Tx {
            version: bp::TxVer::V2,
            inputs: Confined::try_from(vec![]).unwrap(),
            outputs: Confined::try_from(vec![output]).unwrap(),
            lock_time: LockTime::ZERO,
        }
    }

    /// Create a test PSBT with a non-Taproot output (P2WPKH)
    fn create_test_psbt_with_p2wpkh() -> Psbt {
        // Create P2WPKH output (OP_0 + 20-byte pubkey hash)
        let mut script_bytes = vec![0x00]; // OP_0 for witness v0
        script_bytes.extend_from_slice(&[0u8; 20]); // 20-byte pubkey hash

        let script = ScriptPubkey::from_checked(script_bytes);

        let output = TxOut {
            value: Sats::from(10000u64),
            script_pubkey: script,
        };

        let tx = Tx {
            version: bp::TxVer::V2,
            inputs: Confined::try_from(vec![]).unwrap(),
            outputs: Confined::try_from(vec![output]).unwrap(),
            lock_time: LockTime::ZERO,
        };

        Psbt::from_tx(tx)
    }

    // ========================================================================
    // Test 1: Successful Tapret Commitment with Valid Taproot Output
    // ========================================================================

    #[test]
    fn test_embed_tapret_commitment_success() {
        let mut psbt = create_test_psbt_with_taproot();
        let state_hash = [0x42u8; 32]; // Deterministic test hash

        // Embed commitment
        let result = embed_tapret_commitment(&mut psbt, 0, state_hash);

        // Should succeed
        assert!(
            result.is_ok(),
            "Tapret commitment should succeed: {:?}",
            result.err()
        );

        let proof = result.unwrap();

        // Verify proof structure is valid and non-empty
        // 1. Proof should have a valid internal key (not default)
        let internal_pk_bytes = proof.internal_pk.serialize();
        assert!(
            internal_pk_bytes.iter().any(|&b| b != 0),
            "Internal key should not be all zeros"
        );

        // 2. Proof should be able to restore original script pubkey
        let original_script = proof.original_pubkey_script();
        assert!(
            original_script.len() > 0,
            "Original script should be non-empty"
        );
        assert!(original_script.is_p2tr(), "Restored script should be P2TR");

        // 3. Commitment should be embedded in PSBT output
        let outputs: Vec<_> = psbt.outputs().collect();
        assert!(
            outputs[0].script.is_p2tr(),
            "Output should remain P2TR after commitment"
        );
    }

    // ========================================================================
    // Test 2: Error Handling - Invalid Output Index (Out of Bounds)
    // ========================================================================

    #[test]
    fn test_embed_tapret_commitment_invalid_output_index() {
        let mut psbt = create_test_psbt_with_taproot();
        let state_hash = [0x42u8; 32];

        // Try to commit to output index 5 when only 1 output exists
        let result = embed_tapret_commitment(&mut psbt, 5, state_hash);

        // Should fail with InvalidOutputIndex
        assert!(result.is_err(), "Should fail with invalid output index");

        match result.unwrap_err() {
            TapretError::InvalidOutputIndex { index, max } => {
                assert_eq!(index, 5);
                assert_eq!(max, 1); // Only 1 output in PSBT
            }
            other => panic!("Expected InvalidOutputIndex, got: {:?}", other),
        }
    }

    // ========================================================================
    // Test 3: Error Handling - Non-Taproot Output (P2WPKH)
    // ========================================================================

    #[test]
    fn test_embed_tapret_commitment_non_taproot_output() {
        let mut psbt = create_test_psbt_with_p2wpkh();
        let state_hash = [0x42u8; 32];

        // Try to commit to P2WPKH output (should fail)
        let result = embed_tapret_commitment(&mut psbt, 0, state_hash);

        // Should fail with NotTaprootOutput
        assert!(result.is_err(), "Should fail with non-Taproot output");

        match result.unwrap_err() {
            TapretError::NotTaprootOutput => {
                // Expected error
            }
            other => panic!("Expected NotTaprootOutput, got: {:?}", other),
        }
    }

    // ========================================================================
    // Test 4: Anchor Creation and Structure Validation
    // ========================================================================

    #[test]
    fn test_create_anchor_structure() {
        let mut psbt = create_test_psbt_with_taproot();
        let state_hash = [0x42u8; 32];

        // Embed commitment to get proof
        let proof =
            embed_tapret_commitment(&mut psbt, 0, state_hash).expect("Commitment should succeed");

        // Create anchor from proof
        let result = create_anchor(&proof);
        assert!(result.is_ok(), "Anchor creation should succeed");

        let anchor = result.unwrap();

        // Verify anchor structure for F1r3fly-RGB light integration:
        // - dbc_proof (Tapret): Should be Some(proof) - REAL Bitcoin commitment
        // - mmb_proof: Placeholder (StrictDumb) - Not needed for F1r3fly validation
        // - mpc_protocol: Placeholder (StrictDumb) - Not needed for F1r3fly validation
        // - mpc_proof: Placeholder (StrictDumb) - Not needed for F1r3fly validation

        assert!(
            anchor.dbc_proof.is_some(),
            "Anchor should have real Tapret proof"
        );

        // Verify the proof in anchor matches the input proof
        let anchor_proof = anchor.dbc_proof.as_ref().unwrap();
        assert_eq!(
            format!("{:?}", anchor_proof),
            format!("{:?}", &proof),
            "Anchor proof should match input proof"
        );
    }

    // ========================================================================
    // Test 5: End-to-End State Hash Commitment and Verification
    // ========================================================================

    #[test]
    fn test_end_to_end_state_hash_commitment() {
        // Simulate F1r3fly execution result
        let f1r3fly_state_hash = [0xABu8; 32]; // Example state hash from F1r3fly

        // Step 1: Create commitment from state hash (what executor.rs provides)
        let commitment = mpc_cv::Commitment::from(f1r3fly_state_hash);

        // Step 2: Create PSBT for Bitcoin transaction
        let mut psbt = create_test_psbt_with_taproot();

        // Step 3: Embed F1r3fly state hash as Tapret commitment
        let tapret_proof = embed_tapret_commitment(&mut psbt, 0, f1r3fly_state_hash)
            .expect("Tapret commitment should succeed");

        // Step 4: Create RGB Anchor for BitcoinAnchorTracker
        let anchor = create_anchor(&tapret_proof).expect("Anchor creation should succeed");

        // Step 5: Verify anchor contains the real Tapret proof
        assert!(anchor.dbc_proof.is_some(), "Anchor must have Tapret proof");

        // Step 6: Verify commitment bytes match original state hash
        let commitment_bytes = commitment.to_byte_array();
        assert_eq!(
            commitment_bytes, f1r3fly_state_hash,
            "Commitment should preserve state hash"
        );

        // This anchor can now be stored in BitcoinAnchorTracker:
        // tracker.add_witness(opid, txid, tx, anchor, WitnessStatus::Tentative);
    }

    // ========================================================================
    // Test 6: Extraction and Verification (PSBT → Tx)
    // ========================================================================

    #[test]
    fn test_extract_and_verify_workflow() {
        // Part A: Extraction and PSBT verification
        let mut psbt = create_test_psbt_with_taproot();
        let state_hash = [0xABu8; 32];
        let proof =
            embed_tapret_commitment(&mut psbt, 0, state_hash).expect("Embedding should succeed");

        // Extract and verify commitment matches
        let extracted = extract_tapret_commitment(&psbt, 0).expect("Extraction should succeed");
        assert_eq!(extracted, state_hash, "Extracted hash should match");

        // Verify with correct hash (should succeed)
        assert!(verify_tapret_commitment(&psbt, 0, state_hash, &proof).is_ok());

        // Verify with wrong hash (should fail)
        let wrong_hash = [0xFFu8; 32];
        let err = verify_tapret_commitment(&psbt, 0, wrong_hash, &proof).unwrap_err();
        match err {
            TapretError::CommitmentFailed(msg) => assert!(msg.contains("mismatch")),
            _ => panic!("Expected CommitmentFailed"),
        }

        // Part B: Error cases
        let empty_psbt = create_test_psbt_with_taproot();
        assert!(
            extract_tapret_commitment(&empty_psbt, 0).is_err(),
            "No commitment"
        );
        assert!(
            extract_tapret_commitment(&psbt, 99).is_err(),
            "Invalid index"
        );

        // Part C: Full transaction verification
        let tx = psbt.extract().expect("PSBT extraction should succeed");

        // Correct state hash (should succeed)
        assert!(verify_tapret_proof_in_tx(&tx, state_hash, &proof).is_ok());

        // Wrong state hash (should fail)
        assert!(verify_tapret_proof_in_tx(&tx, wrong_hash, &proof).is_err());

        // Part D: Unified verification (automatically uses full verification when finalized)
        let mut psbt2 = create_test_psbt_with_taproot();
        let state_hash2 = [0xCDu8; 32];
        let proof2 =
            embed_tapret_commitment(&mut psbt2, 0, state_hash2).expect("Embedding should succeed");

        verify_tapret_commitment(&psbt2, 0, state_hash2, &proof2)
            .expect("Unified verification should succeed");
    }
}
