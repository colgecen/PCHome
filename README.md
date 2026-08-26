# PChome - Next-Gen Remote Control System

> High-performance, low-latency (<40ms) remote control for Linux ↔ Android.

## Quick Start

### Option 1: Docker (Easiest)

```bash
# One-liner: start signal server
docker compose up -d

# Desktop daemon shows its own egui HUD (PIN + telemetry)
cd pchome-desktop && cargo build --release
sudo ./target/release/pchome-desktop
```

### Option 2: Local Build

```bash
# One-liner: start everything
make run

# Or manually:
./scripts/run-local.sh
```

### Option 3: Build Only

```bash
make build
```

## What You Get

| Component | Port | URL |
|-----------|------|-----|
| Signal Server | 8080 | ws://localhost:8080/ws |
| Desktop HUD | - | egui window (PIN + telemetry) |

## Project Structure

```
pchome/
├── pchome-desktop/   # Rust daemon + egui HUD
├── pchome-mobile/    # Android app
├── pchome-signal/    # Rust signal server (WebSocket relay)
├── scripts/          # Helper scripts
├── Makefile          # Simple build targets
└── docker-compose.yml # Docker setup
```

## Common Commands

```bash
make run          # Start signal + desktop
make run-docker   # Start signal via Docker
make test         # Run all tests
make build        # Build all modules
make clean        # Clean artifacts
```

## Manual Commands

```bash
# Signal server
cd pchome-signal && cargo run --release

# Desktop
cd pchome-desktop && cargo run

# Mobile
cd pchome-mobile && ./gradlew assembleDebug
```

## Requirements

- **Docker**: For signal server (easiest)
- **Rust 1.70+**: For signal server and desktop
- **JDK 17+**: For mobile

## Documentation

- [SECURITY.md](SECURITY.md) - Security guidelines
- [CONTRIBUTING.md](CONTRIBUTING.md) - Contribution guide
- [pchome-desktop/BUILD.md](pchome-desktop/BUILD.md) - Desktop build details
- [pchome-mobile/PERMISSIONS.md](pchome-mobile/PERMISSIONS.md) - Mobile permissions
