use serde_json::Value;
use std::collections::HashSet;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
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
    /// Telefondan gelen imleç/tık paketlerinin son sıra numarası. Farklı
    /// DataChannel'lardan gelen eski paketler imleci geri sıçratmasın.
    pointer_seq: AtomicI64,
    /// Bağlantı koparsa PC'de basılı kalmasın diye izlenen tuş/butonlar.
    held_keys: StdMutex<HashSet<u32>>,
    held_mouse_buttons: StdMutex<HashSet<u16>>,
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
            pointer_seq: AtomicI64::new(0),
            held_keys: StdMutex::new(HashSet::new()),
            held_mouse_buttons: StdMutex::new(HashSet::new()),
        })
    }

    /// Yeni bir telefon oturumu başlarken sıra numarasını sıfırla; yeniden
    /// bağlanan istemci 1'den başlasa bile ilk hareketleri yoksayılmasın.
    pub fn reset_pointer_seq(&self) {
        self.pointer_seq.store(0, Ordering::Relaxed);
    }

    /// Kapanan oturumdan kalan basılı tuş/fare butonlarını bırak. Aksi hâlde
    /// PC'de tuş basılı kalıp sonsuz tekrar edebilir.
    pub async fn release_all_input(&self) {
        let keys: Vec<u32> = {
            let mut set = self.held_keys.lock().unwrap();
            set.drain().collect()
        };
        if !keys.is_empty() {
            if let Some(dev) = &self.keyboard {
                let dev = dev.lock().await;
                for code in keys {
                    if let Err(e) = dev.key(code, false) {
                        log::warn!("uinput key release failed: {}", e);
                    }
                }
            }
        }
        let buttons: Vec<u16> = {
            let mut set = self.held_mouse_buttons.lock().unwrap();
            set.drain().collect()
        };
        if !buttons.is_empty() {
            if let Some(dev) = &self.abs {
                let dev = dev.lock().await;
                for code in buttons {
                    if let Err(e) = dev.button(code, false) {
                        log::warn!("uinput button release failed: {}", e);
                    }
                }
            }
        }
    }

    /// Sıra numarası yeni paketlerde artar. Eski bir paket geldiyse yoksay.
    fn accept_pointer_seq(&self, seq: i64) -> bool {
        if seq <= 0 {
            return true;
        }
        loop {
            let last = self.pointer_seq.load(Ordering::Relaxed);
            if seq <= last {
                return false;
            }
            if self
                .pointer_seq
                .compare_exchange_weak(last, seq, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                return true;
            }
        }
    }

    fn pointer_seq_is_old(&self, seq: i64) -> bool {
        seq > 0 && seq < self.pointer_seq.load(Ordering::Relaxed)
    }

    fn seq_of(msg: &Value) -> i64 {
        msg.get("seq").and_then(|v| v.as_i64()).unwrap_or(0)
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
                if !self.accept_pointer_seq(Self::seq_of(msg)) {
                    return;
                }
                let x = msg.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let y = msg.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                if !self.state.mode_explicit.load(Ordering::SeqCst) {
                    *self.state.mode.lock().unwrap() = "EV_ABS".to_string();
                }
                self.inject_abs(|d| d.move_absolute(x, y)).await;
                // Fare hareketleri saniyede 100+ kez gelebilir; her birini
                // GUI olay günlüğüne yazmak kontrol yolunu gereksiz yorar.
            }
            "move_rel" => {
                if !self.accept_pointer_seq(Self::seq_of(msg)) {
                    return;
                }
                let dx = msg.get("dx").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let dy = msg.get("dy").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                if !self.state.mode_explicit.load(Ordering::SeqCst) {
                    *self.state.mode.lock().unwrap() = "EV_REL".to_string();
                }
                self.inject_rel(|d| d.move_relative(dx, dy)).await;
            }
            "click" => {
                let button = msg.get("button").and_then(|v| v.as_str()).unwrap_or("left");
                let action = msg
                    .get("action")
                    .and_then(|v| v.as_str())
                    .unwrap_or("click");
                let code = button_code(button);
                match action {
                    "down" | "hold" => {
                        self.held_mouse_buttons.lock().unwrap().insert(code);
                    }
                    "up" => {
                        self.held_mouse_buttons.lock().unwrap().remove(&code);
                    }
                    _ => {}
                }
                // Tık paketleri güvenilir kanaldan, imleç hareketleri hızlı
                // kanaldan gelir. Koordinat varsa tıkla aynı uinput kilidi
                // içinde önce imleci taşı: iki kanal sırası karışsa bile tık
                // yanlış noktaya gitmez. Sıra eskiyse imleci geri sıçratma;
                // sadece tıkı uygula.
                let old_position = !self.accept_pointer_seq(Self::seq_of(msg));
                let click_x = msg.get("x").and_then(|v| v.as_i64()).map(|v| v as i32);
                let click_y = msg.get("y").and_then(|v| v.as_i64()).map(|v| v as i32);
                if let (Some(x), Some(y)) = (click_x, click_y) {
                    if !old_position {
                        if !self.state.mode_explicit.load(Ordering::SeqCst) {
                            *self.state.mode.lock().unwrap() = "EV_ABS".to_string();
                        }
                        self.inject_abs(|d| {
                            d.move_absolute(x, y)?;
                            match action {
                                "down" | "hold" => d.button(code, true),
                                "up" => d.button(code, false),
                                "click" => d.click(code),
                                "double" => d.double_click(code),
                                other => {
                                    log::warn!("unknown click action: {}", other);
                                    Ok(())
                                }
                            }
                        })
                        .await;
                    } else {
                        self.inject_abs(|d| match action {
                            "down" | "hold" => d.button(code, true),
                            "up" => d.button(code, false),
                            "click" => d.click(code),
                            "double" => d.double_click(code),
                            other => {
                                log::warn!("unknown click action: {}", other);
                                Ok(())
                            }
                        })
                        .await;
                    }
                } else {
                    match action {
                        "down" | "hold" => self.inject_abs(|d| d.button(code, true)).await,
                        "up" => self.inject_abs(|d| d.button(code, false)).await,
                        "click" => self.inject_abs(|d| d.click(code)).await,
                        "double" => self.inject_abs(|d| d.double_click(code)).await,
                        other => {
                            log::warn!("unknown click action: {}", other);
                        }
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
                if down {
                    self.held_keys.lock().unwrap().insert(code);
                } else {
                    self.held_keys.lock().unwrap().remove(&code);
                }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pointer_sequence_rejects_old_packets_and_resets() {
        let handler = ControlHandler::new(None, None, None, crate::state::new_state());
        assert!(handler.accept_pointer_seq(1));
        assert!(handler.accept_pointer_seq(3));
        assert!(!handler.accept_pointer_seq(2));
        assert!(!handler.accept_pointer_seq(3));
        handler.reset_pointer_seq();
        assert!(handler.accept_pointer_seq(1));
    }
}
