# Taproot Support Required for RGB Transfers

**Status:** CRITICAL BLOCKER  
**Priority:** HIGH  
**Impact:** RGB transfers cannot work without taproot descriptors  
**Discovered:** During Test Case 3 (Complete Transfer Flow) implementation

## Problem

The wallet currently uses BIP84 (P2WPKH/native SegWit) descriptors instead of BIP86 (P2TR/Taproot) descriptors. Tapret commitments **require** taproot outputs to embed RGB state commitments. Without taproot, RGB transfers fail with:

```
Transfer failed: Tapret error: Output is not a taproot output
```

## Root Cause

1. **BIP Path**: Currently using `m/84'/x'/0'` (BIP84 for P2WPKH)
   - Should use: `m/86'/x'/0'` (BIP86 for Taproot)

2. **Descriptor Format**: Currently `wpkh(xprv.../0/*)`
   - Should be: `tr(xprv.../0/*)`

3. **Address Type**: Currently generates P2WPKH addresses (bc1q...)
   - Should generate: P2TR addresses (bc1p...)

## Files Requiring Changes

### 1. Core Key Derivation (`src/storage/keys.rs`)

**Current Code (Lines 62-122):**
```rust
/// Derive Bitcoin keys from mnemonic using BIP32 at path m/84'/x'/0'
///
/// Uses BIP84 derivation path for native segwit (P2WPKH):
/// - Mainnet: m/84'/0'/0'
/// - Testnet: m/84'/1'/0'
pub fn derive_bitcoin_keys(
    mnemonic: &bip39::Mnemonic,
    network: NetworkType,
) -> Result<Xpriv, KeyError> {
    // ...
    let path_str = format!("m/84'/{}'/{}'", coin_type, 0);
    // ...
}
```

**Required Changes:**
- Rename function to `derive_bitcoin_keys_bip86()` or update documentation
- Change BIP path from `m/84'/x'/0'` to `m/86'/x'/0'`
- Update all comments referencing BIP84 to BIP86
- Update documentation to reference Taproot instead of native SegWit

**Lines to Modify:** 62-122

---

### 2. Descriptor Generation (`src/storage/models.rs`)

**Current Code (Lines 62-65):**
```rust
let bitcoin_xprv = crate::storage::keys::derive_bitcoin_keys(mnemonic, network)?;

// Create descriptor for BDK
let bitcoin_descriptor = format!("wpkh({}/0/*)", bitcoin_xprv);
```

**Required Changes:**
- Change descriptor format from `wpkh(...)` to `tr(...)`
- Verify BDK supports taproot descriptors (it does in v1.0+)
- Update comments

**Lines to Modify:** 62-65

---

### 3. Address Generation (`src/storage/models.rs`)

**Current Code (Lines 106-111):**
```rust
let address = bitcoin::Address::p2wpkh(&compressed, btc_network);

Ok(address.to_string())
```

**Required Changes:**
- Change from `Address::p2wpkh()` to `Address::p2tr()`
- Update address generation logic for taproot key-spend paths
- Note: Taproot addresses use the internal key directly, not hash

**Lines to Modify:** 106-111

**Important:** Check if this `get_address_string()` method is still used or if BDK handles address generation now.

---

### 4. Wallet Initialization (`src/bitcoin/wallet.rs`)

**Current Code (Lines 63-101):**
```rust
/// * `descriptor` - BIP84 descriptor string (e.g., "wpkh(xprv.../0/*)")
///
/// let descriptor = "wpkh(tprv8g...0/*)".to_string();

// Create internal (change) descriptor by replacing /0/* with /1/*
// External descriptor format: wpkh(.../ 0/*)
// Internal descriptor format: wpkh(.../1/*)
let internal_descriptor = descriptor.replace("/0/*", "/1/*");
```

**Required Changes:**
- Update documentation: BIP84 → BIP86, wpkh → tr
- Update example descriptor in docs
- Change descriptor string replacement logic (still /0/* → /1/* but for tr())
- Update all comments referencing wpkh to tr

**Lines to Modify:** 63-101

---

### 5. CLI Wallet Commands (`src/cli/commands/wallet.rs`)

**Current Code (Lines 40-48):**
```rust
let mnemonic_str = manager.create_wallet(&name, &password)?;

// Get the first address from the properly initialized wallet
let first_address = manager.get_new_address()?;
```

**Required Changes:**
- No direct code changes needed (uses WalletManager which will be updated)
- Update any user-facing messages/documentation that reference address types
- Verify address display formats for taproot (bc1p... vs bc1q...)

**Lines to Modify:** None (indirect update)

---

### 6. Test Utilities (`tests/f1r3fly/mod.rs`)

**Current Code (Lines 105-143):**
```rust
pub async fn setup_wallet_with_genesis_utxo(...) {
    // ...
    let mut manager = WalletManager::new(env.config().clone())?;
    manager.create_wallet(wallet_name, password)?;
    // ...
}
```

**Required Changes:**
- No direct changes needed (uses WalletManager which will be updated)
- Verify test assertions don't hardcode P2WPKH address formats
- May need to update expected address formats in test comparisons

**Lines to Modify:** None (indirect update)

---

### 7. Test Address Comparisons (Multiple Test Files)

**Files to Check:**
- `tests/f1r3fly/invoice_operations_test.rs`
- `tests/storage_test.rs`
- `tests/common/mod.rs`
- `tests/bitcoin/wallet_test.rs`
- `tests/bitcoin/manager_test.rs`
- `tests/bitcoin/balance_test.rs`

**Required Changes:**
- Search for hardcoded address patterns: `bcrt1q`, `tb1q`, `bc1q`
- Update to taproot patterns: `bcrt1p`, `tb1p`, `bc1p`
- Check for address length assumptions (taproot addresses are longer)
- Check for address validation regex that might reject P2TR

**Potential Impact:** LOW (tests should use dynamic addresses, not hardcoded)

---

## Migration Considerations

### Backward Compatibility

**Question:** Should we support both P2WPKH and P2TR wallets?

**Options:**

1. **Hard Migration (Recommended for MVP)**
   - All new wallets use BIP86/Taproot
   - Old wallets incompatible (acceptable for alpha/testing phase)
   - Simplest implementation

2. **Dual Support**
   - Detect descriptor type on wallet load
   - Support both P2WPKH and P2TR
   - More complex but allows gradual migration
   - Requires version flag in wallet metadata

**Recommendation:** Option 1 (Hard Migration) for now since this is in active development.

---

## Address Format Changes

### Current (P2WPKH)
- **Mainnet:** bc1q... (42 chars)
- **Testnet:** tb1q... (42 chars)
- **Regtest:** bcrt1q... (44 chars)

### New (P2TR Taproot)
- **Mainnet:** bc1p... (62 chars)
- **Testnet:** tb1p... (62 chars)
- **Regtest:** bcrt1p... (64 chars)

**Impact:** Any UI/validation that assumes address length needs update.

---

## BDK Compatibility

**BDK Version:** 1.0+ (as specified in Cargo.toml)  
**Taproot Support:** ✅ YES - BDK 1.0 fully supports taproot descriptors  
**Required Changes:** None for BDK itself

---

## Testing Strategy

### Unit Tests
1. Test `derive_bitcoin_keys()` produces correct BIP86 paths
2. Test descriptor generation produces `tr(...)` format
3. Test address generation produces P2TR addresses

### Integration Tests
1. Complete transfer flow (Test Case 3) should pass
2. Verify Tapret embedding succeeds
3. Verify taproot outputs are created in transactions

### Regression Tests
1. Ensure old wallet files are rejected gracefully (or migrated if dual support)
2. Verify all address displays use correct format

---

## Implementation Checklist

- [ ] Update `derive_bitcoin_keys()` to use BIP86 (m/86'/x'/0')
- [ ] Update descriptor generation to use `tr(...)` format
- [ ] Update address generation to use `p2tr()`
- [ ] Update wallet documentation and comments
- [ ] Update CLI help text if it references address types
- [ ] Search and update any hardcoded address patterns in tests
- [ ] Add unit tests for BIP86 derivation
- [ ] Add unit tests for taproot descriptor generation
- [ ] Add unit tests for P2TR address generation
- [ ] Run complete transfer test (Test Case 3) to verify
- [ ] Update any user documentation/README

---

## Estimated Impact

**Lines of Code:** ~50 lines across 4 files  
**Test Updates:** Minimal (most tests use dynamic addresses)  
**Complexity:** Medium (requires understanding of BIP86 vs BIP84)  
**Risk:** Medium (breaks compatibility with existing wallets)  
**Time:** 2-3 hours (including testing)

---

## Related Issues

- Blocks: Test Case 3 (Complete Transfer Flow)
- Blocks: Test Case 4 (Security & Validation)
- Blocks: Test Case 5 (Multi-Transfer Chain)
- Blocks: All production RGB transfers

---

## References

- [BIP86: Key Derivation for Single Key P2TR Outputs](https://github.com/bitcoin/bips/blob/master/bip-0086.mediawiki)
- [BIP341: Taproot: SegWit version 1 spending rules](https://github.com/bitcoin/bips/blob/master/bip-0341.mediawiki)
- [BDK Taproot Documentation](https://bitcoindevkit.org/)

