use std::sync::OnceLock;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Tr,
}

static LANG: OnceLock<Lang> = OnceLock::new();

fn detect() -> Lang {
    for key in ["LC_ALL", "LC_MESSAGES", "LANG", "LANGUAGE"] {
        if let Ok(value) = std::env::var(key) {
            let lower = value.trim().to_ascii_lowercase();
            if lower.is_empty() || lower == "c" || lower == "posix" {
                continue;
            }
            return if lower.starts_with("tr") || lower.contains(":tr") {
                Lang::Tr
            } else {
                Lang::En
            };
        }
    }
    Lang::En
}

pub fn lang() -> Lang {
    *LANG.get_or_init(detect)
}

pub fn is_tr() -> bool {
    lang() == Lang::Tr
}

/// Static UI text: English unless the desktop session is Turkish.
pub fn t(en: &'static str, tr: &'static str) -> &'static str {
    if is_tr() {
        tr
    } else {
        en
    }
}

pub fn pick(en: String, tr: String) -> String {
    if is_tr() {
        tr
    } else {
        en
    }
}
