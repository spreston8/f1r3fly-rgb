/// Derive Bitcoin address from a mnemonic
/// 
/// Usage:
/// ```bash
/// cd wallet
/// cargo run --example derive_address -- "your twelve word mnemonic here"
/// ```

use bip39::Mnemonic;
use bitcoin::bip32::{DerivationPath, Xpriv, Xpub};
use bitcoin::key::CompressedPublicKey;
use bitcoin::{Address, Network, PublicKey};
use std::env;
use std::str::FromStr;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        eprintln!("\n❌ Error: Mnemonic required\n");
        eprintln!("Usage:");
        eprintln!("  cargo run --example derive_address -- \"your twelve word mnemonic here\"\n");
        eprintln!("Example:");
        eprintln!("  cargo run --example derive_address -- \"abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about\"\n");
        std::process::exit(1);
    }
    
    // Join all args after program name (handles spaces)
    let mnemonic_str = args[1..].join(" ");
    
    println!("\n🔑 Deriving Address from Mnemonic\n");
    println!("{}", "=".repeat(70));
    
    // Parse mnemonic
    let mnemonic = match Mnemonic::parse(&mnemonic_str) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("\n❌ Invalid mnemonic: {}\n", e);
            std::process::exit(1);
        }
    };
    
    println!("\n📝 Mnemonic: {}\n", mnemonic);
    
    // Derive addresses for different networks
    let networks = vec![
        ("Bitcoin Mainnet", Network::Bitcoin, "m/84'/0'/0'/0/0"),
        ("Testnet", Network::Testnet, "m/84'/1'/0'/0/0"),
        ("Signet", Network::Signet, "m/84'/1'/0'/0/0"),
        ("Regtest", Network::Regtest, "m/84'/1'/0'/0/0"),
    ];
    
    println!("📍 Derived Addresses:\n");
    
    for (network_name, network, path_str) in networks {
        let seed = mnemonic.to_seed("");
        let secp = bitcoin::secp256k1::Secp256k1::new();
        
        let master_key = Xpriv::new_master(network, &seed)
            .expect("Failed to create master key");
        
        let path = DerivationPath::from_str(path_str)
            .expect("Invalid derivation path");
        
        let derived_key = master_key.derive_priv(&secp, &path)
            .expect("Failed to derive key");
        
        let xpub = Xpub::from_priv(&secp, &derived_key);
        let pubkey = PublicKey::new(xpub.public_key);
        let compressed = CompressedPublicKey::try_from(pubkey)
            .expect("Failed to compress pubkey");
        let address = Address::p2wpkh(&compressed, network);
        
        println!("  {} ({}):", network_name, path_str);
        println!("    {}\n", address);
    }
    
    println!("{}", "=".repeat(70));
    println!("\n💡 To fund the Regtest address:\n");
    let seed = mnemonic.to_seed("");
    let secp = bitcoin::secp256k1::Secp256k1::new();
    let master_key = Xpriv::new_master(Network::Regtest, &seed)
        .expect("Failed to create master key");
    let path = DerivationPath::from_str("m/84'/1'/0'/0/0")
        .expect("Invalid derivation path");
    let derived_key = master_key.derive_priv(&secp, &path)
        .expect("Failed to derive key");
    let xpub = Xpub::from_priv(&secp, &derived_key);
    let pubkey = PublicKey::new(xpub.public_key);
    let compressed = CompressedPublicKey::try_from(pubkey)
        .expect("Failed to compress pubkey");
    let regtest_address = Address::p2wpkh(&compressed, Network::Regtest);
    
    println!("bitcoin-cli -regtest -datadir=$HOME/.bitcoin sendtoaddress {} 10.0", regtest_address);
    println!("bitcoin-cli -regtest -datadir=$HOME/.bitcoin generatetoaddress 1 $(bitcoin-cli -regtest -datadir=$HOME/.bitcoin getnewaddress)\n");
}

