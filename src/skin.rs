//! The bridge onto gpui-kit: [`apply`] paints a running app with Lace's
//! tokens, [`apply_base`] adds
//! [`BaseStyle`](crate::base_style::BaseStyle) — LaceStyle's flat widget
//! rules — on top.
//!
//! Everything in here is feature-gated (`gpui-kit`); the colour core and the
//! engine never pull a dependency. The mapping starts from gpui-kit's own
//! `ThemeColor` — fields Lace doesn't model keep whatever value the
//! initialized theme already had — so a Lace theme over gpui-kit reads as
//! Lace's palette wearing gpui's widgets.

use gpui_kit::component::dock::PanelStyle;
use gpui_kit::component::theme::{Theme, ThemeTokens};
use gpui_kit::base::ScrollbarMode;
use gpui_kit::gpui::{px, rgba, App, Hsla};

use crate::color::{self, Rgba as Lace};
use crate::engine::{Scrollbar, ThemeSpec, Tokens, depth_steps};

/// sRGB `[r, g, b, a]` -> gpui `Hsla`.
pub fn hsla(c: Lace) -> Hsla {
    Hsla::from(rgba(u32::from_be_bytes(c)))
}

/// A hover/active shade of `c`, one button step off it toward contrast.
fn shade(c: Lace) -> Lace {
    color::step_contrast(c, depth_steps::BUTTON[1])
}

/// Readable text on a status fill.
fn on_status(c: Lace) -> Lace {
    color::on_color(c, [color::WHITE, color::BLACK], 4.5)
}

/// Map every [`Tokens`] colour gpui-kit can name onto `colors`. Fields the
/// engine doesn't model are left as they are.
pub fn apply_to_theme_color(colors: &mut gpui_kit::component::theme::ThemeColor, t: &Tokens) {
    // --- core surfaces ---
    colors.background = hsla(t.canvas_bg);
    colors.foreground = hsla(t.text);
    colors.border = hsla(t.border_color);
    colors.window_border = hsla(t.border_color);
    colors.accent = hsla(t.accent_color);
    colors.accent_foreground = hsla(t.highlighted_text);
    colors.ring = hsla(t.focus_border_color);
    colors.caret = hsla(t.text);
    colors.drag_border = hsla(t.focus_border_color);
    colors.overlay = hsla(t.overlay_background);
    colors.drop_target = hsla(t.highlight);

    // --- text ---
    colors.muted_foreground = hsla(t.text_muted);

    // --- fills ---
    colors.secondary = hsla(t.button_bg);
    colors.secondary_hover = hsla(t.btn_hover_panel);
    colors.secondary_active = hsla(t.button_bg);
    colors.secondary_foreground = hsla(t.text);
    colors.muted = hsla(t.color_mid);
    colors.skeleton = hsla(t.color_mid);
    colors.progress_bar = hsla(t.accent_color);

    // --- primary (the accent button) ---
    colors.primary = hsla(t.accent_color);
    colors.primary_hover = hsla(t.accent_bright);
    colors.primary_active = hsla(t.accent_bright);
    colors.primary_foreground = hsla(t.highlighted_text);
    colors.button_primary = hsla(t.accent_color);
    colors.button_primary_hover = hsla(t.accent_bright);
    colors.button_primary_active = hsla(t.accent_bright);
    colors.button_primary_foreground = hsla(t.highlighted_text);

    // --- secondary button ---
    colors.button = hsla(t.button_bg);
    colors.button_hover = hsla(t.btn_hover_panel);
    colors.button_active = hsla(t.button_bg);
    colors.button_foreground = hsla(t.text);
    colors.button_secondary = hsla(t.button_bg);
    colors.button_secondary_hover = hsla(t.btn_hover_panel);
    colors.button_secondary_active = hsla(t.button_bg);
    colors.button_secondary_foreground = hsla(t.text);

    // --- status colours and their buttons ---
    let status = [
        (t.error, &mut colors.danger, &mut colors.danger_foreground,
            &mut colors.danger_hover, &mut colors.danger_active),
        (t.warning, &mut colors.warning, &mut colors.warning_foreground,
            &mut colors.warning_hover, &mut colors.warning_active),
        (t.success, &mut colors.success, &mut colors.success_foreground,
            &mut colors.success_hover, &mut colors.success_active),
        (t.info, &mut colors.info, &mut colors.info_foreground,
            &mut colors.info_hover, &mut colors.info_active),
    ];
    for (base, fill, fg, hover, active) in status {
        *fill = hsla(base);
        *fg = hsla(on_status(base));
        *hover = hsla(shade(base));
        *active = hsla(shade(base));
    }
    let buttons = [
        (t.error, &mut colors.button_danger, &mut colors.button_danger_foreground,
            &mut colors.button_danger_hover, &mut colors.button_danger_active),
        (t.warning, &mut colors.button_warning, &mut colors.button_warning_foreground,
            &mut colors.button_warning_hover, &mut colors.button_warning_active),
        (t.success, &mut colors.button_success, &mut colors.button_success_foreground,
            &mut colors.button_success_hover, &mut colors.button_success_active),
        (t.info, &mut colors.button_info, &mut colors.button_info_foreground,
            &mut colors.button_info_hover, &mut colors.button_info_active),
    ];
    for (base, fill, fg, hover, active) in buttons {
        *fill = hsla(base);
        *fg = hsla(on_status(base));
        *hover = hsla(shade(base));
        *active = hsla(shade(base));
    }

    // --- links ---
    colors.link = hsla(t.accent_color);
    colors.link_hover = hsla(t.accent_bright);
    colors.link_active = hsla(t.accent_bright);

    // --- input ---
    colors.input = hsla(t.border_color);
    colors.selection = hsla(t.highlight);

    // --- tabs: Lace's tab strip *is* the title strip ---
    colors.tab_bar = hsla(t.tab_bg_normal);
    colors.tab_bar_segmented = hsla(t.tab_bg_normal);
    colors.tab = hsla(t.tab_bg_normal);
    colors.tab_active = hsla(t.tab_bg_active);
    colors.tab_foreground = hsla(t.text_muted);
    colors.tab_active_foreground = hsla(t.text_active);
    colors.title_bar = hsla(t.tab_bg_normal);
    colors.title_bar_border = hsla(t.title_border_color);
    colors.status_bar = hsla(t.panel_bg);
    colors.status_bar_border = hsla(t.border_color);

    // --- sidebar (auto-hide strip) ---
    colors.sidebar = hsla(t.sidebar_bg);
    colors.sidebar_border = hsla(t.border_color);
    colors.sidebar_foreground = hsla(t.text_muted);
    colors.sidebar_accent = hsla(t.sidebar_tab_bg_hover_start);
    colors.sidebar_accent_foreground = hsla(t.text_active);
    colors.sidebar_primary = hsla(t.panel_bg);
    colors.sidebar_primary_foreground = hsla(t.text_active);

    // --- lists / tables: zebra rows take alternate_base ---
    colors.list = hsla(t.panel_bg);
    colors.list_hover = hsla(t.tab_bg_hover);
    colors.list_active = hsla(t.highlight);
    colors.list_active_border = hsla(t.accent_color);
    colors.list_even = hsla(t.alternate_base);
    colors.list_head = hsla(t.tab_bg_normal);
    colors.table = hsla(t.panel_bg);
    colors.table_hover = hsla(t.tab_bg_hover);
    colors.table_active = hsla(t.highlight);
    colors.table_active_border = hsla(t.accent_color);
    colors.table_even = hsla(t.alternate_base);
    colors.table_head = hsla(t.tab_bg_normal);
    colors.table_head_foreground = hsla(t.text_muted);
    colors.table_foot = hsla(t.tab_bg_normal);
    colors.table_foot_foreground = hsla(t.text_muted);
    colors.table_row_border = hsla(t.border_color);
    colors.description_list_label = hsla(t.tab_bg_normal);
    colors.description_list_label_foreground = hsla(t.text_muted);

    // --- popovers ride Lace's tooltip pair ---
    colors.popover = hsla(t.tooltip_bg);
    colors.popover_foreground = hsla(t.tooltip_text);

    // --- containers / controls ---
    colors.accordion = hsla(t.panel_bg);
    colors.group_box = hsla(t.panel_bg);
    colors.group_box_foreground = hsla(t.text);
    colors.slider_bar = hsla(t.color_mid);
    colors.slider_thumb = hsla(t.accent_color);
    colors.switch = hsla(t.color_mid);
    colors.switch_thumb = hsla(t.button_bg);

    // --- scrollbars ---
    colors.scrollbar = hsla(t.panel_bg);
    colors.scrollbar_thumb = hsla(t.border_color);
    colors.scrollbar_thumb_hover = hsla(t.color_mid);

    // --- charts ---
    colors.chart_1 = hsla(t.accent_color);
    colors.chart_2 = hsla(t.success);
    colors.chart_3 = hsla(t.warning);
    colors.chart_4 = hsla(t.error);
    colors.chart_5 = hsla(t.info);
    colors.chart_bullish = hsla(t.success);
    colors.chart_bearish = hsla(t.error);
    colors.chart_grid = hsla(t.border_color);

    // --- base 16 stand-ins from the theme's own hues ---
    colors.red = hsla(t.error);
    colors.red_light = hsla(shade(t.error));
    colors.green = hsla(t.success);
    colors.green_light = hsla(shade(t.success));
    colors.blue = hsla(t.info);
    colors.blue_light = hsla(shade(t.info));
    colors.yellow = hsla(t.warning);
    colors.yellow_light = hsla(shade(t.warning));
    colors.magenta = hsla(t.accent_color);
    colors.magenta_light = hsla(t.accent_bright);
    colors.cyan = hsla(t.info);
    colors.cyan_light = hsla(shade(t.info));
}

/// The non-colour knobs: radii, scrollbar mode, mode.
pub fn apply_tokens(theme: &mut Theme, t: &Tokens) {
    apply_to_theme_color(&mut theme.colors, t);
    // Keep the legacy resolved tokens in step with the colors we wrote.
    theme.tokens = ThemeTokens::from(&theme.colors);
    theme.radius = px(t.control_radius as f32);
    theme.radius_lg = px(t.corner_radius as f32);
    theme.scrollbar_mode = match t.scrollbar {
        Scrollbar::Thin => ScrollbarMode::Hover,
        Scrollbar::Expanding => ScrollbarMode::Scrolling,
        Scrollbar::Fusion => ScrollbarMode::Always,
    };
    theme.mode = if t.is_light {
        gpui_kit::component::theme::ThemeMode::Light
    } else {
        gpui_kit::component::theme::ThemeMode::Dark
    };
}

/// Which widget rendering sits on a Lace palette.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WidgetStyle {
    /// The palette over gpui-kit's stock widgets.
    #[default]
    Theme,
    /// The palette plus [`BaseStyle`](crate::base_style::BaseStyle):
    /// LaceStyle's flat outlines, state steps, focus ring, scrollbar and
    /// slider mixes.
    LaceBase,
}

impl WidgetStyle {
    /// A short label for a toggle or a status line.
    pub fn label(self) -> &'static str {
        match self {
            Self::Theme => "Theme only",
            Self::LaceBase => "Lace style",
        }
    }

    /// True for [`WidgetStyle::LaceBase`].
    pub fn is_lace_base(self) -> bool {
        matches!(self, Self::LaceBase)
    }
}

/// Paint the running app with the named theme. `false` when the name is not
/// one of the 16 curated presets.
pub fn apply(name: &str, cx: &mut App) -> bool {
    apply_styled(name, WidgetStyle::Theme, cx)
}

/// Paint the running app with the named theme and LaceStyle's flat widget
/// layer. `false` when the name is not one of the 16 curated presets.
pub fn apply_base(name: &str, cx: &mut App) -> bool {
    apply_styled(name, WidgetStyle::LaceBase, cx)
}

/// Paint the running app with the named theme and the chosen widget
/// rendering. `false` when the name is not one of the 16 curated presets.
pub fn apply_styled(name: &str, style: WidgetStyle, cx: &mut App) -> bool {
    match crate::themes::by_name(name) {
        Some(spec) => {
            apply_spec_styled(spec, style, cx);
            true
        }
        None => false,
    }
}

/// Build and apply a [`ThemeSpec`] the caller owns.
pub fn apply_spec(spec: &ThemeSpec, cx: &mut App) {
    apply_spec_styled(spec, WidgetStyle::Theme, cx);
}

/// Build and apply a [`ThemeSpec`] with the chosen widget rendering.
pub fn apply_spec_styled(spec: &ThemeSpec, style: WidgetStyle, cx: &mut App) {
    let tokens = crate::build(spec);
    {
        let theme = Theme::global_mut(cx);
        apply_tokens(theme, &tokens);
        if style == WidgetStyle::LaceBase {
            crate::base_style::BaseStyle::from_spec(spec, &tokens).apply(theme);
        } else {
            // LaceStyle gates the ring on focus_width; gpui-kit's stock
            // default is on.
            theme.focus_ring = true;
        }
    }
    // Colors were edited through `global_mut`, which does not rebuild the
    // Base projection (the stock scrollbar and resize handles) on its own.
    Theme::sync_base(cx);
    cx.refresh_windows();
}

/// Cycle to the next curated theme (Lace's dark-to-light order). Returns the
/// name applied — handy for a button label: whatever name was current before
/// (case/`-`/space-insensitive), the theme after it in [`crate::ALL`] is now
/// on screen.
pub fn apply_next(current: &str, cx: &mut App) -> &'static str {
    apply_next_styled(current, WidgetStyle::Theme, cx)
}

/// [`apply_next`] with a chosen widget rendering.
pub fn apply_next_styled(current: &str, style: WidgetStyle, cx: &mut App) -> &'static str {
    let names = crate::themes::ALL;
    let norm = current.trim().to_ascii_lowercase().replace(['-', ' '], "_");
    let ix = names
        .iter()
        .position(|s| s.name == norm)
        .map_or(0, |i| (i + 1) % names.len());
    let next = names[ix].name;
    apply_styled(next, style, cx);
    next
}

/// The chassis a theme rides, as gpui-kit's panel chrome: the Basics' 4px
/// classic chassis keeps the plain title, the 10px card chassis (neo and
/// cyberpunk_neon) always shows the tab bar.
pub fn dock_chrome(spec: &ThemeSpec) -> PanelStyle {
    if spec.corner_radius >= 10.0 {
        PanelStyle::TabBar
    } else {
        PanelStyle::Auto
    }
}

/// Experimental: the 16 presets as Lace-flavoured JSON, for Lace's Theme
/// Studio or a `ThemeRegistry::load_themes_from_str` round trip. Built by
/// hand — the core has no serde dependency.
pub fn theme_set_json() -> String {
    let mut out = String::from("{\"version\":\"0.9.7\",\"themes\":[");
    for (i, spec) in crate::themes::ALL.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        let seed = |c: Option<Lace>| -> String {
            match c {
                Some(c) => color::hex_str(c),
                None => String::from("null"),
            }
        };
        out.push_str(&format!(
            "{{\"name\":\"{}\",\"base\":\"{}\",\"accent\":\"{}\",\"text\":\"{}\",\
             \"surface\":{},\"border\":{},\"is_light\":{},\"contrast\":\"{}\",\
             \"depth\":\"{}\",\"selection\":\"{}\",\"keep_tint\":{},\
             \"corner_radius\":{},\"tab_radius\":{}}}",
            spec.name,
            color::hex_str(spec.base),
            color::hex_str(spec.accent),
            color::hex_str(spec.text),
            seed(spec.surface),
            seed(spec.border),
            spec.is_light == crate::engine::Rel::Light,
            spec.contrast.as_str(),
            spec.depth.as_str(),
            match spec.selection {
                crate::engine::Selection::Solid => "solid",
                crate::engine::Selection::Tint => "tint",
            },
            spec.keep_tint,
            spec.corner_radius,
            spec.tab_radius,
        ));
    }
    out.push_str("]}");
    out
}

/// The scrollbar mapping, exposed for tests and callers that manage
/// `ScrollbarMode` themselves.
pub fn scrollbar_mode(s: Scrollbar) -> ScrollbarMode {
    match s {
        Scrollbar::Thin => ScrollbarMode::Hover,
        Scrollbar::Expanding => ScrollbarMode::Scrolling,
        Scrollbar::Fusion => ScrollbarMode::Always,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::themes::by_name;

    #[test]
    fn hsla_extremes_hold() {
        let w = hsla([255, 255, 255, 255]);
        assert!((w.l - 1.0).abs() < 1e-4, "white lightness {}", w.l);
        assert!(w.a > 0.99, "white alpha {}", w.a);
        let b = hsla([0, 0, 0, 255]);
        assert!(b.l.abs() < 1e-4, "black lightness {}", b.l);
        let accent = hsla([255, 0, 127, 255]);
        assert!(accent.s > 0.9, "neon pink saturation {}", accent.s);
    }

    #[test]
    fn scrollbar_mapping() {
        assert!(matches!(
            scrollbar_mode(Scrollbar::Expanding),
            ScrollbarMode::Scrolling
        ));
        assert!(matches!(scrollbar_mode(Scrollbar::Fusion), ScrollbarMode::Always));
    }

    #[test]
    fn chrome_follows_geometry() {
        assert_eq!(dock_chrome(by_name("cyberpunk_neon").unwrap()), PanelStyle::TabBar);
        assert_eq!(dock_chrome(by_name("midnight").unwrap()), PanelStyle::Auto);
    }

    #[test]
    fn theme_set_json_lists_all() {
        let json = theme_set_json();
        for spec in crate::themes::ALL {
            assert!(json.contains(&format!("\"name\":\"{}\"", spec.name)));
        }
    }
}
