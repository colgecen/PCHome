use anyhow::Result;
use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom, Write};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use which::which;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncoderBackend {
    Vaapi,
    Nvenc,
    Software,
}

/// How ffmpeg/gstreamer reads the desktop. `-f pipewire` only exists in custom
/// ffmpeg builds, so we probe for it and fall back to x11grab. On Wayland the
/// only reliable full-desktop path is the XDG Desktop Portal (ScreenCast) —
/// x11grab sees just XWayland and normally produces a black frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureInput {
    Portal,
    KWin,
    PipeWire,
    X11Grab,
}

pub fn detect_capture_input() -> Result<CaptureInput> {
    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        return Ok(CaptureInput::Portal);
    }
    detect_capture_input_without_portal()
}

/// Same as [`detect_capture_input`], but never chooses the portal. Used as a
/// fallback after a portal capture attempt failed.
fn detect_capture_input_without_portal() -> Result<CaptureInput> {
    if let Ok(output) = std::process::Command::new("ffmpeg")
        .args(["-hide_banner", "-formats"])
        .output()
    {
        if String::from_utf8_lossy(&output.stdout).contains("pipewire") {
            return Ok(CaptureInput::PipeWire);
        }
    }
    if std::env::var_os("DISPLAY").is_some_and(|d| !d.is_empty()) {
        log::info!("ffmpeg lacks pipewire demuxer; falling back to x11grab");
        return Ok(CaptureInput::X11Grab);
    }
    anyhow::bail!(
        "no usable screen capture input: run inside a Wayland session with the \
         XDG Desktop Portal, or in an X11 session with DISPLAY set"
    )
}

pub fn detect_backend() -> EncoderBackend {
    if std::path::Path::new("/dev/dri/renderD128").exists() && has_encoder("h264_vaapi") {
        return EncoderBackend::Vaapi;
    }
    if std::path::Path::new("/dev/nvidia0").exists() && has_encoder("h264_nvenc") {
        return EncoderBackend::Nvenc;
    }
    log::info!("No usable hardware encoder found, falling back to software");
    EncoderBackend::Software
}

/// Returns true if ffmpeg advertises the given video encoder in its
/// `-encoders` list. Used to skip hardware backends whose driver/codec is
/// missing so we don't spawn a process that exits immediately.
fn has_encoder(codec: &str) -> bool {
    if let Ok(output) = std::process::Command::new("ffmpeg")
        .args(["-hide_banner", "-encoders"])
        .output()
    {
        return String::from_utf8_lossy(&output.stdout).contains(codec);
    }
    false
}

/// Software H.264 encoders to try, in order of preference. Fedora ships
/// `libopenh264` (no libx264), most other distros ship `libx264`.
const SOFTWARE_CODECS: [&str; 3] = ["libx264", "libopenh264", "h264"];

fn detect_software_codec() -> &'static str {
    if let Ok(output) = std::process::Command::new("ffmpeg")
        .args(["-hide_banner", "-encoders"])
        .output()
    {
        let list = String::from_utf8_lossy(&output.stdout);
        for codec in SOFTWARE_CODECS {
            if list.contains(codec) {
                return codec;
            }
        }
    }
    "h264"
}

/// Ordered ffmpeg configurations to attempt: hardware first, then software.
fn candidate_configs(
    capture_input: CaptureInput,
    width: u32,
    height: u32,
    out_width: u32,
    out_height: u32,
    fps: u32,
    bitrate: u32,
) -> Vec<(EncoderBackend, Vec<String>)> {
    let mut out = Vec::new();
    match detect_backend() {
        EncoderBackend::Vaapi => out.push((
            EncoderBackend::Vaapi,
            build_args(
                EncoderBackend::Vaapi,
                "h264_vaapi",
                capture_input,
                width,
                height,
                out_width,
                out_height,
                fps,
                bitrate,
            ),
        )),
        EncoderBackend::Nvenc => out.push((
            EncoderBackend::Nvenc,
            build_args(
                EncoderBackend::Nvenc,
                "h264_nvenc",
                capture_input,
                width,
                height,
                out_width,
                out_height,
                fps,
                bitrate,
            ),
        )),
        EncoderBackend::Software => {}
    }
    let sw = detect_software_codec();
    out.push((
        EncoderBackend::Software,
        build_args(
            EncoderBackend::Software,
            sw,
            capture_input,
            width,
            height,
            out_width,
            out_height,
            fps,
            bitrate,
        ),
    ));
    out
}

/// Spawns an `ffmpeg` process that captures the desktop and encodes it to an
/// Annex-B H.264 elementary stream on stdout, trying hardware encoders
/// (VA-API / NVENC) first and falling back to a working software codec
/// (libx264 / libopenh264). A candidate is only accepted once ffmpeg actually
/// produces its first H.264 bytes, so broken hardware init (e.g. VA-API driver
/// failures) transparently falls through to the next option.
///
/// On Wayland the capture is done through the XDG Desktop Portal + GStreamer
/// (PipeWire), because x11grab only sees XWayland and produces a black frame.
pub struct H264Capture {
    child: tokio::process::Child,
    reader: BufReader<tokio::process::ChildStdout>,
    /// Native desktop size (used for display_info and touch/uinput mapping).
    pub width: u32,
    pub height: u32,
    /// Encoded stream size (scaled down for low-latency software encoding).
    pub out_width: u32,
    pub out_height: u32,
    /// Number of physical screens in the portal compositor (1 on X11).
    pub monitors: usize,
    /// Which capture path actually produced frames.
    pub source: CaptureInput,
    pub fps: u32,
    buf: Vec<u8>,
    /// Current access unit being assembled from NALs (H.264).
    au: Vec<u8>,
    au_has_vcl: bool,
    /// Keeps the portal session + PipeWire proxy alive while GStreamer runs.
    _portal: Option<PortalKeepAlive>,
    /// KDE KWin ScreenShot2 yakalama döngüsü.
    _kwin_stop: Option<Arc<AtomicBool>>,
    _kwin_task: Option<tokio::task::JoinHandle<()>>,
}

struct PortalKeepAlive {
    _proxy: ashpd::desktop::screencast::Screencast,
    _session: ashpd::desktop::Session<ashpd::desktop::screencast::Screencast>,
}

impl H264Capture {
    pub async fn spawn(out_width: u32, out_height: u32, fps: u32, bitrate: u32) -> Result<Self> {
        // Wayland first: a real portal screencast is the only way to see the
        // whole compositor, not just XWayland.
        if std::env::var_os("WAYLAND_DISPLAY").is_some() {
            let force = std::env::var("PCHOME_CAPTURE")
                .unwrap_or_default()
                .to_ascii_lowercase();
            let kde = is_kde_wayland();
            // KWin ScreenShot2 captures the compositor cursor (include-cursor)
            // and therefore mirrors the real mouse pointer to the phone. The
            // portal path is smoother (30 fps) but KDE does not embed the
            // cursor into the PipeWire stream. Default to KWin on KDE; use
            // PCHOME_CAPTURE=portal for the faster, cursor-less path.
            let kwin_first = kde && force != "portal";

            if kwin_first {
                match Self::try_start_kwin(out_width, out_height, fps, bitrate).await {
                    Ok(capture) => return Ok(capture),
                    Err(e) => log::warn!(
                        "KDE KWin ekran yakalama başlatılamadı ({}); portal denenecek",
                        e
                    ),
                }
            }

            if force != "kwin" {
                let portal = tokio::time::timeout(
                    std::time::Duration::from_secs(25),
                    Self::start_portal(out_width, out_height, fps, bitrate),
                )
                .await;
                match portal {
                    Ok(Ok(capture)) => return Ok(capture),
                    Ok(Err(e)) => log::warn!(
                        "Wayland portal ekran yakalama başlatılamadı ({}); x11grab'a düşülüyor \
                         (XWayland pencereleri görünür, masaüstü büyük ihtimalle siyah kalır)",
                        e
                    ),
                    Err(_) => {
                        log::warn!("Wayland portal izni 25 sn içinde gelmedi; x11grab'a düşülüyor")
                    }
                }
            }

            // Portal kullanılamadıysa KDE'de son çare olarak hızlı ekran
            // görüntüsü döngüsüne dön (yavaş ama portalsız çalışır).
            if kde && !kwin_first {
                match Self::try_start_kwin(out_width, out_height, fps, bitrate).await {
                    Ok(capture) => return Ok(capture),
                    Err(e) => log::warn!(
                        "KDE KWin ekran yakalama başlatılamadı ({}); x11grab denenecek",
                        e
                    ),
                }
            }
        }

        let capture_input = detect_capture_input_without_portal()?;
        let (width, height) = match capture_input {
            CaptureInput::X11Grab => detect_x11_size().unwrap_or((out_width, out_height)),
            _ => (out_width, out_height),
        };
        let (out_w, out_h) = fit_size(width, height, out_width, out_height);
        log::info!(
            "Capture input: {:?} ({}x{} -> {}x{} @ {}fps)",
            capture_input,
            width,
            height,
            out_w,
            out_h,
            fps
        );

        let ff = which("ffmpeg").unwrap_or_else(|_| std::path::PathBuf::from("ffmpeg"));
        let candidates =
            candidate_configs(capture_input, width, height, out_w, out_h, fps, bitrate);

        let mut last_err = None;
        for (backend, args) in &candidates {
            log::info!(
                "Trying encoder backend {:?}: {} ...",
                backend,
                args.join(" ")
            );
            match Self::try_start(
                &ff,
                args.clone(),
                capture_input,
                width,
                height,
                out_w,
                out_h,
                fps,
            )
            .await
            {
                Ok(capture) => {
                    log::info!("Encoder backend {:?} is producing frames", backend);
                    return Ok(capture);
                }
                Err(e) => {
                    log::info!(
                        "Encoder backend {:?} unavailable ({}), falling back",
                        backend,
                        e
                    );
                    last_err = Some(e);
                }
            }
        }
        anyhow::bail!(
            "no working ffmpeg encoder configuration: {}",
            last_err.map(|e| e.to_string()).unwrap_or_default()
        )
    }

    /// Spawns ffmpeg with `args` and waits up to 8s for the first H.264
    /// output bytes; on failure the process is killed and the error returned
    /// so the caller can try the next candidate.
    async fn try_start(
        ff: &std::path::Path,
        args: Vec<String>,
        capture_input: CaptureInput,
        width: u32,
        height: u32,
        out_width: u32,
        out_height: u32,
        fps: u32,
    ) -> Result<Self> {
        let mut child = Command::new(ff)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("ffmpeg stdout unavailable"))?;
        let mut reader = BufReader::new(stdout);
        spawn_stderr_logger(child.stderr.take(), "ffmpeg");

        let mut buf = Vec::with_capacity(256 * 1024);
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(8);
        loop {
            if find_start(&buf, 0).is_some() {
                return Ok(Self {
                    child,
                    reader,
                    width,
                    height,
                    out_width,
                    out_height,
                    monitors: 1,
                    source: capture_input,
                    fps,
                    buf,
                    au: Vec::with_capacity(256 * 1024),
                    au_has_vcl: false,
                    _portal: None,
                    _kwin_stop: None,
                    _kwin_task: None,
                });
            }
            let mut tmp = [0u8; 16384];
            let n = match tokio::time::timeout_at(deadline, reader.read(&mut tmp)).await {
                Ok(r) => r?,
                Err(_) => anyhow::bail!("timeout waiting for first encoded bytes"),
            };
            if n == 0 {
                anyhow::bail!("ffmpeg exited before producing output");
            }
            buf.extend_from_slice(&tmp[..n]);
        }
    }

    /// Wayland: ask the XDG Desktop Portal for a screen-cast stream and pipe
    /// every selected monitor through a GStreamer compositor. The compositor
    /// keeps the physical monitor layout (the portal reports each screen's
    /// position/size), so the phone sees all screens as one canvas.
    async fn start_portal(out_width: u32, out_height: u32, fps: u32, bitrate: u32) -> Result<Self> {
        use ashpd::desktop::screencast::{
            CursorMode, Screencast, SelectSourcesOptions, SourceType,
        };
        use ashpd::desktop::PersistMode;
        use std::os::fd::AsRawFd;

        #[derive(Clone, Copy)]
        struct PortalScreen {
            node: u32,
            x: i32,
            y: i32,
            w: u32,
            h: u32,
            source_type: Option<SourceType>,
        }

        let token = read_restore_token();
        let proxy = Screencast::new().await?;
        let session = proxy.create_session(Default::default()).await?;

        let mut select = SelectSourcesOptions::default()
            .set_sources(SourceType::MONITOR)
            // Birden fazla fiziksel ekran seçilebilsin; sanal ekran/bölge
            // seçilirse portal yine o kaynağı verir, ama biz monitor isteriz.
            .set_multiple(true)
            .set_cursor_mode(CursorMode::EMBEDDED)
            .set_persist_mode(PersistMode::ExplicitlyRevoked);
        if let Some(token) = token.as_deref() {
            select = select.set_restore_token(token);
        }
        proxy.select_sources(&session, select).await?;

        log::info!(
            "Wayland ekran paylaşımı izni bekleniyor (portal); TÜM fiziksel ekranları seçin…"
        );
        let request = proxy.start(&session, None, Default::default()).await?;
        let streams = request.response()?;
        if let Some(token) = streams.restore_token() {
            save_restore_token(token);
        }

        let mut screens: Vec<PortalScreen> = Vec::new();
        let mut fallback_x = 0i32;
        for stream in streams.streams() {
            let node = stream.pipe_wire_node_id();
            let (w, h) = stream
                .size()
                .map(|(w, h)| (w.max(1) as u32, h.max(1) as u32))
                .unwrap_or((1920, 1080));
            let (x, y) = stream.position().unwrap_or((fallback_x, 0));
            fallback_x = fallback_x.saturating_add(w as i32);
            if screens.iter().any(|s| s.node == node) {
                continue;
            }
            screens.push(PortalScreen {
                node,
                x,
                y,
                w,
                h,
                source_type: stream.source_type(),
            });
        }
        // Yalnızca fiziksel monitörleri tercih et; sanal ekran/pencere
        // seçildiyse ve başka kaynak yoksa mecburen onu kullan.
        let physical: Vec<PortalScreen> = screens
            .iter()
            .filter(|s| {
                s.source_type
                    .map(|t| t.contains(SourceType::MONITOR))
                    .unwrap_or(true)
            })
            .copied()
            .collect();
        if !physical.is_empty() {
            screens = physical;
        } else {
            log::warn!("portal sanal ekran/pencere kaynağı verdi; fiziksel ekran seçin");
        }
        if screens.is_empty() {
            anyhow::bail!("portal ekran akışı döndürmedi");
        }

        let min_x = screens.iter().map(|s| s.x).min().unwrap_or(0);
        let min_y = screens.iter().map(|s| s.y).min().unwrap_or(0);
        let width = screens
            .iter()
            .map(|s| (s.x - min_x).saturating_add(s.w as i32).max(0) as u32)
            .max()
            .unwrap_or(1)
            .max(1);
        let height = screens
            .iter()
            .map(|s| (s.y - min_y).saturating_add(s.h as i32).max(0) as u32)
            .max()
            .unwrap_or(1)
            .max(1);
        let (out_w, out_h) = fit_size(width, height, out_width, out_height);
        let fd = proxy
            .open_pipe_wire_remote(&session, Default::default())
            .await?;
        let raw_fd = fd.as_raw_fd();

        log::info!(
            "Portal akışı hazır: {} ekran, tuval {}x{} -> {}x{} @ {}fps",
            screens.len(),
            width,
            height,
            out_w,
            out_h,
            fps
        );
        for (i, screen) in screens.iter().enumerate() {
            log::info!(
                "  ekran {}: node={} {}x{} @ {},{}",
                i + 1,
                screen.node,
                screen.w,
                screen.h,
                screen.x,
                screen.y
            );
        }

        let gop = (fps.max(2) / 2).max(1).to_string();
        let bitrate_str = bitrate.to_string();
        let fps_str = fps.max(1).to_string();
        let canvas_caps = format!(
            "video/x-raw,width={},height={},format=I420,framerate={}/1",
            width, height, fps
        );
        let out_caps = format!(
            "video/x-raw,width={},height={},format=I420,framerate={}/1",
            out_w, out_h, fps
        );

        let mut cmd = Command::new("gst-launch-1.0");
        unsafe {
            cmd.pre_exec(move || {
                // Hand the portal's PipeWire fd to the child on a fixed number
                // so every pipewiresrc can share the portal connection.
                if libc::dup2(raw_fd, 100) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }

        let mut args: Vec<String> = vec![
            "-q".into(),
            "compositor".into(),
            "name=comp".into(),
            "background=black".into(),
            "latency=0".into(),
            "ignore-inactive-pads=true".into(),
        ];
        for (i, screen) in screens.iter().enumerate() {
            args.push(format!("sink_{}::xpos={}", i, (screen.x - min_x).max(0)));
            args.push(format!("sink_{}::ypos={}", i, (screen.y - min_y).max(0)));
        }
        args.extend([
            "!".into(),
            canvas_caps,
            "!".into(),
            "videoscale".into(),
            "!".into(),
            out_caps,
            "!".into(),
            "openh264enc".into(),
            format!("bitrate={}", bitrate_str),
            format!("gop-size={}", gop),
            "rate-control=bitrate".into(),
            "complexity=low".into(),
            "usage-type=screen".into(),
            "enable-frame-skip=true".into(),
            "!".into(),
            "h264parse".into(),
            "config-interval=-1".into(),
            "!".into(),
            "video/x-h264,stream-format=byte-stream,alignment=au".into(),
            "!".into(),
            "fdsink".into(),
            "fd=1".into(),
            "sync=false".into(),
        ]);

        for (i, screen) in screens.iter().enumerate() {
            args.extend([
                "pipewiresrc".into(),
                format!("path={}", screen.node),
                "fd=100".into(),
                "do-timestamp=true".into(),
                "max-buffers=2".into(),
                // Statik ekranda PipeWire yeni kare üretmeyebilir; son kareyi
                // düzenli geri göndererek yayının donmasını engelle.
                "keepalive-time=100".into(),
                "!".into(),
                "videorate".into(),
                format!("max-rate={}", fps_str),
                "!".into(),
                "videoconvert".into(),
                "!".into(),
                format!(
                    "video/x-raw,width={},height={},format=I420,framerate={}/1",
                    screen.w, screen.h, fps
                ),
                "!".into(),
                format!("comp.sink_{}", i),
            ]);
        }

        cmd.args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = cmd.spawn()?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("gst-launch stdout unavailable"))?;
        let mut reader = BufReader::new(stdout);
        spawn_stderr_logger(child.stderr.take(), "gst");

        let mut buf = Vec::with_capacity(256 * 1024);
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(20);
        loop {
            if find_start(&buf, 0).is_some() {
                return Ok(Self {
                    child,
                    reader,
                    width,
                    height,
                    out_width: out_w,
                    out_height: out_h,
                    monitors: screens.len(),
                    source: CaptureInput::Portal,
                    fps,
                    buf,
                    au: Vec::with_capacity(256 * 1024),
                    au_has_vcl: false,
                    _portal: Some(PortalKeepAlive {
                        _proxy: proxy,
                        _session: session,
                    }),
                    _kwin_stop: None,
                    _kwin_task: None,
                });
            }
            if let Ok(Some(status)) = child.try_wait() {
                anyhow::bail!("gst-launch exited before producing frames: {}", status);
            }
            let mut tmp = [0u8; 16384];
            let n = match tokio::time::timeout_at(deadline, reader.read(&mut tmp)).await {
                Ok(r) => r?,
                Err(_) => anyhow::bail!("timeout waiting for portal H.264 bytes"),
            };
            if n == 0 {
                anyhow::bail!("gst-launch exited before producing output");
            }
            buf.extend_from_slice(&tmp[..n]);
        }
    }

    /// KDE Plasma Wayland: `org.kde.KWin.ScreenShot2.CaptureWorkspace` ile
    /// portal penceresi olmadan tüm çalışma alanını yakalar. KWin, çağıran
    /// uygulamayı `~/.local/share/applications` altındaki, `Exec=` yolu
    /// `/proc/self/exe` ile eşleşen ve `X-KDE-DBUS-Restricted-Interfaces`
    /// satırını taşıyan bir .desktop dosyasıyla yetkilendirir.
    async fn try_start_kwin(
        out_width: u32,
        out_height: u32,
        fps: u32,
        bitrate: u32,
    ) -> Result<Self> {
        ensure_kwin_authorization().await?;
        let conn = zbus::Connection::session().await?;
        let (first, width, height, stride) = kwin_first_frame(&conn).await?;
        let (out_w, out_h) = fit_size(width, height, out_width, out_height);
        let gop = (fps.max(2) / 2).max(1).to_string();
        let codec = detect_software_codec();

        let mut args: Vec<String> = vec![
            "-loglevel".into(),
            "error".into(),
            "-f".into(),
            "rawvideo".into(),
            "-pix_fmt".into(),
            "bgra".into(),
            "-s".into(),
            format!("{}x{}", width, height),
            "-framerate".into(),
            fps.max(1).to_string(),
            "-i".into(),
            "pipe:0".into(),
            "-vf".into(),
            format!("scale={}:{},format=yuv420p", out_w, out_h),
            "-c:v".into(),
            codec.into(),
        ];
        if codec == "libopenh264" {
            args.extend([
                "-profile:v".into(),
                "constrained_baseline".into(),
                "-rc_mode".into(),
                "1".into(),
                "-allow_skip_frames".into(),
                "1".into(),
                "-threads".into(),
                "2".into(),
            ]);
        } else if codec == "libx264" {
            args.extend([
                "-preset".into(),
                "ultrafast".into(),
                "-tune".into(),
                "zerolatency".into(),
                "-profile:v".into(),
                "baseline".into(),
                "-threads".into(),
                "2".into(),
            ]);
        }
        args.extend([
            "-g".into(),
            gop,
            "-b:v".into(),
            bitrate.to_string(),
            "-maxrate".into(),
            bitrate.to_string(),
            "-bufsize".into(),
            (bitrate / 2).to_string(),
            "-bsf:v".into(),
            "h264_metadata=aud=insert".into(),
            "-f".into(),
            "h264".into(),
            "-flush_packets".into(),
            "1".into(),
            "-".into(),
        ]);

        let mut child = Command::new("ffmpeg")
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("ffmpeg stdin unavailable"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("ffmpeg stdout unavailable"))?;
        let mut reader = BufReader::new(stdout);
        spawn_stderr_logger(child.stderr.take(), "ffmpeg-kwin");

        write_raw_frame(&mut stdin, &first, width, height, stride).await?;

        let stop = Arc::new(AtomicBool::new(false));
        let stop_task = Arc::clone(&stop);
        let conn_task = conn.clone();
        let capture_task = tokio::spawn(async move {
            let proxy = match zbus::Proxy::new(
                &conn_task,
                "org.kde.KWin",
                "/org/kde/KWin/ScreenShot2",
                "org.kde.KWin.ScreenShot2",
            )
            .await
            {
                Ok(p) => p,
                Err(e) => {
                    log::warn!("KWin proxy kurulamadı: {}", e);
                    return;
                }
            };
            let interval = std::time::Duration::from_secs_f64(1.0 / fps.max(1) as f64);
            loop {
                if stop_task.load(Ordering::SeqCst) {
                    break;
                }
                let started = std::time::Instant::now();
                match kwin_capture_frame(&proxy).await {
                    Ok((frame, w, h, s)) => {
                        if w != width || h != height {
                            log::debug!("KWin çözünürlüğü değişti ({}x{}), atlanıyor", w, h);
                            continue;
                        }
                        if let Err(e) = write_raw_frame(&mut stdin, &frame, w, h, s).await {
                            log::debug!("KWin -> ffmpeg yazımı bitti: {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        log::warn!("KWin kare yakalama hatası: {}", e);
                        break;
                    }
                }
                let elapsed = started.elapsed();
                if elapsed < interval {
                    tokio::time::sleep(interval - elapsed).await;
                }
            }
            let _ = stdin.shutdown().await;
        });

        let mut buf = Vec::with_capacity(256 * 1024);
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(8);
        loop {
            if find_start(&buf, 0).is_some() {
                log::info!(
                    "KDE KWin yakalama hazır: {}x{} -> {}x{} @ {}fps",
                    width,
                    height,
                    out_w,
                    out_h,
                    fps
                );
                return Ok(Self {
                    child,
                    reader,
                    width,
                    height,
                    out_width: out_w,
                    out_height: out_h,
                    monitors: 1,
                    source: CaptureInput::KWin,
                    fps,
                    buf,
                    au: Vec::with_capacity(256 * 1024),
                    au_has_vcl: false,
                    _portal: None,
                    _kwin_stop: Some(stop),
                    _kwin_task: Some(capture_task),
                });
            }
            if let Ok(Some(status)) = child.try_wait() {
                anyhow::bail!("KWin ffmpeg exited before frames: {}", status);
            }
            let mut tmp = [0u8; 16384];
            let n = match tokio::time::timeout_at(deadline, reader.read(&mut tmp)).await {
                Ok(r) => r?,
                Err(_) => anyhow::bail!("timeout waiting for KWin H.264 frames"),
            };
            if n == 0 {
                anyhow::bail!("KWin ffmpeg exited before output");
            }
            buf.extend_from_slice(&tmp[..n]);
        }
    }

    /// Read the next encoded H.264 access unit (one complete picture with its
    /// SPS/PPS/SEI NALs). Returns the Annex-B bytes and whether it contains an
    /// IDR/SPS (keyframe).
    pub async fn next_frame(&mut self) -> Result<(Vec<u8>, bool)> {
        loop {
            while let Some(nal) = self.try_extract_nal() {
                if let Some(au) = self.push_nal(nal) {
                    let key = is_keyframe(&au);
                    return Ok((au, key));
                }
            }
            let mut tmp = [0u8; 16384];
            let n = self.reader.read(&mut tmp).await?;
            if n == 0 {
                if let Some(au) = self.flush_au() {
                    let key = is_keyframe(&au);
                    return Ok((au, key));
                }
                anyhow::bail!("H.264 stream ended");
            }
            self.buf.extend_from_slice(&tmp[..n]);
        }
    }

    fn try_extract_nal(&mut self) -> Option<Vec<u8>> {
        let p = find_start(&self.buf, 0)?;
        let q = find_start(&self.buf, p + 3)?;
        let nal = self.buf[p..q].to_vec();
        self.buf.drain(..q);
        Some(nal)
    }

    fn flush_au(&mut self) -> Option<Vec<u8>> {
        if self.au.is_empty() {
            return None;
        }
        let au = std::mem::take(&mut self.au);
        self.au_has_vcl = false;
        Some(au)
    }

    /// Feed one NAL into the current access unit. Returns the previous AU when
    /// this NAL starts a new picture, so each sample passed to WebRTC is a
    /// complete frame with a single RTP timestamp.
    fn push_nal(&mut self, nal: Vec<u8>) -> Option<Vec<u8>> {
        feed_nal(&mut self.au, &mut self.au_has_vcl, nal)
    }
}

impl Drop for H264Capture {
    fn drop(&mut self) {
        if let Some(stop) = &self._kwin_stop {
            stop.store(true, Ordering::SeqCst);
        }
        if let Some(task) = &self._kwin_task {
            task.abort();
        }
        let _ = self.child.start_kill();
    }
}

/// `keyframe` is computed from the AU itself; the argument keeps the old
/// return shape stable for the capture loop.
fn is_keyframe(au: &[u8]) -> bool {
    let mut from = 0;
    while let Some(p) = find_start(au, from) {
        let start_len = if au.get(p + 2) == Some(&1) { 3 } else { 4 };
        let Some(header) = au.get(p + start_len) else {
            break;
        };
        let nal_type = header & 0x1F;
        if nal_type == 5 || nal_type == 7 {
            return true;
        }
        from = p + start_len;
    }
    false
}

/// Groups NALs into access units. Returns the previous AU when `nal` starts a
/// new picture; otherwise appends and returns None.
fn feed_nal(au: &mut Vec<u8>, au_has_vcl: &mut bool, nal: Vec<u8>) -> Option<Vec<u8>> {
    let (nal_type, first_mb_zero) = parse_nal(&nal);
    let is_vcl = matches!(nal_type, 1 | 5);
    let new_picture = if au.is_empty() {
        false
    } else if nal_type == 9 {
        true
    } else if matches!(nal_type, 6 | 7 | 8) {
        // Parameter sets / SEI precede the next picture.
        *au_has_vcl
    } else if is_vcl {
        *au_has_vcl && first_mb_zero
    } else {
        false
    };

    if new_picture {
        let out = std::mem::take(au);
        *au = nal;
        *au_has_vcl = is_vcl;
        Some(out)
    } else {
        au.extend_from_slice(&nal);
        *au_has_vcl |= is_vcl;
        None
    }
}

fn parse_nal(nal: &[u8]) -> (u8, bool) {
    let start_len = if nal.get(2) == Some(&1) { 3 } else { 4 };
    let Some(header) = nal.get(start_len) else {
        return (0, false);
    };
    let nal_type = header & 0x1F;
    // first_mb_in_slice is ue(v); a leading 1 bit means value 0 (first slice).
    let first_mb_zero = nal
        .get(start_len + 1)
        .map(|b| b & 0x80 != 0)
        .unwrap_or(false);
    (nal_type, first_mb_zero)
}

fn build_args(
    backend: EncoderBackend,
    codec: &str,
    capture_input: CaptureInput,
    width: u32,
    height: u32,
    out_width: u32,
    out_height: u32,
    fps: u32,
    bitrate: u32,
) -> Vec<String> {
    let gop = (fps.max(2) / 2).max(1).to_string();
    let br = bitrate.to_string();
    let fps_str = fps.max(1).to_string();
    let input: Vec<String> = match capture_input {
        CaptureInput::Portal | CaptureInput::KWin => vec![], // handled separately
        CaptureInput::PipeWire => vec![
            "-f".into(),
            "pipewire".into(),
            "-i".into(),
            "default".into(),
        ],
        CaptureInput::X11Grab => vec![
            "-f".into(),
            "x11grab".into(),
            "-framerate".into(),
            fps_str.clone(),
            "-video_size".into(),
            format!("{}x{}", width, height),
            "-i".into(),
            format!(
                "{}+0,0",
                std::env::var("DISPLAY").unwrap_or_else(|_| ":0".to_string())
            ),
        ],
    };
    // Shared low-latency output parameters. `h264_metadata=aud=insert` puts an
    // AUD before every access unit so the reader can split frames reliably.
    let tail = vec![
        "-bsf:v".into(),
        "h264_metadata=aud=insert".into(),
        "-f".into(),
        "h264".into(),
        "-flush_packets".into(),
        "1".into(),
        "-".into(),
    ];
    match backend {
        EncoderBackend::Vaapi => vec!["-loglevel".into(), "error".into()]
            .into_iter()
            .chain(input)
            .chain(vec![
                "-vaapi_device".into(),
                "/dev/dri/renderD128".into(),
                "-vf".into(),
                format!("scale={}:{},format=nv12,hwupload", out_width, out_height),
                "-c:v".into(),
                codec.into(),
                "-profile:v".into(),
                "baseline".into(),
                "-bf".into(),
                "0".into(),
                "-g".into(),
                gop,
                "-b:v".into(),
                br,
                "-maxrate".into(),
                format!("{}", bitrate),
                "-bufsize".into(),
                format!("{}", bitrate / 2),
            ])
            .chain(tail)
            .collect::<Vec<_>>(),
        EncoderBackend::Nvenc => vec!["-loglevel".into(), "error".into()]
            .into_iter()
            .chain(input)
            .chain(vec![
                "-c:v".into(),
                codec.into(),
                "-preset".into(),
                "p1".into(),
                "-tune".into(),
                "ull".into(),
                "-zerolatency".into(),
                "1".into(),
                "-rc".into(),
                "cbr_ld_hq".into(),
                "-delay".into(),
                "0".into(),
                "-profile:v".into(),
                "baseline".into(),
                "-g".into(),
                gop,
                "-b:v".into(),
                br,
            ])
            .chain(tail)
            .collect::<Vec<_>>(),
        EncoderBackend::Software => {
            let mut args: Vec<String> = vec!["-loglevel".into(), "error".into()]
                .into_iter()
                .chain(input)
                .chain(vec![
                    "-vf".into(),
                    format!("scale={}:{},format=yuv420p", out_width, out_height),
                    "-c:v".into(),
                    codec.into(),
                ])
                .collect();
            // libopenh264 uses its own profile spelling; libx264/h264 use
            // the standard ffmpeg profile names.
            if codec == "libopenh264" {
                args.extend(["-profile:v".into(), "constrained_baseline".into()]);
            } else if codec == "libx264" {
                args.extend(["-profile:v".into(), "baseline".into()]);
            }
            // libx264-only latency tuning; other codecs reject unknown options.
            if codec == "libx264" {
                args.extend([
                    "-preset".into(),
                    "ultrafast".into(),
                    "-tune".into(),
                    "zerolatency".into(),
                    "-threads".into(),
                    "2".into(),
                    "-x264-params".into(),
                    "rc-lookahead=0:sync-lookahead=0".into(),
                ]);
            } else if codec == "libopenh264" {
                args.extend([
                    "-rc_mode".into(),
                    "1".into(), // bitrate mode
                    "-allow_skip_frames".into(),
                    "1".into(),
                    "-threads".into(),
                    "2".into(),
                ]);
            }
            args.extend([
                "-g".into(),
                gop,
                "-b:v".into(),
                br,
                "-maxrate".into(),
                format!("{}", bitrate),
                "-bufsize".into(),
                format!("{}", bitrate / 2),
            ]);
            args.extend(tail);
            args
        }
    }
}

/// KDE Plasma Wayland mı?
fn is_kde_wayland() -> bool {
    if std::env::var_os("WAYLAND_DISPLAY").is_none() {
        return false;
    }
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    desktop.to_ascii_lowercase().contains("kde") || std::env::var_os("KDE_FULL_SESSION").is_some()
}

/// KWin ScreenShot2, çağıran uygulamayı `.desktop` dosyasındaki `Exec=` yolu
/// ile `/proc/self/exe` eşleşmesine göre yetkilendirir. AppImage her açılışta
/// farklı bir mount yolu kullanabildiği için dosya her seferinde güncellenir.
fn write_kwin_desktop_file() -> Result<bool> {
    let exe = std::env::current_exe()?;
    let data_home = if let Ok(v) = std::env::var("XDG_DATA_HOME") {
        std::path::PathBuf::from(v)
    } else {
        std::path::PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string()))
            .join(".local")
            .join("share")
    };
    let dir = data_home.join("applications");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("PCHome-desktop.desktop");
    let content = format!(
        "[Desktop Entry]\nType=Application\nName=PCHome\nExec={}\nIcon=PCHome\nTerminal=false\nNoDisplay=true\nCategories=Network;RemoteAccess;\nX-KDE-DBUS-Restricted-Interfaces=org.kde.KWin.ScreenShot2\n",
        exe.display()
    );
    let changed = std::fs::read_to_string(&path)
        .map(|old| old != content)
        .unwrap_or(true);
    if changed {
        std::fs::write(&path, &content)?;
        log::info!("KWin ekran yakalama izni yazıldı: {}", path.display());
    }
    Ok(changed)
}

fn refresh_kwin_cache_sync() {
    for bin in ["kbuildsycoca6", "kbuildsycoca5"] {
        if std::process::Command::new(bin)
            .arg("--noincremental")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            break;
        }
    }
}

/// GUI penceresi açılmadan ÖNCE çağrılır: KWin, uygulamanın desktop
/// dosyasını pencere oluşurken önbelleğe aldığı için ilk açılışta da
/// yetkilendirme hazır olmalı.
pub fn preauthorize_kde_capture() {
    if !is_kde_wayland() {
        return;
    }
    match write_kwin_desktop_file() {
        Ok(_) => refresh_kwin_cache_sync(),
        Err(e) => log::warn!("KWin izin dosyası yazılamadı: {}", e),
    }
}

async fn ensure_kwin_authorization() -> Result<()> {
    let changed = write_kwin_desktop_file()?;
    if changed {
        let refreshed = tokio::process::Command::new("kbuildsycoca6")
            .arg("--noincremental")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map(|s| s.success())
            .unwrap_or(false);
        if !refreshed {
            let _ = tokio::process::Command::new("kbuildsycoca5")
                .arg("--noincremental")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .await;
        }
    }
    Ok(())
}

async fn refresh_kwin_cache_async() {
    for bin in ["kbuildsycoca6", "kbuildsycoca5"] {
        let ok = tokio::process::Command::new(bin)
            .arg("--noincremental")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map(|s| s.success())
            .unwrap_or(false);
        if ok {
            break;
        }
    }
}

async fn kwin_first_frame(conn: &zbus::Connection) -> Result<(Vec<u8>, u32, u32, usize)> {
    let proxy = zbus::Proxy::new(
        conn,
        "org.kde.KWin",
        "/org/kde/KWin/ScreenShot2",
        "org.kde.KWin.ScreenShot2",
    )
    .await?;
    let mut last_err: Option<anyhow::Error> = None;
    for attempt in 0..4 {
        match kwin_capture_frame(&proxy).await {
            Ok(frame) => return Ok(frame),
            Err(e) => {
                log::warn!("KWin yetkilendirme denemesi {}: {}", attempt + 1, e);
                last_err = Some(e);
                tokio::time::sleep(std::time::Duration::from_millis(600)).await;
                refresh_kwin_cache_async().await;
            }
        }
    }
    Err(last_err.unwrap_or_else(|| anyhow::anyhow!("KWin ekran yakalama başarısız")))
}

/// GUI penceresi oluşmadan önce KWin yetkisini doğrular. KWin ilk açılışta
/// desktop dosyası önbelleğini gecikmeli görüyor; birkaç deneme ile ısıtılır.
pub async fn preflight_kde_capture() -> bool {
    if !is_kde_wayland() {
        return false;
    }
    if let Err(e) = ensure_kwin_authorization().await {
        log::warn!("KWin izin hazırlığı başarısız: {}", e);
        return false;
    }
    let conn = match zbus::Connection::session().await {
        Ok(c) => c,
        Err(e) => {
            log::warn!("KWin DBus bağlantısı yok: {}", e);
            return false;
        }
    };
    match kwin_first_frame(&conn).await {
        Ok((_, w, h, _)) => {
            log::info!("KDE KWin ekran yakalama ön izni hazır ({}x{})", w, h);
            true
        }
        Err(e) => {
            log::warn!("KWin ön izni alınamadı: {}", e);
            false
        }
    }
}

fn kwin_memfd() -> Result<std::fs::File> {
    use std::os::fd::FromRawFd;
    let name = std::ffi::CString::new("pchome-kwin-capture")?;
    let fd = unsafe { libc::memfd_create(name.as_ptr(), libc::MFD_CLOEXEC) };
    if fd < 0 {
        anyhow::bail!("memfd_create failed: {}", std::io::Error::last_os_error());
    }
    Ok(unsafe { std::fs::File::from_raw_fd(fd) })
}

async fn kwin_capture_frame(proxy: &zbus::Proxy<'_>) -> Result<(Vec<u8>, u32, u32, usize)> {
    use std::os::fd::OwnedFd as StdOwnedFd;

    let mut options: HashMap<String, zbus::Variant<'_>> = HashMap::new();
    options.insert("include-cursor".to_string(), zbus::Variant::from(true));
    let mut file = kwin_memfd()?;
    let pass: zbus::OwnedFd = zbus::OwnedFd::from(StdOwnedFd::from(file.try_clone()?));
    let reply: HashMap<String, zbus::OwnedVariant> =
        proxy.call("CaptureWorkspace", &(options, pass)).await?;
    let width = reply
        .get("width")
        .and_then(|v| u32::try_from(v).ok())
        .unwrap_or(0);
    let height = reply
        .get("height")
        .and_then(|v| u32::try_from(v).ok())
        .unwrap_or(0);
    let stride = reply
        .get("stride")
        .and_then(|v| u32::try_from(v).ok())
        .unwrap_or(0) as usize;
    if width == 0 || height == 0 || stride == 0 {
        anyhow::bail!("KWin geçersiz görüntü boyutu döndürdü");
    }
    let len = stride.saturating_mul(height as usize);
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(600);
    loop {
        if let Ok(meta) = file.metadata() {
            if meta.len() as usize >= len {
                break;
            }
        }
        if tokio::time::Instant::now() >= deadline {
            anyhow::bail!("KWin görüntüsü zamanında yazılmadı");
        }
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
    }
    file.seek(SeekFrom::Start(0))?;
    let mut buf = vec![0u8; len];
    file.read_exact(&mut buf)?;
    Ok((buf, width, height, stride))
}

async fn write_raw_frame(
    stdin: &mut tokio::process::ChildStdin,
    buf: &[u8],
    width: u32,
    height: u32,
    stride: usize,
) -> Result<()> {
    let row = width as usize * 4;
    if stride == row {
        let len = row.saturating_mul(height as usize);
        stdin.write_all(&buf[..len]).await?;
    } else {
        for y in 0..height as usize {
            let start = y.saturating_mul(stride);
            stdin
                .write_all(&buf[start..(start + row).min(buf.len())])
                .await?;
        }
    }
    Ok(())
}

fn find_start(buf: &[u8], from: usize) -> Option<usize> {
    let mut i = from;
    while i + 3 <= buf.len() {
        if buf[i] == 0 && buf[i + 1] == 0 {
            if buf[i + 2] == 1 {
                return Some(i);
            }
            if buf[i + 2] == 0 && i + 4 <= buf.len() && buf[i + 3] == 1 {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

/// Scale `target` down to fit inside `native` while keeping aspect ratio.
fn fit_size(native_w: u32, native_h: u32, target_w: u32, target_h: u32) -> (u32, u32) {
    if native_w == 0 || native_h == 0 || target_w == 0 || target_h == 0 {
        return (target_w.max(2) & !1, target_h.max(2) & !1);
    }
    let by_w = target_w as f64 / native_w as f64;
    let by_h = target_h as f64 / native_h as f64;
    let scale = by_w.min(by_h).min(1.0);
    let w = ((native_w as f64 * scale) as u32).max(2) & !1;
    let h = ((native_h as f64 * scale) as u32).max(2) & !1;
    (w, h)
}

/// X11 screen size from `xrandr`; used so the advertised resolution and the
/// uinput absolute range match what ffmpeg actually captures.
fn detect_x11_size() -> Option<(u32, u32)> {
    let display = std::env::var("DISPLAY").ok()?;
    let out = std::process::Command::new("xrandr")
        .args(["--display", &display, "--current"])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    for line in text.lines() {
        if !line.contains(" connected") {
            continue;
        }
        // e.g. "eDP-1 connected primary 1920x1080+0+0 ..."
        for token in line.split_whitespace() {
            let geom = token.split('+').next().unwrap_or(token);
            if let Some((w, h)) = geom.split_once('x') {
                if let (Ok(w), Ok(h)) = (w.parse::<u32>(), h.parse::<u32>()) {
                    if w > 0 && h > 0 {
                        return Some((w, h));
                    }
                }
            }
        }
    }
    None
}

fn spawn_stderr_logger(stderr: Option<tokio::process::ChildStderr>, label: &'static str) {
    let Some(stderr) = stderr else {
        return;
    };
    tokio::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            log::warn!("{}: {}", label, line);
        }
    });
}

fn token_path() -> Option<std::path::PathBuf> {
    let dir = if let Ok(base) = std::env::var("XDG_CONFIG_HOME") {
        std::path::PathBuf::from(base).join("PCHome")
    } else {
        std::path::PathBuf::from(std::env::var("HOME").ok()?)
            .join(".config")
            .join("PCHome")
    };
    Some(dir.join("screencast.token"))
}

fn read_restore_token() -> Option<String> {
    let path = token_path()?;
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn save_restore_token(token: &str) {
    let Some(path) = token_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(target_family = "unix")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    if let Ok(mut f) = options.open(path) {
        let _ = writeln!(f, "{}", token);
    }
}

#[cfg(test)]
mod enc_tests {
    use super::*;

    fn nal(nal_type: u8, first_mb_zero: bool) -> Vec<u8> {
        // 4-byte start code + NAL header + one payload byte whose MSB encodes
        // first_mb_in_slice == 0.
        vec![
            0,
            0,
            0,
            1,
            nal_type,
            if first_mb_zero { 0x80 } else { 0x40 },
        ]
    }

    #[test]
    fn boyut_hedefi_oran_korur() {
        assert_eq!(fit_size(1920, 1080, 1280, 720), (1280, 720));
        assert_eq!(fit_size(1920, 1200, 1280, 720), (1152, 720));
        assert_eq!(fit_size(1280, 720, 1920, 1080), (1280, 720));
    }

    #[test]
    fn nal_baslangic_bilgisi() {
        assert_eq!(parse_nal(&nal(7, false)), (7, false));
        assert_eq!(parse_nal(&nal(5, true)), (5, true));
    }

    #[test]
    fn erisim_birimi_nal_lari_birlestirir() {
        let mut au = Vec::new();
        let mut has_vcl = false;
        assert!(feed_nal(&mut au, &mut has_vcl, nal(7, false)).is_none()); // SPS
        assert!(feed_nal(&mut au, &mut has_vcl, nal(8, false)).is_none()); // PPS
        assert!(feed_nal(&mut au, &mut has_vcl, nal(5, true)).is_none()); // IDR
                                                                          // İkinci dilim (first_mb_in_slice != 0) aynı AU'ya aittir.
        assert!(feed_nal(&mut au, &mut has_vcl, nal(5, false)).is_none());
        // Yeni resim başlayınca önceki AU döner.
        let out = feed_nal(&mut au, &mut has_vcl, nal(1, true)).expect("AU dönmeli");
        assert!(out.windows(2).any(|w| w == [0, 0]));
        assert!(out.len() >= 4 * 3);
        assert_eq!(au.len(), 6);
    }

    #[test]
    fn aud_yeni_birim_baslatir() {
        let mut au = Vec::new();
        let mut has_vcl = false;
        feed_nal(&mut au, &mut has_vcl, nal(5, true));
        let out = feed_nal(&mut au, &mut has_vcl, nal(9, false)).expect("AUD önceki AU'yu kapatır");
        assert!(out.len() >= 6);
        assert!(has_vcl == false);
    }
}
