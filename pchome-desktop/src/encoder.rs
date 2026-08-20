use anyhow::{Context, Result};
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EncodeError {
    #[error("Unsupported pixel format")]
    UnsupportedFormat,
    #[error("Hardware encoder initialization failed: {0}")]
    HardwareInitFailed(String),
    #[error("Software encoder initialization failed: {0}")]
    SoftwareInitFailed(String),
    #[error("Encoding failed: {0}")]
    EncodeFailed(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FourCC(u32);

impl FourCC {
    pub const NV12: Self = Self(0x3231564E);
    pub const I420: Self = Self(0x30323449);
    pub const RGB24: Self = Self(0x00000020);
    pub const XRGB8888: Self = Self(0x34325258);
}

pub enum EncoderBackend {
    Vaapi,
    Nvenc,
    Software,
}

pub struct Encoder {
    backend: EncoderBackend,
    width: u32,
    height: u32,
    bitrate: u32,
    framerate: u32,
}

impl Encoder {
    pub fn new(
        backend: EncoderBackend,
        width: u32,
        height: u32,
        bitrate: u32,
        framerate: u32,
    ) -> Self {
        Self {
            backend,
            width,
            height,
            bitrate,
            framerate,
        }
    }

    pub fn init_encoder() -> Result<Self> {
        let backend = Self::detect_backend();
        info!("Encoder backend selected: {:?}", backend);
        Ok(Self {
            backend,
            width: 1920,
            height: 1080,
            bitrate: 4_000_000,
            framerate: 60,
        })
    }

    fn detect_backend() -> EncoderBackend {
        if std::path::Path::new("/dev/dri/renderD128").exists() {
            return EncoderBackend::Vaapi;
        }
        if std::path::Path::new("/dev/nvidia0").exists() {
            return EncoderBackend::Nvenc;
        }
        warn!("No hardware encoder found, falling back to software");
        EncoderBackend::Software
    }

    pub async fn encode(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        match self.backend {
            EncoderBackend::Vaapi => self.encode_vaapi(data).await,
            EncoderBackend::Nvenc => self.encode_nvenc(data).await,
            EncoderBackend::Software => self.encode_software(data).await,
        }
    }

    async fn encode_vaapi(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        warn!("VA-API hardware encode stub");
        self.encode_software(data).await
    }

    async fn encode_nvenc(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        warn!("NVENC hardware encode stub");
        self.encode_software(data).await
    }

    async fn encode_software(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        Ok(data.to_vec())
    }
}
