//! The modern look: the shared geometry floors and elevation shadows both
//! paint layers read — the LaceStyle widgets (`lace-widgets`) and the dock
//! skin ([`skin`](crate::skin), [`edge_tabs`](crate::edge_tabs),
//! [`sidebar`](crate::sidebar), [`floating`](crate::floating)).
//!
//! The theme data stays Lace's: [`ThemeSpec`](crate::ThemeSpec) and
//! [`Tokens`](crate::Tokens) keep their radii, and every contrast rule is
//! measured on the same colours. This module only decides how those tokens
//! are drawn:
//!
//! - **Radius floors** ([`control_radius`], [`frame_radius`],
//!   [`card_radius`], [`tab_radius`]): a theme may round further, never less.
//! - **Elevation** ([`Elevation`], [`shadow`]): soft, layered drop shadows
//!   for raised faces, popups and dock cards, tuned for light and dark
//!   canvases. Shadows sit outside a face and add no contrast of their own,
//!   so the outline floors still carry every edge.
//!
//! Everything here is plain numbers and [`Rgba`], with no gpui, so the
//! values are unit-testable and usable from either layer.

use crate::color::Rgba;

/// Smallest corner radius of a control (button, field, combo), px.
pub const CONTROL_RADIUS: f64 = 6.0;
/// Smallest corner radius of a frame (group box, item view, text edit), px.
pub const FRAME_RADIUS: f64 = 8.0;
/// Smallest corner radius of a dock card and the slide-out sidebar panel, px.
pub const CARD_RADIUS: f64 = 10.0;
/// Smallest top-corner radius of a dock tab, px.
pub const TAB_RADIUS: f64 = 6.0;
/// Corner radius of the dock's ghost title buttons, px.
pub const CHROME_BUTTON_RADIUS: f64 = 5.0;
/// Inset of a tab's indicator pill from the tab's own edges, px.
pub const INDICATOR_INSET: f64 = 8.0;
/// Thinnest indicator pill, px.
pub const INDICATOR_MIN: f64 = 2.0;

/// A control radius with the modern floor applied.
pub fn control_radius(r: f64) -> f64 {
    r.max(CONTROL_RADIUS)
}

/// A frame radius with the modern floor applied.
pub fn frame_radius(r: f64) -> f64 {
    r.max(FRAME_RADIUS)
}

/// A dock-card radius with the modern floor applied.
pub fn card_radius(r: f64) -> f64 {
    r.max(CARD_RADIUS)
}

/// A dock-tab radius with the modern floor applied.
pub fn tab_radius(r: f64) -> f64 {
    r.max(TAB_RADIUS)
}

/// How high a surface sits above what is behind it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Elevation {
    /// A raised face: button, knob, switch thumb.
    Raised,
    /// A dock card in its gap.
    Card,
    /// A popup, menu, tooltip, the slide-out sidebar panel, a floating pane.
    Overlay,
}

/// One drop shadow layer: colour, vertical offset, blur, spread (px).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shadow {
    /// Shadow colour (black at some alpha).
    pub color: Rgba,
    /// Downward offset, px.
    pub offset_y: f32,
    /// Blur radius, px.
    pub blur: f32,
    /// Spread, px (negative pulls the shadow in under the face).
    pub spread: f32,
}

const fn black(alpha: u8) -> Rgba {
    [0, 0, 0, alpha]
}

/// The shadow layers for `level` on a light or dark canvas, tight key
/// shadow first, then the soft ambient one. Dark canvases need denser
/// shadows to read at all; light ones stay airy.
pub fn shadow(level: Elevation, is_light: bool) -> [Shadow; 2] {
    let (key, ambient) = match (level, is_light) {
        (Elevation::Raised, true) => ((18, 1.0, 2.0, 0.0), (10, 1.0, 3.0, 0.0)),
        (Elevation::Raised, false) => ((60, 1.0, 2.0, 0.0), (28, 1.0, 4.0, 0.0)),
        (Elevation::Card, true) => ((14, 1.0, 3.0, 0.0), (12, 4.0, 14.0, -2.0)),
        (Elevation::Card, false) => ((70, 1.0, 3.0, 0.0), (60, 6.0, 18.0, -2.0)),
        (Elevation::Overlay, true) => ((26, 2.0, 6.0, 0.0), (30, 10.0, 28.0, -4.0)),
        (Elevation::Overlay, false) => ((96, 2.0, 6.0, 0.0), (110, 12.0, 32.0, -4.0)),
    };
    let layer = |(a, y, b, s): (u8, f32, f32, f32)| Shadow { color: black(a), offset_y: y, blur: b, spread: s };
    [layer(key), layer(ambient)]
}

/// The soft accent halo around a focused field: the accent at this alpha,
/// spread by [`FOCUS_HALO_SPREAD`] px outside the field's focus outline.
pub const FOCUS_HALO_ALPHA: f64 = 0.28;
/// Spread of the focused-field halo, px.
pub const FOCUS_HALO_SPREAD: f32 = 3.0;

/// The halo colour for an accent: the accent at [`FOCUS_HALO_ALPHA`].
pub fn focus_halo(accent: Rgba) -> Rgba {
    let mut c = accent;
    c[3] = (f64::from(accent[3]) * FOCUS_HALO_ALPHA).round() as u8;
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floors_only_raise() {
        assert_eq!(control_radius(4.0), CONTROL_RADIUS);
        assert_eq!(control_radius(9.0), 9.0);
        assert_eq!(frame_radius(2.0), FRAME_RADIUS);
        assert_eq!(card_radius(12.0), 12.0);
        assert_eq!(tab_radius(0.0), TAB_RADIUS);
    }

    #[test]
    fn dark_shadows_are_denser_and_overlays_float_highest() {
        for level in [Elevation::Raised, Elevation::Card, Elevation::Overlay] {
            let light = shadow(level, true);
            let dark = shadow(level, false);
            assert!(dark[0].color[3] > light[0].color[3], "{level:?}");
        }
        let raised = shadow(Elevation::Raised, false)[1];
        let overlay = shadow(Elevation::Overlay, false)[1];
        assert!(overlay.blur > raised.blur && overlay.offset_y > raised.offset_y);
    }

    #[test]
    fn halo_keeps_the_accent_hue() {
        let h = focus_halo([10, 120, 220, 255]);
        assert_eq!(&h[..3], &[10, 120, 220]);
        assert!(h[3] > 0 && h[3] < 128);
    }
}
