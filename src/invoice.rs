//! RGB Invoice Generation and Parsing (Production)
//!
//! Core invoice functionality for F1r3fly-RGB smart contracts.
//! Provides standard RGB invoice format with WitnessOut beneficiaries.
//!
//! # Architecture
//!
//! This module implements RGB invoice generation and parsing for F1r3fly-RGB:
//! - **WitnessOut mode**: Recipient provides Bitcoin address for token receipt
//! - **Standard RGB format**: `rgb:tb1q...@contract:xFS2u...:~:0:100`
//! - **Lightweight**: No RGB Runtime required for invoice generation
//! - **Compatible**: Works with F1r3flyConsignment and WTxoSeal
//!
//! # Invoice Flow
//!
//! ## Recipient (Generate Invoice)
//!
//! ```ignore
//! use f1r3fly_rgb::invoice::generate_invoice;
//! use hypersonic::ContractId;
//!
//! let contract_id = ContractId::from([1u8; 32]);
//! let address = bitcoin::Address::from_str("bcrt1q...").unwrap();
//! let invoice = generate_invoice(
//!     contract_id,
//!     100,                        // Amount to receive
//!     address,
//!     0,                          // Nonce
//!     rgb::Consensus::Bitcoin,
//!     true,                       // Testnet
//! )?;
//!
//! // Share invoice string with sender
//! println!("Invoice: {}", invoice.invoice.to_string());
//! ```
//!
//! ## Sender (Parse Invoice)
//!
//! ```ignore
//! use f1r3fly_rgb::invoice::{parse_invoice, extract_seal};
//!
//! let parsed = parse_invoice("rgb:tb1q...@contract:xFS2u...:~:0:100")?;
//! let seal = extract_seal(&parsed.beneficiary)?;
//!
//! // Use seal in transfer
//! let mut seals = SmallOrdMap::new();
//! seals.insert(0, seal);
//! let result = contract.call_method("transfer", params, seals).await?;
//! ```
//!
//! # See Also
//!
//! - [`F1r3flyConsignment`](crate::F1r3flyConsignment) - Uses WTxoSeal from invoices
//! - [`F1r3flyRgbContract`](crate::F1r3flyRgbContract) - Executes transfers
//! - [RGB Invoice Standard](https://github.com/RGB-WG/rgb-invoice) - Specification

use bpstd::{AddressPayload, ScriptBytes, ScriptPubkey};
use hypersonic::ContractId;
use rgb::Consensus;
use rgb_invoice::bp::WitnessOut;
use rgb_invoice::{RgbBeneficiary, RgbInvoice};
use strict_types::StrictVal;

use crate::{F1r3flyRgbError, WTxoSeal};

// ============================================================================
// Data Structures
// ============================================================================

/// Invoice generation request
///
/// Contains all parameters needed to generate an RGB invoice.
#[derive(Debug, Clone)]
pub struct InvoiceRequest {
    /// RGB contract ID
    pub contract_id: ContractId,

    /// Amount to receive (in smallest unit)
    pub amount: u64,

    /// Bitcoin address to receive at
    pub address: bitcoin::Address,

    /// Nonce for seal uniqueness (prevents replay attacks)
    pub nonce: u64,

    /// RGB consensus (Bitcoin or Liquid)
    pub consensus: Consensus,

    /// Whether this is testnet/regtest
    pub testnet: bool,
}

/// Generated RGB invoice with metadata
///
/// Result of invoice generation, containing the RGB invoice object
/// and associated tracking information.
#[derive(Debug, Clone)]
pub struct GeneratedInvoice {
    /// RGB invoice object (can call `.to_string()` for invoice string)
    pub invoice: RgbInvoice<ContractId>,

    /// WTxoSeal for recipient tracking
    ///
    /// This is the seal the sender will commit to in the witness transaction.
    /// Recipient can use this to track the expected UTXO.
    pub seal: WTxoSeal,

    /// Recipient Bitcoin address (for convenience)
    pub address: String,

    /// Amount requested
    pub amount: u64,
}

/// Parsed RGB invoice
///
/// Result of parsing an invoice string, containing extracted components
/// ready for transfer execution.
#[derive(Debug, Clone)]
pub struct ParsedInvoice {
    /// Original RGB invoice object
    pub invoice: RgbInvoice<ContractId>,

    /// Extracted contract ID
    pub contract_id: ContractId,

    /// Extracted beneficiary (WitnessOut or AuthToken)
    pub beneficiary: RgbBeneficiary,

    /// Extracted amount (if specified in invoice)
    pub amount: Option<u64>,
}

// ============================================================================
// Public API Functions
// ============================================================================

/// Generate RGB invoice (WitnessOut mode)
///
/// Creates an RGB invoice for receiving tokens at a specific Bitcoin address.
/// Uses WitnessOut beneficiary type - sender will create witness transaction
/// sending to this address.
///
/// # Arguments
///
/// * `contract_id` - RGB contract ID
/// * `amount` - Amount to receive (in smallest unit, e.g., satoshis for precision 8)
/// * `address` - Bitcoin address to receive at (any format: P2PKH, P2WPKH, P2TR, etc.)
/// * `nonce` - Nonce for seal uniqueness (prevents replay attacks, default: 0)
/// * `consensus` - RGB consensus (Bitcoin or Liquid)
/// * `testnet` - Whether this is testnet/regtest (affects invoice format)
///
/// # Returns
///
/// `GeneratedInvoice` with standard RGB invoice object and WTxoSeal for tracking
///
/// # Errors
///
/// Returns error if:
/// - Amount is zero
/// - Address conversion fails
/// - Invalid script format
///
/// # Example
///
/// ```ignore
/// use f1r3fly_rgb::invoice::generate_invoice;
/// use hypersonic::ContractId;
///
/// let contract_id = ContractId::from([1u8; 32]);
/// let address = bitcoin::Address::from_str("bcrt1q...")?.assume_checked();
/// let invoice = generate_invoice(
///     contract_id,
///     100,
///     address,
///     0,
///     rgb::Consensus::Bitcoin,
///     true,
/// )?;
///
/// println!("Share this: {}", invoice.invoice.to_string());
/// // Output: rgb:tb1q...@contract:xFS2u...:~:0:100
/// ```
pub fn generate_invoice(
    contract_id: ContractId,
    amount: u64,
    address: bitcoin::Address,
    nonce: u64,
    consensus: Consensus,
    testnet: bool,
) -> Result<GeneratedInvoice, F1r3flyRgbError> {
    // 1. Validate amount
    if amount == 0 {
        return Err(F1r3flyRgbError::InvalidResponse(
            "Amount must be greater than 0".to_string(),
        ));
    }

    // 2. Convert Bitcoin address to RgbBeneficiary (WitnessOut)
    let beneficiary = address_to_beneficiary(&address, nonce)?;

    // 3. Create WTxoSeal for recipient tracking
    let seal = extract_seal(&beneficiary)?;

    // 4. Create RGB invoice
    let invoice = RgbInvoice::new(
        contract_id,
        consensus,
        testnet,
        beneficiary,
        Some(StrictVal::num(amount)),
    );

    // 5. Return generated invoice with metadata
    Ok(GeneratedInvoice {
        invoice,
        seal,
        address: address.to_string(),
        amount,
    })
}

/// Parse RGB invoice string
///
/// Parses a standard RGB invoice received from recipient.
/// Extracts contract ID, beneficiary (WTxoSeal), and amount.
///
/// # Arguments
///
/// * `invoice_str` - Standard RGB invoice string (format: "rgb:...")
///
/// # Returns
///
/// `ParsedInvoice` with extracted data ready for transfer
///
/// # Errors
///
/// Returns error if:
/// - Invoice format is invalid
/// - Cannot parse contract ID
/// - Cannot extract beneficiary
///
/// # Example
///
/// ```ignore
/// use f1r3fly_rgb::invoice::parse_invoice;
///
/// let parsed = parse_invoice("rgb:tb1q...@contract:xFS2u...:~:0:100")?;
///
/// println!("Contract: {}", parsed.contract_id);
/// println!("Amount: {:?}", parsed.amount);
///
/// // Extract seal for transfer
/// let seal = extract_seal(&parsed.beneficiary)?;
/// ```
pub fn parse_invoice(invoice_str: &str) -> Result<ParsedInvoice, F1r3flyRgbError> {
    use std::str::FromStr;

    // Parse invoice using RGB standard format
    // Use ContractId directly as the scope type (not CallScope<ContractQuery>)
    // This matches the traditional RGB wallet approach
    let invoice = RgbInvoice::<ContractId>::from_str(invoice_str)
        .map_err(|e| F1r3flyRgbError::InvalidResponse(format!("Invalid RGB invoice: {}", e)))?;

    // Extract contract ID directly from scope (which is ContractId type)
    let contract_id = invoice.scope;

    // Extract amount if present (stored in `data` field as StrictVal)
    let amount = invoice.data.as_ref().and_then(|val| {
        // Match on StrictVal variants to extract number
        use strict_types::StrictVal::*;
        match val {
            Number(num) => {
                // StrictNum to u64 conversion via string
                // StrictNum implements Display, so we convert to string then parse
                num.to_string().parse::<u64>().ok()
            }
            _ => None,
        }
    });

    // Extract beneficiary
    let beneficiary = invoice.auth.clone();

    Ok(ParsedInvoice {
        invoice,
        contract_id,
        beneficiary,
        amount,
    })
}

/// Extract WTxoSeal from RgbBeneficiary
///
/// Converts an RGB beneficiary from a parsed invoice into a WTxoSeal
/// that can be used in transfer operations and consignment creation.
///
/// For WitnessOut beneficiaries (Phase 3), creates a seal pointing to
/// output 0 of the future witness transaction (the txid is unknown until
/// the sender creates the witness transaction).
///
/// # Arguments
///
/// * `beneficiary` - Beneficiary extracted from parsed invoice
///
/// # Returns
///
/// `WTxoSeal` ready for use in transfer operations
///
/// # Errors
///
/// Returns error if:
/// - Beneficiary is AuthToken (not supported in Phase 3)
///
/// # Example
///
/// ```ignore
/// use f1r3fly_rgb::invoice::{parse_invoice, extract_seal};
///
/// let parsed = parse_invoice("rgb:tb1q...@contract:xFS2u...:~:0:100")?;
/// let seal = extract_seal(&parsed.beneficiary)?;
///
/// // Use seal in transfer operation
/// // transfer_asset(contract_id, seal, amount)?;
/// ```
pub fn extract_seal(beneficiary: &RgbBeneficiary) -> Result<WTxoSeal, F1r3flyRgbError> {
    use bp::seals::{Noise, TxoSealExt, WOutpoint};
    use strict_types::StrictDumb;

    match beneficiary {
        RgbBeneficiary::WitnessOut(_wout) => {
            // Create seal pointing to output 0 of future witness transaction
            // The "blinding" is that txid is unknown until witness tx is created
            Ok(WTxoSeal {
                primary: WOutpoint::Wout(bp::Vout::from(0u32)),
                secondary: TxoSealExt::Noise(Noise::strict_dumb()),
            })
        }
        RgbBeneficiary::Token(_) => Err(F1r3flyRgbError::InvalidResponse(
            "AuthToken beneficiary not supported in Phase 3. Use WitnessOut invoices.".to_string(),
        )),
    }
}

/// Get recipient Bitcoin address from RgbBeneficiary
///
/// Extracts the Bitcoin address that will receive the RGB assets.
/// This is useful for displaying transfer destinations to users.
///
/// For WitnessOut beneficiaries, converts the AddressPayload back to
/// a standard Bitcoin address string.
///
/// # Arguments
///
/// * `beneficiary` - Beneficiary from parsed invoice
/// * `network` - Bitcoin network (mainnet, testnet, regtest, etc.)
///
/// # Returns
///
/// Bitcoin address string (e.g., "bcrt1q..." for regtest)
///
/// # Errors
///
/// Returns error if:
/// - Beneficiary is AuthToken (not supported in Phase 3)
/// - Address conversion fails
///
/// # Example
///
/// ```ignore
/// use f1r3fly_rgb::invoice::{parse_invoice, get_recipient_address};
/// use bitcoin::Network;
///
/// let parsed = parse_invoice("rgb:tb1q...@contract:xFS2u...:~:0:100")?;
/// let address = get_recipient_address(&parsed.beneficiary, Network::Regtest)?;
///
/// println!("Recipient address: {}", address);
/// ```
pub fn get_recipient_address(
    beneficiary: &RgbBeneficiary,
    network: bitcoin::Network,
) -> Result<String, F1r3flyRgbError> {
    match beneficiary {
        RgbBeneficiary::WitnessOut(wout) => {
            // Get script_pubkey from WitnessOut
            let bpstd_script = wout.script_pubkey();

            // Convert bpstd ScriptPubkey to bitcoin ScriptBuf
            let script_bytes = bpstd_script.as_slice();
            let bitcoin_script = bitcoin::ScriptBuf::from_bytes(script_bytes.to_vec());

            // Try to construct a Bitcoin address from the script
            let address = bitcoin::Address::from_script(&bitcoin_script, network).map_err(|e| {
                F1r3flyRgbError::InvalidResponse(format!("Cannot convert script to address: {}", e))
            })?;

            Ok(address.to_string())
        }
        RgbBeneficiary::Token(_) => Err(F1r3flyRgbError::InvalidResponse(
            "Cannot extract address from AuthToken beneficiary. Use WitnessOut invoices."
                .to_string(),
        )),
    }
}

// ============================================================================
// Internal Helper Functions
// ============================================================================

/// Convert Bitcoin address to RgbBeneficiary (WitnessOut)
///
/// Takes a Bitcoin address and creates a WitnessOut beneficiary with the specified nonce.
/// This is used internally by invoice generation.
///
/// # Arguments
///
/// * `address` - Bitcoin address (any format)
/// * `nonce` - Nonce for seal uniqueness
///
/// # Returns
///
/// `RgbBeneficiary::WitnessOut` ready for invoice creation
///
/// # Errors
///
/// Returns error if:
/// - Script bytes conversion fails
/// - Address payload extraction fails
fn address_to_beneficiary(
    address: &bitcoin::Address,
    nonce: u64,
) -> Result<RgbBeneficiary, F1r3flyRgbError> {
    // Convert bitcoin::Address to bpstd format
    let script_pubkey = address.script_pubkey();
    let script_bytes = ScriptBytes::try_from(script_pubkey.to_bytes())
        .map_err(|e| F1r3flyRgbError::InvalidResponse(format!("Invalid script bytes: {:?}", e)))?;
    let bpstd_script = ScriptPubkey::from(script_bytes);

    // Extract address payload
    let payload = AddressPayload::from_script(&bpstd_script).map_err(|e| {
        F1r3flyRgbError::InvalidResponse(format!("Cannot extract address payload: {}", e))
    })?;

    // Create WitnessOut beneficiary
    let witness_out = WitnessOut::new(payload, nonce);

    Ok(RgbBeneficiary::WitnessOut(witness_out))
}
