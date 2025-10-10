#!/usr/bin/env bash
set -e

echo "🧪 Testing all submodules (release)..."
git submodule foreach --recursive '
if [ -f Cargo.toml ]; then
    echo "→ Testing $name"
    cargo test --release || true
fi
'
