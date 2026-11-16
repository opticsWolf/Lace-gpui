//! Per-family colour rules — `_buttons`, `_inputs`, `_primitives`, `_range`,
//! `_containers` and `_chrome`, one function per thing LaceStyle paints.
//!
//! Every function is pure: style + state in, colours out. The widgets in
//! [`crate::controls`] only lay out and draw what these return.

use lace_gpui::color::Rgba;

use super::paint::{self as p, State};
use super::LaceStyle;

// ---------------------------------------------------------------------------
// Buttons (`_buttons.py`)
// ---------------------------------------------------------------------------

/// Accent share of a checked (toggled) button's fill.
pub const CHECKED_WASH: f64 = 0.28;

/// Which push-button face to draw.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonKind {
    /// A raised `Button` face with a 1 px outline.
    #[default]
    Normal,
    /// The dialog's default button: filled with the accent.
    Default,
    /// Flat / auto-raise: a face only while hovered or pressed.
    Flat,
}

/// A face: optional fill, optional outline, the label colour on it, and
/// whether keyboard focus replaces the outline with the accent ring.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Face {
    /// The face fill, if any.
    pub fill: Option<Rgba>,
    /// The 1 px outline, if any.
    pub line: Option<Rgba>,
    /// Label and glyph colour.
    pub text: Rgba,
    /// The focus ring, drawn inside the face, when focused from the keyboard.
    pub ring: Option<Rgba>,
}

/// `face_colors` plus the label colour and focus ring.
pub fn button_face(style: &LaceStyle, s: State, kind: ButtonKind) -> Face {
    let pal = style.pal(s);
    let base = pal.button;
    let acc = pal.accent;
    let active = (s.hovered || s.pressed) && s.enabled();
    let is_default = kind == ButtonKind::Default && s.enabled();
    let (fill, line, text) = if is_default {
        let fill = p::state_fill(s, acc);
        (Some(fill), Some(fill), p::on(style, s, fill))
    } else if s.checked {
        let fill = p::state_fill(s, p::mix(base, acc, CHECKED_WASH));
        let line = p::legible(style, s, acc, style.ui_ratio(), None);
        (Some(fill), Some(line), pal.button_text)
    } else if kind == ButtonKind::Flat {
        let fill = if active { Some(p::state_fill(s, base)) } else { None };
        (fill, None, pal.button_text)
    } else {
        let fill = p::state_fill(s, base);
        let line = p::legible(style, s, p::stroke(style, s, base), style.border_ratio(), None);
        (Some(fill), Some(line), pal.button_text)
    };
    let ring = (s.has_focus() && style.focus_width > 0.0).then(|| p::focus_ring(style, s));
    Face { fill, line, text, ring }
}

/// The divider between a split button's face and its menu part.
pub fn split_divider(style: &LaceStyle, s: State, face: &Face) -> Rgba {
    let base = face.fill.unwrap_or(style.pal(s).button);
    p::stroke(style, s, base)
}

// ---------------------------------------------------------------------------
// Indicators (`_primitives.py`)
// ---------------------------------------------------------------------------

/// Accent share of a partially checked box's fill.
pub const PARTIAL_WASH: f64 = 0.28;

/// A check box, radio or item check: fill, outline and glyph colour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Indicator {
    /// Box / disc fill.
    pub fill: Rgba,
    /// Outline.
    pub line: Rgba,
    /// Tick, dash or radio dot.
    pub glyph: Rgba,
}

/// What an indicator sits on: the selection on a selected row, else the window.
fn indicator_surface(style: &LaceStyle, s: State) -> Rgba {
    let pal = style.pal(s);
    if s.selected {
        pal.highlight
    } else {
        pal.window
    }
}

/// Outline of an unchecked indicator: accent on hover, else the stroke,
/// either lifted to the `ui` target against its surface.
fn indicator_outline(style: &LaceStyle, s: State, base: Rgba) -> Rgba {
    let line = if s.active_hover() {
        p::accent(style, s)
    } else {
        p::stroke(style, s, base)
    };
    p::legible(style, s, line, style.ui_ratio(), Some(indicator_surface(style, s)))
}

/// A check box in state `s` (`checked`, `partial`, or neither).
pub fn check_box(style: &LaceStyle, s: State) -> Indicator {
    let pal = style.pal(s);
    if s.checked {
        let fill = p::state_fill(s, pal.accent);
        let line = p::legible(style, s, fill, style.ui_ratio(), Some(indicator_surface(style, s)));
        Indicator { fill, line, glyph: p::on(style, s, fill) }
    } else if s.partial {
        let acc = pal.accent;
        let wash = p::state_fill(s, p::mix(pal.base, acc, PARTIAL_WASH));
        Indicator {
            fill: wash,
            line: p::legible(style, s, acc, style.ui_ratio(), None),
            glyph: p::legible(style, s, acc, style.ui_ratio(), Some(wash)),
        }
    } else {
        let base = p::state_fill(s, pal.base);
        Indicator { fill: base, line: indicator_outline(style, s, base), glyph: base }
    }
}

/// A radio button: `fill` is the disc, `line` its ring, `glyph` the dot.
pub fn radio(style: &LaceStyle, s: State) -> Indicator {
    let pal = style.pal(s);
    let base = p::state_fill(s, pal.base);
    if s.checked {
        let ring = p::state_fill(s, pal.accent);
        Indicator { fill: base, line: ring, glyph: ring }
    } else {
        Indicator { fill: base, line: indicator_outline(style, s, base), glyph: base }
    }
}

/// A menu's check mark / exclusive dot colour.
pub fn menu_check(style: &LaceStyle, s: State) -> Rgba {
    let pal = style.pal(s);
    if s.selected {
        pal.highlighted_text
    } else {
        pal.text
    }
}

/// Glyph colour of arrows, spin indicators and branch chevrons.
pub fn arrow(style: &LaceStyle, s: State) -> Rgba {
    style.pal(s).button_text
}

/// A tree's expand/collapse chevron.
pub fn branch(style: &LaceStyle, s: State) -> Rgba {
    style.pal(s).text
}

// ---------------------------------------------------------------------------
// Frames
// ---------------------------------------------------------------------------

/// The outline of a frame, group box, tab pane or `QFrame` line/box: the
/// window's stroke at the border floor.
pub fn frame_line(style: &LaceStyle, s: State) -> Rgba {
    line_on(style, s, style.pal(s).window)
}

/// A 1 px separation line on `surface` (`_containers._line`).
pub fn line_on(style: &LaceStyle, s: State, surface: Rgba) -> Rgba {
    p::legible(style, s, p::stroke(style, s, surface), style.border_ratio(), None)
}

// ---------------------------------------------------------------------------
// Inputs (`_inputs.py`)
// ---------------------------------------------------------------------------

/// A field: `Base` fill, outline (accent while focused), text colours.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Field {
    /// Fill.
    pub fill: Rgba,
    /// Outline; `None` when the theme turns field outlines off.
    pub line: Option<Rgba>,
    /// Text.
    pub text: Rgba,
    /// Placeholder text.
    pub placeholder: Rgba,
}

/// A line edit, text edit, editable combo or spin box field. Fields show
/// focus however it arrived (mouse or keyboard), as Qt's do.
pub fn field(style: &LaceStyle, s: State) -> Field {
    let pal = style.pal(s);
    let base = pal.base;
    let line = if s.has_focus() {
        Some(p::legible(style, s, pal.accent, style.ui_ratio(), None))
    } else if style.field_outline {
        Some(p::legible(style, s, p::stroke(style, s, base), style.border_ratio(), None))
    } else {
        None
    };
    Field { fill: base, line, text: pal.text, placeholder: pal.placeholder }
}

/// Hover / press tint of a combo arrow or spin button inside a field.
pub fn field_button_tint(style: &LaceStyle, s: State) -> Option<Rgba> {
    if !s.enabled() || !(s.hovered || s.pressed) {
        return None;
    }
    let base = style.pal(s).base;
    Some(p::step(base, if s.pressed { p::PRESS_STEP } else { p::HOVER_STEP }))
}

/// Glyph colour of a field's button (`_glyph_color`); `step_enabled` is
/// false for a spin button at its limit.
pub fn field_glyph(style: &LaceStyle, s: State, read_only: bool, step_enabled: bool) -> Rgba {
    let pal = style.pal(s);
    let c = if read_only { pal.text } else { pal.button_text };
    if s.enabled() && step_enabled {
        c
    } else {
        p::mix(pal.base, c, 0.35)
    }
}

/// The chevron on a non-editable combo's button face.
pub fn combo_glyph(style: &LaceStyle, s: State, face: &Face) -> Rgba {
    let glyph = style.pal(s).button_text;
    match face.fill {
        Some(fill) if s.disabled => p::mix(fill, glyph, 0.5),
        _ => glyph,
    }
}

// ---------------------------------------------------------------------------
// Range (`_range.py`)
// ---------------------------------------------------------------------------

/// Groove thickness of sliders and dials, px.
pub const GROOVE: f32 = 4.0;
/// Track colour: text mixed over the window by this much.
pub const TRACK_MIX: f64 = 0.16;
const DISABLED_TINT: f64 = 0.45;
const DISABLED_BLEND: f64 = 0.5;

fn floored(s: State, plain: Rgba, floored: Rgba) -> Rgba {
    if s.enabled() {
        floored
    } else {
        p::mix(plain, floored, DISABLED_BLEND)
    }
}

/// A slider groove / progress track / dial ring.
pub fn track(style: &LaceStyle, s: State) -> Rgba {
    let pal = style.pal(s);
    let track = p::mix(pal.window, pal.text, TRACK_MIX);
    floored(s, track, p::legible(style, s, track, p::DISABLED_RATIO, None))
}

/// The accent fill of a slider, progress bar or dial.
pub fn range_fill(style: &LaceStyle, s: State) -> Rgba {
    let pal = style.pal(s);
    if s.enabled() {
        return pal.accent;
    }
    let window = pal.window;
    let track = p::legible(style, s, p::mix(window, pal.text, TRACK_MIX), p::DISABLED_RATIO, None);
    let fl = p::legible(
        style,
        s,
        p::mix(track, pal.accent, DISABLED_TINT),
        p::DISABLED_RATIO,
        Some(track),
    );
    floored(s, p::mix(window, pal.accent, 0.4), fl)
}

/// A slider or dial knob: `fill`, `line`, and the focus ring.
pub fn knob(style: &LaceStyle, s: State) -> Face {
    let pal = style.pal(s);
    let fill = p::state_fill(s, pal.button);
    let line = p::legible(style, s, p::stroke(style, s, fill), style.ui_ratio(), None);
    let ring = s.has_focus().then(|| p::focus_ring(style, s));
    Face { fill: Some(fill), line: Some(line), text: pal.button_text, ring }
}

/// Slider tick and dial notch colour.
pub fn ticks(style: &LaceStyle, s: State) -> Rgba {
    let pal = style.pal(s);
    p::mix(pal.window, pal.text, 0.35)
}

// ---------------------------------------------------------------------------
// Scrollbars (`_primitives.scrollbar`)
// ---------------------------------------------------------------------------

/// Handle thickness of an `expanding` bar at rest, px.
pub const EXPANDING_REST: f32 = 4.0;

/// Scrollbar colours: the track (only while expanded) and the handle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollbarColors {
    /// Track fill while hovered or dragged.
    pub track: Option<Rgba>,
    /// Handle fill.
    pub handle: Rgba,
}

/// A scrollbar; `on_handle` is whether the pointer is on the handle itself.
pub fn scrollbar(style: &LaceStyle, s: State, on_handle: bool) -> ScrollbarColors {
    let pal = style.pal(s);
    let expanded = s.active_hover() || s.pressed;
    let strength = if s.disabled {
        0.16
    } else if s.pressed && on_handle {
        0.55
    } else if s.hovered && on_handle {
        0.45
    } else {
        0.32
    };
    ScrollbarColors {
        track: expanded.then(|| p::mix(pal.window, pal.text, 0.05)),
        handle: p::legible(style, s, p::mix(pal.window, pal.text, strength), style.ui_ratio(), None),
    }
}

/// A scrollbar step arrow; `at_end` when the bar cannot scroll that way.
pub fn scroll_step(style: &LaceStyle, s: State, active: bool, at_end: bool) -> (Option<Rgba>, Rgba) {
    let pal = style.pal(s);
    let face = (active && s.enabled())
        .then(|| p::mix(pal.window, pal.text, if s.pressed { 0.18 } else { 0.10 }));
    let strength = if at_end || s.disabled {
        0.35
    } else if active {
        0.85
    } else {
        0.62
    };
    (face, p::mix(pal.window, pal.text, strength))
}

// ---------------------------------------------------------------------------
// Containers (`_containers.py`)
// ---------------------------------------------------------------------------

/// Selected tab's accent underline, px.
pub const TAB_UNDERLINE: f32 = 2.0;
const TAB_ACTIVE_WASH: f64 = 0.12;
const TAB_ACTIVE_HOVER_WASH: f64 = 0.18;
const TAB_HOVER_STEP: f64 = 0.05;
/// Hover wash of item-view rows: text over the surface.
pub const HOVER_WASH: f64 = 0.06;

/// A tab: fill, underline (selected only), label colour, focus ring.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tab {
    /// Tab fill.
    pub fill: Option<Rgba>,
    /// Accent underline on the edge facing the pane.
    pub underline: Option<Rgba>,
    /// Label colour.
    pub text: Rgba,
    /// Keyboard focus ring.
    pub ring: Option<Rgba>,
}

/// A tab in state `s` (`selected` marks the current tab).
pub fn tab(style: &LaceStyle, s: State) -> Tab {
    let pal = style.pal(s);
    let window = pal.window;
    let hovered = s.active_hover();
    let fill = if s.selected {
        Some(p::mix(window, pal.accent, if hovered { TAB_ACTIVE_HOVER_WASH } else { TAB_ACTIVE_WASH }))
    } else if hovered {
        Some(p::step(window, TAB_HOVER_STEP))
    } else {
        None
    };
    let underline = s.selected.then(|| {
        if s.enabled() {
            p::legible(style, s, pal.accent, style.ui_ratio(), None)
        } else {
            p::mix(window, pal.accent, 0.4)
        }
    });
    let ring = (s.has_focus() && style.focus_width > 0.0).then(|| p::focus_ring(style, s));
    Tab { fill, underline, text: pal.window_text, ring }
}

/// A header section: fill, the bottom/separator line, and the sort arrow.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Header {
    /// Section fill.
    pub fill: Rgba,
    /// Bottom edge and trailing separator.
    pub line: Rgba,
    /// Caption.
    pub text: Rgba,
    /// Sort indicator chevron.
    pub arrow: Rgba,
}

/// A header section in state `s`.
pub fn header(style: &LaceStyle, s: State) -> Header {
    let pal = style.pal(s);
    let fill = if s.active_hover() || s.pressed {
        p::state_fill(s, pal.button)
    } else {
        pal.button
    };
    Header {
        fill,
        line: line_on(style, s, fill),
        text: pal.button_text,
        arrow: p::legible(style, s, pal.button_text, style.ui_ratio(), Some(pal.button)),
    }
}

/// An item-view row: fill (None = leave the row's own background) and text.
pub fn item_row(style: &LaceStyle, s: State) -> (Option<Rgba>, Rgba) {
    let pal = style.pal(s);
    if s.selected {
        (Some(pal.highlight), pal.highlighted_text)
    } else if s.active_hover() {
        (Some(p::mix(pal.base, pal.text, HOVER_WASH)), pal.text)
    } else {
        (None, pal.text)
    }
}

/// A menu item: rounded selection fill and label colour.
pub fn menu_item(style: &LaceStyle, s: State) -> (Option<Rgba>, Rgba) {
    let pal = style.pal(s);
    if (s.selected || s.hovered || s.pressed) && s.enabled() {
        (Some(pal.highlight), pal.highlighted_text)
    } else {
        (None, pal.text)
    }
}

/// A menu-bar item: rounded selection while open or hovered.
pub fn menu_bar_item(style: &LaceStyle, s: State) -> (Option<Rgba>, Rgba) {
    let pal = style.pal(s);
    if (s.hovered || s.pressed) && s.enabled() {
        (Some(pal.highlight), pal.highlighted_text)
    } else {
        (None, pal.button_text)
    }
}

/// A popup (menu, combo list) panel: fill and outline.
pub fn popup(style: &LaceStyle, s: State) -> (Rgba, Rgba) {
    let fill = style.pal(s).window;
    (fill, line_on(style, s, fill))
}

/// A tooltip: fill, outline and text.
pub fn tooltip(style: &LaceStyle, s: State) -> (Rgba, Rgba, Rgba) {
    let pal = style.pal(s);
    (pal.tooltip_base, line_on(style, s, pal.tooltip_base), pal.tooltip_text)
}

/// A tool-box page tab: fill, outline, text.
pub fn tool_box_tab(style: &LaceStyle, s: State) -> (Rgba, Rgba, Rgba) {
    let pal = style.pal(s);
    let fill = if s.active_hover() || s.pressed {
        p::state_fill(s, pal.button)
    } else {
        pal.button
    };
    (fill, line_on(style, s, fill), pal.button_text)
}

// ---------------------------------------------------------------------------
// Chrome (`_chrome.py`)
// ---------------------------------------------------------------------------

/// Splitter grip thickness at rest, px.
pub const SPLITTER_GRIP: f32 = EXPANDING_REST - 1.0;
/// How much the grip thickens on hover or drag, px.
pub const SPLITTER_GROW: f32 = 2.0;
/// Clear space each side of the grown grip, px.
pub const SPLITTER_PAD: f32 = 3.0;
/// Handle width: the grown grip plus its padding.
pub const SPLITTER_WIDTH: f32 = SPLITTER_GRIP + SPLITTER_GROW + 2.0 * SPLITTER_PAD;
const SPLITTER_REST_MIX: f64 = 0.28;

/// A splitter grip: colour and thickness.
pub fn splitter_grip(style: &LaceStyle, s: State) -> (Rgba, f32) {
    let pal = style.pal(s);
    let active = s.enabled() && (s.hovered || s.pressed);
    if active {
        (p::legible(style, s, pal.accent, style.ui_ratio(), None), SPLITTER_GRIP + SPLITTER_GROW)
    } else {
        let rest = p::mix(pal.window, pal.window_text, SPLITTER_REST_MIX);
        (p::legible(style, s, rest, p::DISABLED_RATIO, None), SPLITTER_GRIP)
    }
}

/// A tab close button: hover wash and cross ink.
pub fn tab_close(style: &LaceStyle, s: State) -> (Option<Rgba>, Rgba) {
    let pal = style.pal(s);
    let (window, text) = (pal.window, pal.window_text);
    let hovered = s.enabled() && s.hovered;
    let pressed = s.enabled() && s.pressed;
    let wash = (hovered || pressed).then(|| p::mix(window, text, if pressed { 0.22 } else { 0.14 }));
    let ink = if s.disabled {
        p::legible(style, s, p::mix(window, text, 0.35), p::DISABLED_RATIO, None)
    } else if hovered || pressed || s.selected {
        text
    } else {
        p::muted(style, s)
    };
    (wash, ink)
}

/// Toolbar handles and separators, size grips: muted lines.
pub fn chrome_ink(style: &LaceStyle, s: State) -> Rgba {
    p::muted(style, s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lace_gpui::color::contrast_ratio;

    fn styles() -> Vec<LaceStyle> {
        lace_gpui::themes::ALL
            .iter()
            .map(|spec| LaceStyle::new(spec, &lace_gpui::build(spec)))
            .collect()
    }

    #[test]
    fn default_button_label_reads() {
        for st in styles() {
            for s in [State::NORMAL, State::hover(), State::pressed()] {
                let f = button_face(&st, s, ButtonKind::Default);
                let r = contrast_ratio(f.text, f.fill.unwrap());
                assert!(r >= 3.0 - 1e-6, "{}: default label {r:.2}", st.name);
            }
        }
    }

    #[test]
    fn indicator_outlines_meet_ui_floor() {
        for st in styles() {
            let ind = check_box(&st, State::NORMAL);
            let r = contrast_ratio(ind.line, st.active.window);
            assert!(r >= st.ui_ratio() - 0.02, "{}: check outline {r:.2}", st.name);
            let off = check_box(&st, State::disabled());
            let r = contrast_ratio(off.line, st.disabled.window);
            assert!(r >= p::DISABLED_RATIO - 0.02, "{}: disabled outline {r:.2}", st.name);
        }
    }

    #[test]
    fn state_steps_move_the_face() {
        for st in styles() {
            let rest = button_face(&st, State::NORMAL, ButtonKind::Normal).fill;
            let hover = button_face(&st, State::hover(), ButtonKind::Normal).fill;
            let press = button_face(&st, State::pressed(), ButtonKind::Normal).fill;
            assert_ne!(rest, hover, "{}", st.name);
            assert_ne!(hover, press, "{}", st.name);
        }
    }

    #[test]
    fn flat_button_has_no_face_at_rest() {
        let st = LaceStyle::preset("dracula").unwrap();
        let f = button_face(&st, State::NORMAL, ButtonKind::Flat);
        assert!(f.fill.is_none() && f.line.is_none());
        assert!(button_face(&st, State::hover(), ButtonKind::Flat).fill.is_some());
    }

    #[test]
    fn focus_ring_only_from_keyboard_focus_when_enabled() {
        let st = LaceStyle::preset("light").unwrap();
        assert!(button_face(&st, State::focused(), ButtonKind::Normal).ring.is_some());
        let off = State { focused: true, disabled: true, ..State::NORMAL };
        assert!(button_face(&st, off, ButtonKind::Normal).ring.is_none());
    }

    #[test]
    fn disabled_palette_text_is_legible() {
        for st in styles() {
            let d = &st.disabled;
            let r = contrast_ratio(d.button_text, d.button);
            assert!(r >= 2.0, "{}: disabled button text {r:.2}", st.name);
        }
    }
}
