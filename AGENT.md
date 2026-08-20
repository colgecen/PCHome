# PChome AI Agent Working Rules & Constraints

## General Guidelines

### Zero-Lag Policy
All code must prioritize non-blocking async architecture:
- **Rust**: Use Tokio async runtime for all I/O operations
- **Go**: Use Goroutines + channels for concurrent operations
- **Java**: Use Background Threads + Handlers + Coroutines where applicable

Any blocking operation must be offloaded to a background thread/async task.

### Color Strictness
All UI code snippets must apply the following color palette:

| Element | Hex Code |
|---------|----------|
| Primary Background | `#090C10` |
| Card/Container Background | `#0D1117` |
| Primary Accent (active states) | `#00F4FF` |
| Secondary Accent | `#0A84FF` |
| Primary Text | `#FFFFFF` |
| Subtle UI Borders/Grid Lines | `#30363D` |
| Error/Alert/Disconnected State | `#FF2A55` |

Violations of color palette will be flagged during code review.

### Low-Level Native API Preference
Always prefer the following native APIs over alternatives:

| API | Preferred Alternative |
|-----|----------------------|
| Screen Capture | PipeWire DMA-BUF (Linux) / MediaCodec (Android) |
| Input Injection | /dev/uinput over X11 tools |
| Real-time Transport | WebRTC DataChannels over HTTP polling |
| GPU Encoding | VA-API / NVENC H.264 over software bitmap encoding |

### Module Boundaries
Code must remain within its designated module:
- **Rust code** stays in `pchome-desktop/`
- **Java code** stays in `pchome-mobile/`
- **Go code** stays in `pchome-signal/`
- Cross-module dependencies must be explicitly documented and minimal

## Security Requirements

### P2P Encryption
- All data transmitted between Desktop and Mobile must use WebRTC DataChannels
- WebRTC provides built-in SRTP encryption
- No plaintext HTTP traffic allowed between client modules

### Linux udev Security Rules
- /dev/uinput access must be restricted to authorized users
- udev rules must be defined for persistent device permissions
- Signal server must validate PIN authentication before any data transmission

### PIN Security
- 6-digit PIN must be generated using cryptographically secure random number generator
- PIN transmission must occur over WebSockets only after TLS establishment
- PIN TTL of 300 seconds must be strictly enforced

## Development Workflow

### Conventional Commits
All git commits must follow Conventional Commits format:
```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

### Allowed Types
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Formatting, missing semicolons, etc.
- `refactor`: Code restructuring
- `test`: Adding missing tests
- `chore`: Routine maintenance

### Checkbox Tracking
All TODO items must use the format:
```
- [ ] Task description
```

Progress must be updated after each working session.

## Constraints

1. No blocking I/O in async contexts
2. Color palette must be exact (no variations)
3. 6-digit PIN authentication is mandatory for all sessions
4. Latency must remain under 40ms where applicable
5. Cross-platform compatibility: Linux (Desktop) + Android (Mobile)