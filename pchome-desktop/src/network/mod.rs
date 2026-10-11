pub mod socket;
#[cfg(feature = "webrtc")]
pub mod webrtc;

use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::connect_async;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Failed,
}

/// WebSocket client to the PChome Signal server (role=desktop). Relays SDP and
/// ICE JSON messages and exposes the bidirectional channel used by the WebRTC
/// engine for signaling. Oda anahtarı kalıcı PC kimliğidir (`pc_id`).
pub struct ConnectionManager {
    state: Arc<RwLock<ConnectionState>>,
    /// Her connect() çağrısında artar; eski soket görevlerinin yeni
    /// bağlantının durumunu ezmesini engeller.
    epoch: Arc<AtomicU64>,
    signal_url: String,
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
            signal_url: signal_url.into(),
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

        let clean: String = pc_id.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
        if clean.is_empty() {
            anyhow::bail!("geçersiz PC kimliği");
        }
        // Accept both ws://host[:port] and ws://host[:port]/ws spellings
        // without ever producing a doubled /ws/ws path.
        let base = self.signal_url.trim_end_matches('/');
        let url = if base.ends_with("/ws") {
            format!("{}?pc_id={}&role=desktop", base, clean)
        } else {
            format!("{}/ws?pc_id={}&role=desktop", base, clean)
        };
        log::info!("Connecting to signal server at {}", url);

        let (ws, _resp) = connect_async(url)
            .await
            .map_err(|e| anyhow::anyhow!("signal ws connect failed: {}", e))?;
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
        let itx = self.incoming_tx.clone();
        {
            let state = Arc::clone(&self.state);
            let epoch = Arc::clone(&self.epoch);
            tokio::spawn(async move {
                while let Some(Ok(msg)) = read.next().await {
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

        *self.state.write().await = ConnectionState::Connected;
        log::info!("Connected to signal server");
        Ok(())
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
