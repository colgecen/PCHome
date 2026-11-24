use anyhow::Result;
use serde::Deserialize;
use std::net::SocketAddr;

/// Runtime configuration for the PCHome desktop daemon. Loaded from environment
/// variables (with `PCHOME_` prefix) and falling back to sane defaults.
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// WebSocket URL of the PCHome Signal server.
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
            // Stream target. Native screen size is still reported to the
            // phone for touch mapping; 720p keeps software H.264 encoding at
            // 30 fps on modest CPUs (1080p software encoding adds latency).
            capture_width: 1280,
            capture_height: 720,
            // 30 fps keeps software encoding (and therefore latency) under
            // control; 60 fps is still available with PCHOME_FRAME_RATE=60.
            frame_rate: 30,
            bitrate: 2_000_000,
            metrics_addr: "0.0.0.0:9091".parse().unwrap(),
        }
    }
}

impl Config {
    /// Load configuration from the process environment. Unknown variables are
    /// ignored; missing ones fall back to `Default`. Before reading the
    /// environment, a local `.env` file in the current directory (if any) is
    /// applied so each user can keep a personal signal URL out of the repo.
    /// A user settings file under the XDG config dir (`~/.config/PCHome/config.toml`)
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
        if let Ok(v) = std::env::var("PCHOME_FRAME_RATE") {
            cfg.frame_rate = v.parse().unwrap_or(cfg.frame_rate);
        }
        if let Ok(v) = std::env::var("PCHOME_BITRATE") {
            cfg.bitrate = v.parse().unwrap_or(cfg.bitrate);
        }
        if let Ok(v) = std::env::var("PCHOME_METRICS_ADDR") {
            cfg.metrics_addr = v.parse().unwrap_or(cfg.metrics_addr);
        }
        Ok(cfg)
    }

    /// GUI/mobil ile aynı kurallarla sinyal adresini normalize eder:
    /// http(s) -> ws(s), şemasız -> wss, sonda /ws yoksa ekler.
    pub fn normalize_signal_url(input: &str) -> Option<String> {
        let mut v = input.trim().to_string();
        if v.is_empty() {
            return None;
        }
        if let Some(rest) = v.strip_prefix("https://") {
            v = format!("wss://{}", rest);
        } else if let Some(rest) = v.strip_prefix("http://") {
            v = format!("ws://{}", rest);
        } else if !v.starts_with("ws://") && !v.starts_with("wss://") {
            // Şemasız yerel/özel ağ adresleri ws://, dış adresler wss://.
            v = format!(
                "{}{}",
                if Self::looks_local_host(&v) {
                    "ws://"
                } else {
                    "wss://"
                },
                v
            );
        }
        v = v.trim_end_matches('/').to_string();
        if !v.ends_with("/ws") {
            v.push_str("/ws");
        }
        let host = v.split("://").nth(1).unwrap_or("");
        if host.trim_matches('/').is_empty() {
            return None;
        }
        Some(v)
    }

    fn looks_local_host(raw: &str) -> bool {
        let mut host = raw.trim();
        if let Some((_, rest)) = host.split_once("://") {
            host = rest;
        }
        host = host.split('/').next().unwrap_or(host);
        let host = if let Some(rest) = host.strip_prefix('[') {
            rest.split(']').next().unwrap_or(rest)
        } else {
            host.split(':').next().unwrap_or(host)
        }
        .to_ascii_lowercase();
        if matches!(host.as_str(), "localhost" | "0.0.0.0" | "::1") || host.ends_with(".local") {
            return true;
        }
        if host.starts_with("127.") || host.starts_with("10.") || host.starts_with("192.168.") {
            return true;
        }
        if let Some(rest) = host.strip_prefix("172.") {
            if let Some(second) = rest.split('.').next() {
                if let Ok(n) = second.parse::<u8>() {
                    return (16..=31).contains(&n);
                }
            }
        }
        false
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

/// `~/.config/PCHome` directory, honoring XDG_CONFIG_HOME when set.
fn user_config_dir() -> std::path::PathBuf {
    if let Ok(base) = std::env::var("XDG_CONFIG_HOME") {
        return std::path::Path::new(&base).join("PCHome");
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    std::path::Path::new(&home).join(".config").join("PCHome")
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

/// Aynı anda ikinci bir PCHome örneğinin açılmasını engeller. Eski/orphan
/// yakalama süreçlerinin üst üste binip CPU'yu doldurması burada önlenir.
pub fn acquire_instance_lock() -> Option<std::fs::File> {
    use std::os::unix::io::AsRawFd;
    let dir = user_config_dir();
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("pchome.lock");
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
        .ok()?;
    let rc = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if rc == 0 {
        Some(file)
    } else {
        None
    }
}

#[cfg(test)]
mod config_tests {
    use super::*;

    #[test]
    fn sinyal_adresi_normalize() {
        assert_eq!(
            Config::normalize_signal_url("pchome.example.com").as_deref(),
            Some("wss://pchome.example.com/ws")
        );
        assert_eq!(
            Config::normalize_signal_url("http://127.0.0.1:8080").as_deref(),
            Some("ws://127.0.0.1:8080/ws")
        );
        assert_eq!(
            Config::normalize_signal_url("https://x/ws/").as_deref(),
            Some("wss://x/ws")
        );
        assert_eq!(
            Config::normalize_signal_url("192.168.1.20:8080").as_deref(),
            Some("ws://192.168.1.20:8080/ws")
        );
        assert_eq!(
            Config::normalize_signal_url("127.0.0.1:8080").as_deref(),
            Some("ws://127.0.0.1:8080/ws")
        );
        assert_eq!(Config::normalize_signal_url("   "), None);
    }
}
