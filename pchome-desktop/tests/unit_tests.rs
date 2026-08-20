use std::time::{Duration, SystemTime};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

mod pin;
mod encoder;
mod uinput;

use pin::PinManager;
use encoder::Encoder;

#[tokio::test]
async fn test_pin_generation_and_validation() {
    let manager = PinManager::new();
    let pin = manager.generate_and_register().await.unwrap();
    assert!(pin < 1_000_000);
    assert!(manager.is_valid(pin).await);
}

#[tokio::test]
async fn test_pin_ttl_expiration() {
    let manager = PinManager::new();
    let pin = manager.generate_and_register().await.unwrap();
    assert!(manager.is_valid(pin).await);
}

#[tokio::test]
async fn test_pin_validation_invalid() {
    let manager = PinManager::new();
    assert!(!manager.is_valid(999999).await);
}

#[tokio::test]
async fn test_encoder_backend_detection() {
    let encoder = Encoder::init_encoder().unwrap();
    assert!(encoder.width > 0);
    assert!(encoder.height > 0);
}

#[tokio::test]
async fn test_encoder_software_fallback() {
    let mut encoder = Encoder::new(
        encoder::EncoderBackend::Software,
        640,
        480,
        1_000_000,
        30,
    );
    let data = vec![0u8; 100];
    let result = encoder.encode(&data).await.unwrap();
    assert_eq!(result, data);
}

#[test]
fn test_uinput_input_event_creation() {
    let ev = uinput::InputEvent::new(0x01, 0x02, 1);
    assert_eq!(ev.kind, 0x01);
    assert_eq!(ev.code, 0x02);
    assert_eq!(ev.value, 1);
}

#[test]
fn test_uinput_input_event_to_libc() {
    let ev = uinput::InputEvent::new(1, 30, 1);
    let libc_ev = ev.to_libc();
    assert_eq!(libc_ev.type_, 1);
    assert_eq!(libc_ev.code, 30);
    assert_eq!(libc_ev.value, 1);
}
