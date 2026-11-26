//! Linux per-user install for the portable binary: ~/.local/bin, a .desktop
//! launcher (app menu) and an icon. Packages (.deb/.rpm/AppImage) do this themselves.
use crate::i18n::{t, ts};
use std::path::{Path, PathBuf};

fn home() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/tmp".into()))
}

fn data_home() -> PathBuf {
    std::env::var("XDG_DATA_HOME").map(PathBuf::from).ok().filter(|p| p.is_absolute()).unwrap_or_else(|| home().join(".local/share"))
}

fn config_home() -> PathBuf {
    std::env::var("XDG_CONFIG_HOME").map(PathBuf::from).ok().filter(|p| p.is_absolute()).unwrap_or_else(|| home().join(".config"))
}

pub fn install_dir() -> PathBuf {
    home().join(".local/bin")
}

pub fn installed_exe() -> PathBuf {
    install_dir().join("makroplus")
}

fn desktop_file() -> PathBuf {
    data_home().join("applications/makroplus.desktop")
}

fn autostart_file() -> PathBuf {
    config_home().join("autostart/makroplus.desktop")
}

/// Installed by a package manager (/usr) or by this installer.
pub fn is_installed() -> bool {
    Path::new("/usr/bin/makroplus").exists() || (desktop_file().exists() && installed_exe().exists())
}

pub fn running_installed() -> bool {
    std::env::current_exe().map(|p| p.starts_with("/usr") || p == installed_exe()).unwrap_or(false)
}

fn desktop_entry(exe: &Path, extra: &str) -> String {
    format!(
        "[Desktop Entry]\nType=Application\nName=Makro+\nComment={}\nExec={}{}\nIcon=makroplus\nTerminal=false\nCategories=Utility;\nStartupWMClass=makroplus\n",
        t("Keyboard, mouse and touchpad automation", "Klavye, fare ve touchpad otomasyonu"),
        exe.display(),
        extra
    )
}

pub fn install(_desktop_shortcut: bool) -> Result<String, String> {
    let src = std::env::current_exe().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(install_dir()).map_err(|e| e.to_string())?;
    let exe = installed_exe();
    if src != exe {
        std::fs::copy(&src, &exe).map_err(|e| e.to_string())?;
    }
    let icon_dir = data_home().join("icons/hicolor/256x256/apps");
    std::fs::create_dir_all(&icon_dir).map_err(|e| e.to_string())?;
    std::fs::write(icon_dir.join("makroplus.png"), include_bytes!("../../assets/logo.png")).map_err(|e| e.to_string())?;
    let d = desktop_file();
    std::fs::create_dir_all(d.parent().unwrap()).map_err(|e| e.to_string())?;
    std::fs::write(&d, desktop_entry(&exe, "")).map_err(|e| e.to_string())?;
    if autostart_enabled() {
        let _ = set_autostart(true, &exe);
    }
    Ok(ts(format!("Installed to {}. Makro+ now appears in your applications menu.", exe.display()), format!("{} konumuna yüklendi. Makro+ artık uygulama menünüzde görünüyor.", exe.display())))
}

pub fn autostart_enabled() -> bool {
    autostart_file().exists()
}

pub fn set_autostart(on: bool, exe: &Path) -> Result<(), String> {
    let f = autostart_file();
    if on {
        std::fs::create_dir_all(f.parent().unwrap()).map_err(|e| e.to_string())?;
        std::fs::write(&f, desktop_entry(exe, " --tray")).map_err(|e| e.to_string())
    } else {
        let _ = std::fs::remove_file(f);
        Ok(())
    }
}

pub fn uninstall(quiet: bool) -> i32 {
    if !quiet && !crate::win::message_box(t("Remove Makro+ from your applications menu?", "Makro+ uygulama menünüzden kaldırılsın mı?"), "Makro+", true) {
        return 1;
    }
    let _ = std::fs::remove_file(desktop_file());
    let _ = std::fs::remove_file(autostart_file());
    let _ = std::fs::remove_file(installed_exe());
    let _ = std::fs::remove_file(data_home().join("icons/hicolor/256x256/apps/makroplus.png"));
    0
}
