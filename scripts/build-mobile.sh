#!/usr/bin/env bash
# Build a PChome mobile release APK wired to YOUR personal signal server.
#
# Usage:
#   ./scripts/build-mobile.sh [SIGNAL_URL]
#
# If SIGNAL_URL is omitted it is read from pchome-mobile/local.properties
# (gitignored) — add your own line there once:
#     echo 'signalUrl=wss://your-service.onrender.com/ws' >> pchome-mobile/local.properties
#
# The app bakes the URL into BuildConfig.SIGNAL_URL so the PIN screen is
# pre-filled with your server on first launch.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT/pchome-mobile"

if [ $# -ge 1 ]; then
    echo "signalUrl=$1" >> local.properties
fi

if [ ! -f local.properties ]; then
    echo "local.properties not found. Create it with: signalUrl=wss://host/ws" >&2
    exit 1
fi

chmod +x gradlew
./gradlew assembleRelease --no-daemon

APK="app/build/outputs/apk/release/app-release.apk"
if [ -f "$APK" ]; then
    echo "==> Built: $ROOT/pchome-mobile/$APK"
else
    echo "==> Release APK not found; build may have failed." >&2
    exit 1
fi
