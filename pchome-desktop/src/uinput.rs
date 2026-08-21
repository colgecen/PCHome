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
const UI_DEV_SETUP: libc::c_ulong = 0x40105568;
#[cfg(target_family = "unix")]
const UI_DEV_CREATE: libc::c_ulong = 0x40045569;
#[cfg(target_family = "unix")]
const UI_DEV_DESTROY: libc::c_ulong = 0x4004556a;

#[cfg(target_family = "unix")]
const UINPUT_MAX_NAME_SIZE: usize = 80;

#[cfg(target_family = "unix")]
#[repr(C)]
struct UinputId {
    bustype: u16,
    vendor: u16,
    product: u16,
    version: u16,
}

#[cfg(target_family = "unix")]
#[repr(C)]
struct UinputSetup {
    id: UinputId,
    name: [u8; UINPUT_MAX_NAME_SIZE],
    ff_effects_max: u32,
    absmax: [i32; 64],
    absmin: [i32; 64],
    absfuzz: [i32; 64],
    absflat: [i32; 64],
}

#[cfg(target_family = "unix")]
pub struct UInputDevice {
    fd: std::fs::File,
    pub name: String,
}

#[cfg(target_family = "unix")]
impl UInputDevice {
    pub fn open(path: &str, name: &str) -> Result<Self> {
        let fd = std::fs::OpenOptions::new()
            .write(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(path)?;
        let raw_fd = fd.as_raw_fd();

        // Enable the event types this virtual device will emit.
        ioctl(raw_fd, UI_SET_EVBIT, libc::EV_SYN as libc::c_ulong);
        ioctl(raw_fd, UI_SET_EVBIT, libc::EV_KEY as libc::c_ulong);
        ioctl(raw_fd, UI_SET_EVBIT, libc::EV_REL as libc::c_ulong);

        // Advertise a broad key range plus mouse buttons and relative axes.
        for code in 0..=255u32 {
            ioctl(raw_fd, UI_SET_KEYBIT, code as libc::c_ulong);
        }
        ioctl(raw_fd, UI_SET_KEYBIT, libc::BTN_LEFT as libc::c_ulong);
        ioctl(raw_fd, UI_SET_KEYBIT, libc::BTN_RIGHT as libc::c_ulong);
        ioctl(raw_fd, UI_SET_KEYBIT, libc::BTN_MIDDLE as libc::c_ulong);
        ioctl(raw_fd, UI_SET_RELBIT, libc::REL_X as libc::c_ulong);
        ioctl(raw_fd, UI_SET_RELBIT, libc::REL_Y as libc::c_ulong);
        ioctl(raw_fd, UI_SET_RELBIT, libc::REL_WHEEL as libc::c_ulong);

        let mut setup: UinputSetup = unsafe { std::mem::zeroed() };
        setup.id.bustype = libc::BUS_VIRTUAL as u16;
        setup.id.vendor = 0x1234;
        setup.id.product = 0x5678;
        setup.id.version = 1;
        let name_bytes = name.as_bytes();
        let len = name_bytes.len().min(UINPUT_MAX_NAME_SIZE - 1);
        setup.name[..len].copy_from_slice(&name_bytes[..len]);

        unsafe {
            if libc::ioctl(
                raw_fd,
                UI_DEV_SETUP as libc::c_ulong,
                &setup as *const UinputSetup as *const libc::c_void,
            ) < 0
            {
                anyhow::bail!("uinput UI_DEV_SETUP failed: errno {}", *libc::__errno_location());
            }
            if libc::ioctl(raw_fd, UI_DEV_CREATE as libc::c_ulong, 0) < 0 {
                anyhow::bail!("uinput UI_DEV_CREATE failed: errno {}", *libc::__errno_location());
            }
        }

        log::info!("uinput device created: {} ({})", path, name);
        Ok(Self {
            fd,
            name: name.to_string(),
        })
    }

    pub fn emit(&self, event: InputEvent) -> Result<()> {
        let ev = event.to_libc();
        let fd = self.fd.as_raw_fd();
        let ptr = &ev as *const libc::input_event as *const libc::c_void;
        let len = std::mem::size_of::<libc::input_event>();
        // Retry the write on EINTR so a signal interrupt never silently drops
        // an input event.
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
            libc::ioctl(self.fd.as_raw_fd(), UI_DEV_DESTROY as libc::c_ulong, 0);
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
            time: libc::timeval { tv_sec: 0, tv_usec: 0 },
            type_: self.kind,
            code: self.code,
            value: self.value,
        }
    }
}

#[cfg(target_family = "unix")]
pub fn emit_key(key: u16, pressed: bool) -> Result<()> {
    let fd = std::fs::OpenOptions::new().write(true).open("/dev/uinput")?;
    let ev = InputEvent::new(libc::EV_KEY, key, pressed as i32);
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

#[cfg(not(target_family = "unix"))]
pub struct UInputDevice;

#[cfg(not(target_family = "unix"))]
impl UInputDevice {
    pub fn open(_path: &str, _name: &str) -> Result<Self> {
        Ok(Self)
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
