# F1r3fly-RGB Invoice Core API Implementation Tasks

**Date:** November 18, 2025  
**Status:** PENDING APPROVAL - NOT STARTED  
**Architecture:** Core API in `f1r3fly-rgb`, Thin Wallet Layer in `f1r3fly-rgb-wallet`  
**Related:** Phase 3 (Day 29-30) of `f1r3fly-rgb-wallet-implementation-plan.md`

---

## Overview

This implementation moves all RGB invoice logic into the `f1r3fly-rgb` core library, making it the authoritative API for invoice generation, parsing, and seal management. The wallet becomes a thin layer that handles Bitcoin integration (BDK) and user interface concerns only.

### Design Principles

1. **f1r3fly-rgb = RGB Protocol Logic**
   - Invoice generation/parsing
   - Seal conversions
   - Transfer orchestration
   - All RGB standard types

2. **f1r3fly-rgb-wallet = User Interface**
   - Bitcoin wallet (BDK)
   - CLI commands
   - Output formatting
   - Thin wrappers

---

## Task Breakdown

### Phase 1: Core Library (f1r3fly-rgb)

#### Task 1.1: Update Dependencies
**File:** `f1r3fly-rgb/Cargo.toml`

**Changes:**
```toml
[dependencies]
# Add:
rgb-invoice = { version = "0.12.0-rc.3", features = ["bitcoin"] }
bitcoin = { version = "0.32", features = ["serde"] }
```

**Validation:**
- `cargo build` in f1r3fly-rgb succeeds
- No version conflicts
- All existing tests still pass

**Estimated Time:** 5 minutes

---

#### Task 1.2: Create Invoice Module Structure
**File:** `f1r3fly-rgb/src/invoice.rs` (NEW)

**Implement:**
1. Module documentation
2. Error types (delegate to `F1r3flyRgbError` or create `InvoiceError`)
3. Data structures:
   - `InvoiceRequest`
   - `GeneratedInvoice`
   - `ParsedInvoice`
4. Import statements

**Code Structure:**
```rust
//! RGB Invoice Generation and Parsing (Production)
//!
//! Core invoice functionality for F1r3fly-RGB smart contracts.

use std::str::FromStr;
use bp_std::{AddressPayload, ScriptBytes, ScriptPubkey};
use rgb_invoice::{RgbBeneficiary, RgbInvoice};
use hypersonic::ContractId;
use strict_types::StrictVal;
use crate::{WTxoSeal, F1r3flyRgbError};

// Structs: InvoiceRequest, GeneratedInvoice, ParsedInvoice
// ...
```

**Validation:**
- Module compiles
- No lint warnings
- Documentation comments are clear

**Estimated Time:** 15 minutes

---

#### Task 1.3: Implement Core Invoice Generation
**File:** `f1r3fly-rgb/src/invoice.rs`

**Implement:**
```rust
pub fn generate_invoice(
    contract_id: ContractId,
    amount: u64,
    address: bitcoin::Address,
    nonce: u64,
    consensus: rgb::Consensus,
    testnet: bool,
) -> Result<GeneratedInvoice, F1r3flyRgbError>
```

**Logic:**
1. Validate amount > 0
2. Convert `bitcoin::Address` → `AddressPayload`
3. Create `WitnessOut` beneficiary
4. Create `WTxoSeal` for tracking
5. Create `RgbInvoice` with `StrictVal::num(amount)`
6. Return `GeneratedInvoice`

**Validation:**
- Function compiles
- Unit test: valid inputs produce valid invoice
- Unit test: zero amount returns error
- Invoice string starts with "rgb:"

**Estimated Time:** 30 minutes

---

#### Task 1.4: Implement Invoice Parsing
**File:** `f1r3fly-rgb/src/invoice.rs`

**Implement:**
```rust
pub fn parse_invoice(invoice_str: &str) 
    -> Result<ParsedInvoice, F1r3flyRgbError>
```

**Logic:**
1. Parse using `RgbInvoice::from_str()`
2. Extract `contract_id` from `invoice.scope`
3. Extract `amount` from `invoice.amount` (handle `StrictVal`)
4. Extract `beneficiary` from `invoice.auth`
5. Return `ParsedInvoice`

**Validation:**
- Function compiles
- Unit test: valid invoice string parses correctly
- Unit test: invalid format returns error
- Contract ID and amount extracted correctly

**Estimated Time:** 20 minutes

---

#### Task 1.5: Implement Seal Extraction
**File:** `f1r3fly-rgb/src/invoice.rs`

**Implement:**
```rust
pub fn extract_seal(beneficiary: &RgbBeneficiary) 
    -> Result<WTxoSeal, F1r3flyRgbError>
```

**Logic:**
1. Match on beneficiary type
2. If `WitnessOut`: create `WTxoSeal` pointing to output 0
3. If `Token`: return error (not supported in Phase 3)

**Validation:**
- Function compiles
- Unit test: WitnessOut beneficiary produces valid WTxoSeal
- Unit test: AuthToken beneficiary returns error
- Seal structure: `primary = Wout(0)`, `secondary = Noise`

**Estimated Time:** 15 minutes

---

#### Task 1.6: Implement Address Extraction
**File:** `f1r3fly-rgb/src/invoice.rs`

**Implement:**
```rust
pub fn get_recipient_address(
    beneficiary: &RgbBeneficiary,
    network: bitcoin::Network,
) -> Result<String, F1r3flyRgbError>
```

**Logic:**
1. Match on beneficiary type
2. If `WitnessOut`: convert `AddressPayload` → `bitcoin::Address`
3. Return address string
4. If `Token`: return error

**Validation:**
- Function compiles
- Unit test: WitnessOut converts to valid Bitcoin address
- Address format matches network (bcrt1q... for regtest)

**Estimated Time:** 20 minutes

---

#### Task 1.7: Implement Helper Functions
**File:** `f1r3fly-rgb/src/invoice.rs`

**Implement:**
```rust
fn address_to_beneficiary(
    address: &bitcoin::Address,
    nonce: u64,
) -> Result<RgbBeneficiary, F1r3flyRgbError>

fn beneficiary_to_seal(
    beneficiary: &RgbBeneficiary
) -> Result<WTxoSeal, F1r3flyRgbError>
```

**Logic:**
- `address_to_beneficiary`: Convert Bitcoin address to `AddressPayload`, create `WitnessOut`
- `beneficiary_to_seal`: Convert `RgbBeneficiary` to `WTxoSeal`

**Validation:**
- Functions compile
- Used internally by public API
- No direct unit tests needed (covered by public API tests)

**Estimated Time:** 15 minutes

---

#### Task 1.8: Add Unit Tests
**File:** `f1r3fly-rgb/src/invoice.rs` (tests module)

**Implement:**
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_generate_and_parse_invoice_roundtrip()
    
    #[test]
    fn test_generate_invoice_zero_amount_fails()
    
    #[test]
    fn test_extract_seal_from_witness_out()
    
    #[test]
    fn test_extract_seal_rejects_auth_token()
    
    #[test]
    fn test_get_recipient_address()
}
```

**Validation:**
- All tests pass
- Code coverage > 80% for invoice module
- `cargo test invoice` succeeds

**Estimated Time:** 30 minutes

---

#### Task 1.9: Update Module Exports
**File:** `f1r3fly-rgb/src/lib.rs`

**Changes:**
```rust
// Add module
pub mod invoice;

// Re-export public API
pub use invoice::{
    generate_invoice,
    parse_invoice,
    extract_seal,
    get_recipient_address,
    InvoiceRequest,
    GeneratedInvoice,
    ParsedInvoice,
};

// Re-export RGB types
pub use rgb_invoice::{RgbBeneficiary, RgbInvoice};
#[cfg(feature = "bitcoin")]
pub use rgb_invoice::bp::WitnessOut;
```

**Validation:**
- `cargo build` succeeds
- `cargo doc` generates documentation
- Public API is accessible from crate root

**Estimated Time:** 10 minutes

---

#### Task 1.10: Verify Core Library
**Location:** `f1r3fly-rgb/`

**Actions:**
```bash
# Build
cargo build --all-features

# Run all tests
cargo test

# Check documentation
cargo doc --no-deps --open

# Check for warnings
cargo clippy -- -D warnings
```

**Validation:**
- No compilation errors
- All tests pass (existing + new)
- No clippy warnings
- Documentation is clear and complete

**Estimated Time:** 15 minutes

---

### Phase 2: Wallet Layer (f1r3fly-rgb-wallet)

#### Task 2.1: Update Wallet Dependencies
**File:** `f1r3fly-rgb-wallet/Cargo.toml`

**Changes:**
```toml
[dependencies]
# Remove:
# base64 = "0.22"    ← DELETE
# blake2 = "0.10"    ← DELETE

# f1r3fly-rgb already re-exports rgb-invoice, so no need to add it here
```

**Validation:**
- `cargo build` succeeds (may have errors due to removed deps, that's okay)
- Dependencies cleaned up

**Estimated Time:** 5 minutes

---

#### Task 2.2: Backup Current Invoice Module
**File:** `f1r3fly-rgb-wallet/src/f1r3fly/invoice.rs`

**Actions:**
```bash
cp f1r3fly-rgb-wallet/src/f1r3fly/invoice.rs \
   f1r3fly-rgb-wallet/src/f1r3fly/invoice.rs.backup
```

**Validation:**
- Backup file created
- Can restore if needed

**Estimated Time:** 1 minute

---

#### Task 2.3: Rewrite Wallet Invoice Module
**File:** `f1r3fly-rgb-wallet/src/f1r3fly/invoice.rs`

**Implement:**
1. Module documentation (wallet-specific)
2. Wallet error type: `InvoiceError`
3. Wrapper function: `generate_invoice()` (calls core library)
4. Wrapper function: `parse_invoice()` (passthrough)
5. Helper: `NetworkType::to_bitcoin_network()`
6. Helper: `NetworkType::is_testnet()`

**Code Structure:**
```rust
//! Invoice generation wrapper for wallet
//!
//! Thin layer over f1r3fly-rgb core invoice functionality.

use f1r3fly_rgb::invoice::{
    generate_invoice as core_generate, 
    parse_invoice as core_parse
};
use f1r3fly_rgb::{GeneratedInvoice, ParsedInvoice};
use crate::bitcoin::BitcoinWallet;
use crate::config::NetworkType;

// Wallet-specific error type
#[derive(Debug, thiserror::Error)]
pub enum InvoiceError { /* ... */ }

// Thin wrappers
pub fn generate_invoice( /* ... */ ) -> Result<GeneratedInvoice, InvoiceError>
pub fn parse_invoice( /* ... */ ) -> Result<ParsedInvoice, InvoiceError>
```

**Key Points:**
- **No RGB logic here** (all delegated to core library)
- Only handles: Bitcoin wallet integration, address selection, error conversion
- ~100 lines vs ~400 lines before

**Validation:**
- Module compiles
- All wallet-specific concerns handled
- No duplication of RGB logic

**Estimated Time:** 30 minutes

---

#### Task 2.4: Update Wallet Module Exports
**File:** `f1r3fly-rgb-wallet/src/f1r3fly/mod.rs`

**Changes:**
```rust
pub mod invoice;

// Re-export wallet wrappers
pub use invoice::{
    generate_invoice,
    parse_invoice,
    InvoiceError,
};

// Can also re-export core types for convenience
pub use f1r3fly_rgb::{
    GeneratedInvoice,
    ParsedInvoice,
    RgbBeneficiary,
};
```

**Validation:**
- Wallet code can access functions via `crate::f1r3fly::invoice::*`
- `cargo build` progresses (may have CLI errors, that's next)

**Estimated Time:** 5 minutes

---

#### Task 2.5: Create CLI Invoice Commands
**File:** `f1r3fly-rgb-wallet/src/cli/commands/invoice.rs` (NEW)

**Implement:**
```rust
/// Generate invoice CLI command
pub async fn generate_invoice_cmd(
    manager: &WalletManager,
    contract_id: &str,
    amount: u64,
    address: Option<String>,
    format: &OutputFormat,
) -> Result<(), Box<dyn std::error::Error>>

/// Parse invoice CLI command
pub async fn parse_invoice_cmd(
    invoice_str: &str,
    format: &OutputFormat,
) -> Result<(), Box<dyn std::error::Error>>
```

**Key Points:**
- Calls wallet wrapper functions
- Handles output formatting (JSON, table, compact)
- User-facing error messages

**Validation:**
- Commands compile
- Format logic is clean
- No RGB logic in CLI layer

**Estimated Time:** 30 minutes

---

#### Task 2.6: Update CLI Module Exports
**File:** `f1r3fly-rgb-wallet/src/cli/commands/mod.rs`

**Changes:**
```rust
pub mod invoice;

pub use invoice::{
    generate_invoice_cmd,
    parse_invoice_cmd,
};
```

**Validation:**
- CLI commands accessible
- `cargo build` progresses

**Estimated Time:** 2 minutes

---

#### Task 2.7: Add CLI Command Enum Variants
**File:** `f1r3fly-rgb-wallet/src/cli/mod.rs`

**Changes:**
```rust
pub enum CliCommand {
    // ... existing commands ...
    
    GenerateInvoice {
        contract_id: String,
        amount: u64,
        address: Option<String>,
        format: OutputFormat,
    },
    
    ParseInvoice {
        invoice: String,
        format: OutputFormat,
    },
}
```

**Validation:**
- Enum variants added
- Command structure matches function signatures

**Estimated Time:** 5 minutes

---

#### Task 2.8: Wire Up CLI Command Parsing
**File:** `f1r3fly-rgb-wallet/src/main.rs` (or CLI parser location)

**Changes:**
```rust
match command {
    // ... existing command handlers ...
    
    CliCommand::GenerateInvoice { contract_id, amount, address, format } => {
        generate_invoice_cmd(&manager, &contract_id, amount, address, &format).await?;
    }
    
    CliCommand::ParseInvoice { invoice, format } => {
        parse_invoice_cmd(&invoice, &format).await?;
    }
}
```

**Also add argument parsing:**
```rust
// Using clap or similar
.subcommand(
    Command::new("generate-invoice")
        .about("Generate RGB invoice for receiving assets")
        .arg(Arg::new("contract-id").required(true))
        .arg(Arg::new("amount").required(true))
        .arg(Arg::new("address").required(false))
        .arg(Arg::new("format").short('f').default_value("table"))
)
.subcommand(
    Command::new("parse-invoice")
        .about("Parse RGB invoice string")
        .arg(Arg::new("invoice").required(true))
        .arg(Arg::new("format").short('f').default_value("table"))
)
```

**Validation:**
- Arguments parse correctly
- Commands route to correct functions

**Estimated Time:** 20 minutes

---

#### Task 2.9: Build and Fix Compilation Errors
**Location:** `f1r3fly-rgb-wallet/`

**Actions:**
```bash
cd f1r3fly-rgb-wallet
cargo build 2>&1 | grep -E "(error|warning)" | head -30
```

**Fix any errors:**
- Missing imports
- Type mismatches
- Unused imports from old implementation
- Function signature mismatches

**Validation:**
- `cargo build` succeeds
- No compilation errors

**Estimated Time:** 20 minutes

---

#### Task 2.10: Clean Up Deprecated Code
**Files:**
- Remove backup: `src/f1r3fly/invoice.rs.backup` (if no longer needed)
- Check for any other files referencing old invoice format

**Validation:**
- No references to Blake2b or Base64 invoice format
- Codebase is clean

**Estimated Time:** 5 minutes

---

### Phase 3: Testing

#### Task 3.1: Run Core Library Tests
**Location:** `f1r3fly-rgb/`

**Actions:**
```bash
cargo test invoice
cargo test --all
```

**Validation:**
- All new invoice tests pass
- All existing tests still pass
- No test regressions

**Estimated Time:** 5 minutes

---

#### Task 3.2: Create Wallet Integration Tests
**File:** `f1r3fly-rgb-wallet/tests/invoice_integration_test.rs` (NEW)

**Implement:**
```rust
#[tokio::test]
async fn test_generate_invoice_with_wallet() {
    // Setup test environment
    // Generate invoice using wallet
    // Verify invoice format
    // Verify address from wallet
}

#[tokio::test]
async fn test_parse_invoice_roundtrip() {
    // Generate invoice
    // Parse invoice
    // Verify contract ID, amount match
}

#[tokio::test]
async fn test_invoice_format_is_rgb_standard() {
    // Generate invoice
    // Verify format: "rgb:tb...@contract:..."
    // Verify NOT Base64 JSON format
}
```

**Validation:**
- All integration tests pass
- Tests cover wallet-specific concerns (address generation)

**Estimated Time:** 30 minutes

---

#### Task 3.3: Add CLI Tests to test_cli.sh
**File:** `f1r3fly-rgb-wallet/test_cli.sh`

**Add:**
```bash
# Test 13: Generate Invoice
echo "=== Test 13: Generate Invoice ==="
CONTRACT_ID="contract:xFS2u..."  # Use actual contract from test 12
AMOUNT=100

INVOICE_OUTPUT=$($BIN generate-invoice \
    --contract-id "$CONTRACT_ID" \
    --amount $AMOUNT \
    --format json 2>&1)

# Validate invoice format
INVOICE_STRING=$(echo "$INVOICE_OUTPUT" | jq -r '.invoice')
if [[ ! "$INVOICE_STRING" =~ ^rgb: ]]; then
    echo "❌ FAILED: Invoice missing rgb: prefix"
    echo "Got: $INVOICE_STRING"
    exit 1
fi

echo "✅ Invoice generated: ${INVOICE_STRING:0:50}..."

# Test 14: Parse Invoice
echo "=== Test 14: Parse Invoice ==="
PARSED=$($BIN parse-invoice --invoice "$INVOICE_STRING" --format json 2>&1)

PARSED_CONTRACT=$(echo "$PARSED" | jq -r '.contract_id')
PARSED_AMOUNT=$(echo "$PARSED" | jq -r '.amount')

if [ "$PARSED_AMOUNT" != "$AMOUNT" ]; then
    echo "❌ FAILED: Amount mismatch (expected: $AMOUNT, got: $PARSED_AMOUNT)"
    exit 1
fi

echo "✅ Invoice parsed correctly"
```

**Validation:**
- Tests pass when run manually
- Invoice format is RGB standard (not Base64 JSON)

**Estimated Time:** 20 minutes

---

#### Task 3.4: Run All Wallet Tests
**Location:** `f1r3fly-rgb-wallet/`

**Actions:**
```bash
# Unit tests
cargo test

# Integration tests
cargo test --test invoice_integration_test

# CLI tests
./test_cli.sh
```

**Validation:**
- All tests pass
- No regressions in existing tests
- New invoice tests pass

**Estimated Time:** 10 minutes

---

#### Task 3.5: Manual Testing
**Actions:**
1. Start regtest environment
2. Create wallet
3. Issue asset (Test 12 from test_cli.sh)
4. Generate invoice:
   ```bash
   ./target/debug/f1r3fly-rgb-wallet generate-invoice \
       --contract-id "$CONTRACT_ID" \
       --amount 100
   ```
5. Verify invoice format: `rgb:tb1q...@contract:...`
6. Parse invoice:
   ```bash
   ./target/debug/f1r3fly-rgb-wallet parse-invoice \
       --invoice "$INVOICE_STRING"
   ```
7. Verify parsed data matches

**Validation:**
- Commands work end-to-end
- Invoice string is human-readable
- Format matches RGB standard

**Estimated Time:** 15 minutes

---

### Phase 4: Documentation

#### Task 4.1: Update Implementation Plan
**File:** `docs/plans/f1r3fly-rgb-wallet-implementation-plan.md`

**Changes:**
- Mark Day 29-30 (Invoice Generation) as **COMPLETE**
- Add note: "Using WitnessOut beneficiaries (RGB standard)"
- Add note: "Core API in f1r3fly-rgb, thin wallet layer"
- Update Phase 3 summary

**Validation:**
- Plan accurately reflects current state
- Next tasks (Day 31-33: Transfer) are clear

**Estimated Time:** 10 minutes

---

#### Task 4.2: Create Invoice User Guide
**File:** `docs/invoice-user-guide.md` (NEW)

**Contents:**
1. What are RGB invoices?
2. How to generate an invoice (CLI examples)
3. How to parse an invoice (CLI examples)
4. Invoice format explanation
5. Security considerations
6. Troubleshooting

**Validation:**
- Guide is clear for end users
- Examples are accurate

**Estimated Time:** 20 minutes

---

#### Task 4.3: Update Core Library Documentation
**File:** `f1r3fly-rgb/README.md` (if exists) or `f1r3fly-rgb/src/lib.rs`

**Add:**
- Invoice API section
- Example usage
- Link to wallet implementation

**Validation:**
- Documentation is clear for library users
- Examples compile (in doc tests)

**Estimated Time:** 15 minutes

---

#### Task 4.4: Generate and Review Documentation
**Actions:**
```bash
# Core library docs
cd f1r3fly-rgb
cargo doc --no-deps --open

# Wallet docs
cd ../f1r3fly-rgb-wallet
cargo doc --no-deps --open
```

**Review:**
- All public functions documented
- Examples are clear
- No broken links

**Validation:**
- Documentation is complete
- Follows Rust documentation best practices

**Estimated Time:** 10 minutes

---

### Phase 5: Cleanup and Verification

#### Task 5.1: Run Full Test Suite
**Actions:**
```bash
# Core library
cd f1r3fly-rgb
cargo test --all

# Wallet
cd ../f1r3fly-rgb-wallet
cargo test --all
./test_cli.sh
```

**Validation:**
- All tests pass
- No flaky tests
- Test output is clean

**Estimated Time:** 10 minutes

---

#### Task 5.2: Run Linters
**Actions:**
```bash
# Core library
cd f1r3fly-rgb
cargo clippy -- -D warnings
cargo fmt --check

# Wallet
cd ../f1r3fly-rgb-wallet
cargo clippy -- -D warnings
cargo fmt --check
```

**Fix any issues:**
- Clippy warnings
- Formatting issues
- Unused imports

**Validation:**
- No warnings
- Code is formatted

**Estimated Time:** 15 minutes

---

#### Task 5.3: Verify Interoperability
**Manual Check:**
1. Generate invoice with our wallet
2. Verify format matches RGB standard: `rgb:tb1q...@contract:...`
3. Compare with traditional RGB invoice format (from docs)
4. Verify structure:
   - Beneficiary: WitnessOut (not custom format)
   - Seal: WTxoSeal (RGB standard)
   - Contract ID: Standard format

**Validation:**
- Invoice format is 100% RGB standard
- No custom/proprietary elements
- Can theoretically be parsed by other RGB wallets

**Estimated Time:** 10 minutes

---

#### Task 5.4: Final Build Verification
**Actions:**
```bash
# Clean build
cd f1r3fly-rgb
cargo clean
cargo build --release

cd ../f1r3fly-rgb-wallet
cargo clean
cargo build --release
```

**Validation:**
- Release builds succeed
- Binary size is reasonable
- No release-only errors

**Estimated Time:** 5 minutes

---

#### Task 5.5: Commit Changes
**Actions:**
```bash
# Stage changes
git add f1r3fly-rgb/
git add f1r3fly-rgb-wallet/
git add docs/

# Review changes
git status
git diff --staged

# Commit
git commit -m "feat: Implement RGB invoice core API

- Add invoice module to f1r3fly-rgb with full RGB standard support
- Implement WitnessOut beneficiary generation and parsing
- Add seal extraction and address conversion utilities
- Rewrite wallet invoice layer as thin wrapper over core API
- Add CLI commands: generate-invoice, parse-invoice
- Add comprehensive tests (unit + integration + CLI)
- Remove custom Base64 JSON format in favor of RGB standard
- Update documentation

Closes: Phase 3 Day 29-30 (Invoice Generation)
"
```

**Validation:**
- Commit message is clear
- All relevant files staged
- No unintended changes

**Estimated Time:** 5 minutes

---

## Summary

### Total Estimated Time
- **Phase 1 (Core Library):** ~3 hours
- **Phase 2 (Wallet Layer):** ~2 hours
- **Phase 3 (Testing):** ~1.5 hours
- **Phase 4 (Documentation):** ~1 hour
- **Phase 5 (Cleanup):** ~1 hour

**Total: ~8.5 hours** (can be done in 1-2 sessions)

---

### Task Execution Order

**Session 1 (Core API):**
1. Task 1.1 → 1.10 (Complete core library)
2. Verify with tests

**Session 2 (Wallet Integration):**
1. Task 2.1 → 2.10 (Rewrite wallet layer)
2. Task 3.1 → 3.5 (Run all tests)

**Session 3 (Polish):**
1. Task 4.1 → 4.4 (Documentation)
2. Task 5.1 → 5.5 (Cleanup and commit)

---

### Key Deliverables

1. ✅ `f1r3fly-rgb/src/invoice.rs` - Core invoice API (~300 lines)
2. ✅ `f1r3fly-rgb-wallet/src/f1r3fly/invoice.rs` - Wallet wrapper (~100 lines)
3. ✅ `f1r3fly-rgb-wallet/src/cli/commands/invoice.rs` - CLI commands (~150 lines)
4. ✅ Unit tests in core library (5 tests)
5. ✅ Integration tests in wallet (3 tests)
6. ✅ CLI tests in test_cli.sh (2 tests)
7. ✅ Documentation updates
8. ✅ RGB standard compliance (100%)

---

### Success Criteria

- [ ] Core library provides complete invoice API
- [ ] Wallet is thin layer (no RGB logic duplication)
- [ ] Invoice format is RGB standard (`rgb:tb...@contract:...`)
- [ ] All tests pass (unit + integration + CLI)
- [ ] Documentation is complete
- [ ] No clippy warnings
- [ ] Code is formatted
- [ ] Interoperable with RGB ecosystem

---

## Next Steps After Approval

Once approved, I will proceed with:

1. **Task 1.1** (Update core library dependencies)
2. Execute tasks sequentially
3. Report progress after each major phase
4. Address any issues immediately
5. Complete all tasks in order

**Ready to begin upon your approval.**

---

**END OF TASK PLAN**

