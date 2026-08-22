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

pub fn detect_backend() -> EncoderBackend {
    if std::path::Path::new("/dev/dri/renderD128").exists() {
        return EncoderBackend::Vaapi;
    }
    if std::path::Path::new("/dev/nvidia0").exists() {
        return EncoderBackend::Nvenc;
    }
    log::warn!("No hardware encoder found, falling back to software (libx264)");
    EncoderBackend::Software
}

/// Spawns an `ffmpeg` process that captures the desktop via PipeWire and encodes
/// it to an Annex-B H.264 elementary stream on stdout, using VA-API / NVENC when
/// available and falling back to libx264.
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
        let backend = detect_backend();
        log::info!("Capture encoder backend: {:?}", backend);

        let ff = which("ffmpeg").unwrap_or_else(|_| std::path::PathBuf::from("ffmpeg"));
        let args = build_args(backend, width, height, fps, bitrate);
        log::debug!("ffmpeg args: {:?} {:?}", ff, args);

        let mut child = Command::new(ff)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("ffmpeg stdout unavailable"))?;
        let reader = BufReader::new(stdout);

        Ok(Self {
            child,
            reader,
            width,
            height,
            fps,
            buf: Vec::with_capacity(65_536),
        })
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
    width: u32,
    height: u32,
    fps: u32,
    bitrate: u32,
) -> Vec<String> {
    let gop = (fps * 2).to_string();
    let br = bitrate.to_string();
    match backend {
        EncoderBackend::Vaapi => vec![
            "-loglevel".into(),
            "error".into(),
            "-f".into(),
            "pipewire".into(),
            "-i".into(),
            "default".into(),
            "-vaapi_device".into(),
            "/dev/dri/renderD128".into(),
            "-vf".into(),
            "format=nv12,hwupload".into(),
            "-c:v".into(),
            "h264_vaapi".into(),
            "-g".into(),
            gop,
            "-b:v".into(),
            br,
            "-f".into(),
            "h264".into(),
            "-".into(),
        ],
        EncoderBackend::Nvenc => vec![
            "-loglevel".into(),
            "error".into(),
            "-f".into(),
            "pipewire".into(),
            "-i".into(),
            "default".into(),
            "-c:v".into(),
            "h264_nvenc".into(),
            "-preset".into(),
            "p1".into(),
            "-g".into(),
            gop,
            "-b:v".into(),
            br,
            "-f".into(),
            "h264".into(),
            "-".into(),
        ],
        EncoderBackend::Software => vec![
            "-loglevel".into(),
            "error".into(),
            "-f".into(),
            "pipewire".into(),
            "-i".into(),
            "default".into(),
            "-vf".into(),
            "format=yuv420p".into(),
            "-c:v".into(),
            "libx264".into(),
            "-preset".into(),
            "ultrafast".into(),
            "-tune".into(),
            "zerolatency".into(),
            "-g".into(),
            gop,
            "-b:v".into(),
            br,
            "-f".into(),
            "h264".into(),
            "-".into(),
        ],
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
