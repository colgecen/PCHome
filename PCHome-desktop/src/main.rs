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
mod i18n;
mod identity;
#[cfg(feature = "gui")]
mod local_input;
mod metrics;
mod network;
mod pin;
#[cfg(feature = "gui")]
mod preview;
mod state;
#[cfg(feature = "gui")]
mod theme;
#[cfg(all(feature = "capture", target_family = "unix"))]
mod uinput;
#[cfg(feature = "gui")]
mod updater;

#[cfg(all(feature = "capture", target_family = "unix"))]
mod control;
#[cfg(feature = "gui")]
mod gui;

use crate::state::{new_state, SharedState};

#[cfg(all(feature = "capture", target_family = "unix"))]
use crate::control::ControlHandler;
#[cfg(all(feature = "capture", target_family = "unix"))]
use crate::encoder::H264Capture;
#[cfg(feature = "webrtc")]
use crate::network::webrtc::WebRtcEngine;
#[cfg(feature = "webrtc")]
use crate::network::ConnectionManager;
#[cfg(all(feature = "capture", target_family = "unix"))]
use crate::uinput::UInputDevice;

fn main() -> Result<()> {
    env_logger::init();
    log::info!("PCHome Desktop starting");

    // GUI henüz SIGUSR1 dinleyicisini kurmadan önce ikinci bir örnek
    // "pencereyi öne getir" sinyali gönderirse varsayılan davranış süreci
    // sonlandırırdı. GUI hazır olana kadar sinyali yok say.
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGUSR1, libc::SIG_IGN);
    }

    // Tek örnek kilidi: ikinci bir AppImage/daemon aynı anda yakalama
    // yapıp makineyi dondurmasın.
    let _instance_lock = match config::acquire_instance_lock() {
        Some(lock) => lock,
        None => {
            log::warn!("PCHome zaten çalışıyor; mevcut pencere öne getiriliyor");
            #[cfg(unix)]
            if let Some(pid) = config::instance_pid() {
                unsafe {
                    libc::kill(pid as i32, libc::SIGUSR1);
                }
            }
            let _ = std::process::Command::new("notify-send")
                .args([
                    "PCHome",
                    i18n::t(
                        "PCHome is already running — open its window from the taskbar.",
                        "PCHome zaten çalışıyor — görev çubuğundaki PCHome penceresini aç.",
                    ),
                ])
                .spawn();
            return Ok(());
        }
    };

    let state = new_state();
    // Daha önce bağlanan telefonları GUI açılmadan yükle.
    *state.seen_devices.lock().unwrap() = device::load_seen_devices();
    // KDE'de portal penceresini hiç açmamak için KWin izin dosyasını GUI
    // penceresi oluşmadan hazırla.
    #[cfg(all(feature = "capture", target_family = "unix"))]
    encoder::preauthorize_kde_capture();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    // KDE'de KWin ekran yakalama iznini GUI penceresi açılmadan doğrula.
    #[cfg(all(feature = "capture", target_family = "unix"))]
    {
        let _ = runtime.block_on(encoder::preflight_kde_capture());
    }

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
    *state.signal_url.lock().unwrap() = config.signal_url.clone();
    *state.resolution.lock().unwrap() =
        format!("{}x{}", config.capture_width, config.capture_height);
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
    state
        .pin
        .store(id.pc_id.parse().unwrap_or(0), Ordering::SeqCst);
    log::info!(
        "PC kimliği: {} ({}), sinyal: {}",
        id.pc_id,
        id.pc_name,
        config.signal_url
    );

    let connection = Arc::new(ConnectionManager::new(config.signal_url.clone()));
    if let Err(e) = connection.connect(&id.pc_id).await {
        log::warn!("sinyal bağlantısı kurulamadı: {}", e);
        *state.status.lock().unwrap() = i18n::t("SIGNAL: FAILED", "SİNYAL: BAŞARISIZ").to_string();
    } else {
        *state.status.lock().unwrap() = i18n::t("SIGNAL: CONNECTED", "SİNYAL: BAĞLI").to_string();
    }

    // Yeniden bağlanma gözcüsü: bağlantı koparsa AYNI kalıcı ID ile dönülür.
    {
        let connection = Arc::clone(&connection);
        let state = Arc::clone(&state);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(3)).await;
                if state.terminate.load(Ordering::SeqCst) {
                    break;
                }
                // GUI'den sinyal adresi değiştirildiyse çalışırken uygula.
                let desired = state.signal_url.lock().unwrap().clone();
                if !desired.is_empty() && connection.signal_url().await != desired {
                    connection.set_signal_url(&desired).await;
                    connection.disconnect().await;
                    *state.status.lock().unwrap() =
                        i18n::t("SIGNAL: ADDRESS CHANGED", "SİNYAL: ADRES DEĞİŞTİ").to_string();
                    state.push_event(i18n::pick(
                        format!("signal address updated: {}", desired),
                        format!("sinyal adresi güncellendi: {}", desired),
                    ));
                }
                let current = connection.state().await;
                if current != network::ConnectionState::Connected {
                    let pc_id = state.pc_id.lock().unwrap().clone();
                    if pc_id.is_empty() {
                        continue;
                    }
                    match connection.connect(&pc_id).await {
                        Ok(()) => {
                            *state.status.lock().unwrap() =
                                i18n::t("SIGNAL: CONNECTED", "SİNYAL: BAĞLI").to_string();
                            state.push_event(
                                i18n::t("reconnected to signal", "sinyale yeniden bağlanıldı")
                                    .to_string(),
                            );
                        }
                        Err(e) => {
                            *state.status.lock().unwrap() =
                                i18n::t("SIGNAL: RETRYING", "SİNYAL: YENİDEN DENENİYOR")
                                    .to_string();
                            log::debug!("yeniden bağlanma denemesi başarısız: {}", e);
                        }
                    }
                }
            }
        });
    }

    #[cfg(all(feature = "capture", target_family = "unix"))]
    {
        // Capture first: on Wayland the portal determines the real stream
        // resolution, and everything downstream (uinput mapping, display_info,
        // engine) must use that same size.
        if std::env::var_os("WAYLAND_DISPLAY").is_some() {
            *state.capture_status.lock().unwrap() = i18n::t(
                "Waiting for portal permission (select all screens)",
                "Portal izni bekleniyor (tüm ekranları seç)",
            )
            .to_string();
            *state.status.lock().unwrap() = i18n::t(
                "SCREEN: WAITING FOR PORTAL PERMISSION",
                "EKRAN: PORTAL İZNİ BEKLENİYOR",
            )
            .to_string();
            state.push_event(i18n::t(
                "In the Wayland portal window select ALL physical screens and enable 'restore for future sessions'",
                "Wayland portal penceresinde TÜM fiziksel ekranları işaretle; 'gelecekteki oturumlarda geri yükle' seçeneğini aç",
            ).to_string());
        }
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
                *state.capture_status.lock().unwrap() = i18n::pick(
                    format!("CAPTURE ERROR: {}", e),
                    format!("YAKALAMA HATASI: {}", e),
                );
                *state.status.lock().unwrap() =
                    i18n::t("SCREEN: CAPTURE FAILED", "EKRAN: YAKALANAMIYOR").to_string();
                state.push_event(i18n::pick(
                    format!("screen capture failed: {}", e),
                    format!("ekran yakalanamadı: {}", e),
                ));
                return;
            }
        };
        let cap_w = capture.width;
        let cap_h = capture.height;
        *state.resolution.lock().unwrap() = format!("{}x{}", cap_w, cap_h);
        *state.status.lock().unwrap() = i18n::t("SIGNAL: CONNECTED", "SİNYAL: BAĞLI").to_string();
        let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some();
        *state.capture_status.lock().unwrap() = match capture.source {
            crate::encoder::CaptureInput::Portal => {
                format!("Wayland portal • {}p", capture.out_height)
            }
            crate::encoder::CaptureInput::KWin => {
                format!("KDE KWin • {}p", capture.out_height)
            }
            crate::encoder::CaptureInput::X11Grab => {
                format!("X11 / ffmpeg • {}p", capture.out_height)
            }
            crate::encoder::CaptureInput::PipeWire => {
                format!("ffmpeg pipewire • {}p", capture.out_height)
            }
        };
        if wayland
            && matches!(
                capture.source,
                crate::encoder::CaptureInput::X11Grab | crate::encoder::CaptureInput::PipeWire
            )
        {
            state.push_event(
                i18n::t(
                    "warning: Wayland screen capture could not start; the image may be black",
                    "uyarı: Wayland ekran yakalama başlatılamadı; görüntü siyah olabilir",
                )
                .to_string(),
            );
        }
        log::info!(
            "ekran yakalama hazır: {}x{} (yayın {}x{})",
            cap_w,
            cap_h,
            capture.out_width,
            capture.out_height
        );

        // Üç ayrı uinput cihazı: mutlak dokunma, göreli touchpad ve klavye.
        // EV_ABS ile EV_REL aynı cihazda olursa libinput cihazı göreli fare
        // sayıp mutlak koordinatları yok sayıyordu (doğrudan dokunma ölüydü).
        let mut abs = None;
        let mut rel = None;
        let mut keyboard = None;
        match UInputDevice::open_absolute("/dev/uinput", "pchome-pointer", cap_w, cap_h) {
            Ok(d) => abs = Some(Arc::new(Mutex::new(d))),
            Err(e) => {
                log::error!("uinput absolüt işaretçi açılamadı: {}", e);
                state.push_event(i18n::pick(
                    format!(
                        "warning: absolute pointer could not open (touch mode disabled): {}",
                        e
                    ),
                    format!(
                        "uyarı: absolüt işaretçi açılamadı (doğrudan dokunma çalışmaz): {}",
                        e
                    ),
                ));
            }
        }
        match UInputDevice::open_relative("/dev/uinput", "pchome-trackpad") {
            Ok(d) => rel = Some(Arc::new(Mutex::new(d))),
            Err(e) => {
                log::error!("uinput göreli işaretçi açılamadı: {}", e);
                state.push_event(i18n::pick(
                    format!(
                        "warning: relative pointer could not open (trackpad disabled): {}",
                        e
                    ),
                    format!(
                        "uyarı: göreli işaretçi açılamadı (touchpad çalışmaz): {}",
                        e
                    ),
                ));
            }
        }
        match UInputDevice::open_keyboard("/dev/uinput", "pchome-keyboard") {
            Ok(d) => keyboard = Some(Arc::new(Mutex::new(d))),
            Err(e) => {
                log::error!("uinput klavye açılamadı: {}", e);
                state.push_event(i18n::pick(
                    format!(
                        "warning: keyboard could not open (remote keys disabled): {}",
                        e
                    ),
                    format!("uyarı: klavye açılamadı (uzaktan tuşlar çalışmaz): {}", e),
                ));
            }
        }
        state.uinput_active.store(
            abs.is_some() || rel.is_some() || keyboard.is_some(),
            Ordering::SeqCst,
        );
        let control = ControlHandler::new(abs, rel, keyboard, Arc::clone(&state));

        // The current WebRTC session. Rebuilt for every mobile `hello`.
        let session: Arc<parking_lot::Mutex<Option<Arc<WebRtcEngine>>>> =
            Arc::new(parking_lot::Mutex::new(None));
        spawn_signal_supervisor(
            Arc::clone(&connection),
            Arc::clone(&control),
            Arc::clone(&state),
            Arc::clone(&session),
            cap_w,
            cap_h,
            config.frame_rate,
        );

        // Ağ yavaşsa `write_sample` beklerken yakalama hattının (ve encoder
        // kuyruğunun) saniyelerce birikmesini önle: watchdog kanalı yalnızca
        // EN SON kareyi tutar; eski kareler atılır. Gönderici bir kareyi
        // yazarken biriken eski kareler tekrar gönderilmez.
        #[derive(Clone)]
        struct OutFrame {
            generation: u64,
            data: Vec<u8>,
            keyframe: bool,
            at: Instant,
        }
        #[cfg(feature = "gui")]
        let preview_tx = crate::preview::start_stream_preview(Arc::clone(&state));
        let (frame_tx, frame_rx) = tokio::sync::watch::channel::<Option<OutFrame>>(None);
        {
            let mut frame_rx = frame_rx;
            let session_push = Arc::clone(&session);
            tokio::spawn(async move {
                loop {
                    if frame_rx.changed().await.is_err() {
                        break;
                    }
                    let frame = { frame_rx.borrow().clone() };
                    let Some(frame) = frame else {
                        continue;
                    };
                    // 500 ms'den eski kareyi gönderme: gecikme birikmesin.
                    if frame.at.elapsed() > Duration::from_millis(500) {
                        continue;
                    }
                    let engine = session_push.lock().as_ref().cloned();
                    if let Some(engine) = engine {
                        if engine.generation() == frame.generation {
                            match tokio::time::timeout(
                                Duration::from_secs(2),
                                engine.push_video_sample(&frame.data, frame.keyframe),
                            )
                            .await
                            {
                                Ok(Ok(())) => {}
                                Ok(Err(e)) => log::debug!("push_video_sample failed: {}", e),
                                Err(_) => {
                                    log::warn!(
                                        "video gönderimi 2 sn takıldı; oturum yeniden kuruluyor"
                                    );
                                    let old = { session_push.lock().take() };
                                    if let Some(old) = old {
                                        old.close().await;
                                    }
                                }
                            }
                        }
                    }
                }
            });
        }

        let mut last = Instant::now();
        let mut frames = 0u32;
        let mut first_frame = true;
        'capture_outer: loop {
            if state.terminate.load(Ordering::SeqCst) {
                break;
            }
            let refresh_epoch = state.restart_capture.load(Ordering::SeqCst);
            loop {
                if state.terminate.load(Ordering::SeqCst) {
                    break 'capture_outer;
                }
                if state.restart_capture.load(Ordering::SeqCst) != refresh_epoch {
                    log::info!("ekran yakalama yenileme isteği alındı");
                    break;
                }
                // Telefon bağlı değilken tam hızda yakalama/encode yapma:
                // GUI önizlemesi için saniyede ~2 kare yeter. Telefon
                // bağlanınca (session Some) tam hıza dönülür.
                if session.lock().as_ref().is_none() {
                    tokio::time::sleep(Duration::from_millis(400)).await;
                }
                // KWin/portal/GStreamer bazen kare üretmeden takılabilir.
                // 3 sn kare gelmezse yakalayıcıyı sıfırdan kur.
                let frame_result =
                    tokio::time::timeout(Duration::from_secs(3), capture.next_frame()).await;
                match frame_result {
                    Ok(Ok((frame, keyframe))) => {
                        #[cfg(feature = "gui")]
                        let _ = preview_tx.try_send(frame.clone());
                        let generation = session
                            .lock()
                            .as_ref()
                            .map(|engine| engine.generation())
                            .unwrap_or(0);
                        let _ = frame_tx.send(Some(OutFrame {
                            generation,
                            data: frame,
                            keyframe,
                            at: Instant::now(),
                        }));
                        if first_frame {
                            log::info!("WebRTC: capture ready for video samples");
                            first_frame = false;
                        }
                        frames += 1;
                        let now = Instant::now();
                        if now.duration_since(last) >= Duration::from_secs(1) {
                            state.fps.store(frames, Ordering::SeqCst);
                            frames = 0;
                            last = now;
                        }
                    }
                    Ok(Err(e)) => {
                        log::warn!(
                            "capture frame error: {}; yakalayıcı yeniden başlatılıyor",
                            e
                        );
                        break;
                    }
                    Err(_) => {
                        log::warn!("ekran yakalama 3 sn kare üretmedi; yeniden başlatılıyor");
                        break;
                    }
                }
            }

            drop(capture);
            if state.terminate.load(Ordering::SeqCst) {
                break;
            }
            *state.capture_status.lock().unwrap() =
                i18n::t("Restarting…", "Yeniden başlatılıyor…").to_string();
            tokio::time::sleep(Duration::from_millis(400)).await;
            let mut respawned = None;
            for attempt in 0..5 {
                if state.terminate.load(Ordering::SeqCst) {
                    break;
                }
                match H264Capture::spawn(
                    config.capture_width,
                    config.capture_height,
                    config.frame_rate,
                    config.bitrate,
                )
                .await
                {
                    Ok(c) => {
                        respawned = Some(c);
                        break;
                    }
                    Err(e) => {
                        log::warn!("yakalayıcı yeniden başlatılamadı ({}): {}", attempt + 1, e);
                        tokio::time::sleep(Duration::from_secs(2)).await;
                    }
                }
            }
            match respawned {
                Some(c) => {
                    let w = c.width;
                    let h = c.height;
                    *state.resolution.lock().unwrap() = format!("{}x{}", w, h);
                    *state.capture_status.lock().unwrap() = match c.source {
                        crate::encoder::CaptureInput::Portal => {
                            format!("Wayland portal • {}p", c.out_height)
                        }
                        crate::encoder::CaptureInput::KWin => {
                            format!("KDE KWin • {}p", c.out_height)
                        }
                        crate::encoder::CaptureInput::X11Grab => {
                            format!("X11 / ffmpeg • {}p", c.out_height)
                        }
                        crate::encoder::CaptureInput::PipeWire => {
                            format!("ffmpeg pipewire • {}p", c.out_height)
                        }
                    };
                    capture = c;
                }
                None => break,
            }
        }
        drop(frame_tx);
        tokio::time::sleep(Duration::from_millis(100)).await;

        let engine = { session.lock().take() };
        if let Some(engine) = engine {
            engine.close().await;
        }
    }

    log::info!("Daemon shutting down");
}

#[cfg(all(feature = "webrtc", feature = "capture", target_family = "unix"))]
fn spawn_signal_supervisor(
    connection: Arc<ConnectionManager>,
    control: Arc<ControlHandler>,
    state: SharedState,
    session: Arc<parking_lot::Mutex<Option<Arc<WebRtcEngine>>>>,
    width: u32,
    height: u32,
    fps: u32,
) {
    tokio::spawn(async move {
        let incoming = connection.incoming();
        let mut rx = incoming.lock().await;
        let mut authed = false;
        let mut fails = 0u32;
        let mut lockout_until: Option<Instant> = None;
        let mut generation = 0u64;

        loop {
            let msg = tokio::select! {
                msg = rx.recv() => msg,
                _ = tokio::time::sleep(Duration::from_secs(1)) => {
                    if state.terminate.load(Ordering::SeqCst) {
                        break;
                    }
                    continue;
                }
            };
            let Some(msg) = msg else {
                break;
            };
            let typ = msg.get("type").and_then(|v| v.as_str()).unwrap_or("");
            match typ {
                "join" => {
                    if let Some(until) = lockout_until {
                        if Instant::now() < until {
                            connection
                                .send_json(&json_msg("auth_rejected", "kilitli, 60 sn bekleyin"))
                                .await;
                            continue;
                        } else {
                            lockout_until = None;
                            fails = 0;
                        }
                    }
                    let candidate = msg.get("password").and_then(|v| v.as_str()).unwrap_or("");
                    let expected = state.password.lock().unwrap().clone();
                    if crate::identity::verify_password(&expected, candidate) {
                        authed = true;
                        fails = 0;
                        state.peer_last_auth.store(now_epoch(), Ordering::SeqCst);
                        state.peer_fails.store(0, Ordering::SeqCst);
                        let (pc_id, pc_name) = (
                            state.pc_id.lock().unwrap().clone(),
                            state.pc_name.lock().unwrap().clone(),
                        );
                        connection
                            .send_json(&crate::device::auth_ok_json(
                                &pc_id, &pc_name, width, height,
                            ))
                            .await;
                        let dev_name = msg
                            .get("device_name")
                            .and_then(|v| v.as_str())
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .unwrap_or_else(|| "Android telefon".to_string());
                        let dev_os = msg
                            .get("device_os")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let app_version = msg
                            .get("app_version")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        *state.peer_name.lock().unwrap() = dev_name.clone();
                        let seen =
                            crate::device::remember_seen_device(&dev_name, &dev_os, &app_version);
                        *state.seen_devices.lock().unwrap() = seen;
                        state.push_event(i18n::pick(
                            format!("phone verified: {}", dev_name),
                            format!("telefon doğrulandı: {}", dev_name),
                        ));
                        log::info!("mobil doğrulandı (pc_id={}, cihaz={})", pc_id, dev_name);
                    } else {
                        fails += 1;
                        state.peer_fails.store(fails, Ordering::SeqCst);
                        if fails >= 5 {
                            lockout_until = Some(Instant::now() + Duration::from_secs(60));
                            state.push_event(
                                i18n::t(
                                    "5 wrong passwords: locked for 60 s",
                                    "5 yanlış şifre: 60 sn kilit",
                                )
                                .to_string(),
                            );
                            log::warn!("art arda 5 yanlış şifre, 60 sn kilit");
                        }
                        connection
                            .send_json(&json_msg(
                                "auth_rejected",
                                i18n::t("wrong password", "şifre hatalı"),
                            ))
                            .await;
                    }
                }
                "get_device_info" => {
                    if authed {
                        let (pc_id, pc_name) = (
                            state.pc_id.lock().unwrap().clone(),
                            state.pc_name.lock().unwrap().clone(),
                        );
                        connection
                            .send_json(&crate::device::device_info_json(
                                &pc_id, &pc_name, width, height,
                            ))
                            .await;
                    }
                }
                "hello" => {
                    if !authed {
                        log::warn!("yetkisiz hello yoksayıldı (önce join gerekli)");
                        continue;
                    }
                    // Her hello taze bir PeerConnection ister. Mobil,
                    // P2P sağlıklıyken hello göndermez; gönderdiyse el
                    // sıkışması yeniden kurulmalıdır (eski DTLS/ICE state'i
                    // yeni telefon sertifikasıyla kullanılamaz).
                    let existing = session.lock().as_ref().cloned();
                    if let Some(engine) = existing {
                        engine.close().await;
                        session.lock().take();
                    }
                    generation += 1;
                    state.peer_connected.store(false, Ordering::SeqCst);
                    state.peer_generation.store(generation, Ordering::SeqCst);
                    match WebRtcEngine::build(
                        Arc::clone(&connection),
                        Arc::clone(&control),
                        Arc::clone(&state),
                        width,
                        height,
                        fps,
                        generation,
                    )
                    .await
                    {
                        Ok(engine) => {
                            *session.lock() = Some(Arc::clone(&engine));
                            if let Err(e) = engine.create_offer().await {
                                log::warn!("create_offer failed: {}", e);
                                engine.close().await;
                                session.lock().take();
                            }
                        }
                        Err(e) => {
                            log::warn!("WebRTC engine build failed: {}", e);
                            state.push_event(i18n::pick(
                                format!("WebRTC session could not be established: {}", e),
                                format!("WebRTC oturumu kurulamadı: {}", e),
                            ));
                        }
                    }
                }
                "answer" => {
                    if let Some(sdp) = msg.get("sdp").and_then(|v| v.as_str()) {
                        let engine = session.lock().as_ref().cloned();
                        if let Some(engine) = engine {
                            engine.set_answer(sdp).await;
                        }
                    }
                }
                "ice-candidate" => {
                    let candidate = msg.get("candidate").and_then(|v| v.as_str());
                    let sdp_mid = msg
                        .get("sdpMid")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let sdp_mline_index = msg
                        .get("sdpMLineIndex")
                        .and_then(|v| v.as_u64())
                        .map(|n| n as u16);
                    if let Some(candidate) = candidate {
                        let engine = session.lock().as_ref().cloned();
                        if let Some(engine) = engine {
                            engine
                                .add_ice_candidate(candidate.to_string(), sdp_mid, sdp_mline_index)
                                .await;
                        }
                    }
                }
                _ => {}
            }
        }

        let engine = { session.lock().take() };
        if let Some(engine) = engine {
            engine.close().await;
        }
    });
}

#[cfg(all(feature = "webrtc", feature = "capture", target_family = "unix"))]
fn json_msg(typ: &str, reason: &str) -> serde_json::Value {
    serde_json::json!({ "type": typ, "reason": reason })
}

#[cfg(all(feature = "webrtc", feature = "capture", target_family = "unix"))]
fn now_epoch() -> u32 {
    std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs() as u32)
        .unwrap_or(0)
}

#[cfg(not(feature = "webrtc"))]
async fn daemon(_state: SharedState) {
    log::warn!("WebRTC feature disabled; nothing to do");
}
