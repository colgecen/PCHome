use serde_json::Value;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::state::SharedState;
use crate::uinput::{button_code, UInputDevice};

const KEY_LEFTCTRL: u32 = 29;
const KEY_LEFTSHIFT: u32 = 42;
const KEY_LEFTALT: u32 = 56;

/// Dispatches control messages received over the WebRTC DataChannel to the
/// kernel via `/dev/uinput`. A missing uinput device only disables that part of
/// input injection; video viewing keeps working.
///
/// Absolute touch, relative trackpad movement and keyboard events live on
/// separate uinput devices because mixing EV_ABS and EV_REL makes libinput
/// classify the device as a relative mouse and ignore absolute coordinates.
pub struct ControlHandler {
    abs: Option<Arc<Mutex<UInputDevice>>>,
    rel: Option<Arc<Mutex<UInputDevice>>>,
    keyboard: Option<Arc<Mutex<UInputDevice>>>,
    state: SharedState,
}

impl ControlHandler {
    pub fn new(
        abs: Option<Arc<Mutex<UInputDevice>>>,
        rel: Option<Arc<Mutex<UInputDevice>>>,
        keyboard: Option<Arc<Mutex<UInputDevice>>>,
        state: SharedState,
    ) -> Arc<Self> {
        Arc::new(Self {
            abs,
            rel,
            keyboard,
            state,
        })
    }

    async fn inject_abs<F>(&self, action: F)
    where
        F: FnOnce(&UInputDevice) -> anyhow::Result<()>,
    {
        match &self.abs {
            Some(dev) => {
                let dev = dev.lock().await;
                if let Err(e) = action(&dev) {
                    log::warn!("uinput absolute injection failed: {}", e);
                }
            }
            None => log::debug!("mutlak işaretçi yok; kontrol olayı yoksayıldı"),
        }
    }

    async fn inject_rel<F>(&self, action: F)
    where
        F: FnOnce(&mut UInputDevice) -> anyhow::Result<()>,
    {
        match &self.rel {
            Some(dev) => {
                let mut dev = dev.lock().await;
                if let Err(e) = action(&mut dev) {
                    log::warn!("uinput relative injection failed: {}", e);
                }
            }
            None => log::debug!("göreli işaretçi yok; kontrol olayı yoksayıldı"),
        }
    }

    async fn inject_key<F>(&self, action: F)
    where
        F: FnOnce(&UInputDevice) -> anyhow::Result<()>,
    {
        match &self.keyboard {
            Some(dev) => {
                let dev = dev.lock().await;
                if let Err(e) = action(&dev) {
                    log::warn!("uinput keyboard injection failed: {}", e);
                }
            }
            None => log::debug!("klavye cihazı yok; tuş yoksayıldı"),
        }
    }

    /// Değiştirici + tuşu tek kilit altında gönderir; arada başka bir tuş
    /// mesajı girse bile Ctrl+C / Shift+2 gibi kombinasyonlar bölünmez.
    async fn inject_combo(&self, modifier: Option<u32>, code: u32) {
        let Some(dev) = &self.keyboard else {
            log::debug!("klavye cihazı yok; kombo yoksayıldı");
            return;
        };
        let dev = dev.lock().await;
        if let Some(m) = modifier {
            if let Err(e) = dev.key(m, true) {
                log::warn!("uinput modifier down failed: {}", e);
            }
        }
        if let Err(e) = dev.key(code, true) {
            log::warn!("uinput combo key down failed: {}", e);
        }
        if let Err(e) = dev.key(code, false) {
            log::warn!("uinput combo key up failed: {}", e);
        }
        if let Some(m) = modifier {
            if let Err(e) = dev.key(m, false) {
                log::warn!("uinput modifier up failed: {}", e);
            }
        }
    }

    pub async fn handle(&self, msg: &Value) {
        let typ = msg.get("type").and_then(|v| v.as_str()).unwrap_or("");
        match typ {
            // Telefon kip değiştirdiğinde (hareket beklemeden) GUI'yi
            // güncelle; aksi hâlde "Kip" satırı ilk dokunuşa kadar boş kalır.
            "mode" => {
                let mode = match msg.get("mode").and_then(|v| v.as_str()) {
                    Some("direct") => "EV_ABS",
                    Some("trackpad") => "EV_REL",
                    other => {
                        log::debug!("unknown control mode: {:?}", other);
                        return;
                    }
                };
                *self.state.mode.lock().unwrap() = mode.to_string();
                self.state.mode_explicit.store(true, Ordering::SeqCst);
            }
            "move_abs" => {
                let x = msg.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let y = msg.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                if !self.state.mode_explicit.load(Ordering::SeqCst) {
                    *self.state.mode.lock().unwrap() = "EV_ABS".to_string();
                }
                self.inject_abs(|d| d.move_absolute(x, y)).await;
                self.state.push_event(format!("ABS {} {}", x, y));
            }
            "move_rel" => {
                let dx = msg.get("dx").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let dy = msg.get("dy").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                if !self.state.mode_explicit.load(Ordering::SeqCst) {
                    *self.state.mode.lock().unwrap() = "EV_REL".to_string();
                }
                self.inject_rel(|d| d.move_relative(dx, dy)).await;
                self.state.push_event(format!("REL {} {}", dx, dy));
            }
            "click" => {
                let button = msg.get("button").and_then(|v| v.as_str()).unwrap_or("left");
                let action = msg
                    .get("action")
                    .and_then(|v| v.as_str())
                    .unwrap_or("click");
                let code = button_code(button);
                match action {
                    "down" | "hold" => self.inject_abs(|d| d.button(code, true)).await,
                    "up" => self.inject_abs(|d| d.button(code, false)).await,
                    "click" => self.inject_abs(|d| d.click(code)).await,
                    "double" => self.inject_abs(|d| d.double_click(code)).await,
                    other => {
                        log::warn!("unknown click action: {}", other);
                    }
                }
                self.state
                    .push_event(format!("CLICK {} {}", button, action));
            }
            "scroll" => {
                let dx = msg.get("dx").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let dy = msg.get("dy").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                self.inject_rel(|d| d.wheel(dx, dy)).await;
                self.state.push_event(format!("WHEEL {} {}", dx, dy));
            }
            // Yeni istemciler: 120 birim = 1 klasik tık. Eski masaüstü bu
            // türü tanımaz, böylece yüksek çözünürlüklü birimleri yanlışlıkla
            // tam tekerlek tıkı sanıp aşırı hızlı kaydırmaz.
            "scroll_hi" => {
                let dx = msg.get("dx").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let dy = msg.get("dy").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                self.inject_rel(|d| d.wheel_hi_res(dx, dy)).await;
                self.state.push_event(format!("WHEEL-HI {} {}", dx, dy));
            }
            "key" => {
                let code = msg.get("code").and_then(|v| v.as_i64()).unwrap_or(0) as u32;
                let down = msg
                    .get("action")
                    .and_then(|v| v.as_str())
                    .map(|a| a != "up")
                    .unwrap_or(true);
                self.inject_key(|d| d.key(code, down)).await;
                self.state
                    .push_event(format!("KEY {} {}", code, if down { "dn" } else { "up" }));
            }
            // Tek pakette değiştirici + tuş: sırasız DataChannel'da Shift/Ctrl
            // ile tuşun birbirinden ayrı düşmesini engeller (Ctrl+C, @, : ...).
            "combo" => {
                let code = msg.get("code").and_then(|v| v.as_i64()).unwrap_or(0) as u32;
                if code == 0 {
                    return;
                }
                let modifier = match msg.get("mod").and_then(|v| v.as_str()) {
                    Some("ctrl") => Some(KEY_LEFTCTRL),
                    Some("shift") => Some(KEY_LEFTSHIFT),
                    Some("alt") => Some(KEY_LEFTALT),
                    _ => None,
                };
                self.inject_combo(modifier, code).await;
                self.state
                    .push_event(format!("COMBO {:?} {}", modifier, code));
            }
            "pong" => {
                let sent = msg.get("t").and_then(|v| v.as_u64()).unwrap_or(0);
                if sent > 0 {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis() as u64)
                        .unwrap_or(0);
                    let rtt = now.saturating_sub(sent).min(u32::MAX as u64) as u32;
                    self.state.ping_ms.store(rtt, Ordering::SeqCst);
                }
            }
            other => {
                log::debug!("ignoring control message type: {}", other);
            }
        }
    }
}
