#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(clippy::all)]

use anyhow::Result;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

mod config;
mod encoder;
mod metrics;
mod network;
mod pin;
mod state;
#[cfg(all(feature = "capture", target_family = "unix"))]
mod uinput;

#[cfg(all(feature = "capture", target_family = "unix"))]
mod control;
#[cfg(feature = "gui")]
mod gui;

use crate::state::{new_state, SharedState};

#[cfg(feature = "webrtc")]
use crate::network::webrtc::WebRtcEngine;
#[cfg(feature = "webrtc")]
use crate::network::ConnectionManager;
#[cfg(all(feature = "capture", target_family = "unix"))]
use crate::control::ControlHandler;
#[cfg(all(feature = "capture", target_family = "unix"))]
use crate::encoder::H264Capture;
#[cfg(all(feature = "capture", target_family = "unix"))]
use crate::uinput::UInputDevice;

fn main() -> Result<()> {
    env_logger::init();
    log::info!("PChome Desktop starting");

    let state = new_state();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    let daemon_state = Arc::clone(&state);
    runtime.spawn(async move {
        daemon(daemon_state).await;
    });

    #[cfg(feature = "gui")]
    gui::run(state);

    #[cfg(not(feature = "gui"))]
    {
        runtime.block_on(async {
            let _ = tokio::signal::ctrl_c().await;
        });
    }

    runtime.shutdown_background();
    Ok(())
}

#[cfg(feature = "webrtc")]
async fn daemon(state: SharedState) {
    let config = config::Config::from_env().unwrap_or_default();
    *state.resolution.lock().unwrap() = format!("{}x{}", config.capture_width, config.capture_height);
    *state.local_ip.lock().unwrap() = first_non_loopback_ip();

    crate::metrics::serve(config.metrics_addr);

    let pin_manager = pin::PinManager::new();
    pin_manager.start().await.ok();
    let pin = match pin_manager.generate_and_register().await {
        Ok(p) => p,
        Err(e) => {
            log::error!("PIN generation failed: {}", e);
            return;
        }
    };
    state.pin.store(pin, Ordering::SeqCst);
    log::info!("Registered PIN: {:06}", pin);

    let connection = Arc::new(ConnectionManager::new(config.signal_url.clone()));
    if let Err(e) = connection.connect(pin).await {
        log::warn!("Signal connect failed: {}", e);
        *state.status.lock().unwrap() = "SIGNAL: FAIL".to_string();
    } else {
        *state.status.lock().unwrap() = "SIGNAL: OK".to_string();
    }

    // Event-driven PIN rotation: when the signal connection drops we generate
    // a fresh PIN so a stale number never lingers in the HUD after a peer
    // disconnects. The signal server already TTL-evicts empty rooms, but
    // without a new PIN the desktop would keep advertising the old one.
    {
        let pin_manager = Arc::new(pin_manager);
        let connection = Arc::clone(&connection);
        let state = Arc::clone(&state);
        tokio::spawn(async move {
            let mut was_connected = true;
            loop {
                tokio::time::sleep(Duration::from_secs(2)).await;
                let current = connection.state().await;
                if was_connected && current != network::ConnectionState::Connected {
                    if let Ok(new_pin) = pin_manager.generate_and_register().await {
                        state.pin.store(new_pin, Ordering::SeqCst);
                        state.push_event(format!("PIN rotated: {:06}", new_pin));
                        log::info!("PIN rotated on disconnect: {:06}", new_pin);
                    }
                    if let Err(e) = connection.connect(state.pin.load(Ordering::SeqCst)).await {
                        log::warn!("reconnect after PIN rotation failed: {}", e);
                    }
                }
                was_connected = current == network::ConnectionState::Connected;
            }
        });
    }

    #[cfg(all(feature = "capture", target_family = "unix"))]
    let control = {
        let uinput = match UInputDevice::open(
            "/dev/uinput",
            "pchome-virtual-device",
            config.capture_width,
            config.capture_height,
        ) {
            Ok(d) => {
                state.uinput_active.store(true, Ordering::SeqCst);
                Arc::new(Mutex::new(d))
            }
            Err(e) => {
                log::error!("uinput open failed: {}", e);
                return;
            }
        };
        ControlHandler::new(uinput, Arc::clone(&state))
    };

    #[cfg(all(feature = "capture", target_family = "unix"))]
    let engine = match WebRtcEngine::build(
        Arc::clone(&connection),
        control,
        Arc::clone(&state),
        config.capture_width,
        config.capture_height,
        config.frame_rate,
    )
    .await
    {
        Ok(e) => e,
        Err(e) => {
            log::error!("WebRTC engine build failed: {}", e);
            return;
        }
    };

    #[cfg(all(feature = "capture", target_family = "unix"))]
    {
        let engine2 = Arc::clone(&engine);
        tokio::spawn(async move {
            if let Err(e) = engine2.run().await {
                log::warn!("WebRTC engine loop ended: {}", e);
            }
        });

        let mut capture = match H264Capture::spawn(
            config.capture_width,
            config.capture_height,
            config.frame_rate,
            config.bitrate,
        )
        .await
        {
            Ok(c) => c,
            Err(e) => {
                log::error!("Capture spawn failed: {}", e);
                return;
            }
        };

        let mut last = Instant::now();
        let mut frames = 0u32;
        loop {
            if state.terminate.load(Ordering::SeqCst) {
                break;
            }
            match capture.next_frame().await {
                Ok((frame, keyframe)) => {
                    if let Err(e) = engine.push_video_sample(&frame, keyframe).await {
                        log::warn!("push_video_sample failed: {}", e);
                    }
                    frames += 1;
                    let now = Instant::now();
                    if now.duration_since(last) >= Duration::from_secs(1) {
                        state.fps.store(frames, Ordering::SeqCst);
                        frames = 0;
                        last = now;
                    }
                }
                Err(e) => {
                    log::warn!("capture frame error: {}", e);
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }
        }
    }

    log::info!("Daemon shutting down");
}

#[cfg(not(feature = "webrtc"))]
async fn daemon(_state: SharedState) {
    log::warn!("WebRTC feature disabled; nothing to do");
}

/// Best-effort local LAN address: opens a UDP socket towards a public IP
/// (no packets are sent) and reads the bound local address.
fn first_non_loopback_ip() -> String {
    if let Ok(s) = std::net::UdpSocket::bind("0.0.0.0:0") {
        if s.connect("8.8.8.8:80").is_ok() {
            if let Ok(addr) = s.local_addr() {
                return addr.ip().to_string();
            }
        }
    }
    "127.0.0.1".to_string()
}
