pub mod socket;
pub mod webrtc;

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Failed,
}

pub struct ConnectionManager {
    state: Arc<RwLock<ConnectionState>>,
    signal_url: String,
}

impl ConnectionManager {
    pub fn new(signal_url: impl Into<String>) -> Self {
        Self {
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
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

        info!("Connecting to signal server at {}", self.signal_url);

        match socket::connect_websocket(&self.signal_url).await {
            Ok(_ws) => {
                *self.state.write().await = ConnectionState::Connected;
                info!("Connected to signal server");
                Ok(())
            }
            Err(e) => {
                error!("Connection failed: {}", e);
                *self.state.write().await = ConnectionState::Failed;
                Err(e)
            }
        }
    }

    pub async fn disconnect(&self) -> Result<()> {
        *self.state.write().await = ConnectionState::Disconnected;
        info!("Disconnected from signal server");
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
    info!("network module init stub");
    Ok(())
}
