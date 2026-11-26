//! Colour palettes (same presets as 2.x) and egui styling.
use crate::model::{State, ThemePalette};
use eframe::egui::{self, Color32, Rounding, Stroke};

pub fn presets() -> Vec<ThemePalette> {
    let p = |id: &str, name: &str, desc: &str, dark: bool, c: [&str; 7]| ThemePalette {
        id: id.into(),
        name: name.into(),
        description: desc.into(),
        dark,
        background: c[0].into(),
        panel: c[1].into(),
        surface: c[2].into(),
        ink: c[3].into(),
        muted: c[4].into(),
        line: c[5].into(),
        hero: c[6].into(),
        ..Default::default()
    };
    vec![
        p("graphite", "Graphite", "Neutral charcoal", true, ["#181A1E", "#22252B", "#2D323A", "#F3F4F7", "#B5BCC7", "#464D58", "#262C34"]),
        p("midnight", "Midnight", "Deep navy", true, ["#111827", "#1C2536", "#29354A", "#EFF3FA", "#B5C1D6", "#465773", "#202D44"]),
        p("forest", "Forest", "Quiet evergreen", true, ["#131E1A", "#1C2B24", "#2A3A31", "#F0F5F1", "#B4C9BD", "#465E50", "#22372B"]),
        p("paper", "Paper", "Clean neutral light", false, ["#F5F5F4", "#FFFFFF", "#EBEDED", "#252C32", "#59636D", "#CBD1D4", "#E7EBE9"]),
        p("mist", "Mist", "Cool blue-gray", false, ["#F0F4F8", "#FFFFFF", "#E5EBF3", "#233044", "#52647A", "#C6D2E1", "#DFE8F3"]),
        p("sand", "Sand", "Soft warm beige", false, ["#F5F1E9", "#FFFDFA", "#EDE6DA", "#382F28", "#706151", "#D8CCBB", "#E9E0D1"]),
        a(p("ocean", "Ocean", "Deep teal", true, ["#0E1A1F", "#14262D", "#1D343D", "#E8F6F7", "#A9C9CE", "#335159", "#17303A"]), "#3EC1C9", "#062326", "#1F4A50"),
        a(p("plum", "Plum", "Velvet purple", true, ["#18121F", "#211A2B", "#2D2439", "#F4EEFA", "#C4B6D6", "#4A3D5C", "#261D33"]), "#B98CFF", "#1C0F2E", "#3E2C57"),
        a(p("ember", "Ember", "Warm night orange", true, ["#1C1512", "#271D18", "#342721", "#FAF0EA", "#D2BCAE", "#57443A", "#2E221C"]), "#FF9F5A", "#2E1607", "#55331F"),
        a(p("nord", "Nord", "Arctic blue-grey", true, ["#242933", "#2E3440", "#3B4252", "#ECEFF4", "#C5CCD8", "#4C566A", "#323949"]), "#88C0D0", "#132028", "#3B4E5E"),
        a(p("matrix", "Terminal", "Black and green", true, ["#0A0D0A", "#101510", "#172017", "#D8F5D8", "#8FBF8F", "#284028", "#121A12"]), "#3DDC6A", "#04170A", "#1C3D24"),
        a(p("contrast", "High contrast", "Maximum readability", true, ["#000000", "#0B0B0B", "#1A1A1A", "#FFFFFF", "#E0E0E0", "#7A7A7A", "#101010"]), "#FFD400", "#000000", "#3A3200"),
        a(p("rose", "Rose", "Soft blush light", false, ["#FBF3F4", "#FFFFFF", "#F4E4E7", "#3A2328", "#7A5A60", "#E2C6CC", "#F3DDE2"]), "#C2456A", "#FFFFFF", "#F6D3DD"),
        a(p("mint", "Mint", "Fresh green light", false, ["#F1F8F4", "#FFFFFF", "#E2F0E8", "#1F3328", "#4F6B5B", "#C3DCCD", "#DDEFE4"]), "#1F8A5B", "#FFFFFF", "#CFEBDD"),
        a(p("lavender", "Lavender", "Calm violet light", false, ["#F5F3FB", "#FFFFFF", "#E9E5F6", "#2A2540", "#5E5880", "#D2CCEA", "#E6E1F5"]), "#6A4FD1", "#FFFFFF", "#DCD4F7"),
        a(p("solar", "Solar", "Warm paper light", false, ["#FDF6E3", "#FFFCF2", "#EEE8D5", "#3B3A32", "#6E6A5A", "#D9D1B8", "#F1E9CF"]), "#CB4B16", "#FFFFFF", "#F4DCC8"),
    ]
}

fn a(mut p: ThemePalette, accent: &str, accent_ink: &str, selection: &str) -> ThemePalette {
    p.accent_color = Some(accent.into());
    p.accent_text_color = Some(accent_ink.into());
    p.selection_color = Some(selection.into());
    p
}

/// Neutral starting point for a theme built from scratch.
pub fn blank(dark: bool) -> ThemePalette {
    let base = if dark {
        ["#1A1A1C", "#232326", "#2E2E32", "#F2F2F3", "#B8B8BE", "#45454B", "#28282C"]
    } else {
        ["#F4F4F5", "#FFFFFF", "#E8E8EA", "#26262A", "#5E5E66", "#CFCFD4", "#EBEBED"]
    };
    ThemePalette {
        id: format!("custom-{}", &crate::win::uuid_v4()[..8]),
        name: crate::i18n::t("New theme", "Yeni tema").into(),
        description: String::new(),
        dark,
        background: base[0].into(),
        panel: base[1].into(),
        surface: base[2].into(),
        ink: base[3].into(),
        muted: base[4].into(),
        line: base[5].into(),
        hero: base[6].into(),
        accent_color: Some(if dark { "#5AA9FF".into() } else { "#2563EB".into() }),
        accent_text_color: Some(if dark { "#0B1A2B".into() } else { "#FFFFFF".into() }),
        peach_color: None,
        selection_color: Some(if dark { "#253447".into() } else { "#DCE7FB".into() }),
    }
}

fn rgb_to_hsl(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let (mx, mn) = (r.max(g).max(b), r.min(g).min(b));
    let l = (mx + mn) / 2.0;
    if (mx - mn).abs() < 1e-6 {
        return (0.0, 0.0, l);
    }
    let d = mx - mn;
    let s = if l > 0.5 { d / (2.0 - mx - mn) } else { d / (mx + mn) };
    let h = if mx == r { (g - b) / d + if g < b { 6.0 } else { 0.0 } } else if mx == g { (b - r) / d + 2.0 } else { (r - g) / d + 4.0 };
    (h / 6.0, s, l)
}

fn hsl_hex(h: f32, s: f32, l: f32) -> String {
    let f = |n: f32| {
        let k = (n + h * 12.0) % 12.0;
        let a = s * l.min(1.0 - l);
        l - a * (k - 3.0).min(9.0 - k).clamp(-1.0, 1.0)
    };
    to_hex(Color32::from_rgb((f(0.0) * 255.0).round() as u8, (f(8.0) * 255.0).round() as u8, (f(4.0) * 255.0).round() as u8))
}

/// Derive a readable theme from an image's dominant colours.
pub fn palette_from_image(rgba: &[u8], w: usize, h: usize, name: &str) -> ThemePalette {
    let step = ((w * h) / 6000).max(1);
    let mut lum = 0.0f32;
    let mut count = 0.0f32;
    let mut bins = [0.0f32; 24];
    let mut bin_sl = [(0.0f32, 0.0f32); 24];
    let (mut hx, mut hy) = (0.0f32, 0.0f32);
    for i in (0..w * h).step_by(step) {
        let o = i * 4;
        if o + 2 >= rgba.len() {
            break;
        }
        let (r, g, b) = (rgba[o] as f32 / 255.0, rgba[o + 1] as f32 / 255.0, rgba[o + 2] as f32 / 255.0);
        let (hh, s, l) = rgb_to_hsl(r, g, b);
        lum += 0.2126 * r + 0.7152 * g + 0.0722 * b;
        count += 1.0;
        let weight = s * (1.0 - (l - 0.5).abs() * 2.0).max(0.05);
        let bi = ((hh * 24.0) as usize).min(23);
        bins[bi] += weight;
        bin_sl[bi].0 += s * weight;
        bin_sl[bi].1 += l * weight;
        let ang = hh * std::f32::consts::TAU;
        hx += ang.cos() * s;
        hy += ang.sin() * s;
    }
    let avg = if count > 0.0 { lum / count } else { 0.3 };
    let dark = avg < 0.58;
    let (bi, bw) = bins.iter().enumerate().fold((0, 0.0), |a, (i, &v)| if v > a.1 { (i, v) } else { a });
    let accent_h = (bi as f32 + 0.5) / 24.0;
    let accent_s = if bw > 0.0 { (bin_sl[bi].0 / bw).clamp(0.45, 0.85) } else { 0.6 };
    let base_h = (hy.atan2(hx) / std::f32::consts::TAU).rem_euclid(1.0);
    let base_s = ((hx * hx + hy * hy).sqrt() / count.max(1.0)).clamp(0.05, 0.28);
    let (bg, panel, surface, line, hero, ink, muted, accent, accent_ink, selection) = if dark {
        (
            hsl_hex(base_h, base_s, 0.08),
            hsl_hex(base_h, base_s, 0.12),
            hsl_hex(base_h, base_s, 0.17),
            hsl_hex(base_h, base_s, 0.29),
            hsl_hex(base_h, base_s, 0.14),
            hsl_hex(base_h, 0.12, 0.95),
            hsl_hex(base_h, 0.12, 0.74),
            hsl_hex(accent_h, accent_s, 0.66),
            hsl_hex(accent_h, 0.5, 0.12),
            hsl_hex(accent_h, 0.32, 0.24),
        )
    } else {
        (
            hsl_hex(base_h, base_s, 0.96),
            hsl_hex(base_h, base_s, 0.995),
            hsl_hex(base_h, base_s, 0.91),
            hsl_hex(base_h, base_s, 0.82),
            hsl_hex(base_h, base_s, 0.93),
            hsl_hex(base_h, 0.2, 0.15),
            hsl_hex(base_h, 0.12, 0.40),
            hsl_hex(accent_h, accent_s, 0.40),
            "#FFFFFF".into(),
            hsl_hex(accent_h, 0.5, 0.88),
        )
    };
    ThemePalette {
        id: format!("custom-{}", &crate::win::uuid_v4()[..8]),
        name: name.into(),
        description: String::new(),
        dark,
        background: bg,
        panel,
        surface,
        ink,
        muted,
        line,
        hero,
        accent_color: Some(accent),
        accent_text_color: Some(accent_ink),
        peach_color: None,
        selection_color: Some(selection),
    }
}

pub fn all(state: &State) -> Vec<ThemePalette> {
    let mut v = presets();
    v.extend(state.custom_themes.iter().cloned());
    v
}

pub fn resolve(state: &State) -> ThemePalette {
    let id = state.theme_id.clone().unwrap_or_default();
    all(state).into_iter().find(|p| p.id == id).unwrap_or_else(|| presets().into_iter().find(|p| p.id == if state.dark { "graphite" } else { "paper" }).unwrap())
}

pub fn parse_hex(s: &str) -> Color32 {
    let h = s.trim().trim_start_matches('#');
    let v = |i: usize| u8::from_str_radix(h.get(i..i + 2).unwrap_or("00"), 16).unwrap_or(0);
    match h.len() {
        8 => Color32::from_rgba_unmultiplied(v(2), v(4), v(6), v(0)),
        6 => Color32::from_rgb(v(0), v(2), v(4)),
        _ => Color32::GRAY,
    }
}

pub fn to_hex(c: Color32) -> String {
    format!("#{:02X}{:02X}{:02X}", c.r(), c.g(), c.b())
}

#[derive(Clone, Copy)]
pub struct Colors {
    pub bg: Color32,
    pub panel: Color32,
    pub surface: Color32,
    pub ink: Color32,
    pub muted: Color32,
    pub line: Color32,
    pub accent: Color32,
    pub accent_ink: Color32,
    pub selection: Color32,
    pub hero: Color32,
    pub ok: Color32,
    pub warn: Color32,
}

pub fn colors(p: &ThemePalette) -> Colors {
    Colors {
        bg: parse_hex(&p.background),
        panel: parse_hex(&p.panel),
        surface: parse_hex(&p.surface),
        ink: parse_hex(&p.ink),
        muted: parse_hex(&p.muted),
        line: parse_hex(&p.line),
        accent: parse_hex(&p.accent()),
        accent_ink: parse_hex(&p.accent_ink()),
        selection: parse_hex(&p.selection()),
        hero: parse_hex(&p.hero),
        ok: if p.dark { Color32::from_rgb(120, 200, 140) } else { Color32::from_rgb(30, 130, 70) },
        warn: if p.dark { Color32::from_rgb(240, 190, 100) } else { Color32::from_rgb(170, 100, 0) },
    }
}

pub fn apply(ctx: &egui::Context, p: &ThemePalette) -> Colors {
    let c = colors(p);
    let mut v = if p.dark { egui::Visuals::dark() } else { egui::Visuals::light() };
    v.override_text_color = Some(c.ink);
    v.panel_fill = c.bg;
    v.window_fill = c.panel;
    v.extreme_bg_color = c.surface;
    v.faint_bg_color = c.panel;
    v.code_bg_color = c.surface;
    v.window_stroke = Stroke::new(1.0, c.line);
    v.window_rounding = Rounding::same(12.0);
    v.menu_rounding = Rounding::same(8.0);
    v.selection.bg_fill = c.selection;
    v.selection.stroke = Stroke::new(1.0, c.accent);
    v.hyperlink_color = c.accent;
    v.warn_fg_color = c.warn;
    v.error_fg_color = if p.dark { Color32::from_rgb(255, 130, 120) } else { Color32::from_rgb(180, 40, 40) };
    let r = Rounding::same(5.0);
    v.widgets.noninteractive.bg_fill = c.panel;
    v.widgets.noninteractive.weak_bg_fill = c.panel;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, c.line);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, c.ink);
    v.widgets.noninteractive.rounding = r;
    for (w, fill) in [(&mut v.widgets.inactive, c.surface), (&mut v.widgets.hovered, c.selection), (&mut v.widgets.active, c.selection), (&mut v.widgets.open, c.selection)] {
        w.bg_fill = fill;
        w.weak_bg_fill = fill;
        w.bg_stroke = Stroke::new(1.0, c.line);
        w.fg_stroke = Stroke::new(1.0, c.ink);
        w.rounding = r;
    }
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, c.accent);
    v.widgets.active.bg_stroke = Stroke::new(1.0, c.accent);
    ctx.set_visuals(v);
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);
    style.spacing.interact_size.y = 30.0;
    style.spacing.combo_height = 320.0;
    style.spacing.scroll = egui::style::ScrollStyle::solid();
    ctx.set_style(style);
    c
}
