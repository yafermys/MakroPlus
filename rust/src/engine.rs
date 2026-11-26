//! Low-level keyboard and mouse hooks on a dedicated thread.
//!
//! Matching rules follow Makro+ 2.x: actions fire once all keys are released
//! (3 s deadline), two-key chords use a held prefix (Del+5), Makro+'s own
//! synthesized input is ignored, and app-scoped bindings win over global ones.
use crate::keys;
use crate::model::{ActionStep, Binding};
use crate::win::{self, Hwnd};
use parking_lot::RwLock;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::OnceLock;
use std::time::{Duration, Instant};
#[cfg(windows)]
use windows_sys::Win32::{Foundation::*, System::LibraryLoader::GetModuleHandleW, System::Threading::GetCurrentThreadId, UI::WindowsAndMessaging::*};

/// Wakes the engine thread (reset after config/mode changes, or flush pending actions).
#[cfg(windows)]
mod wake {
    use std::sync::atomic::{AtomicU32, Ordering};
    use windows_sys::Win32::UI::WindowsAndMessaging::{PostThreadMessageW, WM_APP};
    pub static THREAD: AtomicU32 = AtomicU32::new(0);
    pub const WM_FLUSH: u32 = WM_APP + 10;
    pub const WM_RESET: u32 = WM_APP + 11;
    pub fn reset() {
        let t = THREAD.load(Ordering::Relaxed);
        if t != 0 {
            unsafe { PostThreadMessageW(t, WM_RESET, 0, 0) };
        }
    }
    pub fn flush() {
        let t = THREAD.load(Ordering::Relaxed);
        if t != 0 {
            unsafe { PostThreadMessageW(t, WM_FLUSH, 0, 0) };
        }
    }
}

#[cfg(not(windows))]
pub mod wake {
    use parking_lot::Mutex;
    use std::sync::mpsc::Sender;
    pub enum Msg {
        Key { key: i32, up: bool, reply: Option<std::sync::mpsc::SyncSender<bool>> },
        Pointer { key: i32, up: bool, delta: i32, horizontal: bool, x: i32, y: i32 },
        Flush,
        Reset,
    }
    pub static TX: Mutex<Option<Sender<Msg>>> = Mutex::new(None);
    pub fn send(m: Msg) {
        if let Some(tx) = TX.lock().as_ref() {
            let _ = tx.send(m);
        }
    }
    pub fn reset() {
        send(Msg::Reset)
    }
    pub fn flush() {
        send(Msg::Flush)
    }
}

#[derive(Default, Clone)]
pub struct Config {
    pub bindings: Vec<Binding>,
    pub paused: bool,
    pub allow_remapped: bool,
    pub guide_enabled: bool,
    pub guide_key: i32,
    pub guide_block: bool,
    pub guide_show: crate::model::GuideShow,
    /// Per-app accelerator lines for the shortcut guide: (app path, [(keys, label)]).
    pub guide_extra: Vec<(String, Vec<(String, String)>)>,
    pub palette: crate::model::ThemePalette,
    pub all_bindings: Vec<Binding>,
}

pub const MODE_NORMAL: u8 = 0;
pub const MODE_CAPTURE: u8 = 1;
pub const MODE_RECORD: u8 = 2;

static CONFIG: OnceLock<RwLock<Config>> = OnceLock::new();
pub static MODE: AtomicU8 = AtomicU8::new(MODE_NORMAL);
pub static RUNNING: AtomicBool = AtomicBool::new(false);

/// Last foreground window that does not belong to Makro+.
pub static LAST_FOREGROUND: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);

pub fn last_foreground() -> Hwnd {
    #[cfg(not(windows))]
    {
        let w = win::foreground();
        if w != 0 && win::window_pid(w) != std::process::id() {
            return w;
        }
    }
    LAST_FOREGROUND.load(Ordering::Relaxed)
}

#[cfg(windows)]
unsafe extern "system" fn on_foreground(_h: windows_sys::Win32::UI::Accessibility::HWINEVENTHOOK, _e: u32, hwnd: HWND, _o: i32, _c: i32, _t: u32, _ms: u32) {
    let w = hwnd as Hwnd;
    if w != 0 && win::window_pid(w) != std::process::id() {
        LAST_FOREGROUND.store(w, Ordering::Relaxed);
    }
}

pub fn config() -> &'static RwLock<Config> {
    CONFIG.get_or_init(|| RwLock::new(Config { guide_key: 163, ..Default::default() }))
}

/// Push the parts of the settings the hook thread needs.
pub fn sync(state: &crate::model::State) {
    crate::gesture::sync(state);
    let mut c = config().write();
    c.bindings = state.effective();
    c.paused = state.paused;
    c.allow_remapped = state.allow_remapped_input;
    c.guide_enabled = state.guide_enabled;
    c.guide_key = state.guide_hold_key;
    c.guide_block = state.guide_block_key;
    c.guide_show = state.guide_show.clone();
    // The guide can list shortcuts of every profile/app, so keep them all.
    c.all_bindings = state.bindings.iter().filter(|b| b.enabled).cloned().collect();
    c.guide_extra = state
        .app_inventories
        .iter()
        .map(|inv| {
            let lines = inv.commands.iter().filter(|c| !c.accelerator.is_empty()).take(18).map(|c| (c.accelerator.clone(), c.label.clone())).collect();
            (inv.app_path.clone(), lines)
        })
        .collect();
    c.palette = crate::theme::resolve(state);
    drop(c);
    wake::reset();
}

pub fn set_mode(mode: u8) {
    MODE.store(mode, Ordering::SeqCst);
    wake::reset();
}

struct Prefix {
    key: i32,
    mods: i32,
    window: Hwnd,
    used: bool,
    forwarded: bool,
}

struct Hook {
    #[allow(dead_code)]
    kb: usize,
    #[allow(dead_code)]
    ms: usize,
    pressed: HashSet<i32>,
    blocked: HashSet<i32>,
    pending: Option<(Binding, Hwnd, Instant)>,
    prefix: Option<Prefix>,
    replay: Vec<(i32, i32, Hwnd)>,
    capture_prefix: i32,
    capture_mods: i32,
    cache_window: Hwnd,
    cache_path: String,
    cache_until: Instant,
    wheel_v: i32,
    wheel_h: i32,
    wheel_window: Hwnd,
    wheel_mods: i32,
    timer: usize,
    guide_down: Option<Instant>,
    rec_last: Option<Instant>,
    rec_window: Hwnd,
    rec_mouse_down: Option<(i32, i32, i32, Instant)>,
    own_path: String,
}

impl Hook {
    fn new(kb: usize, ms: usize, own_path: String) -> Hook {
        let now = Instant::now();
        Hook {
            kb,
            ms,
            pressed: HashSet::new(),
            blocked: HashSet::new(),
            pending: None,
            prefix: None,
            replay: Vec::new(),
            capture_prefix: 0,
            capture_mods: 0,
            cache_window: 0,
            cache_path: String::new(),
            cache_until: now,
            wheel_v: 0,
            wheel_h: 0,
            wheel_window: 0,
            wheel_mods: 0,
            timer: 0,
            guide_down: None,
            rec_last: None,
            rec_window: 0,
            rec_mouse_down: None,
            own_path,
        }
    }
}

/// Linux: one engine thread receives evdev events from the device readers.
#[cfg(not(windows))]
pub fn start() {
    let (tx, rx) = std::sync::mpsc::channel::<wake::Msg>();
    *wake::TX.lock() = Some(tx);
    std::thread::Builder::new()
        .name("makro-engine".into())
        .spawn(move || {
            let own_path = std::env::current_exe().map(|p| p.display().to_string()).unwrap_or_default();
            HOOK.with(|h| *h.borrow_mut() = Some(Hook::new(0, 0, own_path)));
            RUNNING.store(crate::input::start(), Ordering::SeqCst);
            loop {
                let timer = with(|h| h.timer != 0).unwrap_or(false);
                let msg = if timer {
                    match rx.recv_timeout(Duration::from_millis(25)) {
                        Ok(m) => Some(m),
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => None,
                        Err(_) => break,
                    }
                } else {
                    match rx.recv() {
                        Ok(m) => Some(m),
                        Err(_) => break,
                    }
                };
                match msg {
                    None | Some(wake::Msg::Flush) => {
                        with(|h| h.tick());
                    }
                    Some(wake::Msg::Reset) => {
                        with(|h| h.reset());
                    }
                    Some(wake::Msg::Key { key, up, reply }) => {
                        let eat = with(|h| h.handle(key, up)).unwrap_or(false);
                        if let Some(r) = reply {
                            let _ = r.send(eat);
                        }
                    }
                    Some(wake::Msg::Pointer { key, up, delta, horizontal, x, y }) => {
                        with(|h| h.pointer(key, up, delta, horizontal, x, y));
                    }
                }
            }
        })
        .expect("engine thread");
}

thread_local! {
    static HOOK: std::cell::RefCell<Option<Hook>> = const { std::cell::RefCell::new(None) };
}

fn with<R>(f: impl FnOnce(&mut Hook) -> R) -> Option<R> {
    HOOK.with(|h| h.try_borrow_mut().ok().and_then(|mut g| g.as_mut().map(f)))
}

fn mods_of(p: &HashSet<i32>) -> i32 {
    let mut m = 0;
    if p.contains(&162) || p.contains(&163) || p.contains(&17) {
        m |= keys::MOD_CTRL;
    }
    if p.contains(&164) || p.contains(&18) {
        m |= keys::MOD_ALT;
    }
    if p.contains(&160) || p.contains(&161) || p.contains(&16) {
        m |= keys::MOD_SHIFT;
    }
    if p.contains(&91) || p.contains(&92) {
        m |= keys::MOD_WIN;
    }
    m
}

fn has_candidate(b: &[Binding], key: i32, mods: i32) -> bool {
    b.iter().any(|x| x.enabled && x.mods == mods && (x.key == key || x.prefix_key == key))
}

pub fn resolve<'a>(b: &'a [Binding], mods: i32, key: i32, app: &str, prefix: i32) -> Option<&'a Binding> {
    let matches: Vec<&Binding> = b.iter().filter(|x| x.gesture.is_none() && x.enabled && x.mods == mods && x.key == key && x.prefix_key == prefix).collect();
    matches.iter().find(|x| !x.app_path.is_empty() && win::same_path(&x.app_path, app)).or_else(|| matches.iter().find(|x| x.app_path.is_empty())).copied()
}

impl Hook {
    fn foreground_path(&mut self, w: Hwnd) -> String {
        if w != self.cache_window || Instant::now() >= self.cache_until {
            self.cache_window = w;
            self.cache_path = win::window_process_path(w);
            self.cache_until = Instant::now() + Duration::from_millis(500);
        }
        self.cache_path.clone()
    }

    fn ensure_timer(&mut self) {
        if self.timer == 0 {
            #[cfg(windows)]
            {
                self.timer = unsafe { SetTimer(std::ptr::null_mut(), 0, 25, None) };
            }
            #[cfg(not(windows))]
            {
                self.timer = 1;
            }
        }
    }

    fn post_flush(&self) {
        wake::flush();
    }

    fn queue(&mut self, b: &Binding, w: Hwnd) {
        if self.pending.is_none() {
            self.pending = Some((b.clone(), w, Instant::now() + Duration::from_secs(3)));
            self.ensure_timer();
            if self.pressed.is_empty() {
                self.post_flush();
            }
        }
    }

    fn reset(&mut self) {
        self.pending = None;
        self.replay.clear();
        if let Some(p) = &mut self.prefix {
            p.used = true;
        }
        self.cache_until = Instant::now();
        self.capture_prefix = 0;
        self.rec_last = None;
        self.rec_window = 0;
        if self.guide_down.take().is_some() {
            crate::guide::hide();
        }
    }

    fn record(&mut self, step: ActionStep) {
        let now = Instant::now();
        let fg = win::foreground();
        if fg != self.rec_window && fg != 0 {
            let path = win::window_process_path(fg);
            if !path.is_empty() {
                let mut focus = ActionStep::new("Focus");
                focus.app_path = Some(path);
                focus.window_title = Some(win::window_title(fg));
                focus.window_class = Some(win::window_class(fg));
                crate::send_ui(crate::UiEvent::Recorded(focus));
            }
            self.rec_window = fg;
        } else if let Some(last) = self.rec_last {
            let gap = now.duration_since(last).as_millis() as i32;
            if gap > 150 {
                let mut wait = ActionStep::new("Wait");
                wait.delay = gap.min(60_000);
                crate::send_ui(crate::UiEvent::Recorded(wait));
            }
        }
        self.rec_last = Some(now);
        crate::send_ui(crate::UiEvent::Recorded(step));
    }

    fn emit_capture(&self, mods: i32, key: i32, prefix: i32) {
        crate::send_ui(crate::UiEvent::Captured { mods, key, prefix });
    }

    /// Returns true when the event must be swallowed.
    fn handle(&mut self, key: i32, up: bool) -> bool {
        let cfg = config().read();
        if key == 27 && !up && crate::actions::WORKFLOW_RUNNING.load(Ordering::Relaxed) {
            crate::actions::CANCEL.store(true, Ordering::SeqCst);
        }
        let repeat = self.pressed.contains(&key);
        if up {
            self.pressed.remove(&key);
        } else {
            self.pressed.insert(key);
        }
        if up && self.pressed.is_empty() && (self.pending.is_some() || !self.replay.is_empty()) {
            self.post_flush();
        }
        let eat_blocked = self.blocked.contains(&key);
        if up {
            self.blocked.remove(&key);
        }
        // Shortcut guide: hold the guide key alone.
        if cfg.guide_enabled && !keys::is_pointer(key) {
            if key == cfg.guide_key {
                if !up && !repeat && self.pressed.len() == 1 {
                    self.guide_down = Some(Instant::now());
                    self.ensure_timer();
                } else if up && self.guide_down.take().is_some() {
                    crate::guide::hide();
                }
                if cfg.guide_block && MODE.load(Ordering::Relaxed) == MODE_NORMAL {
                    // Guide key reserved for the guide: its normal function is off.
                    if up {
                        self.pressed.remove(&key);
                    }
                    return true;
                }
            } else if self.guide_down.take().is_some() {
                crate::guide::hide();
            }
        }
        let mode = MODE.load(Ordering::Relaxed);
        if mode == MODE_CAPTURE {
            if !keys::is_modifier(key) {
                let mods = mods_of(&self.pressed);
                if !up && !repeat {
                    if key == 27 {
                        self.capture_prefix = 0;
                        self.emit_capture(mods, key, 0);
                    } else if self.capture_prefix > 0 && self.capture_prefix != key {
                        let p = self.capture_prefix;
                        self.capture_prefix = 0;
                        self.emit_capture(self.capture_mods | mods, key, p);
                    } else if keys::prefix_allowed(key) {
                        // Remember modifiers now: they are often released first.
                        self.capture_prefix = key;
                        self.capture_mods = mods;
                    } else {
                        self.emit_capture(mods, key, 0);
                    }
                } else if up && self.capture_prefix == key {
                    self.capture_prefix = 0;
                    self.emit_capture(self.capture_mods | mods, key, 0);
                }
            }
            if !up {
                self.blocked.insert(key);
            }
            return true;
        }
        if mode == MODE_RECORD {
            if key == 19 {
                // Pause/Break stops recording and is not recorded.
                if !up {
                    crate::send_ui(crate::UiEvent::ShowPage("recorder-stop".into()));
                }
                return true;
            }
            let fg = win::foreground();
            if win::window_pid(fg) != std::process::id() && !up && !repeat && !keys::is_modifier(key) && !keys::is_pointer(key) {
                let mut s = ActionStep::new("Key");
                s.mods = mods_of(&self.pressed);
                s.key = key;
                s.alt_gr = self.pressed.contains(&165);
                drop(cfg);
                self.record(s);
            }
            return false;
        }
        // Held prefix key events.
        if let Some(p) = &mut self.prefix {
            if key == p.key {
                if !up {
                    return !p.forwarded;
                }
                let forwarded = p.forwarded;
                if !p.used && !forwarded && !cfg.paused {
                    self.replay.push((p.mods, p.key, p.window));
                    self.ensure_timer();
                }
                self.prefix = None;
                return !forwarded;
            }
        }
        if eat_blocked {
            return true;
        }
        if up || repeat || cfg.paused || keys::is_modifier(key) || self.pressed.contains(&165) {
            return false;
        }
        let window = win::foreground();
        let mods = mods_of(&self.pressed);
        let pointer_prefix = keys::is_pointer(key) && self.prefix.as_ref().map(|p| !p.forwarded).unwrap_or(false);
        let app = if has_candidate(&cfg.bindings, key, mods) || pointer_prefix || self.prefix.is_some() { self.foreground_path(window) } else { String::new() };
        if keys::is_pointer(key) && win::same_path(&app, &self.own_path) {
            return false;
        }
        if let Some(p) = &mut self.prefix {
            if !p.forwarded {
                if window != p.window {
                    p.used = true;
                    return false;
                }
                if let Some(chord) = resolve(&cfg.bindings, mods, key, &app, p.key) {
                    if mods == p.mods {
                        p.used = true;
                        let chord = chord.clone();
                        drop(cfg);
                        self.queue(&chord, window);
                        self.blocked.insert(key);
                        return true;
                    }
                }
                if !p.used && mods != p.mods {
                    p.used = true;
                    self.replay.push((p.mods, p.key, p.window));
                    self.ensure_timer();
                    return false;
                }
                if !p.used {
                    p.forwarded = true;
                    let _ = win::send_key_raw(p.key, false);
                    let _ = win::send_key_raw(key, false);
                    return true;
                }
            }
            return false;
        }
        if cfg.bindings.iter().any(|b| b.enabled && b.mods == mods && b.prefix_key == key && (b.app_path.is_empty() || win::same_path(&b.app_path, &app))) {
            self.prefix = Some(Prefix { key, mods, window, used: false, forwarded: false });
            return true;
        }
        if let Some(b) = resolve(&cfg.bindings, mods, key, &app, 0) {
            if b.kind == "Disable key" {
                // The key's own function is switched off; nothing runs.
                self.blocked.insert(key);
                return true;
            }
            let b = b.clone();
            drop(cfg);
            self.queue(&b, window);
            if !b.pass_through {
                self.blocked.insert(key);
                return true;
            }
        }
        false
    }

    fn would_handle_pointer(&mut self, key: i32) -> bool {
        let cfg = config().read();
        if cfg.paused || self.pressed.contains(&165) {
            return false;
        }
        let mods = mods_of(&self.pressed);
        if !has_candidate(&cfg.bindings, key, mods) {
            return false;
        }
        let w = win::foreground();
        let app = self.foreground_path(w);
        if win::same_path(&app, &self.own_path) {
            return false;
        }
        let prefix = self.prefix.as_ref().filter(|p| !p.forwarded && p.mods == mods).map(|p| p.key).unwrap_or(0);
        resolve(&cfg.bindings, mods, key, &app, prefix).is_some()
    }

    fn tick(&mut self) {
        let cfg = config().read().clone();
        let mode = MODE.load(Ordering::Relaxed);
        if cfg.paused || mode != MODE_NORMAL {
            self.replay.clear();
        } else if self.pressed.is_empty() && !self.replay.is_empty() {
            for (m, k, w) in std::mem::take(&mut self.replay) {
                if win::foreground() == w {
                    let _ = win::send_chord(m, k, false);
                }
            }
        }
        if let Some((b, w, deadline)) = self.pending.take() {
            if cfg.paused || mode != MODE_NORMAL {
            } else if self.pressed.is_empty() {
                crate::actions::submit(b, w);
            } else if Instant::now() > deadline {
                self.pressed.retain(|&k| keys::is_pointer(k) || win::key_down(k));
                let pressed = self.pressed.clone();
                self.blocked.retain(|k| pressed.contains(k));
                crate::actions::log(
                    crate::i18n::t("Input engine", "Giriş motoru"),
                    crate::i18n::t("Keys were not released within 3 seconds; the action was cancelled.", "Tuşlar 3 saniye içinde bırakılmadı; eylem iptal edildi."),
                );
            } else {
                self.pending = Some((b, w, deadline));
            }
        }
        if let Some(t0) = self.guide_down {
            if t0.elapsed() >= Duration::from_millis(500) && !crate::guide::visible() && self.pressed.len() == 1 && self.pressed.contains(&cfg.guide_key) {
                let fg = win::foreground();
                crate::guide::show(fg, &cfg, &self.foreground_path(fg));
            }
        }
        if self.pending.is_none() && self.replay.is_empty() && self.guide_down.is_none() && self.timer != 0 {
            #[cfg(windows)]
            unsafe {
                KillTimer(std::ptr::null_mut(), self.timer);
            }
            self.timer = 0;
        }
    }

    #[cfg(windows)]
    fn mouse(&mut self, msg: u32, data: &MSLLHOOKSTRUCT) -> bool {
        let cfg_allow = config().read().allow_remapped;
        if data.dwExtraInfo == win::INJECT_MARK || (data.flags & 1 != 0 && !cfg_allow) {
            return false;
        }
        let hi = (data.mouseData >> 16) as u16;
        let (key, up) = match msg {
            WM_LBUTTONDOWN => (1, false),
            WM_LBUTTONUP => (1, true),
            WM_RBUTTONDOWN => (2, false),
            WM_RBUTTONUP => (2, true),
            WM_MBUTTONDOWN => (4, false),
            WM_MBUTTONUP => (4, true),
            WM_XBUTTONDOWN | WM_XBUTTONUP => {
                if hi != 1 && hi != 2 {
                    return false;
                }
                (if hi == 1 { 5 } else { 6 }, msg == WM_XBUTTONUP)
            }
            WM_MOUSEWHEEL | WM_MOUSEHWHEEL => {
                let delta = hi as i16 as i32;
                if delta == 0 {
                    return false;
                }
                let k = if msg == WM_MOUSEWHEEL {
                    if delta > 0 { keys::WHEEL_UP } else { keys::WHEEL_DOWN }
                } else if delta > 0 {
                    keys::WHEEL_RIGHT
                } else {
                    keys::WHEEL_LEFT
                };
                (k, false)
            }
            _ => return false,
        };
        self.pointer(key, up, hi as i16 as i32, msg == WM_MOUSEHWHEEL, data.pt.x, data.pt.y)
    }

    /// Mouse button or wheel event (wheel `delta` in 120ths). Returns true to swallow it.
    pub(crate) fn pointer(&mut self, key: i32, up: bool, delta: i32, horizontal: bool, px: i32, py: i32) -> bool {
        let mode = MODE.load(Ordering::Relaxed);
        if mode == MODE_RECORD {
            let fg = win::foreground();
            if win::window_pid(fg) == std::process::id() {
                return false;
            }
            let rect = win::window_rect(fg);
                        if keys::is_wheel(key) {
                if let Some(r) = rect {
                    let mut s = ActionStep::new("Scroll");
                    s.x = (px - r.0) as f64;
                    s.y = (py - r.1) as f64;
                    s.window_width = (r.2 - r.0) as f64;
                    s.window_height = (r.3 - r.1) as f64;
                    s.scroll_delta = delta;
                    s.horizontal = horizontal;
                    s.scale_with_window = true;
                    self.record(s);
                }
            } else if !up {
                self.rec_mouse_down = Some((key, px, py, Instant::now()));
            } else if let (Some((k0, x0, y0, t0)), Some(r)) = (self.rec_mouse_down.take(), rect) {
                if k0 == key {
                    let moved = (px - x0).abs() > 8 || (py - y0).abs() > 8;
                    let mut s = ActionStep::new(if moved { "Mouse drag" } else { "Mouse click" });
                    s.mouse_button = match key {
                        2 => 1,
                        4 => 2,
                        5 => 3,
                        6 => 4,
                        _ => 0,
                    };
                    s.x = (x0 - r.0) as f64;
                    s.y = (y0 - r.1) as f64;
                    s.x2 = (px - r.0) as f64;
                    s.y2 = (py - r.1) as f64;
                    s.window_width = (r.2 - r.0) as f64;
                    s.window_height = (r.3 - r.1) as f64;
                    s.duration = t0.elapsed().as_millis().min(30_000) as i32;
                    if !moved && s.duration < 250 {
                        s.duration = 0;
                    }
                    s.scale_with_window = true;
                    self.record(s);
                }
            }
            return false;
        }
        if mode == MODE_CAPTURE && key == 1 {
            return false; // keep the left button usable for the dialog
        }
        if keys::is_wheel(key) && mode == MODE_NORMAL {
            let w = win::foreground();
            let mods = mods_of(&self.pressed);
            if self.wheel_window != w || self.wheel_mods != mods {
                self.wheel_v = 0;
                self.wheel_h = 0;
                self.wheel_window = w;
                self.wheel_mods = mods;
            }
            if !self.would_handle_pointer(key) {
                if !horizontal {
                    self.wheel_v = 0;
                } else {
                    self.wheel_h = 0;
                }
                return false;
            }
            let total = if !horizontal { self.wheel_v } else { self.wheel_h } + delta;
            let complete = total.abs() >= 120;
            if !horizontal {
                self.wheel_v = total % 120;
            } else {
                self.wheel_h = total % 120;
            }
            if !complete {
                return true;
            }
        }
        let mut eat = self.handle(key, up);
        if keys::is_wheel(key) {
            eat = self.handle(key, true) || eat;
        }
        eat
    }
}

/// `--debug-input`: append every raw keyboard event to data/input-debug.log.
#[cfg(windows)]
fn debug_input(line: String) {
    static ON: OnceLock<bool> = OnceLock::new();
    if *ON.get_or_init(|| std::env::args().any(|a| a == "--debug-input")) {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(crate::storage::data_dir().join("input-debug.log")) {
            let _ = writeln!(f, "{line}");
        }
    }
}

#[cfg(windows)]
unsafe extern "system" fn kb_proc(code: i32, w: WPARAM, l: LPARAM) -> LRESULT {
    if code >= 0 {
        let e = &*(l as *const KBDLLHOOKSTRUCT);
        debug_input(format!("vk={} flags={:#x} extra={:#x} mode={} hook_ready={}", e.vkCode, e.flags, e.dwExtraInfo, MODE.load(Ordering::Relaxed), with(|_| ()).is_some()));
        let allow = config().read().allow_remapped;
        if e.dwExtraInfo != win::INJECT_MARK && (e.flags & LLKHF_INJECTED == 0 || allow) {
            let up = e.flags & LLKHF_UP != 0 || w as u32 == WM_KEYUP || w as u32 == WM_SYSKEYUP;
            if with(|h| h.handle(e.vkCode as i32, up)).unwrap_or(false) {
                return 1;
            }
        }
    }
    CallNextHookEx(std::ptr::null_mut(), code, w, l)
}

#[cfg(windows)]
unsafe extern "system" fn ms_proc(code: i32, w: WPARAM, l: LPARAM) -> LRESULT {
    if code >= 0 && w as u32 != WM_MOUSEMOVE {
        let e = &*(l as *const MSLLHOOKSTRUCT);
        if with(|h| h.mouse(w as u32, e)).unwrap_or(false) {
            return 1;
        }
    }
    CallNextHookEx(std::ptr::null_mut(), code, w, l)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::Receiver;

    fn hook() -> Hook {
        let now = Instant::now();
        Hook {
            kb: 0,
            ms: 0,
            pressed: HashSet::new(),
            blocked: HashSet::new(),
            pending: None,
            prefix: None,
            replay: vec![],
            capture_prefix: 0,
            capture_mods: 0,
            cache_window: 0,
            cache_path: String::new(),
            cache_until: now,
            wheel_v: 0,
            wheel_h: 0,
            wheel_window: 0,
            wheel_mods: 0,
            timer: 0,
            guide_down: None,
            rec_last: None,
            rec_window: 0,
            rec_mouse_down: None,
            own_path: String::new(),
        }
    }

    fn captured(rx: &Receiver<crate::UiEvent>) -> Vec<(i32, i32, i32)> {
        let mut v = vec![];
        while let Ok(e) = rx.try_recv() {
            if let crate::UiEvent::Captured { mods, key, prefix } = e {
                v.push((mods, key, prefix));
            }
        }
        v
    }

    // One test so the shared MODE/CONFIG globals are not raced by parallel tests.
    #[test]
    fn capture_and_matching() {
        let rx = crate::test_events();
        captured(&rx);
        config().write().guide_enabled = false;
        MODE.store(MODE_CAPTURE, Ordering::SeqCst);
        let mut h = hook();
        // Ctrl + Alt + T
        assert!(h.handle(162, false));
        assert!(h.handle(164, false));
        assert!(h.handle(84, false));
        assert!(h.handle(84, true));
        assert!(h.handle(164, true));
        assert!(h.handle(162, true));
        assert_eq!(captured(&rx), vec![(3, 84, 0)]);
        // F8 alone (prefix-capable key) is emitted on release.
        h.handle(119, false);
        assert!(captured(&rx).is_empty());
        h.handle(119, true);
        assert_eq!(captured(&rx), vec![(0, 119, 0)]);
        // Del then 5 → two-key chord.
        h.handle(46, false);
        h.handle(53, false);
        h.handle(53, true);
        h.handle(46, true);
        assert_eq!(captured(&rx), vec![(0, 53, 46)]);
        // Ctrl held, F6 pressed, Ctrl released before F6: modifier must survive.
        h.handle(162, false);
        h.handle(117, false);
        h.handle(162, true);
        h.handle(117, true);
        assert_eq!(captured(&rx), vec![(2, 117, 0)]);

        // Normal mode: a global binding is matched and its key swallowed.
        MODE.store(MODE_NORMAL, Ordering::SeqCst);
        let mut h = hook();
        config().write().bindings = vec![
            Binding { name: "x".into(), mods: 3, key: 120, kind: "System".into(), target: "lock".into(), ..Default::default() },
            Binding { name: "off".into(), key: 45, kind: "Disable key".into(), ..Default::default() },
        ];
        config().write().paused = false;
        assert!(h.handle(162, false) == false);
        assert!(!h.handle(164, false));
        assert!(h.handle(120, false), "bound key must be swallowed");
        assert!(h.pending.is_some());
        assert!(h.handle(120, true));
        h.pending = None;
        h.handle(164, true);
        h.handle(162, true);
        // Unbound key passes through.
        assert!(!h.handle(121, false));
        h.handle(121, true);
        // Disabled key is swallowed and runs nothing.
        assert!(h.handle(45, false));
        assert!(h.pending.is_none());
        assert!(h.handle(45, true));
        config().write().bindings.clear();
    }
}

#[cfg(windows)]
pub fn start() {
    std::thread::Builder::new()
        .name("makro-hooks".into())
        .spawn(|| unsafe {
            let module = GetModuleHandleW(std::ptr::null());
            let kb = SetWindowsHookExW(WH_KEYBOARD_LL, Some(kb_proc), module, 0);
            let ms = SetWindowsHookExW(WH_MOUSE_LL, Some(ms_proc), module, 0);
            if kb.is_null() {
                crate::actions::log("Makro+", crate::i18n::t("Keyboard hook could not be installed.", "Klavye kancası kurulamadı."));
            }
            let own_path = std::env::current_exe().map(|p| p.display().to_string()).unwrap_or_default();
            HOOK.with(|h| *h.borrow_mut() = Some(Hook::new(kb as usize, ms as usize, own_path)));

            // EVENT_SYSTEM_FOREGROUND, delivered to this thread's message loop.
            windows_sys::Win32::UI::Accessibility::SetWinEventHook(3, 3, std::ptr::null_mut(), Some(on_foreground), 0, 0, 0);
            let fg = win::foreground();
            if win::window_pid(fg) != std::process::id() {
                LAST_FOREGROUND.store(fg, Ordering::Relaxed);
            }
            wake::THREAD.store(GetCurrentThreadId(), Ordering::SeqCst);
            RUNNING.store(true, Ordering::SeqCst);
            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                match msg.message {
                    WM_TIMER | wake::WM_FLUSH if msg.hwnd.is_null() => {
                        with(|h| h.tick());
                    }
                    wake::WM_RESET => {
                        with(|h| h.reset());
                    }
                    _ => {
                        TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                }
            }
            with(|h| {
                UnhookWindowsHookEx(h.kb as _);
                UnhookWindowsHookEx(h.ms as _);
            });
        })
        .expect("hook thread");
}
