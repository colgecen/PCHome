use anyhow::Result;

#[cfg(target_family = "unix")]
use std::os::unix::fs::OpenOptionsExt;
#[cfg(target_family = "unix")]
use std::os::unix::io::AsRawFd;

#[cfg(target_family = "unix")]
const UI_SET_EVBIT: libc::c_ulong = 0x40045564;
#[cfg(target_family = "unix")]
const UI_SET_KEYBIT: libc::c_ulong = 0x40045565;
#[cfg(target_family = "unix")]
const UI_SET_RELBIT: libc::c_ulong = 0x40045566;
#[cfg(target_family = "unix")]
const UI_SET_ABSBIT: libc::c_ulong = 0x40045567;
#[cfg(target_family = "unix")]
const UI_SET_PROPBIT: libc::c_ulong = 0x4004556e;
// _IOW('U', 3, struct uinput_setup)
#[cfg(target_family = "unix")]
const UI_DEV_SETUP: libc::c_ulong = 0x405c_5503;
// _IOW('U', 4, struct uinput_abs_setup)
#[cfg(target_family = "unix")]
const UI_ABS_SETUP: libc::c_ulong = 0x401c_5504;
// _IO('U', 1)
#[cfg(target_family = "unix")]
const UI_DEV_CREATE: libc::c_ulong = 0x5501;
// _IO('U', 2)
#[cfg(target_family = "unix")]
const UI_DEV_DESTROY: libc::c_ulong = 0x5502;

#[cfg(target_family = "unix")]
const UINPUT_MAX_NAME_SIZE: usize = 80;

#[cfg(target_family = "unix")]
const INPUT_PROP_POINTER: libc::c_ulong = 0x00;
#[cfg(target_family = "unix")]
const EV_SYN: u16 = 0x00;
#[cfg(target_family = "unix")]
const EV_KEY: u16 = 0x01;
#[cfg(target_family = "unix")]
const EV_REL: u16 = 0x02;
#[cfg(target_family = "unix")]
const EV_ABS: u16 = 0x03;
#[cfg(target_family = "unix")]
const SYN_REPORT: u16 = 0x00;

#[cfg(target_family = "unix")]
const BTN_LEFT: u16 = 0x110;
#[cfg(target_family = "unix")]
const BTN_RIGHT: u16 = 0x111;
#[cfg(target_family = "unix")]
const BTN_MIDDLE: u16 = 0x112;

#[cfg(target_family = "unix")]
const REL_X: u16 = 0x00;
#[cfg(target_family = "unix")]
const REL_Y: u16 = 0x01;
#[cfg(target_family = "unix")]
const REL_WHEEL: u16 = 0x08;
#[cfg(target_family = "unix")]
const REL_HWHEEL: u16 = 0x09;
#[cfg(target_family = "unix")]
const REL_WHEEL_HI_RES: u16 = 0x0b;
#[cfg(target_family = "unix")]
const REL_HWHEEL_HI_RES: u16 = 0x0c;

#[cfg(target_family = "unix")]
const ABS_X: u16 = 0x00;
#[cfg(target_family = "unix")]
const ABS_Y: u16 = 0x01;

#[cfg(target_family = "unix")]
const BUS_VIRTUAL: u16 = 0x06;

// Largest key code advertised so the kernel accepts any Linux keycode forwarded
// from the mobile client (KEY_* range plus BTN_* mouse buttons).
#[cfg(target_family = "unix")]
const KEY_CODE_MAX: u32 = 767;

#[cfg(target_family = "unix")]
#[repr(C)]
struct UinputId {
    bustype: u16,
    vendor: u16,
    product: u16,
    version: u16,
}

/// Mirrors `struct uinput_setup` (linux/uinput.h) used by UI_DEV_SETUP.
#[cfg(target_family = "unix")]
#[repr(C)]
struct UinputSetup {
    id: UinputId,
    name: [u8; UINPUT_MAX_NAME_SIZE],
    ff_effects_max: u32,
}

/// Mirrors `struct input_absinfo` (linux/input.h).
#[cfg(target_family = "unix")]
#[repr(C)]
struct InputAbsInfo {
    value: i32,
    minimum: i32,
    maximum: i32,
    fuzz: i32,
    flat: i32,
    resolution: i32,
}

/// Mirrors `struct uinput_abs_setup` (linux/uinput.h) used by UI_ABS_SETUP.
#[cfg(target_family = "unix")]
#[repr(C)]
struct UinputAbsSetup {
    code: u32,
    absinfo: InputAbsInfo,
}

/// A uinput device can expose only one pointer model. Mixing EV_REL and EV_ABS
/// on a single device makes libinput treat it as a relative mouse and silently
/// ignore the absolute coordinates, so direct touch and trackpad input each
/// need their own device. The keyboard is kept separate as well; this keeps
/// every device's capability set unambiguous.
#[cfg(target_family = "unix")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceKind {
    AbsolutePointer,
    RelativePointer,
    Keyboard,
}

#[cfg(target_family = "unix")]
pub struct UInputDevice {
    fd: std::fs::File,
    pub name: String,
    width: i32,
    height: i32,
    kind: DeviceKind,
    /// High-resolution wheel artıkları: 120 birim = 1 klasik tık.
    wheel_hi_x: i32,
    wheel_hi_y: i32,
}

#[cfg(target_family = "unix")]
impl UInputDevice {
    /// Absolute pointer (direct touch) with left/right/middle buttons.
    pub fn open(path: &str, name: &str, width: u32, height: u32) -> Result<Self> {
        Self::open_kind(path, name, width, height, DeviceKind::AbsolutePointer)
    }

    /// Absolute pointer; alias that reads better at call sites.
    pub fn open_absolute(path: &str, name: &str, width: u32, height: u32) -> Result<Self> {
        Self::open_kind(path, name, width, height, DeviceKind::AbsolutePointer)
    }

    /// Relative pointer for trackpad mode and wheel scrolling.
    pub fn open_relative(path: &str, name: &str) -> Result<Self> {
        Self::open_kind(path, name, 0, 0, DeviceKind::RelativePointer)
    }

    /// Keyboard-only device for remotely forwarded keys.
    pub fn open_keyboard(path: &str, name: &str) -> Result<Self> {
        Self::open_kind(path, name, 0, 0, DeviceKind::Keyboard)
    }

    pub fn open_kind(
        path: &str,
        name: &str,
        width: u32,
        height: u32,
        kind: DeviceKind,
    ) -> Result<Self> {
        let fd = std::fs::OpenOptions::new()
            .write(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(path)?;
        let raw_fd = fd.as_raw_fd();

        ioctl(raw_fd, UI_SET_EVBIT, EV_SYN as libc::c_ulong);
        ioctl(raw_fd, UI_SET_EVBIT, EV_KEY as libc::c_ulong);

        match kind {
            DeviceKind::AbsolutePointer => {
                // Deliberately no EV_REL: a device that advertises both is
                // classified by libinput as a relative mouse and ABS events
                // are ignored, which broke direct touch.
                ioctl(raw_fd, UI_SET_EVBIT, EV_ABS as libc::c_ulong);
                ioctl(raw_fd, UI_SET_ABSBIT, ABS_X as libc::c_ulong);
                ioctl(raw_fd, UI_SET_ABSBIT, ABS_Y as libc::c_ulong);
                ioctl(raw_fd, UI_SET_KEYBIT, BTN_LEFT as libc::c_ulong);
                ioctl(raw_fd, UI_SET_KEYBIT, BTN_RIGHT as libc::c_ulong);
                ioctl(raw_fd, UI_SET_KEYBIT, BTN_MIDDLE as libc::c_ulong);

                let abs_x = UinputAbsSetup {
                    code: ABS_X as u32,
                    absinfo: InputAbsInfo {
                        value: 0,
                        minimum: 0,
                        maximum: 65535,
                        fuzz: 0,
                        flat: 0,
                        resolution: 1,
                    },
                };
                let abs_y = UinputAbsSetup {
                    code: ABS_Y as u32,
                    absinfo: InputAbsInfo {
                        value: 0,
                        minimum: 0,
                        maximum: 65535,
                        fuzz: 0,
                        flat: 0,
                        resolution: 1,
                    },
                };
                unsafe {
                    if libc::ioctl(
                        raw_fd,
                        UI_ABS_SETUP as libc::c_ulong,
                        &abs_x as *const UinputAbsSetup as *const libc::c_void,
                    ) < 0
                    {
                        anyhow::bail!(
                            "uinput UI_ABS_SETUP(ABS_X) failed: errno {}",
                            *libc::__errno_location()
                        );
                    }
                    if libc::ioctl(
                        raw_fd,
                        UI_ABS_SETUP as libc::c_ulong,
                        &abs_y as *const UinputAbsSetup as *const libc::c_void,
                    ) < 0
                    {
                        anyhow::bail!(
                            "uinput UI_ABS_SETUP(ABS_Y) failed: errno {}",
                            *libc::__errno_location()
                        );
                    }
                }
            }
            DeviceKind::RelativePointer => {
                ioctl(raw_fd, UI_SET_EVBIT, EV_REL as libc::c_ulong);
                ioctl(raw_fd, UI_SET_PROPBIT, INPUT_PROP_POINTER);
                ioctl(raw_fd, UI_SET_RELBIT, REL_X as libc::c_ulong);
                ioctl(raw_fd, UI_SET_RELBIT, REL_Y as libc::c_ulong);
                ioctl(raw_fd, UI_SET_RELBIT, REL_WHEEL as libc::c_ulong);
                ioctl(raw_fd, UI_SET_RELBIT, REL_HWHEEL as libc::c_ulong);
                // Yüksek çözünürlüklü tekerlek: modern uygulamalarda akıcı
                // kaydırma; desteklemeyenler için REL_WHEEL'e de düşeriz.
                ioctl(raw_fd, UI_SET_RELBIT, REL_WHEEL_HI_RES as libc::c_ulong);
                ioctl(raw_fd, UI_SET_RELBIT, REL_HWHEEL_HI_RES as libc::c_ulong);
                ioctl(raw_fd, UI_SET_KEYBIT, BTN_LEFT as libc::c_ulong);
                ioctl(raw_fd, UI_SET_KEYBIT, BTN_RIGHT as libc::c_ulong);
                ioctl(raw_fd, UI_SET_KEYBIT, BTN_MIDDLE as libc::c_ulong);
            }
            DeviceKind::Keyboard => {
                // Keyboard keys (1..255) plus the extended KEY_* range. Skip
                // the BTN_* tablet range (0x100..0x15f): advertising
                // BTN_TOOL_PEN etc. makes libinput classify the whole device
                // as a tablet and ignore it.
                for code in 0..=0xFF {
                    ioctl(raw_fd, UI_SET_KEYBIT, code as libc::c_ulong);
                }
                for code in 0x160..=KEY_CODE_MAX {
                    ioctl(raw_fd, UI_SET_KEYBIT, code as libc::c_ulong);
                }
            }
        }

        unsafe {
            let mut setup: UinputSetup = std::mem::zeroed();
            setup.id.bustype = BUS_VIRTUAL;
            setup.id.vendor = 0x1234;
            setup.id.product = 0x5678;
            setup.id.version = 1;
            let name_bytes = name.as_bytes();
            let len = name_bytes.len().min(UINPUT_MAX_NAME_SIZE - 1);
            setup.name[..len].copy_from_slice(&name_bytes[..len]);

            if libc::ioctl(
                raw_fd,
                UI_DEV_SETUP as libc::c_ulong,
                &setup as *const UinputSetup as *const libc::c_void,
            ) < 0
            {
                anyhow::bail!(
                    "uinput UI_DEV_SETUP failed: errno {}",
                    *libc::__errno_location()
                );
            }
            if libc::ioctl(raw_fd, UI_DEV_CREATE, 0) < 0 {
                anyhow::bail!(
                    "uinput UI_DEV_CREATE failed: errno {}",
                    *libc::__errno_location()
                );
            }
        }

        log::info!(
            "uinput device created: {} ({}, {:?}) {}x{}",
            path,
            name,
            kind,
            width,
            height
        );
        Ok(Self {
            fd,
            name: name.to_string(),
            width: width as i32,
            height: height as i32,
            kind,
            wheel_hi_x: 0,
            wheel_hi_y: 0,
        })
    }

    /// Direct-touch teleport to an absolute (PC-pixel) coordinate.
    pub fn move_absolute(&self, x: i32, y: i32) -> Result<()> {
        if self.kind != DeviceKind::AbsolutePointer {
            anyhow::bail!("move_absolute yalnız mutlak işaretçide kullanılabilir");
        }
        let x = x.clamp(0, self.width.saturating_sub(1).max(1));
        let y = y.clamp(0, self.height.saturating_sub(1).max(1));
        // Scale into the fixed 0..65535 range advertised by UI_ABS_SETUP.
        let ax = ((x as i64 * 65535) / self.width.max(1) as i64) as i32;
        let ay = ((y as i64 * 65535) / self.height.max(1) as i64) as i32;
        self.emit(InputEvent::new(EV_ABS, ABS_X, ax))?;
        self.emit(InputEvent::new(EV_ABS, ABS_Y, ay))?;
        self.emit(InputEvent::new(EV_SYN, SYN_REPORT, 0))?;
        Ok(())
    }

    /// Trackpad style relative delta.
    pub fn move_relative(&self, dx: i32, dy: i32) -> Result<()> {
        if self.kind != DeviceKind::RelativePointer {
            anyhow::bail!("move_relative yalnız göreli işaretçide kullanılabilir");
        }
        self.emit(InputEvent::new(EV_REL, REL_X, dx))?;
        self.emit(InputEvent::new(EV_REL, REL_Y, dy))?;
        self.emit(InputEvent::new(EV_SYN, SYN_REPORT, 0))?;
        Ok(())
    }

    pub fn button(&self, code: u16, down: bool) -> Result<()> {
        self.emit(InputEvent::new(EV_KEY, code, down as i32))?;
        self.emit(InputEvent::new(EV_SYN, SYN_REPORT, 0))?;
        Ok(())
    }

    pub fn click(&self, code: u16) -> Result<()> {
        self.button(code, true)?;
        self.button(code, false)?;
        Ok(())
    }

    pub fn double_click(&self, code: u16) -> Result<()> {
        self.button(code, true)?;
        self.button(code, false)?;
        self.button(code, true)?;
        self.button(code, false)?;
        Ok(())
    }

    pub fn wheel(&self, dx: i32, dy: i32) -> Result<()> {
        if self.kind != DeviceKind::RelativePointer {
            anyhow::bail!("wheel yalnız göreli işaretçide kullanılabilir");
        }
        if dy != 0 {
            self.emit(InputEvent::new(EV_REL, REL_WHEEL, dy))?;
        }
        if dx != 0 {
            self.emit(InputEvent::new(EV_REL, REL_HWHEEL, dx))?;
        }
        self.emit(InputEvent::new(EV_SYN, SYN_REPORT, 0))?;
        Ok(())
    }

    /// Akıcı kaydırma: 120 birim = 1 klasik tekerlek tıkı. High-res
    /// destekleyen uygulamalar küçük birimleri yumuşak işler; desteklemeyenler
    /// için 120 birim dolunca normal REL_WHEEL de gönderilir.
    pub fn wheel_hi_res(&mut self, dx: i32, dy: i32) -> Result<()> {
        if self.kind != DeviceKind::RelativePointer {
            anyhow::bail!("wheel_hi_res yalnız göreli işaretçide kullanılabilir");
        }
        if dy != 0 {
            self.emit(InputEvent::new(EV_REL, REL_WHEEL_HI_RES, dy))?;
            self.wheel_hi_y += dy;
            let notches = self.wheel_hi_y / 120;
            if notches != 0 {
                self.wheel_hi_y -= notches * 120;
                self.emit(InputEvent::new(EV_REL, REL_WHEEL, notches))?;
            }
        }
        if dx != 0 {
            self.emit(InputEvent::new(EV_REL, REL_HWHEEL_HI_RES, dx))?;
            self.wheel_hi_x += dx;
            let notches = self.wheel_hi_x / 120;
            if notches != 0 {
                self.wheel_hi_x -= notches * 120;
                self.emit(InputEvent::new(EV_REL, REL_HWHEEL, notches))?;
            }
        }
        self.emit(InputEvent::new(EV_SYN, SYN_REPORT, 0))?;
        Ok(())
    }

    pub fn key(&self, code: u32, down: bool) -> Result<()> {
        if self.kind != DeviceKind::Keyboard {
            anyhow::bail!("key yalnız klavye cihazında kullanılabilir");
        }
        if code > KEY_CODE_MAX {
            return Ok(());
        }
        self.emit(InputEvent::new(EV_KEY, code as u16, down as i32))?;
        self.emit(InputEvent::new(EV_SYN, SYN_REPORT, 0))?;
        Ok(())
    }

    pub fn emit(&self, event: InputEvent) -> Result<()> {
        let ev = event.to_libc();
        let fd = self.fd.as_raw_fd();
        let ptr = &ev as *const libc::input_event as *const libc::c_void;
        let len = std::mem::size_of::<libc::input_event>();
        loop {
            let ret = unsafe { libc::write(fd, ptr, len) };
            if ret < 0 {
                let errno = unsafe { *libc::__errno_location() };
                if errno == libc::EINTR {
                    continue;
                }
                anyhow::bail!("Failed to write input event: errno {}", errno);
            }
            break;
        }
        Ok(())
    }
}

#[cfg(target_family = "unix")]
fn ioctl(fd: i32, request: libc::c_ulong, value: libc::c_ulong) {
    unsafe {
        libc::ioctl(fd, request, value);
    }
}

#[cfg(target_family = "unix")]
impl Drop for UInputDevice {
    fn drop(&mut self) {
        unsafe {
            libc::ioctl(self.fd.as_raw_fd(), UI_DEV_DESTROY, 0);
        }
    }
}

#[cfg(target_family = "unix")]
#[derive(Debug, Clone, Copy)]
pub struct InputEvent {
    pub kind: u16,
    pub code: u16,
    pub value: i32,
}

#[cfg(target_family = "unix")]
impl InputEvent {
    pub fn new(kind: u16, code: u16, value: i32) -> Self {
        Self { kind, code, value }
    }

    fn to_libc(self) -> libc::input_event {
        libc::input_event {
            time: libc::timeval {
                tv_sec: 0,
                tv_usec: 0,
            },
            type_: self.kind,
            code: self.code,
            value: self.value,
        }
    }
}

#[cfg(target_family = "unix")]
pub fn emit_key(key: u16, pressed: bool) -> Result<()> {
    let fd = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/uinput")?;
    let ev = InputEvent::new(EV_KEY, key, pressed as i32);
    unsafe {
        let ev_libc = ev.to_libc();
        libc::write(
            fd.as_raw_fd(),
            &ev_libc as *const libc::input_event as *const libc::c_void,
            std::mem::size_of::<libc::input_event>(),
        );
    }
    Ok(())
}

#[cfg(target_family = "unix")]
pub fn button_code(name: &str) -> u16 {
    match name {
        "right" => BTN_RIGHT,
        "middle" => BTN_MIDDLE,
        _ => BTN_LEFT,
    }
}

#[cfg(not(target_family = "unix"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceKind {
    AbsolutePointer,
    RelativePointer,
    Keyboard,
}

#[cfg(not(target_family = "unix"))]
pub struct UInputDevice;

#[cfg(not(target_family = "unix"))]
impl UInputDevice {
    pub fn open(_path: &str, _name: &str, _width: u32, _height: u32) -> Result<Self> {
        Ok(Self)
    }
    pub fn open_absolute(_path: &str, _name: &str, _width: u32, _height: u32) -> Result<Self> {
        Ok(Self)
    }
    pub fn open_relative(_path: &str, _name: &str) -> Result<Self> {
        Ok(Self)
    }
    pub fn open_keyboard(_path: &str, _name: &str) -> Result<Self> {
        Ok(Self)
    }
    pub fn open_kind(
        _path: &str,
        _name: &str,
        _width: u32,
        _height: u32,
        _kind: DeviceKind,
    ) -> Result<Self> {
        Ok(Self)
    }
    pub fn move_absolute(&self, _x: i32, _y: i32) -> Result<()> {
        Ok(())
    }
    pub fn move_relative(&self, _dx: i32, _dy: i32) -> Result<()> {
        Ok(())
    }
    pub fn button(&self, _code: u16, _down: bool) -> Result<()> {
        Ok(())
    }
    pub fn click(&self, _code: u16) -> Result<()> {
        Ok(())
    }
    pub fn double_click(&self, _code: u16) -> Result<()> {
        Ok(())
    }
    pub fn wheel(&self, _dx: i32, _dy: i32) -> Result<()> {
        Ok(())
    }
    pub fn key(&self, _code: u32, _down: bool) -> Result<()> {
        Ok(())
    }
}

#[cfg(not(target_family = "unix"))]
#[derive(Debug, Clone, Copy)]
pub struct InputEvent;

#[cfg(not(target_family = "unix"))]
impl InputEvent {
    pub fn new(_kind: u16, _code: u16, _value: i32) -> Self {
        Self
    }
}

#[cfg(not(target_family = "unix"))]
pub fn emit_key(_key: u16, _pressed: bool) -> Result<()> {
    Ok(())
}

#[cfg(not(target_family = "unix"))]
pub fn button_code(_name: &str) -> u16 {
    0
}
