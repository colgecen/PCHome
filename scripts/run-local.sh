#!/usr/bin/env bash
set -euo pipefail

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}=== Starting PChome ===${NC}"

# Check dependencies
check_cmd() {
    if ! command -v "$1" &> /dev/null; then
        echo "Warning: $1 not found. Skipping..."
        return 1
    fi
    return 0
}

# Start signal server
if check_cmd go; then
    echo -e "${GREEN}[1/2] Starting Signal Server on :8080${NC}"
    cd pchome-signal
    go mod tidy > /dev/null 2>&1 || true
    go run main.go &
    SIGNAL_PID=$!
    cd ..
else
    echo "Go not installed. Skip signal server."
    SIGNAL_PID=""
fi

# Start desktop
if check_cmd cargo; then
    echo -e "${GREEN}[2/2] Starting Desktop Daemon${NC}"
    cd pchome-desktop
    cargo run &
    DESKTOP_PID=$!
    cd ..
else
    echo "Rust/Cargo not installed. Skip desktop."
    DESKTOP_PID=""
fi

echo ""
echo -e "${GREEN}Services started:${NC}"
echo "  Signal:  http://localhost:8080"
echo "  HUD:     file://$(pwd)/pchome-desktop/src-ui/index.html"
echo ""
echo "Press Ctrl+C to stop..."

trap "kill $SIGNAL_PID $DESKTOP_PID 2>/dev/null || true" EXIT
wait
