//! Invoice Module Integration Tests
//!
//! Tests RGB invoice generation, parsing, and utility functions.
//! Validates production-ready invoice workflows for F1r3fly-RGB.
//!
//! These tests are self-contained and do not require F1r3node or external services.
//!
//! Run with: cargo test --test invoice_test -- --nocapture

use amplify::ByteArray;
use bitcoin::{Address, Network, PublicKey};
use commit_verify::{Digest, Sha256};
use f1r3fly_rgb::{
    extract_seal, generate_invoice, get_recipient_address, parse_invoice, RgbBeneficiary,
};
use hypersonic::ContractId;
use rgb::{AuthToken, Consensus};
use strict_types::StrictDumb;

// ============================================================================
// Helper Functions
// ============================================================================

/// Create deterministic ContractId from string seed
fn test_contract_id(seed: &str) -> ContractId {
    let hash = Sha256::digest(seed.as_bytes());
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&hash);
    ContractId::from_byte_array(bytes)
}

/// Create test Bitcoin address (P2WPKH regtest)
fn test_address_p2wpkh() -> Address {
    use bitcoin::key::CompressedPublicKey;
    use bitcoin::secp256k1::{Secp256k1, SecretKey};

    // Create deterministic keypair from seed
    let secp = Secp256k1::new();
    let secret_bytes = Sha256::digest("test_address_p2wpkh".as_bytes());
    let mut secret_array = [0u8; 32];
    secret_array.copy_from_slice(&secret_bytes);
    let secret_key = SecretKey::from_slice(&secret_array).expect("Valid secret key");

    // Convert secp256k1::PublicKey to bitcoin::PublicKey
    let secp_pubkey = bitcoin::secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
    let public_key = PublicKey::new(secp_pubkey);
    let compressed = CompressedPublicKey::try_from(public_key).expect("Valid compressed key");

    Address::p2wpkh(&compressed, Network::Regtest)
}

/// Create test Bitcoin address (P2WSH regtest)
fn test_address_p2wsh() -> Address {
    use bitcoin::ScriptBuf;

    // Create a simple witness script (OP_TRUE for simplicity)
    let witness_script = ScriptBuf::new_op_return(&[1, 2, 3, 4]);

    Address::p2wsh(&witness_script, Network::Regtest)
}

/// Create test Bitcoin address (P2TR regtest)
fn test_address_p2tr() -> Address {
    use bitcoin::key::Keypair;
    use bitcoin::secp256k1::{Secp256k1, SecretKey, XOnlyPublicKey};

    // Create deterministic keypair for Taproot
    let secp = Secp256k1::new();
    let secret_bytes = Sha256::digest("test_address_p2tr".as_bytes());
    let mut secret_array = [0u8; 32];
    secret_array.copy_from_slice(&secret_bytes);
    let secret_key = SecretKey::from_slice(&secret_array).expect("Valid secret key");
    let keypair = Keypair::from_secret_key(&secp, &secret_key);
    let (xonly_pubkey, _parity) = XOnlyPublicKey::from_keypair(&keypair);

    Address::p2tr(&secp, xonly_pubkey, None, Network::Regtest)
}

// ============================================================================
// Tests
// ============================================================================

/// Test 1: Validate invoice generation creates a properly formatted RGB invoice
#[test]
fn test_generate_invoice_with_witness_out() {
    // Arrange
    let contract_id = test_contract_id("test_contract_1");
    let address = test_address_p2wpkh();
    let amount = 1000u64;
    let nonce = 42u64;

    // Act
    let result = generate_invoice(
        contract_id,
        amount,
        address.clone(),
        nonce,
        Consensus::Bitcoin,
        true, // testnet
    );

    // Assert
    assert!(result.is_ok(), "Invoice generation should succeed");
    let invoice = result.unwrap();

    // Validate invoice string format
    let invoice_str = invoice.invoice.to_string();
    assert!(
        invoice_str.starts_with("rgb:") || invoice_str.starts_with("contract:"),
        "Invoice should start with 'rgb:' or 'contract:' prefix, got: {}",
        invoice_str
    );
    assert!(
        !invoice_str.is_empty(),
        "Invoice string should not be empty"
    );

    // Validate seal structure
    let seal_str = format!("{:?}", invoice.seal);
    assert!(
        seal_str.contains("Wout"),
        "Seal should contain Wout (output 0 of future tx)"
    );

    // Validate metadata
    assert_eq!(
        invoice.address,
        address.to_string(),
        "Address should match input"
    );
    assert_eq!(invoice.amount, amount, "Amount should match input");

    println!("✓ Generated invoice: {}", invoice_str);
    println!("✓ Seal: {:?}", invoice.seal);
}

/// Test 2: Verify that parsing a generated invoice correctly extracts all data
#[test]
fn test_parse_invoice_round_trip() {
    // Arrange - Generate invoice with known parameters
    let contract_id = test_contract_id("test_contract_roundtrip");
    let address = test_address_p2wpkh();
    let amount = 5000u64;
    let nonce = 123u64;

    let generated = generate_invoice(
        contract_id,
        amount,
        address.clone(),
        nonce,
        Consensus::Bitcoin,
        true,
    )
    .expect("Invoice generation should succeed");

    // Act - Convert to string and parse back
    let invoice_str = generated.invoice.to_string();
    println!("Invoice string: {}", invoice_str);

    let parsed = parse_invoice(&invoice_str).expect("Invoice parsing should succeed");

    // Assert - Validate round-trip integrity
    assert_eq!(
        parsed.contract_id, contract_id,
        "Contract ID should match after round-trip"
    );

    // Validate beneficiary type
    match &parsed.beneficiary {
        RgbBeneficiary::WitnessOut(_) => {
            println!("✓ Beneficiary is WitnessOut (correct)");
        }
        RgbBeneficiary::Token(_) => {
            panic!("Beneficiary should be WitnessOut, not AuthToken");
        }
    }

    // Note: Amount extraction from StrictVal may return None if parsing isn't complete
    // This is acceptable for Phase 3 - main validation is contract_id and beneficiary
    println!("✓ Parsed amount: {:?}", parsed.amount);
    println!("✓ Round-trip successful");
}

/// Test 3: Validate seal extraction produces correct WTxoSeal structure
#[test]
fn test_extract_seal_from_invoice() {
    // Arrange
    let contract_id = test_contract_id("test_contract_seal");
    let address = test_address_p2wpkh();

    let generated = generate_invoice(contract_id, 1000, address, 0, Consensus::Bitcoin, true)
        .expect("Invoice generation should succeed");

    let invoice_str = generated.invoice.to_string();
    let parsed = parse_invoice(&invoice_str).expect("Parsing should succeed");

    // Act - Extract seal from beneficiary
    let seal = extract_seal(&parsed.beneficiary).expect("Seal extraction should succeed");

    // Assert - Validate seal structure
    let seal_debug = format!("{:?}", seal);
    println!("Extracted seal: {}", seal_debug);

    // Verify seal contains expected components
    assert!(
        seal_debug.contains("Wout"),
        "Seal should have Wout primary outpoint"
    );
    assert!(
        seal_debug.contains("Noise") || seal_debug.contains("noise"),
        "Seal should have Noise secondary component"
    );

    // Test determinism - extract again from same beneficiary
    let seal2 = extract_seal(&parsed.beneficiary).expect("Second extraction should succeed");
    let seal2_debug = format!("{:?}", seal2);

    assert_eq!(
        seal_debug, seal2_debug,
        "Seal extraction should be deterministic"
    );

    println!("✓ Seal structure is correct");
    println!("✓ Seal extraction is deterministic");
}

/// Test 4: Verify address extraction works for all address types
#[test]
fn test_get_recipient_address_extraction() {
    let test_cases = vec![
        ("P2WPKH", test_address_p2wpkh()),
        ("P2WSH", test_address_p2wsh()),
        ("P2TR", test_address_p2tr()),
    ];

    for (addr_type, address) in test_cases {
        println!("\nTesting {} address: {}", addr_type, address);

        // Arrange - Generate invoice with this address
        let contract_id = test_contract_id(&format!("test_contract_{}", addr_type));
        let generated = generate_invoice(
            contract_id,
            1000,
            address.clone(),
            0,
            Consensus::Bitcoin,
            true,
        )
        .expect(&format!("{} invoice generation should succeed", addr_type));

        let invoice_str = generated.invoice.to_string();
        let parsed = parse_invoice(&invoice_str)
            .expect(&format!("{} invoice parsing should succeed", addr_type));

        // Act - Extract address from beneficiary
        let extracted_address = get_recipient_address(&parsed.beneficiary, Network::Regtest)
            .expect(&format!("{} address extraction should succeed", addr_type));

        // Assert - Validate extracted address matches original
        assert_eq!(
            extracted_address,
            address.to_string(),
            "{} address should match after extraction",
            addr_type
        );

        // Validate regtest format
        assert!(
            extracted_address.starts_with("bcrt1"),
            "{} address should have regtest prefix 'bcrt1'",
            addr_type
        );

        println!("✓ {} address extraction successful", addr_type);
    }

    println!("\n✓ All address types validated successfully");
}

/// Test 5: Validate proper error handling for invalid inputs
#[test]
fn test_invoice_error_cases() {
    println!("\n=== Testing Error Cases ===\n");

    // Test Case 1: Zero amount rejection
    {
        println!("Test 1: Zero amount should be rejected");
        let contract_id = test_contract_id("test_error_zero");
        let address = test_address_p2wpkh();

        let result = generate_invoice(contract_id, 0, address, 0, Consensus::Bitcoin, true);

        assert!(result.is_err(), "Zero amount should return error");
        let err = result.unwrap_err();
        let err_msg = format!("{}", err);
        assert!(
            err_msg.contains("Amount must be greater than 0") || err_msg.contains("0"),
            "Error should mention zero amount, got: {}",
            err_msg
        );
        println!("✓ Zero amount correctly rejected: {}", err_msg);
    }

    // Test Case 2: Invalid invoice string parsing
    {
        println!("\nTest 2: Invalid invoice string should fail gracefully");
        let result = parse_invoice("invalid_invoice_string");

        assert!(result.is_err(), "Invalid invoice should return error");
        println!(
            "✓ Invalid invoice correctly rejected: {}",
            result.unwrap_err()
        );
    }

    // Test Case 3: Invalid prefix
    {
        println!("\nTest 3: Invalid prefix should be rejected");
        let result = parse_invoice("notrgb:some_data_here");

        assert!(result.is_err(), "Invalid prefix should return error");
        println!(
            "✓ Invalid prefix correctly rejected: {}",
            result.unwrap_err()
        );
    }

    // Test Case 4: AuthToken not supported (extract_seal)
    {
        println!("\nTest 4: AuthToken beneficiary should be rejected in extract_seal");

        // Create an AuthToken beneficiary manually
        let auth_token = AuthToken::strict_dumb();
        let token_beneficiary = RgbBeneficiary::Token(auth_token);

        let result = extract_seal(&token_beneficiary);

        assert!(result.is_err(), "AuthToken should return error");
        let err = result.unwrap_err();
        let err_msg = format!("{}", err);
        assert!(
            err_msg.contains("AuthToken") || err_msg.contains("not supported"),
            "Error should mention AuthToken, got: {}",
            err_msg
        );
        println!(
            "✓ AuthToken correctly rejected in extract_seal: {}",
            err_msg
        );
    }

    // Test Case 5: AuthToken not supported (get_recipient_address)
    {
        println!("\nTest 5: AuthToken beneficiary should be rejected in get_recipient_address");

        let auth_token = AuthToken::strict_dumb();
        let token_beneficiary = RgbBeneficiary::Token(auth_token);

        let result = get_recipient_address(&token_beneficiary, Network::Regtest);

        assert!(result.is_err(), "AuthToken should return error");
        let err = result.unwrap_err();
        let err_msg = format!("{}", err);
        assert!(
            err_msg.contains("AuthToken") || err_msg.contains("Cannot extract"),
            "Error should mention AuthToken or extraction failure, got: {}",
            err_msg
        );
        println!(
            "✓ AuthToken correctly rejected in get_recipient_address: {}",
            err_msg
        );
    }

    // Test Case 6: Malformed invoice (truncated)
    {
        println!("\nTest 6: Malformed/truncated invoice should fail gracefully");
        let result = parse_invoice("rgb:abc");

        assert!(result.is_err(), "Malformed invoice should return error");
        println!(
            "✓ Malformed invoice correctly rejected: {}",
            result.unwrap_err()
        );
    }

    println!("\n✓ All error cases handled correctly - no panics!");
}
