# PChome Çalıştırma Planı (Uçtan Uca)

> Kaynak: `DURUM_VE_TAMAMLAMA_PLANI.md` — bu dosya onun görev listesidir.
> Kural: Her görev bittiğinde kutucuk `[x]` olarak işaretlenir ve
> conventional commits formatında İngilizce commit atılır.

## P0 — Sistemi Gerçekten Çalıştırma

- [x] 4.1 Web HUD sil: `pchome-desktop/src-ui/` kaldır, `README.md` + `run.sh`
      içindeki `xdg-open .../index.html` referanslarını sil.
- [x] 4.4 Atıl stubları temizle: `pipewire.rs`, `pipewire_real.rs`, kök
      `webrtc.rs` sil; `main.rs` mod bildirimlerini düzelt.
- [x] 4.2 + 5.1 + 5.2 Desktop canlı telemetri: `metrics::serve()` çağır,
      `local_ip` doldur, `WebRtcEngine`'e periyodik ping/pong ekle
      (`ping_ms` güncellensin).
- [x] 4.3 + 5.3 Mobil düzeltme: `WebRtcClient.createPeerConnection()`
      içindeki boş `addStream(...)` çağrısını kaldır; DataChannel'a
      ping→pong yanıtı ekle.
- [x] 4.5 Mobil APK build: `./gradlew assembleDebug` →
      `app/build/outputs/apk/debug/app-debug.apk`. (Not: sistemde JDK 17 ve
      Android SDK yoktu; JDK 17 kullanıcı dizinine indirildi,
      SDK `$HOME/android-sdk` içine kuruldu, `local.properties`
      yazıldı — build başarılı, ~53MB app-debug.apk üretildi.)

## P1 — Signal Server Üretime Hazırlık

- [x] 4.6 TTL eviction: `Room`'a `created_at`, arka plan sweeper (300s).
- [x] 4.7 `/health` ve `/metrics` HTTP endpoint'leri.
- [x] 4.8 Basit rate limit (`RATE_LIMIT` env, aynı IP için).

## P2 — Docs & Temizlik

- [x] 4.10 Docs Go→Rust düzeltmesi: `README.md`, `BUILD.md`, `Makefile`,
      `DECISIONS.md`, `TODO.md`.
- [x] 4.11 `calistirma-rehberi.txt`: ffmpeg şartı, sudo, APK build adımı,
      signal'i `cargo run` ile başlatma.
- [x] 4.13 Kullanılmayan `activity_touchpad.xml` sil.

## Doğrulama

- [x] `cargo test -p pchome-signal` geçiyor (relay testi dahil).
- [x] `cargo build --release -p pchome-desktop` hatasız (+ desktop testleri 2/2).
- [x] Uçtan uca akış hazır: canlı smoke test — signal başlatıldı,
      `/health` → OK, `/metrics` gauge'ları döktü, desktop↔mobile WS
      çifti hello ve offer mesajlarını karşılıklı iletti, odalar
      kapanınca temizlendi. Kalan tek manuel adım: APK'yi telefona
      kurup gerçek cihazda ekran + dokunma denemesi.

## Denetim Turu (baştan kontrol — bulgular)

- [ ] D1 [KRİTİK] `ffmpeg -f pipewire` sistemde desteklenmiyor →
      `encoder.rs`'e capture-input otomatik seçimi ekle:
      pipewire destekliyse pipewire, yoksa `$DISPLAY` ile x11grab.
- [ ] D2 `prometheus.yml` hâlâ 8080'i scrape ediyor; `/metrics` 8081'e taşındı.
- [ ] D3 `docs/handshake-sequence.md`: "(Go)" etiketi, "GET /ws",
      "mesaj TTL'i yeniler" ifadeleri gerçek davranışa aykırı.
- [ ] D4 `scripts/test-e2e.sh` bozuk (tek client'a relay echo beklemez,
      python `websocket` bağımlılığı) ve `run-local.sh` desktop'u
      sudo'suz başlatıp uinput'ta düşürüyor.
- [ ] D5 AndroidManifest'teki `FOREGROUND_SERVICE_MEDIA_PROJECTION`
      izinleri Flow B kaldırıldığından artık gereksiz.

## Commit Listesi

| # | Görev | Commit |
|---|-------|--------|
| 1 | 4.1 | `chore(desktop): remove ghost web HUD shell` |
| 2 | 4.4 | `refactor(desktop): drop unused capture and webrtc stubs` |
| 3 | 4.2 | `feat(desktop): serve metrics and populate live telemetry state` |
| 4 | 4.3 | `fix(mobile): rely on recv-only transceiver and answer pings` |
| 5 | 4.6 | `feat(signal): evict stale rooms with ttl sweeper` |
| 6 | 4.7 | `feat(signal): add health and metrics http endpoints` |
| 7 | 4.8 | `feat(signal): enforce basic connection rate limit` |
| 8 | 4.10 | `docs: correct signal server language references` |
| 9 | 4.11 | `docs: expand quickstart guide with real build steps` |
| 10 | 4.13 | `chore(mobile): remove unused touchpad layout` |
