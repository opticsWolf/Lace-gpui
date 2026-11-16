//! Shared paint rules — the port of `lace/style/_paint.py`.
//!
//! Two rules every family follows, as in Lace:
//!
//! - **Colours only from the palette**, in the group the state asks for
//!   ([`LaceStyle::pal`]). Nothing reads the theme engine directly.
//! - **Vector only.** Glyphs are stroked paths ([`crate::glyph`]), never
//!   icons, so they stay sharp at any scale.
//!
//! Colour derivations are pure functions of a few fixed theme colours, so
//! each result is memoised, as Lace's `lru_cache` does.

use std::cell::RefCell;
use std::collections::HashMap;

use lace_gpui::color::{self, Rgba};

use super::LaceStyle;

/// Default stroke strength: `Text` mixed over a fill by this much.
pub const STROKE_MIX: f64 = 0.22;
/// Hover fill: OKLCH lightness step toward contrast.
pub const HOVER_STEP: f64 = 0.04;
/// Pressed fill: OKLCH lightness step toward contrast.
pub const PRESS_STEP: f64 = 0.08;
/// Glyph pen width (chevrons, check marks), px.
pub const GLYPH_PEN: f32 = 1.5;
/// Contrast floor of a disabled control's outline: muted, but its shape shows.
pub const DISABLED_RATIO: f64 = 1.5;

/// `scaled_radius`: share of the size at the reference control radius.
const RADIUS_SHARE: f32 = 0.22;
const RADIUS_REF: f32 = 4.0;
const RADIUS_MAX: f32 = 0.30;

/// A control's interaction state — the `QStyle.State` flags LaceStyle reads.
///
/// Widgets fill it from their own hover/press/focus tracking; the gallery
/// forces it, the way Lace's gallery hand-builds style options.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct State {
    /// `State_MouseOver`.
    pub hovered: bool,
    /// `State_Sunken`.
    pub pressed: bool,
    /// `State_HasFocus` from the keyboard (`State_KeyboardFocusChange`).
    pub focused: bool,
    /// `State_On`.
    pub checked: bool,
    /// `State_NoChange` (a partially checked box).
    pub partial: bool,
    /// `State_Selected`.
    pub selected: bool,
    /// Not `State_Enabled`.
    pub disabled: bool,
}

impl State {
    /// At rest.
    pub const NORMAL: Self = Self {
        hovered: false,
        pressed: false,
        focused: false,
        checked: false,
        partial: false,
        selected: false,
        disabled: false,
    };

    /// Under the pointer.
    pub const fn hover() -> Self {
        Self { hovered: true, ..Self::NORMAL }
    }

    /// Pressed under the pointer.
    pub const fn pressed() -> Self {
        Self { hovered: true, pressed: true, ..Self::NORMAL }
    }

    /// Focused from the keyboard.
    pub const fn focused() -> Self {
        Self { focused: true, ..Self::NORMAL }
    }

    /// Checked (on).
    pub const fn checked() -> Self {
        Self { checked: true, ..Self::NORMAL }
    }

    /// Disabled.
    pub const fn disabled() -> Self {
        Self { disabled: true, ..Self::NORMAL }
    }

    /// Whether the control takes input.
    pub fn enabled(self) -> bool {
        !self.disabled
    }

    /// Hovered and enabled.
    pub fn active_hover(self) -> bool {
        self.hovered && !self.disabled
    }

    /// Keyboard focus on an enabled control.
    pub fn has_focus(self) -> bool {
        self.focused && !self.disabled
    }

    /// The same state with `checked` set to `on`.
    pub fn with_checked(mut self, on: bool) -> Self {
        self.checked = on;
        self
    }

    /// The same state with `disabled` set.
    pub fn with_disabled(mut self, disabled: bool) -> Self {
        // Adds to the state: a forced or tracked disabled state stays disabled.
        self.disabled |= disabled;
        self
    }

    /// The same state with `selected` set.
    pub fn with_selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// The same state with `partial` set.
    pub fn with_partial(mut self, partial: bool) -> Self {
        self.partial = partial;
        self
    }

    /// The state merged with `other`'s flags (either set → set).
    pub fn union(self, other: Self) -> Self {
        Self {
            hovered: self.hovered || other.hovered,
            pressed: self.pressed || other.pressed,
            focused: self.focused || other.focused,
            checked: self.checked || other.checked,
            partial: self.partial || other.partial,
            selected: self.selected || other.selected,
            disabled: self.disabled || other.disabled,
        }
    }
}

// ---------------------------------------------------------------------------
// Memo
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Op {
    Mix,
    Step,
    On,
    Ensure,
}

type Key = (Op, Rgba, Rgba, Rgba, u64);

thread_local! {
    static MEMO: RefCell<HashMap<Key, Rgba>> = RefCell::new(HashMap::new());
}

/// Memo size cap; a theme switch fills it with a new set, so it is simply
/// cleared when it grows past this.
const MEMO_CAP: usize = 8192;

fn memo(key: Key, f: impl FnOnce() -> Rgba) -> Rgba {
    if let Some(hit) = MEMO.with(|m| m.borrow().get(&key).copied()) {
        return hit;
    }
    let value = f();
    MEMO.with(|m| {
        let mut m = m.borrow_mut();
        if m.len() >= MEMO_CAP {
            m.clear();
        }
        m.insert(key, value);
    });
    value
}

const NONE: Rgba = [0, 0, 0, 0];

// ---------------------------------------------------------------------------
// Colour
// ---------------------------------------------------------------------------

/// `a` blended toward `b` by `t`, in OKLab.
pub fn mix(a: Rgba, b: Rgba, t: f64) -> Rgba {
    memo((Op::Mix, a, b, NONE, t.to_bits()), || color::mix(a, b, t))
}

/// `c` lightened (dark colours) or darkened (light ones) by `amount` of OKLCH L.
pub fn step(c: Rgba, amount: f64) -> Rgba {
    memo((Op::Step, c, NONE, NONE, amount.to_bits()), || {
        color::step_contrast(c, amount)
    })
}

/// Readable glyph colour on `c`: the palette's highlighted or body text,
/// whichever reads, lifted to 3:1.
pub fn on(style: &LaceStyle, s: State, c: Rgba) -> Rgba {
    let pal = style.pal(s);
    let prefer = [pal.highlighted_text, pal.text];
    memo((Op::On, c, prefer[0], prefer[1], 0), || {
        color::on_color(c, prefer, 3.0)
    })
}

/// `fg` lifted to `ratio` on `bg`.
pub fn ensure(fg: Rgba, bg: Rgba, ratio: f64) -> Rgba {
    memo((Op::Ensure, fg, bg, NONE, ratio.to_bits()), || {
        color::ensure_contrast(fg, bg, ratio)
    })
}

/// The 1 px outline of a face filled with `fill`: text mixed over it by the
/// theme's outline strength.
pub fn stroke(style: &LaceStyle, s: State, fill: Rgba) -> Rgba {
    mix(fill, style.pal(s).text, style.outline_strength)
}

/// `c` lifted to `ratio` against `surface` (default `Window`).
///
/// Disabled controls are exempt from the target, as in WCAG, but keep
/// [`DISABLED_RATIO`] so their shape stays visible.
pub fn legible(style: &LaceStyle, s: State, c: Rgba, ratio: f64, surface: Option<Rgba>) -> Rgba {
    let ratio = if s.disabled { ratio.min(DISABLED_RATIO) } else { ratio };
    let bg = surface.unwrap_or(style.pal(s).window);
    ensure(c, bg, ratio)
}

/// `fill` with the hover or pressed lightness step the state asks for. A
/// checked control under the pointer takes the pressed step, as in Lace.
pub fn state_fill(s: State, fill: Rgba) -> Rgba {
    if s.disabled {
        return fill;
    }
    if s.pressed || (s.checked && s.hovered) {
        return step(fill, PRESS_STEP);
    }
    if s.hovered {
        return step(fill, HOVER_STEP);
    }
    fill
}

/// The accent, in the state's group.
pub fn accent(style: &LaceStyle, s: State) -> Rgba {
    style.pal(s).accent
}

/// A corner radius that scales with `size` and follows `control_radius`:
/// `size * min(0.22 * control_radius / 4, 0.30)`; 0 stays square.
pub fn scaled_radius(style: &LaceStyle, size: f32) -> f32 {
    let share = RADIUS_SHARE * style.control_radius / RADIUS_REF;
    size * share.min(RADIUS_MAX)
}

/// The focus ring colour: the accent at the `ui` floor.
pub fn focus_ring(style: &LaceStyle, s: State) -> Rgba {
    legible(style, s, accent(style, s), style.ui_ratio(), None)
}

/// Muted text: window text mixed halfway over the window, held to the
/// `muted` floor (`_chrome.muted`).
pub fn muted(style: &LaceStyle, s: State) -> Rgba {
    let pal = style.pal(s);
    let c = mix(pal.window, pal.window_text, 0.5);
    legible(style, s, c, style.muted_ratio(), None)
}
