//! F1r3fly Registry Operations
//!
//! This module provides cryptographic functions for interacting with F1r3fly's
//! `rho:registry:insertSigned:secp256k1` system contract.
//!
//! Key operations:
//! - Generate signatures for `insertSigned` registry operations
//! - Convert public keys to deterministic F1r3fly URIs
//! - Compute registry URIs from private keys

use blake2::digest::consts::U32;
use blake2::{Blake2b, Digest};
use chrono::{DateTime, Utc};
use prost::Message as _;
use secp256k1::{Message, PublicKey, Secp256k1, SecretKey};

/// Generate a signature for `insertSigned` registry operation
///
/// Creates a cryptographic signature required by F1r3fly's
/// `rho:registry:insertSigned:secp256k1` system contract.
///
/// # Arguments
/// * `key` - The secret key to sign with
/// * `timestamp` - The deployment timestamp
/// * `deployer` - The public key of the deployer
/// * `version` - The version number of the contract
///
/// # Returns
/// DER-encoded ECDSA signature as bytes
///
/// # Implementation
/// 1. Create a tuple: (timestamp_millis, deployer_pubkey_bytes, version)
/// 2. Encode as protobuf Par message
/// 3. Hash with Blake2b-256
/// 4. Sign with secp256k1
/// 5. Return DER-encoded signature
pub fn generate_insert_signed_signature(
    key: &SecretKey,
    timestamp: DateTime<Utc>,
    deployer: &PublicKey,
    version: i64,
) -> Vec<u8> {
    use f1r3fly_models::rhoapi;

    let par = rhoapi::Par {
        exprs: vec![rhoapi::Expr {
            expr_instance: Some(rhoapi::expr::ExprInstance::ETupleBody(rhoapi::ETuple {
                ps: vec![
                    rhoapi::Par {
                        exprs: vec![rhoapi::Expr {
                            expr_instance: Some(rhoapi::expr::ExprInstance::GInt(
                                timestamp.timestamp_millis(),
                            )),
                        }],
                        ..Default::default()
                    },
                    rhoapi::Par {
                        exprs: vec![rhoapi::Expr {
                            expr_instance: Some(rhoapi::expr::ExprInstance::GByteArray(
                                deployer.serialize_uncompressed().into(),
                            )),
                        }],
                        ..Default::default()
                    },
                    rhoapi::Par {
                        exprs: vec![rhoapi::Expr {
                            expr_instance: Some(rhoapi::expr::ExprInstance::GInt(version)),
                        }],
                        ..Default::default()
                    },
                ],
                ..Default::default()
            })),
        }],
        ..Default::default()
    }
    .encode_to_vec();

    let hash = Blake2b::<U32>::new().chain_update(par).finalize();
    let message = Message::from_digest(hash.into());

    Secp256k1::new()
        .sign_ecdsa(&message, key)
        .serialize_der()
        .to_vec()
}

/// Convert a public key to a F1r3fly registry URI
///
/// The URI format is: `rho:id:<zbase32-encoded-hash-with-crc14>`
///
/// # Arguments
/// * `public_key` - The secp256k1 public key
///
/// # Returns
/// A deterministic URI string that can be used to look up the contract
pub fn public_key_to_uri(public_key: &PublicKey) -> String {
    let pubkey_bytes = public_key.serialize_uncompressed();
    let hash = Blake2b::<U32>::new().chain_update(&pubkey_bytes).finalize();

    let crc_bytes = compute_crc14(&hash);

    let mut full_key = Vec::with_capacity(34);
    full_key.extend_from_slice(hash.as_ref());
    full_key.push(crc_bytes[0]);
    full_key.push(crc_bytes[1] << 2);

    let encoded = zbase32::encode(&full_key, 270);

    format!("rho:id:{}", encoded)
}

/// Compute registry URI from private key
///
/// Derives the public key from the private key and computes the deterministic URI.
///
/// # Arguments
/// * `private_key_hex` - Hex-encoded private key
///
/// # Returns
/// The registry URI or an error if the private key is invalid
pub fn compute_registry_uri_from_private_key(
    private_key_hex: &str,
) -> anyhow::Result<String> {
    let secp = Secp256k1::new();
    let secret_key_bytes = hex::decode(private_key_hex)?;
    let secret_key = SecretKey::from_slice(&secret_key_bytes)?;
    let public_key = PublicKey::from_secret_key(&secp, &secret_key);

    Ok(public_key_to_uri(&public_key))
}

/// Compute CRC14 checksum for URI generation
///
/// Returns the CRC as little-endian bytes
fn compute_crc14(data: &[u8]) -> [u8; 2] {
    use crc::{Algorithm, Crc};

    const CRC14: Algorithm<u16> = Algorithm {
        width: 14,
        poly: 0x4805,
        init: 0x0000,
        refin: false,
        refout: false,
        xorout: 0x0000,
        check: 0,
        residue: 0x0000,
    };

    let crc = Crc::<u16>::new(&CRC14);
    let mut digest = crc.digest();
    digest.update(data);
    digest.finalize().to_le_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_public_key_to_uri() {
        let secp = Secp256k1::new();
        let secret_key = SecretKey::from_slice(&[1u8; 32]).unwrap();
        let public_key = PublicKey::from_secret_key(&secp, &secret_key);

        let uri = public_key_to_uri(&public_key);
        assert!(uri.starts_with("rho:id:"));
        assert_eq!(uri.len(), 61); // "rho:id:" (7) + zbase32 (54)
    }

    #[test]
    fn test_compute_registry_uri_from_private_key() {
        let private_key_hex = "0101010101010101010101010101010101010101010101010101010101010101";
        let uri = compute_registry_uri_from_private_key(private_key_hex).unwrap();
        assert!(uri.starts_with("rho:id:"));
    }

    #[test]
    fn test_generate_insert_signed_signature() {
        let secp = Secp256k1::new();
        let secret_key = SecretKey::from_slice(&[1u8; 32]).unwrap();
        let public_key = PublicKey::from_secret_key(&secp, &secret_key);
        let timestamp = Utc::now();
        let version = 1;

        let signature = generate_insert_signed_signature(&secret_key, timestamp, &public_key, version);
        assert!(!signature.is_empty());
        assert!(signature.len() > 60); // DER signatures are typically 70-72 bytes
    }
}

