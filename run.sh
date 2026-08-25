#!/usr/bin/env bash
set -euo pipefail

echo "=== PChome Quick Start ==="

if command -v docker &> /dev/null && command -v docker compose &> /dev/null; then
    echo "Starting signal server via Docker..."
    docker compose up -d
    echo ""
    echo "Signal server running at:"
    echo "  HTTP:  http://localhost:8080"
    echo "  HTTPS: https://localhost:8443"
    echo ""
    echo "Desktop daemon (egui HUD) must be started manually:"
    echo "  sudo env PCHOME_SIGNAL_URL=ws://127.0.0.1:8080/ws pchome-desktop/target/release/pchome-desktop"
else
    echo "Docker not found. Starting local services..."
    ./scripts/run-local.sh
fi
