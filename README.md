# f1r3fly-rgb

High-level abstractions for RGB-like token operations using F1r3fly for state management and Bitcoin for anchoring.

## What It Does

`f1r3fly-rgb` combines F1r3fly's Rholang execution with RGB's UTXO-based token model:

- **High-Level APIs**: `F1r3flyRgbContract` and `F1r3flyRgbContracts` for simplified token operations (issue, transfer, balance)
- **State Management**: Token balances stored in F1r3fly shards (persistent, queryable)
- **Bitcoin Anchoring**: Cryptographic proofs embedded in Bitcoin transactions via Tapret commitments
- **Transfer Packages**: Lightweight consignments for asset transfers with F1r3fly state proofs
- **Low-Level Primitives**: `F1r3flyExecutor`, `BitcoinAnchorTracker`, and Tapret utilities for custom workflows

Unlike traditional RGB (client-side validation), f1r3fly-rgb uses F1r3fly for state coordination while maintaining Bitcoin's censorship resistance for finality.

## Running Tests

### Prerequisites

All tests require:
- Running F1r3node instance
- Environment variables set in `.env` (see `.env.example`)

### Unit Tests

```bash
cargo test --lib
```

### Integration Tests

```bash
# Run all integration tests (requires fresh F1r3node)
cargo test --test '*' -- --test-threads=1

# Run specific test suite
cargo test --test contract_test -- --test-threads=1
cargo test --test contracts_test
cargo test --test bitcoin_anchor_test
cargo test --test consignment_test
cargo test --test f1r3fly_executor_test -- --test-threads=1
```

**Note**: `contract_test` and `f1r3fly_executor_test` must run sequentially (`--test-threads=1`).

### Test Categories

- **Contract Tests**: High-level API (`F1r3flyRgbContract`, `F1r3flyRgbContracts`)
- **Executor Tests**: Low-level F1r3fly execution and state queries
- **Consignment Tests**: Transfer package creation and validation
- **Anchor Tests**: Bitcoin witness tracking (`BitcoinAnchorTracker`)