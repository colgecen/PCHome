use serde_json::Value;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::state::SharedState;
use crate::uinput::{button_code, UInputDevice};

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
        F: FnOnce(&UInputDevice) -> anyhow::Result<()>,
    {
        match &self.rel {
            Some(dev) => {
                let dev = dev.lock().await;
                if let Err(e) = action(&dev) {
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
            }
            "move_abs" => {
                let x = msg.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let y = msg.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                *self.state.mode.lock().unwrap() = "EV_ABS".to_string();
                self.inject_abs(|d| d.move_absolute(x, y)).await;
                self.state.push_event(format!("ABS {} {}", x, y));
            }
            "move_rel" => {
                let dx = msg.get("dx").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let dy = msg.get("dy").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                *self.state.mode.lock().unwrap() = "EV_REL".to_string();
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
