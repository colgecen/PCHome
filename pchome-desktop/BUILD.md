# PChome Desktop Build Guide

## Prerequisites

### System Requirements
- Linux kernel 5.10+ with uinput support
- PipeWire 0.3+ (screen source for ffmpeg)
- ffmpeg with PipeWire support (`-f pipewire` input)
- Rust 1.70 or newer
- Cargo for package management

### Dependencies

#### Core Rust Dependencies (Cargo.toml)
```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
webrtc = "0.12"
eframe = "0.29"
egui = "0.29"
serde = { version = "1.0", features = ["derive"] }
```

#### Hardware Acceleration (Optional)
- VA-API (`/dev/dri/renderD128`) or NVIDIA GPU for H.264 encoding
- ffmpeg falls back to `libx264` automatically

## Build Steps

### 1. Clone Repository
```bash
git clone https://github.com/yourorg/pchome.git
cd pchome
```

### 2. Build Signal Server
```bash
cd pchome-signal
cargo build --release
```

### 3. Build Desktop Daemon
```bash
cd pchome-desktop
cargo build --release
```

### 4. Build GUI Frontend
The desktop HUD is a native egui window compiled into the daemon
(feature `gui`, enabled by default) — no separate frontend build step.

## Configuration

### udev Rules
Install udev rules for /dev/uinput access:
```bash
sudo cp etc/udev/rules.d/99-pchome-uinput.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules
sudo udevadm trigger
```

### PIN Configuration
Desktop generates cryptographically secure 6-digit PIN on startup:
- PIN valid for 300 seconds (TTL)
- Automatically registered to Signal server via WebSockets
- Displayed in HUD for user input

### PipeWire Configuration
ffmpeg reads the desktop through the PipeWire screen source:
```bash
# Check PipeWire is running
pw-cli info 0

# Verify ffmpeg supports the pipewire input format
ffmpeg -hide_banner -formats 2>/dev/null | grep -i pipewire
```

## Running

### Start Signal Server (separate terminal)
```bash
cd pchome-signal
./target/release/pchome-signal
# Health: http://localhost:8081/health  Metrics: /metrics
```

### Start Desktop Daemon
```bash
sudo env PCHOME_SIGNAL_URL=ws://127.0.0.1:8080/ws \
    ./target/release/pchome-desktop
```
The egui HUD window shows the PIN, connection status, and live telemetry.

### Access HUD
Run the desktop daemon — the HUD is the native egui window it opens.

## Debugging

### Common Issues

1. **uinput permission denied**
   - Run the daemon with sudo, or verify udev rules loaded: `ls -la /dev/uinput`
   - Add user to pchome group: `usermod -aG pchome $USER`

2. **Capture failure / no frames**
   - Check ffmpeg is installed and supports `-f pipewire`: `ffmpeg -formats | grep pipewire`
   - Check PipeWire is running: `pw-cli list-objects`

3. **PIN not registering**
   - Check Signal server WebSocket connection
   - Verify PIN TTL (300s from generation)
   - Ensure network connectivity

### Performance Verification
```bash
# Measure latency
pchome-desktop --measure-latency

# Expected: <40ms from capture to display
```