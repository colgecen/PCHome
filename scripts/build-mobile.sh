#!/usr/bin/env bash
# Build a PChome mobile release APK.
#
# Usage:
#   ./scripts/build-mobile.sh
#
# NOT: Sinyal adresi APK'ya gömülmez; kullanıcı bağlantı ekranına eliyle
# yazar. local.properties yalnız Android SDK yolunu (sdk.dir) içerir.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT/pchome-mobile"

chmod +x gradlew
./gradlew assembleRelease --no-daemon

APK="app/build/outputs/apk/release/app-release.apk"
if [ -f "$APK" ]; then
    echo "==> Built: $ROOT/pchome-mobile/$APK"
else
    echo "==> Release APK not found; build may have failed." >&2
    exit 1
fi
