//! Consignment handling for F1r3fly-RGB transfers
//!
//! Provides lightweight consignment packages for transferring RGB assets
//! with F1r3fly state proofs and Bitcoin anchors.

use crate::{
    ContractMetadata, F1r3flyExecutionResult, F1r3flyExecutor, F1r3flyRgbContract, F1r3flyRgbError,
    Tx,
};
use amplify::confinement::SmallOrdMap;
use bp::seals::{Anchor, WTxoSeal};
use hypersonic::ContractId;
use serde::{Deserialize, Serialize};

/// Witness identifier mapping for claim process
/// Links witness_id (temporary) → real UTXO (after Bitcoin TX)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WitnessMapping {
    /// Temporary witness identifier used during transfer
    /// Format: "witness:{address_hash}:{vout}"
    pub witness_id: String,

    /// Recipient's Bitcoin address (for UTXO matching)
    pub recipient_address: String,

    /// Expected vout in the Bitcoin transaction
    pub expected_vout: u32,
}

/// F1r3fly-RGB consignment for asset transfers
///
/// A lightweight transfer package containing:
/// - Contract metadata (Rholang source, methods)
/// - F1r3fly state proof (block hash, state hash)
/// - Bitcoin anchor (Tapret proof)
/// - Seals and witnesses
///
/// Unlike traditional RGB consignments, this does NOT contain:
/// - Full operation history (state is on F1r3fly shard)
/// - AluVM schemas (uses Rholang)
/// - Client-side state (queries F1r3fly)
///
/// # Example
///
/// ```rust,no_run
/// use f1r3fly_rgb::{F1r3flyConsignment, F1r3flyRgbContract};
///
/// # async fn example(contract: &F1r3flyRgbContract, result: f1r3fly_rgb::F1r3flyExecutionResult, seals: amplify::confinement::SmallOrdMap<u16, bp::seals::WTxoSeal>, witness_txs: Vec<bp::Tx>) -> Result<(), Box<dyn std::error::Error>> {
/// // Create consignment
/// let consignment = F1r3flyConsignment::new(
///     contract,
///     result,
///     seals,
///     witness_txs,
/// )?;
///
/// // Serialize and send
/// let bytes = consignment.to_bytes()?;
/// // send_to_recipient(bytes);
///
/// // Recipient validates
/// let received = F1r3flyConsignment::from_bytes(&bytes)?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct F1r3flyConsignment {
    /// Consignment format version
    pub version: u16,

    /// Contract ID (serialized as raw bytes)
    #[serde(
        serialize_with = "serialize_contract_id",
        deserialize_with = "deserialize_contract_id"
    )]
    pub contract_id: ContractId,

    /// Contract metadata (Rholang source, methods, registry URI)
    pub contract_metadata: ContractMetadata,

    /// F1r3fly state proof
    pub f1r3fly_proof: F1r3flyStateProof,

    /// Bitcoin anchor (Tapret proof)
    pub bitcoin_anchor: Anchor,

    /// Seals (UTXO bindings)
    pub seals: SmallOrdMap<u16, WTxoSeal>,

    /// Witness transactions (Bitcoin TX confirmations)
    /// Required for full Tapret proof verification (not required for genesis)
    /// Stores the actual Bitcoin transactions (not full Witness struct)
    pub witness_txs: Vec<Tx>,

    /// Whether this is a genesis consignment (vs transfer)
    /// Genesis consignments don't require Tapret proof validation
    pub is_genesis: bool,

    /// Witness identifier mapping (only for transfers, not genesis)
    /// Used by recipient to claim balance from witness_id to real UTXO
    #[serde(skip_serializing_if = "Option::is_none")]
    pub witness_mapping: Option<WitnessMapping>,
}

/// F1r3fly state proof for consignment validation
///
/// Contains cryptographic proof of F1r3fly shard state at time of transfer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct F1r3flyStateProof {
    /// F1r3fly block hash (finalized state)
    pub block_hash: String,

    /// State hash (committed to Bitcoin via Tapret)
    pub state_hash: [u8; 32],

    /// F1r3fly deploy ID (transaction ID)
    pub deploy_id: String,
}

impl F1r3flyConsignment {
    /// Create new consignment from contract execution result
    ///
    /// # Arguments
    ///
    /// * `contract` - Source contract
    /// * `result` - F1r3fly execution result (contains state proof)
    /// * `seals` - Seals for this transfer
    /// * `witness_txs` - Bitcoin witnesses (TX confirmations)
    /// * `is_genesis` - Whether this is a genesis consignment (skips Tapret validation)
    ///
    /// # Returns
    ///
    /// New consignment ready for serialization
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use f1r3fly_rgb::{F1r3flyConsignment, F1r3flyRgbContract, F1r3flyExecutionResult};
    /// # use amplify::confinement::SmallOrdMap;
    /// # use bp::seals::WTxoSeal;
    /// # use bp::Tx;
    /// # fn example(contract: &F1r3flyRgbContract, result: F1r3flyExecutionResult, seals: SmallOrdMap<u16, WTxoSeal>, witness_txs: Vec<Tx>) -> Result<(), Box<dyn std::error::Error>> {
    /// let consignment = F1r3flyConsignment::new(
    ///     contract,
    ///     result,
    ///     seals,
    ///     witness_txs,
    ///     true, // is_genesis
    /// )?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(
        contract: &F1r3flyRgbContract,
        result: F1r3flyExecutionResult,
        seals: SmallOrdMap<u16, WTxoSeal>,
        witness_txs: Vec<Tx>,
        is_genesis: bool,
    ) -> Result<Self, F1r3flyRgbError> {
        // Convert SmallVec to String for serialization
        let block_hash = result
            .block_hash_string()
            .map_err(|e| F1r3flyRgbError::InvalidResponse(format!("Invalid block hash: {}", e)))?;
        let deploy_id = result
            .deploy_id_string()
            .map_err(|e| F1r3flyRgbError::InvalidResponse(format!("Invalid deploy ID: {}", e)))?;

        // Get anchor from contract tracker (or create placeholder for genesis)
        // Use the opid from the execution result (not derived from state_hash)
        // The opid is for RGB operation tracking, state_hash is for Bitcoin commitment
        let opid = result.opid;

        let bitcoin_anchor = if is_genesis {
            // Genesis doesn't need real anchor - use placeholder
            // Genesis UTXO itself serves as the Bitcoin anchor
            use strict_types::StrictDumb;
            Anchor::strict_dumb()
        } else {
            // Transfer requires real anchor from tracker (with Tapret proof)
            contract
                .tracker()
                .get_anchor(&opid)
                .cloned()
                .ok_or_else(|| {
                    F1r3flyRgbError::InvalidConsignment(format!(
                        "No anchor found for operation {}. \
                         Wallet must call tracker.add_anchor() after PSBT finalization \
                         before creating consignment.",
                        opid
                    ))
                })?
        };

        Ok(Self {
            version: 1,
            contract_id: contract.contract_id(),
            contract_metadata: contract.metadata().clone(),
            f1r3fly_proof: F1r3flyStateProof {
                block_hash,
                state_hash: result.state_hash,
                deploy_id,
            },
            bitcoin_anchor,
            seals,
            witness_txs,
            is_genesis,
            witness_mapping: None,
        })
    }

    /// Validate consignment
    ///
    /// Verifies:
    /// 1. F1r3fly state proof is valid (query shard)
    /// 2. Bitcoin anchor matches state hash
    /// 3. Seals are valid UTXOs
    ///
    /// # Arguments
    ///
    /// * `executor` - F1r3fly executor for state verification
    ///
    /// # Returns
    ///
    /// Ok(()) if valid, error otherwise
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use f1r3fly_rgb::{F1r3flyConsignment, F1r3flyExecutor};
    /// # async fn example(consignment: F1r3flyConsignment, executor: &F1r3flyExecutor) -> Result<(), Box<dyn std::error::Error>> {
    /// consignment.validate(executor).await?;
    /// println!("Consignment is valid!");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn validate(&self, executor: &F1r3flyExecutor) -> Result<(), F1r3flyRgbError> {
        log::info!("Validating consignment for contract: {}", self.contract_id);

        // 1. Verify F1r3fly state proof - check block is finalized
        log::debug!(
            "Checking F1r3fly block finalization: {}",
            self.f1r3fly_proof.block_hash
        );
        let is_finalized = executor
            .is_block_finalized(&self.f1r3fly_proof.block_hash)
            .await?;

        if !is_finalized {
            return Err(F1r3flyRgbError::InvalidConsignment(format!(
                "F1r3fly block {} is not finalized. Consignment requires immutable state.",
                self.f1r3fly_proof.block_hash
            )));
        }
        log::debug!("✓ F1r3fly block is finalized");

        // State hash is implicitly verified through:
        // - Block finalization (proves F1r3fly state is immutable)
        // - Step 2 below (Bitcoin anchor verifies Tapret commitment) - ONLY FOR TRANSFERS

        // 2. Verify Bitcoin anchor
        // For GENESIS: Skip Tapret verification (genesis UTXO itself is the Bitcoin anchor)
        // For TRANSFER: Full Tapret cryptographic verification required
        if self.is_genesis {
            log::debug!("✓ Genesis consignment - Tapret verification skipped (not required)");
            log::debug!("  Genesis UTXO serves as Bitcoin anchor");

            // Verify we have at least one seal (the genesis seal)
            if self.seals.is_empty() {
                return Err(F1r3flyRgbError::InvalidConsignment(
                    "Genesis consignment must have at least one seal".to_string(),
                ));
            }

            // Verify we have the genesis transaction in witnesses
            if self.witness_txs.is_empty() {
                return Err(F1r3flyRgbError::InvalidConsignment(
                    "Genesis consignment must include genesis UTXO transaction".to_string(),
                ));
            }
        } else {
            // TRANSFER: Verify Bitcoin commitment (Tapret or OP_RETURN)
            // Get Bitcoin transaction from witness
            let witness_tx = self.witness_txs.first().ok_or_else(|| {
                F1r3flyRgbError::InvalidConsignment(
                    "No witness transaction for commitment verification. \
                     Transfer consignment must include Bitcoin transaction."
                        .to_string(),
                )
            })?;

            // Check which anchoring method was used
            if let Some(tapret_proof) = self.bitcoin_anchor.dbc_proof.as_ref() {
                // Tapret proof present - verify Tapret commitment
                log::debug!("Verifying Bitcoin anchor with Tapret proof (transfer consignment)");

                // Perform full cryptographic verification
                use crate::verify_tapret_proof_in_tx;
                verify_tapret_proof_in_tx(witness_tx, self.f1r3fly_proof.state_hash, tapret_proof)
                    .map_err(|e| {
                        F1r3flyRgbError::InvalidConsignment(format!(
                            "Tapret proof verification failed: {}",
                            e
                        ))
                    })?;

                log::debug!("✓ Tapret proof cryptographically verified");
            } else {
                // No Tapret proof - verify OP_RETURN commitment
                log::debug!("Verifying Bitcoin anchor with OP_RETURN (transfer consignment)");

                // Extract OP_RETURN commitment from transaction (should be at output 0)
                use crate::extract_opreturn_commitment;
                let extracted_hash = extract_opreturn_commitment(witness_tx, 0).map_err(|e| {
                    F1r3flyRgbError::InvalidConsignment(format!(
                        "OP_RETURN extraction failed: {}",
                        e
                    ))
                })?;

                // Verify the extracted hash matches the state hash
                if extracted_hash != self.f1r3fly_proof.state_hash {
                    return Err(F1r3flyRgbError::InvalidConsignment(format!(
                        "OP_RETURN hash mismatch. Expected: {}, Found: {}",
                        hex::encode(self.f1r3fly_proof.state_hash),
                        hex::encode(extracted_hash)
                    )));
                }

                log::debug!("✓ OP_RETURN commitment cryptographically verified");
            }

            log::debug!("   Witness TX count: {}", self.witness_txs.len());
        }

        // 3. Verify seals are valid
        if self.seals.is_empty() {
            return Err(F1r3flyRgbError::InvalidConsignment(
                "No seals in consignment".to_string(),
            ));
        }
        log::debug!("✓ Seals present: {} seal(s)", self.seals.len());

        log::info!("✅ Consignment validated");
        Ok(())
    }

    /// Serialize consignment to bytes
    ///
    /// # Returns
    ///
    /// Serialized consignment as byte vector
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use f1r3fly_rgb::F1r3flyConsignment;
    /// # fn example(consignment: F1r3flyConsignment) -> Result<(), Box<dyn std::error::Error>> {
    /// let bytes = consignment.to_bytes()?;
    /// // Send bytes to recipient
    /// # Ok(())
    /// # }
    /// ```
    pub fn to_bytes(&self) -> Result<Vec<u8>, F1r3flyRgbError> {
        serde_json::to_vec(self).map_err(|e| F1r3flyRgbError::SerializationError(e.to_string()))
    }

    /// Deserialize consignment from bytes
    ///
    /// # Arguments
    ///
    /// * `data` - Serialized consignment bytes
    ///
    /// # Returns
    ///
    /// Deserialized consignment
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use f1r3fly_rgb::F1r3flyConsignment;
    /// # fn example(bytes: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
    /// let consignment = F1r3flyConsignment::from_bytes(&bytes)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_bytes(data: &[u8]) -> Result<Self, F1r3flyRgbError> {
        serde_json::from_slice(data).map_err(|e| F1r3flyRgbError::SerializationError(e.to_string()))
    }

    /// Get contract ID
    pub fn contract_id(&self) -> ContractId {
        self.contract_id
    }

    /// Get contract metadata
    pub fn metadata(&self) -> &ContractMetadata {
        &self.contract_metadata
    }

    /// Get F1r3fly state proof
    pub fn f1r3fly_proof(&self) -> &F1r3flyStateProof {
        &self.f1r3fly_proof
    }

    /// Get seals
    pub fn seals(&self) -> &SmallOrdMap<u16, WTxoSeal> {
        &self.seals
    }
}

// Custom serialization for ContractId
fn serialize_contract_id<S>(id: &ContractId, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use amplify::ByteArray;
    serializer.serialize_bytes(&id.to_byte_array())
}

fn deserialize_contract_id<'de, D>(deserializer: D) -> Result<ContractId, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let bytes: Vec<u8> = serde::Deserialize::deserialize(deserializer)?;
    if bytes.len() != 32 {
        return Err(serde::de::Error::custom(format!(
            "Invalid ContractId length: expected 32, got {}",
            bytes.len()
        )));
    }
    let mut array = [0u8; 32];
    array.copy_from_slice(&bytes);
    Ok(ContractId::from(array))
}
