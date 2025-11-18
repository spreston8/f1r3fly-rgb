//! Bitcoin Anchor Tracker
//!
//! Tracks Bitcoin seals and witnesses for RGB operations, implementing the `Pile` trait
//! for 100% RGB protocol compatibility.
//!
//! ## Architecture
//!
//! - **BitcoinAnchorTracker**: Tracks Bitcoin UTXOs (ownership layer)
//! - **F1r3flyExecutor**: Manages contract state (state layer on F1r3fly shards)
//!
//! ## Persistence
//!
//! **Explicit Persistence Model:**
//! - `save(path)` - Explicitly serialize to JSON file
//! - `load_from_disk(path)` - Explicitly deserialize from JSON file
//!
//! **Pile Trait Integration:**
//! - `Pile::new(conf)` - Creates empty tracker; enables auto-persist if `conf.persistence_path` is set
//! - `Pile::load(conf)` - Loads from `conf.persistence_path` if provided; enables auto-persist
//! - `commit_transaction()` - Automatically saves to configured path (if set)
//!
//! **Automatic vs. Manual Persistence:**
//! - With path: `BitcoinAnchorTracker::with_persistence("./data.json")` → auto-saves on `commit_transaction()`
//! - Without path: `BitcoinAnchorTracker::new()` → call `save()` manually when needed
//!
//! **Future Consideration:**
//! - For production at scale (>100K anchors, multi-process access), consider migrating to
//!   `PileFs` from `rgb-persist-fs` crate. It provides binary storage via `aora`, ACID transactions,
//!   and multi-database architecture. Migration is trivial: change type alias `BitcoinAnchorTracker<TxoSeal>`
//!   → `PileFs<TxoSeal>`. Current JSON approach chosen for: faster development, human-readable debugging,
//!   and right-sized for single-user wallet needs.
//!
//! ## RGB Compliance
//!
//! Fully implements the `Pile` trait with 14 methods for seals, witnesses, and their
//! relationships. Preserves RGB semantics: seal persistence, witness status progression,
//! bidirectional mappings, batch transactions, RBF support, and genesis operations

use amplify::confinement::SmallOrdMap;
use bp::seals::Anchor;
use rgb::{CellAddr, OpRels, Opid, Pile, RgbSeal, Witness, WitnessStatus};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error as StdError;
use std::fmt;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

// Re-export for convenience (these are from rgb-std::pile)
pub use rgb::{OpRels as RgbOpRels, Witness as RgbWitness};

/// Configuration for BitcoinAnchorTracker persistence
#[derive(Debug, Clone, Default)]
pub struct AnchorConfig {
    /// Optional path to load/save tracker data
    pub persistence_path: Option<PathBuf>,
}

/// Error type for BitcoinAnchorTracker operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BitcoinAnchorError {
    /// Configuration error
    InvalidConfiguration(String),
    /// Witness not found
    WitnessNotFound(String),
    /// Persistence error (I/O, serialization, etc.)
    PersistenceError(String),
    /// Serialization error
    SerializationError(String),
    /// Deserialization error
    DeserializationError(String),
}

impl fmt::Display for BitcoinAnchorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfiguration(msg) => write!(f, "Invalid configuration: {}", msg),
            Self::WitnessNotFound(msg) => write!(f, "Witness not found: {}", msg),
            Self::PersistenceError(msg) => write!(f, "Persistence error: {}", msg),
            Self::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            Self::DeserializationError(msg) => write!(f, "Deserialization error: {}", msg),
        }
    }
}

impl StdError for BitcoinAnchorError {}

/// Witness data for a Bitcoin transaction
///
/// Stores both the published witness (Bitcoin transaction data) and the
/// client anchor (SPV proof or similar), along with confirmation status.
#[derive(Serialize, Deserialize)]
#[serde(bound(
    serialize = "Seal::Published: Serialize, Seal::Client: Serialize",
    deserialize = "Seal::Published: Deserialize<'de>, Seal::Client: Deserialize<'de>"
))]
pub struct WitnessData<Seal: RgbSeal> {
    /// Published witness (Bitcoin transaction)
    pub published: Seal::Published,

    /// Client-side anchor (SPV proof, block header, etc.)
    pub client: Seal::Client,

    /// Confirmation status (mined, tentative, offchain, etc.)
    pub status: WitnessStatus,
}

/// Tracks Bitcoin seals and witnesses for RGB operations
///
/// This is a lightweight alternative to the `Pile` trait that focuses solely
/// on Bitcoin anchoring without client-side state storage.
///
/// # Usage
///
/// ```ignore
/// let mut tracker = BitcoinAnchorTracker::<TxoSeal>::new();
///
/// // When an operation is created
/// let seals = SmallOrdMap::from([(0u16, TxoSeal { txid, vout })]);
/// tracker.add_seals(opid, seals);
///
/// // When Bitcoin transaction is published
/// tracker.add_witness(opid, wid, published, client, WitnessStatus::Tentative);
///
/// // When transaction confirms
/// tracker.update_witness_status(wid, WitnessStatus::Mined(confirmations));
/// ```
#[derive(Serialize, Deserialize)]
#[serde(bound(
    serialize = "Seal::Definition: Serialize, Seal::WitnessId: Serialize, Seal::Published: Serialize, Seal::Client: Serialize",
    deserialize = "Seal::Definition: Deserialize<'de>, Seal::WitnessId: Deserialize<'de>, Seal::Published: Deserialize<'de>, Seal::Client: Deserialize<'de>"
))]
pub struct BitcoinAnchorTracker<Seal: RgbSeal> {
    /// Seal definitions for each operation
    /// Maps: Opid → (output_index → seal_definition)
    seals: HashMap<Opid, SmallOrdMap<u16, Seal::Definition>>,

    /// Witness data for each Bitcoin transaction
    /// Maps: WitnessId → WitnessData
    witnesses: HashMap<Seal::WitnessId, WitnessData<Seal>>,

    /// Operation to witness mapping
    /// Maps: Opid → [WitnessId]
    op_witnesses: HashMap<Opid, Vec<Seal::WitnessId>>,

    /// Reverse mapping: witness to operations
    /// Maps: WitnessId → [Opid]
    witness_ops: HashMap<Seal::WitnessId, Vec<Opid>>,

    /// Anchors for each operation (Tapret proofs)
    /// Maps: Opid → Anchor
    /// Stored when Bitcoin PSBT is finalized and Tapret proof is created
    anchors: HashMap<Opid, Anchor>,

    /// Optional path for automatic persistence
    /// If set, `commit_transaction()` will automatically save to this path
    #[serde(skip)]
    persistence_path: Option<PathBuf>,

    /// Marker for seal type
    #[serde(skip)]
    _seal: PhantomData<Seal>,
}

impl<Seal: RgbSeal> BitcoinAnchorTracker<Seal> {
    /// Create a new empty tracker without automatic persistence
    pub fn new() -> Self {
        Self {
            seals: HashMap::new(),
            witnesses: HashMap::new(),
            op_witnesses: HashMap::new(),
            witness_ops: HashMap::new(),
            anchors: HashMap::new(),
            persistence_path: None,
            _seal: PhantomData,
        }
    }

    /// Create a new empty tracker with automatic persistence
    ///
    /// When `commit_transaction()` is called, data will be automatically
    /// saved to the specified path.
    pub fn with_persistence<P: Into<PathBuf>>(path: P) -> Self {
        Self {
            seals: HashMap::new(),
            witnesses: HashMap::new(),
            op_witnesses: HashMap::new(),
            witness_ops: HashMap::new(),
            anchors: HashMap::new(),
            persistence_path: Some(path.into()),
            _seal: PhantomData,
        }
    }

    /// Set or update the persistence path
    ///
    /// This enables automatic persistence on `commit_transaction()` calls.
    pub fn set_persistence_path<P: Into<PathBuf>>(&mut self, path: Option<P>) {
        self.persistence_path = path.map(|p| p.into());
    }

    /// Get the current persistence path
    pub fn persistence_path(&self) -> Option<&Path> {
        self.persistence_path.as_deref()
    }

    /// Get count of tracked operations (not part of Pile trait)
    pub fn operation_count(&self) -> usize {
        self.seals.len()
    }

    /// Get count of tracked witnesses (not part of Pile trait)
    pub fn witness_count(&self) -> usize {
        self.witnesses.len()
    }

    // ========================================================================
    // Persistence Methods
    // ========================================================================

    /// Save tracker to disk (JSON format)
    ///
    /// Serializes the entire tracker state to a JSON file, including all seals,
    /// witnesses, and their relationships.
    ///
    /// # Arguments
    /// - `path`: File path to save to
    ///
    /// # Returns
    /// `Ok(())` on success, error if serialization or I/O fails
    ///
    /// # Example
    /// ```ignore
    /// tracker.save("./tracker_data.json")?;
    /// ```
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), BitcoinAnchorError>
    where
        Seal::Definition: Serialize,
        Seal::WitnessId: Serialize,
        Seal::Published: Serialize,
        Seal::Client: Serialize,
    {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| BitcoinAnchorError::SerializationError(e.to_string()))?;

        std::fs::write(path, json)
            .map_err(|e| BitcoinAnchorError::PersistenceError(e.to_string()))?;

        Ok(())
    }

    /// Load tracker from disk (JSON format)
    ///
    /// Deserializes a previously saved tracker from a JSON file, restoring all
    /// seals, witnesses, and their relationships. The loaded tracker will have
    /// automatic persistence enabled using the path it was loaded from.
    ///
    /// # Arguments
    /// - `path`: File path to load from
    ///
    /// # Returns
    /// Loaded tracker on success, error if file doesn't exist or deserialization fails
    ///
    /// # Example
    /// ```ignore
    /// let tracker = BitcoinAnchorTracker::load_from_disk("./tracker_data.json")?;
    /// // Future commit_transaction() calls will auto-save to ./tracker_data.json
    /// ```
    pub fn load_from_disk<P: AsRef<Path>>(path: P) -> Result<Self, BitcoinAnchorError>
    where
        Seal::Definition: for<'de> Deserialize<'de>,
        Seal::WitnessId: for<'de> Deserialize<'de>,
        Seal::Published: for<'de> Deserialize<'de>,
        Seal::Client: for<'de> Deserialize<'de>,
    {
        let json = std::fs::read_to_string(path.as_ref())
            .map_err(|e| BitcoinAnchorError::PersistenceError(e.to_string()))?;

        let mut tracker: Self = serde_json::from_str(&json)
            .map_err(|e| BitcoinAnchorError::DeserializationError(e.to_string()))?;

        // Enable automatic persistence using the path we loaded from
        tracker.persistence_path = Some(path.as_ref().to_path_buf());

        Ok(tracker)
    }

    // ============================================================================
    // Anchor Management
    // ============================================================================

    /// Store anchor for an operation
    ///
    /// Anchors are typically created after Bitcoin PSBTs are finalized and contain
    /// Tapret proofs linking the F1r3fly state hash to a Bitcoin transaction.
    ///
    /// # Arguments
    ///
    /// * `opid` - Operation ID (typically derived from state_hash)
    /// * `anchor` - Tapret proof anchor
    ///
    /// # Example
    ///
    /// ```ignore
    /// // After finalizing PSBT and creating Tapret proof
    /// let anchor = create_anchor(&proof)?;
    /// let opid = rgb::Opid::from(state_hash);
    /// tracker.add_anchor(opid, anchor);
    /// ```
    pub fn add_anchor(&mut self, opid: Opid, anchor: Anchor) {
        self.anchors.insert(opid, anchor);
    }

    /// Retrieve anchor for an operation
    ///
    /// Returns `None` if no anchor has been stored for this operation.
    ///
    /// # Arguments
    ///
    /// * `opid` - Operation ID
    ///
    /// # Returns
    ///
    /// Reference to the anchor, or `None` if not found
    pub fn get_anchor(&self, opid: &Opid) -> Option<&Anchor> {
        self.anchors.get(opid)
    }

    /// Check if anchor exists for an operation
    ///
    /// # Arguments
    ///
    /// * `opid` - Operation ID
    ///
    /// # Returns
    ///
    /// `true` if anchor exists, `false` otherwise
    pub fn has_anchor(&self, opid: &Opid) -> bool {
        self.anchors.contains_key(opid)
    }

    /// Remove anchor for an operation
    ///
    /// Used for RBF (replace-by-fee) scenarios or anchor updates.
    ///
    /// # Arguments
    ///
    /// * `opid` - Operation ID
    ///
    /// # Returns
    ///
    /// The removed anchor, or `None` if it didn't exist
    pub fn remove_anchor(&mut self, opid: &Opid) -> Option<Anchor> {
        self.anchors.remove(opid)
    }
}

impl<Seal: RgbSeal> Default for BitcoinAnchorTracker<Seal> {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Pile Trait Implementation
// ============================================================================

impl<Seal: RgbSeal> Pile for BitcoinAnchorTracker<Seal>
where
    Seal::WitnessId: Clone + Eq + std::hash::Hash,
    Seal::Published: Clone,
    Seal::Client: Clone,
    Seal::Definition: Clone,
    Seal::Definition: Serialize + for<'de> Deserialize<'de>,
    Seal::WitnessId: Serialize + for<'de> Deserialize<'de>,
    Seal::Published: Serialize + for<'de> Deserialize<'de>,
    Seal::Client: Serialize + for<'de> Deserialize<'de>,
{
    type Seal = Seal;
    type Conf = AnchorConfig;
    type Error = BitcoinAnchorError;

    /// Create a new pile with configuration
    ///
    /// Creates an empty in-memory tracker. If `conf.persistence_path` is provided,
    /// automatic persistence will be enabled for `commit_transaction()` calls.
    fn new(conf: Self::Conf) -> Result<Self, Self::Error> {
        match conf.persistence_path {
            Some(path) => Ok(Self::with_persistence(path)),
            None => Ok(Self::new()),
        }
    }

    /// Load pile from persistence
    ///
    /// If `conf.persistence_path` is provided, loads from that file and enables
    /// automatic persistence. Otherwise, creates a new empty in-memory tracker.
    fn load(conf: Self::Conf) -> Result<Self, Self::Error> {
        match conf.persistence_path {
            Some(path) => Self::load_from_disk(path),
            None => Ok(Self::new()),
        }
    }

    /// Get published witness (Bitcoin transaction) by ID
    ///
    /// # Panics
    /// Panics if the witness is not known
    fn pub_witness(&self, wid: Seal::WitnessId) -> Seal::Published {
        self.witnesses
            .get(&wid)
            .expect("Witness must exist")
            .published
            .clone()
    }

    /// Check if witness exists
    fn has_witness(&self, wid: Seal::WitnessId) -> bool {
        self.witnesses.contains_key(&wid)
    }

    /// Get client anchor (Tapret proof) by witness ID
    ///
    /// # Panics
    /// Panics if the witness is not known
    fn cli_witness(&self, wid: Seal::WitnessId) -> Seal::Client {
        self.witnesses
            .get(&wid)
            .expect("Witness must exist")
            .client
            .clone()
    }

    /// Get witness confirmation status
    ///
    /// Returns `WitnessStatus::Archived` if witness is not found
    fn witness_status(&self, wid: Seal::WitnessId) -> WitnessStatus {
        self.witnesses
            .get(&wid)
            .map(|w| w.status)
            .unwrap_or(WitnessStatus::Archived)
    }

    /// Iterate over all witness IDs
    fn witness_ids(&self) -> impl Iterator<Item = Seal::WitnessId> {
        self.witnesses
            .keys()
            .copied()
            .collect::<Vec<_>>()
            .into_iter()
    }

    /// Iterate over all witnesses with full data
    fn witnesses(&self) -> impl Iterator<Item = Witness<Self::Seal>> {
        self.witnesses.iter().map(|(wid, data)| {
            let opids = self
                .witness_ops
                .get(wid)
                .map(|ops| ops.iter().copied().collect())
                .unwrap_or_default();

            Witness {
                id: *wid,
                published: data.published.clone(),
                client: data.client.clone(),
                status: data.status,
                opids,
            }
        })
    }

    /// Get witness IDs for an operation
    fn op_witness_ids(&self, opid: Opid) -> impl ExactSizeIterator<Item = Seal::WitnessId> {
        // Return a wrapper that implements ExactSizeIterator
        ExactSizeIter {
            inner: self
                .op_witnesses
                .get(&opid)
                .map(|v| v.clone())
                .unwrap_or_default()
                .into_iter(),
        }
    }

    /// Get operations anchored by a witness
    fn ops_by_witness_id(&self, wid: Seal::WitnessId) -> impl ExactSizeIterator<Item = Opid> {
        ExactSizeIter {
            inner: self
                .witness_ops
                .get(&wid)
                .map(|v| v.clone())
                .unwrap_or_default()
                .into_iter(),
        }
    }

    /// Get seal for a specific cell address
    ///
    /// Returns the Bitcoin UTXO (seal) for a specific cell in an operation's output.
    fn seal(&self, addr: CellAddr) -> Option<Seal::Definition> {
        self.seals.get(&addr.opid)?.get(&addr.pos).cloned()
    }

    /// Get seals for an operation (up to output index)
    ///
    /// Returns all seals up to and including the specified output index.
    fn seals(&self, opid: Opid, up_to: u16) -> SmallOrdMap<u16, Seal::Definition> {
        self.seals
            .get(&opid)
            .map(|seals| {
                let mut result = SmallOrdMap::new();
                for (idx, seal) in seals.iter() {
                    if *idx <= up_to {
                        result.insert(*idx, seal.clone()).ok();
                    }
                }
                result
            })
            .unwrap_or_default()
    }

    /// Get full operation relations (witnesses + seals)
    fn op_relations(&self, opid: Opid, up_to: u16) -> OpRels<Self::Seal> {
        let witness_ids = self
            .op_witnesses
            .get(&opid)
            .map(|v| v.iter().copied().collect())
            .unwrap_or_default();

        let defines = self.seals(opid, up_to);

        OpRels {
            opid,
            witness_ids,
            defines,
            _phantom: PhantomData,
        }
    }

    /// Add witness data for an operation
    ///
    /// Links an RGB operation to a Bitcoin transaction (witness).
    /// If the witness already exists, it will be updated.
    fn add_witness(
        &mut self,
        opid: Opid,
        wid: Seal::WitnessId,
        published: &Seal::Published,
        anchor: &Seal::Client,
        status: WitnessStatus,
    ) {
        // Store witness data
        self.witnesses.insert(
            wid,
            WitnessData {
                published: published.clone(),
                client: anchor.clone(),
                status,
            },
        );

        // Add to op → witnesses mapping
        self.op_witnesses
            .entry(opid)
            .or_insert_with(Vec::new)
            .push(wid);

        // Add to witness → ops mapping
        self.witness_ops
            .entry(wid)
            .or_insert_with(Vec::new)
            .push(opid);
    }

    /// Add seal definitions for an operation
    ///
    /// Seals define which Bitcoin UTXOs control the assets created by this operation.
    fn add_seals(&mut self, opid: Opid, seals: SmallOrdMap<u16, Seal::Definition>) {
        self.seals.insert(opid, seals);
    }

    /// Update witness confirmation status
    ///
    /// # Panics
    /// Panics if the witness is not known
    fn update_witness_status(&mut self, wid: Seal::WitnessId, status: WitnessStatus) {
        self.witnesses
            .get_mut(&wid)
            .expect("Witness must exist")
            .status = status;
    }

    /// Commit changes to persistence
    ///
    /// If a persistence path is configured (via `with_persistence()`, `set_persistence_path()`,
    /// or `load_from_disk()`), this automatically saves the tracker to disk.
    ///
    /// Errors during save are silently ignored to maintain compatibility with the
    /// Pile trait. Check logs for persistence errors.
    fn commit_transaction(&mut self) {
        if let Some(path) = &self.persistence_path {
            if let Err(e) = self.save(path) {
                log::warn!("Failed to auto-save tracker to {:?}: {}", path, e);
            }
        }
    }
}

/// Wrapper to make Vec iterator into ExactSizeIterator
struct ExactSizeIter<T> {
    inner: std::vec::IntoIter<T>,
}

impl<T> Iterator for ExactSizeIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<T> ExactSizeIterator for ExactSizeIter<T> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}
