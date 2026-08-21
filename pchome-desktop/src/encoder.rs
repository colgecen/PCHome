use crate::metrics::{BITRATE_BYTES_TOTAL, ENCODE_LATENCY, ERRORS_TOTAL, FRAMES_ENCODED};
use anyhow::Result;
use std::time::Instant;
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

pub use crate::pixelformat::FourCC;

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
        log::info!("Encoder backend selected: {:?}", backend);
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
        log::warn!("No hardware encoder found, falling back to software");
        EncoderBackend::Software
    }

    pub async fn encode(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        let start = Instant::now();
        let result = match self.backend {
            EncoderBackend::Vaapi => self.encode_vaapi(data).await,
            EncoderBackend::Nvenc => self.encode_nvenc(data).await,
            EncoderBackend::Software => self.encode_software(data).await,
        };

        let elapsed = start.elapsed();
        ENCODE_LATENCY.observe(elapsed.as_secs_f64() * 1000.0);

        match result {
            Ok(bytes) => {
                FRAMES_ENCODED.inc();
                BITRATE_BYTES_TOTAL.inc_by(bytes.len() as u64);
                Ok(bytes)
            }
            Err(e) => {
                ERRORS_TOTAL.inc();
                Err(e)
            }
        }
    }

    async fn encode_vaapi(&mut self, _data: &[u8]) -> Result<Vec<u8>> {
        Err(EncodeError::HardwareInitFailed(
            "VA-API hardware encoding is not implemented yet".into(),
        ))
    }

    async fn encode_nvenc(&mut self, _data: &[u8]) -> Result<Vec<u8>> {
        Err(EncodeError::HardwareInitFailed(
            "NVENC hardware encoding is not implemented yet".into(),
        ))
    }

    async fn encode_software(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        Ok(data.to_vec())
    }
}
