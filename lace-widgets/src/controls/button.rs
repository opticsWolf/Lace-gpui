//! Push and tool buttons (`_buttons.py`).
//!
//! One face for every button: a `Button` fill with a 1 px outline, hover and
//! pressed as lightness steps, checked as an accent wash, the default button
//! filled with the accent. Keyboard focus replaces the outline with the
//! accent ring, drawn inside the face so no layout has to make room.

use std::rc::Rc;

use gpui_kit::gpui::{
    div, prelude::FluentBuilder, px, App, Div, ElementId, InteractiveElement, IntoElement,
    KeyDownEvent, ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, Styled,
    Window,
};

use super::interaction::{child_id, Interaction};
use crate::glyph::{self, Dir};
use crate::style::rules::{self, ButtonKind, Face};
use crate::style::{hsla, ActiveStyle, LaceStyle, State};

/// A click handler.
pub type Handler = Rc<dyn Fn(&mut Window, &mut App)>;

/// Paint `face` onto `el`: fill, then the focus ring or the outline.
pub fn apply_face<E: Styled>(el: E, face: &Face, style: &LaceStyle, radius: f32) -> E {
    let el = el.rounded(px(radius));
    let el = match face.fill {
        Some(fill) => el.bg(hsla(fill)),
        None => el,
    };
    if let Some(ring) = face.ring {
        el.border(px(style.focus_width)).border_color(hsla(ring))
    } else if let Some(line) = face.line {
        el.border_1().border_color(hsla(line))
    } else {
        el.border_1().border_color(gpui_kit::gpui::transparent_black())
    }
}

/// Activate on Enter or Space, like a focused Qt button.
fn activates(ev: &KeyDownEvent) -> bool {
    matches!(ev.keystroke.key.as_str(), "enter" | "space")
}

/// A `QPushButton`: normal, default, flat, or checkable (toggle).
#[derive(IntoElement)]
pub struct PushButton {
    id: ElementId,
    label: SharedString,
    kind: ButtonKind,
    checked: bool,
    disabled: bool,
    forced: Option<State>,
    min_width: f32,
    on_click: Option<Handler>,
}

impl PushButton {
    /// A push button with a text label.
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            kind: ButtonKind::Normal,
            checked: false,
            disabled: false,
            forced: None,
            min_width: 64.0,
            on_click: None,
        }
    }

    /// The dialog's default button: filled with the accent.
    pub fn default_button(mut self) -> Self {
        self.kind = ButtonKind::Default;
        self
    }

    /// Flat: a face only while hovered or pressed.
    pub fn flat(mut self) -> Self {
        self.kind = ButtonKind::Flat;
        self
    }

    /// Show as checked (a toggle button that is on).
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Disable the button.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Paint in `state` regardless of the pointer (gallery rendering).
    pub fn state(mut self, state: State) -> Self {
        self.forced = Some(state);
        self
    }

    /// Minimum width, px.
    pub fn min_width(mut self, w: f32) -> Self {
        self.min_width = w;
        self
    }

    /// Called on click, Enter or Space.
    pub fn on_click(mut self, f: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for PushButton {
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
        let face = rules::button_face(&style, s, self.kind);
        let el = div()
            .id(self.id.clone())
            .h(px(style.control_height))
            .min_w(px(self.min_width))
            .px(px(10.))
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .text_size(px(style.font_size))
            .text_color(hsla(face.text))
            .child(self.label);
        let el = apply_face(el, &face, &style, style.control_radius);
        let disabled = s.disabled;
        let el = ix.bind(el, disabled);
        match self.on_click {
            Some(f) if !disabled => {
                let key = f.clone();
                el.on_click(move |_, window, cx| f(window, cx)).on_key_down(move |ev, window, cx| {
                    if activates(ev) {
                        key(window, cx);
                        cx.stop_propagation();
                    }
                })
            }
            _ => el,
        }
    }
}

/// A `QToolButton`: a compact face around an icon or short text; auto-raise
/// shows the face only on hover; split adds a menu part with a divider.
#[derive(IntoElement)]
pub struct ToolButton {
    id: ElementId,
    content: Option<gpui_kit::gpui::AnyElement>,
    label: SharedString,
    auto_raise: bool,
    split: bool,
    checked: bool,
    disabled: bool,
    forced: Option<State>,
    on_click: Option<Handler>,
    on_menu: Option<Handler>,
}

/// Width of a split button's menu part, px (Fusion's indicator + Lace's pad).
pub const SPLIT_PART: f32 = 16.0;

impl ToolButton {
    /// A tool button with a text label.
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            content: None,
            label: label.into(),
            auto_raise: false,
            split: false,
            checked: false,
            disabled: false,
            forced: None,
            on_click: None,
            on_menu: None,
        }
    }

    /// Show `el` (an icon) instead of the label.
    pub fn icon(mut self, el: impl IntoElement) -> Self {
        self.content = Some(el.into_any_element());
        self
    }

    /// Auto-raise: a face only while hovered, pressed or checked.
    pub fn auto_raise(mut self) -> Self {
        self.auto_raise = true;
        self
    }

    /// Add a menu part (`MenuButtonPopup`).
    pub fn split(mut self) -> Self {
        self.split = true;
        self
    }

    /// Show as checked.
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
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

    /// Called on click.
    pub fn on_click(mut self, f: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(f));
        self
    }

    /// Called when the menu part is clicked.
    pub fn on_menu(mut self, f: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_menu = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for ToolButton {
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
        let kind = if self.auto_raise { ButtonKind::Flat } else { ButtonKind::Normal };
        let face = rules::button_face(&style, s, kind);
        let h = style.control_height;
        let content: gpui_kit::gpui::AnyElement = match self.content {
            Some(c) => c,
            None => div().child(self.label).into_any_element(),
        };
        let main = div()
            .h_full()
            .min_w(px(h - 2.0))
            .px(px(6.))
            .flex()
            .items_center()
            .justify_center()
            .child(content);
        let el = div()
            .id(self.id.clone())
            .h(px(h))
            .flex()
            .flex_none()
            .text_size(px(style.font_size))
            .text_color(hsla(face.text))
            .child(main);
        let el = if self.split {
            let divider = rules::split_divider(&style, s, &face);
            let glyph_color = hsla(face.text);
            let shows_face = face.fill.is_some() || face.line.is_some();
            let menu_id = child_id(&self.id, "menu");
            let on_menu = self.on_menu.clone();
            el.child(
                div()
                    .id(menu_id)
                    .w(px(SPLIT_PART))
                    .h_full()
                    .flex()
                    .items_center()
                    .when(shows_face, |d| {
                        d.child(div().w(px(1.)).h(px(h - 8.)).bg(hsla(divider)))
                    })
                    .child(glyph::glyph_box(SPLIT_PART - 1.0, h, move |b, window, _| {
                        glyph::paint_chevron(window, b, Dir::Down, glyph_color, 7.0)
                    }))
                    .when_some(on_menu.filter(|_| !s.disabled), |d, f| {
                        d.on_click(move |_, window, cx| {
                            cx.stop_propagation();
                            f(window, cx)
                        })
                    }),
            )
        } else {
            el
        };
        let el = apply_face(el, &face, &style, style.control_radius);
        let disabled = s.disabled;
        let el = ix.bind(el, disabled);
        match self.on_click {
            Some(f) if !disabled => {
                let key = f.clone();
                el.on_click(move |_, window, cx| f(window, cx)).on_key_down(move |ev, window, cx| {
                    if activates(ev) {
                        key(window, cx);
                        cx.stop_propagation();
                    }
                })
            }
            _ => el,
        }
    }
}

/// A bare face `div` for custom content, painted for `state` and `kind`.
pub fn face_div(style: &LaceStyle, state: State, kind: ButtonKind) -> Div {
    let face = rules::button_face(style, state, kind);
    apply_face(div().text_color(hsla(face.text)), &face, style, style.control_radius)
}
