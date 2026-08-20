# PChome Architectural Decisions (ADR)

## ADR-001: Language Selection per Module

**Decision**: Use language-specific best practices for each platform
- **Desktop**: Rust for performance-critical daemon + zero-copy guarantees
- **Mobile**: Java/Android SDK for native MediaProjection/MediaCodec access
- **Signal Server**: Go for lightweight WebSocket handling & concurrent connections

**Rationale**: Each language offers optimal ecosystem support for the specific native APIs required (/uinput, MediaProjection, WebSockets).

---

## ADR-002: DMA-BUF Zero-Copy Screen Capture

**Decision**: Use PipeWire with DMA-BUF for screen capture instead of X11 or software bitmap encoding

**Rationale**: Achieves <40ms latency target by avoiding memory copies between kernel and user space. Critical for real-time remote control experience.

**Impact**: Desktop only; Mobile uses Android MediaCodec equivalent.

---

## ADR-003: 6-Digit PIN Authentication with 300s TTL

**Decision**: Cryptographically secure random 6-digit PIN valid for 300 seconds

**Rationale**: Balances security (sufficient entropy for session auth) with usability (not too frequent re-authentication). TTL ensures stale sessions are cleaned up automatically.

**Implementation**: Go `crypto/rand` (or Rust `rand::rngs::OsRng`) for cryptographically secure 6-digit PIN generation, registered via WebSockets.

---

## ADR-004: Cyber-Futuristic HUD Design Language

**Decision**: Strict adherence to specified color palette and design aesthetic

**Color Palette Enforcement**:
- Primary Background: `#090C10`
- Card Background: `#0D1117`
- Primary Accent: `#00F4FF` (with glow effect)
- Secondary Accent: `#0A84FF`
- Text: `#FFFFFF`
- Subtle Borders: `#30363D`
- Error State: `#FF2A55`

**Rationale**: Consistent UI/UX across Desktop and Mobile platforms creates cohesive brand experience.

---

## ADR-005: WebRTC DataChannels over HTTP Polling

**Decision**: Use WebRTC DataChannels for real-time transmission instead of HTTP polling or websocket binary frames

**Rationale**: WebRTC provides native real-time transport with built-in flow control, congestion control, and NAT traversal. Essential for low-latency requirements.

---

## ADR-006: /dev/uinput over X11 Tools

**Decision**: Use Linux /dev/uinput for virtual input injection instead of X11-specific tools (xdotool, xdotool, etc.)

**Rationale**: 
- X11 is not available on all target Linux systems
- /dev/uinput works on any Linux kernel with uinput support
- Provides kernel-level input injection for reliable remote control

---

## ADR-007: Modular Monorepo Structure

**Decision**: Single monorepo with per-module directories rather than separate repositories

**Rationale**: 
- Shared configuration and tooling
- Atomic changes across modules when needed
- Simplified development workflow for AI assistant
- Version consistency across all modules

---

## ADR-008: Async-First Architecture

**Decision**: Prioritize non-blocking async architecture throughout

**Implementation**:
- Rust: Tokio async runtime
- Go: Goroutines + channels
- Java: Background Handlers + Coroutines

**Rationale**: Meets low-latency requirements and prevents UI blocking across all platforms.