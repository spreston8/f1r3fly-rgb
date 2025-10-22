# RGB-Compatible Bitcoin Wallet

A Bitcoin Signet wallet with RGB compatibility, starting with basic wallet operations and balance tracking.

## Usage

### Development

```bash
# Start server (listens on 0.0.0.0:3000 by default)
cargo run

# Or with custom configuration
BIND_ADDRESS=127.0.0.1:3000 RUST_LOG=debug cargo run
```

### Production

See `.env.example` for configuration options. Key environment variables:

- `BIND_ADDRESS` - Server bind address (default: `0.0.0.0:3000`)
  - Use `127.0.0.1:3000` for local development only
  - Use `0.0.0.0:3000` for production (accepts external connections)

- `ALLOWED_ORIGINS` - CORS allowed origins (default: allow all with warning)
  - Set to your frontend URL(s) for production
  - Example: `https://your-app.vercel.app,https://your-app-git-*.vercel.app`

- `RUST_LOG` - Log level (default: `info`)
  - Options: `error`, `warn`, `info`, `debug`, `trace`

```bash
# Production example
BIND_ADDRESS=0.0.0.0:3000 \
ALLOWED_ORIGINS=https://your-app.vercel.app \
RUST_LOG=info \
cargo run --release
```

## Network
Currently Signet only.

## Storage
Wallet data stored in `./wallets/{name}/`

