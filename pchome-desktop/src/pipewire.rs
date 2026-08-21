use crate::metrics::CAPTURE_LATENCY;
use anyhow::Result;
use std::time::Instant;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CaptureError {
    #[error("PipeWire not available")]
    PipeWireUnavailable,
    #[error("DMA-BUF format not supported")]
    DmaBufUnsupported,
    #[error("Framebuffer fallback failed")]
    FramebufferFallbackFailed,
    #[error("Capture stream error: {0}")]
    StreamError(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FourCC(u32);

impl FourCC {
    pub const RGB24: Self = Self(0x00000020);
    pub const XRGB8888: Self = Self(0x34325258);
    pub const ARGB8888: Self = Self(0x34325241);
    pub const NV12: Self = Self(0x3231564E);

    pub fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    pub fn raw(&self) -> u32 {
        self.0
    }
}

#[cfg(target_family = "unix")]
#[derive(Debug, Clone)]
pub enum Frame {
    DmaBuf {
        fd: std::os::unix::io::RawFd,
        width: u32,
        height: u32,
        stride: u32,
        format: FourCC,
    },
    Memory {
        data: Vec<u8>,
        width: u32,
        height: u32,
        stride: u32,
        format: FourCC,
    },
}

#[cfg(not(target_family = "unix"))]
#[derive(Debug, Clone)]
pub enum Frame {
    Memory {
        data: Vec<u8>,
        width: u32,
        height: u32,
        stride: u32,
        format: FourCC,
    },
}

pub struct CaptureStream {
    width: u32,
    height: u32,
    framerate: u32,
    use_dmabuf: bool,
}

impl CaptureStream {
    pub fn new(width: u32, height: u32, framerate: u32) -> Self {
        Self {
            width,
            height,
            framerate,
            use_dmabuf: true,
        }
    }

    pub async fn next_frame(&mut self) -> Result<Frame> {
        let start = Instant::now();
        #[cfg(target_family = "unix")]
        if self.use_dmabuf {
            match self.capture_dmabuf().await {
                Ok(frame) => {
                    CAPTURE_LATENCY.observe(start.elapsed().as_secs_f64() * 1000.0);
                    return Ok(frame);
                }
                Err(_) => {
                    log::warn!("DMA-BUF capture failed, falling back to memory");
                    self.use_dmabuf = false;
                }
            }
        }
        let frame = self.capture_memory().await;
        CAPTURE_LATENCY.observe(start.elapsed().as_secs_f64() * 1000.0);
        frame
    }

    #[cfg(target_family = "unix")]
    async fn capture_dmabuf(&self) -> Result<Frame> {
        log::warn!("DMA-BUF capture not implemented, falling back");
        Err(CaptureError::PipeWireUnavailable.into())
    }

    fn capture_memory(&self) -> Result<Frame> {
        let size = (self.width * self.height * 4) as usize;
        let mut data = vec![0u8; size];
        fill_framebuffer_rgb24(&mut data, self.width, self.height);
        Ok(Frame::Memory {
            data,
            width: self.width,
            height: self.height,
            stride: self.width * 4,
            format: FourCC::RGB24,
        })
    }
}

fn fill_framebuffer_rgb24(buf: &mut [u8], width: u32, height: u32) {
    for y in 0..height {
        for x in 0..width {
            let idx = ((y * width + x) * 4) as usize;
            buf[idx] = ((x * 255) / width) as u8;
            buf[idx + 1] = ((y * 255) / height) as u8;
            buf[idx + 2] = 128;
            buf[idx + 3] = 255;
        }
    }
}

pub fn init_capture() -> Result<CaptureStream> {
    let stream = CaptureStream::new(1920, 1080, 60);
    log::info!("Capture stream initialized (prefer DMA-BUF, fallback memory)");
    Ok(stream)
}
