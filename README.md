# F1r3fly-RGB

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

## Building

Build the RGB wallet:
```bash
cd rgb
cargo build -p rgb-wallet --release
```

Verify the build:
```bash
../rgb/target/release/rgb --help
```

## Quick Start

Start the backend:
```bash
cd rgb-wallet/backend
cargo run --release
```

Server will start at http://127.0.0.1:8080

### Other

`cargo test --release && git submodule foreach --recursive 'if [ "$name" != "f1r3node" ] && [ -f Cargo.toml ]; then echo "Testing $name"; cargo test --release; fi'
`
