use anyhow::Result;
use nix::sys::ioctl::{ioctl_none, ioctl_write_ptr_buf};
use std::fs::OpenOptions;
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::io::AsRawFd;

nix::ioctl_write_ptr_buf!(
    write_ev,
    b'E',
    0x01,
    std::os::raw::c_ushort
);

nix::ioctl_none!(
    create_device,
    b'U',
    0x02
);

nix::ioctl_none!(
    destroy_device,
    b'U',
    0x03
);

const UINPUT_MAX_NAME_SIZE: usize = 80;

#[repr(C)]
pub struct UInputUserDev {
    pub id: libc::input_id,
    pub name: [u8; UINPUT_MAX_NAME_SIZE],
    pub ff_effects_max: u32,
    pub absmax: [libc::__s32; 64],
    pub absmin: [libc::__s32; 64],
    pub absfuzz: [libc::__s32; 64],
    pub absflat: [libc::__s32; 64],
}

pub struct UInputDevice {
    fd: std::fs::File,
    pub name: String,
}

impl UInputDevice {
    pub fn open(path: &str, name: &str) -> Result<Self> {
        let fd = OpenOptions::new()
            .write(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(path)
            .with_context(|| format!("Failed to open {}", path))?;

        let mut dev = UInputUserDev {
            id: unsafe { std::mem::zeroed() },
            name: [0u8; UINPUT_MAX_NAME_SIZE],
            ff_effects_max: 0,
            absmax: [0; 64],
            absmin: [0; 64],
            absfuzz: [0; 64],
            absflat: [0; 64],
        };

        let name_bytes = name.as_bytes();
        let len = std::cmp::min(name_bytes.len(), UINPUT_MAX_NAME_SIZE - 1);
        dev.name[..len].copy_from_slice(&name_bytes[..len]);

        unsafe {
            ioctl_write_ptr_buf!(write_ev, b'E', 0x01, u16);
            let ret = libc::ioctl(fd.as_raw_fd(), write_ev(), &dev as *const _ as *mut _);
            if ret < 0 {
                anyhow::bail!("ioctl EV_ABS failed");
            }
        }

        unsafe {
            create_device(fd.as_raw_fd())?;
        }

        Ok(Self {
            fd,
            name: name.to_string(),
        })
    }

    pub fn emit(&self, event: InputEvent) -> Result<()> {
        let ev = event.to_libc();
        unsafe {
            let ret = libc::write(
                self.fd.as_raw_fd(),
                &ev as *const libc::input_event as *const libc::c_void,
                std::mem::size_of::<libc::input_event>(),
            );
            if ret < 0 {
                anyhow::bail!("Failed to write input event");
            }
        }
        Ok(())
    }

    pub fn destroy(self) -> Result<()> {
        unsafe {
            destroy_device(self.fd.as_raw_fd())?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct InputEvent {
    pub time: libc::timeval,
    pub kind: u16,
    pub code: u16,
    pub value: i32,
}

impl InputEvent {
    pub fn new(kind: u16, code: u16, value: i32) -> Self {
        Self {
            time: libc::timeval {
                tv_sec: 0,
                tv_usec: 0,
            },
            kind,
            code,
            value,
        }
    }

    fn to_libc(self) -> libc::input_event {
        libc::input_event {
            time: self.time,
            type_: self.kind,
            code: self.code,
            value: self.value,
        }
    }
}

pub fn emit_key(key: u16, pressed: bool) -> Result<()> {
    let fd = OpenOptions::new().write(true).open("/dev/uinput")?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_event_creation() {
        let ev = InputEvent::new(0x01, 0x02, 1);
        assert_eq!(ev.kind, 0x01);
        assert_eq!(ev.code, 0x02);
        assert_eq!(ev.value, 1);
    }

    #[test]
    fn test_input_event_to_libc() {
        let ev = InputEvent::new(libc::EV_KEY, 30, 1);
        let libc_ev = ev.to_libc();
        assert_eq!(libc_ev.type_, libc::EV_KEY);
        assert_eq!(libc_ev.code, 30);
        assert_eq!(libc_ev.value, 1);
    }

    #[test]
    fn test_emit_key_signature() {
        let _ = emit_key(30, true);
    }
}
