#!/usr/bin/env bash
set -euo pipefail

echo "=== PChome Local Development Setup ==="

echo "[1/3] Building signal server..."
cd pchome-signal
go mod tidy
go build -o /tmp/pchome-signal ./...
cd ..

echo "[2/3] Building desktop daemon..."
cd pchome-desktop
cargo build
cd ..

echo "[3/3] Starting services..."
echo "Starting signal server on :8080..."
/tmp/pchome-signal &
SIGNAL_PID=$!

echo "Starting desktop daemon..."
cd pchome-desktop
cargo run &
DESKTOP_PID=$!
cd ..

echo ""
echo "Services started:"
echo "  Signal server:  http://localhost:8080"
echo "  Desktop HUD:    file://$(pwd)/pchome-desktop/src-ui/index.html"
echo ""
echo "Press Ctrl+C to stop all services..."

trap "kill $DESKTOP_PID $SIGNAL_PID 2>/dev/null || true" EXIT
wait
