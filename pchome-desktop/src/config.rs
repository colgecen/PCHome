use anyhow::Result;
use serde::Deserialize;
use std::net::SocketAddr;

/// Runtime configuration for the PChome desktop daemon. Loaded from environment
/// variables (with `PCHOME_` prefix) and falling back to sane defaults.
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// WebSocket URL of the PChome Signal server.
    pub signal_url: String,
    /// Local capture width in pixels.
    pub capture_width: u32,
    /// Local capture height in pixels.
    pub capture_height: u32,
    /// Capture/encode frame rate.
    pub frame_rate: u32,
    /// Target H.264 bitrate in bits per second.
    pub bitrate: u32,
    /// Address the Prometheus metrics endpoint binds to.
    pub metrics_addr: SocketAddr,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            signal_url: "ws://localhost:8080".to_string(),
            capture_width: 1920,
            capture_height: 1080,
            frame_rate: 60,
            bitrate: 4_000_000,
            metrics_addr: "0.0.0.0:9091".parse().unwrap(),
        }
    }
}

impl Config {
    /// Load configuration from the process environment. Unknown variables are
    /// ignored; missing ones fall back to `Default`.
    pub fn from_env() -> Result<Self> {
        let mut cfg = Config::default();
        if let Ok(v) = std::env::var("PCHOME_SIGNAL_URL") {
            cfg.signal_url = v;
        }
        if let Ok(v) = std::env::var("PCHOME_CAPTURE_WIDTH") {
            cfg.capture_width = v.parse().unwrap_or(cfg.capture_width);
        }
        if let Ok(v) = std::env::var("PCHOME_CAPTURE_HEIGHT") {
            cfg.capture_height = v.parse().unwrap_or(cfg.capture_height);
        }
        if let Ok(v) = std::env::var("PCHOME_BITRATE") {
            cfg.bitrate = v.parse().unwrap_or(cfg.bitrate);
        }
        if let Ok(v) = std::env::var("PCHOME_METRICS_ADDR") {
            cfg.metrics_addr = v.parse().unwrap_or(cfg.metrics_addr);
        }
        Ok(cfg)
    }
}
