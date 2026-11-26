//! Native Windows actions (recommended shortcuts).
use crate::i18n::{is_tr, t};
use std::sync::atomic::{AtomicBool, Ordering};

pub struct Spec {
    pub id: &'static str,
    pub en: &'static str,
    pub tr: &'static str,
    pub desc_en: &'static str,
    pub desc_tr: &'static str,
    pub category: &'static str,
    pub confirm: bool,
}

pub const CATALOG: &[Spec] = &[
    Spec { id: "lock", en: "Lock computer", tr: "Bilgisayarı kilitle", desc_en: "Lock your session immediately.", desc_tr: "Oturumunuzu hemen kilitler.", category: "System", confirm: false },
    Spec { id: "desktop", en: "Show desktop", tr: "Masaüstünü göster", desc_en: "Hide or restore your open windows.", desc_tr: "Açık pencereleri gizler veya geri getirir.", category: "Everyday", confirm: false },
    Spec { id: "task-view", en: "Switch workspace", tr: "Çalışma alanı değiştir", desc_en: "See open windows and virtual desktops.", desc_tr: "Açık pencereleri ve sanal masaüstlerini gösterir.", category: "Everyday", confirm: false },
    Spec { id: "screenshot", en: "Capture a screen region", tr: "Ekran bölgesi yakala", desc_en: "Choose an area and copy a screenshot.", desc_tr: "Bir alan seçip ekran görüntüsünü kopyalar.", category: "Everyday", confirm: false },
    Spec { id: "clipboard", en: "Windows clipboard history", tr: "Windows pano geçmişi", desc_en: "Open the Windows clipboard history (Win+V).", desc_tr: "Windows pano geçmişini açar (Win+V).", category: "Everyday", confirm: false },
    Spec { id: "makro-clipboard", en: "Makro+ clipboard", tr: "Makro+ panosu", desc_en: "Open the Makro+ clipboard page to view and edit copied items.", desc_tr: "Kopyalananları görmek ve düzenlemek için Makro+ pano sayfasını açar.", category: "Everyday", confirm: false },
    Spec { id: "emoji", en: "Emoji and symbols", tr: "Emoji ve semboller", desc_en: "Insert symbols, emoji and special characters.", desc_tr: "Sembol, emoji ve özel karakter ekler.", category: "Everyday", confirm: false },
    Spec { id: "explorer", en: "Open File Explorer", tr: "Dosya Gezgini'ni aç", desc_en: "Jump straight to your files.", desc_tr: "Doğrudan dosyalarınıza gider.", category: "Everyday", confirm: false },
    Spec { id: "task-manager", en: "Open Task Manager", tr: "Görev Yöneticisi'ni aç", desc_en: "Inspect running applications and resource usage.", desc_tr: "Çalışan uygulamaları ve kaynak kullanımını gösterir.", category: "System", confirm: false },
    Spec { id: "audio", en: "Sound output settings", tr: "Ses çıkışı ayarları", desc_en: "Change speakers, headphones and input devices.", desc_tr: "Hoparlör, kulaklık ve giriş aygıtlarını değiştirir.", category: "System", confirm: false },
    Spec { id: "network", en: "Network settings", tr: "Ağ ayarları", desc_en: "Manage Wi-Fi, Ethernet and connection settings.", desc_tr: "Wi-Fi, Ethernet ve bağlantı ayarlarını yönetir.", category: "System", confirm: false },
    Spec { id: "storage", en: "Storage settings", tr: "Depolama ayarları", desc_en: "Review disk usage and cleanup options.", desc_tr: "Disk kullanımını ve temizleme seçeneklerini gösterir.", category: "System", confirm: false },
    Spec { id: "updates", en: "Check Windows updates", tr: "Windows güncellemelerini denetle", desc_en: "Start an online scan. Nothing is installed.", desc_tr: "Çevrimiçi tarama başlatır. Hiçbir şey yüklenmez.", category: "System", confirm: false },
    Spec { id: "update-settings", en: "Open Windows Update", tr: "Windows Update'i aç", desc_en: "View update status and controls.", desc_tr: "Güncelleme durumunu ve denetimlerini gösterir.", category: "System", confirm: false },
    Spec { id: "close-apps", en: "Close open applications", tr: "Açık uygulamaları kapat", desc_en: "Request closure of visible app windows. Save dialogs are respected.", desc_tr: "Görünür uygulama pencerelerinin kapanmasını ister. Kaydetme soruları korunur.", category: "Power", confirm: true },
    Spec { id: "shutdown", en: "Shut down computer", tr: "Bilgisayarı kapat", desc_en: "Ask Windows to shut down after confirmation.", desc_tr: "Onaydan sonra Windows'un kapanmasını ister.", category: "Power", confirm: true },
    Spec { id: "restart", en: "Restart computer", tr: "Bilgisayarı yeniden başlat", desc_en: "Ask Windows to restart after confirmation.", desc_tr: "Onaydan sonra Windows'un yeniden başlamasını ister.", category: "Power", confirm: true },
];

pub fn find(id: &str) -> Option<&'static Spec> {
    CATALOG.iter().find(|s| s.id == id)
}

pub fn title(id: &str) -> String {
    find(id).map(|s| if is_tr() { s.tr } else { s.en }.to_string()).unwrap_or_else(|| id.to_string())
}

pub fn description(id: &str) -> String {
    find(id).map(|s| if is_tr() { s.desc_tr } else { s.desc_en }.to_string()).unwrap_or_default()
}

static BUSY: AtomicBool = AtomicBool::new(false);

#[cfg(windows)]
fn sys32(exe: &str) -> String {
    let root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into());
    format!("{root}\\System32\\{exe}")
}

/// Run a system action. `ui` receives requests that need the Makro+ window.
#[cfg(windows)]
pub fn run(id: &str, ui: &dyn Fn(crate::UiEvent)) -> Result<String, String> {
    let sent = t("Request sent to Windows.", "İstek Windows'a gönderildi.").to_string();
    let spec = find(id).ok_or_else(|| t("Unknown system action.", "Bilinmeyen sistem eylemi.").to_string())?;
    let chord = |mods: i32, key: i32| crate::win::send_chord(mods, key, false).map(|_| sent.clone());
    match id {
        "lock" => {
            if unsafe { windows_sys::Win32::System::Shutdown::LockWorkStation() } == 0 {
                Err(t("Could not lock the session.", "Oturum kilitlenemedi.").into())
            } else {
                Ok(sent)
            }
        }
        "desktop" => chord(8, 68),
        "task-view" => chord(8, 9),
        "screenshot" => chord(12, 83),
        "clipboard" => chord(8, 86),
        "emoji" => chord(8, 190),
        "makro-clipboard" => {
            ui(crate::UiEvent::ShowPage("clipboard".into()));
            Ok(t("Clipboard page opened.", "Pano sayfası açıldı.").into())
        }
        "explorer" => crate::win::shell_open("explorer.exe", "", "").map(|_| sent),
        "task-manager" => crate::win::shell_open(&sys32("Taskmgr.exe"), "", "").map(|_| sent),
        "audio" => crate::win::shell_open("ms-settings:sound", "", "").map(|_| sent),
        "network" => crate::win::shell_open("ms-settings:network", "", "").map(|_| sent),
        "storage" => crate::win::shell_open("ms-settings:storagesense", "", "").map(|_| sent),
        "update-settings" => crate::win::shell_open("ms-settings:windowsupdate", "", "").map(|_| sent),
        "updates" => {
            if BUSY.swap(true, Ordering::SeqCst) {
                return Ok(t("An update scan is already running.", "Bir güncelleme taraması zaten çalışıyor.").into());
            }
            let script = "$s=New-Object -ComObject Microsoft.Update.Session;$r=$s.CreateUpdateSearcher().Search(\"IsInstalled=0 and IsHidden=0 and Type='Software'\");Write-Output ($r.ResultCode.ToString()+' '+$r.Updates.Count)";
            let out = crate::actions::run_powershell(script, &|| false);
            BUSY.store(false, Ordering::SeqCst);
            let (code, text) = out?;
            let parts: Vec<&str> = text.split_whitespace().collect();
            if code != 0 || parts.len() < 2 {
                return Err(t("Windows Update scan failed. Check your connection and policy.", "Windows Update taraması başarısız. Bağlantınızı ve ilkeleri kontrol edin.").into());
            }
            Ok(crate::i18n::ts(
                format!("Available software updates: {}. Open Windows Update to review. Nothing was installed.", parts[1]),
                format!("Bekleyen yazılım güncellemesi: {}. Ayrıntılar için Windows Update'i açın. Hiçbir şey yüklenmedi.", parts[1]),
            ))
        }
        "close-apps" => {
            let own = std::process::id();
            let excluded = ["shellexperiencehost", "startmenuexperiencehost", "searchhost", "searchapp", "textinputhost", "lockapp"];
            let mut targets = Vec::new();
            for w in crate::win::top_windows() {
                let pid = crate::win::window_pid(w);
                let class = crate::win::window_class(w);
                if pid == own || ["Progman", "WorkerW", "Shell_TrayWnd", "Shell_SecondaryTrayWnd", "#32770"].contains(&class.as_str()) {
                    continue;
                }
                let exe = crate::win::file_stem(&crate::win::process_path(pid)).to_lowercase();
                if exe.is_empty() || excluded.contains(&exe.as_str()) {
                    continue;
                }
                targets.push((w, exe, crate::win::window_title(w)));
            }
            if targets.is_empty() {
                return Ok(t("No eligible application windows are open.", "Kapatılabilecek açık uygulama penceresi yok.").into());
            }
            let list: Vec<String> = targets.iter().take(12).map(|(_, e, tt)| format!("{e} — {tt}")).collect();
            let q = format!("{}\n\n{}{}", t("Close these application windows?", "Bu uygulama pencereleri kapatılsın mı?"), list.join("\n"), if targets.len() > 12 { format!("\n… +{}", targets.len() - 12) } else { String::new() });
            if !crate::win::message_box(&q, "Makro+", true) {
                return Ok(t("Cancelled. No action was sent.", "İptal edildi. Hiçbir işlem gönderilmedi.").into());
            }
            let sent_count = targets.iter().filter(|(w, _, _)| crate::win::post_close(*w)).count();
            Ok(crate::i18n::ts(
                format!("Close requests sent: {sent_count} / {}. Save prompts may keep apps open.", targets.len()),
                format!("Kapatma isteği gönderildi: {sent_count} / {}. Kaydetme soruları uygulamaları açık tutabilir.", targets.len()),
            ))
        }
        "shutdown" | "restart" => {
            let q = format!("{}?\n\n{}", title(id), description(id));
            if spec.confirm && !crate::win::message_box(&q, "Makro+", true) {
                return Ok(t("Cancelled. No action was sent.", "İptal edildi. Hiçbir işlem gönderilmedi.").into());
            }
            let arg = if id == "shutdown" { "/s /t 0" } else { "/r /t 0" };
            crate::apps::hidden_command(&sys32("shutdown.exe")).args(arg.split(' ')).spawn().map(|_| sent).map_err(|e| e.to_string())
        }
        _ => Err(t("Unknown system action.", "Bilinmeyen sistem eylemi.").into()),
    }
}

/// First command that starts successfully.
#[cfg(not(windows))]
fn first(cmds: &[&[&str]]) -> Result<(), String> {
    for c in cmds {
        if std::process::Command::new(c[0]).args(&c[1..]).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn().is_ok() {
            return Ok(());
        }
    }
    Err(t("No suitable tool is installed for this action on this desktop.", "Bu masaüstünde bu eylem için uygun bir araç kurulu değil.").into())
}

#[cfg(not(windows))]
pub fn run(id: &str, ui: &dyn Fn(crate::UiEvent)) -> Result<String, String> {
    let sent = t("Request sent.", "İstek gönderildi.").to_string();
    let spec = find(id).ok_or_else(|| t("Unknown system action.", "Bilinmeyen sistem eylemi.").to_string())?;
    let r = match id {
        "lock" => first(&[&["loginctl", "lock-session"], &["xdg-screensaver", "lock"], &["dm-tool", "lock"]]),
        "screenshot" => first(&[&["gnome-screenshot", "-a", "-c"], &["spectacle", "-r", "-b", "-c"], &["flameshot", "gui"], &["sh", "-c", "grim -g \"$(slurp)\" - | wl-copy"], &["xfce4-screenshooter", "-r", "-c"]]),
        "explorer" => first(&[&["xdg-open", &std::env::var("HOME").unwrap_or_else(|_| "/".into())]]),
        "task-manager" => first(&[&["gnome-system-monitor"], &["plasma-systemmonitor"], &["ksysguard"], &["xfce4-taskmanager"], &["mate-system-monitor"]]),
        "audio" => first(&[&["gnome-control-center", "sound"], &["systemsettings", "kcm_pulseaudio"], &["pavucontrol"]]),
        "network" => first(&[&["gnome-control-center", "network"], &["systemsettings", "kcm_networkmanagement"], &["nm-connection-editor"]]),
        "storage" => first(&[&["baobab"], &["filelight"], &["gnome-disks"]]),
        "updates" | "update-settings" => first(&[&["gnome-software", "--mode=updates"], &["plasma-discover", "--mode", "update"], &["mintupdate"], &["update-manager"]]),
        "makro-clipboard" => {
            ui(crate::UiEvent::ShowPage("clipboard".into()));
            Ok(())
        }
        "shutdown" | "restart" => {
            let q = format!("{}?\n\n{}", title(id), description(id));
            if spec.confirm && !crate::win::message_box(&q, "Makro+", true) {
                return Ok(t("Cancelled. No action was sent.", "İptal edildi. Hiçbir işlem gönderilmedi.").into());
            }
            first(&[&["systemctl", if id == "shutdown" { "poweroff" } else { "reboot" }]])
        }
        "close-apps" => {
            let wins = crate::win::top_windows();
            let own = std::process::id();
            let targets: Vec<_> = wins.into_iter().filter(|&w| crate::win::window_pid(w) != own).collect();
            if targets.is_empty() {
                return Ok(t("No eligible application windows are open (or this desktop does not expose them).", "Kapatılabilecek pencere yok (veya bu masaüstü pencereleri paylaşmıyor).").into());
            }
            let q = format!("{} ({})", t("Close these application windows?", "Bu uygulama pencereleri kapatılsın mı?"), targets.len());
            if !crate::win::message_box(&q, "Makro+", true) {
                return Ok(t("Cancelled. No action was sent.", "İptal edildi. Hiçbir işlem gönderilmedi.").into());
            }
            let n = targets.iter().filter(|&&w| crate::win::post_close(w)).count();
            return Ok(crate::i18n::ts(format!("Close requests sent: {n}."), format!("Kapatma isteği gönderildi: {n}.")));
        }
        // Desktop-specific keyboard shortcuts on Linux; sent as keys.
        "desktop" => crate::win::send_chord(8, 68, false),
        "task-view" => crate::win::send_chord(0, 91, false),
        "clipboard" => {
            ui(crate::UiEvent::ShowPage("clipboard".into()));
            Ok(())
        }
        "emoji" => first(&[&["gnome-characters"], &["plasma-emojier"], &["ibus", "emoji"]]),
        _ => Err(t("Unknown system action.", "Bilinmeyen sistem eylemi.").into()),
    };
    r.map(|_| sent)
}