#!/usr/bin/env bash
# Launch the full PChome stack locally for development.
#
#   1. PChome Signal server (Go)
#   2. PChome Desktop daemon (Rust)
#
# The Android app (pchome-mobile) must be launched separately via Android
# Studio or `./gradlew installDebug` against an emulator/device, then pair
# with the PIN printed by the desktop daemon.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SIGNAL_ADDR="ws://127.0.0.1:8080/ws"

echo "==> Starting PChome Signal server"
( cd "$ROOT/pchome-signal" && go run ./cmd/server ) &
SIGNAL_PID=$!

cleanup() {
    kill "$SIGNAL_PID" 2>/dev/null || true
}
trap cleanup EXIT

# Give the signal server a moment to bind.
sleep 2

echo "==> Starting PChome Desktop daemon"
( cd "$ROOT/pchome-desktop" && cargo run -- ) &
DESKTOP_PID=$!

echo "==> Signal: $SIGNAL_ADDR"
echo "==> Desktop daemon is running (PID $DESKTOP_PID)"
echo "==> Pair the Android app using the PIN printed above."
wait
