//! Linux input: evdev readers (keyboard, mouse, touchpad) and a uinput device
//! for synthetic keys and clicks. Works on X11 and Wayland.
//!
//! Requires read access to /dev/input/event* and write access to /dev/uinput
//! (the packages install a udev rule; the user must be in the `input` group).
use evdev::{uinput::VirtualDevice, AttributeSet, Device, EventType, InputEvent, InputEventKind, Key, RelativeAxisType};
use parking_lot::Mutex;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::time::Duration;

pub const VIRTUAL_NAME: &str = "Makro+ virtual input";

/// Exclusive mode: keyboards are grabbed so matched keys can be swallowed.
pub static GRAB: AtomicBool = AtomicBool::new(false);
static PRESSED: Mutex<Option<HashSet<i32>>> = Mutex::new(None);

/// (evdev code, Windows virtual-key) pairs.
const MAP: &[(u16, i32)] = &[
    (1, 27), (2, 49), (3, 50), (4, 51), (5, 52), (6, 53), (7, 54), (8, 55), (9, 56), (10, 57), (11, 48), (12, 189), (13, 187), (14, 8), (15, 9),
    (16, 81), (17, 87), (18, 69), (19, 82), (20, 84), (21, 89), (22, 85), (23, 73), (24, 79), (25, 80), (26, 219), (27, 221), (28, 13), (29, 162),
    (30, 65), (31, 83), (32, 68), (33, 70), (34, 71), (35, 72), (36, 74), (37, 75), (38, 76), (39, 186), (40, 222), (41, 192), (42, 160), (43, 220),
    (44, 90), (45, 88), (46, 67), (47, 86), (48, 66), (49, 78), (50, 77), (51, 188), (52, 190), (53, 191), (54, 161), (55, 106), (56, 164), (57, 32),
    (58, 20), (59, 112), (60, 113), (61, 114), (62, 115), (63, 116), (64, 117), (65, 118), (66, 119), (67, 120), (68, 121), (69, 144), (70, 145),
    (71, 103), (72, 104), (73, 105), (74, 109), (75, 100), (76, 101), (77, 102), (78, 107), (79, 97), (80, 98), (81, 99), (82, 96), (83, 110),
    (86, 226), (87, 122), (88, 123), (97, 163), (98, 111), (99, 44), (100, 165), (102, 36), (103, 38), (104, 33), (105, 37), (106, 39), (107, 35),
    (108, 40), (109, 34), (110, 45), (111, 46), (113, 173), (114, 174), (115, 175), (119, 19), (125, 91), (126, 92), (127, 93), (140, 183),
    (142, 95), (155, 180), (156, 171), (158, 166), (159, 167), (163, 176), (164, 179), (165, 177), (166, 178), (172, 172), (173, 168), (217, 170),
    (183, 124), (184, 125), (185, 126), (186, 127), (187, 128), (188, 129), (189, 130), (190, 131), (191, 132), (192, 133), (193, 134), (194, 135),
];

pub fn to_vk(code: u16) -> Option<i32> {
    MAP.iter().find(|m| m.0 == code).map(|m| m.1)
}

pub fn to_code(vk: i32) -> Option<u16> {
    match vk {
        16 => Some(42),
        17 => Some(29),
        18 => Some(56),
        _ => MAP.iter().find(|m| m.1 == vk).map(|m| m.0),
    }
}

fn mouse_key(code: u16) -> Option<i32> {
    match code {
        0x110 => Some(1),
        0x111 => Some(2),
        0x112 => Some(4),
        0x113 => Some(5),
        0x114 => Some(6),
        _ => None,
    }
}

pub fn is_down(vk: i32) -> bool {
    PRESSED.lock().as_ref().map(|p| p.contains(&vk)).unwrap_or(false)
}

fn set_pressed(vk: i32, down: bool) {
    let mut g = PRESSED.lock();
    let p = g.get_or_insert_with(HashSet::new);
    if down {
        p.insert(vk);
    } else {
        p.remove(&vk);
    }
}

// ---------------- synthetic input ----------------

fn virtual_device() -> &'static Mutex<Option<VirtualDevice>> {
    static V: OnceLock<Mutex<Option<VirtualDevice>>> = OnceLock::new();
    V.get_or_init(|| {
        let build = || -> std::io::Result<VirtualDevice> {
            let mut keys = AttributeSet::<Key>::new();
            for (code, _) in MAP {
                keys.insert(Key::new(*code));
            }
            for c in [0x110u16, 0x111, 0x112, 0x113, 0x114] {
                keys.insert(Key::new(c));
            }
            let mut rel = AttributeSet::<RelativeAxisType>::new();
            for r in [RelativeAxisType::REL_X, RelativeAxisType::REL_Y, RelativeAxisType::REL_WHEEL, RelativeAxisType::REL_HWHEEL] {
                rel.insert(r);
            }
            let dev = evdev::uinput::VirtualDeviceBuilder::new()?.name(VIRTUAL_NAME).with_keys(&keys)?.with_relative_axes(&rel)?.build()?;
            // Give the compositor a moment to pick up the new device.
            std::thread::sleep(Duration::from_millis(250));
            Ok(dev)
        };
        Mutex::new(build().ok())
    })
}

pub fn can_send() -> bool {
    virtual_device().lock().is_some()
}

fn emit(events: &[InputEvent]) -> Result<(), String> {
    let mut g = virtual_device().lock();
    let dev = g.as_mut().ok_or_else(|| crate::i18n::t("Makro+ cannot write to /dev/uinput. See Settings › Linux permissions.", "Makro+ /dev/uinput'a yazamıyor. Ayarlar › Linux izinleri bölümüne bakın.").to_string())?;
    dev.emit(events).map_err(|e| e.to_string())
}

fn key_ev(code: u16, down: bool) -> InputEvent {
    InputEvent::new(EventType::KEY, code, if down { 1 } else { 0 })
}

pub fn send_key_raw(vk: i32, up: bool) -> Result<(), String> {
    let code = to_code(vk).ok_or("unknown key")?;
    emit(&[key_ev(code, !up)])
}

pub fn send_chord(mods: i32, key: i32, altgr: bool) -> Result<(), String> {
    if crate::keys::is_pointer(key) {
        return send_pointer(mods, key);
    }
    let mut pre = Vec::new();
    if altgr {
        pre.push(100);
    } else {
        if mods & 2 != 0 {
            pre.push(29);
        }
        if mods & 1 != 0 {
            pre.push(56);
        }
    }
    if mods & 4 != 0 {
        pre.push(42);
    }
    if mods & 8 != 0 {
        pre.push(125);
    }
    let code = to_code(key).ok_or_else(|| format!("key {key} has no Linux code"))?;
    let mut ev: Vec<InputEvent> = pre.iter().map(|&c| key_ev(c, true)).collect();
    ev.push(key_ev(code, true));
    emit(&ev)?;
    std::thread::sleep(Duration::from_millis(8));
    let mut ev = vec![key_ev(code, false)];
    ev.extend(pre.iter().rev().map(|&c| key_ev(c, false)));
    emit(&ev)
}

pub fn send_pointer(mods: i32, key: i32) -> Result<(), String> {
    let mut pre = Vec::new();
    for (bit, c) in [(2, 29u16), (1, 56), (4, 42), (8, 125)] {
        if mods & bit != 0 {
            pre.push(c);
        }
    }
    let mut ev: Vec<InputEvent> = pre.iter().map(|&c| key_ev(c, true)).collect();
    match key {
        1 | 2 | 4 | 5 | 6 => {
            let code = match key {
                1 => 0x110,
                2 => 0x111,
                4 => 0x112,
                5 => 0x113,
                _ => 0x114,
            };
            ev.push(key_ev(code, true));
            ev.push(key_ev(code, false));
        }
        k => {
            let (axis, v) = match k {
                crate::keys::WHEEL_UP => (RelativeAxisType::REL_WHEEL, 1),
                crate::keys::WHEEL_DOWN => (RelativeAxisType::REL_WHEEL, -1),
                crate::keys::WHEEL_RIGHT => (RelativeAxisType::REL_HWHEEL, 1),
                _ => (RelativeAxisType::REL_HWHEEL, -1),
            };
            ev.push(InputEvent::new(EventType::RELATIVE, axis.0, v));
        }
    }
    ev.extend(pre.iter().rev().map(|&c| key_ev(c, false)));
    emit(&ev)
}

fn has(cmd: &str) -> bool {
    std::env::var_os("PATH").map(|p| std::env::split_paths(&p).any(|d| d.join(cmd).exists())).unwrap_or(false)
}

/// Type text: wtype (Wayland) or xdotool (X11), otherwise paste via the clipboard.
pub fn type_text(text: &str) -> Result<(), String> {
    let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some();
    if wayland && has("wtype") {
        if std::process::Command::new("wtype").arg("--").arg(text).status().map(|s| s.success()).unwrap_or(false) {
            return Ok(());
        }
    }
    if !wayland && has("xdotool") {
        if std::process::Command::new("xdotool").args(["type", "--delay", "2", "--"]).arg(text).status().map(|s| s.success()).unwrap_or(false) {
            return Ok(());
        }
    }
    let previous = crate::clipboard::get_text();
    crate::shell::suppress_clipboard(1500);
    crate::clipboard::set_text(text)?;
    std::thread::sleep(Duration::from_millis(60));
    send_chord(2, 86, false)?;
    if let Some(p) = previous {
        std::thread::sleep(Duration::from_millis(350));
        let _ = crate::clipboard::set_text(&p);
    }
    Ok(())
}

/// Click/drag/scroll at a screen position (needs X11 to place the pointer).
pub fn mouse_at(kind: &str, button: i32, x: i32, y: i32, x2: i32, y2: i32, delta: i32, horizontal: bool, hold_ms: u64) -> Result<(), String> {
    if !crate::win::warp_pointer(x, y) {
        return Err(crate::i18n::t("Positioned mouse steps need an X11 session on Linux.", "Konumlu fare adımları Linux'ta X11 oturumu gerektirir.").into());
    }
    std::thread::sleep(Duration::from_millis(20));
    let code = match button {
        1 => 0x111,
        2 => 0x112,
        3 => 0x113,
        4 => 0x114,
        _ => 0x110,
    };
    match kind {
        "Scroll" => {
            let axis = if horizontal { RelativeAxisType::REL_HWHEEL } else { RelativeAxisType::REL_WHEEL };
            emit(&[InputEvent::new(EventType::RELATIVE, axis.0, (delta / 120).max(1) * delta.signum())])
        }
        "Mouse drag" => {
            emit(&[key_ev(code, true)])?;
            for i in 1..=12 {
                crate::win::warp_pointer(x + (x2 - x) * i / 12, y + (y2 - y) * i / 12);
                std::thread::sleep(Duration::from_millis((hold_ms / 12).clamp(5, 60)));
            }
            emit(&[key_ev(code, false)])
        }
        _ => {
            emit(&[key_ev(code, true)])?;
            if hold_ms > 0 {
                std::thread::sleep(Duration::from_millis(hold_ms.min(30_000)));
            }
            emit(&[key_ev(code, false)])
        }
    }
}

// ---------------- readers ----------------

#[derive(PartialEq)]
enum Kind {
    Keyboard,
    Mouse,
    Touchpad,
}

fn classify(d: &Device) -> Option<Kind> {
    if d.name() == Some(VIRTUAL_NAME) {
        return None;
    }
    let keys = d.supported_keys();
    let has_key = |k: Key| keys.map(|s| s.contains(k)).unwrap_or(false);
    let abs = d.supported_absolute_axes();
    if has_key(Key::BTN_TOOL_FINGER) && abs.map(|a| a.contains(evdev::AbsoluteAxisType::ABS_X)).unwrap_or(false) && !has_key(Key::BTN_TOOL_PEN) {
        return Some(Kind::Touchpad);
    }
    if has_key(Key::KEY_A) && has_key(Key::KEY_SPACE) {
        return Some(Kind::Keyboard);
    }
    if has_key(Key::BTN_LEFT) && d.supported_relative_axes().map(|r| r.contains(RelativeAxisType::REL_X)).unwrap_or(false) {
        return Some(Kind::Mouse);
    }
    None
}

fn keyboard_thread(mut dev: Device) {
    let grab = GRAB.load(Ordering::Relaxed) && can_send();
    if grab && dev.grab().is_err() {
        crate::actions::log("Makro+", crate::i18n::t("A keyboard could not be grabbed; keys will not be swallowed.", "Bir klavye yakalanamadı; tuşlar yutulmayacak."));
    }
    let grabbed = grab;
    // Keys whose press was swallowed: their repeats and release are swallowed too.
    let mut eaten: HashSet<u16> = HashSet::new();
    loop {
        let events: Vec<InputEvent> = match dev.fetch_events() {
            Ok(it) => it.collect(),
            Err(_) => return,
        };
        let mut forward = Vec::new();
        for ev in events {
            if let InputEventKind::Key(k) = ev.kind() {
                if ev.value() == 2 {
                    // Auto-repeat: pass through unless the key is swallowed.
                    if grabbed && !eaten.contains(&k.code()) {
                        forward.push(ev);
                    }
                    continue;
                }
                let Some(vk) = to_vk(k.code()) else {
                    if grabbed {
                        forward.push(ev);
                    }
                    continue;
                };
                let up = ev.value() == 0;
                set_pressed(vk, !up);
                if grabbed {
                    let (tx, rx) = std::sync::mpsc::sync_channel(1);
                    crate::engine::wake::send(crate::engine::wake::Msg::Key { key: vk, up, reply: Some(tx) });
                    // Never hold the keyboard hostage: undecided keys pass through.
                    let eat = rx.recv_timeout(Duration::from_millis(200)).unwrap_or(false);
                    if up {
                        eaten.remove(&k.code());
                    } else if eat {
                        eaten.insert(k.code());
                    }
                    if !eat {
                        forward.push(ev);
                    }
                } else {
                    crate::engine::wake::send(crate::engine::wake::Msg::Key { key: vk, up, reply: None });
                }
            } else if grabbed && ev.event_type() != EventType::SYNCHRONIZATION {
                forward.push(ev);
            }
        }
        if grabbed && !forward.is_empty() {
            let _ = emit(&forward);
        }
    }
}

fn mouse_thread(mut dev: Device) {
    loop {
        let events: Vec<InputEvent> = match dev.fetch_events() {
            Ok(it) => it.collect(),
            Err(_) => return,
        };
        for ev in events {
            match ev.kind() {
                InputEventKind::Key(k) => {
                    if let Some(key) = mouse_key(k.code()) {
                        let up = ev.value() == 0;
                        set_pressed(key, !up);
                        let (x, y) = crate::win::cursor_pos();
                        crate::engine::wake::send(crate::engine::wake::Msg::Pointer { key, up, delta: 0, horizontal: false, x, y });
                    }
                }
                InputEventKind::RelAxis(a) if a == RelativeAxisType::REL_WHEEL || a == RelativeAxisType::REL_HWHEEL => {
                    let horizontal = a == RelativeAxisType::REL_HWHEEL;
                    let v = ev.value();
                    if v == 0 {
                        continue;
                    }
                    let key = match (horizontal, v > 0) {
                        (false, true) => crate::keys::WHEEL_UP,
                        (false, false) => crate::keys::WHEEL_DOWN,
                        (true, true) => crate::keys::WHEEL_RIGHT,
                        (true, false) => crate::keys::WHEEL_LEFT,
                    };
                    let (x, y) = crate::win::cursor_pos();
                    crate::engine::wake::send(crate::engine::wake::Msg::Pointer { key, up: false, delta: v * 120, horizontal, x, y });
                }
                _ => {}
            }
        }
    }
}

fn touchpad_thread(mut dev: Device) {
    let name = dev.name().unwrap_or("touchpad").to_string();
    let (mut minx, mut maxx) = (0, 1000);
    if let Ok(abs) = dev.get_abs_state() {
        let a = abs[evdev::AbsoluteAxisType::ABS_X.0 as usize];
        if a.maximum > a.minimum {
            minx = a.minimum;
            maxx = a.maximum;
        }
    }
    let scale = 1000.0 / (maxx - minx).max(1) as f64;
    let (mut touching, mut fingers, mut last): (bool, i32, Option<(i32, i32)>) = (false, 0, None);
    let (mut x, mut y) = (0, 0);
    loop {
        let events: Vec<InputEvent> = match dev.fetch_events() {
            Ok(it) => it.collect(),
            Err(_) => return,
        };
        let mut buttons = false;
        for ev in events {
            match ev.kind() {
                InputEventKind::Key(k) => match k {
                    Key::BTN_TOUCH => {
                        touching = ev.value() != 0;
                        if !touching {
                            last = None;
                        }
                    }
                    Key::BTN_TOOL_FINGER => fingers = if ev.value() != 0 { 1 } else { 0 },
                    Key::BTN_TOOL_DOUBLETAP | Key::BTN_TOOL_TRIPLETAP | Key::BTN_TOOL_QUADTAP => {
                        if ev.value() != 0 {
                            fingers = 2;
                        }
                    }
                    Key::BTN_LEFT | Key::BTN_RIGHT | Key::BTN_MIDDLE => buttons = true,
                    _ => {}
                },
                InputEventKind::AbsAxis(a) if a == evdev::AbsoluteAxisType::ABS_X => x = ev.value(),
                InputEventKind::AbsAxis(a) if a == evdev::AbsoluteAxisType::ABS_Y => y = ev.value(),
                InputEventKind::Synchronization(_) => {
                    if touching && fingers == 1 {
                        if let Some((lx, ly)) = last {
                            let (dx, dy) = ((x - lx) as f64 * scale, (y - ly) as f64 * scale);
                            if dx != 0.0 || dy != 0.0 || buttons {
                                crate::gesture::feed_delta(dx, dy, name.clone(), buttons);
                            }
                        }
                        last = Some((x, y));
                    } else {
                        last = None;
                    }
                }
                _ => {}
            }
        }
    }
}

/// Read access to input devices and write access to uinput.
pub fn permissions() -> (bool, bool) {
    let read = evdev::enumerate().any(|(_, d)| classify(&d) == Some(Kind::Keyboard));
    (read, can_send())
}

/// Start the device readers. Returns false when no keyboard can be read.
pub fn start() -> bool {
    let mut keyboards = 0;
    let mut touchpad = false;
    for (_path, dev) in evdev::enumerate() {
        match classify(&dev) {
            Some(Kind::Keyboard) => {
                keyboards += 1;
                std::thread::Builder::new().name("makro-kbd".into()).spawn(move || keyboard_thread(dev)).ok();
            }
            Some(Kind::Mouse) => {
                std::thread::Builder::new().name("makro-mouse".into()).spawn(move || mouse_thread(dev)).ok();
            }
            Some(Kind::Touchpad) => {
                touchpad = true;
                std::thread::Builder::new().name("makro-touchpad".into()).spawn(move || touchpad_thread(dev)).ok();
            }
            None => {}
        }
    }
    if touchpad {
        std::thread::Builder::new()
            .name("makro-gesture".into())
            .spawn(|| loop {
                std::thread::sleep(Duration::from_millis(60));
                crate::gesture::tick();
            })
            .ok();
    }
    if keyboards == 0 {
        crate::actions::log(
            "Makro+",
            crate::i18n::t(
                "No keyboard can be read. Add your user to the 'input' group and log in again (Settings › Linux permissions).",
                "Klavye okunamıyor. Kullanıcınızı 'input' grubuna ekleyip yeniden oturum açın (Ayarlar › Linux izinleri).",
            ),
        );
    }
    keyboards > 0
}
