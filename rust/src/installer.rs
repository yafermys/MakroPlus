//! Optional per-user installation: Start menu, "Installed apps" entry, autostart.
//! The portable exe keeps working without any of this.
use crate::i18n::{t, ts};
use std::path::{Path, PathBuf};
use winreg::enums::*;
use winreg::RegKey;

const UNINSTALL_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\MakroPlus";
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

pub fn install_dir() -> PathBuf {
    PathBuf::from(std::env::var("LOCALAPPDATA").unwrap_or_default()).join("Programs").join("MakroPlus")
}

pub fn installed_exe() -> PathBuf {
    install_dir().join("MakroPlus.exe")
}

pub fn is_installed() -> bool {
    RegKey::predef(HKEY_CURRENT_USER).open_subkey(UNINSTALL_KEY).is_ok() && installed_exe().exists()
}

pub fn running_installed() -> bool {
    std::env::current_exe().map(|p| crate::win::same_path(&p.display().to_string(), &installed_exe().display().to_string())).unwrap_or(false)
}

fn start_menu() -> PathBuf {
    PathBuf::from(std::env::var("APPDATA").unwrap_or_default()).join(r"Microsoft\Windows\Start Menu\Programs").join("Makro+.lnk")
}

fn desktop() -> PathBuf {
    PathBuf::from(std::env::var("USERPROFILE").unwrap_or_default()).join("Desktop").join("Makro+.lnk")
}

fn ps_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

fn create_shortcut(lnk: &Path, target: &Path) -> Result<(), String> {
    let dir = target.parent().unwrap_or(Path::new("."));
    let script = format!(
        "$s=(New-Object -ComObject WScript.Shell).CreateShortcut({});$s.TargetPath={};$s.WorkingDirectory={};$s.IconLocation={};$s.Description='Makro+';$s.Save()",
        ps_quote(&lnk.display().to_string()),
        ps_quote(&target.display().to_string()),
        ps_quote(&dir.display().to_string()),
        ps_quote(&format!("{},0", target.display()))
    );
    let (code, _) = crate::actions::run_powershell(&script, &|| false)?;
    if code == 0 {
        Ok(())
    } else {
        Err(t("Shortcut could not be created.", "Kısayol oluşturulamadı.").into())
    }
}

fn copy_dir(src: &Path, dst: &Path) -> std::io::Result<u64> {
    let mut total = 0;
    std::fs::create_dir_all(dst)?;
    for e in std::fs::read_dir(src)? {
        let e = e?;
        let p = e.path();
        let d = dst.join(e.file_name());
        if p.is_dir() {
            total += copy_dir(&p, &d)?;
        } else {
            total += std::fs::copy(&p, &d)?;
        }
    }
    Ok(total)
}

fn dir_size(p: &Path) -> u64 {
    std::fs::read_dir(p).map(|rd| rd.flatten().map(|e| if e.path().is_dir() { dir_size(&e.path()) } else { e.metadata().map(|m| m.len()).unwrap_or(0) }).sum()).unwrap_or(0)
}

/// Copy the app into %LOCALAPPDATA%\Programs\MakroPlus and register it.
pub fn install(desktop_shortcut: bool) -> Result<String, String> {
    let src_exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let src_dir = src_exe.parent().unwrap().to_path_buf();
    let dir = install_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let exe = installed_exe();
    if !crate::win::same_path(&src_exe.display().to_string(), &exe.display().to_string()) {
        std::fs::copy(&src_exe, &exe).map_err(|e| format!("{}: {e}", t("Could not copy MakroPlus.exe (close the installed copy first)", "MakroPlus.exe kopyalanamadı (önce kurulu kopyayı kapatın)")))?;
        for extra in ["runtime", "models", "USER_GUIDE.html", "LICENSE", "THIRD_PARTY_NOTICES.md", "BENIOKU.txt", "START_HERE.txt"] {
            let p = src_dir.join(extra);
            if p.is_dir() {
                let _ = copy_dir(&p, &dir.join(extra));
            } else if p.exists() {
                let _ = std::fs::copy(&p, dir.join(extra));
            }
        }
        // A runtime that only exists in the development tree.
        if !dir.join("runtime").exists() {
            if let Some(rt) = crate::llm::find_runtime() {
                if let Some(llama) = rt.parent() {
                    let _ = copy_dir(llama, &dir.join("runtime").join("llama"));
                }
            }
        }
    }
    std::fs::create_dir_all(dir.join("models")).ok();
    create_shortcut(&start_menu(), &exe)?;
    if desktop_shortcut {
        let _ = create_shortcut(&desktop(), &exe);
    }
    let (key, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(UNINSTALL_KEY).map_err(|e| e.to_string())?;
    let q = |p: &Path| format!("\"{}\"", p.display());
    let set = |n: &str, v: &str| key.set_value(n, &v.to_string()).map_err(|e| e.to_string());
    set("DisplayName", "Makro+")?;
    set("DisplayVersion", env!("CARGO_PKG_VERSION"))?;
    set("Publisher", "Makro+")?;
    set("DisplayIcon", &format!("{},0", exe.display()))?;
    set("InstallLocation", &dir.display().to_string())?;
    set("UninstallString", &format!("{} --uninstall", q(&exe)))?;
    set("QuietUninstallString", &format!("{} --uninstall --quiet", q(&exe)))?;
    set("URLInfoAbout", "https://github.com/yafermys/MakroPlus")?;
    set("InstallDate", &chrono::Local::now().format("%Y%m%d").to_string())?;
    key.set_value("NoModify", &1u32).map_err(|e| e.to_string())?;
    key.set_value("NoRepair", &1u32).map_err(|e| e.to_string())?;
    key.set_value("EstimatedSize", &((dir_size(&dir) / 1024) as u32)).map_err(|e| e.to_string())?;
    // Keep autostart pointing at the installed copy.
    if autostart_enabled() {
        let _ = set_autostart(true, &exe);
    }
    Ok(ts(
        format!("Installed to {}. Makro+ now appears in Start and in Settings › Apps › Installed apps.", dir.display()),
        format!("{} konumuna yüklendi. Makro+ artık Başlat menüsünde ve Ayarlar › Uygulamalar › Yüklü uygulamalar listesinde görünüyor.", dir.display()),
    ))
}

pub fn autostart_enabled() -> bool {
    RegKey::predef(HKEY_CURRENT_USER).open_subkey(RUN_KEY).and_then(|k| k.get_value::<String, _>("MakroPlus")).is_ok()
}

pub fn set_autostart(on: bool, exe: &Path) -> Result<(), String> {
    let (key, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(RUN_KEY).map_err(|e| e.to_string())?;
    if on {
        key.set_value("MakroPlus", &format!("\"{}\" --tray", exe.display())).map_err(|e| e.to_string())
    } else {
        let _ = key.delete_value("MakroPlus");
        Ok(())
    }
}

/// Entry point for `--uninstall` (Settings › Apps runs this).
pub fn uninstall(quiet: bool) -> i32 {
    if !quiet && !crate::win::message_box(t("Remove Makro+ from this computer?", "Makro+ bu bilgisayardan kaldırılsın mı?"), "Makro+", true) {
        return 1;
    }
    let remove_data = !quiet && crate::win::message_box(t("Also delete your Makro+ settings and clipboard history?\n\nChoose No to keep them for a later reinstall.", "Makro+ ayarlarınız ve pano geçmişiniz de silinsin mi?\n\nDaha sonra yeniden kurmak için saklamak isterseniz Hayır'ı seçin."), "Makro+", true);
    // Stop other running copies of the installed exe.
    let exe = installed_exe().display().to_string();
    let me = std::process::id();
    for w in crate::win::top_windows() {
        let pid = crate::win::window_pid(w);
        if pid != me && crate::win::same_path(&crate::win::process_path(pid), &exe) {
            unsafe {
                let h = windows_sys::Win32::System::Threading::OpenProcess(windows_sys::Win32::System::Threading::PROCESS_TERMINATE, 0, pid);
                if !h.is_null() {
                    windows_sys::Win32::System::Threading::TerminateProcess(h, 0);
                    windows_sys::Win32::Foundation::CloseHandle(h);
                }
            }
        }
    }
    let _ = RegKey::predef(HKEY_CURRENT_USER).delete_subkey_all(UNINSTALL_KEY);
    if let Ok(k) = RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(RUN_KEY, KEY_WRITE) {
        let _ = k.delete_value("MakroPlus");
    }
    let _ = std::fs::remove_file(start_menu());
    let _ = std::fs::remove_file(desktop());
    if remove_data {
        let _ = std::fs::remove_dir_all(crate::storage::data_dir());
    }
    // The running exe cannot delete itself; a detached cmd removes the folder after we exit.
    let dir = install_dir();
    let cmd = format!("/c ping 127.0.0.1 -n 3 >nul & rmdir /s /q \"{}\"", dir.display());
    use std::os::windows::process::CommandExt;
    let _ = crate::apps::hidden_command("cmd.exe").raw_arg(cmd).current_dir(std::env::temp_dir()).spawn();
    if !quiet {
        crate::win::message_box(t("Makro+ was removed.", "Makro+ kaldırıldı."), "Makro+", false);
    }
    0
}
