# PChome Mobile Build Guide

## Prerequisites

- Android Studio Iguana (2023.2.1) or newer
- JDK 17
- Android SDK with API level 34
- Gradle 8.2+ (wrapper included)

## Dependencies

- **WebRTC**: `org.webrtc:google-webrtc:1.0.32006`
- **WebSocket**: `org.java-websocket:Java-WebSocket:1.5.4`
- **HTTP**: `com.squareup.okhttp3:okhttp:4.12.0`
- **Lifecycle**: `androidx.lifecycle:lifecycle-runtime-ktx:2.7.0`

## Build Steps

### 1. Clone and Setup

```bash
git clone https://github.com/yourorg/pchome.git
cd pchome/pchome-mobile
```

### 2. Generate Gradle Wrapper (if missing)

```bash
gradle wrapper
```

### 3. Build Debug APK

```bash
./gradlew assembleDebug
```

Output: `app/build/outputs/apk/debug/app-debug.apk`

### 4. Run Tests

```bash
# Unit tests
./gradlew test

# Instrumented tests (requires connected device/emulator)
./gradlew connectedAndroidTest
```

### 5. Build Release APK

```bash
./gradlew assembleRelease
```

## Permissions Setup

See `PERMISSIONS.md` for runtime permission flows.

## Troubleshooting

### WebRTC Build Issues
Ensure `org.webrtc:google-webrtc` version matches your target SDK.

### Accessibility Service Not Working
Enable in Settings > Accessibility > PChome Accessibility Service.

### MediaProjection Denied
Re-request projection permission from the app.
