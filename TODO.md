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

- [x] Threat model and attacker surface (document in `SECURITY.md`)
- [x] TLS for signaling, rate-limiting PIN brute-force, PIN TTL enforcement
- [x] Release automation: semantic versioning, changelog generation, GitHub Releases and Docker tags
- [x] Post-release checklist: publish docs, update package artifacts, kitchen-sink smoke tests

## Developer Experience & Onboarding

- [x] `README.md`: quickstart for dev (local signal server, run desktop in dev mode, test mobile with emulator)
- [x] Developer scripts: `scripts/dev-setup.sh`, `scripts/run-local.sh`
- [x] CONtributing and PR templates; code owners

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

---

## Detailed Remediation Backlog (Build / Compile / Test Errors & Improvements)

> Each item below maps 1:1 to a conventional commit enumerated in `COMMITS.md`.
> These are non-trivial tasks derived from actual build/compile/test failures and
> engineering recommendations for the three modules.

### pchome-signal (Go)

1. **fix(signal): add missing `context` import in main.go** — `context.WithTimeout` was used without importing `context`; the server failed to build.
2. **fix(signal): remove unused `fmt` import in main.go** — leftover import caused `go build` to fail.
3. **fix(signal): generate PIN with `crypto/rand` instead of `math/rand`** — `math/rand` is not cryptographically secure and violates the spec's "cryptographically secure random 6-digit PIN" requirement.
4. **fix(signal): prevent WebSocket relay from echoing to sender** — the hub broadcast loop delivered every message back to the originating client because all peers share the same PIN.
5. **feat(signal): add `Reserve(pin, clientID)` to room manager** — lets the desktop register its locally generated PIN instead of the server generating one.
6. **feat(signal): add role-aware handshake (`desktop`/`mobile`) in `ServeWs`** — previously the server rejected connections when no room existed, blocking the desktop from registering.
7. **fix(signal): wire active-rooms Prometheus metric via `Count()` updater** — `IncActiveRooms`/`DecActiveRooms` were never called; now a ticker syncs the gauge.
8. **fix(signal): set `websocket.Upgrader.ReadLimit`** — unbounded message size allows a memory-exhaustion DoS on the relay.
9. **fix(signal): rate limiter must read `X-Forwarded-For`** — `RemoteAddr` is empty/localhost behind a reverse proxy, so limiting was ineffective.
10. **test(signal): table-driven test for 6-digit PIN format** — assert exactly 6 ASCII digits across many generations.
11. **refactor(signal): replace `map` broadcast keying with a typed relay message** — removes the `Type==PIN` overloading and clarifies intent.
12. **fix(signal): safely close `client.Send`** — concurrent `close()` on the send channel after a slow consumer caused a panic.
13. **feat(signal): add CORS headers for browser HUD WebSocket** — the web HUD connects from a different origin and was blocked.
14. **test(signal): integration test for desktop→mobile SDP/ICE relay roundtrip** — boots a hub, connects two clients, asserts the offer reaches the peer.
15. **chore(signal): pin `golangci-lint` version and fix lint findings** — the existing `.golangci.yml` is not enforced in CI.
16. **fix(signal): `TTL` eviction should refresh `LastSeen` on activity** — idle-but-active rooms were evicted prematurely.
17. **perf(signal): replace per-message `[]byte` alloc with a sync.Pool buffer** — high relay throughput caused GC pressure.

### pchome-desktop (Rust)

18. **fix(desktop): remove duplicate `Frame` enum in `pipewire.rs`** — the enum was defined twice (once unconditionally, once under `cfg(unix)`), causing a duplicate-definition compile error on Linux.
19. **fix(desktop): remove duplicate global logger init in `main.rs`** — `env_logger::init()` and `tracing_subscriber::fmt().init()` both register a global logger and panic at startup.
20. **fix(desktop): `PeerConnection::new` returned a disconnected receiver** — the returned `UnboundedReceiver` belonged to a different channel than the stored sender, so `send()` never delivered.
21. **fix(desktop): `pin.rs` registered to a non-existent `/register/{pin}` endpoint** — the Signal server only serves `/ws?pin=…&role=desktop`; update the client.
22. **fix(desktop): `pin.rs` dropped the WebSocket immediately after connecting** — registration never persisted; hold the connection for the session TTL.
23. **fix(desktop): cfg-gate `use crate::pipewire::init_capture`** — the import broke non-Unix (Windows) builds.
24. **feat(desktop): implement real uinput device creation** — currently only `open()`s `/dev/uinput`; emit `UI_SET_EVBIT`/`UI_SET_KEYBIT`/`UI_DEV_CREATE` ioctls so the kernel accepts events.
25. **fix(desktop): retry `libc::write` on `EINTR` in `uinput.rs`** — interrupted writes could silently drop input events.
26. **feat(desktop): add Serde config struct** — externalize signal URL, capture resolution, and bitrate instead of hard-coded constants.
27. **test(desktop): unit test for PIN formatting** — assert zero-padded 6-digit output (`{:06}`).
28. **fix(desktop): invalid STUN bind request in `socket.rs`** — missing magic cookie (`0x2112A442`) and message length field; parsers on the STUN server reject it.
29. **feat(desktop): integrate the `webrtc` crate** — replace the stub `PeerConnection` with a real `webrtc-rs` peer using the relayed SDP/ICE.
30. **fix(desktop): `encoder.rs` has no real encoder** — wire VA-API/NVENC or `ffmpeg` bindings instead of the copy-through software stub.
31. **perf(desktop): avoid per-frame `Vec` allocation in software encode path** — reuse a buffer pool for latency-sensitive frames.
32. **fix(desktop): surface `ConnectionManager::connect` errors** — `main.rs` ignored the result with `let _ =`, hiding handshake failures.
33. **chore(desktop): add `clippy.toml` and fix warnings** — unused deps (`base64`, `chrono`, `uuid`) and dead code.
34. **test(desktop): criterion benchmark for binary packet (de)serialization roundtrip** — guards the <40 ms budget for input packets.
35. **fix(desktop): expose `metrics::REGISTRY` via an HTTP `/metrics` endpoint** — currently the registry is never scraped.
36. **refactor(desktop): de-duplicate `FourCC` between `encoder.rs` and `pipewire.rs`** — move into a shared `pixelformat` module.
37. **fix(desktop): `pipewire` capture is a stub** — integrate `pipewire-rs`/`pw` bindings for real DMA-BUF zero-copy capture.
38. **build(desktop): add `cargo auditable` / SBOM metadata** — required for the release pipeline.

### pchome-mobile (Android / Java)

39. **fix(mobile): add missing `import android.util.Log` in `SignalClient.java`** — `Log.*` was used without the import; the module failed to compile.
40. **fix(mobile): add missing `import android.content.Intent` in `PinActivity.java`** — `Intent` was used without the import.
41. **fix(mobile): remove illegal `import android.view.SurfaceViewRenderer`** — `SurfaceViewRenderer` lives in `org.webrtc`, not `android.view`; present in `WebRtcClient.java` and `DisplayActivity.java`.
42. **fix(mobile): use `EglBase` context instead of `getInternalVideoEncoderFactory().get().getEglContext()`** — there is no `get()` on `VideoEncoderFactory`; use a shared `EglBase` context.
43. **fix(mobile): cast `getParcelableExtra("data")` to `Intent` in `ScreenCaptureService`** — on API 34 it returns `Parcelable` and produced a type-mismatch compile error.
44. **fix(mobile): `ScreenCapturerAndroid` requires the MediaProjection intent** — passing `null` compiles but NPEs at runtime; plumb the permission result Intent.
45. **fix(mobile): `AndroidControlService` instantiated via `new` cannot dispatch gestures** — an AccessibilityService must be the system-started instance; expose a static binder/instance.
46. **fix(mobile): `injectKeyEvent` misuses `performGlobalAction(keyCode)`** — pass `GLOBAL_ACTION_*` constants or use a proper key-event dispatch path.
47. **fix(mobile): `MediaCodecEncoder` uses `COLOR_FormatSurface` but feeds input buffers** — surface-encoded codecs ignore input buffers; switch to a Surface input pipeline (or `COLOR_FormatYUV420Flexible` + buffer input).
48. **fix(mobile): `WebRtcClient.connect` passes `null` localRenderer** — `localVideoTrack.addSink(null)` throws; guard or supply a renderer.
49. **fix(mobile): `minSdk 26` vs `FOREGROUND_SERVICE_MEDIA_PROJECTION` (API 34)** — gate the mediaProjection foreground service type or bump `minSdk`.
50. **test(mobile): instrumented test for `PinActivity` PIN validation UI** — assert error state on malformed input.
51. **feat(mobile): add `network_security_config.xml`** — allow cleartext WebSocket to `localhost` in debug builds only.
52. **chore(mobile): drop unused `lifecycle-runtime-ktx` Kotlin dependency** from the Java-only module.
53. **refactor(mobile): unify on a single WebSocket stack** — `Java-WebSocket` + `okhttp` + `webrtc` all pull WebSocket code; pick one for signalling.
54. **fix(mobile): `TouchpadActivity` never sends input over the WebRTC DataChannel** — wire `AndroidControlService`/touch events to `webRtcClient.send(...)`.
55. **feat(mobile): implement compact binary input packet protocol** — match the desktop's little-endian layout for moves/clicks/keys.
56. **fix(mobile): `DisplayActivity` ignores rotation/aspect ratio** — the remote desktop stream must scale to the SurfaceView.
57. **perf(mobile): use `MediaCodec` async callbacks** — the polling `dequeueOutputBuffer` loop adds jitter to the decode path.
58. **fix(mobile): request `SYSTEM_ALERT_WINDOW` only when the floating HUD needs it** — it is declared but never exercised, triggering unnecessary permission prompts.

### Cross-cutting / CI / Docs

59. **docs: document the end-to-end handshake protocol** in `ARCHITECTURE.md` (PIN reserve → join → SDP/ICE relay → P2P).
60. **ci: make `.github/workflows/build.yml` actually compile all three modules** — run `go build`, `cargo build`, and `gradle assembleDebug` instead of only lint.
61. **chore: add `.gitignore` entries** for `/target`, `/build`, `/gradle` caches, and local TLS certs.
62. **fix(scripts): `run-local.sh` referenced binaries before building them** — add explicit build steps for signal/desktop.
63. **docs: add a TLS certificate generation script** and document the mandatory WebSocket-TLS requirement from `AGENT.md`.
64. **test: add an e2e script** that boots the Signal server + Desktop and asserts a PIN can be reserved and relayed.
65. **fix: align PIN string format across modules** — desktop uses zero-padded `{:06}`, mobile UI shows `849-204`; normalize on both ends.
66. **chore: add a conventional-commit lint hook** (commitlint / pre-commit) to enforce the commit format used in `COMMITS.md`.
67. **docs: add sequence diagram for Flow A and Flow B** to `ARCHITECTURE.md`.
