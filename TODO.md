# PChome - To Do List & Expanded Phase Plan
> Her görev bitince conventional commits formatında İngilizce commit at ve kutucuğu işaretle

NOT: Bu dosya `PCHome-prompt.txt` içeriğine göre genişletildi — proje detayları, CI, test, güvenlik ve release adımları eklendi.

## Phase 1: Foundation & Infrastructure

### Completed
- [x] Project specification analysis
- [x] Monorepo structure definition
- [x] Required .md file identification and scaffold

### Essential Setup (High priority)
- [x] Create root-level docs ([README.md](README.md), [ARCHITECTURE.md](ARCHITECTURE.md), [DECISIONS.md](DECISIONS.md), [TODO.md](TODO.md), [AGENT.md](AGENT.md), [SECURITY.md](SECURITY.md))
- [x] Create module-specific docs (pchome-desktop/BUILD.md, pchome-mobile/PERMISSIONS.md, pchome-signal/README.md)
- [x] Initialize `git` repository (conventional commits, branch protection, PR templates)
- [x] Add `.gitignore`, `.editorconfig`, basic `CODEOWNERS`

### CI / Tooling
- [x] Add CI workflows: `lint`, `build`, `test` for each module (GitHub Actions / GitLab CI)
- [x] Configure `pre-commit` hooks: `rustfmt`, `clippy`, `ktlint`/Android lint, `gofmt`/`go vet`
- [x] Dependency security scanning (Dependabot / Snyk) and secret scanning

## Phase 2: PChome Desktop (Rust - Linux)

### Core Daemon & Low-level
- [x] `uinput.rs`: implement safe wrapper for `/dev/uinput` (feature gated, unit tests)
- [x] `pin.rs`: cryptographically secure 6-digit PIN generator, TTL store (300s), WebSocket registration
- [x] `pipewire.rs`: DMA-BUF zero-copy screen capture, fallback to framebuffer if unavailable
- [x] `encoder.rs`: hardware-accelerated H.264 (VA-API / NVENC) with a software fallback
- [x] `network/mod.rs`: async runtime (Tokio) scaffolding, connection manager
- [x] `socket.rs`: UDP helper (NAT probe, STUN helper functions)
- [x] `webrtc.rs`: WebRTC PeerConnection integration, DataChannel for control, track for video

### Security & Permissions
- [x] Udev rules and minimal permission policy for `/dev/uinput` (document in `SECURITY.md`)
- [x] Sandbox the daemon process where possible; capability limits

### GUI (HUD)
- [x] `src-ui/index.html`: frameless HUD shell + PIN display
- [x] `src-ui/styles/hud.css`: enforce color palette (#090C10, #00F4FF, #30363D, #FF2A55)
- [x] `src-ui/js/app.js`: state machine for PIN lifecycle, connection states, error overlays
- [x] Accessibility and keyboard navigation for HUD controls

### Tests & Quality
- [x] Rust unit tests for core modules (`pin`, `encoder` mocks, `uinput` abstractions)
- [x] Integration tests: simulate WebRTC SDP exchange with local signal server

## Phase 3: PChome Mobile (Android)

### Permissions & Services
- [x] `AndroidManifest.xml`: request `AccessibilityService`, `INTERNET`, `FOREGROUND_SERVICE`, `MediaProjection`
- [x] `PERMISSIONS.md`: document runtime flow and how to enable Accessibility + MediaProjection

### Services & Encoding
- [x] `ScreenCaptureService.java`: MediaProjection + MediaCodec hardware encoder pipeline
- [x] `AndroidControlService.java`: AccessibilityService-based input injector and gesture simulator
- [x] `WebRtcClient.java`: WebRTC PeerConnection, video render surface, DataChannel control
- [x] `SignalClient.java`: resilient WebSocket connection with reconnect/backoff

### UI (HUD mobile)
- [x] `PinActivity.java`: PIN entry UI with HUD styling and timeout handling
- [x] `TouchpadActivity.java`: touchpad + multi-finger gestures, hotkeys overlay
- [x] `DisplayActivity.java`: low-latency SurfaceView/GL renderer for remote stream
- [x] Resource polish: `colors.xml`, `styles.xml`, responsive layouts

### Testing
- [x] Instrumented UI tests for `PinActivity` and `TouchpadActivity` (Espresso/UiAutomator)
- [x] Emulated performance tests for MediaCodec pipeline

## Phase 4: PChome Signal (Go)

### Core Server
- [x] `main.go`: HTTP endpoints for health, metrics, and WebSocket handshake route
- [x] `internal/room/manager.go`: 6-digit room mapping, TTL eviction, concurrency safe
- [x] `internal/signal/websocket.go`: relay of SDP and ICE candidates; auth via ephemeral PIN
- [x] Add metrics (Prometheus) and structured logging

### Deployment
- [x] `Dockerfile`: multi-stage build, small runtime image
- [x] Provide basic `docker-compose.yml` for local dev

## Phase 5: Testing, Perf & Monitoring

- [x] End-to-end test harness: automated WebRTC handshake + media loopback tests
- [x] Performance benchmark: measure capture→encode→decode roundtrip, target <40ms median
- [x] CI perf checks: run benchmarks on PRs for regressions
- [x] Add logging, tracing, and Prometheus metrics for latency, bitrate, errors

## Phase 6: Security & Release

- [ ] Threat model and attacker surface (document in `SECURITY.md`)
- [ ] TLS for signaling, rate-limiting PIN brute-force, PIN TTL enforcement
- [ ] Release automation: semantic versioning, changelog generation, GitHub Releases and Docker tags
- [ ] Post-release checklist: publish docs, update package artifacts, kitchen-sink smoke tests

## Developer Experience & Onboarding

- [ ] `README.md`: quickstart for dev (local signal server, run desktop in dev mode, test mobile with emulator)
- [ ] Developer scripts: `scripts/dev-setup.sh`, `scripts/run-local.sh`
- [ ] CONtributing and PR templates; code owners

## Short-Term Milestones (first 4 weeks)

1. Repo init + CI, docs scaffold (week 1)
2. Minimal signal server + PIN lifecycle (week 2)
3. Desktop daemon prototype: PIN gen + headless pipewire capture stub (week 3)
4. Android minimal client: PIN entry + signal handshake (week 4)

---

If istersen bu genişletilmiş içeriği doğrudan `TODO.md` üzerinde daha da ayrıntılı alt görevlere bölüp, her maddeye `assignee`/`estimate`/`priority` ekleyebilirim.

## Commit Checklist — files created in this session
Each checked box corresponds to a created file included in the combined commit below.

- [x] `.gitignore`
- [x] `.editorconfig`
- [x] `CODEOWNERS`
- [x] `.github/workflows/ci.yml`
- [x] `pchome-desktop/Cargo.toml`
- [x] `pchome-desktop/src/main.rs`
- [x] `pchome-desktop/src/pin.rs`
- [x] `pchome-desktop/src/uinput.rs`
- [x] `pchome-desktop/src/pipewire.rs`
- [x] `pchome-desktop/src/encoder.rs`
- [x] `pchome-desktop/src/network/mod.rs`
- [x] `pchome-desktop/src/network/socket.rs`
- [x] `pchome-desktop/src/network/webrtc.rs`
- [x] `pchome-desktop/src-ui/index.html`
- [x] `pchome-desktop/src-ui/styles/hud.css`
- [x] `pchome-desktop/src-ui/js/app.js`
- [x] `.github/PULL_REQUEST_TEMPLATE.md`
- [x] `.github/dependabot.yml`
- [x] `.pre-commit-config.yaml`
- [x] `pchome-signal/.golangci.yml`
- [x] `CONTRIBUTING.md`
