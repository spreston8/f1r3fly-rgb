use bip39::Mnemonic;
use bitcoin::bip32::{DerivationPath, Fingerprint, Xpriv, Xpub};
use bitcoin::key::rand;
use bitcoin::secp256k1::Secp256k1;
use bitcoin::Network;
use std::str::FromStr;

pub struct KeyManager;

impl KeyManager {
    pub fn generate() -> Result<WalletKeys, crate::error::WalletError> {
        let entropy = rand::random::<[u8; 16]>();

        let mnemonic = Mnemonic::from_entropy(&entropy)
            .map_err(|e| crate::error::WalletError::InvalidMnemonic(e.to_string()))?;

        Self::derive_keys(mnemonic)
    }

    pub fn from_mnemonic(words: &str) -> Result<WalletKeys, crate::error::WalletError> {
        let mnemonic = Mnemonic::parse(words)
            .map_err(|e| crate::error::WalletError::InvalidMnemonic(e.to_string()))?;

        Self::derive_keys(mnemonic)
    }

    fn derive_keys(mnemonic: Mnemonic) -> Result<WalletKeys, crate::error::WalletError> {
        // Load network from config
        let config = crate::config::WalletConfig::from_env();
        let network = config.bitcoin_network;
        let secp = Secp256k1::new();

        let seed = mnemonic.to_seed("");

        let master_key = Xpriv::new_master(network, &seed)
            .map_err(|e| crate::error::WalletError::Bitcoin(e.to_string()))?;

        let fingerprint = master_key.fingerprint(&secp);

        let derivation_path = DerivationPath::from_str("m/84'/1'/0'")
            .map_err(|e| crate::error::WalletError::Bitcoin(e.to_string()))?;

        let account_key = master_key
            .derive_priv(&secp, &derivation_path)
            .map_err(|e| crate::error::WalletError::Bitcoin(e.to_string()))?;

        let xpub = Xpub::from_priv(&secp, &account_key);

        let descriptor = Self::create_descriptor(&xpub, fingerprint);

        Ok(WalletKeys {
            mnemonic,
            xprv: account_key,
            xpub,
            descriptor,
            fingerprint: format!("{:08x}", fingerprint),
            network,
        })
    }

    fn create_descriptor(xpub: &Xpub, fingerprint: Fingerprint) -> String {
        format!("[{:08x}/84h/1h/0h]{}/<0;1>/*", fingerprint, xpub)
    }

    /// Derive RGB issuer public key from wallet mnemonic
    /// 
    /// Uses a dedicated derivation path: m/84'/1'/0'/2/0
    /// - Chain 2 is reserved for RGB identity (separate from spending keys)
    /// - Returns compressed public key in hex format (33 bytes)
    /// 
    /// This provides:
    /// - Deterministic issuer identity (same wallet = same issuer)
    /// - Cryptographic verifiability (can prove ownership with signature)
    /// - Security isolation (separate from spending keys)
    pub fn derive_rgb_issuer_pubkey(
        storage: &super::Storage,
        wallet_name: &str,
    ) -> Result<String, crate::error::WalletError> {
        // Load wallet mnemonic
        let mnemonic = storage.load_mnemonic(wallet_name)?;
        
        // Derive keys
        let secp = Secp256k1::new();
        let seed = mnemonic.to_seed("");
        
        // Get network from config
        let config = crate::config::WalletConfig::from_env();
        let network = config.bitcoin_network;
        
        // Derive master key
        let master_key = Xpriv::new_master(network, &seed)
            .map_err(|e| crate::error::WalletError::Bitcoin(format!("Failed to derive master key: {}", e)))?;
        
        // Use dedicated derivation path for RGB issuer identity
        // m/84'/1'/0'/2/0 where:
        // - 84' = BIP84 (native segwit)
        // - 1' = testnet/signet
        // - 0' = account 0
        // - 2 = RGB identity chain (not used for spending)
        // - 0 = first key in chain
        let derivation_path = DerivationPath::from_str("m/84'/1'/0'/2/0")
            .map_err(|e| crate::error::WalletError::Bitcoin(format!("Invalid derivation path: {}", e)))?;
        
        // Derive the key
        let derived_key = master_key
            .derive_priv(&secp, &derivation_path)
            .map_err(|e| crate::error::WalletError::Bitcoin(format!("Failed to derive issuer key: {}", e)))?;
        
        // Get the public key from the private key
        let public_key = Xpub::from_priv(&secp, &derived_key);
        
        // Serialize as compressed public key (33 bytes)
        let compressed_pubkey = public_key.public_key.serialize();
        
        // Return as hex string
        Ok(hex::encode(&compressed_pubkey))
    }
}

pub struct WalletKeys {
    pub mnemonic: Mnemonic,
    pub xprv: Xpriv,
    pub xpub: Xpub,
    pub descriptor: String,
    pub fingerprint: String,
    pub network: Network,
}
