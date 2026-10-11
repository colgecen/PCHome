use anyhow::Result;

/// GUI önizlemesinden yerel tıklama: önizlemeye dokunma = sol tık,
/// uzun basma = sağ tık. Daemon'un uinput tutamacına dokunulmaz; GUI ilk
/// etkileşimde kendi sanal cihazını açar (`pchome-yerel-tiklayici`).
pub struct LocalClicker {
    #[cfg(all(feature = "capture", target_family = "unix"))]
    dev: Option<crate::uinput::UInputDevice>,
    width: u32,
    height: u32,
}

impl LocalClicker {
    pub fn open(width: u32, height: u32) -> Self {
        #[cfg(all(feature = "capture", target_family = "unix"))]
        {
            match crate::uinput::UInputDevice::open(
                "/dev/uinput",
                "pchome-yerel-tiklayici",
                width,
                height,
            ) {
                Ok(dev) => {
                    log::info!("yerel tıklayıcı hazır (/dev/uinput)");
                    return Self { dev: Some(dev), width, height };
                }
                Err(e) => log::warn!("yerel tıklayıcı açılamadı: {}", e),
            }
            Self { dev: None, width, height }
        }
        #[cfg(not(all(feature = "capture", target_family = "unix")))]
        {
            Self { width, height }
        }
    }

    pub fn available(&self) -> bool {
        #[cfg(all(feature = "capture", target_family = "unix"))]
        {
            self.dev.is_some()
        }
        #[cfg(not(all(feature = "capture", target_family = "unix")))]
        {
            false
        }
    }

    /// Ekran pikselini cihaz aralığına kırp.
    fn clamp_point(&self, x: i32, y: i32) -> (i32, i32) {
        (
            x.clamp(0, self.width.max(1) as i32 - 1),
            y.clamp(0, self.height.max(1) as i32 - 1),
        )
    }

    pub fn click_at(&self, x: i32, y: i32) -> Result<()> {
        #[cfg(all(feature = "capture", target_family = "unix"))]
        {
            let dev = self.dev.as_ref().ok_or_else(|| anyhow::anyhow!("yerel tıklayıcı yok"))?;
            let (x, y) = self.clamp_point(x, y);
            dev.move_absolute(x, y)?;
            dev.click(crate::uinput::button_code("left"))?;
            return Ok(());
        }
        #[cfg(not(all(feature = "capture", target_family = "unix")))]
        {
            let _ = (x, y);
            anyhow::bail!("bu platformda yerel tıklama yok")
        }
    }

    pub fn right_click_at(&self, x: i32, y: i32) -> Result<()> {
        #[cfg(all(feature = "capture", target_family = "unix"))]
        {
            let dev = self.dev.as_ref().ok_or_else(|| anyhow::anyhow!("yerel tıklayıcı yok"))?;
            let (x, y) = self.clamp_point(x, y);
            dev.move_absolute(x, y)?;
            dev.click(crate::uinput::button_code("right"))?;
            return Ok(());
        }
        #[cfg(not(all(feature = "capture", target_family = "unix")))]
        {
            let _ = (x, y);
            anyhow::bail!("bu platformda yerel tıklama yok")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nokta_ekran_disina_tasmaz() {
        let c = LocalClicker::open(1920, 1080);
        assert_eq!(c.clamp_point(-5, 99999), (0, 1079));
        assert_eq!(c.clamp_point(100, 200), (100, 200));
    }
}
