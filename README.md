# PChome - Next-Gen Remote Control System

> High-performance, low-latency (<40ms) remote control for Linux ↔ Android.

## Quick Start

### Option 1: Docker (Easiest)

```bash
# One-liner: start signal server
docker compose up -d

# Open HUD
xdg-open pchome-desktop/src-ui/index.html
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
| Signal Server | 8080/8443 | http://localhost:8080 |
| Desktop HUD | - | `file://.../pchome-desktop/src-ui/index.html` |

## Project Structure

```
pchome/
├── pchome-desktop/   # Rust daemon + HUD
├── pchome-mobile/    # Android app
├── pchome-signal/    # Go signal server
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
cd pchome-signal && go run main.go

# Desktop
cd pchome-desktop && cargo run

# Mobile
cd pchome-mobile && ./gradlew assembleDebug
```

## Requirements

- **Docker**: For signal server (easiest)
- **Go 1.23+**: For signal server (if not using Docker)
- **Rust 1.70+**: For desktop
- **JDK 17+**: For mobile

## Documentation

- [SECURITY.md](SECURITY.md) - Security guidelines
- [CONTRIBUTING.md](CONTRIBUTING.md) - Contribution guide
- [pchome-desktop/BUILD.md](pchome-desktop/BUILD.md) - Desktop build details
- [pchome-mobile/PERMISSIONS.md](pchome-mobile/PERMISSIONS.md) - Mobile permissions
