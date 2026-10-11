use serde_json::Value;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::state::SharedState;
use crate::uinput::{button_code, UInputDevice};

/// Dispatches control messages received over the WebRTC DataChannel to the
/// kernel via `/dev/uinput`.
pub struct ControlHandler {
    uinput: Arc<Mutex<UInputDevice>>,
    state: SharedState,
}

impl ControlHandler {
    pub fn new(uinput: Arc<Mutex<UInputDevice>>, state: SharedState) -> Arc<Self> {
        Arc::new(Self { uinput, state })
    }

    pub async fn handle(&self, msg: &Value) {
        let typ = msg.get("type").and_then(|v| v.as_str()).unwrap_or("");
        match typ {
            "move_abs" => {
                let x = msg.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let y = msg.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                *self.state.mode.lock().unwrap() = "EV_ABS".to_string();
                if let Err(e) = self.uinput.lock().await.move_absolute(x, y) {
                    log::warn!("move_absolute failed: {}", e);
                }
                self.state.push_event(format!("ABS {} {}", x, y));
            }
            "move_rel" => {
                let dx = msg.get("dx").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let dy = msg.get("dy").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                *self.state.mode.lock().unwrap() = "EV_REL".to_string();
                if let Err(e) = self.uinput.lock().await.move_relative(dx, dy) {
                    log::warn!("move_relative failed: {}", e);
                }
                self.state.push_event(format!("REL {} {}", dx, dy));
            }
            "click" => {
                let button = msg.get("button").and_then(|v| v.as_str()).unwrap_or("left");
                let action = msg.get("action").and_then(|v| v.as_str()).unwrap_or("click");
                let code = button_code(button);
                let r = match action {
                    "down" => self.uinput.lock().await.button(code, true),
                    "up" => self.uinput.lock().await.button(code, false),
                    "click" => self.uinput.lock().await.click(code),
                    "double" => self.uinput.lock().await.double_click(code),
                    "hold" => self.uinput.lock().await.button(code, true),
                    other => {
                        log::warn!("unknown click action: {}", other);
                        Ok(())
                    }
                };
                if let Err(e) = r {
                    log::warn!("click failed: {}", e);
                }
                self.state.push_event(format!("CLICK {} {}", button, action));
            }
            "scroll" => {
                let dx = msg.get("dx").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let dy = msg.get("dy").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                if let Err(e) = self.uinput.lock().await.wheel(dx, dy) {
                    log::warn!("wheel failed: {}", e);
                }
                self.state.push_event(format!("WHEEL {} {}", dx, dy));
            }
            "key" => {
                let code = msg.get("code").and_then(|v| v.as_i64()).unwrap_or(0) as u32;
                let down = msg
                    .get("action")
                    .and_then(|v| v.as_str())
                    .map(|a| a != "up")
                    .unwrap_or(true);
                if let Err(e) = self.uinput.lock().await.key(code, down) {
                    log::warn!("key failed: {}", e);
                }
                self.state.push_event(format!(
                    "KEY {} {}",
                    code,
                    if down { "dn" } else { "up" }
                ));
            }
            "pong" => {
                let sent = msg.get("t").and_then(|v| v.as_u64()).unwrap_or(0);
                if sent > 0 {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::SystemTime::UNIX_EPOCH)
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
