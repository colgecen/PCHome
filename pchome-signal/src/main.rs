//! Minimal PIN-routed WebSocket signaling relay for PChome.
//!
//! Two peers connect with `?pin=<6 digits>&role=desktop|mobile`. The server
//! pairs them into a room and relays every JSON message (offer / answer /
//! ice-candidate / hello) to the *other* peer only. It never inspects the
//! payload. The server itself needs no public IP for the media path: both
//! peers dial *out* to this relay, and WebRTC media flows P2P via STUN.
//!
//! Deploy behind any TLS-terminating proxy (Railway, Render, Fly, ...); the
//! container only serves plain `ws://0.0.0.0:${PORT}`.

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

/// Rooms older than this are swept even if a peer is still connected,
/// matching the desktop PIN lifetime.
const ROOM_TTL_SECS: u64 = 300;

/// Sliding-window length and default cap for new connections per client IP.
const RATE_WINDOW_SECS: u64 = 60;

#[derive(Default)]
struct RateLimiter {
    /// ip -> timestamps of recent accepted connections.
    hits: HashMap<std::net::IpAddr, Vec<std::time::Instant>>,
}

impl RateLimiter {
    /// Returns false when the IP exceeded `limit` connections inside the window.
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
    /// Set when the first peer joins; refreshed on every join.
    created_at: Option<std::time::Instant>,
}

impl Room {
    fn touch(&mut self) {
        if self.created_at.is_none() {
            self.created_at = Some(std::time::Instant::now());
        }
    }

    fn is_expired(&self) -> bool {
        match self.created_at {
            Some(t) => t.elapsed() >= std::time::Duration::from_secs(ROOM_TTL_SECS),
            None => false,
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
    log::info!("PChome signal server listening on ws://{}/ws", addr);

    let registry = Arc::new(Mutex::new(Registry::default()));
    let rate_limiter = Arc::new(Mutex::new(RateLimiter::default()));

    spawn_health_server(registry.clone(), health_port);

    // Background sweeper: evict rooms whose PIN lifetime (TTL) has elapsed.
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
                    log::info!("room sweeper: evicted {} stale rooms", before - reg.rooms.len());
                }
            }
        });
    }

    loop {
        let (stream, peer) = listener.accept().await?;
        {
            let mut limiter = rate_limiter.lock().await;
            if !limiter.allow(peer.ip(), rate_limit) {
                log::warn!("rate limit exceeded for {}, dropping", peer.ip());
                drop(stream);
                continue;
            }
        }
        let registry = Arc::clone(&registry);
        tokio::spawn(async move {
            if let Err(e) = handle_conn(stream, registry).await {
                log::warn!("connection ended: {}", e);
            }
        });
    }
}

/// Minimal HTTP sidecar serving `/health` and Prometheus-style `/metrics`
/// on a dedicated port so orchestrators can probe the relay without
/// speaking WebSocket.
fn spawn_health_server(registry: Arc<Mutex<Registry>>, port: u16) {
    tokio::spawn(async move {
        let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
        let listener = match tokio::net::TcpListener::bind(addr).await {
            Ok(l) => l,
            Err(e) => {
                log::warn!("health endpoint unavailable on {}: {}", addr, e);
                return;
            }
        };
        log::info!("health endpoint listening on http://{}/health", addr);
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
                let (status, body) = match path {
                    "/health" => ("200 OK", "OK".to_string()),
                    "/metrics" => {
                        let reg = registry.lock().await;
                        let rooms = reg.rooms.len();
                        let desktops = reg.rooms.values().filter(|r| r.desktop.is_some()).count();
                        let mobiles = reg.rooms.values().filter(|r| r.mobile.is_some()).count();
                        let body = format!(
                            "# TYPE pchome_rooms_active gauge\npchome_rooms_active {}\n\
                             # TYPE pchome_desktops_connected gauge\npchome_desktops_connected {}\n\
                             # TYPE pchome_mobiles_connected gauge\npchome_mobiles_connected {}\n",
                            rooms, desktops, mobiles
                        );
                        ("200 OK", body)
                    }
                    _ => ("404 Not Found", "not found\n".to_string()),
                };
                drop(registry);
                let resp = format!(
                    "HTTP/1.1 {}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    status,
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
    let pin = match params.get("pin") {
        Some(p) if !p.is_empty() => p.clone(),
        _ => {
            log::warn!("reject: missing pin");
            return Ok(());
        }
    };
    let role = params.get("role").cloned().unwrap_or_default();

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Message>();
    {
        let mut reg = registry.lock().await;
        let room = reg.rooms.entry(pin.clone()).or_default();
        room.touch();
        match role.as_str() {
            "desktop" => room.desktop = Some(tx.clone()),
            "mobile" => room.mobile = Some(tx.clone()),
            _ => {}
        }
        log::info!("peer joined pin={} role={}", pin, role);
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
            let room = reg.rooms.get(&pin);
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
    if let Some(room) = reg.rooms.get_mut(&pin) {
        match role.as_str() {
            "desktop" => room.desktop = None,
            "mobile" => room.mobile = None,
            _ => {}
        }
        if room.desktop.is_none() && room.mobile.is_none() {
            reg.rooms.remove(&pin);
        }
    }
    log::info!("peer left pin={} role={}", pin, role);
    Ok(())
}
