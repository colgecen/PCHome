# PChome - Next-Gen Remote Control System

## Overview

PChome is a high-performance, low-latency (<40ms) cross-platform remote control and display mirroring ecosystem designed for Linux and Android. The project operates as a monorepo consisting of three decoupled yet seamlessly integrated modules:

1. **PChome Desktop** - Linux Daemon + GUI written in Rust
2. **PChome Mobile** - Android Native application written in Java
3. **PChome Signal** - P2P Handshake Server written in Go

## Architecture

```
┌─────────────────┐     WebSocket TLS      ┌─────────────────┐
│  PChome Mobile  │◄──────────────────────►│ PChome Signal   │
│  (Android/Java) │     PIN Auth + SDP      │   (Go)          │
└────────┬────────┘                         └─────────────────┘
         │                                           ▲
         │ WebRTC SRTP                               │
         │ P2P Media + Control                       │
         ▼                                           │
┌─────────────────┐                         ┌─────────────────┐
│ PChome Desktop  │◄──────────────────────►│  Linux Kernel   │
│   (Rust)        │   /dev/uinput + PipeWire │  (uinput/PW)   │
└─────────────────┘                         └─────────────────┘
```

## Color Palette

- **Primary Background**: `#090C10` (Deep Obsidian)
- **Card/Container**: `#0D1117`
- **Primary Accent**: `#00F4FF` (Cyber Neon Cyan)
- **Secondary Accent**: `#0A84FF` (Electric Blue)
- **Primary Text**: `#FFFFFF`
- **Subtle UI Borders**: `#30363D`
- **Error/Alert**: `#FF2A55` (Neon Red)

## Bi-directional Control Flows

- **Flow A — Phone Controls Linux PC**: Touchpad / keyboard events captured on the mobile HUD, serialized as binary over the WebRTC DataChannel, and injected via `/dev/uinput` on the Linux desktop.
- **Flow B — Linux PC Controls / Mirrors Android Phone**: Desktop screen is captured via PipeWire (DMA-BUF), H.264 encoded, and streamed over WebRTC `MediaStream`; mouse clicks over the video window are sent back as coordinates to Android's `AccessibilityService` for gesture injection.

## Authentication

- 6-digit cryptographic PIN generated per session
- Valid for 300 seconds (TTL)
- Registered via WebSockets to Signal server
- PIN format: e.g., `849-204`

## Quick Start

### Prerequisites

- Linux system with PipeWire support
- Android device with MediaProjection support
- Go 1.23+ for Signal server
- Rust 1.70+ for Desktop client
- Java JDK 17+ for Mobile client
- Docker & Docker Compose (optional, for signal server)

### 1. Clone Repository

```bash
git clone https://github.com/yourorg/pchome.git
cd pchome
```

### 2. Run Local Development Environment

```bash
# One-liner to build and start all services
./scripts/run-local.sh
```

Or manually:

```bash
# Terminal 1: Start Signal Server
cd pchome-signal
go mod tidy
go run main.go

# Terminal 2: Start Desktop Daemon
cd pchome-desktop
cargo run
```

### 3. Access HUD

Open browser to `file://$(pwd)/pchome-desktop/src-ui/index.html` or run the GUI frontend.

### 4. Build Mobile App

```bash
cd pchome-mobile
./gradlew assembleDebug
```

Install APK to connected device/emulator:

```bash
adb install app/build/outputs/apk/debug/app-debug.apk
```

### 5. Test Connection

1. Open PChome HUD in browser
2. Note the generated PIN
3. Open mobile app and enter PIN
4. WebRTC handshake completes automatically
5. Control Linux desktop from mobile touchpad

## Developer Guide

### Project Structure

```
pchome/
├── pchome-desktop/          # Rust desktop daemon + HUD
│   ├── src/                 # Core modules
│   ├── src-ui/              # HUD frontend
│   ├── benches/             # Performance benchmarks
│   ├── tests/               # Integration tests
│   └── Cargo.toml
├── pchome-mobile/           # Android app
│   ├── app/src/main/        # Java sources + resources
│   ├── app/src/test/        # Unit tests
│   ├── app/src/androidTest/ # Instrumented tests
│   └── build.gradle
├── pchome-signal/           # Go signal server
│   ├── internal/room/       # PIN room management
│   ├── internal/signal/     # WebSocket hub + metrics
│   ├── main.go
│   ├── Dockerfile
│   └── go.mod
├── scripts/                 # Dev scripts
├── .github/workflows/       # CI/CD pipelines
├── SECURITY.md
├── CONTRIBUTING.md
└── TODO.md
```

### Development Scripts

```bash
# Setup development environment (install Rust, Go, JDK)
./scripts/dev-setup.sh

# Run local development stack
./scripts/run-local.sh

# Run end-to-end tests
./scripts/test-e2e.sh
```

### Running Tests

```bash
# Desktop unit tests
cd pchome-desktop && cargo test --all-targets --all-features

# Desktop benchmarks
cd pchome-desktop && cargo bench --bench roundtrip_bench

# Signal unit tests
cd pchome-signal && go test ./...

# Mobile unit tests
cd pchome-mobile && ./gradlew test

# Mobile instrumented tests (requires emulator/device)
cd pchome-mobile && ./gradlew connectedAndroidTest

# End-to-end test
./scripts/test-e2e.sh
```

### Code Style

- **Rust**: `cargo fmt` + `cargo clippy` (enforced by CI)
- **Go**: `gofmt` + `golangci-lint` (enforced by CI)
- **Java**: Android Lint (enforced by CI)

### Commit Convention

We follow [Conventional Commits](https://www.conventionalcommits.org/):

- `feat:` A new feature
- `fix:` A bug fix
- `docs:` Documentation changes
- `style:` Code style changes (formatting, missing semicolons)
- `refactor:` Code refactoring without feature or bug fix changes
- `perf:` Performance improvements
- `test:` Adding or updating tests
- `build:` Changes to build system or dependencies
- `ci:` CI configuration changes
- `chore:` Other changes that don't modify src or test files
- `revert:` Reverting previous commits

### Branching Strategy

- `main` is the primary production branch
- `develop` is the integration branch
- Feature branches: `feature/<short-description>`
- Bug fix branches: `fix/<issue-number>-<short-description>`
- Release branches: `release/<version>`

## Configuration

### Signal Server

```bash
# Run with TLS
pchome-signal -tls-cert=/path/to/tls.crt -tls-key=/path/to/tls.key

# Run with custom rate limit
pchome-signal -rate-limit=50

# Run with custom PIN TTL
pchome-signal -pin-ttl=10m
```

### Desktop Daemon

```bash
# Run with custom signal server URL
PCHOME_SIGNAL_URL=ws://your-server:8080/ws cargo run
```

### Mobile App

Configure signal server URL in `PinActivity.java`:

```java
signalClient = new SignalClient("ws://your-server:8080/ws", pin, listener);
```

## Deployment

### Docker (Signal Server)

```bash
cd pchome-signal
docker compose up -d
```

Services:
- Signal Server: `https://localhost:8443`
- Prometheus: `http://localhost:9090`
- Grafana: `http://localhost:3000`

### systemd (Desktop Daemon)

See `SECURITY.md` for complete systemd service configuration.

## Troubleshooting

### Desktop Issues

1. **uinput permission denied**
   - Verify udev rules: `ls -la /dev/uinput`
   - Add user to `pchome` group: `sudo usermod -aG pchome $USER`

2. **PipeWire capture failure**
   - Check PipeWire is running: `pw-cli list-objects`
   - Verify DMA-BUF format support: `pactl list modules | grep -i dmabuf`

3. **PIN not registering**
   - Check Signal server WebSocket connection
   - Verify PIN TTL (300s from generation)
   - Ensure network connectivity

### Mobile Issues

1. **Accessibility Service not working**
   - Enable in Settings > Accessibility > PChome Accessibility Service

2. **MediaProjection denied**
   - Re-request projection permission from the app

3. **WebRTC connection failed**
   - Check signal server is reachable
   - Verify PIN is correct and not expired
   - Check firewall rules for STUN/TURN

### Signal Server Issues

1. **TLS certificate errors**
   - Ensure certificate matches server hostname
   - Check certificate expiration: `openssl x509 -in tls.crt -noout -dates`

2. **Rate limiting too aggressive**
   - Adjust `-rate-limit` flag
   - Check `/metrics` for rate limit hits

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for contribution guidelines.

## License

[LICENSE](LICENSE)
