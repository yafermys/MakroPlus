//! Linux tray (StatusNotifierItem), clipboard watcher and notifications.
use crate::i18n::t;
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU64, Ordering};
use std::sync::OnceLock;

pub static MAIN_WINDOW: AtomicIsize = AtomicIsize::new(0);
pub static MAIN_VISIBLE: AtomicBool = AtomicBool::new(true);
static SUPPRESS_UNTIL: AtomicU64 = AtomicU64::new(0);

pub fn clip_store() -> &'static Mutex<crate::clipboard::Store> {
    static S: OnceLock<Mutex<crate::clipboard::Store>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(crate::clipboard::Store::load(&crate::storage::clipboard_path())))
}

fn now_ms() -> u64 {
    static START: OnceLock<std::time::Instant> = OnceLock::new();
    START.get_or_init(std::time::Instant::now).elapsed().as_millis() as u64
}

pub fn suppress_clipboard(ms: u64) {
    SUPPRESS_UNTIL.store(now_ms() + ms, Ordering::SeqCst);
}

static CLIP_SAVE_GEN: AtomicU64 = AtomicU64::new(0);

pub fn save_clipboard_soon() {
    let gen = CLIP_SAVE_GEN.fetch_add(1, Ordering::SeqCst) + 1;
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(800));
        if CLIP_SAVE_GEN.load(Ordering::SeqCst) != gen {
            return;
        }
        let persist = crate::app_state().lock().clipboard.persist;
        let path = crate::storage::clipboard_path();
        if !persist {
            let _ = std::fs::remove_file(&path);
            return;
        }
        let bytes = clip_store().lock().save_bytes();
        let _ = crate::storage::write_atomic(&path, &bytes);
    });
}

pub fn show_main() {
    MAIN_VISIBLE.store(true, Ordering::Relaxed);
    if let Some(ctx) = crate::ctx() {
        ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Focus);
    }
    crate::send_ui(crate::UiEvent::Shown);
}

struct Tray;

fn icon() -> Vec<ksni::Icon> {
    let Ok(img) = image::load_from_memory(include_bytes!("../../assets/logo.png")) else { return vec![] };
    let img = img.resize(64, 64, image::imageops::FilterType::Triangle).into_rgba8();
    let (w, h) = img.dimensions();
    // SNI wants ARGB32 in network byte order.
    let data = img.pixels().flat_map(|p| [p[3], p[0], p[1], p[2]]).collect();
    vec![ksni::Icon { width: w as i32, height: h as i32, data }]
}

impl ksni::Tray for Tray {
    fn id(&self) -> String {
        "makroplus".into()
    }
    fn title(&self) -> String {
        if crate::engine::config().read().paused {
            format!("Makro+ • {}", t("paused", "duraklatıldı"))
        } else {
            "Makro+".into()
        }
    }
    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        icon()
    }
    fn activate(&mut self, _x: i32, _y: i32) {
        show_main();
    }
    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::*;
        let paused = crate::engine::config().read().paused;
        vec![
            StandardItem { label: t("Show Makro+", "Makro+'ı göster").into(), activate: Box::new(|_| show_main()), ..Default::default() }.into(),
            StandardItem {
                label: t("Clipboard", "Pano").into(),
                activate: Box::new(|_| {
                    show_main();
                    crate::send_ui(crate::UiEvent::ShowPage("clipboard".into()));
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: if paused { t("Resume shortcuts", "Kısayolları sürdür") } else { t("Pause shortcuts", "Kısayolları duraklat") }.into(),
                activate: Box::new(|_| crate::send_ui(crate::UiEvent::TogglePause)),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: t("Exit", "Çıkış").into(),
                activate: Box::new(|_| {
                    show_main();
                    crate::send_ui(crate::UiEvent::Exit);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}

static TRAY: Mutex<Option<ksni::blocking::Handle<Tray>>> = Mutex::new(None);

pub fn start() {
    use ksni::blocking::TrayMethods;
    std::thread::Builder::new()
        .name("makro-tray".into())
        .spawn(|| {
            if let Ok(h) = Tray.spawn() {
                *TRAY.lock() = Some(h);
            }
        })
        .ok();
    // Clipboard watcher: Linux has no portable change notification, so poll.
    std::thread::Builder::new()
        .name("makro-clipboard".into())
        .spawn(|| {
            let _ = clip_store();
            crate::clipboard::poll_changed();
            loop {
                std::thread::sleep(std::time::Duration::from_millis(600));
                if !crate::clipboard::poll_changed() || now_ms() < SUPPRESS_UNTIL.load(Ordering::SeqCst) {
                    continue;
                }
                let opts = crate::app_state().lock().clipboard.clone();
                let source = crate::win::file_stem(&crate::win::window_process_path(crate::win::foreground()));
                if clip_store().lock().capture(&opts, source) {
                    crate::send_ui(crate::UiEvent::ClipboardChanged);
                    save_clipboard_soon();
                }
            }
        })
        .ok();
}

pub fn notify(title: &str, text: &str) {
    let _ = std::process::Command::new("notify-send").args(["-a", "Makro+", "-i", "makroplus", title, &crate::model::short(text, 250)]).spawn();
}

pub fn refresh_tip() {
    if let Some(h) = TRAY.lock().as_ref() {
        h.update(|_| {});
    }
}

pub fn remove_icon() {
    if let Some(h) = TRAY.lock().take() {
        let _ = h.shutdown();
    }
}

pub fn app_icon(_size: i32) -> isize {
    0
}
