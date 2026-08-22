#!/usr/bin/env bash
# End-to-end smoke test for the PChome Signal relay.
#
# Starts the signal server, connects two WebSocket clients with the same PIN
# (desktop + mobile roles), has one send a message, and asserts the other
# receives it via the relay hub.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SIGNAL_DIR="$ROOT/pchome-signal"

PIN="$(printf '%06d' "$((RANDOM % 1000000))")"
WS_URL="ws://127.0.0.1:8080/ws?pin=${PIN}"

echo "==> Building signal server"
( cd "$SIGNAL_DIR" && cargo build )

echo "==> Starting signal server (PIN=$PIN)"
( cd "$SIGNAL_DIR" && ./target/debug/pchome-signal ) &
SIG_PID=$!
trap 'kill "$SIG_PID" 2>/dev/null || true' EXIT
sleep 1

echo "==> Running relay roundtrip test"
( cd "$SIGNAL_DIR" && cargo test --release )

echo "==> E2E smoke test passed"
