//! Tabs, group boxes, headers, item rows, menus, tooltips and tool boxes
//! (`_containers.py`, `_popup.py`).

use std::rc::Rc;

use gpui_kit::gpui::{
    div, prelude::FluentBuilder, px, AnyElement, App, BoxShadow, Div, ElementId, InteractiveElement,
    IntoElement, ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window,
};

use super::interaction::Interaction;
use crate::glyph::{self, Dir};
use crate::style::rules::{self, TAB_UNDERLINE};
use crate::style::{hsla, ActiveStyle, LaceStyle, State};

type Pick = Rc<dyn Fn(usize, &mut Window, &mut App)>;
type Handler = Rc<dyn Fn(&mut Window, &mut App)>;

fn indexed(id: &ElementId, label: &str, i: usize) -> ElementId {
    ElementId::NamedChild(std::sync::Arc::new(id.clone()), SharedString::from(format!("{label}{i}")))
}

// ---------------------------------------------------------------------------
// Tabs
// ---------------------------------------------------------------------------

/// Which side of the pane a tab bar sits on (`QTabWidget::TabPosition`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TabPosition {
    /// Above the pane.
    #[default]
    North,
    /// Below.
    South,
    /// Left. Labels stay horizontal: gpui cannot rotate text.
    West,
    /// Right.
    East,
}

impl TabPosition {
    fn vertical(self) -> bool {
        matches!(self, Self::West | Self::East)
    }
}

/// One tab, painted for `s` (`s.selected` marks the current tab).
pub fn tab_div(style: &LaceStyle, s: State, pos: TabPosition) -> Div {
    let t = rules::tab(style, s);
    let r = px(style.control_radius);
    let el = div()
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .text_size(px(style.font_size))
        .text_color(hsla(t.text))
        .when_some(t.fill, |d, f| d.bg(hsla(f)));
    // Round only the corners away from the pane.
    let el = match pos {
        TabPosition::North => el.rounded_t(r),
        TabPosition::South => el.rounded_b(r),
        TabPosition::West => el.rounded_l(r),
        TabPosition::East => el.rounded_r(r),
    };
    let el = match pos {
        TabPosition::North | TabPosition::South => el.h(px(style.control_height + 4.0)).px(px(14.)),
        _ => el.w_full().py(px(10.)).px(px(6.)),
    };
    let el = match (t.underline, pos) {
        (Some(u), TabPosition::North) => el.border_b(px(TAB_UNDERLINE)).border_color(hsla(u)),
        (Some(u), TabPosition::South) => el.border_t(px(TAB_UNDERLINE)).border_color(hsla(u)),
        (Some(u), TabPosition::West) => el.border_r(px(TAB_UNDERLINE)).border_color(hsla(u)),
        (Some(u), TabPosition::East) => el.border_l(px(TAB_UNDERLINE)).border_color(hsla(u)),
        (None, _) => el,
    };
    match t.ring {
        Some(ring) => el.border(px(style.focus_width)).border_color(hsla(ring)).rounded(r),
        None => el,
    }
}

/// A `QTabBar`.
#[derive(IntoElement)]
pub struct TabBar {
    id: ElementId,
    tabs: Vec<(SharedString, bool)>,
    current: usize,
    position: TabPosition,
    closable: bool,
    forced: Option<State>,
    on_select: Option<Pick>,
    on_close: Option<Pick>,
}

impl TabBar {
    /// A tab bar over `labels`.
    pub fn new(id: impl Into<ElementId>, labels: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        Self {
            id: id.into(),
            tabs: labels.into_iter().map(|l| (l.into(), true)).collect(),
            current: 0,
            position: TabPosition::North,
            closable: false,
            forced: None,
            on_select: None,
            on_close: None,
        }
    }

    /// The selected tab.
    pub fn current(mut self, ix: usize) -> Self {
        self.current = ix;
        self
    }

    /// Disable tab `ix`.
    pub fn disable_tab(mut self, ix: usize) -> Self {
        if let Some(t) = self.tabs.get_mut(ix) {
            t.1 = false;
        }
        self
    }

    /// Where the bar sits.
    pub fn position(mut self, pos: TabPosition) -> Self {
        self.position = pos;
        self
    }

    /// Close buttons on every tab.
    pub fn closable(mut self) -> Self {
        self.closable = true;
        self
    }

    /// Paint every tab in `state` (the gallery).
    pub fn state(mut self, state: State) -> Self {
        self.forced = Some(state);
        self
    }

    /// Called with the clicked tab.
    pub fn on_select(mut self, f: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(f));
        self
    }

    /// Called with the tab whose close button was clicked.
    pub fn on_close(mut self, f: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for TabBar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let style = cx.lace().clone();
        let pos = self.position;
        let mut bar = div().flex().flex_none();
        bar = if pos.vertical() { bar.flex_col().w(px(48.)) } else { bar.flex_row() };
        for (i, (label, enabled)) in self.tabs.into_iter().enumerate() {
            let tid = indexed(&self.id, "tab", i);
            let ix = Interaction::new(tid.clone(), window, cx).focusable(&tid, window, cx).force(self.forced);
            let s = ix.state(window, cx).union(State {
                selected: i == self.current,
                disabled: !enabled,
                ..State::NORMAL
            });
            let mut tab = tab_div(&style, s, pos).id(tid.clone()).gap(px(6.)).child(label);
            if self.closable {
                let close = TabClose::new(indexed(&self.id, "close", i)).selected(s.selected).disabled(s.disabled);
                let close = match &self.on_close {
                    Some(f) => {
                        let f = f.clone();
                        close.on_click(move |window, cx| f(i, window, cx))
                    }
                    None => close,
                };
                tab = tab.child(close);
            }
            let tab = ix.bind(tab, s.disabled);
            let tab = match self.on_select.clone().filter(|_| !s.disabled) {
                Some(f) => tab.on_click(move |_, window, cx| f(i, window, cx)),
                None => tab,
            };
            bar = bar.child(tab);
        }
        bar
    }
}

/// A `QTabWidget`: a tab bar and its pane, in a frame.
#[derive(IntoElement)]
pub struct TabWidget {
    bar: TabBar,
    pane: Option<AnyElement>,
}

impl TabWidget {
    /// A tab widget whose bar is `bar`, showing `pane`.
    pub fn new(bar: TabBar, pane: impl IntoElement) -> Self {
        Self { bar, pane: Some(pane.into_any_element()) }
    }
}

impl RenderOnce for TabWidget {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let style = cx.lace().clone();
        let pos = self.bar.position;
        let line = hsla(rules::frame_line(&style, State::NORMAL));
        let pane = div()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .border_1()
            .border_color(line)
            .rounded(px(style.corner_radius))
            .p(px(6.))
            .children(self.pane);
        let el = div().flex().size_full();
        match pos {
            TabPosition::North => el.flex_col().child(self.bar).child(pane),
            TabPosition::South => el.flex_col().child(pane).child(self.bar),
            TabPosition::West => el.flex_row().child(self.bar).child(pane),
            TabPosition::East => el.flex_row().child(pane).child(self.bar),
        }
    }
}

// ---------------------------------------------------------------------------
// Group box, frames
// ---------------------------------------------------------------------------

/// A `QFrame` box: a rounded outline.
pub fn frame_box(style: &LaceStyle, s: State) -> Div {
    div()
        .border_1()
        .border_color(hsla(rules::frame_line(style, s)))
        .rounded(px(style.corner_radius))
}

/// A `QGroupBox`: a title over a rounded frame.
#[derive(IntoElement)]
pub struct GroupBox {
    title: Option<SharedString>,
    disabled: bool,
    children: Vec<AnyElement>,
}

impl GroupBox {
    /// A group box titled `title`.
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self { title: Some(title.into()), disabled: false, children: Vec::new() }
    }

    /// An untitled box.
    pub fn untitled() -> Self {
        Self { title: None, disabled: false, children: Vec::new() }
    }

    /// Disable (greys the title and frame).
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl ParentElement for GroupBox {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for GroupBox {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let style = cx.lace().clone();
        let s = State::NORMAL.with_disabled(self.disabled);
        // Fills a stretched parent, so boxes side by side match heights.
        div()
            .h_full()
            .flex()
            .flex_col()
            .gap(px(4.))
            .text_size(px(style.font_size))
            .text_color(hsla(style.pal(s).window_text))
            .when_some(self.title, |d, t| d.child(div().px(px(1.)).child(t)))
            .child(frame_box(&style, s).flex_1().min_h_0().p(px(10.)).flex().flex_col().gap(px(8.)).children(self.children))
    }
}

// ---------------------------------------------------------------------------
// Header
// ---------------------------------------------------------------------------

/// Sort order shown on a header section.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sort {
    /// Ascending (chevron up).
    Ascending,
    /// Descending.
    Descending,
}

/// One header section painted for `s`: fill, bottom line, trailing
/// separator, caption and optional sort chevron.
pub fn header_section(style: &LaceStyle, s: State, caption: SharedString, sort: Option<Sort>) -> Div {
    let h = rules::header(style, s);
    let arrow = hsla(h.arrow);
    div()
        .h(px(style.control_height))
        .flex()
        .items_center()
        .px(px(4.))
        .gap(px(4.))
        .bg(hsla(h.fill))
        .border_b_1()
        .border_r_1()
        .border_color(hsla(h.line))
        .text_size(px(style.font_size))
        .text_color(hsla(h.text))
        .overflow_hidden()
        .child(div().flex_1().min_w_0().overflow_hidden().whitespace_nowrap().child(caption))
        .when_some(sort, move |d, sort| {
            let dir = if sort == Sort::Ascending { Dir::Up } else { Dir::Down };
            d.child(glyph::glyph_box(8.0, 8.0, move |b, window, _| {
                glyph::paint_chevron(window, b, dir, arrow, 6.0)
            }))
        })
}

// ---------------------------------------------------------------------------
// Rows, menus, popups, tooltips
// ---------------------------------------------------------------------------

/// An item-view row painted for `s`.
pub fn item_row_div(style: &LaceStyle, s: State) -> Div {
    let (fill, text) = rules::item_row(style, s);
    let el = div()
        .flex()
        .items_center()
        .text_size(px(style.font_size))
        .text_color(hsla(text))
        .when_some(fill, |d, f| d.bg(hsla(f)));
    match s.has_focus().then(|| crate::style::paint::focus_ring(style, s)) {
        // An overlay, so the ring never shifts the row's cells.
        Some(ring) => el.relative().child(
            div()
                .absolute()
                .inset_0()
                .border(px(style.focus_width))
                .border_color(hsla(ring))
                .rounded(px(style.control_radius)),
        ),
        None => el,
    }
}

/// A popup panel (menus, combo lists): window fill, outline, soft shadow.
pub fn popup_panel(style: &LaceStyle) -> Div {
    let (fill, line) = rules::popup(style, State::NORMAL);
    div()
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
}

/// A `QMenu` item.
#[derive(IntoElement)]
pub struct MenuItem {
    id: ElementId,
    label: SharedString,
    shortcut: Option<SharedString>,
    checked: Option<bool>,
    submenu: bool,
    disabled: bool,
    forced: Option<State>,
    on_click: Option<Handler>,
}

impl MenuItem {
    /// A menu item.
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            shortcut: None,
            checked: None,
            submenu: false,
            disabled: false,
            forced: None,
            on_click: None,
        }
    }

    /// Shortcut text, right-aligned.
    pub fn shortcut(mut self, s: impl Into<SharedString>) -> Self {
        self.shortcut = Some(s.into());
        self
    }

    /// Checkable, with its current state.
    pub fn checked(mut self, on: bool) -> Self {
        self.checked = Some(on);
        self
    }

    /// Show a submenu chevron.
    pub fn submenu(mut self) -> Self {
        self.submenu = true;
        self
    }

    /// Disable.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Paint in `state`.
    pub fn state(mut self, state: State) -> Self {
        self.forced = Some(state);
        self
    }

    /// Called on click.
    pub fn on_click(mut self, f: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for MenuItem {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let ix = Interaction::new(self.id.clone(), window, cx).force(self.forced);
        let s = ix.state(window, cx).with_disabled(self.disabled);
        let s = State { checked: s.checked || self.checked == Some(true), ..s };
        let style = cx.lace().clone();
        let (fill, text) = rules::menu_item(&style, s);
        let check = hsla(if fill.is_some() { text } else { rules::menu_check(&style, s) });
        let text_c = hsla(text);
        let el = div()
            .id(self.id.clone())
            .h(px(style.control_height))
            .mx(px(3.))
            .pr(px(8.))
            .flex()
            .items_center()
            .rounded(px(style.control_radius))
            .text_size(px(style.font_size))
            .text_color(text_c)
            .when_some(fill, |d, f| d.bg(hsla(f)))
            .child(div().w(px(22.)).h_full().when(s.checked, |d| {
                d.child(glyph::painter(move |b, window, _| glyph::paint_check(window, glyph::centered(b, 10.0), check)))
            }))
            .child(div().flex_1().whitespace_nowrap().child(self.label))
            .when_some(self.shortcut, |d, k| d.child(div().pl(px(16.)).opacity(0.7).child(k)))
            .when(self.submenu, |d| {
                d.child(glyph::glyph_box(12.0, 12.0, move |b, window, _| {
                    glyph::paint_chevron(window, b, Dir::Right, text_c, 7.0)
                }))
            });
        let el = ix.bind(el, s.disabled);
        match self.on_click.filter(|_| !s.disabled) {
            Some(f) => el.on_click(move |_, window, cx| f(window, cx)),
            None => el,
        }
    }
}

/// A menu separator.
pub fn menu_separator(style: &LaceStyle) -> Div {
    div().my(px(3.)).mx(px(8.)).h(px(1.)).bg(hsla(rules::frame_line(style, State::NORMAL)))
}

/// A `QMenuBar` item.
#[derive(IntoElement)]
pub struct MenuBarItem {
    id: ElementId,
    label: SharedString,
    open: bool,
    disabled: bool,
    forced: Option<State>,
    on_click: Option<Handler>,
}

impl MenuBarItem {
    /// A menu-bar item.
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self { id: id.into(), label: label.into(), open: false, disabled: false, forced: None, on_click: None }
    }

    /// Its menu is open.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Disable.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Paint in `state`.
    pub fn state(mut self, state: State) -> Self {
        self.forced = Some(state);
        self
    }

    /// Called on click.
    pub fn on_click(mut self, f: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for MenuBarItem {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let ix = Interaction::new(self.id.clone(), window, cx).force(self.forced);
        let mut s = ix.state(window, cx).with_disabled(self.disabled);
        s.pressed |= self.open;
        let style = cx.lace().clone();
        let (fill, text) = rules::menu_bar_item(&style, s);
        let el = div()
            .id(self.id.clone())
            .h(px(style.control_height - 4.0))
            .px(px(8.))
            .flex()
            .flex_none()
            .items_center()
            .rounded(px(style.control_radius))
            .text_size(px(style.font_size))
            .text_color(hsla(text))
            .when_some(fill, |d, f| d.bg(hsla(f)))
            .child(self.label);
        let el = ix.bind(el, s.disabled);
        match self.on_click.filter(|_| !s.disabled) {
            Some(f) => el.on_click(move |_, window, cx| f(window, cx)),
            None => el,
        }
    }
}

/// A tooltip bubble around `text`.
pub fn tooltip(style: &LaceStyle, s: State, text: impl Into<SharedString>) -> Div {
    let (fill, line, ink) = rules::tooltip(style, s);
    div()
        .px(px(6.))
        .py(px(3.))
        .bg(hsla(fill))
        .border_1()
        .border_color(hsla(line))
        .rounded(px(style.control_radius))
        .text_size(px(style.font_size))
        .text_color(hsla(ink))
        .child(text.into())
}

/// A tool-box page tab painted for `s`.
pub fn tool_box_tab(style: &LaceStyle, s: State, label: SharedString) -> Div {
    let (fill, line, text) = rules::tool_box_tab(style, s);
    div()
        .h(px(style.control_height))
        .px(px(6.))
        .flex()
        .items_center()
        .bg(hsla(fill))
        .border_1()
        .border_color(hsla(line))
        .rounded(px(style.control_radius))
        .text_size(px(style.font_size))
        .text_color(hsla(text))
        .overflow_hidden()
        .whitespace_nowrap()
        .child(label)
}

/// A `QToolBox`: stacked page tabs, the current page open below its tab.
#[derive(IntoElement)]
pub struct ToolBox {
    id: ElementId,
    pages: Vec<(SharedString, AnyElement)>,
    current: usize,
    on_select: Option<Pick>,
}

impl ToolBox {
    /// An empty tool box.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self { id: id.into(), pages: Vec::new(), current: 0, on_select: None }
    }

    /// Add a page.
    pub fn page(mut self, label: impl Into<SharedString>, content: impl IntoElement) -> Self {
        self.pages.push((label.into(), content.into_any_element()));
        self
    }

    /// The open page.
    pub fn current(mut self, ix: usize) -> Self {
        self.current = ix;
        self
    }

    /// Called with the clicked page.
    pub fn on_select(mut self, f: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for ToolBox {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let style = cx.lace().clone();
        let mut col = div().flex().flex_col().gap(px(2.)).size_full();
        for (i, (label, content)) in self.pages.into_iter().enumerate() {
            let tid = indexed(&self.id, "page", i);
            let ix = Interaction::new(tid.clone(), window, cx);
            let s = ix.state(window, cx);
            let tab = ix.bind(tool_box_tab(&style, s, label).id(tid), false);
            let tab = match self.on_select.clone() {
                Some(f) => tab.on_click(move |_, window, cx| f(i, window, cx)),
                None => tab,
            };
            col = col.child(tab);
            if i == self.current {
                col = col.child(div().flex_1().min_h_0().p(px(4.)).child(content));
            }
        }
        col
    }
}

// ---------------------------------------------------------------------------
// Tab close
// ---------------------------------------------------------------------------

/// A tab's close button: a cross with a hover wash.
#[derive(IntoElement)]
pub struct TabClose {
    id: ElementId,
    selected: bool,
    disabled: bool,
    forced: Option<State>,
    on_click: Option<Handler>,
}

/// Tab close button side, px.
pub const TAB_CLOSE: f32 = 16.0;

impl TabClose {
    /// A close button.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self { id: id.into(), selected: false, disabled: false, forced: None, on_click: None }
    }

    /// On the current tab (full-strength ink).
    pub fn selected(mut self, on: bool) -> Self {
        self.selected = on;
        self
    }

    /// Disable.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Paint in `state`.
    pub fn state(mut self, state: State) -> Self {
        self.forced = Some(state);
        self
    }

    /// Called on click.
    pub fn on_click(mut self, f: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for TabClose {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let ix = Interaction::new(self.id.clone(), window, cx).force(self.forced);
        let s = ix.state(window, cx).union(State { selected: self.selected, disabled: self.disabled, ..State::NORMAL });
        let style = cx.lace().clone();
        let (wash, ink) = rules::tab_close(&style, s);
        let ink = hsla(ink);
        let el = div()
            .id(self.id.clone())
            .size(px(TAB_CLOSE))
            .flex_none()
            .rounded(px(3.))
            .when_some(wash, |d, w| d.bg(hsla(w)))
            .child(glyph::cross(ink, 8.0));
        let el = ix.bind(el, s.disabled);
        match self.on_click.filter(|_| !s.disabled) {
            Some(f) => el.on_click(move |_, window, cx| {
                cx.stop_propagation();
                f(window, cx)
            }),
            None => el,
        }
    }
}

/// `id` with an index, for list children.
pub fn nth_id(id: &ElementId, i: usize) -> ElementId {
    indexed(id, "n", i)
}
