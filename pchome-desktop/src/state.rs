use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU32};
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
    /// Cihazın bağlandığı zaman (epoch sn, 0 = hiç).
    pub peer_since: AtomicU32,
    /// Son başarılı şifre doğrulama zamanı (epoch sn, 0 = hiç).
    pub peer_last_auth: AtomicU32,
    /// Art arda yanlış şifre sayısı (başarılı girişte sıfırlanır).
    pub peer_fails: AtomicU32,
    /// Canlı önizleme için son JPEG kare (ham bayt + genişlik + yükseklik).
    pub preview_jpeg: Mutex<Option<(Vec<u8>, u32, u32)>>,
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
