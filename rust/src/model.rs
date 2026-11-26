//! Settings model. Field names match the 2.x C# DataContract JSON so existing
//! settings import unchanged; unknown fields are preserved in `extra`.
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

fn yes() -> bool {
    true
}
fn new_id() -> String {
    crate::win::uuid_v4()
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct Binding {
    pub id: String,
    pub name: String,
    pub key: i32,
    pub mods: i32,
    #[serde(skip_serializing_if = "is_zero")]
    pub prefix_key: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser_path: Option<String>,
    pub app_path: String,
    pub kind: String,
    pub target: String,
    pub arguments: String,
    #[serde(default = "yes")]
    pub enabled: bool,
    pub pass_through: bool,
    pub send_key: i32,
    pub send_mods: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steps: Option<Vec<ActionStep>>,
    pub profile_id: String,
    #[serde(skip_serializing_if = "is_false")]
    pub is_macro: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gesture: Option<ShapeTemplate>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "PascalCase", default)]
pub struct GesturePoint {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "PascalCase", default)]
pub struct ShapeTemplate {
    pub points: Vec<GesturePoint>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct GestureOptions {
    pub enabled: bool,
    pub device: String,
    pub pause_milliseconds: i32,
    pub minimum_length: f64,
    pub minimum_score: f64,
    pub minimum_margin: f64,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}
impl Default for GestureOptions {
    fn default() -> Self {
        GestureOptions { enabled: true, device: String::new(), pause_milliseconds: 600, minimum_length: 120.0, minimum_score: 0.88, minimum_margin: 0.06, extra: Map::new() }
    }
}

impl Default for Binding {
    fn default() -> Self {
        Binding {
            id: new_id(),
            name: String::new(),
            key: 0,
            mods: 0,
            prefix_key: 0,
            browser_path: None,
            app_path: String::new(),
            kind: "Workflow".into(),
            target: String::new(),
            arguments: String::new(),
            enabled: true,
            pass_through: false,
            send_key: 0,
            send_mods: 0,
            prompt: None,
            steps: Some(Vec::new()),
            profile_id: String::new(),
            is_macro: false,
            gesture: None,
            extra: Map::new(),
        }
    }
}

fn null_default<'de, D, T>(d: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    Ok(Option::<T>::deserialize(d)?.unwrap_or_default())
}

fn is_zero(v: &i32) -> bool {
    *v == 0
}
fn is_false(v: &bool) -> bool {
    !*v
}
fn is_zero_f(v: &f64) -> bool {
    *v == 0.0
}

impl Binding {
    pub fn steps(&self) -> &[ActionStep] {
        self.steps.as_deref().unwrap_or(&[])
    }
    pub fn scope_name(&self) -> String {
        if self.app_path.is_empty() {
            crate::i18n::t("Global", "Genel").to_string()
        } else {
            crate::win::file_name(&self.app_path)
        }
    }
    pub fn combo(&self) -> String {
        if self.gesture.is_some() {
            return crate::i18n::t("Touchpad shape", "Touchpad şekli").to_string();
        }
        crate::keys::label(self.mods, self.key, self.prefix_key)
    }
    pub fn summary(&self) -> String {
        match self.kind.as_str() {
            "System" => crate::system_actions::title(&self.target),
            "Browser website" => format!(
                "{} → {}",
                crate::apps::display_name(self.browser_path.as_deref().unwrap_or("")),
                self.target
            ),
            "Workflow" => self
                .steps()
                .iter()
                .map(|s| s.summary())
                .collect::<Vec<_>>()
                .join(" → "),
            "Send key" => crate::keys::label(self.send_mods, self.send_key, 0),
            "Disable key" => crate::i18n::t("Key switched off", "Tuş kapalı").into(),
            _ => self.target.clone(),
        }
    }
    /// The application paths this binding touches (scope, launch target, steps).
    pub fn touches(&self, path: &str) -> bool {
        let same = |p: &str| crate::win::same_path(p, path);
        same(&self.app_path)
            || (self.kind == "Application" && same(&self.target))
            || (self.kind == "Browser website" && same(self.browser_path.as_deref().unwrap_or("")))
            || self.steps().iter().any(|s| same(s.app_path.as_deref().unwrap_or("")))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct ActionStep {
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_path: Option<String>,
    pub key: i32,
    pub mods: i32,
    pub delay: i32,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub automation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_class: Option<String>,
    #[serde(skip_serializing_if = "is_zero")]
    pub control_type: i32,
    #[serde(skip_serializing_if = "is_zero")]
    pub mouse_button: i32,
    #[serde(skip_serializing_if = "is_zero")]
    pub scroll_delta: i32,
    #[serde(skip_serializing_if = "is_false")]
    pub alt_gr: bool,
    #[serde(skip_serializing_if = "is_zero")]
    pub duration: i32,
    #[serde(skip_serializing_if = "is_false")]
    pub horizontal: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub scale_with_window: bool,
    #[serde(skip_serializing_if = "is_zero_f")]
    pub x: f64,
    #[serde(skip_serializing_if = "is_zero_f")]
    pub y: f64,
    #[serde(skip_serializing_if = "is_zero_f")]
    pub x2: f64,
    #[serde(skip_serializing_if = "is_zero_f")]
    pub y2: f64,
    #[serde(skip_serializing_if = "is_zero_f")]
    pub window_width: f64,
    #[serde(skip_serializing_if = "is_zero_f")]
    pub window_height: f64,
    // 3.0 additions
    /// File path or second operand (destination, menu path…).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Variable that receives the step's output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub var: Option<String>,
    /// Instruction for AI steps.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instruction: Option<String>,
    /// Where an AI/read step delivers its result: var, type, paste, clipboard, notify, file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    #[serde(skip_serializing_if = "is_false")]
    pub append: bool,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Default for ActionStep {
    fn default() -> Self {
        ActionStep {
            kind: "Key".into(),
            app_path: None,
            key: 0,
            mods: 0,
            delay: 0,
            text: String::new(),
            automation_id: None,
            window_title: None,
            window_class: None,
            control_type: 0,
            mouse_button: 0,
            scroll_delta: 0,
            alt_gr: false,
            duration: 0,
            horizontal: false,
            scale_with_window: false,
            x: 0.0,
            y: 0.0,
            x2: 0.0,
            y2: 0.0,
            window_width: 0.0,
            window_height: 0.0,
            path: None,
            var: None,
            instruction: None,
            output: None,
            append: false,
            extra: Map::new(),
        }
    }
}

impl ActionStep {
    pub fn new(kind: &str) -> Self {
        ActionStep { kind: kind.into(), ..Default::default() }
    }
    pub fn app(&self) -> &str {
        self.app_path.as_deref().unwrap_or("")
    }
    pub fn summary(&self) -> String {
        use crate::i18n::t;
        let app = || crate::apps::display_name(self.app());
        let p = self.path.clone().unwrap_or_default();
        match self.kind.as_str() {
            "Key" => crate::keys::label(self.mods, self.key, 0),
            "Text" => format!("{} “{}”", t("Type", "Yaz"), short(&self.text, 60)),
            "Paste text" => format!("{} “{}”", t("Paste", "Yapıştır"), short(&self.text, 60)),
            "Paste files" => format!("{} {}", t("Paste files:", "Dosya yapıştır:"), short(&self.text.lines().map(crate::win::file_name).collect::<Vec<_>>().join(", "), 60)),
            "Wait" => format!("{} ms {}", self.delay, t("wait", "bekle")),
            "Click button" => format!("{}: {}", t("Button", "Düğme"), self.text),
            "Click element" => format!("{}: {}", t("Element", "Öğe"), if self.text.is_empty() { self.automation_id.clone().unwrap_or_default() } else { self.text.clone() }),
            "Set field" => format!("{} {} = “{}”", t("Field", "Alan"), self.automation_id.clone().filter(|s| !s.is_empty()).unwrap_or(p.clone()), short(&self.text, 40)),
            "Read field" => format!("{} {} → {{{}}}", t("Read field", "Alanı oku"), self.automation_id.clone().filter(|s| !s.is_empty()).unwrap_or(self.text.clone()), self.var.clone().unwrap_or("field".into())),
            "Menu command" => format!("{}: {} › {}", t("Menu", "Menü"), app(), self.text),
            "Open application" => format!("{}: {}{}", t("Open", "Aç"), app(), if self.text.is_empty() { String::new() } else { format!(" {}", self.text) }),
            "Open content" => format!("{}: {} → {}", t("Open", "Aç"), app(), short(&self.text, 60)),
            "Focus" => format!("{}: {}", t("Focus", "Öne getir"), app()),
            "Wait for window" => format!("{}: {}", t("Wait for window", "Pencere bekle"), if self.text.is_empty() { app() } else { self.text.clone() }),
            "Window action" => format!("{}: {}", t("Window", "Pencere"), self.text),
            "PowerShell" | "Terminal (cmd)" => format!("{}: {}", self.kind, short(&self.text, 60)),
            "System" => crate::system_actions::title(&self.text),
            "Copy selection" => format!("{} → {{{}}}", t("Copy selection", "Seçimi kopyala"), self.var.clone().unwrap_or("selection".into())),
            "Clipboard set" => format!("{} “{}”", t("Clipboard =", "Pano ="), short(&self.text, 50)),
            "Clipboard append" => format!("{} “{}”", t("Clipboard +=", "Panoya ekle"), short(&self.text, 50)),
            "Write file" => format!("{} {} {}", if self.append { t("Append to", "Dosyaya ekle") } else { t("Write", "Dosyaya yaz") }, p, short(&self.text, 30)),
            "Read file" => format!("{} {} → {{{}}}", t("Read", "Oku"), p, self.var.clone().unwrap_or("file".into())),
            "Copy file" => format!("{} {} → {}", t("Copy file", "Dosya kopyala"), p, self.text),
            "Move file" => format!("{} {} → {}", t("Move file", "Dosya taşı"), p, self.text),
            "AI" => format!("AI: {}", short(self.instruction.as_deref().unwrap_or(""), 60)),
            "Notify" => format!("{}: {}", t("Notify", "Bildir"), short(&self.text, 60)),
            "Set variable" => format!("{{{}}} = “{}”", self.var.clone().unwrap_or_default(), short(&self.text, 40)),
            "Mouse click" | "Mouse drag" | "Scroll" => format!(
                "{} • ({}, {}){}",
                self.kind,
                self.x.round(),
                self.y.round(),
                if self.kind == "Mouse drag" { format!(" → ({}, {})", self.x2.round(), self.y2.round()) } else { String::new() }
            ),
            other => other.to_string(),
        }
    }
}

pub fn short(s: &str, n: usize) -> String {
    let one_line = s.replace(['\r', '\n'], " ");
    if one_line.chars().count() <= n {
        one_line
    } else {
        one_line.chars().take(n).collect::<String>() + "…"
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct Activity {
    pub time: String,
    pub name: String,
    pub result: String,
}
impl Default for Activity {
    fn default() -> Self {
        Activity { time: String::new(), name: String::new(), result: String::new() }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct ShortcutProfile {
    pub id: String,
    pub name: String,
    pub suppress_typing_keys: bool,
}
impl Default for ShortcutProfile {
    fn default() -> Self {
        ShortcutProfile { id: String::new(), name: String::new(), suppress_typing_keys: false }
    }
}

pub fn default_profiles() -> Vec<ShortcutProfile> {
    vec![
        ShortcutProfile { id: "work".into(), name: "Work".into(), suppress_typing_keys: false },
        ShortcutProfile { id: "game".into(), name: "Game".into(), suppress_typing_keys: true },
        ShortcutProfile { id: "study".into(), name: "Study".into(), suppress_typing_keys: false },
    ]
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct ThemePalette {
    pub id: String,
    pub name: String,
    pub description: String,
    pub dark: bool,
    pub background: String,
    pub panel: String,
    pub surface: String,
    pub ink: String,
    pub muted: String,
    pub line: String,
    pub hero: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accent_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accent_text_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peach_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selection_color: Option<String>,
}
impl Default for ThemePalette {
    fn default() -> Self {
        ThemePalette {
            id: String::new(),
            name: String::new(),
            description: String::new(),
            dark: true,
            background: "#181A1E".into(),
            panel: "#22252B".into(),
            surface: "#2D323A".into(),
            ink: "#F3F4F7".into(),
            muted: "#B5BCC7".into(),
            line: "#464D58".into(),
            hero: "#262C34".into(),
            accent_color: None,
            accent_text_color: None,
            peach_color: None,
            selection_color: None,
        }
    }
}
impl ThemePalette {
    pub fn accent(&self) -> String {
        self.accent_color.clone().unwrap_or_else(|| if self.dark { "#FF8174".into() } else { "#B63F43".into() })
    }
    pub fn accent_ink(&self) -> String {
        self.accent_text_color.clone().unwrap_or_else(|| if self.dark { "#321719".into() } else { "#FFFFFF".into() })
    }
    pub fn selection(&self) -> String {
        self.selection_color.clone().unwrap_or_else(|| if self.dark { "#493738".into() } else { "#F9E2DB".into() })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct ModelOptions {
    pub remote: bool,
    pub endpoint: String,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protected_key: Option<String>,
    pub structured_output: bool,
    pub fast_mode: bool,
    pub max_output_tokens: i32,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}
impl Default for ModelOptions {
    fn default() -> Self {
        ModelOptions {
            remote: false,
            endpoint: "http://localhost:1234/v1/".into(),
            model: String::new(),
            protected_key: None,
            structured_output: true,
            fast_mode: true,
            max_output_tokens: 4096,
            extra: Map::new(),
        }
    }
}
impl ModelOptions {
    pub fn token_budget(&self) -> i32 {
        if self.max_output_tokens <= 0 {
            4096
        } else {
            self.max_output_tokens.clamp(256, 65536)
        }
    }
    pub fn api_key(&self) -> String {
        self.protected_key
            .as_deref()
            .and_then(crate::win::dpapi_unprotect_b64)
            .unwrap_or_default()
    }
    pub fn set_api_key(&mut self, key: &str) {
        self.protected_key = if key.is_empty() { None } else { crate::win::dpapi_protect_b64(key) };
    }
}

/// One element found inside an application: a menu item, a button, a field…
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "PascalCase", default)]
pub struct AppCommand {
    /// "menu" (Win32 menu, invoked by WM_COMMAND), "element" (UI Automation), "shortcut".
    pub source: String,
    /// Human label, for menus the full path: "File › Save As…".
    pub label: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub accelerator: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub automation_id: String,
    #[serde(skip_serializing_if = "is_zero")]
    pub control_type: i32,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub role: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "PascalCase", default)]
pub struct AppInventory {
    pub app_path: String,
    pub scanned: String,
    pub window_title: String,
    pub commands: Vec<AppCommand>,
}

/// What the shortcut guide lists.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct GuideShow {
    /// Makro+ shortcuts that work only in the app in front.
    pub this_app: bool,
    /// Global Makro+ shortcuts.
    pub global: bool,
    /// Makro+ shortcuts of every app.
    pub all_apps: bool,
    /// The app's own shortcuts found by a scan.
    pub app_own: bool,
    /// Windows system shortcuts.
    pub system: bool,
}
impl Default for GuideShow {
    fn default() -> Self {
        GuideShow { this_app: true, global: true, all_apps: false, app_own: true, system: false }
    }
}

/// Window background: image, animated GIF or video.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct Background {
    pub path: String,
    /// 0 = fully visible media, 1 = hidden behind the theme colour.
    pub dim: f32,
    /// Opacity of the panels drawn over the background.
    pub panel_opacity: f32,
    pub enabled: bool,
}
impl Default for Background {
    fn default() -> Self {
        Background { path: String::new(), dim: 0.55, panel_opacity: 0.82, enabled: false }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct ClipboardOptions {
    pub enabled: bool,
    pub persist: bool,
    pub max_items: usize,
    pub capture_images: bool,
}
impl Default for ClipboardOptions {
    fn default() -> Self {
        ClipboardOptions { enabled: true, persist: true, max_items: 300, capture_images: false }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct AiOptions {
    /// Send scanned in-app commands of the selected app to the model.
    pub share_app_commands: bool,
    /// Let the model also see the current clipboard text while planning.
    pub share_clipboard: bool,
    /// Correction rounds when a plan fails validation.
    pub correction_rounds: u32,
}
impl Default for AiOptions {
    fn default() -> Self {
        AiOptions { share_app_commands: true, share_clipboard: false, correction_rounds: 2 }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct State {
    pub bindings: Vec<Binding>,
    pub history: Vec<Activity>,
    pub dark: bool,
    pub language: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme_id: Option<String>,
    pub custom_themes: Vec<ThemePalette>,
    pub paused: bool,
    pub application_paths: Vec<String>,
    pub model_path: String,
    pub model_options: ModelOptions,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step_interval_ms: Option<i32>,
    pub shortcut_sources: Vec<String>,
    pub profiles: Vec<ShortcutProfile>,
    pub active_profile_id: String,
    pub preferences: Value,
    pub guide_enabled: bool,
    pub guide_hold_key: i32,
    /// Switch off the guide key's normal function (it only opens the guide).
    pub guide_block_key: bool,
    pub guide_show: GuideShow,
    pub background: Background,
    /// Re-derive the theme from the background when it changes.
    pub auto_theme: bool,
    /// Linux: grab keyboards so matched keys can be swallowed (needs restart).
    pub linux_exclusive: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_gpu: Option<bool>,
    #[serde(deserialize_with = "null_default")]
    pub gestures: GestureOptions,
    pub allow_remapped_input: bool,
    pub updates: Value,
    // 3.0 additions
    pub app_inventories: Vec<AppInventory>,
    pub clipboard: ClipboardOptions,
    pub ai: AiOptions,
    pub start_with_windows: bool,
    pub start_minimized: bool,
    pub close_to_tray: bool,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Default for State {
    fn default() -> Self {
        State {
            bindings: Vec::new(),
            history: Vec::new(),
            dark: true,
            language: "tr".into(),
            theme_id: None,
            custom_themes: Vec::new(),
            paused: false,
            application_paths: Vec::new(),
            model_path: String::new(),
            model_options: ModelOptions::default(),
            step_interval_ms: None,
            shortcut_sources: Vec::new(),
            profiles: default_profiles(),
            active_profile_id: "work".into(),
            preferences: Value::Null,
            guide_enabled: true,
            guide_hold_key: 163,
            guide_block_key: false,
            guide_show: GuideShow::default(),
            background: Background::default(),
            auto_theme: false,
            linux_exclusive: false,
            use_gpu: None,
            gestures: GestureOptions::default(),
            allow_remapped_input: false,
            updates: Value::Null,
            app_inventories: Vec::new(),
            clipboard: ClipboardOptions::default(),
            ai: AiOptions::default(),
            start_with_windows: false,
            start_minimized: false,
            close_to_tray: true,
            extra: Map::new(),
        }
    }
}

impl State {
    pub fn normalize(&mut self) {
        if self.profiles.is_empty() {
            self.profiles = default_profiles();
        }
        self.profiles.retain(|p| !p.id.trim().is_empty() && !p.name.trim().is_empty());
        if self.profiles.is_empty() {
            self.profiles = default_profiles();
        }
        if !self.profiles.iter().any(|p| p.id == self.active_profile_id) {
            self.active_profile_id = self.profiles[0].id.clone();
        }
        let ids: Vec<String> = self.profiles.iter().map(|p| p.id.clone()).collect();
        let mut seen = std::collections::HashSet::new();
        for b in &mut self.bindings {
            if b.id.trim().is_empty() || !seen.insert(b.id.clone()) {
                b.id = new_id();
                seen.insert(b.id.clone());
            }
            if !b.profile_id.is_empty() && !ids.contains(&b.profile_id) {
                b.profile_id.clear();
            }
        }
        if self.language != "en" && self.language != "tr" {
            self.language = "tr".into();
        }
        if self.guide_hold_key == 0 {
            self.guide_hold_key = 163;
        }
        if let Some(ms) = self.step_interval_ms {
            self.step_interval_ms = Some(ms.clamp(0, 2000));
        }
        let mut apps: Vec<String> = Vec::new();
        for p in std::mem::take(&mut self.application_paths) {
            if !p.trim().is_empty() && !apps.iter().any(|a| crate::win::same_path(a, &p)) {
                apps.push(p);
            }
        }
        self.application_paths = apps;
        if self.history.len() > 300 {
            self.history.truncate(300);
        }
        if self.clipboard.max_items == 0 {
            self.clipboard.max_items = 300;
        }
    }

    pub fn active_profile(&self) -> Option<&ShortcutProfile> {
        self.profiles.iter().find(|p| p.id == self.active_profile_id)
    }

    pub fn profile_active(b: &Binding, profile: Option<&ShortcutProfile>) -> bool {
        let in_profile = b.profile_id.is_empty() || profile.map(|p| p.id == b.profile_id).unwrap_or(false);
        let suppressed = profile.map(|p| p.suppress_typing_keys).unwrap_or(false)
            && b.gesture.is_none()
            && b.app_path.is_empty()
            && b.mods == 0
            && b.prefix_key == 0
            && b.key < 112
            && !crate::keys::is_pointer(b.key);
        in_profile && !suppressed
    }

    /// Bindings the engine may run, profile-specific first.
    pub fn effective(&self) -> Vec<Binding> {
        let profile = self.active_profile();
        let mut list: Vec<Binding> = self
            .bindings
            .iter()
            .filter(|b| b.enabled && b.gesture.is_none() && Self::profile_active(b, profile))
            .cloned()
            .collect();
        list.sort_by_key(|b| b.profile_id.is_empty());
        list
    }

    pub fn remember_app(&mut self, path: &str) -> bool {
        if !crate::platform::is_app_path(path) {
            return false;
        }
        if self.application_paths.iter().any(|p| crate::win::same_path(p, path)) {
            return false;
        }
        self.application_paths.push(path.to_string());
        true
    }

    pub fn log(&mut self, name: &str, result: &str) {
        self.history.insert(
            0,
            Activity {
                time: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                name: name.to_string(),
                result: result.to_string(),
            },
        );
        self.history.truncate(300);
    }

    pub fn inventory(&self, app: &str) -> Option<&AppInventory> {
        self.app_inventories.iter().find(|i| crate::win::same_path(&i.app_path, app))
    }
}
