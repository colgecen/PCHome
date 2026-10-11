use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::atomic::Ordering;
use std::time::Duration;

use crate::state::SharedState;

/// Önizleme aynası: RustDesk mantığıyla ekranın birebir görüntüsü.
/// Tek uzun-ömürlü ffmpeg prosesi (`x11grab` → küçük MJPEG akışı, ~5fps)
/// çalışır; JPEG kareler SOI/EOI işaretlerinden ayıklanıp
/// `state.preview_jpeg`'e yazılır. İmleç konumu ayrıca normalize edilip
/// `state.preview_cursor`'da tutulur; GUI dokuyu basarken üstüne ok çizer
/// (x11grab imleci yakalayamaz).
pub fn spawn_preview(state: SharedState, width: u32, height: u32) {
    std::thread::Builder::new()
        .name("pchome-preview".to_string())
        .spawn(move || loop {
            if state.terminate.load(Ordering::SeqCst) {
                break;
            }
            run_mirror(&state, width, height);
            state.preview_alive.store(false, Ordering::SeqCst);
            std::thread::sleep(Duration::from_secs(5));
        })
        .ok();
}

fn run_mirror(state: &SharedState, width: u32, height: u32) {
    let display = std::env::var("DISPLAY").unwrap_or_default();
    if display.is_empty() {
        log::debug!("önizleme atlandı: DISPLAY yok (Wayland/başsız oturum)");
        return;
    }
    let mut child = match Command::new("ffmpeg")
        .args([
            "-loglevel", "error",
            "-f", "x11grab",
            "-framerate", "5",
            "-video_size", &format!("{}x{}", width, height),
            "-i", &display,
            "-vf", "scale=544:-2",
            "-q:v", "6",
            "-f", "mjpeg",
            "-",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            log::warn!("önizleme ffmpeg'i başlatılamadı: {}", e);
            return;
        }
    };
    let mut out = match child.stdout.take() {
        Some(o) => o,
        None => return,
    };
    state.preview_alive.store(true, Ordering::SeqCst);
    log::info!("önizleme aynası başladı (MJPEG, ~5fps)");

    let cursor = CursorQuery::connect();
    let mut buf: Vec<u8> = Vec::with_capacity(128 * 1024);
    let mut tmp = [0u8; 16384];
    loop {
        if state.terminate.load(Ordering::SeqCst) {
            break;
        }
        let n = match out.read(&mut tmp) {
            Ok(0) => break, // ffmpeg öldü
            Ok(n) => n,
            Err(_) => break,
        };
        buf.extend_from_slice(&tmp[..n]);
        // Tamamlanmış JPEG kareleri çıkar (FFD8 ... FFD9).
        while let Some(frame) = take_jpeg(&mut buf) {
            let (fw, fh) = jpeg_size(&frame).unwrap_or((544, 306));
            *state.preview_jpeg.lock().unwrap() = Some((frame, fw, fh));
            if let Some((cx, cy)) = cursor.position(width, height) {
                *state.preview_cursor.lock().unwrap() = Some((cx, cy));
            }
        }
        if buf.len() > 4 * 1024 * 1024 {
            buf.clear(); // senkron kaybolduysa at, yeniden yakala
        }
    }
    let _ = child.kill();
}

/// Tamponun başındaki ilk bütün JPEG kareyi alıp tampondan düşer.
fn take_jpeg(buf: &mut Vec<u8>) -> Option<Vec<u8>> {
    let start = find_marker(buf, &[0xFF, 0xD8])?;
    let end = find_marker(&buf[start + 2..], &[0xFF, 0xD9])?;
    let end_abs = start + 2 + end + 2;
    let frame = buf[start..end_abs].to_vec();
    buf.drain(..end_abs);
    Some(frame)
}

fn find_marker(buf: &[u8], marker: &[u8]) -> Option<usize> {
    buf.windows(2).position(|w| w == marker)
}

/// JPEG SOF işaretinden kare boyutunu okur (bulunamazsa None).
fn jpeg_size(frame: &[u8]) -> Option<(u32, u32)> {
    let mut i = 2;
    while i + 9 < frame.len() {
        if frame[i] != 0xFF {
            i += 1;
            continue;
        }
        let m = frame[i + 1];
        // SOF0..SOF3 (C4/D8 hariç): yükseklik+genişlik taşır.
        if (0xC0..=0xCF).contains(&m) && m != 0xC4 && m != 0xC8 && m != 0xCC {
            let h = u16::from_be_bytes([frame[i + 5], frame[i + 6]]) as u32;
            let w = u16::from_be_bytes([frame[i + 7], frame[i + 8]]) as u32;
            if w > 0 && h > 0 {
                return Some((w, h));
            }
            return None;
        }
        if m == 0xD8 || m == 0xD9 || (0xD0..=0xD7).contains(&m) || m == 0x01 {
            i += 2;
            continue;
        }
        let len = u16::from_be_bytes([frame[i + 2], frame[i + 3]]) as usize;
        if len < 2 {
            return None;
        }
        i += 2 + len;
    }
    None
}

/// X11 imleç sorgusu (kalıcı bağlantı, kare başına bir soru).
struct CursorQuery {
    #[cfg(feature = "gui")]
    conn: Option<(x11rb::rust_connection::RustConnection, u32)>,
}

impl CursorQuery {
    fn connect() -> Self {
        #[cfg(feature = "gui")]
        {
            use x11rb::connection::Connection as _;
            match x11rb::connect(None) {
                Ok((conn, screen)) => {
                    let root = conn.setup().roots[screen].root;
                    return Self { conn: Some((conn, root)) };
                }
                Err(e) => log::debug!("X11 imleç sorgusu yok: {}", e),
            }
            Self { conn: None }
        }
        #[cfg(not(feature = "gui"))]
        {
            Self {}
        }
    }

    /// İmleç konumunu 0..1 normalize döndürür.
    fn position(&self, width: u32, height: u32) -> Option<(f32, f32)> {
        #[cfg(feature = "gui")]
        {
            let (conn, root) = self.conn.as_ref()?;
            let r = x11rb::protocol::xproto::query_pointer(conn, *root)
                .ok()?
                .reply()
                .ok()?;
            if width == 0 || height == 0 {
                return None;
            }
            return Some((
                (r.root_x as f32 / width as f32).clamp(0.0, 1.0),
                (r.root_y as f32 / height as f32).clamp(0.0, 1.0),
            ));
        }
        #[cfg(not(feature = "gui"))]
        {
            let _ = (width, height);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jpeg_ayiklama_soi_eoi() {
        let mut buf = vec![0x00, 0x11, 0xFF, 0xD8, 0x01, 0x02, 0xFF, 0xD9, 0x33];
        let frame = take_jpeg(&mut buf).unwrap();
        assert_eq!(frame, vec![0xFF, 0xD8, 0x01, 0x02, 0xFF, 0xD9]);
        assert_eq!(buf, vec![0x33]);
        assert!(take_jpeg(&mut buf).is_none());
    }

    #[test]
    fn jpeg_boyutu_sof0() {
        // Minimal SOF0: FFD8 FFC0 len=11 prec=8 h=0x012C(300) w=0x0220(544)
        let frame = vec![
            0xFF, 0xD8, 0xFF, 0xC0, 0x00, 0x0B, 0x08, 0x01, 0x2C, 0x02, 0x20, 0xFF, 0xD9,
        ];
        assert_eq!(jpeg_size(&frame), Some((544, 300)));
    }
}
