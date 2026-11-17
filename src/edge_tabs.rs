//! Lace's edge treatments for dock tab strips: a left/top/right outline on
//! each tab, an accent bar on the active one, and the title rule along the
//! strip's bottom edge.
//!
//! gpui-kit 0.8's stock tab is fixed at 1px side borders in the theme's
//! border colour and gives a skin no handle on where each tab lands, so a
//! theme that outlines its tabs gets this strip instead. It is built only
//! from public dock API and keeps the stock strip's behaviour: select,
//! close, drag out, drop to reorder or merge, the zoom and ellipsis
//! controls, the panel's own toolbar, and the dock collapse affordances.
//!
//! One difference: the collapse buttons toggle the dock directly, so they do
//! not raise gpui-kit's `DockToggleEvent` (its fields are crate-private).
//! The dock's own `DockEvent::LayoutChanged` still fires.
//!
//! The geometry follows the installed theme: [`crate::skin`] writes a
//! [`TabEdgeStyle`] global whenever it applies one, and the
//! [`GeometryDockSkin`](crate::geometry::GeometryDockSkin) reads it per frame.

use std::{cell::Cell, rc::Rc, sync::Arc};

use gpui_kit::base::dock::{
    AnyDrag, DockArea, DockPlacement, DragPanel, NodeId, PaneNode, PaneRef, PanelId,
    TabGroupContext,
};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::dock::{BasePanelView, ClosePanel, PanelHandle, ToggleZoom};
use gpui_kit::component::menu::DropdownMenu as _;
use gpui_kit::component::theme::ActiveTheme as _;
use gpui_kit::component::{IconName, Sizable as _};
use gpui_kit::gpui::{
    div, prelude::FluentBuilder as _, px, size, transparent_black, AnyElement, App,
    AppContext as _, Anchor, Context, Global, Hsla, InteractiveElement as _, IntoElement,
    ParentElement as _, Pixels, Point, Render, ScrollHandle, SharedString,
    StatefulInteractiveElement as _, Styled as _, WeakEntity, Window,
};

use crate::dock_icons::DockIcon;
use crate::engine::TabIndicator;

/// The tab-strip edge geometry of the installed theme.
///
/// The default draws nothing extra, which leaves the stock strip in place.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TabEdgeStyle {
    /// Rule along the strip's bottom edge; zero for none.
    pub rule: Pixels,
    /// The rule while its group does not hold focus.
    pub rule_color: Hsla,
    /// The rule while its group holds focus.
    pub rule_focus: Hsla,
    /// Left/top/right outline on every tab; zero for none.
    pub outline: Pixels,
    /// Inactive tabs' outline. Transparent outlines the active tab alone.
    pub outline_color: Hsla,
    /// The active tab's outline.
    pub outline_active: Hsla,
    /// Where the active tab's accent bar sits.
    pub indicator: TabIndicator,
    pub indicator_width: Pixels,
    pub indicator_color: Hsla,
    /// Top-corner radius of an outlined tab.
    pub radius: Pixels,
    /// Strip height.
    pub height: Pixels,
    /// Fade the active tab's outline, bar and label while its group is
    /// unfocused (Lace's `tab_dimming`).
    pub dimming: bool,
    /// The active tab's outline while unfocused, replacing the dimming;
    /// transparent drops it (Lace's `tab_border_unfocused_color`).
    pub outline_unfocused: Option<Hsla>,
    /// The group frame starts under the strip and continues the active
    /// tab's outline (Lace's `border_below_title`).
    pub below_title: bool,
    /// The group frame's colour while it holds focus.
    pub frame_focus: Hsla,
    /// The group frame's width, and its bottom-corner radius.
    pub frame_width: Pixels,
    pub frame_radius: Pixels,
}

impl Default for TabEdgeStyle {
    fn default() -> Self {
        Self {
            rule: px(0.),
            rule_color: transparent_black(),
            rule_focus: transparent_black(),
            outline: px(0.),
            outline_color: transparent_black(),
            outline_active: transparent_black(),
            indicator: TabIndicator::None,
            indicator_width: px(0.),
            indicator_color: transparent_black(),
            radius: px(0.),
            height: px(30.),
            dimming: false,
            outline_unfocused: None,
            below_title: false,
            frame_focus: transparent_black(),
            frame_width: px(0.),
            frame_radius: px(0.),
        }
    }
}

/// `a` moved `t` of the way to `b`, per sRGB channel — Lace's
/// `blend_colors`.
pub(crate) fn mix(a: Hsla, b: Hsla, t: f32) -> Hsla {
    let (a, b) = (a.to_rgb(), b.to_rgb());
    let m = |x: f32, y: f32| x + (y - x) * t;
    Hsla::from(gpui_kit::gpui::Rgba {
        r: m(a.r, b.r),
        g: m(a.g, b.g),
        b: m(a.b, b.b),
        a: m(a.a, b.a),
    })
}

impl Global for TabEdgeStyle {}

impl TabEdgeStyle {
    /// The style the running app installed, or the stock look.
    pub fn current(cx: &App) -> Self {
        cx.try_global::<Self>().copied().unwrap_or_default()
    }

    /// Install `style` for every dock in the app.
    pub fn install(style: Self, cx: &mut App) {
        cx.set_global(style);
    }

    /// True when tabs need Lace's strip rather than the stock one.
    pub fn draws_tabs(&self) -> bool {
        self.outline > px(0.) || self.indicator != TabIndicator::None
    }

    /// True when the strip carries a rule.
    pub fn has_rule(&self) -> bool {
        self.rule > px(0.)
    }

    /// The strip rule's colour (Lace's `resolve_title_bar_border_color`):
    /// under `below_title` the rule is the frame's top side, so it takes the
    /// frame's colour; otherwise the focus colour while focused.
    pub(crate) fn rule_for(&self, focused: bool, tab_bg: Hsla) -> Hsla {
        if self.below_title && self.outline > px(0.) {
            return self.active_outline(focused, tab_bg).unwrap_or(transparent_black());
        }
        if focused {
            self.rule_focus
        } else {
            self.rule_color
        }
    }

    /// The active tab's outline in this focus state, or none (Lace's
    /// `resolve_tab_outline_color`). `tab_bg` is the active tab's fill,
    /// which a dimmed outline fades halfway into.
    pub(crate) fn active_outline(&self, focused: bool, tab_bg: Hsla) -> Option<Hsla> {
        if self.outline <= px(0.) {
            return None;
        }
        if !focused {
            if let Some(c) = self.outline_unfocused {
                return (c.a > 0.).then_some(c);
            }
        }
        let c = self.outline_active;
        if c.a <= 0. {
            return None;
        }
        Some(if !focused && self.dimming { mix(c, tab_bg, 0.5) } else { c })
    }

    /// The inactive tabs' outline, or none.
    fn inactive_outline(&self) -> Option<Hsla> {
        (self.outline > px(0.) && self.outline_color.a > 0.).then_some(self.outline_color)
    }

    /// The active tab's bar colour in this focus state.
    fn indicator_for(&self, focused: bool, tab_bg: Hsla) -> Hsla {
        if !focused && self.dimming {
            mix(self.indicator_color, tab_bg, 0.5)
        } else {
            self.indicator_color
        }
    }

    /// The frame under a `below_title` strip: its colour in this focus
    /// state, transparent when the tab has no outline.
    pub(crate) fn below_title_frame(&self, focused: bool, tab_bg: Hsla) -> Option<Hsla> {
        (self.below_title && self.outline > px(0.))
            .then(|| self.active_outline(focused, tab_bg).unwrap_or(transparent_black()))
    }
}

/// A title-bar button box: Lace's 17px button with 2px padding, its 16px
/// icon (gpui-kit sizes the icon at three quarters of the box).
pub(crate) const TITLE_BUTTON: Pixels = px(22.);
/// Space between title-bar buttons: Lace's 17px boxes sit 30px apart once
/// their padding and `button_spacing` add up.
pub(crate) const TITLE_BUTTON_GAP: Pixels = px(8.);
/// The tab close box: Lace's 17px with its 14px icon.
const TAB_CLOSE_BUTTON: Pixels = px(18.);

/// A ghost icon button with Lace's 3px hover rounding.
pub(crate) fn chrome_button(id: impl Into<gpui_kit::gpui::ElementId>, icon: DockIcon, size: Pixels) -> Button {
    Button::new(id)
        .icon(icon)
        .with_size(size)
        .ghost()
        .rounded(px(3.))
        .tab_stop(false)
}

/// Whether the group's displayed panel holds keyboard focus.
pub(crate) fn group_focused(group: &TabGroupContext, window: &Window, cx: &App) -> bool {
    group
        .active_panel()
        .is_some_and(|panel| panel.focus_handle(cx).contains_focused(window, cx))
}

/// A strip's rule as a full-width line along its bottom, painted under
/// whatever comes after it in the same parent.
pub(crate) fn rule_line(width: Pixels, color: Hsla) -> impl IntoElement {
    div()
        .absolute()
        .left_0()
        .right_0()
        .bottom_0()
        .h(width)
        .bg(color)
}

/// What a panel shows as its tab label.
fn panel_title(panel: &Arc<dyn BasePanelView>, window: &mut Window, cx: &mut App) -> AnyElement {
    match PanelHandle::of(panel) {
        Some(handle) => match handle.tab_name(cx) {
            Some(name) => name.into_any_element(),
            None => handle.title(window, cx),
        },
        None => SharedString::from(panel.panel_name(cx)).into_any_element(),
    }
}

/// The ghost tab that follows the pointer while a tab is dragged.
struct EdgeDragPreview {
    panel: Arc<dyn BasePanelView>,
}

impl Render for EdgeDragPreview {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let title = panel_title(&self.panel, window, cx);
        let theme = cx.theme();
        div()
            .id("lace-edge-drag")
            .cursor_grab()
            .py_1()
            .px_3()
            .w_24()
            .overflow_hidden()
            .whitespace_nowrap()
            .border_1()
            .border_color(theme.border)
            .rounded(theme.radius)
            .text_color(theme.tab_foreground)
            .bg(theme.tokens.tab_active)
            .opacity(0.75)
            .child(title)
    }
}

/// The left-most, top-most group in a tree: where a left (or bottom) dock's
/// collapse affordance goes.
fn left_top_group(node: &PaneNode) -> Option<NodeId> {
    match node.kind() {
        PaneRef::Tabs { .. } => Some(node.id()),
        PaneRef::Split { children, .. } => children.first().and_then(left_top_group),
    }
}

/// The right-most, top-most group: where a right dock's affordance goes.
fn right_top_group(node: &PaneNode) -> Option<NodeId> {
    match node.kind() {
        PaneRef::Tabs { .. } => Some(node.id()),
        PaneRef::Split { axis, children, .. } => match axis {
            gpui_kit::gpui::Axis::Vertical => children.first(),
            gpui_kit::gpui::Axis::Horizontal => children.last(),
        }
        .and_then(right_top_group),
    }
}

/// Per-group strip state: where its tabs are scrolled to, and which tab the
/// last frame showed, so a newly displayed tab is brought into view once.
#[derive(Default)]
pub(crate) struct EdgeTabBar {
    scroll: ScrollHandle,
    last_active: Rc<Cell<Option<usize>>>,
}

/// Settings the dock skin owns and the strip reads.
pub(crate) struct StripOptions<'a> {
    pub area: &'a WeakEntity<DockArea>,
    pub toggles_visible: bool,
    pub close_visible: bool,
    pub pin: Option<Rc<dyn PinTarget>>,
}

/// Where the title bar's pin button sends a panel: Lace's auto-hide
/// sidebars ([`crate::sidebar::Sidebars`]) install one on the dock skin.
pub trait PinTarget {
    /// Whether `panel` may leave the dock for a sidebar.
    fn can_pin(&self, panel: PanelId, cx: &App) -> bool;
    /// Pin `panel` to the sidebar nearest `at` (window coordinates).
    fn pin(&self, panel: PanelId, at: Point<Pixels>, window: &mut Window, cx: &mut App);
}

impl EdgeTabBar {
    fn dock_toggle(
        &self,
        placement: DockPlacement,
        group: &TabGroupContext,
        opts: &StripOptions,
        cx: &App,
    ) -> Option<Button> {
        if group.is_zoomed() || !opts.toggles_visible {
            return None;
        }
        let handle = opts.area.upgrade()?;
        let area = handle.read(cx);
        if !area.is_dock_collapsible(placement) {
            return None;
        }
        let designated = match placement {
            DockPlacement::Left => area
                .layout(DockPlacement::Center)
                .and_then(|tree| left_top_group(tree.root())),
            DockPlacement::Right => area
                .layout(DockPlacement::Center)
                .and_then(|tree| right_top_group(tree.root())),
            DockPlacement::Bottom => area
                .layout(DockPlacement::Bottom)
                .and_then(|tree| left_top_group(tree.root())),
            DockPlacement::Center => None,
        };
        if designated != Some(group.node()) {
            return None;
        }
        let open = area.is_dock_open(placement);
        let icon = match (placement, open) {
            (DockPlacement::Left, true) => IconName::PanelLeft,
            (DockPlacement::Left, false) => IconName::PanelLeftOpen,
            (DockPlacement::Right, true) => IconName::PanelRight,
            (DockPlacement::Right, false) => IconName::PanelRightOpen,
            (DockPlacement::Bottom, true) => IconName::PanelBottom,
            (DockPlacement::Bottom, false) => IconName::PanelBottomOpen,
            (DockPlacement::Center, _) => return None,
        };
        let weak = opts.area.clone();
        Some(
            Button::new(SharedString::from(format!("lace-toggle-dock:{placement:?}")))
                .icon(icon)
                .xsmall()
                .ghost()
                .tab_stop(false)
                .tooltip(if open { "Collapse" } else { "Expand" })
                .on_click(move |_, window, cx| {
                    _ = weak.update(cx, |area, cx| area.toggle_dock(placement, window, cx));
                }),
        )
    }

    /// The trailing controls in Lace's order: the panel's own buttons, the
    /// area menu, pin (when sidebars are installed), maximize/restore and
    /// close. Lace's float sits before close; gpui-kit has no floating
    /// windows to back it, so it is left out rather than shown dead.
    fn toolbar(
        &self,
        group: &TabGroupContext,
        close_visible: bool,
        pin: Option<Rc<dyn PinTarget>>,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let zoomed = group.is_zoomed();
        let handle = group.active_panel().and_then(PanelHandle::of);
        let control = group
            .active_panel()
            .filter(|panel| panel.zoomable(cx))
            .and_then(PanelHandle::of)
            .and_then(|handle| handle.zoom_control(cx));
        // Lace's title bar always carries maximize, so any zoom control,
        // menu-only included, puts the button here too.
        let toolbar_zoom = control.is_some();
        let menu_zoom = control.is_some_and(|c| c.menu_visible());
        let closable = group.is_closable();
        let active_id = group.active_panel().map(|panel| panel.panel_id(cx));
        let close_active = close_visible
            && active_id.is_some_and(|id| group.is_panel_closable(id, cx));
        let buttons = handle.and_then(|handle| handle.toolbar_buttons(window, cx));
        let panel = handle.map(|handle| handle.panel());

        div()
            .flex()
            .items_center()
            .gap(TITLE_BUTTON_GAP)
            .occlude()
            .when_some(buttons, |this, buttons| {
                this.children(
                    buttons
                        .into_iter()
                        .map(|button| button.with_size(TITLE_BUTTON).ghost().tab_stop(false)),
                )
            })
            .child(
                chrome_button("lace-dock-menu", DockIcon::TabsMenu, TITLE_BUTTON)
                    .dropdown_menu(move |menu, window, cx| {
                        menu.when_some(panel.clone(), |menu, panel| {
                            panel.dropdown_menu(menu, window, cx)
                        })
                        .separator()
                        .menu_with_disabled(
                            if zoomed { "Zoom Out" } else { "Zoom In" },
                            Box::new(ToggleZoom),
                            !menu_zoom,
                        )
                        .when(closable, |menu| {
                            menu.separator().menu("Close", Box::new(ClosePanel))
                        })
                    })
                    .anchor(Anchor::TopRight),
            )
            .when_some(
                active_id
                    .zip(pin)
                    .filter(|(id, pin)| !zoomed && pin.can_pin(*id, cx)),
                |this, (id, pin)| {
                    this.child(
                        chrome_button("lace-dock-pin", DockIcon::Pin, TITLE_BUTTON)
                            .tooltip("Pin to Sidebar")
                            .on_click(move |event, window, cx| {
                                pin.pin(id, event.position(), window, cx)
                            }),
                    )
                },
            )
            .when(zoomed || toolbar_zoom, |this| {
                let (icon, tooltip) = if zoomed {
                    (DockIcon::Restore, "Restore")
                } else {
                    (DockIcon::Maximize, "Maximize")
                };
                this.child(
                    chrome_button("lace-dock-maximize", icon, TITLE_BUTTON)
                        .tooltip(tooltip)
                        .on_click({
                            let group = group.clone();
                            move |_, window, cx| group.toggle_zoom(window, cx)
                        }),
                )
            })
            .when_some(active_id.filter(|_| close_active), |this, id| {
                this.child(
                    chrome_button("lace-dock-close", DockIcon::Close, TITLE_BUTTON)
                        .tooltip("Close")
                        .on_click({
                            let group = group.clone();
                            move |_, window, cx| group.close(id, window, cx)
                        }),
                )
            })
            .into_any_element()
    }

    /// The whole strip.
    pub(crate) fn render(
        &self,
        group: &TabGroupContext,
        edges: TabEdgeStyle,
        opts: StripOptions,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let left = self.dock_toggle(DockPlacement::Left, group, &opts, cx);
        let bottom = self.dock_toggle(DockPlacement::Bottom, group, &opts, cx);
        let right = self.dock_toggle(DockPlacement::Right, group, &opts, cx);
        let is_bottom_dock = bottom.is_some();
        let collapsed = group.is_collapsed();
        let droppable = group.is_droppable();
        let tabs_count = group.panels().len();
        let active_ix = group.active_ix();
        let displayed = group.active_panel().map(|panel| panel.panel_id(cx));
        let visible: Vec<usize> = group
            .panels()
            .iter()
            .enumerate()
            .filter(|(_, panel)| panel.visible(cx))
            .map(|(ix, _)| ix)
            .collect();
        let displayed_ix = displayed.and_then(|id| {
            group
                .panels()
                .iter()
                .position(|panel| panel.panel_id(cx) == id)
        });

        if self.last_active.replace(Some(active_ix)) != Some(active_ix) {
            if let Some(pos) = visible.iter().position(|ix| *ix == active_ix) {
                self.scroll.scroll_to_item(pos);
            }
        }

        let focused = group_focused(group, window, cx);
        let theme = cx.theme();
        let strip_bg = theme.tab_bar;
        let tab_active_bg = theme.tab_active;
        let hover_bg = theme.tab_bar.blend(theme.foreground.opacity(0.06));
        let fg = theme.tab_foreground;
        let dimmed = edges.dimming && !focused;
        // A dimmed active label sits halfway to the inactive colour.
        let fg_active = if dimmed {
            mix(theme.tab_active_foreground, fg, 0.5)
        } else {
            theme.tab_active_foreground
        };
        let drag_border = theme.drag_border;
        let drop_target = theme.drop_target;

        let rule = edges.rule.max(px(0.));
        let rule_color = edges.rule_for(focused, tab_active_bg);
        let outline = edges.outline.max(px(0.));
        let outlined = outline > px(0.);
        let active_outline = edges.active_outline(focused, tab_active_bg);
        let inactive_outline = edges.inactive_outline();
        let indicator_color = edges.indicator_for(focused, tab_active_bg);
        // Outlined tabs stand clear of the strip's top edge.
        let top_gap = if outlined { px(3.) } else { px(0.) };
        let bar = edges.indicator_width.max(px(1.));

        let mut tabs: Vec<AnyElement> = Vec::with_capacity(visible.len() + 1);
        for ix in visible {
            let panel = group.panels()[ix].clone();
            let selected = !collapsed && Some(ix) == displayed_ix;
            let title = panel_title(&panel, window, cx);
            let drag = group
                .is_draggable()
                .then(|| group.drag_panel(ix, cx))
                .flatten();
            let close = !collapsed
                && opts.close_visible
                && group.is_panel_closable(panel.panel_id(cx), cx);

            let tab = div()
                .id(("lace-edge-tab", ix))
                .relative()
                .flex()
                .flex_row()
                .flex_shrink_0()
                .items_center()
                .gap_1()
                .px_3()
                .text_sm()
                .whitespace_nowrap()
                .cursor_pointer()
                // The active tab reaches the strip's bottom and covers the
                // rule there, opening into its panel; the others stop on it.
                .map(|this| match selected {
                    true => this
                        .h(edges.height - top_gap)
                        .bg(tab_active_bg)
                        .text_color(fg_active),
                    false => this
                        .h(edges.height - top_gap - rule)
                        .mb(rule)
                        .text_color(fg)
                        .hover(|this| this.bg(hover_bg).text_color(fg_active)),
                })
                .when(outlined, |this| {
                    this.border_l(outline)
                        .border_t(outline)
                        .border_r(outline)
                        // A state without an outline keeps the width, so
                        // labels do not shift as focus moves.
                        .border_color(
                            if selected { active_outline } else { inactive_outline }
                                .unwrap_or(transparent_black()),
                        )
                        .rounded_t(edges.radius)
                })
                .when(!outlined && edges.radius > px(0.), |this| {
                    this.rounded_t(edges.radius)
                })
                .when(selected && edges.indicator == TabIndicator::Top, |this| {
                    this.child(
                        div()
                            .absolute()
                            .top_0()
                            .left_0()
                            .right_0()
                            .h(bar)
                            .bg(indicator_color),
                    )
                })
                .when(selected && edges.indicator == TabIndicator::Bottom, |this| {
                    this.child(
                        div()
                            .absolute()
                            .bottom_0()
                            .left_0()
                            .right_0()
                            .h(bar)
                            .bg(indicator_color),
                    )
                })
                .child(div().flex_shrink_0().child(title))
                // Lace boxes the ✕ and shows it on the active tab only.
                .when(close && selected, |this| {
                    let group = group.clone();
                    let id = panel.panel_id(cx);
                    this.child(
                        chrome_button(("lace-close-tab", ix), DockIcon::CloseTab, TAB_CLOSE_BUTTON)
                            .ml(px(4.))
                            .mr(-px(4.))
                            .on_click(move |_, window, cx| {
                                cx.stop_propagation();
                                group.close(id, window, cx);
                            }),
                    )
                })
                .on_click({
                    let group = group.clone();
                    let area = opts.area.clone();
                    let last_active = self.last_active.clone();
                    move |_, window, cx| {
                        // A click reveals the tab itself; no snap.
                        last_active.set(Some(ix));
                        group.select_tab(ix, window, cx);
                        if is_bottom_dock && collapsed {
                            _ = area.update(cx, |area, cx| {
                                area.toggle_dock(DockPlacement::Bottom, window, cx)
                            });
                        }
                    }
                })
                .when(!collapsed, |this| {
                    this.when_some(drag, |this, drag| {
                        this.on_drag(drag, {
                            let panel = panel.clone();
                            move |drag: &DragPanel, offset, _, cx| {
                                cx.stop_propagation();
                                drag.set_drag_offset(offset);
                                drag.set_preview_size(size(px(96.), px(30.)));
                                cx.new(|_| EdgeDragPreview {
                                    panel: panel.clone(),
                                })
                            }
                        })
                    })
                    .when(droppable, |this| {
                        this.drag_over::<DragPanel>(move |this, _, _, _| {
                            this.border_l_2().border_color(drag_border)
                        })
                        .on_drop({
                            let group = group.clone();
                            move |drag: &DragPanel, window, cx| {
                                group.drop_panel(drag.clone(), Some(ix), true, window, cx);
                            }
                        })
                        .drag_over::<AnyDrag>(move |this, _, _, _| {
                            this.border_l_2().border_color(drag_border)
                        })
                        .on_drop({
                            let group = group.clone();
                            move |item: &AnyDrag, window, cx| {
                                group.drop_item(item.clone(), None, window, cx);
                            }
                        })
                    })
                });
            tabs.push(tab.into_any_element());
        }

        // Empty space past the last tab, so a panel can be dropped there.
        tabs.push(
            div()
                .id("lace-edge-empty")
                .h_full()
                .flex_grow(1.)
                .min_w_16()
                .when(droppable, |this| {
                    this.drag_over::<DragPanel>(move |this, _, _, _| this.bg(drop_target))
                        .on_drop({
                            let group = group.clone();
                            let node = group.node();
                            move |drag: &DragPanel, window, cx| {
                                let ix = (drag.source() == node).then(|| tabs_count - 1);
                                group.drop_panel(drag.clone(), ix, false, window, cx);
                            }
                        })
                        .drag_over::<AnyDrag>(move |this, _, _, _| this.bg(drop_target))
                        .on_drop({
                            let group = group.clone();
                            move |item: &AnyDrag, window, cx| {
                                group.drop_item(item.clone(), None, window, cx);
                            }
                        })
                })
                .into_any_element(),
        );

        let leading = left.is_some() || bottom.is_some();
        let trailing = (!collapsed).then(|| {
            let suffix = group
                .active_panel()
                .and_then(PanelHandle::of)
                .and_then(|handle| handle.title_suffix(window, cx));
            let toolbar = self.toolbar(group, opts.close_visible, opts.pin.clone(), window, cx);
            div()
                .flex()
                .flex_none()
                .items_center()
                .gap(TITLE_BUTTON_GAP)
                .pl_2()
                .pr(px(6.))
                .h(edges.height - rule)
                .children(suffix)
                .child(toolbar)
                .children(right)
        });

        div()
            .id(("lace-edge-strip", group.node().as_u64()))
            .relative()
            .flex()
            .flex_row()
            .flex_none()
            .w_full()
            .h(edges.height)
            .bg(strip_bg)
            // First, so tabs paint over it: the active one hides it.
            .when(rule > px(0.), |this| this.child(rule_line(rule, rule_color)))
            .when(leading, |this| {
                this.child(
                    div()
                        .flex()
                        .flex_none()
                        .items_center()
                        .gap_1()
                        .px_2()
                        .h(edges.height - rule)
                        .children(left)
                        .children(bottom),
                )
            })
            .child(
                div()
                    .id("lace-edge-tabs")
                    .flex()
                    .flex_row()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .items_end()
                    // Under `below_title` the first tab's outline is the
                    // frame's side carried up, so it sits flush left.
                    // Lace's `TAB.margin` (3) leaves about 5px between
                    // outlined tabs.
                    .when(outlined, |this| this.gap(px(5.)))
                    .when(outlined && !edges.below_title, |this| this.pl(px(4.)))
                    .overflow_x_scroll()
                    .track_scroll(&self.scroll)
                    .children(tabs),
            )
            .children(trailing)
            .into_any_element()
    }
}
