# PChome — Mevcut Durum Tespiti ve Eksiksiz Tamamlama Planı

> Bu belge, depodaki **tüm** kaynak dosyaları okunup analiz edildikten sonra
> hazırlanmıştır. Amacı: projenin gerçekte ne kadarının çalıştığını, hangi
> parçaların "görünürde var ama boş" (stub/placeholder) olduğunu ve bunları
> **uçtan uca çalışan** bir sisteme dönüştürmek için neyin, nerede, nasıl
> yapılması gerektiğini en ince ayrıntısına kadar listelemektir.

---

## 0. Özet (Çok Kısa)

Proje **üç modülden** oluşuyor ama hepsi **aynı monorepo** içinde:

| Modül | Dil | Gerçekten Çalışan | Sadece İskelet / Boş |
|-------|-----|-------------------|----------------------|
| `pchome-signal` | **Rust** (docs "Go" diyor, yanlış) | WebSocket PIN relay — çalışır | TTL eviction, rate-limit, /health, /metrics, TLS — yok |
| `pchome-desktop` | Rust + egui | PIN üretimi, ffmpeg+pipewire ekran yakalama, H.264 encode, /dev/uinput enjeksiyonu, WebRTC offerer, egui GUI — **çalışır** | `pipewire.rs` DMA-BUF stub (kullanılmıyor), `LIVE PREVIEW` placeholder, `metrics::serve` çağrılmamış, `local_ip/remote_ip/ping_ms` hiç set edilmiyor |
| `pchome-mobile` | Android/Java | `WebRtcClient`, `SignalClient`, `DisplayActivity`, `KeyCodeMap`, `NeonKeyboard`, `ReticleView`, `PinActivity` — **kod mevcut** | APK henüz build edilmemiş; `TouchpadActivity` yok (layout'u var); `MediaProjectionHolder` boş; `WebRtcClient` transceiver/stream çakışması |

**Sonuç:** Sistemin çekirdeği (sinyal → WebRTC → ekran yakala → telefona gönder → telefondan gelen dokunuşu uinput'a bas) **kod olarak yazılmış** ve teorik olarak çalışmalıdır. Ancak:

1. Hiçbir yerde **uçtan uca test** yapılmamış.
2. `src-ui/` içindeki **web HUD tamamen hayalet**: `/api/*` endpoint'lerini çizen bir backend yok.
3. `README.md` / `BUILD.md` / `ARCHITECTURE.md` "Go" diyor ama signal **Rust**.
4. Mobil APK build edilmemiş; emülatör/cihazda denenmemiş.
5. `pipewire.rs`, `pipewire_real.rs`, `webrtc.rs` (desktop kök) stub ama **kullanılmıyor** — kafa karıştırıyor.
6. `metrics`, `local_ip`, `remote_ip`, `ping_ms` state alanları atıl durumda.

Aşağıda **adım adım ne yapılacağı** var.

---

## 1. Şu An Gerçekten Çalışanlar (Kanıtlı)

### 1.1 Signal Server (`pchome-signal/src/main.rs`)
- `tokio-tungstenite` ile gerçek WS sunucusu. `:8080/ws?pin=<6hane>&role=desktop|mobile` kabul eder.
- `Registry` içinde `HashMap<String, Room>`; `Room { desktop: Tx, mobile: Tx }`.
- Bir peer'den gelen metni **sadece diğer role'e** iletir (echo yok). (`main.rs:127-138`)
- Peer ayrılınca oda temizlenir. (`main.rs:142-153`)
- Test: `tests/relay.rs` gerçek binary'yi başlatıp desktop↔mobile relay'i doğruluyor.

### 1.2 Desktop — Ekran Yakalama + Encode (`encoder.rs`)
- `H264Capture::spawn()` **gerçek ffmpeg süreci** başlatır.
- Backend tespiti: `/dev/dri/renderD128` → VA-API, `/dev/nvidia0` → NVENC, yoksa `libx264`. (`encoder.rs:14-23`)
- ffmpeg argümanları: `-f pipewire -i default ... -c:v h264_vaapi/h264_nvenc/libx264 -f h264 -`. (`encoder.rs:119-193`)
- `next_frame()` Annex-B H.264 NAL'larını start-code'a göre ayırır, keyframe (SPS) tespiti yapar. (`encoder.rs:75-110`)

### 1.3 Desktop — Giriş Enjeksiyonu (`uinput.rs`)
- `/dev/uinput` açılır, `UI_SET_EVBIT/KEYBIT/RELBIT/ABSBIT` ioctl'leriyle gerçek sanal cihaz kurulur. (`uinput.rs:104-148`)
- `move_absolute`, `move_relative`, `click`, `double_click`, `wheel`, `key` → kernel'e `input_event` yazar. (`uinput.rs:160-235`)
- `EINTR` için retry var. (`uinput.rs:222-232`)

### 1.4 Desktop — Kontrol Yönlendirme (`control.rs`)
- Mobilten gelen JSON (`move_abs`/`move_rel`/`click`/`scroll`/`key`) → `UInputDevice` metotlarına map edilir. (`control.rs:20-89`)

### 1.5 Desktop — WebRTC (`network/webrtc.rs`)
- `webrtc` crate (v0.12) ile **gerçek PeerConnection**; offerer tarafı.
- `video/H264` track eklenir (packetization-mode=1). (`webrtc.rs:52-66`)
- `control` adında **unordered/unreliable** DataChannel oluşturulur. (`webrtc.rs:69-74`)
- `on_message` → `ControlHandler::handle`. (`webrtc.rs:108-117`)
- `run()` hello alınca offer üretir, answer/ice-candidate uygular. (`webrtc.rs:132-189`)

### 1.6 Desktop — GUI (`gui.rs`, egui)
- Gerçek pencere: PIN, STATUS, LOCAL/REMOTE IP, PING, FPS, LIVE PREVIEW (placeholder), INPUT MANAGER, LIVE EVENT STREAM, TERMINATE butonu. (`gui.rs:24-128`)
- `state.events` üzerinden canlı olay akışı gösteriyor.

### 1.7 Mobile — Çekirdek Sınıflar
- `PinActivity.java`: sunucu + PIN girişi, `DisplayActivity`'ye intent ile geçer.
- `SignalClient.java`: Java-WebSocket ile reconnect/backoff'lu istemci.
- `WebRtcClient.java`: PeerConnection, offer/answer/ICE, `SurfaceViewRenderer` sink, `control` DataChannel.
- `DisplayActivity.java`: dokunma → `move_abs`/`move_rel`/`click`/`scroll`/`key` JSON → `sendControl()`.
- `KeyCodeMap.java`, `NeonKeyboard.java`, `ReticleView.java`: tam kod.

**Yani:** Masaüstü programı derlenip root ile çalıştırılırsa PIN basar, signal'e bağlanır, ekranı ffmpeg ile yakalar, WebRTC ile yayınlar; telefon doğru bağlanırsa ekranı görür ve dokununca masaüstünde fare/klavye oynar. Teorik olarak **çalışan bir P2P uzaktan kontrol var**.

---

## 2. Şu An ÇALIŞMAYAN / BOŞ / STUB Olanlar

### 2.1 `src-ui/` — Hayalet Web HUD
- `index.html` + `app.js` + `hud.css` var ama `app.js` şu endpoint'leri çağırıyor:
  - `POST /api/pin/generate`
  - `POST /api/connection/connect`
  - `GET  /api/status`
- **Bu endpoint'leri sunan hiçbir backend yok.** Desktop ne bir HTTP server açıyor ne de bu yolları işliyor.
- Ayrıca masaüstünde **egui GUI zaten var** (`gui.rs`). İki ayrı UI çakışıyor/gereksiz.
- **Karar gerekiyor:** Ya web HUD tamamen silinir, ya da desktop'a küçük bir HTTP server eklenir (aşağıda 4.1).

### 2.2 `pipewire.rs` / `pipewire_real.rs` / kök `webrtc.rs` — Atıl Stublar
- `pchome-desktop/src/pipewire.rs`: `capture_dmabuf()` sadece `Err` döndürüp fallback'e düşüyor; `capture_memory()` gökkuşağı gradient çiziyor (ekran değil). **Ama `main.rs` bu modülü kullanmıyor** — ekran yakalama `encoder.rs` (ffmpeg) üzerinden oluyor.
- `pipewire_real.rs`: "not compiled in" diyerek `bail!` ediyor.
- Kök `pchome-desktop/src/webrtc.rs`: "not compiled in" stub. Asıl WebRTC `network/webrtc.rs`'de.
- **Bunlar kafa karıştırıyor.** Silinmesi veya açıkça "unused, legacy" olarak işaretlenmesi lazım.

### 2.3 `metrics` Atıl
- `metrics/mod.rs` içinde `serve(addr)` fonksiyonu var ama `main.rs` hiç çağırmıyor → `:9091/metrics` çalışmıyor.
- `config.metrics_addr` okunuyor ama kullanılmıyor.

### 2.4 State Alanları Set Edilmiyor
- `state.local_ip`, `state.remote_ip`, `state.ping_ms` başlangıçta boş/0 kalıyor; GUI hep "LOCAL: " "REMOTE: " "PING: 0 ms" gösterir.
- `state.resolution` sadece başta config'ten set ediliyor; gerçek negotye edilen çözünürlüğü yansıtmaz.

### 2.5 Mobile — `TouchpadActivity` Yok
- `ARCHITECTURE.md` "TouchpadActivity" diyor; `res/layout/activity_touchpad.xml` var ama karşılık gelen Java sınıfı yok.
- `DisplayActivity` zaten `MODE_TRACKPAD` içeriyor, yani ayrı activity gereksiz. `activity_touchpad.xml` silinebilir ya da `DisplayActivity` onu kullanacak şekilde düzenlenebilir.

### 2.6 Mobile — `WebRtcClient` Transceiver/Stream Çakışması
- `createPeerConnection()` içinde:
  - `addTransceiver(VIDEO, RECV_ONLY)` ekleniyor (`WebRtcClient.java:253-259`)
  - Hemen ardından `addStream(stream)` ile (boş) local stream ekleniyor (`WebRtcClient.java:261-268`)
- Local projection olmadığı için local video track null; yine de `addStream` Unified Plan'da fazladan m-line üretebilir ve SDP negotiation'ı bozabilir. **Düzeltme:** Sadece `RECV_ONLY` transceiver yeterli; `addStream` çağrısı kaldırılmalı (aşağıda 4.3).

### 2.7 Signal Server — Eksik Üretim Özellikleri
- **TTL eviction yok:** Spec 300s diyor; `Registry` sadece peer ayrılınca oda siliniyor. PIN sonsuza kadar kalabilir.
- **Rate limiting yok:** `docker-compose.yml` `RATE_LIMIT=20` diyor ama kod okumuyor.
- **`/health`, `/metrics` yok:** `docker-compose` healthcheck `/health` bekliyor ama endpoint yok → container hep unhealthy.
- **TLS yok:** `docker-compose` 8443/`wss` ima ediyor, kod plain `ws://0.0.0.0:8080`.
- **CORS yok:** web HUD farklı origin'den bağlansa engellenir (zaten web HUD çalışmıyor).

### 2.8 Docs vs Gerçeklik Tutarsızlıkları
- `README.md`, `BUILD.md`, `DECISIONS.md`, `TODO.md` signal'in **Go** olduğunu söylüyor → aslında **Rust**.
- `BUILD.md` "go mod tidy / go run main.go" diyor → `cargo build` olmalı.
- `Makefile` signal için `go mod tidy && go build` diyor → `cargo build` olmalı.
- `run.sh` / `scripts/run-local.sh` signal'i `cargo run` ile başlatıyor (doğru) ama README "go run" diyor (yanlış).

### 2.9 Mobil APK Build Edilmedi
- `pchome-mobile/app/build/outputs/` boş (sadece `intermediates` var).
- `gradlew` izni/bağımlılıkları henüz çalıştırılmamış.

---

## 3. Uçtan Uca Akışın Gerçek Durumu (Doğru mu?)

```
[Desktop]                         [Signal]                    [Mobile]
PIN üret (pin.rs)  ───WS register──▶  oda aç (pin,role)       
WebRTC build (offerer)            
ffmpeg yakala+encode ─┐                                             
                       ├──▶ H.264 video track (P2P) ──────────▶ SurfaceViewRenderer
uinput açık           │                                             
control DC dinle  ◀──┘  kontrol JSON (P2P) ◀── touch (DisplayActivity)
hello gelince offer ──▶ relay ──▶ answer+ICE ──▶ relay ──▶ apply
```

**Akış mantıken doğru.** Teknik riskler:
- ffmpeg sistemde kurulu değilse encoder prosesi exit olur → `capture frame error` loglanır, stream durur.
- `/dev/uinput` root gerektirir; root değilse `UInputDevice::open` patlar → daemon erken döner (main.rs:106-109).
- Mobil tarafta transceiver/stream çakışması SDP'yi bozabilir (2.6).
- Signal'de TTL yok ama bu ilk bağlantıyı engellemez.

---

## 4. Yapılması Gerekenler (Öncelikli Sıra)

### P0 — Sistemi Gerçekten Çalıştırıp Test Etme

#### 4.0 Ön gereksinimler (sistem)
```bash
# ffmpeg (pipewire destekli) kurulu olmalı
ffmpeg -version | grep -- "--enable-libpipewire"

# /dev/uinput var mı
ls -l /dev/uinput

# root gerekiyor (uinput için)
sudo -v
```

#### 4.1 Karar: Web HUD mı, egui GUI mi?
**Öneri:** `src-ui/` tamamen silinsin (veya `deprecated/` altına taşınsın). Masaüstü zaten egui GUI'de PIN gösteriyor. Böylece "boş arayüz" algısı kalkar.
- Sil: `pchome-desktop/src-ui/index.html`, `src-ui/js/app.js`, `src-ui/styles/hud.css`.
- `README.md` ve `run.sh` içindeki `xdg-open .../index.html` satırlarını kaldır.

Alternatif (web HUD isteniyorsa): desktop'a `warp`/`axum` HTTP server ekle ve `/api/pin/generate`, `/api/connection/connect`, `/api/status`, `/metrics` yollarını `SharedState` üzerinden sun. (Ek iş; P0 değil.)

#### 4.2 Desktop `main.rs` — Eksik çağrıları ekle
`daemon()` içinde, `ConnectionManager::new` sonrası ve `WebRtcEngine::build` sonrası şunları yap:

```rust
// metrics endpoint'ini başlat
crate::metrics::serve(config.metrics_addr);

// Gerçek IP'leri doldur (örnek; signal sunucuya bağlanınca alınabilir)
if let Ok(ip) = local_ip::get_local_ip() {  // ya da basit:
    *state.local_ip.lock().unwrap() = local_ip_or_unknown();
}
```

`state.ping_ms` ve `remote_ip`, WebRTC `on_ice_connection_change` / periodic ping ile güncellenmeli. Basit başlangıç: `WebRtcEngine::run` içinde her 5sn `connection.send_json({"type":"ping"})` gönder, mobil `pong` döndürsün, desktop'ta zamanı ölçüp `state.ping_ms`e yaz.

#### 4.3 Mobile `WebRtcClient.java` — Transceiver/Stream düzeltmesi
`createPeerConnection()` içinde `addStream(stream)` bloğunu **kaldır** (local projection yok):

```java
// ÖNCEKİ (yanlış):
// MediaStream stream = factory.createLocalMediaStream("ARDAMS");
// if (localVideoTrack != null) stream.addTrack(localVideoTrack);
// if (localAudioTrack != null) stream.addTrack(localAudioTrack);
// peerConnection.addStream(stream);

// SONRAKİ (doğru): sadece recvonly transceiver yeterli
// (zaten yukarıda addTransceiver RECV_ONLY var)
```

Ayrıca `startLocalMedia()` içindeki `createVideoCapturer()` her zaman çağrılıyor; projection yoksa null dönüyor, bu OK. Ama `addStream` kaldırılınca `localVideoTrack`/`localAudioTrack` zaten eklenmiyor → sorun yok.

#### 4.4 Atıl stub dosyalarını temizle
- Sil: `pchome-desktop/src/pipewire.rs`, `pchome-desktop/src/pipewire_real.rs`, `pchome-desktop/src/webrtc.rs` (kök).
- `main.rs` içindeki `mod pipewire;` ve `mod webrtc;` (kök) satırlarını kaldır; `use crate::encoder::H264Capture` zaten doğru yolda.
- `ARCHITECTURE.md` ve `TODO.md` içindeki "DMA-BUF zero-copy PipeWire crate" ifadelerini güncelle: "ekran yakalama ffmpeg+pipewire ile yapılıyor; DMA-BUF stub kaldırıldı".

#### 4.5 Mobil APK build et ve test et
```bash
cd pchome-mobile
chmod +x gradlew
./gradlew assembleDebug
# Çıktı: app/build/outputs/apk/debug/app-debug.apk
# Emülatör veya USB cihaza yükle:
adb install -r app/build/outputs/apk/debug/app-debug.apk
```
Emülatörde `PinActivity` açılır; signal URL + PIN girilip Connect'e basılır.

---

### P1 — Signal Server'ı Üretime Hazırla

#### 4.6 TTL Eviction Ekle (`pchome-signal/src/main.rs`)
`Registry` içine `created_at: Instant` ve periyodik temizleyici ekle:
```rust
struct Room {
    desktop: Option<Tx>,
    mobile: Option<Tx>,
    created_at: std::time::Instant,
}
// 10sn'de bir: created_at + 300s geçen odaları sil
```

#### 4.7 `/health` ve `/metrics` HTTP endpoint'i
`Listener`'dan önce basit bir TCP/HTTP sunucu ya da `tokio::spawn` ile ayrı bir `hyper`/`tiny_http` listener:
```rust
// GET /health -> 200 OK
// GET /metrics -> room sayısı vs (basit text)
```

#### 4.8 Rate Limit (basit)
Aynı IP'den 1sn'de N'den fazla yeni WS bağlantısı → 429 benzeri kapat. `RATE_LIMIT` env'ini oku.

#### 4.9 TLS (opsiyonel ama docs istiyor)
Railway/Render zaten TLS sonlandırıyor; container plain `ws` kalabilir. Sadece README'de "TLS sonlandırma proxy'de" diye net yaz.

---

### P2 — Polis / UX / Docs

#### 4.10 Docs düzeltmesi (Go → Rust)
- `README.md`: "Go 1.23+" → "Rust 1.70+"; `go run main.go` → `cargo run`.
- `BUILD.md`: `go mod tidy` satırlarını sil, `cd pchome-signal && cargo build --release` yaz.
- `Makefile`: `cd pchome-signal && go mod tidy && go build ./...` → `cd pchome-signal && cargo build`.
- `DECISIONS.md` ADR-001: "Signal Server: Go" → "Signal Server: Rust (tokio-tungstenite)".
- `TODO.md` Phase 4 başlığını "PChome Signal (Rust)" yap.

#### 4.11 `calistirma-rehberi.txt` güncelle
Mevcut rehber doğru ama şunu ekle:
- ffmpeg gerektiği, `sudo` şartı, APK build adımı, signal'in `cargo run` ile başlatılması.

#### 4.12 GUI LIVE PREVIEW gerçeğe dönüştür (opsiyonel)
Masaüstü penceresinde kendi ekranını göstermek **gerekli değil** (telefon gösteriyor). İstersen egui'ye son H.264 kareyi decode edip gösteren küçük bir panel eklenebilir ama P0 değildir.

#### 4.13 `TouchpadActivity` kararı
`activity_touchpad.xml` kullanılmıyorsa sil, ya da `DisplayActivity` içindeki trackpad modunu ayrı bir activity'ye taşı. Şu an `DisplayActivity` her ikisini de içerdiği için sorun yok; sadece gereksiz dosya.

---

## 5. Eksik / Eklenmesi Gereken Kod Parçaları (Taslak)

### 5.1 Desktop `main.rs` metrics + IP doldurma (ekle)
`daemon()` içinde, `connection.connect(pin)` bloğundan hemen sonra:
```rust
// metrics HTTP sunucusunu başlat
crate::metrics::serve(config.metrics_addr);

// yerel IP'yi öğren (basit yardımcı)
fn first_non_loopback_ip() -> String {
    use std::net::UdpSocket;
    let s = UdpSocket::bind("0.0.0.0:0").ok();
    if let Some(s) = s {
        if s.connect("8.8.8.8:80").is_ok() {
            if let Ok(addr) = s.local_addr() {
                return addr.ip().to_string();
            }
        }
    }
    "127.0.0.1".into()
}
*state.local_ip.lock().unwrap() = first_non_loopback_ip();
```

### 5.2 Desktop `WebRtcEngine` ping/pong (ekle)
`build()` içinde `dc.on_message`'a ping/pong ve `display_info` yanıtı ekle:
```rust
let msg_type = v.get("type").and_then(|x| x.as_str()).unwrap_or("");
if msg_type == "pong" {
    // round-trip hesapla, state.ping_ms'e yaz
}
```
ve `run()` içinde periyodik `send_json({"type":"ping","t":now_ms})`.

### 5.3 Mobile `WebRtcClient` pong yanıtı (ekle)
`handleSignalMessage` benzeri bir yerde `onDataChannelMessage` içinde:
```java
if ("ping".equals(type)) {
    JSONObject pong = new JSONObject();
    pong.put("type", "pong");
    pong.put("t", message.optLong("t"));
    sendControl(pong.toString());
}
```

### 5.4 Signal `/health` (ekle)
`Listener` döngüsünden önce ayrı bir `tokio::spawn` ile:
```rust
tokio::spawn(async move {
    let listener = std::net::TcpListener::bind("0.0.0.0:8081").unwrap();
    for stream in listener.incoming().flatten() {
        let mut s = stream;
        let body = "OK";
        let resp = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}", body.len(), body);
        let _ = s.write_all(resp.as_bytes());
    }
});
```
(Not: port çakışmaması için 8081 kullanıldı; docker-compose healthcheck'i buna göre güncellenmeli.)

---

## 6. Doğru Build & Run Sırası

```bash
# 1) Signal server (Rust)
cd pchome-signal
cargo build --release
./target/release/pchome-signal &   # PORT=8080 (varsayılan)

# 2) Desktop daemon (Rust, root gerekir)
cd pchome-desktop
cargo build --release
sudo env PCHOME_SIGNAL_URL=ws://127.0.0.1:8080/ws \
     ./target/release/pchome-desktop
# Terminalde "Registered PIN: 123456" ve egui penceresi açılır

# 3) Mobile APK (ayrı terminal)
cd pchome-mobile
./gradlew assembleDebug
adb install -r app/build/outputs/apk/debug/app-debug.apk

# 4) Telefonda PChome aç → sinyal URL = ws://<bilgisayar-ip>:8080/ws
#    PIN = 123456 → Connect
#    Birkaç sn sonra bilgisayar ekranı telefonda belirir.
```

**Aynı ağ dışı ("her yerden") kullanım:** Signal'i Railway/Render'a deploy et (`pchome-signal/Dockerfile` hazır), desktop'a `PCHOME_SIGNAL_URL=wss://<railway-url>/ws` ver, telefona aynı URL'yi yaz. STUN (`stun.l.google.com:19302`) zaten her iki tarafta tanımlı.

---

## 7. Test Planı (uçtan uca)

1. **Unit (desktop):** `cargo test -p pchome-desktop` → `pin.rs` testleri, `encoder` start-code ayrıştırma.
2. **Integration (signal):** `cargo test -p pchome-signal` → `tests/relay.rs` desktop↔mobile relay.
3. **Manual e2e:**
   - [ ] Desktop PIN basıyor mu? (egui + terminal log)
   - [ ] Signal bağlantı durumu "SIGNAL: OK" mı?
   - [ ] Mobil Connect sonrası ekran geliyor mu?
   - [ ] Mobil dokunma → masaüstünde fare hareket ediyor mu?
   - [ ] Mobil klavye → masaüstüne karakter gidiyor mu?
   - [ ] `state.events` (egui LIVE EVENT STREAM) olayları gösteriyor mu?
   - [ ] TERMINATE butonu süreci kapatıyor mu?
4. **Perf:** ffmpeg pipeline + WebRTC ile median <40ms hedefleniyor; `metrics` (/metrics) ile FPS/bitrate izlenir.

---

## 8. Kontrol Listesi (Tamamlandığında)

- [ ] `src-ui/` silindi VEYA desktop HTTP backend ile besleniyor
- [ ] `pipewire.rs`, `pipewire_real.rs`, kök `webrtc.rs` stubları silindi
- [ ] `metrics::serve()` `main.rs`'ta çağrılıyor
- [ ] `local_ip` / `ping_ms` state'leri doluyor
- [ ] Mobile `addStream` çağrısı kaldırıldı (transceiver-only)
- [ ] Mobile APK build edildi, cihaza yüklendi
- [ ] Signal'e TTL eviction + `/health` eklendi
- [ ] Docs (Go→Rust) düzeltildi
- [ ] `calistirma-rehberi.txt` ffmpeg+sudo+APK adımlarıyla güncellendi
- [ ] Uçtan uca test (ekran görünümü + dokunma + klavye) başarılı

---

## 9. Kısa Not — "Neden şu an sadece arayüz varmış gibi görünüyor?"

Çünkü:
1. **Web HUD (`src-ui`) boş backend'e bağlı** → tarayıcıda açınca hiçbir şey yapmaz.
2. **egui GUI gerçek ama LIVE PREVIEW'sı placeholder** → "ekran yansıması yok" izlenimi verir (oysa asıl yansıma telefonda).
3. **Mobil APK hiç build edilmediği için** telefon tarafı hiç denenmedi.
4. **Stub dosyalar (`pipewire.rs` vb.)** kodu okuyanı "yarım" sanmaya iter.

Yukarıdaki P0 adımları uygulandığında sistem **gerçekten çalışan** bir uzaktan kontrole dönüşür.

Özet: Sistemin çekirdeği yazılmış ve teorik olarak çalışır; "sadece arayüz" izlenimi hayalet web HUD, placeholder preview, build edilmemiş APK ve atıl stub dosyalarından kaynaklanıyor. P0 maddelerini uygulayıp APK'yi build edince uçtan uca çalışır hale gelir.
