//! PCHome sinyal sunucusu (protokol v2): kalıcı PC kimlikli WebSocket relay.
//!
//! İki istemci `?pc_id=<6 hane>&role=desktop|mobile` ile bağlanır. Sunucu
//! ikisini aynı odada eşleştirir ve her JSON mesajını (join / auth_ok /
//! auth_rejected / offer / answer / ice-candidate / hello / device_info)
//! SADECE diğer uca iletir, gönderene asla yankı yapmaz.
//!
//! Şifre doğrulaması BİLEREK sunucuda yapılmaz: mobil `join` mesajıyla şifre
//! adayını gönderir, desktop kendi `identity.toml` dosyasındaki sabit şifreyle
//! karşılaştırıp `auth_ok` / `auth_rejected` döner. Sunucu yeniden başlasa
//! bile sır kaybolmaz, medya yolu hep P2P kalır.
//!
//! Geriye uyumluluk: eski `?pin=<6 hane>` istemcileri hâlâ kabul edilir
//! (300 sn TTL'li geçici oda). Yeni istemciler `pc_id` kullanmalıdır.
//!
//! Tek port üzerinden TLS-sonlandırıcı arkasında çalışır (Render/Railway/Fly);
//! konteyner yalnız düz `ws://0.0.0.0:${PORT}` sunar. Aynı port düz HTTP
//! yoklamaları da yanıtlar: `GET /health`, `GET /metrics`, `GET /version`.

#![allow(clippy::all)]

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::http::Request as HttpRequest;
use tokio_tungstenite::tungstenite::http::Response as HttpResponse;

type Tx = tokio::sync::mpsc::UnboundedSender<Message>;

/// Eski `pin` odalarının ömrü (sn). Desktop bağlı olsa bile süpürülür.
const ROOM_TTL_SECS: u64 = 300;
/// `pc_id` odası boşken (iki uç da ayrıldıysa) ne kadar tutulacağı (sn).
const PERSISTENT_EMPTY_TTL_SECS: u64 = 60;
/// `pc_id` odasının güvenlik supabı: desktop bağlı olsa bile en fazla bu kadar
/// yaşar, sonra süpürülür ve desktop yeniden `register` olur (sn).
const PERSISTENT_MAX_AGE_SECS: u64 = 24 * 3600;

/// Dakika başına IP başına yeni bağlantı kayan penceresi.
const RATE_WINDOW_SECS: u64 = 60;

#[derive(Default)]
struct RateLimiter {
    /// ip -> kabul edilen son bağlantıların zaman damgaları.
    hits: HashMap<std::net::IpAddr, Vec<std::time::Instant>>,
}

impl RateLimiter {
    /// Pencere içindeki limit aşıldıysa false döner.
    fn allow(&mut self, ip: std::net::IpAddr, limit: u32) -> bool {
        let now = std::time::Instant::now();
        let cutoff = std::time::Duration::from_secs(RATE_WINDOW_SECS);
        let entry = self.hits.entry(ip).or_default();
        entry.retain(|t| now.duration_since(*t) < cutoff);
        if entry.len() >= limit as usize {
            return false;
        }
        entry.push(now);
        true
    }
}

#[derive(Default)]
struct Room {
    desktop: Option<Tx>,
    mobile: Option<Tx>,
    /// İlk katılımda kurulur; her katılımda tazelenir.
    created_at: Option<std::time::Instant>,
    /// Son katılım/aktivite zamanı (boş oda süpürme için).
    last_activity: Option<std::time::Instant>,
    /// true: kalıcı `pc_id` odası (süpürmeye dayanıklı). false: eski `pin` odası.
    persistent: bool,
}

impl Room {
    fn touch(&mut self, persistent: bool) {
        let now = std::time::Instant::now();
        if self.created_at.is_none() {
            self.created_at = Some(now);
        }
        self.last_activity = Some(now);
        self.persistent |= persistent;
    }

    fn is_expired(&self) -> bool {
        let now = std::time::Instant::now();
        match (self.created_at, self.last_activity) {
            (Some(created), _) if self.persistent => {
                // Güvenlik supabı: 24 saatten yaşlı oda her hâlde gider.
                if now.duration_since(created) >= std::time::Duration::from_secs(PERSISTENT_MAX_AGE_SECS) {
                    return true;
                }
                // Desktop bağlıyken oda yaşar; iki uç da yoksa 60 sn sonra gider.
                if self.desktop.is_some() || self.mobile.is_some() {
                    return false;
                }
                match self.last_activity {
                    Some(last) => now.duration_since(last) >= std::time::Duration::from_secs(PERSISTENT_EMPTY_TTL_SECS),
                    None => false,
                }
            }
            (Some(created), _) => now.duration_since(created) >= std::time::Duration::from_secs(ROOM_TTL_SECS),
            _ => false,
        }
    }
}

#[derive(Default)]
struct Registry {
    rooms: HashMap<String, Room>,
}

fn parse_query(query: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for pair in query.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            out.insert(k.to_string(), v.to_string());
        }
    }
    out
}

/// Oda anahtarını çıkarır: önce `pc_id`, yoksa eski `pin`.
/// Boşluklar atılır; rakam dışı karakterler temizlenir.
/// Geçerli anahtar + kalıcılık bayrağı döner.
fn room_key(params: &HashMap<String, String>) -> Option<(String, bool)> {
    if let Some(raw) = params.get("pc_id") {
        let digits: String = raw.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
        if !digits.is_empty() && digits.len() <= 32 {
            return Some((digits, true));
        }
        return None;
    }
    if let Some(raw) = params.get("pin") {
        let digits: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
        if !digits.is_empty() && digits.len() <= 32 {
            return Some((digits, false));
        }
    }
    None
}

fn version_json() -> String {
    format!(
        "{{\"name\":\"pchome-signal\",\"version\":\"{}\",\"protocol\":2}}",
        env!("CARGO_PKG_VERSION")
    )
}

fn metrics_body(reg: &Registry) -> String {
    let rooms = reg.rooms.len();
    let desktops = reg.rooms.values().filter(|r| r.desktop.is_some()).count();
    let mobiles = reg.rooms.values().filter(|r| r.mobile.is_some()).count();
    let persistent = reg.rooms.values().filter(|r| r.persistent).count();
    format!(
        "# TYPE pchome_rooms_active gauge\npchome_rooms_active {}\n\
         # TYPE pchome_desktops_connected gauge\npchome_desktops_connected {}\n\
         # TYPE pchome_mobiles_connected gauge\npchome_mobiles_connected {}\n\
         # TYPE pchome_persistent_rooms gauge\npchome_persistent_rooms {}\n",
        rooms, desktops, mobiles, persistent
    )
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    let health_port: u16 = std::env::var("HEALTH_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8081);
    let rate_limit: u32 = std::env::var("RATE_LIMIT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(20);
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    log::info!("PCHome sinyal sunucusu dinliyor: ws://{}/ws (protokol v2)", addr);

    let registry = Arc::new(Mutex::new(Registry::default()));
    let rate_limiter = Arc::new(Mutex::new(RateLimiter::default()));

    spawn_health_server(registry.clone(), health_port);

    // Arka plan süpürücü: süresi dolmuş odaları atar.
    {
        let registry = Arc::clone(&registry);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
            loop {
                interval.tick().await;
                let mut reg = registry.lock().await;
                let before = reg.rooms.len();
                reg.rooms.retain(|_, room| !room.is_expired());
                if before != reg.rooms.len() {
                    log::info!("oda süpürücü: {} bayat oda atıldı", before - reg.rooms.len());
                }
            }
        });
    }

    loop {
        let (stream, peer) = listener.accept().await?;
        {
            let mut limiter = rate_limiter.lock().await;
            if !limiter.allow(peer.ip(), rate_limit) {
                log::warn!("oran limiti aşıldı ({}), bağlantı düşürüldü", peer.ip());
                drop(stream);
                continue;
            }
        }
        let registry = Arc::clone(&registry);
        tokio::spawn(async move {
            // Tek portta barındırma yapan platformlar için: WebSocket yükseltmesi
            // OLMAYAN düz HTTP istekleri aynı portta yanıtlanır.
            let stream = match serve_http_probe(stream, &registry).await {
                Some(s) => s,
                None => return,
            };
            if let Err(e) = handle_conn(stream, registry).await {
                log::warn!("bağlantı kapandı: {}", e);
            }
        });
    }
}

/// İstek başına bakış: WebSocket yükseltmesi olmayan düz HTTP istekleri
/// içeride yanıtlanır (`GET /health`, `/metrics`, `/version` -> 200, diğer
/// -> 404) ve None dönülür. Diğer her şey WebSocket'e bırakılır.
async fn serve_http_probe(
    mut stream: tokio::net::TcpStream,
    registry: &Arc<Mutex<Registry>>,
) -> Option<tokio::net::TcpStream> {
    use tokio::io::AsyncWriteExt;
    let mut buf = [0u8; 1024];
    let n = match stream.peek(&mut buf).await {
        Ok(n) => n,
        Err(_) => return Some(stream),
    };
    let head = String::from_utf8_lossy(&buf[..n]);
    let lower = head.to_ascii_lowercase();
    let is_upgrade = lower.contains("upgrade: websocket");
    let looks_http = ["GET ", "HEAD ", "POST ", "PUT ", "DELETE ", "OPTIONS "]
        .iter()
        .any(|m| head.starts_with(m));
    if is_upgrade || !looks_http {
        return Some(stream);
    }
    let path = head.split_whitespace().nth(1).unwrap_or("");
    let (status, content_type, body) = match path {
        "/health" => ("200 OK", "text/plain", "OK".to_string()),
        "/metrics" => {
            let reg = registry.lock().await;
            ("200 OK", "text/plain", metrics_body(&reg))
        }
        "/version" => ("200 OK", "application/json", version_json()),
        _ => ("404 Not Found", "text/plain", "not found\n".to_string()),
    };
    let resp = format!(
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status,
        content_type,
        body.len(),
        body
    );
    let _ = stream.write_all(resp.as_bytes()).await;
    let _ = stream.shutdown().await;
    None
}

/// Ayrı porttaki minimal HTTP yardımcısı (`/health` + `/metrics` + `/version`).
fn spawn_health_server(registry: Arc<Mutex<Registry>>, port: u16) {
    tokio::spawn(async move {
        let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
        let listener = match tokio::net::TcpListener::bind(addr).await {
            Ok(l) => l,
            Err(e) => {
                log::warn!("sağlık uç noktası {} üzerinde açılamadı: {}", addr, e);
                return;
            }
        };
        log::info!("sağlık uç noktası dinliyor: http://{}/health", addr);
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                continue;
            };
            let registry = Arc::clone(&registry);
            tokio::spawn(async move {
                use tokio::io::{AsyncReadExt, AsyncWriteExt};
                let mut buf = [0u8; 2048];
                let n = stream.read(&mut buf).await.unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]);
                let path = req.split_whitespace().nth(1).unwrap_or("/");
                let (status, content_type, body) = match path {
                    "/health" => ("200 OK", "text/plain", "OK".to_string()),
                    "/metrics" => {
                        let reg = registry.lock().await;
                        ("200 OK", "text/plain", metrics_body(&reg))
                    }
                    "/version" => ("200 OK", "application/json", version_json()),
                    _ => ("404 Not Found", "text/plain", "not found\n".to_string()),
                };
                drop(registry);
                let resp = format!(
                    "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    status,
                    content_type,
                    body.len(),
                    body
                );
                let _ = stream.write_all(resp.as_bytes()).await;
            });
        }
    });
}

async fn handle_conn(
    stream: tokio::net::TcpStream,
    registry: Arc<Mutex<Registry>>,
) -> Result<()> {
    let query_cell: Arc<std::sync::Mutex<Option<String>>> =
        Arc::new(std::sync::Mutex::new(None));
    let qc = Arc::clone(&query_cell);
    let callback = move |req: &HttpRequest<()>,
                          resp: HttpResponse<()>|
                          -> std::result::Result<
        HttpResponse<()>,
        HttpResponse<Option<String>>,
    > {
        *qc.lock().unwrap() = req.uri().query().map(|s| s.to_string());
        Ok(resp)
    };
    let ws = tokio_tungstenite::accept_hdr_async(stream, callback).await?;
    let query = query_cell.lock().unwrap().take().unwrap_or_default();
    let params = parse_query(&query);
    let (key, persistent) = match room_key(&params) {
        Some(k) => k,
        None => {
            log::warn!("reddedildi: pc_id/pin eksik ya da bozuk");
            return Ok(());
        }
    };
    let role = params.get("role").cloned().unwrap_or_default();
    if role != "desktop" && role != "mobile" {
        log::warn!("reddedildi: bilinmeyen rol '{}'", role);
        return Ok(());
    }

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Message>();
    {
        let mut reg = registry.lock().await;
        let room = reg.rooms.entry(key.clone()).or_default();
        room.touch(persistent);
        match role.as_str() {
            "desktop" => room.desktop = Some(tx.clone()),
            "mobile" => room.mobile = Some(tx.clone()),
            _ => {}
        }
        log::info!(
            "uç katıldı oda={} rol={} kalıcı={}",
            key,
            role,
            room.persistent
        );
    }

    let (mut writer, mut reader) = ws.split();
    let write_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if writer.send(msg).await.is_err() {
                break;
            }
        }
    });

    while let Some(Ok(msg)) = reader.next().await {
        if msg.is_close() {
            break;
        }
        if !msg.is_text() && !msg.is_binary() {
            continue;
        }
        let text = msg.into_text().unwrap_or_default();
        let peer = {
            let reg = registry.lock().await;
            let room = reg.rooms.get(&key);
            match role.as_str() {
                "desktop" => room.and_then(|r| r.mobile.clone()),
                "mobile" => room.and_then(|r| r.desktop.clone()),
                _ => None,
            }
        };
        if let Some(peer) = peer {
            let _ = peer.send(Message::Text(text));
        }
    }

    write_task.abort();
    let mut reg = registry.lock().await;
    if let Some(room) = reg.rooms.get_mut(&key) {
        match role.as_str() {
            "desktop" => room.desktop = None,
            "mobile" => room.mobile = None,
            _ => {}
        }
        room.last_activity = Some(std::time::Instant::now());
        if room.desktop.is_none() && room.mobile.is_none() && !room.persistent {
            reg.rooms.remove(&key);
        }
    }
    log::info!("uç ayrıldı oda={} rol={}", key, role);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oda_anahtari_once_pc_id() {
        let mut p = HashMap::new();
        p.insert("pc_id".to_string(), "482-913".to_string());
        p.insert("pin".to_string(), "111111".to_string());
        p.insert("role".to_string(), "mobile".to_string());
        let (key, persistent) = room_key(&p).unwrap();
        assert_eq!(key, "482913");
        assert!(persistent);
    }

    #[test]
    fn oda_anahtari_eski_pin() {
        let mut p = HashMap::new();
        p.insert("pin".to_string(), "123456".to_string());
        let (key, persistent) = room_key(&p).unwrap();
        assert_eq!(key, "123456");
        assert!(!persistent);
    }

    #[test]
    fn oda_anahtari_bos_reddedilir() {
        let p = HashMap::new();
        assert!(room_key(&p).is_none());
    }

    #[test]
    fn kalici_oda_desktop_bagliyken_supurulmez() {
        let mut room = Room::default();
        room.touch(true);
        room.desktop = Some(tokio::sync::mpsc::unbounded_channel().0);
        // 300 sn geçse bile desktop bağlıysa yaşar.
        room.created_at = Some(std::time::Instant::now() - std::time::Duration::from_secs(600));
        assert!(!room.is_expired());
    }

    #[test]
    fn eski_pin_odasi_300sn_sonra_gider() {
        let mut room = Room::default();
        room.touch(false);
        room.created_at = Some(std::time::Instant::now() - std::time::Duration::from_secs(301));
        assert!(room.is_expired());
    }

    #[test]
    fn bos_kalici_oda_60sn_sonra_gider() {
        let mut room = Room::default();
        room.touch(true);
        room.last_activity = Some(std::time::Instant::now() - std::time::Duration::from_secs(61));
        assert!(room.is_expired());
    }
}
