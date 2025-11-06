use bip39::Mnemonic;
use bitcoin::bip32::{DerivationPath, Fingerprint, Xpriv, Xpub};
use bitcoin::key::rand;
use bitcoin::secp256k1::Secp256k1;
use bitcoin::Network;
use std::str::FromStr;

pub struct KeyManager;

impl KeyManager {
    /// Generate a new random wallet with mnemonic and keys
    pub fn generate() -> Result<WalletKeys, crate::error::WalletError> {
        let entropy = rand::random::<[u8; 16]>();

        let mnemonic = Mnemonic::from_entropy(&entropy)
            .map_err(|e| crate::error::WalletError::InvalidMnemonic(e.to_string()))?;

        Self::derive_keys(mnemonic)
    }

    /// Import a wallet from an existing mnemonic phrase
    pub fn from_mnemonic(words: &str) -> Result<WalletKeys, crate::error::WalletError> {
        let mnemonic = Mnemonic::parse(words)
            .map_err(|e| crate::error::WalletError::InvalidMnemonic(e.to_string()))?;

        Self::derive_keys(mnemonic)
    }

    /// Derive BIP84 keys and descriptor from a mnemonic
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

    /// Create a BIP84 descriptor string with the given xpub and fingerprint
    fn create_descriptor(xpub: &Xpub, fingerprint: Fingerprint) -> String {
        format!("[{:08x}/84h/1h/0h]{}/<0;1>/*", fingerprint, xpub)
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

