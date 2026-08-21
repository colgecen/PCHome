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
    echo "Open HUD:"
    echo "  file://$(pwd)/pchome-desktop/src-ui/index.html"
else
    echo "Docker not found. Starting local services..."
    ./scripts/run-local.sh
fi
