//! Linux window and system helpers with the same API as the Windows layer.
//!
//! Windows ("Hwnd") are X11 window ids. On Wayland compositors that expose
//! focus (Hyprland, Sway) a window is represented by its process id, negated.
use std::sync::OnceLock;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;
use x11rb::rust_connection::RustConnection;

pub type Hwnd = isize;

pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn from_wide(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

pub fn file_name(path: &str) -> String {
    std::path::Path::new(path).file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| path.to_string())
}

pub fn file_stem(path: &str) -> String {
    std::path::Path::new(path).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| path.to_string())
}

pub fn same_path(a: &str, b: &str) -> bool {
    !a.is_empty() && a == b
}

// ---------------- X11 ----------------

struct X {
    conn: RustConnection,
    root: Window,
    active: Atom,
    pid: Atom,
    clients: Atom,
    name: Atom,
    utf8: Atom,
    close: Atom,
    state: Atom,
    above: Atom,
    max_v: Atom,
    max_h: Atom,
    change_state: Atom,
}

fn x() -> Option<&'static X> {
    static C: OnceLock<Option<X>> = OnceLock::new();
    C.get_or_init(|| {
        std::env::var_os("DISPLAY")?;
        let (conn, n) = x11rb::connect(None).ok()?;
        let root = conn.setup().roots.get(n)?.root;
        let atom = |name: &str| conn.intern_atom(false, name.as_bytes()).ok()?.reply().ok().map(|r| r.atom);
        Some(X {
            active: atom("_NET_ACTIVE_WINDOW")?,
            pid: atom("_NET_WM_PID")?,
            clients: atom("_NET_CLIENT_LIST")?,
            name: atom("_NET_WM_NAME")?,
            utf8: atom("UTF8_STRING")?,
            close: atom("_NET_CLOSE_WINDOW")?,
            state: atom("_NET_WM_STATE")?,
            above: atom("_NET_WM_STATE_ABOVE")?,
            max_v: atom("_NET_WM_STATE_MAXIMIZED_VERT")?,
            max_h: atom("_NET_WM_STATE_MAXIMIZED_HORZ")?,
            change_state: atom("WM_CHANGE_STATE")?,
            conn,
            root,
        })
    })
    .as_ref()
}

/// Prefer compositor IPC on Wayland; X11 only sees XWayland windows there.
fn wayland() -> bool {
    std::env::var_os("WAYLAND_DISPLAY").is_some()
}

fn prop32(x: &X, w: Window, atom: Atom) -> Vec<u32> {
    x.conn
        .get_property(false, w, atom, AtomEnum::ANY, 0, 4096)
        .ok()
        .and_then(|c| c.reply().ok())
        .and_then(|r| r.value32().map(|v| v.collect()))
        .unwrap_or_default()
}

fn client_message(x: &X, w: Window, atom: Atom, data: [u32; 5]) {
    let ev = ClientMessageEvent::new(32, w, atom, data);
    let _ = x.conn.send_event(false, x.root, EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY, ev);
    let _ = x.conn.flush();
}

// ---------------- Wayland compositors (pid based) ----------------

fn json_cmd(cmd: &str, args: &[&str]) -> Option<serde_json::Value> {
    let out = std::process::Command::new(cmd).args(args).output().ok()?;
    serde_json::from_slice(&out.stdout).ok()
}

fn hypr() -> bool {
    std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some()
}

fn sway() -> bool {
    std::env::var_os("SWAYSOCK").is_some()
}

fn sway_nodes(v: &serde_json::Value, out: &mut Vec<(i64, bool, String)>) {
    if let Some(pid) = v.get("pid").and_then(|p| p.as_i64()) {
        out.push((pid, v.get("focused").and_then(|f| f.as_bool()).unwrap_or(false), v.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string()));
    }
    for key in ["nodes", "floating_nodes"] {
        if let Some(list) = v.get(key).and_then(|n| n.as_array()) {
            for n in list {
                sway_nodes(n, out);
            }
        }
    }
}

fn wl_windows() -> Vec<(i64, bool, String)> {
    if hypr() {
        let active = json_cmd("hyprctl", &["activewindow", "-j"]).and_then(|v| v.get("pid").and_then(|p| p.as_i64())).unwrap_or(0);
        return json_cmd("hyprctl", &["clients", "-j"])
            .and_then(|v| v.as_array().cloned())
            .unwrap_or_default()
            .iter()
            .filter_map(|c| Some((c.get("pid")?.as_i64()?, false, c.get("title").and_then(|t| t.as_str()).unwrap_or("").to_string())))
            .map(|(p, _, t)| (p, p == active, t))
            .collect();
    }
    if sway() {
        let mut out = Vec::new();
        if let Some(tree) = json_cmd("swaymsg", &["-t", "get_tree"]) {
            sway_nodes(&tree, &mut out);
        }
        return out;
    }
    Vec::new()
}

// ---------------- window API ----------------

pub fn foreground() -> Hwnd {
    if wayland() && (hypr() || sway()) {
        return wl_windows().into_iter().find(|w| w.1).map(|w| -(w.0 as isize)).unwrap_or(0);
    }
    if let Some(x) = x() {
        return prop32(x, x.root, x.active).first().copied().unwrap_or(0) as Hwnd;
    }
    0
}

pub fn window_pid(w: Hwnd) -> u32 {
    if w < 0 {
        return (-w) as u32;
    }
    x().map(|x| prop32(x, w as Window, x.pid).first().copied().unwrap_or(0)).unwrap_or(0)
}

pub fn process_path(pid: u32) -> String {
    if pid == 0 {
        return String::new();
    }
    std::fs::read_link(format!("/proc/{pid}/exe")).map(|p| p.display().to_string()).unwrap_or_default()
}

pub fn window_process_path(w: Hwnd) -> String {
    process_path(window_pid(w))
}

pub fn window_title(w: Hwnd) -> String {
    if w < 0 {
        return wl_windows().into_iter().find(|x| x.0 == -(w as i64)).map(|x| x.2).unwrap_or_default();
    }
    let Some(x) = x() else { return String::new() };
    x.conn
        .get_property(false, w as Window, x.name, x.utf8, 0, 1024)
        .ok()
        .and_then(|c| c.reply().ok())
        .map(|r| String::from_utf8_lossy(&r.value).into_owned())
        .unwrap_or_default()
}

pub fn window_class(w: Hwnd) -> String {
    if w < 0 {
        return String::new();
    }
    let Some(x) = x() else { return String::new() };
    x.conn
        .get_property(false, w as Window, AtomEnum::WM_CLASS, AtomEnum::STRING, 0, 256)
        .ok()
        .and_then(|c| c.reply().ok())
        .map(|r| String::from_utf8_lossy(&r.value).split('\0').nth(1).unwrap_or("").to_string())
        .unwrap_or_default()
}

pub fn is_window(w: Hwnd) -> bool {
    if w < 0 {
        return std::path::Path::new(&format!("/proc/{}", -w)).exists();
    }
    x().map(|x| x.conn.get_geometry(w as Window).ok().and_then(|c| c.reply().ok()).is_some()).unwrap_or(false)
}

pub fn top_windows() -> Vec<Hwnd> {
    if wayland() && (hypr() || sway()) {
        return wl_windows().into_iter().map(|w| -(w.0 as isize)).collect();
    }
    x().map(|x| prop32(x, x.root, x.clients).into_iter().map(|w| w as Hwnd).collect()).unwrap_or_default()
}

pub fn app_windows(path: &str) -> Vec<Hwnd> {
    top_windows().into_iter().filter(|&w| same_path(&window_process_path(w), path)).collect()
}

fn desktop_entries() -> Vec<String> {
    let mut dirs = vec![std::path::PathBuf::from("/usr/share/applications"), std::path::PathBuf::from("/var/lib/flatpak/exports/share/applications")];
    if let Ok(h) = std::env::var("HOME") {
        dirs.push(std::path::PathBuf::from(&h).join(".local/share/applications"));
        dirs.push(std::path::PathBuf::from(&h).join(".local/share/flatpak/exports/share/applications"));
    }
    let mut out = Vec::new();
    for d in dirs {
        if let Ok(rd) = std::fs::read_dir(d) {
            for e in rd.flatten() {
                let p = e.path();
                if p.extension().map(|x| x == "desktop").unwrap_or(false) {
                    if let Ok(t) = std::fs::read_to_string(&p) {
                        if t.contains("Type=Application") && !t.contains("NoDisplay=true") && t.contains("Exec=") {
                            out.push(p.display().to_string());
                        }
                    }
                }
            }
        }
    }
    out
}

/// Running apps first, then installed applications (.desktop launchers).
pub fn running_apps() -> Vec<String> {
    let own = std::env::current_exe().map(|p| p.display().to_string()).unwrap_or_default();
    let mut paths: Vec<String> = Vec::new();
    for w in top_windows() {
        let p = window_process_path(w);
        if !p.is_empty() && p != own && !paths.contains(&p) {
            paths.push(p);
        }
    }
    let mut desktop = desktop_entries();
    desktop.sort_by_key(|p| crate::apps::display_name(p).to_lowercase());
    paths.extend(desktop);
    paths
}

pub fn window_rect(w: Hwnd) -> Option<(i32, i32, i32, i32)> {
    if w < 0 {
        return None;
    }
    let x = x()?;
    let g = x.conn.get_geometry(w as Window).ok()?.reply().ok()?;
    let t = x.conn.translate_coordinates(w as Window, x.root, 0, 0).ok()?.reply().ok()?;
    Some((t.dst_x as i32, t.dst_y as i32, t.dst_x as i32 + g.width as i32, t.dst_y as i32 + g.height as i32))
}

pub fn activate(w: Hwnd) -> bool {
    if w < 0 {
        let pid = -w;
        if hypr() {
            let _ = std::process::Command::new("hyprctl").args(["dispatch", "focuswindow", &format!("pid:{pid}")]).output();
        } else if sway() {
            let _ = std::process::Command::new("swaymsg").arg(format!("[pid={pid}] focus")).output();
        }
        std::thread::sleep(std::time::Duration::from_millis(60));
        return foreground() == w;
    }
    let Some(x) = x() else { return false };
    let _ = x.conn.map_window(w as Window);
    client_message(x, w as Window, x.active, [2, 0, 0, 0, 0]);
    std::thread::sleep(std::time::Duration::from_millis(60));
    foreground() == w
}

fn set_state(w: Hwnd, add: bool, a: Atom, b: Atom) {
    if let Some(x) = x() {
        client_message(x, w as Window, x.state, [if add { 1 } else { 0 }, a, b, 2, 0]);
    }
}

pub fn hide(w: Hwnd) {
    minimize(w)
}

pub fn show_normal(w: Hwnd) {
    activate(w);
}

pub fn minimize(w: Hwnd) {
    if w < 0 {
        return;
    }
    if let Some(x) = x() {
        client_message(x, w as Window, x.change_state, [3, 0, 0, 0, 0]);
    }
}

pub fn maximize(w: Hwnd) {
    if w >= 0 {
        if let Some(x) = x() {
            set_state(w, true, x.max_v, x.max_h);
        }
    }
}

pub fn restore(w: Hwnd) {
    if w >= 0 {
        if let Some(x) = x() {
            set_state(w, false, x.max_v, x.max_h);
        }
        activate(w);
    }
}

pub fn post_close(w: Hwnd) -> bool {
    if w < 0 {
        let pid = -w;
        return if hypr() {
            std::process::Command::new("hyprctl").args(["dispatch", "closewindow", &format!("pid:{pid}")]).status().is_ok()
        } else if sway() {
            std::process::Command::new("swaymsg").arg(format!("[pid={pid}] kill")).status().is_ok()
        } else {
            false
        };
    }
    match x() {
        Some(x) => {
            client_message(x, w as Window, x.close, [0, 2, 0, 0, 0]);
            true
        }
        None => false,
    }
}

pub fn set_topmost(w: Hwnd, on: bool) {
    if w >= 0 {
        if let Some(x) = x() {
            set_state(w, on, x.above, 0);
        }
    }
}

pub fn cursor_pos() -> (i32, i32) {
    x().and_then(|x| x.conn.query_pointer(x.root).ok()?.reply().ok()).map(|p| (p.root_x as i32, p.root_y as i32)).unwrap_or((0, 0))
}

/// Move the pointer (X11 only).
pub fn warp_pointer(px: i32, py: i32) -> bool {
    match x() {
        Some(x) if !wayland() => {
            let _ = x.conn.warp_pointer(x11rb::NONE, x.root, 0, 0, 0, 0, px as i16, py as i16);
            x.conn.flush().is_ok()
        }
        _ => false,
    }
}

// ---------------- input (uinput) ----------------

pub fn send_chord(mods: i32, key: i32, altgr: bool) -> Result<(), String> {
    crate::input::send_chord(mods, key, altgr)
}

pub fn send_key_raw(vk: i32, up: bool) -> Result<(), String> {
    crate::input::send_key_raw(vk, up)
}

pub fn type_text(text: &str) -> Result<(), String> {
    crate::input::type_text(text)
}

#[allow(clippy::too_many_arguments)]
pub fn mouse_at(kind: &str, button: i32, x: i32, y: i32, x2: i32, y2: i32, delta: i32, horizontal: bool, hold_ms: u64) -> Result<(), String> {
    crate::input::mouse_at(kind, button, x, y, x2, y2, delta, horizontal, hold_ms)
}

pub fn key_down(vk: i32) -> bool {
    crate::input::is_down(vk)
}

pub fn key_char(_vk: i32) -> Option<String> {
    None
}

// ---------------- system ----------------

pub fn message_box(text: &str, caption: &str, yes_no: bool) -> bool {
    let d = rfd::MessageDialog::new().set_title(caption).set_description(text).set_level(if yes_no { rfd::MessageLevel::Warning } else { rfd::MessageLevel::Info }).set_buttons(if yes_no { rfd::MessageButtons::YesNo } else { rfd::MessageButtons::Ok });
    let r = d.show();
    !yes_no || r == rfd::MessageDialogResult::Yes
}

pub fn shell_open(target: &str, args: &str, _dir: &str) -> Result<(), String> {
    let r = if args.is_empty() {
        std::process::Command::new("xdg-open").arg(target).spawn()
    } else {
        std::process::Command::new(target).args(crate::apps::split_args(args)).spawn()
    };
    r.map(|_| ()).map_err(|e| format!("{}: {target} ({e})", crate::i18n::t("Could not open", "Açılamadı")))
}

/// Single instance through an advisory lock file.
pub fn single_instance(_name: &str) -> bool {
    use std::os::unix::io::AsRawFd;
    let dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into());
    let path = format!("{dir}/makroplus-{}.lock", unsafe { libc::getuid() });
    let Ok(f) = std::fs::OpenOptions::new().create(true).write(true).truncate(false).open(&path) else { return true };
    let ok = unsafe { libc::flock(f.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0;
    if ok {
        std::mem::forget(f); // held for the life of the process
    }
    ok
}

pub fn find_own_window_titled(_title: &str) -> Option<Hwnd> {
    None
}

pub fn find_window_by_title(_title: &str) -> Option<Hwnd> {
    None
}

pub fn find_own_window() -> Option<Hwnd> {
    None
}

/// Linux has no DPAPI: the key is stored only in the user's settings file
/// (mode 600 directory), base64-encoded and marked as such.
pub fn dpapi_protect_b64(text: &str) -> Option<String> {
    Some(format!("plain:{}", b64_encode(text.as_bytes())))
}

pub fn dpapi_unprotect_b64(text: &str) -> Option<String> {
    text.strip_prefix("plain:").and_then(b64_decode).and_then(|v| String::from_utf8(v).ok())
}

pub fn random_bytes(buf: &mut [u8]) {
    use std::io::Read;
    if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
        let _ = f.read_exact(buf);
    }
}

pub fn uuid_v4() -> String {
    let mut b = [0u8; 16];
    random_bytes(&mut b);
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let hex: String = b.iter().map(|x| format!("{x:02x}")).collect();
    format!("{}-{}-{}-{}-{}", &hex[0..8], &hex[8..12], &hex[12..16], &hex[16..20], &hex[20..32])
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn b64_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(B64[(n >> 18) as usize & 63] as char);
        out.push(B64[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 { B64[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if chunk.len() > 2 { B64[n as usize & 63] as char } else { '=' });
    }
    out
}

pub fn b64_decode(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut buf = 0u32;
    let mut bits = 0;
    for c in text.bytes() {
        if c == b'=' || c.is_ascii_whitespace() {
            continue;
        }
        let v = B64.iter().position(|&x| x == c)? as u32;
        buf = (buf << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    Some(out)
}

pub fn dark_title_bar(_w: Hwnd, _dark: bool) {}
