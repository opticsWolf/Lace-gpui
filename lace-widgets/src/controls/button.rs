//! Push and tool buttons (`_buttons.py`).
//!
//! One face for every button: a `Button` fill with a 1 px outline, hover and
//! pressed as lightness steps, checked as an accent wash, the default button
//! filled with the accent. Keyboard focus replaces the outline with the
//! accent ring, drawn inside the face so no layout has to make room.

use std::rc::Rc;

use gpui_kit::gpui::{
    canvas, div, prelude::FluentBuilder, px, AnyElement, App, AppContext, Div, ElementId, Hsla,
    InteractiveElement, IntoElement, KeyDownEvent, ParentElement, RenderOnce, SharedString,
    StatefulInteractiveElement, Styled, TextRun, Window,
};

use super::interaction::{child_id, Interaction};
use super::margin::Margin;
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

/// A button icon: built for the face's text colour, so it follows the
/// state like the label does.
pub type Icon = Rc<dyn Fn(Hsla) -> AnyElement>;

/// Side of a button icon, px.
pub const ICON: f32 = 14.0;
/// Horizontal padding of a push button, px.
const PAD: f32 = 10.0;
/// Gap between icon and label, px.
const ICON_GAP: f32 = 6.0;

/// How a push button's label behaves when squeezed (Weave's `LabelCollapse`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Collapse {
    /// Never drops the label.
    #[default]
    Never,
    /// Drops it when narrower than its labelled width (a lone button).
    Alone,
    /// Told by its [`ButtonRow`]: `true` shows the icon only.
    Row(bool),
}

/// A `QPushButton`: normal, default, flat, or checkable (toggle).
///
/// With an [`icon`](Self::icon) it can be [`collapsible`](Self::collapsible):
/// squeezed below its labelled width it paints the icon alone and names
/// itself in a tooltip, and the label returns once there is room.
#[derive(IntoElement)]
pub struct PushButton {
    id: ElementId,
    label: SharedString,
    icon: Option<Icon>,
    collapse: Collapse,
    kind: ButtonKind,
    checked: bool,
    disabled: bool,
    forced: Option<State>,
    min_width: f32,
    fill: Option<Margin>,
    on_click: Option<Handler>,
}

/// A collapsed button's tooltip.
struct TipView(SharedString);

impl gpui_kit::gpui::Render for TipView {
    fn render(&mut self, _: &mut Window, cx: &mut gpui_kit::gpui::Context<Self>) -> impl IntoElement {
        let style = cx.lace().clone();
        super::container::tooltip(&style, State::NORMAL, self.0.clone())
    }
}

impl PushButton {
    /// A push button with a text label.
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            collapse: Collapse::Never,
            kind: ButtonKind::Normal,
            checked: false,
            disabled: false,
            forced: None,
            min_width: 64.0,
            fill: None,
            on_click: None,
        }
    }

    /// An icon before the label, built for the face's text colour.
    pub fn icon(mut self, f: impl Fn(Hsla) -> AnyElement + 'static) -> Self {
        self.icon = Some(Rc::new(f));
        self
    }

    /// Let the label give way to the icon when the button is squeezed.
    /// A button without an icon never collapses: nothing would be left.
    pub fn collapsible(mut self) -> Self {
        self.collapse = Collapse::Alone;
        self
    }

    /// Fill the parent, height included, with no margin (a button
    /// delegate in an item-view cell).
    pub fn fill(self) -> Self {
        self.fill_with(Margin::ZERO)
    }

    /// Fill the parent less `margin`: one number for all sides, `(h, v)`,
    /// or `[left, top, right, bottom]`.
    pub fn fill_with(mut self, margin: impl Into<Margin>) -> Self {
        self.fill = Some(margin.into());
        self
    }

    /// Width with the label shown, px.
    pub fn labelled_width(&self, window: &mut Window, style: &LaceStyle) -> f32 {
        let text = measure_text(&self.label, window, style);
        let icon = if self.icon.is_some() { ICON + if text > 0.0 { ICON_GAP } else { 0.0 } } else { 0.0 };
        (text + icon + 2.0 * PAD).max(self.min_width)
    }

    /// Width with the icon alone, px.
    pub fn icon_width(&self, style: &LaceStyle) -> f32 {
        (ICON + 2.0 * 6.0).max(style.control_height)
    }

    fn may_collapse(&self) -> bool {
        self.icon.is_some() && !self.label.is_empty() && self.collapse != Collapse::Never
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

impl PushButton {
    fn render_face(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
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
        let may_collapse = self.may_collapse();
        let labelled = self.labelled_width(window, &style);
        let icon_only = self.icon_width(&style);
        // A lone collapsible button measures the width its parent gave it.
        let seen = (may_collapse && self.collapse == Collapse::Alone)
            .then(|| window.use_keyed_state(child_id(&self.id, "width"), cx, |_, _| f32::MAX));
        let collapsed = may_collapse
            && match self.collapse {
                Collapse::Row(c) => c,
                // The canvas sees the padding box: add back the 1 px border.
                Collapse::Alone => seen.as_ref().is_some_and(|w| *w.read(cx) + 2.0 + 0.5 < labelled),
                Collapse::Never => false,
            };
        let label = self.label.clone();
        let el = div()
            .id(self.id.clone())
            .h(px(style.control_height))
            .px(px(if collapsed { 6.0 } else { PAD }))
            .flex()
            .items_center()
            .justify_center()
            .gap(px(ICON_GAP))
            .overflow_hidden()
            .text_size(px(style.font_size))
            .text_color(hsla(face.text))
            .when_some(self.icon.clone(), |d, icon| d.child(div().flex_none().size(px(ICON)).child(icon(hsla(face.text)))))
            .when(!collapsed, |d| d.child(div().whitespace_nowrap().child(self.label)));
        let el = match (self.fill.is_some(), self.collapse) {
            (true, _) => el.w_full().h_full(),
            (false, Collapse::Alone) if may_collapse => {
                el.flex_basis(px(labelled)).flex_shrink(1.).min_w(px(icon_only))
            }
            (false, Collapse::Row(_)) if may_collapse => {
                el.flex_none().w(px(if collapsed { icon_only } else { labelled }))
            }
            _ => el.flex_none().min_w(px(self.min_width)),
        };
        let el = match seen {
            Some(seen) => el.relative().child(
                canvas(
                    move |b, _, cx| {
                        let w = f32::from(b.size.width);
                        seen.update(cx, |s, cx| {
                            if (*s - w).abs() > 0.5 {
                                *s = w;
                                cx.notify();
                            }
                        });
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full(),
            ),
            None => el,
        };
        // The label is the name; with it gone, the tooltip says it.
        let el = el.when(collapsed, |d| d.tooltip(move |_, cx| cx.new(|_| TipView(label.clone())).into()));
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

/// Width of `text` in the body font, px.
pub fn measure_text(text: &str, window: &mut Window, style: &LaceStyle) -> f32 {
    if text.is_empty() {
        return 0.0;
    }
    let run = TextRun {
        len: text.len(),
        font: window.text_style().font(),
        color: gpui_kit::gpui::black(),
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    f32::from(window.text_system().layout_line(text, px(style.font_size), &[run], None).width)
}

/// Which collapsible button of a [`ButtonRow`] gives up its label first.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FoldOrder {
    /// The last button first, then leftward (a toolbar overflowing).
    #[default]
    FromEnd,
    /// The first button first, then rightward.
    FromStart,
    /// All at once (Weave's `LabelCollapse` row rule).
    Together,
}

/// How many of a row's collapsible buttons are folded, with the room the
/// row was last given.
#[derive(Clone, Copy)]
struct RowFold {
    room: f32,
    folded: usize,
}

/// A row of push buttons whose labels give way one button at a time.
///
/// Squeezed below its labelled width, the row folds its collapsible buttons
/// to their icons one by one, in [`FoldOrder`], until it fits; the tooltip
/// keeps each folded label. Folding is decided with hysteresis, as Weave's
/// splitter restacks: a button folds as soon as the row is short of room,
/// but only unfolds once the row has its labelled width plus
/// [`hysteresis`](Self::hysteresis) px to spare, so a splitter dragged
/// back and forth around one width does not make a label flicker.
#[derive(IntoElement)]
pub struct ButtonRow {
    id: ElementId,
    buttons: Vec<PushButton>,
    gap: f32,
    reserve: f32,
    order: FoldOrder,
    hysteresis: f32,
}

impl ButtonRow {
    /// An empty row.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self { id: id.into(), buttons: Vec::new(), gap: 6.0, reserve: 0.0, order: FoldOrder::FromEnd, hysteresis: 12.0 }
    }

    /// Add a button; it folds with the row if it has an icon and a label.
    pub fn button(mut self, b: PushButton) -> Self {
        self.buttons.push(b);
        self
    }

    /// Gap between buttons, px.
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// Room the row must leave for anything beside it, px.
    pub fn reserve(mut self, px: f32) -> Self {
        self.reserve = px;
        self
    }

    /// Which button folds first.
    pub fn order(mut self, order: FoldOrder) -> Self {
        self.order = order;
        self
    }

    /// Spare room a folded button needs before its label returns, px
    /// (default 12; 0 unfolds exactly when the label fits).
    pub fn hysteresis(mut self, px: f32) -> Self {
        self.hysteresis = px.max(0.0);
        self
    }
}

/// The new fold count of a row given `room`, from `folded` before.
///
/// `need(k)` is the row's width with `k` buttons folded; it falls as `k`
/// grows. Fold while short of room; unfold while the next label fits with
/// `hysteresis` to spare. The two thresholds are `hysteresis` apart, so no
/// room is both short enough to fold and wide enough to unfold.
fn refold(room: f32, folded: usize, steps: usize, hysteresis: f32, need: impl Fn(usize) -> f32) -> usize {
    let mut k = folded.min(steps);
    while k < steps && room + 0.5 < need(k) {
        k += 1;
    }
    while k > 0 && room + 0.5 >= need(k - 1) + hysteresis {
        k -= 1;
    }
    k
}

impl RenderOnce for ButtonRow {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let style = cx.lace().clone();
        let fold = window.use_keyed_state(child_id(&self.id, "fold"), cx, |_, _| RowFold { room: f32::MAX, folded: 0 });
        let gaps = self.gap * self.buttons.len().saturating_sub(1) as f32;
        let widths: Vec<f32> = self.buttons.iter().map(|b| b.labelled_width(window, &style)).collect();
        let labelled: f32 = widths.iter().sum::<f32>() + gaps + self.reserve;
        // The collapsible buttons in the order they fold, with what each
        // one's label adds over its icon.
        // A row member folds if it has an icon to fall back on and a label.
        let can_fold = |b: &PushButton| b.icon.is_some() && !b.label.is_empty();
        let mut order: Vec<usize> = (0..self.buttons.len()).filter(|&i| can_fold(&self.buttons[i])).collect();
        if self.order == FoldOrder::FromEnd {
            order.reverse();
        }
        let saved: Vec<f32> = order.iter().map(|&i| (widths[i] - self.buttons[i].icon_width(&style)).max(0.0)).collect();
        let need = |k: usize| labelled - saved[..k].iter().sum::<f32>();
        let RowFold { room, folded } = *fold.read(cx);
        let folded = if self.order == FoldOrder::Together {
            // Two states only: nothing or everything folded.
            let all = order.len();
            let together = |k: usize| if k == 0 { need(0) } else { need(all) };
            let k = refold(room, folded.min(1), 1.min(all), self.hysteresis, together);
            k * all
        } else {
            refold(room, folded, order.len(), self.hysteresis, need)
        };
        fold.update(cx, |f, _| f.folded = folded);
        let mut shut = vec![false; self.buttons.len()];
        for &i in &order[..folded] {
            shut[i] = true;
        }
        let buttons = self.buttons.into_iter().zip(shut).map(|(mut b, shut)| {
            if b.icon.is_some() {
                b.collapse = Collapse::Row(shut);
            }
            b
        });
        div()
            .id(self.id)
            .relative()
            .flex()
            .items_center()
            .gap(px(self.gap))
            .w_full()
            .min_w_0()
            .overflow_hidden()
            .children(buttons)
            .child(
                canvas(
                    move |b, _, cx| {
                        let w = f32::from(b.size.width);
                        fold.update(cx, |f, cx| {
                            if (f.room - w).abs() > 0.5 {
                                f.room = w;
                                cx.notify();
                            }
                        });
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full(),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::refold;

    // Three buttons, each saving 50 px when folded: 300, 250, 200, 150.
    fn need(k: usize) -> f32 {
        300.0 - 50.0 * k as f32
    }

    #[test]
    fn folds_one_at_a_time() {
        assert_eq!(refold(300.0, 0, 3, 12.0, need), 0);
        assert_eq!(refold(290.0, 0, 3, 12.0, need), 1);
        assert_eq!(refold(240.0, 0, 3, 12.0, need), 2);
        assert_eq!(refold(100.0, 0, 3, 12.0, need), 3);
    }

    #[test]
    fn unfolds_only_past_the_hysteresis() {
        // Folded once at 290: the label needs 300 + 12 to come back.
        assert_eq!(refold(305.0, 1, 3, 12.0, need), 1);
        assert_eq!(refold(311.0, 1, 3, 12.0, need), 1);
        assert_eq!(refold(312.0, 1, 3, 12.0, need), 0);
        // Wide again in one jump: everything unfolds.
        assert_eq!(refold(400.0, 3, 3, 12.0, need), 0);
        // Inside the band the state holds either way.
        assert_eq!(refold(255.0, 1, 3, 12.0, need), 1);
        assert_eq!(refold(255.0, 2, 3, 12.0, need), 2);
    }
}
