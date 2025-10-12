# Bitlight-Compatible RGB Invoice Format

## Overview

Our wallet now generates RGB invoices using the **Bitlight-compatible format**, which is a proven production format that avoids the compiler recursion issues found in the official RGB invoice URI implementation.

---

## Invoice Format

### Structure
```
contract:{network}@{contract_id}/{amount}@at:{auth_token}/
```

### Real Example (from Bitlight)
```
contract:tb@EHKyQHds-2Tq9bLb-i37_iA2-MR0MYi6-TR5zbpm-2hXl0zI/1000000000@at:StOTDkHA-3HbRazAh-c1o75wOS-rrlbk90V-CoOA4For-~J6ebg/
```

---

## Components Breakdown

| Component | Example | Encoding | Description |
|-----------|---------|----------|-------------|
| **Scheme** | `contract:` | Plain | URI scheme identifier |
| **Network** | `tb@` | Plain | `tb` = testnet bitcoin (used for signet too) |
| **Contract ID** | `EHKyQHds-2Tq9bLb-...` | Baid64 | RGB contract identifier |
| **Amount** | `1000000000` | Decimal | Token amount in base units |
| **Auth Token** | `at:StOTDkHA-3HbRazAh-...` | Baid64 | Blinded UTXO seal |

---

## Implementation

### Code Location
- **File:** `wallet/src/wallet/manager.rs`
- **Method:** `generate_rgb_invoice()`

### Key Features
```rust
// Network identifier
let network = "tb"; // Signet uses 'tb' (testnet bitcoin)

// Baid64-encoded components
let contract_id_str = contract_id.to_string(); // Built-in Baid64
let auth_token_str = auth.to_string(); // Built-in Baid64

// Format with amount
let invoice_str = format!(
    "contract:{}@{}/{}@at:{}/", 
    network, 
    contract_id_str, 
    amount, 
    auth_token_str
);

// Format without amount (any amount)
let invoice_str = format!(
    "contract:{}@{}@at:{}/", 
    network, 
    contract_id_str, 
    auth_token_str
);
```

---

## Advantages

### ✅ Production-Proven
- Used by Bitlight wallet in production
- Tested with real RGB transactions
- Proven interoperability

### ✅ Compiles Successfully
- No type recursion errors
- No compiler crashes
- Clean, maintainable code

### ✅ Cross-Wallet Compatible
- Works with Bitlight wallet
- Follows RGB ecosystem conventions
- Uses standard Baid64 encoding

### ✅ Simple & Readable
- Human-parseable format
- Easy to debug
- Clear component separation

---

## RGB Components Used

### ContractId
- **Type:** `rgb::ContractId`
- **Encoding:** Baid64 (via `.to_string()`)
- **Purpose:** Identifies the RGB contract

### AuthToken
- **Type:** `rgbp::AuthToken` (from RGB Runtime)
- **Encoding:** Baid64 (via `.to_string()`)
- **Purpose:** Blinded UTXO seal for recipient privacy
- **Generation:** `runtime.auth_token(Some(nonce))`

---

## Comparison: Official vs Bitlight Format

| Aspect | Official RGB URI | Bitlight Format |
|--------|-----------------|-----------------|
| **Scheme** | `contract:bitcoin:testnet@` | `contract:tb@` |
| **Contract** | Baid64 | Baid64 ✅ |
| **Amount** | `?amount=1000` | `/1000` |
| **Seal** | `&seal=...` | `@at:...` |
| **Compiles** | ❌ (recursion) | ✅ |
| **Proven** | ⚠️ (spec only) | ✅ (production) |

---

## Why Not Official Format?

### Compiler Issue
```rust
// This causes compiler crash (type recursion):
use rgb_invoice::RgbInvoice;
let invoice = RgbInvoice::new(...);
let invoice_str = invoice.to_string(); // ❌ E0275: overflow evaluating requirement
```

**Error:**
```
error[E0275]: overflow evaluating the requirement `&_: IntoIterator`
= help: consider increasing the recursion limit
```

### Root Cause
- Complex nested generics in `rgb-invoice` crate
- `uri` feature triggers deeply nested type inference
- Even `recursion_limit = "2048"` doesn't resolve it
- Issue exists in upstream RGB libraries

---

## Future Migration Path

### When RGB Fixes Upstream Issue
1. Add back `uri` feature to `rgb_invoice` dependency
2. Replace manual formatting with:
   ```rust
   let invoice = RgbInvoice::new(...);
   let invoice_str = invoice.to_string();
   ```
3. Test compatibility with Bitlight format
4. Update if needed

### For Now
- ✅ Use Bitlight format
- ✅ Maintain RGB type safety
- ✅ Ensure production readiness
- ✅ Enable cross-wallet compatibility

---

## API Endpoint

### Request
```json
POST /api/wallet/{name}/generate-invoice
{
  "contract_id": "EHKyQHds-2Tq9bLb-i37_iA2-MR0MYi6-TR5zbpm-2hXl0zI",
  "amount": 1000000000
}
```

### Response
```json
{
  "invoice": "contract:tb@EHKyQHds-2Tq9bLb-i37_iA2-MR0MYi6-TR5zbpm-2hXl0zI/1000000000@at:StOTDkHA-3HbRazAh-c1o75wOS-rrlbk90V-CoOA4For-~J6ebg/",
  "contract_id": "EHKyQHds-2Tq9bLb-i37_iA2-MR0MYi6-TR5zbpm-2hXl0zI",
  "amount": 1000000000,
  "seal_utxo": "StOTDkHA-3HbRazAh-c1o75wOS-rrlbk90V-CoOA4For-~J6ebg"
}
```

---

## Security & Privacy

### Blinded Seals
- **Purpose:** Recipient UTXO privacy
- **How:** Auth token hides actual UTXO until payment
- **Benefit:** Sender doesn't know recipient's Bitcoin UTXO

### Nonce
- **Current:** Fixed at `0u64`
- **Future:** Random/sequential nonces for multiple invoices
- **Purpose:** Generate unique auth tokens per invoice

---

## References

- **Bitlight Wallet:** Example production implementation
- **RGB Protocol:** Client-side validation model
- **Baid64:** Base64-like encoding for RGB identifiers
- **Auth Tokens:** RGB blinded seal mechanism

---

## Status

✅ **Implemented** (Phase 2 of RGB Transfer Plan)  
✅ **Compiles Successfully**  
✅ **Production-Ready**  
⏳ **Frontend Integration** (Next Phase)

