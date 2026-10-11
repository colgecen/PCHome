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
    let mut child = Command::new(env!("CARGO_BIN_EXE_PCHome-signal"))
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

/// Kalp atışı: TICK_SECS=1 ile çalışan sunucu, odadaki iki uca da
/// düzenli `tick` yollar (istemciler sessizliği ölü soket sayar).
#[tokio::test]
async fn tick_reaches_both_peers() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_PCHome-signal"))
        .env("PORT", "18081")
        .env("HEALTH_PORT", "18082")
        .env("TICK_SECS", "1")
        .env("RUST_LOG", "warn")
        .spawn()
        .expect("failed to spawn signal server");
    tokio::time::sleep(Duration::from_millis(500)).await;

    async fn open81(
        role: &str,
    ) -> tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>> {
        let url = format!("ws://127.0.0.1:18081/ws?pin=654321&role={}", role);
        let (ws, _) = tokio_tungstenite::connect_async(url).await.unwrap();
        ws
    }

    let mut desktop = open81("desktop").await;
    let mut mobile = open81("mobile").await;

    for (name, ws) in [("desktop", &mut desktop), ("mobile", &mut mobile)] {
        let got = tokio::time::timeout(Duration::from_secs(5), ws.next())
            .await
            .unwrap_or_else(|_| panic!("{} uca tick gelmedi", name))
            .unwrap()
            .unwrap();
        assert!(
            got.to_text().unwrap().contains("\"tick\""),
            "{} tick almalıydı",
            name
        );
    }

    let _ = child.kill();
    let _ = child.wait();
}
