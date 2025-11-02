// F1r3fly/RSpace Integration Tests
// Tests for FireflyClient interactions with F1r3fly node
//
// Scope:
// - F1r3fly/Rholang contract deployment and state management
// - RGB contract metadata, allocations, and transitions (storage/retrieval)
// - Client-side validation logic (balance checks, token conservation)
// - Secondary index patterns (ticker search)
//
// Bitcoin Interaction:
// - Bitcoin transactions are MOCKED (fake txids like "aaaa...aaa")
// - Bitcoin confirmations are ASSUMED (no real Bitcoin node required)
// - Focus is on F1r3fly state management, not Bitcoin validation
//
// For tests requiring real Bitcoin validation, see:
// - test_bitcoin_validator.rs (Bitcoin validation logic)
// - test_e2e_phase0.rs (F1r3fly + Bitcoin integration)
// - rgb_transfer_balance_test.rs (Full RGB workflow)
//
// Prerequisites:
// - F1r3fly node must be running locally
// - Configure via environment variables:
//   - FIREFLY_TEST_HOST (default: localhost)
//   - FIREFLY_TEST_GRPC_PORT (default: 40401)
//   - FIREFLY_TEST_HTTP_PORT (default: 40403)

pub mod test_contract_metadata;
pub mod test_allocations;
pub mod test_transitions;

