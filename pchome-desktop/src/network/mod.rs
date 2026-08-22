pub mod socket;
#[cfg(feature = "webrtc")]
pub mod webrtc;

use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
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
/// engine for signaling.
pub struct ConnectionManager {
    state: RwLock<ConnectionState>,
    signal_url: String,
    outgoing: Arc<Mutex<Option<mpsc::UnboundedSender<String>>>>,
    incoming_rx: Arc<Mutex<mpsc::UnboundedReceiver<Value>>>,
    incoming_tx: mpsc::UnboundedSender<Value>,
}

impl ConnectionManager {
    pub fn new(signal_url: impl Into<String>) -> Self {
        let (itx, irx) = mpsc::unbounded_channel::<Value>();
        Self {
            state: RwLock::new(ConnectionState::Disconnected),
            signal_url: signal_url.into(),
            outgoing: Arc::new(Mutex::new(None)),
            incoming_rx: Arc::new(Mutex::new(irx)),
            incoming_tx: itx,
        }
    }

    pub async fn connect(&self, pin: u32) -> Result<()> {
        {
            let mut state = self.state.write().await;
            if *state == ConnectionState::Connected {
                return Ok(());
            }
            *state = ConnectionState::Connecting;
        }

        let pin_str = format!("{:06}", pin);
        let url = format!("{}/ws?pin={}&role=desktop", self.signal_url, pin_str);
        log::info!("Connecting to signal server at {}", url);

        let (ws, _resp) = connect_async(url)
            .await
            .map_err(|e| anyhow::anyhow!("signal ws connect failed: {}", e))?;
        let (mut write, mut read) = ws.split();

        let (otx, mut o_rx) = mpsc::unbounded_channel::<String>();
        *self.outgoing.lock().await = Some(otx);

        // Writer task: forward outbound signaling messages to the socket.
        tokio::spawn(async move {
            while let Some(msg) = o_rx.recv().await {
                if write.send(Message::Text(msg)).await.is_err() {
                    break;
                }
            }
        });

        // Reader task: parse inbound JSON and forward to the engine.
        let itx = self.incoming_tx.clone();
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
        });

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
