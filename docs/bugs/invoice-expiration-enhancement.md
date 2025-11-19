# Invoice Expiration Support - Future Enhancement

**Status:** Not Implemented  
**Priority:** Low  
**Category:** Feature Enhancement

## Summary

RGB invoices currently do not support expiration timestamps in the F1r3fly-RGB wallet implementation. This enhancement would add the ability to generate and parse invoices with expiration times to prevent stale invoice reuse.

## Current Behavior

- Invoice generation accepts all parameters except expiration
- Invoices remain valid indefinitely once generated
- No timestamp validation occurs during payment processing

## Proposed Enhancement

Add expiration timestamp support to invoice operations:

1. **Generation**: Accept optional `expiration_secs` parameter (Unix timestamp)
2. **Parsing**: Extract and validate expiration timestamp
3. **Transfer**: Reject transfers to expired invoices

## Implementation Notes

- RGB invoice standard supports expiration via `expires` field in `RgbInvoice`
- Core library (`f1r3fly-rgb`) already has `RgbInvoice` with expiration support
- Wallet wrapper needs to expose this parameter
- Transfer validation should check expiration before executing

## Test Coverage

Current test verifies basic invoice generation without expiration:

**Test Location:** `f1r3fly-rgb-wallet/tests/f1r3fly/invoice_operations_test.rs`  
**Test Function:** `test_invoice_with_expiration()` (lines 257-319)

This test currently generates invoices without expiration and documents the limitation. When implementing this enhancement, update this test to:
- Generate invoice with actual expiration timestamp
- Verify expiration is present in parsed invoice
- Test expired invoice rejection in transfer flow

## Related Files

- `f1r3fly-rgb/src/invoice.rs` - Core invoice generation
- `f1r3fly-rgb-wallet/src/f1r3fly/invoice.rs` - Wallet wrapper
- `f1r3fly-rgb-wallet/src/f1r3fly/transfer.rs` - Transfer validation

