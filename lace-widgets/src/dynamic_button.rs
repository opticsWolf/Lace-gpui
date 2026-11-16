//! A text button that collapses to a square icon button when its share of
//! horizontal space runs out, with hysteresis so it never jitters.
//!
//! gpui lays out in one pass, so a button cannot measure what it was granted
//! and react in the same frame. Instead the owner feeds each button the
//! available width (here: the viewport width — resize the window and watch
//! the toolbar below) and keeps one `bool` per button. [`DynamicButton::show_label`]
//! is the whole decision: collapse below `collapse_at`, re-expand only above
//! `collapse_at + HYSTERESIS`. Sitting exactly on a threshold — or a
//! scrollbar appearing and stealing a dozen pixels — stays inside the dead
//! band instead of flip-flopping the row every frame.
//!
//! Hovering shows a tooltip: the label by default, or [`DynamicButton::tooltip`]
//! when the collapsed icon needs more explanation than the label gives.
//!
//! ```ignore
//! use lace_widgets::DynamicButton;
//!
//! // View state: one flag per button, initialised to `false` (expanded).
//! struct Toolbar {
//!     buttons: Vec<DynamicButton>,
//!     expanded: Vec<bool>,
//! }
//!
//! // In `render`, before building the row:
//! let width = f32::from(window.viewport_size().width);
//! for (i, b) in toolbar.buttons.iter().enumerate() {
//!     toolbar.expanded[i] = b.show_label(width, toolbar.expanded[i]);
//! }
//! let row = div().children(
//!     toolbar.buttons.iter().enumerate()
//!         .map(|(i, b)| b.render(toolbar.expanded[i], cx)),
//! );
//! ```

use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::{ActiveTheme, IconName};
use gpui_kit::gpui::{App, Hsla};
use gpui_kit::SharedString;
use gpui_kit::Styled;

use super::edge::{edge_color, on_fill};

/// Dead band, px, between the collapse and re-expand widths. Must exceed the
/// largest width wobble the layout can produce on its own (scrollbar
/// appearing/disappearing is ~12px); 48px leaves comfortable margin.
pub const HYSTERESIS: f32 = 48.0;

/// An icon+text button with its own collapse threshold.
pub struct DynamicButton {
    /// Stable element id, e.g. `"dyn-new"`.
    pub id: &'static str,
    /// Text shown while expanded.
    pub label: &'static str,
    /// Icon shown always; alone it makes the collapsed square button.
    pub icon: IconName,
    /// Viewport width below which this button collapses. Stagger these
    /// across a toolbar (rightmost lowest… or highest, your call) so buttons
    /// collapse one at a time instead of all at once.
    pub collapse_at: f32,
    /// Primary styling when `true`, secondary otherwise. Ignored while
    /// [`DynamicButton::fill`] is set — the override owns the face.
    pub primary: bool,
    /// Hover tooltip. `None` (the default) shows [`DynamicButton::label`];
    /// `Some` overrides it — useful when the collapsed icon alone is
    /// cryptic. `&'static str` so the whole struct stays `const`-buildable.
    pub tooltip: Option<&'static str>,
    /// Custom face color. `None` keeps the stock primary/secondary fill;
    /// `Some` paints it and derives the outer edge ([`edge_color`]) and the
    /// text ([`on_fill`]) from it, so edge and text always match the fill.
    /// `Option<Hsla>` keeps the whole struct `const`-buildable.
    pub fill: Option<Hsla>,
}

impl DynamicButton {
    /// Whether to show the label at `width` given the previous frame's
    /// state. Pure function of `(width, was_expanded)` — no window needed,
    /// so the hysteresis is unit-testable.
    pub fn show_label(&self, width: f32, was_expanded: bool) -> bool {
        if was_expanded {
            width >= self.collapse_at
        } else {
            width >= self.collapse_at + HYSTERESIS
        }
    }

    /// Build the stock button for this frame: icon+label, or icon only.
    /// Collapsed buttons always carry the hover text; expanded ones only
    /// when a custom [`DynamicButton::tooltip`] adds information — the
    /// default label would merely duplicate the visible text.
    ///
    /// With no [`DynamicButton::fill`] the stock face is left untouched so
    /// the button matches a regular primary/secondary button exactly; only
    /// the faint theme hairline is added. A `fill` override paints the face
    /// and derives edge and text from it. Hover/press keep the stock
    /// variant's state colors (gpui-kit sets the hover style once inside
    /// its render and stateless buttons expose no press hook).
    pub fn render(&self, expanded: bool, cx: &App) -> Button {
        let mut button = Button::new(self.id).icon(self.icon.clone());
        if let Some(tip) = self.tooltip_for(expanded) {
            button = button.tooltip(tip);
        }
        let button = if self.primary {
            button.primary()
        } else {
            button.secondary()
        };
        // Caller modifiers win over stock internals (`refine_style` runs
        // last) — but only at rest. State styles refine on top, so
        // hover/press deliberately stay stock.
        let button = match self.fill {
            Some(fill) => button
                .bg(fill)
                .border_1()
                .border_color(edge_color(fill))
                .text_color(on_fill(fill)),
            None => button.border_1().border_color(cx.theme().border),
        };
        if expanded {
            button.label(self.label)
        } else {
            button
        }
    }

    /// The face this button wears given the stock `stock` fill: the
    /// override, or the variant's own fill. Pair with
    /// [`edged`](super::edge::edged) to give stock-fill buttons the same
    /// outer edge without repainting their face.
    pub fn effective_fill(&self, stock: Hsla) -> Hsla {
        self.fill.unwrap_or(stock)
    }

    /// The hover text for this button: the override, or the label.
    pub fn tooltip_text(&self) -> SharedString {
        self.tooltip.unwrap_or(self.label).into()
    }

    /// Hover text for this frame, or `None` when it would duplicate the
    /// visible label (expanded with default tooltip).
    pub fn tooltip_for(&self, expanded: bool) -> Option<SharedString> {
        if expanded && self.tooltip.is_none() {
            None
        } else {
            Some(self.tooltip_text())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn button(at: f32) -> DynamicButton {
        DynamicButton {
            id: "t",
            label: "T",
            icon: IconName::Plus,
            collapse_at: at,
            primary: false,
            tooltip: None,
            fill: None,
        }
    }

    #[test]
    fn tooltip_defaults_to_label() {
        assert_eq!(button(800.0).tooltip_text().to_string(), "T");
        let mut b = button(800.0);
        b.tooltip = Some("Create a new thing");
        assert_eq!(b.tooltip_text().to_string(), "Create a new thing");
    }

    #[test]
    fn tooltip_hidden_when_label_visible() {
        let plain = button(800.0);
        // Expanded with default text: no tooltip, it would echo the label.
        assert!(plain.tooltip_for(true).is_none());
        // Collapsed: the label is gone, so the tooltip stands in.
        assert_eq!(plain.tooltip_for(false).unwrap().to_string(), "T");

        let mut custom = button(800.0);
        custom.tooltip = Some("Create a new thing");
        // Custom text adds information, so it shows in both states.
        assert_eq!(
            custom.tooltip_for(true).unwrap().to_string(),
            "Create a new thing"
        );
        assert_eq!(
            custom.tooltip_for(false).unwrap().to_string(),
            "Create a new thing"
        );
    }

    #[test]
    fn hysteresis_dead_band() {
        let b = button(800.0);
        // Expanded: stays expanded right down to the threshold.
        assert!(b.show_label(900.0, true));
        assert!(b.show_label(800.0, true));
        // Collapses just below it.
        assert!(!b.show_label(799.0, true));
        // Collapsed: hovering at or just above the threshold does NOT expand.
        assert!(!b.show_label(800.0, false));
        assert!(!b.show_label(800.0 + HYSTERESIS - 1.0, false));
        // Only the full dead band above re-expands.
        assert!(b.show_label(800.0 + HYSTERESIS, false));
    }

    #[test]
    fn no_jitter_on_small_wobble() {
        let b = button(800.0);
        // Simulate a scrollbar flickering the width ±12px around the edge.
        let mut expanded = true;
        let mut flips = 0;
        let mut widths = vec![810.0; 4];
        widths.extend([798.0, 805.0, 799.0, 806.0, 798.0, 850.0]);
        for w in widths {
            let next = b.show_label(w, expanded);
            if next != expanded {
                flips += 1;
                expanded = next;
            }
        }
        assert_eq!(flips, 2, "one collapse, one re-expand, no flicker");
        assert!(expanded);
    }
}
