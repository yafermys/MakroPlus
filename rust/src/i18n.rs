//! Two-language UI: every string is written in both languages at the call site.
use std::sync::atomic::{AtomicBool, Ordering};

static TURKISH: AtomicBool = AtomicBool::new(true);

pub fn set_language(code: &str) {
    TURKISH.store(code != "en", Ordering::Relaxed);
}

pub fn is_tr() -> bool {
    TURKISH.load(Ordering::Relaxed)
}

/// Pick the English or Turkish text for the active language.
pub fn t<'a>(en: &'a str, tr: &'a str) -> &'a str {
    if is_tr() {
        tr
    } else {
        en
    }
}

/// Owned variant for formatted strings.
pub fn ts(en: String, tr: String) -> String {
    if is_tr() {
        tr
    } else {
        en
    }
}
