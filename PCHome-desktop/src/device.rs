use serde_json::{json, Value};

/// Cihaz bilgisi yardımcıları: telefondaki "Son Girilenler" kartında
/// gösterilen isim / IP / MAC bu modülden gelir. Yeni bağımlılık yok,
/// hepsi Linux sysfs + std ile okunur.

/// Gönderimi UDP soketiyle öğrenilen LAN adresi (paket yollanmaz).
pub fn local_ip() -> String {
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
pub fn device_info_json(
    pc_id: &str,
    pc_name: &str,
    width: u32,
    height: u32,
) -> Value {
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
