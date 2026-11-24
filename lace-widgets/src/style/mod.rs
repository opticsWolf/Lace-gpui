//! LaceStyle for gpui — the palette, the tokens and the paint rules.
//!
//! Lace's `LaceStyle` is a `QProxyStyle` that paints every control from the
//! widget's `QPalette` plus a handful of tokens (radius, focus width, outline
//! strength, contrast level). This module carries the same two inputs into
//! gpui:
//!
//! - [`Palette`] — the QPalette roles LaceStyle reads, for the active and the
//!   disabled colour group, built from a Lace theme exactly as
//!   `lace.dock_theme.build_dock_palette` builds them.
//! - [`LaceStyle`] — that palette plus the tokens, installed as a gpui
//!   [`Global`] so every widget in [`crate::controls`] paints from one place.
//!
//! The colour decisions themselves live in [`paint`] (the port of
//! `lace/style/_paint.py`) and [`rules`] (one function per control family,
//! the port of `_buttons`, `_inputs`, `_primitives`, `_range`, `_containers`
//! and `_chrome`). Both are pure: they take a [`LaceStyle`] and a [`State`],
//! return colours, and need no window — the unit tests run them headless.

pub mod paint;
pub mod rules;
pub mod tones;

use gpui_kit::component::theme::Theme;
use gpui_kit::gpui::{
    AnyElement, App, Bounds, Element, ElementId, Global, GlobalElementId, Hsla, InspectorElementId, IntoElement,
    LayoutId, Pixels, Window,
};
use lace_gpui::color::{self, Rgba};
use lace_gpui::contrast::{contrast_target, Role};
use lace_gpui::engine::{Contrast, Scrollbar, ThemeSpec, Tokens};

pub use paint::State;

/// The QPalette roles LaceStyle paints from, for one colour group.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    /// `Window`: the surface controls sit on.
    pub window: Rgba,
    /// `WindowText`: labels on the window.
    pub window_text: Rgba,
    /// `Base`: field and item-view fill.
    pub base: Rgba,
    /// `AlternateBase`: alternating item-view rows.
    pub alternate_base: Rgba,
    /// `Text`: text on `Base`, and the ink every stroke is mixed from.
    pub text: Rgba,
    /// `Button`: raised faces (buttons, header sections, slider knobs).
    pub button: Rgba,
    /// `ButtonText`: labels and glyphs on `Button`.
    pub button_text: Rgba,
    /// `Highlight`: item-view and menu selection.
    pub highlight: Rgba,
    /// `HighlightedText`: text on `Highlight`.
    pub highlighted_text: Rgba,
    /// `Accent`: checked, focused and filled state.
    pub accent: Rgba,
    /// `PlaceholderText`: empty-field hints.
    pub placeholder: Rgba,
    /// `ToolTipBase`.
    pub tooltip_base: Rgba,
    /// `ToolTipText`.
    pub tooltip_text: Rgba,
}

impl Palette {
    /// The active group, as `build_dock_palette(is_panel)` sets it.
    pub fn active(t: &Tokens, is_panel: bool) -> Self {
        let window = if is_panel { t.panel_bg } else { t.canvas_bg };
        let mut placeholder = t.text;
        placeholder[3] /= 2;
        Self {
            window,
            window_text: t.text,
            base: t.input_bg,
            alternate_base: t.alternate_base,
            text: t.text,
            button: t.button_bg,
            button_text: t.text,
            highlight: t.highlight,
            highlighted_text: t.highlighted_text,
            accent: t.accent_color,
            placeholder,
            tooltip_base: t.tooltip_bg,
            tooltip_text: t.tooltip_text,
        }
    }

    /// The disabled group (`_apply_state_groups`): fills fade halfway toward
    /// the window, selection and accent 60%, and every text role is the
    /// theme's disabled text flattened onto its surface and lifted to the
    /// `disabled` floor there.
    pub fn disabled(t: &Tokens, active: &Palette) -> Self {
        let win = active.window;
        let target = contrast_target(Role::Disabled, Contrast::Normal);
        let base = color::mix(active.base, win, 0.5);
        let button = color::mix(active.button, win, 0.5);
        let highlight = color::mix(active.highlight, win, 0.6);
        let legible = |surface: Rgba| {
            let flat = color::composite(t.disabled_text, surface);
            color::ensure_contrast(flat, surface, target)
        };
        Self {
            window: win,
            window_text: legible(win),
            base,
            alternate_base: color::mix(active.alternate_base, win, 0.5),
            text: legible(base),
            button,
            button_text: legible(button),
            highlight,
            highlighted_text: legible(highlight),
            accent: color::mix(active.accent, win, 0.6),
            placeholder: legible(base),
            tooltip_base: active.tooltip_base,
            tooltip_text: active.tooltip_text,
        }
    }
}

/// The theme's status colours (Lace's `success_color` .. `info_color`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StatusColors {
    /// Success / done.
    pub success: Rgba,
    /// Warning.
    pub warning: Rgba,
    /// Error / danger.
    pub error: Rgba,
    /// Information.
    pub info: Rgba,
}

/// How scrollbars rest and grow (`LaceStyle(scrollbar=...)`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScrollbarMode {
    /// 8 px, no step buttons.
    Thin,
    /// 12 px reserved; a 4 px handle at rest that grows, with step arrows, on hover.
    #[default]
    Expanding,
}

impl ScrollbarMode {
    /// `PM_ScrollBarExtent` for the mode.
    pub fn extent(self) -> f32 {
        match self {
            Self::Thin => 8.0,
            Self::Expanding => 12.0,
        }
    }
}

/// LaceStyle: the palette, the tokens, and the ratios the contrast level sets.
#[derive(Clone, Debug, PartialEq)]
pub struct LaceStyle {
    /// The theme's name, for display.
    pub name: &'static str,
    /// Whether the theme is light.
    pub is_light: bool,
    /// The active colour group.
    pub active: Palette,
    /// The disabled colour group.
    pub disabled: Palette,
    /// Status colours: tags, alerts, badges, toned buttons.
    pub status: StatusColors,
    /// Corner radius of controls, px.
    pub control_radius: f32,
    /// Corner radius of dock cards; popups blend toward it.
    pub corner_radius: f32,
    /// Corner radius of frames: group boxes, item views, text edits
    /// (Lace's `RoundedArea`). The theme's `frame_radius`, else its
    /// `control_radius`, as in Lace.
    pub frame_radius: f32,
    /// Keyboard focus ring width, px.
    pub focus_width: f32,
    /// Share of `Text` mixed over a fill for its 1 px outline.
    pub outline_strength: f64,
    /// Splitter grip length, px.
    pub splitter_length: f32,
    /// Whether fields and views draw their outline.
    pub field_outline: bool,
    /// Scrollbar behaviour.
    pub scrollbar: ScrollbarMode,
    /// Width a scrollbar reserves (`PM_ScrollBarExtent`), px; also its
    /// thickness when hovered.
    pub scrollbar_width: f32,
    /// Handle thickness of an expanding bar at rest, px.
    pub scrollbar_rest: f32,
    /// Time an expanding bar takes to grow or shrink, ms; 0 snaps.
    pub scrollbar_grow_ms: f32,
    /// Whether item views lay their scrollbars over the content, keeping
    /// only the slim resting width free, instead of reserving the full
    /// `scrollbar_width` beside it.
    pub scrollbar_overlay: bool,
    /// The theme's contrast level.
    pub contrast: Contrast,
    /// The level outlines are held to (`outline_contrast`, `auto` → `contrast`,
    /// except the classic low-outline chassis).
    pub outline_level: Contrast,
    /// Body font size, px — Fusion's 9 pt at 96 dpi.
    pub font_size: f32,
    /// Small font size (gallery labels, header captions), px.
    pub small_font_size: f32,
    /// Height of a one-line control (button, field, combo), px.
    pub control_height: f32,
    /// The font scale the three sizes above are at: 1 is Fusion's 9 pt.
    /// [`LaceStyle::scaled`] moves them to another.
    pub font_scale: f32,
}

impl Global for LaceStyle {}

/// `_BASIC_CLASSIC_OUTLINE`: at or below this strength outlines use the low floor.
const CLASSIC_OUTLINE_STRENGTH: f64 = 0.1;

impl LaceStyle {
    /// LaceStyle for a built theme.
    pub fn new(spec: &ThemeSpec, t: &Tokens) -> Self {
        let active = Palette::active(t, false);
        let disabled = Palette::disabled(t, &active);
        let outline_level = if spec.outline_strength <= CLASSIC_OUTLINE_STRENGTH {
            Contrast::Low
        } else {
            spec.contrast
        };
        let scrollbar = match spec.scrollbar {
            Scrollbar::Thin => ScrollbarMode::Thin,
            _ => ScrollbarMode::Expanding,
        };
        Self {
            name: spec.name,
            is_light: t.is_light,
            active,
            disabled,
            status: StatusColors { success: t.success, warning: t.warning, error: t.error, info: t.info },
            control_radius: spec.button_radius.unwrap_or(spec.control_radius).max(0.0) as f32,
            corner_radius: spec.corner_radius.max(0.0) as f32,
            frame_radius: spec.frame_radius.unwrap_or(spec.control_radius).max(0.0) as f32,
            scrollbar_width: scrollbar.extent(),
            scrollbar_rest: rules::EXPANDING_REST,
            scrollbar_grow_ms: 120.0,
            scrollbar_overlay: true,
            focus_width: spec.focus_width.max(0.0) as f32,
            outline_strength: spec.outline_strength,
            splitter_length: 50.0,
            field_outline: true,
            scrollbar,
            contrast: spec.contrast,
            outline_level,
            font_size: 12.0,
            small_font_size: 11.0,
            control_height: 24.0,
            font_scale: 1.0,
        }
    }

    /// This style at font scale *scale*: its font sizes and control height
    /// grown or shrunk by the same factor, the way a control's size hint
    /// follows its font in Qt.
    pub fn scaled(&self, scale: f32) -> Self {
        let k = scale / self.font_scale;
        Self {
            font_size: self.font_size * k,
            small_font_size: self.small_font_size * k,
            control_height: self.control_height * k,
            font_scale: scale,
            ..self.clone()
        }
    }

    /// LaceStyle for one of Lace's curated presets, by name.
    pub fn preset(name: &str) -> Option<Self> {
        let spec = lace_gpui::themes::by_name(name)?;
        Some(Self::new(spec, &lace_gpui::build(spec)))
    }

    /// The colour group a control in state `s` paints in.
    pub fn pal(&self, s: State) -> &Palette {
        if s.disabled {
            &self.disabled
        } else {
            &self.active
        }
    }

    /// Contrast floor of non-text UI (indicators, focus ring, thumbs).
    pub fn ui_ratio(&self) -> f64 {
        contrast_target(Role::Ui, self.contrast)
    }

    /// Contrast floor of control outlines.
    pub fn border_ratio(&self) -> f64 {
        contrast_target(Role::Border, self.outline_level)
    }

    /// Contrast floor of muted text.
    pub fn muted_ratio(&self) -> f64 {
        contrast_target(Role::Muted, self.contrast)
    }

    /// Contrast floor of body text.
    pub fn text_ratio(&self) -> f64 {
        contrast_target(Role::Text, self.contrast)
    }

    /// How far a frame's content keeps from its edges so nothing reaches
    /// into a rounded corner, px.
    ///
    /// gpui clips to rectangles only, so where Lace masks a view to its
    /// rounded shape, the content here is inset instead: by `r·(1 − 1/√2)`,
    /// the inner rectangle's corner then lies exactly on the arc.
    pub fn frame_inset(&self, radius: f32) -> f32 {
        (radius * (1.0 - std::f32::consts::FRAC_1_SQRT_2)).ceil()
    }

    /// A popup's corner radius: a third of the way from control to card.
    pub fn popup_radius(&self) -> f32 {
        (self.control_radius + (self.corner_radius - self.control_radius) / 3.0)
            .round()
            .max(self.control_radius)
    }
}

/// Install `style` for every LaceStyle widget, and refresh open windows.
/// It is drawn at the central [`font_scale`], whatever scale it was made at.
pub fn install(style: LaceStyle, cx: &mut App) {
    let scale = font_scale(cx);
    cx.set_global(style.scaled(scale));
    cx.refresh_windows();
}

/// The smallest and largest central font scale.
pub const FONT_SCALE_RANGE: (f32, f32) = (0.75, 2.0);

/// The central font scale every widget is drawn at (Qt's application font
/// grown or shrunk): Lace's styles and the kit's rem both follow it.
struct FontScale(f32);

impl Global for FontScale {}

/// The central font scale: 1 until [`set_font_scale`] changes it.
pub fn font_scale(cx: &App) -> f32 {
    cx.try_global::<FontScale>().map_or(1.0, |s| s.0)
}

/// Set the central font scale (clamped to [`FONT_SCALE_RANGE`]): the
/// installed [`LaceStyle`] and the kit theme's font sizes (its rem) move to
/// it, and open windows redraw. Styles installed later are drawn at it too.
pub fn set_font_scale(scale: f32, cx: &mut App) {
    let scale = scale.clamp(FONT_SCALE_RANGE.0, FONT_SCALE_RANGE.1);
    let old = font_scale(cx);
    if scale == old {
        return;
    }
    cx.set_global(FontScale(scale));
    if cx.has_global::<Theme>() {
        let k = scale / old;
        let theme = Theme::global_mut(cx);
        theme.font_size = theme.font_size * k;
        theme.mono_font_size = theme.mono_font_size * k;
    }
    if cx.has_global::<LaceStyle>() {
        let style = cx.lace().scaled(scale);
        cx.set_global(style);
    }
    cx.refresh_windows();
}

/// Draw *child* as though the central font scale were *scale*: Lace's
/// widgets in it use the installed style at that scale, and the kit's at the
/// matching rem. A canvas draws a zoomed form this way, at its zoom.
pub fn with_font_scale(scale: f32, child: impl IntoElement) -> FontScaled {
    FontScaled { scale, child: child.into_any_element() }
}

/// The element [`with_font_scale`] returns. It adds no layout node of its
/// own, so the child keeps its own position.
pub struct FontScaled {
    scale: f32,
    child: AnyElement,
}

impl FontScaled {
    /// Run *f* under this element's style and rem, then put both back.
    fn under<R>(scale: f32, window: &mut Window, cx: &mut App, f: impl FnOnce(&mut Window, &mut App) -> R) -> R {
        let rem = window.rem_size() * (scale / font_scale(cx));
        let saved = cx.has_global::<LaceStyle>().then(|| cx.lace().clone());
        if let Some(s) = &saved {
            cx.set_global(s.scaled(scale));
        }
        let out = window.with_rem_size(Some(rem), |window| f(window, cx));
        if let Some(s) = saved {
            cx.set_global(s);
        }
        out
    }
}

impl IntoElement for FontScaled {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for FontScaled {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let child = &mut self.child;
        (Self::under(self.scale, window, cx, |window, cx| child.request_layout(window, cx)), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        let child = &mut self.child;
        Self::under(self.scale, window, cx, |window, cx| child.prepaint(window, cx));
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut (),
        _prepaint: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        let child = &mut self.child;
        Self::under(self.scale, window, cx, |window, cx| child.paint(window, cx));
    }
}

/// Install one of Lace's presets; `false` when there is no such preset.
pub fn install_preset(name: &str, cx: &mut App) -> bool {
    match LaceStyle::preset(name) {
        Some(style) => {
            install(style, cx);
            true
        }
        None => false,
    }
}

/// Read access to the installed [`LaceStyle`].
pub trait ActiveStyle {
    /// The installed style. Panics when none is installed — call
    /// [`install`] or [`install_preset`] at startup.
    fn lace(&self) -> &LaceStyle;
}

impl ActiveStyle for App {
    fn lace(&self) -> &LaceStyle {
        self.global::<LaceStyle>()
    }
}

/// An [`Rgba`] as a gpui colour.
pub fn hsla(c: Rgba) -> Hsla {
    lace_gpui::skin::hsla(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_style_scales_from_whatever_scale_it_is_at() {
        let base = LaceStyle::preset("dark").expect("a preset");
        let big = base.scaled(1.5);
        assert_eq!((big.font_size, big.small_font_size, big.control_height), (18.0, 16.5, 36.0));
        let back = big.scaled(1.0);
        assert_eq!(back, base);
        assert_eq!(base.scaled(0.75).scaled(2.0), base.scaled(2.0));
    }
}
