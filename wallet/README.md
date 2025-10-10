# RGB-Compatible Bitcoin Wallet

A Bitcoin Signet wallet with RGB compatibility, starting with basic wallet operations and balance tracking.

## Phase 1: Bitcoin Wallet Foundation

### Features
- BIP39 mnemonic generation (24 words)
- BIP32 HD key derivation (m/84'/1'/0' for Signet)
- P2WPKH address generation
- Balance checking via Esplora API
- Local file storage
- HTTP REST API

### Usage

```bash
# Start server
cargo run --release

# Create wallet
curl -X POST http://localhost:3000/api/wallet/create \
  -H "Content-Type: application/json" \
  -d '{"name":"my_wallet"}'

# Get addresses
curl http://localhost:3000/api/wallet/my_wallet/addresses?count=5

# Check balance
curl http://localhost:3000/api/wallet/my_wallet/balance

# Sync wallet
curl -X POST http://localhost:3000/api/wallet/my_wallet/sync
```

### Network
Currently Signet only.

### Storage
Wallet data stored in `./wallets/{name}/`

