//! Registered applications: names, packaged-app identity, URI handlers and launching.
use crate::win::{file_name, file_stem, same_path};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub fn display_name(path: &str) -> String {
    if path.is_empty() {
        return String::new();
    }
    if path.ends_with(".desktop") {
        if let Ok(t) = std::fs::read_to_string(path) {
            if let Some(n) = t.lines().find(|l| l.starts_with("Name=")) {
                return n[5..].trim().to_string();
            }
        }
    }
    match file_name(path).to_lowercase().as_str() {
        "brave.exe" => "Brave".into(),
        "chrome.exe" => "Google Chrome".into(),
        "msedge.exe" => "Microsoft Edge".into(),
        "firefox.exe" => "Firefox".into(),
        "opera.exe" => "Opera".into(),
        "code.exe" => "Visual Studio Code".into(),
        "winword.exe" => "Word".into(),
        "excel.exe" => "Excel".into(),
        "powerpnt.exe" => "PowerPoint".into(),
        "outlook.exe" => "Outlook".into(),
        "notepad.exe" => "Notepad".into(),
        "explorer.exe" => "Explorer".into(),
        _ => file_stem(path),
    }
}

pub fn is_browser(path: &str) -> bool {
    matches!(file_name(path).to_lowercase().as_str(), "brave.exe" | "chrome.exe" | "msedge.exe" | "firefox.exe" | "opera.exe" | "vivaldi.exe")
}

#[derive(Clone, Debug, Default)]
pub struct AppDescriptor {
    pub name: String,
    /// AppUserModelID for packaged (Store/MSIX) apps.
    pub package_id: String,
    pub protocols: Vec<String>,
    pub protocol_args: HashMap<String, String>,
}

fn cache() -> &'static Mutex<HashMap<String, AppDescriptor>> {
    static C: OnceLock<Mutex<HashMap<String, AppDescriptor>>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(HashMap::new()))
}

/// All registered URL protocols and their open command, read once.
#[cfg(windows)]
fn url_protocols() -> &'static Vec<(String, String)> {
    static P: OnceLock<Vec<(String, String)>> = OnceLock::new();
    P.get_or_init(|| {
        use winreg::enums::HKEY_CLASSES_ROOT;
        let mut out = Vec::new();
        let root = winreg::RegKey::predef(HKEY_CLASSES_ROOT);
        for name in root.enum_keys().flatten() {
            if name.contains('.') || name.contains('\\') || name.starts_with('{') {
                continue;
            }
            let Ok(key) = root.open_subkey(&name) else { continue };
            if key.get_raw_value("URL Protocol").is_err() {
                continue;
            }
            let cmd: String = key.open_subkey("shell\\open\\command").and_then(|c| c.get_value("")).unwrap_or_default();
            if !cmd.is_empty() {
                out.push((name, cmd));
            }
        }
        out
    })
}

fn attr(tag: &str, name: &str) -> Option<String> {
    let needle = format!("{name}=\"");
    let i = tag.find(&needle)? + needle.len();
    let j = tag[i..].find('"')? + i;
    Some(tag[i..j].to_string())
}

pub fn describe(path: &str) -> AppDescriptor {
    if let Some(d) = cache().lock().unwrap().get(&path.to_lowercase()) {
        return d.clone();
    }
    let mut info = AppDescriptor { name: display_name(path), ..Default::default() };
    // Packaged apps: find AppxManifest.xml above the executable.
    let mut dir = std::path::Path::new(path).parent().map(|p| p.to_path_buf());
    for _ in 0..5 {
        let Some(d) = dir.clone() else { break };
        let manifest = d.join("AppxManifest.xml");
        if let Ok(xml) = std::fs::read_to_string(&manifest) {
            let ident_name = xml.find("<Identity").and_then(|i| attr(&xml[i..xml[i..].find('>').map(|j| i + j).unwrap_or(xml.len())], "Name"));
            let dir_name = d.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            let publisher_id = dir_name.rsplit('_').next().unwrap_or("").to_string();
            let mut pos = 0;
            while let Some(i) = xml[pos..].find("<Application ") {
                let start = pos + i;
                let end = xml[start..].find("</Application>").map(|j| start + j).unwrap_or(xml.len());
                let block = &xml[start..end];
                let head_end = block.find('>').unwrap_or(block.len());
                let head = &block[..head_end];
                let exe = attr(head, "Executable").unwrap_or_default();
                let matches = !exe.is_empty() && same_path(&d.join(&exe).display().to_string(), path) || block.contains(&format!("Executable=\"{}", exe)) && same_path(&file_name(&exe), &file_name(path));
                if matches {
                    if let (Some(n), Some(id)) = (ident_name.clone(), attr(head, "Id")) {
                        if !publisher_id.is_empty() {
                            info.package_id = format!("{n}_{publisher_id}!{id}");
                        }
                    }
                    let mut p2 = 0;
                    while let Some(k) = block[p2..].find("Protocol ") {
                        let s = p2 + k;
                        let e = block[s..].find('>').map(|j| s + j).unwrap_or(block.len());
                        if let Some(scheme) = attr(&block[s..e], "Name") {
                            if let Some(params) = attr(&block[s..e], "Parameters") {
                                info.protocol_args.insert(scheme.to_lowercase(), params);
                            }
                            if !info.protocols.contains(&scheme) {
                                info.protocols.push(scheme);
                            }
                        }
                        p2 = e;
                    }
                    break;
                }
                pos = end;
            }
            break;
        }
        dir = d.parent().map(|p| p.to_path_buf());
    }
    // Desktop URI handlers registered for this executable.
    #[allow(unused_variables)]
    let lower = path.to_lowercase();
    #[cfg(windows)]
    for (scheme, cmd) in url_protocols() {
        let lc = cmd.to_lowercase();
        if let Some(i) = lc.find(&lower) {
            let mut end = i + lower.len();
            if cmd.as_bytes().get(end) == Some(&b'"') {
                end += 1;
            }
            let args = cmd[end..].trim().to_string();
            if args.contains("%1") {
                info.protocol_args.insert(scheme.to_lowercase(), args);
            }
            if !info.protocols.contains(scheme) {
                info.protocols.push(scheme.clone());
            }
        }
    }
    info.protocols.sort();
    cache().lock().unwrap().insert(path.to_lowercase(), info.clone());
    info
}

/// Windows command-line quoting for one argument.
pub fn quote(value: &str) -> String {
    let mut out = String::from("\"");
    let mut slashes = 0;
    for c in value.chars() {
        if c == '\\' {
            slashes += 1;
            continue;
        }
        if c == '"' {
            out.push_str(&"\\".repeat(slashes * 2 + 1));
        } else {
            out.push_str(&"\\".repeat(slashes));
        }
        out.push(c);
        slashes = 0;
    }
    out.push_str(&"\\".repeat(slashes * 2));
    out.push('"');
    out
}

pub fn normalize_spotify(text: &str) -> Option<String> {
    let t = text.trim();
    let re = regex::Regex::new(r"^spotify:(track|album|playlist|artist|episode|show):[A-Za-z0-9]+$").unwrap();
    if re.is_match(t) {
        return Some(t.to_string());
    }
    let web = regex::Regex::new(r"^https?://open\.spotify\.com/(?:intl-[a-z-]+/)?(track|album|playlist|artist|episode|show)/([A-Za-z0-9]+)").unwrap();
    web.captures(t).map(|c| format!("spotify:{}:{}", &c[1], &c[2]))
}

fn resource_for(path: &str, resource: &str) -> String {
    let r = resource.trim();
    if file_name(path).eq_ignore_ascii_case("spotify.exe") {
        if let Some(s) = normalize_spotify(r) {
            return s;
        }
    }
    r.to_string()
}

fn scheme_of(value: &str) -> Option<String> {
    let i = value.find(':')?;
    let s = &value[..i];
    if s.len() > 1 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.') {
        Some(s.to_lowercase())
    } else {
        None
    }
}

#[cfg(windows)]
fn spawn(path: &str, raw_args: &str) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    let dir = std::path::Path::new(path).parent().map(|p| p.to_path_buf()).unwrap_or_default();
    std::process::Command::new(path).raw_arg(raw_args).current_dir(dir).spawn().map(|_| ()).map_err(|e| format!("{}: {e}", display_name(path)))
}

/// Split a command line into words (double/single quotes, backslash escapes).
pub fn split_args(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let (mut q, mut any) = (None::<char>, false);
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        match (q, c) {
            (None, ' ' | '\t') => {
                if any {
                    out.push(std::mem::take(&mut cur));
                    any = false;
                }
            }
            (None, '"' | '\'') => {
                q = Some(c);
                any = true;
            }
            (Some(qc), _) if c == qc => q = None,
            (_, '\\') if q != Some('\'') => {
                if let Some(n) = it.next() {
                    cur.push(n);
                    any = true;
                }
            }
            _ => {
                cur.push(c);
                any = true;
            }
        }
    }
    if any {
        out.push(cur);
    }
    out
}

/// Linux: executables, or .desktop launchers (Exec= line with %u/%f filled in).
#[cfg(not(windows))]
fn spawn(path: &str, raw_args: &str) -> Result<(), String> {
    let mut args = split_args(raw_args);
    let (program, mut words) = if path.ends_with(".desktop") {
        let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let exec = text.lines().find(|l| l.starts_with("Exec=")).map(|l| l[5..].to_string()).ok_or("Exec= missing")?;
        let mut w = split_args(&exec);
        let mut filled = false;
        for x in w.iter_mut() {
            if ["%u", "%U", "%f", "%F"].contains(&x.as_str()) {
                *x = args.first().cloned().unwrap_or_default();
                filled = true;
            }
        }
        w.retain(|x| !x.is_empty() && !(x.starts_with('%') && x.len() == 2));
        if filled {
            args.clear();
        }
        let p = w.remove(0);
        (p, w)
    } else {
        (path.to_string(), Vec::new())
    };
    words.extend(args);
    std::process::Command::new(&program).args(&words).spawn().map(|_| ()).map_err(|e| format!("{}: {e}", display_name(path)))
}

/// Launch an application, optionally with a resource (file, URL, URI) or raw arguments.
pub fn open(path: &str, resource: &str, arguments: &str) -> Result<(), String> {
    use crate::i18n::t;
    let value = resource_for(path, resource);
    let desc = describe(path);
    if !desc.package_id.is_empty() {
        // URIs for packaged apps are routed by the shell to the right package.
        if let Some(scheme) = scheme_of(&value) {
            if desc.protocols.iter().any(|p| p.eq_ignore_ascii_case(&scheme)) {
                return crate::win::shell_open(&value, "", "");
            }
        }
        let args = if value.is_empty() { arguments.to_string() } else { quote(&value) };
        return crate::uia::activate_package(&desc.package_id, &args);
    }
    if !std::path::Path::new(path).exists() {
        return Err(format!("{}: {path}", t("Registered application not found", "Kayıtlı uygulama bulunamadı")));
    }
    if let Some(scheme) = scheme_of(&value) {
        if let Some(template) = desc.protocol_args.get(&scheme) {
            let args = template.replace("\"%1\"", &quote(&value)).replace("%1", &quote(&value));
            return spawn(path, &args);
        }
    }
    let args = if value.is_empty() { arguments.to_string() } else { quote(&value) };
    spawn(path, &args)
}

pub fn open_browser(browser: &str, url: &str) -> Result<(), String> {
    spawn(browser, &quote(url))
}

/// Launch (if needed) and return the app's window, waiting up to `timeout_ms`.
pub fn ensure_window(path: &str, title: Option<&str>, class: Option<&str>, timeout_ms: u64, launch: bool, cancel: &dyn Fn() -> bool) -> Result<isize, String> {
    use crate::i18n::t;
    let pick = || {
        let mut wins = crate::win::app_windows(path);
        if let Some(c) = class.filter(|c| !c.is_empty()) {
            wins.retain(|&w| crate::win::window_class(w) == c);
        }
        if let Some(tt) = title.filter(|t| !t.is_empty()) {
            let exact: Vec<isize> = wins.iter().copied().filter(|&w| crate::win::window_title(w) == tt).collect();
            if !exact.is_empty() {
                return exact.first().copied();
            }
        }
        let fg = crate::win::foreground();
        if wins.contains(&fg) {
            return Some(fg);
        }
        wins.first().copied()
    };
    if let Some(w) = pick() {
        return Ok(w);
    }
    if launch {
        open(path, "", "")?;
    }
    let start = std::time::Instant::now();
    while start.elapsed().as_millis() < timeout_ms as u128 {
        if cancel() {
            return Err(t("Cancelled.", "İptal edildi.").into());
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
        if let Some(w) = pick() {
            // Give the new window a moment to finish initialising.
            if launch {
                std::thread::sleep(std::time::Duration::from_millis(400));
            }
            return Ok(w);
        }
    }
    Err(format!("{}: {}", t("Application window not found", "Uygulama penceresi bulunamadı"), display_name(path)))
}

pub fn hidden_command(program: &str) -> std::process::Command {
    #[allow(unused_mut)]
    let mut c = std::process::Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(CREATE_NO_WINDOW);
    }
    c
}
