#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(clippy::all)]

use anyhow::Result;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

mod config;
mod device;
mod encoder;
mod identity;
mod metrics;
mod network;
mod pin;
mod state;
#[cfg(feature = "gui")]
mod theme;
#[cfg(feature = "gui")]
mod updater;
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
    *state.local_ip.lock().unwrap() = device::local_ip();

    crate::metrics::serve(config.metrics_addr);

    // Kalıcı kimlik: ilk açılışta üretilir, dosyada saklanır, asla dönmez.
    let id = match identity::Identity::load_or_create() {
        Ok(id) => id,
        Err(e) => {
            log::error!("kalıcı kimlik açılamadı: {}", e);
            return;
        }
    };
    *state.pc_id.lock().unwrap() = id.pc_id.clone();
    *state.password.lock().unwrap() = id.password.clone();
    *state.pc_name.lock().unwrap() = id.pc_name.clone();
    state.pin.store(
        id.pc_id.parse().unwrap_or(0),
        Ordering::SeqCst,
    );
    log::info!("PC kimliği: {} ({}), sinyal: {}", id.pc_id, id.pc_name, config.signal_url);

    let connection = Arc::new(ConnectionManager::new(config.signal_url.clone()));
    if let Err(e) = connection.connect(&id.pc_id).await {
        log::warn!("sinyal bağlantısı kurulamadı: {}", e);
        *state.status.lock().unwrap() = "SİNYAL: BAŞARISIZ".to_string();
    } else {
        *state.status.lock().unwrap() = "SİNYAL: BAĞLI".to_string();
    }

    // Yeniden bağlanma gözcüsü: bağlantı koparsa AYNI kalıcı ID ile dönülür.
    // (Eski PIN rotasyonu kaldırıldı: ID/şifre sabittir, telefonda kayıtlı
    // "Son Girilenler" kartı geçersiz kalmaz.)
    {
        let connection = Arc::clone(&connection);
        let state = Arc::clone(&state);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(3)).await;
                if state.terminate.load(Ordering::SeqCst) {
                    break;
                }
                let current = connection.state().await;
                if current != network::ConnectionState::Connected {
                    let pc_id = state.pc_id.lock().unwrap().clone();
                    if pc_id.is_empty() {
                        continue;
                    }
                    match connection.connect(&pc_id).await {
                        Ok(()) => {
                            *state.status.lock().unwrap() = "SİNYAL: BAĞLI".to_string();
                            state.push_event("sinyale yeniden bağlanıldı".to_string());
                        }
                        Err(e) => {
                            *state.status.lock().unwrap() = "SİNYAL: YENİDEN DENENİYOR".to_string();
                            log::debug!("yeniden bağlanma denemesi başarısız: {}", e);
                        }
                    }
                }
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
                    if frames == 0 {
                        log::info!("WebRTC: first video sample pushed to track");
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
