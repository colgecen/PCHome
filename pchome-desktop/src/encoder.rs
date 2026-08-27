use anyhow::Result;
use std::process::Stdio;
use tokio::io::{AsyncReadExt, BufReader};
use tokio::process::Command;
use which::which;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncoderBackend {
    Vaapi,
    Nvenc,
    Software,
}

/// How ffmpeg reads the desktop. `-f pipewire` only exists in custom
/// ffmpeg builds, so we probe for it and fall back to x11grab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureInput {
    PipeWire,
    X11Grab,
}

pub fn detect_capture_input() -> Result<CaptureInput> {
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
        "no usable ffmpeg capture input: build ffmpeg with pipewire support \
         or run the daemon inside a session with DISPLAY set (X11/XWayland)"
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
    fps: u32,
    bitrate: u32,
) -> Vec<(EncoderBackend, Vec<String>)> {
    let mut out = Vec::new();
    match detect_backend() {
        EncoderBackend::Vaapi => out.push((
            EncoderBackend::Vaapi,
            build_args(EncoderBackend::Vaapi, "h264_vaapi", capture_input, width, height, fps, bitrate),
        )),
        EncoderBackend::Nvenc => out.push((
            EncoderBackend::Nvenc,
            build_args(EncoderBackend::Nvenc, "h264_nvenc", capture_input, width, height, fps, bitrate),
        )),
        EncoderBackend::Software => {}
    }
    let sw = detect_software_codec();
    out.push((
        EncoderBackend::Software,
        build_args(EncoderBackend::Software, sw, capture_input, width, height, fps, bitrate),
    ));
    out
}

/// Spawns an `ffmpeg` process that captures the desktop via PipeWire and encodes
/// it to an Annex-B H.264 elementary stream on stdout, trying hardware
/// encoders (VA-API / NVENC) first and falling back to a working software
/// codec (libx264 / libopenh264). A candidate is only accepted once ffmpeg
/// actually produces its first H.264 bytes, so broken hardware init (e.g.
/// VA-API driver failures) transparently falls through to the next option.
pub struct H264Capture {
    child: tokio::process::Child,
    reader: BufReader<tokio::process::ChildStdout>,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    buf: Vec<u8>,
}

impl H264Capture {
    pub async fn spawn(
        width: u32,
        height: u32,
        fps: u32,
        bitrate: u32,
    ) -> Result<Self> {
        let capture_input = detect_capture_input()?;
        log::info!("Capture input: {:?}", capture_input);

        let ff = which("ffmpeg").unwrap_or_else(|_| std::path::PathBuf::from("ffmpeg"));
        let candidates = candidate_configs(capture_input, width, height, fps, bitrate);

        let mut last_err = None;
        for (backend, args) in &candidates {
            log::info!("Trying encoder backend {:?}: {} ...", backend, args.join(" "));
            match Self::try_start(&ff, args.clone(), width, height, fps).await {
                Ok(capture) => {
                    log::info!("Encoder backend {:?} is producing frames", backend);
                    return Ok(capture);
                }
                Err(e) => {
                    log::warn!("Encoder backend {:?} failed: {}", backend, e);
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
        width: u32,
        height: u32,
        fps: u32,
    ) -> Result<Self> {
        let mut child = Command::new(ff)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("ffmpeg stdout unavailable"))?;
        let mut reader = BufReader::new(stdout);

        let mut buf = Vec::with_capacity(65_536);
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(8);
        loop {
            if find_start(&buf, 0).is_some() {
                return Ok(Self {
                    child,
                    reader,
                    width,
                    height,
                    fps,
                    buf,
                });
            }
            let mut tmp = [0u8; 8192];
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

    /// Read the next encoded H.264 frame (one or more NALs up to the next start
    /// code). Returns the raw Annex-B bytes and whether it is a keyframe (SPS present).
    pub async fn next_frame(&mut self) -> Result<(Vec<u8>, bool)> {
        loop {
            if let Some((frame, key)) = self.try_extract() {
                return Ok((frame, key));
            }
            let mut tmp = [0u8; 8192];
            let n = self.reader.read(&mut tmp).await?;
            if n == 0 {
                if let Some((frame, key)) = self.try_extract_eof() {
                    return Ok((frame, key));
                }
                anyhow::bail!("ffmpeg H.264 stream ended");
            }
            self.buf.extend_from_slice(&tmp[..n]);
        }
    }

    fn try_extract(&mut self) -> Option<(Vec<u8>, bool)> {
        let p = find_start(&self.buf, 0)?;
        let q = find_start(&self.buf, p + 3)?;
        let frame = self.buf[p..q].to_vec();
        let key = is_keyframe(&self.buf, p);
        self.buf.drain(..q);
        Some((frame, key))
    }

    fn try_extract_eof(&mut self) -> Option<(Vec<u8>, bool)> {
        let p = find_start(&self.buf, 0)?;
        if p == 0 && self.buf.len() <= 4 {
            return None;
        }
        let frame = self.buf[p..].to_vec();
        let key = is_keyframe(&self.buf, p);
        self.buf.drain(..p);
        Some((frame, key))
    }
}

impl Drop for H264Capture {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
}

fn build_args(
    backend: EncoderBackend,
    codec: &str,
    capture_input: CaptureInput,
    _width: u32,
    _height: u32,
    fps: u32,
    bitrate: u32,
) -> Vec<String> {
    let gop = (fps * 2).to_string();
    let br = bitrate.to_string();
    let fps_str = fps.to_string();
    let input: Vec<String> = match capture_input {
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
            fps_str,
            "-i".into(),
            format!(
                "{}",
                std::env::var("DISPLAY").unwrap_or_else(|_| ":0".to_string())
            ),
        ],
    };
    match backend {
        EncoderBackend::Vaapi => vec![
            "-loglevel".into(),
            "error".into(),
        ]
        .into_iter()
        .chain(input)
        .chain(vec![
            "-vaapi_device".into(),
            "/dev/dri/renderD128".into(),
            "-vf".into(),
            "format=nv12,hwupload".into(),
            "-c:v".into(),
            codec.into(),
            "-g".into(),
            gop,
            "-b:v".into(),
            br,
            "-f".into(),
            "h264".into(),
            "-".into(),
        ])
        .collect::<Vec<_>>(),
        EncoderBackend::Nvenc => vec!["-loglevel".into(), "error".into()]
            .into_iter()
            .chain(input)
            .chain(vec![
                "-c:v".into(),
                codec.into(),
                "-preset".into(),
                "p1".into(),
                "-g".into(),
                gop,
                "-b:v".into(),
                br,
                "-f".into(),
                "h264".into(),
                "-".into(),
            ])
            .collect::<Vec<_>>(),
        EncoderBackend::Software => {
            let mut args: Vec<String> = vec!["-loglevel".into(), "error".into()]
                .into_iter()
                .chain(input)
                .chain(vec![
                    "-vf".into(),
                    "format=yuv420p".into(),
                    "-c:v".into(),
                    codec.into(),
                ])
                .collect();
            // libx264-only latency tuning; other codecs ignore unknown
            // options with an error, so only add them when applicable.
            if codec == "libx264" {
                args.extend(["-preset".into(), "ultrafast".into(), "-tune".into(), "zerolatency".into()]);
            }
            args.extend([
                "-g".into(),
                gop,
                "-b:v".into(),
                br,
                "-f".into(),
                "h264".into(),
                "-".into(),
            ]);
            args
        }
    }
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

fn is_keyframe(buf: &[u8], start: usize) -> bool {
    let start_len = if buf[start + 2] == 1 { 3 } else { 4 };
    if start + start_len < buf.len() {
        let nal = buf[start + start_len] & 0x1F;
        return nal == 7; // SPS => keyframe
    }
    false
}
