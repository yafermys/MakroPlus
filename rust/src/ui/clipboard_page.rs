//! Clipboard page: history, editing, pinning and AI transformations.
use super::{accent_button, button, header, ico, muted, App};
use crate::i18n::{t, ts};
use crate::model::State;
use eframe::egui::{self, RichText};

#[derive(Default)]
pub struct ClipState {
    pub search: String,
    pub selected: Option<String>,
    pub edit: String,
    pub edit_for: Option<String>,
    pub instruction: String,
    pub ai_busy: bool,
    pub ai_result: Option<Result<String, String>>,
    pub only_pinned: bool,
}

impl ClipState {
    pub fn on_ai(&mut self, r: Result<String, String>) {
        self.ai_busy = false;
        self.ai_result = Some(r);
    }
}

const QUICK_AI: &[(&str, &str, &str, &str)] = &[
    ("Fix", "Düzelt", "Fix spelling and grammar. Keep the meaning and language.", "Yazım ve dil bilgisini düzelt. Anlamı ve dili koru."),
    ("To English", "İngilizceye", "Translate to English.", "İngilizceye çevir."),
    ("To Turkish", "Türkçeye", "Translate to Turkish.", "Türkçeye çevir."),
    ("Summarise", "Özetle", "Summarise in a few bullet points.", "Birkaç madde ile özetle."),
    ("Formal", "Resmî yap", "Rewrite in a polite, formal tone.", "Kibar ve resmî bir dille yeniden yaz."),
    ("Clean up", "Temizle", "Remove extra spaces, line breaks and formatting noise.", "Fazla boşlukları, satır sonlarını ve biçim kalıntılarını temizle."),
];

pub fn show(app: &mut App, ui: &mut egui::Ui, st: &mut State) {
    let c = app.c;
    header(ui, &c, t("Clipboard", "Pano"), t("Everything you copy appears here. Select an item to view, edit, pin, copy back or transform it with AI. Content that password managers mark as private is never stored.", "Kopyaladığınız her şey burada görünür. Bir öğeyi seçip görüntüleyin, düzenleyin, sabitleyin, panoya geri koyun veya yapay zekâ ile dönüştürün. Parola yöneticilerinin gizli olarak işaretlediği içerik asla saklanmaz."));
    let mut changed_opts = false;
    ui.horizontal_wrapped(|ui| {
        changed_opts |= ui.checkbox(&mut st.clipboard.enabled, t("Record clipboard", "Panoyu kaydet")).changed();
        changed_opts |= ui.checkbox(&mut st.clipboard.persist, t("Keep after restart", "Yeniden başlatınca koru")).changed();
        changed_opts |= ui.checkbox(&mut st.clipboard.capture_images, t("Include images (this session)", "Resimleri dahil et (bu oturum)")).changed();
        ui.label(t("Keep", "Sakla"));
        changed_opts |= ui.add(egui::DragValue::new(&mut st.clipboard.max_items).range(20..=5000)).changed();
        ui.label(t("items", "öğe"));
        if ui.button(t("Clear unpinned", "Sabitlenmemişleri temizle")).clicked() {
            crate::shell::clip_store().lock().entries.retain(|e| e.pinned);
            crate::shell::save_clipboard_soon();
            app.clip.selected = None;
        }
    });
    if changed_opts {
        crate::shell::clip_store().lock().trim(st.clipboard.max_items);
        crate::shell::save_clipboard_soon();
        app.commit(st);
    }
    ui.add_space(6.0);
    let mut store = crate::shell::clip_store().lock();
    if app.clip.selected.is_none() {
        app.clip.selected = store.entries.iter().find(|e| e.pinned).or(store.entries.first()).map(|e| e.id.clone());
    }
    let q = app.clip.search.to_lowercase();
    ui.columns(2, |cols| {
        let ui = &mut cols[0];
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut app.clip.search).hint_text(t("Search…", "Ara…")).desired_width(ui.available_width() - 110.0));
            ui.checkbox(&mut app.clip.only_pinned, t("Pinned", "Sabit"));
        });
        egui::ScrollArea::vertical().id_salt("clip-list").auto_shrink([false, false]).show(ui, |ui| {
            let mut ids: Vec<usize> = (0..store.entries.len()).filter(|&i| {
                let e = &store.entries[i];
                (!app.clip.only_pinned || e.pinned) && (q.is_empty() || e.text.to_lowercase().contains(&q) || e.files.iter().any(|f| f.to_lowercase().contains(&q)) || e.source.to_lowercase().contains(&q))
            }).collect();
            ids.sort_by_key(|&i| !store.entries[i].pinned);
            for i in ids {
                let (id, title, meta, pinned, sel) = {
                    let e = &store.entries[i];
                    (e.id.clone(), e.title(), format!("{} • {}", e.time.get(5..16).unwrap_or(&e.time), if e.source.is_empty() { "—" } else { &e.source }), e.pinned, app.clip.selected.as_deref() == Some(&e.id))
                };
                let fill = if sel { c.selection } else { c.panel };
                let resp = egui::Frame::none().fill(fill).rounding(8.0).inner_margin(10.0).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        if pinned {
                            ui.label(ico('\u{E718}').size(12.0).color(c.accent));
                        }
                        ui.label(RichText::new(crate::model::short(&title, 90)).size(13.5));
                    });
                    ui.label(RichText::new(meta).size(11.5).color(c.muted));
                    if store.entries[i].kind == "image" {
                        let tex = {
                            let e = &mut store.entries[i];
                            if e.texture.is_none() {
                                if let Some(img) = &e.image {
                                    e.texture = Some(ui.ctx().load_texture(format!("clip-{}", e.id), egui::ColorImage::from_rgba_unmultiplied([img.width, img.height], &img.rgba), egui::TextureOptions::LINEAR));
                                }
                            }
                            e.texture.clone()
                        };
                        if let Some(tex) = tex {
                            let size = tex.size_vec2();
                            let scale = (120.0 / size.y).min(1.0);
                            ui.add(egui::Image::new(egui::load::SizedTexture::new(tex.id(), size * scale)));
                        }
                    }
                });
                if resp.response.interact(egui::Sense::click()).clicked() {
                    app.clip.selected = Some(id);
                }
                ui.add_space(4.0);
            }
            if store.entries.is_empty() {
                muted(ui, &c, t("Copy something (Ctrl+C) in any app — it will show up here.", "Herhangi bir uygulamada bir şey kopyalayın (Ctrl+C) — burada görünecek."));
            }
        });
        let ui = &mut cols[1];
        let Some(sel) = app.clip.selected.clone() else {
            muted(ui, &c, t("Select an item on the left.", "Soldan bir öğe seçin."));
            return;
        };
        let Some(idx) = store.entries.iter().position(|e| e.id == sel) else {
            app.clip.selected = None;
            return;
        };
        if app.clip.edit_for.as_deref() != Some(&sel) {
            app.clip.edit_for = Some(sel.clone());
            app.clip.edit = if store.entries[idx].kind == "files" { store.entries[idx].files.join("\n") } else { store.entries[idx].text.clone() };
            app.clip.ai_result = None;
        }
        let kind = store.entries[idx].kind.clone();
        egui::Frame::none().fill(c.panel).rounding(12.0).inner_margin(14.0).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                if accent_button(ui, &c, t("Copy to clipboard", "Panoya kopyala")).clicked() {
                    let e = &store.entries[idx];
                    let r = if e.kind == "text" { crate::clipboard::set_text(&app.clip.edit) } else { e.restore() };
                    app.status = match r {
                        Ok(_) => t("Copied to the clipboard.", "Panoya kopyalandı.").into(),
                        Err(e) => e,
                    };
                }
                let pinned = store.entries[idx].pinned;
                if button(ui, if pinned { t("Unpin", "Sabitlemeyi kaldır") } else { t("Pin", "Sabitle") }).clicked() {
                    store.entries[idx].pinned = !pinned;
                    crate::shell::save_clipboard_soon();
                }
                if store.entries[idx].kind != "image" && button(ui, t("Assign to a key", "Kısayola ata")).on_hover_text(t("Creates a shortcut that pastes this item wherever you are.", "Bu öğeyi bulunduğunuz yere yapıştıran bir kısayol oluşturur.")).clicked() {
                    let e = &store.entries[idx];
                    let mut step = crate::model::ActionStep::new(if e.kind == "files" { "Paste files" } else { "Paste text" });
                    step.text = if e.kind == "files" { e.files.join("\n") } else { app.clip.edit.clone() };
                    let name = format!("{}: {}", t("Paste", "Yapıştır"), crate::model::short(&e.title(), 30));
                    app.editor = Some(super::editor::Editor::new(crate::model::Binding { name, steps: Some(vec![step]), ..Default::default() }, false));
                }
                if button(ui, t("Delete", "Sil")).clicked() {
                    store.entries.remove(idx);
                    app.clip.selected = None;
                    crate::shell::save_clipboard_soon();
                }
            });
            if app.clip.selected.is_none() {
                return;
            }
            ui.add_space(6.0);
            if kind == "image" {
                muted(ui, &c, t("Images stay in memory for this session only.", "Resimler yalnızca bu oturum boyunca bellekte kalır."));
                return;
            }
            let rows = ((ui.available_height() - 260.0) / 18.0).clamp(6.0, 30.0) as usize;
            ui.add(egui::TextEdit::multiline(&mut app.clip.edit).desired_rows(rows).desired_width(f32::INFINITY).font(egui::TextStyle::Monospace));
            ui.horizontal(|ui| {
                let dirty = kind == "text" && app.clip.edit != store.entries[idx].text;
                if ui.add_enabled(dirty, egui::Button::new(t("Save changes", "Değişiklikleri kaydet"))).clicked() {
                    store.entries[idx].text = app.clip.edit.clone();
                    crate::shell::save_clipboard_soon();
                    app.status = t("Item updated.", "Öğe güncellendi.").into();
                }
                ui.label(RichText::new(ts(format!("{} characters", app.clip.edit.chars().count()), format!("{} karakter", app.clip.edit.chars().count()))).size(12.0).color(c.muted));
            });
            ui.separator();
            ui.label(RichText::new(t("Transform with AI", "Yapay zekâ ile dönüştür")).family(super::bold()));
            ui.horizontal_wrapped(|ui| {
                for (en, tr, ien, itr) in QUICK_AI {
                    if ui.small_button(t(en, tr)).clicked() {
                        app.clip.instruction = t(ien, itr).to_string();
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut app.clip.instruction).hint_text(t("Instruction, e.g. make it a bulleted list", "Talimat, örn. madde işaretli liste yap")).desired_width(ui.available_width() - 90.0));
                if ui.add_enabled(!app.clip.ai_busy && !app.clip.instruction.trim().is_empty(), egui::Button::new(t("Apply", "Uygula"))).clicked() {
                    app.clip.ai_busy = true;
                    app.clip.ai_result = None;
                    let ins = app.clip.instruction.clone();
                    let input = app.clip.edit.clone();
                    std::thread::spawn(move || {
                        let r = crate::llm::complete_text(&ins, &input);
                        crate::send_ui(crate::UiEvent::AiText("clip".into(), r));
                    });
                }
                if app.clip.ai_busy {
                    ui.spinner();
                }
            });
            match app.clip.ai_result.clone() {
                Some(Ok(text)) => {
                    egui::Frame::none().fill(c.surface).rounding(8.0).inner_margin(10.0).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        egui::ScrollArea::vertical().id_salt("ai-out").max_height(160.0).show(ui, |ui| {
                            ui.label(&text);
                        });
                    });
                    ui.horizontal(|ui| {
                        if ui.button(t("Use in editor", "Düzenleyiciye al")).clicked() {
                            app.clip.edit = text.clone();
                        }
                        if ui.button(t("Copy result", "Sonucu kopyala")).clicked() {
                            let _ = crate::clipboard::set_text(&text);
                            app.status = t("Result copied to the clipboard.", "Sonuç panoya kopyalandı.").into();
                        }
                    });
                }
                Some(Err(e)) => {
                    ui.label(RichText::new(e).color(ui.visuals().error_fg_color));
                }
                None => {}
            }
        });
    });
}
