//! Create shortcuts by describing them to the language model.
use super::{accent_button, app_combo, button, card, header, muted, section, App, CaptureTarget, Page};
use crate::i18n::{t, ts};
use crate::llm::{PlanRequest, PlanResult};
use crate::model::State;
use eframe::egui::{self, RichText};

#[derive(Default)]
pub struct PromptState {
    pub text: String,
    pub trigger: String,
    pub scope: String,
    pub target: String,
    pub busy: bool,
    pub progress: String,
    pub result: Option<PlanResult>,
    pub error: String,
    pub started: Option<std::time::Instant>,
    pub shape: bool,
    pub trace: Vec<crate::model::GesturePoint>,
    pub drawing: bool,
    pub shape_status: String,
}

impl PromptState {
    pub fn on_result(&mut self, r: Result<PlanResult, String>) {
        self.busy = false;
        self.progress.clear();
        match r {
            Ok(p) => {
                self.error.clear();
                self.result = Some(p);
            }
            Err(e) => {
                self.result = None;
                self.error = e;
            }
        }
    }
}

const EXAMPLES: &[(&str, &str)] = &[
    ("When I press Ctrl+Alt+T, translate the selected text to English and paste it in place.", "Ctrl+Alt+T'ye basınca seçili metni İngilizceye çevir ve yerine yapıştır."),
    ("When I press F9, save this file and copy it to Documents\\Backup.", "F9'a basınca bu uygulamadaki dosyayı kaydet ve Belgeler\\Yedek klasörüne kopyala."),
    ("When I press Ctrl+Shift+1, copy the selection, switch to Notepad and paste it on a new line.", "Ctrl+Shift+1 ile seçili metni kopyala, Not Defteri'ne geç ve yeni satıra yapıştır."),
    ("When I press Ctrl+Alt+N, append the clipboard to Documents\\notes.txt with the date.", "Ctrl+Alt+N ile panodaki metni tarihle birlikte Belgeler\\notlar.txt dosyasına ekle."),
    ("Open https://www.youtube.com/ in Brave when I press Del+5.", "Del+5'e basınca YouTube'u Brave'de aç."),
    ("When I press F8, summarise the selected text and show it as a notification.", "F8'e basınca seçili metni özetle ve bildirim olarak göster."),
];

pub fn show(app: &mut App, ui: &mut egui::Ui, st: &mut State) {
    let c = app.c;
    header(ui, &c, t("Create with AI", "Yapay zekâ ile oluştur"), t("Describe what should happen in plain words. One shortcut can work across several apps, use the clipboard, files and AI. You review the plan before saving.", "Ne olacağını sade bir dille anlatın. Tek kısayol birden çok uygulamada çalışabilir; pano, dosyalar ve yapay zekâ kullanılabilir. Kaydetmeden önce planı incelersiniz."));
    let configured = if st.model_options.remote { !st.model_options.model.trim().is_empty() } else { !crate::storage::resolve_model(&st.model_path).is_empty() };
    if !configured {
        egui::Frame::none().fill(c.selection).rounding(10.0).inner_margin(12.0).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(t("No language model is set up yet.", "Henüz bir dil modeli ayarlanmadı."));
                if ui.link(t("Set up the AI model", "Yapay zekâ modelini ayarla")).clicked() {
                    app.page = Page::Model;
                }
            });
        });
        ui.add_space(8.0);
    }
    egui::ScrollArea::vertical().id_salt("prompt").auto_shrink([false, false]).show(ui, |ui| {
        card(ui, &c, |ui| {
            ui.set_width(ui.available_width());
            ui.add(egui::TextEdit::multiline(&mut app.prompt.text).desired_rows(5).desired_width(f32::INFINITY).hint_text(t(EXAMPLES[0].0, EXAMPLES[0].1)));
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(t("Trigger type", "Tetikleyici türü"));
                if ui.radio(!app.prompt.shape, t("Key / mouse button", "Tuş / fare düğmesi")).clicked() {
                    app.prompt.shape = false;
                    app.prompt.drawing = false;
                    crate::gesture::set_mode(crate::gesture::MODE_LISTEN);
                }
                if ui.radio(app.prompt.shape, t("Touchpad shape", "Touchpad şekli")).clicked() {
                    app.prompt.shape = true;
                }
            });
            if app.prompt.shape {
                let p = &mut app.prompt;
                super::editor::shape_box(ui, &c, &mut p.trace, &mut p.drawing, &mut p.shape_status, &st.gestures.device);
                ui.add_space(4.0);
            }
            egui::Grid::new("prompt-opts").num_columns(4).spacing([12.0, 8.0]).show(ui, |ui| {
                ui.label(if app.prompt.shape { "" } else { t("Trigger", "Tetikleyici") });
                ui.add_enabled_ui(!app.prompt.shape, |ui| ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut app.prompt.trigger).hint_text(t("from the text, or e.g. Ctrl+Alt+T", "metinden veya örn. Ctrl+Alt+T")).desired_width(200.0));
                    let rec = app.capture == Some(CaptureTarget::PromptTrigger);
                    if ui.button(if rec { t("Press keys…", "Tuşlara basın…") } else { "⌨" }).on_hover_text(t("Record keys", "Tuşları kaydet")).clicked() && !rec {
                        app.start_capture(CaptureTarget::PromptTrigger);
                    }
                }));
                ui.label(t("Works in", "Çalıştığı yer"));
                app_combo(ui, "p-scope", &mut app.prompt.scope, &mut st.application_paths, true, t("All applications", "Tüm uygulamalar"), 230.0);
                ui.end_row();
                ui.label(t("Main target app", "Ana hedef uygulama"));
                app_combo(ui, "p-target", &mut app.prompt.target, &mut st.application_paths, true, t("Automatic", "Otomatik"), 230.0);
                ui.label("");
                ui.horizontal(|ui| {
                    if ui.checkbox(&mut st.ai.share_app_commands, t("Use scanned in-app commands", "Taranan uygulama içi komutları kullan")).changed() {
                        crate::request_save();
                    }
                    if ui.checkbox(&mut st.ai.share_clipboard, t("Show clipboard to the model", "Panoyu modele göster")).changed() {
                        crate::request_save();
                    }
                });
                ui.end_row();
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let shape_ok = !app.prompt.shape || app.prompt.trace.len() >= 8;
                let can = !app.prompt.busy && !app.prompt.text.trim().is_empty() && shape_ok;
                if app.prompt.shape && !shape_ok {
                    ui.label(RichText::new(t("Draw the touchpad shape first.", "Önce touchpad şeklini çizin.")).color(c.warn));
                }
                if ui.add_enabled(can, egui::Button::new(RichText::new(t("Create plan", "Planı oluştur")).color(c.accent_ink).family(super::bold())).fill(c.accent).min_size(egui::vec2(0.0, 32.0))).clicked() {
                    start(app, st);
                }
                if app.prompt.busy {
                    ui.spinner();
                    let secs = app.prompt.started.map(|s| s.elapsed().as_secs()).unwrap_or(0);
                    ui.label(RichText::new(format!("{}  ({secs} s)", app.prompt.progress)).color(c.muted));
                    ui.ctx().request_repaint_after(std::time::Duration::from_millis(500));
                }
            });
        });
        ui.add_space(8.0);
        if !app.prompt.error.is_empty() {
            ui.label(RichText::new(&app.prompt.error).color(ui.visuals().error_fg_color));
        }
        if let Some(r) = app.prompt.result.clone() {
            card(ui, &c, |ui| {
                ui.set_width(ui.available_width());
                for e in &r.errors {
                    ui.label(RichText::new(format!("• {e}")).color(c.warn));
                }
                if let Some(b) = &r.binding {
                    section(ui, &c, &format!("{} — {}", b.name, b.combo()));
                    muted(ui, &c, &ts(format!("Works in: {}", b.scope_name()), format!("Çalıştığı yer: {}", b.scope_name())));
                    for (i, s) in b.steps().iter().enumerate() {
                        ui.label(format!("{:>2}. {}", i + 1, s.summary()));
                    }
                    for n in &r.notes {
                        ui.label(RichText::new(format!("ⓘ {n}")).size(12.5).color(c.muted));
                    }
                    ui.horizontal(|ui| {
                        if accent_button(ui, &c, t("Review and save…", "İncele ve kaydet…")).clicked() {
                            app.open_editor(b.clone(), false);
                        }
                        if r.errors.is_empty() && button(ui, t("Save directly", "Doğrudan kaydet")).clicked() {
                            let mut nb = b.clone();
                            if nb.app_path.is_empty() && !app.prompt.scope.is_empty() {
                                nb.app_path = app.prompt.scope.clone();
                            }
                            match crate::rules::validate(&nb, &st.bindings) {
                                Some(e) => app.prompt.error = e,
                                None => {
                                    for s in nb.steps() {
                                        let p = s.app().to_string();
                                        st.remember_app(&p);
                                    }
                                    st.log(&nb.name, &ts(format!("Saved from AI plan • {}", nb.combo()), format!("Yapay zekâ planından kaydedildi • {}", nb.combo())));
                                    st.bindings.push(nb);
                                    app.prompt.result = None;
                                    app.prompt.text.clear();
                                    app.status = t("Shortcut saved.", "Kısayol kaydedildi.").into();
                                    app.commit(st);
                                }
                            }
                        }
                    });
                } else {
                    for n in &r.notes {
                        ui.label(RichText::new(format!("ⓘ {n}")).size(12.5).color(c.muted));
                    }
                }
            });
        }
        ui.add_space(10.0);
        section(ui, &c, t("Examples", "Örnekler"));
        for (en, tr) in EXAMPLES {
            if ui.link(t(en, tr)).clicked() {
                app.prompt.text = t(en, tr).to_string();
            }
        }
    });
}

fn start(app: &mut App, st: &State) {
    app.prompt.busy = true;
    app.prompt.error.clear();
    app.prompt.result = None;
    app.prompt.progress = t("Preparing…", "Hazırlanıyor…").into();
    app.prompt.started = Some(std::time::Instant::now());
    let req = PlanRequest {
        prompt: app.prompt.text.clone(),
        trigger: app.prompt.trigger.clone(),
        scope_app: app.prompt.scope.clone(),
        target_app: app.prompt.target.clone(),
        apps: st.application_paths.clone(),
        inventories: st.app_inventories.clone(),
        share_commands: st.ai.share_app_commands,
        clipboard: if st.ai.share_clipboard { crate::clipboard::get_text() } else { None },
        rounds: st.ai.correction_rounds,
        gesture: if app.prompt.shape && app.prompt.trace.len() >= 8 { Some(crate::model::ShapeTemplate { points: app.prompt.trace.clone() }) } else { None },
    };
    std::thread::spawn(move || {
        let r = crate::llm::plan(&req, &|p| crate::send_ui(crate::UiEvent::Progress(p)));
        crate::send_ui(crate::UiEvent::Plan(r));
    });
}
