//! Small cross-platform facade used by shared code.

/// Whether the main window is currently shown (not hidden in the tray or minimised).
#[cfg(windows)]
pub fn main_window_visible() -> bool {
    let w = crate::shell::MAIN_WINDOW.load(std::sync::atomic::Ordering::Relaxed);
    if w == 0 {
        return true;
    }
    unsafe { windows_sys::Win32::UI::WindowsAndMessaging::IsWindowVisible(w as _) != 0 && windows_sys::Win32::UI::WindowsAndMessaging::IsIconic(w as _) == 0 }
}

#[cfg(not(windows))]
pub fn main_window_visible() -> bool {
    crate::shell::MAIN_VISIBLE.load(std::sync::atomic::Ordering::Relaxed)
}

pub const OS: &str = if cfg!(windows) { "windows" } else { "linux" };

/// A path that can be registered as an application on this OS.
pub fn is_app_path(p: &str) -> bool {
    if p.trim().is_empty() {
        return false;
    }
    if cfg!(windows) {
        p.to_lowercase().ends_with(".exe") && std::path::Path::new(p).is_absolute()
    } else {
        std::path::Path::new(p).is_absolute()
    }
}

pub fn is_windows() -> bool {
    cfg!(windows)
}
