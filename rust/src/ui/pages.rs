//! Home, shortcut list, recorder, Windows actions, conflicts and history.
use super::{accent_button, button, card, header, icon_button, ico, muted, section, App, Page};
use crate::i18n::{t, ts};
use crate::model::{ActionStep, Binding, State};
use eframe::egui::{self, RichText};

#[derive(Default)]
pub struct ShortcutsState {
    pub search: String,
    pub filter: usize,
    pub confirm_delete: Option<String>,
}

#[derive(Default)]
pub struct RecorderState {
    pub recording: bool,
    pub steps: Vec<ActionStep>,
    pub keep_waits: bool,
    pub initialized: bool,
}

fn stat(ui: &mut egui::Ui, app: &App, value: String, label: &str, icon: char) {
    let c = app.c;
    egui::Frame::none().fill(c.panel).rounding(12.0).inner_margin(16.0).stroke(egui::Stroke::new(1.0, c.line.gamma_multiply(0.6))).show(ui, |ui| {
        ui.set_min_width(170.0);
        ui.horizontal(|ui| {
            ui.label(ico(icon).size(22.0).color(c.accent));
            ui.vertical(|ui| {
                ui.label(RichText::new(value).family(super::bold()).size(24.0));
                ui.label(RichText::new(label).size(12.0).color(c.muted));
            });
        });
    });
}

pub fn home(app: &mut App, ui: &mut egui::Ui, st: &mut State) {
    let c = app.c;
    egui::ScrollArea::vertical().id_salt("home").show(ui, |ui| {
        header(ui, &c, t("Your next action, one shortcut away.", "Bir sonraki işiniz, tek kısayol uzağınızda."), t("Keyboard, mouse, apps, clipboard and AI — in one place.", "Klavye, fare, uygulamalar, pano ve yapay zekâ — tek yerde."));
        let active = st.effective().len();
        let clip = crate::shell::clip_store().lock().entries.len();
        ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP).with_main_wrap(true), |ui| {
            stat(ui, app, active.to_string(), t("active shortcuts", "etkin kısayol"), '\u{E765}');
            stat(ui, app, st.application_paths.len().to_string(), t("registered apps", "kayıtlı uygulama"), '\u{E71D}');
            stat(ui, app, st.app_inventories.len().to_string(), t("apps scanned in detail", "ayrıntılı taranan uygulama"), '\u{E721}');
            stat(ui, app, clip.to_string(), t("clipboard items", "pano öğesi"), '\u{E77F}');
        });
        ui.add_space(14.0);
        card(ui, &c, |ui| {
            ui.set_width(ui.available_width());
            section(ui, &c, t("Start quickly", "Hızlı başla"));
            ui.horizontal_wrapped(|ui| {
                if accent_button(ui, &c, &format!("+ {}", t("New shortcut", "Yeni kısayol"))).clicked() {
                    app.open_editor(Binding { name: t("New shortcut", "Yeni kısayol").into(), ..Default::default() }, false);
                }
                if button(ui, t("Create with AI", "Yapay zekâ ile oluştur")).clicked() {
                    app.page = Page::Prompt;
                }
                if button(ui, t("Record a macro", "Makro kaydet")).clicked() {
                    app.page = Page::Recorder;
                }
                if button(ui, t("Add applications", "Uygulama ekle")).clicked() {
                    app.page = Page::Apps;
                    app.apps.open_add();
                }
                if button(ui, t("Open clipboard", "Panoyu aç")).clicked() {
                    app.page = Page::Clipboard;
                }
            });
        });
        ui.add_space(12.0);
        ui.columns(2, |cols| {
            card(&mut cols[0], &c, |ui| {
                ui.set_width(ui.available_width());
                section(ui, &c, t("What's new in 3.0", "3.0 ile gelenler"));
                for (en, tr) in [
                    ("One shortcut, several apps: copy in one app, paste or save into another.", "Tek kısayol, birden çok uygulama: birinde kopyala, diğerine yapıştır veya kaydet."),
                    ("Inside apps: scan menus, buttons and fields, then run them directly.", "Uygulama içi: menüleri, düğmeleri ve alanları tarayıp doğrudan çalıştırın."),
                    ("Clipboard page: view, edit, pin and transform copied items with AI.", "Pano sayfası: kopyalananları görün, düzenleyin, sabitleyin, AI ile dönüştürün."),
                    ("AI steps run when the shortcut runs: translate, summarise, rewrite.", "AI adımları kısayol çalışırken işler: çeviri, özet, yeniden yazma."),
                    ("Optional install: appears in Start and Installed apps; the exe still runs portable.", "İsteğe bağlı kurulum: Başlat'ta ve Yüklü uygulamalarda görünür; exe taşınabilir çalışmaya devam eder."),
                ] {
                    ui.label(format!("•  {}", t(en, tr)));
                }
                if !crate::installer::is_installed() && ui.link(t("Install Makro+ on this computer…", "Makro+'ı bu bilgisayara yükle…")).clicked() {
                    app.page = Page::Settings;
                }
            });
            card(&mut cols[1], &c, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    section(ui, &c, t("Recent activity", "Son etkinlik"));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.link(t("All", "Tümü")).clicked() {
                            app.page = Page::History;
                        }
                    });
                });
                if st.history.is_empty() {
                    muted(ui, &c, t("Results appear here when a shortcut runs.", "Bir kısayol çalıştığında sonuçlar burada görünür."));
                }
                for h in st.history.iter().take(7) {
                    ui.label(RichText::new(format!("{}  •  {}", h.time.get(11..16).unwrap_or(&h.time), h.name)).size(13.0));
                    ui.label(RichText::new(crate::model::short(&h.result, 90)).size(12.0).color(c.muted));
                }
            });
        });
    });
}

fn is_mouse(b: &Binding) -> bool {
    crate::keys::is_pointer(b.key) || crate::keys::is_pointer(b.send_key) || b.steps().iter().any(|s| s.kind.starts_with("Mouse") || s.kind == "Scroll" || crate::keys::is_pointer(s.key))
}

pub fn shortcuts(app: &mut App, ui: &mut egui::Ui, st: &mut State) {
    let c = app.c;
    header(ui, &c, t("Shortcuts", "Kısayollar"), t("Your assignments. Double-check conflicts before saving; app-specific shortcuts win inside their app.", "Atamalarınız. Kaydetmeden önce çakışmalara bakın; uygulamaya özel kısayollar kendi uygulamasında önceliklidir."));
    ui.horizontal(|ui| {
        if accent_button(ui, &c, &format!("+ {}", t("New shortcut", "Yeni kısayol"))).clicked() {
            app.open_editor(Binding { name: t("New shortcut", "Yeni kısayol").into(), ..Default::default() }, false);
        }
        if button(ui, t("Create with AI", "Yapay zekâ ile oluştur")).clicked() {
            app.page = Page::Prompt;
        }
        if button(ui, t("Record macro", "Makro kaydet")).clicked() {
            app.page = Page::Recorder;
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            egui::ComboBox::from_id_salt("sc-filter")
                .selected_text([t("All", "Tümü"), t("Keyboard", "Klavye"), t("Mouse", "Fare"), t("By application", "Uygulamaya göre"), t("Disabled", "Devre dışı")][app.shortcuts.filter.min(4)])
                .show_ui(ui, |ui| {
                    for (i, l) in [t("All", "Tümü"), t("Keyboard", "Klavye"), t("Mouse", "Fare"), t("By application", "Uygulamaya göre"), t("Disabled", "Devre dışı")].iter().enumerate() {
                        ui.selectable_value(&mut app.shortcuts.filter, i, *l);
                    }
                });
            ui.add(egui::TextEdit::singleline(&mut app.shortcuts.search).hint_text(t("Search name, key, app…", "Ad, tuş, uygulama ara…")).desired_width(240.0));
        });
    });
    ui.add_space(8.0);
    let q = app.shortcuts.search.to_lowercase();
    let profile = st.active_profile().cloned();
    let mut rows: Vec<usize> = (0..st.bindings.len())
        .filter(|&i| {
            let b = &st.bindings[i];
            let hay = format!("{} {} {} {} {}", b.name, b.combo(), b.summary(), b.scope_name(), b.kind).to_lowercase();
            (q.is_empty() || hay.contains(&q))
                && match app.shortcuts.filter {
                    1 => !is_mouse(b),
                    2 => is_mouse(b),
                    4 => !b.enabled,
                    _ => true,
                }
        })
        .collect();
    if app.shortcuts.filter == 3 {
        rows.sort_by_key(|&i| (st.bindings[i].scope_name().to_lowercase(), st.bindings[i].name.to_lowercase()));
    }
    let mut act: Option<(&str, usize)> = None;
    let total = ui.available_width() - 20.0 - 190.0 - 5.0 * 14.0 - 60.0;
    let col_name = (total * 0.24).max(110.0);
    let col_key = (total * 0.16).max(90.0);
    let col_scope = (total * 0.14).max(80.0);
    let col_action = (total - col_name - col_key - col_scope).max(120.0);
    egui::ScrollArea::vertical().id_salt("sc-list").auto_shrink([false, false]).show(ui, |ui| {
        egui::Grid::new("sc-grid").num_columns(6).striped(true).spacing([14.0, 10.0]).min_col_width(40.0).show(ui, |ui| {
            for h in ["", t("Name", "Ad"), t("Trigger", "Tetikleyici"), t("Action", "Eylem"), t("Works in", "Çalıştığı yer"), ""] {
                ui.label(RichText::new(h).size(12.5).color(c.muted).family(super::bold()));
            }
            ui.end_row();
            for &i in &rows {
                let b = &st.bindings[i];
                let live = b.enabled && !st.paused && State::profile_active(b, profile.as_ref());
                ui.label(RichText::new(if live { "●" } else { "○" }).color(if live { c.ok } else { c.muted })).on_hover_text(if live { t("Active", "Etkin") } else if !b.enabled { t("Disabled", "Devre dışı") } else { t("Not in the active profile or paused", "Etkin profilde değil veya duraklatıldı") });
                let name_resp = super::cell(ui, col_name, egui::Label::new(RichText::new(&b.name).family(super::bold())).truncate().sense(egui::Sense::click()));
                if name_resp.double_clicked() {
                    act = Some(("edit", i));
                }
                super::cell(ui, col_key, egui::Label::new(RichText::new(b.combo()).color(c.accent)).truncate());
                super::cell(ui, col_action, egui::Label::new(RichText::new(format!("{} • {}", crate::rules::action_label(&b.kind), b.summary())).size(13.0)).truncate()).on_hover_text(b.summary());
                let prof = if b.profile_id.is_empty() { String::new() } else { format!(" • {}", st.profiles.iter().find(|p| p.id == b.profile_id).map(|p| super::profile_name(&p.name)).unwrap_or_default()) };
                super::cell(ui, col_scope, egui::Label::new(RichText::new(format!("{}{}", b.scope_name(), prof)).size(13.0).color(c.muted)).truncate());
                ui.horizontal(|ui| {
                    if icon_button(ui, &c, '\u{E70F}', t("Edit", "Düzenle")).clicked() {
                        act = Some(("edit", i));
                    }
                    if icon_button(ui, &c, if b.enabled { '\u{E769}' } else { '\u{E8FB}' }, if b.enabled { t("Disable", "Devre dışı bırak") } else { t("Enable", "Etkinleştir") }).clicked() {
                        act = Some(("toggle", i));
                    }
                    if icon_button(ui, &c, '\u{E8C8}', t("Duplicate", "Çoğalt")).clicked() {
                        act = Some(("dup", i));
                    }
                    if icon_button(ui, &c, '\u{E768}', t("Run now", "Şimdi çalıştır")).clicked() {
                        act = Some(("run", i));
                    }
                    if app.shortcuts.confirm_delete.as_deref() == Some(&b.id) {
                        if ui.button(RichText::new(t("Delete?", "Silinsin mi?")).color(ui.visuals().error_fg_color)).clicked() {
                            act = Some(("delete", i));
                        }
                    } else if icon_button(ui, &c, '\u{E74D}', t("Delete", "Sil")).clicked() {
                        app.shortcuts.confirm_delete = Some(b.id.clone());
                    }
                });
                ui.end_row();
            }
        });
        if rows.is_empty() {
            ui.add_space(20.0);
            muted(ui, &c, if st.bindings.is_empty() { t("No shortcuts yet. Create one, or start from Windows actions.", "Henüz kısayol yok. Bir tane oluşturun veya Windows eylemlerinden başlayın.") } else { t("Nothing matches the search.", "Aramayla eşleşen yok.") });
        }
    });
    if let Some((what, i)) = act {
        match what {
            "edit" => {
                let b = st.bindings[i].clone();
                app.open_editor(b, true);
            }
            "toggle" => {
                let mut b = st.bindings[i].clone();
                b.enabled = !b.enabled;
                if b.enabled {
                    let others: Vec<Binding> = st.bindings.iter().filter(|x| x.id != b.id).cloned().collect();
                    if let Some(e) = crate::rules::validate(&b, &others) {
                        app.status = format!("{} • {e}", b.name);
                        return;
                    }
                    if !b.profile_id.is_empty() && b.profile_id != st.active_profile_id {
                        st.active_profile_id = b.profile_id.clone();
                    }
                }
                st.bindings[i] = b;
                app.commit(st);
            }
            "dup" => {
                let mut b = st.bindings[i].clone();
                b.id = crate::win::uuid_v4();
                b.name = format!("{} ({})", b.name, t("copy", "kopya"));
                b.enabled = false;
                st.bindings.insert(i + 1, b);
                app.commit(st);
            }
            "run" => {
                let b = st.bindings[i].clone();
                app.status = ts(format!("Running “{}”…", b.name), format!("“{}” çalıştırılıyor…", b.name));
                crate::actions::run_now(b);
            }
            "delete" => {
                st.bindings.remove(i);
                app.shortcuts.confirm_delete = None;
                app.commit(st);
            }
            _ => {}
        }
    }
}

pub fn recorder(app: &mut App, ui: &mut egui::Ui, st: &mut State) {
    let c = app.c;
    if !app.recorder.initialized {
        app.recorder.initialized = true;
        app.recorder.keep_waits = true;
    }
    header(ui, &c, t("Record a macro", "Makro kaydet"), t("Start recording, use your apps normally, then stop. Keys, clicks, scrolling, waits and app switches are captured; clicks inside Makro+ are ignored.", "Kaydı başlatın, uygulamalarınızı normal kullanın, sonra durdurun. Tuşlar, tıklamalar, kaydırma, beklemeler ve uygulama geçişleri kaydedilir; Makro+ içindeki tıklamalar yok sayılır."));
    ui.horizontal(|ui| {
        if app.recorder.recording {
            if accent_button(ui, &c, &format!("⏹ {}", t("Stop recording", "Kaydı durdur"))).clicked() {
                app.recorder.recording = false;
                crate::engine::set_mode(crate::engine::MODE_NORMAL);
            }
            ui.label(RichText::new(format!("● {} — {}", t("Recording", "Kaydediliyor"), app.recorder.steps.len())).color(ui.visuals().error_fg_color));
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(300));
        } else {
            if accent_button(ui, &c, &format!("⏺ {}", t("Start recording", "Kaydı başlat"))).clicked() {
                app.recorder.recording = true;
                app.recorder.steps.clear();
                crate::engine::set_mode(crate::engine::MODE_RECORD);
                app.hide_to_tray_for_recording();
            }
            if !app.recorder.steps.is_empty() && button(ui, t("Clear", "Temizle")).clicked() {
                app.recorder.steps.clear();
            }
        }
        ui.checkbox(&mut app.recorder.keep_waits, t("Keep pauses between actions", "Eylemler arası duraklamaları koru"));
    });
    if app.recorder.recording {
        muted(ui, &c, t("Makro+ minimised itself. Press Pause/Break to stop recording and come back here.", "Makro+ kendini küçülttü. Kaydı durdurup buraya dönmek için Pause/Break tuşuna basın."));
    }
    ui.add_space(8.0);
    let steps: Vec<ActionStep> = app.recorder.steps.iter().filter(|s| app.recorder.keep_waits || s.kind != "Wait").cloned().collect();
    card(ui, &c, |ui| {
        ui.set_width(ui.available_width());
        egui::ScrollArea::vertical().id_salt("rec").max_height(ui.available_height() - 80.0).show(ui, |ui| {
            for (i, s) in steps.iter().enumerate() {
                ui.label(format!("{:>3}. {}", i + 1, s.summary()));
            }
            if steps.is_empty() {
                muted(ui, &c, t("Nothing recorded yet.", "Henüz bir şey kaydedilmedi."));
            }
        });
    });
    if !app.recorder.recording && !steps.is_empty() && accent_button(ui, &c, t("Save as shortcut…", "Kısayol olarak kaydet…")).clicked() {
        let b = Binding { name: t("Recorded macro", "Kayıtlı makro").into(), is_macro: true, steps: Some(steps), ..Default::default() };
        app.open_editor(b, false);
        let _ = st;
    }
}

impl App {
    pub fn hide_to_tray_for_recording(&mut self) {
        self.win_minimize();
    }
}

pub fn recommended(app: &mut App, ui: &mut egui::Ui, st: &mut State) {
    let c = app.c;
    header(ui, &c, t("Windows actions", "Windows eylemleri"), t("Ready-made actions. Pick one, choose a key, save. No model needed.", "Hazır eylemler. Birini seçin, tuş belirleyin, kaydedin. Model gerekmez."));
    egui::ScrollArea::vertical().id_salt("rec-actions").show(ui, |ui| {
        let width = ui.available_width();
        let cols = ((width / 300.0).floor() as usize).max(1);
        egui::Grid::new("sys-grid").num_columns(cols).spacing([12.0, 12.0]).show(ui, |ui| {
            for (i, a) in crate::system_actions::CATALOG.iter().enumerate() {
                egui::Frame::none().fill(c.panel).rounding(12.0).inner_margin(14.0).stroke(egui::Stroke::new(1.0, c.line.gamma_multiply(0.6))).show(ui, |ui| {
                    ui.set_width(width / cols as f32 - 44.0);
                    ui.set_min_height(108.0);
                    ui.vertical(|ui| {
                    ui.label(RichText::new(crate::system_actions::title(a.id)).family(super::bold()).size(15.5));
                    muted(ui, &c, &crate::system_actions::description(a.id));
                    let used: Vec<String> = st.bindings.iter().filter(|b| b.kind == "System" && b.target == a.id).map(|b| b.combo()).collect();
                    ui.horizontal(|ui| {
                        if ui.button(t("Assign a key", "Tuş ata")).clicked() {
                            let b = Binding { name: crate::system_actions::title(a.id), kind: "System".into(), target: a.id.into(), steps: None, ..Default::default() };
                            app.open_editor(b, false);
                        }
                        if !used.is_empty() {
                            ui.label(RichText::new(used.join(", ")).color(c.accent).size(12.5));
                        }
                    });
                    });
                });
                if (i + 1) % cols == 0 {
                    ui.end_row();
                }
            }
        });
    });
}

pub fn conflicts(app: &mut App, ui: &mut egui::Ui, st: &mut State) {
    let c = app.c;
    header(ui, &c, t("Conflicts", "Çakışmalar"), t("CONFLICT: same key, overlapping scope. PRIORITY: an app-specific or profile-specific assignment wins. POSSIBLE: a known default of Windows or the app.", "ÇAKIŞMA: aynı tuş, örtüşen kapsam. ÖNCELİK: uygulamaya veya profile özel atama kazanır. OLASI: Windows'un veya uygulamanın bilinen varsayılanı."));
    let mut any = false;
    egui::ScrollArea::vertical().id_salt("conf").show(ui, |ui| {
        for b in st.bindings.iter().filter(|b| b.enabled) {
            let issues = crate::rules::conflicts(b, &st.bindings);
            let reserved = crate::keys::reserved(b.mods, b.key);
            if issues.is_empty() && reserved.is_none() {
                continue;
            }
            any = true;
            card(ui, &c, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new(format!("{} — {}", b.name, b.combo())).family(super::bold()));
                if let Some(r) = reserved {
                    ui.label(RichText::new(r).color(ui.visuals().error_fg_color));
                }
                for i in issues {
                    let (tag, col) = match i.severity {
                        crate::rules::Severity::Conflict => (t("CONFLICT", "ÇAKIŞMA"), ui.visuals().error_fg_color),
                        crate::rules::Severity::Priority => (t("PRIORITY", "ÖNCELİK"), c.warn),
                        crate::rules::Severity::Possible => (t("POSSIBLE", "OLASI"), c.muted),
                    };
                    ui.label(RichText::new(format!("{tag} • {}", i.text)).color(col));
                }
            });
            ui.add_space(8.0);
        }
        if !any {
            muted(ui, &c, t("No conflicts among your shortcuts.", "Kısayollarınız arasında çakışma yok."));
        }
    });
}

pub fn history(app: &mut App, ui: &mut egui::Ui, st: &mut State) {
    let c = app.c;
    header(ui, &c, t("History", "Geçmiş"), t("The last 300 results are kept on this computer.", "Son 300 sonuç bu bilgisayarda saklanır."));
    if button(ui, t("Clear history", "Geçmişi temizle")).clicked() {
        st.history.clear();
        app.commit(st);
    }
    ui.add_space(8.0);
    egui::ScrollArea::vertical().id_salt("hist").show(ui, |ui| {
        for h in &st.history {
            egui::Frame::none().fill(c.panel).rounding(8.0).inner_margin(10.0).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new(format!("{}  •  {}", h.time, h.name)).family(super::bold()).size(13.5));
                ui.label(RichText::new(&h.result).size(13.0).color(c.muted));
            });
            ui.add_space(4.0);
        }
        if st.history.is_empty() {
            muted(ui, &c, t("No activity yet.", "Henüz etkinlik yok."));
        }
    });
}
