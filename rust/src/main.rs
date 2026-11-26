#![cfg_attr(windows, windows_subsystem = "windows")]
//! Makro+ 3 — keyboard, mouse and multi-app automation for Windows.

mod actions;
mod apps;
mod background;
mod clipstore;
mod engine;
mod gesture;
mod i18n;
mod keys;
mod llm;
mod model;
mod platform;
mod rules;
mod storage;
mod system_actions;
mod theme;
mod ui;
mod updates;

// Platform layers with the same public API on every OS.
#[cfg(windows)]
mod clipboard;
#[cfg(windows)]
mod guide;
#[cfg(windows)]
mod installer;
#[cfg(windows)]
mod shell;
#[cfg(windows)]
mod uia;
#[cfg(windows)]
mod win;

#[cfg(not(windows))]
#[path = "linux/clipboard.rs"]
mod clipboard;
#[cfg(not(windows))]
#[path = "linux/guide.rs"]
mod guide;
#[cfg(not(windows))]
#[path = "linux/input.rs"]
mod input;
#[cfg(not(windows))]
#[path = "linux/installer.rs"]
mod installer;
#[cfg(not(windows))]
#[path = "linux/shell.rs"]
mod shell;
#[cfg(not(windows))]
#[path = "linux/uia.rs"]
mod uia;
#[cfg(not(windows))]
#[path = "linux/win.rs"]
mod win;
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::OnceLock;

pub const APP_TITLE: &str = "Makro+";

/// Messages from background threads to the window.
pub enum UiEvent {
    Status(String),
    Captured { mods: i32, key: i32, prefix: i32 },
    Recorded(model::ActionStep),
    ClipboardChanged,
    ShowPage(String),
    TogglePause,
    Shown,
    Exit,
    Plan(Result<llm::PlanResult, String>),
    Progress(String),
    Scan(Result<model::AppInventory, String>),
    AiText(String, Result<String, String>),
    UpdateChecked,
    GestureDevice(String),
    GestureStatus(String),
    GestureTrace(Vec<model::GesturePoint>),
    GestureRecorded(Vec<model::GesturePoint>),
}

static STATE: OnceLock<Mutex<model::State>> = OnceLock::new();
static UI_TX: OnceLock<Mutex<Sender<UiEvent>>> = OnceLock::new();
static CTX: OnceLock<eframe::egui::Context> = OnceLock::new();
static SAVE_PENDING: AtomicBool = AtomicBool::new(false);
static SAVE_SIGNAL: (std::sync::Mutex<bool>, std::sync::Condvar) = (std::sync::Mutex::new(false), std::sync::Condvar::new());

/// Tests share one UI event channel; holding this lock serialises them.
#[cfg(test)]
pub fn test_events() -> parking_lot::MutexGuard<'static, Receiver<UiEvent>> {
    static R: OnceLock<Mutex<Receiver<UiEvent>>> = OnceLock::new();
    R.get_or_init(|| {
        let (tx, rx) = channel();
        let _ = UI_TX.set(Mutex::new(tx));
        let _ = STATE.set(Mutex::new(model::State::default()));
        Mutex::new(rx)
    })
    .lock()
}

pub fn ctx() -> Option<&'static eframe::egui::Context> {
    CTX.get()
}

pub fn app_state() -> &'static Mutex<model::State> {
    STATE.get().expect("state")
}

pub fn send_ui(e: UiEvent) {
    if let Some(tx) = UI_TX.get() {
        let _ = tx.lock().send(e);
    }
    if let Some(ctx) = CTX.get() {
        ctx.request_repaint();
    }
}

/// Debounced background save of the settings.
pub fn request_save() {
    SAVE_PENDING.store(true, Ordering::SeqCst);
    let (lock, cv) = &SAVE_SIGNAL;
    *lock.lock().unwrap() = true;
    cv.notify_one();
}

pub fn save_now() -> Result<(), String> {
    let data = storage::serialize(&app_state().lock())?;
    SAVE_PENDING.store(false, Ordering::SeqCst);
    storage::write_atomic(&storage::settings_path(), &data)
}

/// Save while the caller already holds the state lock (UI thread).
pub fn save_now_from_ui(st: &model::State) -> Result<(), String> {
    let data = storage::serialize(st)?;
    SAVE_PENDING.store(false, Ordering::SeqCst);
    storage::write_atomic(&storage::settings_path(), &data)
}

fn start_saver() {
    std::thread::Builder::new()
        .name("makro-saver".into())
        .spawn(|| loop {
            {
                let (lock, cv) = &SAVE_SIGNAL;
                let mut flag = lock.lock().unwrap();
                while !*flag {
                    flag = cv.wait(flag).unwrap();
                }
                *flag = false;
            }
            std::thread::sleep(std::time::Duration::from_millis(400));
            if SAVE_PENDING.swap(false, Ordering::SeqCst) {
                let data = storage::serialize(&app_state().lock());
                if let Ok(data) = data {
                    if let Err(e) = storage::write_atomic(&storage::settings_path(), &data) {
                        send_ui(UiEvent::Status(format!("{} {e}", i18n::t("Settings could not be saved:", "Ayarlar kaydedilemedi:"))));
                    }
                }
            }
        })
        .expect("saver");
}

#[cfg(windows)]
fn wait_for_pid(pid: u32) {
    use windows_sys::Win32::System::Threading::*;
    unsafe {
        let h = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
        if !h.is_null() {
            WaitForSingleObject(h, 15000);
            windows_sys::Win32::Foundation::CloseHandle(h);
        }
    }
}

#[cfg(not(windows))]
fn wait_for_pid(pid: u32) {
    let proc_dir = format!("/proc/{pid}");
    for _ in 0..150 {
        if !std::path::Path::new(&proc_dir).exists() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

fn load_icon() -> Option<eframe::egui::IconData> {
    let img = image::load_from_memory(include_bytes!("../assets/logo.png")).ok()?.into_rgba8();
    let (w, h) = img.dimensions();
    Some(eframe::egui::IconData { rgba: img.into_raw(), width: w, height: h })
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let has = |a: &str| args.iter().any(|x| x == a);
    if has("--uninstall") {
        std::process::exit(installer::uninstall(has("--quiet")));
    }
    if has("--install-ui") {
        // Install.cmd: per-user install with simple dialogs.
        let lang = storage::load().state.language;
        i18n::set_language(&lang);
        match installer::install(true) {
            Ok(msg) => {
                if win::message_box(&format!("{msg}\n\n{}", i18n::t("Start Makro+ now?", "Makro+ şimdi başlatılsın mı?")), APP_TITLE, true) {
                    let _ = std::process::Command::new(installer::installed_exe()).spawn();
                }
            }
            Err(e) => {
                win::message_box(&e, APP_TITLE, false);
            }
        }
        return;
    }
    if let Some(i) = args.iter().position(|a| a == "--wait-pid") {
        if let Some(pid) = args.get(i + 1).and_then(|p| p.parse().ok()) {
            wait_for_pid(pid);
        }
    }
    // Portable copies with their own data folder may run side by side.
    let instance = if storage::is_portable() {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        storage::data_dir().hash(&mut h);
        format!("Local\\MakroPlus3-{:x}", h.finish())
    } else {
        "Local\\MakroPlus3Manager".to_string()
    };
    if !win::single_instance(&instance) {
        // Bring the running copy to the front instead of starting twice.
        if let Some(w) = win::find_window_by_title(APP_TITLE) {
            win::show_normal(w);
            win::activate(w);
        } else {
            win::message_box(i18n::t("Makro+ is already running. Check the system tray.", "Makro+ zaten çalışıyor. Sistem tepsisine bakın."), APP_TITLE, false);
        }
        return;
    }
    let loaded = storage::load();
    i18n::set_language(&loaded.state.language);
    let mut state = loaded.state;
    // Shortcuts that no longer validate are disabled instead of misfiring.
    let snapshot = state.bindings.clone();
    for b in state.bindings.iter_mut().filter(|b| b.enabled) {
        if let Some(e) = rules::validate(b, &snapshot) {
            b.enabled = false;
            let name = b.name.clone();
            state.history.insert(0, model::Activity { time: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(), name, result: format!("{} • {e}", i18n::t("Disabled on start", "Başlangıçta devre dışı bırakıldı")) });
        }
    }
    let start_hidden = has("--tray") || state.start_minimized;
    let _ = STATE.set(Mutex::new(state));
    let (tx, rx): (Sender<UiEvent>, Receiver<UiEvent>) = channel();
    let _ = UI_TX.set(Mutex::new(tx));
    start_saver();
    engine::sync(&app_state().lock());
    #[cfg(not(windows))]
    input::GRAB.store(app_state().lock().linux_exclusive, Ordering::SeqCst);
    engine::start();
    shell::start();
    gesture::start();
    if has("--test-gesture-calibrate") {
        gesture::set_mode(gesture::MODE_CALIBRATE);
    }
    // Check for a newer release on every start (unless turned off in Settings).
    if updates::enabled(&app_state().lock()) && !has("--snapshot") {
        updates::check_async(true);
    }

    // No system title bar: Makro+ draws its own window controls, so maximised
    // and full-screen windows have no title strip.
    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_title(APP_TITLE)
        .with_inner_size([1240.0, 800.0])
        .with_min_inner_size([940.0, 600.0])
        .with_decorations(false)
        .with_resizable(true)
        .with_app_id("makroplus");
    if let Some(icon) = load_icon() {
        viewport = viewport.with_icon(std::sync::Arc::new(icon));
    }
    if has("--snapshot") {
        // Developer screenshots: open off-screen and never take focus.
        viewport = viewport.with_active(false).with_position([-6000.0, 40.0]);
    }
    let options = eframe::NativeOptions { viewport, centered: true, ..Default::default() };
    let notice = loaded.notice;
    let _ = eframe::run_native(
        APP_TITLE,
        options,
        Box::new(move |cc| {
            let _ = CTX.set(cc.egui_ctx.clone());
            Ok(Box::new(ui::App::new(cc, rx, notice, start_hidden)))
        }),
    );
    shell::remove_icon();
    llm::stop();
    if SAVE_PENDING.load(Ordering::SeqCst) {
        let _ = save_now();
    }
}
