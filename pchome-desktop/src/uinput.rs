use anyhow::Result;

#[cfg(target_family = "unix")]
use std::os::unix::fs::OpenOptionsExt;
#[cfg(target_family = "unix")]
use std::os::unix::io::AsRawFd;

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

        log::info!("uinput device opened: {} ({})", path, name);
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
