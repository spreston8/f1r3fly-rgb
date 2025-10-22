# RGB-Compatible Bitcoin Wallet

A Bitcoin Signet wallet with RGB compatibility, starting with basic wallet operations and balance tracking.

## Usage

```bash
# Start server (production)
cargo run --release

# Start server (development with debug logs)
RUST_LOG=wallet=debug cargo run
```

## Debug Logging

Enable detailed logging for development and troubleshooting:

```bash
# Start server
cargo run --release
```

## Network
Currently Signet only.

## Storage
Wallet data stored in `./wallets/{name}/`

