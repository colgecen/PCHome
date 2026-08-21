#!/usr/bin/env sh
set -e

CERT_DIR="/certs"
CERT_FILE="${CERT_DIR}/tls.crt"
KEY_FILE="${CERT_DIR}/tls.key"

mkdir -p "${CERT_DIR}"

if [ ! -f "${CERT_FILE}" ] || [ ! -f "${KEY_FILE}" ]; then
    echo "Generating self-signed TLS certificate..."
    openssl req -x509 -nodes -days 365 -newkey rsa:2048 \
        -keyout "${KEY_FILE}" \
        -out "${CERT_FILE}" \
        -subj "/CN=pchome-signal" \
        -addext "subjectAltName=DNS:pchome-signal,IP:127.0.0.1"
    echo "Self-signed certificate generated at ${CERT_FILE}"
else
    echo "Using existing TLS certificate"
fi

exec "$@"
