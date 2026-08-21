# PChome Conventional Commits (Remediation Plan)

> 50 conventional commits (English) corresponding 1:1 to the first 50 items of the
> "Detailed Remediation Backlog" in `TODO.md`. Scope `signal` = pchome-signal,
> `desktop` = pchome-desktop, `mobile` = pchome-mobile.

1. `fix(signal): add missing context import in main.go`
2. `fix(signal): remove unused fmt import in main.go`
3. `fix(signal): generate PIN with crypto/rand instead of math/rand`
4. `fix(signal): prevent websocket relay from echoing messages to the sender`
5. `feat(signal): add Reserve(pin, clientID) to room manager`
6. `feat(signal): add role-aware handshake in ServeWs`
7. `fix(signal): wire active-rooms prometheus metric via Count updater`
8. `fix(signal): set websocket Upgrader ReadLimit to bound message size`
9. `fix(signal): read X-Forwarded-For in rate limiter behind proxies`
10. `test(signal): add table-driven test for 6-digit PIN format`
11. `refactor(signal): use typed relay message instead of PIN-keyed broadcast`
12. `fix(signal): safely close client.Send channel on slow consumer`
13. `feat(signal): add CORS headers for browser HUD websocket`
14. `test(signal): add integration test for SDP/ICE relay roundtrip`
15. `chore(signal): pin golangci-lint and fix lint findings`
16. `fix(signal): refresh LastSeen on activity for TTL eviction`
17. `perf(signal): use sync.Pool buffer for relayed messages`
18. `fix(desktop): remove duplicate Frame enum in pipewire.rs`
19. `fix(desktop): remove duplicate global logger init in main.rs`
20. `fix(desktop): return connected receiver from PeerConnection::new`
21. `fix(desktop): register PIN against /ws?role=desktop endpoint`
22. `fix(desktop): hold websocket connection for the session TTL`
23. `fix(desktop): cfg-gate pipewire init_capture import for non-unix`
24. `feat(desktop): implement real uinput device creation ioctls`
25. `fix(desktop): retry libc::write on EINTR in uinput.rs`
26. `feat(desktop): add Serde config struct for runtime settings`
27. `test(desktop): unit test for zero-padded 6-digit PIN format`
28. `fix(desktop): build valid STUN bind request in socket.rs`
29. `feat(desktop): integrate webrtc-rs PeerConnection`
30. `fix(desktop): implement real VA-API/NVENC encoder path`
31. `perf(desktop): reuse buffer pool in software encode path`
32. `fix(desktop): surface ConnectionManager connect errors`
33. `chore(desktop): add clippy.toml and fix lint warnings`
34. `test(desktop): criterion benchmark for packet roundtrip`
35. `fix(desktop): expose metrics REGISTRY via /metrics endpoint`
36. `refactor(desktop): de-duplicate FourCC into shared pixelformat module`
37. `fix(desktop): integrate pipewire-rs for real DMA-BUF capture`
38. `build(desktop): add cargo auditable SBOM metadata`
39. `fix(mobile): add missing Log import in SignalClient.java`
40. `fix(mobile): add missing Intent import in PinActivity.java`
41. `fix(mobile): remove illegal android.view.SurfaceViewRenderer import`
42. `fix(mobile): use EglBase context for video factories`
43. `fix(mobile): cast getParcelableExtra data to Intent`
44. `fix(mobile): plumb MediaProjection intent into ScreenCapturerAndroid`
45. `fix(mobile): expose AccessibilityService instance via binder`
46. `fix(mobile): correct injectKeyEvent global action usage`
47. `fix(mobile): fix MediaCodecEncoder surface vs buffer input mismatch`
48. `fix(mobile): guard null localRenderer in WebRtcClient.connect`
49. `fix(mobile): gate mediaProjection foreground type by minSdk`
50. `test(mobile): instrumented PinActivity validation test`
