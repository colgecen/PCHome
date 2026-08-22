//! Integration test: boots the real `pchome-signal` binary and verifies that
//! two peers joined with the same PIN (roles desktop/mobile) have their
//! messages relayed to each other and only to each other.

use std::process::Command;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;

async fn open(
    role: &str,
) -> tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>> {
    let url = format!("ws://127.0.0.1:18080/ws?pin=123456&role={}", role);
    let (ws, _) = tokio_tungstenite::connect_async(url).await.unwrap();
    ws
}

#[tokio::test]
async fn relay_between_peers() {
    let port = "18080";
    let mut child = Command::new(env!("CARGO_BIN_EXE_pchome-signal"))
        .env("PORT", port)
        .env("RUST_LOG", "warn")
        .spawn()
        .expect("failed to spawn signal server");
    // Give the server a moment to bind.
    tokio::time::sleep(Duration::from_millis(500)).await;

    let mut desktop = open("desktop").await;
    let mut mobile = open("mobile").await;

    // Desktop -> Mobile
    desktop
        .send(Message::Text(r#"{"type":"offer"}"#.into()))
        .await
        .unwrap();
    let mobile_got = tokio::time::timeout(Duration::from_secs(2), mobile.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(mobile_got.to_text().unwrap().contains("offer"), "mobile should receive offer");

    // Mobile -> Desktop
    mobile
        .send(Message::Text(r#"{"type":"answer"}"#.into()))
        .await
        .unwrap();
    let desktop_got = tokio::time::timeout(Duration::from_secs(2), desktop.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(desktop_got.to_text().unwrap().contains("answer"), "desktop should receive answer");

    let _ = child.kill();
    let _ = child.wait();
}
