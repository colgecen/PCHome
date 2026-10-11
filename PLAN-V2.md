# PCHome V2 Planı — Kalıcı ID + Son Girilenler + Neon Arayüz + OTA (v0.3.0)

> Durum: TÜM FAZLAR TAMAMLANDI (her faz Türkçe conventional commit + build/test ile kapatıldı).

## Canlı sunucu adresleri

| Ne | Adres |
|---|---|
| Sinyal (WebSocket) | `wss://PCHome.onrender.com/ws` |
| Sağlık | `https://PCHome.onrender.com/health` |
| Ölçümler | `https://PCHome.onrender.com/metrics` |
| Sürüm (v2 sunucuda) | `https://PCHome.onrender.com/version` |

## Fazlar ve kabul kriterleri

### Faz 1 — Sinyal sunucusu v2 ✅ (`64fdecc`)
- Oda anahtarı `pc_id` (kalıcı), eski `pin` geriye uyumlu.
- Kalıcı oda desktop bağlıyken SÜPÜRÜLMEZ; boşken 60 sn'de gider; 24 saat supabı.
- `GET /version` + kalıcı oda metriği. 6 birim + 1 relay testi YEŞİL.

### Faz 2 — Desktop kimlik ✅
- `identity.rs`: ilk açılışta 6 haneli ID + şifre üretir, `~/.config/PCHome/identity.toml` (0600).
- PIN rotasyonu KALDIRILDI; kopunca AYNI ID ile dönülür.
- `join` → sabit-süreli şifre karşılaştırma → `auth_ok` (+cihaz kartı) / `auth_rejected`; 5 yanlışta 60 sn kilit.
- `device.rs`: isim/IP/MAC/çözünürlük kartı. 10 test YEŞİL.

### Faz 3 — Desktop arayüz + güncelleyici ✅
- `theme.rs`: Neon-Mavi jetonlar (mobil `colors.xml` ile aynı hex).
- ID/şifre kartı (göster/gizle/yenile/kopyala) + güncelleme bandı + ayarlar (sinyal URL, PC adı, şifre, denetle).
- `updater.rs`: GitHub `releases/latest` (curl, yeni bağımlılık yok); AppImage yanına / tarball üzerine kurulum + `.bak`.

### Faz 4 — Mobil bağlantı + Son Girilenler ✅
- `SignalClient`: `?pc_id=` + otomatik `join`; ret dışı yeniden deneme yok.
- `WebRtcClient`: `auth_ok` → hello; `auth_rejected` → hata + çık; `device_info`/`display_info` dinleyiciye.
- `RecentStore` + `RecentAdapter`: 2 sütun kare kart (küçük resim + isim + ID/IP/MAC + son görülme); dokun-bağlan, uzun-bas-sil.
- `DisplayActivity`: kapanışta PixelCopy ile 320px JPEG kart görüntüsü. 7 test + `assembleDebug` YEŞİL.

### Faz 5 — Mobil tema + güncelleyici ✅
- Material3 koyu Neon-Mavi tema; `REQUEST_INSTALL_PACKAGES` + FileProvider.
- `UpdateChecker`: günde 1 denetim, güncelleme kartı, DownloadManager ile APK + kurulum ekranı. 11 test YEŞİL. Sürüm 0.3.0.

### Faz 6 — CI + sürüm ✅ (bu commit)
- `release.yml`: her sürüme `latest.json` (sürüm + 3 doğrudan bağlantı) eklenir.
- Masaüstü + sinyal sürümü 0.3.0'a eşitlendi. README güncellendi.

## Yayınlama (sahibi yapar)

```bash
git tag v0.3.0 && git push origin v0.3.0   # release iş akışı AppImage + tar.gz + APK + latest.json üretir
```

Render `PCHome` servisi `master` itişinde otomatik yeniden dağıtır (v2 sunucu `/version` ile doğrulanır).
Telefon + PC'deki uygulamalar bir sonraki denetimde güncellemeyi kendileri bulur.
