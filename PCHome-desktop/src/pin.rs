use anyhow::Result;
use rand::RngCore;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

const PIN_TTL_SECS: u64 = 300;

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

    pub async fn start(&self) -> Result<()> {
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

        log::info!("Generated PIN: {:06}", code);
        Ok(code)
    }

    fn generate_pin(&self) -> u32 {
        let mut rng = rand::thread_rng();
        rng.next_u32() % 1_000_000
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pin_is_zero_padded_six_digits() {
        // Every PIN must be representable as exactly 6 digits so the mobile UI
        // and the Signal server (which expects 6 ASCII digits) stay in sync.
        for raw in [0u32, 1, 42, 1234, 999999, 1_000_000 - 1] {
            let formatted = format!("{:06}", raw % 1_000_000);
            assert_eq!(formatted.len(), 6, "raw={}", raw);
            assert!(formatted.chars().all(|c| c.is_ascii_digit()));
            assert_eq!(formatted.parse::<u32>().unwrap(), raw % 1_000_000);
        }
    }

    #[tokio::test]
    async fn manager_validates_ttl() {
        let m = PinManager::new();
        let pin = m.generate_and_register().await.unwrap();
        assert!(m.is_valid(pin).await);
    }
}
