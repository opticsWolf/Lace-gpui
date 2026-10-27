//! `LaceStyle` — Lace's flat `QProxyStyle` over Fusion — as a widget layer
//! for gpui-kit.
//!
//! [`crate::skin`] ports Lace's palette, the colours its dock chrome and its
//! widgets share. `LaceStyle` adds a second, orthogonal layer: it keeps
//! Fusion's metrics, but replaces Fusion's gradients, bevels and pixmap glyphs
//! with flat fills, one outline, and a handful of derived state colours
//! (`lace/lace_style.py`, `lace/style/_paint.py`). gpui-kit's theme has slots
//! for some of those rules and not others; this module derives LaceStyle's
//! values from a [`Tokens`] and writes the ones gpui-kit can name.
//!
//! LaceStyle itself is flat. [`BaseStyle::apply`] adds a shallow depth pass on
//! top: gpui-kit tokens carry a renderable `Background`, so faces, tabs,
//! headers and thumbs can take a small vertical sheen, progress can sweep
//! horizontally, and saturated accent faces only shade away from their label.
//! Hover and pressed faces get a stronger sheen than resting ones. gpui-kit's
//! 3 px focus band is switched off because the renderer cannot blur it; focus
//! falls back to a 1 px accent-tinted edge, and the gallery adds a soft
//! app-side glow around the focused control. The gallery also supplies the
//! soft shadows and accent halos where the theme surface has no shadow slot.
//!
//! The derivation is pure — [`BaseStyle::from_tokens`] and
//! [`BaseStyle::from_spec`] need no gpui context. [`BaseStyle::apply`] is the
//! write half used by [`crate::skin::apply_styled`].

use gpui_kit::component::theme::{Theme, ThemeTokens};
use gpui_kit::gpui::{linear_color_stop, linear_gradient, px, Background};

use crate::color::{self, Rgba};
use crate::contrast::{contrast_target, Role};
use crate::engine::{Contrast, ThemeSpec, Tokens};
use crate::skin::hsla;

/// Hover lightness step, in OKLCH L (`_paint.HOVER_STEP`).
pub const HOVER_STEP: f64 = 0.04;
/// Pressed lightness step, in OKLCH L (`_paint.PRESS_STEP`).
pub const PRESS_STEP: f64 = 0.08;
/// Share of text mixed over the window for a resting scrollbar thumb
/// (`_primitives.scrollbar`).
pub const SCROLLBAR_REST: f64 = 0.32;
/// Share of text mixed over the window for a hovered scrollbar thumb.
pub const SCROLLBAR_HOVER: f64 = 0.45;
/// Share of text mixed over the window for a dragged scrollbar thumb.
///
/// gpui-kit paints its active thumb with the hover token, so this one is
/// recorded but not mapped separately.
pub const SCROLLBAR_PRESSED: f64 = 0.55;
/// Share of text mixed over the window for an expanded scrollbar track.
pub const SCROLLBAR_TRACK: f64 = 0.05;
/// Share of text mixed over the window for a slider's neutral groove.
///
/// gpui-kit's single `slider_bar` colour drives both the groove and the
/// accent fill, so the base layer keeps the accent there and cannot show this
/// mix; the constant documents the rule.
pub const SLIDER_TRACK: f64 = 0.16;
/// LaceStyle's splitter grip length, in px (`splitter_length`).
///
/// gpui-kit has no theme-level splitter length; the dock geometry skin owns
/// its own splitter sizes.
pub const SPLITTER_LENGTH: f32 = 50.0;
/// Share of the distance from the control radius to the card radius that a
/// popup rounds to (`_popup.RADIUS_BLEND`).
const POPUP_RADIUS_BLEND: f32 = 1.0 / 3.0;
/// `_BASIC_CLASSIC_OUTLINE`: at or below this strength the classic chassis
/// holds its outlines to the low border floor (1.15:1).
pub const CLASSIC_OUTLINE_STRENGTH: f64 = 0.1;
/// Share of the non-text UI floor used for the shared field/indicator outline.
///
/// 1 px at the full 3:1 floor reads as a hard line around a checkbox, radio or
/// field. The base layer keeps it findable at 80% of that floor (2.4:1 on the
/// normal contrast level): still clearly visible, but no longer a bright edge.
pub const INDICATOR_FLOOR_SCALE: f64 = 0.8;

// ---------------------------------------------------------------------------
// Depth
// ---------------------------------------------------------------------------
// LaceStyle itself is flat. The base layer adds a barely-there vertical sheen
// so large controls read as objects rather than fills, and so the palette's
// surfaces keep some depth on a dark ground. The steps are deliberately small:
// at 8-bit sRGB, 0.01 of OKLCH L is only a few channel values.

/// Lightness lift at the top of a raised neutral face.
pub const FACE_LIFT: f64 = 0.010;
/// Lightness shade at the bottom of a raised neutral face.
pub const FACE_SHADE: f64 = 0.014;
/// Lightness lift at the top of a saturated accent face.
pub const ACCENT_LIFT: f64 = 0.008;
/// Lightness shade at the bottom of a saturated accent face.
pub const ACCENT_SHADE: f64 = 0.010;
/// Inset faces invert the sheen: the top is the shaded end.
pub const INSET_TOP: f64 = 0.006;
/// Inset faces lift the bottom end.
pub const INSET_BOTTOM: f64 = 0.006;
/// Header and tab surfaces carry a flatter sheen than controls.
pub const HEADER_LIFT: f64 = 0.006;
/// Bottom shade of a header or tab surface.
pub const HEADER_SHADE: f64 = 0.005;
/// Canvas and panel surfaces get the quietest sheen of all.
pub const SURFACE_LIFT: f64 = 0.004;
/// Bottom shade of a canvas or panel surface.
pub const SURFACE_SHADE: f64 = 0.006;
/// Scrollbar and slider thumbs get a rounder, stronger highlight.
pub const THUMB_LIFT: f64 = 0.014;
/// Bottom shade of a thumb.
pub const THUMB_SHADE: f64 = 0.014;
/// Hover faces carry more relief than at rest, so the state change reads as a
/// lit surface rather than a flat colour swap.
pub const STATE_FACE_LIFT: f64 = 0.018;
/// Bottom shade of a hovered neutral face.
pub const STATE_FACE_SHADE: f64 = 0.022;
/// Inset depth of a pressed neutral face.
pub const STATE_INSET: f64 = 0.014;
/// Top lift of a hovered saturated face whose label is dark.
pub const STATE_ACCENT_LIFT: f64 = 0.012;
/// Bottom shade of a hovered or pressed saturated face whose label is light.
pub const STATE_ACCENT_SHADE: f64 = 0.016;

/// LaceStyle's widget-level rules, derived from one [`Tokens`] palette.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BaseStyle {
    /// Multiplier for the depth pass, 1.0 by default.
    ///
    /// Every gradient lift/shade in [`BaseStyle::apply`] is multiplied by
    /// this. 1.0 is the spec look, 0.0 is flat LaceStyle; use
    /// [`BaseStyle::with_depth_scale`] to tune it per application.
    pub depth_scale: f64,
    /// Control corner radius, px.
    pub control_radius: f32,
    /// Dock card / large-surface corner radius, px.
    pub corner_radius: f32,
    /// Radius LaceStyle blends for popups — between the control and card
    /// radius, never below the control radius.
    ///
    /// gpui-kit has one large-radius slot (`Theme::radius_lg`), which cards
    /// use too, so [`BaseStyle::apply`] keeps the card radius there.
    pub popup_radius: f32,
    /// Splitter grip length, px. Advisory — see [`SPLITTER_LENGTH`].
    pub splitter_length: f32,
    /// Keyboard focus ring pen width, px; 0 hides the ring.
    pub focus_width: f32,
    /// Share of the text colour LaceStyle mixes into a control's fill for its
    /// outline.
    pub outline_strength: f64,
    /// Contrast floor for control outlines and frames.
    pub border_ratio: f64,
    /// Contrast floor for non-text UI: focus ring, scrollbar thumb, checked
    /// indicators.
    pub ui_ratio: f64,
    /// Whether unfocused fields keep their outline (LaceStyle's
    /// `field_outline`, true by default).
    pub field_outline: bool,
    /// Button face fill.
    pub button: Rgba,
    /// Button face on hover.
    pub button_hover: Rgba,
    /// Button face while pressed.
    pub button_active: Rgba,
    /// Outline of an unfocused button.
    pub button_outline: Rgba,
    /// Outline of an unfocused input field, held to the border floor.
    ///
    /// Kept and documented even though gpui-kit gives fields and indicators
    /// the same `ThemeColor::input` slot; see [`Self::indicator_outline_color`].
    pub field_outline_color: Rgba,
    /// Outline of an unchecked checkbox or radio, held to 80% of
    /// [`BaseStyle::ui_ratio`].
    ///
    /// This is the value fed to the shared `ThemeColor::input` slot, so the
    /// indicators and field edges stay findable at gpui's fixed 1 px width
    /// without becoming a hard, bright line.
    pub indicator_outline_color: Rgba,
    /// Opaque outline for cards, frames and structural dividers.
    ///
    /// Replaces the palette's translucent border so a 1 px line does not
    /// composite away against the surface.
    pub frame_outline_color: Rgba,
    /// Accent, as used for a default button's face and a slider's fill.
    pub accent: Rgba,
    /// Accent face on hover.
    pub accent_hover: Rgba,
    /// Accent face while pressed.
    pub accent_active: Rgba,
    /// Accent focus ring at full strength, held to [`BaseStyle::ui_ratio`].
    ///
    /// [`BaseStyle::apply`] writes the full-strength colour to the theme for
    /// the 1 px tinted edge; gpui-kit's hard 3 px band stays off and the
    /// gallery supplies the soft halo.
    pub focus_ring: Rgba,
    /// Faint scrollbar track, shown when a bar expands.
    pub scrollbar_track: Rgba,
    /// Resting scrollbar thumb.
    pub scrollbar_thumb: Rgba,
    /// Hovered scrollbar thumb.
    pub scrollbar_thumb_hover: Rgba,
    /// Slider handle face: the button face, as LaceStyle draws it.
    pub slider_thumb: Rgba,
}

/// A top-to-bottom linear gradient between two opaque colours.
fn vertical(top: Rgba, bottom: Rgba) -> Background {
    linear_gradient(
        180.0,
        linear_color_stop(hsla(top), 0.0),
        linear_color_stop(hsla(bottom), 1.0),
    )
}

/// A left-to-right sweep, for progress fills.
fn sweep(left: Rgba, right: Rgba) -> Background {
    linear_gradient(
        90.0,
        linear_color_stop(hsla(left), 0.0),
        linear_color_stop(hsla(right), 1.0),
    )
}

/// A raised face: a light top edge, a slightly deeper bottom.
fn raised(fill: Rgba, lift: f64, shade: f64) -> Background {
    vertical(color::step(fill, lift), color::step(fill, -shade))
}

/// A pressed face: the sheen inverted, so the top reads as recessed.
fn inset(fill: Rgba, lift: f64, shade: f64) -> Background {
    vertical(color::step(fill, -lift), color::step(fill, shade))
}

/// The two stops of a saturated face, ordered top to bottom.
///
/// White-labelled accents keep their enforced face at the top and shade
/// downward; dark-labelled accents lift the top and keep the enforced face at
/// the bottom. This keeps both stops at least as readable as the base face.
fn accent_stops(fill: Rgba, lift: f64, shade: f64) -> (Rgba, Rgba) {
    if color::contrast_ratio(color::WHITE, fill) >= color::contrast_ratio(color::BLACK, fill) {
        (fill, color::step(fill, -shade))
    } else {
        (color::step(fill, lift), fill)
    }
}

/// A saturated face whose gradient stays on the safe side of its label.
fn accent_face(fill: Rgba, lift: f64, shade: f64) -> Background {
    let (top, bottom) = accent_stops(fill, lift, shade);
    vertical(top, bottom)
}

impl BaseStyle {
    /// Derive LaceStyle's rules from a palette.
    ///
    /// `Tokens` does not carry the spec's contrast level, so editor themes
    /// are assumed to be at `normal` — true of all sixteen curated presets.
    /// Prefer [`BaseStyle::from_spec`] when the spec is at hand.
    pub fn from_tokens(t: &Tokens) -> Self {
        let outline_level = Self::outline_level(t.outline_strength, Contrast::Normal);
        Self::derive(
            t,
            outline_level,
            Contrast::Normal,
            t.control_radius,
            t.corner_radius,
        )
    }

    /// Derive LaceStyle's rules from a spec and its built palette.
    pub fn from_spec(spec: &ThemeSpec, t: &Tokens) -> Self {
        let outline_level = Self::outline_level(spec.outline_strength, spec.contrast);
        Self::derive(
            t,
            outline_level,
            spec.contrast,
            spec.control_radius,
            spec.corner_radius,
        )
    }

    /// Lace's classic Basics pin `outline_contrast="low"`; that chassis is
    /// the one at or below [`CLASSIC_OUTLINE_STRENGTH`]. Everything else
    /// follows the theme's own contrast level.
    fn outline_level(outline_strength: f64, level: Contrast) -> Contrast {
        if outline_strength <= CLASSIC_OUTLINE_STRENGTH {
            Contrast::Low
        } else {
            level
        }
    }

    /// Scale the depth pass: 1.0 is the spec look, 0.0 is flat LaceStyle.
    ///
    /// Negative scales clamp to 0.0; values above 1.0 deepen the relief.
    /// Solid token colours are untouched — only the gradient stops move.
    pub fn with_depth_scale(mut self, scale: f64) -> Self {
        self.depth_scale = scale.max(0.0);
        self
    }

    fn derive(
        t: &Tokens,
        outline_level: Contrast,
        contrast: Contrast,
        control_radius: f64,
        corner_radius: f64,
    ) -> Self {
        let canvas = t.canvas_bg;
        let panel = t.panel_bg;
        let button = t.button_bg;
        let accent = t.accent_color;
        let border_ratio = contrast_target(Role::Border, outline_level);
        let ui_ratio = contrast_target(Role::Ui, contrast);
        let stroke = |fill: Rgba| color::mix(fill, t.text, t.outline_strength);
        let legible_on =
            |c: Rgba, surface: Rgba, ratio: f64| color::ensure_contrast(c, surface, ratio);
        let mix_text = |amount: f64| color::mix(canvas, t.text, amount);

        let control_radius = control_radius.max(0.0) as f32;
        let corner_radius = corner_radius.max(0.0) as f32;
        let popup_radius = (control_radius + (corner_radius - control_radius) * POPUP_RADIUS_BLEND)
            .round()
            .max(control_radius);

        Self {
            control_radius,
            corner_radius,
            popup_radius,
            splitter_length: SPLITTER_LENGTH,
            depth_scale: 1.0,
            focus_width: t.focus_width.max(0.0) as f32,
            outline_strength: t.outline_strength,
            border_ratio,
            ui_ratio,
            field_outline: true,
            button,
            button_hover: color::step_contrast(button, HOVER_STEP),
            button_active: color::step_contrast(button, PRESS_STEP),
            button_outline: legible_on(stroke(button), panel, border_ratio),
            field_outline_color: legible_on(stroke(t.input_bg), panel, border_ratio),
            // LaceStyle draws unchecked indicators at the non-text UI floor,
            // not the border floor: they are 1 px and have to stay findable.
            // The base layer backs that off by 20% so the opaque line is a
            // control edge, not a hard glow.
            indicator_outline_color: legible_on(
                stroke(t.input_bg),
                panel,
                ui_ratio * INDICATOR_FLOOR_SCALE,
            ),
            frame_outline_color: legible_on(stroke(panel), panel, border_ratio),
            accent,
            accent_hover: color::step_contrast(accent, HOVER_STEP),
            accent_active: color::step_contrast(accent, PRESS_STEP),
            focus_ring: legible_on(accent, panel, ui_ratio),
            scrollbar_track: mix_text(SCROLLBAR_TRACK),
            scrollbar_thumb: legible_on(mix_text(SCROLLBAR_REST), canvas, ui_ratio),
            scrollbar_thumb_hover: legible_on(mix_text(SCROLLBAR_HOVER), canvas, ui_ratio),
            slider_thumb: button,
        }
    }

    /// Write the rules gpui-kit can represent onto `theme`.
    ///
    /// `source` is the palette the base layer was derived from; it supplies
    /// the surfaces that get a quiet sheen. The caller must rebuild the base
    /// layer and refresh windows; [`crate::skin::apply_spec_styled`] does
    /// both.
    pub fn apply(&self, theme: &mut Theme, source: &Tokens) {
        // LaceStyle's widget frames use one opaque stroke. The palette's
        // translucent border is the dock-chrome value; used on a widget it
        // composites down to a line that may as well not be there. Feed the
        // opaque frame to every structural border instead.
        let frame = hsla(self.frame_outline_color);
        theme.colors.border = frame;
        theme.colors.window_border = frame;
        theme.colors.sidebar_border = frame;
        theme.colors.title_bar_border = frame;
        theme.colors.status_bar_border = frame;
        theme.colors.table_row_border = frame;

        // gpui-kit shares one `input` slot across fields, default buttons and
        // unchecked indicators. LaceStyle lifts indicators to the 3:1 UI floor
        // but fields only to the border floor; with one slot available the
        // indicator wins, because a 1 px box cannot afford a 1.15:1 line.
        // The derived value is already backed off by `INDICATOR_FLOOR_SCALE`.
        theme.colors.input = if self.field_outline {
            hsla(self.indicator_outline_color)
        } else {
            frame
        };
        // The ring is the full, contrast-checked accent. Focus does not use
        // gpui-kit's 3 px band (see `focus_ring` below), so this colour is
        // drawn as the 1 px tinted edge and as the gallery's glow source.
        // `drag_border` stays opaque for the dock's resize handles.
        theme.colors.ring = hsla(self.focus_ring);
        theme.colors.drag_border = hsla(self.focus_ring);

        theme.colors.scrollbar = hsla(self.scrollbar_track);
        theme.colors.scrollbar_thumb = hsla(self.scrollbar_thumb);
        theme.colors.scrollbar_thumb_hover = hsla(self.scrollbar_thumb_hover);

        // One colour drives both the groove and its accent fill in gpui-kit;
        // LaceStyle's accent fill is the half that carries the look.
        theme.colors.slider_bar = hsla(self.accent);
        theme.colors.slider_thumb = hsla(self.slider_thumb);

        // LaceStyle steps every button face by lightness, not by the theme's
        // hover surfaces: +0.04 on hover, +0.08 pressed. Status buttons
        // follow the same rule.
        theme.colors.button_hover = hsla(self.button_hover);
        theme.colors.button_active = hsla(self.button_active);
        theme.colors.button_secondary_hover = hsla(self.button_hover);
        theme.colors.button_secondary_active = hsla(self.button_active);
        theme.colors.secondary_hover = hsla(self.button_hover);
        theme.colors.secondary_active = hsla(self.button_active);
        theme.colors.button_primary_hover = hsla(self.accent_hover);
        theme.colors.button_primary_active = hsla(self.accent_active);
        theme.colors.primary_hover = hsla(self.accent_hover);
        theme.colors.primary_active = hsla(self.accent_active);
        for (base, hover, active) in [
            (
                source.error,
                &mut theme.colors.button_danger_hover,
                &mut theme.colors.button_danger_active,
            ),
            (
                source.warning,
                &mut theme.colors.button_warning_hover,
                &mut theme.colors.button_warning_active,
            ),
            (
                source.success,
                &mut theme.colors.button_success_hover,
                &mut theme.colors.button_success_active,
            ),
            (
                source.info,
                &mut theme.colors.button_info_hover,
                &mut theme.colors.button_info_active,
            ),
        ] {
            *hover = hsla(color::step_contrast(base, HOVER_STEP));
            *active = hsla(color::step_contrast(base, PRESS_STEP));
        }

        // A default button's border is ThemeColor::input, and an input's
        // border is the same field slot; the field outline wins because it
        // also shapes the dark input fill (`input.mix(transparent, 0.3)`).
        theme.radius = px(self.control_radius);
        theme.radius_lg = px(self.corner_radius);
        // LaceStyle's focus ring is `focus_width` pixels of accent. gpui-kit
        // draws the same idea as a hard 3 px band with no blur, which reads as
        // a bright outline rather than a glow. Keep the theme's 1 px tinted
        // edge fallback and leave the soft halo to the application; the
        // gallery adds one around focused fields and controls.
        theme.focus_ring = false;

        // Solid tokens first, then the depth layer: every background below
        // keeps its token's representative `color` for text and icons.
        // `depth_scale` (1.0 = spec, 0.0 = flat) multiplies every lift and
        // shade; the progress sweep is a hue sweep, not relief, and stays.
        theme.tokens = ThemeTokens::from(&theme.colors);
        let tokens = &mut theme.tokens;
        let depth = self.depth_scale;
        let at = |v: f64| v * depth;

        // Neutral button faces. Resting faces stay quiet; hover and press get
        // a stronger sheen so a state change is visible at a glance.
        let face = raised(self.button, at(FACE_LIFT), at(FACE_SHADE));
        tokens.button.background = face;
        tokens.button_secondary.background = face;
        tokens.secondary.background = raised(self.button, at(HEADER_LIFT), at(HEADER_SHADE));
        let hover = raised(self.button_hover, at(STATE_FACE_LIFT), at(STATE_FACE_SHADE));
        tokens.button_hover.background = hover;
        tokens.button_secondary_hover.background = hover;
        tokens.secondary_hover.background = hover;
        let active = inset(self.button_active, at(STATE_INSET), at(STATE_INSET));
        tokens.button_active.background = active;
        tokens.button_secondary_active.background = active;
        tokens.secondary_active.background = active;

        // Accent faces and the accent wash behind checkboxes, radios and
        // switch tracks.
        tokens.button_primary.background =
            accent_face(self.accent, at(ACCENT_LIFT), at(ACCENT_SHADE));
        tokens.button_primary_hover.background = accent_face(
            self.accent_hover,
            at(STATE_ACCENT_LIFT),
            at(STATE_ACCENT_SHADE),
        );
        tokens.button_primary_active.background = accent_face(
            self.accent_active,
            at(STATE_ACCENT_LIFT),
            at(STATE_ACCENT_SHADE),
        );
        tokens.primary.background =
            accent_face(self.accent, at(ACCENT_LIFT * 0.5), at(ACCENT_SHADE * 0.5));
        tokens.primary_hover.background = accent_face(
            self.accent_hover,
            at(STATE_ACCENT_LIFT * 0.5),
            at(STATE_ACCENT_SHADE * 0.5),
        );
        tokens.primary_active.background = accent_face(
            self.accent_active,
            at(STATE_ACCENT_LIFT * 0.5),
            at(STATE_ACCENT_SHADE * 0.5),
        );
        tokens.accent.background =
            accent_face(self.accent, at(ACCENT_LIFT * 0.5), at(ACCENT_SHADE * 0.5));

        // Status buttons: same relief, same state steps.
        for (base, normal, hover, active) in [
            (
                source.error,
                &mut tokens.button_danger,
                &mut tokens.button_danger_hover,
                &mut tokens.button_danger_active,
            ),
            (
                source.warning,
                &mut tokens.button_warning,
                &mut tokens.button_warning_hover,
                &mut tokens.button_warning_active,
            ),
            (
                source.success,
                &mut tokens.button_success,
                &mut tokens.button_success_hover,
                &mut tokens.button_success_active,
            ),
            (
                source.info,
                &mut tokens.button_info,
                &mut tokens.button_info_hover,
                &mut tokens.button_info_active,
            ),
        ] {
            let hover_colour = color::step_contrast(base, HOVER_STEP);
            let active_colour = color::step_contrast(base, PRESS_STEP);
            normal.color = hsla(base);
            hover.color = hsla(hover_colour);
            active.color = hsla(active_colour);
            normal.background = accent_face(base, at(ACCENT_LIFT), at(ACCENT_SHADE));
            hover.background =
                accent_face(hover_colour, at(STATE_ACCENT_LIFT), at(STATE_ACCENT_SHADE));
            active.background =
                accent_face(active_colour, at(STATE_ACCENT_LIFT), at(STATE_ACCENT_SHADE));
        }

        // A cool-to-bright sweep along a progress fill.
        tokens.progress_bar.background = sweep(self.accent, source.accent_bright);

        // Thumbs get a little more roundness than a face.
        tokens.scrollbar_thumb.background =
            raised(self.scrollbar_thumb, at(THUMB_LIFT), at(THUMB_SHADE));
        tokens.scrollbar_thumb_hover.background =
            raised(self.scrollbar_thumb_hover, at(THUMB_LIFT), at(THUMB_SHADE));
        tokens.slider_thumb.background = raised(self.slider_thumb, at(THUMB_LIFT), at(THUMB_SHADE));
        tokens.switch_thumb.background = raised(self.slider_thumb, at(THUMB_LIFT), at(THUMB_SHADE));

        // Header, tab and table-head surfaces: a flatter sheen than controls.
        let header = raised(source.tab_bg_normal, at(HEADER_LIFT), at(HEADER_SHADE));
        tokens.tab_bar.background = header;
        tokens.tab_bar_segmented.background = header;
        tokens.tab.background = header;
        tokens.title_bar.background = header;
        tokens.table_head.background = header;
        tokens.list_head.background = header;
        tokens.description_list_label.background = header;
        tokens.table_foot.background = header;
        tokens.tab_active.background =
            raised(source.tab_bg_active, at(HEADER_LIFT), at(HEADER_SHADE));
        tokens.status_bar.background = raised(source.panel_bg, at(HEADER_LIFT), at(HEADER_SHADE));

        // Quiet large surfaces. The background gradient is what gives the
        // window its atmosphere; panels and cards sit on it with their own.
        tokens.background.background =
            raised(source.canvas_bg, at(SURFACE_LIFT), at(SURFACE_SHADE));
        tokens.sidebar.background = raised(source.sidebar_bg, at(SURFACE_LIFT), at(SURFACE_SHADE));
        tokens.sidebar_primary.background =
            raised(source.panel_bg, at(SURFACE_LIFT), at(SURFACE_SHADE));
        let panel = raised(source.panel_bg, at(SURFACE_LIFT), at(SURFACE_SHADE));
        tokens.list.background = panel;
        tokens.table.background = panel;
        tokens.accordion.background = panel;
        tokens.group_box.background = panel;
        tokens.popover.background = raised(source.tooltip_bg, at(HEADER_LIFT), at(HEADER_SHADE));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::themes::by_name;

    fn spec(name: &str) -> &'static ThemeSpec {
        by_name(name).unwrap()
    }

    #[test]
    fn classic_themes_use_the_low_outline_floor() {
        let classic = BaseStyle::from_spec(spec("midnight"), &crate::build(spec("midnight")));
        assert!((classic.border_ratio - 1.15).abs() < 1e-9);
        let editor = BaseStyle::from_spec(spec("dracula"), &crate::build(spec("dracula")));
        assert!((editor.border_ratio - 1.3).abs() < 1e-9);
    }

    #[test]
    fn control_outlines_clear_their_floor() {
        for name in ["midnight", "cream", "dracula", "cyberpunk_neon"] {
            let t = crate::build(spec(name));
            let b = BaseStyle::from_tokens(&t);
            for (label, colour, floor) in [
                ("button", b.button_outline, b.border_ratio),
                ("field", b.field_outline_color, b.border_ratio),
                (
                    "indicator",
                    b.indicator_outline_color,
                    b.ui_ratio * INDICATOR_FLOOR_SCALE,
                ),
                ("frame", b.frame_outline_color, b.border_ratio),
            ] {
                let ratio = color::contrast_ratio(colour, t.panel_bg);
                assert!(
                    ratio + 1e-6 >= floor,
                    "{name} {label} outline {ratio} < {floor}"
                );
            }
        }
    }

    #[test]
    fn indicators_are_stronger_than_field_outlines() {
        for name in ["midnight", "cream", "dracula", "cyberpunk_neon"] {
            let t = crate::build(spec(name));
            let b = BaseStyle::from_tokens(&t);
            let field = color::contrast_ratio(b.field_outline_color, t.panel_bg);
            let indicator = color::contrast_ratio(b.indicator_outline_color, t.panel_bg);
            assert!(
                indicator > field,
                "{name}: indicator {indicator} should beat field {field}"
            );
            // The strengthened indicator is deliberately backed off by 20%
            // from the non-text floor the previous pass used.
            assert!(
                (indicator - b.ui_ratio * INDICATOR_FLOOR_SCALE).abs() < 0.05,
                "{name}: indicator {indicator} should sit at 80% of {}",
                b.ui_ratio
            );
            assert_eq!(b.indicator_outline_color[3], 255, "{name}: opacity");
            assert_eq!(b.frame_outline_color[3], 255, "{name}: opacity");
        }
    }

    #[test]
    fn state_steps_head_toward_contrast() {
        let light = |c| color::to_oklch(c).0;

        let dark = crate::build(spec("midnight"));
        let b = BaseStyle::from_tokens(&dark);
        assert!(light(b.button_hover) > light(b.button));
        assert!(light(b.button_active) > light(b.button_hover));

        let pale = crate::build(spec("cream"));
        let b = BaseStyle::from_tokens(&pale);
        assert!(light(b.button_hover) < light(b.button));
        assert!(light(b.button_active) < light(b.button_hover));
    }

    #[test]
    fn scrollbar_thumb_is_ui_legible() {
        for name in ["midnight", "cream", "dracula", "cyberpunk_neon"] {
            let t = crate::build(spec(name));
            let b = BaseStyle::from_tokens(&t);
            let ratio = color::contrast_ratio(b.scrollbar_thumb, t.canvas_bg);
            assert!(
                ratio + 1e-6 >= b.ui_ratio,
                "{name} thumb {ratio} < {}",
                b.ui_ratio
            );
        }
    }

    #[test]
    fn accent_gradients_never_weaken_their_foreground() {
        for name in ["midnight", "cream", "dracula", "cyberpunk_neon"] {
            let t = crate::build(spec(name));
            for base in [t.accent_color, t.warning, t.error, t.success, t.info] {
                let fg = color::on_color(base, [color::WHITE, color::BLACK], 4.5);
                let base_ratio = color::contrast_ratio(fg, base);
                for (label, lift, shade) in [
                    ("rest", ACCENT_LIFT, ACCENT_SHADE),
                    ("state", STATE_ACCENT_LIFT, STATE_ACCENT_SHADE),
                    ("wash", STATE_ACCENT_LIFT * 0.5, STATE_ACCENT_SHADE * 0.5),
                ] {
                    let (top, bottom) = accent_stops(base, lift, shade);
                    for (end, colour) in [("top", top), ("bottom", bottom)] {
                        let ratio = color::contrast_ratio(fg, colour);
                        assert!(
                            ratio + 1e-6 >= base_ratio,
                            "{name} {label}: {end} end {ratio} < base {base_ratio} on {base:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn focus_uses_the_edge_fallback_not_the_hard_band() {
        let s = spec("midnight");
        let t = crate::build(s);
        let mut theme = Theme::default();
        crate::skin::apply_tokens(&mut theme, &t);
        let base = BaseStyle::from_spec(s, &t);
        base.apply(&mut theme, &t);

        // The derivation keeps the full, contrast-checked ring colour...
        assert_eq!(base.focus_ring[3], 255);
        assert!(
            color::contrast_ratio(base.focus_ring, t.panel_bg) + 1e-6 >= base.ui_ratio,
            "focus ring should be derived at the UI floor"
        );
        // ...and hands the full colour to the theme for the 1 px edge.
        assert_eq!(theme.colors.ring.a, 1.0);
        // The 3 px band is off: the renderer cannot blur a border, so a band
        // would read as a hard outline. Focus keeps a 1 px tinted edge and the
        // gallery supplies the soft halo.
        assert!(!theme.focus_ring);
        assert_eq!(theme.colors.drag_border.a, 1.0);
    }

    #[test]
    fn popup_radius_bridges_control_and_card() {
        let b = BaseStyle::from_tokens(&crate::build(spec("cyberpunk_neon")));
        assert_eq!(b.control_radius, 4.0);
        assert_eq!(b.corner_radius, 10.0);
        assert_eq!(b.popup_radius, 6.0);
    }

    #[test]
    fn depth_scale_flattens_the_relief() {
        let s = spec("midnight");
        let tokens = crate::build(s);
        assert_eq!(BaseStyle::from_spec(s, &tokens).depth_scale, 1.0);
        assert_eq!(
            BaseStyle::from_spec(s, &tokens)
                .with_depth_scale(-0.5)
                .depth_scale,
            0.0,
            "negative scales clamp to flat"
        );

        let mut full = Theme::default();
        crate::skin::apply_tokens(&mut full, &tokens);
        BaseStyle::from_spec(s, &tokens).apply(&mut full, &tokens);
        let mut flat = Theme::default();
        crate::skin::apply_tokens(&mut flat, &tokens);
        BaseStyle::from_spec(s, &tokens)
            .with_depth_scale(0.0)
            .apply(&mut flat, &tokens);

        // Solid representative colours are untouched — only stops move.
        assert_eq!(full.tokens.button.color, flat.tokens.button.color);
        // The relief itself collapses: same gradient shape, moved stops.
        let full_dbg = format!("{:?}", full.tokens.button.background);
        let flat_dbg = format!("{:?}", flat.tokens.button.background);
        assert_ne!(full_dbg, flat_dbg, "scale must move the stops");
        assert!(
            flat_dbg.contains("LinearGradient"),
            "flat keeps the gradient shape so easing back up needs no reshape"
        );
    }

    #[test]
    fn base_apply_then_stock_apply_round_trips() {
        let s = spec("midnight");
        let tokens = crate::build(s);
        let mut theme = Theme::default();
        crate::skin::apply_tokens(&mut theme, &tokens);
        let stock = (
            theme.colors.input,
            theme.colors.border,
            theme.colors.button_hover,
            theme.colors.scrollbar_thumb,
        );

        BaseStyle::from_spec(s, &tokens).apply(&mut theme, &tokens);
        assert_ne!(theme.colors.border, stock.1);
        assert_ne!(theme.colors.button_hover, stock.2);
        assert_ne!(theme.colors.scrollbar_thumb, stock.3);

        crate::skin::apply_tokens(&mut theme, &tokens);
        assert_eq!(
            (
                theme.colors.input,
                theme.colors.border,
                theme.colors.button_hover,
                theme.colors.scrollbar_thumb
            ),
            stock
        );
        assert!(theme.tokens.button.background.as_solid().is_some());
    }

    #[test]
    fn base_apply_lays_surface_gradients() {
        let s = spec("midnight");
        let tokens = crate::build(s);
        let mut theme = Theme::default();
        crate::skin::apply_tokens(&mut theme, &tokens);
        BaseStyle::from_spec(s, &tokens).apply(&mut theme, &tokens);

        for (name, background) in [
            ("button", theme.tokens.button.background),
            ("button hover", theme.tokens.button_hover.background),
            ("button active", theme.tokens.button_active.background),
            ("button primary", theme.tokens.button_primary.background),
            (
                "button primary hover",
                theme.tokens.button_primary_hover.background,
            ),
            (
                "button primary active",
                theme.tokens.button_primary_active.background,
            ),
            (
                "button danger hover",
                theme.tokens.button_danger_hover.background,
            ),
            (
                "button danger active",
                theme.tokens.button_danger_active.background,
            ),
            ("scrollbar thumb", theme.tokens.scrollbar_thumb.background),
            ("slider thumb", theme.tokens.slider_thumb.background),
            ("tab bar", theme.tokens.tab_bar.background),
            ("table head", theme.tokens.table_head.background),
            ("progress", theme.tokens.progress_bar.background),
            ("window", theme.tokens.background.background),
        ] {
            assert!(
                background.as_solid().is_none(),
                "{name} should be a gradient: {background:?}"
            );
        }

        // The gradient stops are deliberately different, so the face reads
        // as lit from above rather than as a flat fill.
        let debug = format!("{:?}", theme.tokens.button.background);
        assert!(debug.contains("LinearGradient"), "{debug}");
    }
}
