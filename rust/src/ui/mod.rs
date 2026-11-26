//! Window: navigation, shared widgets and event handling.
mod apps_page;
mod clipboard_page;
mod editor;
mod pages;
mod prompt;
mod settings;

use crate::i18n::t;
use crate::model::{ActionStep, Binding, State};
use crate::theme::Colors;
use crate::UiEvent;
use eframe::egui::{self, Color32, FontFamily, RichText, Stroke, Vec2};
use std::sync::mpsc::Receiver;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Page {
    Home,
    Shortcuts,
    Apps,
    Prompt,
    Clipboard,
    Recorder,
    Recommended,
    Conflicts,
    History,
    Model,
    Appearance,
    Settings,
}

impl Page {
    fn title(self) -> &'static str {
        match self {
            Page::Home => t("Home", "Ana sayfa"),
            Page::Shortcuts => t("Shortcuts", "Kısayollar"),
            Page::Apps => t("Applications", "Uygulamalar"),
            Page::Prompt => t("Create with AI", "Yapay zekâ ile oluştur"),
            Page::Clipboard => t("Clipboard", "Pano"),
            Page::Recorder => t("Record macro", "Makro kaydet"),
            Page::Recommended => t("Windows actions", "Windows eylemleri"),
            Page::Conflicts => t("Conflicts", "Çakışmalar"),
            Page::History => t("History", "Geçmiş"),
            Page::Model => t("AI model", "Yapay zekâ modeli"),
            Page::Appearance => t("Appearance", "Görünüm"),
            Page::Settings => t("Settings", "Ayarlar"),
        }
    }
    fn icon(self) -> char {
        match self {
            Page::Home => '\u{E80F}',
            Page::Shortcuts => '\u{E765}',
            Page::Apps => '\u{E71D}',
            Page::Prompt => '\u{E99A}',
            Page::Clipboard => '\u{E77F}',
            Page::Recorder => '\u{E7C8}',
            Page::Recommended => '\u{E770}',
            Page::Conflicts => '\u{E7BA}',
            Page::History => '\u{E81C}',
            Page::Model => '\u{E950}',
            Page::Appearance => '\u{E790}',
            Page::Settings => '\u{E713}',
        }
    }
}

/// Where a captured key goes.
#[derive(Clone, Copy, PartialEq)]
pub enum CaptureTarget {
    EditorTrigger,
    EditorSendKey,
    EditorStep(usize),
    PromptTrigger,
}

pub struct App {
    rx: Receiver<UiEvent>,
    pub page: Page,
    pub c: Colors,
    palette_key: String,
    logo: Option<egui::TextureHandle>,
    pub status: String,
    notice: Option<String>,
    hwnd: isize,
    #[allow(dead_code)]
    ctx: egui::Context,
    start_hidden: bool,
    frames: u64,
    quitting: bool,
    tray_hint_shown: bool,
    pub capture: Option<CaptureTarget>,
    pub editor: Option<editor::Editor>,
    pub shortcuts: pages::ShortcutsState,
    pub recorder: pages::RecorderState,
    pub prompt: prompt::PromptState,
    pub apps: apps_page::AppsState,
    pub clip: clipboard_page::ClipState,
    pub settings: settings::SettingsState,
    lang: String,
    update_dismissed: Vec<String>,
    pub bg: crate::background::Background,
    auto_theme_for: String,
}

pub fn alpha(c: Color32, a: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (a.clamp(0.0, 1.0) * 255.0) as u8)
}

pub fn icon_font() -> FontFamily {
    FontFamily::Name("icons".into())
}

pub fn bold() -> FontFamily {
    FontFamily::Name("bold".into())
}

pub fn ico(ch: char) -> RichText {
    RichText::new(ch.to_string()).family(icon_font())
}

fn load_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let dir = std::path::PathBuf::from(std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into())).join("Fonts");
    let read = |name: &str| std::fs::read(dir.join(name)).ok();
    if let Some(b) = read("segoeui.ttf") {
        fonts.font_data.insert("segoe".into(), egui::FontData::from_owned(b));
        fonts.families.get_mut(&FontFamily::Proportional).unwrap().insert(0, "segoe".into());
    }
    // Symbol fallback first, so every family (bold, icons) inherits it.
    if let Some(b) = read("seguisym.ttf") {
        fonts.font_data.insert("symbols".into(), egui::FontData::from_owned(b));
        fonts.families.get_mut(&FontFamily::Proportional).unwrap().push("symbols".into());
    }
    let bold_data = read("seguisb.ttf").or_else(|| read("segoeuib.ttf"));
    let mut bold_family = Vec::new();
    if let Some(b) = bold_data {
        fonts.font_data.insert("segoe-bold".into(), egui::FontData::from_owned(b));
        bold_family.push("segoe-bold".to_string());
    }
    bold_family.extend(fonts.families[&FontFamily::Proportional].iter().cloned());
    fonts.families.insert(bold(), bold_family);
    let icons = read("SegoeIcons.ttf").or_else(|| read("segmdl2.ttf"));
    let mut icon_family = Vec::new();
    if let Some(b) = icons {
        fonts.font_data.insert("icons".into(), egui::FontData::from_owned(b));
        icon_family.push("icons".to_string());
    }
    icon_family.extend(fonts.families[&FontFamily::Proportional].iter().cloned());
    fonts.families.insert(icon_font(), icon_family);
    if let Some(b) = read("consola.ttf") {
        fonts.font_data.insert("consolas".into(), egui::FontData::from_owned(b));
        fonts.families.get_mut(&FontFamily::Monospace).unwrap().insert(0, "consolas".into());
    }
    ctx.set_fonts(fonts);
    ctx.style_mut(|s| {
        use egui::{FontId, TextStyle};
        s.text_styles.insert(TextStyle::Body, FontId::proportional(14.5));
        s.text_styles.insert(TextStyle::Button, FontId::proportional(14.0));
        s.text_styles.insert(TextStyle::Small, FontId::proportional(12.0));
        s.text_styles.insert(TextStyle::Heading, FontId::new(26.0, bold()));
        s.text_styles.insert(TextStyle::Monospace, FontId::monospace(13.5));
    });
}

impl App {
    pub fn new(cc: &eframe::CreationContext, rx: Receiver<UiEvent>, notice: String, start_hidden: bool) -> App {
        load_fonts(&cc.egui_ctx);
        let (palette, lang) = {
            let st = crate::app_state().lock();
            (crate::theme::resolve(&st), st.language.clone())
        };
        let c = crate::theme::apply(&cc.egui_ctx, &palette);
        let logo = image::load_from_memory(include_bytes!("../../assets/logo.png")).ok().map(|i| {
            let i = i.into_rgba8();
            let (w, h) = i.dimensions();
            cc.egui_ctx.load_texture("logo", egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &i.into_raw()), egui::TextureOptions::LINEAR)
        });
        let args: Vec<String> = std::env::args().collect();
        let page = args.iter().position(|a| a == "--page").and_then(|i| args.get(i + 1)).map(|p| match p.as_str() {
            "shortcuts" => Page::Shortcuts,
            "apps" => Page::Apps,
            "prompt" => Page::Prompt,
            "clipboard" => Page::Clipboard,
            "recorder" => Page::Recorder,
            "actions" => Page::Recommended,
            "conflicts" => Page::Conflicts,
            "history" => Page::History,
            "model" => Page::Model,
            "appearance" => Page::Appearance,
            "settings" => Page::Settings,
            _ => Page::Home,
        });
        App {
            rx,
            page: page.unwrap_or(Page::Home),
            c,
            palette_key: format!("{:?}{}", palette.id, palette.dark),
            logo,
            status: String::new(),
            notice: if notice.is_empty() { None } else { Some(notice) },
            hwnd: 0,
            ctx: cc.egui_ctx.clone(),
            start_hidden,
            frames: 0,
            quitting: false,
            tray_hint_shown: false,
            capture: None,
            editor: None,
            shortcuts: Default::default(),
            recorder: Default::default(),
            prompt: Default::default(),
            apps: Default::default(),
            clip: Default::default(),
            settings: Default::default(),
            lang,
            update_dismissed: Vec::new(),
            bg: Default::default(),
            auto_theme_for: String::new(),
        }
    }

    /// Persist and push changed settings to the engine.
    pub fn commit(&mut self, st: &mut State) {
        st.normalize();
        crate::engine::sync(st);
        crate::request_save();
        crate::shell::refresh_tip();
    }

    pub fn start_capture(&mut self, target: CaptureTarget) {
        self.capture = Some(target);
        crate::engine::set_mode(crate::engine::MODE_CAPTURE);
    }

    pub fn stop_capture(&mut self) {
        self.capture = None;
        crate::engine::set_mode(if self.recorder.recording { crate::engine::MODE_RECORD } else { crate::engine::MODE_NORMAL });
    }

    pub fn open_editor(&mut self, b: Binding, existing: bool) {
        self.editor = Some(editor::Editor::new(b, existing));
    }

    /// Hide the window (tray). Linux minimises instead, which every desktop supports.
    pub fn win_hide(&mut self) {
        #[cfg(windows)]
        if self.hwnd != 0 {
            crate::win::hide(self.hwnd);
        }
        #[cfg(not(windows))]
        {
            self.ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
            crate::shell::MAIN_VISIBLE.store(false, std::sync::atomic::Ordering::Relaxed);
        }
    }

    pub fn win_show(&mut self) {
        #[cfg(windows)]
        if self.hwnd != 0 {
            crate::win::restore(self.hwnd);
            crate::win::show_normal(self.hwnd);
            crate::win::activate(self.hwnd);
        }
        #[cfg(not(windows))]
        {
            self.ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            self.ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            crate::shell::MAIN_VISIBLE.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }

    pub fn win_minimize(&mut self) {
        #[cfg(windows)]
        if self.hwnd != 0 {
            crate::win::minimize(self.hwnd);
        }
        #[cfg(not(windows))]
        self.ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
    }

    pub fn hide_to_tray(&mut self) {
        self.win_hide();
        if !self.tray_hint_shown {
            self.tray_hint_shown = true;
            crate::shell::notify("Makro+", t("Makro+ keeps running in the tray. Right-click the tray icon to exit.", "Makro+ tepside çalışmaya devam ediyor. Çıkmak için tepsi simgesine sağ tıklayın."));
        }
    }

    fn handle_events(&mut self, ctx: &egui::Context, st: &mut State) {
        while let Ok(e) = self.rx.try_recv() {
            match e {
                UiEvent::Status(s) => self.status = s,
                UiEvent::Captured { mods, key, prefix } => {
                    let target = self.capture;
                    self.stop_capture();
                    if key == 27 && mods == 0 && prefix == 0 {
                        self.status = t("Key recording cancelled.", "Tuş kaydı iptal edildi.").into();
                        continue;
                    }
                    match target {
                        Some(CaptureTarget::EditorTrigger) => {
                            if let Some(ed) = &mut self.editor {
                                ed.b.mods = mods;
                                ed.b.key = key;
                                ed.b.prefix_key = prefix;
                                ed.trigger_text = crate::keys::trigger_text(mods, key, prefix);
                            }
                        }
                        Some(CaptureTarget::EditorSendKey) => {
                            if let Some(ed) = &mut self.editor {
                                ed.b.send_mods = mods;
                                ed.b.send_key = key;
                                ed.send_text = crate::keys::trigger_text(mods, key, 0);
                            }
                        }
                        Some(CaptureTarget::EditorStep(i)) => {
                            if let Some(ed) = &mut self.editor {
                                if let Some(s) = ed.b.steps.as_mut().and_then(|v| v.get_mut(i)) {
                                    s.mods = mods;
                                    s.key = key;
                                    ed.step_combo = crate::keys::trigger_text(mods, key, 0);
                                }
                            }
                        }
                        Some(CaptureTarget::PromptTrigger) => self.prompt.trigger = crate::keys::trigger_text(mods, key, prefix),
                        None => {}
                    }
                }
                UiEvent::Recorded(step) => {
                    if self.recorder.recording {
                        self.recorder.steps.push(step);
                    }
                }
                UiEvent::ClipboardChanged => {}
                UiEvent::ShowPage(p) => {
                    if p == "recorder-stop" {
                        self.recorder.recording = false;
                        crate::engine::set_mode(crate::engine::MODE_NORMAL);
                        self.page = Page::Recorder;
                        self.win_show();
                        continue;
                    }
                    self.page = match p.as_str() {
                        "clipboard" => Page::Clipboard,
                        "apps" => Page::Apps,
                        _ => self.page,
                    };
                    self.show_window();
                }
                UiEvent::TogglePause => {
                    st.paused = !st.paused;
                    if st.paused {
                        crate::actions::CANCEL.store(true, std::sync::atomic::Ordering::SeqCst);
                    }
                    self.commit(st);
                }
                UiEvent::Shown => {}
                UiEvent::Exit => {
                    self.quitting = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                UiEvent::Plan(r) => self.prompt.on_result(r),
                UiEvent::Progress(p) => {
                    if self.prompt.busy {
                        self.prompt.progress = p;
                    } else {
                        self.settings.test_progress = p;
                    }
                }
                UiEvent::Scan(r) => self.apps.on_scan(r, st, &mut self.status),
                UiEvent::UpdateChecked => {}
                UiEvent::GestureDevice(d) => {
                    st.gestures.device = d;
                    if let Some(ed) = &mut self.editor {
                        ed.shape_status = t("Touchpad detected. Now draw your shape.", "Touchpad tanındı. Şimdi şeklinizi çizin.").into();
                    }
                    self.settings.msg = t("Touchpad detected.", "Touchpad tanındı.").into();
                    self.commit(st);
                }
                UiEvent::GestureStatus(s) => {
                    if let Some(ed) = &mut self.editor {
                        ed.shape_status = s;
                    }
                }
                UiEvent::GestureTrace(p) => {
                    if let Some(ed) = &mut self.editor {
                        ed.trace = p;
                    }
                }
                UiEvent::GestureRecorded(p) => {
                    if let Some(ed) = &mut self.editor {
                        ed.trace = p.clone();
                        ed.b.gesture = Some(crate::model::ShapeTemplate { points: p });
                        ed.drawing = false;
                        ed.shape_status = t("Shape recorded. Save to use it.", "Şekil kaydedildi. Kullanmak için kaydedin.").into();
                    }
                }
                UiEvent::AiText(tag, r) => match tag.as_str() {
                    "clip" => self.clip.on_ai(r),
                    _ => self.settings.on_test(r),
                },
            }
        }
        if self.quitting {
            return;
        }
        let _ = ctx;
    }

    fn show_window(&mut self) {
        self.win_show();
    }

    fn sidebar(&mut self, ui: &mut egui::Ui, st: &mut State) {
        let c = self.c;
        let top = ui.max_rect();
        drag_area(ui, egui::Rect::from_min_size(top.min, egui::vec2(top.width(), 82.0)), "drag-side");
        ui.add_space(18.0);
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            if let Some(logo) = &self.logo {
                ui.add(egui::Image::new(egui::load::SizedTexture::new(logo.id(), Vec2::splat(44.0))));
            }
            ui.add_space(4.0);
            ui.vertical(|ui| {
                ui.add_space(9.0);
                ui.label(RichText::new("MAKRO+").family(bold()).size(23.0).color(c.ink));
            });
        });
        ui.add_space(18.0);
        let groups: [&[Page]; 3] = [
            &[Page::Home, Page::Shortcuts, Page::Apps, Page::Prompt, Page::Clipboard, Page::Recorder],
            &[Page::Recommended, Page::Conflicts, Page::History],
            &[Page::Model, Page::Appearance, Page::Settings],
        ];
        egui::ScrollArea::vertical().id_salt("nav").max_height(ui.available_height() - 178.0).show(ui, |ui| {
            for (gi, group) in groups.iter().enumerate() {
                if gi > 0 {
                    ui.add_space(6.0);
                    ui.separator();
                    ui.add_space(4.0);
                }
                for &p in group.iter() {
                    let selected = self.page == p;
                    let fill = if selected { c.selection } else { Color32::TRANSPARENT };
                    let text = RichText::new(format!("{}", p.title())).color(if selected { c.ink } else { c.ink.gamma_multiply(0.86) });
                    let resp = ui
                        .add_sized(
                            [ui.available_width(), 33.0],
                            egui::Button::new(egui::text::LayoutJob::default())
                                .fill(fill)
                                .stroke(if selected { Stroke::new(1.0, c.line) } else { Stroke::NONE }),
                        )
                        .on_hover_cursor(egui::CursorIcon::PointingHand);
                    // Paint icon + label manually for alignment.
                    let r = resp.rect;
                    let painter = ui.painter();
                    if selected {
                        painter.rect_filled(egui::Rect::from_min_size(r.min + egui::vec2(0.0, 8.0), egui::vec2(3.0, r.height() - 16.0)), 2.0, c.accent);
                    }
                    painter.text(r.left_center() + egui::vec2(16.0, 0.0), egui::Align2::LEFT_CENTER, p.icon().to_string(), egui::FontId::new(16.0, icon_font()), if selected { c.accent } else { c.muted });
                    painter.text(r.left_center() + egui::vec2(44.0, 0.0), egui::Align2::LEFT_CENTER, text.text(), egui::FontId::proportional(14.5), if selected { c.ink } else { c.ink.gamma_multiply(0.9) });
                    if resp.clicked() {
                        self.page = p;
                    }
                    if p == Page::Clipboard {
                        let n = crate::shell::clip_store().lock().entries.len();
                        if n > 0 {
                            painter.text(r.right_center() - egui::vec2(12.0, 0.0), egui::Align2::RIGHT_CENTER, n.to_string(), egui::FontId::proportional(12.0), c.muted);
                        }
                    }
                    if p == Page::Shortcuts {
                        painter.text(r.right_center() - egui::vec2(12.0, 0.0), egui::Align2::RIGHT_CENTER, st.bindings.len().to_string(), egui::FontId::proportional(12.0), c.muted);
                    }
                }
            }
        });
        // Footer
        ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
            ui.add_space(12.0);
            ui.label(RichText::new(format!("MAKRO+ • v{}{}", env!("CARGO_PKG_VERSION"), if crate::storage::is_portable() { t(" • portable", " • taşınabilir") } else { "" })).size(11.0).color(c.muted));
            let (label, accent) = if st.paused { (format!("▶  {}", t("Resume shortcuts", "Kısayolları sürdür")), true) } else { (format!("⏸  {}", t("Pause shortcuts", "Kısayolları duraklat")), false) };
            let b = egui::Button::new(RichText::new(label).color(if accent { c.accent_ink } else { c.ink })).fill(if accent { c.accent } else { c.surface }).min_size(egui::vec2(ui.available_width(), 34.0));
            if ui.add(b).clicked() {
                st.paused = !st.paused;
                if st.paused {
                    crate::actions::CANCEL.store(true, std::sync::atomic::Ordering::SeqCst);
                }
                self.commit(st);
            }
            ui.add_space(2.0);
            let current = st.active_profile().map(|p| profile_name(&p.name)).unwrap_or_default();
            let mut chosen = st.active_profile_id.clone();
            egui::ComboBox::from_id_salt("profile").width(ui.available_width()).selected_text(current).show_ui(ui, |ui| {
                for p in &st.profiles {
                    ui.selectable_value(&mut chosen, p.id.clone(), profile_name(&p.name));
                }
            });
            if chosen != st.active_profile_id {
                st.active_profile_id = chosen;
                self.commit(st);
            }
            ui.label(RichText::new(t("ACTIVE PROFILE", "ETKİN PROFİL")).size(10.5).color(c.muted));
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                for (code, name) in [("tr", "Türkçe"), ("en", "English")] {
                    if ui.selectable_label(st.language == code, name).clicked() && st.language != code {
                        st.language = code.into();
                        crate::i18n::set_language(code);
                        self.commit(st);
                    }
                }
            });
        });
    }
}

/// Drag to move, double-click to maximise/restore.
pub fn drag_area(ui: &mut egui::Ui, rect: egui::Rect, id: &str) {
    let r = ui.interact(rect, egui::Id::new(id), egui::Sense::click_and_drag());
    if r.double_clicked() {
        let max = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!max));
    } else if r.drag_started_by(egui::PointerButton::Primary) {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }
}

/// Window buttons in the top-right corner, plus edge resizing.
fn window_chrome(ctx: &egui::Context, c: &Colors, close_to_tray: bool) -> Option<&'static str> {
    use egui::{ResizeDirection as R, ViewportCommand as V};
    let (max, full) = ctx.input(|i| (i.viewport().maximized.unwrap_or(false), i.viewport().fullscreen.unwrap_or(false)));
    if ctx.input(|i| i.key_pressed(egui::Key::F11)) {
        ctx.send_viewport_cmd(V::Fullscreen(!full));
    }
    let mut action = None;
    egui::Area::new(egui::Id::new("window-buttons")).anchor(egui::Align2::RIGHT_TOP, [-6.0, 4.0]).order(egui::Order::Foreground).show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            let btn = |ui: &mut egui::Ui, ch: char, tip: &str, danger: bool| {
                let r = ui.add(egui::Button::new(ico(ch).size(11.0).color(c.ink)).frame(false).min_size(egui::vec2(40.0, 28.0))).on_hover_text(tip);
                if r.hovered() {
                    ui.painter().rect_filled(r.rect, 4.0, if danger { Color32::from_rgb(196, 43, 28) } else { alpha(c.ink, 0.12) });
                    ui.painter().text(r.rect.center(), egui::Align2::CENTER_CENTER, ch.to_string(), egui::FontId::new(11.0, icon_font()), if danger { Color32::WHITE } else { c.ink });
                }
                r.clicked()
            };
            if btn(ui, if full { '\u{E73F}' } else { '\u{E740}' }, if full { t("Exit full screen (F11)", "Tam ekrandan çık (F11)") } else { t("Full screen (F11)", "Tam ekran (F11)") }, false) {
                ctx.send_viewport_cmd(V::Fullscreen(!full));
            }
            if !full {
                if btn(ui, '\u{E921}', t("Minimise", "Simge durumuna küçült"), false) {
                    ctx.send_viewport_cmd(V::Minimized(true));
                }
                if btn(ui, if max { '\u{E923}' } else { '\u{E922}' }, if max { t("Restore", "Önceki boyut") } else { t("Maximise", "Ekranı kapla") }, false) {
                    ctx.send_viewport_cmd(V::Maximized(!max));
                }
                if btn(ui, '\u{E8BB}', if close_to_tray { t("Close to tray", "Tepsiye kapat") } else { t("Close", "Kapat") }, true) {
                    action = Some("close");
                }
            }
        });
    });
    if !max && !full {
        let rect = ctx.screen_rect();
        if let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) {
            let m = 6.0;
            let (l, r, top, b) = (pos.x - rect.left() < m, rect.right() - pos.x < m, pos.y - rect.top() < m, rect.bottom() - pos.y < m);
            let dir = match (l, r, top, b) {
                (true, _, true, _) => Some(R::NorthWest),
                (_, true, true, _) => Some(R::NorthEast),
                (true, _, _, true) => Some(R::SouthWest),
                (_, true, _, true) => Some(R::SouthEast),
                (true, ..) => Some(R::West),
                (_, true, ..) => Some(R::East),
                (_, _, true, _) => Some(R::North),
                (.., true) => Some(R::South),
                _ => None,
            };
            if let Some(d) = dir {
                ctx.set_cursor_icon(match d {
                    R::North | R::South => egui::CursorIcon::ResizeVertical,
                    R::East | R::West => egui::CursorIcon::ResizeHorizontal,
                    R::NorthWest | R::SouthEast => egui::CursorIcon::ResizeNwSe,
                    _ => egui::CursorIcon::ResizeNeSw,
                });
                if ctx.input(|i| i.pointer.primary_pressed()) {
                    ctx.send_viewport_cmd(V::BeginResize(d));
                }
            }
        }
        // Thin border so the frameless window stays visible on similar backgrounds.
        ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("border"))).rect_stroke(rect.shrink(0.5), 0.0, Stroke::new(1.0, c.line));
    }
    action
}

pub fn profile_name(name: &str) -> String {
    match name {
        "Work" => t("Work", "İş").into(),
        "Game" => t("Game", "Oyun").into(),
        "Study" => t("Study", "Ders").into(),
        other => other.into(),
    }
}

// ---------- shared widgets ----------

pub fn card<R>(ui: &mut egui::Ui, c: &Colors, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::none().fill(c.panel).rounding(12.0).stroke(Stroke::new(1.0, c.line.gamma_multiply(0.6))).inner_margin(16.0).show(ui, add).inner
}

pub fn accent_button(ui: &mut egui::Ui, c: &Colors, text: &str) -> egui::Response {
    ui.add(egui::Button::new(RichText::new(text).color(c.accent_ink).family(bold())).fill(c.accent).min_size(egui::vec2(0.0, 32.0)))
}

/// A left-aligned, fixed-width table cell.
pub fn cell(ui: &mut egui::Ui, width: f32, widget: impl egui::Widget) -> egui::Response {
    ui.allocate_ui_with_layout(egui::vec2(width, 24.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
        ui.set_width(width);
        ui.add(widget)
    })
    .inner
}

pub fn button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(egui::Button::new(text).min_size(egui::vec2(0.0, 32.0)))
}

pub fn icon_button(ui: &mut egui::Ui, c: &Colors, ch: char, tip: &str) -> egui::Response {
    ui.add(egui::Button::new(ico(ch).size(14.0).color(c.ink)).min_size(egui::vec2(30.0, 28.0))).on_hover_text(tip)
}

pub fn muted(ui: &mut egui::Ui, c: &Colors, text: &str) {
    ui.label(RichText::new(text).color(c.muted).size(13.0));
}

pub fn section(ui: &mut egui::Ui, c: &Colors, text: &str) {
    ui.add_space(6.0);
    ui.label(RichText::new(text).family(bold()).size(17.0).color(c.ink));
}

pub fn header(ui: &mut egui::Ui, c: &Colors, title: &str, note: &str) {
    ui.label(RichText::new(title).family(bold()).size(27.0).color(c.ink));
    if !note.is_empty() {
        ui.label(RichText::new(note).color(c.muted).size(13.5));
    }
    ui.add_space(12.0);
}

/// Application chooser: registered apps plus "browse".
pub fn app_combo(ui: &mut egui::Ui, id: &str, value: &mut String, apps: &mut Vec<String>, allow_none: bool, none_label: &str, width: f32) -> bool {
    let mut changed = false;
    let shown = if value.is_empty() { none_label.to_string() } else { crate::apps::display_name(value) };
    let mut browse = false;
    egui::ComboBox::from_id_salt(id).width(width).selected_text(shown).show_ui(ui, |ui| {
        if allow_none && ui.selectable_label(value.is_empty(), none_label).clicked() {
            value.clear();
            changed = true;
        }
        for p in apps.iter() {
            if ui.selectable_label(crate::win::same_path(value, p), crate::apps::display_name(p)).on_hover_text(p.as_str()).clicked() {
                *value = p.clone();
                changed = true;
            }
        }
        ui.separator();
        if ui.button(t("Browse for .exe…", ".exe dosyasına gözat…")).clicked() {
            browse = true;
        }
    });
    if browse {
        if let Some(p) = rfd::FileDialog::new().add_filter("exe", &["exe"]).pick_file() {
            let s = p.display().to_string();
            if !apps.iter().any(|a| crate::win::same_path(a, &s)) {
                apps.push(s.clone());
            }
            *value = s;
            changed = true;
        }
    }
    changed
}

pub fn new_step(kind: &str) -> ActionStep {
    let mut s = ActionStep::new(kind);
    match kind {
        "Wait" => s.delay = 500,
        "Window action" => s.text = "maximize".into(),
        "System" => s.text = "lock".into(),
        "AI" => {
            s.text = "{selection}".into();
            s.output = Some("paste".into());
        }
        "Copy selection" => s.var = Some("selection".into()),
        _ => {}
    }
    s
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.frames += 1;
        #[cfg(windows)]
        if self.hwnd == 0 {
            if let Some(w) = crate::win::find_own_window_titled(crate::APP_TITLE) {
                self.hwnd = w;
                crate::shell::MAIN_WINDOW.store(w, std::sync::atomic::Ordering::SeqCst);
            }
        }
        let state = crate::app_state();
        let mut guard = state.lock();
        let st: &mut State = &mut guard;
        if self.frames == 1 {
            // Developer screenshot helpers: --edit N, --detail N.
            let args: Vec<String> = std::env::args().collect();
            let arg = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).and_then(|v| v.parse::<usize>().ok());
            if let Some(b) = arg("--edit").and_then(|i| st.bindings.get(i).cloned()) {
                self.open_editor(b, true);
            }
            if let Some(p) = arg("--detail").and_then(|i| st.application_paths.get(i).cloned()) {
                self.apps.detail = Some(p);
            }
        }
        self.handle_events(ctx, st);
        if self.lang != st.language {
            self.lang = st.language.clone();
            crate::i18n::set_language(&self.lang);
        }
        let palette = crate::theme::resolve(st);
        let key = format!("{}{}{}{}", palette.id, palette.dark, palette.accent(), palette.background);
        if key != self.palette_key {
            self.palette_key = key;
            self.c = crate::theme::apply(ctx, &palette);
            crate::engine::sync(st);
        }
        if self.start_hidden && self.frames == 3 {
            self.start_hidden = false;
            self.tray_hint_shown = true;
            self.win_hide();
        }
        if ctx.input(|i| i.viewport().close_requested()) && !self.quitting && st.close_to_tray {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.hide_to_tray();
        }
        // Background media (image, GIF or video) behind semi-transparent panels.
        let bg_path = if st.background.enabled { st.background.path.clone() } else { String::new() };
        self.bg.set(ctx, &bg_path);
        let has_bg = self.bg.paint(ctx, ctx.screen_rect());
        if st.auto_theme && has_bg && self.auto_theme_for != bg_path {
            let sample = self.bg.sample.lock().clone();
            if let Some((px, w, h)) = sample {
                self.auto_theme_for = bg_path.clone();
                let mut p = crate::theme::palette_from_image(&px, w, h, t("From background", "Arka plandan"));
                p.id = "custom-auto".into();
                st.custom_themes.retain(|x| x.id != p.id);
                st.theme_id = Some(p.id.clone());
                st.dark = p.dark;
                st.custom_themes.push(p);
                self.commit(st);
            }
        }
        let c = self.c;
        let (side_fill, main_fill) = if has_bg { (alpha(c.panel, st.background.panel_opacity), alpha(c.bg, st.background.dim)) } else { (c.panel, c.bg) };
        let modal = self.editor.is_some() || self.apps.adding.is_some();
        egui::SidePanel::left("nav")
            .exact_width(236.0)
            .resizable(false)
            .frame(egui::Frame::none().fill(side_fill).inner_margin(egui::Margin::symmetric(14.0, 6.0)).stroke(Stroke::new(1.0, c.line.gamma_multiply(0.7))))
            .show(ctx, |ui| {
                ui.add_enabled_ui(!modal, |ui| self.sidebar(ui, st));
            });
        egui::TopBottomPanel::bottom("status").frame(egui::Frame::none().fill(main_fill).inner_margin(egui::Margin::symmetric(26.0, 6.0))).show(ctx, |ui| {
            ui.horizontal(|ui| {
                let running = crate::engine::RUNNING.load(std::sync::atomic::Ordering::Relaxed);
                let (dot, txt) = if st.paused {
                    (c.warn, t("Paused", "Duraklatıldı"))
                } else if running {
                    (c.ok, t("Shortcuts active", "Kısayollar etkin"))
                } else {
                    (c.warn, t("Engine starting…", "Motor başlıyor…"))
                };
                ui.label(RichText::new("●").color(dot).size(12.0));
                ui.label(RichText::new(txt).size(12.0).color(c.muted));
                if crate::actions::WORKFLOW_RUNNING.load(std::sync::atomic::Ordering::Relaxed) {
                    ui.label(RichText::new(t("• workflow running (Esc cancels)", "• iş akışı çalışıyor (Esc iptal eder)")).size(12.0).color(c.accent));
                }
                if !self.status.is_empty() {
                    ui.label(RichText::new(format!("•  {}", crate::model::short(&self.status, 160))).size(12.0).color(c.muted));
                }
            });
        });
        egui::CentralPanel::default().frame(egui::Frame::none().fill(main_fill).inner_margin(egui::Margin { left: 28.0, right: 24.0, top: 22.0, bottom: 4.0 })).show(ctx, |ui| {
            let area = ui.max_rect();
            drag_area(ui, egui::Rect::from_min_max(egui::pos2(area.left(), area.top() - 22.0), egui::pos2(area.right() - 180.0, area.top() - 2.0)), "drag-main");
            ui.add_enabled_ui(!modal, |ui| {
                if let Some(n) = self.notice.clone() {
                    egui::Frame::none().fill(c.selection).rounding(10.0).inner_margin(12.0).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&n).color(c.ink));
                            if ui.button(t("OK", "Tamam")).clicked() {
                                self.notice = None;
                            }
                        });
                    });
                    ui.add_space(10.0);
                }
                let upd = crate::updates::status();
                if let Some(a) = upd.available.filter(|a| !self.update_dismissed.contains(&a.tag)) {
                    egui::Frame::none().fill(c.selection).rounding(10.0).stroke(Stroke::new(1.0, c.accent)).inner_margin(12.0).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.label(ico('\u{E896}').size(16.0).color(c.accent));
                            ui.label(RichText::new(crate::i18n::ts(format!("Makro+ {} is available.", a.tag), format!("Makro+ {} yayımlandı.", a.tag))).family(bold()));
                            if accent_button(ui, &c, t("Open download page", "İndirme sayfasını aç")).clicked() {
                                let _ = crate::win::shell_open(&a.url, "", "");
                            }
                            if ui.button(t("Later", "Sonra")).clicked() {
                                self.update_dismissed.push(a.tag.clone());
                            }
                        });
                        if !a.notes.is_empty() {
                            ui.label(RichText::new(&a.notes).size(12.5).color(c.muted));
                        }
                    });
                    ui.add_space(10.0);
                }
                match self.page {
                    Page::Home => pages::home(self, ui, st),
                    Page::Shortcuts => pages::shortcuts(self, ui, st),
                    Page::Apps => apps_page::show(self, ui, st),
                    Page::Prompt => prompt::show(self, ui, st),
                    Page::Clipboard => clipboard_page::show(self, ui, st),
                    Page::Recorder => pages::recorder(self, ui, st),
                    Page::Recommended => pages::recommended(self, ui, st),
                    Page::Conflicts => pages::conflicts(self, ui, st),
                    Page::History => pages::history(self, ui, st),
                    Page::Model => settings::model(self, ui, st),
                    Page::Appearance => settings::appearance(self, ui, st),
                    Page::Settings => settings::settings(self, ui, st),
                }
            });
        });
        if self.editor.is_some() {
            editor::show(self, ctx, st);
        }
        if self.apps.adding.is_some() {
            apps_page::add_dialog(self, ctx, st);
        }
        if self.capture.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(250));
        }
        if window_chrome(ctx, &c, st.close_to_tray) == Some("close") {
            if st.close_to_tray {
                self.hide_to_tray();
            } else {
                self.quitting = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        crate::engine::set_mode(crate::engine::MODE_NORMAL);
    }
}
