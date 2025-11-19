# Rho20 Persistent Contract Testing - Implementation Plan

**Document Purpose:** Implementation plan for Phase B testing with persistent rho20 contracts, registry URIs, HTTP state verification, and robust assertion testing.

**Created:** November 6, 2025  
**Project:** F1r3fly-RGB Wallet  
**Status:** Planning - Awaiting Review

---

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Layer 1: Connection Manager Extensions](#layer-1-connection-manager-extensions)
4. [Layer 2: Test Helpers](#layer-2-test-helpers)
5. [Layer 3: Test Cases](#layer-3-test-cases)
6. [Configuration & Timeouts](#configuration--timeouts)
7. [Dependencies](#dependencies)
8. [Implementation Checklist](#implementation-checklist)
9. [Open Questions & Decisions](#open-questions--decisions)

---

## Overview

### Goals

- ✅ Deploy persistent rho20 contracts with `insertSigned` registry registration
- ✅ Call contract methods via registry URI lookup
- ✅ Verify state changes using HTTP queries with full JSON parsing
- ✅ Work on both fresh nodes (CI) and nodes with existing state (dev)
- ✅ Remove all `println!` debug output
- ✅ Use robust assertions on parsed JSON responses
- ✅ Configurable timeouts with proper error handling

### Test Flow

```
┌─────────────────────────────────────────────────────────────────┐
│ PHASE 1: Deploy Persistent Rho20 Contract (Once per suite)     │
├─────────────────────────────────────────────────────────────────┤
│ 1. Generate Rholang with insertSigned                           │
│ 2. Sign with private key (signature bundle)                     │
│ 3. Deploy to f1r3node                                           │
│ 4. Compute deterministic registry URI                           │
│ 5. Cache URI with OnceLock (thread-safe, idempotent)           │
│                                                                   │
│ Result: rho:id:abc123... (persistent, callable contract)       │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│ PHASE 2: Call Contract Methods                                  │
├─────────────────────────────────────────────────────────────────┤
│ 1. Lookup registry URI                                          │
│ 2. Call contract method (e.g., "storeAllocation")              │
│ 3. Wait for deploy finalization                                 │
│                                                                   │
│ Example Rholang:                                                 │
│   new rl(`rho:registry:lookup`), contractCh, ack in {          │
│     rl!(`rho:id:abc123...`, *contractCh) |                     │
│     for(@(_, Rho20Token) <- contractCh) {                      │
│       @Rho20Token!("storeAllocation", seal, amount, *ack)      │
│     }                                                            │
│   }                                                              │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│ PHASE 3: Verify State with HTTP Query                          │
├─────────────────────────────────────────────────────────────────┤
│ 1. Generate query Rholang                                       │
│ 2. POST to HTTP API (/api/explore-deploy)                      │
│ 3. Parse full JSON response                                     │
│ 4. Unwrap Rholang types (ExprMap → plain JSON)                 │
│ 5. Assert on values                                              │
│                                                                   │
│ Example Query:                                                   │
│   new ret in {                                                  │
│     @Rho20Token!("getAllocation", seal, *ret)                  │
│   }                                                              │
│                                                                   │
│ Response (HTTP):                                                 │
│   {"expr": [{"ExprMap": {"data": {...}}}]}                     │
│                                                                   │
│ Parsed to Plain JSON:                                           │
│   {"success": true, "allocation": {"amount": 1000, ...}}       │
│                                                                   │
│ Assertion:                                                       │
│   assert_eq!(result["allocation"]["amount"].as_i64(), 1000);   │
└─────────────────────────────────────────────────────────────────┘
```

---

## Architecture

### Three-Layer Approach

```
┌────────────────────────────────────────────────────────────────┐
│ Layer 3: Test Cases (rgb-std/tests/rholang_execution_test.rs) │
│  - Deploy rho20 contract (once, cached with OnceLock)          │
│  - Call methods (storeAllocation, getAllocation, transfer)     │
│  - Query & assert state via HTTP                               │
└────────────────────────────────────────────────────────────────┘
                             ↓ uses
┌────────────────────────────────────────────────────────────────┐
│ Layer 2: Test Helpers (rgb-std/tests/rholang_test_helpers.rs) │
│  - ensure_rho20_deployed() with OnceLock caching               │
│  - generate_rho20_deployment() with template substitution      │
│  - generate_store_allocation_call()                            │
│  - generate_get_allocation_call()                              │
│  - generate_transfer_call()                                    │
│  - generate_get_metadata_call()                                │
└────────────────────────────────────────────────────────────────┘
                             ↓ uses
┌────────────────────────────────────────────────────────────────┐
│ Layer 1: Connection Manager Extensions (rust-client/src/)     │
│  - query_via_http() - POST to /api/explore-deploy             │
│  - query_and_parse() - HTTP + unwrap Rholang types            │
│  - Registry module (compute URI, generate signature)           │
│  - Rholang helpers (ExprMap/ExprString/etc. unwrapping)       │
└────────────────────────────────────────────────────────────────┘
```

---

## Layer 1: Connection Manager Extensions

### 1.1: HTTP Query Methods

**File:** `rust-client/src/connection_manager.rs`

**New Methods:**

```rust
impl F1r3flyConnectionManager {
    /// Query via HTTP API (returns full JSON with Rholang type wrappers)
    /// 
    /// This method uses the HTTP `/api/explore-deploy` endpoint instead of gRPC
    /// because it returns the complete JSON structure with type information,
    /// whereas gRPC returns simplified strings like "Complex expression with 2 fields".
    /// 
    /// # Arguments
    /// 
    /// * `rholang_code` - The Rholang query code to execute
    /// 
    /// # Returns
    /// 
    /// Full JSON response: `{"expr": [...], "block": {...}}`
    /// 
    /// # Errors
    /// 
    /// Returns `ConnectionError::QueryFailed` if HTTP request fails
    pub async fn query_via_http(
        &self,
        rholang_code: &str,
    ) -> Result<serde_json::Value, ConnectionError> {
        use reqwest::Client;
        
        let url = format!(
            "http://{}:{}/api/explore-deploy",
            self.config.node_host,
            self.config.http_port
        );
        
        let client = Client::new();
        let response = client
            .post(&url)
            .header("Content-Type", "text/plain")
            .body(rholang_code.to_string())
            .send()
            .await
            .map_err(|e| ConnectionError::OperationFailed(
                format!("HTTP request failed: {}", e)
            ))?;
        
        if !response.status().is_success() {
            return Err(ConnectionError::OperationFailed(
                format!("HTTP error: {}", response.status())
            ));
        }
        
        response
            .json()
            .await
            .map_err(|e| ConnectionError::OperationFailed(
                format!("Failed to parse JSON response: {}", e)
            ))
    }
    
    /// Query and parse Rholang response to plain JSON
    /// 
    /// This is a convenience method that:
    /// 1. Executes the query via HTTP
    /// 2. Extracts the first expression from the response
    /// 3. Unwraps Rholang type wrappers (ExprMap, ExprString, etc.)
    /// 4. Returns plain, parseable JSON
    /// 
    /// # Arguments
    /// 
    /// * `rholang_code` - The Rholang query code to execute
    /// 
    /// # Returns
    /// 
    /// Plain JSON value with Rholang types unwrapped
    /// 
    /// # Example
    /// 
    /// ```ignore
    /// let result = manager.query_and_parse(query_code).await?;
    /// let amount = result["allocation"]["amount"].as_i64().unwrap();
    /// assert_eq!(amount, 1000);
    /// ```
    pub async fn query_and_parse(
        &self,
        rholang_code: &str,
    ) -> Result<serde_json::Value, ConnectionError> {
        use crate::rholang_helpers::convert_rholang_to_json;
        
        // Get full JSON response
        let json_response = self.query_via_http(rholang_code).await?;
        
        // Extract expr array
        let expr_array = json_response
            .get("expr")
            .and_then(|v| v.as_array())
            .ok_or_else(|| ConnectionError::OperationFailed(
                "Invalid response format: missing 'expr' array".to_string()
            ))?;
        
        if expr_array.is_empty() {
            return Err(ConnectionError::OperationFailed(
                "No data returned from query".to_string()
            ));
        }
        
        // Get first expression (the return value)
        let rholang_expr = &expr_array[0];
        
        // Convert Rholang types to plain JSON
        convert_rholang_to_json(rholang_expr)
            .map_err(|e| ConnectionError::OperationFailed(
                format!("Failed to parse Rholang response: {}", e)
            ))
    }
}
```

**Error Types to Add:**

```rust
pub enum ConnectionError {
    // ... existing variants ...
    
    /// Query operation failed
    QueryFailed(String),
    
    /// Timeout waiting for operation
    Timeout(String),
}
```

---

### 1.2: Rholang Parser Module

**File:** `rust-client/src/rholang_helpers.rs` (EXISTING - REUSED)

```rust
//! Rholang JSON Parser
//! 
//! Converts Rholang-typed JSON responses to plain JSON for easy parsing.
//! 
//! F1r3fly's HTTP API returns Rholang expressions wrapped in type tags:
//! - `{"ExprString": {"data": "hello"}}` → `"hello"`
//! - `{"ExprInt": {"data": 42}}` → `42`
//! - `{"ExprBool": {"data": true}}` → `true`
//! - `{"ExprMap": {"data": {...}}}` → `{...}` (recursive)
//! - `{"ExprList": {"data": [...]}}` → `[...]` (recursive)

use serde_json::{Value, Map};

/// Convert Rholang-typed JSON to plain JSON
/// 
/// Recursively unwraps Rholang type wrappers to produce clean, parseable JSON.
/// 
/// # Arguments
/// 
/// * `value` - The Rholang-typed JSON value
/// 
/// # Returns
/// 
/// Plain JSON value with type wrappers removed
/// 
/// # Errors
/// 
/// Returns error if JSON structure is invalid or unexpected
pub fn convert_rholang_to_json(value: &Value) -> Result<Value, String> {
    match value {
        Value::Object(obj) => {
            // ExprString: {"ExprString": {"data": "..."}}
            if let Some(expr_string) = obj.get("ExprString") {
                if let Some(data) = expr_string.get("data") {
                    return Ok(data.clone());
                }
            }
            
            // ExprInt: {"ExprInt": {"data": 42}}
            if let Some(expr_int) = obj.get("ExprInt") {
                if let Some(data) = expr_int.get("data") {
                    return Ok(data.clone());
                }
            }
            
            // ExprBool: {"ExprBool": {"data": true}}
            if let Some(expr_bool) = obj.get("ExprBool") {
                if let Some(data) = expr_bool.get("data") {
                    return Ok(data.clone());
                }
            }
            
            // ExprMap: {"ExprMap": {"data": {...}}} (recursive)
            if let Some(expr_map) = obj.get("ExprMap") {
                if let Some(data) = expr_map.get("data").and_then(|v| v.as_object()) {
                    let mut result = Map::new();
                    for (key, value) in data {
                        result.insert(
                            key.clone(),
                            convert_rholang_to_json(value)?
                        );
                    }
                    return Ok(Value::Object(result));
                }
            }
            
            // ExprList: {"ExprList": {"data": [...]}} (recursive)
            if let Some(expr_list) = obj.get("ExprList") {
                if let Some(data) = expr_list.get("data").and_then(|v| v.as_array()) {
                    let mut result = Vec::new();
                    for item in data {
                        result.push(convert_rholang_to_json(item)?);
                    }
                    return Ok(Value::Array(result));
                }
            }
            
            // Plain object (recurse on values)
            let mut result = Map::new();
            for (key, value) in obj {
                result.insert(
                    key.clone(),
                    convert_rholang_to_json(value)?
                );
            }
            Ok(Value::Object(result))
        }
        
        // Primitives pass through
        _ => Ok(value.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[test]
    fn test_expr_string() {
        let input = json!({"ExprString": {"data": "hello"}});
        let output = convert_rholang_to_json(&input).unwrap();
        assert_eq!(output, json!("hello"));
    }
    
    #[test]
    fn test_expr_int() {
        let input = json!({"ExprInt": {"data": 42}});
        let output = convert_rholang_to_json(&input).unwrap();
        assert_eq!(output, json!(42));
    }
    
    #[test]
    fn test_expr_bool() {
        let input = json!({"ExprBool": {"data": true}});
        let output = convert_rholang_to_json(&input).unwrap();
        assert_eq!(output, json!(true));
    }
    
    #[test]
    fn test_expr_map() {
        let input = json!({
            "ExprMap": {
                "data": {
                    "name": {"ExprString": {"data": "Alice"}},
                    "balance": {"ExprInt": {"data": 1000}},
                    "active": {"ExprBool": {"data": true}}
                }
            }
        });
        let output = convert_rholang_to_json(&input).unwrap();
        assert_eq!(output, json!({
            "name": "Alice",
            "balance": 1000,
            "active": true
        }));
    }
    
    #[test]
    fn test_expr_list() {
        let input = json!({
            "ExprList": {
                "data": [
                    {"ExprInt": {"data": 1}},
                    {"ExprInt": {"data": 2}},
                    {"ExprInt": {"data": 3}}
                ]
            }
        });
        let output = convert_rholang_to_json(&input).unwrap();
        assert_eq!(output, json!([1, 2, 3]));
    }
    
    #[test]
    fn test_nested_structures() {
        let input = json!({
            "ExprMap": {
                "data": {
                    "users": {
                        "ExprList": {
                            "data": [
                                {
                                    "ExprMap": {
                                        "data": {
                                            "name": {"ExprString": {"data": "Alice"}},
                                            "balance": {"ExprInt": {"data": 100}}
                                        }
                                    }
                                },
                                {
                                    "ExprMap": {
                                        "data": {
                                            "name": {"ExprString": {"data": "Bob"}},
                                            "balance": {"ExprInt": {"data": 200}}
                                        }
                                    }
                                }
                            ]
                        }
                    }
                }
            }
        });
        let output = convert_rholang_to_json(&input).unwrap();
        assert_eq!(output, json!({
            "users": [
                {"name": "Alice", "balance": 100},
                {"name": "Bob", "balance": 200}
            ]
        }));
    }
}
```

---

### 1.3: Registry Module

**File:** `rust-client/src/registry.rs` (NEW)

```rust
//! Registry URI and Signature Generation
//! 
//! Provides utilities for:
//! - Computing deterministic registry URIs from private keys
//! - Generating signatures for insertSigned registration
//! 
//! Based on RChain's registry implementation using Blake2b-256 hashing,
//! CRC14 checksums, and zbase32 encoding.

use blake2::{Blake2b, Digest};
use blake2::digest::generic_array::typenum::U32;
use secp256k1::{Secp256k1, SecretKey, PublicKey, Message};
use chrono::{DateTime, Utc};

/// Compute registry URI from private key
/// 
/// The URI is deterministic - same private key always produces the same URI.
/// This allows URI computation without querying the blockchain.
/// 
/// # Arguments
/// 
/// * `private_key_hex` - Private key as hex string
/// 
/// # Returns
/// 
/// Registry URI in format: `rho:id:{zbase32_encoded_hash}`
/// 
/// # Example
/// 
/// ```ignore
/// let uri = compute_registry_uri_from_private_key("5f668a7e...")?;
/// assert!(uri.starts_with("rho:id:"));
/// ```
pub fn compute_registry_uri_from_private_key(
    private_key_hex: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let secp = Secp256k1::new();
    let secret_key_bytes = hex::decode(private_key_hex)?;
    let secret_key = SecretKey::from_slice(&secret_key_bytes)?;
    let public_key = PublicKey::from_secret_key(&secp, &secret_key);
    Ok(public_key_to_uri(&public_key))
}

/// Convert public key to registry URI
/// 
/// Algorithm:
/// 1. Serialize public key (uncompressed = 65 bytes)
/// 2. Hash with Blake2b-256
/// 3. Compute CRC14 checksum
/// 4. Combine hash + CRC (34 bytes)
/// 5. Encode with zbase32
/// 6. Format as `rho:id:{encoded}`
/// 
/// # Arguments
/// 
/// * `public_key` - secp256k1 public key
/// 
/// # Returns
/// 
/// Registry URI string
pub fn public_key_to_uri(public_key: &PublicKey) -> String {
    // 1. Serialize public key (uncompressed = 65 bytes)
    let pubkey_bytes = public_key.serialize_uncompressed();
    
    // 2. Hash with Blake2b-256
    let hash = Blake2b::<U32>::new()
        .chain_update(&pubkey_bytes)
        .finalize();
    
    // 3. Compute CRC14 checksum
    let crc_bytes = compute_crc14(hash.as_ref());
    
    // 4. Combine hash + CRC (34 bytes total)
    let mut full_key = Vec::with_capacity(34);
    full_key.extend_from_slice(hash.as_ref()); // 32 bytes
    full_key.push(crc_bytes[0]);
    full_key.push(crc_bytes[1] << 2);
    
    // 5. Encode with zbase32
    let encoded = zbase32::encode_full_bytes(&full_key);
    
    // 6. Format as registry URI
    format!("rho:id:{}", encoded)
}

/// Generate signature for insertSigned
/// 
/// Creates a DER-encoded ECDSA signature for insertSigned verification.
/// 
/// **IMPLEMENTATION:** Uses proper protobuf encoding via `f1r3fly_models::rhoapi`.
/// This is the proven, working implementation from the production branch.
/// 
/// # Arguments
/// 
/// * `key` - Private key for signing
/// * `timestamp` - Deployment timestamp
/// * `deployer` - Deployer's public key
/// * `version` - Version number (typically timestamp millis)
/// 
/// # Returns
/// 
/// DER-encoded ECDSA signature bytes
/// 
/// # Implementation Details
/// 1. Create a protobuf Par message with ETuple containing (timestamp, deployer_pubkey, version)
/// 2. Encode to protobuf bytes
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

/// Compute CRC14 checksum
/// 
/// Uses CRC14 algorithm with polynomial 0x4805 (RChain standard).
/// 
/// # Arguments
/// 
/// * `data` - Input data to checksum
/// 
/// # Returns
/// 
/// 2-byte checksum (14-bit value in little-endian)
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
    fn test_uri_is_deterministic() {
        let private_key = "5f668a7ee96d944a4494cc947e4005e172d7ab3461ee5538f1f2a45a835e9657";
        let uri1 = compute_registry_uri_from_private_key(private_key).unwrap();
        let uri2 = compute_registry_uri_from_private_key(private_key).unwrap();
        assert_eq!(uri1, uri2);
    }
    
    #[test]
    fn test_uri_format() {
        let private_key = "5f668a7ee96d944a4494cc947e4005e172d7ab3461ee5538f1f2a45a835e9657";
        let uri = compute_registry_uri_from_private_key(private_key).unwrap();
        assert!(uri.starts_with("rho:id:"));
        // zbase32 encoding of 34 bytes = 55 chars
        assert_eq!(uri.len(), 7 + 55); // "rho:id:" + 55 chars
    }
    
    #[test]
    fn test_different_keys_produce_different_uris() {
        let key1 = "5f668a7ee96d944a4494cc947e4005e172d7ab3461ee5538f1f2a45a835e9657";
        let key2 = "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        let uri1 = compute_registry_uri_from_private_key(key1).unwrap();
        let uri2 = compute_registry_uri_from_private_key(key2).unwrap();
        assert_ne!(uri1, uri2);
    }
}
```

---

### 1.4: Update Exports

**File:** `rust-client/src/lib.rs`

Add:
```rust
pub mod registry;
```

Note: `rholang_helpers` already exists and is exported.

---

## Layer 2: Test Helpers

### 2.1: Copy Rho20 Template

**Action:** Copy `docs/rho20_template.rho` → `rgb-std/tests/rho20_template.rho`

**Note:** Template is confirmed working, uses `bundle+{*Rho20Token}` syntax.

---

### 2.2: Test Helper Module

**File:** `rgb-std/tests/rholang_test_helpers.rs` (NEW)

```rust
//! Test Helpers for Rho20 Contract Testing
//! 
//! Provides utilities for:
//! - Deploying rho20 contracts with caching
//! - Generating method call Rholang
//! - Managing test state

use std::sync::OnceLock;
use chrono::Utc;
use node_cli::connection_manager::F1r3flyConnectionManager;
use node_cli::registry;

/// Cached registry URI for rho20 contract
/// OnceLock ensures thread-safe, single deployment
static RHO20_URI: OnceLock<String> = OnceLock::new();

/// Rho20 contract template
const RHO20_TEMPLATE: &str = include_str!("rho20_template.rho");

/// Default timeout for finalization (seconds)
pub const DEFAULT_FINALIZATION_TIMEOUT: u32 = 60;

/// Ensure rho20 contract is deployed (idempotent, cached)
/// 
/// This function:
/// 1. Checks if contract is already deployed (cached in OnceLock)
/// 2. If not, generates deployment Rholang with insertSigned
/// 3. Deploys to f1r3node
/// 4. Waits for finalization
/// 5. Caches the registry URI
/// 
/// Subsequent calls return the cached URI immediately.
/// 
/// # Returns
/// 
/// Static reference to registry URI (e.g., "rho:id:abc123...")
/// 
/// # Panics
/// 
/// Panics if:
/// - FIREFLY_PRIVATE_KEY not set
/// - Deployment fails
/// - Finalization times out
pub async fn ensure_rho20_deployed() -> &'static String {
    // Return cached URI if already deployed
    if let Some(uri) = RHO20_URI.get() {
        log::debug!("✓ Using cached rho20 URI: {}", uri);
        return uri;
    }
    
    log::info!("🚀 Deploying rho20 contract (first time)");
    
    dotenvy::dotenv().ok();
    let manager = get_f1r3node_manager();
    let private_key = std::env::var("FIREFLY_PRIVATE_KEY")
        .expect("FIREFLY_PRIVATE_KEY must be set");
    
    // Generate deployment Rholang
    let (rholang, uri) = generate_rho20_deployment(
        &private_key,
        "TEST",
        "Test Token",
        21_000_000,
        8,
    ).expect("Failed to generate rho20 deployment");
    
    log::debug!("Generated rho20 deployment, computed URI: {}", uri);
    
    // Deploy (regular deploy, writes to blockchain)
    let deploy_id = manager
        .deploy(&rholang)
        .await
        .expect("Rho20 deploy failed");
    
    log::debug!("Rho20 deployed with ID: {}", deploy_id);
    
    // Wait for finalization
    let timeout = std::env::var("FIREFLY_FINALIZATION_TIMEOUT")
        .ok()
        .and_then(|t| t.parse().ok())
        .unwrap_or(DEFAULT_FINALIZATION_TIMEOUT);
    
    manager
        .wait_for_finalization(&deploy_id, timeout)
        .await
        .expect("Rho20 finalization timeout");
    
    log::info!("✓ Rho20 contract deployed and finalized");
    
    // Cache and return URI
    RHO20_URI.get_or_init(|| uri)
}

/// Generate rho20 deployment Rholang with insertSigned
/// 
/// Replaces template placeholders with actual values:
/// - {{TICKER}}, {{NAME}}, {{SUPPLY}}, {{DECIMALS}}
/// - {{PUBLIC_KEY}}, {{SIGNATURE}}, {{VERSION}}
/// - {{URI}}, {{DEPLOYER_PUBKEY}}
/// 
/// # Arguments
/// 
/// * `private_key_hex` - Private key for signing (hex string)
/// * `ticker` - Token ticker (e.g., "BTC")
/// * `name` - Token name (e.g., "Bitcoin")
/// * `supply` - Total supply
/// * `decimals` - Decimal precision
/// 
/// # Returns
/// 
/// Tuple of (rholang_code, registry_uri)
pub fn generate_rho20_deployment(
    private_key_hex: &str,
    ticker: &str,
    name: &str,
    supply: u64,
    decimals: u8,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    use secp256k1::{Secp256k1, SecretKey, PublicKey};
    
    // 1. Parse private key
    let secp = Secp256k1::new();
    let secret_key_bytes = hex::decode(private_key_hex)?;
    let secret_key = SecretKey::from_slice(&secret_key_bytes)?;
    let public_key = PublicKey::from_secret_key(&secp, &secret_key);
    
    // 2. Generate timestamp and version
    let timestamp = Utc::now();
    let version = timestamp.timestamp_millis();
    
    // 3. Generate signature for insertSigned
    let signature = registry::generate_insert_signed_signature(
        &secret_key,
        timestamp,
        &public_key,
        version,
    );
    
    // 4. Compute deterministic URI
    let uri = registry::public_key_to_uri(&public_key);
    
    // 5. Convert to hex for Rholang
    let public_key_hex = hex::encode(public_key.serialize_uncompressed());
    let signature_hex = hex::encode(&signature);
    let deployer_pubkey_hex = hex::encode(public_key.serialize());
    
    // 6. Replace template placeholders
    let rholang = RHO20_TEMPLATE
        .replace("{{TICKER}}", ticker)
        .replace("{{NAME}}", name)
        .replace("{{SUPPLY}}", &supply.to_string())
        .replace("{{DECIMALS}}", &decimals.to_string())
        .replace("{{DEPLOYER_PUBKEY}}", &deployer_pubkey_hex)
        .replace("{{PUBLIC_KEY}}", &public_key_hex)
        .replace("{{VERSION}}", &version.to_string())
        .replace("{{SIGNATURE}}", &signature_hex)
        .replace("{{URI}}", &uri);
    
    Ok((rholang, uri))
}

/// Generate Rholang to call storeAllocation method
/// 
/// # Arguments
/// 
/// * `uri` - Registry URI of rho20 contract
/// * `seal` - Seal identifier (e.g., "bc1q...txid:vout")
/// * `amount` - Amount to allocate
/// 
/// # Returns
/// 
/// Rholang code that calls storeAllocation
pub fn generate_store_allocation_call(uri: &str, seal: &str, amount: u64) -> String {
    format!(
        r#"new rl(`rho:registry:lookup`), contractCh, ack in {{
  rl!(`{}`, *contractCh) |
  for(@(_, Rho20Token) <- contractCh) {{
    @Rho20Token!("storeAllocation", "{}", {}, *ack)
  }}
}}"#,
        uri, seal, amount
    )
}

/// Generate Rholang to call getAllocation method
/// 
/// # Arguments
/// 
/// * `uri` - Registry URI of rho20 contract
/// * `seal` - Seal identifier to query
/// 
/// # Returns
/// 
/// Rholang code that queries allocation
pub fn generate_get_allocation_call(uri: &str, seal: &str) -> String {
    format!(
        r#"new rl(`rho:registry:lookup`), contractCh, ret in {{
  rl!(`{}`, *contractCh) |
  for(@(_, Rho20Token) <- contractCh) {{
    @Rho20Token!("getAllocation", "{}", *ret)
  }}
}}"#,
        uri, seal
    )
}

/// Generate Rholang to call transfer method
/// 
/// # Arguments
/// 
/// * `uri` - Registry URI of rho20 contract
/// * `from_seal` - Source seal
/// * `to_seal` - Destination seal
/// * `amount` - Amount to transfer
/// 
/// # Returns
/// 
/// Rholang code that executes transfer
pub fn generate_transfer_call(
    uri: &str,
    from_seal: &str,
    to_seal: &str,
    amount: u64,
) -> String {
    format!(
        r#"new rl(`rho:registry:lookup`), contractCh, ret in {{
  rl!(`{}`, *contractCh) |
  for(@(_, Rho20Token) <- contractCh) {{
    @Rho20Token!("transfer", {{
      "from_seal": "{}",
      "to_seal": "{}",
      "amount": {}
    }}, *ret)
  }}
}}"#,
        uri, from_seal, to_seal, amount
    )
}

/// Generate Rholang to call getMetadata method
/// 
/// # Arguments
/// 
/// * `uri` - Registry URI of rho20 contract
/// 
/// # Returns
/// 
/// Rholang code that queries metadata
pub fn generate_get_metadata_call(uri: &str) -> String {
    format!(
        r#"new rl(`rho:registry:lookup`), contractCh, ret in {{
  rl!(`{}`, *contractCh) |
  for(@(_, Rho20Token) <- contractCh) {{
    @Rho20Token!("getMetadata", *ret)
  }}
}}"#,
        uri
    )
}

/// Helper to get connection manager from environment
pub fn get_f1r3node_manager() -> F1r3flyConnectionManager {
    dotenvy::dotenv().ok();
    F1r3flyConnectionManager::from_env()
        .expect("Failed to create connection manager - check FIREFLY_PRIVATE_KEY")
}

/// Helper to query allocation and extract amount
/// 
/// Convenience wrapper around query_and_parse.
/// 
/// # Arguments
/// 
/// * `manager` - Connection manager
/// * `uri` - Registry URI
/// * `seal` - Seal to query
/// 
/// # Returns
/// 
/// Allocation amount or 0 if not found
pub async fn query_allocation_amount(
    manager: &F1r3flyConnectionManager,
    uri: &str,
    seal: &str,
) -> u64 {
    let query = generate_get_allocation_call(uri, seal);
    let result = manager.query_and_parse(&query).await.unwrap();
    
    if result["success"].as_bool().unwrap_or(false) {
        result["allocation"]["amount"].as_i64().unwrap_or(0) as u64
    } else {
        0
    }
}
```

---

## Layer 3: Test Cases

### 3.1: Rewrite Test File

**File:** `rgb-std/tests/rholang_execution_test.rs` (COMPLETE REWRITE)

```rust
//! Rho20 Persistent Contract Execution Tests
//! 
//! These tests verify end-to-end functionality of rho20 contracts:
//! - Deployment with insertSigned registry registration
//! - Method calls via registry URI lookup
//! - State verification via HTTP queries
//! - Proper error handling and edge cases
//! 
//! Requirements:
//! - Running f1r3node instance
//! - FIREFLY_* environment variables set
//! 
//! Run with: cargo test --test rholang_execution_test -- --nocapture

mod rholang_test_helpers;

use rholang_test_helpers::*;
use chrono::Utc;

// ============================================================================
// Test 1: Deploy and Verify Registry
// ============================================================================

#[tokio::test]
async fn test_rho20_deployment_and_registry() {
    let uri = ensure_rho20_deployed().await;
    
    // Verify URI format
    assert!(uri.starts_with("rho:id:"), "URI should start with 'rho:id:'");
    assert_eq!(uri.len(), 62, "URI should be 62 chars (rho:id: + 55 zbase32)");
    
    // Query metadata via HTTP
    let manager = get_f1r3node_manager();
    let metadata_query = generate_get_metadata_call(uri);
    let result = manager
        .query_and_parse(&metadata_query)
        .await
        .expect("Failed to query metadata");
    
    // Assert metadata structure and values
    assert_eq!(
        result["success"].as_bool().unwrap(),
        true,
        "Metadata query should succeed"
    );
    assert_eq!(
        result["metadata"]["ticker"].as_str().unwrap(),
        "TEST",
        "Ticker should be 'TEST'"
    );
    assert_eq!(
        result["metadata"]["name"].as_str().unwrap(),
        "Test Token",
        "Name should be 'Test Token'"
    );
    assert_eq!(
        result["metadata"]["supply"].as_i64().unwrap(),
        21_000_000,
        "Supply should be 21,000,000"
    );
    assert_eq!(
        result["metadata"]["decimals"].as_i64().unwrap(),
        8,
        "Decimals should be 8"
    );
}

// ============================================================================
// Test 2: Store and Query Allocation
// ============================================================================

#[tokio::test]
async fn test_store_and_query_allocation() {
    let manager = get_f1r3node_manager();
    let uri = ensure_rho20_deployed().await;
    
    // Create unique seal for this test
    let seal = format!(
        "test2_seal_{}",
        Utc::now().timestamp_nanos_opt().unwrap()
    );
    let amount = 1_000_000_u64;
    
    // Store allocation
    let store_rho = generate_store_allocation_call(uri, &seal, amount);
    let deploy_id = manager
        .deploy(&store_rho)
        .await
        .expect("Store allocation deploy failed");
    
    let timeout = std::env::var("FIREFLY_FINALIZATION_TIMEOUT")
        .ok()
        .and_then(|t| t.parse().ok())
        .unwrap_or(DEFAULT_FINALIZATION_TIMEOUT);
    
    manager
        .wait_for_finalization(&deploy_id, timeout)
        .await
        .expect("Store allocation finalization timeout");
    
    // Query allocation via HTTP
    let query_rho = generate_get_allocation_call(uri, &seal);
    let result = manager
        .query_and_parse(&query_rho)
        .await
        .expect("Query allocation failed");
    
    // Assert allocation data
    assert_eq!(
        result["success"].as_bool().unwrap(),
        true,
        "Query should succeed"
    );
    assert_eq!(
        result["allocation"]["amount"].as_i64().unwrap(),
        amount as i64,
        "Stored amount should match queried amount"
    );
    assert_eq!(
        result["allocation"]["seal"].as_str().unwrap(),
        seal,
        "Seal should match"
    );
}

// ============================================================================
// Test 3: Transfer Updates Balances
// ============================================================================

#[tokio::test]
async fn test_transfer_updates_balances() {
    let manager = get_f1r3node_manager();
    let uri = ensure_rho20_deployed().await;
    
    let timestamp = Utc::now().timestamp_nanos_opt().unwrap();
    let from_seal = format!("test3_alice_{}", timestamp);
    let to_seal = format!("test3_bob_{}", timestamp);
    
    let timeout = std::env::var("FIREFLY_FINALIZATION_TIMEOUT")
        .ok()
        .and_then(|t| t.parse().ok())
        .unwrap_or(DEFAULT_FINALIZATION_TIMEOUT);
    
    // Step 1: Store initial allocation for Alice
    let store_rho = generate_store_allocation_call(uri, &from_seal, 1_000_000);
    let deploy_id = manager.deploy(&store_rho).await.unwrap();
    manager.wait_for_finalization(&deploy_id, timeout).await.unwrap();
    
    // Step 2: Transfer from Alice to Bob
    let transfer_rho = generate_transfer_call(uri, &from_seal, &to_seal, 100);
    let deploy_id = manager.deploy(&transfer_rho).await.unwrap();
    manager.wait_for_finalization(&deploy_id, timeout).await.unwrap();
    
    // Step 3: Verify Alice's balance (should be 999,900)
    let alice_amount = query_allocation_amount(&manager, uri, &from_seal).await;
    assert_eq!(
        alice_amount,
        999_900,
        "Alice should have 999,900 after transferring 100"
    );
    
    // Step 4: Verify Bob's balance (should be 100)
    let bob_amount = query_allocation_amount(&manager, uri, &to_seal).await;
    assert_eq!(
        bob_amount,
        100,
        "Bob should have 100 after receiving transfer"
    );
}

// ============================================================================
// Test 4: Insufficient Balance Fails Gracefully
// ============================================================================

#[tokio::test]
async fn test_insufficient_balance_transfer_fails() {
    let manager = get_f1r3node_manager();
    let uri = ensure_rho20_deployed().await;
    
    let timestamp = Utc::now().timestamp_nanos_opt().unwrap();
    let from_seal = format!("test4_poor_alice_{}", timestamp);
    let to_seal = format!("test4_rich_bob_{}", timestamp);
    
    let timeout = std::env::var("FIREFLY_FINALIZATION_TIMEOUT")
        .ok()
        .and_then(|t| t.parse().ok())
        .unwrap_or(DEFAULT_FINALIZATION_TIMEOUT);
    
    // Give Alice only 50 tokens
    let store_rho = generate_store_allocation_call(uri, &from_seal, 50);
    let deploy_id = manager.deploy(&store_rho).await.unwrap();
    manager.wait_for_finalization(&deploy_id, timeout).await.unwrap();
    
    // Try to transfer 100 tokens (more than balance)
    let transfer_rho = generate_transfer_call(uri, &from_seal, &to_seal, 100);
    let deploy_id = manager.deploy(&transfer_rho).await.unwrap();
    manager.wait_for_finalization(&deploy_id, timeout).await.unwrap();
    
    // Verify Alice still has 50 (transfer failed)
    let alice_amount = query_allocation_amount(&manager, uri, &from_seal).await;
    assert_eq!(
        alice_amount,
        50,
        "Alice should still have 50 (transfer should fail)"
    );
    
    // Verify Bob has no allocation (transfer failed)
    let bob_amount = query_allocation_amount(&manager, uri, &to_seal).await;
    assert_eq!(
        bob_amount,
        0,
        "Bob should have 0 (transfer should fail)"
    );
}

// ============================================================================
// Test 5: Query Non-Existent Allocation
// ============================================================================

#[tokio::test]
async fn test_query_nonexistent_allocation() {
    let manager = get_f1r3node_manager();
    let uri = ensure_rho20_deployed().await;
    
    // Query seal that was never stored
    let seal = format!("test5_nonexistent_{}", Utc::now().timestamp_nanos_opt().unwrap());
    let query_rho = generate_get_allocation_call(uri, &seal);
    let result = manager.query_and_parse(&query_rho).await.unwrap();
    
    // Should return success: false
    assert_eq!(
        result["success"].as_bool().unwrap(),
        false,
        "Query for non-existent seal should fail"
    );
    assert!(
        result["error"].as_str().unwrap().contains("No allocation found"),
        "Error message should indicate allocation not found"
    );
}

// ============================================================================
// Test 6: Multiple Sequential Transfers
// ============================================================================

#[tokio::test]
async fn test_multiple_sequential_transfers() {
    let manager = get_f1r3node_manager();
    let uri = ensure_rho20_deployed().await;
    
    let timestamp = Utc::now().timestamp_nanos_opt().unwrap();
    let alice = format!("test6_alice_{}", timestamp);
    let bob = format!("test6_bob_{}", timestamp);
    let charlie = format!("test6_charlie_{}", timestamp);
    
    let timeout = std::env::var("FIREFLY_FINALIZATION_TIMEOUT")
        .ok()
        .and_then(|t| t.parse().ok())
        .unwrap_or(DEFAULT_FINALIZATION_TIMEOUT);
    
    // Setup: Alice has 10,000 tokens
    let store_rho = generate_store_allocation_call(uri, &alice, 10_000);
    let deploy_id = manager.deploy(&store_rho).await.unwrap();
    manager.wait_for_finalization(&deploy_id, timeout).await.unwrap();
    
    // Transfer 1: Alice -> Bob (1,000)
    let transfer1 = generate_transfer_call(uri, &alice, &bob, 1_000);
    let deploy_id = manager.deploy(&transfer1).await.unwrap();
    manager.wait_for_finalization(&deploy_id, timeout).await.unwrap();
    
    // Transfer 2: Alice -> Charlie (2,000)
    let transfer2 = generate_transfer_call(uri, &alice, &charlie, 2_000);
    let deploy_id = manager.deploy(&transfer2).await.unwrap();
    manager.wait_for_finalization(&deploy_id, timeout).await.unwrap();
    
    // Transfer 3: Bob -> Charlie (500)
    let transfer3 = generate_transfer_call(uri, &bob, &charlie, 500);
    let deploy_id = manager.deploy(&transfer3).await.unwrap();
    manager.wait_for_finalization(&deploy_id, timeout).await.unwrap();
    
    // Verify final balances
    let alice_amount = query_allocation_amount(&manager, uri, &alice).await;
    assert_eq!(alice_amount, 7_000, "Alice: 10000 - 1000 - 2000 = 7000");
    
    let bob_amount = query_allocation_amount(&manager, uri, &bob).await;
    assert_eq!(bob_amount, 500, "Bob: 1000 - 500 = 500");
    
    let charlie_amount = query_allocation_amount(&manager, uri, &charlie).await;
    assert_eq!(charlie_amount, 2_500, "Charlie: 2000 + 500 = 2500");
}

// ============================================================================
// Test 7: Transfer to Self (Edge Case)
// ============================================================================

#[tokio::test]
async fn test_transfer_to_self() {
    let manager = get_f1r3node_manager();
    let uri = ensure_rho20_deployed().await;
    
    let timestamp = Utc::now().timestamp_nanos_opt().unwrap();
    let seal = format!("test7_self_{}", timestamp);
    
    let timeout = std::env::var("FIREFLY_FINALIZATION_TIMEOUT")
        .ok()
        .and_then(|t| t.parse().ok())
        .unwrap_or(DEFAULT_FINALIZATION_TIMEOUT);
    
    // Store allocation
    let store_rho = generate_store_allocation_call(uri, &seal, 1_000);
    let deploy_id = manager.deploy(&store_rho).await.unwrap();
    manager.wait_for_finalization(&deploy_id, timeout).await.unwrap();
    
    // Transfer to self
    let transfer_rho = generate_transfer_call(uri, &seal, &seal, 100);
    let deploy_id = manager.deploy(&transfer_rho).await.unwrap();
    manager.wait_for_finalization(&deploy_id, timeout).await.unwrap();
    
    // Balance should remain unchanged
    let amount = query_allocation_amount(&manager, uri, &seal).await;
    assert_eq!(
        amount,
        1_000,
        "Transfer to self should not change balance"
    );
}
```

---

## Configuration & Timeouts

### Environment Variables

Add to `.env` files:

```bash
# Existing
FIREFLY_HOST=localhost
FIREFLY_GRPC_PORT=40401
FIREFLY_HTTP_PORT=40403
FIREFLY_PRIVATE_KEY=5f668a7ee96d944a4494cc947e4005e172d7ab3461ee5538f1f2a45a835e9657

# NEW: Configurable timeouts
FIREFLY_FINALIZATION_TIMEOUT=60  # seconds, default 60
```

### Timeout Configuration

Tests will use:
```rust
let timeout = std::env::var("FIREFLY_FINALIZATION_TIMEOUT")
    .ok()
    .and_then(|t| t.parse().ok())
    .unwrap_or(DEFAULT_FINALIZATION_TIMEOUT); // 60 seconds
```

For CI/CD with slower nodes:
```bash
FIREFLY_FINALIZATION_TIMEOUT=120 cargo test
```

### Error Handling

All timeout errors will:
1. Return proper `ConnectionError::Timeout` variant
2. Include context (deploy ID, operation type)
3. Allow test to fail with clear error message

---

## Dependencies

### rust-client/Cargo.toml

Add:
```toml
[dependencies]
# Existing...

# NEW: HTTP queries
reqwest = { version = "0.12", features = ["json"] }

# NEW: Registry and signatures
secp256k1 = { version = "0.29", features = ["rand-std"] }
blake2 = "0.10"
crc = "3.0"
zbase32 = "0.1"
hex = "0.4"
chrono = "0.4"
```

### rgb-std/Cargo.toml

No changes needed (uses `node_cli` as dev-dependency).

---

## Implementation Checklist

### Phase 1: rust-client Extensions

- [x] Add `reqwest` and crypto dependencies to `Cargo.toml`
- [x] Reuse existing `src/rholang_helpers.rs` for JSON conversion
- [x] Create `src/registry.rs` with URI and signature functions
- [x] Add `query_via_http()` to `connection_manager.rs`
- [x] Add `query_and_parse()` to `connection_manager.rs`
- [x] Add `QueryFailed` and `Timeout` to `ConnectionError` enum
- [x] Export new modules in `lib.rs`
- [x] Run `cargo test --lib` in `rust-client/` to verify

### Phase 2: Test Helpers

- [ ] Copy `docs/rho20_template.rho` to `rgb-std/tests/rho20_template.rho`
- [ ] Create `rgb-std/tests/rholang_test_helpers.rs`
- [ ] Implement `ensure_rho20_deployed()` with `OnceLock`
- [ ] Implement `generate_rho20_deployment()`
- [ ] Implement helper functions for method calls
- [ ] Implement `query_allocation_amount()` convenience helper

### Phase 3: Test Cases

- [ ] Rewrite `rgb-std/tests/rholang_execution_test.rs`
- [ ] Remove all `println!` statements
- [ ] Add Test 1: Deployment and registry verification
- [ ] Add Test 2: Store and query allocation
- [ ] Add Test 3: Transfer updates balances
- [ ] Add Test 4: Insufficient balance fails gracefully
- [ ] Add Test 5: Query non-existent allocation
- [ ] Add Test 6: Multiple sequential transfers
- [ ] Add Test 7: Transfer to self (edge case)

### Phase 4: Verification

- [ ] Run `cargo build` in workspace root - verify compilation
- [ ] Start f1r3node: `cd f1r3node && sbt run`
- [ ] Run `cargo test --test rholang_execution_test -- --nocapture`
- [ ] Verify all tests pass
- [ ] Test with clean node (CI simulation)
- [ ] Test with existing state (dev simulation)
- [ ] Verify timeout configuration works

---

## Open Questions & Decisions

### Question 1: Protobuf for insertSigned Signature

**Status:** ✅ **RESOLVED**

**Solution:** Using proven production implementation with proper protobuf encoding via `f1r3fly_models::rhoapi`. This creates a protobuf Par message with ETuple structure before hashing and signing.

---

### Question 2: Test Isolation Strategy

**Current:** Each test uses timestamp-based unique seals to avoid conflicts.

**Pros:**
- Works on fresh and existing nodes
- No cleanup required
- Tests can run in parallel

**Cons:**
- State accumulates over time
- May slow down node with many seals

**Decision:** Use timestamp-based isolation. If accumulation becomes an issue, add periodic cleanup script.

---

### Question 3: HTTP Port Exposure

**Status:** ✅ **RESOLVED**

`ConnectionConfig` already has `http_port` field (line 14).  
`F1r3flyConnectionManager` stores `config: ConnectionConfig` (line 114).  
Can access via `self.config.http_port`.

---

### Question 4: Timeout Configuration

**Status:** ✅ **RESOLVED**

Use environment variable `FIREFLY_FINALIZATION_TIMEOUT` (default 60 seconds).  
Tests read from env or fall back to `DEFAULT_FINALIZATION_TIMEOUT` constant.

---

## Success Criteria

✅ All tests pass on fresh f1r3node  
✅ All tests pass on node with existing state  
✅ No `println!` statements in test output  
✅ All assertions use proper error messages  
✅ HTTP queries correctly parse complex JSON  
✅ Registry URIs are deterministic and cached  
✅ Timeouts are configurable via environment  
✅ Zero compilation warnings

---

**Ready for Implementation: Awaiting User Review**

