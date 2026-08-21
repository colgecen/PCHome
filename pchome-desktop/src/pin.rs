use anyhow::Result;
use futures_util::stream::StreamExt;
use rand::RngCore;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::connect_async;
use uuid::Uuid;

const PIN_TTL_SECS: u64 = 300;
const SIGNAL_WS_URL: &str = "ws://localhost:8080/ws";

#[derive(Debug, Clone)]
pub struct PinEntry {
    pub code: u32,
    pub created_at: SystemTime,
    pub session_id: Uuid,
    pub tx: mpsc::UnboundedSender<PinEvent>,
}

#[derive(Debug, Clone)]
pub enum PinEvent {
    Registered,
    TtlExpired,
    Error(String),
}

#[derive(Clone)]
pub struct PinManager {
    entries: Arc<RwLock<HashMap<u32, PinEntry>>>,
}

impl PinManager {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn start(self) -> Result<()> {
        let manager = self.clone();
        tokio::spawn(async move {
            manager.gc_loop().await;
        });

        log::info!("PinManager started with TTL={}s", PIN_TTL_SECS);
        Ok(())
    }

    pub async fn generate_and_register(&self) -> Result<u32> {
        let code = self.generate_pin();
        let (tx, _rx) = mpsc::unbounded_channel();

        let entry = PinEntry {
            code,
            created_at: SystemTime::now(),
            session_id: Uuid::new_v4(),
            tx,
        };

        self.entries.write().await.insert(code, entry);

        if let Err(e) = self.register_to_signal(code).await {
            log::warn!("Failed to register PIN {}: {}", code, e);
            self.entries.write().await.remove(&code);
            return Err(e);
        }

        log::info!("Generated and registered PIN: {:06}", code);
        Ok(code)
    }

    fn generate_pin(&self) -> u32 {
        let mut rng = rand::thread_rng();
        rng.next_u32() % 1_000_000
    }

    async fn register_to_signal(&self, pin: u32) -> Result<()> {
        let url = format!("{}/register/{}", SIGNAL_WS_URL, pin);
        match connect_async(&url).await {
            Ok((ws, _)) => {
                log::info!("WebSocket connected for PIN {}", pin);
                let (mut _tx, _rx) = ws.split();
                let _ = _tx;
                Ok(())
            }
            Err(e) => Err(anyhow::anyhow!("WebSocket connection failed: {}", e)),
        }
    }

    async fn gc_loop(&self) {
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        loop {
            interval.tick().await;
            self.evict_expired().await;
        }
    }

    async fn evict_expired(&self) {
        let cutoff = SystemTime::now() - Duration::from_secs(PIN_TTL_SECS);
        let mut entries = self.entries.write().await;
        let before = entries.len();
        entries.retain(|_code, entry| {
            let alive = entry.created_at >= cutoff;
            if !alive {
                let _ = entry.tx.send(PinEvent::TtlExpired);
            }
            alive
        });
        let after = entries.len();
        if before != after {
            log::info!("Pin GC: {} entries evicted", before - after);
        }
    }

    pub async fn is_valid(&self, pin: u32) -> bool {
        let entries = self.entries.read().await;
        if let Some(entry) = entries.get(&pin) {
            let cutoff = SystemTime::now() - Duration::from_secs(PIN_TTL_SECS);
            entry.created_at >= cutoff
        } else {
            false
        }
    }

    pub async fn validate(&self, pin: u32) -> Result<Uuid> {
        let entries = self.entries.read().await;
        match entries.get(&pin) {
            Some(entry) => {
                let cutoff = SystemTime::now() - Duration::from_secs(PIN_TTL_SECS);
                if entry.created_at >= cutoff {
                    Ok(entry.session_id)
                } else {
                    Err(anyhow::anyhow!("PIN expired"))
                }
            }
            None => Err(anyhow::anyhow!("PIN not found")),
        }
    }
}

impl Default for PinManager {
    fn default() -> Self {
        Self::new()
    }
}
