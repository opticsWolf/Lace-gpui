//! Outer edges for filled stock buttons, derived from the fill.
//!
//! Stock gpui-kit draws its 1px border only for `Default` and `outline`
//! buttons — and for filled variants the border token equals the fill, so
//! even where a border exists it is invisible. But [`Button`] implements
//! `Styled`, and its `render` applies the caller's own modifiers last
//! (`refine_style`), so setting widths + color from the outside wins at
//! rest without touching stock internals (hover/active keep their stock
//! state colors on top).
//!
//! [`edged`] is the one-call version: hand it a stock button and the fill
//! it already wears (usually a theme token), and it returns the same button
//! with a readable face, a darker derived edge, and legible text.

use gpui_kit::component::button::Button;
use gpui_kit::gpui::{black, white, Hsla};
use gpui_kit::Styled;

/// Edge strength: share of the fill's lightness the outer edge keeps.
/// 0.62 reads as a defined rim on both dark and light fills without going
/// black on already-dark faces.
const EDGE_LIGHTNESS: f32 = 0.62;

/// Lightness lift toward white for hover, as a share of the remaining
/// headroom — visible on dark fills, subtle on light ones.
const HOVER_LIFT: f32 = 0.12;

/// Lightness kept while pressed.
const PRESS_KEEP: f32 = 0.85;

/// Lightness above which dark text wins over white text.
const TEXT_CUTOFF: f32 = 0.55;

/// A darker rim derived from `fill`: same hue and saturation, scaled
/// lightness, alpha untouched.
pub fn edge_color(fill: Hsla) -> Hsla {
    Hsla {
        l: (fill.l * EDGE_LIGHTNESS).clamp(0.0, 1.0),
        ..fill
    }
}

/// Readable text on `fill`: black on light fills, white on dark ones.
pub fn on_fill(fill: Hsla) -> Hsla {
    if fill.l > TEXT_CUTOFF {
        black()
    } else {
        white()
    }
}

/// Hover face derived from `fill`: lifted toward white, edge and text stay
/// matched to the new face.
pub fn hover_fill(fill: Hsla) -> Hsla {
    Hsla {
        l: (fill.l + (1.0 - fill.l) * HOVER_LIFT).clamp(0.0, 1.0),
        ..fill
    }
}

/// Pressed face derived from `fill`: darkened, edge and text stay matched.
pub fn press_fill(fill: Hsla) -> Hsla {
    Hsla {
        l: (fill.l * PRESS_KEEP).clamp(0.0, 1.0),
        ..fill
    }
}

/// Give a filled stock button an outer edge: `fill` is painted, the edge is
/// [`edge_color`] of it, text is [`on_fill`] of it. Pass the same color the
/// variant already wears (e.g. `cx.theme().primary`) — the button keeps its
/// stock hover/active behavior, only the resting edge is added.
pub fn edged(button: Button, fill: Hsla) -> Button {
    button
        .bg(fill)
        .border_1()
        .border_color(edge_color(fill))
        .text_color(on_fill(fill))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_is_darker_same_hue() {
        let fill = Hsla {
            h: 0.6,
            s: 0.8,
            l: 0.5,
            a: 1.0,
        };
        let edge = edge_color(fill);
        assert_eq!(edge.h, fill.h);
        assert_eq!(edge.s, fill.s);
        assert_eq!(edge.a, fill.a);
        assert!((edge.l - 0.31).abs() < 1e-6);
        assert!(edge.l < fill.l);
    }

    #[test]
    fn edge_clamps_black() {
        let edge = edge_color(black());
        assert_eq!(edge.l, 0.0);
    }

    #[test]
    fn text_picks_contrast() {
        let dark = Hsla {
            h: 0.6,
            s: 0.8,
            l: 0.3,
            a: 1.0,
        };
        let light = Hsla {
            h: 0.1,
            s: 0.9,
            l: 0.7,
            a: 1.0,
        };
        assert_eq!(on_fill(dark), white());
        assert_eq!(on_fill(light), black());
    }

    #[test]
    fn hover_lifts_press_darkens() {
        let fill = Hsla {
            h: 0.6,
            s: 0.8,
            l: 0.5,
            a: 1.0,
        };
        let hover = hover_fill(fill);
        let press = press_fill(fill);
        assert!(hover.l > fill.l, "hover must lift");
        assert!(press.l < fill.l, "press must darken");
        assert_eq!((hover.h, hover.s, hover.a), (fill.h, fill.s, fill.a));
        assert!((press.l - 0.425).abs() < 1e-6);
    }
}
