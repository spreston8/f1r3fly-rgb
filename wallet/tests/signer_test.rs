use bip39::Mnemonic;
use bitcoin::Network;
use bpstd::Sign as DeriveSign;
use bpstd::{KeyOrigin, LegacyPk, Sighash};
use std::str::FromStr;

// Import the signer from wallet crate
use wallet::wallet::shared::signer::WalletSigner;

#[test]
fn test_signer_derivation() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .is_test(true)
        .try_init()
        .ok();

    log::info!("=== Starting Signer Derivation Test ===");

    // Test mnemonic (12 words)
    let mnemonic_phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    let mnemonic = Mnemonic::parse(mnemonic_phrase).expect("Failed to parse mnemonic");
    
    log::info!("Created test mnemonic");

    // Create signer
    let signer = WalletSigner::new(mnemonic, Network::Signet);
    log::info!("Created WalletSigner");

    // Create a KeyOrigin for path: m/84'/1'/0'/0/1
    // This is the path that's failing in the logs
    let derivation_path_str = "84h/1h/0h/0/1";
    log::info!("Creating KeyOrigin with path: {}", derivation_path_str);

    let derivation_path = bpstd::DerivationPath::from_str(derivation_path_str)
        .expect("Failed to parse derivation path");
    
    // Master fingerprint (dummy for testing)
    let fingerprint_bytes = [0xbd, 0x4c, 0x46, 0xf7]; // From logs
    let fingerprint = bpstd::XpubFp::from(fingerprint_bytes);
    
    let key_origin = KeyOrigin::new(fingerprint, derivation_path);
    log::info!("KeyOrigin created: {:?}", key_origin.as_derivation());

    // Test public key (from logs)
    let pk_hex = "02166b147542d51052b25cbcf9c74d5a22bbd3a271e3bf5e46341f24848ef25089";
    let pk_bytes = hex::decode(pk_hex).expect("Failed to decode pubkey hex");
    let pk = LegacyPk::from_bytes(&pk_bytes).expect("Failed to create LegacyPk");
    log::info!("Created test public key");

    // Create a dummy sighash (not used for derivation test)
    let sighash_bytes = [0u8; 32];
    let sighash = Sighash::from(sighash_bytes);

    // Test signing (this will call our derive_key_from_origin internally)
    log::info!("Attempting to sign with origin: {:?}", key_origin.as_derivation());
    
    let result = signer.sign_ecdsa(sighash, pk, Some(&key_origin));
    
    match result {
        Some(sig) => {
            log::info!("✅ SUCCESS: Signature created!");
            log::info!("Signature: {:?}", sig);
        }
        None => {
            log::error!("❌ FAILED: Could not create signature");
            panic!("Signer failed to derive key and sign");
        }
    }
}

#[test]
fn test_path_parsing() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .is_test(true)
        .try_init()
        .ok();

    log::info!("=== Testing Path Parsing ===");

    // Test different path formats
    let test_cases = vec![
        ("84h/1h/0h/0/1", "bpstd format (h notation)"),
        ("m/84'/1'/0'/0/1", "bitcoin crate format (' notation)"),
        ("/84h/1h/0h/0/1", "leading slash + h notation"),
    ];

    for (path_str, description) in test_cases {
        log::info!("Testing: {} - '{}'", description, path_str);
        
        // Try bpstd parsing
        let result: Result<bpstd::DerivationPath, _> = bpstd::DerivationPath::from_str(path_str);
        match result {
            Ok(p) => log::info!("  ✅ bpstd parsed successfully: {:?}", p),
            Err(e) => log::warn!("  ❌ bpstd failed: {:?}", e),
        }
        
        // Try bitcoin crate parsing
        let mut bitcoin_path = path_str.to_string();
        if !bitcoin_path.starts_with("m/") {
            bitcoin_path = format!("m/{}", bitcoin_path);
        }
        bitcoin_path = bitcoin_path.replace('h', "'");
        
        log::info!("  Converting to bitcoin format: '{}'", bitcoin_path);
        match bitcoin::bip32::DerivationPath::from_str(&bitcoin_path) {
            Ok(p) => log::info!("  ✅ bitcoin crate parsed successfully: {:?}", p),
            Err(e) => log::warn!("  ❌ bitcoin crate failed: {:?}", e),
        }
    }
}

