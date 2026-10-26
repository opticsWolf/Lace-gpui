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
//! The derivation is pure — [`BaseStyle::from_tokens`] and
//! [`BaseStyle::from_spec`] need no gpui context. [`BaseStyle::apply`] is the
//! write half used by [`crate::skin::apply_styled`].

use gpui_kit::component::theme::{Theme, ThemeTokens};
use gpui_kit::gpui::px;

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

/// LaceStyle's widget-level rules, derived from one [`Tokens`] palette.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BaseStyle {
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
    /// Outline of an unfocused input field.
    pub field_outline_color: Rgba,
    /// Accent, as used for a default button's face and a slider's fill.
    pub accent: Rgba,
    /// Accent face on hover.
    pub accent_hover: Rgba,
    /// Accent face while pressed.
    pub accent_active: Rgba,
    /// Accent focus ring, held to [`BaseStyle::ui_ratio`].
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

    fn derive(
        t: &Tokens,
        outline_level: Contrast,
        contrast: Contrast,
        control_radius: f64,
        corner_radius: f64,
    ) -> Self {
        let surface = t.canvas_bg;
        let button = t.button_bg;
        let accent = t.accent_color;
        let border_ratio = contrast_target(Role::Border, outline_level);
        let ui_ratio = contrast_target(Role::Ui, contrast);
        let stroke = |fill: Rgba| color::mix(fill, t.text, t.outline_strength);
        let legible = |c: Rgba, ratio: f64| color::ensure_contrast(c, surface, ratio);
        let mix_text = |amount: f64| color::mix(surface, t.text, amount);

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
            focus_width: t.focus_width.max(0.0) as f32,
            outline_strength: t.outline_strength,
            border_ratio,
            ui_ratio,
            field_outline: true,
            button,
            button_hover: color::step_contrast(button, HOVER_STEP),
            button_active: color::step_contrast(button, PRESS_STEP),
            button_outline: legible(stroke(button), border_ratio),
            field_outline_color: legible(stroke(t.input_bg), border_ratio),
            accent,
            accent_hover: color::step_contrast(accent, HOVER_STEP),
            accent_active: color::step_contrast(accent, PRESS_STEP),
            focus_ring: legible(accent, ui_ratio),
            scrollbar_track: mix_text(SCROLLBAR_TRACK),
            scrollbar_thumb: legible(mix_text(SCROLLBAR_REST), ui_ratio),
            scrollbar_thumb_hover: legible(mix_text(SCROLLBAR_HOVER), ui_ratio),
            slider_thumb: button,
        }
    }

    /// Write the rules gpui-kit can represent onto `theme`.
    ///
    /// The caller must rebuild gpui-kit's derived `ThemeTokens` and sync the
    /// base layer; [`crate::skin::apply_spec_styled`] does both.
    pub fn apply(&self, theme: &mut Theme) {
        theme.colors.input = if self.field_outline {
            hsla(self.field_outline_color)
        } else {
            theme.colors.border
        };
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
        // hover surfaces: +0.04 on hover, +0.08 pressed.
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

        // A default button's border is ThemeColor::input, and an input's
        // border is the same field slot; the field outline wins because it
        // also shapes the dark input fill (`input.mix(transparent, 0.3)`).
        theme.radius = px(self.control_radius);
        theme.radius_lg = px(self.corner_radius);
        theme.focus_ring = self.focus_width > 0.0;
        theme.tokens = ThemeTokens::from(&theme.colors);
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
            for (label, colour) in [
                ("button", b.button_outline),
                ("field", b.field_outline_color),
            ] {
                let ratio = color::contrast_ratio(colour, t.canvas_bg);
                assert!(
                    ratio + 1e-6 >= b.border_ratio,
                    "{name} {label} outline {ratio} < {}",
                    b.border_ratio
                );
            }
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
    fn popup_radius_bridges_control_and_card() {
        let b = BaseStyle::from_tokens(&crate::build(spec("cyberpunk_neon")));
        assert_eq!(b.control_radius, 4.0);
        assert_eq!(b.corner_radius, 10.0);
        assert_eq!(b.popup_radius, 6.0);
    }

    #[test]
    fn base_apply_then_stock_apply_round_trips() {
        let s = spec("midnight");
        let tokens = crate::build(s);
        let mut theme = Theme::default();
        crate::skin::apply_tokens(&mut theme, &tokens);
        let stock = (
            theme.colors.input,
            theme.colors.button_hover,
            theme.colors.scrollbar_thumb,
        );

        BaseStyle::from_spec(s, &tokens).apply(&mut theme);
        assert_ne!(theme.colors.button_hover, stock.1);
        assert_ne!(theme.colors.scrollbar_thumb, stock.2);
        assert_eq!(theme.tokens, ThemeTokens::from(&theme.colors));

        crate::skin::apply_tokens(&mut theme, &tokens);
        assert_eq!(
            (
                theme.colors.input,
                theme.colors.button_hover,
                theme.colors.scrollbar_thumb
            ),
            stock
        );
    }
}
