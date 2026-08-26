# PChome Improvement Plan (v0.0.2)

> Her görev bitince conventional commit + kutucuk `[x]` işaretlenecek.

## A0 — Tanılama
- [x] Render build loglarını al (deploy başarılı, son commit 6638918 canlıda)
- [x] Sunucu relay testi (python): desktop↔mobile mesajlaşması ÇALIŞIYOR
- [x] WebRTC STUN yapılandırması mevcut (`stun:stun.l.google.com:19302`)
- [ ] Android logcat (kullanıcıda adb yoksa atlanabilir) — SignalClient/WebRtcClient hata satırları

## A1 — Bağlantı kök düzeltmesi
- [x] **Kök neden: Java-WebSocket 1.5.4 + Cloudflare TLS uyumsuzluğu** (mobilde bağlantı kuruluyor ama hemen kapanıyor)
- [x] `pchome-mobile/.../SignalClient.java`: OkHttp WebSocket'e taşı (pingInterval=20s Cloudflare idle timeout'u önler)
- [x] `pchome-mobile/app/build.gradle`: `org.java_websocket:Java-WebSocket` bağımlılığını kaldır, `com.squareup.okhttp3:okhttp:4.12.0` eklendi

## A2 — Desktop PIN rotasyonu (olay bazlı)
- [x] `pchome-desktop/src/main.rs`: bağlantı koptuğunda yeni PIN üret + HUD'a yansıt (background watcher)
- [x] `cargo check --features gui` temiz

## A3 — Telefon UI Material 3
- [x] `Prefs.java` (SharedPreferences sarmalayıcı: serverUrls[], recentPins[], lastServerIndex)
- [x] `activity_pin.xml` yeniden (TextInputLayout + AutoCompleteTextView + ChipGroup + PIN girişi)
- [x] `PinActivity.java` yeniden (Prefs'ten oku, ilk eleman seçili, bağlanınca güncelle)
- [x] `activity_display.xml` yeniden (üstte bağlantı Chip'i, ortada SurfaceView, altta BottomAppBar + FAB disconnect)
- [x] `DisplayActivity.java` yeniden (state machine, hata mesajları chip'te, sistem klavyesi)
- [x] `NeonKeyboard.java` → `InputMethodManager.showInputMethodPicker()` ile sistem klavyesi
- [x] `themes.xml` zaten Material 3 (Theme.Material3.DayNight) — renk paleti korundu
- [x] `build.gradle` versionName 0.0.2

## A4 — scripts/build-mobile.sh
- [x] Tek satır: local.properties'ten URL oku → gradle assembleRelease
- [x] Çıktı: `pchome-mobile/app/build/outputs/apk/release/app-release.apk`

## A5 — Release
- [ ] `v0.0.2` tag → release workflow otomatik paketlesin (AppImage + APK + RPM + tar.gz)
