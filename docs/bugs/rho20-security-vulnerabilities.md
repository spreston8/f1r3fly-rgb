# RHO20 Contract Security Vulnerabilities & Solutions

**Status:** Critical Security Issues  
**Priority:** P0 - Must Fix Before Production  
**Created:** 2025-11-19  
**Affects:** All RHO20 token contracts  

## Overview

The current RHO20 contract implementation has three critical security vulnerabilities that must be addressed before production deployment. These issues stem from missing authorization checks in the contract methods.

---

## Issue 1: Unauthorized Token Issuance

### **Severity:** CRITICAL 🔴

### Problem Description

The `issue()` method has no authorization checks, allowing **anyone** to mint unlimited tokens from the unallocated supply.

### Current Vulnerable Code

```rholang
// f1r3fly-rgb/src/templates/rho20_contract.rho (lines 89-130)
contract Rho20Token(@"issue", @recipient, @amount, ret) = {
  if (amount <= 0) {
    ret!({"success": false, "error": "Amount must be positive"})
  } else {
    // NO AUTHENTICATION CHECK!
    // Anyone can call this and mint tokens
    for(@currentUnallocated <- unallocated) {
      if (amount <= currentUnallocated) {
        // Allocates tokens without verifying caller
        balances.set(recipient, amount)
        ret!({"success": true})
      }
    }
  }
}
```

### Attack Scenario

```rust
// Attacker with F1r3node access
attacker.call_contract("issue", [
    ("recipient", "attacker_address"),
    ("amount", 1000000)
]);

// Result: Attacker mints 1M tokens ❌
// No signature required
// No authorization check
// Inflation of total supply
```

### Proposed Solution: Deployer Signature + Nonce

Require the contract deployer to sign each issuance with a nonce for replay protection.

```rholang
new deployerPubKeyCh, usedNoncesCh in {
  // Store deployer's public key at deployment
  deployerPubKeyCh!("{{DEPLOYER_PUBLIC_KEY}}") |
  usedNoncesCh!({}) |  // Track used nonces for replay protection
  
  contract Rho20Token(@"issue", @recipient, @amount, @nonce, @signature, ret) = {
    if (amount <= 0) {
      ret!({"success": false, "error": "Amount must be positive"})
    } else {
      for(@deployerPubKey <<- deployerPubKeyCh) {
        // Verify signature: sign(recipient || amount || nonce)
        new sigValidCh in {
          verifySignature!(
            (recipient, amount, nonce),  // Message
            signature,                    // Signature from caller
            deployerPubKey,              // Expected signer
            *sigValidCh
          ) |
          
          for(@isValid <- sigValidCh) {
            if (isValid) {
              // Check nonce hasn't been used (prevent replay)
              for(@usedNonces <- usedNoncesCh) {
                if (usedNonces.contains(nonce)) {
                  ret!({"success": false, "error": "Nonce already used"})
                } else {
                  // Mark nonce as used
                  usedNoncesCh!(usedNonces.add(nonce)) |
                  
                  // Proceed with issue
                  for(@currentUnallocated <- unallocated) {
                    if (amount <= currentUnallocated) {
                      unallocated!(currentUnallocated - amount) |
                      balances.set(recipient, amount) |
                      ret!({"success": true, "balance": amount})
                    } else {
                      ret!({"success": false, "error": "Insufficient unallocated supply"})
                    }
                  }
                }
              }
            } else {
              ret!({"success": false, "error": "Invalid signature - unauthorized"})
            }
          }
        }
      }
    }
  }
}
```

### Wallet Implementation

```rust
// In issue_asset() - wallet code
pub async fn issue_asset(&mut self, request: IssueAssetRequest) -> Result<AssetInfo> {
    let deployer_key = self.get_deployer_key()?;
    let nonce = self.generate_nonce()?;
    
    // Create message: (recipient, amount, nonce)
    let message = format!("{}{}{}", 
        request.genesis_utxo, 
        request.supply, 
        nonce
    );
    
    // Sign with deployer's private key
    let signature = deployer_key.sign(&message)?;
    
    // Call contract with signature
    let result = contract.call_method("issue", &[
        ("recipient", request.genesis_utxo),
        ("amount", request.supply),
        ("nonce", nonce),
        ("signature", hex::encode(signature))
    ]).await?;
    
    Ok(result)
}
```

### Security Properties

✅ **Prevents unauthorized minting** - Only deployer can sign  
✅ **Replay protection** - Nonces prevent reusing old signatures  
✅ **Cryptographically secure** - Uses secp256k1 (Bitcoin's signature scheme)  
✅ **Feasible** - Rholang supports signature verification  

### Implementation Effort

**Complexity:** Low  
**Estimated Time:** 1-2 days  
**Files to Modify:**
- `f1r3fly-rgb/src/templates/rho20_contract.rho` - Add signature verification
- `f1r3fly-rgb-wallet/src/f1r3fly/asset.rs` - Add signature generation

---

## Issue 2: Unauthorized Transfers

### **Severity:** CRITICAL 🔴

### Problem Description

The `transfer()` method has no authorization checks, allowing **anyone** to transfer tokens from any address to any other address.

### Current Vulnerable Code

```rholang
// f1r3fly-rgb/src/templates/rho20_contract.rho (lines 135-175)
contract Rho20Token(@"transfer", @from, @to, @amount, ret) = {
  if (amount <= 0) {
    ret!({"success": false, "error": "Amount must be positive"})
  } else {
    // NO AUTHENTICATION CHECK!
    for(@fromBalance <- balances.get(from)) {
      if (fromBalance >= amount) {
        // Anyone can call this with any 'from' address
        balances.set(from, fromBalance - amount) |
        balances.set(to, balances.get(to) + amount)
        ret!({"success": true})
      }
    }
  }
}
```

### Attack Scenario

```rust
// Attacker observes Alice has 10,000 tokens at "alice_utxo:0"
attacker.call_contract("transfer", [
    ("from", "alice_utxo:0"),       // Alice's UTXO
    ("to", "attacker_utxo:0"),      // Attacker's address
    ("amount", 10000)               // All of Alice's tokens
]);

// Result: Alice's balance is zeroed in F1r3fly state ❌
// Even though attacker can't create valid consignment,
// Alice's tokens are destroyed in the contract
```

### Why This Is Critical

Even though the attacker cannot create a valid Bitcoin-anchored consignment (requires Bitcoin private keys), the damage is done:
- Alice's balance in F1r3fly contract is set to 0
- Attacker's balance in F1r3fly contract is set to 10,000
- Contract state is corrupted
- Alice loses access to her tokens

### Proposed Solution: Signature-Required Transfer + Owner Registration

Every transfer requires a signature from the owner of the `from` UTXO, proving they authorized the transfer.

#### Part 1: Register Owners During Issue

```rholang
new utxoOwnersCh in {
  utxoOwnersCh!({}) |  // Map: utxo → owner_pubkey
  
  // Modified issue to register owner
  contract Rho20Token(@"issue", @recipient, @amount, @recipientPubKey, @nonce, @deployerSig, ret) = {
    // Verify deployer signature (Issue 1 solution)
    verifyDeployerSignature!(deployerSig, nonce, *deployerValidCh) |
    
    for(@deployerValid <- deployerValidCh) {
      if (deployerValid) {
        // Issue tokens
        balances.set(recipient, amount) |
        
        // REGISTER OWNER
        for(@owners <- utxoOwnersCh) {
          utxoOwnersCh!(owners.set(recipient, recipientPubKey)) |
          ret!({"success": true, "balance": amount})
        }
      }
    }
  }
}
```

#### Part 2: Require Signature for Transfer

```rholang
contract Rho20Token(@"transfer", @from, @to, @amount, @toPubKey, @fromSignature, ret) = {
  if (amount <= 0) {
    ret!({"success": false, "error": "Amount must be positive"})
  } else {
    // Get owner of 'from' UTXO
    for(@ownerPubKey <- utxoOwners.get(from)) {
      if (ownerPubKey == Nil) {
        ret!({"success": false, "error": "Unknown sender - no registered owner"})
      } else {
        // Verify signature: sign(from || to || amount)
        new sigValidCh in {
          verifySignature!(
            (from, to, amount),    // Message
            fromSignature,         // Signature from caller
            ownerPubKey,          // Expected signer (owner of 'from')
            *sigValidCh
          ) |
          
          for(@isValid <- sigValidCh) {
            if (isValid) {
              // Authorized - execute transfer
              for(@fromBalance <- balances.get(from)) {
                if (fromBalance >= amount) {
                  // Deduct from sender
                  balances.set(from, fromBalance - amount) |
                  
                  // Add to recipient
                  for(@toBalance <- balances.get(to)) {
                    balances.set(to, toBalance + amount) |
                    
                    // Register new owner
                    for(@owners <- utxoOwnersCh) {
                      utxoOwnersCh!(owners.set(to, toPubKey)) |
                      ret!({"success": true, "from_balance": fromBalance - amount})
                    }
                  }
                } else {
                  ret!({"success": false, "error": "Insufficient balance"})
                }
              }
            } else {
              ret!({"success": false, "error": "Invalid signature - unauthorized transfer"})
            }
          }
        }
      }
    }
  }
}
```

### Wallet Implementation

```rust
// In send_transfer() - wallet code
pub async fn send_transfer(&mut self, invoice: &str, fee_rate: &FeeRateConfig) 
    -> Result<TransferResponse> {
    
    let parsed = parse_invoice(invoice)?;
    let from_utxo = self.get_genesis_utxo()?;
    let to_identifier = extract_recipient_identifier(&parsed)?;
    let amount = parsed.amount.unwrap();
    
    // Get recipient's public key (from invoice or generate)
    let recipient_pubkey = extract_or_generate_pubkey(&parsed)?;
    
    // Create message to sign
    let message = format!("{}{}{}", from_utxo, to_identifier, amount);
    
    // Sign with wallet's RGB signing key
    let signature = self.rgb_signing_key.sign(&message)?;
    
    // Call contract with signature
    let result = contract.call_method("transfer", &[
        ("from", from_utxo),
        ("to", to_identifier),
        ("amount", amount),
        ("toPubKey", recipient_pubkey),
        ("fromSignature", hex::encode(signature))
    ]).await?;
    
    // Continue with Bitcoin TX creation, Tapret, etc.
    // ...
}
```

### Key Derivation

```rust
// Derive RGB signing key from mnemonic
let seed = Mnemonic::from_phrase(&mnemonic)?;
let master_key = seed.to_seed("");

// Derive Bitcoin keys (for Bitcoin transactions)
let bitcoin_key = master_key.derive("m/84'/0'/0'/0/0")?;

// Derive RGB signing key (for contract signatures)
// Standard path: m/44'/RGB'/0'/0/0
let rgb_signing_key = master_key.derive("m/44'/1919'/0'/0/0")?;
```

### Security Properties

✅ **Prevents unauthorized transfers** - Only owner can sign  
✅ **Cryptographically secure** - Uses secp256k1  
✅ **Ownership tracking** - Each UTXO has registered owner  
✅ **Enables transfer chains** - Ownership migrates with transfers  

### Challenges & Considerations

1. **Public Key Management**
   - Issuer must obtain recipient's public key during issuance
   - Adds complexity to issuance flow
   - Need standardized key derivation path

2. **Invoice Format Changes**
   - Invoices must include recipient's public key
   - Breaking change to invoice format
   - Need invoice version bump

3. **Key Derivation Standardization**
   - All wallets must use same derivation path
   - Document: `m/44'/1919'/0'/0/0` for RGB signing

4. **Interoperability**
   - Wallets must agree on standards
   - Need specification document

### Implementation Effort

**Complexity:** Medium-High  
**Estimated Time:** 1 week  
**Files to Modify:**
- `f1r3fly-rgb/src/templates/rho20_contract.rho` - Add signature verification and owner tracking
- `f1r3fly-rgb-wallet/src/f1r3fly/transfer.rs` - Add signature generation
- `f1r3fly-rgb-wallet/src/f1r3fly/invoice.rs` - Include pubkey in invoices
- `f1r3fly-rgb-wallet/src/keys.rs` - Add RGB signing key derivation

---

## Issue 3: Witness Identifier Balance Tracking

### **Severity:** MEDIUM 🟡

### Problem Description

When tokens are transferred, the F1r3fly contract stores balances using a **witness identifier** (generated before Bitcoin TX), but the recipient's actual Bitcoin UTXO has a **different identifier**. This causes balance queries to fail for recipients.

### Technical Background

**Why Witness Identifiers Exist:**

F1r3fly-RGB requires executing the contract BEFORE broadcasting the Bitcoin transaction:
1. Execute contract → get `state_hash`
2. Embed `state_hash` in Bitcoin TX (Tapret commitment)
3. Broadcast Bitcoin TX

This means the recipient's UTXO doesn't exist when the contract executes, so we use a deterministic placeholder.

### The Mismatch

```
Alice sends 2,500 tokens to Bob:

1. Bob's invoice contains address: bc1q...xyz

2. Wallet generates witness identifier:
   witness_id = "witness:" + SHA256("bc1q...xyz")[0..16] + ":0"
   = "witness:a3467636599ef254:0"

3. Contract executes:
   balances["alice_utxo:0"] = 10000 → 7500
   balances["witness:a3467636599ef254:0"] = 0 → 2500

4. Bitcoin TX broadcasts → creates Bob's UTXO:
   actual_utxo = "9b7b09e4cd136021:0"  ← DIFFERENT!

5. Bob queries: contract.balanceOf("9b7b09e4cd136021:0")
   Result: 0 (balance is at witness_id, not actual UTXO) ❌
```

### Current Workaround

```rust
// f1r3fly-rgb-wallet/src/f1r3fly/balance.rs
// Query in TWO passes (inefficient, fragile)

// Pass 1: Check actual UTXOs
for utxo in wallet.list_unspent() {
    balance += contract.balanceOf(format!("{}:{}", utxo.txid, utxo.vout));
}

// Pass 2: Check witness identifiers (THE HACK)
for address in wallet.addresses() {
    let witness_id = format!("witness:{}:0", SHA256(address)[0..16]);
    balance += contract.balanceOf(witness_id);
}
```

**Problems:**
- Tight coupling between transfer and balance logic
- Performance overhead (2x queries)
- Not RGB-standard

### Proposed Solution: Claim Method with Ownership Migration

Add a `claim()` method that migrates balance AND ownership from witness identifier to real UTXO.

#### Part 1: Store Witness Mapping in Consignment

```rust
// In F1r3flyConsignment (f1r3fly-rgb crate)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct F1r3flyConsignment {
    // ... existing fields ...
    
    /// Witness identifier mapping (only for transfers)
    /// Links witness_id → (real_utxo, recipient_address)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub witness_mapping: Option<WitnessMapping>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WitnessMapping {
    pub witness_id: String,
    pub recipient_address: String,
    pub expected_vout: u32,
}
```

#### Part 2: Add Claim Method to Contract

```rholang
contract Rho20Token(@"claim", @witness_id, @real_utxo, @claimantSignature, ret) = {
  // Step 1: Get owner of witness_id
  for(@ownerPubKey <- utxoOwners.get(witness_id)) {
    if (ownerPubKey == Nil) {
      ret!({"success": false, "error": "No owner registered for witness_id"})
    } else {
      // Step 2: Verify signature proves ownership
      new sigValidCh in {
        verifySignature!(
          (witness_id, real_utxo),  // Message
          claimantSignature,         // Signature
          ownerPubKey,              // Expected signer
          *sigValidCh
        ) |
        
        for(@isValid <- sigValidCh) {
          if (isValid) {
            // Step 3: Get balance at witness_id
            for(@balance <- balances.get(witness_id)) {
              if (balance > 0) {
                // Step 4: ATOMIC MIGRATION
                // Migrate BOTH balance AND ownership
                
                // Delete from witness location
                balances.delete(witness_id) |
                utxoOwners.delete(witness_id) |
                
                // Set at real location
                balances.set(real_utxo, balance) |
                utxoOwners.set(real_utxo, ownerPubKey) |
                
                ret!({"success": true, "migrated_balance": balance, "new_owner": ownerPubKey})
              } else {
                ret!({"success": false, "error": "No balance at witness_id"})
              }
            }
          } else {
            ret!({"success": false, "error": "Invalid signature - unauthorized claim"})
          }
        }
      }
    }
  }
}
```

**CRITICAL:** Claim must migrate BOTH balance AND ownership, otherwise recipient cannot transfer later!

#### Part 3: Auto-Claim During Consignment Acceptance

```rust
// In accept_consignment() - wallet code
pub async fn accept_consignment(
    contracts_manager: &mut F1r3flyContractsManager,
    bitcoin_wallet: &BitcoinWallet,
    consignment_path: &Path
) -> Result<AcceptConsignmentResponse> {
    // ... existing validation ...
    
    // If transfer consignment with witness mapping
    if !consignment.is_genesis {
        if let Some(witness_mapping) = &consignment.witness_mapping {
            // Find Bob's actual UTXO that matches the recipient address
            let my_utxos = bitcoin_wallet.list_unspent();
            
            let actual_utxo = my_utxos.iter()
                .find(|utxo| 
                    utxo.address == witness_mapping.recipient_address &&
                    utxo.vout == witness_mapping.expected_vout
                )
                .ok_or("Expected UTXO not found in wallet")?;
            
            let real_utxo = format!("{}:{}", actual_utxo.txid, actual_utxo.vout);
            
            // Sign claim: prove ownership of address
            let message = format!("{}{}", witness_mapping.witness_id, real_utxo);
            let signature = bitcoin_wallet.sign_message(&message)?;
            
            // Call claim method to migrate balance
            let claim_result = contract.call_method("claim", &[
                ("witness_id", witness_mapping.witness_id.clone()),
                ("real_utxo", real_utxo.clone()),
                ("claimantSignature", hex::encode(signature))
            ]).await?;
            
            log::info!("✓ Balance migrated from {} to {}", 
                witness_mapping.witness_id, real_utxo);
        }
    }
    
    Ok(response)
}
```

### Flow Diagram

```
Alice → Bob Transfer:

1. Bob generates invoice (address: bc1q...xyz)
2. Alice's wallet creates witness_id: "witness:a346...:0"
3. Contract executes:
   balances["witness:a346...:0"] = 2500
   utxoOwners["witness:a346...:0"] = bob_pubkey

4. Bitcoin TX broadcasts → Bob's UTXO: "9b7b...21:0"

5. Consignment created with mapping:
   witness_mapping: {
     witness_id: "witness:a346...:0",
     recipient_address: "bc1q...xyz",
     expected_vout: 0
   }

6. Bob accepts consignment:
   - Finds his UTXO matching address
   - Signs claim message
   - Calls claim()

7. Contract migrates:
   DELETE: balances["witness:a346...:0"]
   DELETE: utxoOwners["witness:a346...:0"]
   SET: balances["9b7b...21:0"] = 2500
   SET: utxoOwners["9b7b...21:0"] = bob_pubkey

8. Bob can now:
   - Query: balanceOf("9b7b...21:0") → 2500 ✓
   - Transfer: Uses his real UTXO ✓
```

### Security Properties

✅ **Signature-protected** - Only owner can claim  
✅ **Atomic migration** - Balance and ownership together  
✅ **Enables future transfers** - Owner correctly registered  
✅ **Cannot be exploited** - Attacker can't forge signatures  

### Implementation Effort

**Complexity:** Medium  
**Estimated Time:** 3-4 days  
**Files to Modify:**
- `f1r3fly-rgb/src/consignment.rs` - Add witness_mapping field
- `f1r3fly-rgb/src/templates/rho20_contract.rho` - Add claim method
- `f1r3fly-rgb-wallet/src/f1r3fly/transfer.rs` - Generate witness mapping
- `f1r3fly-rgb-wallet/src/f1r3fly/consignment.rs` - Auto-claim logic
- `f1r3fly-rgb-wallet/src/f1r3fly/balance.rs` - Remove witness querying workaround

---

## Implementation Priority

### Phase 1: Issue Security (P0)
**Timeline:** 1-2 days  
**Blocker:** Yes - prevents unauthorized minting  

Implement Issue 1 solution:
- Add deployer signature verification to `issue()`
- Add nonce tracking for replay protection

### Phase 2: Transfer Security (P0)
**Timeline:** 1 week  
**Blocker:** Yes - prevents token theft  

Implement Issue 2 solution:
- Add owner registration during `issue()`
- Add signature verification to `transfer()`
- Implement RGB signing key derivation
- Update invoice format with pubkeys

### Phase 3: Witness Migration (P1)
**Timeline:** 3-4 days  
**Blocker:** No - workaround exists  

Implement Issue 3 solution:
- Add `claim()` method
- Auto-execute during consignment acceptance
- Remove balance query workaround

---

## Testing Requirements

### Issue 1 Tests
```rust
#[test]
async fn test_issue_requires_deployer_signature() {
    // Should fail without signature
    // Should fail with invalid signature
    // Should fail with reused nonce
    // Should succeed with valid signature
}
```

### Issue 2 Tests
```rust
#[test]
async fn test_transfer_requires_owner_signature() {
    // Should fail without signature
    // Should fail with wrong signature
    // Should succeed with correct signature
}

#[test]
async fn test_recipient_can_transfer_after_receiving() {
    // Alice → Bob → Carol
    // Verify Bob can transfer what he received
}
```

### Issue 3 Tests
```rust
#[test]
async fn test_claim_migrates_balance() {
    // Should fail with invalid signature
    // Should migrate balance from witness → real UTXO
    // Should allow querying with real UTXO
}

#[test]
async fn test_recipient_can_transfer_after_claim() {
    // Alice → Bob (Bob claims) → Carol
    // Verify ownership migrated correctly
}
```

---

## Security Model Summary

### Current State (VULNERABLE)
```
┌─────────────────────────────────────┐
│ Wallet Software                     │
│ - User authorization                │
│ - Consignment validation            │
└─────────────────────────────────────┘
            ↓
┌─────────────────────────────────────┐
│ F1r3fly Contract (NO PROTECTION!)   │
│ ❌ issue() - anyone can call        │
│ ❌ transfer() - anyone can call     │
│ ⚠️  balances at wrong identifiers   │
└─────────────────────────────────────┘
```

### Proposed State (SECURE)
```
┌─────────────────────────────────────┐
│ Wallet Software                     │
│ - Signs with RGB key                │
│ - Provides pubkeys                  │
│ - Auto-claims on accept             │
└─────────────────────────────────────┘
            ↓ signed messages
┌─────────────────────────────────────┐
│ F1r3fly Contract (PROTECTED)        │
│ ✅ issue() - deployer sig required  │
│ ✅ transfer() - owner sig required  │
│ ✅ claim() - recipient sig required │
│ ✅ balances at correct identifiers  │
└─────────────────────────────────────┘
```

---

## References

- RHO20 Contract: `f1r3fly-rgb/src/templates/rho20_contract.rho`
- Transfer Logic: `f1r3fly-rgb-wallet/src/f1r3fly/transfer.rs`
- Balance Queries: `f1r3fly-rgb-wallet/src/f1r3fly/balance.rs`
- Consignment: `f1r3fly-rgb/src/consignment.rs`
- RevVault Reference: `f1r3node/casper/src/main/resources/RevVault.rho`

---

## Related Documents

- [Witness Identifier Balance Tracking](witness-identifier-balance-tracking.md) - Original Issue 3 analysis
- Test Plan: `docs/plans/remaining-integration-tests-plan.md` (Test Cases 4-5)

---

## Notes

**Why Not Bitcoin Anchor Validation in Contract?**

We cannot implement full consignment validation (Bitcoin anchor, Tapret proofs) in the contract because:
- Rholang cannot query Bitcoin blockchain
- Rholang cannot query F1r3fly blockchain (check finalization)
- Rholang cannot make external API calls

Therefore, we use **signature-based authorization** which:
- ✅ Is feasible with Rholang's capabilities
- ✅ Provides cryptographic security
- ✅ Prevents unauthorized operations
- ✅ Complements client-side validation

**Defense in Depth:**
- **Contract Level:** Signature verification (this document)
- **Client Level:** Consignment validation (existing)
- **Bitcoin Level:** Tapret anchoring (existing)
- **Access Level:** F1r3node RPC restrictions (infrastructure)

