# Esplora Mock Server

A lightweight mock server that translates [Esplora API](https://github.com/Blockstream/esplora/blob/master/API.md) calls to Bitcoin Core RPC. Designed for Regtest testing and development.

## Features

- **Esplora-compatible API**: Drop-in replacement for Esplora endpoints
- **Bitcoin Core RPC backend**: Uses real Bitcoin Core for transaction data
- **Regtest helpers**: Additional endpoints for mining blocks during tests
- **Fast confirmations**: No waiting for real network blocks
- **EC2 deployable**: Can run on remote servers for demos

## Prerequisites

1. **Bitcoin Core** running in Regtest mode:
   ```bash
   bitcoind -regtest -daemon \
     -rpcuser=regtest \
     -rpcpassword=regtest \
     -rpcport=18443 \
     -fallbackfee=0.00001
   ```

2. **Create a wallet** (first time only):
   ```bash
   bitcoin-cli -regtest -rpcuser=regtest -rpcpassword=regtest \
     createwallet "test_wallet"
   ```

3. **Generate initial blocks** (for funding):
   ```bash
   bitcoin-cli -regtest -rpcuser=regtest -rpcpassword=regtest \
     -generate 101
   ```

## Configuration

Create a `.env` file or set environment variables:

```bash
# Required
BITCOIN_RPC_USER=regtest
BITCOIN_RPC_PASSWORD=regtest

# Optional (defaults shown)
BITCOIN_RPC_URL=http://localhost:18443
SERVER_HOST=0.0.0.0
SERVER_PORT=3000
```

## Running

```bash
# Development
cargo run

# Production
cargo build --release
./target/release/esplora-mock
```

The server will start on `http://0.0.0.0:3000` (or your configured port).

## API Endpoints

### Standard Esplora Endpoints

- `GET /blocks/tip/height` - Current blockchain height
- `GET /blocks/tip/hash` - Current block hash
- `GET /address/{address}/utxo` - UTXOs for an address
- `GET /tx/{txid}` - Transaction details
- `GET /tx/{txid}/status` - Transaction confirmation status
- `GET /tx/{txid}/raw` - Raw transaction hex
- `GET /tx/{txid}/outspend/{index}` - Output spending status
- `POST /tx` - Broadcast transaction (hex in body)

### Regtest Helper Endpoints

- `POST /regtest/mine` - Mine blocks instantly
  ```bash
  curl -X POST http://localhost:3000/regtest/mine \
    -H "Content-Type: application/json" \
    -d '{"count": 1}'
  ```

- `GET /health` - Health check

## Usage with Wallet

Update your wallet's `.env`:

```bash
BITCOIN_NETWORK=regtest
ESPLORA_URL=http://localhost:3000

# For EC2 deployment
# ESPLORA_URL=http://your-ec2-ip:3000
```

## Example: Mining Blocks from Tests

```rust
// In your integration test
async fn mine_block() -> Result<()> {
    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:3000/regtest/mine")
        .json(&serde_json::json!({ "count": 1 }))
        .send()
        .await?;
    
    let result: serde_json::Value = response.json().await?;
    println!("Mined to height: {}", result["new_height"]);
    Ok(())
}
```

## EC2 Deployment

1. **Launch EC2 instance** (Ubuntu 22.04 or later)

2. **Install Bitcoin Core**:
   ```bash
   sudo add-apt-repository ppa:luke-jr/bitcoincore
   sudo apt-get update
   sudo apt-get install bitcoind
   ```

3. **Configure Bitcoin Core** (`~/.bitcoin/bitcoin.conf`):
   ```
   regtest=1
   server=1
   rpcuser=regtest
   rpcpassword=YOUR_SECURE_PASSWORD
   rpcport=18443
   rpcallowip=127.0.0.1
   fallbackfee=0.00001
   ```

4. **Start Bitcoin Core**:
   ```bash
   bitcoind -daemon
   bitcoin-cli -regtest createwallet "test_wallet"
   bitcoin-cli -regtest -generate 101
   ```

5. **Build and run esplora-mock**:
   ```bash
   # Install Rust
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   
   # Clone and build
   cd /path/to/f1r3fly-rgb/esplora-mock
   cargo build --release
   
   # Run (with systemd or screen)
   BITCOIN_RPC_USER=regtest \
   BITCOIN_RPC_PASSWORD=YOUR_SECURE_PASSWORD \
   SERVER_HOST=0.0.0.0 \
   SERVER_PORT=3000 \
   ./target/release/esplora-mock
   ```

6. **Configure security group**: Allow inbound TCP on port 3000 from your IP

## Troubleshooting

**"Failed to connect to Bitcoin Core"**
- Ensure bitcoind is running: `bitcoin-cli -regtest getblockchaininfo`
- Check RPC credentials match `.env`
- Check RPC port (18443 for Regtest)

**"Address not found" errors**
- The mock server auto-imports addresses on first query
- May take a moment for RPC to index

**CORS errors in browser**
- Server has CORS enabled for all origins
- Check browser console for actual error

## Architecture

```
┌─────────────────────────────────────┐
│     Wallet / Integration Tests      │
│  (HTTP requests to mock)            │
└───────────────┬─────────────────────┘
                │ HTTP (Esplora API)
                ↓
┌─────────────────────────────────────┐
│      Esplora Mock Server            │
│  • Axum HTTP server                 │
│  • Translates Esplora → RPC         │
│  • Port 3000                        │
└───────────────┬─────────────────────┘
                │ JSON-RPC
                ↓
┌─────────────────────────────────────┐
│    Bitcoin Core (Regtest)           │
│  • localhost:18443                  │
│  • Fast block mining                │
└─────────────────────────────────────┘
```

## License

Same as parent project.

