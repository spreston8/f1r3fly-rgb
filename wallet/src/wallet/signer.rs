use bip39::Mnemonic;
use bitcoin::bip32::{DerivationPath, Xpriv};
use bitcoin::Network;
use bpstd::psbt::{Rejected, Signer};
use bpstd::{KeyOrigin, LegacyPk, TapLeafHash, TapSighash, XOnlyPk, InternalPk, TapNodeHash, TapMerklePath};
use bpstd::secp256k1::Secp256k1;
use bpstd::Sign as DeriveSign;
use bpstd::Sighash;
use amplify::ByteArray;
use std::str::FromStr;

/// Wallet signer for P2WPKH (SegWit) addresses only.
///
/// This signer implementation:
/// - Derives keys from BIP39 mnemonic using BIP84 derivation paths (m/84'/1'/0'/0/x)
/// - Signs ECDSA signatures for P2WPKH witness v0 transactions
/// - Does NOT support Taproot/BIP340 signing (returns None for all Taproot methods)
///
/// For Taproot support, a different signer implementation would be needed.

#[derive(Clone)]
pub struct WalletSigner {
    mnemonic: Mnemonic,
    network: Network,
}

impl WalletSigner {
    pub fn new(mnemonic: Mnemonic, network: Network) -> Self {
        Self { mnemonic, network }
    }

    fn derive_key_from_origin(&self, origin: &KeyOrigin) -> Option<bpstd::secp256k1::SecretKey> {
        let seed = self.mnemonic.to_seed("");
        let secp = bitcoin::secp256k1::Secp256k1::new();

        let master_key = Xpriv::new_master(self.network, &seed).ok()?;

        // Build derivation path from KeyOrigin
        // KeyOrigin contains: (fingerprint, derivation_path)
        let path_str = format!("m/{}", origin.as_derivation());
        let path = DerivationPath::from_str(&path_str).ok()?;

        let derived_key = master_key.derive_priv(&secp, &path).ok()?;

        // Convert bitcoin::secp256k1::SecretKey to bpstd::secp256k1::SecretKey
        let key_bytes = derived_key.private_key.secret_bytes();
        bpstd::secp256k1::SecretKey::from_slice(&key_bytes).ok()
    }
}

impl Signer for WalletSigner {
    type Sign<'s> = Self where Self: 's;

    fn approve(&self, _psbt: &bpstd::Psbt) -> Result<Self::Sign<'_>, Rejected> {
        // No user interaction needed in backend wallet
        Ok(self.clone())
    }
}

impl DeriveSign for WalletSigner {
    fn sign_ecdsa(
        &self,
        sighash: Sighash,
        _pk: LegacyPk,
        origin: Option<&KeyOrigin>,
    ) -> Option<bpstd::secp256k1::ecdsa::Signature> {
        let origin = origin?;
        let secret_key = self.derive_key_from_origin(origin)?;

        let secp = Secp256k1::new();
        let message = bpstd::secp256k1::Message::from_digest(sighash.to_byte_array());

        Some(secp.sign_ecdsa(&message, &secret_key))
    }

    fn sign_bip340_key_only(
        &self,
        _sighash: TapSighash,
        _pk: InternalPk,
        _origin: Option<&KeyOrigin>,
        _merkle_root: Option<TapNodeHash>,
    ) -> Option<bpstd::secp256k1::schnorr::Signature> {
        // Not needed for P2WPKH (our descriptor type)
        None
    }

    fn sign_bip340_script_path(
        &self,
        _sighash: TapSighash,
        _pk: XOnlyPk,
        _origin: Option<&KeyOrigin>,
    ) -> Option<bpstd::secp256k1::schnorr::Signature> {
        // Not needed for P2WPKH (our descriptor type)
        None
    }

    fn should_sign_script_path(&self, _input: usize, _path: &TapMerklePath, _leaf_hash: TapLeafHash) -> bool {
        // Not needed for P2WPKH (our descriptor type)
        false
    }

    fn should_sign_key_path(&self, _input: usize) -> bool {
        // Not needed for P2WPKH (our descriptor type)
        false
    }
}

