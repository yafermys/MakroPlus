//! Validation and conflict detection for shortcuts.
use crate::i18n::{t, ts};
use crate::keys;
use crate::model::{ActionStep, Binding};
use crate::win::same_path;

/// Step kinds available in the editor: (id, English, Turkish, group).
pub const STEP_KINDS: &[(&str, &str, &str, &str)] = &[
    ("Key", "Press keys", "Tuşlara bas", "input"),
    ("Text", "Type text", "Metin yaz", "input"),
    ("Paste text", "Paste text", "Metin yapıştır", "input"),
    ("Wait", "Wait", "Bekle", "input"),
    ("Focus", "Bring app to front (opens it if needed)", "Uygulamayı öne getir (gerekirse açar)", "apps"),
    ("Open application", "Open application", "Uygulama aç", "apps"),
    ("Open content", "Open file / URL / link", "Dosya / adres / bağlantı aç", "apps"),
    ("Wait for window", "Wait for a window", "Pencereyi bekle", "apps"),
    ("Window action", "Window: minimize / maximize / close…", "Pencere: küçült / büyüt / kapat…", "apps"),
    ("Menu command", "Run an app menu command", "Uygulama menü komutu çalıştır", "inapp"),
    ("Click element", "Click an element in the app", "Uygulamadaki bir öğeye tıkla", "inapp"),
    ("Click button", "Click a button by its name", "Adıyla düğmeye tıkla", "inapp"),
    ("Set field", "Fill a field in the app", "Uygulamadaki alanı doldur", "inapp"),
    ("Read field", "Read a field into a variable", "Alanı değişkene oku", "inapp"),
    ("Copy selection", "Copy selection into a variable", "Seçimi değişkene kopyala", "clipboard"),
    ("Paste files", "Paste files (one path per line)", "Dosyaları yapıştır (satır başına bir yol)", "clipboard"),
    ("Clipboard set", "Set clipboard text", "Pano metnini ayarla", "clipboard"),
    ("Clipboard append", "Append to clipboard", "Panoya ekle", "clipboard"),
    ("Write file", "Write / append to a file", "Dosyaya yaz / ekle", "files"),
    ("Read file", "Read a file into a variable", "Dosyayı değişkene oku", "files"),
    ("Copy file", "Copy file", "Dosya kopyala", "files"),
    ("Move file", "Move file", "Dosya taşı", "files"),
    ("AI", "AI: transform text when it runs", "AI: çalışırken metni işle", "ai"),
    ("Set variable", "Set a variable", "Değişken ata", "ai"),
    ("Notify", "Show a notification", "Bildirim göster", "ai"),
    ("PowerShell", "PowerShell script", "PowerShell betiği", "system"),
    ("Terminal (cmd)", "Command prompt (cmd)", "Komut istemi (cmd)", "system"),
    ("System", "Windows action", "Windows eylemi", "system"),
    ("Mouse click", "Mouse click (window-relative)", "Fare tıklaması (pencereye göre)", "input"),
    ("Mouse drag", "Mouse drag", "Fare sürükleme", "input"),
    ("Scroll", "Scroll", "Kaydırma", "input"),
];

pub fn step_label(kind: &str) -> String {
    STEP_KINDS.iter().find(|k| k.0 == kind).map(|k| t(k.1, k.2).to_string()).unwrap_or_else(|| kind.to_string())
}

/// Simple (non-workflow) action kinds.
pub const ACTION_KINDS: &[(&str, &str, &str)] = &[
    ("Workflow", "Steps (workflow, multi-app)", "Adımlar (iş akışı, çok uygulamalı)"),
    ("Application", "Open application", "Uygulama aç"),
    ("File / folder", "Open file or folder", "Dosya veya klasör aç"),
    ("Web address", "Open web address", "Web adresi aç"),
    ("Browser website", "Open site in a chosen browser", "Siteyi seçilen tarayıcıda aç"),
    ("URI / deep link", "Open app link (URI)", "Uygulama bağlantısı (URI)"),
    ("Spotify", "Spotify content", "Spotify içeriği"),
    ("Send key", "Send another key", "Başka bir tuş gönder"),
    ("Media", "Media key", "Medya tuşu"),
    ("System", "Windows action", "Windows eylemi"),
    ("PowerShell", "PowerShell script", "PowerShell betiği"),
    ("Terminal (cmd)", "Command prompt (cmd)", "Komut istemi (cmd)"),
    ("Disable key", "Switch this key off (its function is disabled)", "Bu tuşu kapat (işlevi devre dışı)"),
];

pub fn action_label(kind: &str) -> String {
    ACTION_KINDS.iter().find(|k| k.0 == kind).map(|k| t(k.1, k.2).to_string()).unwrap_or_else(|| kind.to_string())
}

pub const MEDIA: &[(&str, &str, &str)] = &[
    ("Play / pause", "Play / pause", "Oynat / duraklat"),
    ("Next track", "Next track", "Sonraki parça"),
    ("Previous track", "Previous track", "Önceki parça"),
    ("Volume up", "Volume up", "Sesi aç"),
    ("Volume down", "Volume down", "Sesi kıs"),
    ("Mute / unmute", "Mute / unmute", "Sessiz / sesli"),
];

struct Known {
    name_en: &'static str,
    name_tr: &'static str,
    mods: i32,
    key: i32,
    app: &'static str,
}

const KNOWN: &[Known] = &[
    Known { name_en: "Copy", name_tr: "Kopyala", mods: 2, key: 67, app: "Windows" },
    Known { name_en: "Paste", name_tr: "Yapıştır", mods: 2, key: 86, app: "Windows" },
    Known { name_en: "Cut", name_tr: "Kes", mods: 2, key: 88, app: "Windows" },
    Known { name_en: "Undo", name_tr: "Geri al", mods: 2, key: 90, app: "Windows" },
    Known { name_en: "Select all", name_tr: "Tümünü seç", mods: 2, key: 65, app: "Windows" },
    Known { name_en: "Save", name_tr: "Kaydet", mods: 2, key: 83, app: "Windows" },
    Known { name_en: "Switch apps", name_tr: "Uygulama değiştir", mods: 1, key: 9, app: "Windows" },
    Known { name_en: "Task Manager", name_tr: "Görev Yöneticisi", mods: 6, key: 27, app: "Windows" },
    Known { name_en: "Play / pause", name_tr: "Oynat / duraklat", mods: 0, key: 32, app: "Spotify.exe" },
    Known { name_en: "Command palette", name_tr: "Komut paleti", mods: 6, key: 80, app: "Code.exe" },
    Known { name_en: "Go to file", name_tr: "Dosyaya git", mods: 2, key: 80, app: "Code.exe" },
];

#[derive(Clone, PartialEq)]
pub enum Severity {
    Conflict,
    Priority,
    Possible,
}

pub struct Issue {
    pub severity: Severity,
    pub text: String,
}

fn overlap_profiles(a: &Binding, b: &Binding) -> bool {
    a.profile_id.is_empty() || b.profile_id.is_empty() || a.profile_id == b.profile_id
}

pub fn conflicts(b: &Binding, all: &[Binding]) -> Vec<Issue> {
    let mut out = Vec::new();
    if b.gesture.is_some() || b.key == 0 {
        return out;
    }
    for old in all.iter().filter(|x| x.gesture.is_none() && x.id != b.id && x.enabled && b.enabled && overlap_profiles(b, x) && x.mods == b.mods) {
        let same_chord = old.key == b.key && old.prefix_key == b.prefix_key;
        let prefix_clash = (old.prefix_key == 0 && old.key == b.prefix_key && b.prefix_key != 0) || (b.prefix_key == 0 && b.key == old.prefix_key && old.prefix_key != 0);
        if !same_chord && !prefix_clash {
            continue;
        }
        let scope_overlap = b.app_path.is_empty() || old.app_path.is_empty() || same_path(&b.app_path, &old.app_path);
        if prefix_clash && scope_overlap {
            out.push(Issue { severity: Severity::Conflict, text: ts(format!("{} is the start of “{}”.", b.combo(), old.name), format!("{} tuşu “{}” kısayolunun başlangıç tuşu.", b.combo(), old.name)) });
            continue;
        }
        if same_path(&b.app_path, &old.app_path) || (b.app_path.is_empty() && old.app_path.is_empty()) {
            if b.profile_id != old.profile_id {
                out.push(Issue { severity: Severity::Priority, text: ts(format!("{}: the profile-specific assignment runs before “{}”.", b.combo(), old.name), format!("{}: profile özel atama “{}” atamasından önce çalışır.", b.combo(), old.name)) });
            } else {
                out.push(Issue { severity: Severity::Conflict, text: ts(format!("{} is already used by “{}” [{}].", b.combo(), old.name, old.scope_name()), format!("{} zaten “{}” [{}] tarafından kullanılıyor.", b.combo(), old.name, old.scope_name())) });
            }
        } else if b.app_path.is_empty() || old.app_path.is_empty() {
            out.push(Issue {
                severity: Severity::Priority,
                text: ts(format!("{}: the app-specific assignment wins inside its app (“{}” [{}]).", b.combo(), old.name, old.scope_name()), format!("{}: uygulamaya özel atama kendi uygulamasında öncelikli (“{}” [{}]).", b.combo(), old.name, old.scope_name())),
            });
        }
    }
    if b.prefix_key == 0 {
        for k in KNOWN.iter().filter(|k| k.mods == b.mods && k.key == b.key) {
            if k.app == "Windows" || b.app_path.is_empty() || crate::win::file_name(&b.app_path).eq_ignore_ascii_case(k.app) {
                out.push(Issue { severity: Severity::Possible, text: ts(format!("{} is normally “{}” [{}].", b.combo(), k.name_en, k.app), format!("{} normalde “{}” [{}] kısayoludur.", b.combo(), k.name_tr, k.app)) });
            }
        }
    }
    out
}

fn exe_ok(p: &str) -> bool {
    crate::platform::is_app_path(p)
}

pub fn validate_step(s: &ActionStep, is_macro: bool) -> Option<String> {
    let need_app = |kind: &str| -> Option<String> {
        if s.app().is_empty() || !exe_ok(s.app()) {
            Some(ts(format!("{kind}: choose a registered application."), format!("{kind}: kayıtlı bir uygulama seçin.")))
        } else {
            None
        }
    };
    match s.kind.as_str() {
        "Key" => keys::sendable(s.mods, s.key).map(|e| e.to_string()),
        "Wait" => {
            if !(0..=600_000).contains(&s.delay) {
                Some(t("Wait must be 0–600000 ms.", "Bekleme 0–600000 ms olmalı.").into())
            } else {
                None
            }
        }
        "Text" | "Paste text" | "Paste files" | "Click button" | "Notify" | "Clipboard set" | "Clipboard append" => {
            if s.text.is_empty() || s.text.chars().count() > 100_000 {
                Some(t("Enter 1–100000 characters.", "1–100000 karakter girin.").into())
            } else {
                None
            }
        }
        "Open application" | "Focus" => need_app(&crate::rules::step_label(&s.kind)),
        "Open content" => {
            if s.text.trim().is_empty() {
                Some(t("Enter what to open.", "Açılacak şeyi girin.").into())
            } else {
                None
            }
        }
        "Menu command" => {
            if s.text.trim().is_empty() {
                Some(t("Enter the menu path, e.g. File › Save As.", "Menü yolunu girin, örn. Dosya › Farklı Kaydet.").into())
            } else {
                None
            }
        }
        "Click element" | "Read field" => {
            if s.text.is_empty() && s.automation_id.as_deref().unwrap_or("").is_empty() {
                Some(t("Enter the element name or automation id.", "Öğe adını veya otomasyon kimliğini girin.").into())
            } else {
                None
            }
        }
        "Set field" => {
            if s.automation_id.as_deref().unwrap_or("").is_empty() && s.path.as_deref().unwrap_or("").is_empty() {
                Some(t("Enter the field's automation id or name.", "Alanın otomasyon kimliğini veya adını girin.").into())
            } else {
                None
            }
        }
        "Write file" | "Read file" => {
            if s.path.as_deref().unwrap_or("").trim().is_empty() {
                Some(t("Enter a file path.", "Dosya yolu girin.").into())
            } else {
                None
            }
        }
        "Copy file" | "Move file" => {
            if s.path.as_deref().unwrap_or("").trim().is_empty() || s.text.trim().is_empty() {
                Some(t("Enter source and destination.", "Kaynak ve hedef girin.").into())
            } else {
                None
            }
        }
        "AI" => {
            if s.instruction.as_deref().unwrap_or("").trim().is_empty() {
                Some(t("Enter the AI instruction.", "AI talimatını girin.").into())
            } else {
                None
            }
        }
        "Set variable" => {
            if s.var.as_deref().unwrap_or("").trim().is_empty() {
                Some(t("Enter a variable name.", "Değişken adı girin.").into())
            } else {
                None
            }
        }
        "PowerShell" | "Terminal (cmd)" => {
            if s.text.trim().is_empty() {
                Some(t("The command is empty.", "Komut boş.").into())
            } else {
                None
            }
        }
        "System" => {
            if crate::system_actions::find(&s.text).is_none() {
                Some(t("Choose a Windows action.", "Bir Windows eylemi seçin.").into())
            } else {
                None
            }
        }
        "Window action" => {
            if !["minimize", "maximize", "restore", "close", "topmost", "untopmost"].contains(&s.text.as_str()) {
                Some(t("Choose a window action.", "Bir pencere eylemi seçin.").into())
            } else {
                None
            }
        }
        "Wait for window" | "Copy selection" => None,
        "Mouse click" | "Mouse drag" | "Scroll" => {
            if s.window_width <= 0.0 || s.window_height <= 0.0 || !s.x.is_finite() || !s.y.is_finite() {
                Some(t("Invalid recorded mouse step.", "Geçersiz kayıtlı fare adımı.").into())
            } else if s.kind == "Scroll" && s.scroll_delta == 0 {
                Some(t("Scroll amount cannot be zero.", "Kaydırma miktarı sıfır olamaz.").into())
            } else {
                let _ = is_macro;
                None
            }
        }
        other => Some(format!("{} {other}", t("Unsupported step:", "Desteklenmeyen adım:"))),
    }
}

pub fn validate(b: &Binding, all: &[Binding]) -> Option<String> {
    if b.name.trim().is_empty() {
        return Some(t("Enter a name for the shortcut.", "Kısayol için bir ad girin.").into());
    }
    if let Some(g) = &b.gesture {
        if let Some(e) = crate::gesture::validate(g) {
            return Some(e.into());
        }
        if b.enabled {
            if let Some((_, msg)) = crate::gesture::conflicts(b, all).into_iter().find(|(hard, _)| *hard) {
                return Some(msg);
            }
        }
    } else if let Some(r) = keys::reserved(b.mods, b.key) {
        // Switching a key off is allowed for Windows-key combinations too.
        let win_only = b.kind == "Disable key" && b.mods & keys::MOD_WIN != 0 && b.key != 0 && !keys::is_modifier(b.key);
        if !win_only {
            return Some(r.into());
        }
    }
    if b.gesture.is_none() && b.prefix_key > 0 {
        if !keys::prefix_allowed(b.prefix_key) || b.prefix_key == b.key {
            return Some(t("The first key must be Delete, Insert or F1–F24 (not F12) and differ from the main key.", "İlk tuş Delete, Insert veya F1–F24 (F12 hariç) olmalı ve ana tuştan farklı olmalı.").into());
        }
        if b.pass_through {
            return Some(t("Pass-through is not supported for two-key shortcuts.", "İki tuşlu kısayollarda tuşu uygulamaya da iletme desteklenmez.").into());
        }
    }
    if !b.app_path.is_empty() && !exe_ok(&b.app_path) {
        return Some(t("The scope must be an .exe application.", "Kapsam bir .exe uygulaması olmalı.").into());
    }
    if b.enabled {
        let c: Vec<String> = conflicts(b, all).into_iter().filter(|i| i.severity == Severity::Conflict).map(|i| i.text).collect();
        if !c.is_empty() {
            return Some(c.join("\n"));
        }
    }
    match b.kind.as_str() {
        "Workflow" => {
            let n = b.steps().len();
            let max = if b.is_macro { 5000 } else { 500 };
            if n == 0 || n > max {
                return Some(ts(format!("A workflow needs 1–{max} steps."), format!("İş akışı 1–{max} adım içermeli.")));
            }
            for (i, s) in b.steps().iter().enumerate() {
                if let Some(e) = validate_step(s, b.is_macro) {
                    return Some(ts(format!("Step {}: {e}", i + 1), format!("Adım {}: {e}", i + 1)));
                }
            }
            None
        }
        "Disable key" => None,
        "System" => crate::system_actions::find(&b.target).is_none().then(|| t("Choose a Windows action.", "Bir Windows eylemi seçin.").into()),
        "Send key" => keys::sendable(b.send_mods, b.send_key).map(|e| e.into()),
        "Media" => (!MEDIA.iter().any(|m| m.0 == b.target)).then(|| t("Choose a media action.", "Bir medya eylemi seçin.").into()),
        "Application" => (!exe_ok(&b.target)).then(|| t("Choose an .exe application.", "Bir .exe uygulaması seçin.").into()),
        "Browser website" => {
            if !crate::apps::is_browser(b.browser_path.as_deref().unwrap_or("")) {
                Some(t("Choose a supported browser.", "Desteklenen bir tarayıcı seçin.").into())
            } else if !(b.target.starts_with("http://") || b.target.starts_with("https://")) {
                Some(t("Enter an http(s) address.", "http(s) adresi girin.").into())
            } else {
                None
            }
        }
        "Web address" => (!(b.target.starts_with("http://") || b.target.starts_with("https://"))).then(|| t("Enter an http(s) address.", "http(s) adresi girin.").into()),
        "Spotify" => crate::apps::normalize_spotify(&b.target).is_none().then(|| t("Enter a valid Spotify link or URI.", "Geçerli bir Spotify bağlantısı veya URI girin.").into()),
        "File / folder" | "URI / deep link" | "PowerShell" | "Terminal (cmd)" => b.target.trim().is_empty().then(|| t("Fill in the action target.", "Eylem hedefini doldurun.").into()),
        other => Some(format!("{} {other}", t("Unknown action type:", "Bilinmeyen eylem türü:"))),
    }
}
