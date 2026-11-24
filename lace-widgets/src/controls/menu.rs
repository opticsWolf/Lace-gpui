//! A `QMenu`: a popup of items, separators and section headers whose
//! submenus fan out beside the item that opens them.
//!
//! The menu is *controlled*: the app keeps its [`MenuNav`] (which item is
//! current at each level, which submenus are open), hands it to [`Menu`]
//! each frame, and updates it from the menu's callbacks and its own keys.
//! `MenuNav` does the bookkeeping (Up and Down past separators and greyed
//! items, Right and Enter into a submenu, Left out of one), so the app only
//! decides *when*: Qt opens a submenu a moment after the pointer rests on
//! its item ([`MenuNav::hover`] then, later, [`MenuNav::settle`]), at once
//! on a click ([`MenuNav::open`]).
//!
//! ```ignore
//! let entries = vec![
//!     MenuEntry::action(MenuAction::new("Undo").icon_named("undo", cx).shortcut("Ctrl+Z")),
//!     MenuEntry::Separator,
//!     MenuEntry::submenu(MenuAction::new("Themes"), themes),
//! ];
//! Menu::new("menu", entries, self.nav.clone())
//!     .on_hover(cx.listener(|this, path: &[usize], _, cx| { this.nav.hover(path); cx.notify() }))
//!     .on_activate(cx.listener(|this, path: &[usize], window, cx| this.activate(path, window, cx)))
//! ```

use std::rc::Rc;

use gpui_kit::gpui::{
    deferred, div, point, prelude::FluentBuilder, px, AnyElement, App, Bounds, Display, Element, ElementId,
    GlobalElementId, InspectorElementId, InteractiveElement, IntoElement, LayoutId, MouseButton, ParentElement,
    Pixels, Position, RenderOnce, ScrollHandle, SharedString, StatefulInteractiveElement, Style, Styled, Window,
};

use super::button::Icon;
use super::container::{menu_section, menu_section_width, menu_separator, popup_panel, MenuItem};
use super::interaction::child_id;
use crate::style::{hsla, ActiveStyle, LaceStyle, State};

/// Whether a menu item can be checked, and how.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Check {
    /// Not checkable.
    #[default]
    None,
    /// Checkable, with a tick while on.
    Check(bool),
    /// One of an exclusive group, with a dot while on.
    Radio(bool),
}

/// What a menu item shows: label, icon, shortcut, check state.
#[derive(Clone, Default)]
pub struct MenuAction {
    /// The text.
    pub label: SharedString,
    /// The icon in the leading column.
    pub icon: Option<Icon>,
    /// Right-aligned text: a shortcut, or any short hint.
    pub shortcut: Option<SharedString>,
    /// Whether, and how, it is checked.
    pub check: Check,
    /// Greyed out: never current, never run.
    pub disabled: bool,
    /// A debug selector for tests.
    pub selector: Option<SharedString>,
}

impl MenuAction {
    /// An item labelled `label`.
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self { label: label.into(), ..Self::default() }
    }

    /// An icon, built for the item's text colour.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// The registered icon `name` ([`crate::icons`]), if there is one.
    pub fn icon_named(mut self, name: &str, cx: &App) -> Self {
        if let Some(icon) = crate::icons::named(name, cx) {
            self.icon = Some(icon);
        }
        self
    }

    /// Right-aligned text.
    pub fn shortcut(mut self, s: impl Into<SharedString>) -> Self {
        self.shortcut = Some(s.into());
        self
    }

    /// Checkable, ticked while `on`.
    pub fn checked(mut self, on: bool) -> Self {
        self.check = Check::Check(on);
        self
    }

    /// Exclusive, dotted while `on`.
    pub fn radio(mut self, on: bool) -> Self {
        self.check = Check::Radio(on);
        self
    }

    /// Greyed out.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// A debug selector for tests.
    pub fn selector(mut self, s: impl Into<SharedString>) -> Self {
        self.selector = Some(s.into());
        self
    }
}

/// One row of a menu.
#[derive(Clone)]
pub enum MenuEntry {
    /// An item that runs something.
    Action(MenuAction),
    /// An item that opens a submenu of these entries.
    Submenu(MenuAction, Vec<MenuEntry>),
    /// A line.
    Separator,
    /// A section header: a muted title and a line.
    Section(SharedString),
    /// A row holding a widget (Qt's `QWidgetAction`): a slider, say. The
    /// keyboard passes over it; the pointer works it.
    Widget(MenuWidget),
}

/// The widget of a [`MenuEntry::Widget`] row, built afresh each frame.
#[derive(Clone)]
pub struct MenuWidget {
    width: f32,
    build: Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>,
}

impl MenuWidget {
    /// A row *width* px wide (the panel grows to it) holding what *build*
    /// returns.
    pub fn new(width: f32, build: impl Fn(&mut Window, &mut App) -> AnyElement + 'static) -> Self {
        Self { width, build: Rc::new(build) }
    }
}

impl MenuEntry {
    /// A widget row.
    pub fn widget(width: f32, build: impl Fn(&mut Window, &mut App) -> AnyElement + 'static) -> Self {
        Self::Widget(MenuWidget::new(width, build))
    }

    /// An item.
    pub fn action(action: MenuAction) -> Self {
        Self::Action(action)
    }

    /// A submenu.
    pub fn submenu(action: MenuAction, entries: Vec<MenuEntry>) -> Self {
        Self::Submenu(action, entries)
    }

    /// A section header.
    pub fn section(title: impl Into<SharedString>) -> Self {
        Self::Section(title.into())
    }

    /// Its item, unless it is a separator or a header.
    pub fn item(&self) -> Option<&MenuAction> {
        match self {
            Self::Action(a) | Self::Submenu(a, _) => Some(a),
            _ => None,
        }
    }

    /// Whether it can be current: an item, not greyed out.
    pub fn selectable(&self) -> bool {
        self.item().is_some_and(|a| !a.disabled)
    }

    /// Its submenu's entries.
    pub fn children(&self) -> Option<&[MenuEntry]> {
        match self {
            Self::Submenu(_, c) => Some(c),
            _ => None,
        }
    }
}

/// The entry at `path` (an index per level) under `root`.
pub fn entry_at<'a>(root: &'a [MenuEntry], path: &[usize]) -> Option<&'a MenuEntry> {
    let (last, outer) = path.split_last()?;
    let mut level = root;
    for &i in outer {
        level = level.get(i)?.children()?;
    }
    level.get(*last)
}

/// What [`MenuNav::activate`] did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Activation {
    /// Opened the current item's submenu.
    Opened,
    /// The current item is to run: its path.
    Run(Vec<usize>),
    /// Nothing (no current item, or a greyed-out one).
    Nothing,
}

/// Where a menu is: the submenus open, and the current item at each level.
///
/// Level 0 is the menu itself; level *n* + 1 is the submenu opened from
/// item `open[n]` of level *n*. The keyboard works on the last level.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MenuNav {
    open: Vec<usize>,
    current: Vec<Option<usize>>,
}

impl MenuNav {
    /// A menu with nothing current.
    pub fn new() -> Self {
        Self { open: Vec::new(), current: vec![None] }
    }

    /// A menu with its first selectable item current (as the keyboard opens
    /// one).
    pub fn first(root: &[MenuEntry]) -> Self {
        Self { open: Vec::new(), current: vec![first_selectable(root)] }
    }

    /// How many panels are open: 1 is the menu alone.
    pub fn depth(&self) -> usize {
        self.current.len().max(1)
    }

    /// The submenus open, by their item's index in the level above.
    pub fn opened(&self) -> &[usize] {
        &self.open
    }

    /// The current item at `level`.
    pub fn current(&self, level: usize) -> Option<usize> {
        self.current.get(level).copied().flatten()
    }

    /// The path of the last level's current item.
    pub fn current_path(&self) -> Option<Vec<usize>> {
        let last = self.current(self.depth() - 1)?;
        let mut path = self.open.clone();
        path.push(last);
        Some(path)
    }

    /// The entries of `level` (the menu's own at 0).
    pub fn level<'a>(&self, root: &'a [MenuEntry], level: usize) -> Option<&'a [MenuEntry]> {
        let mut entries = root;
        for &i in self.open.get(..level)? {
            entries = entries.get(i)?.children()?;
        }
        Some(entries)
    }

    /// Move the last level's current item by `by` selectable items,
    /// wrapping (Up: -1, Down: 1).
    pub fn step(&mut self, root: &[MenuEntry], by: isize) {
        let level = self.depth() - 1;
        let Some(entries) = self.level(root, level) else { return };
        let n = entries.len() as isize;
        if n == 0 {
            return;
        }
        let from = match self.current(level) {
            Some(i) => i as isize,
            // Nothing current: Down takes the first, Up the last.
            None if by > 0 => -1,
            None => n,
        };
        let next = (1..=n)
            .map(|k| (from + by.signum() * k).rem_euclid(n) as usize)
            .find(|&i| entries[i].selectable());
        if let Some(i) = next {
            self.set_current(level, Some(i));
        }
    }

    /// The pointer is on the item at `path`: it becomes current at its
    /// level. Submenus stay as they are until [`settle`](Self::settle).
    pub fn hover(&mut self, path: &[usize]) {
        let Some((&i, outer)) = path.split_last() else { return };
        // Only an item of an open panel can be under the pointer.
        if outer.len() < self.depth() && self.open.starts_with(outer) {
            self.set_current(outer.len(), Some(i));
        }
    }

    /// The pointer rested on the item at `path`: close the submenus below
    /// its level, and open its own if it has one. False if it is no longer
    /// current (the pointer moved on).
    pub fn settle(&mut self, root: &[MenuEntry], path: &[usize]) -> bool {
        let Some((&i, outer)) = path.split_last() else { return false };
        let level = outer.len();
        if !self.open.starts_with(outer) || self.current(level) != Some(i) {
            return false;
        }
        if self.open.get(level) == Some(&i) {
            return true;
        }
        self.truncate(level);
        if entry_at(root, path).is_some_and(|e| e.selectable() && e.children().is_some()) {
            self.open.push(i);
            self.current.push(None);
        }
        true
    }

    /// Make the item at `path` current and open its submenu, if it has one
    /// (a click on it).
    pub fn open(&mut self, root: &[MenuEntry], path: &[usize]) -> bool {
        let Some((&i, outer)) = path.split_last() else { return false };
        if outer.len() >= self.depth() || !self.open.starts_with(outer) {
            return false;
        }
        self.set_current(outer.len(), Some(i));
        self.settle(root, path)
    }

    /// Right (or Enter on a submenu item): open the current item's
    /// submenu and make its first item current.
    pub fn enter(&mut self, root: &[MenuEntry]) -> bool {
        let Some(path) = self.current_path() else { return false };
        let Some(children) = entry_at(root, &path).filter(|e| e.selectable()).and_then(|e| e.children()) else {
            return false;
        };
        let level = path.len() - 1;
        self.truncate(level);
        self.open.push(path[level]);
        self.current.push(first_selectable(children));
        true
    }

    /// Left (or Esc in a submenu): close the last submenu. False at the
    /// menu itself.
    pub fn back(&mut self) -> bool {
        if self.open.is_empty() {
            return false;
        }
        self.truncate(self.depth() - 2);
        true
    }

    /// Enter: open the current item's submenu, or say which item to run.
    pub fn activate(&mut self, root: &[MenuEntry]) -> Activation {
        let Some(path) = self.current_path() else { return Activation::Nothing };
        match entry_at(root, &path) {
            Some(e) if !e.selectable() => Activation::Nothing,
            Some(MenuEntry::Submenu(..)) => {
                self.enter(root);
                Activation::Opened
            }
            Some(MenuEntry::Action(_)) => Activation::Run(path),
            _ => Activation::Nothing,
        }
    }

    fn set_current(&mut self, level: usize, i: Option<usize>) {
        if let Some(c) = self.current.get_mut(level) {
            *c = i;
        }
    }

    /// Keep levels up to `level`, closing the submenus below it.
    fn truncate(&mut self, level: usize) {
        self.open.truncate(level);
        self.current.truncate(level + 1);
    }
}

fn first_selectable(entries: &[MenuEntry]) -> Option<usize> {
    entries.iter().position(MenuEntry::selectable)
}

/// A path handler: the item's index at each level.
type PathHandler = Rc<dyn Fn(&[usize], &mut Window, &mut App)>;

/// How far a submenu overlaps its parent's edge, px.
const OVERLAP: f32 = 1.0;
/// A panel's border and vertical padding: a submenu's first item lines up
/// with the item that opened it, px.
const PANEL_TOP: f32 = 4.0;
/// Room left between a submenu and the window's edges, px.
const MARGIN: f32 = 4.0;

/// A `QMenu` with fan-out submenus (see the module docs).
///
/// It draws the menu's own panel where its parent puts it; submenus are
/// drawn above everything, beside their item: right of the panel, or left
/// of it where the window has no room on the right.
#[derive(IntoElement)]
pub struct Menu {
    id: ElementId,
    entries: Rc<Vec<MenuEntry>>,
    nav: MenuNav,
    header: Option<AnyElement>,
    title: Option<SharedString>,
    empty: Option<SharedString>,
    min_width: f32,
    max_height: Option<f32>,
    on_hover: Option<PathHandler>,
    on_activate: Option<PathHandler>,
}

impl Menu {
    /// A menu of `entries`, at `nav`.
    pub fn new(id: impl Into<ElementId>, entries: impl Into<Rc<Vec<MenuEntry>>>, nav: MenuNav) -> Self {
        Self {
            id: id.into(),
            entries: entries.into(),
            nav,
            header: None,
            title: None,
            empty: None,
            min_width: 160.0,
            max_height: None,
            on_hover: None,
            on_activate: None,
        }
    }

    /// Something above the menu's own items (a search field).
    pub fn header(mut self, el: impl IntoElement) -> Self {
        self.header = Some(el.into_any_element());
        self
    }

    /// A muted title above everything.
    pub fn title(mut self, t: impl Into<SharedString>) -> Self {
        self.title = Some(t.into());
        self
    }

    /// Muted text shown when the menu has no entries.
    pub fn empty_text(mut self, t: impl Into<SharedString>) -> Self {
        self.empty = Some(t.into());
        self
    }

    /// The least width of each panel, px (160 by default).
    pub fn min_width(mut self, w: f32) -> Self {
        self.min_width = w;
        self
    }

    /// The tallest the menu's own list grows before it scrolls, px.
    /// Submenus scroll past the window's height.
    pub fn max_height(mut self, h: f32) -> Self {
        self.max_height = Some(h);
        self
    }

    /// Called with an item's path as the pointer enters it.
    pub fn on_hover(mut self, f: impl Fn(&[usize], &mut Window, &mut App) + 'static) -> Self {
        self.on_hover = Some(Rc::new(f));
        self
    }

    /// Called with an item's path when it is clicked (a submenu item
    /// included: open it with [`MenuNav::open`]).
    pub fn on_activate(mut self, f: impl Fn(&[usize], &mut Window, &mut App) + 'static) -> Self {
        self.on_activate = Some(Rc::new(f));
        self
    }

    /// Width of a panel of `entries`, px.
    fn panel_width(&self, entries: &[MenuEntry], window: &mut Window, style: &LaceStyle) -> f32 {
        let widest = entries
            .iter()
            .map(|e| match e {
                MenuEntry::Action(a) => row("menu-row", a, false).needed_width(window, style),
                MenuEntry::Submenu(a, _) => row("menu-row", a, true).needed_width(window, style),
                MenuEntry::Section(t) => menu_section_width(t, window, style),
                MenuEntry::Widget(w) => w.width,
                MenuEntry::Separator => 0.0,
            })
            .fold(0.0, f32::max);
        // The panel's 1 px border, both sides.
        (widest + 2.0).max(self.min_width).ceil()
    }

    fn panel(
        &self,
        level: usize,
        prefix: &[usize],
        entries: &[MenuEntry],
        mut top: Option<(Option<SharedString>, Option<AnyElement>)>,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let style = cx.lace().clone();
        let width = self.panel_width(entries, window, &style);
        let max_h = match (level, self.max_height) {
            (0, Some(h)) => h,
            _ => f32::from(window.viewport_size().height) - 2.0 * MARGIN,
        };
        // Keep the current item in view as the keyboard moves it (only
        // when it moves, so the wheel can scroll freely).
        let scroll = window.use_keyed_state(child_id(&level_id(&self.id, level), "scroll"), cx, |_, _| ScrollHandle::new());
        let scroll = scroll.read(cx).clone();
        let last = window.use_keyed_state(child_id(&level_id(&self.id, level), "shown"), cx, |_, _| None::<usize>);
        let current = self.nav.current(level);
        if *last.read(cx) != current {
            last.update(cx, |l, _| *l = current);
            if let Some(i) = current {
                scroll.scroll_to_item(i);
            }
        }

        let mut list = div()
            .id(child_id(&level_id(&self.id, level), "list"))
            .flex()
            .flex_col()
            .max_h(px(max_h))
            .overflow_y_scroll()
            .track_scroll(&scroll);
        if entries.is_empty() {
            if let Some(t) = self.empty.clone().filter(|_| level == 0) {
                let muted = hsla(crate::style::paint::muted(&style, State::NORMAL));
                list = list.child(div().px(px(8.)).py(px(3.)).text_color(muted).child(t));
            }
        }
        let open_here = self.nav.opened().get(level).copied().filter(|_| self.nav.opened().starts_with(prefix));
        for (i, entry) in entries.iter().enumerate() {
            let mut path = prefix.to_vec();
            path.push(i);
            let el: AnyElement = match entry {
                MenuEntry::Separator => menu_separator(&style).into_any_element(),
                MenuEntry::Section(t) => menu_section(&style, t.clone()).into_any_element(),
                MenuEntry::Widget(w) => div()
                    .id(path_id(&self.id, &path))
                    .w_full()
                    // The pointer over it leaves the item rows: a submenu
                    // open beside one closes as it would over any other row.
                    .on_hover({
                        let (hover, hp) = (self.on_hover.clone(), path.clone());
                        move |entered, window, cx| {
                            if *entered {
                                if let Some(f) = &hover {
                                    f(&hp, window, cx)
                                }
                            }
                        }
                    })
                    .child((w.build)(window, cx))
                    .into_any_element(),
                MenuEntry::Action(a) | MenuEntry::Submenu(a, _) => {
                    let is_sub = matches!(entry, MenuEntry::Submenu(..));
                    let on = self.nav.current(level) == Some(i) || open_here == Some(i);
                    let id = path_id(&self.id, &path);
                    let mut item = row(id, a, is_sub)
                        .state(if on { State::hover() } else { State::NORMAL })
                        .disabled(a.disabled);
                    let (hover, click) = (self.on_hover.clone(), self.on_activate.clone());
                    let (hp, cp) = (path.clone(), path.clone());
                    item = item.on_hover(move |entered, window, cx| {
                        if entered {
                            if let Some(f) = &hover {
                                f(&hp, window, cx)
                            }
                        }
                    });
                    if let Some(f) = click {
                        item = item.on_click(move |window, cx| f(&cp, window, cx));
                    }
                    let mut wrap = div().relative().child(item);
                    if open_here == Some(i) {
                        if let Some(children) = entry.children() {
                            let sub = self.panel(level + 1, &path, children, None, window, cx);
                            wrap = wrap.child(
                                deferred(Fan { row_width: px(width - 2.0), children: vec![sub] })
                                    .with_priority(level + 1),
                            );
                        }
                    }
                    wrap.into_any_element()
                }
            };
            list = list.child(el);
        }

        let (title, header) = top.take().unwrap_or((None, None));
        let muted = hsla(crate::style::paint::muted(&style, State::NORMAL));
        popup_panel(&style)
            .id(level_id(&self.id, level))
            .w(px(width))
            .occlude()
            // Its own clicks and wheel are not what lies under it.
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
            .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
            .when_some(title, |d, t| d.child(div().px(px(8.)).pb(px(2.)).text_color(muted).child(t)))
            .when_some(header, |d, h| d.child(div().mx(px(MENU_HEADER_INSET)).mb(px(3.)).child(h)))
            .child(list)
            .into_any_element()
    }
}

/// A header's inset from the panel's sides, px.
const MENU_HEADER_INSET: f32 = 4.0;

fn row(id: impl Into<ElementId>, a: &MenuAction, submenu: bool) -> MenuItem {
    let mut item = MenuItem::new(id, a.label.clone()).icon_rc(a.icon.clone());
    if let Some(s) = &a.shortcut {
        item = item.shortcut(s.clone());
    }
    item = match a.check {
        Check::None => item,
        Check::Check(on) => item.checked(on),
        Check::Radio(on) => item.radio(on),
    };
    if let Some(s) = &a.selector {
        item = item.selector(s.clone());
    }
    if submenu {
        item = item.submenu();
    }
    item
}

fn level_id(id: &ElementId, level: usize) -> ElementId {
    ElementId::NamedChild(std::sync::Arc::new(id.clone()), SharedString::from(format!("level{level}")))
}

fn path_id(id: &ElementId, path: &[usize]) -> ElementId {
    let key = path.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(".");
    ElementId::NamedChild(std::sync::Arc::new(id.clone()), SharedString::from(format!("item{key}")))
}

impl RenderOnce for Menu {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let entries = self.entries.clone();
        let mut me = self;
        let top = Some((me.title.clone(), me.header.take()));
        me.panel(0, &[], &entries, top, window, cx)
    }
}

/// Places a submenu beside the row it sits in: right of the parent panel,
/// or left of it when the window has no room on the right; moved up as far
/// as it must to stay inside the window.
struct Fan {
    /// The parent panel's inner width: from the row's left to its right edge.
    row_width: Pixels,
    children: Vec<AnyElement>,
}

impl IntoElement for Fan {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for Fan {
    type RequestLayoutState = Vec<LayoutId>;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let ids: Vec<LayoutId> = self.children.iter_mut().map(|c| c.request_layout(window, cx)).collect();
        let style = Style { position: Position::Absolute, display: Display::Flex, ..Style::default() };
        (window.request_layout(style, ids.iter().copied(), cx), ids)
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        ids: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let Some(size) = ids.iter().map(|id| window.layout_bounds(*id)).reduce(|a, b| a.union(&b)).map(|b| b.size)
        else {
            return;
        };
        let at = fan_origin(
            (f32::from(bounds.origin.x), f32::from(bounds.origin.y)),
            f32::from(self.row_width),
            (f32::from(size.width), f32::from(size.height)),
            (f32::from(window.viewport_size().width), f32::from(window.viewport_size().height)),
        );
        let offset = point(px(at.0 - f32::from(bounds.origin.x)), px(at.1 - f32::from(bounds.origin.y)));
        window.with_element_offset(point(offset.x.round(), offset.y.round()), |window| {
            for child in &mut self.children {
                child.prepaint(window, cx);
            }
        });
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _ids: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        for child in &mut self.children {
            child.paint(window, cx);
        }
    }
}

/// Where a submenu of `size` goes for a row at `row` (its top-left)
/// `row_width` wide, in a window of `window` size.
fn fan_origin(row: (f32, f32), row_width: f32, size: (f32, f32), window: (f32, f32)) -> (f32, f32) {
    let right = row.0 + row_width - OVERLAP;
    let left = row.0 - size.0 + OVERLAP;
    let x = if right + size.0 <= window.0 - MARGIN || left < MARGIN {
        right.min(window.0 - MARGIN - size.0).max(MARGIN.min(right))
    } else {
        left
    };
    let y = (row.1 - PANEL_TOP).min(window.1 - MARGIN - size.1).max(MARGIN);
    (x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(label: &str) -> MenuEntry {
        MenuEntry::action(MenuAction::new(label.to_string()))
    }

    fn off(label: &str) -> MenuEntry {
        MenuEntry::action(MenuAction::new(label.to_string()).disabled(true))
    }

    /// Undo, —, (Paste), View ▸ [Dots, Lines, Theme ▸ [Dark, Light]], Quit
    fn menu() -> Vec<MenuEntry> {
        vec![
            item("Undo"),
            MenuEntry::Separator,
            off("Paste"),
            MenuEntry::submenu(
                MenuAction::new("View"),
                vec![
                    MenuEntry::section("Grid"),
                    item("Dots"),
                    item("Lines"),
                    MenuEntry::submenu(MenuAction::new("Theme"), vec![item("Dark"), item("Light")]),
                ],
            ),
            item("Quit"),
        ]
    }

    #[test]
    fn the_keyboard_passes_over_a_widget_row() {
        let m = vec![item("A"), MenuEntry::widget(200.0, |_, _| div().into_any_element()), item("B")];
        assert!(!m[1].selectable());
        let mut nav = MenuNav::first(&m);
        nav.step(&m, 1);
        assert_eq!(nav.current(0), Some(2));
        // The pointer resting on it closes nothing it should not, and runs nothing.
        nav.hover(&[1]);
        assert!(nav.settle(&m, &[1]));
        assert!(matches!(nav.activate(&m), Activation::Nothing));
    }

    #[test]
    fn up_and_down_skip_separators_headers_and_greyed_items_and_wrap() {
        let m = menu();
        let mut nav = MenuNav::first(&m);
        assert_eq!(nav.current(0), Some(0));
        nav.step(&m, 1);
        assert_eq!(nav.current(0), Some(3), "past the line and Paste");
        nav.step(&m, 1);
        nav.step(&m, 1);
        assert_eq!(nav.current(0), Some(0), "wraps");
        nav.step(&m, -1);
        assert_eq!(nav.current(0), Some(4));
        let mut fresh = MenuNav::new();
        fresh.step(&m, -1);
        assert_eq!(fresh.current(0), Some(4), "Up with nothing current takes the last");
    }

    #[test]
    fn enter_opens_a_submenu_at_its_first_item_and_left_closes_it() {
        let m = menu();
        let mut nav = MenuNav::first(&m);
        nav.step(&m, 1);
        assert_eq!(nav.activate(&m), Activation::Opened);
        assert_eq!(nav.depth(), 2);
        assert_eq!(nav.current(1), Some(1), "past the section header");
        nav.step(&m, 1);
        nav.step(&m, 1);
        assert!(nav.enter(&m), "Right into Theme");
        assert_eq!(nav.current_path(), Some(vec![3, 3, 0]));
        assert_eq!(nav.activate(&m), Activation::Run(vec![3, 3, 0]));
        assert!(nav.back() && nav.back());
        assert_eq!(nav.depth(), 1);
        assert!(!nav.back(), "the menu itself stays");
        assert_eq!(nav.current(0), Some(3), "its item still current");
    }

    #[test]
    fn hovering_moves_the_current_item_and_settling_opens_or_closes() {
        let m = menu();
        let mut nav = MenuNav::new();
        nav.hover(&[3]);
        assert_eq!((nav.current(0), nav.depth()), (Some(3), 1), "not open yet");
        assert!(nav.settle(&m, &[3]));
        assert_eq!(nav.depth(), 2);
        assert_eq!(nav.current(1), None, "opened by the pointer: nothing current inside");
        // Into the submenu, then back over its own item: it stays open.
        nav.hover(&[3, 2]);
        nav.hover(&[3]);
        assert!(nav.settle(&m, &[3]));
        assert_eq!(nav.depth(), 2);
        // Onto Quit, but away again before it settles: nothing changes.
        nav.hover(&[4]);
        nav.hover(&[3]);
        assert!(!nav.settle(&m, &[4]));
        assert_eq!(nav.depth(), 2);
        nav.hover(&[4]);
        assert!(nav.settle(&m, &[4]));
        assert_eq!(nav.depth(), 1, "Quit closes the submenu");
        nav.hover(&[3, 1]);
        assert_eq!(nav.current(0), Some(4), "a closed panel's item cannot be hovered");
    }

    #[test]
    fn a_click_opens_a_submenu_at_once_and_a_greyed_item_does_nothing() {
        let m = menu();
        let mut nav = MenuNav::new();
        assert!(nav.open(&m, &[3]));
        assert!(nav.open(&m, &[3, 3]));
        assert_eq!(nav.opened(), &[3, 3]);
        assert_eq!(nav.depth(), 3);
        assert!(nav.open(&m, &[0]), "a plain item closes them");
        assert_eq!(nav.depth(), 1);
        nav.hover(&[2]);
        assert_eq!(nav.activate(&m), Activation::Nothing);
        assert!(entry_at(&m, &[3, 3, 1]).is_some_and(|e| e.item().unwrap().label == "Light"));
    }

    #[test]
    fn a_submenu_opens_right_of_its_panel_or_left_where_there_is_no_room() {
        // A 200 px wide panel at x 100; a 150 x 100 submenu.
        assert_eq!(fan_origin((100.0, 50.0), 200.0, (150.0, 100.0), (800.0, 600.0)), (299.0, 46.0));
        // At the right edge: to the left of the panel.
        assert_eq!(fan_origin((600.0, 50.0), 200.0, (150.0, 100.0), (800.0, 600.0)), (451.0, 46.0));
        // No room either side: kept inside the window.
        assert_eq!(fan_origin((10.0, 50.0), 200.0, (700.0, 100.0), (800.0, 600.0)), (96.0, 46.0));
        // Near the bottom: moved up.
        assert_eq!(fan_origin((100.0, 560.0), 200.0, (150.0, 100.0), (800.0, 600.0)), (299.0, 496.0));
    }
}
