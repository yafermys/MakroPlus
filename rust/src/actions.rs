//! Runs saved shortcuts: simple actions and multi-app workflows with variables.
use crate::i18n::{t, ts};
use crate::model::{ActionStep, Binding};
use crate::win::{self, Hwnd};
use std::collections::HashMap;
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

pub static WORKFLOW_RUNNING: AtomicBool = AtomicBool::new(false);
pub static CANCEL: AtomicBool = AtomicBool::new(false);

/// Add an entry to history (any thread).
pub fn log(name: &str, result: &str) {
    {
        let mut st = crate::app_state().lock();
        st.log(name, result);
    }
    crate::request_save();
    crate::send_ui(crate::UiEvent::Status(format!("{name} • {result}")));
}

/// Run a binding off the hook thread.
pub fn submit(b: Binding, window: Hwnd) {
    std::thread::spawn(move || {
        crate::uia::com_init();
        let name = b.name.clone();
        let r = run_binding(&b, window);
        match r {
            Ok(msg) => log(&name, &msg),
            Err(e) => log(&name, &format!("{} • {e}", t("ERROR", "HATA"))),
        }
    });
}

/// Run a shortcut immediately from the Makro+ window. Steps that type or click
/// go to the shortcut's own app, or to the window used just before Makro+.
pub fn run_now(b: Binding) {
    std::thread::spawn(move || {
        crate::uia::com_init();
        let needs_window = matches!(b.kind.as_str(), "Workflow" | "Send key");
        let mut window = win::foreground();
        if needs_window {
            let target = if !b.app_path.is_empty() {
                crate::apps::ensure_window(&b.app_path, None, None, 10_000, true, &|| false).ok()
            } else {
                Some(crate::engine::last_foreground()).filter(|&w| w != 0 && win::is_window(w))
            };
            if let Some(w) = target {
                win::activate(w);
                std::thread::sleep(Duration::from_millis(180));
                window = w;
            }
        }
        let name = b.name.clone();
        match run_binding(&b, window) {
            Ok(msg) => log(&name, &msg),
            Err(e) => log(&name, &format!("{} • {e}", t("ERROR", "HATA"))),
        }
    });
}

fn media_key(name: &str) -> Option<i32> {
    Some(match name {
        "Play / pause" => 179,
        "Next track" => 176,
        "Previous track" => 177,
        "Volume up" => 175,
        "Volume down" => 174,
        "Mute / unmute" => 173,
        _ => return None,
    })
}

pub fn run_binding(b: &Binding, window: Hwnd) -> Result<String, String> {
    let sent = t("Request sent.", "İstek gönderildi.").to_string();
    match b.kind.as_str() {
        "Workflow" => {
            if WORKFLOW_RUNNING.swap(true, Ordering::SeqCst) {
                return Ok(t("Another workflow is running; this trigger was skipped.", "Başka bir iş akışı çalışıyor; bu tetikleme atlandı.").into());
            }
            CANCEL.store(false, Ordering::SeqCst);
            let mut ctx = Ctx::new(window);
            let r = ctx.run_steps(b.steps());
            WORKFLOW_RUNNING.store(false, Ordering::SeqCst);
            r.map(|_| {
                let mut msg = ts(format!("Workflow finished • {} steps", b.steps().len()), format!("İş akışı tamamlandı • {} adım", b.steps().len()));
                if !ctx.notes.is_empty() {
                    msg += " • ";
                    msg += &ctx.notes.join(" • ");
                }
                msg
            })
        }
        "Disable key" => Ok(t("This key is switched off.", "Bu tuş kapalı.").into()),
        "Paste files" => Ok(String::new()),
        "System" => crate::system_actions::run(&b.target, &crate::send_ui),
        "Send key" => {
            if win::foreground() != window {
                return Err(t("Foreground window changed; the key was not sent.", "Ön plandaki pencere değişti; tuş gönderilmedi.").into());
            }
            win::send_chord(b.send_mods, b.send_key, false).map(|_| sent)
        }
        "Media" => {
            let k = media_key(&b.target).ok_or_else(|| t("Unknown media action.", "Bilinmeyen medya eylemi.").to_string())?;
            win::send_chord(0, k, false).map(|_| sent)
        }
        "Application" => crate::apps::open(&b.target, "", &b.arguments).map(|_| ts(format!("Opened {}", crate::apps::display_name(&b.target)), format!("{} açıldı", crate::apps::display_name(&b.target)))),
        "Browser website" => crate::apps::open_browser(b.browser_path.as_deref().unwrap_or(""), &b.target).map(|_| sent),
        "Spotify" => {
            let uri = crate::apps::normalize_spotify(&b.target).ok_or_else(|| t("Enter a valid Spotify link or URI.", "Geçerli bir Spotify bağlantısı veya URI girin.").to_string())?;
            win::shell_open(&uri, "", "").map(|_| sent)
        }
        "PowerShell" => run_powershell(&b.target, &|| false).and_then(|(code, out)| if code == 0 { Ok(finish_output(&out)) } else { Err(format!("exit {code}: {}", crate::model::short(&out, 200))) }),
        "Terminal (cmd)" => run_cmd(&b.target, &|| false).and_then(|(code, out)| if code == 0 { Ok(finish_output(&out)) } else { Err(format!("exit {code}: {}", crate::model::short(&out, 200))) }),
        "File / folder" | "Web address" | "URI / deep link" => win::shell_open(&b.target, "", "").map(|_| sent),
        other => Err(format!("{} {other}", t("Unsupported action:", "Desteklenmeyen eylem:"))),
    }
}

fn finish_output(out: &str) -> String {
    let o = out.trim();
    if o.is_empty() {
        t("Command completed • exit code 0", "Komut tamamlandı • çıkış kodu 0").into()
    } else {
        format!("{} • {}", t("Command completed", "Komut tamamlandı"), crate::model::short(o, 160))
    }
}

fn wait_child(mut child: std::process::Child, cancel: &dyn Fn() -> bool) -> Result<(i32, String), String> {
    let mut stdout = child.stdout.take();
    let reader = std::thread::spawn(move || {
        let mut s = Vec::new();
        if let Some(o) = stdout.as_mut() {
            let _ = o.read_to_end(&mut s);
        }
        s
    });
    loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            let bytes = reader.join().unwrap_or_default();
            let text = String::from_utf8(bytes.clone()).unwrap_or_else(|_| bytes.iter().map(|&b| b as char).collect());
            return Ok((status.code().unwrap_or(-1), text));
        }
        if cancel() {
            let _ = child.kill();
            return Err(t("Cancelled.", "İptal edildi.").into());
        }
        std::thread::sleep(Duration::from_millis(40));
    }
}

#[cfg(not(windows))]
pub fn run_powershell(script: &str, cancel: &dyn Fn() -> bool) -> Result<(i32, String), String> {
    let child = std::process::Command::new("pwsh")
        .args(["-NoLogo", "-NoProfile", "-NonInteractive", "-Command", script])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .stdin(std::process::Stdio::null())
        .spawn()
        .map_err(|_| t("PowerShell (pwsh) is not installed. Use a shell command step instead.", "PowerShell (pwsh) kurulu değil. Bunun yerine kabuk komutu adımı kullanın.").to_string())?;
    wait_child(child, cancel)
}

/// Linux: shell commands run with `sh -c`.
#[cfg(not(windows))]
pub fn run_cmd(command: &str, cancel: &dyn Fn() -> bool) -> Result<(i32, String), String> {
    let child = std::process::Command::new("sh")
        .args(["-c", command])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .stdin(std::process::Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;
    wait_child(child, cancel)
}

#[cfg(windows)]
pub fn run_powershell(script: &str, cancel: &dyn Fn() -> bool) -> Result<(i32, String), String> {
    let utf16: Vec<u8> = script.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
    let root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into());
    let exe = format!("{root}\\System32\\WindowsPowerShell\\v1.0\\powershell.exe");
    let prelude = "[Console]::OutputEncoding=[Text.Encoding]::UTF8;";
    let full: Vec<u8> = prelude.encode_utf16().flat_map(|u| u.to_le_bytes()).chain(utf16).collect();
    let child = crate::apps::hidden_command(&exe)
        .args(["-NoLogo", "-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-EncodedCommand", &win::b64_encode(&full)])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .stdin(std::process::Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;
    wait_child(child, cancel)
}

#[cfg(windows)]
pub fn run_cmd(command: &str, cancel: &dyn Fn() -> bool) -> Result<(i32, String), String> {
    use std::os::windows::process::CommandExt;
    let root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into());
    let child = crate::apps::hidden_command(&format!("{root}\\System32\\cmd.exe"))
        .raw_arg(format!("/d /s /c \"chcp 65001 >nul & {command}\""))
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .stdin(std::process::Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;
    wait_child(child, cancel)
}

/// Expand %VAR% environment references in a path.
fn expand_env(s: &str) -> String {
    let re = regex::Regex::new(r"%([A-Za-z_][A-Za-z0-9_()]*)%").unwrap();
    re.replace_all(s, |c: &regex::Captures| std::env::var(&c[1]).unwrap_or_else(|_| c[0].to_string())).into_owned()
}

pub struct Ctx {
    pub target: Hwnd,
    pub vars: HashMap<String, String>,
    pub notes: Vec<String>,
    interval: u64,
}

impl Ctx {
    pub fn new(window: Hwnd) -> Ctx {
        let interval = crate::app_state().lock().step_interval_ms.unwrap_or(15).max(0) as u64;
        Ctx { target: window, vars: HashMap::new(), notes: Vec::new(), interval }
    }

    fn cancelled(&self) -> Result<(), String> {
        if CANCEL.load(Ordering::SeqCst) {
            Err(t("Workflow cancelled (Esc). Steps already sent cannot be undone.", "İş akışı iptal edildi (Esc). Gönderilen adımlar geri alınamaz.").into())
        } else {
            Ok(())
        }
    }

    fn sleep(&self, ms: u64) -> Result<(), String> {
        let end = Instant::now() + Duration::from_millis(ms);
        while Instant::now() < end {
            self.cancelled()?;
            std::thread::sleep(Duration::from_millis(30.min(ms.max(1))));
        }
        Ok(())
    }

    /// Replace {name} placeholders. `{{` stays a literal brace.
    pub fn expand(&self, s: &str) -> String {
        if !s.contains('{') {
            return s.to_string();
        }
        let marker = "\u{1}";
        let s = s.replace("{{", marker);
        let re = regex::Regex::new(r"\{([A-Za-z_][A-Za-z0-9_.:\-]*)\}").unwrap();
        let out = re.replace_all(&s, |c: &regex::Captures| {
            let name = &c[1];
            if let Some(v) = self.vars.get(name) {
                return v.clone();
            }
            let now = chrono::Local::now();
            match name {
                "clipboard" => crate::clipboard::get_text().unwrap_or_default(),
                "date" => now.format("%Y-%m-%d").to_string(),
                "time" => now.format("%H:%M:%S").to_string(),
                "datetime" => now.format("%Y-%m-%d %H:%M").to_string(),
                "timestamp" => now.format("%Y%m%d-%H%M%S").to_string(),
                "app" => win::file_stem(&win::window_process_path(self.target)),
                "title" => win::window_title(self.target),
                "newline" => "\n".into(),
                "tab" => "\t".into(),
                "desktop" => std::env::var("USERPROFILE").map(|p| format!("{p}\\Desktop")).unwrap_or_default(),
                "documents" => std::env::var("USERPROFILE").map(|p| format!("{p}\\Documents")).unwrap_or_default(),
                "downloads" => std::env::var("USERPROFILE").map(|p| format!("{p}\\Downloads")).unwrap_or_default(),
                n if n.starts_with("env:") => std::env::var(&n[4..]).unwrap_or_default(),
                _ => c[0].to_string(),
            }
        });
        out.replace(marker, "{")
    }

    fn path(&self, s: &str) -> String {
        expand_env(&self.expand(s.trim()))
    }

    fn ensure_target(&self, step: &ActionStep) -> Result<(), String> {
        if self.target == 0 || win::foreground() != self.target {
            return Err(t(
                "The foreground window changed; remaining steps were cancelled. Add a Focus step before typing into another app.",
                "Ön plandaki pencere değişti; kalan adımlar iptal edildi. Başka bir uygulamaya yazmadan önce Öne getir adımı ekleyin.",
            )
            .into());
        }
        if !step.app().is_empty() && !win::same_path(step.app(), &win::window_process_path(self.target)) {
            return Err(ts(
                format!("{} is not in the foreground. Add a Focus step first.", crate::apps::display_name(step.app())),
                format!("{} ön planda değil. Önce Öne getir adımı ekleyin.", crate::apps::display_name(step.app())),
            ));
        }
        Ok(())
    }

    fn set_var(&mut self, step: &ActionStep, default: &str, value: String) {
        let name = step.var.clone().filter(|v| !v.trim().is_empty()).unwrap_or_else(|| default.to_string());
        self.vars.insert(name, value);
    }

    fn deliver(&mut self, step: &ActionStep, default_var: &str, value: String) -> Result<(), String> {
        self.set_var(step, default_var, value.clone());
        match step.output.as_deref().unwrap_or("var") {
            "type" => {
                self.ensure_target(&ActionStep::default())?;
                win::type_text(&value)
            }
            "paste" => self.paste(&value, true),
            "clipboard" => {
                crate::clipboard::set_text(&value)?;
                Ok(())
            }
            "notify" => {
                crate::shell::notify("Makro+", &value);
                Ok(())
            }
            "file" => {
                let p = self.path(step.path.as_deref().unwrap_or(""));
                write_file(&p, &value, step.append)
            }
            _ => Ok(()),
        }
    }

    fn paste(&mut self, text: &str, restore: bool) -> Result<(), String> {
        self.ensure_target(&ActionStep::default())?;
        let previous = if restore { crate::clipboard::get_text() } else { None };
        crate::shell::suppress_clipboard(1500);
        crate::clipboard::set_text(text)?;
        std::thread::sleep(Duration::from_millis(30));
        win::send_chord(2, 86, false)?;
        if let Some(prev) = previous {
            std::thread::sleep(Duration::from_millis(350));
            crate::shell::suppress_clipboard(800);
            let _ = crate::clipboard::set_text(&prev);
        }
        Ok(())
    }

    fn focus(&mut self, step: &ActionStep, launch: bool, timeout: u64) -> Result<(), String> {
        let app = step.app();
        if app.is_empty() {
            return Err(t("Choose the application to focus.", "Öne getirilecek uygulamayı seçin.").into());
        }
        let w = crate::apps::ensure_window(app, step.window_title.as_deref(), step.window_class.as_deref(), timeout, launch, &|| CANCEL.load(Ordering::SeqCst))?;
        if !win::activate(w) {
            std::thread::sleep(Duration::from_millis(120));
            win::activate(w);
        }
        std::thread::sleep(Duration::from_millis(120));
        self.target = win::foreground();
        if self.target != w {
            self.target = w;
            if !win::activate(w) {
                return Err(ts(
                    format!("{} could not be brought to the foreground.", crate::apps::display_name(app)),
                    format!("{} öne getirilemedi.", crate::apps::display_name(app)),
                ));
            }
        }
        Ok(())
    }

    pub fn run_steps(&mut self, steps: &[ActionStep]) -> Result<(), String> {
        for (i, step) in steps.iter().enumerate() {
            self.cancelled()?;
            self.run_step(step).map_err(|e| ts(format!("Step {} ({}): {e}", i + 1, step.kind), format!("Adım {} ({}): {e}", i + 1, step.kind)))?;
            if i + 1 < steps.len() && step.kind != "Wait" && self.interval > 0 {
                self.sleep(self.interval)?;
            }
        }
        Ok(())
    }

    fn run_step(&mut self, s: &ActionStep) -> Result<(), String> {
        match s.kind.as_str() {
            "Wait" => self.sleep(s.delay.max(0) as u64),
            "Key" => {
                self.ensure_target(s)?;
                win::send_chord(s.mods, s.key, s.alt_gr)
            }
            "Text" => {
                self.ensure_target(s)?;
                let text = self.expand(&s.text);
                win::type_text(&text)
            }
            "Paste text" => {
                let text = self.expand(&s.text);
                let keep = s.output.as_deref() == Some("keep");
                self.ensure_target(s)?;
                self.paste(&text, !keep)
            }
            "Paste files" => {
                self.ensure_target(s)?;
                let files: Vec<String> = s.text.lines().map(|l| self.path(l)).filter(|l| !l.is_empty()).collect();
                crate::shell::suppress_clipboard(1500);
                crate::clipboard::set_files(&files)?;
                std::thread::sleep(Duration::from_millis(30));
                win::send_chord(2, 86, false)
            }
            "Click button" => {
                self.ensure_target(s)?;
                crate::uia::click_element(self.target, "", &self.expand(&s.text), 0)
            }
            "Click element" => {
                self.ensure_target(s)?;
                crate::uia::click_element(self.target, s.automation_id.as_deref().unwrap_or(""), &self.expand(&s.text), s.control_type)
            }
            "Set field" => {
                self.ensure_target(s)?;
                let value = self.expand(&s.text);
                crate::uia::set_field(self.target, s.automation_id.as_deref().unwrap_or(""), &self.expand(s.path.as_deref().unwrap_or("")), &value)
            }
            "Read field" => {
                let v = crate::uia::read_field(self.target, s.automation_id.as_deref().unwrap_or(""), &self.expand(&s.text))?;
                self.set_var(s, "field", v);
                Ok(())
            }
            "Menu command" => {
                if !s.app().is_empty() && !win::same_path(s.app(), &win::window_process_path(self.target)) {
                    self.focus(s, false, 6000)?;
                }
                crate::uia::menu_command(self.target, &s.text)
            }
            "Open application" => crate::apps::open(s.app(), "", &self.expand(&s.text)),
            "Open content" => {
                let r = self.path(&s.text);
                if s.app().is_empty() {
                    win::shell_open(&r, "", "")
                } else {
                    crate::apps::open(s.app(), &r, "")
                }
            }
            "Focus" => self.focus(s, true, if s.delay > 0 { s.delay as u64 } else { 15000 }),
            "Wait for window" => {
                let needle = self.expand(&s.text).to_lowercase();
                let timeout = if s.delay > 0 { s.delay as u64 } else { 10000 };
                let start = Instant::now();
                loop {
                    self.cancelled()?;
                    let fg = win::foreground();
                    let candidates: Vec<Hwnd> = std::iter::once(fg).chain(win::top_windows()).collect();
                    let found = candidates.into_iter().find(|&w| {
                        (s.app().is_empty() || win::same_path(s.app(), &win::window_process_path(w)))
                            && (needle.is_empty() || win::window_title(w).to_lowercase().contains(&needle))
                            && s.window_class.as_deref().map(|c| c.is_empty() || win::window_class(w) == c).unwrap_or(true)
                    });
                    if let Some(w) = found {
                        win::activate(w);
                        std::thread::sleep(Duration::from_millis(100));
                        self.target = w;
                        return Ok(());
                    }
                    if start.elapsed().as_millis() as u64 > timeout {
                        return Err(ts(format!("Window “{}” did not appear in time.", s.text), format!("“{}” penceresi zamanında açılmadı.", s.text)));
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
            }
            "Window action" => {
                let w = if !s.app().is_empty() { win::app_windows(s.app()).first().copied().unwrap_or(self.target) } else { self.target };
                match s.text.to_lowercase().as_str() {
                    "minimize" => win::minimize(w),
                    "maximize" => win::maximize(w),
                    "restore" => win::restore(w),
                    "close" => {
                        win::post_close(w);
                    }
                    "topmost" => win::set_topmost(w, true),
                    "untopmost" | "normal" => win::set_topmost(w, false),
                    other => return Err(format!("{} {other}", t("Unknown window action:", "Bilinmeyen pencere eylemi:"))),
                }
                Ok(())
            }
            "PowerShell" | "Terminal (cmd)" => {
                let script = self.expand(&s.text);
                let (code, out) = if s.kind == "PowerShell" { run_powershell(&script, &|| CANCEL.load(Ordering::SeqCst))? } else { run_cmd(&script, &|| CANCEL.load(Ordering::SeqCst))? };
                let out = out.trim_end().to_string();
                self.vars.insert("last_output".into(), out.clone());
                if let Some(v) = s.var.as_ref().filter(|v| !v.is_empty()) {
                    self.vars.insert(v.clone(), out.clone());
                }
                if code != 0 {
                    return Err(format!("exit {code}: {}", crate::model::short(&out, 200)));
                }
                Ok(())
            }
            "System" => crate::system_actions::run(&s.text, &crate::send_ui).map(|m| self.notes.push(m)),
            "Copy selection" => {
                self.ensure_target(s)?;
                let before = crate::clipboard::sequence();
                win::send_chord(2, 67, false)?;
                let start = Instant::now();
                while crate::clipboard::sequence() == before && start.elapsed() < Duration::from_millis(1500) {
                    std::thread::sleep(Duration::from_millis(25));
                }
                if crate::clipboard::sequence() == before {
                    self.notes.push(t("Nothing was selected to copy.", "Kopyalanacak seçim yoktu.").into());
                    self.set_var(s, "selection", String::new());
                } else {
                    std::thread::sleep(Duration::from_millis(30));
                    let text = crate::clipboard::get_text().unwrap_or_default();
                    self.set_var(s, "selection", text);
                }
                Ok(())
            }
            "Clipboard set" => crate::clipboard::set_text(&self.expand(&s.text)),
            "Clipboard append" => {
                let cur = crate::clipboard::get_text().unwrap_or_default();
                let add = self.expand(&s.text);
                let sep = if cur.is_empty() || cur.ends_with('\n') { "" } else { "\n" };
                crate::clipboard::set_text(&format!("{cur}{sep}{add}"))
            }
            "Write file" => {
                let p = self.path(s.path.as_deref().unwrap_or(""));
                write_file(&p, &self.expand(&s.text), s.append)
            }
            "Read file" => {
                let p = self.path(s.path.as_deref().unwrap_or(&s.text));
                let data = std::fs::read(&p).map_err(|e| format!("{p}: {e}"))?;
                if data.len() > 50 * 1024 * 1024 {
                    return Err(t("File is larger than 50 MB.", "Dosya 50 MB'tan büyük.").into());
                }
                self.set_var(s, "file", String::from_utf8_lossy(&data).into_owned());
                Ok(())
            }
            "Copy file" | "Move file" => {
                let src = self.path(s.path.as_deref().unwrap_or(""));
                let mut dst = std::path::PathBuf::from(self.path(&s.text));
                if src.is_empty() || dst.as_os_str().is_empty() {
                    return Err(t("Source and destination are required.", "Kaynak ve hedef gerekli.").into());
                }
                // Wait briefly for a file that an app is still writing (after Save).
                let start = Instant::now();
                while !std::path::Path::new(&src).exists() && start.elapsed() < Duration::from_secs(8) {
                    self.sleep(150)?;
                }
                if dst.is_dir() || s.text.ends_with('\\') || s.text.ends_with('/') {
                    std::fs::create_dir_all(&dst).map_err(|e| e.to_string())?;
                    dst = dst.join(std::path::Path::new(&src).file_name().unwrap_or_default());
                } else if let Some(parent) = dst.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                if s.kind == "Copy file" {
                    std::fs::copy(&src, &dst).map(|_| ()).map_err(|e| format!("{src}: {e}"))?;
                } else if std::fs::rename(&src, &dst).is_err() {
                    std::fs::copy(&src, &dst).map_err(|e| format!("{src}: {e}"))?;
                    std::fs::remove_file(&src).map_err(|e| e.to_string())?;
                }
                self.vars.insert("last_file".into(), dst.display().to_string());
                Ok(())
            }
            "AI" => {
                let instruction = self.expand(s.instruction.as_deref().unwrap_or(""));
                let input = self.expand(if s.text.trim().is_empty() { "{clipboard}" } else { &s.text });
                let result = crate::llm::complete_text(&instruction, &input)?;
                self.deliver(s, "ai", result)
            }
            "Notify" => {
                crate::shell::notify("Makro+", &self.expand(&s.text));
                Ok(())
            }
            "Set variable" => {
                let v = self.expand(&s.text);
                self.set_var(s, "value", v);
                Ok(())
            }
            "Mouse click" | "Mouse drag" | "Scroll" => {
                self.ensure_target(s)?;
                let r = win::window_rect(self.target).ok_or("window rect")?;
                let (w, h) = ((r.2 - r.0) as f64, (r.3 - r.1) as f64);
                let (sx, sy) = if s.scale_with_window && s.window_width > 0.0 && s.window_height > 0.0 {
                    (w / s.window_width, h / s.window_height)
                } else {
                    if (w - s.window_width).abs() > 2.0 || (h - s.window_height).abs() > 2.0 {
                        return Err(t("The window size changed since recording. Restore it or record again.", "Pencere boyutu kayıttan sonra değişti. Eski boyuta getirin veya yeniden kaydedin.").into());
                    }
                    (1.0, 1.0)
                };
                let px = |x: f64| r.0 + (x * sx).round() as i32;
                let py = |y: f64| r.1 + (y * sy).round() as i32;
                win::mouse_at(&s.kind, s.mouse_button, px(s.x), py(s.y), px(s.x2), py(s.y2), s.scroll_delta, s.horizontal, s.duration.max(0) as u64)
            }
            other => Err(format!("{} {other}", t("Unsupported step:", "Desteklenmeyen adım:"))),
        }
    }
}

pub fn write_file(path: &str, text: &str, append: bool) -> Result<(), String> {
    use std::io::Write;
    if path.trim().is_empty() {
        return Err(t("File path is empty.", "Dosya yolu boş.").into());
    }
    if let Some(parent) = std::path::Path::new(path).parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut f = std::fs::OpenOptions::new().create(true).append(append).write(true).truncate(!append).open(path).map_err(|e| format!("{path}: {e}"))?;
    let mut data = text.to_string();
    if append && !data.ends_with('\n') {
        data.push('\n');
    }
    f.write_all(data.as_bytes()).map_err(|e| e.to_string())
}
