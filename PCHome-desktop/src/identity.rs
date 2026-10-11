use anyhow::{Context, Result};
use rand::RngCore;
use serde::{Deserialize, Serialize};
#[cfg(target_family = "unix")]
use std::os::unix::fs::PermissionsExt;

/// Bu PC'nin kalıcı kimliği: ilk açılışta bir kez üretilir,
/// `~/.config/PCHome/identity.toml` dosyasında saklanır ve asla
/// kendiliğinden değişmez. Telefon bu ID + sabit şifreyle bağlanır.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Identity {
    /// Kalıcı 6 haneli PC kimliği, örn. "482913".
    pub pc_id: String,
    /// Sabit 6 haneli bağlantı şifresi (kullanıcı değiştirebilir).
    pub password: String,
    /// Ekranda gösterilen dostça isim, örn. "colgecen-masaustu".
    pub pc_name: String,
}

impl Identity {
    /// Dosyadan okur; yoksa üretip 0600 izniyle kaydeder.
    /// `PCHOME_IDENTITY_FILE` ortam değişkeni tanımlıysa o yol kullanılır
    /// (testler gerçek kullanıcı dosyasını ezmesin diye).
    pub fn load_or_create() -> Result<Self> {
        let path = identity_path();
        log::info!("kimlik dosyası: {}", path.display());
        if path.exists() {
            let text = std::fs::read_to_string(&path)
                .with_context(|| format!("kimlik dosyası okunamadı: {}", path.display()))?;
            let mut id: Identity = toml::from_str(&text)
                .with_context(|| "kimlik dosyası bozuk, silinip yeniden oluşturulacak")?;
            if !is_valid_code(&id.pc_id) || !is_valid_code(&id.password) {
                log::warn!("kimlik dosyasındaki ID/şifre biçimi bozuk, yenileniyor");
                id.pc_id = (!is_valid_code(&id.pc_id))
                    .then(generate_6digit)
                    .unwrap_or(id.pc_id);
                id.password = (!is_valid_code(&id.password))
                    .then(generate_6digit)
                    .unwrap_or(id.password);
                id.save()?;
            }
            return Ok(id);
        }
        let id = Self {
            pc_id: generate_6digit(),
            password: generate_6digit(),
            pc_name: default_pc_name(),
        };
        id.save()?;
        log::info!("yeni kalıcı kimlik üretildi: ID={}", id.pc_id);
        Ok(id)
    }

    /// ID aynı kalır, yalnız şifre yenilenir (GUI'deki "Şifreyi Yenile").
    pub fn regenerate_password(&mut self) -> Result<()> {
        self.password = generate_6digit();
        self.save()
    }

    /// Şifreyi kullanıcı girdisiyle değiştirir (6 hane rakam olmalı).
    pub fn set_password(&mut self, password: &str) -> Result<()> {
        let clean: String = password.chars().filter(|c| c.is_ascii_digit()).collect();
        if !is_valid_code(&clean) {
            anyhow::bail!("şifre tam 6 hane rakam olmalı");
        }
        self.password = clean;
        self.save()
    }

    pub fn set_pc_name(&mut self, name: &str) -> Result<()> {
        let clean = name.trim();
        if clean.is_empty() || clean.len() > 64 {
            anyhow::bail!("pc adı 1-64 karakter olmalı");
        }
        self.pc_name = clean.to_string();
        self.save()
    }

    fn save(&self) -> Result<()> {
        let path = identity_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let text = toml::to_string_pretty(self)?;
        std::fs::write(&path, text)?;
        #[cfg(target_family = "unix")]
        {
            let mut perms = std::fs::metadata(&path)?.permissions();
            perms.set_mode(0o600);
            std::fs::set_permissions(&path, perms)?;
        }
        Ok(())
    }
}

/// Zamanlama saldırısına dayanıklı sabit süreli karşılaştırma.
pub fn verify_password(expected: &str, candidate: &str) -> bool {
    let a = expected.as_bytes();
    let b = candidate.trim().as_bytes();
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0 && is_valid_code(expected)
}

pub fn is_valid_code(s: &str) -> bool {
    s.len() == 6 && s.chars().all(|c| c.is_ascii_digit())
}

fn generate_6digit() -> String {
    let mut rng = rand::thread_rng();
    format!("{:06}", rng.next_u32() % 1_000_000)
}

fn default_pc_name() -> String {
    if let Ok(name) = std::env::var("HOSTNAME") {
        if !name.trim().is_empty() {
            return name.trim().to_string();
        }
    }
    if let Ok(text) = std::fs::read_to_string("/etc/hostname") {
        let clean = text.trim().to_string();
        if !clean.is_empty() {
            return clean;
        }
    }
    "PCHome-PC".to_string()
}

fn identity_path() -> std::path::PathBuf {
    if let Ok(p) = std::env::var("PCHOME_IDENTITY_FILE") {
        if !p.trim().is_empty() {
            return std::path::PathBuf::from(p);
        }
    }
    if let Ok(base) = std::env::var("XDG_CONFIG_HOME") {
        return std::path::Path::new(&base).join("PCHome").join("identity.toml");
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    std::path::Path::new(&home)
        .join(".config")
        .join("PCHome")
        .join("identity.toml")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Kimlik testleri `PCHOME_IDENTITY_FILE` ortam değişkenini paylaşır;
    /// paralel koşup birbirinin dosyasını ezmemeleri için seri çalışırlar.
    static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn temp_path(tag: &str) -> String {
        format!("/tmp/PCHome-identity-test-{}-{}.toml", tag, std::process::id())
    }

    #[test]
    fn sifre_dogrulama_sabit_sureli() {
        assert!(verify_password("123456", "123456"));
        assert!(verify_password("123456", " 123456 "));
        assert!(!verify_password("123456", "123455"));
        assert!(!verify_password("123456", "12345"));
        assert!(!verify_password("123456", "1234567"));
    }

    #[test]
    fn kimlik_uretilir_ve_ayni_kalir() {
        let _guard = TEST_LOCK.lock().unwrap();
        let path = temp_path("kalici");
        let _ = std::fs::remove_file(&path);
        std::env::set_var("PCHOME_IDENTITY_FILE", &path);
        let first = Identity::load_or_create().unwrap();
        assert!(is_valid_code(&first.pc_id));
        assert!(is_valid_code(&first.password));
        let second = Identity::load_or_create().unwrap();
        assert_eq!(first, second, "ikinci açılışta ID değişmemeli");
        std::env::remove_var("PCHOME_IDENTITY_FILE");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn sifre_yenileme_idyi_korur() {
        let _guard = TEST_LOCK.lock().unwrap();
        let path = temp_path("yenile");
        let _ = std::fs::remove_file(&path);
        std::env::set_var("PCHOME_IDENTITY_FILE", &path);
        let mut id = Identity::load_or_create().unwrap();
        let old_id = id.pc_id.clone();
        id.regenerate_password().unwrap();
        assert_eq!(id.pc_id, old_id);
        assert!(is_valid_code(&id.password));
        std::env::remove_var("PCHOME_IDENTITY_FILE");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn bozuk_sifre_reddedilir() {
        let _guard = TEST_LOCK.lock().unwrap();
        let path = temp_path("bozuk");
        let _ = std::fs::remove_file(&path);
        std::env::set_var("PCHOME_IDENTITY_FILE", &path);
        let mut id = Identity::load_or_create().unwrap();
        assert!(id.set_password("abc").is_err());
        assert!(id.set_password("12345").is_err());
        assert!(id.set_password("123456").is_ok());
        std::env::remove_var("PCHOME_IDENTITY_FILE");
        let _ = std::fs::remove_file(&path);
    }
}
