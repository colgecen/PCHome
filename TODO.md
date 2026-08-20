# PChome - To Do List & Phase Plan

## Phase 1: Foundation & Infrastructure

### Completed
- [x] Project specification analysis
- [x] Monorepo structure definition
- [x] Required .md file identification

### Pending
- [ ] Create root-level .md files (README, ARCHITECTURE, DECISIONS, TODO, AGENT, SECURITY)
- [ ] Create module-specific .md files (pchome-desktop/BUILD.md, pchome-mobile/PERMISSIONS.md)
- [ ] Initialize git repository with proper structure
- [ ] Set up CI/CD pipeline basics

---

## Phase 2: PChome Desktop Development

### Rust Backend
- [x] Define uinput.rs wrapper for /dev/uinput
- [x] Implement pipewire.rs DMA-BUF screen capturer
- [x] Build encoder.rs H.264 hardware encoder
- [x] Implement network/mod.rs async handler
- [x] Build socket.rs UDP handler
- [x] Develop webrtc.rs WebRTC PeerConnection engine
- [x] Create pin.rs 6-digit crypto PIN generator

### GUI Frontend
- [x] Design hud.css Neon Cyan/Deep Black aesthetic
- [x] Implement app.js HUD state controller & PIN display
- [x] Create index.html Main UI frame

---

## Phase 3: PChome Mobile Development

### Android Backend
- [x] Configure AndroidManifest.xml permissions
  - [ ] AccessibilityService
  - [ ] MediaProjection
  - [ ] Internet
- [x] Set up ScreenCaptureService.java
  - [ ] MediaProjection integration
  - [ ] MediaCodec hardware encoding
- [x] Implement AndroidControlService.java
  - [ ] AccessibilityService gesture simulation
- [x] Build WebRtcClient.java WebRTC engine
- [x] Develop SignalClient.java WebSocket connection

### UI Frontend
- [x] PinActivity.java 6-Digit HUD PIN Entry
- [x] TouchpadActivity.java Remote Trackpad & Hotkeys
- [x] DisplayActivity.java Remote PC Stream Renderer
- [x] Color definitions (colors.xml)
- [x] Theme styles (styles.xml)
- [x] Layout XML files (activity_pin.xml, activity_touchpad.xml, activity_display.xml)

---

## Phase 4: PChome Signal Server

### Go Server
- [x] main.go Entry point & HTTP/WebSocket router
- [x] Room manager.go PIN room mapping & TTL cleaner
- [x] WebSocket.go SDP offer/answer & ICE candidate relay
- [x] Go module configuration (go.mod, go.sum)
- [x] Dockerfile for 24/7 cloud hosting

---

## Phase 5: Documentation & Testing

### Documentation
- [x] All .md file creation (in progress)
- [ ] Usage examples and tutorials
- [ ] API documentation
- [ ] Deployment guides

### Testing
- [ ] Unit tests for Rust backend
- [ ] Instrumented tests for Android app
- [ ] Integration tests for WebRTC handshake
- [ ] Performance benchmarks (<40ms latency verification)

---

## Phase 6: Release

- [ ] Stabilize all modules
- [ ] Final security review
- [ ] Release binary packages
- [ ] Publish documentation