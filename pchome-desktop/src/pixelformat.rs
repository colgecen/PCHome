/// Pixel format four-character code, shared between the encoder and the capture
/// pipeline so the two modules agree on a single definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FourCC(pub u32);

impl FourCC {
    pub const NV12: Self = Self(0x3231_564E);
    pub const I420: Self = Self(0x3032_3449);
    pub const RGB24: Self = Self(0x0000_0020);
    pub const XRGB8888: Self = Self(0x3432_5258);
    pub const ARGB8888: Self = Self(0x3432_5241);

    pub fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    pub fn raw(self) -> u32 {
        self.0
    }
}
