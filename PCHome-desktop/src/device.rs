use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::{SystemTime, UNIX_EPOCH};

/// Cihaz bilgisi yardımcıları: telefondaki "Son Girilenler" kartında
/// gösterilen isim / IP / MAC bu modülden gelir. Yeni bağımlılık yok,
/// hepsi Linux sysfs + std ile okunur.

/// Gönderimi UDP soketiyle öğrenilen LAN adresi (paket yollanmaz).
pub fn local_ip() -> String {
    // Allow overriding the advertised address on multi-homed/NAT setups and
    // for reproducible screenshots.
    if let Ok(ip) = std::env::var("PCHOME_LOCAL_IP") {
        if !ip.trim().is_empty() {
            return ip.trim().to_string();
        }
    }
    if let Ok(s) = std::net::UdpSocket::bind("0.0.0.0:0") {
        if s.connect("8.8.8.8:80").is_ok() {
            if let Ok(addr) = s.local_addr() {
                return addr.ip().to_string();
            }
        }
    }
    "127.0.0.1".to_string()
}

/// İlk fiziksel arayüzün MAC adresi (`lo` atlanır).
pub fn mac_address() -> String {
    if let Ok(entries) = std::fs::read_dir("/sys/class/net") {
        let mut fallback = String::from("bilinmiyor");
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name == "lo" {
                continue;
            }
            let addr_path = entry.path().join("address");
            if let Ok(raw) = std::fs::read_to_string(&addr_path) {
                let mac = raw.trim().to_uppercase();
                if mac.len() == 17 && mac != "00:00:00:00:00:00" {
                    // Kablolu/kablosuz gerçek arayüz öncelikli.
                    if name.starts_with("en") || name.starts_with("wl") || name.starts_with("eth") {
                        return mac;
                    }
                    if fallback == "bilinmiyor" {
                        fallback = mac;
                    }
                }
            }
        }
        return fallback;
    }
    "bilinmiyor".to_string()
}

pub fn hostname() -> String {
    if let Ok(name) = std::env::var("HOSTNAME") {
        if !name.trim().is_empty() {
            return name.trim().to_string();
        }
    }
    if let Ok(text) = std::fs::read_to_string("/etc/hostname") {
        let clean = text.trim().to_string();
        if !clean.is_empty() {
            return clean;
        }
    }
    "PCHome-PC".to_string()
}

/// Telefona hem `auth_ok` içinde hem de `get_device_info` yanıtı olarak
/// gönderilen cihaz kartı.
pub fn device_info_json(pc_id: &str, pc_name: &str, width: u32, height: u32) -> Value {
    json!({
        "type": "device_info",
        "pc_id": pc_id,
        "pc_name": pc_name,
        "ip": local_ip(),
        "mac": mac_address(),
        "resolution": format!("{}x{}", width, height),
    })
}

/// `auth_ok` yanıtı: cihaz kartı + kabul bilgisi tek mesajda.
pub fn auth_ok_json(pc_id: &str, pc_name: &str, width: u32, height: u32) -> Value {
    let mut v = device_info_json(pc_id, pc_name, width, height);
    v["type"] = Value::String("auth_ok".to_string());
    v["accepted"] = Value::Bool(true);
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cihaz_bilgisi_alanlari_dolu() {
        let v = device_info_json("482913", "test-pc", 1920, 1080);
        assert_eq!(v["type"], "device_info");
        assert_eq!(v["pc_id"], "482913");
        assert_eq!(v["pc_name"], "test-pc");
        assert_eq!(v["resolution"], "1920x1080");
        assert!(!v["ip"].as_str().unwrap_or("").is_empty());
        assert!(!v["mac"].as_str().unwrap_or("").is_empty());
    }

    #[test]
    fn auth_ok_kabul_icerir() {
        let v = auth_ok_json("482913", "test-pc", 1920, 1080);
        assert_eq!(v["type"], "auth_ok");
        assert_eq!(v["accepted"], true);
        assert_eq!(v["pc_id"], "482913");
    }
}

/// Telefondan `join` ile gelen, masaüstünün hatırladığı cihaz kaydı.
/// Dosya repo dışındadır (`~/.config/PCHome/devices.json`, 0600).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SeenDevice {
    pub name: String,
    #[serde(default)]
    pub os: String,
    #[serde(default)]
    pub app_version: String,
    #[serde(default)]
    pub last_seen: u64,
    #[serde(default)]
    pub connects: u32,
}

fn devices_path() -> std::path::PathBuf {
    if let Ok(p) = std::env::var("PCHOME_DEVICES_FILE") {
        if !p.trim().is_empty() {
            return std::path::PathBuf::from(p);
        }
    }
    let dir = if let Ok(base) = std::env::var("XDG_CONFIG_HOME") {
        std::path::PathBuf::from(base).join("PCHome")
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        std::path::Path::new(&home).join(".config").join("PCHome")
    };
    dir.join("devices.json")
}

fn now_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn load_seen_devices() -> Vec<SeenDevice> {
    let Ok(text) = std::fs::read_to_string(devices_path()) else {
        return Vec::new();
    };
    serde_json::from_str::<Vec<SeenDevice>>(&text).unwrap_or_default()
}

fn save_seen_devices(devices: &[SeenDevice]) {
    let path = devices_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let Ok(text) = serde_json::to_string_pretty(devices) else {
        return;
    };
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(target_family = "unix")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    if let Ok(mut f) = options.open(path) {
        use std::io::Write;
        let _ = f.write_all(text.as_bytes());
    }
}

/// Telefonu en son görülenler listesine taşır; en fazla 10 kayıt tutar.
pub fn remember_seen_device(name: &str, os: &str, app_version: &str) -> Vec<SeenDevice> {
    let clean_name = name.trim();
    if clean_name.is_empty() {
        return load_seen_devices();
    }
    let now = now_epoch();
    let mut devices = load_seen_devices();
    if let Some(pos) = devices
        .iter()
        .position(|d| d.name.eq_ignore_ascii_case(clean_name))
    {
        let mut existing = devices.remove(pos);
        existing.os = if os.trim().is_empty() {
            existing.os
        } else {
            os.trim().to_string()
        };
        existing.app_version = if app_version.trim().is_empty() {
            existing.app_version
        } else {
            app_version.trim().to_string()
        };
        existing.last_seen = now;
        existing.connects = existing.connects.saturating_add(1);
        devices.insert(0, existing);
    } else {
        devices.insert(
            0,
            SeenDevice {
                name: clean_name.to_string(),
                os: os.trim().to_string(),
                app_version: app_version.trim().to_string(),
                last_seen: now,
                connects: 1,
            },
        );
    }
    devices.truncate(10);
    save_seen_devices(&devices);
    devices
}

/// GUI'de "son görülme" için kısa tarih/saat.
pub fn format_seen(epoch: u64) -> String {
    if epoch == 0 {
        return "—".to_string();
    }
    chrono::DateTime::from_timestamp(epoch as i64, 0)
        .map(|dt| {
            dt.with_timezone(&chrono::Local)
                .format("%d.%m %H:%M")
                .to_string()
        })
        .unwrap_or_else(|| "—".to_string())
}

#[cfg(test)]
mod seen_device_tests {
    use super::*;

    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn cihaz_hatirlanir_ve_guncellenir() {
        let _guard = LOCK.lock().unwrap();
        let path = format!("/tmp/PCHome-devices-test-{}.json", std::process::id());
        let _ = std::fs::remove_file(&path);
        std::env::set_var("PCHOME_DEVICES_FILE", &path);

        let first = remember_seen_device("Pixel 7", "Android 14", "0.4.0");
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].name, "Pixel 7");
        assert_eq!(first[0].connects, 1);

        let second = remember_seen_device("Pixel 7", "Android 15", "0.4.1");
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].os, "Android 15");
        assert_eq!(second[0].app_version, "0.4.1");
        assert_eq!(second[0].connects, 2);

        let third = remember_seen_device("Galaxy S24", "Android 14", "0.4.1");
        assert_eq!(third[0].name, "Galaxy S24");
        assert_eq!(third.len(), 2);

        std::env::remove_var("PCHOME_DEVICES_FILE");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn cihaz_adi_bossa_kayit_yok() {
        let _guard = LOCK.lock().unwrap();
        let path = format!("/tmp/PCHome-devices-empty-{}.json", std::process::id());
        let _ = std::fs::remove_file(&path);
        std::env::set_var("PCHOME_DEVICES_FILE", &path);
        let list = remember_seen_device("   ", "Android", "0.4.0");
        assert!(list.is_empty());
        std::env::remove_var("PCHOME_DEVICES_FILE");
        let _ = std::fs::remove_file(&path);
    }
}
