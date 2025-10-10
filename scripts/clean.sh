#!/usr/bin/env bash
set -e

echo "Cleaning all submodules..."
git submodule foreach --recursive '
if [ -f Cargo.toml ]; then
    echo "→ Cleaning $name"
    cargo clean || true
fi
'
