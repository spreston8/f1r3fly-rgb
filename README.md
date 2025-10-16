# F1r3fly-RGB

A Bitcoin Signet wallet with RGB compatibility, featuring a Rust backend and modern React frontend.

## Setup

Clone the project:
```bash
git clone <repository-url>
cd f1r3fly-rgb
```

Initialize submodules:
```bash
git submodule update --init --recursive
```

Update submodules:
```bash
git submodule update --remote
```

## Components

### [Wallet Backend](./wallet/README.md)
RGB-compatible Bitcoin Signet wallet backend service.

**Quick Start:**
```bash
cd wallet
cargo run --release
```
Server runs on `http://localhost:3000`

See [wallet/README.md](./wallet/README.md) for detailed documentation.

### [Wallet Frontend](./wallet-frontend/README.md)
Modern React UI for interacting with the wallet backend.

**Quick Start:**
```bash
cd wallet-frontend
npm install
npm run dev
```
Frontend runs on `http://localhost:5173`

See [wallet-frontend/README.md](./wallet-frontend/README.md) for detailed documentation.

## Development

### Running the Full Stack

1. **Start the backend** (in one terminal):
   ```bash
   cd wallet
   cargo run --release
   ```

2. **Start the frontend** (in another terminal):
   ```bash
   cd wallet-frontend
   npm run dev
   ```

3. Open `http://localhost:5173` in your browser

### Testing

Run all tests across the project (excluding f1r3node):
```bash
cargo test --release && git submodule foreach --recursive 'if [ "$name" != "f1r3node" ] && [ -f Cargo.toml ]; then echo "Testing $name"; cargo test --release; fi'
```

Run all tests including f1r3node:
```bash
cargo test --release && git submodule foreach --recursive 'if [ -f Cargo.toml ]; then echo "Testing $name"; cargo test --release; fi'
```
