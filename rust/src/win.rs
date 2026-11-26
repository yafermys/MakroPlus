//! Thin, safe wrappers over the Win32 APIs Makro+ uses.
#![allow(clippy::missing_safety_doc)]
use std::ffi::c_void;
use std::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Security::Cryptography::*;
use windows_sys::Win32::System::Threading::*;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

/// Marker placed in dwExtraInfo of every event Makro+ synthesizes (same value as 2.x).
pub const INJECT_MARK: usize = 0x414B_534D;

pub type Hwnd = isize;

pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn from_wide(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

pub fn h(hwnd: Hwnd) -> HWND {
    hwnd as HWND
}

pub fn file_name(path: &str) -> String {
    std::path::Path::new(path).file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| path.to_string())
}

pub fn file_stem(path: &str) -> String {
    std::path::Path::new(path).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| path.to_string())
}

pub fn same_path(a: &str, b: &str) -> bool {
    !a.is_empty() && a.eq_ignore_ascii_case(b) || (!a.is_empty() && a.to_lowercase() == b.to_lowercase())
}

pub fn foreground() -> Hwnd {
    unsafe { GetForegroundWindow() as Hwnd }
}

pub fn window_pid(hwnd: Hwnd) -> u32 {
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(h(hwnd), &mut pid) };
    pid
}

pub fn process_path(pid: u32) -> String {
    unsafe {
        let p = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if p.is_null() {
            return String::new();
        }
        let mut buf = vec![0u16; 32768];
        let mut n = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(p, 0, buf.as_mut_ptr(), &mut n);
        CloseHandle(p);
        if ok == 0 {
            String::new()
        } else {
            String::from_utf16_lossy(&buf[..n as usize])
        }
    }
}

pub fn window_process_path(hwnd: Hwnd) -> String {
    process_path(window_pid(hwnd))
}

pub fn window_title(hwnd: Hwnd) -> String {
    unsafe {
        let n = GetWindowTextLengthW(h(hwnd));
        if n <= 0 {
            return String::new();
        }
        let mut buf = vec![0u16; n as usize + 1];
        let got = GetWindowTextW(h(hwnd), buf.as_mut_ptr(), buf.len() as i32);
        String::from_utf16_lossy(&buf[..got.max(0) as usize])
    }
}

pub fn window_class(hwnd: Hwnd) -> String {
    unsafe {
        let mut buf = [0u16; 256];
        let n = GetClassNameW(h(hwnd), buf.as_mut_ptr(), buf.len() as i32);
        String::from_utf16_lossy(&buf[..n.max(0) as usize])
    }
}

pub fn is_window(hwnd: Hwnd) -> bool {
    unsafe { IsWindow(h(hwnd)) != 0 }
}

/// Visible, unowned, titled top-level windows.
pub fn top_windows() -> Vec<Hwnd> {
    unsafe extern "system" fn cb(hwnd: HWND, l: LPARAM) -> BOOL {
        let list = &mut *(l as *mut Vec<Hwnd>);
        if IsWindowVisible(hwnd) != 0 && GetWindow(hwnd, GW_OWNER).is_null() && GetWindowTextLengthW(hwnd) > 0 {
            list.push(hwnd as Hwnd);
        }
        1
    }
    let mut list: Vec<Hwnd> = Vec::new();
    unsafe { EnumWindows(Some(cb), &mut list as *mut _ as LPARAM) };
    list
}

/// Top-level windows that belong to the given executable.
pub fn app_windows(path: &str) -> Vec<Hwnd> {
    top_windows().into_iter().filter(|&w| same_path(&window_process_path(w), path)).collect()
}

/// Executables that currently have a visible window, sorted by name.
pub fn running_apps() -> Vec<String> {
    let own = std::env::current_exe().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
    let mut paths: Vec<String> = Vec::new();
    for w in top_windows() {
        let p = window_process_path(w);
        if p.is_empty() || same_path(&p, &own) || p.to_lowercase().ends_with("\\explorer.exe") && window_class(w) == "Progman" {
            continue;
        }
        if !paths.iter().any(|x| same_path(x, &p)) {
            paths.push(p);
        }
    }
    paths.sort_by_key(|p| file_stem(p).to_lowercase());
    paths
}

/// Window bounds as (left, top, right, bottom) in screen pixels.
pub fn window_rect(hwnd: Hwnd) -> Option<(i32, i32, i32, i32)> {
    let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    if unsafe { GetWindowRect(h(hwnd), &mut r) } != 0 {
        Some((r.left, r.top, r.right, r.bottom))
    } else {
        None
    }
}

pub fn hide(hwnd: Hwnd) {
    show(hwnd, SW_HIDE);
}

pub fn show_normal(hwnd: Hwnd) {
    show(hwnd, SW_SHOW);
}

pub fn minimize(hwnd: Hwnd) {
    show(hwnd, SW_MINIMIZE);
}

pub fn maximize(hwnd: Hwnd) {
    show(hwnd, SW_MAXIMIZE);
}

pub fn restore(hwnd: Hwnd) {
    show(hwnd, SW_RESTORE);
}

fn nudge_input() {
    // Any synthesized input makes this process the last input source, which
    // lets SetForegroundWindow succeed. A zero mouse move is invisible.
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 { mi: MOUSEINPUT { dx: 0, dy: 0, mouseData: 0, dwFlags: MOUSEEVENTF_MOVE, time: 0, dwExtraInfo: INJECT_MARK } },
    };
    unsafe { SendInput(1, &input, std::mem::size_of::<INPUT>() as i32) };
}

pub fn activate(hwnd: Hwnd) -> bool {
    unsafe {
        let w = h(hwnd);
        if IsIconic(w) != 0 {
            ShowWindow(w, SW_RESTORE);
        }
        if GetForegroundWindow() == w {
            return true;
        }
        nudge_input();
        let fg = GetForegroundWindow();
        let fg_thread = GetWindowThreadProcessId(fg, null_mut());
        let me = GetCurrentThreadId();
        let attached = fg_thread != 0 && fg_thread != me && AttachThreadInput(me, fg_thread, 1) != 0;
        SetForegroundWindow(w);
        BringWindowToTop(w);
        if attached {
            AttachThreadInput(me, fg_thread, 0);
        }
        GetForegroundWindow() == w
    }
}

pub fn show(hwnd: Hwnd, cmd: i32) {
    unsafe { ShowWindow(h(hwnd), cmd) };
}

pub fn post_close(hwnd: Hwnd) -> bool {
    unsafe { PostMessageW(h(hwnd), WM_CLOSE, 0, 0) != 0 }
}

pub fn set_topmost(hwnd: Hwnd, on: bool) {
    unsafe {
        SetWindowPos(h(hwnd), if on { HWND_TOPMOST } else { HWND_NOTOPMOST }, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
    }
}

fn is_extended(vk: i32) -> bool {
    matches!(vk, 33..=40 | 45 | 46 | 91 | 92 | 93 | 111 | 144 | 163 | 165 | 166..=183)
}

fn key_input(vk: i32, up: bool) -> INPUT {
    let mut flags = if up { KEYEVENTF_KEYUP } else { 0 };
    if is_extended(vk) {
        flags |= KEYEVENTF_EXTENDEDKEY;
    }
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT { wVk: vk as u16, wScan: unsafe { MapVirtualKeyW(vk as u32, MAPVK_VK_TO_VSC) } as u16, dwFlags: flags, time: 0, dwExtraInfo: INJECT_MARK },
        },
    }
}

fn send(inputs: &[INPUT]) -> Result<(), String> {
    if inputs.is_empty() {
        return Ok(());
    }
    let n = unsafe { SendInput(inputs.len() as u32, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32) };
    if n as usize != inputs.len() {
        Err(crate::i18n::t(
            "Input could not be sent. The target app may run with higher privileges.",
            "Giriş gönderilemedi. Hedef uygulama daha yüksek yetkiyle çalışıyor olabilir.",
        )
        .into())
    } else {
        Ok(())
    }
}

/// Press a chord: modifiers down, key down/up, modifiers up.
pub fn send_chord(mods: i32, key: i32, altgr: bool) -> Result<(), String> {
    if crate::keys::is_pointer(key) {
        return send_pointer(mods, key);
    }
    let mut keys = Vec::new();
    if altgr {
        keys.push(162);
        keys.push(165);
    } else {
        if mods & 2 != 0 {
            keys.push(162);
        }
        if mods & 1 != 0 {
            keys.push(164);
        }
    }
    if mods & 4 != 0 {
        keys.push(160);
    }
    if mods & 8 != 0 {
        keys.push(91);
    }
    let mut inputs: Vec<INPUT> = keys.iter().map(|&k| key_input(k, false)).collect();
    inputs.push(key_input(key, false));
    inputs.push(key_input(key, true));
    for &k in keys.iter().rev() {
        inputs.push(key_input(k, true));
    }
    send(&inputs)
}

pub fn send_key_raw(vk: i32, up: bool) -> Result<(), String> {
    send(&[key_input(vk, up)])
}

/// Mouse buttons / wheel as "keys" (1,2,4,5,6,256-259) with modifiers held.
pub fn send_pointer(mods: i32, key: i32) -> Result<(), String> {
    let mut pre = Vec::new();
    for (bit, vk) in [(2, 162), (1, 164), (4, 160), (8, 91)] {
        if mods & bit != 0 {
            pre.push(vk);
        }
    }
    let mut inputs: Vec<INPUT> = pre.iter().map(|&k| key_input(k, false)).collect();
    let mouse = |flags: u32, data: i32| INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 { mi: MOUSEINPUT { dx: 0, dy: 0, mouseData: data as _, dwFlags: flags, time: 0, dwExtraInfo: INJECT_MARK } },
    };
    match key {
        1 => {
            inputs.push(mouse(MOUSEEVENTF_LEFTDOWN, 0));
            inputs.push(mouse(MOUSEEVENTF_LEFTUP, 0));
        }
        2 => {
            inputs.push(mouse(MOUSEEVENTF_RIGHTDOWN, 0));
            inputs.push(mouse(MOUSEEVENTF_RIGHTUP, 0));
        }
        4 => {
            inputs.push(mouse(MOUSEEVENTF_MIDDLEDOWN, 0));
            inputs.push(mouse(MOUSEEVENTF_MIDDLEUP, 0));
        }
        5 | 6 => {
            let b = if key == 5 { 1 } else { 2 };
            inputs.push(mouse(MOUSEEVENTF_XDOWN, b));
            inputs.push(mouse(MOUSEEVENTF_XUP, b));
        }
        crate::keys::WHEEL_UP => inputs.push(mouse(MOUSEEVENTF_WHEEL, 120)),
        crate::keys::WHEEL_DOWN => inputs.push(mouse(MOUSEEVENTF_WHEEL, -120)),
        crate::keys::WHEEL_RIGHT => inputs.push(mouse(MOUSEEVENTF_HWHEEL, 120)),
        crate::keys::WHEEL_LEFT => inputs.push(mouse(MOUSEEVENTF_HWHEEL, -120)),
        _ => return Err("Unknown pointer key".into()),
    }
    for &k in pre.iter().rev() {
        inputs.push(key_input(k, true));
    }
    send(&inputs)
}

/// Type literal text with Unicode events; newlines become Enter, tabs Tab.
pub fn type_text(text: &str) -> Result<(), String> {
    let mut inputs = Vec::new();
    let mut prev_cr = false;
    for unit in text.encode_utf16() {
        if unit == '\n' as u16 {
            if !prev_cr {
                inputs.push(key_input(13, false));
                inputs.push(key_input(13, true));
            }
            prev_cr = false;
            continue;
        }
        prev_cr = unit == '\r' as u16;
        if prev_cr {
            inputs.push(key_input(13, false));
            inputs.push(key_input(13, true));
            continue;
        }
        if unit == '\t' as u16 {
            inputs.push(key_input(9, false));
            inputs.push(key_input(9, true));
            continue;
        }
        for up in [false, true] {
            inputs.push(INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT { wVk: 0, wScan: unit, dwFlags: KEYEVENTF_UNICODE | if up { KEYEVENTF_KEYUP } else { 0 }, time: 0, dwExtraInfo: INJECT_MARK },
                },
            });
        }
    }
    // Long texts in chunks so the target's input queue keeps up.
    for chunk in inputs.chunks(400) {
        send(chunk)?;
        if inputs.len() > 400 {
            std::thread::sleep(std::time::Duration::from_millis(8));
        }
    }
    Ok(())
}

pub fn cursor_pos() -> (i32, i32) {
    let mut p = POINT { x: 0, y: 0 };
    unsafe { GetCursorPos(&mut p) };
    (p.x, p.y)
}

/// Click/drag/scroll at screen coordinates.
pub fn mouse_at(kind: &str, button: i32, x: i32, y: i32, x2: i32, y2: i32, delta: i32, horizontal: bool, hold_ms: u64) -> Result<(), String> {
    let (down, up, data) = match button {
        1 => (MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, 0),
        2 => (MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, 0),
        3 => (MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP, 1),
        4 => (MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP, 2),
        _ => (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, 0),
    };
    let m = |flags: u32, d: i32| INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 { mi: MOUSEINPUT { dx: 0, dy: 0, mouseData: d as _, dwFlags: flags, time: 0, dwExtraInfo: INJECT_MARK } },
    };
    unsafe { SetCursorPos(x, y) };
    std::thread::sleep(std::time::Duration::from_millis(15));
    match kind {
        "Scroll" => send(&[m(if horizontal { MOUSEEVENTF_HWHEEL } else { MOUSEEVENTF_WHEEL }, delta)]),
        "Mouse drag" => {
            send(&[m(down, data)])?;
            let steps = 12;
            for i in 1..=steps {
                let px = x + (x2 - x) * i / steps;
                let py = y + (y2 - y) * i / steps;
                unsafe { SetCursorPos(px, py) };
                std::thread::sleep(std::time::Duration::from_millis((hold_ms / steps as u64).clamp(5, 60)));
            }
            send(&[m(up, data)])
        }
        _ => {
            send(&[m(down, data)])?;
            if hold_ms > 0 {
                std::thread::sleep(std::time::Duration::from_millis(hold_ms.min(30000)));
            }
            send(&[m(up, data)])
        }
    }
}

pub fn key_down(vk: i32) -> bool {
    unsafe { (GetAsyncKeyState(vk) as u16 & 0x8000) != 0 }
}

/// Character printed by an OEM key in the current layout.
pub fn key_char(vk: i32) -> Option<String> {
    let c = unsafe { MapVirtualKeyW(vk as u32, MAPVK_VK_TO_CHAR) } & 0xFFFF;
    if c == 0 {
        None
    } else {
        char::from_u32(c).map(|c| c.to_uppercase().to_string())
    }
}

pub fn message_box(text: &str, caption: &str, yes_no: bool) -> bool {
    let style = if yes_no { MB_YESNO | MB_ICONWARNING | MB_DEFBUTTON2 } else { MB_OK | MB_ICONINFORMATION };
    let r = unsafe { MessageBoxW(null_mut(), wide(text).as_ptr(), wide(caption).as_ptr(), style | MB_TOPMOST | MB_SETFOREGROUND) };
    if yes_no {
        r == IDYES
    } else {
        true
    }
}

/// ShellExecute "open" a file, folder, URL or URI.
pub fn shell_open(target: &str, args: &str, dir: &str) -> Result<(), String> {
    let t = wide(target);
    let a = wide(args);
    let d = wide(dir);
    let r = unsafe {
        ShellExecuteW(
            null_mut(),
            wide("open").as_ptr(),
            t.as_ptr(),
            if args.is_empty() { null() } else { a.as_ptr() },
            if dir.is_empty() { null() } else { d.as_ptr() },
            SW_SHOWNORMAL,
        )
    } as isize;
    if r > 32 {
        Ok(())
    } else {
        Err(format!("{} ({}): {}", crate::i18n::t("Could not open", "Açılamadı"), r, target))
    }
}

/// Single-instance guard. Returns false when another copy is running.
pub fn single_instance(name: &str) -> bool {
    unsafe {
        let hnd = windows_sys::Win32::System::Threading::CreateMutexW(null(), 1, wide(name).as_ptr());
        !hnd.is_null() && GetLastError() != ERROR_ALREADY_EXISTS
    }
}

pub fn find_own_window() -> Option<Hwnd> {
    let pid = std::process::id();
    let mut found = None;
    for w in top_windows() {
        if window_pid(w) == pid {
            found = Some(w);
        }
    }
    found
}

/// This process's top-level window with the given title, visible or not.
pub fn find_own_window_titled(title: &str) -> Option<Hwnd> {
    unsafe extern "system" fn cb(hwnd: HWND, l: LPARAM) -> BOOL {
        let list = &mut *(l as *mut Vec<Hwnd>);
        list.push(hwnd as Hwnd);
        1
    }
    let mut all: Vec<Hwnd> = Vec::new();
    unsafe { EnumWindows(Some(cb), &mut all as *mut _ as LPARAM) };
    let pid = std::process::id();
    all.into_iter().find(|&w| window_pid(w) == pid && window_title(w) == title)
}

/// Find the main window of another running Makro+ by title.
pub fn find_window_by_title(title: &str) -> Option<Hwnd> {
    let w = unsafe { FindWindowW(null(), wide(title).as_ptr()) };
    if w.is_null() {
        None
    } else {
        Some(w as Hwnd)
    }
}

// ---- DPAPI (compatible with .NET ProtectedData, CurrentUser, no entropy) ----

fn blob_from(data: &[u8]) -> CRYPT_INTEGER_BLOB {
    CRYPT_INTEGER_BLOB { cbData: data.len() as u32, pbData: data.as_ptr() as *mut u8 }
}

pub fn dpapi_protect(data: &[u8]) -> Option<Vec<u8>> {
    unsafe {
        let input = blob_from(data);
        let mut out = CRYPT_INTEGER_BLOB { cbData: 0, pbData: null_mut() };
        if CryptProtectData(&input, null(), null(), null(), null(), CRYPTPROTECT_UI_FORBIDDEN, &mut out) == 0 {
            return None;
        }
        let v = std::slice::from_raw_parts(out.pbData, out.cbData as usize).to_vec();
        LocalFree(out.pbData as *mut c_void);
        Some(v)
    }
}

pub fn dpapi_unprotect(data: &[u8]) -> Option<Vec<u8>> {
    unsafe {
        let input = blob_from(data);
        let mut out = CRYPT_INTEGER_BLOB { cbData: 0, pbData: null_mut() };
        if CryptUnprotectData(&input, null_mut(), null(), null(), null(), CRYPTPROTECT_UI_FORBIDDEN, &mut out) == 0 {
            return None;
        }
        let v = std::slice::from_raw_parts(out.pbData, out.cbData as usize).to_vec();
        LocalFree(out.pbData as *mut c_void);
        Some(v)
    }
}

pub fn dpapi_protect_b64(text: &str) -> Option<String> {
    dpapi_protect(text.as_bytes()).map(|v| b64_encode(&v))
}

pub fn dpapi_unprotect_b64(text: &str) -> Option<String> {
    b64_decode(text).and_then(|v| dpapi_unprotect(&v)).and_then(|v| String::from_utf8(v).ok())
}

pub fn random_bytes(buf: &mut [u8]) {
    unsafe {
        BCryptGenRandom(null_mut(), buf.as_mut_ptr(), buf.len() as u32, BCRYPT_USE_SYSTEM_PREFERRED_RNG);
    }
}

/// Random RFC 4122 version-4 UUID.
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

/// Turn on the dark title bar for a window when using a dark theme.
pub fn dark_title_bar(hwnd: Hwnd, dark: bool) {
    let v: BOOL = if dark { 1 } else { 0 };
    unsafe {
        windows_sys::Win32::Graphics::Dwm::DwmSetWindowAttribute(h(hwnd), 20, &v as *const BOOL as *const c_void, 4);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn b64_round_trip() {
        for s in ["", "a", "ab", "abc", "hello world", "ğüşiöç"] {
            assert_eq!(super::b64_decode(&super::b64_encode(s.as_bytes())).unwrap(), s.as_bytes());
        }
        assert_eq!(super::b64_encode(b"Man"), "TWFu");
    }
}
