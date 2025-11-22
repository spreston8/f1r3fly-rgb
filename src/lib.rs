//! F1r3fly-RGB: RGB Smart Contracts with F1r3fly Execution
//!
//! This crate provides RGB smart contract functionality using F1r3fly
//! for state management and execution, while maintaining Bitcoin-based
//! ownership through UTXO anchoring.
//!
//! # Architecture
//!
//! - **F1r3fly Executor**: Manages contract state and executes Rholang contracts
//! - **Bitcoin Anchor Tracker**: Tracks UTXO ownership and Bitcoin witnesses
//! - **RGB Compatibility**: Uses RGB's proven Bitcoin primitives
//!
//! # Example
//!
//! ```ignore
//! use f1r3fly_rgb::{F1r3flyExecutor, RholangContractLibrary};
//!
//! // Create executor
//! let mut executor = F1r3flyExecutor::new()?;
//!
//! // Deploy RGB20 token
//! let contract = RholangContractLibrary::rho20_contract();
//! let rholang = RholangContractLibrary::substitute(contract, &[
//!     ("TICKER", "BTC"),
//!     ("NAME", "Bitcoin"),
//!     ("TOTAL_SUPPLY", "21000000"),
//!     ("PRECISION", "8"),
//! ]);
//!
//! let contract_id = executor.deploy_contract(
//!     rholang,
//!     vec!["issue".to_string(), "transfer".to_string(), "balanceOf".to_string()],
//!     &private_key,
//! ).await?;
//! ```

// Public modules
pub mod bitcoin_anchor;
pub mod consignment;
pub mod contract;
pub mod contract_library;
pub mod contracts;
pub mod error;
pub mod executor;
pub mod invoice;
pub mod signature_utils;
pub mod tapret;

// Re-exports for convenience
pub use bitcoin_anchor::{AnchorConfig, BitcoinAnchorError, BitcoinAnchorTracker};
pub use consignment::{F1r3flyConsignment, F1r3flyStateProof};
pub use contract::F1r3flyRgbContract;
pub use contract_library::RholangContractLibrary;
pub use contracts::F1r3flyRgbContracts;
pub use error::F1r3flyRgbError;
pub use executor::{ContractMetadata, F1r3flyExecutionResult, F1r3flyExecutor};
pub use signature_utils::{generate_issue_signature, generate_nonce, generate_transfer_signature};
pub use tapret::{
    create_anchor, create_tapret_anchor, create_test_psbt_with_taproot, embed_tapret_commitment,
    extract_tapret_commitment, verify_tapret_commitment, verify_tapret_proof_in_tx, TapretError,
};

// Re-export invoice module API
pub use invoice::{
    address_to_beneficiary, extract_seal, generate_invoice, get_recipient_address, parse_invoice,
    GeneratedInvoice, InvoiceRequest, ParsedInvoice,
};

// Re-export commonly used RGB types
pub use bp::seals::{TxoSeal, WTxoSeal};
pub use bp::{Sats, Tx, Txid};
pub use bpstd::psbt::TxParams; // For custom PSBT construction
pub use hypersonic::{CellAddr, ContractId, Opid};
pub use rgb::{Pile, RgbSeal, WitnessStatus};
pub use rgb_invoice::bp::WitnessOut;
pub use rgb_invoice::{RgbBeneficiary, RgbInvoice};
pub use strict_types::StrictVal;

// Common result type
pub type Result<T> = std::result::Result<T, F1r3flyRgbError>;
