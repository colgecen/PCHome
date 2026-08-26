# PChome Architecture

## System Overview

PChome operates as a monorepo with three decoupled yet seamlessly integrated modules:

### 1. PChome Desktop (Linux Daemon + GUI)

**Language**: Rust  
**Key Dependencies**: tokio, webrtc-rs, eframe/egui  
**Input Injection**: `/dev/uinput` for virtual input  
**Screen Capture**: ffmpeg over PipeWire (VA-API / NVENC / libx264 fallback)  
**Encoding**: Hardware accelerated VA-API / NVENC H.264 encoder  
**Network**: WebSocket signaling + WebRTC PeerConnection  

**Architecture Flow**:
1. Screen capture via ffmpeg (PipeWire source, HW encode when available)
2. Hardware-accelerated H.264 encoding
3. Transmission via WebRTC video track
4. Rendering in the Android app's SurfaceViewRenderer

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

**Language**: Rust (tokio + tokio-tungstenite)  
**Framework**: WebSocket server  
**Components**:
- **Room Manager**: Active 6-digit PIN room mapping & TTL sweeper
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
|               (Rust / Room Manager & WebSocket Relay)              |
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
3. **Stream Phase**: ffmpeg PipeWire capture → H.264 encoding → WebRTC video track
4. **Input Phase**: /dev/uinput (Desktop) / on-screen touch + hardware keyboard (Mobile) → WebRTC DataChannel
5. **Control Phase**: Asynchronous command routing via the WebRTC DataChannel

## Control Flows

### Flow A: Phone Controls Linux PC (Remote Touchpad / Keyboard)

- **Mobile (Java)**: `DisplayActivity` renders the desktop's H.264 stream on a
  `SurfaceViewRenderer` and translates touch into control JSON
  (`move_abs`/`move_rel`/`click`/`scroll`) sent over the unordered/unreliable
  `control` DataChannel. Two modes are supported: **Direct** (touch point maps
  to absolute PC coordinates; two-finger drag = scroll) and **Trackpad**
  (relative deltas + on-screen reticle). A neon `NeonKeyboard` and hardware key
  events (`KeyCodeMap`) send `key` messages.
- **Desktop (Rust)**: `WebRtcEngine` is the WebRTC **offerer**; it opens the
  `control` DataChannel and, on each message, `ControlHandler` injects motion /
  button / wheel / key events into the kernel via `/dev/uinput`, bypassing
  Wayland/X11 restrictions.
- **Result**: The user controls the Linux desktop's mouse cursor and keyboard
  from the Android device with no root and no AccessibilityService.

### Flow B: Linux PC Mirrors / Controls Android Phone — REMOVED

This direction was removed in the current revision. The mobile client no longer
captures its screen or injects gestures; it is purely a remote viewer/controller
of the desktop. The desktop remains the WebRTC offerer and the sole input
injector via `/dev/uinput`.

## Connection Handshake Protocol

The Signal server is a thin, stateless relay. It never inspects SDP/ICE
payloads; it only forwards them between the two peers that share a PIN.

1. **PIN generation (Desktop)**: The daemon generates a cryptographically
   secure 6-digit PIN using `rand::RngCore` and registers the room simply by
   being the first peer to connect. The room carries a TTL (300s) that is
   refreshed on every join and swept by a background cleaner.
2. **Registration**: Desktop opens
   `ws://<signal>/ws?pin=<123456>&role=desktop` and stays connected.
3. **Join**: Mobile opens
   `ws://<signal>/ws?pin=<123456>&role=mobile` (the UI groups the digits as
   `123-456`; separators are stripped before connecting).
4. **Hub formation**: On the first peer's connect the Signal server creates a
    `Room` (`desktop`/`mobile` sender slots). When the second peer
     connects, both clients are linked in a `Room` so messages route between them
     and are never echoed back to the sender.
 5. **Offer/answer**: The desktop is the WebRTC **offerer**. After the hub forms,
    the mobile sends `{ "type": "hello", "role": "mobile" }`; the desktop replies
    with `{ "type": "offer", ... }` and the mobile answers. ICE candidates then
    flow both ways.
 6. **SDP/ICE relay**: Each side sends JSON `{ "type": "offer" | "answer" |
    "ice-candidate", ... }`. The hub JSON-encodes a typed `relayMessage`
    (`from`, `to`, `data`) and writes it to the *other* peer only.
 7. **NAT traversal**: ICE candidates are exchanged through the same relay. If
    both peers are on the same network, host candidates connect directly;
    otherwise a STUN server (`stun:stun.l.google.com:19302`) yields the
    XOR-MAPPED-ADDRESS used for the connection. The desktop builds a valid
    STUN Binding Request (magic cookie `0x2112A442`) and parses the
    XOR-MAPPED-ADDRESS response.
 8. **Media + data**: Once the WebRTC `PeerConnection` is established, screen
    frames and input events flow over the P2P `MediaStream` / `DataChannel`,
    bypassing the Signal server entirely.