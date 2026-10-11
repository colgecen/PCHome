use std::sync::atomic::Ordering;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::state::SharedState;

/// GitHub Releases tabanlı OTA güncelleyici (masaüstü).
/// Yeni bağımlılık YOK: indirme + API sorgusu sistem `curl` ile yapılır
/// (Linux masaüstlerinde varsayılan olarak bulunur).
///
/// Akış: açılıştan 20 sn sonra + her 6 saatte bir + Ayarlar'daki düğmeyle
/// `releases/latest` denetlenir. Yeni sürüm varsa üst bantta
/// "vX.Y.Z mevcut" görünür; "Şimdi Güncelle" indirim kurulum yapar:
/// - AppImage (`$APPIMAGE` tanımlı): yeni AppImage mevcut dosyanın yanına
///   indirilir, çalıştırılabilir yapılır.
/// - Diğer (tar.gz/dev ikilisi): tar.gz içindeki `pchome-desktop` mevcut
///   çalıştırılabilir dosyanın üzerine yazılır (önce `.bak` yedeklenir).
pub const REPO_OWNER: &str = "colgecen";
pub const REPO_NAME: &str = "PCHome";
pub const CHECK_INTERVAL_SECS: u64 = 6 * 3600;

#[derive(Debug, Clone, Default)]
pub struct UpdateInfo {
    pub version: String,
    pub notes: String,
    pub appimage_url: String,
    pub tarball_url: String,
}

#[derive(Debug, Deserialize)]
struct ReleaseAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, Deserialize)]
struct ReleaseJson {
    tag_name: String,
    body: Option<String>,
    assets: Vec<ReleaseAsset>,
}

pub fn current_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// `true`: uzak etiket güncel sürümden yenidir ("v" öneki yoksayılır).
pub fn is_newer(remote_tag: &str, current: &str) -> bool {
    compare_versions(normalize(remote_tag), normalize(current)) == std::cmp::Ordering::Greater
}

fn normalize(v: &str) -> Vec<u64> {
    v.trim()
        .trim_start_matches('v')
        .split(|c| c == '.' || c == '-' || c == '+')
        .filter_map(|p| p.chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse::<u64>().ok())
        .collect()
}

fn compare_versions(a: Vec<u64>, b: Vec<u64>) -> std::cmp::Ordering {
    let n = a.len().max(b.len());
    for i in 0..n {
        let x = *a.get(i).unwrap_or(&0);
        let y = *b.get(i).unwrap_or(&0);
        match x.cmp(&y) {
            std::cmp::Ordering::Equal => continue,
            other => return other,
        }
    }
    std::cmp::Ordering::Equal
}

fn latest_api_url() -> String {
    format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        REPO_OWNER, REPO_NAME
    )
}

fn curl_get(url: &str) -> Result<String> {
    let out = std::process::Command::new("curl")
        .args(["-fsSL", "-m", "30", "-A", "PCHome-desktop-updater", url])
        .output()
        .context("curl çalıştırılamadı (curl kurulu mu?)")?;
    if !out.status.success() {
        anyhow::bail!("indirme başarısız: {}", url);
    }
    String::from_utf8(out.stdout).context("yanıt UTF-8 değil")
}

/// `releases/latest` yanıtını ayrıştırıp indirme bağlantılarını çıkarır.
pub fn parse_release(json_text: &str) -> Result<UpdateInfo> {
    let rel: ReleaseJson = serde_json::from_str(json_text).context("sürüm yanıtı çözülemedi")?;
    let mut info = UpdateInfo {
        version: rel.tag_name.clone(),
        notes: rel.body.unwrap_or_default(),
        ..Default::default()
    };
    for asset in &rel.assets {
        let name = asset.name.to_lowercase();
        if info.appimage_url.is_empty() && name.ends_with(".appimage") {
            info.appimage_url = asset.browser_download_url.clone();
        }
        if info.tarball_url.is_empty()
            && name.ends_with(".tar.gz")
            && name.contains("pchome-desktop")
        {
            info.tarball_url = asset.browser_download_url.clone();
        }
    }
    Ok(info)
}

pub fn check_for_update() -> Result<Option<UpdateInfo>> {
    let body = curl_get(&latest_api_url())?;
    let info = parse_release(&body)?;
    if is_newer(&info.version, &current_version()) {
        Ok(Some(info))
    } else {
        Ok(None)
    }
}

fn set_status(state: &SharedState, line: String) {
    *state.update_status.lock().unwrap() = line;
}

/// Tek seferlik denetleme; sonucu `state.update_status` satırına yazar.
/// Ağ yoksa sessizce "denetlenemedi" yazar, daemon etkilenmez.
pub fn check_once(state: &SharedState) {
    set_status(state, "güncelleme denetleniyor…".to_string());
    match check_for_update() {
        Ok(Some(info)) => {
            set_status(
                state,
                format!("yeni sürüm mevcut: {} — Ayarlar'dan güncelleyin", info.version),
            );
            state.push_event(format!("güncelleme bulundu: {}", info.version));
        }
        Ok(None) => set_status(state, format!("güncel (v{})", current_version())),
        Err(e) => set_status(state, format!("denetlenemedi: {}", e)),
    }
}

/// Yeni sürümü indirip kurar. AppImage ise dosyanın yanına, normal ikili
/// ise üzerine yazar. Başarıda YENİ çalıştırılabilir dosyanın yolunu döndürür
/// (arayüz buradan yeniden başlatır).
pub fn install_update(state: &SharedState) -> Result<std::path::PathBuf> {
    let body = curl_get(&latest_api_url())?;
    let info = parse_release(&body)?;
    if !is_newer(&info.version, &current_version()) {
        set_status(state, format!("zaten güncel (v{})", current_version()));
        return Ok(std::env::current_exe().unwrap_or_default());
    }
    let new_path = if std::env::var_os("APPIMAGE").is_some() {
        if info.appimage_url.is_empty() {
            anyhow::bail!("sürümde AppImage dosyası yok");
        }
        install_appimage_next_to_current(state, &info)?
    } else {
        if info.tarball_url.is_empty() {
            anyhow::bail!("sürümde masaüstü paketi (tar.gz) yok");
        }
        install_tarball_over_current(state, &info)?
    };
    set_status(
        state,
        format!("{} kuruldu — yeniden başlatılıyor", info.version),
    );
    state.push_event(format!("güncelleme kuruldu: {}", info.version));
    Ok(new_path)
}

/// Yeni çalıştırılabilir dosyayı ayrı süreçte başlatıp bu süreci kapatır.
/// Kurulum sonrası arayüz "yeni haliyle yeniden başlar".
pub fn relaunch(new_exe: &std::path::Path) -> ! {
    log::info!("yeniden başlatılıyor: {}", new_exe.display());
    let _ = std::process::Command::new(new_exe)
        .args(std::env::args_os().skip(1))
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
    std::process::exit(0);
}

/// Uzak dosyanın bayt boyutu (HEAD isteği; olmazsa None).
/// Arayüz "şu kadar indirme" bilgisini buradan gösterir.
pub fn asset_size(url: &str) -> Option<u64> {
    let out = std::process::Command::new("curl")
        .args(["-fsSIL", "-m", "20", "-A", "PCHome-desktop-updater", url])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let head = String::from_utf8_lossy(&out.stdout);
    head.lines().rev().find_map(|l| {
        let low = l.to_ascii_lowercase();
        low.strip_prefix("content-length:")
            .and_then(|v| v.trim().parse::<u64>().ok())
    })
}

/// İnsan dostu boyut ("11,5 MB").
pub fn human_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    let b = bytes as f64;
    if b >= MB {
        format!("{:.1} MB", b / MB).replace('.', ",")
    } else if b >= KB {
        format!("{:.0} KB", b / KB)
    } else {
        format!("{} B", bytes)
    }
}

fn download_to(url: &str, dest: &std::path::Path) -> Result<()> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let status = std::process::Command::new("curl")
        .args([
            "-fSL",
            "-m",
            "600",
            "-A",
            "pchome-desktop-updater",
            "-o",
            &dest.to_string_lossy(),
            url,
        ])
        .status()
        .context("curl çalıştırılamadı")?;
    if !status.success() {
        anyhow::bail!("paket indirilemedi");
    }
    Ok(())
}

fn install_appimage_next_to_current(state: &SharedState, info: &UpdateInfo) -> Result<std::path::PathBuf> {
    let current = std::env::var("APPIMAGE").context("APPIMAGE yolu yok")?;
    let dir = std::path::Path::new(&current)
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let dest = dir.join(format!("PCHome-{}-x86_64.AppImage", info.version));
    set_status(state, "yeni AppImage indiriliyor…".to_string());
    download_to(&info.appimage_url, &dest)?;
    #[cfg(target_family = "unix")]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&dest)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&dest, perms)?;
    }
    Ok(dest)
}

fn install_tarball_over_current(state: &SharedState, info: &UpdateInfo) -> Result<std::path::PathBuf> {
    let tmp = std::env::temp_dir().join(format!("pchome-update-{}", std::process::id()));
    std::fs::create_dir_all(&tmp)?;
    let tgz = tmp.join("pkg.tar.gz");
    set_status(state, "güncelleme paketi indiriliyor…".to_string());
    download_to(&info.tarball_url, &tgz)?;
    let status = std::process::Command::new("tar")
        .args(["-xzf", &tgz.to_string_lossy(), "-C", &tmp.to_string_lossy()])
        .status()
        .context("tar açılamadı")?;
    if !status.success() {
        anyhow::bail!("paket açılamadı");
    }
    let new_bin = find_file(&tmp, "PCHome-desktop")
        .or_else(|| find_file(&tmp, "pchome-desktop"))
        .context("pakette ikili dosya yok")?;
    let current = std::env::current_exe().context("çalışan dosya yolu bilinmiyor")?;
    let backup = current.with_extension("bak");
    let _ = std::fs::remove_file(&backup);
    std::fs::copy(&current, &backup).ok();
    std::fs::copy(&new_bin, &current).context("ikili dosya yazılamadı (yetki?)")?;
    let _ = std::fs::remove_dir_all(&tmp);
    Ok(current)
}

fn find_file(dir: &std::path::Path, name: &str) -> Option<std::path::PathBuf> {
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        if let Ok(entries) = std::fs::read_dir(&d) {
            for e in entries.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p
                    .file_name()
                    .map(|n| n.to_string_lossy().to_lowercase() == name.to_lowercase())
                    .unwrap_or(false)
                {
                    return Some(p);
                }
            }
        }
    }
    None
}

/// Açılıştan 20 sn sonra ilk denetleme, sonra her 6 saatte bir.
/// Sinyal/WebRTC yolunu hiç etkilemez; hata yutumludur.
pub fn spawn_background_checker(state: SharedState) {
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(20));
        loop {
            if state.terminate.load(Ordering::SeqCst) {
                break;
            }
            check_once(&state);
            std::thread::sleep(std::time::Duration::from_secs(CHECK_INTERVAL_SECS));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surum_karsilastirma() {
        assert!(is_newer("v0.2.0", "0.1.0"));
        assert!(is_newer("0.1.1", "0.1.0"));
        assert!(!is_newer("v0.1.0", "0.1.0"));
        assert!(!is_newer("v0.0.9", "0.1.0"));
        assert!(is_newer("v0.10.0", "v0.9.9"));
    }

    #[test]
    fn insan_dostu_boyut() {
        assert_eq!(human_size(512), "512 B");
        assert_eq!(human_size(2048), "2 KB");
        assert_eq!(human_size(12_058_624), "11,5 MB");
    }

    #[test]
    fn surum_yaniti_cozulur() {
        let fixture = r#"{
            "tag_name": "v0.2.0",
            "body": "yenilikler",
            "assets": [
                {"name": "PCHome-v0.2.0-x86_64.AppImage", "browser_download_url": "https://x/app.AppImage"},
                {"name": "PCHome-desktop-v0.2.0-x86_64.tar.gz", "browser_download_url": "https://x/d.tar.gz"},
                {"name": "p chome-mobile-v0.2.0.apk", "browser_download_url": "https://x/m.apk"}
            ]
        }"#;
        let info = parse_release(fixture).unwrap();
        assert_eq!(info.version, "v0.2.0");
        assert_eq!(info.notes, "yenilikler");
        assert_eq!(info.appimage_url, "https://x/app.AppImage");
        assert_eq!(info.tarball_url, "https://x/d.tar.gz");
    }
}
