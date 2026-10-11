# PCHome

> Turn your Linux PC into a remote-controlled machine — stream the screen to your Android phone over a direct WebRTC connection and control mouse & keyboard from your pocket.

PCHome is a self-hosted remote control system consisting of three components: a Linux desktop daemon with a permanent PC ID and fixed password, an Android app that connects with ID + password and keeps a "Recent" grid with thumbnails, and a tiny WebSocket signaling server that pairs the two. Media never touches the server — it flows peer-to-peer. Both apps update themselves from GitHub Releases.

| | |
|---|---|
| **Desktop** | Rust · PipeWire capture · VA-API/NVENC H.264 · uinput injection · egui HUD |
| **Mobile** | Java · WebRTC `SurfaceViewRenderer` · touchpad overlay |
| **Signal** | Rust · tokio · WebSocket relay · Prometheus metrics |

## Table of Contents

- [Features](#features)
- [How It Works](#how-it-works)
- [Installation](#installation)
- [Usage](#usage)
- [Configuration](#configuration)
- [Project Structure](#project-structure)
- [Running Tests](#running-tests)
- [Contributing](#contributing)
- [License](#license)

## Features

- **Low-latency P2P streaming** — screen video travels over a direct WebRTC peer connection; the relay only handles handshake.
- **Hardware-accelerated encoding** — VA-API or NVENC H.264 on the desktop, MediaCodec on Android, automatic software fallback.
- **Permanent PC ID + fixed password** — the desktop creates a 6-digit ID and 6-digit password once (`~/.config/PCHome/identity.toml`); the phone connects with ID + password, no rotating PINs.
- **Recent grid** — the phone keeps the last PCs with a thumbnail square plus ID/IP/MAC underneath; tap to reconnect, long-press to delete.
- **Remote input** — tap, swipe and hotkeys from the phone, injected via `/dev/uinput`.
- **Zero-config cloud deploy** — ship the signal server anywhere with the included Dockerfile; `/health` and `/metrics` are served on the same port for platforms that expose only one.
- **Built-in HUD** — the desktop daemon opens an egui window showing the active PIN and live telemetry.
- **Rate limiting** — per-IP sliding-window cap on new relay connections.

## How It Works

```
+------------------+   WebSocket    +----------------+   WebSocket    +----------------+
| PCHome-desktop   |<-------------->|  PCHome-signal |<-------------->| PCHome-mobile  |
| capture + encode |  PIN register  | PIN room match |  join w/ PIN   | render + input |
+------------------+                +----------------+                +----------------+
         \                                                                                /
          \____________________ direct WebRTC (video + data channel) ____________________/
```

 1. The desktop daemon loads (or generates) its permanent PC ID + password and registers the ID with the signal server.
 2. You type the PC ID + password into the Android app.
 3. The desktop verifies the password itself (`auth_ok` / `auth_rejected`, 60 s lockout after 5 wrong tries) and the server relays SDP offers/answers plus ICE candidates.
 4. A direct WebRTC connection carries the video stream and input events from then on.

## Installation

### Prerequisites

| Component | Requirement |
|-----------|-------------|
| Desktop daemon | Linux (kernel 5.10+), Rust 1.70+, PipeWire 0.3+, ffmpeg, `/dev/uinput` access |
| Hardware encoding *(optional)* | VA-API (`/dev/dri/renderD128`) or NVIDIA GPU — falls back to `libx264` |
| Mobile app | Android Studio Iguana (2023.2.1)+, JDK 17, Android SDK (API 34) |
| Signal server | Rust 1.70+ *or* Docker |

### Clone

```bash
git clone https://github.com/colgecen/PCHome.git
cd PCHome
```

### Build the signal server

```bash
cd PCHome-signal
cargo build --release
# binary: target/release/PCHome-signal
```

Or with Docker:

```bash
docker compose up -d
```

### Build the desktop daemon

```bash
cd PCHome-desktop
cargo build --release
```

See [PCHome-desktop/BUILD.md](PCHome-desktop/BUILD.md) for system dependencies and udev rules.

### Build the Android app

```bash
cd PCHome-mobile
./gradlew assembleDebug
# APK: app/build/outputs/apk/debug/app-debug.apk
```

See [PCHome-mobile/BUILD.md](PCHome-mobile/BUILD.md) for details.

## Usage

### 1. Start the signal server

Local development:

```bash
cd PCHome-signal
./target/release/PCHome-signal
# relay:  ws://0.0.0.0:8080/ws
# health: http://0.0.0.0:8081/health
```

Or point everything at a hosted instance — any platform that exposes a single TCP port works (the same port serves both the WebSocket relay and plain-HTTP probes):

```bash
curl https://your-signal-host.example.com/health   # should print: OK
```

### 2. Start the desktop daemon

```bash
cd PCHome-desktop
sudo ./target/release/PCHome-desktop
```

An egui window opens showing the permanent PC ID and password. (`sudo` is needed for `/dev/uinput`; see [PCHome-desktop/etc/udev/rules.d](PCHome-desktop/etc/udev/rules.d) for passwordless setups.)

### 3. Connect the phone

Install the APK, enter the signal server address, the PC ID and the password shown on the desktop — the desktop verifies you and streaming starts. The PC is saved to the "Recent" grid with its last screenshot.

## Updates

Both apps check GitHub Releases automatically (desktop: 20 s after start, then every 6 h; mobile: at most once a day on the connect screen):

- **Desktop** — the "Ayarlar" panel shows the status; "Şimdi Güncelle" downloads and installs the new AppImage/tarball (`.bak` backup kept next to the binary).
- **Mobile** — an update card appears on the connect screen; "Güncelle" downloads the APK and opens the system installer.
- Every release also ships a `latest.json` with direct asset URLs, and the signal server reports its version at `GET /version`.

Live signal server: `wss://PCHome.onrender.com/ws` (health: `https://PCHome.onrender.com/health`).

### One-liner for local development

```bash
make run        # builds and starts signal server + desktop daemon
```

## Configuration

Everything is configured through environment variables (a local untracked `.env` next to the desktop daemon is also loaded).

### Signal server

| Variable | Default | Description |
|----------|---------|-------------|
| `PORT` | `8080` | WebSocket relay port |
| `HEALTH_PORT` | `8081` | Dedicated health/metrics port |
| `RATE_LIMIT` | `20` | New connections per minute per IP |
| `RUST_LOG` | – | Log level, e.g. `info`, `debug` |

### Desktop daemon

| Variable | Default | Description |
|----------|---------|-------------|
| `PCHOME_SIGNAL_URL` | `ws://localhost:8080` | Signal server URL (`wss://…` in production) |
| `PCHOME_CAPTURE_WIDTH` | `1920` | Capture width in pixels |
| `PCHOME_CAPTURE_HEIGHT` | `1080` | Capture height in pixels |
| `PCHOME_BITRATE` | `4000000` | Target H.264 bitrate (bits/s) |
| `PCHOME_METRICS_ADDR` | `0.0.0.0:9091` | Bind address for the metrics endpoint |

Example:

```bash
sudo env PCHOME_SIGNAL_URL=wss://your-signal-host.example.com/ws \
         PCHOME_BITRATE=8000000 \
    ./target/release/PCHome-desktop
```

## Project Structure

```
PCHome/
├── PCHome-desktop/   # Linux daemon: capture, encode, uinput input, egui HUD
├── PCHome-mobile/    # Android app: WebRTC rendering + touchpad overlay
├── PCHome-signal/    # WebSocket signaling relay with PIN rooms
├── scripts/          # Helper scripts (local run, e2e smoke test, TLS certs)
├── docker-compose.yml
└── Makefile          # make run / build / test / clean
```

Module-specific documentation lives next to the code: [`ARCHITECTURE.md`](ARCHITECTURE.md) for the overall design, plus `BUILD.md` / `README.md` / `PERMISSIONS.md` inside each module.

## Running Tests

```bash
make test                      # desktop + signal

cd PCHome-signal && cargo test --release     # includes a live relay roundtrip test
cd PCHome-desktop && cargo test --all-targets --all-features

cd PCHome-mobile && ./gradlew test           # Android unit tests
./scripts/e2e-test.sh                        # full-stack smoke test
```

## Contributing

Contributions are welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md) first — it covers the fork → branch → pull request flow, conventional commit format (`feat`, `fix`, `docs`, … enforced by [commitlint](commitlint.config.js)) and code style expectations. Security-sensitive findings have their own channel in [SECURITY.md](SECURITY.md).

## License

Distributed under the MIT License. See [LICENSE](LICENSE) for details.
