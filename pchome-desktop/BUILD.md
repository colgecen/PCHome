# PChome Desktop Build Guide

## Prerequisites

### System Requirements
- Linux kernel 5.10+ with uinput support
- PipeWire 0.3+ with DMA-BUF support
- Rust 1.70 or newer
- Cargo for package management
- Optional: Tauri or Slint for GUI framework

### Dependencies

#### Core Rust Dependencies (Cargo.toml)
```toml
[dependencies]
tokio = { version = "1.38", features = ["full"] }
pipewire = "0.8"
uinput = "0.5"
display-interface = "0.5"
tauri = "2.0" # or slint = "1.5"
sha2 = "0.10"
serde = { version = "1.0", features = ["derive"] }
```

#### Hardware Acceleration (Optional)
- `libva-dev` or `libnvenc` for H.264 encoding
- `gstreamer-1.0` with good/plugins elements

## Build Steps

### 1. Clone Repository
```bash
git clone https://github.com/yourorg/pchome.git
cd pchome
```

### 2. Build Signal Server
```bash
cd pchome-signal
go mod tidy
go run main.go
```

### 3. Build Desktop Daemon
```bash
cd pchome-desktop
cargo build --release
```

### 4. Build GUI Frontend
```bash
# If using Tauri
cd pchome-desktop/src-ui
npm install
npm run build

# If using Slint
cd pchome-desktop/src-ui
slint build
```

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
Ensure PipeWire captures screen with DMA-BUF:
```bash
# Check PipeWire version
pw-info --version

# Verify DMA-BUF support
pactl list modules | grep -i dmabuf
```

## Running

### Start Signal Server (separate terminal)
```bash
cd pchome-signal
go run main.go
```

### Start Desktop Daemon
```bash
cd pchome-desktop
./target/release/pchome-desktop
```

### Access HUD
Open browser to `http://localhost:3000` or run GUI frontend

## Debugging

### Common Issues

1. **uinput permission denied**
   - Verify udev rules loaded: `ls -la /dev/uinput`
   - Add user to pchome group: `usermod -aG pchome $USER`

2. **PipeWire capture failure**
   - Check PipeWire is running: `pw-cli list-objects`
   - Verify DMA-BUF format support

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