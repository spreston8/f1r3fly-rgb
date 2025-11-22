//! Signature generation utilities for secured RHO20 contract methods
//!
//! Provides helper functions for generating ECDSA signatures that match
//! Rholang's signature verification requirements using protobuf encoding.

use blake2::digest::consts::U32;
use blake2::{Blake2b, Digest as Blake2Digest};
use prost::Message as ProstMessage;
use secp256k1::{Message, SecretKey};

/// Generate signature for issue() method call
///
/// Creates a signature that matches the Rholang contract's verification using protobuf encoding.
/// The message format must exactly match what Rholang's `.toByteArray()` produces for the tuple
/// `(recipient, amount, nonce)`.
///
/// # Arguments
/// * `recipient` - The recipient identifier (typically a seal ID like "txid:vout")
/// * `amount` - The amount of tokens to issue
/// * `nonce` - A unique nonce for replay protection
/// * `signing_key` - The secp256k1 private key to sign with
///
/// # Returns
/// Hex-encoded DER signature string that can be passed to the Rholang `issue()` method
///
/// # Example
/// ```ignore
/// let signature = generate_issue_signature("alice", 1000, 12345, &private_key)?;
/// ```
pub fn generate_issue_signature(
    recipient: &str,
    amount: u64,
    nonce: u64,
    signing_key: &SecretKey,
) -> Result<String, Box<dyn std::error::Error>> {
    // Build protobuf Par structure for tuple: (recipient, amount, nonce)
    // Must match exactly what Rholang's .toByteArray() produces
    let par = f1r3fly_models::rhoapi::Par {
        exprs: vec![f1r3fly_models::rhoapi::Expr {
            expr_instance: Some(f1r3fly_models::rhoapi::expr::ExprInstance::ETupleBody(
                f1r3fly_models::rhoapi::ETuple {
                    ps: vec![
                        // First element: recipient (String)
                        f1r3fly_models::rhoapi::Par {
                            exprs: vec![f1r3fly_models::rhoapi::Expr {
                                expr_instance: Some(
                                    f1r3fly_models::rhoapi::expr::ExprInstance::GString(
                                        recipient.to_string(),
                                    ),
                                ),
                            }],
                            ..Default::default()
                        },
                        // Second element: amount (Int)
                        f1r3fly_models::rhoapi::Par {
                            exprs: vec![f1r3fly_models::rhoapi::Expr {
                                expr_instance: Some(
                                    f1r3fly_models::rhoapi::expr::ExprInstance::GInt(amount as i64),
                                ),
                            }],
                            ..Default::default()
                        },
                        // Third element: nonce (Int)
                        f1r3fly_models::rhoapi::Par {
                            exprs: vec![f1r3fly_models::rhoapi::Expr {
                                expr_instance: Some(
                                    f1r3fly_models::rhoapi::expr::ExprInstance::GInt(nonce as i64),
                                ),
                            }],
                            ..Default::default()
                        },
                    ],
                    ..Default::default()
                },
            )),
        }],
        ..Default::default()
    };

    // Encode to protobuf bytes (matches Rholang's .toByteArray())
    let message_bytes = par.encode_to_vec();

    // Hash with Blake2b-256
    let mut hasher = Blake2b::<U32>::new();
    hasher.update(&message_bytes);
    let message_hash: [u8; 32] = hasher.finalize().into();

    // Sign with secp256k1
    let secp = secp256k1::Secp256k1::new();
    let message_obj = Message::from_digest(message_hash);
    let signature = secp.sign_ecdsa(&message_obj, signing_key);

    // Rholang secpVerify expects DER-encoded signatures (variable length, typically 70-72 bytes)
    // This matches what rust-client uses in generate_insert_signed_signature
    Ok(hex::encode(signature.serialize_der()))
}

/// Generate signature for transfer() method call
///
/// Creates a signature that matches the Rholang contract's verification using protobuf encoding.
/// The message format must exactly match what Rholang's `.toByteArray()` produces for the tuple
/// `(from, to, amount, nonce)`.
///
/// # Arguments
/// * `from` - The sender's identifier (typically a seal ID like "txid:vout")
/// * `to` - The recipient's identifier (typically a seal ID like "txid:vout")
/// * `amount` - The amount of tokens to transfer
/// * `nonce` - A unique nonce for replay protection
/// * `signing_key` - The secp256k1 private key to sign with
///
/// # Returns
/// Hex-encoded DER signature string that can be passed to the Rholang `transfer()` method
///
/// # Example
/// ```ignore
/// let signature = generate_transfer_signature("alice_utxo", "bob_utxo", 100, 67890, &private_key)?;
/// ```
pub fn generate_transfer_signature(
    from: &str,
    to: &str,
    amount: u64,
    nonce: u64,
    signing_key: &SecretKey,
) -> Result<String, Box<dyn std::error::Error>> {
    // Build protobuf Par structure for tuple: (from, to, amount, nonce)
    // Must match exactly what Rholang's .toByteArray() produces
    let par = f1r3fly_models::rhoapi::Par {
        exprs: vec![f1r3fly_models::rhoapi::Expr {
            expr_instance: Some(f1r3fly_models::rhoapi::expr::ExprInstance::ETupleBody(
                f1r3fly_models::rhoapi::ETuple {
                    ps: vec![
                        // First element: from (String)
                        f1r3fly_models::rhoapi::Par {
                            exprs: vec![f1r3fly_models::rhoapi::Expr {
                                expr_instance: Some(
                                    f1r3fly_models::rhoapi::expr::ExprInstance::GString(
                                        from.to_string(),
                                    ),
                                ),
                            }],
                            ..Default::default()
                        },
                        // Second element: to (String)
                        f1r3fly_models::rhoapi::Par {
                            exprs: vec![f1r3fly_models::rhoapi::Expr {
                                expr_instance: Some(
                                    f1r3fly_models::rhoapi::expr::ExprInstance::GString(
                                        to.to_string(),
                                    ),
                                ),
                            }],
                            ..Default::default()
                        },
                        // Third element: amount (Int)
                        f1r3fly_models::rhoapi::Par {
                            exprs: vec![f1r3fly_models::rhoapi::Expr {
                                expr_instance: Some(
                                    f1r3fly_models::rhoapi::expr::ExprInstance::GInt(amount as i64),
                                ),
                            }],
                            ..Default::default()
                        },
                        // Fourth element: nonce (Int)
                        f1r3fly_models::rhoapi::Par {
                            exprs: vec![f1r3fly_models::rhoapi::Expr {
                                expr_instance: Some(
                                    f1r3fly_models::rhoapi::expr::ExprInstance::GInt(nonce as i64),
                                ),
                            }],
                            ..Default::default()
                        },
                    ],
                    ..Default::default()
                },
            )),
        }],
        ..Default::default()
    };

    // Encode to protobuf bytes (matches Rholang's .toByteArray())
    let message_bytes = par.encode_to_vec();

    // Hash with Blake2b-256
    let mut hasher = Blake2b::<U32>::new();
    hasher.update(&message_bytes);
    let message_hash: [u8; 32] = hasher.finalize().into();

    // Sign with secp256k1
    let secp = secp256k1::Secp256k1::new();
    let message_obj = Message::from_digest(message_hash);
    let signature = secp.sign_ecdsa(&message_obj, signing_key);

    // Rholang secpVerify expects DER-encoded signatures (variable length, typically 70-72 bytes)
    // This matches what rust-client uses in generate_insert_signed_signature
    Ok(hex::encode(signature.serialize_der()))
}

/// Generate a unique nonce for replay protection
///
/// Creates a nonce using timestamp (seconds since epoch) combined with a random component.
/// The result stays within Rholang's safe integer range while maintaining uniqueness.
///
/// # Returns
/// A unique 64-bit nonce value suitable for use in signed method calls
///
/// # Implementation
/// - Uses timestamp in seconds (not milliseconds) to keep numbers smaller
/// - Adds 16-bit random component for uniqueness within the same second
/// - Formula: `timestamp_seconds * 100000 + random` (keeps result < i64::MAX)
///
/// # Example
/// ```ignore
/// let nonce = generate_nonce();
/// ```
pub fn generate_nonce() -> u64 {
    use chrono::Utc;
    use secp256k1::rand::RngCore;

    // Use timestamp in seconds (not milliseconds) to keep numbers smaller
    let timestamp_sec = Utc::now().timestamp() as u64;

    // Add small random component for uniqueness within same second
    let mut rng_bytes = [0u8; 2];
    secp256k1::rand::rngs::OsRng.fill_bytes(&mut rng_bytes);
    let random = u16::from_le_bytes(rng_bytes) as u64;

    // Combine: timestamp * 100000 + random (keeps result < i64::MAX)
    timestamp_sec * 100000 + random
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nonce_uniqueness() {
        let n1 = generate_nonce();
        let n2 = generate_nonce();
        assert_ne!(n1, n2, "Nonces should be unique");
    }

    #[test]
    fn test_nonce_range() {
        let nonce = generate_nonce();
        // Ensure nonce stays within i64::MAX for Rholang compatibility
        assert!(nonce < i64::MAX as u64, "Nonce should be within i64 range");
    }

    #[test]
    fn test_signature_generation() {
        let private_key = SecretKey::from_slice(&[0x42; 32]).expect("valid key");
        let signature = generate_issue_signature("alice", 1000, 12345, &private_key).unwrap();

        // DER signatures are typically 70-72 bytes (140-144 hex chars)
        assert!(!signature.is_empty());
        assert!(signature.len() >= 140 && signature.len() <= 148);
    }

    #[test]
    fn test_signature_deterministic() {
        let private_key = SecretKey::from_slice(&[0x42; 32]).expect("valid key");
        let nonce = 12345u64; // Fixed for determinism

        let sig1 = generate_issue_signature("alice", 1000, nonce, &private_key).unwrap();
        let sig2 = generate_issue_signature("alice", 1000, nonce, &private_key).unwrap();

        assert_eq!(sig1, sig2, "Same inputs should produce same signature");
    }

    #[test]
    fn test_transfer_signature_generation() {
        let private_key = SecretKey::from_slice(&[0x42; 32]).expect("valid key");
        let signature =
            generate_transfer_signature("alice_utxo:0", "bob_utxo:0", 100, 67890, &private_key)
                .unwrap();

        // DER signatures are typically 70-72 bytes (140-144 hex chars)
        assert!(!signature.is_empty());
        assert!(signature.len() >= 140 && signature.len() <= 148);
    }

    #[test]
    fn test_transfer_signature_deterministic() {
        let private_key = SecretKey::from_slice(&[0x42; 32]).expect("valid key");
        let nonce = 67890u64; // Fixed for determinism

        let sig1 =
            generate_transfer_signature("alice_utxo:0", "bob_utxo:0", 100, nonce, &private_key)
                .unwrap();
        let sig2 =
            generate_transfer_signature("alice_utxo:0", "bob_utxo:0", 100, nonce, &private_key)
                .unwrap();

        assert_eq!(
            sig1, sig2,
            "Same inputs should produce same transfer signature"
        );
    }
}
