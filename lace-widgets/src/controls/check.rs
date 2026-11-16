//! Check boxes and radio buttons (`_primitives.check_box`, `radio_button`).

use std::rc::Rc;

use gpui_kit::gpui::{
    div, prelude::FluentBuilder, px, App, ElementId, InteractiveElement, IntoElement,
    ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window,
};

use super::interaction::Interaction;
use crate::glyph;
use crate::style::paint::scaled_radius;
use crate::style::rules;
use crate::style::{hsla, ActiveStyle, LaceStyle, State};

/// Indicator side, px (Fusion's `PM_IndicatorWidth` at 96 dpi).
pub const INDICATOR: f32 = 14.0;
/// Gap between an indicator and its label, px.
pub const LABEL_GAP: f32 = 6.0;

/// A check state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CheckState {
    /// Off.
    #[default]
    Unchecked,
    /// Partially checked (a tri-state parent).
    Partial,
    /// On.
    Checked,
}

impl CheckState {
    /// The next state a click moves to; `tristate` boxes cycle through partial.
    pub fn next(self, tristate: bool) -> Self {
        match (self, tristate) {
            (Self::Unchecked, true) => Self::Partial,
            (Self::Partial, _) => Self::Checked,
            (Self::Unchecked, false) => Self::Checked,
            (Self::Checked, _) => Self::Unchecked,
        }
    }

    fn apply(self, s: State) -> State {
        match self {
            Self::Unchecked => s,
            Self::Partial => s.with_partial(true),
            Self::Checked => s.with_checked(true),
        }
    }
}

/// The check-box indicator alone, `size` px, for state `s` — item views use
/// it for their check column.
pub fn check_indicator(style: &LaceStyle, s: State, size: f32) -> impl IntoElement {
    let ind = rules::check_box(style, s);
    let glyph_color = hsla(ind.glyph);
    let (checked, partial) = (s.checked, s.partial && !s.checked);
    div()
        .size(px(size))
        .flex_none()
        .rounded(px(scaled_radius(style, size)))
        .bg(hsla(ind.fill))
        .border_1()
        .border_color(hsla(ind.line))
        .when(checked, |d| d.child(glyph::check_mark(glyph_color)))
        .when(partial, |d| d.child(glyph::dash(glyph_color)))
}

/// The radio indicator alone.
pub fn radio_indicator(style: &LaceStyle, s: State, size: f32) -> impl IntoElement {
    let ind = rules::radio(style, s);
    let on = s.checked;
    let dot = size * (1.0 - 2.0 * 0.28);
    div()
        .size(px(size))
        .flex_none()
        .rounded_full()
        .bg(hsla(ind.fill))
        .border(px(if on { 1.5 } else { 1.0 }))
        .border_color(hsla(ind.line))
        .flex()
        .items_center()
        .justify_center()
        .when(on, |d| d.child(div().size(px(dot)).rounded_full().bg(hsla(ind.glyph))))
}

type Toggle = Rc<dyn Fn(CheckState, &mut Window, &mut App)>;

/// A `QCheckBox`: indicator and label; optionally tri-state.
#[derive(IntoElement)]
pub struct CheckBox {
    id: ElementId,
    label: Option<SharedString>,
    value: CheckState,
    tristate: bool,
    disabled: bool,
    forced: Option<State>,
    on_change: Option<Toggle>,
}

impl CheckBox {
    /// A check box.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            label: None,
            value: CheckState::Unchecked,
            tristate: false,
            disabled: false,
            forced: None,
            on_change: None,
        }
    }

    /// Label text.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Checked or not.
    pub fn checked(mut self, on: bool) -> Self {
        self.value = if on { CheckState::Checked } else { CheckState::Unchecked };
        self
    }

    /// The full check state.
    pub fn value(mut self, value: CheckState) -> Self {
        self.value = value;
        self
    }

    /// Cycle through the partial state on click.
    pub fn tristate(mut self, tristate: bool) -> Self {
        self.tristate = tristate;
        self
    }

    /// Disable.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Paint in `state` regardless of the pointer.
    pub fn state(mut self, state: State) -> Self {
        self.forced = Some(state);
        self
    }

    /// Called with the new state on click or Space.
    pub fn on_change(mut self, f: impl Fn(CheckState, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for CheckBox {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let ix = Interaction::new(self.id.clone(), window, cx)
            .focusable(&self.id, window, cx)
            .force(self.forced);
        let s = self.value.apply(ix.state(window, cx).with_disabled(self.disabled));
        let s = match self.forced {
            Some(f) => s.union(f),
            None => s,
        };
        let style = cx.lace().clone();
        let text = style.pal(s).window_text;
        let ring = s.has_focus().then(|| crate::style::paint::focus_ring(&style, s));
        let el = div()
            .id(self.id.clone())
            .flex()
            .flex_none()
            .items_center()
            .gap(px(LABEL_GAP))
            .h(px(style.control_height - 4.0))
            .px(px(2.))
            .rounded(px(style.control_radius))
            .text_size(px(style.font_size))
            .text_color(hsla(text))
            .child(check_indicator(&style, s, INDICATOR))
            .when_some(self.label, |d, l| d.child(l))
            .when_some(ring, |d, r| d.border(px(1.)).border_color(hsla(r)));
        let el = ix.bind(el, s.disabled);
        match self.on_change {
            Some(f) if !s.disabled => {
                let (value, tri) = (self.value, self.tristate);
                let key = f.clone();
                el.on_click(move |_, window, cx| f(value.next(tri), window, cx)).on_key_down(
                    move |ev, window, cx| {
                        if ev.keystroke.key == "space" {
                            key(value.next(tri), window, cx);
                            cx.stop_propagation();
                        }
                    },
                )
            }
            _ => el,
        }
    }
}

/// A `QRadioButton`.
#[derive(IntoElement)]
pub struct RadioButton {
    id: ElementId,
    label: Option<SharedString>,
    checked: bool,
    disabled: bool,
    forced: Option<State>,
    on_select: Option<Rc<dyn Fn(&mut Window, &mut App)>>,
}

impl RadioButton {
    /// A radio button.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self { id: id.into(), label: None, checked: false, disabled: false, forced: None, on_select: None }
    }

    /// Label text.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Selected or not.
    pub fn checked(mut self, on: bool) -> Self {
        self.checked = on;
        self
    }

    /// Disable.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Paint in `state` regardless of the pointer.
    pub fn state(mut self, state: State) -> Self {
        self.forced = Some(state);
        self
    }

    /// Called when the user selects it.
    pub fn on_select(mut self, f: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for RadioButton {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let ix = Interaction::new(self.id.clone(), window, cx)
            .focusable(&self.id, window, cx)
            .force(self.forced);
        let s = ix.state(window, cx).union(State {
            checked: self.checked,
            disabled: self.disabled,
            ..State::NORMAL
        });
        let style = cx.lace().clone();
        let text = style.pal(s).window_text;
        let ring = s.has_focus().then(|| crate::style::paint::focus_ring(&style, s));
        let el = div()
            .id(self.id.clone())
            .flex()
            .flex_none()
            .items_center()
            .gap(px(LABEL_GAP))
            .h(px(style.control_height - 4.0))
            .px(px(2.))
            .rounded(px(style.control_radius))
            .text_size(px(style.font_size))
            .text_color(hsla(text))
            .child(radio_indicator(&style, s, INDICATOR))
            .when_some(self.label, |d, l| d.child(l))
            .when_some(ring, |d, r| d.border(px(1.)).border_color(hsla(r)));
        let el = ix.bind(el, s.disabled);
        match self.on_select {
            Some(f) if !s.disabled => {
                let key = f.clone();
                el.on_click(move |_, window, cx| f(window, cx)).on_key_down(move |ev, window, cx| {
                    if ev.keystroke.key == "space" {
                        key(window, cx);
                        cx.stop_propagation();
                    }
                })
            }
            _ => el,
        }
    }
}
