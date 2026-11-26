//! AI model, appearance and general settings (including optional install).
use super::{accent_button, button, card, header, muted, section, App};
use crate::i18n::{t, ts};
use crate::model::{State, ThemePalette};
use eframe::egui::{self, RichText};

#[derive(Default)]
pub struct SettingsState {
    pub api_key: Option<String>,
    pub test_busy: bool,
    pub test_progress: String,
    pub test_result: Option<Result<String, String>>,
    pub theme_edit: Option<ThemePalette>,
    pub install_msg: Option<Result<String, String>>,
    pub desktop_shortcut: bool,
    pub new_profile: String,
    pub msg: String,
}

impl SettingsState {
    pub fn on_test(&mut self, r: Result<String, String>) {
        self.test_busy = false;
        self.test_progress.clear();
        self.test_result = Some(r);
    }
}

pub fn model(app: &mut App, ui: &mut egui::Ui, st: &mut State) {
    let c = app.c;
    header(ui, &c, t("AI model", "Yapay zekâ modeli"), t("Bring your own model: a local GGUF file (runs on this PC) or any OpenAI-compatible API. Saved shortcuts never need the model, except AI steps.", "Kendi modelinizi kullanın: yerel bir GGUF dosyası (bu bilgisayarda çalışır) veya OpenAI uyumlu herhangi bir API. Kaydedilmiş kısayollar AI adımları dışında modele ihtiyaç duymaz."));
    let mut dirty = false;
    egui::ScrollArea::vertical().id_salt("model").show(ui, |ui| {
        card(ui, &c, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                dirty |= ui.radio_value(&mut st.model_options.remote, false, t("Local GGUF file", "Yerel GGUF dosyası")).changed();
                dirty |= ui.radio_value(&mut st.model_options.remote, true, t("API server", "API sunucusu")).changed();
            });
            ui.add_space(6.0);
            if !st.model_options.remote {
                let resolved = crate::storage::resolve_model(&st.model_path);
                ui.label(t("Model file", "Model dosyası"));
                ui.horizontal(|ui| {
                    if ui.add(egui::TextEdit::singleline(&mut st.model_path).hint_text(t("models\\your-model.gguf", "models\\modeliniz.gguf")).desired_width(ui.available_width() - 200.0)).changed() {
                        dirty = true;
                    }
                    if ui.button(t("Browse GGUF…", "GGUF seç…")).clicked() {
                        if let Some(p) = rfd::FileDialog::new().add_filter("GGUF", &["gguf"]).pick_file() {
                            st.model_path = p.display().to_string();
                            dirty = true;
                        }
                    }
                    if ui.button(t("Models folder", "Modeller klasörü")).clicked() {
                        let dir = crate::storage::exe_dir().join("models");
                        let _ = std::fs::create_dir_all(&dir);
                        let _ = crate::win::shell_open(&dir.display().to_string(), "", "");
                    }
                });
                let found = crate::storage::model_candidates();
                if !found.is_empty() {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(t("Found:", "Bulunan:")).color(c.muted));
                        for f in found {
                            if ui.selectable_label(crate::win::same_path(&f, &resolved), crate::win::file_name(&f)).clicked() {
                                st.model_path = f;
                                dirty = true;
                            }
                        }
                    });
                }
                if resolved.is_empty() {
                    muted(ui, &c, t("No model selected. Copy a .gguf into the models folder next to MakroPlus.exe. Recommended: Qwen3.5-4B Q4_0.", "Model seçilmedi. MakroPlus.exe yanındaki models klasörüne bir .gguf kopyalayın. Önerilen: Qwen3.5-4B Q4_0."));
                }
                let mut gpu = st.use_gpu.unwrap_or(true);
                if ui.checkbox(&mut gpu, t("Use the GPU when available (falls back to CPU)", "Varsa GPU kullan (olmazsa CPU'ya geçer)")).changed() {
                    st.use_gpu = Some(gpu);
                    dirty = true;
                }
                dirty |= ui.checkbox(&mut st.model_options.fast_mode, t("Fast planning (turn off optional reasoning)", "Hızlı planlama (isteğe bağlı akıl yürütmeyi kapat)")).changed();
                if crate::llm::find_runtime().is_none() {
                    ui.label(RichText::new(t("The local engine (runtime\\llama) is missing next to MakroPlus.exe; use the full package or an API server.", "Yerel motor (runtime\\llama) MakroPlus.exe yanında yok; tam paketi veya bir API sunucusu kullanın.")).color(c.warn));
                }
            } else {
                egui::Grid::new("api").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
                    ui.label(t("Base URL", "Temel adres"));
                    dirty |= ui.add(egui::TextEdit::singleline(&mut st.model_options.endpoint).desired_width(420.0)).changed();
                    ui.end_row();
                    ui.label(t("Model ID", "Model kimliği"));
                    dirty |= ui.add(egui::TextEdit::singleline(&mut st.model_options.model).desired_width(420.0)).changed();
                    ui.end_row();
                    ui.label(t("API key", "API anahtarı"));
                    let key = app.settings.api_key.get_or_insert_with(|| st.model_options.api_key());
                    if ui.add(egui::TextEdit::singleline(key).password(true).desired_width(420.0)).changed() {
                        let k = key.clone();
                        st.model_options.set_api_key(&k);
                        dirty = true;
                    }
                    ui.end_row();
                });
                muted(ui, &c, t("The key is encrypted for your Windows account. Prompts, app names and scanned commands are sent to this server.", "Anahtar Windows hesabınıza özel şifrelenir. İstemler, uygulama adları ve taranan komutlar bu sunucuya gönderilir."));
            }
        });
        ui.add_space(10.0);
        card(ui, &c, |ui| {
            ui.set_width(ui.available_width());
            section(ui, &c, t("Planning", "Planlama"));
            dirty |= ui.checkbox(&mut st.model_options.structured_output, t("Ask for structured JSON output", "Yapılandırılmış JSON çıktısı iste")).changed();
            ui.horizontal(|ui| {
                ui.label(t("Max output tokens", "En fazla çıktı jetonu"));
                dirty |= ui.add(egui::DragValue::new(&mut st.model_options.max_output_tokens).range(256..=65536).speed(64)).changed();
                ui.label(t("Correction rounds", "Düzeltme turu"));
                dirty |= ui.add(egui::DragValue::new(&mut st.ai.correction_rounds).range(0..=4)).changed();
            });
            dirty |= ui.checkbox(&mut st.ai.share_app_commands, t("Give the model the scanned in-app commands", "Taranan uygulama içi komutları modele ver")).changed();
            dirty |= ui.checkbox(&mut st.ai.share_clipboard, t("Give the model the current clipboard text when planning", "Planlarken panodaki metni modele ver")).changed();
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.add_enabled(!app.settings.test_busy, egui::Button::new(t("Test connection", "Bağlantıyı dene"))).clicked() {
                    app.settings.test_busy = true;
                    app.settings.test_result = None;
                    crate::request_save();
                    std::thread::spawn(|| {
                        let r = crate::llm::test_connection(&|p| crate::send_ui(crate::UiEvent::Progress(p)));
                        crate::send_ui(crate::UiEvent::AiText("test".into(), r));
                    });
                }
                if app.settings.test_busy {
                    ui.spinner();
                    ui.label(RichText::new(&app.settings.test_progress).color(c.muted));
                }
                if ui.button(t("Stop local model", "Yerel modeli durdur")).on_hover_text(t("Frees the memory used by the local model.", "Yerel modelin kullandığı belleği boşaltır.")).clicked() {
                    std::thread::spawn(crate::llm::stop);
                }
            });
            match &app.settings.test_result {
                Some(Ok(s)) => {
                    ui.label(RichText::new(format!("✓ {} “{s}”", t("The model answered:", "Model yanıt verdi:"))).color(c.ok));
                }
                Some(Err(e)) => {
                    ui.label(RichText::new(e).color(ui.visuals().error_fg_color));
                }
                None => {}
            }
        });
    });
    if dirty {
        app.commit(st);
    }
}

pub fn appearance(app: &mut App, ui: &mut egui::Ui, st: &mut State) {
    let c = app.c;
    header(ui, &c, t("Appearance", "Görünüm"), t("Pick a theme or make your own. Changes apply immediately.", "Bir tema seçin veya kendinizinkini yapın. Değişiklikler hemen uygulanır."));
    let current = crate::theme::resolve(st);
    // ---- Background ----
    card(ui, &c, |ui| {
        ui.set_width(ui.available_width());
        section(ui, &c, t("Background", "Arka plan"));
        let mut dirty = false;
        ui.horizontal(|ui| {
            if accent_button(ui, &c, t("Choose image, GIF or video…", "Görsel, GIF veya video seç…")).clicked() {
                if let Some(f) = rfd::FileDialog::new().add_filter(t("Images and videos", "Görseller ve videolar"), crate::background::EXTENSIONS).pick_file() {
                    st.background.path = f.display().to_string();
                    st.background.enabled = true;
                    app.auto_theme_for.clear();
                    dirty = true;
                }
            }
            if !st.background.path.is_empty() {
                dirty |= ui.checkbox(&mut st.background.enabled, t("Show", "Göster")).changed();
                if ui.button(t("Remove", "Kaldır")).clicked() {
                    st.background.path.clear();
                    st.background.enabled = false;
                    dirty = true;
                }
            }
        });
        if !st.background.path.is_empty() {
            ui.label(RichText::new(crate::win::file_name(&st.background.path)).size(12.0).color(c.muted)).on_hover_text(st.background.path.as_str());
            if !app.bg.error.is_empty() {
                ui.label(RichText::new(&app.bg.error).color(ui.visuals().error_fg_color));
            }
            ui.horizontal(|ui| {
                ui.label(t("Darken", "Karartma"));
                dirty |= ui.add(egui::Slider::new(&mut st.background.dim, 0.0..=0.95).show_value(false)).changed();
                ui.label(t("Panel opacity", "Panel saydamlığı"));
                dirty |= ui.add(egui::Slider::new(&mut st.background.panel_opacity, 0.3..=1.0).show_value(false)).changed();
            });
            ui.horizontal(|ui| {
                let sample = app.bg.sample.lock().clone();
                if ui.add_enabled(sample.is_some(), egui::Button::new(t("Make a theme from it", "Ondan tema oluştur"))).on_hover_text(t("Builds a readable theme from the background's colours.", "Arka planın renklerinden okunabilir bir tema üretir.")).clicked() {
                    if let Some((px, w, h)) = sample {
                        let p = crate::theme::palette_from_image(&px, w, h, &format!("{} {}", t("Theme from", "Tema:"), crate::win::file_stem(&st.background.path)));
                        st.theme_id = Some(p.id.clone());
                        st.dark = p.dark;
                        st.custom_themes.push(p);
                        dirty = true;
                    }
                }
                if ui.checkbox(&mut st.auto_theme, t("Adapt the theme automatically when the background changes", "Arka plan değişince temayı otomatik uyarla")).changed() {
                    app.auto_theme_for.clear();
                    dirty = true;
                }
            });
            if crate::background::is_video(&st.background.path) {
                muted(ui, &c, t("Videos play muted and pause while Makro+ is hidden.", "Videolar sessiz oynar ve Makro+ gizliyken durur."));
            }
        }
        if dirty {
            app.commit(st);
        }
    });
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        if accent_button(ui, &c, t("New dark theme from scratch", "Sıfırdan koyu tema")).clicked() {
            app.settings.theme_edit = Some(crate::theme::blank(true));
        }
        if accent_button(ui, &c, t("New light theme from scratch", "Sıfırdan açık tema")).clicked() {
            app.settings.theme_edit = Some(crate::theme::blank(false));
        }
        if button(ui, t("Copy current theme", "Mevcut temayı kopyala")).clicked() {
            let mut p = current.clone();
            p.id = format!("custom-{}", &crate::win::uuid_v4()[..8]);
            p.name = t("My theme", "Temam").into();
            p.description = String::new();
            app.settings.theme_edit = Some(p);
        }
        if button(ui, t("Import theme…", "Tema içe aktar…")).clicked() {
            if let Some(f) = rfd::FileDialog::new().add_filter("JSON", &["json"]).pick_file() {
                match std::fs::read_to_string(&f).map_err(|e| e.to_string()).and_then(|s| serde_json::from_str::<ThemePalette>(&s).map_err(|e| e.to_string())) {
                    Ok(mut p) => {
                        if !p.id.starts_with("custom-") {
                            p.id = format!("custom-{}", &crate::win::uuid_v4()[..8]);
                        }
                        st.custom_themes.retain(|x| x.id != p.id);
                        st.theme_id = Some(p.id.clone());
                        st.dark = p.dark;
                        st.custom_themes.push(p);
                        app.commit(st);
                    }
                    Err(e) => app.status = e,
                }
            }
        }
        if button(ui, t("Export current…", "Mevcut temayı dışa aktar…")).clicked() {
            if let Some(f) = rfd::FileDialog::new().set_file_name(format!("{}.json", current.name)).add_filter("JSON", &["json"]).save_file() {
                let _ = std::fs::write(f, serde_json::to_string_pretty(&current).unwrap_or_default());
            }
        }
        if current.id.starts_with("custom-") {
            if button(ui, t("Edit", "Düzenle")).clicked() {
                app.settings.theme_edit = Some(current.clone());
            }
            if button(ui, t("Delete", "Sil")).clicked() {
                st.custom_themes.retain(|x| x.id != current.id);
                st.theme_id = None;
                app.commit(st);
            }
        }
    });
    ui.add_space(10.0);
    if let Some(mut p) = app.settings.theme_edit.clone() {
        let mut close = false;
        card(ui, &c, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(t("Name", "Ad"));
                ui.text_edit_singleline(&mut p.name);
                ui.checkbox(&mut p.dark, t("Dark", "Koyu"));
            });
            egui::Grid::new("theme-edit").num_columns(4).spacing([16.0, 8.0]).show(ui, |ui| {
                let mut accent = p.accent();
                let mut sel = p.selection();
                let mut accent_ink = p.accent_ink();
                let fields: Vec<(&str, &mut String)> = vec![
                    (t("Accent text", "Vurgu üstü metin"), &mut accent_ink),
                    (t("Highlight areas", "Vurgulu alanlar"), &mut p.hero),
                    (t("Background", "Arka plan"), &mut p.background),
                    (t("Panel", "Panel"), &mut p.panel),
                    (t("Surface", "Yüzey"), &mut p.surface),
                    (t("Text", "Metin"), &mut p.ink),
                    (t("Muted text", "Soluk metin"), &mut p.muted),
                    (t("Lines", "Çizgiler"), &mut p.line),
                    (t("Accent", "Vurgu"), &mut accent),
                    (t("Selection", "Seçim"), &mut sel),
                ];
                for (i, (label, value)) in fields.into_iter().enumerate() {
                    let mut col = crate::theme::parse_hex(value);
                    ui.label(label);
                    if ui.color_edit_button_srgba(&mut col).changed() {
                        *value = crate::theme::to_hex(col);
                    }
                    if i % 2 == 1 {
                        ui.end_row();
                    }
                }
                p.accent_color = Some(accent);
                p.selection_color = Some(sel);
                p.accent_text_color = Some(accent_ink);
            });
            ui.horizontal(|ui| {
                if accent_button(ui, &c, t("Save theme", "Temayı kaydet")).clicked() {
                    st.custom_themes.retain(|x| x.id != p.id);
                    st.custom_themes.push(p.clone());
                    st.theme_id = Some(p.id.clone());
                    st.dark = p.dark;
                    close = true;
                }
                if button(ui, t("Cancel", "İptal")).clicked() {
                    close = true;
                }
            });
        });
        if close {
            app.settings.theme_edit = None;
            app.commit(st);
        } else {
            app.settings.theme_edit = Some(p);
        }
        ui.add_space(10.0);
    }
    let width = ui.available_width();
    let cols = ((width / 260.0).floor() as usize).max(1);
    let mut chosen = None;
    egui::ScrollArea::vertical().id_salt("themes").show(ui, |ui| {
        egui::Grid::new("theme-grid").num_columns(cols).spacing([12.0, 12.0]).show(ui, |ui| {
            for (i, p) in crate::theme::all(st).iter().enumerate() {
                let pc = crate::theme::colors(p);
                let selected = p.id == current.id;
                let resp = egui::Frame::none().fill(if selected { c.selection } else { c.panel }).rounding(12.0).inner_margin(12.0).stroke(egui::Stroke::new(if selected { 2.0 } else { 1.0 }, if selected { c.accent } else { c.line.gamma_multiply(0.6) })).show(ui, |ui| {
                    ui.set_width(width / cols as f32 - 40.0);
                    ui.vertical(|ui| {
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 64.0), egui::Sense::hover());
                    let painter = ui.painter();
                    painter.rect_filled(rect, 8.0, pc.bg);
                    painter.rect_filled(egui::Rect::from_min_size(rect.min + egui::vec2(6.0, 6.0), egui::vec2(rect.width() * 0.26, rect.height() - 12.0)), 5.0, pc.panel);
                    painter.rect_filled(egui::Rect::from_min_size(rect.min + egui::vec2(rect.width() * 0.34, 14.0), egui::vec2(80.0, 6.0)), 3.0, pc.ink);
                    painter.rect_filled(egui::Rect::from_min_size(rect.min + egui::vec2(rect.width() * 0.34, 26.0), egui::vec2(120.0, 5.0)), 3.0, pc.muted);
                    painter.rect_filled(egui::Rect::from_min_size(rect.min + egui::vec2(rect.width() * 0.34, 40.0), egui::vec2(34.0, 11.0)), 3.0, pc.accent);
                    ui.label(RichText::new(format!("{}{}", p.name, if selected { "  ✓" } else { "" })).family(super::bold()));
                    ui.label(RichText::new(format!("{} • {}", if p.dark { t("Dark", "Koyu") } else { t("Light", "Açık") }, if p.id.starts_with("custom-") { t("custom", "özel").to_string() } else { p.description.clone() })).size(12.0).color(c.muted));
                    });
                });
                if resp.response.interact(egui::Sense::click()).clicked() {
                    chosen = Some((p.id.clone(), p.dark));
                }
                if (i + 1) % cols == 0 {
                    ui.end_row();
                }
            }
        });
    });
    if let Some((id, dark)) = chosen {
        st.theme_id = Some(id);
        st.dark = dark;
        app.commit(st);
    }
}

pub fn settings(app: &mut App, ui: &mut egui::Ui, st: &mut State) {
    let c = app.c;
    header(ui, &c, t("Settings", "Ayarlar"), "");
    let mut dirty = false;
    egui::ScrollArea::vertical().id_salt("settings").show(ui, |ui| {
        // ---- Install ----
        card(ui, &c, |ui| {
            ui.set_width(ui.available_width());
            section(ui, &c, t("Install on this computer (optional)", "Bu bilgisayara yükle (isteğe bağlı)"));
            let installed = crate::installer::is_installed();
            let running_installed = crate::installer::running_installed();
            if running_installed {
                muted(ui, &c, &ts(format!("Installed in {}. Makro+ appears in Start and in Settings › Apps › Installed apps.", crate::installer::install_dir().display()), format!("{} konumunda kurulu. Makro+ Başlat menüsünde ve Ayarlar › Uygulamalar › Yüklü uygulamalar listesinde görünür.", crate::installer::install_dir().display())));
            } else {
                muted(ui, &c, t("Makro+ runs as a single exe without installing. Installing copies it to your user folder, adds it to Start and to Settings › Apps › Installed apps (with an uninstaller). No administrator rights are needed.", "Makro+ kurulmadan tek bir exe olarak çalışır. Yüklerseniz kullanıcı klasörünüze kopyalanır; Başlat menüsüne ve Ayarlar › Uygulamalar › Yüklü uygulamalar listesine (kaldırıcısıyla birlikte) eklenir. Yönetici izni gerekmez."));
                if installed {
                    ui.label(RichText::new(t("An installed copy already exists; installing again updates it.", "Kurulu bir kopya zaten var; tekrar yüklemek onu günceller.")).color(c.muted));
                }
                ui.checkbox(&mut app.settings.desktop_shortcut, t("Also add a desktop shortcut", "Masaüstüne de kısayol ekle"));
                ui.horizontal(|ui| {
                    if accent_button(ui, &c, if installed { t("Update installed copy", "Kurulu kopyayı güncelle") } else { t("Install Makro+", "Makro+'ı yükle") }).clicked() {
                        crate::request_save();
                        app.settings.install_msg = Some(crate::installer::install(app.settings.desktop_shortcut));
                    }
                });
                if let Some(Ok(_)) = &app.settings.install_msg {
                    if button(ui, t("Switch to the installed copy now", "Şimdi kurulu kopyaya geç")).clicked() {
                        let _ = crate::save_now_from_ui(st);
                        let exe = crate::installer::installed_exe();
                        let _ = std::process::Command::new(&exe).args(["--wait-pid", &std::process::id().to_string()]).spawn();
                        crate::shell::remove_icon();
                        std::process::exit(0);
                    }
                }
            }
            match &app.settings.install_msg {
                Some(Ok(m)) => {
                    ui.label(RichText::new(m).color(c.ok));
                }
                Some(Err(e)) => {
                    ui.label(RichText::new(e).color(ui.visuals().error_fg_color));
                }
                None => {}
            }
            if installed {
                ui.horizontal(|ui| {
                    if ui.button(t("Uninstall…", "Kaldır…")).clicked() {
                        let exe = crate::installer::installed_exe();
                        let _ = std::process::Command::new(&exe).arg("--uninstall").spawn();
                    }
                    muted(ui, &c, t("You can also uninstall from Settings › Apps.", "Ayarlar › Uygulamalar üzerinden de kaldırabilirsiniz."));
                });
            }
        });
        #[cfg(not(windows))]
        {
            ui.add_space(10.0);
            card(ui, &c, |ui| {
                ui.set_width(ui.available_width());
                section(ui, &c, t("Linux permissions", "Linux izinleri"));
                let (read, write) = crate::input::permissions();
                let mark = |ok: bool| if ok { "✓" } else { "✗" };
                ui.label(RichText::new(format!("{} {}", mark(read), t("Read keyboard/mouse (/dev/input)", "Klavye/fare okuma (/dev/input)"))).color(if read { c.ok } else { c.warn }));
                ui.label(RichText::new(format!("{} {}", mark(write), t("Send keys and clicks (/dev/uinput)", "Tuş ve tıklama gönderme (/dev/uinput)"))).color(if write { c.ok } else { c.warn }));
                let cmd = "sudo usermod -aG input \"$USER\" && echo 'KERNEL==\"uinput\", GROUP=\"input\", MODE=\"0660\", OPTIONS+=\"static_node=uinput\"' | sudo tee /etc/udev/rules.d/60-makroplus.rules && sudo udevadm control --reload && sudo udevadm trigger";
                if !read || !write {
                    muted(ui, &c, t("Run this once in a terminal, then log out and back in (the .deb/.rpm packages already install the udev rule):", "Bunu bir terminalde bir kez çalıştırın, sonra oturumu kapatıp açın (.deb/.rpm paketleri udev kuralını zaten kurar):"));
                    ui.add(egui::TextEdit::multiline(&mut cmd.to_string()).code_editor().desired_rows(2).desired_width(f32::INFINITY));
                    if ui.button(t("Copy command", "Komutu kopyala")).clicked() {
                        ui.ctx().copy_text(cmd.to_string());
                    }
                }
                dirty |= ui.checkbox(&mut st.linux_exclusive, t("Exclusive mode: swallow matched keys and allow switching keys off (restart needed)", "Özel mod: eşleşen tuşları yut ve tuş kapatmaya izin ver (yeniden başlatma gerekir)")).changed();
                muted(ui, &c, t("Without exclusive mode, shortcuts still run but the key also reaches the app. App-specific shortcuts need X11, Sway or Hyprland; other Wayland desktops do not share the active window.", "Özel mod olmadan kısayollar çalışır ama tuş uygulamaya da ulaşır. Uygulamaya özel kısayollar X11, Sway veya Hyprland gerektirir; diğer Wayland masaüstleri etkin pencereyi paylaşmaz."));
            });
        }
        ui.add_space(10.0);
        // ---- Updates ----
        card(ui, &c, |ui| {
            ui.set_width(ui.available_width());
            section(ui, &c, t("App updates", "Uygulama güncellemeleri"));
            let mut on = crate::updates::enabled(st);
            if ui.checkbox(&mut on, t("Check for a new version every time Makro+ starts", "Makro+ her açıldığında yeni sürümü denetle")).changed() {
                crate::updates::set_enabled(st, on);
                dirty = true;
            }
            let s = crate::updates::status();
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{} v{}", t("Installed version:", "Yüklü sürüm:"), env!("CARGO_PKG_VERSION"))).color(c.muted));
                if ui.add_enabled(!s.checking, egui::Button::new(t("Check now", "Şimdi denetle"))).clicked() {
                    crate::updates::check_async(false);
                }
                if s.checking {
                    ui.spinner();
                }
                if ui.link(t("All releases", "Tüm sürümler")).clicked() {
                    let _ = crate::win::shell_open(crate::updates::RELEASES, "", "");
                }
            });
            if !s.message.is_empty() {
                ui.label(RichText::new(&s.message).color(if s.available.is_some() { c.accent } else { c.muted }));
            }
            if let Some(a) = s.available {
                if accent_button(ui, &c, &ts(format!("Download {}", a.tag), format!("{} sürümünü indir", a.tag))).clicked() {
                    let _ = crate::win::shell_open(&a.url, "", "");
                }
            }
            muted(ui, &c, t("Only the release page on GitHub is read. You decide when to download and install.", "Yalnızca GitHub'daki sürüm sayfası okunur. Ne zaman indirip kuracağınıza siz karar verirsiniz."));
        });
        ui.add_space(10.0);
        // ---- Behaviour ----
        card(ui, &c, |ui| {
            ui.set_width(ui.available_width());
            section(ui, &c, t("Behaviour", "Davranış"));
            let mut auto = crate::installer::autostart_enabled();
            if ui.checkbox(&mut auto, t("Start with Windows (in the tray)", "Windows ile başlat (tepside)")).changed() {
                let exe = std::env::current_exe().unwrap_or_default();
                if let Err(e) = crate::installer::set_autostart(auto, &exe) {
                    app.status = e;
                }
                st.start_with_windows = auto;
                dirty = true;
            }
            dirty |= ui.checkbox(&mut st.close_to_tray, t("Closing the window keeps Makro+ running in the tray", "Pencereyi kapatınca Makro+ tepside çalışmaya devam etsin")).changed();
            dirty |= ui.checkbox(&mut st.start_minimized, t("Start minimised to the tray", "Tepside küçültülmüş başlat")).changed();
            dirty |= ui.checkbox(&mut st.allow_remapped_input, t("Accept keys/buttons remapped by other software (device utilities)", "Başka yazılımların yeniden eşlediği tuş/düğmeleri kabul et (aygıt araçları)")).changed();
            ui.horizontal(|ui| {
                ui.label(t("Pause between workflow steps", "İş akışı adımları arası bekleme"));
                let mut v = st.step_interval_ms.unwrap_or(15);
                egui::ComboBox::from_id_salt("speed").selected_text(format!("{v} ms")).show_ui(ui, |ui| {
                    for (ms, en, tr) in [(0, "Instant", "Anında"), (15, "Fast (default)", "Hızlı (varsayılan)"), (50, "Balanced", "Dengeli"), (100, "Compatible", "Uyumlu"), (250, "Slow apps", "Yavaş uygulamalar")] {
                        ui.selectable_value(&mut v, ms, format!("{} • {ms} ms", t(en, tr)));
                    }
                });
                if Some(v) != st.step_interval_ms && !(st.step_interval_ms.is_none() && v == 15) {
                    st.step_interval_ms = Some(v);
                    dirty = true;
                }
            });
            ui.add_space(4.0);
            dirty |= ui.checkbox(&mut st.guide_enabled, t("Shortcut guide: hold a key to see the shortcuts of the app in front", "Kısayol rehberi: öndeki uygulamanın kısayollarını görmek için bir tuşu basılı tutun")).changed();
            ui.horizontal(|ui| {
                ui.label(t("Guide key", "Rehber tuşu"));
                egui::ComboBox::from_id_salt("guide-key").selected_text(match st.guide_hold_key { 162 => t("Left Ctrl", "Sol Ctrl"), 161 => t("Right Shift", "Sağ Shift"), 160 => t("Left Shift", "Sol Shift"), _ => t("Right Ctrl", "Sağ Ctrl") }).show_ui(ui, |ui| {
                    for (k, en, tr) in [(163, "Right Ctrl", "Sağ Ctrl"), (162, "Left Ctrl", "Sol Ctrl"), (161, "Right Shift", "Sağ Shift"), (160, "Left Shift", "Sol Shift")] {
                        dirty |= ui.selectable_value(&mut st.guide_hold_key, k, t(en, tr)).changed();
                    }
                });
            });
            dirty |= ui.checkbox(&mut st.guide_block_key, t("Switch off the guide key's normal function (it only opens the guide)", "Rehber tuşunun normal işlevini kapat (yalnızca rehberi açsın)")).changed();
            ui.label(RichText::new(t("The guide shows:", "Rehberde gösterilecekler:")).family(super::bold()));
            ui.horizontal_wrapped(|ui| {
                let g = &mut st.guide_show;
                dirty |= ui.checkbox(&mut g.this_app, t("My shortcuts for the app in front", "Öndeki uygulamaya atadıklarım")).changed();
                dirty |= ui.checkbox(&mut g.global, t("My global shortcuts", "Genel kısayollarım")).changed();
                dirty |= ui.checkbox(&mut g.all_apps, t("My shortcuts for every app", "Tüm uygulamalara atadıklarım")).changed();
                dirty |= ui.checkbox(&mut g.app_own, t("The app's own shortcuts (after a scan)", "Uygulamanın kendi kısayolları (taramadan sonra)")).changed();
                dirty |= ui.checkbox(&mut g.system, t("Windows system shortcuts", "Windows sistem kısayolları")).changed();
            });
            muted(ui, &c, t("Actions run once all keys are released. Esc cancels a running workflow. Makro+'s own synthetic input never triggers shortcuts.", "Eylemler tüm tuşlar bırakılınca bir kez çalışır. Esc çalışan iş akışını iptal eder. Makro+'ın kendi ürettiği girişler kısayolları tetiklemez."));
        });
        ui.add_space(10.0);
        // ---- Switched-off keys ----
        card(ui, &c, |ui| {
            ui.set_width(ui.available_width());
            section(ui, &c, t("Switched-off keys", "Kapatılan tuşlar"));
            muted(ui, &c, t("Turn off a key's function everywhere or only in one app. Nothing is removed: untick it and the key works again.", "Bir tuşun işlevini her yerde veya yalnızca bir uygulamada kapatın. Hiçbir şey silinmez: işareti kaldırınca tuş yeniden çalışır."));
            let mut changed = false;
            for b in st.bindings.iter_mut().filter(|b| b.kind == "Disable key") {
                let combo = b.combo();
                ui.horizontal(|ui| {
                    changed |= ui.checkbox(&mut b.enabled, RichText::new(combo).color(c.accent)).changed();
                    ui.label(RichText::new(format!("• {}", b.scope_name())).color(c.muted));
                });
            }
            if changed {
                dirty = true;
            }
            if ui.button(format!("+ {}", t("Switch a key off…", "Bir tuşu kapat…"))).clicked() {
                app.editor = Some(super::editor::Editor::new(crate::model::Binding { name: t("Switched-off key", "Kapatılan tuş").into(), kind: "Disable key".into(), steps: None, ..Default::default() }, false));
            }
        });
        ui.add_space(10.0);
        // ---- Touchpad ----
        card(ui, &c, |ui| {
            ui.set_width(ui.available_width());
            section(ui, &c, t("Touchpad shapes", "Touchpad şekilleri"));
            dirty |= ui.checkbox(&mut st.gestures.enabled, t("Run shortcuts when I draw their shape on the touchpad", "Touchpad'de şeklini çizdiğimde kısayolları çalıştır")).changed();
            ui.horizontal(|ui| {
                ui.label(RichText::new(if st.gestures.device.is_empty() { t("Touchpad: Windows Precision Touchpad / not detected yet", "Touchpad: Windows Precision Touchpad / henüz tanıtılmadı") } else { t("Touchpad detected", "Touchpad tanıtıldı") }).color(c.muted));
                if ui.button(t("Detect again", "Yeniden tanıt")).clicked() {
                    crate::gesture::set_mode(crate::gesture::MODE_CALIBRATE);
                    app.settings.msg = t("Move one finger on your touchpad…", "Touchpad'de tek parmağınızı gezdirin…").into();
                }
            });
            ui.horizontal(|ui| {
                ui.label(t("Pause that ends a shape", "Şekli bitiren bekleme"));
                dirty |= ui.add(egui::DragValue::new(&mut st.gestures.pause_milliseconds).range(250..=2000).speed(10).suffix(" ms")).changed();
                ui.label(t("Strictness", "Hassasiyet"));
                dirty |= ui.add(egui::Slider::new(&mut st.gestures.minimum_score, 0.75..=0.97).fixed_decimals(2)).changed();
            });
            muted(ui, &c, t("Create a shape shortcut from New shortcut → Trigger type → Touchpad shape.", "Şekilli kısayolu Yeni kısayol → Tetikleyici türü → Touchpad şekli ile oluşturun."));
        });
        ui.add_space(10.0);
        // ---- Profiles ----
        card(ui, &c, |ui| {
            ui.set_width(ui.available_width());
            section(ui, &c, t("Profiles", "Profiller"));
            let mut remove = None;
            for (i, p) in st.profiles.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    dirty |= ui.add(egui::TextEdit::singleline(&mut p.name).desired_width(200.0)).changed();
                    dirty |= ui.checkbox(&mut p.suppress_typing_keys, t("Ignore plain typing keys (games)", "Düz yazı tuşlarını yok say (oyunlar)")).changed();
                    if ui.small_button(t("Remove", "Kaldır")).clicked() {
                        remove = Some(i);
                    }
                });
            }
            if let Some(i) = remove {
                if st.profiles.len() > 1 {
                    let id = st.profiles.remove(i).id;
                    for b in st.bindings.iter_mut().filter(|b| b.profile_id == id) {
                        b.profile_id.clear();
                    }
                    dirty = true;
                }
            }
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut app.settings.new_profile).hint_text(t("New profile name", "Yeni profil adı")).desired_width(200.0));
                if ui.button(t("Add profile", "Profil ekle")).clicked() && !app.settings.new_profile.trim().is_empty() {
                    st.profiles.push(crate::model::ShortcutProfile { id: format!("p-{}", &crate::win::uuid_v4()[..8]), name: app.settings.new_profile.trim().into(), suppress_typing_keys: false });
                    app.settings.new_profile.clear();
                    dirty = true;
                }
            });
        });
        ui.add_space(10.0);
        // ---- Data ----
        card(ui, &c, |ui| {
            ui.set_width(ui.available_width());
            section(ui, &c, t("Data and backup", "Veri ve yedek"));
            muted(ui, &c, &ts(format!("Settings: {}", crate::storage::settings_path().display()), format!("Ayarlar: {}", crate::storage::settings_path().display())));
            ui.horizontal_wrapped(|ui| {
                if ui.button(t("Open data folder", "Veri klasörünü aç")).clicked() {
                    let d = crate::storage::data_dir();
                    let _ = std::fs::create_dir_all(&d);
                    let _ = crate::win::shell_open(&d.display().to_string(), "", "");
                }
                if ui.button(t("Export shortcuts…", "Kısayolları dışa aktar…")).clicked() {
                    if let Some(f) = rfd::FileDialog::new().set_file_name("makroplus-shortcuts.json").add_filter("JSON", &["json"]).save_file() {
                        let mut export = st.clone();
                        export.history.clear();
                        export.model_options.protected_key = None;
                        let _ = std::fs::write(f, serde_json::to_vec_pretty(&export).unwrap_or_default());
                        app.settings.msg = t("Exported (without history and API key).", "Dışa aktarıldı (geçmiş ve API anahtarı olmadan).").into();
                    }
                }
                if ui.button(t("Import shortcuts…", "Kısayolları içe aktar…")).clicked() {
                    if let Some(f) = rfd::FileDialog::new().add_filter("JSON", &["json"]).pick_file() {
                        app.settings.msg = import(st, &f);
                        dirty = true;
                    }
                }
                let legacy = crate::storage::legacy_path();
                if legacy.exists() && ui.button(t("Import again from Makro+ 2.x", "Makro+ 2.x'ten yeniden içe aktar")).clicked() {
                    app.settings.msg = import(st, &legacy);
                    dirty = true;
                }
            });
            if !app.settings.msg.is_empty() {
                ui.label(RichText::new(&app.settings.msg).color(c.muted));
            }
            if crate::storage::is_portable() {
                muted(ui, &c, t("Portable mode: portable.txt next to the exe keeps all data in its data folder.", "Taşınabilir mod: exe yanındaki portable.txt tüm verileri data klasöründe tutar."));
            }
        });
    });
    if dirty {
        app.commit(st);
    }
}

/// Merge shortcuts from another settings file; existing ones are kept.
fn import(st: &mut State, path: &std::path::Path) -> String {
    let parsed = std::fs::read(path).map_err(|e| e.to_string()).and_then(|b| serde_json::from_slice::<State>(String::from_utf8_lossy(&b).trim_start_matches('\u{feff}').as_bytes()).map_err(|e| e.to_string()));
    match parsed {
        Err(e) => format!("{} {e}", t("Import failed:", "İçe aktarma başarısız:")),
        Ok(other) => {
            let mut added = 0;
            for mut b in other.bindings {
                if st.bindings.iter().any(|x| x.id == b.id) {
                    continue;
                }
                if crate::rules::validate(&b, &st.bindings).is_some() {
                    b.enabled = false;
                }
                st.bindings.push(b);
                added += 1;
            }
            for p in other.application_paths {
                st.remember_app(&p);
            }
            ts(format!("{added} shortcuts imported. Shortcuts with conflicts were added disabled."), format!("{added} kısayol içe aktarıldı. Çakışanlar devre dışı olarak eklendi."))
        }
    }
}
