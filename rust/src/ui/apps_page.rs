//! Applications: registered apps, in-app command scan and per-app shortcuts.
use super::{accent_button, button, card, header, muted, section, App, Page};
use crate::i18n::{t, ts};
use crate::model::{ActionStep, AppCommand, AppInventory, Binding, State};
use eframe::egui::{self, RichText};

#[derive(Default)]
pub struct AppsState {
    pub detail: Option<String>,
    pub search: String,
    pub cmd_search: String,
    pub cmd_filter: usize,
    pub scanning: Option<String>,
    pub adding: Option<AddApps>,
}

pub struct AddApps {
    pub running: Vec<(String, bool)>,
}

impl AppsState {
    pub fn open_add(&mut self) {
        self.adding = Some(AddApps { running: crate::win::running_apps().into_iter().map(|p| (p, false)).collect() });
    }

    pub fn on_scan(&mut self, r: Result<AppInventory, String>, st: &mut State, status: &mut String) {
        self.scanning = None;
        match r {
            Ok(inv) => {
                *status = ts(format!("Scan finished: {} commands found in {}.", inv.commands.len(), crate::apps::display_name(&inv.app_path)), format!("Tarama bitti: {} içinde {} komut bulundu.", crate::apps::display_name(&inv.app_path), inv.commands.len()));
                st.app_inventories.retain(|i| !crate::win::same_path(&i.app_path, &inv.app_path));
                st.app_inventories.push(inv);
                crate::engine::sync(st);
                crate::request_save();
            }
            Err(e) => *status = e,
        }
    }
}

fn start_scan(app: &mut App, path: &str) {
    app.apps.scanning = Some(path.to_string());
    let p = path.to_string();
    std::thread::spawn(move || {
        crate::uia::com_init();
        let r = crate::uia::scan_app(&p);
        crate::send_ui(crate::UiEvent::Scan(r));
    });
}

pub fn show(app: &mut App, ui: &mut egui::Ui, st: &mut State) {
    if let Some(p) = app.apps.detail.clone() {
        detail(app, ui, st, &p);
        return;
    }
    let c = app.c;
    header(ui, &c, t("Applications", "Uygulamalar"), t("Register the apps you use. Apps do not need to be open to get shortcuts; open them once to scan their menus and controls.", "Kullandığınız uygulamaları kaydedin. Kısayol için açık olmaları gerekmez; menülerini ve denetimlerini taramak için bir kez açın."));
    ui.horizontal(|ui| {
        if accent_button(ui, &c, &format!("+ {}", t("Add applications", "Uygulama ekle"))).clicked() {
            app.apps.open_add();
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add(egui::TextEdit::singleline(&mut app.apps.search).hint_text(t("Search apps…", "Uygulama ara…")).desired_width(220.0));
        });
    });
    ui.add_space(10.0);
    let q = app.apps.search.to_lowercase();
    let list: Vec<String> = st.application_paths.iter().filter(|p| q.is_empty() || p.to_lowercase().contains(&q) || crate::apps::display_name(p).to_lowercase().contains(&q)).cloned().collect();
    let running = crate::win::running_apps();
    egui::ScrollArea::vertical().id_salt("apps").auto_shrink([false, false]).show(ui, |ui| {
        let width = ui.available_width();
        let cols = ((width / 290.0).floor() as usize).max(1);
        let mut open: Option<String> = None;
        egui::Grid::new("apps-grid").num_columns(cols).spacing([12.0, 12.0]).show(ui, |ui| {
            for (i, p) in list.iter().enumerate() {
                let count = st.bindings.iter().filter(|b| b.touches(p)).count();
                let scanned = st.inventory(p).map(|i| i.commands.len());
                let is_running = running.iter().any(|r| crate::win::same_path(r, p));
                let resp = egui::Frame::none().fill(c.panel).rounding(12.0).inner_margin(14.0).stroke(egui::Stroke::new(1.0, c.line.gamma_multiply(0.6))).show(ui, |ui| {
                    ui.set_width(width / cols as f32 - 44.0);
                    ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(crate::apps::display_name(p)).family(super::bold()).size(16.0));
                        if is_running {
                            ui.label(RichText::new("●").color(c.ok).size(11.0)).on_hover_text(t("Running", "Çalışıyor"));
                        }
                    });
                    ui.label(RichText::new(crate::model::short(p, 52)).size(11.5).color(c.muted)).on_hover_text(p.as_str());
                    ui.label(RichText::new(format!(
                        "{} • {}",
                        ts(format!("{count} shortcuts"), format!("{count} kısayol")),
                        match scanned {
                            Some(n) => ts(format!("{n} in-app commands"), format!("{n} uygulama içi komut")),
                            None => t("not scanned", "taranmadı").to_string(),
                        }
                    ))
                    .size(12.5));
                    if !std::path::Path::new(p).exists() {
                        ui.label(RichText::new(t("File not found", "Dosya bulunamadı")).color(c.warn).size(12.0));
                    }
                    });
                });
                if resp.response.interact(egui::Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    open = Some(p.clone());
                }
                if (i + 1) % cols == 0 {
                    ui.end_row();
                }
            }
        });
        if list.is_empty() {
            muted(ui, &c, t("No applications yet. Add them from running apps or .exe files.", "Henüz uygulama yok. Çalışan uygulamalardan veya .exe dosyalarından ekleyin."));
        }
        if let Some(p) = open {
            app.apps.detail = Some(p);
            app.apps.cmd_search.clear();
        }
    });
}

fn binding_for(path: &str, cmd: &AppCommand) -> Binding {
    let mut step = ActionStep::default();
    let combo = crate::keys::parse_combo(&cmd.accelerator);
    if cmd.source == "menu" {
        step.kind = "Menu command".into();
        step.text = cmd.label.clone();
    } else if cmd.role == "field" {
        step.kind = "Set field".into();
        step.automation_id = Some(cmd.automation_id.clone()).filter(|s| !s.is_empty());
        step.path = Some(cmd.label.rsplit(" › ").next().unwrap_or(&cmd.label).to_string());
        step.text = "{clipboard}".into();
    } else if !cmd.automation_id.is_empty() || !cmd.label.is_empty() {
        step.kind = "Click element".into();
        step.automation_id = Some(cmd.automation_id.clone()).filter(|s| !s.is_empty());
        step.text = cmd.label.rsplit(" › ").next().unwrap_or(&cmd.label).to_string();
        step.control_type = cmd.control_type;
    } else if let Some((m, k)) = combo {
        step.kind = "Key".into();
        step.mods = m;
        step.key = k;
    }
    step.app_path = Some(path.to_string());
    let label = cmd.label.rsplit(" › ").next().unwrap_or(&cmd.label).to_string();
    Binding { name: format!("{} – {}", crate::apps::display_name(path), label), app_path: path.to_string(), steps: Some(vec![step]), ..Default::default() }
}

fn detail(app: &mut App, ui: &mut egui::Ui, st: &mut State, path: &str) {
    let c = app.c;
    ui.horizontal(|ui| {
        if ui.button(format!("← {}", t("Applications", "Uygulamalar"))).clicked() {
            app.apps.detail = None;
        }
    });
    let running = !crate::win::app_windows(path).is_empty();
    header(ui, &c, &crate::apps::display_name(path), path);
    ui.horizontal_wrapped(|ui| {
        if accent_button(ui, &c, t("Shortcut inside this app", "Bu uygulamada kısayol")).clicked() {
            let b = Binding { name: format!("{} {}", crate::apps::display_name(path), t("shortcut", "kısayolu")), app_path: path.into(), ..Default::default() };
            app.open_editor(b, false);
        }
        if button(ui, t("Shortcut that opens it", "Açan kısayol")).clicked() {
            let b = Binding { name: format!("{} {}", crate::apps::display_name(path), t("open", "aç")), kind: "Application".into(), target: path.into(), steps: None, ..Default::default() };
            app.open_editor(b, false);
        }
        if button(ui, t("Describe it to AI", "Yapay zekâya anlat")).clicked() {
            app.prompt.scope = path.into();
            app.page = Page::Prompt;
        }
        if !running && button(ui, t("Start app", "Uygulamayı başlat")).clicked() {
            if let Err(e) = crate::apps::open(path, "", "") {
                app.status = e;
            }
        }
        if button(ui, t("Remove from list", "Listeden kaldır")).clicked() {
            st.application_paths.retain(|p| !crate::win::same_path(p, path));
            st.app_inventories.retain(|i| !crate::win::same_path(&i.app_path, path));
            app.apps.detail = None;
            app.commit(st);
        }
    });
    ui.add_space(10.0);
    ui.columns(2, |cols| {
        let ui = &mut cols[0];
        card(ui, &c, |ui| {
            ui.set_width(ui.available_width());
            section(ui, &c, t("Assigned shortcuts", "Atanmış kısayollar"));
            let list: Vec<usize> = (0..st.bindings.len()).filter(|&i| st.bindings[i].touches(path)).collect();
            if list.is_empty() {
                muted(ui, &c, t("None yet.", "Henüz yok."));
            }
            let mut edit = None;
            egui::ScrollArea::vertical().id_salt("app-sc").max_height(420.0).show(ui, |ui| {
                for i in list {
                    let b = &st.bindings[i];
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(b.combo()).color(c.accent));
                        if ui.link(crate::model::short(&b.name, 40)).clicked() {
                            edit = Some(i);
                        }
                    });
                    ui.label(RichText::new(crate::model::short(&b.summary(), 80)).size(12.0).color(c.muted));
                }
            });
            if let Some(i) = edit {
                let b = st.bindings[i].clone();
                app.open_editor(b, true);
            }
        });
        let ui = &mut cols[1];
        card(ui, &c, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                section(ui, &c, t("Inside the app", "Uygulama içi"));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let busy = app.apps.scanning.is_some();
                    let label = if busy { t("Scanning…", "Taranıyor…") } else if st.inventory(path).is_some() { t("Scan again", "Yeniden tara") } else { t("Scan app", "Uygulamayı tara") };
                    if ui.add_enabled(!busy && running, egui::Button::new(label)).on_disabled_hover_text(t("Start the app first.", "Önce uygulamayı başlatın.")).clicked() {
                        start_scan(app, path);
                    }
                    if busy {
                        ui.spinner();
                    }
                });
            });
            match st.inventory(path).cloned() {
                None => muted(ui, &c, t("Scan reads the app's menus, buttons, tabs, fields and their keyboard shortcuts. Then you can run any of them from a shortcut, and the AI can plan with them.", "Tarama; uygulamanın menülerini, düğmelerini, sekmelerini, alanlarını ve klavye kısayollarını okur. Sonra bunlardan herhangi birini kısayolla çalıştırabilirsiniz; yapay zekâ da planlarken bunları kullanır.")),
                Some(inv) => {
                    muted(ui, &c, &ts(format!("Scanned {} • window “{}”", inv.scanned, crate::model::short(&inv.window_title, 40)), format!("Tarandı: {} • pencere “{}”", inv.scanned, crate::model::short(&inv.window_title, 40))));
                    ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::singleline(&mut app.apps.cmd_search).hint_text(t("Filter…", "Filtrele…")).desired_width(180.0));
                        for (i, l) in [t("All", "Tümü"), t("Menus", "Menüler"), t("Controls", "Denetimler"), t("With keys", "Tuşlu")].iter().enumerate() {
                            ui.selectable_value(&mut app.apps.cmd_filter, i, *l);
                        }
                    });
                    let q = app.apps.cmd_search.to_lowercase();
                    let items: Vec<&AppCommand> = inv
                        .commands
                        .iter()
                        .filter(|cmd| {
                            (q.is_empty() || cmd.label.to_lowercase().contains(&q) || cmd.accelerator.to_lowercase().contains(&q))
                                && match app.apps.cmd_filter {
                                    1 => cmd.source == "menu",
                                    2 => cmd.source == "element",
                                    3 => !cmd.accelerator.is_empty(),
                                    _ => true,
                                }
                        })
                        .collect();
                    let mut assign = None;
                    egui::ScrollArea::vertical().id_salt("app-cmds").max_height(400.0).show(ui, |ui| {
                        egui::Grid::new("cmd-grid").num_columns(3).striped(true).spacing([10.0, 6.0]).show(ui, |ui| {
                            for (i, cmd) in items.iter().enumerate().take(600) {
                                let kind = if cmd.source == "menu" { t("menu", "menü") } else { match cmd.role.as_str() { "button" => t("button", "düğme"), "field" => t("field", "alan"), "tab" => t("tab", "sekme"), "menu item" => t("menu", "menü"), "link" => t("link", "bağlantı"), "checkbox" => t("checkbox", "onay kutusu"), "list item" => t("list item", "liste öğesi"), "combobox" => t("dropdown", "açılır liste"), other => other } };
                                ui.label(RichText::new(crate::model::short(&cmd.label, 54)).size(13.0)).on_hover_text(&cmd.label);
                                ui.label(RichText::new(format!("{}{}", kind, if cmd.accelerator.is_empty() { String::new() } else { format!(" • {}", cmd.accelerator) })).size(12.0).color(c.muted));
                                if ui.small_button(t("Assign key", "Tuş ata")).clicked() {
                                    assign = Some(i);
                                }
                                ui.end_row();
                            }
                        });
                        if items.is_empty() {
                            muted(ui, &c, t("Nothing found. Try another filter, or open the window/menu you need and scan again.", "Bulunamadı. Başka bir filtre deneyin veya gereken pencereyi/menüyü açıp yeniden tarayın."));
                        }
                    });
                    if let Some(i) = assign {
                        let b = binding_for(path, items[i]);
                        app.open_editor(b, false);
                    }
                }
            }
        });
    });
}

pub fn add_dialog(app: &mut App, ctx: &egui::Context, st: &mut State) {
    let c = app.c;
    let mut open = true;
    let mut done = false;
    egui::Window::new(RichText::new(t("Add applications", "Uygulama ekle")).family(super::bold()).size(18.0))
        .collapsible(false)
        .resizable(true)
        .default_size([640.0, 520.0])
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .open(&mut open)
        .show(ctx, |ui| {
            muted(ui, &c, t("Pick from running apps, or choose .exe files. Adding does not start the app.", "Çalışan uygulamalardan seçin veya .exe dosyalarını seçin. Eklemek uygulamayı başlatmaz."));
            ui.horizontal(|ui| {
                if button(ui, t("Choose .exe files…", ".exe dosyaları seç…")).clicked() {
                    if let Some(files) = rfd::FileDialog::new().add_filter("exe", &["exe"]).pick_files() {
                        let add = app.apps.adding.as_mut().unwrap();
                        for f in files {
                            let s = f.display().to_string();
                            if !add.running.iter().any(|(p, _)| crate::win::same_path(p, &s)) {
                                add.running.insert(0, (s, true));
                            }
                        }
                    }
                }
                if button(ui, t("Refresh", "Yenile")).clicked() {
                    app.apps.open_add();
                }
            });
            egui::ScrollArea::vertical().id_salt("add-list").max_height(360.0).show(ui, |ui| {
                let add = app.apps.adding.as_mut().unwrap();
                for (p, sel) in add.running.iter_mut() {
                    let known = st.application_paths.iter().any(|a| crate::win::same_path(a, p));
                    ui.add_enabled_ui(!known, |ui| {
                        ui.checkbox(sel, format!("{}{}", crate::apps::display_name(p), if known { t("  • already added", "  • zaten ekli") } else { "" })).on_hover_text(p.as_str());
                    });
                    ui.label(RichText::new(crate::model::short(p, 80)).size(11.0).color(c.muted));
                }
            });
            ui.separator();
            if accent_button(ui, &c, t("Add selected", "Seçilenleri ekle")).clicked() {
                done = true;
            }
        });
    if done {
        let picked: Vec<String> = app.apps.adding.as_ref().unwrap().running.iter().filter(|(_, s)| *s).map(|(p, _)| p.clone()).collect();
        let mut n = 0;
        for p in picked {
            if st.remember_app(&p) {
                n += 1;
            }
        }
        app.status = ts(format!("{n} applications added."), format!("{n} uygulama eklendi."));
        app.apps.adding = None;
        app.commit(st);
    } else if !open {
        app.apps.adding = None;
    }
}
