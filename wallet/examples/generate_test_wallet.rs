/// Generate a test wallet mnemonic and show its Regtest address
/// 
/// Usage:
/// ```bash
/// cd wallet
/// cargo run --example generate_test_wallet
/// ```

use bip39::Mnemonic;
use bitcoin::bip32::{DerivationPath, Xpriv, Xpub};
use bitcoin::key::{rand, CompressedPublicKey};
use bitcoin::{Address, Network, PublicKey};
use std::str::FromStr;

fn main() {
    println!("\n🔑 Generating Test Wallet for Regtest\n");
    println!("{}", "=".repeat(70));
    
    // Generate new mnemonic (12 words = 128 bits of entropy)
    let entropy: [u8; 16] = rand::random();
    let mnemonic = Mnemonic::from_entropy(&entropy).expect("Failed to generate mnemonic");
    let mnemonic_str = mnemonic.to_string();
    
    println!("\n📝 Mnemonic (12 words):");
    println!("   {}\n", mnemonic_str);
    
    // Derive first address for Regtest
    let network = Network::Regtest;
    let seed = mnemonic.to_seed("");
    let secp = bitcoin::secp256k1::Secp256k1::new();
    
    let master_key = Xpriv::new_master(network, &seed)
        .expect("Failed to create master key");
    
    // BIP84 path for Regtest: m/84'/1'/0'/0/0
    let path = DerivationPath::from_str("m/84'/1'/0'/0/0")
        .expect("Invalid derivation path");
    
    let derived_key = master_key.derive_priv(&secp, &path)
        .expect("Failed to derive key");
    
    let xpub = Xpub::from_priv(&secp, &derived_key);
    let pubkey = PublicKey::new(xpub.public_key);
    let compressed = CompressedPublicKey::try_from(pubkey)
        .expect("Failed to compress pubkey");
    let address = Address::p2wpkh(&compressed, network);
    
    println!("📍 First Address (m/84'/1'/0'/0/0):");
    println!("   {}\n", address);
    
    println!("{}", "=".repeat(70));
    println!("\n✅ To use this wallet for testing:\n");
    println!("1. Add to wallet/.env:");
    println!("   TEST_MNEMONIC=\"{}\"\n", mnemonic_str);
    println!("2. Fund the address:");
    println!("   bitcoin-cli -regtest -datadir=$HOME/.bitcoin sendtoaddress {} 10.0\n", address);
    println!("3. Mine a block:");
    println!("   bitcoin-cli -regtest -datadir=$HOME/.bitcoin generatetoaddress 1 $(bitcoin-cli -regtest -datadir=$HOME/.bitcoin getnewaddress)\n");
    println!("4. Run tests:");
    println!("   cargo test --test rgb_transfer_balance_test -- --ignored --nocapture\n");
}

