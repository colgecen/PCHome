#!/usr/bin/env bash
# Generate a self-signed certificate for the PChome Signal server so it can
# terminate TLS (wss://) instead of plain ws://.
#
# Output: ./pchome-signal/certs/server.{crt,key}
set -euo pipefail

OUT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../pchome-signal" && pwd)/certs"
mkdir -p "$OUT_DIR"

openssl req -x509 -newkey rsa:4096 -nodes \
    -keyout "$OUT_DIR/server.key" \
    -out "$OUT_DIR/server.crt" \
    -days 365 \
    -subj "/CN=pchome-signal.local" \
    -addext "subjectAltName=DNS:pchome-signal.local,IP:127.0.0.1"

echo "Generated:"
echo "  $OUT_DIR/server.crt"
echo "  $OUT_DIR/server.key"
