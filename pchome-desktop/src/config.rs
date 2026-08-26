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
    /// ignored; missing ones fall back to `Default`. Before reading the
    /// environment, a local `.env` file in the current directory (if any) is
    /// applied so each user can keep a personal signal URL out of the repo.
    /// A user settings file under the XDG config dir (`~/.config/pchome/config.toml`)
    /// is merged first, so a URL chosen in the GUI persists across launches.
    pub fn from_env() -> Result<Self> {
        load_dotenv(".env");
        let mut cfg = Config::default();
        if let Some(path) = user_config_path() {
            if let Ok(text) = std::fs::read_to_string(&path) {
                if let Ok(toml_cfg) = toml::from_str::<ConfigToml>(&text) {
                    if let Some(url) = toml_cfg.signal_url {
                        cfg.signal_url = url;
                    }
                }
            }
        }
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

    /// Persist just the signal URL to the user settings file so the GUI choice
    /// survives relaunches. Other fields keep their defaults/env values.
    pub fn save_signal_url(url: &str) -> std::io::Result<()> {
        let dir = user_config_dir();
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("config.toml");
        let mut existing = String::new();
        if let Ok(text) = std::fs::read_to_string(&path) {
            existing = text;
        }
        let mut lines: Vec<String> = existing
            .lines()
            .filter(|l| !l.trim_start().starts_with("signal_url"))
            .map(|l| l.to_string())
            .collect();
        lines.push(format!("signal_url = \"{}\"", url));
        std::fs::write(&path, lines.join("\n") + "\n")
    }
}

#[derive(Debug, Deserialize)]
struct ConfigToml {
    signal_url: Option<String>,
}

/// `~/.config/pchome` directory, honoring XDG_CONFIG_HOME when set.
fn user_config_dir() -> std::path::PathBuf {
    if let Ok(base) = std::env::var("XDG_CONFIG_HOME") {
        return std::path::Path::new(&base).join("pchome");
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    std::path::Path::new(&home).join(".config").join("pchome")
}

fn user_config_path() -> Option<std::path::PathBuf> {
    let p = user_config_dir().join("config.toml");
    if p.exists() {
        Some(p)
    } else {
        None
    }
}

/// Minimal `.env` loader: KEY=VALUE lines, `#` comments, optional quotes.
/// Existing environment variables win over the file.
fn load_dotenv(path: &str) {
    let Ok(content) = std::fs::read_to_string(path) else {
        return;
    };
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let mut value = value.trim();
            if (value.starts_with('"') && value.ends_with('"') && value.len() >= 2)
                || (value.starts_with('\'') && value.ends_with('\'') && value.len() >= 2)
            {
                value = &value[1..value.len() - 1];
            }
            if !key.is_empty() && std::env::var_os(key).is_none() {
                std::env::set_var(key, value);
            }
        }
    }
}
