//! Theme-driven button halos: hover glow, denser press glow, zero styling.
//!
//! gpui-kit has no per-state shadow slot — a stock [`Button`] owns its
//! hover/active background, border and foreground, but its shadow is a single
//! boolean. [`GlowButton::glow`] wraps any stock button so it picks up the
//! theme's halo behaviour with one call and no extra view state:
//!
//! ```ignore
//! use lace_gpui::button::GlowButton;
//!
//! Button::new("save")
//!     .label("Save")
//!     .primary()
//!     .on_click(|_, _, _, _| {})
//!     .glow("save", false, window, cx)
//! ```
//!
//! Hover is pure style: the button's own group drives a `group_hover` halo on
//! itself. Presses are tracked in a keyed `bool` owned by the helper, so the
//! application view needs no new fields — updating it refreshes the view
//! through the keyed-state subscription. Pass `disabled = true` for inert
//! buttons (a disabled button never glows); ghost/link/text variants have no
//! face, so leaving them bare is the intended look.
//!
//! Inside tightly-composed components — [`Popover`] triggers, dialog footers —
//! a wrapper div would disturb the composed layout or the builder may not
//! offer a view for keyed state, so [`GlowButton::glow_hover`] paints only
//! the hover halo, with no wrapper at all, for those spots. `ButtonGroup`
//! segments stay stock buttons on purpose: styling their children trips a
//! framework stack overflow (see `examples/button-group-repro.rs`), so the
//! gallery leaves them bare pending an upstream fix.
//!
//! [`Button`]: gpui_kit::component::button::Button
//! [`Popover`]: gpui_kit::component::popover::Popover

use gpui_kit::component::ActiveTheme;
use gpui_kit::component::button::Button;
use gpui_kit::gpui::{
    App, BoxShadow, Div, ElementId, Hsla, InteractiveElement, MouseButton, MouseDownEvent,
    MouseUpEvent, ParentElement, SharedString, Styled, Window, div, px,
};
use gpui_kit::prelude::FluentBuilder;

/// Hover halo: tight enough to read as density, not haze.
const HOVER_BLUR: f32 = 8.0;
/// Hover halo strength.
const HOVER_ALPHA: f32 = 0.30;
/// Press halo: smaller blur, higher alpha — denser and brighter.
const PRESS_BLUR: f32 = 5.0;
/// Press halo strength.
const PRESS_ALPHA: f32 = 0.45;

/// A blurred accent halo. `BoxShadow` is gpui's only blur primitive; with no
/// offset and an accent colour it reads as a glow.
fn halo(color: Hsla, blur: f32, alpha: f32) -> BoxShadow {
    BoxShadow::new(px(0.), px(0.), color.opacity(alpha)).blur_radius(px(blur))
}

/// An accent hover halo, plus a denser one while pressed, for any stock
/// button — behaviour and style straight from the theme.
pub trait GlowButton: Sized {
    /// Hover-only halo, pure style with no wrapper: the button's own group
    /// (named `lace-glow-{id}`) drives a `group_hover` halo on itself. `id`
    /// must be unique per button, or hovering one would light them all.
    /// `glow` is the halo colour — normally the theme's primary. Disabled
    /// buttons render bare.
    ///
    /// Use this inside composed layouts (dialog footers, [`Popover`]
    /// triggers) where a wrapper div would disturb the geometry or where no
    /// view is available for press tracking; everywhere else prefer
    /// [`GlowButton::glow`], which adds the press halo too.
    ///
    /// [`Popover`]: gpui_kit::component::popover::Popover
    fn glow_hover(self, id: &str, disabled: bool, glow: Hsla) -> Button;

    /// Hover halo plus a denser, brighter press halo, with no per-button
    /// definitions beyond this call: the hover halo is style-only, and the
    /// press is tracked in a keyed `bool` owned by the helper, so the
    /// application view needs no new fields. `id` must be unique per button.
    /// Disabled buttons render bare — they never glow.
    ///
    /// The wrapper is a plain flex row around a hugging inner div, so it
    /// stays layout-neutral in row, column and block parents, inserts no
    /// focus or accessibility nodes of its own, and leaves the stock button
    /// underneath to keep its click, focus and keyboard handling. Do not use
    /// this inside dialog builders (no view for the keyed press state) or on
    /// `ButtonGroup` segments (framework stack overflow) — use
    /// [`GlowButton::glow_hover`] there instead.
    fn glow(self, id: &str, disabled: bool, window: &mut Window, cx: &mut App) -> Div;
}

impl GlowButton for Button {
    fn glow_hover(self, id: &str, disabled: bool, glow: Hsla) -> Button {
        if disabled {
            return self;
        }
        let group: SharedString = format!("lace-glow-{id}").into();
        self.group(group.clone())
            .group_hover(group, move |s| {
                s.shadow(vec![halo(glow, HOVER_BLUR, HOVER_ALPHA)])
            })
    }

    fn glow(self, id: &str, disabled: bool, window: &mut Window, cx: &mut App) -> Div {
        if disabled {
            return div().child(self);
        }
        let _ = (id, window);
        let primary = cx.theme().primary;
        // No-op handlers: no keyed state, no listener_for, no press shadow.
        // If this crashes, handler attachment/hitboxes alone are the trigger.
        let button = self
            .glow_hover(id, false, primary)
            .on_mouse_down(MouseButton::Left, |_, _, _| {})
            .on_mouse_up(MouseButton::Left, |_, _, _| {})
            .on_mouse_up_out(MouseButton::Left, |_, _, _| {});
        div().child(button)
    }
}
