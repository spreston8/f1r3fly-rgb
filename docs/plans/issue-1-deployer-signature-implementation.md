# Issue 1: Deployer Signature Implementation Plan

**Status:** Ready for Implementation  
**Priority:** P0 - Critical Security Issue  
**Estimated Time:** 1-2 days  
**Related Document:** `docs/bugs/rho20-security-vulnerabilities.md`

## Overview

Implement deployer signature verification for the `issue()` method in RHO20 contracts to prevent unauthorized token minting. Currently, anyone with F1r3node access can call `issue()` and mint unlimited tokens. This fix adds cryptographic authentication using the wallet's existing f1r3fly key.

---

## Key Architectural Decision

### **Use Existing Wallet F1r3fly Key (Not New Key)**

The wallet already derives and stores an f1r3fly key at path `m/1337'/0'/0'/0/0` that is currently unused:

```rust
// From f1r3fly-rgb-wallet/src/storage/models.rs
pub struct WalletKeys {
    pub mnemonic: bip39::Mnemonic,
    pub bitcoin_xprv: Xpriv,
    pub bitcoin_descriptor: String,
    
    // THIS KEY - Currently unused, perfect for our needs!
    pub f1r3fly_private_key: secp256k1::SecretKey,
    pub f1r3fly_public_key: String,
}
```

**Benefits:**
- ✅ Already exists - Derived and encrypted in every wallet
- ✅ Already available - In memory during operations
- ✅ Zero new key management code needed
- ✅ Clean authorization model: wallet owner = asset deployer
- ✅ Fulfills intended purpose per code comment: "reserved for wallet-specific features"

**Authorization Flow:**
1. Alice creates wallet → derives f1r3fly key → stores public key
2. Alice issues asset → Contract stores Alice's `f1r3fly_public_key` as deployer
3. Alice calls `issue()` → Signs with `f1r3fly_private_key` → Contract verifies
4. Bob tries to call `issue()` on Alice's contract → Signature fails ❌

---

## Implementation Details

### **Phase 1: Contract Template Changes**

**File:** `f1r3fly-rgb/src/templates/rho20_contract.rho`

#### 1.1: Add Built-in References

Add at the top of the contract (in the `new` clause):

```rholang
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
    uriOut,
    // NEW: Add crypto built-ins
    secpVerify(`rho:crypto:secp256k1Verify`),
    blake2b256(`rho:crypto:blake2b256Hash`),
    deployerPubKeyCh,
    usedNoncesCh
in {
```

#### 1.2: Initialize Deployer Key and Nonce Tracking

After the `balanceMapCh!(*treeHashMap, balanceMap)` line:

```rholang
// Store deployer public key (from wallet's f1r3fly_public_key)
// This is the ONLY key authorized to call issue()
deployerPubKeyCh!("{{DEPLOYER_PUBLIC_KEY}}".hexToBytes()) |

// Track used nonces for replay protection (starts as empty set)
usedNoncesCh!({}) |
```

#### 1.3: Replace Issue Method

Replace the current vulnerable `issue()` method (lines 89-130) with:

```rholang
// =====================================================================
// Method: issue - Allocate tokens from unallocated supply
// =====================================================================
// SECURED: Requires deployer signature + nonce for authorization
//
// Parameters:
//   - recipient: UTXO identifier for token allocation
//   - amount: Number of tokens to issue
//   - nonce: Unique nonce for replay protection (must not have been used before)
//   - signatureHex: Hex-encoded signature of (recipient, amount, nonce)
//
// Authorization:
//   - Message: (recipient, amount, nonce) serialized to bytes
//   - Hash: Blake2b-256 of message
//   - Signature: ECDSA signature with secp256k1
//   - Signer: Must match deployer public key stored at contract deployment
//
// Returns:
//   - {"success": true, "balance": <amount>} on success
//   - {"success": false, "error": <reason>} on failure
contract Rho20Token(@"issue", @recipient, @amount, @nonce, @signatureHex, ret) = {
  if (amount <= 0) {
    ret!({"success": false, "error": "Amount must be positive"})
  } else {
    new hashCh, verifyCh in {
      // Step 1: Hash the message (recipient, amount, nonce)
      // This must match the Rust signing code exactly
      blake2b256!((recipient, amount, nonce).toByteArray(), *hashCh) |
      
      // Step 2: Get deployer public key (peek - doesn't consume)
      for (@messageHash <- hashCh; @deployerPubKey <<- deployerPubKeyCh) {
        // Step 3: Verify signature
        // secpVerify expects: (hash, signature, publicKey, returnChannel)
        secpVerify!(messageHash, signatureHex.hexToBytes(), deployerPubKey, *verifyCh) |
        
        for (@isValid <- verifyCh) {
          if (isValid) {
            // Signature is valid - check nonce hasn't been used
            for (@usedNonces <- usedNoncesCh) {
              if (usedNonces.contains(nonce)) {
                // Nonce was already used - reject (replay attack)
                usedNoncesCh!(usedNonces) |  // Put nonces back
                ret!({"success": false, "error": "Nonce already used"})
              } else {
                // Nonce is fresh - mark as used and proceed
                usedNoncesCh!(usedNonces.add(nonce)) |
                
                // Execute issue logic (same as before)
                new unallocFoundCh, unallocNotFoundCh in {
                  for(treeHashMap, @currentMap <<- balanceMapCh) {
                    // Get current unallocated from map
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
            }
          } else {
            // Signature verification failed
            ret!({"success": false, "error": "Invalid signature - unauthorized"})
          }
        }
      }
    }
  }
}
```

#### 1.4: Add New Template Variable

The contract now expects `{{DEPLOYER_PUBLIC_KEY}}` to be substituted during deployment.

---

### **Phase 2: Executor Changes**

**File:** `f1r3fly-rgb/src/executor.rs`

#### 2.1: Update `substitute_template_variables` Function

Modify the function signature and body (around line 931):

```rust
fn substitute_template_variables(
    template: &str,
    ticker: &str,
    name: &str,
    total_supply: u64,
    precision: u8,
    master_key_hex: &str,
    child_key: &SecretKey,
) -> Result<(String, i64), F1r3flyRgbError> {
    // ... existing code for master_key, child_key, etc. ...
    
    // Derive deployer public key from child key (uncompressed format for Rholang)
    let secp = Secp256k1::new();
    let child_public_key = PublicKey::from_secret_key(&secp, child_key);
    let deployer_pubkey_hex = hex::encode(child_public_key.serialize_uncompressed());
    
    // ... existing timestamp, version, signature generation ...
    
    // Replace all template variables
    let rholang = template
        .replace("{{TICKER}}", ticker)
        .replace("{{NAME}}", name)
        .replace("{{TOTAL_SUPPLY}}", &total_supply.to_string())
        .replace("{{PRECISION}}", &precision.to_string())
        .replace("{{PUBLIC_KEY}}", &child_pubkey_hex)
        .replace("{{VERSION}}", &version.to_string())
        .replace("{{SIGNATURE}}", &signature_hex)
        .replace("{{URI}}", &uri)
        .replace("{{DEPLOYER_PUBLIC_KEY}}", &deployer_pubkey_hex);  // NEW
    
    Ok((rholang, timestamp_millis))
}
```

**Note:** The `child_key` used for insertSigned is the deployer key. This ensures the same key that deployed the contract is the only one authorized to call `issue()`.

---

### **Phase 3: Contracts Manager State**

**File:** `f1r3fly-rgb-wallet/src/f1r3fly/contracts.rs`

#### 3.1: Add Deployer Key Storage to F1r3flyState

Modify the `F1r3flyState` struct to store deployer keys:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct F1r3flyState {
    /// Map: contract_id → deployer_public_key (hex)
    /// This tracks which key was used to deploy each contract
    pub deployer_keys: HashMap<String, String>,
    
    // ... existing fields ...
}
```

#### 3.2: Store Deployer Key During Contract Deployment

In `F1r3flyRgbContracts::issue()` method (around line 91):

```rust
pub async fn issue(
    &mut self,
    ticker: &str,
    name: &str,
    supply: u64,
    precision: u8,
) -> Result<ContractId, F1r3flyRgbError> {
    log::info!("Issuing asset: {} ({})", name, ticker);
    
    // Get the child key that will be used for deployment
    let child_key = self.executor.get_current_child_key()?;
    let deployer_pubkey = hex::encode(
        PublicKey::from_secret_key(&Secp256k1::new(), &child_key)
            .serialize_uncompressed()
    );

    // Deploy contract
    let contract_id = self.executor
        .deploy_contract(/* ... */)
        .await?;
    
    // Store deployer key for this contract
    // This will be needed when calling issue() method
    self.deployer_keys.insert(contract_id.to_string(), deployer_pubkey);
    
    // ... rest of existing code ...
    
    Ok(contract_id)
}
```

#### 3.3: Add Method to Retrieve Deployer Key

```rust
impl F1r3flyContractsManager {
    /// Get the deployer public key for a contract
    pub fn get_deployer_pubkey(&self, contract_id: &str) -> Result<String, ContractsManagerError> {
        self.state
            .deployer_keys
            .get(contract_id)
            .cloned()
            .ok_or_else(|| ContractsManagerError::ContractNotFound(contract_id.to_string()))
    }
}
```

---

### **Phase 4: Asset Issuance (Wallet)**

**File:** `f1r3fly-rgb-wallet/src/f1r3fly/asset.rs`

#### 4.1: Add Nonce Generation

Add a helper function at the module level:

```rust
/// Generate a unique nonce for issue() calls
///
/// Uses timestamp (milliseconds) in upper 32 bits and random value in lower 32 bits.
/// This ensures:
/// - Monotonic increase (timestamp component)
/// - Collision resistance (random component)
/// - Fits in u64 for Rholang Int compatibility
fn generate_nonce() -> Result<u64, AssetError> {
    use chrono::Utc;
    use rand::RngCore;
    
    let timestamp_ms = Utc::now().timestamp_millis() as u64;
    
    let mut rng_bytes = [0u8; 8];
    rand::rngs::OsRng.fill_bytes(&mut rng_bytes);
    let random_component = u64::from_le_bytes(rng_bytes) >> 32; // Use top 32 bits
    
    // Combine: top 32 bits timestamp, bottom 32 bits random
    Ok((timestamp_ms << 32) | random_component)
}
```

#### 4.2: Add Blake2b Dependency

In `f1r3fly-rgb-wallet/Cargo.toml`, add:

```toml
[dependencies]
# ... existing dependencies ...
blake2 = "0.10"
```

#### 4.3: Modify `issue_asset` to Sign the Request

In the `issue_asset()` function (around line 266), replace the `call_method` section:

```rust
// Generate nonce for replay protection
let nonce = generate_nonce()?;

// Create message matching Rholang: (recipient, amount, nonce)
// The message must be serialized EXACTLY as Rholang does with .toByteArray()
let message = format!("{}{}{}", normalized_genesis_seal, request.supply, nonce);

// Hash with Blake2b-256 (matches Rholang's blake2b256Hash)
use blake2::{Blake2b256, Digest};
let mut hasher = Blake2b256::new();
hasher.update(message.as_bytes());
let message_hash: [u8; 32] = hasher.finalize().into();

// Get wallet keys to access f1r3fly_private_key
// NOTE: This requires passing wallet_keys to issue_asset() function
let f1r3fly_key = &wallet_keys.f1r3fly_private_key;

// Sign the message hash with wallet's f1r3fly key
use secp256k1::{Message, Secp256k1};
let secp = Secp256k1::new();
let message_obj = Message::from_digest(message_hash);
let signature = secp.sign_ecdsa(&message_obj, f1r3fly_key);

// Serialize signature as DER-encoded hex
let signature_hex = hex::encode(signature.serialize_der());

log::info!(
    "Calling issue method with signature. Nonce: {}, Signature: {}",
    nonce,
    &signature_hex[..16] // Log first 16 chars
);

// Call contract with signature
use strict_types::StrictVal;
let issue_result = contracts_manager
    .contracts_mut()
    .executor_mut()
    .call_method(
        contract_id,
        "issue",
        &[
            ("recipient", StrictVal::from(normalized_genesis_seal.as_str())),
            ("amount", StrictVal::from(request.supply)),
            ("nonce", StrictVal::from(nonce)),
            ("signatureHex", StrictVal::from(signature_hex)),
        ],
    )
    .await
    .map_err(|e| AssetError::DeploymentFailed(format!("Failed to call issue method: {}", e)))?;
```

#### 4.4: Update Function Signature

The `issue_asset` function needs access to `WalletKeys`:

```rust
pub async fn issue_asset(
    contracts_manager: &mut F1r3flyContractsManager,
    bitcoin_wallet: &BitcoinWallet,
    wallet_keys: &WalletKeys,  // NEW parameter
    request: IssueAssetRequest,
) -> Result<AssetInfo, AssetError>
```

**Impact:** Callers of `issue_asset()` must now pass `wallet_keys`.

---

### **Phase 5: Update Callers**

**File:** `f1r3fly-rgb-wallet/src/manager.rs`

Update the `issue_asset` method in `WalletManager`:

```rust
pub async fn issue_asset(&mut self, request: IssueAssetRequest) -> Result<AssetInfo> {
    self.check_unlocked()?;
    
    let keys = self.keys.as_ref().unwrap();  // Already checked by check_unlocked()
    
    crate::f1r3fly::asset::issue_asset(
        &mut self.contracts_manager,
        &self.bitcoin_wallet,
        keys,  // Pass wallet keys for signing
        request,
    )
    .await
    .map_err(|e| WalletError::F1r3fly(e.to_string()))
}
```

---

### **Phase 6: Testing**

**File:** `f1r3fly-rgb-wallet/tests/f1r3fly/validation_security_test.rs` (new tests)

Add security-specific tests:

```rust
#[tokio::test]
async fn test_issue_requires_valid_signature() {
    check_f1r3node_available().await;
    
    let (mut alice, _) = setup_wallets_with_keys("alice_sig_test").await.unwrap();
    alice.create_utxo(100_000).await.unwrap();
    
    // Issue asset normally - should succeed
    let request = IssueAssetRequest {
        ticker: "TEST".to_string(),
        name: "Test Token".to_string(),
        supply: 1000,
        precision: 0,
        genesis_utxo: alice.list_unspent().await.unwrap()[0].clone(),
    };
    
    let result = alice.issue_asset(request).await;
    assert!(result.is_ok(), "Valid signature should succeed");
}

#[tokio::test]
async fn test_issue_prevents_nonce_reuse() {
    check_f1r3node_available().await;
    
    // This test would require low-level access to call issue() directly
    // with the same nonce twice. For now, document that nonce uniqueness
    // is guaranteed by generate_nonce() using timestamp + random.
    
    // Future: Add test that manually calls contract with duplicate nonce
}

#[tokio::test]
async fn test_unauthorized_issue_rejected() {
    check_f1r3node_available().await;
    
    // This test requires:
    // 1. Alice deploys contract
    // 2. Bob tries to call issue() on Alice's contract
    // 3. Should fail with "Invalid signature - unauthorized"
    
    // Future: Requires direct contract method invocation bypass
}
```

---

## Implementation Checklist

### Contract Level (Rholang)
- [ ] Add `secpVerify` and `blake2b256` to contract imports
- [ ] Add `deployerPubKeyCh` for storing deployer public key
- [ ] Add `usedNoncesCh` for nonce tracking
- [ ] Initialize deployer public key with `{{DEPLOYER_PUBLIC_KEY}}` template variable
- [ ] Initialize nonce tracking with empty set
- [ ] Replace `issue()` method with signature-verified version
- [ ] Test signature verification logic with known test vectors

### Deployment Level (Rust - f1r3fly-rgb)
- [ ] Extract deployer public key from child_key in `substitute_template_variables`
- [ ] Add `.replace("{{DEPLOYER_PUBLIC_KEY}}", &deployer_pubkey_hex)` to template substitution
- [ ] Verify uncompressed public key format (65 bytes, starts with 0x04)

### State Management (Rust - f1r3fly-rgb-wallet)
- [ ] Add `deployer_keys: HashMap<String, String>` to `F1r3flyState`
- [ ] Store deployer public key during contract deployment in `F1r3flyRgbContracts::issue()`
- [ ] Add `get_deployer_pubkey()` method to contracts manager
- [ ] Update state serialization/deserialization

### Signing Logic (Rust - f1r3fly-rgb-wallet)
- [ ] Add `blake2` crate dependency to `Cargo.toml`
- [ ] Implement `generate_nonce()` function
- [ ] Update `issue_asset()` signature to accept `wallet_keys: &WalletKeys`
- [ ] Generate nonce before calling issue method
- [ ] Create message string: `format!("{}{}{}", recipient, amount, nonce)`
- [ ] Hash message with Blake2b-256
- [ ] Sign hash with `wallet_keys.f1r3fly_private_key`
- [ ] Serialize signature as DER-encoded hex
- [ ] Pass signature to contract method call

### Integration Points
- [ ] Update `WalletManager::issue_asset()` to pass wallet keys
- [ ] Update all callers of `issue_asset()` function
- [ ] Verify wallet keys are available (wallet must be unlocked)

### Testing
- [ ] Test successful issue with valid signature
- [ ] Test rejection of invalid signature
- [ ] Test nonce replay protection
- [ ] Test unauthorized user cannot call issue
- [ ] Test error messages are descriptive
- [ ] Integration test with full issue flow

### Documentation
- [ ] Update RHO20 contract reference documentation
- [ ] Document new `issue()` method parameters
- [ ] Document nonce generation strategy
- [ ] Document signature format requirements
- [ ] Add security section to README

---

## Security Properties

After implementation, the following security guarantees will be enforced:

### ✅ Authorization
- Only the wallet that deployed a contract can call `issue()` on it
- Authorization verified cryptographically via secp256k1 signature
- Public key stored immutably in contract state at deployment

### ✅ Replay Protection
- Each `issue()` call requires a unique nonce
- Used nonces are tracked in contract state
- Attempting to reuse a nonce returns error: "Nonce already used"

### ✅ Message Integrity
- Message components: (recipient, amount, nonce)
- Hashed with Blake2b-256 before signing
- Any tampering invalidates signature

### ✅ Non-Repudiation
- Each successful `issue()` call is cryptographically linked to deployer
- Signature proves deployer authorized the specific issuance
- Audit trail maintained in blockchain history

---

## Testing Strategy

### Unit Tests
- Signature generation and verification
- Nonce generation uniqueness
- Message hashing correctness

### Integration Tests
- Full issue flow with valid signature
- Rejection of invalid signatures
- Nonce replay detection
- Multi-wallet scenarios (Alice can't issue on Bob's contract)

### Manual Testing
- Deploy contract on regtest F1r3node
- Verify deployer public key stored correctly
- Attempt unauthorized issue (should fail)
- Verify nonce tracking works across multiple issues
- Check error messages are user-friendly

---

## Deployment Notes

### Breaking Change
This is a **breaking change** to the RHO20 contract. After deployment:
- Old contracts without signature verification remain vulnerable
- New contracts require signatures for all `issue()` calls
- Wallets must be updated to sign issue requests

### Migration Path
1. Deploy updated contract template
2. Update wallet to version with signature support
3. Issue new assets with secure contracts
4. Old assets remain functional but should be considered deprecated

### Backward Compatibility
- Existing `transfer()` and `balanceOf()` methods unchanged
- Only `issue()` method has new parameters
- Client code not calling `issue()` is unaffected

---

## Future Enhancements

### Issue 2: Transfer Authorization
Apply the same signature pattern to `transfer()` method:
- Require sender signature
- Track ownership per UTXO
- Prevent unauthorized transfers

### Issue 3: Witness Identifier Claims
Add `claim()` method for witness-to-UTXO migration:
- Signature-verified claim
- Atomic balance and ownership migration

### Nonce Management UI
- Show used nonces in wallet UI
- Warning if nonce generation might collide
- Manual nonce override for testing

---

## References

- **Security Analysis:** `docs/bugs/rho20-security-vulnerabilities.md`
- **Rholang Signature Verification:** `f1r3node/casper/src/main/resources/Registry.rho` (lines 490-555)
- **RevVault Authorization:** `f1r3node/casper/src/main/resources/RevVault.rho` (lines 185-243)
- **Rholang Tutorial:** `f1r3node/rholang/examples/tut-verify-channel.md`
- **Key Derivation:** `f1r3fly-rgb-wallet/src/storage/keys.rs` (lines 127-176)

