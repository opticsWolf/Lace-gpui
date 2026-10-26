//! Declarative theme specs and the derivation pipeline.
//!
//! Rust port of Lace's `lace/dock_theme.py` build path (`ThemeSpec`,
//! `_DEPTH`, `_build_theme`), flattened from Lace's per-category dicts into
//! one [`Tokens`] struct. Colour math is [`crate::color`]; rule enforcement
//! is [`crate::contrast`]. [`build`] = derive + enforce, exactly like Lace's
//! `build_theme`.

use crate::color::{self, Rgba};
use crate::contrast::enforce;

/// WCAG floor level for text and UI tokens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Contrast {
    Low,
    Normal,
    High,
}

impl Contrast {
    pub fn as_str(self) -> &'static str {
        match self {
            Contrast::Low => "low",
            Contrast::Normal => "normal",
            Contrast::High => "high",
        }
    }
    /// Index into the per-level target tables.
    pub fn index(self) -> usize {
        match self {
            Contrast::Low => 0,
            Contrast::Normal => 1,
            Contrast::High => 2,
        }
    }
}

/// How far derived surfaces step off each other.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Depth {
    Flat,
    Subtle,
    Raised,
}

impl Depth {
    pub fn as_str(self) -> &'static str {
        match self {
            Depth::Flat => "flat",
            Depth::Subtle => "subtle",
            Depth::Raised => "raised",
        }
    }
    pub fn index(self) -> usize {
        match self {
            Depth::Flat => 0,
            Depth::Subtle => 1,
            Depth::Raised => 2,
        }
    }
}

/// Selected items: `Solid` accent fill, or `Tint`, an accent wash over the
/// input background that keeps the normal text colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Selection {
    Solid,
    Tint,
}

/// Scrollbar look, mirroring Lace's `scrollbar` knob ("thin" | "expanding" |
/// "fusion") — bridged onto `gpui_kit::base::ScrollbarMode` by [`crate::skin`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scrollbar {
    Thin,
    Expanding,
    Fusion,
}

/// Whether a theme is light. `Auto` decides from the base colour's OKLCH
/// lightness (`is_dark`), exactly like `is_light=None` in Lace.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rel {
    Auto,
    Light,
    Dark,
}

/// Title-bar direction relative to the panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TitleMode {
    Darker,
    Lighter,
}

/// Hover direction relative to its container. "darker" hovers are the
/// quieter of the two (their step is scaled by 0.7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HoverMode {
    Darker,
    Lighter,
}

/// OKLCH lightness steps per derived surface, by depth (flat, subtle, raised).
///
/// Port of Lace's `_DEPTH`. "subtle" is calibrated to the median step Lace
/// 0.7.6 produced over every preset, so the default look is kept while the
/// per-theme spread goes away.
pub mod depth_steps {
    /// Off base, when no surface is given.
    pub const PANEL: [f64; 3] = [0.03, 0.05, 0.08];
    /// Neutral border off the panel/base.
    pub const BORDER: [f64; 3] = [0.04, 0.06, 0.09];
    /// Off panel, sign from title_mode.
    pub const TITLE: [f64; 3] = [0.03, 0.05, 0.07];
    /// Off panel.
    pub const TOOLTIP: [f64; 3] = [0.05, 0.08, 0.11];
    /// Toward contrast; x0.7 for hover_mode="darker".
    pub const HOVER: [f64; 3] = [0.06, 0.09, 0.12];
    /// Recessed off panel.
    pub const INPUT: [f64; 3] = [0.02, 0.035, 0.05];
    /// Zebra rows off input.
    pub const ALT: [f64; 3] = [0.03, 0.05, 0.07];
    /// Off panel.
    pub const BUTTON: [f64; 3] = [0.04, 0.07, 0.10];
    /// Fusion bevel highlight.
    pub const LIGHT: [f64; 3] = [0.09, 0.13, 0.17];
    /// Fusion mid tone.
    pub const MID: [f64; 3] = [0.03, 0.045, 0.06];
    /// Fusion shadow edge.
    pub const DARK: [f64; 3] = [0.08, 0.11, 0.14];
}

/// Declarative 3- to 5-colour theme input for [`build`].
///
/// Subset of Lace's `ThemeSpec` that survives the trip to gpui-kit: the five
/// seeds, the optional overrides, the three levels, and the geometry gpui
/// widgets actually consume. All fields are const-constructible so presets
/// can live in `const` items ([`ThemeSpec::DEFAULT`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThemeSpec {
    pub name: &'static str,
    /// Darkest background (canvas).
    pub base: Rgba,
    /// Primary accent / highlight.
    pub accent: Rgba,
    /// Primary text.
    pub text: Rgba,
    /// Optional inner content-area background (the panel).
    pub surface: Option<Rgba>,
    /// Optional structural border / divider.
    pub border: Option<Rgba>,
    /// Optional focus outline; defaults from `border` or the accent.
    pub focus_border: Option<Rgba>,
    /// Explicit tab/title-bar background. Overrides the derived value (a
    /// fixed step off the panel). Same role for the header that `surface`
    /// plays for the panel.
    pub title_bg: Option<Rgba>,
    pub tooltip_bg: Option<Rgba>,
    pub tooltip_text: Option<Rgba>,
    pub success: Option<Rgba>,
    pub warning: Option<Rgba>,
    pub error: Option<Rgba>,
    pub info: Option<Rgba>,
    /// Light/dark; `Auto` decides from the base colour.
    pub is_light: Rel,
    /// WCAG floor for text and UI tokens. Colours that already pass are left
    /// alone.
    pub contrast: Contrast,
    /// How far derived surfaces step off each other.
    pub depth: Depth,
    /// Selected items: solid accent fill, or an accent wash.
    pub selection: Selection,
    /// Scrollbar look.
    pub scrollbar: Scrollbar,
    pub title_mode: TitleMode,
    pub hover_mode: HoverMode,
    /// Derived colours keep the theme's hue: the active text (tabs, title
    /// bars, sidebar) stops short of pure white or black. A palette may opt
    /// out (midnight keeps white).
    pub keep_tint: bool,
    /// Corner radius of every control; 0 = square.
    pub control_radius: f64,
    /// Dock-area card corner radius.
    pub corner_radius: f64,
    /// Tab corner radius.
    pub tab_radius: f64,
    /// Dock-area outline width.
    pub border_width: f64,
    /// Share of text colour mixed over a control's fill for its outline.
    pub outline_strength: f64,
    /// Pen width of the keyboard focus ring; 0 hides it.
    pub focus_width: f64,
    /// Title-bar height.
    pub title_height: f64,
}

/// Lace's `ThemeSpec` defaults, one for one — the const twin of `Default`
/// so presets can be built with struct-update syntax in `const` items.
impl ThemeSpec {
    pub const DEFAULT: ThemeSpec = ThemeSpec {
    name: "",
    base: [28, 26, 29, 255],
    accent: [0, 120, 212, 255],
    text: [202, 204, 207, 255],
    surface: None,
    border: None,
    focus_border: None,
    title_bg: None,
    tooltip_bg: None,
    tooltip_text: None,
    success: None,
    warning: None,
    error: None,
    info: None,
    is_light: Rel::Auto,
    contrast: Contrast::Normal,
    depth: Depth::Subtle,
    selection: Selection::Solid,
    scrollbar: Scrollbar::Expanding,
    title_mode: TitleMode::Darker,
    hover_mode: HoverMode::Lighter,
    keep_tint: false,
    control_radius: 4.0,
    corner_radius: 2.0,
    tab_radius: 0.0,
    border_width: 0.0,
    outline_strength: 0.22,
    focus_width: 2.0,
    title_height: 30.0,
    };
}

/// Convenience constructor: five hex seeds and the stock chassis.
///
/// ```
/// use lace_gpui::engine::{spec_from_hex, build};
///
/// let spec = spec_from_hex("mine", "#1e1e2e", "#f5c2e7", "#cdd6f4", None, None);
/// let tokens = build(&spec);
/// assert!(!tokens.is_light);
/// ```
pub fn spec_from_hex(
    name: &'static str,
    base: &str,
    accent: &str,
    text: &str,
    surface: Option<&str>,
    border: Option<&str>,
) -> ThemeSpec {
    ThemeSpec {
        name,
        base: color::hex(base),
        accent: color::hex(accent),
        text: color::hex(text),
        surface: surface.map(color::hex),
        border: border.map(color::hex),
        ..ThemeSpec::DEFAULT
    }
}

// ---------------------------------------------------------------------------
// Tokens
// ---------------------------------------------------------------------------

/// The complete derived theme, flattened from Lace's per-category style dicts
/// into one struct. Field names follow the Lace token they came from; where
/// several categories shared one colour list in Lace (the title background is
/// both `TAB.bg_normal` and `TITLE_BAR.bg_normal`), this struct keeps a
/// canonical field and the mirrors are resynced by [`crate::contrast::Ref`]
/// whenever a rule writes to it.
#[derive(Clone, Debug, PartialEq)]
pub struct Tokens {
    // --- CORE ---
    pub canvas_bg: Rgba,
    pub border_color: Rgba,
    pub accent_color: Rgba,
    pub focus_border_color: Rgba,
    pub text: Rgba,
    pub disabled_text: Rgba,
    pub success: Rgba,
    pub warning: Rgba,
    pub error: Rgba,
    pub info: Rgba,
    pub tooltip_bg: Rgba,
    pub tooltip_text: Rgba,

    // --- PANEL ---
    pub panel_bg: Rgba,
    pub input_bg: Rgba,
    pub alternate_base: Rgba,
    pub button_bg: Rgba,
    pub color_light: Rgba,
    pub color_mid: Rgba,
    pub color_dark: Rgba,
    pub color_shadow: Rgba,
    pub highlight: Rgba,
    pub highlighted_text: Rgba,

    // --- derived text/accent intermediates (shared lists in Lace) ---
    pub text_muted: Rgba,
    pub text_active: Rgba,
    pub btn_disabled: Rgba,
    pub accent_bright: Rgba,
    pub accent_dim: Rgba,
    pub hover: Rgba,
    pub hover_end: Rgba,
    pub btn_hover_title: Rgba,
    pub btn_hover_panel: Rgba,

    // --- SIDEBAR (auto-hide strip; bg mirrors the canvas in Lace) ---
    pub sidebar_bg: Rgba,
    pub sidebar_tab_bg_normal: Rgba,
    pub sidebar_tab_bg_hover_start: Rgba,
    pub sidebar_tab_bg_hover_end: Rgba,
    pub sidebar_tab_bg_active: Rgba,
    pub sidebar_tab_border_normal: Rgba,
    pub sidebar_tab_border_active: Rgba,
    pub sidebar_tab_text_normal: Rgba,
    pub sidebar_tab_text_active: Rgba,
    pub sidebar_tab_text_disabled: Rgba,
    pub sidebar_indicator: Rgba,
    pub sidebar_badge_bg: Rgba,
    pub sidebar_badge_text: Rgba,

    // --- SIDEPANEL ---
    pub sidepanel_bg: Rgba,
    pub sidepanel_title_text: Rgba,
    pub sidepanel_button_color: Rgba,
    pub sidepanel_button_disabled: Rgba,
    pub sidepanel_button_hover: Rgba,
    pub sidepanel_shadow: Rgba,
    pub sidepanel_border: Rgba,
    pub sidepanel_focus_border: Rgba,

    // --- TAB (dock widget tabs; bg_normal IS the title strip) ---
    pub tab_bg_normal: Rgba,
    pub tab_bg_hover: Rgba,
    pub tab_bg_active: Rgba,
    pub tab_border_normal: Rgba,
    pub tab_border_active: Rgba,
    pub tab_text_normal: Rgba,
    pub tab_text_active: Rgba,
    pub tab_indicator: Rgba,
    pub tab_close_btn: Rgba,
    pub tab_close_btn_hover: Rgba,
    pub tab_close_btn_disabled: Rgba,

    // --- TITLE_BAR ---
    pub title_border_color: Rgba,
    pub title_focus_border: Rgba,
    pub title_text_normal: Rgba,
    pub title_text_active: Rgba,
    pub title_active_edge: Rgba,
    pub title_button_color: Rgba,
    pub title_button_disabled: Rgba,
    pub title_button_hover: Rgba,

    // --- SPLITTER ---
    pub splitter_handle: Rgba,
    pub splitter_handle_hover: Rgba,

    // --- OVERLAY ---
    pub overlay_frame: Rgba,
    pub overlay_background: Rgba,
    pub overlay_color: Rgba,
    pub overlay_arrow: Rgba,
    pub overlay_shadow: Rgba,

    // --- geometry / behaviour (passthrough to gpui) ---
    pub is_light: bool,
    pub scrollbar: Scrollbar,
    pub control_radius: f64,
    pub corner_radius: f64,
    pub tab_radius: f64,
    pub border_width: f64,
    pub outline_strength: f64,
    pub focus_width: f64,
    pub title_height: f64,
}

impl Tokens {
    /// OKLCH lightness fingerprint of every colour token — two themes with
    /// the same fingerprint are the same theme to the eye.
    pub fn fingerprint(&self) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        let mut feed = |bytes: &[u8]| {
            for &b in bytes {
                h ^= b as u64;
                h = h.wrapping_mul(0x100000001b3);
            }
        };
        for c in [
            self.canvas_bg, self.border_color, self.accent_color, self.focus_border_color,
            self.text, self.disabled_text, self.success, self.warning, self.error, self.info,
            self.tooltip_bg, self.tooltip_text, self.panel_bg, self.input_bg,
            self.alternate_base, self.button_bg, self.highlight, self.highlighted_text,
            self.text_muted, self.text_active, self.tab_bg_normal, self.tab_bg_hover,
            self.tab_bg_active, self.sidebar_bg, self.sidepanel_bg,
        ] {
            feed(&c);
        }
        h
    }
}

/// Which colour tokens came from the spec itself. These may only move within
/// `EXPLICIT_MAX_DE` of their original value however many rules touch them,
/// and only when that improves the rule they miss; derived colours move as
/// far as they need to.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Explicit {
    pub canvas_bg: bool,
    pub accent_color: bool,
    pub text: bool,
    pub panel_bg: bool,
    pub border_color: bool,
    pub focus_border_color: bool,
    pub tab_bg_normal: bool,
    pub tooltip_bg: bool,
    pub tooltip_text: bool,
    pub success: bool,
    pub warning: bool,
    pub error: bool,
    pub info: bool,
}

/// Derive every token from the spec's 3–5 seeds, without enforcement.
///
/// Port of Lace's `_build_theme` derivation section; see [`build`] for the
/// full pipeline (this + [`enforce`]).
pub fn build_unenforced(spec: &ThemeSpec) -> (Tokens, Explicit) {
    let base = spec.base;
    let accent = spec.accent;
    let text = spec.text;

    let is_light = match spec.is_light {
        Rel::Light => true,
        Rel::Dark => false,
        Rel::Auto => !color::is_dark(base),
    };
    // Direction multiplier: light themes darken, dark themes lighten.
    let d = if is_light { -1.0 } else { 1.0 };
    // Title bar direction, independent of the theme's direction.
    let t_mode = match spec.title_mode {
        TitleMode::Darker => -1.0,
        TitleMode::Lighter => 1.0,
    };
    // "darker" hovers are the quieter of the two.
    let hover_scale = match spec.hover_mode {
        HoverMode::Darker => 0.7,
        HoverMode::Lighter => 1.0,
    };
    let level = spec.depth.index();
    let dl = |steps: &[f64; 3]| steps[level];

    // === DERIVED BACKGROUNDS (OKLCH lightness steps, see depth_steps) ===
    let panel = match spec.surface {
        Some(s) => s,
        None => color::step(base, d * dl(&depth_steps::PANEL)),
    };

    // Neutral border derived from surface or base depending on light/dark.
    let ref_col = if spec.surface.is_some() { panel } else { base };
    let neutral_border = match spec.border {
        Some(b) => b,
        None => color::step(ref_col, d * dl(&depth_steps::BORDER)),
    };
    let focus_border = match spec.focus_border {
        Some(f) => f,
        None => match spec.border {
            Some(b) => b,
            None => color::step(accent, d * 0.07),
        },
    };

    // Title bar / header background: a step darker or lighter than the
    // panel, by title_mode.
    let title_bg = match spec.title_bg {
        Some(t) => t,
        None => color::step(panel, t_mode * dl(&depth_steps::TITLE)),
    };

    // Tooltip surface: a clearly-distinct step off the panel; text defaults
    // to the full-strength seed text colour.
    let tooltip_bg = match spec.tooltip_bg {
        Some(t) => t,
        None => color::step(panel, d * dl(&depth_steps::TOOLTIP)),
    };
    let tooltip_text = spec.tooltip_text.unwrap_or(text);

    // Interactive hovers: always step in the direction of high contrast.
    let hover_dl = dl(&depth_steps::HOVER) * hover_scale;
    let hover = color::step_contrast(base, hover_dl);
    let hover_end = color::step_contrast(base, (0.03_f64).max(hover_dl * 0.65));

    // Button hover fill, resolved relative to the container it sits on.
    let btn_hover_title = color::step_contrast(title_bg, hover_dl);
    let btn_hover_panel = color::step_contrast(panel, hover_dl);

    // Input widget backgrounds: recessed.
    let input_bg = color::step(panel, -d * dl(&depth_steps::INPUT));
    let mut alternate_base = color::step(input_bg, d * dl(&depth_steps::ALT));
    if spec.keep_tint {
        let (_, c_in, _) = color::to_oklch(input_bg);
        let (_, c_pan, _) = color::to_oklch(panel);
        if c_in < 0.002 && c_pan >= 0.002 {
            // The input clipped to white and lost the panel's hue, so the
            // rows striped off it would be plain grey; give them the
            // panel's tint.
            let (l_a, _, _) = color::to_oklch(alternate_base);
            let (_, c_p, h_p) = color::to_oklch(panel);
            alternate_base = color::from_oklch_a(l_a, c_p, h_p, alternate_base[3]);
        }
    }

    // Button face background.
    let button_bg = color::step(panel, d * dl(&depth_steps::BUTTON));

    // 3D structural colors (widget borders, scrollbars, frames).
    let color_light = color::step(panel, d * dl(&depth_steps::LIGHT));
    let color_mid = color::step(panel, -d * dl(&depth_steps::MID));
    let color_dark = color::step(panel, -d * dl(&depth_steps::DARK));
    let color_shadow = [0, 0, 0, if is_light { 48 } else { 72 }];

    // === DERIVED TEXT (toward the background, or away for "active") ===
    let text_muted = color::step(text, -d * 0.09);
    let text_disabled = color::step(text, -d * 0.26);
    let mut text_active = color::step(text, d * 0.15);
    // Selected text on a solid accent wants the full-strength step.
    let text_on_accent = text_active;
    if spec.keep_tint {
        // Stop short of pure white / black, which have no hue to keep.
        let (l_t, c_t, h_t) = color::to_oklch(text);
        let target = (l_t + d * 0.15).clamp(0.08, 0.965);
        text_active = color::from_oklch_a(target, c_t, h_t, text[3]);
    }

    // === BUTTON DISABLED: a glyph colour between base and text ===
    let btn_disabled = color::step(base, d * 0.20);

    // === ACCENT VARIANTS ===
    // The bright accent steps toward contrast with the canvas, so on a light
    // theme it darkens and stays visible instead of washing out.
    let accent_bright = color::step(accent, d * 0.07);
    let accent_dim = [accent[0], accent[1], accent[2],
        ((accent[3] as f64) * 0.25).round() as u8];

    // === SELECTION ===
    let (highlight, highlighted_text) = match spec.selection {
        Selection::Solid => (accent, color::on_color(accent, [text_on_accent, base], 4.5)),
        Selection::Tint => {
            let tint = [accent[0], accent[1], accent[2], 72];
            (color::composite(tint, input_bg), text)
        }
    };

    // === STATUS COLORS ===
    // Defaults branch on is_light exactly like Lace's _build_theme.
    let success = spec
        .success
        .unwrap_or(if is_light { [34, 134, 58, 255] } else { [78, 201, 112, 255] });
    let warning = spec
        .warning
        .unwrap_or(if is_light { [179, 134, 0, 255] } else { [230, 167, 0, 255] });
    let error = spec
        .error
        .unwrap_or(if is_light { [203, 36, 49, 255] } else { [241, 76, 76, 255] });
    let info = spec
        .info
        .unwrap_or(if is_light { [0, 102, 214, 255] } else { [55, 148, 255, 255] });

    let shadow = [0, 0, 0, if is_light { 32 } else { 64 }];

    let mut explicit = Explicit {
        canvas_bg: true,
        accent_color: true,
        text: true,
        panel_bg: spec.surface.is_some(),
        border_color: spec.border.is_some(),
        focus_border_color: spec.focus_border.is_some(),
        tab_bg_normal: spec.title_bg.is_some(),
        tooltip_bg: spec.tooltip_bg.is_some(),
        tooltip_text: spec.tooltip_text.is_some(),
        success: spec.success.is_some(),
        warning: spec.warning.is_some(),
        error: spec.error.is_some(),
        info: spec.info.is_some(),
    };
    // An explicit surface seeds every surface that mirrors the panel.
    if spec.surface.is_some() {
        explicit.panel_bg = true;
    }

    let tokens = Tokens {
        // CORE
        canvas_bg: base,
        border_color: neutral_border,
        accent_color: accent,
        focus_border_color: focus_border,
        text,
        disabled_text: text_disabled,
        success,
        warning,
        error,
        info,
        tooltip_bg,
        tooltip_text,
        // PANEL
        panel_bg: panel,
        input_bg,
        alternate_base,
        button_bg,
        color_light,
        color_mid,
        color_dark,
        color_shadow,
        highlight,
        highlighted_text,
        // derived intermediates
        text_muted,
        text_active,
        btn_disabled,
        accent_bright,
        accent_dim,
        hover,
        hover_end,
        btn_hover_title,
        btn_hover_panel,
        // SIDEBAR
        sidebar_bg: base,
        sidebar_tab_bg_normal: [0, 0, 0, 0],
        sidebar_tab_bg_hover_start: hover,
        sidebar_tab_bg_hover_end: hover_end,
        sidebar_tab_bg_active: panel,
        sidebar_tab_border_normal: neutral_border,
        sidebar_tab_border_active: accent,
        sidebar_tab_text_normal: text_muted,
        sidebar_tab_text_active: text_active,
        sidebar_tab_text_disabled: text_disabled,
        sidebar_indicator: accent,
        sidebar_badge_bg: accent,
        sidebar_badge_text: text_muted,
        // SIDEPANEL
        sidepanel_bg: panel,
        sidepanel_title_text: text,
        sidepanel_button_color: text_muted,
        sidepanel_button_disabled: btn_disabled,
        sidepanel_button_hover: btn_hover_panel,
        sidepanel_shadow: shadow,
        sidepanel_border: neutral_border,
        sidepanel_focus_border: focus_border,
        // TAB
        tab_bg_normal: title_bg,
        tab_bg_hover: hover,
        tab_bg_active: panel,
        tab_border_normal: neutral_border,
        tab_border_active: accent,
        tab_text_normal: text_muted,
        tab_text_active: text_active,
        tab_indicator: accent,
        tab_close_btn: blend30(text_active, panel),
        tab_close_btn_hover: btn_hover_panel,
        tab_close_btn_disabled: btn_disabled,
        // TITLE_BAR
        title_border_color: neutral_border,
        title_focus_border: focus_border,
        title_text_normal: text_muted,
        title_text_active: text_active,
        title_active_edge: accent_bright,
        title_button_color: text_muted,
        title_button_disabled: btn_disabled,
        title_button_hover: btn_hover_title,
        // SPLITTER
        splitter_handle: base,
        splitter_handle_hover: accent,
        // OVERLAY
        overlay_frame: accent_bright,
        overlay_background: panel,
        overlay_color: accent_dim,
        overlay_arrow: text,
        overlay_shadow: shadow,
        // geometry
        is_light,
        scrollbar: spec.scrollbar,
        control_radius: spec.control_radius,
        corner_radius: spec.corner_radius,
        tab_radius: spec.tab_radius,
        border_width: spec.border_width,
        outline_strength: spec.outline_strength,
        focus_width: spec.focus_width,
        title_height: spec.title_height,
    };
    (tokens, explicit)
}

/// Blend `fg` 70/30 into `bg`, per channel — the tab close glyph colour,
/// harmonized with its container.
fn blend30(fg: Rgba, bg: Rgba) -> Rgba {
    let f = 0.30;
    let mix = |a: u8, b: u8| -> u8 { (a as f64 * (1.0 - f) + b as f64 * f).round() as u8 };
    [
        mix(fg[0], bg[0]),
        mix(fg[1], bg[1]),
        mix(fg[2], bg[2]),
        255,
    ]
}

/// Build a complete theme: derive, then lift whatever still misses its floor.
pub fn build(spec: &ThemeSpec) -> Tokens {
    let (mut tokens, explicit) = build_unenforced(spec);
    enforce(&mut tokens, spec, &explicit);
    tokens
}
