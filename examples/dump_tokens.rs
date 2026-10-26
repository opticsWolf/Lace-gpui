//! Dump every preset's enforced [`Tokens`] as JSON — the data source for
//! documentation, the theme sheet, and the sandbox preview pipeline.
//!
//! ```sh
//! cargo run --example dump_tokens > tokens.json
//! ```
//!
//! No gpui anywhere: the crate's core (colour + engine + themes) is
//! zero-dependency, so this builds and runs in any environment. The JSON
//! carries all 79 colour tokens per theme, the 7 geometry tokens, the spec
//! knobs that produced them, and Lace's outline recipe
//! (`color::mix(button_bg, text, outline_strength)`).

use lace_gpui::{build, color::Rgba, themes};

/// `#rrggbb`, plus `[aa]` appended when the colour is not opaque.
fn hx(c: Rgba) -> String {
    let [r, g, b, a] = c;
    if a == 255 {
        format!("#{:02x}{:02x}{:02x}", r, g, b)
    } else {
        format!("#{:02x}{:02x}{:02x}[{:02x}]", r, g, b, a)
    }
}

fn hx_opt(c: Option<Rgba>) -> String {
    match c {
        Some(c) => format!("\"{}\"", hx(c)),
        None => "null".to_string(),
    }
}

/// A JSON object from `(key, colour)` pairs.
fn obj(pairs: &[(&str, Rgba)]) -> String {
    let inner: Vec<String> = pairs
        .iter()
        .map(|(k, v)| format!("\"{}\":\"{}\"", k, hx(*v)))
        .collect();
    format!("{{{}}}", inner.join(","))
}

/// A JSON object from `(key, pre-formatted value)` pairs.
fn raw_obj(pairs: &[(&str, String)]) -> String {
    let inner: Vec<String> = pairs
        .iter()
        .map(|(k, v)| format!("\"{}\":{}", k, v))
        .collect();
    format!("{{{}}}", inner.join(","))
}

fn quoted(s: &str) -> String {
    format!("\"{}\"", s)
}

fn num(v: f64) -> String {
    format!("{}", v)
}

fn main() {
    let mut out = String::from("{\"version\":\"0.9.7\",\"themes\":[");
    for (i, spec) in themes::ALL.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        let t = build(spec);

        // Mirror of skin::dock_chrome, for feature-less builds.
        let chrome = if spec.corner_radius >= 10.0 {
            "TabBar"
        } else {
            "Auto"
        };

        let core = obj(&[
            ("canvas_bg", t.canvas_bg),
            ("border_color", t.border_color),
            ("accent_color", t.accent_color),
            ("focus_border_color", t.focus_border_color),
            ("text", t.text),
            ("disabled_text", t.disabled_text),
            ("success", t.success),
            ("warning", t.warning),
            ("error", t.error),
            ("info", t.info),
            ("tooltip_bg", t.tooltip_bg),
            ("tooltip_text", t.tooltip_text),
        ]);
        let panel = obj(&[
            ("panel_bg", t.panel_bg),
            ("input_bg", t.input_bg),
            ("alternate_base", t.alternate_base),
            ("button_bg", t.button_bg),
            ("color_light", t.color_light),
            ("color_mid", t.color_mid),
            ("color_dark", t.color_dark),
            ("color_shadow", t.color_shadow),
            ("highlight", t.highlight),
            ("highlighted_text", t.highlighted_text),
        ]);
        let derived = obj(&[
            ("text_muted", t.text_muted),
            ("text_active", t.text_active),
            ("btn_disabled", t.btn_disabled),
            ("accent_bright", t.accent_bright),
            ("accent_dim", t.accent_dim),
            ("hover", t.hover),
            ("hover_end", t.hover_end),
            ("btn_hover_title", t.btn_hover_title),
            ("btn_hover_panel", t.btn_hover_panel),
        ]);
        let sidebar = obj(&[
            ("sidebar_bg", t.sidebar_bg),
            ("sidebar_tab_bg_normal", t.sidebar_tab_bg_normal),
            ("sidebar_tab_bg_hover_start", t.sidebar_tab_bg_hover_start),
            ("sidebar_tab_bg_hover_end", t.sidebar_tab_bg_hover_end),
            ("sidebar_tab_bg_active", t.sidebar_tab_bg_active),
            ("sidebar_tab_border_normal", t.sidebar_tab_border_normal),
            ("sidebar_tab_border_active", t.sidebar_tab_border_active),
            ("sidebar_tab_border_hover", t.sidebar_tab_border_hover),
            ("sidebar_tab_text_normal", t.sidebar_tab_text_normal),
            ("sidebar_tab_text_active", t.sidebar_tab_text_active),
            ("sidebar_tab_text_disabled", t.sidebar_tab_text_disabled),
            ("sidebar_indicator", t.sidebar_indicator),
            ("sidebar_badge_bg", t.sidebar_badge_bg),
            ("sidebar_badge_text", t.sidebar_badge_text),
        ]);
        let sidepanel = obj(&[
            ("sidepanel_bg", t.sidepanel_bg),
            ("sidepanel_title_text", t.sidepanel_title_text),
            ("sidepanel_button_color", t.sidepanel_button_color),
            ("sidepanel_button_disabled", t.sidepanel_button_disabled),
            ("sidepanel_button_hover", t.sidepanel_button_hover),
            ("sidepanel_shadow", t.sidepanel_shadow),
            ("sidepanel_border", t.sidepanel_border),
            ("sidepanel_focus_border", t.sidepanel_focus_border),
        ]);
        let tab = obj(&[
            ("tab_bg_normal", t.tab_bg_normal),
            ("tab_bg_hover", t.tab_bg_hover),
            ("tab_bg_active", t.tab_bg_active),
            ("tab_border_normal", t.tab_border_normal),
            ("tab_border_active", t.tab_border_active),
            ("tab_text_normal", t.tab_text_normal),
            ("tab_text_active", t.tab_text_active),
            ("tab_indicator", t.tab_indicator),
            ("tab_close_btn", t.tab_close_btn),
            ("tab_close_btn_hover", t.tab_close_btn_hover),
            ("tab_close_btn_disabled", t.tab_close_btn_disabled),
        ]);
        let title_bar = obj(&[
            ("title_border_color", t.title_border_color),
            ("title_focus_border", t.title_focus_border),
            ("title_text_normal", t.title_text_normal),
            ("title_text_active", t.title_text_active),
            ("title_active_edge", t.title_active_edge),
            ("title_button_color", t.title_button_color),
            ("title_button_disabled", t.title_button_disabled),
            ("title_button_hover", t.title_button_hover),
        ]);
        let splitter = obj(&[
            ("splitter_handle", t.splitter_handle),
            ("splitter_handle_hover", t.splitter_handle_hover),
        ]);
        let overlay = obj(&[
            ("overlay_frame", t.overlay_frame),
            ("overlay_background", t.overlay_background),
            ("overlay_color", t.overlay_color),
            ("overlay_arrow", t.overlay_arrow),
            ("overlay_shadow", t.overlay_shadow),
        ]);

        let scrollbar = match t.scrollbar {
            lace_gpui::engine::Scrollbar::Thin => "thin",
            lace_gpui::engine::Scrollbar::Expanding => "expanding",
            lace_gpui::engine::Scrollbar::Fusion => "fusion",
        };

        let spec_json = raw_obj(&[
            ("name", quoted(spec.name)),
            ("base", quoted(&hx(spec.base))),
            ("accent", quoted(&hx(spec.accent))),
            ("text", quoted(&hx(spec.text))),
            ("surface", hx_opt(spec.surface)),
            ("border", hx_opt(spec.border)),
            ("focus_border", hx_opt(spec.focus_border)),
            ("sidebar_tab_border", hx_opt(spec.sidebar_tab_border)),
            ("sidebar_tab_border_active", hx_opt(spec.sidebar_tab_border_active)),
            ("sidebar_tab_border_hover", hx_opt(spec.sidebar_tab_border_hover)),
            ("title_bg", hx_opt(spec.title_bg)),
            ("tooltip_bg", hx_opt(spec.tooltip_bg)),
            ("tooltip_text", hx_opt(spec.tooltip_text)),
            ("success", hx_opt(spec.success)),
            ("warning", hx_opt(spec.warning)),
            ("error", hx_opt(spec.error)),
            ("info", hx_opt(spec.info)),
            (
                "is_light",
                quoted(match spec.is_light {
                    lace_gpui::engine::Rel::Auto => "auto",
                    lace_gpui::engine::Rel::Light => "light",
                    lace_gpui::engine::Rel::Dark => "dark",
                }),
            ),
            ("keep_tint", format!("{}", spec.keep_tint)),
            ("scrollbar", quoted(scrollbar)),
            ("control_radius", num(spec.control_radius)),
            ("corner_radius", num(spec.corner_radius)),
            ("tab_radius", num(spec.tab_radius)),
            ("border_width", num(spec.border_width)),
            ("outline_strength", num(spec.outline_strength)),
            ("focus_width", num(spec.focus_width)),
            ("title_height", num(spec.title_height)),
        ]);

        let geometry = raw_obj(&[
            ("control_radius", num(t.control_radius)),
            ("corner_radius", num(t.corner_radius)),
            ("tab_radius", num(t.tab_radius)),
            ("border_width", num(t.border_width)),
            ("outline_strength", num(t.outline_strength)),
            ("focus_width", num(t.focus_width)),
            ("title_height", num(t.title_height)),
        ]);

        let outline_mix = lace_gpui::color::mix(t.button_bg, t.text, t.outline_strength);

        out.push_str(&format!(
            "{{\"name\":{},\"is_light\":{},\"scrollbar\":{},\"dock_chrome\":{},\"spec\":{},\"colors\":{{\"core\":{},\"panel\":{},\"derived\":{},\"sidebar\":{},\"sidepanel\":{},\"tab\":{},\"title_bar\":{},\"splitter\":{},\"overlay\":{}}},\"geometry\":{},\"outline_mix\":{}}}",
            quoted(spec.name),
            t.is_light,
            quoted(scrollbar),
            quoted(chrome),
            spec_json,
            core,
            panel,
            derived,
            sidebar,
            sidepanel,
            tab,
            title_bar,
            splitter,
            overlay,
            geometry,
            quoted(&hx(outline_mix)),
        ));
    }
    out.push_str("]}");
    println!("{}", out);
}
