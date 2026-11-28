//! Tabs, group boxes, headers, item rows, menus, tooltips and tool boxes
//! (`_containers.py`, `_popup.py`).

use std::rc::Rc;

use gpui_kit::gpui::{
    div, prelude::FluentBuilder, px, AnyElement, App, BoxShadow, Div, ElementId, Hsla, InteractiveElement,
    IntoElement, ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window,
};

use super::interaction::{child_id, Interaction};
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
            .rounded(px(style.frame_radius))
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
        .rounded(px(style.frame_radius))
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

type Toggle = Rc<dyn Fn(bool, &mut Window, &mut App)>;

/// A `QGroupBox` that folds away: a chevron and title over a rounded frame.
/// Clicking the title, or Space / Enter while it has focus, opens and closes
/// the box.
///
/// Open, it is a [`GroupBox`] with a chevron before the title. Closed, the
/// content and its frame go, and a rule runs on from the title, so a
/// column of folded boxes still reads as sections. Unlike [`GroupBox`] it
/// keeps its natural height: a box that folds should give its room back.
///
/// The open state lives with the widget (keyed by its id) unless
/// [`open`](Self::open) pins it; [`on_toggle`](Self::on_toggle) hears every
/// change either way.
///
/// ```ignore
/// CollapsibleGroupBox::new("advanced", "Advanced")
///     .default_open(false)
///     .child(CheckBox::new("wrap", "Wrap lines"))
/// ```
#[derive(IntoElement)]
pub struct CollapsibleGroupBox {
    id: ElementId,
    title: SharedString,
    default_open: bool,
    open: Option<bool>,
    disabled: bool,
    forced: Option<State>,
    on_toggle: Option<Toggle>,
    children: Vec<AnyElement>,
}

impl CollapsibleGroupBox {
    /// A box titled `title`, open until the user folds it.
    pub fn new(id: impl Into<ElementId>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            default_open: true,
            open: None,
            disabled: false,
            forced: None,
            on_toggle: None,
            children: Vec::new(),
        }
    }

    /// Whether the box starts open (ignored once it has been shown, and when
    /// [`open`](Self::open) pins the state).
    pub fn default_open(mut self, open: bool) -> Self {
        self.default_open = open;
        self
    }

    /// Pin the open state: the box shows `open` and only reports clicks
    /// through [`on_toggle`](Self::on_toggle).
    pub fn open(mut self, open: bool) -> Self {
        self.open = Some(open);
        self
    }

    /// Disable (greys the title and frame; the box no longer folds).
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Paint the title row in `state` (gallery rendering).
    pub fn state(mut self, state: State) -> Self {
        self.forced = Some(state);
        self
    }

    /// Called with the new open state whenever the user folds or unfolds it.
    pub fn on_toggle(mut self, f: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_toggle = Some(Rc::new(f));
        self
    }
}

impl ParentElement for CollapsibleGroupBox {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for CollapsibleGroupBox {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let style = cx.lace().clone();
        let header_id = child_id(&self.id, "header");
        let default_open = self.default_open;
        let kept = window.use_keyed_state(child_id(&self.id, "open"), cx, move |_, _| default_open);
        let open = self.open.unwrap_or(*kept.read(cx));

        let ix = Interaction::new(header_id.clone(), window, cx)
            .focusable(&header_id, window, cx)
            .force(self.forced);
        let s = ix.state(window, cx).with_disabled(self.disabled);
        let frame_state = State::NORMAL.with_disabled(self.disabled);
        let text = style.pal(s).window_text;
        // The chevron sits back until the title row is pointed at, as the
        // tree's branch chevron does against its row.
        let arrow = if s.hovered || s.pressed || s.has_focus() { text } else { crate::style::paint::muted(&style, s) };
        let ring = s.has_focus().then(|| crate::style::paint::focus_ring(&style, s));
        let dir = if open { Dir::Down } else { Dir::Right };

        let header = div()
            .id(header_id)
            .flex()
            .items_center()
            .gap(px(4.))
            .h(px(style.font_size + 8.0))
            .pr(px(4.))
            .rounded(px(style.control_radius))
            .border_1()
            .border_color(ring.map_or(gpui_kit::gpui::transparent_black(), hsla))
            .when(!self.disabled, |d| d.cursor_pointer())
            .child(glyph::glyph_box(14.0, 14.0, move |b, window, _| {
                glyph::paint_chevron(window, b, dir, hsla(arrow), 0.0)
            }))
            .child(div().flex_none().text_color(hsla(text)).child(self.title))
            .when(!open, |d| {
                d.child(div().flex_1().ml(px(4.)).h(px(1.)).bg(hsla(rules::frame_line(&style, frame_state))))
            });
        let header = ix.bind(header, self.disabled);
        let header = if self.disabled {
            header
        } else {
            let kept_click = kept.clone();
            let pinned = self.open.is_some();
            let report = self.on_toggle.clone();
            // gpui raises the click for Space and Enter on the focused header
            // too (on key-up), so no key handler of our own: one would fold
            // the box twice.
            header.on_click(move |_, window, cx| {
                let next = !open;
                if !pinned {
                    kept_click.update(cx, |o, cx| {
                        *o = next;
                        cx.notify();
                    });
                }
                if let Some(f) = &report {
                    f(next, window, cx);
                }
            })
        };

        div()
            .flex()
            .flex_col()
            .gap(px(4.))
            .text_size(px(style.font_size))
            .text_color(hsla(style.pal(frame_state).window_text))
            .child(header)
            .when(open, |d| {
                d.child(frame_box(&style, frame_state).p(px(10.)).flex().flex_col().gap(px(8.)).children(self.children))
            })
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

/// Width of a menu item's leading column (icon, check mark or dot), px.
pub const MENU_LEAD: f32 = 22.0;
/// Side of a menu item's icon, px.
pub const MENU_ICON: f32 = 16.0;
/// A menu item's horizontal inset in its panel, px (each side).
pub const MENU_INSET: f32 = 3.0;
/// A menu item's padding after the label (or shortcut, or arrow), px.
const MENU_TAIL: f32 = 8.0;
/// The gap before a shortcut, px.
const MENU_SHORTCUT_GAP: f32 = 16.0;
/// The submenu arrow's box, px.
const MENU_ARROW: f32 = 12.0;

/// A hover handler: `true` on entering, `false` on leaving.
type HoverHandler = Rc<dyn Fn(bool, &mut Window, &mut App)>;

/// A `QMenu` item.
///
/// The leading column holds the item's icon, or, without one, its check
/// mark (or an exclusive item's dot) while checked: a checkable `QAction`
/// draws its mark in the icon column, and an icon takes its place.
#[derive(IntoElement)]
pub struct MenuItem {
    id: ElementId,
    label: SharedString,
    icon: Option<super::button::Icon>,
    shortcut: Option<SharedString>,
    checked: Option<bool>,
    radio: bool,
    submenu: bool,
    disabled: bool,
    forced: Option<State>,
    selector: Option<SharedString>,
    on_click: Option<Handler>,
    on_hover: Option<HoverHandler>,
}

impl MenuItem {
    /// A menu item.
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            shortcut: None,
            checked: None,
            radio: false,
            submenu: false,
            disabled: false,
            forced: None,
            selector: None,
            on_click: None,
            on_hover: None,
        }
    }

    /// An icon in the leading column, built for the item's text colour.
    pub fn icon(mut self, f: impl Fn(Hsla) -> AnyElement + 'static) -> Self {
        self.icon = Some(Rc::new(f));
        self
    }

    /// An icon from an already shared builder.
    pub fn icon_rc(mut self, icon: Option<super::button::Icon>) -> Self {
        self.icon = icon;
        self
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

    /// One of an exclusive group (`QActionGroup`): checked, it shows a dot
    /// instead of a tick.
    pub fn radio(mut self, on: bool) -> Self {
        self.checked = Some(on);
        self.radio = true;
        self
    }

    /// A debug selector for tests (`VisualTestContext::debug_bounds`).
    pub fn selector(mut self, s: impl Into<SharedString>) -> Self {
        self.selector = Some(s.into());
        self
    }

    /// Called as the pointer enters (`true`) or leaves (`false`). An item
    /// with this handler is driven by its menu: it paints the state given
    /// by [`state`](Self::state) (hovered = the menu's current item), not
    /// the pointer's.
    pub fn on_hover(mut self, f: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_hover = Some(Rc::new(f));
        self
    }

    /// Width the item needs, px: inset, leading column, label, shortcut,
    /// arrow and tail.
    pub fn needed_width(&self, window: &mut Window, style: &LaceStyle) -> f32 {
        let label = super::button::measure_text(&self.label, window, style);
        let shortcut = self
            .shortcut
            .as_ref()
            .filter(|s| !s.is_empty())
            .map_or(0.0, |s| MENU_SHORTCUT_GAP + super::button::measure_text(s, window, style));
        let arrow = if self.submenu { MENU_ARROW } else { 0.0 };
        2.0 * MENU_INSET + MENU_LEAD + label + shortcut + arrow + MENU_TAIL
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
        // Driven by a menu, the item paints the state it is given.
        let driven = self.on_hover.is_some();
        let forced = if driven { Some(self.forced.unwrap_or(State::NORMAL)) } else { self.forced };
        let ix = Interaction::new(self.id.clone(), window, cx).force(forced);
        let s = ix.state(window, cx).with_disabled(self.disabled);
        let s = State { checked: s.checked || self.checked == Some(true), ..s };
        let style = cx.lace().clone();
        let (fill, text) = rules::menu_item(&style, s);
        let check = hsla(if fill.is_some() { text } else { rules::menu_check(&style, s) });
        let text_c = hsla(text);
        let lead = div().w(px(MENU_LEAD)).h_full().flex_none().flex().items_center().justify_center();
        let lead = match (self.icon, s.checked, self.radio) {
            (Some(icon), _, _) => lead.child(icon(text_c)),
            (None, true, false) => {
                lead.child(glyph::painter(move |b, window, _| glyph::paint_check(window, glyph::centered(b, 10.0), check)))
            }
            (None, true, true) => lead.child(div().size(px(6.)).rounded_full().bg(check)),
            (None, false, _) => lead,
        };
        let el = div()
            .id(self.id.clone())
            .h(px(style.control_height))
            .mx(px(MENU_INSET))
            .pr(px(MENU_TAIL))
            .flex()
            .items_center()
            .rounded(px(style.control_radius))
            .text_size(px(style.font_size))
            .text_color(text_c)
            .when_some(fill, |d, f| d.bg(hsla(f)))
            .when_some(self.selector, |d, sel| d.debug_selector(move || sel.to_string()))
            .child(lead)
            // A label or hint too long for a fixed-width menu elides
            // rather than spilling past the panel; the hint gives way
            // first, down to half the row.
            .child(div().flex_1().min_w_0().overflow_hidden().whitespace_nowrap().text_ellipsis().child(self.label))
            .when_some(self.shortcut.filter(|k| !k.is_empty()), |d, k| {
                d.child(
                    div()
                        .pl(px(MENU_SHORTCUT_GAP))
                        .min_w_0()
                        .max_w(gpui_kit::gpui::relative(0.5))
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .opacity(0.7)
                        .child(k),
                )
            })
            .when(self.submenu, |d| {
                d.child(glyph::glyph_box(MENU_ARROW, MENU_ARROW, move |b, window, _| {
                    glyph::paint_chevron(window, b, Dir::Right, text_c, 7.0)
                }))
            });
        let el = ix.bind(el, s.disabled);
        let el = match self.on_hover {
            Some(f) => el.on_hover(move |h, window, cx| f(*h, window, cx)),
            None => el,
        };
        match self.on_click.filter(|_| !s.disabled) {
            Some(f) => el.on_click(move |_, window, cx| f(window, cx)),
            None => el,
        }
    }
}

/// A menu section header (`QMenu::addSection`): its title in muted text,
/// then the separator line.
pub fn menu_section(style: &LaceStyle, title: impl Into<SharedString>) -> Div {
    let line = hsla(rules::frame_line(style, State::NORMAL));
    div()
        .flex()
        .items_center()
        .px(px(MENU_SECTION_INSET))
        .py(px(MENU_SECTION_PAD))
        .gap(px(MENU_SECTION_GAP))
        .text_size(px(style.font_size))
        .text_color(hsla(crate::style::paint::muted(style, State::NORMAL)))
        .child(div().flex_none().whitespace_nowrap().child(title.into()))
        .child(div().flex_1().min_w(px(MENU_SECTION_STUB)).h(px(1.)).bg(line))
}

/// Width a section header needs, px.
pub fn menu_section_width(title: &str, window: &mut Window, style: &LaceStyle) -> f32 {
    2.0 * MENU_SECTION_INSET + super::button::measure_text(title, window, style) + MENU_SECTION_GAP + MENU_SECTION_STUB
}

/// A section title's inset from the panel's sides, px (`MENU_SEPARATOR_INSET`).
const MENU_SECTION_INSET: f32 = 6.0;
/// Between a section title and its line, px.
const MENU_SECTION_GAP: f32 = 6.0;
/// Above and below a section title, px.
const MENU_SECTION_PAD: f32 = 4.0;
/// The least line after a section title, px.
const MENU_SECTION_STUB: f32 = 12.0;

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
