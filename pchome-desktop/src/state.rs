use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU32};
use std::sync::{Arc, Mutex};

/// Shared, cross-thread state between the daemon tasks and the egui GUI.
#[derive(Default)]
pub struct AppState {
    /// 6-digit PIN shown to the user.
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
}

impl AppState {
    pub fn push_event(&self, line: String) {
        let mut events = self.events.lock().unwrap();
        events.push_back(line);
        if events.len() > 200 {
            events.pop_front();
        }
    }
}

pub type SharedState = Arc<AppState>;

pub fn new_state() -> SharedState {
    Arc::new(AppState::default())
}
