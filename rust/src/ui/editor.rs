//! Shortcut editor window.
use super::{accent_button, app_combo, button, ico, muted, new_step, App, CaptureTarget};
use crate::i18n::{t, ts};
use crate::model::{ActionStep, Binding, State};
use crate::rules::{self, Severity};
use eframe::egui::{self, RichText};

pub struct Editor {
    pub b: Binding,
    pub existing: bool,
    pub trigger_text: String,
    pub send_text: String,
    pub step_combo: String,
    pub selected: Option<usize>,
    pub add_kind: String,
    pub error: String,
    pub shape: bool,
    pub shape_status: String,
    pub trace: Vec<crate::model::GesturePoint>,
    pub drawing: bool,
}

impl Editor {
    pub fn new(b: Binding, existing: bool) -> Editor {
        let trigger_text = if b.key != 0 { crate::keys::trigger_text(b.mods, b.key, b.prefix_key) } else { String::new() };
        let send_text = if b.send_key != 0 { crate::keys::trigger_text(b.send_mods, b.send_key, 0) } else { String::new() };
        let selected = if b.steps().is_empty() { None } else { Some(0) };
        let step_combo = b.steps().first().filter(|s| s.kind == "Key" && s.key != 0).map(|s| crate::keys::trigger_text(s.mods, s.key, 0)).unwrap_or_default();
        let shape = b.gesture.is_some();
        let trace = b.gesture.as_ref().map(|g| g.points.clone()).unwrap_or_default();
        Editor { b, existing, trigger_text, send_text, step_combo, selected, add_kind: "Key".into(), error: String::new(), shape, shape_status: String::new(), trace, drawing: false }
    }
}

const VAR_HELP_EN: &str = "Variables: {selection} {clipboard} {ai} {file} {field} {last_output} {last_file} {date} {time} {datetime} {timestamp} {app} {title} {desktop} {documents} {downloads} {newline} {env:NAME} and your own {name}.";
const VAR_HELP_TR: &str = "Değişkenler: {selection} {clipboard} {ai} {file} {field} {last_output} {last_file} {date} {time} {datetime} {timestamp} {app} {title} {desktop} {documents} {downloads} {newline} {env:AD} ve kendi {ad} değişkenleriniz.";

fn path_field(ui: &mut egui::Ui, value: &mut String, hint: &str, folder: bool) {
    ui.horizontal(|ui| {
        ui.add(egui::TextEdit::singleline(value).hint_text(hint).desired_width(ui.available_width() - 90.0));
        if ui.button(t("Browse…", "Gözat…")).clicked() {
            let picked = if folder { rfd::FileDialog::new().pick_folder() } else { rfd::FileDialog::new().pick_file() };
            if let Some(p) = picked {
                *value = p.display().to_string();
            }
        }
    });
}

fn opt_field(ui: &mut egui::Ui, label: &str, v: &mut Option<String>, hint: &str) {
    ui.label(label);
    let mut s = v.clone().unwrap_or_default();
    if ui.add(egui::TextEdit::singleline(&mut s).hint_text(hint).desired_width(f32::INFINITY)).changed() {
        *v = if s.is_empty() { None } else { Some(s) };
    }
}

/// Inventory picker for in-app steps: fills the step from a scanned command.
fn inventory_picker(ui: &mut egui::Ui, st: &State, step: &mut ActionStep, filter: &[&str]) {
    let Some(inv) = st.inventory(step.app()) else {
        if !step.app().is_empty() {
            muted(ui, &crate::theme::colors(&crate::theme::resolve(st)), t("Tip: scan this app on the Applications page to pick its menus and controls from a list.", "İpucu: menüleri ve denetimleri listeden seçmek için bu uygulamayı Uygulamalar sayfasında tarayın."));
        }
        return;
    };
    let items: Vec<&crate::model::AppCommand> = inv.commands.iter().filter(|c| filter.contains(&c.source.as_str())).collect();
    if items.is_empty() {
        return;
    }
    egui::ComboBox::from_id_salt("inv-pick").width(ui.available_width()).selected_text(t("Pick from scanned commands…", "Taranan komutlardan seç…")).show_ui(ui, |ui| {
        for cmd in items.iter().take(400) {
            let label = if cmd.accelerator.is_empty() { cmd.label.clone() } else { format!("{}   [{}]", cmd.label, cmd.accelerator) };
            if ui.selectable_label(false, label).clicked() {
                match step.kind.as_str() {
                    "Menu command" => step.text = cmd.label.clone(),
                    _ => {
                        step.automation_id = Some(cmd.automation_id.clone()).filter(|s| !s.is_empty());
                        step.text = cmd.label.rsplit(" › ").next().unwrap_or(&cmd.label).to_string();
                        step.control_type = cmd.control_type;
                        if step.kind == "Set field" {
                            step.path = Some(step.text.clone());
                            step.text.clear();
                        }
                    }
                }
            }
        }
    });
}

fn step_form(app: &mut App, ui: &mut egui::Ui, st: &mut State, i: usize) {
    let c = app.c;
    let ed = app.editor.as_mut().unwrap();
    let mut capture = false;
    {
        let steps = ed.b.steps.get_or_insert_with(Vec::new);
        let Some(s) = steps.get_mut(i) else { return };
        ui.label(RichText::new(rules::step_label(&s.kind)).family(super::bold()).size(16.0));
        let app_kinds = ["Focus", "Open application", "Open content", "Menu command", "Wait for window", "Window action", "Key", "Text", "Paste text", "Click button", "Click element", "Set field"];
        if app_kinds.contains(&s.kind.as_str()) {
            let required = matches!(s.kind.as_str(), "Focus" | "Open application");
            ui.label(if required { t("Application", "Uygulama") } else { t("Application (optional — must be in front)", "Uygulama (isteğe bağlı — önde olmalı)") });
            let mut v = s.app_path.clone().unwrap_or_default();
            if app_combo(ui, &format!("step-app-{i}"), &mut v, &mut st.application_paths, !required, t("Window in front", "Öndeki pencere"), ui.available_width()) {
                s.app_path = if v.is_empty() { None } else { Some(v) };
            }
        }
        match s.kind.as_str() {
            "Key" => {
                ui.label(t("Keys to press", "Basılacak tuşlar"));
                ui.horizontal(|ui| {
                    if ui.add(egui::TextEdit::singleline(&mut ed.step_combo).hint_text("Ctrl+S").desired_width(220.0)).changed() {
                        if let Some((m, k)) = crate::keys::parse_combo(&ed.step_combo) {
                            s.mods = m;
                            s.key = k;
                        }
                    }
                    if ui.button(format!("⌨ {}", t("Record", "Kaydet"))).clicked() {
                        capture = true;
                    }
                    ui.label(RichText::new(crate::keys::label(s.mods, s.key, 0)).color(c.accent));
                });
                ui.checkbox(&mut s.alt_gr, t("Use AltGr", "AltGr kullan"));
            }
            "Text" | "Paste text" | "Paste files" | "Notify" | "Clipboard set" | "Clipboard append" => {
                ui.label(if s.kind == "Notify" { t("Message", "Mesaj") } else if s.kind == "Paste files" { t("Files (one path per line)", "Dosyalar (satır başına bir yol)") } else { t("Text", "Metin") });
                ui.add(egui::TextEdit::multiline(&mut s.text).desired_rows(4).desired_width(f32::INFINITY));
                if s.kind == "Paste text" {
                    let mut keep = s.output.as_deref() == Some("keep");
                    if ui.checkbox(&mut keep, t("Leave the text on the clipboard afterwards", "Sonrasında metni panoda bırak")).changed() {
                        s.output = if keep { Some("keep".into()) } else { None };
                    }
                }
            }
            "Wait" => {
                ui.horizontal(|ui| {
                    ui.label(t("Duration", "Süre"));
                    ui.add(egui::DragValue::new(&mut s.delay).range(0..=600_000).speed(10).suffix(" ms"));
                });
            }
            "Focus" => {
                ui.horizontal(|ui| {
                    ui.label(t("Wait up to", "En fazla bekle"));
                    let mut v = if s.delay > 0 { s.delay } else { 15000 };
                    if ui.add(egui::DragValue::new(&mut v).range(1000..=120_000).speed(100).suffix(" ms")).changed() {
                        s.delay = v;
                    }
                });
                opt_field(ui, t("Window title (optional, exact)", "Pencere başlığı (isteğe bağlı, tam)"), &mut s.window_title, "");
                muted(ui, &c, t("Starts the app if it is not running, then brings its window to the front.", "Uygulama çalışmıyorsa başlatır, sonra penceresini öne getirir."));
            }
            "Open application" => {
                ui.label(t("Arguments (optional)", "Argümanlar (isteğe bağlı)"));
                ui.text_edit_singleline(&mut s.text);
            }
            "Open content" => {
                ui.label(t("File, folder, URL or app link", "Dosya, klasör, adres veya uygulama bağlantısı"));
                path_field(ui, &mut s.text, "https://… / C:\\… / spotify:…", false);
            }
            "Menu command" => {
                ui.label(t("Menu path", "Menü yolu"));
                ui.add(egui::TextEdit::singleline(&mut s.text).hint_text(t("File › Save As", "Dosya › Farklı Kaydet")).desired_width(f32::INFINITY));
                inventory_picker(ui, st, s, &["menu", "element"]);
            }
            "Click element" | "Click button" | "Read field" => {
                ui.label(t("Element name", "Öğe adı"));
                ui.add(egui::TextEdit::singleline(&mut s.text).desired_width(f32::INFINITY));
                if s.kind != "Click button" {
                    opt_field(ui, t("Automation id (more precise)", "Otomasyon kimliği (daha kesin)"), &mut s.automation_id, "");
                }
                if s.kind == "Read field" {
                    opt_field(ui, t("Store in variable", "Değişkene kaydet"), &mut s.var, "field");
                }
                inventory_picker(ui, st, s, &["element"]);
            }
            "Set field" => {
                opt_field(ui, t("Automation id", "Otomasyon kimliği"), &mut s.automation_id, "");
                opt_field(ui, t("…or field name", "…veya alan adı"), &mut s.path, "");
                ui.label(t("Value", "Değer"));
                ui.add(egui::TextEdit::multiline(&mut s.text).desired_rows(2).desired_width(f32::INFINITY));
                inventory_picker(ui, st, s, &["element"]);
            }
            "Wait for window" => {
                ui.label(t("Title contains", "Başlık şunu içersin"));
                ui.add(egui::TextEdit::singleline(&mut s.text).hint_text(t("Save As", "Farklı Kaydet")).desired_width(f32::INFINITY));
                ui.horizontal(|ui| {
                    ui.label(t("Timeout", "Zaman aşımı"));
                    let mut v = if s.delay > 0 { s.delay } else { 10000 };
                    if ui.add(egui::DragValue::new(&mut v).range(500..=600_000).speed(100).suffix(" ms")).changed() {
                        s.delay = v;
                    }
                });
            }
            "Window action" => {
                egui::ComboBox::from_id_salt("winact").selected_text(s.text.clone()).show_ui(ui, |ui| {
                    for (v, en, tr) in [("minimize", "Minimize", "Simge durumuna küçült"), ("maximize", "Maximize", "Ekranı kapla"), ("restore", "Restore", "Geri getir"), ("close", "Close", "Kapat"), ("topmost", "Always on top", "Her zaman üstte"), ("untopmost", "Not on top", "Üstte değil")] {
                        ui.selectable_value(&mut s.text, v.to_string(), t(en, tr));
                    }
                });
            }
            "Copy selection" => opt_field(ui, t("Store in variable", "Değişkene kaydet"), &mut s.var, "selection"),
            "Write file" => {
                ui.label(t("File", "Dosya"));
                let mut p = s.path.clone().unwrap_or_default();
                path_field(ui, &mut p, "{documents}\\notes.txt", false);
                s.path = Some(p);
                ui.checkbox(&mut s.append, t("Append instead of overwrite", "Üzerine yazmak yerine sona ekle"));
                ui.label(t("Content", "İçerik"));
                ui.add(egui::TextEdit::multiline(&mut s.text).desired_rows(3).desired_width(f32::INFINITY).hint_text("{clipboard}"));
            }
            "Read file" => {
                ui.label(t("File", "Dosya"));
                let mut p = s.path.clone().unwrap_or_default();
                path_field(ui, &mut p, "", false);
                s.path = Some(p);
                opt_field(ui, t("Store in variable", "Değişkene kaydet"), &mut s.var, "file");
            }
            "Copy file" | "Move file" => {
                ui.label(t("Source file", "Kaynak dosya"));
                let mut p = s.path.clone().unwrap_or_default();
                path_field(ui, &mut p, "C:\\…\\report.docx", false);
                s.path = Some(p);
                ui.label(t("Destination (file or folder ending with \\)", "Hedef (dosya veya \\ ile biten klasör)"));
                path_field(ui, &mut s.text, "D:\\Backup\\", true);
            }
            "AI" => {
                ui.label(t("Instruction", "Talimat"));
                let mut ins = s.instruction.clone().unwrap_or_default();
                ui.add(egui::TextEdit::multiline(&mut ins).desired_rows(3).desired_width(f32::INFINITY).hint_text(t("Translate to English. Return only the translation.", "Türkçeye çevir. Yalnızca çeviriyi döndür.")));
                s.instruction = Some(ins);
                ui.label(t("Input", "Girdi"));
                ui.add(egui::TextEdit::multiline(&mut s.text).desired_rows(2).desired_width(f32::INFINITY).hint_text("{selection}"));
                ui.horizontal(|ui| {
                    ui.label(t("Result goes to", "Sonuç nereye"));
                    let mut out = s.output.clone().unwrap_or_else(|| "var".into());
                    egui::ComboBox::from_id_salt("ai-out").selected_text(out.clone()).show_ui(ui, |ui| {
                        for (v, en, tr) in [("paste", "Paste into the app", "Uygulamaya yapıştır"), ("type", "Type into the app", "Uygulamaya yaz"), ("clipboard", "Clipboard", "Pano"), ("notify", "Notification", "Bildirim"), ("file", "File", "Dosya"), ("var", "Variable only", "Yalnızca değişken")] {
                            ui.selectable_value(&mut out, v.to_string(), t(en, tr));
                        }
                    });
                    s.output = Some(out);
                });
                if s.output.as_deref() == Some("file") {
                    let mut p = s.path.clone().unwrap_or_default();
                    path_field(ui, &mut p, "", false);
                    s.path = Some(p);
                    ui.checkbox(&mut s.append, t("Append", "Sona ekle"));
                }
                opt_field(ui, t("Variable name", "Değişken adı"), &mut s.var, "ai");
            }
            "Set variable" => {
                opt_field(ui, t("Variable name", "Değişken adı"), &mut s.var, "name");
                ui.label(t("Value", "Değer"));
                ui.add(egui::TextEdit::multiline(&mut s.text).desired_rows(2).desired_width(f32::INFINITY));
            }
            "PowerShell" | "Terminal (cmd)" => {
                ui.label(t("Command", "Komut"));
                ui.add(egui::TextEdit::multiline(&mut s.text).code_editor().desired_rows(6).desired_width(f32::INFINITY));
                opt_field(ui, t("Store output in variable", "Çıktıyı değişkene kaydet"), &mut s.var, "last_output");
                muted(ui, &c, t("Runs hidden, without a profile. Do not put passwords in commands.", "Gizli ve profilsiz çalışır. Komutlara parola yazmayın."));
            }
            "System" => {
                egui::ComboBox::from_id_salt("sys-step").width(ui.available_width()).selected_text(crate::system_actions::title(&s.text)).show_ui(ui, |ui| {
                    for a in crate::system_actions::CATALOG {
                        ui.selectable_value(&mut s.text, a.id.to_string(), crate::system_actions::title(a.id));
                    }
                });
            }
            "Mouse click" | "Mouse drag" | "Scroll" => {
                ui.horizontal(|ui| {
                    ui.label("X");
                    ui.add(egui::DragValue::new(&mut s.x));
                    ui.label("Y");
                    ui.add(egui::DragValue::new(&mut s.y));
                    if s.kind == "Mouse drag" {
                        ui.label("→ X");
                        ui.add(egui::DragValue::new(&mut s.x2));
                        ui.label("Y");
                        ui.add(egui::DragValue::new(&mut s.y2));
                    }
                    if s.kind == "Scroll" {
                        ui.label(t("Amount", "Miktar"));
                        ui.add(egui::DragValue::new(&mut s.scroll_delta));
                    }
                });
                ui.checkbox(&mut s.scale_with_window, t("Scale with window size", "Pencere boyutuyla ölçekle"));
            }
            _ => {}
        }
        if matches!(s.kind.as_str(), "Text" | "Paste text" | "Notify" | "Clipboard set" | "Clipboard append" | "Write file" | "AI" | "PowerShell" | "Terminal (cmd)" | "Set variable" | "Open content" | "Copy file" | "Move file" | "Set field") {
            muted(ui, &c, t(VAR_HELP_EN, VAR_HELP_TR));
        }
    }
    if capture {
        app.start_capture(CaptureTarget::EditorStep(i));
    }
}

fn simple_form(app: &mut App, ui: &mut egui::Ui, st: &mut State) {
    let c = app.c;
    let mut capture = false;
    let ed = app.editor.as_mut().unwrap();
    let b = &mut ed.b;
    match b.kind.as_str() {
        "Application" => {
            ui.label(t("Application", "Uygulama"));
            app_combo(ui, "target-app", &mut b.target, &mut st.application_paths, false, "—", ui.available_width());
            ui.label(t("Arguments (optional)", "Argümanlar (isteğe bağlı)"));
            ui.text_edit_singleline(&mut b.arguments);
        }
        "File / folder" => {
            ui.label(t("File or folder", "Dosya veya klasör"));
            path_field(ui, &mut b.target, "", false);
            if ui.button(t("Pick a folder…", "Klasör seç…")).clicked() {
                if let Some(p) = rfd::FileDialog::new().pick_folder() {
                    b.target = p.display().to_string();
                }
            }
        }
        "Web address" | "URI / deep link" | "Spotify" => {
            ui.label(match b.kind.as_str() {
                "Spotify" => t("Spotify link or URI", "Spotify bağlantısı veya URI"),
                "Web address" => t("Web address", "Web adresi"),
                _ => t("App link (URI)", "Uygulama bağlantısı (URI)"),
            });
            ui.add(egui::TextEdit::singleline(&mut b.target).desired_width(f32::INFINITY));
        }
        "Browser website" => {
            ui.label(t("Browser", "Tarayıcı"));
            let mut browsers: Vec<String> = st.application_paths.iter().filter(|p| crate::apps::is_browser(p)).cloned().collect();
            let mut v = b.browser_path.clone().unwrap_or_default();
            if app_combo(ui, "browser", &mut v, &mut browsers, false, "—", ui.available_width()) {
                b.browser_path = Some(v.clone());
                if !st.application_paths.iter().any(|p| crate::win::same_path(p, &v)) {
                    st.application_paths.push(v);
                }
            }
            ui.label(t("Web address", "Web adresi"));
            ui.add(egui::TextEdit::singleline(&mut b.target).hint_text("https://").desired_width(f32::INFINITY));
        }
        "Send key" => {
            ui.label(t("Key to send", "Gönderilecek tuş"));
            ui.horizontal(|ui| {
                if ui.add(egui::TextEdit::singleline(&mut ed.send_text).hint_text("Ctrl+Shift+T").desired_width(220.0)).changed() {
                    if let Some((m, k)) = crate::keys::parse_combo(&ed.send_text) {
                        b.send_mods = m;
                        b.send_key = k;
                    }
                }
                if ui.button(format!("⌨ {}", t("Record", "Kaydet"))).clicked() {
                    capture = true;
                }
                ui.label(RichText::new(crate::keys::label(b.send_mods, b.send_key, 0)).color(c.accent));
            });
        }
        "Media" => {
            egui::ComboBox::from_id_salt("media").selected_text(rules::MEDIA.iter().find(|m| m.0 == b.target).map(|m| t(m.1, m.2)).unwrap_or("—")).show_ui(ui, |ui| {
                for m in rules::MEDIA {
                    ui.selectable_value(&mut b.target, m.0.to_string(), t(m.1, m.2));
                }
            });
        }
        "System" => {
            egui::ComboBox::from_id_salt("sys").width(ui.available_width()).selected_text(crate::system_actions::title(&b.target)).show_ui(ui, |ui| {
                for a in crate::system_actions::CATALOG {
                    ui.selectable_value(&mut b.target, a.id.to_string(), crate::system_actions::title(a.id));
                }
            });
            muted(ui, &c, &crate::system_actions::description(&b.target));
        }
        "PowerShell" | "Terminal (cmd)" => {
            ui.label(t("Command", "Komut"));
            ui.add(egui::TextEdit::multiline(&mut b.target).code_editor().desired_rows(8).desired_width(f32::INFINITY));
        }
        _ => {}
    }
    if capture {
        app.start_capture(CaptureTarget::EditorSendKey);
    }
}

pub fn show(app: &mut App, ctx: &egui::Context, st: &mut State) {
    let c = app.c;
    let mut open = true;
    let mut save = false;
    let mut cancel = false;
    let title = if app.editor.as_ref().unwrap().existing { t("Edit shortcut", "Kısayolu düzenle") } else { t("New shortcut", "Yeni kısayol") };
    let screen = ctx.screen_rect();
    egui::Window::new(RichText::new(title).family(super::bold()).size(18.0))
        .collapsible(false)
        .resizable(true)
        .default_size([screen.width().min(900.0) - 40.0, screen.height() - 90.0])
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .open(&mut open)
        .show(ctx, |ui| {
            egui::TopBottomPanel::bottom("editor-buttons").show_inside(ui, |ui| {
                ui.add_space(6.0);
                let ed = app.editor.as_mut().unwrap();
                if !ed.error.is_empty() {
                    ui.label(RichText::new(&ed.error).color(ui.visuals().error_fg_color));
                }
                ui.horizontal(|ui| {
                    if accent_button(ui, &c, t("Save", "Kaydet")).clicked() {
                        save = true;
                    }
                    if button(ui, t("Cancel", "İptal")).clicked() {
                        cancel = true;
                    }
                    ui.separator();
                    if button(ui, &format!("▶ {}", t("Run now", "Şimdi çalıştır"))).on_hover_text(t("Runs the shortcut once right away, in the app you used last (or its own app).", "Kısayolu hemen bir kez çalıştırır; son kullandığınız uygulamada (veya kendi uygulamasında).")).clicked() {
                        crate::actions::run_now(ed.b.clone());
                    }
                });
                ui.add_space(4.0);
            });
            egui::ScrollArea::vertical().id_salt("editor-scroll").auto_shrink([false, false]).show(ui, |ui| body(app, ui, st));
        });
    if !open || cancel {
        app.editor = None;
        crate::gesture::set_mode(crate::gesture::MODE_LISTEN);
        if app.capture.is_some() {
            app.stop_capture();
        }
        return;
    }
    if save {
        let ed = app.editor.as_mut().unwrap();
        let mut b = ed.b.clone();
        if b.kind != "Workflow" {
            b.steps = None;
        }
        let others: Vec<Binding> = st.bindings.iter().filter(|x| x.id != b.id).cloned().collect();
        match rules::validate(&b, &others) {
            Some(e) => ed.error = e,
            None => {
                if !b.app_path.is_empty() {
                    st.remember_app(&b.app_path.clone());
                }
                if b.kind == "Application" {
                    st.remember_app(&b.target.clone());
                }
                for s in b.steps() {
                    if !s.app().is_empty() {
                        let p = s.app().to_string();
                        st.remember_app(&p);
                    }
                }
                if let Some(i) = st.bindings.iter().position(|x| x.id == b.id) {
                    st.bindings[i] = b.clone();
                } else {
                    st.bindings.push(b.clone());
                }
                st.log(&b.name, &ts(format!("Shortcut saved • {} • {}", b.combo(), b.scope_name()), format!("Kısayol kaydedildi • {} • {}", b.combo(), b.scope_name())));
                app.status = ts(format!("Saved “{}”.", b.name), format!("“{}” kaydedildi.", b.name));
                app.editor = None;
                app.commit(st);
            }
        }
    }
}

/// Shape trigger: preview, record and touchpad calibration.
pub fn shape_box(ui: &mut egui::Ui, c: &crate::theme::Colors, trace: &mut Vec<crate::model::GesturePoint>, drawing: &mut bool, status: &mut String, device: &str) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(260.0, 160.0), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 10.0, c.surface);
    let pts = &*trace;
    if pts.len() > 1 {
        let (minx, maxx) = pts.iter().fold((f64::MAX, f64::MIN), |a, p| (a.0.min(p.x), a.1.max(p.x)));
        let (miny, maxy) = pts.iter().fold((f64::MAX, f64::MIN), |a, p| (a.0.min(p.y), a.1.max(p.y)));
        let size = (maxx - minx).max(maxy - miny).max(1.0);
        let scale = (rect.height() - 30.0) as f64 / size;
        let ox = rect.center().x as f64 - (minx + maxx) / 2.0 * scale;
        let oy = rect.center().y as f64 - (miny + maxy) / 2.0 * scale;
        let line: Vec<egui::Pos2> = pts.iter().map(|p| egui::pos2((ox + p.x * scale) as f32, (oy + p.y * scale) as f32)).collect();
        painter.add(egui::Shape::line(line.clone(), egui::Stroke::new(3.0, c.accent)));
        painter.circle_filled(line[0], 5.0, c.ok);
    } else {
        painter.text(rect.center(), egui::Align2::CENTER_CENTER, t("No shape yet", "Henüz şekil yok"), egui::FontId::proportional(13.0), c.muted);
    }
    ui.horizontal(|ui| {
        if *drawing {
            ui.label(RichText::new(t("Draw with one finger on the touchpad, then pause…", "Touchpad'de tek parmakla çizin, sonra bekleyin…")).color(c.accent));
            if ui.button(t("Stop", "Durdur")).clicked() {
                *drawing = false;
                crate::gesture::set_mode(crate::gesture::MODE_LISTEN);
            }
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(60));
        } else if device.is_empty() && crate::gesture::MODE.load(std::sync::atomic::Ordering::Relaxed) != crate::gesture::MODE_CALIBRATE {
            if ui.button(t("Detect touchpad", "Touchpad'i tanıt")).on_hover_text(t("Move one finger on the touchpad after clicking.", "Tıkladıktan sonra touchpad'de tek parmağınızı gezdirin.")).clicked() {
                crate::gesture::set_mode(crate::gesture::MODE_CALIBRATE);
                *status = t("Move one finger on your touchpad…", "Touchpad'de tek parmağınızı gezdirin…").into();
            }
        } else if crate::gesture::MODE.load(std::sync::atomic::Ordering::Relaxed) == crate::gesture::MODE_CALIBRATE {
            ui.label(RichText::new(t("Move one finger on your touchpad…", "Touchpad'de tek parmağınızı gezdirin…")).color(c.accent));
            if ui.button(t("Cancel", "İptal")).clicked() {
                crate::gesture::set_mode(crate::gesture::MODE_LISTEN);
                status.clear();
            }
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
        } else {
            if ui.button(if trace.is_empty() { t("Draw shape", "Şekli çiz") } else { t("Draw again", "Yeniden çiz") }).clicked() {
                *drawing = true;
                trace.clear();
                status.clear();
                crate::gesture::begin_record();
            }
            if ui.small_button(t("Detect touchpad again", "Touchpad'i yeniden tanıt")).clicked() {
                crate::gesture::set_mode(crate::gesture::MODE_CALIBRATE);
            }
        }
    });
    if !status.is_empty() {
        ui.label(RichText::new(status.as_str()).size(12.5).color(c.muted));
    }
    muted(ui, c, t("Draw the shape without clicking. When you later draw it anywhere, the shortcut runs.", "Şekli tıklamadan çizin. Daha sonra herhangi bir yerde çizdiğinizde kısayol çalışır."));
}

fn body(app: &mut App, ui: &mut egui::Ui, st: &mut State) {
    let c = app.c;
    let mut capture_trigger = false;
    {
        let ed = app.editor.as_mut().unwrap();
        egui::Grid::new("editor-head").num_columns(2).spacing([14.0, 10.0]).show(ui, |ui| {
            ui.label(t("Name", "Ad"));
            ui.add(egui::TextEdit::singleline(&mut ed.b.name).desired_width(f32::INFINITY));
            ui.end_row();
            ui.label(t("Trigger type", "Tetikleyici türü"));
            ui.horizontal(|ui| {
                if ui.radio(!ed.shape, t("Key / mouse button", "Tuş / fare düğmesi")).clicked() && ed.shape {
                    ed.shape = false;
                    ed.b.gesture = None;
                    crate::gesture::set_mode(crate::gesture::MODE_LISTEN);
                    ed.drawing = false;
                }
                if ui.radio(ed.shape, t("Touchpad shape", "Touchpad şekli")).clicked() && !ed.shape {
                    ed.shape = true;
                    ed.b.key = 0;
                    ed.b.mods = 0;
                    ed.b.prefix_key = 0;
                    ed.trigger_text.clear();
                }
            });
            ui.end_row();
            if ed.shape {
                ui.label(t("Shape", "Şekil"));
                ui.vertical(|ui| shape_box(ui, &c, &mut ed.trace, &mut ed.drawing, &mut ed.shape_status, &st.gestures.device));
                ui.end_row();
            }
            if !ed.shape {
            ui.label(t("Trigger", "Tetikleyici"));
            ui.horizontal(|ui| {
                if ui.add(egui::TextEdit::singleline(&mut ed.trigger_text).hint_text("Ctrl+Alt+F6 / Del+5 / XButton1").desired_width(220.0)).changed() {
                    if let Some((m, k, p)) = crate::keys::parse_trigger(&ed.trigger_text) {
                        ed.b.mods = m;
                        ed.b.key = k;
                        ed.b.prefix_key = p;
                    }
                }
                let recording = app.capture == Some(CaptureTarget::EditorTrigger);
                if ui.button(if recording { format!("⏺ {}", t("Press the keys… (Esc cancels)", "Tuşlara basın… (Esc iptal)")) } else { format!("⌨ {}", t("Record keys", "Tuşları kaydet")) }).clicked() && !recording {
                    capture_trigger = true;
                }
                ui.label(RichText::new(ed.b.combo()).color(c.accent).family(super::bold()));
            });
            ui.end_row();
            }
            ui.label(t("Works in", "Çalıştığı yer"));
            let mut scope = ed.b.app_path.clone();
            if app_combo(ui, "scope", &mut scope, &mut st.application_paths, true, t("All applications (global)", "Tüm uygulamalar (genel)"), 320.0) {
                ed.b.app_path = scope;
            }
            ui.end_row();
            ui.label(t("Profile", "Profil"));
            let shown = if ed.b.profile_id.is_empty() { t("All profiles", "Tüm profiller").to_string() } else { st.profiles.iter().find(|p| p.id == ed.b.profile_id).map(|p| super::profile_name(&p.name)).unwrap_or_default() };
            egui::ComboBox::from_id_salt("ed-profile").selected_text(shown).show_ui(ui, |ui| {
                ui.selectable_value(&mut ed.b.profile_id, String::new(), t("All profiles", "Tüm profiller"));
                for p in &st.profiles {
                    ui.selectable_value(&mut ed.b.profile_id, p.id.clone(), super::profile_name(&p.name));
                }
            });
            ui.end_row();
            ui.label(t("Action", "Eylem"));
            egui::ComboBox::from_id_salt("ed-kind").width(320.0).selected_text(rules::action_label(&ed.b.kind)).show_ui(ui, |ui| {
                for k in rules::ACTION_KINDS {
                    if ui.selectable_value(&mut ed.b.kind, k.0.to_string(), t(k.1, k.2)).clicked() && k.0 == "Workflow" && ed.b.steps.is_none() {
                        ed.b.steps = Some(Vec::new());
                    }
                }
            });
            ui.end_row();
        });
        ui.horizontal(|ui| {
            ui.checkbox(&mut ed.b.enabled, t("Enabled", "Etkin"));
            ui.checkbox(&mut ed.b.pass_through, t("Also pass the key to the app", "Tuşu uygulamaya da ilet"));
        });
        let issues = rules::conflicts(&ed.b, &st.bindings);
        for i in issues {
            let col = match i.severity {
                Severity::Conflict => ui.visuals().error_fg_color,
                Severity::Priority => c.warn,
                Severity::Possible => c.muted,
            };
            ui.label(RichText::new(format!("• {}", i.text)).color(col).size(13.0));
        }
        if ed.b.kind != "Workflow" {
            ui.separator();
        }
    }
    if capture_trigger {
        app.start_capture(CaptureTarget::EditorTrigger);
    }
    let kind = app.editor.as_ref().unwrap().b.kind.clone();
    if kind != "Workflow" {
        simple_form(app, ui, st);
        return;
    }
    ui.separator();
    ui.horizontal(|ui| {
        ui.label(RichText::new(t("Steps", "Adımlar")).family(super::bold()).size(16.0));
        muted(ui, &c, t("One shortcut can act on several apps: add “Bring app to front” before each app's steps.", "Tek kısayol birden çok uygulamada çalışabilir: her uygulamanın adımlarından önce “Uygulamayı öne getir” ekleyin."));
    });
    let n = app.editor.as_ref().unwrap().b.steps().len();
    let mut action: Option<(&str, usize)> = None;
    ui.columns(2, |cols| {
        let ui = &mut cols[0];
        egui::ScrollArea::vertical().id_salt("steps-list").max_height(380.0).show(ui, |ui| {
            let ed = app.editor.as_ref().unwrap();
            for (i, s) in ed.b.steps().iter().enumerate() {
                let sel = ed.selected == Some(i);
                ui.horizontal(|ui| {
                    let label = format!("{:>2}. {}", i + 1, crate::model::short(&s.summary(), 46));
                    if ui.selectable_label(sel, label).clicked() {
                        action = Some(("select", i));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button(ico('\u{E74D}')).on_hover_text(t("Delete", "Sil")).clicked() {
                            action = Some(("delete", i));
                        }
                        if i + 1 < n && ui.small_button(ico('\u{E70D}')).clicked() {
                            action = Some(("down", i));
                        }
                        if i > 0 && ui.small_button(ico('\u{E70E}')).clicked() {
                            action = Some(("up", i));
                        }
                        if ui.small_button(ico('\u{E8C8}')).on_hover_text(t("Duplicate", "Çoğalt")).clicked() {
                            action = Some(("dup", i));
                        }
                    });
                });
            }
            if n == 0 {
                muted(ui, &c, t("No steps yet. Add the first one below.", "Henüz adım yok. Aşağıdan ilkini ekleyin."));
            }
        });
        ui.separator();
        let ed = app.editor.as_mut().unwrap();
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("add-kind").width(ui.available_width() - 80.0).selected_text(rules::step_label(&ed.add_kind)).show_ui(ui, |ui| {
                let groups = [("input", t("Keyboard & mouse", "Klavye ve fare")), ("apps", t("Applications & windows", "Uygulamalar ve pencereler")), ("inapp", t("Inside the app", "Uygulama içinde")), ("clipboard", t("Clipboard", "Pano")), ("files", t("Files", "Dosyalar")), ("ai", t("AI & variables", "AI ve değişkenler")), ("system", t("System & scripts", "Sistem ve betikler"))];
                for (g, gname) in groups {
                    ui.label(RichText::new(gname).size(12.0).color(c.muted));
                    for k in rules::STEP_KINDS.iter().filter(|k| k.3 == g && !k.0.starts_with("Mouse") && k.0 != "Scroll") {
                        ui.selectable_value(&mut ed.add_kind, k.0.to_string(), t(k.1, k.2));
                    }
                }
            });
            if ui.button(format!("+ {}", t("Add", "Ekle"))).clicked() {
                action = Some(("add", 0));
            }
        });
        let ui = &mut cols[1];
        if let Some(i) = app.editor.as_ref().unwrap().selected.filter(|i| *i < n) {
            egui::Frame::none().fill(c.surface).rounding(10.0).inner_margin(12.0).show(ui, |ui| step_form(app, ui, st, i));
        } else {
            muted(ui, &c, t("Select a step to edit it.", "Düzenlemek için bir adım seçin."));
        }
    });
    if let Some((what, i)) = action {
        let ed = app.editor.as_mut().unwrap();
        let steps = ed.b.steps.get_or_insert_with(Vec::new);
        match what {
            "select" => ed.selected = Some(i),
            "delete" => {
                steps.remove(i);
                ed.selected = if steps.is_empty() { None } else { Some(i.min(steps.len() - 1)) };
            }
            "up" => {
                steps.swap(i, i - 1);
                ed.selected = Some(i - 1);
            }
            "down" => {
                steps.swap(i, i + 1);
                ed.selected = Some(i + 1);
            }
            "dup" => {
                let s = steps[i].clone();
                steps.insert(i + 1, s);
                ed.selected = Some(i + 1);
            }
            "add" => {
                let at = ed.selected.map(|s| s + 1).unwrap_or(steps.len()).min(steps.len());
                let mut s = new_step(&ed.add_kind);
                if matches!(ed.add_kind.as_str(), "Focus" | "Menu command") && !ed.b.app_path.is_empty() {
                    s.app_path = Some(ed.b.app_path.clone());
                }
                steps.insert(at, s);
                ed.selected = Some(at);
            }
            _ => {}
        }
        if let Some(sel) = ed.selected {
            ed.step_combo = ed.b.steps().get(sel).filter(|s| s.kind == "Key" && s.key != 0).map(|s| crate::keys::trigger_text(s.mods, s.key, 0)).unwrap_or_default();
        }
    }
}
