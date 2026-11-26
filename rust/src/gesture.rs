//! Touchpad shapes: raw pointer input, shape recording and recognition.
//! The recogniser matches Makro+ 2.x (64 resampled points, rotation-tolerant).
use crate::i18n::t;
use crate::model::{Binding, GestureOptions, GesturePoint, ShapeTemplate};
use crate::win::Hwnd;
use parking_lot::RwLock;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

// ---------------- recognition ----------------

fn dist(a: GesturePoint, b: GesturePoint) -> f64 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt()
}

pub fn length(p: &[GesturePoint]) -> f64 {
    p.windows(2).map(|w| dist(w[0], w[1])).sum()
}

pub fn validate(s: &ShapeTemplate) -> Option<&'static str> {
    let p = &s.points;
    if p.len() < 8 || p.len() > 512 || p.iter().any(|q| !q.x.is_finite() || !q.y.is_finite() || q.x.abs() > 1e7 || q.y.abs() > 1e7) {
        return Some(t("Record a valid shape with 8–512 points.", "8–512 noktalı geçerli bir şekil kaydedin."));
    }
    if length(p) < 20.0 {
        return Some(t("The shape is too small. Draw a larger shape.", "Şekil çok küçük. Daha büyük çizin."));
    }
    None
}

pub fn normalize(src: &[GesturePoint]) -> Vec<GesturePoint> {
    let mut clean: Vec<GesturePoint> = Vec::new();
    for &p in src {
        if !p.x.is_finite() || !p.y.is_finite() {
            return vec![];
        }
        if clean.last().map(|l| dist(*l, p) > 1e-5).unwrap_or(true) {
            clean.push(p);
        }
    }
    if clean.len() < 2 {
        return vec![];
    }
    let total = length(&clean);
    if total < 1e-4 {
        return vec![];
    }
    let mut out = Vec::with_capacity(64);
    let (mut seg, mut passed) = (1usize, 0.0);
    for i in 0..64 {
        let wanted = total * i as f64 / 63.0;
        while seg < clean.len() - 1 && passed + dist(clean[seg - 1], clean[seg]) < wanted {
            passed += dist(clean[seg - 1], clean[seg]);
            seg += 1;
        }
        let span = dist(clean[seg - 1], clean[seg]);
        let tt = if span > 0.0 { (wanted - passed) / span } else { 0.0 };
        out.push(GesturePoint { x: clean[seg - 1].x + (clean[seg].x - clean[seg - 1].x) * tt, y: clean[seg - 1].y + (clean[seg].y - clean[seg - 1].y) * tt });
    }
    let (minx, maxx) = out.iter().fold((f64::MAX, f64::MIN), |a, p| (a.0.min(p.x), a.1.max(p.x)));
    let (miny, maxy) = out.iter().fold((f64::MAX, f64::MIN), |a, p| (a.0.min(p.y), a.1.max(p.y)));
    let size = (maxx - minx).max(maxy - miny);
    if size < 1e-4 {
        return vec![];
    }
    let cx = out.iter().map(|p| p.x).sum::<f64>() / 64.0;
    let cy = out.iter().map(|p| p.y).sum::<f64>() / 64.0;
    out.iter().map(|p| GesturePoint { x: (p.x - cx) / size, y: (p.y - cy) / size }).collect()
}

fn error(a: &[GesturePoint], b: &[GesturePoint], reverse: bool, shift: usize, angle: f64, closed: bool) -> f64 {
    let (c, s) = (angle.cos(), angle.sin());
    let count = if closed { 63 } else { 64 };
    let mut sum = 0.0;
    for i in 0..count {
        let mut idx = if closed { (i + shift) % count } else { i };
        if reverse {
            idx = count - 1 - idx;
        }
        let x = a[idx].x * c - a[idx].y * s;
        let y = a[idx].x * s + a[idx].y * c;
        sum += ((x - b[i].x).powi(2) + (y - b[i].y).powi(2)).sqrt();
    }
    sum / count as f64
}

pub fn similarity_normalized(a: &[GesturePoint], b: &[GesturePoint]) -> f64 {
    if a.len() != 64 || b.len() != 64 {
        return 0.0;
    }
    let closed = dist(a[0], a[63]) < 0.23 && dist(b[0], b[63]) < 0.23;
    let mut best = f64::MAX;
    for deg in (-12..=12).step_by(6) {
        let ang = deg as f64 * std::f64::consts::PI / 180.0;
        if closed {
            for shift in (0..63).step_by(3) {
                best = best.min(error(a, b, false, shift, ang, true)).min(error(a, b, true, shift, ang, true));
            }
        } else {
            best = best.min(error(a, b, false, 0, ang, false));
        }
    }
    (1.0 - best / std::f64::consts::FRAC_1_SQRT_2).max(0.0)
}

pub fn similarity(a: &[GesturePoint], b: &[GesturePoint]) -> f64 {
    similarity_normalized(&normalize(a), &normalize(b))
}

/// Similar shapes among other enabled shape shortcuts.
pub fn conflicts(b: &Binding, all: &[Binding]) -> Vec<(bool, String)> {
    let mut out = Vec::new();
    let Some(g) = &b.gesture else { return out };
    if validate(g).is_some() {
        return out;
    }
    for o in all.iter().filter(|o| o.id != b.id && o.enabled && b.enabled && (b.profile_id.is_empty() || o.profile_id.is_empty() || b.profile_id == o.profile_id)) {
        let Some(og) = &o.gesture else { continue };
        if !b.app_path.is_empty() && !o.app_path.is_empty() && !crate::win::same_path(&b.app_path, &o.app_path) {
            continue;
        }
        let s = similarity(&g.points, &og.points);
        if s >= 0.92 {
            let hard = crate::win::same_path(&b.app_path, &o.app_path) && b.profile_id == o.profile_id || (b.app_path.is_empty() && o.app_path.is_empty());
            out.push((hard, crate::i18n::ts(format!("Similar shape: “{}” ({:.0}%).", o.name, s * 100.0), format!("Benzer şekil: “{}” (%{:.0}).", o.name, s * 100.0))));
        }
    }
    out
}

struct Prepared {
    binding: Binding,
    norm: Vec<GesturePoint>,
}

/// Best match for a stroke, honouring score and margin thresholds.
fn best_match<'a>(stroke: &[GesturePoint], cands: &'a [Prepared], o: &GestureOptions) -> (Option<&'a Binding>, String) {
    if stroke.len() < 8 || length(stroke) < o.minimum_length {
        return (None, t("Movement is too short.", "Hareket çok kısa.").into());
    }
    let n = normalize(stroke);
    if n.len() != 64 {
        return (None, String::new());
    }
    let (mut first, mut second, mut best) = (-1.0, -1.0, None);
    for c in cands {
        let s = similarity_normalized(&n, &c.norm);
        if s > first {
            second = first;
            first = s;
            best = Some(&c.binding);
        } else if s > second {
            second = s;
        }
    }
    let Some(b) = best else { return (None, String::new()) };
    if first < o.minimum_score {
        return (None, t("No confident match.", "Emin bir eşleşme yok.").into());
    }
    if (if second < 0.0 { 1.0 } else { first - second }) < o.minimum_margin {
        return (None, t("Ambiguous shape; nothing ran.", "Şekil belirsiz; hiçbir şey çalışmadı.").into());
    }
    (Some(b), format!("{} {}", t("Matched:", "Eşleşti:"), b.name))
}

// ---------------- raw pointer input ----------------

pub const MODE_LISTEN: u8 = 0;
pub const MODE_RECORD: u8 = 1;
pub const MODE_CALIBRATE: u8 = 2;
pub static MODE: AtomicU8 = AtomicU8::new(MODE_LISTEN);

struct Shared {
    options: GestureOptions,
    shapes: Vec<Prepared>,
    paused: bool,
}

fn shared() -> &'static RwLock<Shared> {
    static S: OnceLock<RwLock<Shared>> = OnceLock::new();
    S.get_or_init(|| RwLock::new(Shared { options: GestureOptions::default(), shapes: vec![], paused: false }))
}

pub fn sync(state: &crate::model::State) {
    let profile = state.active_profile();
    let shapes = state
        .bindings
        .iter()
        .filter(|b| b.enabled && crate::model::State::profile_active(b, profile))
        .filter_map(|b| b.gesture.as_ref().filter(|g| validate(g).is_none()).map(|g| Prepared { binding: b.clone(), norm: normalize(&g.points) }))
        .collect();
    let mut s = shared().write();
    s.options = state.gestures.clone();
    s.shapes = shapes;
    s.paused = state.paused;
}

pub fn set_mode(m: u8) {
    MODE.store(m, Ordering::SeqCst);
}

struct Stroke {
    points: Vec<GesturePoint>,
    x: f64,
    y: f64,
    started: Instant,
    last: Instant,
    last_sample: Instant,
    last_trace: Instant,
    window: Hwnd,
    device: String,
    blocked_until: Instant,
}

static STROKE: parking_lot::Mutex<Option<Stroke>> = parking_lot::Mutex::new(None);

fn with_stroke(f: impl FnOnce(&mut Stroke)) {
    let mut g = STROKE.lock();
    if g.is_none() {
        let now = Instant::now();
        *g = Some(Stroke { points: vec![], x: 0.0, y: 0.0, started: now, last: now, last_sample: now, last_trace: now, window: 0, device: String::new(), blocked_until: now });
    }
    f(g.as_mut().unwrap());
}

/// Relative finger movement from a pointer source.
pub fn feed_delta(dx: f64, dy: f64, device: String, buttons: bool) {
    with_stroke(|s| feed(s, dx, dy, device, buttons));
}

/// Called periodically to end a stroke after a pause.
pub fn tick() {
    with_stroke(finish);
}

fn reset(s: &mut Stroke) {
    s.points.clear();
    s.x = 0.0;
    s.y = 0.0;
    s.window = 0;
}

fn any_button_down() -> bool {
    [1, 2, 4, 5, 6].iter().any(|&k| crate::win::key_down(k))
}

fn feed(s: &mut Stroke, dx: f64, dy: f64, device: String, buttons: bool) {
    let now = Instant::now();
    let mode = MODE.load(Ordering::Relaxed);
    if now < s.blocked_until {
        return;
    }
    if buttons {
        reset(s);
        return;
    }
    if mode == MODE_CALIBRATE {
        set_mode(MODE_LISTEN);
        reset(s);
        crate::send_ui(crate::UiEvent::GestureDevice(device));
        return;
    }
    let sh = shared().read();
    if mode == MODE_LISTEN && (sh.shapes.is_empty() || !sh.options.enabled || sh.paused || crate::actions::WORKFLOW_RUNNING.load(Ordering::Relaxed) || crate::engine::MODE.load(Ordering::Relaxed) != crate::engine::MODE_NORMAL) {
        if !s.points.is_empty() {
            reset(s);
        }
        return;
    }
    if !device.eq_ignore_ascii_case(&sh.options.device) {
        if !s.points.is_empty() {
            reset(s);
        }
        return;
    }
    drop(sh);
    if dx.abs() > 5000.0 || dy.abs() > 5000.0 {
        reset(s);
        return;
    }
    let fg = crate::win::foreground();
    if !s.points.is_empty() && (fg != s.window || now.duration_since(s.started) > Duration::from_secs(5)) {
        reset(s);
        return;
    }
    if s.points.is_empty() {
        if mode == MODE_LISTEN && crate::win::window_pid(fg) == std::process::id() {
            return;
        }
        s.window = fg;
        s.device = device;
        s.started = now;
        s.last_sample = now - Duration::from_millis(50);
        s.points.push(GesturePoint { x: 0.0, y: 0.0 });
    }
    s.x += dx;
    s.y += dy;
    s.last = now;
    let p = GesturePoint { x: s.x, y: s.y };
    if now.duration_since(s.last_sample) >= Duration::from_millis(6) && dist(*s.points.last().unwrap(), p) >= 1.0 {
        s.points.push(p);
        s.last_sample = now;
        if s.points.len() > 512 {
            reset(s);
            crate::send_ui(crate::UiEvent::GestureStatus(t("Movement was too long; draw a shorter shape.", "Hareket çok uzun; daha kısa bir şekil çizin.").into()));
            return;
        }
        if mode == MODE_RECORD && now.duration_since(s.last_trace) >= Duration::from_millis(40) {
            s.last_trace = now;
            crate::send_ui(crate::UiEvent::GestureTrace(s.points.clone()));
        }
    }
}

fn finish(s: &mut Stroke) {
    if s.points.is_empty() {
        return;
    }
    let sh = shared().read();
    if s.last.elapsed() < Duration::from_millis(sh.options.pause_milliseconds.clamp(200, 3000) as u64) {
        return;
    }
    let points = std::mem::take(&mut s.points);
    let window = s.window;
    reset(s);
    if MODE.load(Ordering::Relaxed) == MODE_RECORD {
        let tpl = ShapeTemplate { points: points.clone() };
        if validate(&tpl).is_some() || length(&points) < sh.options.minimum_length {
            crate::send_ui(crate::UiEvent::GestureStatus(t("Draw a larger, distinct shape and pause.", "Daha büyük, belirgin bir şekil çizip bekleyin.").into()));
            return;
        }
        set_mode(MODE_LISTEN);
        crate::send_ui(crate::UiEvent::GestureRecorded(points));
        return;
    }
    if any_button_down() || crate::win::foreground() != window || [16, 17, 18, 91, 92].iter().any(|&k| crate::win::key_down(k)) {
        return;
    }
    let app = crate::win::window_process_path(window);
    // App-specific shapes win over global ones.
    let scoped: Vec<&Prepared> = sh.shapes.iter().filter(|c| c.binding.app_path.is_empty() || crate::win::same_path(&c.binding.app_path, &app)).collect();
    let specific: Vec<Prepared> = scoped.iter().filter(|c| !c.binding.app_path.is_empty()).map(|c| Prepared { binding: c.binding.clone(), norm: c.norm.clone() }).collect();
    let global: Vec<Prepared> = scoped.iter().filter(|c| c.binding.app_path.is_empty()).map(|c| Prepared { binding: c.binding.clone(), norm: c.norm.clone() }).collect();
    let (mut hit, mut reason) = best_match(&points, &specific, &sh.options);
    if hit.is_none() {
        let r = best_match(&points, &global, &sh.options);
        if let Some(b) = r.0 {
            let b = b.clone();
            drop(sh);
            s.blocked_until = Instant::now() + Duration::from_millis(900);
            crate::actions::submit(b, window);
            return;
        }
        reason = r.1;
    }
    if let Some(b) = hit.take() {
        let b = b.clone();
        drop(sh);
        s.blocked_until = Instant::now() + Duration::from_millis(900);
        crate::actions::submit(b, window);
        return;
    }
    let _ = reason;
}

#[cfg(windows)]
mod raw {
    //! Windows Raw Input: relative pointer movement per device.
    use super::*;
    use crate::win::wide;
    use windows_sys::Win32::Foundation::*;
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::Input::*;
    use windows_sys::Win32::UI::WindowsAndMessaging::*;

fn device_name(h: HANDLE) -> String {
    if h.is_null() {
        return String::new();
    }
    unsafe {
        let mut n = 0u32;
        GetRawInputDeviceInfoW(h, RIDI_DEVICENAME, std::ptr::null_mut(), &mut n);
        if n == 0 || n > 2048 {
            return String::new();
        }
        let mut buf = vec![0u16; n as usize + 1];
        GetRawInputDeviceInfoW(h, RIDI_DEVICENAME, buf.as_mut_ptr() as _, &mut n);
        crate::win::from_wide(&buf)
    }
}

unsafe extern "system" fn proc_(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    if msg == WM_INPUT {
        let mut size = 0u32;
        let header = std::mem::size_of::<RAWINPUTHEADER>() as u32;
        GetRawInputData(l as HRAWINPUT, RID_INPUT, std::ptr::null_mut(), &mut size, header);
        if size > 0 && size < 1024 {
            let mut buf = vec![0u8; size as usize];
            if GetRawInputData(l as HRAWINPUT, RID_INPUT, buf.as_mut_ptr() as _, &mut size, header) == size {
                let raw = &*(buf.as_ptr() as *const RAWINPUT);
                if raw.header.dwType == RIM_TYPEMOUSE {
                    let m = raw.data.mouse;
                    let flags = m.Anonymous.Anonymous.usButtonFlags;
                    let buttons = flags & 0x03FF != 0;
                    let absolute = m.usFlags & 1 != 0;
                    if !absolute && (m.lLastX != 0 || m.lLastY != 0 || buttons) {
                        let dev = device_name(raw.header.hDevice);
                        feed_delta(m.lLastX as f64, m.lLastY as f64, dev, buttons);
                    }
                }
            }
        }
        return DefWindowProcW(hwnd, msg, w, l);
    }
    if msg == WM_TIMER {
        tick();
        return 0;
    }
    DefWindowProcW(hwnd, msg, w, l)
}

pub fn start() {
    std::thread::Builder::new()
        .name("makro-gesture".into())
        .spawn(|| unsafe {
            let class = wide("MakroPlusGesture");
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
            let hwnd = CreateWindowExW(0, class.as_ptr(), wide("Makro+ gesture").as_ptr(), 0, 0, 0, 0, 0, HWND_MESSAGE, std::ptr::null_mut(), module, std::ptr::null());
            let dev = RAWINPUTDEVICE { usUsagePage: 1, usUsage: 2, dwFlags: RIDEV_INPUTSINK, hwndTarget: hwnd };
            RegisterRawInputDevices(&dev, 1, std::mem::size_of::<RAWINPUTDEVICE>() as u32);
            SetTimer(hwnd, 1, 60, None);
            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        })
        .expect("gesture thread");
}

}

#[cfg(windows)]
pub use raw::start;

/// Linux reads the touchpad together with the keyboard (see input.rs).
#[cfg(not(windows))]
pub fn start() {}

/// Start recording a new shape (ignores input for a moment so the click is not drawn).
pub fn begin_record() {
    set_mode(MODE_RECORD);
}

#[cfg(test)]
mod tests {
    use super::*;
    fn circle(n: usize, r: f64, phase: f64) -> Vec<GesturePoint> {
        (0..n).map(|i| {
            let a = phase + i as f64 / (n - 1) as f64 * std::f64::consts::TAU;
            GesturePoint { x: r * a.cos(), y: r * a.sin() }
        }).collect()
    }
    fn zigzag(n: usize) -> Vec<GesturePoint> {
        (0..n).map(|i| GesturePoint { x: i as f64 * 10.0, y: if i % 2 == 0 { 0.0 } else { 60.0 } }).collect()
    }
    #[test]
    fn recognises() {
        let c1 = circle(80, 100.0, 0.0);
        let c2 = circle(120, 300.0, 1.0);
        assert!(similarity(&c1, &c2) > 0.9, "{}", similarity(&c1, &c2));
        assert!(similarity(&c1, &zigzag(20)) < 0.8);
        assert!(validate(&ShapeTemplate { points: c1 }).is_none());
        assert!(validate(&ShapeTemplate { points: vec![GesturePoint::default(); 3] }).is_some());
    }
}
