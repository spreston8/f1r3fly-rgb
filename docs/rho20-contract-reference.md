# Rho20 Token Contract - Complete Reference

This document contains everything needed to deploy and query Rholang token contracts on F1r3fly.

## Table of Contents
1. [Complete Rholang Contract](#complete-rholang-contract)
2. [Rust Helper Functions](#rust-helper-functions)
3. [Query Patterns](#query-patterns)
4. [Test Implementation](#test-implementation)

---

## Complete Rholang Contract

**File**: `rho20_contract.rho`

This contract demonstrates:
- ✅ Correct `insertSigned` registration for persistent deployment
- ✅ State management with `treeHashMap` (all state in map, no separate channels)
- ✅ Issue, transfer, and balance query operations
- ✅ Proper URI binding so lookups work

```rholang
// Complete Rho20 Token Contract with State - CORRECTED VERSION
// Based on embers insert_signed.rho pattern with proper insertSigned format
//
// This is a fully working token contract that demonstrates:
// - Correct insertSigned registration for persistent contract deployment
// - State management with treeHashMap (all state in map, no separate channels)
// - Issue, transfer, and balance query operations
// - Proper URI binding so lookups work
//
// Template variables (replaced by executor.rs):
// - {{TICKER}}, {{NAME}}, {{TOTAL_SUPPLY}}, {{PRECISION}}
// - {{PUBLIC_KEY}}, {{SIGNATURE}}, {{VERSION}}, {{URI}}

new rl(`rho:registry:lookup`),
    rs(`rho:registry:insertSigned:secp256k1`),
    stdout(`rho:io:stdout`),
    abort(`rho:execution:abort`),
    devNull(`rho:io:devNull`),
    treeHashMapCh,
    balanceMapCh,
    prevEnvCh,
    initEnv,
    Rho20Token,
    uriOut
in {
  // Check if contract already exists (Embers upgrade pattern)
  rl!(`{{URI}}`, *prevEnvCh) |
  
  for(@Nil <- prevEnvCh) {
    initEnv!()
  } |
  
  for(@(version, _) <- prevEnvCh) {
    if (version < {{VERSION}}) {
      initEnv!()
    }
  } |
  
  for(<- initEnv) {
    // Initialize treeHashMap for ALL state (balances + unallocated)
    rl!(`rho:lang:treeHashMap`, *treeHashMapCh) |
    
    for(treeHashMap <- treeHashMapCh) {
      treeHashMap!("init", 3, *treeHashMapCh) |
      
      for(@balanceMap <- treeHashMapCh) {
        // CRITICAL: Initialize unallocated supply IN the map
        // This prevents race conditions and ensures persistence
        treeHashMap!("set", balanceMap, "unallocated", {{TOTAL_SUPPLY}}, *devNull) |
        
        // Store treeHashMap and map for persistent access (uses <<- peek)
        balanceMapCh!(*treeHashMap, balanceMap) |
        
        // =====================================================================
        // Method: getMetadata - Returns token metadata
        // =====================================================================
        contract Rho20Token(@"getMetadata", ret) = {
          ret!({
            "ticker": "{{TICKER}}",
            "name": "{{NAME}}",
            "supply": {{TOTAL_SUPPLY}},
            "decimals": {{PRECISION}}
          })
        } |
        
        // =====================================================================
        // Method: balanceOf - Query balance for an address
        // =====================================================================
        contract Rho20Token(@"balanceOf", @address, ret) = {
          new foundCh, notFoundCh in {
            for(treeHashMap, @currentMap <<- balanceMapCh) {
              treeHashMap!("getOrElse", currentMap, address, *foundCh, *notFoundCh)
            } |
            
            for(@balance <- foundCh) {
              ret!(balance)
            } |
            
            for(<- notFoundCh) {
              ret!(0)
            }
          }
        } |
        
        // =====================================================================
        // Method: issue - Allocate tokens from unallocated supply
        // =====================================================================
        // FIXED: Store unallocated in treeHashMap, not separate channel
        contract Rho20Token(@"issue", @recipient, @amount, ret) = {
          if (amount <= 0) {
            ret!({"success": false, "error": "Amount must be positive"})
          } else {
            new unallocFoundCh, unallocNotFoundCh in {
              for(treeHashMap, @currentMap <<- balanceMapCh) {
                // Get current unallocated from map (stored with key "unallocated")
                treeHashMap!("getOrElse", currentMap, "unallocated", *unallocFoundCh, *unallocNotFoundCh) |
                
                for(@currentUnallocated <- unallocFoundCh) {
                  if (amount <= currentUnallocated) {
                    // Update unallocated in map
                    treeHashMap!("set", currentMap, "unallocated", currentUnallocated - amount, *devNull) |
                    
                    // Update recipient balance
                    new balFoundCh, balNotFoundCh in {
                      treeHashMap!("getOrElse", currentMap, recipient, *balFoundCh, *balNotFoundCh) |
                      
                      for(@existingBalance <- balFoundCh) {
                        // Add to existing balance
                        treeHashMap!("set", currentMap, recipient, existingBalance + amount, *devNull) |
                        ret!({"success": true, "balance": existingBalance + amount})
                      } |
                      
                      for(<- balNotFoundCh) {
                        // Create new balance entry
                        treeHashMap!("set", currentMap, recipient, amount, *devNull) |
                        ret!({"success": true, "balance": amount})
                      }
                    }
                  } else {
                    ret!({"success": false, "error": "Insufficient unallocated supply", "available": currentUnallocated, "requested": amount})
                  }
                } |
                
                for(<- unallocNotFoundCh) {
                  ret!({"success": false, "error": "Contract not initialized - unallocated supply not found"})
                }
              }
            }
          }
        } |
        
        // =====================================================================
        // Method: transfer - Transfer tokens between addresses
        // =====================================================================
        contract Rho20Token(@"transfer", @from, @to, @amount, ret) = {
          if (amount <= 0) {
            ret!({"success": false, "error": "Amount must be positive"})
          } else {
            new fromFoundCh, fromNotFoundCh in {
              for(treeHashMap, @currentMap <<- balanceMapCh) {
                treeHashMap!("getOrElse", currentMap, from, *fromFoundCh, *fromNotFoundCh) |
                
                for(@fromBalance <- fromFoundCh) {
                  if (fromBalance >= amount) {
                    // Deduct from sender
                    treeHashMap!("set", currentMap, from, fromBalance - amount, *devNull) |
                    
                    // Update recipient
                    new toFoundCh, toNotFoundCh in {
                      treeHashMap!("getOrElse", currentMap, to, *toFoundCh, *toNotFoundCh) |
                      
                      for(@toBalance <- toFoundCh) {
                        // Add to existing recipient balance
                        treeHashMap!("set", currentMap, to, toBalance + amount, *devNull) |
                        ret!({"success": true, "from_balance": fromBalance - amount, "to_balance": toBalance + amount})
                      } |
                      
                      for(<- toNotFoundCh) {
                        // Create new recipient balance
                        treeHashMap!("set", currentMap, to, amount, *devNull) |
                        ret!({"success": true, "from_balance": fromBalance - amount, "to_balance": amount})
                      }
                    }
                  } else {
                    ret!({"success": false, "error": "Insufficient balance", "balance": fromBalance, "requested": amount})
                  }
                } |
                
                for(<- fromNotFoundCh) {
                  ret!({"success": false, "error": "Sender has no balance"})
                }
              }
            }
          }
        } |
        
        // =====================================================================
        // Register with insertSigned - CORRECTED FORMAT
        // =====================================================================
        // 1. Call rs (insertSigned) directly - it's bound at the top
        // 2. Signature goes IN the tuple as second element
        // 3. Contract reference goes IN the tuple as third element
        
        rs!(
          "{{PUBLIC_KEY}}".hexToBytes(),
          ({{VERSION}}, bundle+{*Rho20Token}),
          "{{SIGNATURE}}".hexToBytes(),
          *uriOut
        ) |
        
        for(@Nil <- uriOut) {
          abort!("insertSigned failed - signature verification error")
        } |
        
        for(@uri <- uriOut) {
          // insertSigned with bundle+{*Rho20Token} already binds the contract to the URI
          // The registry will return (version, bundle+{*Rho20Token})
          stdout!(("Rho20 Token Registered", "URI:", uri, "Ticker:", "{{TICKER}}", "Supply:", {{TOTAL_SUPPLY}}))
        }
      }
    }
  }
}
```

---

## Rust Helper Functions

### 1. Generate insertSigned Signature

```rust
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
```

### 2. Convert Public Key to Registry URI

```rust
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

/// Compute CRC14 checksum for URI generation
fn compute_crc14(data: &[u8]) -> [u8; 2] {
    use crc::{Algorithm, Crc};

    const CRC14: Algorithm<u16> = Algorithm {
        width: 14,
        poly: 0x4805,
        init: 0x0000,
        refin: false,
        refout: false,
        xorout: 0x0000,
        check: 0x0000,
        residue: 0x0000,
    };

    let crc = Crc::<u16>::new(&CRC14);
    let mut digest = crc.digest();
    digest.update(data);
    let crc_value = digest.finalize();

    crc_value.to_le_bytes()
}
```

### 3. Generate Contract with Replaced Placeholders

```rust
use chrono::Utc;
use secp256k1::{PublicKey, Secp256k1, SecretKey};

/// Generate rho20_contract.rho with replaced placeholders
fn generate_rho20_contract(
    ticker: &str,
    name: &str,
    total_supply: u64,
    precision: u8,
) -> anyhow::Result<(String, String, i64)> {
    // Load the template
    let template = include_str!("../../rho20_contract.rho");

    // Get private key from env
    let private_key_hex = std::env::var("FIREFLY_PRIVATE_KEY")
        .expect("FIREFLY_PRIVATE_KEY environment variable required");

    let secp = Secp256k1::new();
    let secret_key_bytes = hex::decode(&private_key_hex)?;
    let secret_key = SecretKey::from_slice(&secret_key_bytes)?;
    let public_key = PublicKey::from_secret_key(&secp, &secret_key);

    // Generate signature for insertSigned
    let timestamp = Utc::now();
    let timestamp_millis = timestamp.timestamp_millis();
    let version = timestamp_millis;

    let signature = generate_insert_signed_signature(
        &secret_key,
        timestamp,
        &public_key,
        version,
    );

    // Compute the deterministic URI
    let contract_uri = public_key_to_uri(&public_key);

    let public_key_hex = hex::encode(public_key.serialize_uncompressed());
    let signature_hex = hex::encode(&signature);

    // Replace all placeholders
    let rholang_code = template
        .replace("{{URI}}", &contract_uri)
        .replace("{{PUBLIC_KEY}}", &public_key_hex)
        .replace("{{VERSION}}", &version.to_string())
        .replace("{{SIGNATURE}}", &signature_hex)
        .replace("{{TICKER}}", ticker)
        .replace("{{NAME}}", name)
        .replace("{{TOTAL_SUPPLY}}", &total_supply.to_string())
        .replace("{{PRECISION}}", &precision.to_string());

    Ok((rholang_code, contract_uri, timestamp_millis))
}
```

---

## Query Patterns

### ⚠️ CRITICAL: The `return!()` Pattern

**ALL queries involving async operations (like registry lookups) MUST use the `return!()` channel pattern.**

This is the same pattern used by RChain's system contracts (RevVault, PoS, etc.) and is **required** for exploratory deploys to wait for async operations to complete.

#### ❌ Wrong (returns empty `expr` array):

```rholang
new rl(`rho:registry:lookup`), contractCh, resultCh in {
  rl!(`rho:id:...`, *contractCh) |
  for(@(_, Bundle) <- contractCh) {
    @{Bundle}!("method", params, *resultCh) |
    for(@result <- resultCh) {
      result  // ❌ NO! This doesn't block exploratory deploys
    }
  }
}
```

#### ✅ Correct (returns data in `expr` array):

```rholang
new return, rl(`rho:registry:lookup`), contractCh, resultCh in {
  rl!(`rho:id:...`, *contractCh) |
  for(@(_, Bundle) <- contractCh) {
    @{Bundle}!("method", params, *resultCh) |
    for(@result <- resultCh) {
      return!(result)  // ✅ YES! This blocks until result is ready
    }
  }
}
```

### Query: getMetadata

```rholang
new return, rl(`rho:registry:lookup`), contractCh, resultCh in {
  rl!(`rho:id:nbhkwm5hjryr9m7cqbcsw5a5bfy8us475juqd9ku6mnw66mprmxd96`, *contractCh) |
  for(@(_, Rho20TokenBundle) <- contractCh) {
    @{Rho20TokenBundle}!("getMetadata", *resultCh) |
    for(@result <- resultCh) {
      return!(result)
    }
  }
}
```

**Returns:**
```json
{
  "ticker": "TEST",
  "name": "Test Token",
  "supply": 1000000,
  "decimals": 8
}
```

### Query: balanceOf

```rholang
new return, rl(`rho:registry:lookup`), contractCh, resultCh in {
  rl!(`rho:id:nbhkwm5hjryr9m7cqbcsw5a5bfy8us475juqd9ku6mnw66mprmxd96`, *contractCh) |
  for(@(_, Rho20TokenBundle) <- contractCh) {
    @{Rho20TokenBundle}!("balanceOf", "alice", *resultCh) |
    for(@result <- resultCh) {
      return!(result)
    }
  }
}
```

**Returns:** `1000` (integer)

### Deploy: issue (requires regular deploy, not exploratory)

```rholang
new rl(`rho:registry:lookup`), contractCh, resultCh, stdout(`rho:io:stdout`) in {
  rl!(`rho:id:nbhkwm5hjryr9m7cqbcsw5a5bfy8us475juqd9ku6mnw66mprmxd96`, *contractCh) |
  for(@(_, Rho20TokenBundle) <- contractCh) {
    @{Rho20TokenBundle}!("issue", "alice", 1000, *resultCh) |
    for(@result <- resultCh) {
      stdout!(("ISSUE_RESULT", result))
    }
  }
}
```

### Deploy: transfer (requires regular deploy, not exploratory)

```rholang
new rl(`rho:registry:lookup`), contractCh, resultCh, stdout(`rho:io:stdout`) in {
  rl!(`rho:id:nbhkwm5hjryr9m7cqbcsw5a5bfy8us475juqd9ku6mnw66mprmxd96`, *contractCh) |
  for(@(_, Rho20TokenBundle) <- contractCh) {
    @{Rho20TokenBundle}!("transfer", "alice", "bob", 300, *resultCh) |
    for(@result <- resultCh) {
      stdout!(("TRANSFER_RESULT", result))
    }
  }
}
```

---

## Test Implementation

### Rust Helper for Calling Contract Methods

```rust
use node_cli::connection_manager::F1r3flyConnectionManager;
use serde_json::Value;

/// Helper to call contract methods via exploratory deploy
async fn call_method(
    connection: &F1r3flyConnectionManager,
    contract_uri: &str,
    method: &str,
    params: Vec<&str>,
) -> anyhow::Result<serde_json::Value> {
    let params_str = if params.is_empty() {
        String::new()
    } else {
        format!(", {}", params.join(", "))
    };

    // Use the same pattern as RevVault balance queries: new return channel
    // This makes exploratory deploys wait for async operations to complete
    // Note: insertSigned with bundle+{*Contract} returns (version, bundle)
    // We need to unbundle it with @{bundle}!() to call methods
    let query_code = format!(
        r#"new return, rl(`rho:registry:lookup`), contractCh, resultCh in {{
  rl!(`{}`, *contractCh) |
  for(@(_, Rho20TokenBundle) <- contractCh) {{
    @{{Rho20TokenBundle}}!("{}"{}, *resultCh) |
    for(@result <- resultCh) {{
      return!(result)
    }}
  }}
}}"#,
        contract_uri, method, params_str
    );

    log::debug!("Query code:\n{}", query_code);

    // Use HTTP API
    let http_port = std::env::var("FIREFLY_HTTP_PORT")
        .unwrap_or_else(|_| "40403".to_string());
    let host = std::env::var("FIREFLY_HOST")
        .unwrap_or_else(|_| "localhost".to_string());

    let http_client = reqwest::Client::new();
    let response = http_client
        .post(format!("http://{}:{}/api/explore-deploy", host, http_port))
        .body(query_code)
        .header("Content-Type", "text/plain")
        .send()
        .await?;

    if !response.status().is_success() {
        anyhow::bail!("HTTP error: {}", response.status());
    }

    let json_response: serde_json::Value = response.json().await?;
    log::debug!("Raw JSON response: {}", serde_json::to_string_pretty(&json_response)?);

    // Extract expr array
    let expr_array = json_response
        .get("expr")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow::anyhow!("Invalid response format: missing 'expr' array"))?;

    if expr_array.is_empty() {
        anyhow::bail!("No data returned from contract");
    }

    // Check for Nil
    let rholang_expr = &expr_array[0];
    if rholang_expr.get("ExprNil").is_some() {
        anyhow::bail!("Contract method returned Nil - method may not exist or contract not initialized");
    }

    // Convert to plain JSON
    let plain_json = node_cli::rholang_helpers::convert_rholang_to_json(rholang_expr)
        .map_err(|e| anyhow::anyhow!("Failed to parse Rholang result: {}", e))?;

    log::debug!("Converted to plain JSON: {}", serde_json::to_string_pretty(&plain_json)?);

    Ok(plain_json)
}
```

### Example Test Flow

```rust
#[tokio::test]
#[ignore]
async fn test_rho20_contract_full_flow() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    env_logger::builder().is_test(true).filter_level(log::LevelFilter::Debug).try_init();

    // Phase 0: Generate contract
    let (rholang_code, contract_uri, timestamp_millis) = generate_rho20_contract(
        "TEST", "Test Token", 1_000_000, 8,
    )?;
    
    let config = ConnectionConfig::from_env()?;
    let connection = F1r3flyConnectionManager::new(config);

    // Phase 1: Deploy
    let deploy_id = connection.deploy_with_timestamp(&rholang_code, timestamp_millis).await?;
    let block_hash = connection.wait_for_deploy(&deploy_id, 60).await?;
    connection.wait_for_finalization(&block_hash, 24).await?;

    // Phase 2: Query metadata
    let metadata = call_method(&connection, &contract_uri, "getMetadata", vec![]).await?;
    assert_eq!(metadata["ticker"], "TEST");
    assert_eq!(metadata["supply"], 1_000_000);

    // Phase 3: Query initial balance
    let balance = call_method(&connection, &contract_uri, "balanceOf", vec![r#""alice""#]).await?;
    assert_eq!(balance.as_u64().unwrap(), 0);

    // Phase 4: Issue tokens (regular deploy)
    let issue_rholang = format!(
        r#"new rl(`rho:registry:lookup`), contractCh, resultCh, stdout(`rho:io:stdout`) in {{
  rl!(`{}`, *contractCh) |
  for(@(_, Rho20Token) <- contractCh) {{
    @Rho20Token!("issue", "alice", 1000, *resultCh) |
    for(@result <- resultCh) {{
      stdout!(("ISSUE_RESULT", result))
    }}
  }}
}}"#,
        contract_uri
    );
    let issue_deploy_id = connection.deploy(&issue_rholang).await?;
    connection.wait_for_deploy(&issue_deploy_id, 60).await?;

    // Phase 5: Verify balance after issue
    let balance = call_method(&connection, &contract_uri, "balanceOf", vec![r#""alice""#]).await?;
    assert_eq!(balance.as_u64().unwrap(), 1000);

    // Phase 6: Transfer tokens (regular deploy)
    let transfer_rholang = format!(
        r#"new rl(`rho:registry:lookup`), contractCh, resultCh, stdout(`rho:io:stdout`) in {{
  rl!(`{}`, *contractCh) |
  for(@(_, Rho20Token) <- contractCh) {{
    @Rho20Token!("transfer", "alice", "bob", 300, *resultCh) |
    for(@result <- resultCh) {{
      stdout!(("TRANSFER_RESULT", result))
    }}
  }}
}}"#,
        contract_uri
    );
    let transfer_deploy_id = connection.deploy(&transfer_rholang).await?;
    connection.wait_for_deploy(&transfer_deploy_id, 60).await?;

    // Phase 7: Verify balances after transfer
    let alice_balance = call_method(&connection, &contract_uri, "balanceOf", vec![r#""alice""#]).await?;
    let bob_balance = call_method(&connection, &contract_uri, "balanceOf", vec![r#""bob""#]).await?;
    assert_eq!(alice_balance.as_u64().unwrap(), 700);
    assert_eq!(bob_balance.as_u64().unwrap(), 300);

    Ok(())
}
```

---

## Key Learnings

### 1. insertSigned Format

The correct format for `insertSigned` is:

```rholang
rs!(
  public_key_bytes,
  (version, bundle+{*Contract}),
  signature_bytes,
  *uriOut
)
```

**NOT**:
- ❌ `@rs!()` (the `@` is wrong)
- ❌ Signature outside the tuple
- ❌ Contract outside the tuple

### 2. Registry Lookup Returns Bundle

When you look up a contract registered with `bundle+{*Contract}`, you get back:

```rholang
(version, bundle+{*Contract})
```

To call methods, you must **unbundle** it:

```rholang
for(@(_, ContractBundle) <- contractCh) {
  @{ContractBundle}!("method", params, *resultCh)
}
```

### 3. Exploratory Deploys Need `return!()`

**ALL** queries with async operations (registry lookups, contract calls, etc.) **MUST** use the `return!()` pattern:

```rholang
new return, ... in {
  // ... async operations ...
  for(@result <- resultCh) {
    return!(result)  // ← CRITICAL!
  }
}
```

Without `return!()`, the HTTP response will have an empty `expr` array.

### 4. State Management

Store ALL state in a single `treeHashMap`, including:
- User balances
- Unallocated supply
- Any other contract state

Use `<<-` (peek) to access the map without consuming it:

```rholang
for(treeHashMap, @currentMap <<- balanceMapCh) {
  treeHashMap!("get", currentMap, key, *resultCh)
}
```

---

## Dependencies

### Cargo.toml

```toml
[dependencies]
node_cli = { path = "../rust-client" }
secp256k1 = { version = "0.29", features = ["rand"] }
blake2 = "0.10"
chrono = "0.4"
hex = "0.4"
prost = "0.13"
crc = "3.0"
zbase32 = "0.1"
f1r3fly_models = { path = "../f1r3node/models" }
reqwest = { version = "0.12", features = ["json"] }
tokio = { version = "1", features = ["full"] }
serde_json = "1.0"
anyhow = "1.0"
log = "0.4"
env_logger = "0.11"
dotenv = "0.15"
```

---

## Environment Variables

```bash
# Required
FIREFLY_PRIVATE_KEY=your_private_key_hex

# Optional (defaults shown)
FIREFLY_HOST=localhost
FIREFLY_GRPC_PORT=40401
FIREFLY_HTTP_PORT=40403
```

---

## Running Tests

```bash
# Run the full test suite
cargo test --test test_rho20_contract -- --ignored --nocapture

# Run with debug logging
RUST_LOG=debug cargo test --test test_rho20_contract -- --ignored --nocapture
```

---

## Success Indicators

When the contract deploys successfully, you'll see in the F1r3fly logs:

```
("Rho20 Token Registered", "URI:", `rho:id:nbhkwm5hjryr9m7cqbcsw5a5bfy8us475juqd9ku6mnw66mprmxd96`, "Ticker:", "TEST", "Supply:", 1000000)
```

When queries work, you'll see non-empty `expr` arrays in HTTP responses:

```json
{
  "expr": [
    {
      "ExprMap": {
        "data": { ... }
      }
    }
  ]
}
```

---

**Document Version**: 1.0  
**Last Updated**: 2025-11-10  
**Status**: ✅ Production Ready - All tests passing

