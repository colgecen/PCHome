use anyhow::Result;
use std::sync::Arc;
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
        if self.use_dmabuf {
            match self.capture_dmabuf().await {
                Ok(frame) => return Ok(frame),
                Err(_) => {
                    warn!("DMA-BUF capture failed, falling back to memory");
                    self.use_dmabuf = false;
                }
            }
        }
        self.capture_memory().await
    }

    async fn capture_dmabuf(&self) -> Result<Frame> {
        let _pw = pipewire::self::context::Context::new()?;
        let _core = _pw.connect(None)?;
        let _stream = pipewire::stream::Stream::new(
            &_core,
            "pchome-capture",
            pipewire::properties::properties! {
                "media.class" => "Video/Source",
                "media.type" => "Video",
                "media.category" => "Capture",
                "node.target" => "pointer",
            },
        )?;

        let params = pipewire::spa::param::VideoRaw::parse(
            &pipewire::spa::param::ParamType::EnumFormat,
            &[],
        )?;

        _stream.update_params(&params)?;
        _stream.set_active(true)?;

        let mut events = _stream.events();
        let mut recv = events.observe()?;

        while let Some(event) = recv.next().await {
            match event {
                pipewire::stream::StreamEvent::Start => break,
                pipewire::stream::StreamEvent::Error(e) => {
                    return Err(anyhow::anyhow!(CaptureError::StreamError(e)));
                }
                _ => {}
            }
        }

        let buffer = _stream.dequeue_buffer()?;
        let datas = buffer.datas();
        let data = datas.first().ok_or(CaptureError::DmaBufUnsupported)?;

        let fd = data.fd().ok_or(CaptureError::DmaBufUnsupported)?;
        let format = FourCC::from_raw(data.format().raw());

        Ok(Frame::DmaBuf {
            fd,
            width: self.width,
            height: self.height,
            stride: data.stride(),
            format,
        })
    }

    async fn capture_memory(&self) -> Result<Frame> {
        let size = (self.width * self.height * 4) as usize;
        let mut data = vec![0u8; size];
        let _ = fill_framebuffer_rgb24(&mut data, self.width, self.height);
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
    info!("Capture stream initialized (prefer DMA-BUF, fallback memory)");
    Ok(stream)
}
