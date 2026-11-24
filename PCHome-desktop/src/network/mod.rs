pub mod socket;
#[cfg(feature = "webrtc")]
pub mod webrtc;

use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

/// 3xx yanıtındaki `Location` başlığını mevcut WebSocket adresine göre çözer.
/// Render/Cloudflare gibi sağlayıcılar `http://` isteğini `https://` adresine
/// 301 ile yönlendirir; tungstenite yönlendirmeleri kendiliğinden takip
/// etmediği için `ws://` yapılandırması sonsuz yeniden bağlanma döngüsüne
/// girer. Bu yardımcı, mutlak (`https://...`) ve kök-göreli (`/ws?...`)
/// yönlendirmeleri `wss://` adresine çevirir.
fn resolve_ws_redirect(base: &str, location: &str) -> Option<String> {
    let location = location.trim();
    if location.is_empty() {
        return None;
    }
    if let Some(rest) = location.strip_prefix("https://") {
        return Some(format!("wss://{}", rest));
    }
    if let Some(rest) = location.strip_prefix("http://") {
        return Some(format!("ws://{}", rest));
    }
    if location.starts_with("wss://") || location.starts_with("ws://") {
        return Some(location.to_string());
    }

    let scheme_end = base.find("://")?;
    let scheme = &base[..scheme_end];
    let rest = &base[scheme_end + 3..];
    let authority_end = rest.find('/').unwrap_or(rest.len());
    let origin = format!("{}://{}", scheme, &rest[..authority_end]);

    if location.starts_with('/') {
        return Some(format!("{}{}", origin, location));
    }
    if location.starts_with('?') {
        let path = &rest[authority_end..];
        let path = path.split('?').next().unwrap_or(path);
        return Some(format!("{}{}{}", origin, path, location));
    }
    // Göreli yol: sinyal uç noktaları kök altında olduğu için kökten çöz.
    Some(format!("{}/{}", origin, location.trim_start_matches("./")))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Failed,
}

/// WebSocket client to the PCHome Signal server (role=desktop). Relays SDP and
/// ICE JSON messages and exposes the bidirectional channel used by the WebRTC
/// engine for signaling. Oda anahtarı kalıcı PC kimliğidir (`pc_id`).
pub struct ConnectionManager {
    state: Arc<RwLock<ConnectionState>>,
    /// Her connect() çağrısında artar; eski soket görevlerinin yeni
    /// bağlantının durumunu ezmesini engeller.
    epoch: Arc<AtomicU64>,
    /// Soketten gelen son mesajın zamanı (kalp atışı bekçisi için).
    last_seen: Arc<Mutex<std::time::Instant>>,
    signal_url: Arc<RwLock<String>>,
    outgoing: Arc<Mutex<Option<mpsc::UnboundedSender<String>>>>,
    incoming_rx: Arc<Mutex<mpsc::UnboundedReceiver<Value>>>,
    incoming_tx: mpsc::UnboundedSender<Value>,
}

impl ConnectionManager {
    pub fn new(signal_url: impl Into<String>) -> Self {
        let (itx, irx) = mpsc::unbounded_channel::<Value>();
        Self {
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            epoch: Arc::new(AtomicU64::new(0)),
            last_seen: Arc::new(Mutex::new(std::time::Instant::now())),
            signal_url: Arc::new(RwLock::new(signal_url.into())),
            outgoing: Arc::new(Mutex::new(None)),
            incoming_rx: Arc::new(Mutex::new(irx)),
            incoming_tx: itx,
        }
    }

    /// Sinyal sunucusuna kalıcı PC kimliğiyle bağlanır. Bağlantı kopsa bile
    /// aynı ID ile dönülür; ID asla döndürülmez (PIN rotasyonu kaldırıldı).
    pub async fn connect(&self, pc_id: &str) -> Result<()> {
        {
            let mut state = self.state.write().await;
            if *state == ConnectionState::Connected {
                return Ok(());
            }
            *state = ConnectionState::Connecting;
        }

        let clean: String = pc_id
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect();
        if clean.is_empty() {
            anyhow::bail!("geçersiz PC kimliği");
        }
        // Accept both ws://host[:port] and ws://host[:port]/ws spellings
        // without ever producing a doubled /ws/ws path.
        let signal_url = self.signal_url.read().await.clone();
        let base = signal_url.trim_end_matches('/');
        let mut url = if base.ends_with("/ws") {
            format!("{}?pc_id={}&role=desktop", base, clean)
        } else {
            format!("{}/ws?pc_id={}&role=desktop", base, clean)
        };
        log::info!("Connecting to signal server at {}", url);

        // 301/302/307/308 yönlendirmelerini takip et (en fazla 3 adım).
        // Render, `ws://pchome.onrender.com` isteğini `wss://...` adresine
        // yönlendirir; tungstenite bunu otomatik yapmaz.
        let mut redirects = 0u8;
        let (ws, _resp) = loop {
            match connect_async(&url).await {
                Ok(pair) => break pair,
                Err(tokio_tungstenite::tungstenite::Error::Http(resp))
                    if resp.status().is_redirection() && redirects < 3 =>
                {
                    let status = resp.status();
                    let location = resp
                        .headers()
                        .get("location")
                        .and_then(|v| v.to_str().ok())
                        .map(str::to_string);
                    let Some(location) = location else {
                        anyhow::bail!(
                            "signal ws connect failed: HTTP error: {} (Location başlığı yok)",
                            status
                        );
                    };
                    let Some(next) = resolve_ws_redirect(&url, &location) else {
                        anyhow::bail!(
                            "signal ws connect failed: HTTP error: {} (yönlendirme çözülemedi: {})",
                            status,
                            location
                        );
                    };
                    redirects += 1;
                    log::warn!(
                        "sinyal sunucusu yönlendirdi ({}): {} -> {}",
                        status,
                        url,
                        next
                    );
                    url = next;
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("signal ws connect failed: {}", e));
                }
            }
        };
        let (mut write, mut read) = ws.split();

        let (otx, mut o_rx) = mpsc::unbounded_channel::<String>();
        *self.outgoing.lock().await = Some(otx);
        let my_epoch = self.epoch.fetch_add(1, Ordering::SeqCst) + 1;

        /// Soket görevi bittiğinde, hâlâ güncel kuşaksa durumu kopuğa çeker.
        /// (Eskiden durum takılı kalıyor, yeniden bağlanma gözcüsü hiç
        /// devreye girmiyordu.)
        async fn mark_dropped(
            state: Arc<RwLock<ConnectionState>>,
            epoch: Arc<AtomicU64>,
            my_epoch: u64,
            why: &str,
        ) {
            if epoch.load(Ordering::SeqCst) == my_epoch {
                *state.write().await = ConnectionState::Disconnected;
                log::warn!("sinyal bağlantısı koptu ({})", why);
            }
        }

        // Writer task: forward outbound signaling messages to the socket.
        {
            let state = Arc::clone(&self.state);
            let epoch = Arc::clone(&self.epoch);
            tokio::spawn(async move {
                while let Some(msg) = o_rx.recv().await {
                    if write.send(Message::Text(msg)).await.is_err() {
                        break;
                    }
                }
                mark_dropped(state, epoch, my_epoch, "yazma").await;
            });
        }

        // Reader task: parse inbound JSON and forward to the engine.
        // Her gelen mesaj (tick dahil) soketin canlı kanıtıdır.
        let itx = self.incoming_tx.clone();
        {
            let state = Arc::clone(&self.state);
            let epoch = Arc::clone(&self.epoch);
            let seen = Arc::clone(&self.last_seen);
            tokio::spawn(async move {
                while let Some(Ok(msg)) = read.next().await {
                    if epoch.load(Ordering::SeqCst) != my_epoch {
                        break;
                    }
                    *seen.lock().await = std::time::Instant::now();
                    match msg {
                        Message::Text(t) => {
                            if let Ok(v) = serde_json::from_str::<Value>(&t) {
                                let _ = itx.send(v);
                            }
                        }
                        Message::Close(_) => break,
                        _ => {}
                    }
                }
                mark_dropped(state, epoch, my_epoch, "okuma").await;
            });
        }

        // Sessizlik bekçisi: sunucu her 25 sn'de tick yollar; 75 sn
        // hiç ses çıkmazsa soket yarı-açık ölmüştür, kopuk ilan edilir ve
        // yeniden bağlanma gözcüsü devreye girer.
        {
            let state = Arc::clone(&self.state);
            let epoch = Arc::clone(&self.epoch);
            let seen = Arc::clone(&self.last_seen);
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(15)).await;
                    if epoch.load(Ordering::SeqCst) != my_epoch {
                        break; // yerimize yeni bağlantı geldi
                    }
                    let silent = seen.lock().await.elapsed();
                    if silent >= std::time::Duration::from_secs(75) {
                        *state.write().await = ConnectionState::Disconnected;
                        log::warn!("sinyal 75 sn sessiz kaldı, yeniden bağlanılıyor (ölü soket?)");
                        break;
                    }
                }
            });
        }

        *self.state.write().await = ConnectionState::Connected;
        log::info!("Connected to signal server");
        Ok(())
    }

    /// Kullanıcı GUI'den adresi değiştirdiğinde bir sonraki bağlantı yeni
    /// adresi kullanır.
    pub async fn set_signal_url(&self, url: impl Into<String>) {
        *self.signal_url.write().await = url.into();
    }

    pub async fn signal_url(&self) -> String {
        self.signal_url.read().await.clone()
    }

    /// Mevcut WebSocket'i bırakır; yeniden bağlanma gözcüsü yeni adresle
    /// bağlanır. Eski soket görevlerinin durumu ezmesini epoch engeller.
    pub async fn disconnect(&self) {
        self.epoch.fetch_add(1, Ordering::SeqCst);
        *self.outgoing.lock().await = None;
        *self.state.write().await = ConnectionState::Disconnected;
    }

    pub async fn send_json(&self, v: &Value) {
        if let Some(tx) = self.outgoing.lock().await.as_ref() {
            let _ = tx.send(v.to_string());
        }
    }

    /// Returns the receiver used by the WebRTC engine to consume signaling
    /// messages (`offer`/`answer`/`ice-candidate`/`hello`).
    pub fn incoming(&self) -> Arc<Mutex<mpsc::UnboundedReceiver<Value>>> {
        self.incoming_rx.clone()
    }

    pub async fn state(&self) -> ConnectionState {
        *self.state.read().await
    }
}

pub async fn init_network() -> Result<()> {
    log::info!("network module initialized");
    Ok(())
}

#[cfg(test)]
mod connection_manager_tests {
    use super::*;

    #[tokio::test]
    async fn sinyal_adresi_degistirilebilir() {
        let cm = ConnectionManager::new("ws://127.0.0.1:8080/ws");
        assert_eq!(cm.signal_url().await, "ws://127.0.0.1:8080/ws");
        cm.set_signal_url("ws://127.0.0.1:9090/ws").await;
        assert_eq!(cm.signal_url().await, "ws://127.0.0.1:9090/ws");
        cm.disconnect().await;
        assert_eq!(cm.state().await, ConnectionState::Disconnected);
    }

    #[test]
    fn yonlendirme_mutlak_https_adresini_wss_yapar() {
        let next = resolve_ws_redirect(
            "ws://pchome.onrender.com/ws?pc_id=1&role=desktop",
            "https://pchome.onrender.com/ws?pc_id=1&role=desktop",
        )
        .unwrap();
        assert_eq!(next, "wss://pchome.onrender.com/ws?pc_id=1&role=desktop");
    }

    #[test]
    fn yonlendirme_kok_goreli_yolu_cozer() {
        let next = resolve_ws_redirect(
            "ws://pchome.onrender.com/ws?pc_id=1&role=desktop",
            "/ws?pc_id=1&role=desktop",
        )
        .unwrap();
        assert_eq!(next, "ws://pchome.onrender.com/ws?pc_id=1&role=desktop");
    }

    #[test]
    fn yonlendirme_bos_basligi_reddeder() {
        assert!(resolve_ws_redirect("ws://x/ws", "   ").is_none());
    }
}
