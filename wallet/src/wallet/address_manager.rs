use bitcoin::bip32::{ChildNumber, Xpub};
use bitcoin::key::CompressedPublicKey;
use bitcoin::secp256k1::Secp256k1;
use bitcoin::{Address, Network, PublicKey};
use std::str::FromStr;

pub struct AddressManager;

impl AddressManager {
    /// Derive a single P2WPKH address from a BIP84 descriptor at the specified index
    pub fn derive_address(
        descriptor: &str,
        index: u32,
        network: Network,
    ) -> Result<Address, crate::error::WalletError> {
        let xpub = Self::extract_xpub(descriptor)?;
        let secp = Secp256k1::new();

        let external_child = ChildNumber::from_normal_idx(0)
            .map_err(|e| crate::error::WalletError::Bitcoin(e.to_string()))?;
        let child_number = ChildNumber::from_normal_idx(index)
            .map_err(|e| crate::error::WalletError::Bitcoin(e.to_string()))?;

        let derived_key = xpub
            .derive_pub(&secp, &[external_child, child_number])
            .map_err(|e| crate::error::WalletError::Bitcoin(e.to_string()))?;

        let pubkey = PublicKey::new(derived_key.public_key);
        let compressed = CompressedPublicKey::try_from(pubkey)
            .map_err(|e| crate::error::WalletError::Bitcoin(e.to_string()))?;
        let address = Address::p2wpkh(&compressed, network);

        Ok(address)
    }

    /// Derive multiple addresses from a descriptor, returning (index, address) pairs
    pub fn derive_addresses(
        descriptor: &str,
        start: u32,
        count: u32,
        network: Network,
    ) -> Result<Vec<(u32, Address)>, crate::error::WalletError> {
        let mut addresses = Vec::with_capacity(count as usize);

        for i in 0..count {
            let index = start + i;
            let address = Self::derive_address(descriptor, index, network)?;
            addresses.push((index, address));
        }

        Ok(addresses)
    }

    /// Find the next unused address index by scanning for gaps in the used indices list
    pub fn find_next_unused(
        descriptor: &str,
        used_indices: &[u32],
        network: Network,
    ) -> Result<(u32, Address), crate::error::WalletError> {
        let mut index = 0u32;

        while used_indices.contains(&index) {
            index += 1;
        }

        let address = Self::derive_address(descriptor, index, network)?;
        Ok((index, address))
    }

    /// Extract the xpub/tpub from a BIP84 descriptor string
    fn extract_xpub(descriptor: &str) -> Result<Xpub, crate::error::WalletError> {
        let start = descriptor
            .find("tpub")
            .or_else(|| descriptor.find("xpub"))
            .ok_or_else(|| {
                crate::error::WalletError::InvalidDescriptor("No xpub/tpub found".into())
            })?;

        let end = descriptor[start..]
            .find('/')
            .map(|i| start + i)
            .unwrap_or(descriptor.len());

        let xpub_str = &descriptor[start..end];

        Xpub::from_str(xpub_str)
            .map_err(|e| crate::error::WalletError::InvalidDescriptor(e.to_string()))
    }
}
