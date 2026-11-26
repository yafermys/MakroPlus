//! Virtual-key names, labels and trigger parsing (compatible with 2.x key codes).
pub const MOD_ALT: i32 = 1;
pub const MOD_CTRL: i32 = 2;
pub const MOD_SHIFT: i32 = 4;
pub const MOD_WIN: i32 = 8;

pub const WHEEL_UP: i32 = 256;
pub const WHEEL_DOWN: i32 = 257;
pub const WHEEL_LEFT: i32 = 258;
pub const WHEEL_RIGHT: i32 = 259;

pub fn is_wheel(k: i32) -> bool {
    (WHEEL_UP..=WHEEL_RIGHT).contains(&k)
}
pub fn is_pointer(k: i32) -> bool {
    matches!(k, 1 | 2 | 4 | 5 | 6) || is_wheel(k)
}
pub fn is_modifier(k: i32) -> bool {
    matches!(k, 16 | 17 | 18 | 91 | 92) || (160..=165).contains(&k)
}
pub fn valid(k: i32) -> bool {
    (1..=254).contains(&k) || is_wheel(k)
}
/// Keys that may start a two-key chord (Del+5, F13+A…).
pub fn prefix_allowed(k: i32) -> bool {
    k == 46 || k == 45 || ((112..=135).contains(&k) && k != 123)
}

pub fn pointer_name(k: i32) -> &'static str {
    use crate::i18n::t;
    match k {
        1 => t("Mouse left", "Fare sol"),
        2 => t("Mouse right", "Fare sağ"),
        4 => t("Mouse middle", "Fare orta"),
        5 => t("Mouse X1 (back)", "Fare X1 (geri)"),
        6 => t("Mouse X2 (forward)", "Fare X2 (ileri)"),
        WHEEL_UP => t("Wheel up", "Tekerlek yukarı"),
        WHEEL_DOWN => t("Wheel down", "Tekerlek aşağı"),
        WHEEL_LEFT => t("Wheel left", "Tekerlek sola"),
        WHEEL_RIGHT => t("Wheel right", "Tekerlek sağa"),
        _ => "",
    }
}

fn pointer_token(k: i32) -> &'static str {
    match k {
        1 => "LButton",
        2 => "RButton",
        4 => "MButton",
        5 => "XButton1",
        6 => "XButton2",
        WHEEL_UP => "WheelUp",
        WHEEL_DOWN => "WheelDown",
        WHEEL_LEFT => "WheelLeft",
        WHEEL_RIGHT => "WheelRight",
        _ => "",
    }
}

/// Stable token name for a key (used in triggers and model plans).
pub fn token(k: i32) -> String {
    if is_pointer(k) {
        return pointer_token(k).into();
    }
    let s = match k {
        8 => "Backspace",
        9 => "Tab",
        12 => "Clear",
        13 => "Enter",
        19 => "Pause",
        20 => "CapsLock",
        27 => "Esc",
        32 => "Space",
        33 => "PageUp",
        34 => "PageDown",
        35 => "End",
        36 => "Home",
        37 => "Left",
        38 => "Up",
        39 => "Right",
        40 => "Down",
        44 => "PrintScreen",
        45 => "Insert",
        46 => "Del",
        93 => "Apps",
        95 => "Sleep",
        106 => "Multiply",
        107 => "Add",
        108 => "Separator",
        109 => "Subtract",
        110 => "Decimal",
        111 => "Divide",
        144 => "NumLock",
        145 => "ScrollLock",
        166 => "BrowserBack",
        167 => "BrowserForward",
        168 => "BrowserRefresh",
        169 => "BrowserStop",
        170 => "BrowserSearch",
        171 => "BrowserFavorites",
        172 => "BrowserHome",
        173 => "VolumeMute",
        174 => "VolumeDown",
        175 => "VolumeUp",
        176 => "MediaNextTrack",
        177 => "MediaPreviousTrack",
        178 => "MediaStop",
        179 => "MediaPlayPause",
        180 => "LaunchMail",
        181 => "SelectMedia",
        182 => "LaunchApp1",
        183 => "LaunchApp2",
        186 => "Oem1",
        187 => "OemPlus",
        188 => "OemComma",
        189 => "OemMinus",
        190 => "OemPeriod",
        191 => "Oem2",
        192 => "Oem3",
        219 => "Oem4",
        220 => "Oem5",
        221 => "Oem6",
        222 => "Oem7",
        223 => "Oem8",
        226 => "Oem102",
        _ => "",
    };
    if !s.is_empty() {
        return s.into();
    }
    match k {
        48..=57 | 65..=90 => ((k as u8) as char).to_string(),
        96..=105 => format!("Numpad{}", k - 96),
        112..=135 => format!("F{}", k - 111),
        _ => format!("VK{}", k),
    }
}

/// Human readable label ("Ctrl + Alt + F6", "Delete + 5").
pub fn label(mods: i32, key: i32, prefix: i32) -> String {
    use crate::i18n::t;
    if key == 0 {
        return t("Key not selected", "Tuş seçilmedi").into();
    }
    let mut s = String::new();
    if mods & MOD_CTRL != 0 {
        s += "Ctrl + ";
    }
    if mods & MOD_ALT != 0 {
        s += "Alt + ";
    }
    if mods & MOD_SHIFT != 0 {
        s += "Shift + ";
    }
    if mods & MOD_WIN != 0 {
        s += "Win + ";
    }
    if prefix > 0 {
        s += &key_display(prefix);
        s += " + ";
    }
    s += &key_display(key);
    s
}

fn key_display(k: i32) -> String {
    if is_pointer(k) {
        return pointer_name(k).into();
    }
    match k {
        46 => "Delete".into(),
        96..=105 => format!("Numpad {}", k - 96),
        106 => "Numpad *".into(),
        107 => "Numpad +".into(),
        109 => "Numpad -".into(),
        110 => "Numpad .".into(),
        111 => "Numpad /".into(),
        186..=192 | 219..=223 | 226 => crate::win::key_char(k).unwrap_or_else(|| token(k)),
        _ => token(k),
    }
}

/// Compact trigger text, parseable by [`parse_trigger`] ("Ctrl+Alt+F6", "Del+5").
pub fn trigger_text(mods: i32, key: i32, prefix: i32) -> String {
    let mut s = String::new();
    if mods & MOD_CTRL != 0 {
        s += "Ctrl+";
    }
    if mods & MOD_ALT != 0 {
        s += "Alt+";
    }
    if mods & MOD_SHIFT != 0 {
        s += "Shift+";
    }
    if mods & MOD_WIN != 0 {
        s += "Win+";
    }
    if prefix > 0 {
        s += &token(prefix);
        s += "+";
    }
    s += &token(key);
    s
}

fn fold(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            'İ' | 'I' | 'ı' => 'i',
            'Ş' | 'ş' => 's',
            'Ğ' | 'ğ' => 'g',
            'Ü' | 'ü' => 'u',
            'Ö' | 'ö' => 'o',
            'Ç' | 'ç' => 'c',
            other => other.to_ascii_lowercase(),
        })
        .collect()
}

/// Parse one key name (no modifiers).
pub fn parse_key(name: &str) -> Option<i32> {
    let n = fold(name.trim()).replace([' ', '_', '-'], "");
    if n.is_empty() {
        return None;
    }
    if n.len() == 1 {
        let c = n.chars().next().unwrap();
        if c.is_ascii_alphanumeric() {
            return Some(c.to_ascii_uppercase() as i32);
        }
        return match c {
            ';' => Some(186),
            '=' => Some(187),
            ',' => Some(188),
            '.' => Some(190),
            '/' => Some(191),
            '`' => Some(192),
            '[' => Some(219),
            '\\' => Some(220),
            ']' => Some(221),
            '\'' => Some(222),
            '*' => Some(106),
            _ => None,
        };
    }
    if let Some(rest) = n.strip_prefix('f') {
        if let Ok(v) = rest.parse::<i32>() {
            if (1..=24).contains(&v) {
                return Some(111 + v);
            }
        }
    }
    for prefix in ["numpad", "num"] {
        if let Some(rest) = n.strip_prefix(prefix) {
            if let Ok(v) = rest.parse::<i32>() {
                if (0..=9).contains(&v) {
                    return Some(96 + v);
                }
            }
            match rest {
                "*" | "multiply" => return Some(106),
                "+" | "add" => return Some(107),
                "-" | "subtract" => return Some(109),
                "." | "decimal" => return Some(110),
                "/" | "divide" => return Some(111),
                _ => {}
            }
        }
    }
    if let Some(rest) = n.strip_prefix('d') {
        if rest.len() == 1 && rest.chars().all(|c| c.is_ascii_digit()) {
            return rest.parse::<i32>().ok().map(|v| 48 + v);
        }
    }
    if let Some(rest) = n.strip_prefix("vk") {
        if let Ok(v) = rest.parse::<i32>() {
            if valid(v) {
                return Some(v);
            }
        }
    }
    let v = match n.as_str() {
        "backspace" | "back" => 8,
        "tab" => 9,
        "clear" => 12,
        "enter" | "return" => 13,
        "pause" | "break" => 19,
        "capslock" | "capital" => 20,
        "esc" | "escape" => 27,
        "space" | "spacebar" | "bosluk" => 32,
        "pageup" | "pgup" | "prior" => 33,
        "pagedown" | "pgdn" | "next" => 34,
        "end" => 35,
        "home" => 36,
        "left" | "sol" | "leftarrow" => 37,
        "up" | "yukari" | "uparrow" => 38,
        "right" | "sag" | "rightarrow" => 39,
        "down" | "asagi" | "downarrow" => 40,
        "printscreen" | "prtsc" | "snapshot" => 44,
        "insert" | "ins" => 45,
        "delete" | "del" => 46,
        "apps" | "menu" | "contextmenu" => 93,
        "sleep" => 95,
        "multiply" => 106,
        "add" => 107,
        "separator" => 108,
        "subtract" => 109,
        "decimal" => 110,
        "divide" => 111,
        "numlock" => 144,
        "scrolllock" | "scroll" => 145,
        "browserback" => 166,
        "browserforward" => 167,
        "browserrefresh" => 168,
        "browserstop" => 169,
        "browsersearch" => 170,
        "browserfavorites" => 171,
        "browserhome" => 172,
        "volumemute" | "mute" => 173,
        "volumedown" => 174,
        "volumeup" => 175,
        "medianexttrack" | "nexttrack" => 176,
        "mediaprevioustrack" | "previoustrack" | "prevtrack" => 177,
        "mediastop" => 178,
        "mediaplaypause" | "playpause" => 179,
        "launchmail" => 180,
        "selectmedia" => 181,
        "launchapp1" | "launchapplication1" => 182,
        "launchapp2" | "launchapplication2" => 183,
        "oem1" | "oemsemicolon" => 186,
        "oemplus" | "plus" => 187,
        "oemcomma" | "comma" => 188,
        "oemminus" | "minus" => 189,
        "oemperiod" | "period" => 190,
        "oem2" | "oemquestion" => 191,
        "oem3" | "oemtilde" => 192,
        "oem4" | "oemopenbrackets" => 219,
        "oem5" | "oempipe" => 220,
        "oem6" | "oemclosebrackets" => 221,
        "oem7" | "oemquotes" => 222,
        "oem8" => 223,
        "oem102" | "oembackslash" => 226,
        "lbutton" | "leftclick" | "mouseleft" => 1,
        "rbutton" | "rightclick" | "mouseright" => 2,
        "mbutton" | "middleclick" | "mousemiddle" => 4,
        "xbutton1" | "mouse4" | "mouseback" => 5,
        "xbutton2" | "mouse5" | "mouseforward" => 6,
        "wheelup" => WHEEL_UP,
        "wheeldown" => WHEEL_DOWN,
        "wheelleft" => WHEEL_LEFT,
        "wheelright" => WHEEL_RIGHT,
        _ => return None,
    };
    Some(v)
}

fn modifier_flag(part: &str) -> i32 {
    match fold(part.trim()).as_str() {
        "ctrl" | "control" | "kontrol" | "ctl" => MOD_CTRL,
        "alt" => MOD_ALT,
        "shift" => MOD_SHIFT,
        "win" | "windows" | "super" | "meta" => MOD_WIN,
        _ => 0,
    }
}

fn split_combo(text: &str) -> Vec<String> {
    let t = text.trim();
    // "Ctrl++" means Ctrl and the plus key.
    let t = if t.ends_with("++") { format!("{}OemPlus", &t[..t.len() - 1]) } else { t.to_string() };
    t.split('+').map(|p| p.trim().to_string()).filter(|p| !p.is_empty()).collect()
}

/// Parse a combo with exactly one main key ("Ctrl+S").
pub fn parse_combo(text: &str) -> Option<(i32, i32)> {
    let parts = split_combo(text);
    if parts.is_empty() {
        return None;
    }
    let mut mods = 0;
    for p in &parts[..parts.len() - 1] {
        let f = modifier_flag(p);
        if f == 0 || mods & f != 0 {
            return None;
        }
        mods |= f;
    }
    let key = parse_key(parts.last().unwrap())?;
    if is_modifier(key) {
        return None;
    }
    Some((mods, key))
}

/// Parse a trigger that may contain a prefix key ("Del+5", "Ctrl+F13+A").
pub fn parse_trigger(text: &str) -> Option<(i32, i32, i32)> {
    let parts = split_combo(text);
    if parts.is_empty() {
        return None;
    }
    let mut mods = 0;
    let mut keys = Vec::new();
    for p in &parts {
        let f = modifier_flag(p);
        if f != 0 {
            if mods & f != 0 {
                return None;
            }
            mods |= f;
        } else {
            keys.push(parse_key(p)?);
        }
    }
    match keys.len() {
        1 => Some((mods, keys[0], 0)),
        2 if prefix_allowed(keys[0]) && keys[0] != keys[1] => Some((mods, keys[1], keys[0])),
        _ => None,
    }
}

/// Why a trigger cannot be used, or None.
pub fn reserved(mods: i32, key: i32) -> Option<&'static str> {
    use crate::i18n::t;
    if !(0..=15).contains(&mods) || !valid(key) {
        return Some(t("Invalid key or modifier.", "Geçersiz tuş veya değiştirici."));
    }
    if key == 0 || is_modifier(key) {
        return Some(t("Select a primary key.", "Bir ana tuş seçin."));
    }
    if mods & MOD_WIN != 0 && !is_pointer(key) {
        // Win+letter combos are mostly owned by Windows; still allowed for F13–F24 and media keys.
        if !(124..=135).contains(&key) && !(166..=183).contains(&key) {
            return Some(t(
                "Most Windows-key combinations are reserved by Windows. Use F13–F24, media keys or another modifier.",
                "Windows tuşlu kombinasyonların çoğu Windows'a ayrılmıştır. F13–F24, medya tuşları veya başka bir değiştirici kullanın.",
            ));
        }
    }
    if (mods == 3 && key == 46) || (mods == 1 && matches!(key, 9 | 27 | 115)) || (mods == 2 && key == 27) || (mods == 6 && key == 27) {
        return Some(t("This system combination cannot be assigned securely.", "Bu sistem kombinasyonu güvenli şekilde atanamaz."));
    }
    None
}

pub fn sendable(mods: i32, key: i32) -> Option<&'static str> {
    use crate::i18n::t;
    if !(0..=15).contains(&mods) || !valid(key) || is_modifier(key) {
        return Some(t("Key to send is invalid.", "Gönderilecek tuş geçersiz."));
    }
    if mods == 3 && key == 46 {
        return Some(t("Ctrl+Alt+Del cannot be synthesized by applications.", "Ctrl+Alt+Del uygulamalar tarafından gönderilemez."));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses() {
        assert_eq!(parse_combo("Ctrl+S"), Some((2, 83)));
        assert_eq!(parse_combo("ctrl + shift + p"), Some((6, 80)));
        assert_eq!(parse_trigger("Del+5"), Some((0, 53, 46)));
        assert_eq!(parse_trigger("Ctrl+Alt+F6"), Some((3, 117, 0)));
        assert_eq!(parse_trigger("Shift+Numpad*"), Some((4, 106, 0)));
        assert_eq!(parse_combo("WheelUp"), Some((0, WHEEL_UP)));
        assert_eq!(trigger_text(0, 53, 46), "Del+5");
        assert_eq!(parse_trigger(&trigger_text(3, 117, 0)), Some((3, 117, 0)));
    }
}
