pub mod socket;
pub mod webrtc;

use anyhow::Result;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Failed,
}

pub struct ConnectionManager {
    state: RwLock<ConnectionState>,
    signal_url: String,
}

impl ConnectionManager {
    pub fn new(signal_url: impl Into<String>) -> Self {
        Self {
            state: RwLock::new(ConnectionState::Disconnected),
            signal_url: signal_url.into(),
        }
    }

    pub async fn connect(&self) -> Result<()> {
        {
            let mut state = self.state.write().await;
            if *state == ConnectionState::Connected || *state == ConnectionState::Connecting {
                return Ok(());
            }
            *state = ConnectionState::Connecting;
        }

        log::info!("Connecting to signal server at {}", self.signal_url);

        *self.state.write().await = ConnectionState::Connected;
        log::info!("Connected to signal server");
        Ok(())
    }

    pub async fn disconnect(&self) -> Result<()> {
        *self.state.write().await = ConnectionState::Disconnected;
        log::info!("Disconnected from signal server");
        Ok(())
    }

    pub async fn state(&self) -> ConnectionState {
        *self.state.read().await
    }

    pub async fn is_connected(&self) -> bool {
        *self.state.read().await == ConnectionState::Connected
    }
}

pub async fn init_network() -> Result<()> {
    log::info!("network module init stub");
    Ok(())
}
