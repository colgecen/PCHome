# PChome Architecture

## System Overview

PChome operates as a monorepo with three decoupled yet seamlessly integrated modules:

### 1. PChome Desktop (Linux Daemon + GUI)

**Language**: Rust  
**Key Dependencies**: tokio, tauri/slint, display-interface  
**Input Injection**: `/dev/uinput` for virtual input  
**Screen Capture**: PipeWire (DMA-BUF zero-copy) for ultra-low latency  
**Encoding**: Hardware accelerated VA-API / NVENC H.264 encoder  
**Network**: Async UDP handler + WebRTC PeerConnection  

**Architecture Flow**:
1. Screen capture via Pipe DMA-BUF (zero-copy)
2. Hardware-accelerated H.264 encoding
3. Transmission via WebRTC DataChannel
4. Rendering in HUD frontend with Cyber-Futuristic theme

### 2. PChome Mobile (Android Native)

**Language**: Java (Android SDK)  
**Screen Capture**: MediaProjection + MediaCodec  
**Input Injection**: AccessibilityService gesture simulator  
**Network**: WebSocket connection to Signal server + WebRTC DataChannel  

**Architecture Flow**:
1. Screen capture via Android MediaProjection
2. Hardware encoding via MediaCodec
3. WebRTC DataChannel transmission
4. HUD rendering with touchpad/hotkeys overlay

### 3. PChome Signal (P2P Handshake Server)

**Language**: Go (Golang)  
**Framework**: WebSocket server  
**Components**:
- **Room Manager**: Active 6-digit PIN room mapping & TTL cleaner
- **WebSocket Relay**: SDP offer/answer & ICE candidate exchange

**Architecture Flow**:
1. Desktop generates cryptographically secure 6-digit PIN (TTL: 300s)
2. PIN registered to Signal server via WebSockets
3. User inputs PIN in mobile app
4. Signal server matches PIN and facilitates WebRTC peer connection
5. NAT traversal via ICE candidate exchange

## Data Flow Diagram

```text
+------------------+     WebRTC     +------------------+
|   PChome Desktop |  ──────────▶ |   PChome Mobile  |
|   (Linux Daemon) |              |   (Android App)  |
+------------------+              +------------------+
      ▲                               ▲
      │   PIN Auth (WebSocket)      │
      ▼                               ▼
+------------------+     SDP/ICE   +------------------+
|  PChome Signal   | ──────────▶ |  PChome Signal   |
|  (Go Server)     |  Offer/Ans  |  (Room Manager)  |
+------------------+             +------------------+

+------------------+     UDP       +------------------+
|   PChome Desktop | ──────────▶ |   PChome Desktop |
|   (Control Signals)          |   (Local Input)  |
+------------------+             +------------------+
```

## Component Interaction

1. **Handshake Phase**: PIN authentication via Signal server WebSockets
2. **Connection Phase**: WebRTC SDP/ICE candidate exchange for NAT traversal
3. **Stream Phase**: DMA-BUF zero-copy screen capture → H.264 encoding → WebRTC DataChannel
4. **Input Phase**: /dev/uinput (Desktop) / AccessibilityService (Mobile) → WebRTC DataChannel
5. **Control Phase**: Asynchronous command routing via UDP/WebSocket