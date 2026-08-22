#!/usr/bin/env bash
set -euo pipefail

echo "=== PChome Developer Setup ==="

if ! command -v rustc &> /dev/null; then
    echo "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi

if ! command -v java &> /dev/null; then
    echo "Installing JDK 17..."
    sudo apt-get update -qq
    sudo apt-get install -y -qq openjdk-17-jdk
fi

echo "Setting up pre-commit hooks..."
if command -v pre-commit &> /dev/null; then
    pre-commit install
else
    echo "pre-commit not found. Install with: pip install pre-commit"
fi

echo "Building all modules..."
cd pchome-signal && cargo build && cd ..
cd pchome-desktop && cargo build && cd ..

echo "Setup complete. Run ./scripts/run-local.sh to start development environment."
