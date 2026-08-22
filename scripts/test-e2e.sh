#!/usr/bin/env bash
set -euo pipefail

echo "=== PChome E2E Test Harness ==="

SIGNAL_URL="${SIGNAL_URL:-ws://localhost:8080/ws}"
PIN="${PIN:-000000}"

echo "[1/4] Starting signal server..."
cd pchome-signal
cargo build
./target/debug/pchome-signal &
SIGNAL_PID=$!
cd ..

sleep 2

echo "[2/4] Starting desktop daemon stub..."
cd pchome-desktop
cargo run --quiet &
DESKTOP_PID=$!
cd ..

sleep 3

echo "[3/4] Running WebRTC handshake simulation..."
python3 - <<'PY'
import json
import time
import websocket

SIGNAL_URL = "ws://localhost:8080/ws"
PIN = "000000"

try:
    ws = websocket.create_connection(f"{SIGNAL_URL}?pin={PIN}")
    offer = {
        "type": "offer",
        "sdp": "v=0\r\no=- 0 0 IN IP4 127.0.0.1\r\ns=-\r\nc=IN IP4 127.0.0.1\r\nt=0 0\r\n"
    }
    ws.send(json.dumps(offer))
    time.sleep(1)
    response = ws.recv()
    print(f"Received: {response[:80]}...")
    ws.close()
    print("handshake_ok")
except Exception as e:
    print(f"handshake_failed: {e}")
    exit(1)
PY

HANDSHAKE_RESULT=$?

echo "[4/4] Cleaning up..."
kill $DESKTOP_PID 2>/dev/null || true
kill $SIGNAL_PID 2>/dev/null || true

if [ $HANDSHAKE_RESULT -eq 0 ]; then
    echo "E2E test passed"
    exit 0
else
    echo "E2E test failed"
    exit 1
fi
