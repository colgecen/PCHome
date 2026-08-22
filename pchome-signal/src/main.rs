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

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::http::Request as HttpRequest;
use tokio_tungstenite::tungstenite::http::Response as HttpResponse;

type Tx = tokio::sync::mpsc::UnboundedSender<Message>;

#[derive(Default)]
struct Room {
    desktop: Option<Tx>,
    mobile: Option<Tx>,
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
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    log::info!("PChome signal server listening on ws://{}/ws", addr);

    let registry = Arc::new(Mutex::new(Registry::default()));
    loop {
        let (stream, _) = listener.accept().await?;
        let registry = Arc::clone(&registry);
        tokio::spawn(async move {
            if let Err(e) = handle_conn(stream, registry).await {
                log::warn!("connection ended: {}", e);
            }
        });
    }
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
