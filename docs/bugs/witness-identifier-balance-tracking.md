# Witness Identifier Balance Tracking Issue

**Status:** Known Technical Debt  
**Priority:** Medium  
**Affects:** Balance queries, Transfer flow  
**Created:** 2025-11-19  
**Related Files:**  
- `f1r3fly-rgb-wallet/src/f1r3fly/balance.rs` (lines 303-349)
- `f1r3fly-rgb-wallet/src/f1r3fly/transfer.rs` (lines 156-190)

## Problem Summary

The current implementation uses a **witness identifier workaround** to track balances for in-flight transfers. This creates tight coupling between the transfer and balance query logic, and deviates from RGB protocol standards.

## Technical Background

### RGB Standard Behavior

In traditional RGB:
1. Alice sends transfer → Creates new Bitcoin UTXO for Bob on-chain
2. Bob syncs wallet → Discovers the new UTXO (txid:vout)
3. Bob queries balance → Queries his actual UTXO directly
4. **Result:** Balance is always tied to real Bitcoin UTXOs

### F1r3fly-RGB Current Behavior

Our implementation diverges because the Rholang contract state is updated **before** the Bitcoin transaction is broadcast:

1. **Alice sends transfer:**
   - Invoice contains Bob's address: `bc1p...xyz`
   - Transfer creates witness placeholder: `witness:a3467636599ef2549183ae15:0`
     - Generated via: `SHA256(bob_address)[0..16] + ":0"`
   - Rholang contract executes:
     ```rholang
     balances[genesis_utxo] -= 2500
     balances["witness:a3467636..."] = 2500  // Bob's placeholder
     ```

2. **Bitcoin TX is broadcast:**
   - Creates actual UTXO: `9b7b09e4...cd136021:0`
   - This is a **different identifier** than the witness placeholder

3. **Bob syncs wallet:**
   - Discovers UTXO: `9b7b09e4...cd136021:0`
   - But contract has balance at: `witness:a3467636...`
   - **Mismatch!**

## Current Workaround (Option A)

The `get_asset_balance()` function now performs **two passes**:

### Pass 1: Query Actual UTXOs
```rust
for utxo in wallet_utxos {
    query balanceOf("9b7b09e4...cd136021:0")  // Real Bitcoin UTXO
}
```

### Pass 2: Query Witness Identifiers
```rust
for address in wallet_addresses {
    witness_id = "witness:" + SHA256(address)[0..16] + ":0"
    query balanceOf(witness_id)  // Placeholder identifier
}
```

This allows Bob to find his balance at the temporary witness identifier while the real UTXO exists.

## Why This Is Problematic

### 1. Layer Violation
- Balance query logic is tightly coupled to transfer implementation
- If `transfer.rs` changes its witness ID generation, `balance.rs` breaks
- Bob's wallet must "know" about Alice's transfer logic

### 2. Performance Overhead
```rust
// If Bob has 100 addresses:
// - Queries 100 actual UTXOs
// - Queries 100 witness identifiers
// = 200 total queries per balance check
```

### 3. Not RGB-Standard
- Traditional RGB doesn't need witness placeholders
- Balances are always tied to real Bitcoin UTXOs
- Our approach is a workaround specific to F1r3fly-RGB

### 4. Fragile Assumptions
- Assumes `vout=0` for all witness IDs
- Assumes SHA256 truncation to 16 bytes
- Assumes `"witness:"` prefix
- Any change breaks compatibility

### 5. State Inconsistency Window
- From broadcast until Bob syncs: Balance exists at witness ID
- After Bob syncs: Balance should exist at real UTXO
- No mechanism to migrate balance from witness → real UTXO

## Alternative Solutions

### Option B: Store Balance at Real UTXO From Start
**Challenge:** Cannot predict the txid:vout before broadcast.

```rust
// At transfer time, we don't know:
let future_txid = ???;  // Unknown until broadcast
let future_vout = ???;  // Unknown until broadcast
```

**Verdict:** Not feasible without predicting Bitcoin transaction IDs.

### Option C: Balance Migration via Claim (RECOMMENDED)

Implement a **claim mechanism** where Bob explicitly migrates his balance from witness ID to real UTXO:

#### 1. Extend Consignment Data
```rust
pub struct F1r3flyConsignment {
    // ... existing fields ...
    pub recipient_witness_id: String,  // NEW: "witness:..."
    pub recipient_vout: u32,            // NEW: Expected vout
}
```

#### 2. Add Claim Method to RHO20 Contract
```rholang
new claim in {
  contract claim(@witness_id, @real_utxo, return) = {
    for (@balance <- balances.get(witness_id)) {
      balances.set!(real_utxo, balance) |
      balances.delete!(witness_id) |
      return!(true)
    }
  }
}
```

#### 3. Auto-Claim During Consignment Acceptance
```rust
// In accept_consignment():
if let Some(witness_id) = consignment.recipient_witness_id {
    let my_utxo = discover_new_utxo_from_consignment();
    contract.call_method("claim", &[
        ("witness_id", StrictVal::from(witness_id)),
        ("real_utxo", StrictVal::from(serialize_seal(my_utxo))),
    ]).await?;
}
```

#### Benefits:
- ✅ Explicit state transition (witness → real UTXO)
- ✅ No coupling between transfer and balance logic
- ✅ Aligns with RGB's UTXO-bound state model
- ✅ Clean balance queries (only check real UTXOs)
- ✅ Verifiable in consignment validation

## Additional Fixes Applied

### Fix 1: Wallet Persistence After Invoice Generation
**Problem:** After `reveal_next_address()` in invoice generation, the wallet wasn't persisted, causing the new address to not be saved to the database.

**Solution:** Added `wallet.persist()` call after `reveal_next_address()` in `src/f1r3fly/invoice.rs:84`.

### Fix 2: Address Range for Balance Query
**Problem:** Balance query used `derivation_index()` which only reflects persisted state. If a wallet generates an invoice and isn't reloaded, the invoice address isn't checked.

**Solution:** Changed balance query to check addresses 0-19 (external) and 0-9 (internal) instead of relying on `derivation_index()`. This ensures invoice addresses are found even if the wallet wasn't reloaded after invoice generation.

**Location:** `src/f1r3fly/balance.rs:259-275`

## Recommendation

**Short-term:** Keep Option A (current workaround) with the fixes above for MVP.

**Long-term:** Implement Option C (claim mechanism) with:
1. Extend `F1r3flyConsignment` to include witness identifier
2. Add `claim` method to RHO20 contract template
3. Auto-claim during `accept_consignment()`
4. Remove witness querying from `balance.rs`

## Testing Strategy

Current test (`test_complete_transfer_alice_to_bob`) verifies:
- ✅ Bob can query balance via witness identifier
- ✅ Balance is correct after transfer

Future test should verify:
- Claim mechanism migrates balance correctly
- Balance query only checks real UTXOs
- Multiple claims don't duplicate balance
- Claim fails for non-existent witness ID

## References

- Issue discussion: [this conversation]
- Related file: `f1r3fly-rgb-wallet/src/f1r3fly/balance.rs:303-349`
- Transfer logic: `f1r3fly-rgb-wallet/src/f1r3fly/transfer.rs:156-190`
- Test case: `f1r3fly-rgb-wallet/tests/f1r3fly/complete_transfer_test.rs`

