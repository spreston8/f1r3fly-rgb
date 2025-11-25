//! OP_RETURN commitment utilities for F1r3fly state hashes
//!
//! Provides OP_RETURN-based anchoring as an alternative to Tapret.
//! Used primarily for Lightning channel commitment transactions where
//! OP_RETURN provides compatibility with LDK's RGB integration.
//!
//! # OP_RETURN vs Tapret
//!
//! - **Tapret**: Privacy-preserving, uses Taproot commitments (default for on-chain)
//! - **OP_RETURN**: Explicit data field, compatible with Lightning (for channels)
//!
//! Both methods embed the same F1r3fly state hash, just in different ways.

use bitcoin::{Amount, ScriptBuf, Transaction, TxOut};

pub type Result<T> = std::result::Result<T, OpReturnError>;

#[derive(Debug)]
pub enum OpReturnError {
    InvalidOutputIndex { index: usize, max: usize },
    CommitmentFailed(String),
    ExtractionFailed(String),
    NotOpReturn,
}

impl std::fmt::Display for OpReturnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidOutputIndex { index, max } => {
                write!(f, "Invalid output index {} (max: {})", index, max)
            }
            Self::CommitmentFailed(msg) => write!(f, "OP_RETURN commitment failed: {}", msg),
            Self::ExtractionFailed(msg) => write!(f, "OP_RETURN extraction failed: {}", msg),
            Self::NotOpReturn => write!(f, "Output is not an OP_RETURN"),
        }
    }
}

impl std::error::Error for OpReturnError {}

/// Embed F1r3fly state hash as OP_RETURN in Bitcoin transaction
///
/// Adds an OP_RETURN output containing the 32-byte state hash.
/// This is compatible with Lightning commitment transactions and
/// rgb-lightning-node's OP_RETURN-based RGB anchoring.
///
/// # Arguments
/// * `tx` - Mutable Bitcoin transaction (already built with outputs)
/// * `output_index` - Where to insert OP_RETURN (typically 0 for Lightning)
/// * `state_hash` - F1r3fly state hash (32 bytes from contract execution)
///
/// # Returns
/// The output index where OP_RETURN was inserted
///
/// # Example
///
/// ```rust,no_run
/// use f1r3fly_rgb::embed_opreturn_commitment;
/// use bdk_wallet::bitcoin::Transaction;
///
/// # fn example(mut tx: Transaction, state_hash: [u8; 32]) -> Result<(), Box<dyn std::error::Error>> {
/// // Embed state hash at output index 0
/// let index = embed_opreturn_commitment(&mut tx, 0, state_hash)?;
/// assert_eq!(index, 0);
/// assert!(tx.output[0].script_pubkey.is_op_return());
/// # Ok(())
/// # }
/// ```
pub fn embed_opreturn_commitment(
    tx: &mut Transaction,
    output_index: usize,
    state_hash: [u8; 32],
) -> Result<usize> {
    // Validate output index
    if output_index > tx.output.len() {
        return Err(OpReturnError::InvalidOutputIndex {
            index: output_index,
            max: tx.output.len(),
        });
    }

    // Create OP_RETURN output with state hash
    let opreturn_output = TxOut {
        value: Amount::ZERO,
        script_pubkey: ScriptBuf::new_op_return(&state_hash),
    };

    // Insert at specified index
    tx.output.insert(output_index, opreturn_output);

    log::info!("✅ OP_RETURN commitment embedded");
    log::debug!("   State hash: {}", hex::encode(state_hash));
    log::debug!("   Output index: {}", output_index);

    Ok(output_index)
}

/// Extract OP_RETURN commitment from Bitcoin transaction
///
/// Reads the embedded F1r3fly state hash from an OP_RETURN output.
/// Useful for verification and validation of received transactions.
///
/// # Arguments
/// * `tx` - Bitcoin transaction containing OP_RETURN
/// * `output_index` - Which output to extract from (typically 0)
///
/// # Returns
/// The 32-byte state hash that was embedded
///
/// # Errors
/// Returns error if:
/// - Output index is out of bounds
/// - Output is not an OP_RETURN
/// - OP_RETURN data is not 32 bytes
///
/// # Example
///
/// ```rust,no_run
/// use f1r3fly_rgb::extract_opreturn_commitment;
/// use bdk_wallet::bitcoin::Transaction;
///
/// # fn example(tx: Transaction) -> Result<(), Box<dyn std::error::Error>> {
/// let state_hash = extract_opreturn_commitment(&tx, 0)?;
/// println!("Extracted state hash: {}", hex::encode(state_hash));
/// # Ok(())
/// # }
/// ```
pub fn extract_opreturn_commitment(tx: &bp::Tx, output_index: usize) -> Result<[u8; 32]> {
    // Validate output index
    if output_index >= tx.outputs.len() {
        return Err(OpReturnError::InvalidOutputIndex {
            index: output_index,
            max: tx.outputs.len(),
        });
    }

    let output = &tx.outputs[output_index];

    // Get script bytes as a slice
    let script_bytes = output.script_pubkey.as_slice();

    // Check if OP_RETURN (first byte should be 0x6a)
    if script_bytes.is_empty() || script_bytes[0] != 0x6a {
        return Err(OpReturnError::NotOpReturn);
    }

    // Extract data from OP_RETURN script
    // Format: OP_RETURN <push_opcode> <32_bytes>

    // Minimum length: OP_RETURN (1) + PUSHBYTES_32 (1) + data (32) = 34
    if script_bytes.len() < 34 {
        return Err(OpReturnError::ExtractionFailed(format!(
            "OP_RETURN data too short: {} bytes (expected at least 34)",
            script_bytes.len()
        )));
    }

    // Extract 32-byte hash (skip OP_RETURN opcode and length prefix)
    let hash: [u8; 32] = script_bytes[2..34].try_into().map_err(|_| {
        OpReturnError::ExtractionFailed("Failed to extract 32-byte hash".to_string())
    })?;

    Ok(hash)
}

/// Create an anchor for OP_RETURN commitment
///
/// Creates an RGB anchor structure for OP_RETURN-based state commitments.
/// Unlike Tapret which has cryptographic proofs, OP_RETURN anchors are
/// simple references to the commitment in the transaction.
///
/// # Arguments
/// * `state_hash` - F1r3fly state hash (32 bytes)
/// * `output_index` - Output index containing the OP_RETURN (typically 0)
///
/// # Returns
/// RGB Anchor structure for tracking the commitment
///
/// # Example
///
/// ```rust,no_run
/// use f1r3fly_rgb::create_opreturn_anchor;
///
/// # fn example(state_hash: [u8; 32]) -> Result<(), Box<dyn std::error::Error>> {
/// let anchor = create_opreturn_anchor(state_hash, 0);
/// // Use anchor for consignment creation
/// # Ok(())
/// # }
/// ```
pub fn create_opreturn_anchor(_state_hash: [u8; 32], _output_index: usize) -> bp::seals::Anchor {
    use bp::seals::{mmb, mpc, Anchor};
    use commit_verify::ReservedBytes;
    use strict_encoding::StrictDumb;

    // OP_RETURN anchors use simple structure without DBC proofs
    // The commitment is validated by checking the OP_RETURN data directly
    Anchor {
        mmb_proof: mmb::BundleProof::strict_dumb(),
        mpc_protocol: mpc::ProtocolId::strict_dumb(),
        mpc_proof: mpc::MerkleProof::strict_dumb(),
        dbc_proof: None, // OP_RETURN doesn't have DBC proof like Tapret
        fallback_proof: ReservedBytes::strict_dumb(),
    }
}
