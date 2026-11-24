//! Line edits, combo boxes and spin boxes (`_inputs.py`).
//!
//! Each input is one rounded field: a recessed `Base` fill with a 1 px
//! outline that turns accent on focus. Combo and spin boxes draw their
//! buttons inside that field, with vector chevrons and no separator bevels;
//! hover and press show as a tint on the button area only.

use std::rc::Rc;

use gpui_kit::base::{Decrement, Increment};
use gpui_kit::component::input::{Input, InputEvent, InputState, Textarea, TextareaState};
use gpui_kit::component::Sizable;
use gpui_kit::gpui::{
    anchored, canvas, deferred, div, prelude::FluentBuilder, px, AnyElement, App, AppContext as _, BoxShadow,
    Context, Focusable, Subscription,
    ElementId, Entity, InteractiveElement, IntoElement, ParentElement, RenderOnce, ScrollWheelEvent,
    SharedString, StatefulInteractiveElement, Styled, Window,
};

use super::button::apply_face;
use super::interaction::{child_id, Interaction};
use super::margin::Margin;
use crate::glyph::{self, Dir};
use crate::style::rules::{self, ButtonKind};
use crate::style::{hsla, paint, ActiveStyle, LaceStyle, State};

/// Width of a combo's arrow area, px.
pub const ARROW_AREA: f32 = 20.0;
/// Width of a spin box's button column, px.
pub const SPIN_AREA: f32 = 16.0;
/// gpui-kit's key context for number fields: Up / Down are bound there
/// to `Increment` / `Decrement`.
const NUMBER_INPUT_CONTEXT: &str = "NumberInput";

/// The rounded field container for state `s`.
pub fn field_div(style: &LaceStyle, s: State) -> gpui_kit::gpui::Div {
    let f = rules::field(style, s);
    div()
        .h(px(style.control_height))
        .flex()
        .items_center()
        .rounded(px(style.control_radius))
        .bg(hsla(f.fill))
        .border_1()
        .border_color(hsla(f.line.unwrap_or(f.fill)))
        .text_size(px(style.font_size))
        .text_color(hsla(f.text))
        .overflow_hidden()
}

// ---------------------------------------------------------------------------
// Line edit
// ---------------------------------------------------------------------------

/// A `QLineEdit`: gpui-kit's text engine inside a LaceStyle field.
#[derive(IntoElement)]
pub struct LineEdit {
    id: ElementId,
    state: Entity<InputState>,
    disabled: bool,
    read_only: bool,
    forced: Option<State>,
    fill: Option<Margin>,
}

impl LineEdit {
    /// A line edit over `state` (create it with `InputState::new`).
    pub fn new(id: impl Into<ElementId>, state: &Entity<InputState>) -> Self {
        Self { id: id.into(), state: state.clone(), disabled: false, read_only: false, forced: None, fill: None }
    }

    /// Fill the parent, height included, with no margin (an item view's
    /// editor: Qt's `updateEditorGeometry` gives it the whole cell).
    pub fn fill(self) -> Self {
        self.fill_with(Margin::ZERO)
    }

    /// Fill the parent less `margin`: one number for all sides, `(h, v)`,
    /// or `[left, top, right, bottom]`.
    pub fn fill_with(mut self, margin: impl Into<Margin>) -> Self {
        self.fill = Some(margin.into());
        self
    }

    /// Disable.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Read-only: selectable, not editable.
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    /// Paint in `state` regardless of focus.
    pub fn state(mut self, state: State) -> Self {
        self.forced = Some(state);
        self
    }
}

impl RenderOnce for LineEdit {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        // A margin is a box around the filled widget, so hover, focus and
        // clicks stay on the widget itself.
        if let Some(m) = self.fill.filter(|m| !m.is_zero()) {
            self.fill = Some(Margin::ZERO);
            return m.wrap(self).into_any_element();
        }
        self.render_face(window, cx).into_any_element()
    }
}

impl LineEdit {
    fn render_face(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let focus = self.state.read(cx).focus_handle(cx);
        let ix = Interaction::new(self.id.clone(), window, cx).with_focus(focus).force(self.forced);
        // Fields show focus however it arrived.
        let mut s = ix.state(window, cx).with_disabled(self.disabled);
        s.focused = s.focused || ix.is_focused(window);
        let style = cx.lace().clone();
        let el = field_div(&style, s)
            .id(self.id.clone())
            .px(px(4.))
            .child(
                div().flex_1().min_w_0().child(
                    Input::new(&self.state).xsmall()
                        .appearance(false)
                        .disabled(self.disabled || self.read_only),
                ),
            )
            .when(self.fill.is_some(), |d| d.size_full());
        ix.bind(el, true)
    }
}

// ---------------------------------------------------------------------------
// Text edit
// ---------------------------------------------------------------------------

/// A `QPlainTextEdit`: gpui-kit's multi-line editor inside a LaceStyle
/// field, framed with the frame radius. It fills its parent's height.
#[derive(IntoElement)]
pub struct TextEdit {
    id: ElementId,
    state: Entity<TextareaState>,
    disabled: bool,
    forced: Option<State>,
}

impl TextEdit {
    /// A text edit over `state` (create it with `TextareaState::new`).
    pub fn new(id: impl Into<ElementId>, state: &Entity<TextareaState>) -> Self {
        Self { id: id.into(), state: state.clone(), disabled: false, forced: None }
    }

    /// Disable.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Paint in `state` regardless of focus.
    pub fn state(mut self, state: State) -> Self {
        self.forced = Some(state);
        self
    }
}

impl RenderOnce for TextEdit {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let focus = self.state.read(cx).focus_handle(cx);
        let ix = Interaction::new(self.id.clone(), window, cx).with_focus(focus).force(self.forced);
        let mut s = ix.state(window, cx).with_disabled(self.disabled);
        s.focused = s.focused || ix.is_focused(window);
        let style = cx.lace().clone();
        let inset = style.frame_inset(style.frame_radius).max(2.0);
        let el = field_div(&style, s)
            .id(self.id.clone())
            .h_full()
            .items_start()
            .rounded(px(style.frame_radius))
            .p(px(inset))
            .child(
                div()
                    .size_full()
                    .min_w_0()
                    .child(Textarea::new(&self.state).xsmall().appearance(false).disabled(self.disabled).h_full()),
            );
        ix.bind(el, true)
    }
}

// ---------------------------------------------------------------------------
// Combo box
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Default)]
struct ComboUi {
    open: bool,
    width: f32,
    hovered_row: Option<usize>,
}

type Select = Rc<dyn Fn(usize, &mut Window, &mut App)>;

/// A `QComboBox`: a button face (or, editable, a field) with a drop-down
/// chevron and a popup list.
#[derive(IntoElement)]
pub struct ComboBox {
    id: ElementId,
    items: Vec<SharedString>,
    selected: Option<usize>,
    editable: Option<Entity<InputState>>,
    disabled: bool,
    forced: Option<State>,
    on_select: Option<Select>,
    fill: Option<Margin>,
}

impl ComboBox {
    /// A combo over `items`.
    pub fn new(id: impl Into<ElementId>, items: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        Self {
            id: id.into(),
            items: items.into_iter().map(Into::into).collect(),
            selected: None,
            editable: None,
            disabled: false,
            forced: None,
            on_select: None,
            fill: None,
        }
    }

    /// Fill the parent, height included, with no margin (an item view's
    /// editor: Qt's `updateEditorGeometry` gives it the whole cell).
    pub fn fill(self) -> Self {
        self.fill_with(Margin::ZERO)
    }

    /// Fill the parent less `margin`: one number for all sides, `(h, v)`,
    /// or `[left, top, right, bottom]`.
    pub fn fill_with(mut self, margin: impl Into<Margin>) -> Self {
        self.fill = Some(margin.into());
        self
    }

    /// The current item.
    pub fn selected(mut self, ix: Option<usize>) -> Self {
        self.selected = ix;
        self
    }

    /// Editable: the text lives in `state`; picking an item writes it there.
    pub fn editable(mut self, state: &Entity<InputState>) -> Self {
        self.editable = Some(state.clone());
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

    /// Called with the picked index.
    pub fn on_select(mut self, f: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for ComboBox {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        // A margin is a box around the filled widget, so hover, focus and
        // clicks stay on the widget itself.
        if let Some(m) = self.fill.filter(|m| !m.is_zero()) {
            self.fill = Some(Margin::ZERO);
            return m.wrap(self).into_any_element();
        }
        self.render_face(window, cx).into_any_element()
    }
}

impl ComboBox {
    fn render_face(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let ui = window.use_keyed_state(child_id(&self.id, "ui"), cx, |_, _| ComboUi::default());
        let mut ix = Interaction::new(self.id.clone(), window, cx).force(self.forced);
        ix = match &self.editable {
            Some(state) => ix.with_focus(state.read(cx).focus_handle(cx)),
            None => ix.focusable(&self.id, window, cx),
        };
        let mut s = ix.state(window, cx).with_disabled(self.disabled);
        let style = cx.lace().clone();
        let h = style.control_height;
        let open = ui.read(cx).open && !s.disabled;

        let arrow_state = s;
        let (body, glyph_color): (gpui_kit::gpui::Stateful<gpui_kit::gpui::Div>, _) =
            match &self.editable {
                Some(state) => {
                    s.focused = s.focused || ix.is_focused(window);
                    let tint = rules::field_button_tint(&style, arrow_state);
                    let glyph = rules::field_glyph(&style, s, false, true);
                    let el = field_div(&style, s)
                        .id(self.id.clone())
                        .pl(px(1.))
                        .child(
                            div().flex_1().min_w_0().child(
                                Input::new(state).xsmall().appearance(false).disabled(self.disabled),
                            ),
                        )
                        .child(
                            div()
                                .id(child_id(&self.id, "arrow"))
                                .w(px(ARROW_AREA))
                                .h_full()
                                .when_some(tint, |d, t| d.bg(hsla(t)))
                                .child(glyph::painter(move |b, window, _| {
                                    glyph::paint_chevron(window, b, Dir::Down, hsla(glyph), 7.0)
                                })),
                        );
                    (el, glyph)
                }
                None => {
                    let face = rules::button_face(&style, s, ButtonKind::Normal);
                    let glyph = rules::combo_glyph(&style, s, &face);
                    let label = self
                        .selected
                        .and_then(|i| self.items.get(i).cloned())
                        .unwrap_or_default();
                    let el = div()
                        .id(self.id.clone())
                        .h(px(h))
                        .flex()
                        .items_center()
                        .pl(px(6.))
                        .text_size(px(style.font_size))
                        .text_color(hsla(face.text))
                        .child(div().flex_1().min_w_0().overflow_hidden().child(label))
                        .child(div().w(px(ARROW_AREA)).h_full().child(glyph::painter(
                            move |b, window, _| {
                                glyph::paint_chevron(window, b, Dir::Down, hsla(glyph), 7.0)
                            },
                        )));
                    (apply_face(el, &face, &style, style.control_radius), glyph)
                }
            };
        let _ = glyph_color;

        // Measure the combo so the popup matches its width.
        let measure = ui.clone();
        let body = body.relative().child(
            canvas(
                move |bounds, _, cx| {
                    let w = f32::from(bounds.size.width);
                    measure.update(cx, |u, _| u.width = w);
                },
                |_, _, _, _| {},
            )
            .absolute()
            .size_full(),
        );

        let body = body.when(self.fill.is_some(), |d| d.size_full());
        let toggle = ui.clone();
        let body = ix.bind(body, s.disabled);
        let body = if s.disabled {
            body
        } else {
            body.on_click(move |_, _, cx| {
                toggle.update(cx, |u, cx| {
                    u.open = !u.open;
                    cx.notify();
                })
            })
        };

        let popup: Option<AnyElement> = open.then(|| {
            let (fill, line) = rules::popup(&style, State::NORMAL);
            let width = ui.read(cx).width.max(80.0);
            let hovered_row = ui.read(cx).hovered_row;
            let close = ui.clone();
            let rows = self.items.iter().enumerate().map(|(i, item)| {
                let row_state = State {
                    hovered: hovered_row == Some(i),
                    selected: hovered_row.is_none() && self.selected == Some(i),
                    ..State::NORMAL
                };
                let (row_fill, row_text) = rules::menu_item(&style, row_state);
                let hover = ui.clone();
                let pick = ui.clone();
                let on_select = self.on_select.clone();
                let editable = self.editable.clone();
                let text = item.clone();
                div()
                    .id(ElementId::NamedChild(
                        std::sync::Arc::new(self.id.clone()),
                        SharedString::from(format!("row{i}")),
                    ))
                    .h(px(h - 2.0))
                    .mx(px(3.))
                    .px(px(6.))
                    .flex()
                    .items_center()
                    .rounded(px(style.control_radius))
                    .text_color(hsla(row_text))
                    .when_some(row_fill, |d, f| d.bg(hsla(f)))
                    .child(item.clone())
                    .on_hover(move |h, _, cx| {
                        hover.update(cx, |u, cx| {
                            if *h {
                                u.hovered_row = Some(i);
                            } else if u.hovered_row == Some(i) {
                                u.hovered_row = None;
                            }
                            cx.notify();
                        })
                    })
                    .on_click(move |_, window, cx| {
                        if let Some(state) = &editable {
                            let text = text.clone();
                            state.update(cx, |st, cx| st.set_value(text, window, cx));
                        }
                        if let Some(f) = &on_select {
                            f(i, window, cx);
                        }
                        pick.update(cx, |u, cx| {
                            u.open = false;
                            u.hovered_row = None;
                            cx.notify();
                        })
                    })
            });
            deferred(
                anchored().snap_to_window_with_margin(px(8.)).child(
                    div()
                        .occlude()
                        .mt(px(h + 2.0))
                        .min_w(px(width))
                        .py(px(3.))
                        .flex()
                        .flex_col()
                        .bg(hsla(fill))
                        .border_1()
                        .border_color(hsla(line))
                        .rounded(px(style.popup_radius()))
                        .shadow(vec![BoxShadow {
                            color: gpui_kit::gpui::black().opacity(0.28),
                            offset: gpui_kit::gpui::point(px(0.), px(3.)),
                            blur_radius: px(10.),
                            spread_radius: px(0.),
                            inset: false,
                        }])
                        .text_size(px(style.font_size))
                        .children(rows)
                        .on_mouse_down_out(move |_, _, cx| {
                            close.update(cx, |u, cx| {
                                u.open = false;
                                cx.notify();
                            })
                        }),
                ),
            )
            .with_priority(1)
            .into_any_element()
        });

        body.when_some(popup, |d, p| d.child(p))
    }
}

// ---------------------------------------------------------------------------
// Spin box
// ---------------------------------------------------------------------------

type Change = Rc<dyn Fn(f64, &mut Window, &mut App)>;

/// A `QSpinBox` / `QDoubleSpinBox`: an editable number field with up/down
/// buttons. The value can be typed (Enter or leaving the field commits it,
/// clamped to the range and rounded to `decimals`; text that is no number
/// goes back to the value), and the buttons, the mouse wheel, Up / Down and
/// Page Up / Page Down step it.
///
/// The text state lives with the element (keyed by its id), so the API
/// stays a plain value plus `on_change`, as for the other controls. A host
/// that needs the text field itself (its own menus, say) passes one with
/// [`SpinBox::input`].
#[derive(IntoElement)]
pub struct SpinBox {
    id: ElementId,
    input: Option<Entity<InputState>>,
    value: f64,
    min: f64,
    max: f64,
    step: f64,
    decimals: usize,
    prefix: SharedString,
    suffix: SharedString,
    plus_minus: bool,
    buttons: bool,
    disabled: bool,
    read_only: bool,
    forced: Option<State>,
    forced_part: Option<(bool, State)>,
    on_change: Option<Change>,
    fill: Option<Margin>,
}

/// What a spin box knows each frame: written on every render, read by the
/// text field's commit and by the steps.
#[derive(Clone)]
struct SpinSpec {
    value: f64,
    min: f64,
    max: f64,
    decimals: usize,
    on_change: Option<Change>,
}

impl SpinSpec {
    fn text(&self, v: f64) -> SharedString {
        format!("{:.*}", self.decimals, v).into()
    }

    /// `v` in range and rounded as shown.
    fn fit(&self, v: f64) -> f64 {
        let v = v.clamp(self.min, self.max.max(self.min));
        let scale = 10f64.powi(self.decimals as i32);
        (v * scale).round() / scale
    }
}

/// A typed number, as `QDoubleSpinBox` reads one: surrounding blanks and
/// a decimal comma allowed.
fn parse_number(text: &str) -> Option<f64> {
    let t = text.trim().replace(',', ".");
    t.parse::<f64>().ok().filter(|v| v.is_finite())
}

/// A spin box's own state: its text field, and the text last written into
/// it from the value (so a new value shows, but typing is left alone).
struct SpinEdit {
    input: Entity<InputState>,
    shown: SharedString,
    spec: SpinSpec,
    _subscription: Subscription,
}

impl SpinEdit {
    fn new(spec: SpinSpec, input: Option<Entity<InputState>>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let shown = spec.text(spec.value);
        let input = input.unwrap_or_else(|| cx.new(|cx| InputState::new(window, cx).default_value(shown.clone())));
        let _subscription = Self::watch(&input, window, cx);
        Self { input, shown, spec, _subscription }
    }

    /// Commit the typed text on Enter, or on leaving the field.
    fn watch(input: &Entity<InputState>, window: &mut Window, cx: &mut Context<Self>) -> Subscription {
        cx.subscribe_in(input, window, |this, _, ev: &InputEvent, window, cx| {
            if matches!(ev, InputEvent::PressEnter { .. } | InputEvent::Blur) {
                this.commit(window, cx);
            }
        })
    }

    /// Edit through `input` from now on (the host passed another one).
    fn attach(&mut self, input: Entity<InputState>, window: &mut Window, cx: &mut Context<Self>) {
        self._subscription = Self::watch(&input, window, cx);
        self.input = input;
        self.shown = SharedString::default();
    }

    /// Show `text` in the field.
    fn show(&mut self, text: SharedString, window: &mut Window, cx: &mut Context<Self>) {
        if self.input.read(cx).value() != text {
            self.input.update(cx, |s, cx| s.set_value(text.clone(), window, cx));
        }
        self.shown = text;
    }

    /// Take `v` as the new value: tell the owner, and show it.
    fn set(&mut self, v: f64, window: &mut Window, cx: &mut Context<Self>) {
        let v = self.spec.fit(v);
        if v != self.spec.value {
            self.spec.value = v;
            if let Some(f) = self.spec.on_change.clone() {
                f(v, window, cx);
            }
        }
        let text = self.spec.text(v);
        self.show(text, window, cx);
    }

    /// The typed text as the value, or back to the value if it is no number.
    fn commit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let typed = self.input.read(cx).value();
        match parse_number(&typed) {
            Some(v) => self.set(v, window, cx),
            None => {
                let text = self.spec.text(self.spec.value);
                self.show(text, window, cx);
            }
        }
    }

    /// One step of `by` from what the field shows (typed text included, as
    /// `QSpinBox` steps from its text).
    fn step_by(&mut self, by: f64, window: &mut Window, cx: &mut Context<Self>) {
        let base = parse_number(&self.input.read(cx).value()).unwrap_or(self.spec.value);
        self.set(base + by, window, cx);
    }
}

impl SpinBox {
    /// A spin box on `value`, 0..=99 by 1.
    pub fn new(id: impl Into<ElementId>, value: f64) -> Self {
        Self {
            id: id.into(),
            input: None,
            value,
            min: 0.0,
            max: 99.0,
            step: 1.0,
            decimals: 0,
            prefix: SharedString::default(),
            suffix: SharedString::default(),
            plus_minus: false,
            buttons: true,
            disabled: false,
            read_only: false,
            forced: None,
            forced_part: None,
            on_change: None,
            fill: None,
        }
    }

    /// Fill the parent, height included, with no margin (an item view's
    /// editor: Qt's `updateEditorGeometry` gives it the whole cell).
    pub fn fill(self) -> Self {
        self.fill_with(Margin::ZERO)
    }

    /// Fill the parent less `margin`: one number for all sides, `(h, v)`,
    /// or `[left, top, right, bottom]`.
    pub fn fill_with(mut self, margin: impl Into<Margin>) -> Self {
        self.fill = Some(margin.into());
        self
    }

    /// Range.
    pub fn range(mut self, min: f64, max: f64) -> Self {
        self.min = min;
        self.max = max;
        self
    }

    /// Step per click / key / wheel notch.
    pub fn step(mut self, step: f64) -> Self {
        self.step = step;
        self
    }

    /// Decimal places shown (a `QDoubleSpinBox`).
    pub fn decimals(mut self, decimals: usize) -> Self {
        self.decimals = decimals;
        self
    }

    /// Type into `state` rather than a text field of the spin box's own,
    /// so the host can reach it (its menus, its focus). The spin box still
    /// commits it on Enter or on leaving it, and writes new values into it.
    pub fn input(mut self, state: &Entity<InputState>) -> Self {
        self.input = Some(state.clone());
        self
    }

    /// Text before the number, not part of the value (`setPrefix`).
    pub fn prefix(mut self, prefix: impl Into<SharedString>) -> Self {
        self.prefix = prefix.into();
        self
    }

    /// Text after the number, not part of the value (`setSuffix`), e.g. `" s"`.
    pub fn suffix(mut self, suffix: impl Into<SharedString>) -> Self {
        self.suffix = suffix.into();
        self
    }

    /// Read-only: the value shows and can be selected, but not typed or
    /// stepped (`setReadOnly`).
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    /// `+` / `-` symbols instead of chevrons.
    pub fn plus_minus(mut self) -> Self {
        self.plus_minus = true;
        self
    }

    /// No buttons (`NoButtons`).
    pub fn no_buttons(mut self) -> Self {
        self.buttons = false;
        self
    }

    /// Disable.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Paint the field in `state`; hover / press go to the up button.
    pub fn state(mut self, state: State) -> Self {
        self.forced = Some(state);
        if state.hovered || state.pressed {
            self.forced_part = Some((true, state));
        }
        self
    }

    /// Called with the new value.
    pub fn on_change(mut self, f: impl Fn(f64, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }

    fn spec(&self) -> SpinSpec {
        SpinSpec {
            value: self.value,
            min: self.min,
            max: self.max,
            decimals: self.decimals,
            on_change: self.on_change.clone(),
        }
    }
}

impl RenderOnce for SpinBox {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        // A margin is a box around the filled widget, so hover, focus and
        // clicks stay on the widget itself.
        if let Some(m) = self.fill.filter(|m| !m.is_zero()) {
            self.fill = Some(Margin::ZERO);
            return m.wrap(self).into_any_element();
        }
        self.render_face(window, cx).into_any_element()
    }
}

impl SpinBox {
    fn render_face(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        // The text field and what it last showed, kept with the element.
        let spec = self.spec();
        let edit = window.use_keyed_state(child_id(&self.id, "edit"), cx, {
            let (spec, input) = (spec.clone(), self.input.clone());
            move |window, cx| SpinEdit::new(spec, input, window, cx)
        });
        // A new value from the owner shows at once; while the user types,
        // the value is unchanged and the text is left alone.
        edit.update(cx, |e, cx| {
            if let Some(input) = self.input.clone().filter(|i| *i != e.input) {
                e.attach(input, window, cx);
            }
            e.spec = spec.clone();
            let text = spec.text(spec.value);
            if text != e.shown {
                e.show(text, window, cx);
            }
        });
        let input = edit.read(cx).input.clone();
        let focus = input.read(cx).focus_handle(cx);

        let ix = Interaction::new(self.id.clone(), window, cx)
            .with_focus(focus)
            .force(self.forced.map(|f| State { hovered: false, pressed: false, ..f }));
        let mut s = ix.state(window, cx).with_disabled(self.disabled);
        s.focused = s.focused || ix.is_focused(window);
        let style = cx.lace().clone();
        let (value, min, max, step) = (self.value, self.min, self.max, self.step);
        let steps = self.on_change.is_some() && !self.disabled && !self.read_only;

        let affix = |text: &SharedString| {
            (!text.is_empty()).then(|| div().flex_none().child(text.clone()))
        };
        let field = Input::new(&input).xsmall().appearance(false).disabled(self.disabled || self.read_only);
        let mut el = field_div(&style, s)
            .id(self.id.clone())
            .pl(px(4.))
            .when_some(affix(&self.prefix), |d, p| d.child(p));
        el = match affix(&self.suffix) {
            // Qt's suffix is part of the text: it follows the value ("2,00
            // sec"), so the text field is as wide as what it holds and the
            // rest of the face, clicked, focuses it.
            Some(suffix) => {
                let text = input.read(cx).value();
                let size = window.rem_size() * 0.75;
                let run = gpui_kit::gpui::TextRun {
                    len: text.len(),
                    font: window.text_style().font(),
                    color: gpui_kit::gpui::black(),
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                };
                let w = f32::from(window.text_system().layout_line(&text, size, &[run], None).width).ceil();
                // The kit's xsmall padding on each side, and the caret.
                let w = w + 2.0 * 4.0 + 2.0;
                let focus = input.read(cx).focus_handle(cx);
                el.child(div().min_w_0().w(px(w)).child(field))
                    .child(suffix.pr(px(2.)))
                    .child(div().flex_1().h_full().on_mouse_down(gpui_kit::gpui::MouseButton::Left, move |_, window, cx| {
                        focus.focus(window, cx);
                    }))
            }
            None => el.child(div().flex_1().min_w_0().child(field)),
        };

        if self.buttons {
            let mut col = div().w(px(SPIN_AREA)).h_full().flex().flex_col();
            for (up, label) in [(true, "up"), (false, "down")] {
                let part_id = child_id(&self.id, label);
                let forced = self.forced_part.filter(|(u, _)| *u == up).map(|(_, st)| st);
                let pix = Interaction::new(part_id.clone(), window, cx).force(
                    forced.or(self.forced.map(|_| State::NORMAL)),
                );
                let enabled = if up { value < max } else { value > min };
                let ps = pix.state(window, cx).with_disabled(s.disabled || self.read_only || !enabled);
                let tint = rules::field_button_tint(&style, ps);
                let glyph = rules::field_glyph(&style, s, self.read_only, enabled);
                let plus_minus = self.plus_minus;
                let part = div()
                    .id(part_id)
                    .flex_1()
                    .when_some(tint, |d, t| d.bg(hsla(t)))
                    .child(glyph::painter(move |b, window, _| {
                        if plus_minus {
                            glyph::paint_plus_minus(window, b, up, hsla(glyph));
                        } else {
                            let side = (f32::from(b.size.width)).min(f32::from(b.size.height) * 2.0).min(12.0);
                            glyph::paint_chevron(
                                window,
                                b,
                                if up { Dir::Up } else { Dir::Down },
                                hsla(glyph),
                                side * 0.6,
                            );
                        }
                    }));
                let part = pix.bind(part, ps.disabled);
                let part = if steps && !ps.disabled {
                    let edit = edit.clone();
                    part.on_click(move |_, window, cx| {
                        cx.stop_propagation();
                        edit.update(cx, |e, cx| e.step_by(if up { step } else { -step }, window, cx));
                    })
                } else {
                    part
                };
                col = col.child(part);
            }
            el = el.child(col);
        }

        let el = el.when(self.fill.is_some(), |d| d.size_full());
        let el = ix.bind(el, s.disabled);
        if !steps {
            return el;
        }
        let (wheel, key, up, down) = (edit.clone(), edit.clone(), edit.clone(), edit);
        // Up / Down come through gpui-kit's number-input bindings (a
        // single-line text field leaves them unhandled); Page Up / Down step
        // ten at a time, as in Qt.
        el.key_context(NUMBER_INPUT_CONTEXT)
            .on_action(move |_: &Increment, window, cx| {
                up.update(cx, |e, cx| e.step_by(step, window, cx));
            })
            .on_action(move |_: &Decrement, window, cx| {
                down.update(cx, |e, cx| e.step_by(-step, window, cx));
            })
            .on_scroll_wheel(move |ev: &ScrollWheelEvent, window, cx| {
                // The wheel is the spin box's: the view beneath must not
                // scroll as well (Qt accepts the wheel event).
                cx.stop_propagation();
                let dy = f32::from(ev.delta.pixel_delta(px(20.)).y);
                if dy.abs() > 0.0 {
                    let by = if dy > 0.0 { step } else { -step };
                    wheel.update(cx, |e, cx| e.step_by(by, window, cx));
                }
            })
            .on_key_down(move |ev, window, cx| {
                let by = match ev.keystroke.key.as_str() {
                    "pageup" => step * 10.0,
                    "pagedown" => -step * 10.0,
                    _ => return,
                };
                cx.stop_propagation();
                key.update(cx, |e, cx| e.step_by(by, window, cx));
            })
    }
}

/// The field's focus ring colour, exposed for custom fields.
pub fn field_focus(style: &LaceStyle, s: State) -> gpui_kit::gpui::Hsla {
    hsla(paint::focus_ring(style, s))
}

#[cfg(test)]
mod tests {
    use super::{parse_number, SpinSpec};

    fn spec(min: f64, max: f64, decimals: usize) -> SpinSpec {
        SpinSpec { value: 0.0, min, max, decimals, on_change: None }
    }

    #[test]
    fn a_typed_number_is_read_as_a_double_spin_box_reads_it() {
        assert_eq!(parse_number(" 2.5 "), Some(2.5));
        assert_eq!(parse_number("2,5"), Some(2.5), "a decimal comma");
        assert_eq!(parse_number("-3"), Some(-3.0));
        assert_eq!(parse_number("1e3"), Some(1000.0));
        assert_eq!(parse_number(""), None);
        assert_eq!(parse_number("abc"), None);
        assert_eq!(parse_number("inf"), None, "only finite numbers");
        assert_eq!(parse_number("NaN"), None);
    }

    #[test]
    fn a_value_is_kept_in_range_and_rounded_as_shown() {
        let s = spec(0.0, 10.0, 2);
        assert_eq!(s.fit(3.14159), 3.14);
        assert_eq!(s.fit(-1.0), 0.0);
        assert_eq!(s.fit(12.0), 10.0);
        assert_eq!(s.text(3.14159), "3.14");
        let ints = spec(-5.0, 5.0, 0);
        assert_eq!(ints.fit(2.6), 3.0, "an int spin box rounds");
        assert_eq!(ints.text(3.0), "3");
        assert_eq!(spec(4.0, 1.0, 0).fit(9.0), 4.0, "an empty range holds its minimum");
    }
}
