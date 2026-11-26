//! Shortcut guide: a non-activating overlay listing shortcuts for the foreground app.
use crate::win::{wide, Hwnd};
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::sync::Mutex;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

static WINDOW: AtomicIsize = AtomicIsize::new(0);
static VISIBLE: AtomicBool = AtomicBool::new(false);
static CONTENT: Mutex<Option<Content>> = Mutex::new(None);

struct Content {
    title: String,
    lines: Vec<(String, String)>,
    bg: u32,
    ink: u32,
    muted: u32,
    accent: u32,
}

fn rgb(hex: &str) -> u32 {
    let c = crate::theme::parse_hex(hex);
    (c.r() as u32) | ((c.g() as u32) << 8) | ((c.b() as u32) << 16)
}

pub fn visible() -> bool {
    VISIBLE.load(Ordering::Relaxed)
}

unsafe extern "system" fn proc_(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    if msg == WM_PAINT {
        let mut ps: PAINTSTRUCT = std::mem::zeroed();
        let hdc = BeginPaint(hwnd, &mut ps);
        if let Some(c) = CONTENT.lock().unwrap().as_ref() {
            let mut rc = RECT { left: 0, top: 0, right: 0, bottom: 0 };
            GetClientRect(hwnd, &mut rc);
            let bg = CreateSolidBrush(c.bg);
            FillRect(hdc, &rc, bg);
            DeleteObject(bg);
            let accent = CreateSolidBrush(c.accent);
            let bar = RECT { left: 0, top: 0, right: 5, bottom: rc.bottom };
            FillRect(hdc, &bar, accent);
            DeleteObject(accent);
            SetBkMode(hdc, TRANSPARENT as _);
            let face = wide("Segoe UI");
            let big = CreateFontW(-20, 0, 0, 0, 600, 0, 0, 0, 0, 0, 0, 5, 0, face.as_ptr());
            let normal = CreateFontW(-15, 0, 0, 0, 400, 0, 0, 0, 0, 0, 0, 5, 0, face.as_ptr());
            let bold = CreateFontW(-15, 0, 0, 0, 600, 0, 0, 0, 0, 0, 0, 5, 0, face.as_ptr());
            let old = SelectObject(hdc, big);
            SetTextColor(hdc, c.ink);
            let mut t = RECT { left: 22, top: 14, right: rc.right - 16, bottom: 44 };
            let title = wide(&c.title);
            DrawTextW(hdc, title.as_ptr(), -1, &mut t, DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS);
            let cols = if c.lines.len() > 22 { 2 } else { 1 };
            let rows = c.lines.len().div_ceil(cols).max(1);
            for (i, (k, label)) in c.lines.iter().enumerate() {
                let col = (i / rows) as i32;
                let y = 52 + (i % rows) as i32 * 25;
                let x0 = 22 + col * 620;
                SelectObject(hdc, bold);
                SetTextColor(hdc, c.accent);
                let mut kr = RECT { left: x0, top: y, right: x0 + 200, bottom: y + 24 };
                let kw = wide(k);
                DrawTextW(hdc, kw.as_ptr(), -1, &mut kr, DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS);
                SelectObject(hdc, normal);
                SetTextColor(hdc, c.ink);
                let mut lr = RECT { left: x0 + 208, top: y, right: x0 + 600, bottom: y + 24 };
                let lw = wide(label);
                DrawTextW(hdc, lw.as_ptr(), -1, &mut lr, DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS);
            }
            let y = 52;
            if c.lines.is_empty() {
                SelectObject(hdc, normal);
                SetTextColor(hdc, c.muted);
                let mut lr = RECT { left: 22, top: y, right: rc.right - 16, bottom: y + 24 };
                let msg = wide(crate::i18n::t("No shortcuts for this app yet.", "Bu uygulama için henüz kısayol yok."));
                DrawTextW(hdc, msg.as_ptr(), -1, &mut lr, DT_LEFT | DT_SINGLELINE | DT_VCENTER);
            }
            SelectObject(hdc, old);
            DeleteObject(big);
            DeleteObject(normal);
            DeleteObject(bold);
        }
        EndPaint(hwnd, &ps);
        return 0;
    }
    DefWindowProcW(hwnd, msg, w, l)
}

fn ensure_window() -> Hwnd {
    let w = WINDOW.load(Ordering::Relaxed);
    if w != 0 {
        return w;
    }
    unsafe {
        let class = wide("MakroPlusGuide");
        let module = GetModuleHandleW(std::ptr::null());
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
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
        let hwnd = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_LAYERED | WS_EX_TRANSPARENT,
            class.as_ptr(),
            wide("Makro+ guide").as_ptr(),
            WS_POPUP,
            0,
            0,
            100,
            100,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            module,
            std::ptr::null(),
        );
        SetLayeredWindowAttributes(hwnd, 0, 246, LWA_ALPHA);
        WINDOW.store(hwnd as Hwnd, Ordering::Relaxed);
        hwnd as Hwnd
    }
}

const SYSTEM: &[(&str, &str, &str)] = &[
    ("Win + D", "Show desktop", "Masaüstünü göster"),
    ("Win + E", "File Explorer", "Dosya Gezgini"),
    ("Win + L", "Lock", "Kilitle"),
    ("Win + V", "Clipboard history", "Pano geçmişi"),
    ("Win + Shift + S", "Screen snip", "Ekran alıntısı"),
    ("Win + Tab", "Task view", "Görev görünümü"),
    ("Win + .", "Emoji", "Emoji"),
    ("Win + I", "Settings", "Ayarlar"),
    ("Alt + Tab", "Switch apps", "Uygulama değiştir"),
    ("Alt + F4", "Close window", "Pencereyi kapat"),
    ("Ctrl + Shift + Esc", "Task Manager", "Görev Yöneticisi"),
    ("Ctrl + C / V / X", "Copy / paste / cut", "Kopyala / yapıştır / kes"),
    ("Ctrl + Z / Y", "Undo / redo", "Geri al / yinele"),
    ("Ctrl + A", "Select all", "Tümünü seç"),
    ("Ctrl + S", "Save", "Kaydet"),
    ("Win + ← / →", "Snap window", "Pencereyi yasla"),
];

/// Build and show the overlay (called on the hook thread).
pub fn show(fg: Hwnd, cfg: &crate::engine::Config, app: &str) {
    let show = &cfg.guide_show;
    let mut lines: Vec<(String, String)> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut push = |lines: &mut Vec<(String, String)>, k: String, l: String| {
        if seen.insert(format!("{k}|{l}")) {
            lines.push((k, l));
        }
    };
    let label = |b: &crate::model::Binding| {
        if b.app_path.is_empty() || crate::win::same_path(&b.app_path, app) {
            b.name.clone()
        } else {
            format!("{} · {}", b.name, crate::apps::display_name(&b.app_path))
        }
    };
    if show.this_app && !app.is_empty() {
        for b in cfg.bindings.iter().filter(|b| crate::win::same_path(&b.app_path, app)) {
            push(&mut lines, b.combo(), label(b));
        }
    }
    if show.global {
        for b in cfg.bindings.iter().filter(|b| b.app_path.is_empty()) {
            push(&mut lines, b.combo(), label(b));
        }
    }
    if show.all_apps {
        for b in cfg.all_bindings.iter() {
            push(&mut lines, b.combo(), label(b));
        }
    }
    if show.app_own {
        if let Some((_, extra)) = cfg.guide_extra.iter().find(|(p, _)| crate::win::same_path(p, app)) {
            for (k, l) in extra {
                push(&mut lines, k.clone(), l.clone());
            }
        }
    }
    if show.system {
        for (k, en, tr) in SYSTEM {
            push(&mut lines, k.to_string(), crate::i18n::t(en, tr).to_string());
        }
    }
    lines.truncate(60);
    let name = if app.is_empty() { "Windows".to_string() } else { crate::apps::display_name(app) };
    let p = &cfg.palette;
    *CONTENT.lock().unwrap() = Some(Content {
        title: format!("{} — {}", name, crate::i18n::t("shortcuts", "kısayollar")),
        lines,
        bg: rgb(&p.panel),
        ink: rgb(&p.ink),
        muted: rgb(&p.muted),
        accent: rgb(&p.accent()),
    });
    let n = CONTENT.lock().unwrap().as_ref().map(|c| c.lines.len().max(1)).unwrap_or(1) as i32;
    let cols = if n > 22 { 2 } else { 1 };
    let rows = (n + cols - 1) / cols;
    let width = 620 * cols;
    let height = 66 + rows * 25;
    let w = ensure_window();
    unsafe {
        let mon = MonitorFromWindow(fg as HWND, MONITOR_DEFAULTTONEAREST);
        let mut mi: MONITORINFO = std::mem::zeroed();
        mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        GetMonitorInfoW(mon, &mut mi);
        let r = mi.rcWork;
        let x = r.left + ((r.right - r.left) - width) / 2;
        let y = r.top + ((r.bottom - r.top) - height) / 2;
        SetWindowPos(w as HWND, HWND_TOPMOST, x, y, width, height, SWP_NOACTIVATE | SWP_SHOWWINDOW);
        InvalidateRect(w as HWND, std::ptr::null(), 1);
    }
    VISIBLE.store(true, Ordering::Relaxed);
}

pub fn hide() {
    let w = WINDOW.load(Ordering::Relaxed);
    if w != 0 && VISIBLE.swap(false, Ordering::Relaxed) {
        unsafe { ShowWindow(w as HWND, SW_HIDE) };
    }
}
