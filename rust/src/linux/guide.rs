//! Shortcut guide on Linux: Wayland does not allow floating overlays from
//! regular apps, so the guide is shown as a desktop notification.
use crate::win::Hwnd;
use std::sync::atomic::{AtomicBool, Ordering};

static VISIBLE: AtomicBool = AtomicBool::new(false);

pub fn visible() -> bool {
    VISIBLE.load(Ordering::Relaxed)
}

pub fn show(_fg: Hwnd, cfg: &crate::engine::Config, app: &str) {
    VISIBLE.store(true, Ordering::Relaxed);
    let mut lines: Vec<String> = cfg
        .bindings
        .iter()
        .filter(|b| b.app_path.is_empty() || crate::win::same_path(&b.app_path, app))
        .take(14)
        .map(|b| format!("{}  —  {}", b.combo(), b.name))
        .collect();
    if lines.is_empty() {
        lines.push(crate::i18n::t("No shortcuts for this app yet.", "Bu uygulama için henüz kısayol yok.").into());
    }
    let title = if app.is_empty() { "Makro+".to_string() } else { format!("{} — {}", crate::apps::display_name(app), crate::i18n::t("shortcuts", "kısayollar")) };
    crate::shell::notify(&title, &lines.join("\n"));
}

pub fn hide() {
    VISIBLE.store(false, Ordering::Relaxed);
}
