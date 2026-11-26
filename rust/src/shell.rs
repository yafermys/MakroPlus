//! Tray icon and clipboard listener on their own thread (never the hook thread,
//! so a slow clipboard owner cannot delay typing).
use crate::i18n::t;
use crate::win::{wide, Hwnd};
use parking_lot::Mutex;
use std::sync::atomic::{AtomicIsize, AtomicU64, Ordering};
use std::sync::OnceLock;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::System::DataExchange::{AddClipboardFormatListener, GetClipboardOwner};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Shell::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

static SHELL: AtomicIsize = AtomicIsize::new(0);
pub static MAIN_WINDOW: AtomicIsize = AtomicIsize::new(0);
/// Clipboard changes are ignored until this tick (ms since start) — used while
/// Makro+ temporarily borrows the clipboard for paste steps.
static SUPPRESS_UNTIL: AtomicU64 = AtomicU64::new(0);
static NOTES: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new());
static ICON: AtomicIsize = AtomicIsize::new(0);

const WM_TRAY: u32 = WM_APP + 1;
const WM_NOTIFY_USER: u32 = WM_APP + 2;
const WM_REMOVE_ICON: u32 = WM_APP + 3;
const WM_TIP: u32 = WM_APP + 4;

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

fn icon_from_ico(bytes: &[u8], size: i32) -> isize {
    if bytes.len() < 6 {
        return 0;
    }
    let count = u16::from_le_bytes([bytes[4], bytes[5]]) as usize;
    let mut best: Option<(i32, usize, usize)> = None;
    for i in 0..count {
        let o = 6 + i * 16;
        if o + 16 > bytes.len() {
            break;
        }
        let w = if bytes[o] == 0 { 256 } else { bytes[o] as i32 };
        let len = u32::from_le_bytes([bytes[o + 8], bytes[o + 9], bytes[o + 10], bytes[o + 11]]) as usize;
        let off = u32::from_le_bytes([bytes[o + 12], bytes[o + 13], bytes[o + 14], bytes[o + 15]]) as usize;
        let score = if w >= size { w - size } else { (size - w) * 4 };
        if best.map(|b| score < b.0).unwrap_or(true) && off + len <= bytes.len() {
            best = Some((score, off, len));
        }
    }
    let Some((_, off, len)) = best else { return 0 };
    unsafe { CreateIconFromResourceEx(bytes[off..].as_ptr(), len as u32, 1, 0x0003_0000, size, size, LR_DEFAULTCOLOR) as isize }
}

pub fn app_icon(size: i32) -> isize {
    icon_from_ico(include_bytes!("../assets/MakroPlus.ico"), size)
}

fn copy_wide(dst: &mut [u16], s: &str) {
    let w: Vec<u16> = s.encode_utf16().take(dst.len() - 1).collect();
    dst[..w.len()].copy_from_slice(&w);
    dst[w.len()] = 0;
}

unsafe fn tray_data(hwnd: HWND) -> NOTIFYICONDATAW {
    let mut d: NOTIFYICONDATAW = std::mem::zeroed();
    d.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    d.hWnd = hwnd;
    d.uID = 1;
    d
}

unsafe fn add_icon(hwnd: HWND) {
    let mut d = tray_data(hwnd);
    d.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    d.uCallbackMessage = WM_TRAY;
    d.hIcon = ICON.load(Ordering::Relaxed) as HICON;
    copy_wide(&mut d.szTip, &tip_text());
    Shell_NotifyIconW(NIM_ADD, &d);
}

fn tip_text() -> String {
    let paused = crate::engine::config().read().paused;
    if paused {
        format!("Makro+ • {}", t("paused", "duraklatıldı"))
    } else {
        "Makro+".into()
    }
}

pub fn show_main() {
    let mut w = MAIN_WINDOW.load(Ordering::Relaxed);
    if w == 0 {
        w = crate::win::find_own_window().unwrap_or(0);
    }
    if w != 0 {
        crate::win::show(w, SW_SHOW);
        crate::win::activate(w);
    }
    crate::send_ui(crate::UiEvent::Shown);
}

unsafe fn menu(hwnd: HWND) {
    let m = CreatePopupMenu();
    let paused = crate::engine::config().read().paused;
    let add = |id: usize, text: &str, flags: u32| {
        AppendMenuW(m, flags, id, wide(text).as_ptr());
    };
    add(1, t("Show Makro+", "Makro+'ı göster"), MF_STRING);
    add(3, t("Clipboard", "Pano"), MF_STRING);
    add(2, if paused { t("Resume shortcuts", "Kısayolları sürdür") } else { t("Pause shortcuts", "Kısayolları duraklat") }, MF_STRING);
    AppendMenuW(m, MF_SEPARATOR, 0, std::ptr::null());
    add(9, t("Exit", "Çıkış"), MF_STRING);
    let mut p = POINT { x: 0, y: 0 };
    GetCursorPos(&mut p);
    SetForegroundWindow(hwnd);
    let cmd = TrackPopupMenu(m, TPM_RETURNCMD | TPM_RIGHTBUTTON | TPM_BOTTOMALIGN, p.x, p.y, 0, hwnd, std::ptr::null());
    DestroyMenu(m);
    match cmd {
        1 => show_main(),
        2 => crate::send_ui(crate::UiEvent::TogglePause),
        3 => {
            show_main();
            crate::send_ui(crate::UiEvent::ShowPage("clipboard".into()));
        }
        9 => {
            show_main();
            crate::send_ui(crate::UiEvent::Exit);
        }
        _ => {}
    }
}

fn on_clipboard() {
    if now_ms() < SUPPRESS_UNTIL.load(Ordering::SeqCst) {
        return;
    }
    let opts = crate::app_state().lock().clipboard.clone();
    let owner = unsafe { GetClipboardOwner() } as Hwnd;
    let source = if owner != 0 { crate::win::window_process_path(owner) } else { crate::win::window_process_path(crate::win::foreground()) };
    let changed = clip_store().lock().capture(&opts, crate::win::file_stem(&source));
    if changed {
        crate::send_ui(crate::UiEvent::ClipboardChanged);
        save_clipboard_soon();
    }
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

unsafe extern "system" fn proc_(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    static TASKBAR: OnceLock<u32> = OnceLock::new();
    let taskbar = *TASKBAR.get_or_init(|| RegisterWindowMessageW(wide("TaskbarCreated").as_ptr()));
    match msg {
        WM_TRAY => {
            match l as u32 {
                WM_LBUTTONUP | WM_LBUTTONDBLCLK => show_main(),
                WM_RBUTTONUP | WM_CONTEXTMENU => menu(hwnd),
                _ => {}
            }
            0
        }
        WM_CLIPBOARDUPDATE => {
            on_clipboard();
            0
        }
        WM_NOTIFY_USER => {
            let notes = std::mem::take(&mut *NOTES.lock());
            for (title, text) in notes {
                let mut d = tray_data(hwnd);
                d.uFlags = NIF_INFO;
                copy_wide(&mut d.szInfoTitle, &title);
                copy_wide(&mut d.szInfo, &text);
                d.dwInfoFlags = NIIF_INFO;
                Shell_NotifyIconW(NIM_MODIFY, &d);
            }
            0
        }
        WM_TIP => {
            let mut d = tray_data(hwnd);
            d.uFlags = NIF_TIP;
            copy_wide(&mut d.szTip, &tip_text());
            Shell_NotifyIconW(NIM_MODIFY, &d);
            0
        }
        WM_REMOVE_ICON => {
            let d = tray_data(hwnd);
            Shell_NotifyIconW(NIM_DELETE, &d);
            0
        }
        _ if msg == taskbar => {
            add_icon(hwnd);
            0
        }
        _ => DefWindowProcW(hwnd, msg, w, l),
    }
}

pub fn start() {
    std::thread::Builder::new()
        .name("makro-shell".into())
        .spawn(|| unsafe {
            let class = wide("MakroPlusShell");
            let module = GetModuleHandleW(std::ptr::null());
            let wc = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: 0,
                lpfnWndProc: Some(proc_),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: module,
                hIcon: std::ptr::null_mut(),
                hCursor: std::ptr::null_mut(),
                hbrBackground: std::ptr::null_mut(),
                lpszMenuName: std::ptr::null(),
                lpszClassName: class.as_ptr(),
                hIconSm: std::ptr::null_mut(),
            };
            RegisterClassExW(&wc);
            // A hidden top-level window (not message-only) so TaskbarCreated reaches it.
            let hwnd = CreateWindowExW(WS_EX_TOOLWINDOW, class.as_ptr(), wide("Makro+ shell").as_ptr(), WS_POPUP, 0, 0, 0, 0, std::ptr::null_mut(), std::ptr::null_mut(), module, std::ptr::null());
            SHELL.store(hwnd as Hwnd, Ordering::SeqCst);
            ICON.store(app_icon(GetSystemMetrics(SM_CXSMICON).max(16)), Ordering::Relaxed);
            add_icon(hwnd);
            AddClipboardFormatListener(hwnd);
            let _ = clip_store();
            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        })
        .expect("shell thread");
}

/// Show a tray balloon / Windows notification.
pub fn notify(title: &str, text: &str) {
    NOTES.lock().push((title.to_string(), crate::model::short(text, 250)));
    let w = SHELL.load(Ordering::Relaxed);
    if w != 0 {
        unsafe { PostMessageW(w as HWND, WM_NOTIFY_USER, 0, 0) };
    }
}

pub fn refresh_tip() {
    let w = SHELL.load(Ordering::Relaxed);
    if w != 0 {
        unsafe { PostMessageW(w as HWND, WM_TIP, 0, 0) };
    }
}

pub fn remove_icon() {
    let w = SHELL.load(Ordering::Relaxed);
    if w != 0 {
        unsafe {
            let d = tray_data(w as HWND);
            Shell_NotifyIconW(NIM_DELETE, &d);
        }
    }
}
