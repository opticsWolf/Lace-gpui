//! Margins around a widget that fills its parent (an item view's editor or
//! button delegate, Qt's `option.rect` less a margin).

use gpui_kit::gpui::{div, px, Div, IntoElement, ParentElement, Styled};

/// Space left free on each side, px.
///
/// Converts from one number (all four sides), two (`(horizontal,
/// vertical)`) or four (`[left, top, right, bottom]` or the tuple):
///
/// ```ignore
/// editor.fill();                       // no margin
/// editor.fill_with(1.0);               // 1 px all around
/// editor.fill_with((0.0, 2.0));        // 2 px above and below
/// editor.fill_with([4.0, 1.0, 4.0, 1.0]);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Margin {
    /// Left, px.
    pub left: f32,
    /// Top, px.
    pub top: f32,
    /// Right, px.
    pub right: f32,
    /// Bottom, px.
    pub bottom: f32,
}

impl Margin {
    /// No margin.
    pub const ZERO: Self = Self { left: 0.0, top: 0.0, right: 0.0, bottom: 0.0 };

    /// The same margin on every side.
    pub const fn all(v: f32) -> Self {
        Self { left: v, top: v, right: v, bottom: v }
    }

    /// Left, top, right, bottom.
    pub const fn new(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self { left, top, right, bottom }
    }

    /// Whether every side is zero.
    pub fn is_zero(&self) -> bool {
        *self == Self::ZERO
    }

    /// A box filling its parent that holds `el` this far from its edges.
    pub fn wrap(self, el: impl IntoElement) -> Div {
        div()
            .size_full()
            .flex()
            .pl(px(self.left))
            .pt(px(self.top))
            .pr(px(self.right))
            .pb(px(self.bottom))
            .child(el)
    }
}

impl From<f32> for Margin {
    fn from(v: f32) -> Self {
        Self::all(v)
    }
}

impl From<(f32, f32)> for Margin {
    /// `(horizontal, vertical)`.
    fn from((h, v): (f32, f32)) -> Self {
        Self::new(h, v, h, v)
    }
}

impl From<[f32; 4]> for Margin {
    /// `[left, top, right, bottom]`.
    fn from([l, t, r, b]: [f32; 4]) -> Self {
        Self::new(l, t, r, b)
    }
}

impl From<(f32, f32, f32, f32)> for Margin {
    /// `(left, top, right, bottom)`.
    fn from((l, t, r, b): (f32, f32, f32, f32)) -> Self {
        Self::new(l, t, r, b)
    }
}
