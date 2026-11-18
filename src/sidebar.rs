//! Lace's auto-hide sidebars: a rail of tabs along the window's left, right
//! and bottom edges, each standing for a panel pinned out of the dock. A tab
//! slides its panel out over the dock: hovering it peeks (and the panel hides
//! again once the pointer leaves), clicking it opens and focuses the panel
//! (it stays until a click elsewhere, Escape, or another click on the tab).
//!
//! gpui-kit's dock has no sidebars, so this is built beside it from public
//! API: [`Sidebars`] wraps the [`DockArea`] in its own view, pinning removes
//! the panel from the dock while keeping its view alive, and unpinning adds
//! it back to the group it came from. The title bar's pin button reaches it
//! through [`PinTarget`], which [`Sidebars::new`] installs on the
//! [`GeometryDockSkin`].
//!
//! `DockArea::remove_panel` is generic over the panel type, so a panel can
//! only be pinned once the host has named it with [`Sidebars::pinnable`].
//!
//! Lace's sidebar tab paints its label turned a quarter; gpui lays text out
//! horizontally only, so the label is drawn as SVG text (gpui's SVG renderer
//! shapes text with the system fonts) and tinted like an icon.

use std::{cell::Cell, collections::HashMap, rc::Rc, sync::Arc, time::Duration};

use gpui_kit::base::dock::{
    AnyDrag, DockArea, DockEvent, DockPlacement, DropTarget, InsertTarget, Panel as BasePanel,
    PanelId, PanelView,
};
use gpui_kit::component::dock::PanelHandle;
use gpui_kit::component::Icon;
use gpui_kit::gpui::{
    canvas, div, font, prelude::FluentBuilder as _, px, svg, AnyElement, App, AppContext as _,
    Bounds, Context, Entity, FocusHandle, FontWeight, Global, Hsla, InteractiveElement as _, IntoElement,
    KeyDownEvent, linear_color_stop, linear_gradient, MouseButton, MouseMoveEvent, MouseUpEvent, ParentElement as _, Pixels, Point,
    Render, SharedString, Size, StatefulInteractiveElement as _, Styled as _, Subscription, Task,
    TextRun, WeakEntity, Window,
};

use crate::floating::FloatingPane;

use crate::dock_icons::DockIcon;
use crate::edge_tabs::{chrome_button, PinTarget, TITLE_BUTTON, TITLE_BUTTON_GAP};
use crate::engine::SideTabFlat;
use crate::geometry::GeometryDockSkin;

/// Pointer rest on a tab before its panel peeks out (Lace's hover delay).
const SHOW_DELAY: Duration = Duration::from_millis(150);
/// Grace after the pointer leaves before a peeked panel hides.
const HIDE_DELAY: Duration = Duration::from_millis(400);
/// The smallest panel Lace lets a sidebar overlay shrink to.
const MIN_WIDTH: Pixels = px(200.);
const MIN_HEIGHT: Pixels = px(150.);
/// What a resize always leaves of the dock behind the overlay.
const MIN_CENTER_GAP: Pixels = px(50.);
/// The resize grip on the overlay's inner edge.
const GRIP: Pixels = px(6.);
/// Space between a rail and its overlay, and around a maximized one.
const INSET: Pixels = px(2.);
/// Lace's sidebar and side-panel title font: 10pt.
const TEXT_SIZE: Pixels = px(13.33);

/// Which window edge a sidebar runs along.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SideEdge {
    Left,
    Right,
    Bottom,
}

impl SideEdge {
    fn vertical(self) -> bool {
        self != SideEdge::Bottom
    }
}

/// The sidebar geometry and colours of the installed theme. [`crate::skin`]
/// installs it with every theme; the default is a neutral dark rail.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SidebarStyle {
    /// Rail thickness. Lace's `SIDEBAR.width`.
    pub rail: Pixels,
    pub rail_bg: Hsla,
    pub tab_bg_active: Hsla,
    /// A hovered tab fills with a left-to-right gradient from `tab_bg_hover`
    /// to this, as Lace's `tab_bg_hover_start`/`_end`.
    pub tab_bg_hover: Hsla,
    pub tab_bg_hover_end: Hsla,
    pub tab_text: Hsla,
    pub tab_text_active: Hsla,
    /// Outline width; zero for the classic flat rail.
    pub outline: Pixels,
    pub outline_normal: Hsla,
    pub outline_active: Hsla,
    pub outline_hover: Hsla,
    pub flat: SideTabFlat,
    pub tab_radius: Pixels,
    pub indicator: Hsla,
    pub indicator_width: Pixels,
    pub panel_bg: Hsla,
    pub panel_border: Hsla,
    pub panel_focus: Hsla,
    pub panel_border_width: Pixels,
    pub panel_radius: Pixels,
    pub title_text: Hsla,
    /// The title bar's height. Lace's `SIDEPANEL.height`.
    pub title_height: Pixels,
}

impl Default for SidebarStyle {
    fn default() -> Self {
        let gray = |l: f32| gpui_kit::gpui::hsla(0., 0., l, 1.);
        Self {
            rail: px(30.),
            rail_bg: gray(0.1),
            tab_bg_active: gray(0.18),
            tab_bg_hover: gray(0.22),
            tab_bg_hover_end: gray(0.22),
            tab_text: gray(0.75),
            tab_text_active: gray(1.),
            outline: px(0.),
            outline_normal: gpui_kit::gpui::transparent_black(),
            outline_active: gpui_kit::gpui::transparent_black(),
            outline_hover: gpui_kit::gpui::transparent_black(),
            flat: SideTabFlat::All,
            tab_radius: px(4.),
            indicator: gpui_kit::gpui::hsla(0.58, 0.8, 0.55, 1.),
            indicator_width: px(3.),
            panel_bg: gray(0.16),
            panel_border: gray(0.28),
            panel_focus: gpui_kit::gpui::hsla(0.58, 0.8, 0.55, 1.),
            panel_border_width: px(1.),
            panel_radius: px(4.),
            title_text: gray(0.95),
            title_height: px(30.),
        }
    }
}

impl Global for SidebarStyle {}

impl SidebarStyle {
    /// The style the running app installed, or the default.
    pub fn current(cx: &App) -> Self {
        cx.try_global::<Self>().copied().unwrap_or_default()
    }

    /// Install `style` for every sidebar in the app.
    pub fn install(style: Self, cx: &mut App) {
        cx.set_global(style);
    }

    /// The outline a tab paints in one state, or `None` when that state has
    /// none (a transparent colour means none, as in Lace).
    fn outline_for(&self, active: bool, hovered: bool) -> Option<Hsla> {
        if self.outline <= px(0.) {
            return None;
        }
        let color = if active {
            self.outline_active
        } else if hovered {
            self.outline_hover
        } else {
            self.outline_normal
        };
        (color.a > 0.).then_some(color)
    }
}

/// How a host-named panel leaves the dock: `DockArea::remove_panel` needs
/// the concrete entity, so it is captured when the panel is named.
type Remover = Rc<dyn Fn(&mut DockArea, &mut Window, &mut Context<DockArea>)>;

struct Pinnable {
    remove: Remover,
    icon: Option<Icon>,
}

struct Pinned {
    id: PanelId,
    view: Arc<dyn PanelView>,
    edge: SideEdge,
    /// Where it came from, to go back to on unpin.
    placement: DockPlacement,
    node: gpui_kit::base::dock::NodeId,
}

/// A sidebar tab in flight. The dock hands it back in
/// [`DockEvent::DragDrop`] when it lands on a group.
struct SidebarDrag {
    panel: PanelId,
}

/// The dock with Lace's auto-hide sidebars around it. Render this in place
/// of the [`DockArea`].
pub struct Sidebars {
    area: Entity<DockArea>,
    pinnable: HashMap<PanelId, Pinnable>,
    pinned: Vec<Pinned>,
    open: Option<PanelId>,
    /// Opened by hovering: hides again once the pointer leaves.
    peek: bool,
    maximized: bool,
    extents: HashMap<PanelId, Pixels>,
    over_rail: bool,
    over_panel: bool,
    pending: Option<Task<()>>,
    resizing: Rc<Cell<bool>>,
    center: Rc<Cell<Bounds<Pixels>>>,
    /// Drops of sidebar tabs on the dock; taken on the first render, which
    /// has the window the docking needs.
    dock_events: Option<Subscription>,
    focus_mode: SidebarFocus,
    /// Focus from before the overlay opened, for `TakeAndRestore`.
    restore: Option<FocusHandle>,
    /// The panel that just closed while `restore` waits for the next render,
    /// which has the window to move focus.
    closed_focus: Option<FocusHandle>,
}

/// How a sidebar panel takes keyboard focus when it slides out (Lace's
/// `SideBarFocusBehavior`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SidebarFocus {
    /// Focus the panel, and give focus back where it was once it hides.
    #[default]
    TakeAndRestore,
    /// Focus the panel and leave focus wherever it ends up.
    TakeOnly,
    /// Leave focus alone; the panel gets it only when clicked.
    NoTransfer,
}

impl SidebarFocus {
    pub const ALL: [SidebarFocus; 3] = [Self::TakeAndRestore, Self::TakeOnly, Self::NoTransfer];

    pub fn label(self) -> &'static str {
        match self {
            Self::TakeAndRestore => "Take Focus & Restore",
            Self::TakeOnly => "Take Focus Only",
            Self::NoTransfer => "No Focus Transfer",
        }
    }
}

impl Sidebars {
    /// Wrap `area` and install the pin button on `skin`.
    pub fn new(area: Entity<DockArea>, skin: &GeometryDockSkin, cx: &mut Context<Self>) -> Self {
        skin.set_pin_target(Some(Rc::new(SidebarPin(cx.weak_entity()))), cx);
        Self {
            area,
            pinnable: HashMap::new(),
            pinned: Vec::new(),
            open: None,
            peek: false,
            maximized: false,
            extents: HashMap::new(),
            over_rail: false,
            over_panel: false,
            pending: None,
            resizing: Rc::new(Cell::new(false)),
            center: Rc::new(Cell::new(Bounds::default())),
            dock_events: None,
            focus_mode: SidebarFocus::default(),
            restore: None,
            closed_focus: None,
        }
    }

    fn on_dock_event(
        &mut self,
        _: &Entity<DockArea>,
        event: &DockEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let DockEvent::DragDrop { item, target } = event else {
            return;
        };
        if let Some(drag) = item.value().downcast_ref::<SidebarDrag>() {
            self.dock_at(drag.panel, *target, window, cx);
        }
    }

    /// Put a pinned panel into the dock where a drag of its sidebar tab
    /// landed: into the target group, or split off one of its sides.
    pub fn dock_at(
        &mut self,
        panel: PanelId,
        target: DropTarget,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(ix) = self.pinned.iter().position(|p| p.id == panel) else {
            return;
        };
        let pinned = self.pinned.remove(ix);
        if self.open == Some(panel) {
            self.close_overlay(cx);
        }
        let node = target.node();
        self.area.update(cx, |area, cx| {
            let placement = [
                DockPlacement::Center,
                DockPlacement::Left,
                DockPlacement::Right,
                DockPlacement::Bottom,
            ]
            .into_iter()
            .find(|placement| {
                area.layout(*placement)
                    .is_some_and(|tree| tree.node_ids().contains(&node))
            })
            .unwrap_or(pinned.placement);
            area.add_panel_view(pinned.view.clone(), placement, None, window, cx);
            if placement != DockPlacement::Center && !area.is_dock_open(placement) {
                area.toggle_dock(placement, window, cx);
            }
            let insert = match target.placement() {
                None => InsertTarget::Tabs {
                    node,
                    ix: None,
                    activate: true,
                },
                Some(side) => InsertTarget::Split {
                    node,
                    placement: side,
                    size: None,
                },
            };
            area.move_panel(panel, insert, window, cx);
            area.select_panel(panel, window, cx);
        });
        cx.notify();
    }

    /// The size a pinned panel floats at while its tab is dragged: its
    /// overlay's.
    fn float_size(&self, pinned: &Pinned) -> Size<Pixels> {
        let center = self.center.get().size;
        let extent = self.extent(pinned);
        if pinned.edge.vertical() {
            Size {
                width: extent,
                height: (center.height - INSET * 2.).max(MIN_HEIGHT),
            }
        } else {
            Size {
                width: (center.width - INSET * 2.).max(MIN_WIDTH),
                height: extent,
            }
        }
    }

    /// Allow `panel` to be pinned, with `icon` on its sidebar tab.
    pub fn pinnable<P: BasePanel>(&mut self, panel: &Entity<P>, icon: Option<Icon>) {
        let weak = panel.downgrade();
        let remove: Remover = Rc::new(move |area, window, cx| {
            if let Some(panel) = weak.upgrade() {
                area.remove_panel(panel, window, cx);
            }
        });
        self.pinnable
            .insert(PanelId::from(panel.entity_id()), Pinnable { remove, icon });
    }

    /// Whether `panel` sits in a sidebar now.
    pub fn is_pinned(&self, panel: PanelId) -> bool {
        self.pinned.iter().any(|p| p.id == panel)
    }

    /// Move `panel` from the dock into a sidebar: the one matching its dock,
    /// or for the centre the one nearest `at` (window coordinates).
    pub fn pin(
        &mut self,
        panel: PanelId,
        at: Option<Point<Pixels>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.is_pinned(panel) {
            return;
        }
        let Some(remove) = self.pinnable.get(&panel).map(|p| p.remove.clone()) else {
            return;
        };
        let area = self.area.read(cx);
        let Some(view) = area.panel(panel).cloned() else {
            return;
        };
        let home = [
            DockPlacement::Center,
            DockPlacement::Left,
            DockPlacement::Right,
            DockPlacement::Bottom,
        ]
        .into_iter()
        .find_map(|placement| {
            area.layout(placement)
                .and_then(|tree| tree.find_panel_node(panel))
                .map(|node| (placement, node))
        });
        let Some((placement, node)) = home else {
            return;
        };
        let edge = match placement {
            DockPlacement::Left => SideEdge::Left,
            DockPlacement::Right => SideEdge::Right,
            DockPlacement::Bottom => SideEdge::Bottom,
            DockPlacement::Center => nearest_edge(at, window.viewport_size()),
        };
        self.area.update(cx, |area, cx| {
            remove(area, window, cx);
            // A dock left with nothing to show folds away, as in Lace,
            // rather than standing as an empty card.
            if placement != DockPlacement::Center
                && area.is_empty(placement, cx)
                && area.is_dock_open(placement)
                && area.is_dock_collapsible(placement)
            {
                area.toggle_dock(placement, window, cx);
            }
        });
        self.pinned.push(Pinned {
            id: panel,
            view,
            edge,
            placement,
            node,
        });
        cx.notify();
    }

    /// Put a pinned panel back into the dock, into the group it left when
    /// that group still exists.
    pub fn unpin(&mut self, panel: PanelId, window: &mut Window, cx: &mut Context<Self>) {
        let Some(ix) = self.pinned.iter().position(|p| p.id == panel) else {
            return;
        };
        let pinned = self.pinned.remove(ix);
        if self.open == Some(panel) {
            self.close_overlay(cx);
        }
        self.area.update(cx, |area, cx| {
            area.add_panel_view(pinned.view.clone(), pinned.placement, None, window, cx);
            if pinned.placement != DockPlacement::Center && !area.is_dock_open(pinned.placement) {
                area.toggle_dock(pinned.placement, window, cx);
            }
            let tree = area.layout(pinned.placement);
            let home_alive = tree.is_some_and(|tree| tree.node_ids().contains(&pinned.node));
            let landed = tree.and_then(|tree| tree.find_panel_node(panel));
            if home_alive && landed != Some(pinned.node) {
                let target = InsertTarget::Tabs {
                    node: pinned.node,
                    ix: None,
                    activate: true,
                };
                area.move_panel(panel, target, window, cx);
            }
            area.select_panel(panel, window, cx);
        });
        cx.notify();
    }

    /// Slide `panel`'s overlay out. A peek hides again on its own.
    pub fn open(&mut self, panel: PanelId, peek: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(pinned) = self.pinned.iter().find(|p| p.id == panel) else {
            return;
        };
        if self.open != Some(panel) {
            self.maximized = false;
        }
        let handle = pinned.view.focus_handle(cx);
        self.open = Some(panel);
        self.peek = peek;
        self.pending = None;
        self.closed_focus = None;
        // As in Lace, a peek takes focus too: the panel is meant to be used
        // straight away, and focus does not keep it open.
        if self.focus_mode != SidebarFocus::NoTransfer {
            if self.focus_mode == SidebarFocus::TakeAndRestore && self.restore.is_none() {
                let inside = self
                    .pinned
                    .iter()
                    .any(|p| p.view.focus_handle(cx).contains_focused(window, cx));
                self.restore = window.focused(cx).filter(|_| !inside);            }
            window.focus(&handle, cx);
        }
        cx.notify();
    }

    /// How opening a sidebar panel moves keyboard focus.
    pub fn focus_behavior(&self) -> SidebarFocus {
        self.focus_mode
    }

    pub fn set_focus_behavior(&mut self, mode: SidebarFocus, cx: &mut Context<Self>) {
        self.focus_mode = mode;
        if mode != SidebarFocus::TakeAndRestore {
            self.restore = None;
        }
        cx.notify();
    }

    /// Give focus back to where it was before the overlay opened, once the
    /// overlay is gone, unless the user has put it somewhere else since.
    fn restore_focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(closed) = self.closed_focus.take() else {
            return;
        };
        let Some(previous) = self.restore.take() else {
            return;
        };
        let elsewhere = window.focused(cx).is_some()
            && !closed.is_focused(window)
            && !closed.contains_focused(window, cx);
        if !elsewhere {
            window.focus(&previous, cx);
        }
    }

    /// Slide the open overlay back in.
    pub fn close_overlay(&mut self, cx: &mut Context<Self>) {
        if self.restore.is_some() {
            self.closed_focus = self.open_pinned().map(|p| p.view.focus_handle(cx));
            if self.closed_focus.is_none() {
                self.restore = None;
            }
        }
        self.open = None;
        self.peek = false;
        self.maximized = false;
        self.over_panel = false;
        self.pending = None;
        cx.notify();
    }

    /// Close a pinned panel for good (the title bar's ✕).
    fn close_pinned(&mut self, panel: PanelId, cx: &mut Context<Self>) {
        if self.open == Some(panel) {
            self.close_overlay(cx);
        }
        self.pinned.retain(|p| p.id != panel);
        cx.notify();
    }

    fn schedule(
        &mut self,
        delay: Duration,
        window: &mut Window,
        cx: &mut Context<Self>,
        then: impl FnOnce(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) {
        self.pending = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(delay).await;
            this.update_in(cx, |this, window, cx| then(this, window, cx)).ok();
        }));
    }

    /// Hide a peeked overlay after the grace delay, unless the pointer came
    /// back to it by then. Focus does not count: a peek takes focus itself.
    fn schedule_hide(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.open.is_none() || !self.peek {
            return;
        }
        self.schedule(HIDE_DELAY, window, cx, |this, _, cx| {
            if this.peek && !this.over_panel && !this.over_rail {
                this.close_overlay(cx);
            }
        });
    }

    fn open_pinned(&self) -> Option<&Pinned> {
        let open = self.open?;
        self.pinned.iter().find(|p| p.id == open)
    }

    fn tab_hovered(&mut self, panel: PanelId, hovered: bool, window: &mut Window, cx: &mut Context<Self>) {
        if hovered {
            // A panel opened by click stays put; hovering other tabs only
            // switches between peeks.
            if self.open == Some(panel) || (self.open.is_some() && !self.peek) {
                self.pending = None;
                return;
            }
            self.schedule(SHOW_DELAY, window, cx, move |this, window, cx| {
                if this.over_rail {
                    this.open(panel, true, window, cx);
                }
            });
        } else {
            self.pending = None;
            self.schedule_hide(window, cx);
        }
    }

    fn tab_clicked(&mut self, panel: PanelId, window: &mut Window, cx: &mut Context<Self>) {
        if self.open == Some(panel) && !self.peek {
            self.close_overlay(cx);
        } else {
            self.open(panel, false, window, cx);
        }
    }

    fn extent(&self, pinned: &Pinned) -> Pixels {
        let center = self.center.get().size;
        let (default, min, room) = if pinned.edge.vertical() {
            (px(300.), MIN_WIDTH, center.width)
        } else {
            (px(220.), MIN_HEIGHT, center.height)
        };
        let room = room - INSET * 2.;
        if self.maximized {
            return room.max(min);
        }
        let wanted = self.extents.get(&pinned.id).copied().unwrap_or(default);
        wanted.min(room - MIN_CENTER_GAP).max(min)
    }

    fn resize_to(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) {
        let Some(pinned) = self.open_pinned() else {
            return;
        };
        let center = self.center.get();
        let extent = match pinned.edge {
            SideEdge::Left => position.x - center.left() - INSET,
            SideEdge::Right => center.right() - position.x - INSET,
            SideEdge::Bottom => center.bottom() - position.y - INSET,
        };
        let id = pinned.id;
        self.maximized = false;
        self.extents.insert(id, extent);
        cx.notify();
    }

    fn rail(&self, edge: SideEdge, style: SidebarStyle, window: &mut Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let tabs: Vec<AnyElement> = self
            .pinned
            .iter()
            .filter(|p| p.edge == edge)
            .map(|p| self.tab(p, style, window, cx))
            .collect();
        if tabs.is_empty() {
            return None;
        }
        let rail = div()
            .id(SharedString::from(format!("lace-sidebar-rail:{edge:?}")))
            .flex()
            .flex_none()
            .gap(px(2.))
            .bg(style.rail_bg)
            .on_hover(cx.listener(|this, hovered: &bool, window, cx| {
                this.over_rail = *hovered;
                if !*hovered {
                    this.schedule_hide(window, cx);
                }
            }));
        let rail = if edge.vertical() {
            rail.w(style.rail).h_full().flex_col().items_center().py(INSET)
        } else {
            rail.h(style.rail).w_full().flex_row().items_center().px(INSET)
        };
        Some(rail.children(tabs).into_any_element())
    }

    fn tab(&self, pinned: &Pinned, style: SidebarStyle, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let id = pinned.id;
        let edge = pinned.edge;
        let active = self.open == Some(id);
        let text_color = if active { style.tab_text_active } else { style.tab_text };
        let name = panel_name(&pinned.view, cx);
        let icon = self.pinnable.get(&id).and_then(|p| p.icon.clone());
        let thickness = style.rail - INSET;

        let text = window.text_style();
        let font_size = TEXT_SIZE;
        let family = svg_family(&text.font_family);
        let width = window
            .text_system()
            .shape_line(
                name.clone(),
                font_size,
                &[TextRun {
                    len: name.len(),
                    font: font(family.clone()),
                    color: text_color,
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                }],
                None,
            )
            .width
            .ceil();

        let icon_el = icon.map(|icon| icon.size(px(16.)).flex_none().text_color(text_color));
        let label: AnyElement = if edge.vertical() {
            let length = width + px(2.);
            svg()
                .data(&vertical_label(&name, &family, f32::from(font_size), f32::from(thickness), f32::from(length)))
                .w(thickness)
                .h(length)
                .flex_none()
                .text_color(text_color)
                .into_any_element()
        } else {
            div()
                .text_color(text_color)
                .whitespace_nowrap()
                .text_size(TEXT_SIZE)
                .child(name.clone())
                .into_any_element()
        };

        let outline = style.outline_for(active, false);
        let hover_outline = style.outline_for(active, true);
        let mut tab = div()
            .id(SharedString::from(format!("lace-sidebar-tab:{}", id.as_u64())))
            .relative()
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .gap(px(8.))
            .when(active, |this| this.bg(style.tab_bg_active))
            .when(!active, |this| {
                this.hover(|s| {
                    s.bg(linear_gradient(
                        90.,
                        linear_color_stop(style.tab_bg_hover, 0.),
                        linear_color_stop(style.tab_bg_hover_end, 1.),
                    ))
                })
            })
            .when(style.outline > px(0.), |this| {
                this.border(style.outline)
                    .border_color(outline.unwrap_or(gpui_kit::gpui::transparent_black()))
                    .when_some(hover_outline.filter(|_| !active), |this, color| {
                        this.hover(move |s| s.border_color(color))
                    })
            });
        tab = if edge.vertical() {
            tab.w(thickness).flex_col().py(px(11.))
        } else {
            tab.h(thickness).flex_row().px(px(11.))
        };
        tab = round_tab(tab, edge, style);

        // Lace clips the indicator to the tab and strokes the outline over
        // it, so an outline at least as wide hides it; only the rounded
        // outline shows then.
        let covered = outline.is_some() && style.indicator_width <= style.outline;
        tab.when(active && !covered, |this| this.child(indicator(edge, style)))
            .children(icon_el)
            .child(label)
            .on_hover(cx.listener(move |this, hovered: &bool, window, cx| {
                this.tab_hovered(id, *hovered, window, cx)
            }))
            .on_click(cx.listener(move |this, _, window, cx| this.tab_clicked(id, window, cx)))
            // Dragging the tab lifts the pane out of the sidebar; the dock
            // shows where it would land, and `on_dock_event` docks it there.
            .on_drag(AnyDrag::new(SidebarDrag { panel: id }), {
                let view = pinned.view.clone();
                let size = self.float_size(pinned);
                let me = cx.weak_entity();
                move |_: &AnyDrag, _, _, cx| {
                    cx.stop_propagation();
                    _ = me.update(cx, |this, cx| this.close_overlay(cx));
                    cx.new(|_| FloatingPane::new(view.clone(), size))
                }
            })
            .into_any_element()
    }

    fn overlay(&self, style: SidebarStyle, window: &mut Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let pinned = self.open_pinned()?;
        let id = pinned.id;
        let edge = pinned.edge;
        let extent = self.extent(pinned);
        let focused = pinned.view.focus_handle(cx).contains_focused(window, cx);
        let name = panel_name(&pinned.view, cx);
        let closable = pinned.view.closable(cx);
        let handle = PanelHandle::of(&pinned.view);
        let padded = handle.is_some_and(|handle| handle.inner_padding(cx));
        let maximized = self.maximized;

        let title = div()
            .flex()
            .flex_none()
            .items_center()
            .h(style.title_height)
            .pl(px(10.))
            .pr(px(6.))
            .gap(TITLE_BUTTON_GAP)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .font_weight(FontWeight::BOLD)
                    .text_size(TEXT_SIZE)
                    .text_color(style.title_text)
                    .child(name),
            )
            .child(
                chrome_button("lace-sidebar-unpin", DockIcon::Unpin, TITLE_BUTTON)
                    .tooltip("Unpin from Sidebar")
                    .on_click(cx.listener(move |this, _, window, cx| this.unpin(id, window, cx))),
            )
            .child(
                chrome_button(
                    "lace-sidebar-maximize",
                    if maximized { DockIcon::Restore } else { DockIcon::Maximize },
                    TITLE_BUTTON,
                )
                .tooltip(if maximized { "Restore" } else { "Maximize" })
                .on_click(cx.listener(|this, _, _, cx| {
                    this.maximized = !this.maximized;
                    cx.notify();
                })),
            )
            .when(closable, |this| {
                this.child(
                    chrome_button("lace-sidebar-close", DockIcon::Close, TITLE_BUTTON)
                        .tooltip("Close")
                        .on_click(cx.listener(move |this, _, _, cx| this.close_pinned(id, cx))),
                )
            });

        let grip = div()
            .id("lace-sidebar-grip")
            .absolute()
            .occlude()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.resizing.set(true);
                    cx.stop_propagation();
                }),
            );
        let grip = match edge {
            SideEdge::Left => grip.top_0().bottom_0().right(-GRIP / 2.).w(GRIP).cursor_ew_resize(),
            SideEdge::Right => grip.top_0().bottom_0().left(-GRIP / 2.).w(GRIP).cursor_ew_resize(),
            SideEdge::Bottom => grip.left_0().right_0().top(-GRIP / 2.).h(GRIP).cursor_ns_resize(),
        };

        let panel = div()
            .id("lace-sidebar-panel")
            .absolute()
            .occlude()
            .flex()
            .flex_col()
            .bg(style.panel_bg)
            .rounded(style.panel_radius)
            .border(style.panel_border_width)
            .border_color(if focused { style.panel_focus } else { style.panel_border })
            .shadow_lg()
            .on_hover(cx.listener(|this, hovered: &bool, window, cx| {
                this.over_panel = *hovered;
                if !*hovered {
                    this.schedule_hide(window, cx);
                }
            }))
            // The pane's handle lives here, as the dock's group holds it for
            // docked panes: a click anywhere in the pane focuses it (unless a
            // focusable element inside takes the click), and keys reach the
            // overlay's Escape.
            .track_focus(&pinned.view.focus_handle(cx))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    // Working in a peeked panel keeps it open.
                    this.peek = false;
                    cx.notify();
                }),
            )
            .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                if !this.over_rail && !this.resizing.get() {
                    this.close_overlay(cx);
                }
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                if event.keystroke.key == "escape" {
                    this.close_overlay(cx);
                    cx.stop_propagation();
                }
            }))
            .child(title)
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .when(padded, |this| this.px(px(10.)).pb(px(8.)))
                    .child(pinned.view.view()),
            )
            .child(grip);
        let panel = match edge {
            SideEdge::Left => panel.left(INSET).top(INSET).bottom(INSET).w(extent),
            SideEdge::Right => panel.right(INSET).top(INSET).bottom(INSET).w(extent),
            SideEdge::Bottom => panel.left(INSET).right(INSET).bottom(INSET).h(extent),
        };
        Some(panel.into_any_element())
    }
}

impl Render for Sidebars {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.dock_events.is_none() {
            let area = self.area.clone();
            self.dock_events = Some(cx.subscribe_in(&area, window, Self::on_dock_event));
        }
        if self.open.is_none() {
            self.restore_focus(window, cx);
        }
        let style = SidebarStyle::current(cx);
        let left = self.rail(SideEdge::Left, style, window, cx);
        let right = self.rail(SideEdge::Right, style, window, cx);
        let bottom = self.rail(SideEdge::Bottom, style, window, cx);
        let overlay = self.overlay(style, window, cx);
        let center_cell = self.center.clone();
        let resizing = self.resizing.clone();
        let me = cx.weak_entity();

        let center = div()
            .relative()
            .flex_1()
            .min_h_0()
            .size_full()
            .child(self.area.clone())
            .child(
                canvas(
                    move |bounds, _, _| center_cell.set(bounds),
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full(),
            )
            .children(overlay);

        div()
            .size_full()
            .flex()
            .flex_row()
            .children(left)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .flex()
                    .flex_col()
                    .child(center)
                    .children(bottom),
            )
            .children(right)
            .when(self.resizing.get(), |this| {
                this.child(
                    canvas(
                        |_, _, _| {},
                        move |_, _, window, _| {
                            let weak = me.clone();
                            window.on_mouse_event(move |event: &MouseMoveEvent, _, _, cx| {
                                _ = weak.update(cx, |this, cx| this.resize_to(event.position, cx));
                            });
                            let weak = me.clone();
                            let resizing = resizing.clone();
                            window.on_mouse_event(move |_: &MouseUpEvent, _, _, cx| {
                                resizing.set(false);
                                _ = weak.update(cx, |_, cx| cx.notify());
                            });
                        },
                    )
                    .absolute()
                    .size_full(),
                )
            })
    }
}

/// The pin button's way into [`Sidebars`].
struct SidebarPin(WeakEntity<Sidebars>);

impl PinTarget for SidebarPin {
    fn can_pin(&self, panel: PanelId, cx: &App) -> bool {
        self.0
            .upgrade()
            .is_some_and(|sidebars| sidebars.read(cx).pinnable.contains_key(&panel))
    }

    fn pin(&self, panel: PanelId, at: Point<Pixels>, window: &mut Window, cx: &mut App) {
        _ = self
            .0
            .update(cx, |sidebars, cx| sidebars.pin(panel, Some(at), window, cx));
    }
}

/// The text a sidebar tab and its overlay show for a panel.
fn panel_name(view: &Arc<dyn PanelView>, cx: &App) -> SharedString {
    PanelHandle::of(view)
        .and_then(|handle| handle.tab_name(cx))
        .unwrap_or_else(|| view.panel_name(cx).into())
}

/// The window edge nearest `at`: a centre panel pins to whichever side of
/// the window its title bar is closest to.
fn nearest_edge(at: Option<Point<Pixels>>, window: gpui_kit::gpui::Size<Pixels>) -> SideEdge {
    let Some(at) = at else {
        return SideEdge::Left;
    };
    let left = at.x;
    let right = window.width - at.x;
    let bottom = window.height - at.y;
    if bottom < left && bottom < right {
        SideEdge::Bottom
    } else if right < left {
        SideEdge::Right
    } else {
        SideEdge::Left
    }
}

/// The accent bar on the open tab, on the edge facing the content (Lace's
/// `indicator_position = "right"`).
fn indicator(edge: SideEdge, style: SidebarStyle) -> impl IntoElement {
    let bar = div().absolute().bg(style.indicator);
    match edge {
        SideEdge::Left => bar.top_0().bottom_0().right_0().w(style.indicator_width),
        SideEdge::Right => bar.top_0().bottom_0().left_0().w(style.indicator_width),
        SideEdge::Bottom => bar.left_0().right_0().top_0().h(style.indicator_width),
    }
}

/// Round a tab per Lace's `sidebar_tab_flat_edge`; the outward side is the
/// one against the window edge.
fn round_tab<E: gpui_kit::gpui::Styled>(tab: E, edge: SideEdge, style: SidebarStyle) -> E {
    let r = style.tab_radius;
    match (style.flat, edge) {
        (SideTabFlat::All, _) => tab,
        (SideTabFlat::None, _) => tab.rounded(r),
        (SideTabFlat::Outward, SideEdge::Left) | (SideTabFlat::Inward, SideEdge::Right) => {
            tab.rounded_tr(r).rounded_br(r)
        }
        (SideTabFlat::Outward, SideEdge::Right) | (SideTabFlat::Inward, SideEdge::Left) => {
            tab.rounded_tl(r).rounded_bl(r)
        }
        (SideTabFlat::Outward, SideEdge::Bottom) => tab.rounded_tl(r).rounded_tr(r),
        (SideTabFlat::Inward, SideEdge::Bottom) => tab.rounded_bl(r).rounded_br(r),
    }
}

/// The family name the SVG renderer can resolve: gpui's `.SystemUIFont`
/// alias means the platform UI face.
fn svg_family(family: &SharedString) -> SharedString {
    if family.starts_with('.') {
        "Segoe UI".into()
    } else {
        family.clone()
    }
}

/// A label turned a quarter clockwise, reading top to bottom as Lace's side
/// tabs do: `thickness` wide, `length` tall, in black (gpui tints it).
fn vertical_label(text: &str, family: &str, size: f32, thickness: f32, length: f32) -> Vec<u8> {
    // After rotate(90) the baseline runs down the box and the glyphs rise
    // towards +x; centre the x-height band (about 0.35em above the baseline).
    let baseline = thickness / 2. - 0.35 * size;
    format!(
        concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">"#,
            r##"<text transform="translate({x} 0) rotate(90)" font-family="{f}" font-size="{s}" fill="#000">{t}</text>"##,
            "</svg>"
        ),
        w = thickness,
        h = length,
        x = baseline,
        f = xml_escape(family),
        s = size,
        t = xml_escape(text),
    )
    .into_bytes()
}

fn xml_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vertical_labels_escape_and_size() {
        let svg = String::from_utf8(vertical_label("A & <B>", "Segoe UI", 14., 28., 90.)).unwrap();
        assert!(svg.contains(r#"width="28" height="90""#));
        assert!(svg.contains("A &amp; &lt;B&gt;"));
        assert!(svg.contains("rotate(90)"));
    }

    #[test]
    fn centre_panels_pin_to_the_nearest_edge() {
        let window = gpui_kit::gpui::size(px(1000.), px(800.));
        let at = |x: f32, y: f32| Some(gpui_kit::gpui::point(px(x), px(y)));
        assert_eq!(nearest_edge(at(100., 300.), window), SideEdge::Left);
        assert_eq!(nearest_edge(at(900., 300.), window), SideEdge::Right);
        assert_eq!(nearest_edge(at(500., 750.), window), SideEdge::Bottom);
    }

    #[test]
    fn outlines_follow_lace_states() {
        let mut style = SidebarStyle::default();
        assert_eq!(style.outline_for(true, false), None);
        style.outline = px(2.);
        style.outline_active = gpui_kit::gpui::hsla(0.7, 0.8, 0.7, 1.);
        style.outline_hover = gpui_kit::gpui::hsla(0.7, 0.8, 0.7, 0.5);
        assert_eq!(style.outline_for(true, true), Some(style.outline_active));
        assert_eq!(style.outline_for(false, true), Some(style.outline_hover));
        assert_eq!(style.outline_for(false, false), None);
    }
}
