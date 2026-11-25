use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64};
use std::sync::{Arc, Mutex};

/// Shared, cross-thread state between the daemon tasks and the egui GUI.
#[derive(Default)]
pub struct AppState {
    /// Kalıcı 6 haneli PC kimliği (telefonda ID alanına yazılır).
    pub pc_id: Mutex<String>,
    /// Sabit 6 haneli bağlantı şifresi.
    pub password: Mutex<String>,
    /// Dostça PC adı.
    pub pc_name: Mutex<String>,
    /// GUI'de seçilen/kaydedilen sinyal adresi (çalışırken değiştirilebilir).
    pub signal_url: Mutex<String>,
    /// Şu an/son bağlanan telefonun adı.
    pub peer_name: Mutex<String>,
    /// Daha önce bağlanan telefonların kalıcı listesi.
    pub seen_devices: Mutex<Vec<crate::device::SeenDevice>>,
    /// Eski geçici PIN alanı (geriye uyumluluk için tutulur, kullanılmaz).
    pub pin: AtomicU32,
    /// Connection status string, e.g. "SIGNAL: OK".
    pub status: Mutex<String>,
    pub local_ip: Mutex<String>,
    pub remote_ip: Mutex<String>,
    /// Round-trip ping in milliseconds.
    pub ping_ms: AtomicU32,
    /// Current encode/stream FPS.
    pub fps: AtomicU32,
    /// Whether the uinput virtual device is live.
    pub uinput_active: AtomicBool,
    /// Active injection mode ("EV_ABS" for direct touch, "EV_REL" for trackpad).
    pub mode: Mutex<String>,
    /// Kip telefondan açıkça bildirildi mi? Eski istemcilerde kip, gelen
    /// hareket mesajlarından çıkarılır; yeni istemcilerde `mode` mesajı
    /// hareket mesajlarının kipi ezmesini engeller.
    pub mode_explicit: AtomicBool,
    /// Rolling terminal-style log of incoming control events.
    pub events: Mutex<VecDeque<String>>,
    /// Set by the GUI TERMINATE button to shut the daemon down.
    pub terminate: AtomicBool,
    /// Latest captured resolution string for the preview panel.
    pub resolution: Mutex<String>,
    /// Güncelleme durumu satırı ("güncel", "vX.Y.Z mevcut", ...).
    pub update_status: Mutex<String>,
    /// Bağlı mobil cihaz var mı (DataChannel açıkken true).
    pub peer_connected: AtomicBool,
    /// Hangi WebRTC oturumunun bağlı olduğu (eski oturumun kapanması yeni
    /// oturumun durumunu ezmesin diye).
    pub peer_generation: AtomicU64,
    /// Cihazın bağlandığı zaman (epoch sn, 0 = hiç).
    pub peer_since: AtomicU32,
    /// Son başarılı şifre doğrulama zamanı (epoch sn, 0 = hiç).
    pub peer_last_auth: AtomicU32,
    /// Art arda yanlış şifre sayısı (başarılı girişte sıfırlanır).
    pub peer_fails: AtomicU32,
    /// Ekran yakalama durumu ("Wayland portal", "KDE KWin", "X11", hata...).
    pub capture_status: Mutex<String>,
    /// GUI'deki "Yayını yenile" düğmesi her basışta artırır; yakalama döngüsü
    /// bunu görüp yakalayıcıyı yeniden başlatır.
    pub restart_capture: AtomicU64,
    /// Canlı önizleme için son JPEG kare (ham bayt + genişlik + yükseklik).
    pub preview_jpeg: Mutex<Option<(Vec<u8>, u32, u32)>>,
    /// Her yeni önizleme karesinde artar; GUI aynı boyutlu JPEG'leri de
    /// yenilesin (eskiden yalnızca boyut değişince doku güncelleniyordu).
    pub preview_seq: AtomicU64,
    /// Önizlemedeki imleç konumu (0..1 normalize ekran koordinatı).
    pub preview_cursor: Mutex<Option<(f32, f32)>>,
    /// Önizleme yakalama akışı çalışıyor mu.
    pub preview_alive: AtomicBool,
}

impl AppState {
    pub fn push_event(&self, line: String) {
        let mut events = self.events.lock().unwrap();
        events.push_back(line);
        if events.len() > 200 {
            events.pop_front();
        }
    }

    pub fn clear_events(&self) {
        self.events.lock().unwrap().clear();
    }
}

pub type SharedState = Arc<AppState>;

pub fn new_state() -> SharedState {
    Arc::new(AppState::default())
}
