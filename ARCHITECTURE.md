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
+-----------------------+                    +-----------------------+
|    PChome Desktop     |   WebRTC P2P Stream|    PChome Mobile      |
|     (Linux Daemon)    |====================|     (Android App)     |
|                       |  Media / DataChan  |                       |
+-----------------------+                    +-----------------------+
            │                                            │
            │ WebSocket                                  │ WebSocket
            │ (PIN Auth & SDP/ICE Relay)                 │ (PIN Auth & SDP/ICE Relay)
            ▼                                            ▼
+--------------------------------------------------------------------+
|                       PChome Signal Server                         |
|                 (Go / Room Manager & WebSocket)                    |
+--------------------------------------------------------------------+

+-----------------------+                    +-----------------------+
|    Linux Kernel       | ◀─── /dev/uinput   | PChome Desktop Daemon |
| (Virtual Input Dev)   |   Input Injection  | (Coordinate Processing)|
+-----------------------+                    +-----------------------+

+-----------------------+                    +-----------------------+
|   Android Kernel      | ◀── Accessibility  | PChome Mobile Service |
|  (System Gestures)    |   Gesture Inject   | (Remote Coordinate)   |
+-----------------------+                    +-----------------------+
```

## Component Interaction

1. **Handshake Phase**: PIN authentication via Signal server WebSockets
2. **Connection Phase**: WebRTC SDP/ICE candidate exchange for NAT traversal
3. **Stream Phase**: DMA-BUF zero-copy screen capture → H.264 encoding → WebRTC DataChannel
4. **Input Phase**: /dev/uinput (Desktop) / AccessibilityService (Mobile) → WebRTC DataChannel
5. **Control Phase**: Asynchronous command routing via UDP/WebSocket

## Control Flows

### Flow A: Phone Controls Linux PC (Remote Touchpad / Keyboard)

- **Mobile (Java)**: The `TouchpadActivity` tracks finger deltas (`ΔX, ΔY`), swipes, and gestures via `View.OnTouchListener`. Input is serialized into compact binary packets and sent over the WebRTC DataChannel.
- **Desktop (Rust)**: The daemon receives the binary packets, scales coordinates, and injects motion directly into the kernel level via `/dev/uinput`, bypassing Wayland/X11 display server restrictions.
- **Result**: The user can control the Linux desktop's mouse cursor and keyboard from the Android device.

### Flow B: Linux PC Controls / Mirrors Android Phone

- **Mobile Capture (Java)**: The `ScreenCaptureService` captures frames via `MediaProjection`, hardware‑encodes video to H.264 using `MediaCodec`, and broadcasts the stream over the WebRTC `MediaStream`.
- **Desktop Renderer (Rust)**: The incoming H.264 stream is rendered onto the Rust UI canvas. Mouse clicks over the video window are captured, and scaled X/Y coordinates are sent back to the Android device.
- **Mobile Injection (Java)**: The `AndroidControlService` (AccessibilityService) receives the remote coordinates and executes real‑time gestures using `GestureDescription.Builder` without requiring root access.
- **Result**: The Linux desktop mirrors the Android phone's screen and can inject touch gestures back.