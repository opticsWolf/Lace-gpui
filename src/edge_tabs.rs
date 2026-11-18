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
use gpui_kit::component::{Disableable as _, IconName, Sizable as _};
use gpui_kit::gpui::{
    canvas, div, point, prelude::FluentBuilder as _, px, transparent_black, AnyElement, App,
    AppContext as _, Anchor, Bounds, Global, Hsla, InteractiveElement as _, IntoElement,
    ParentElement as _, Pixels, Point, ScrollHandle, SharedString, TextRun,
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
    /// The pane body's fill (Lace's `PANEL.bg_normal`). `None` keeps the
    /// theme background.
    pub panel_bg: Option<Hsla>,
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
            panel_bg: None,
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
/// Lace's dock tab margins at its 10pt font (spacing = 4): 2·spacing ahead
/// of the title, spacing before the close box, 4/3·spacing after it.
const TAB_LEAD: Pixels = px(8.);
const TAB_BEFORE_CLOSE: Pixels = px(4.);
const TAB_TAIL: Pixels = px(5.);
/// Lace's tab bar spacing between tabs (`TAB.margin`).
const TAB_SPACING: Pixels = px(3.);
/// The ‹ › pair's share of the strip: its 4px lead, two title buttons and
/// the 2px between them.
const TAB_ARROWS_WIDTH: Pixels = px(4. + 22. + 2. + 22.);
/// A title may shrink to this multiple of its elided stub (two characters
/// and an ellipsis), Lace's `CRAMPED_FACTOR`; a lone tab, with nowhere to
/// scroll to, goes down to the stub itself.
const CRAMPED_FACTOR: f32 = 2.0;
/// Room for the leading icon of a Lace [`tab_label`](crate::dock_icons::tab_label)
/// (16px and the 6px gap), kept by titles that come as elements.
const TAB_ICON_ROOM: Pixels = px(16. + 6.);

/// How wide `text` sets in the strip's tab font (`text_sm`).
fn tab_text_width(text: SharedString, window: &Window) -> Pixels {
    let style = window.text_style();
    let run = TextRun {
        len: text.len(),
        font: style.font(),
        color: style.color,
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    let size = window.rem_size() * 0.875;
    window.text_system().shape_line(text, size, &[run], None).width
}

/// The narrowest a tab's title may get, as Lace's cramped minimum: `factor`
/// times the width at which the label starts to elide, but never wider than
/// the whole label.
fn title_minimum(label: &str, icon: Pixels, factor: f32, window: &Window) -> Pixels {
    let stub: String = label.chars().take(2).chain(['…']).collect();
    let full = tab_text_width(SharedString::from(label.to_string()), window);
    let stub = tab_text_width(stub.into(), window);
    icon + full.min(stub * factor)
}

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
pub(crate) fn panel_title(
    panel: &Arc<dyn BasePanelView>,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    match PanelHandle::of(panel) {
        Some(handle) => match handle.tab_name(cx) {
            Some(name) => name.into_any_element(),
            None => handle.title(window, cx),
        },
        None => SharedString::from(panel.panel_name(cx)).into_any_element(),
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
    /// The overflow state the last render drew its arrows for.
    overflow: Cell<TabOverflow>,
}

/// Whether a strip's tabs overflow, and which ways they can scroll: Lace's
/// ‹ › arrows show only while the tabs overflow and there is more than one
/// (a lone tab has nowhere to scroll to), each enabled while its side hides
/// a tab.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct TabOverflow {
    pub shown: bool,
    pub left: bool,
    pub right: bool,
}

impl TabOverflow {
    /// From how far the tabs overflow their row (`excess`), whether the
    /// arrows are up already, and whether a click either way would move.
    ///
    /// The tabs elide down to their cramped minimums before they overflow,
    /// so `excess` is only ever above zero once they can shrink no more. The
    /// arrows take their own width from the row, so once up they stay only
    /// while the tabs would still overflow without them: they go as soon as
    /// the tabs fit again, not once they fit beside the arrows.
    pub fn from_excess(tabs: usize, excess: Pixels, was_shown: bool, back: bool, forward: bool) -> Self {
        let slack = px(0.5);
        let room = if was_shown { TAB_ARROWS_WIDTH } else { px(0.) };
        let shown = tabs > 1 && excess > room + slack;
        Self {
            shown,
            left: shown && back,
            right: shown && forward,
        }
    }

    /// From the tab row's scroll handle, as its last layout left it.
    ///
    /// Each arrow is enabled while a click on it would move the row: back
    /// while the row is scrolled at all, forward until the last tab sits
    /// where the forward arrow puts it. That end is the last tab, not the
    /// scroll extent.
    fn of(scroll: &ScrollHandle, tabs: usize, was_shown: bool) -> Self {
        let slack = px(0.5);
        let offset = scroll.offset().x;
        let end = tabs
            .checked_sub(1)
            .and_then(|ix| scroll.bounds_for_item(ix))
            .map(|b| reveal_offset(scroll, b, false));
        Self::from_excess(
            tabs,
            scroll.max_offset().x,
            was_shown,
            offset < -slack,
            end.is_some_and(|end| offset > end + slack),
        )
    }
}

/// The row offset that brings a tab in from the left (`back`) or right.
/// Child bounds are laid out unscrolled; the offset moves them. The tab
/// aligns to the side it comes in from; one wider than the row aligns
/// left, as in Lace.
fn reveal_offset(scroll: &ScrollHandle, tab: Bounds<Pixels>, back: bool) -> Pixels {
    let view = scroll.bounds();
    let x = if back || tab.size.width >= view.size.width {
        view.left() - tab.left()
    } else {
        view.right() - tab.right()
    };
    x.max(-scroll.max_offset().x).min(px(0.))
}

/// Reveal the next tab hidden to the left (`back`) or right, as Lace's
/// arrows do; with none left on that side, scroll to the end.
fn scroll_tabs(scroll: &ScrollHandle, tabs: usize, back: bool) {
    let offset = scroll.offset();
    let view = scroll.bounds();
    let slack = px(0.5);
    // As in Lace, the target is a tab wholly out of view: a partly clipped
    // one may be wider than the row, and scrolling to it would not move.
    let target = if back {
        (0..tabs).rev().find(|&ix| {
            scroll
                .bounds_for_item(ix)
                .is_some_and(|b| b.right() + offset.x <= view.left() + slack)
        })
    } else {
        (0..tabs).find(|&ix| {
            scroll
                .bounds_for_item(ix)
                .is_some_and(|b| b.left() + offset.x >= view.right() - slack)
        })
    };
    // With none hidden on that side, scroll to that end: the start, or the
    // last tab (not past it into the drop zone). The offset is set here
    // rather than through `scroll_to_item`, so the arrows drawn next
    // already see it.
    let x = match target.or(if back { None } else { tabs.checked_sub(1) }) {
        Some(ix) => match scroll.bounds_for_item(ix) {
            Some(tab) => reveal_offset(scroll, tab, back),
            None => return,
        },
        None => px(0.),
    };
    scroll.set_offset(point(x, offset.y));
}

/// Settings the dock skin owns and the strip reads.
pub(crate) struct StripOptions<'a> {
    pub area: &'a WeakEntity<DockArea>,
    pub toggles_visible: bool,
    pub close_visible: bool,
    pub pin: Option<Rc<dyn PinTarget>>,
    /// The group frame's inner corner radius. gpui clips children to a
    /// rectangle, so the strip rounds its own top corners to stay inside.
    pub corner: Pixels,
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
        let bar = edges.indicator_width.max(px(1.));
        let corner = opts.corner.max(px(0.));
        // A tab sitting in the frame's top-left corner follows its curve.
        let flush_left = left.is_none()
            && bottom.is_none()
            && (!outlined || edges.below_title)
            && corner > edges.radius;
        let first_visible = visible.first().copied();
        let tab_n = visible.len();
        let overflow = TabOverflow::of(&self.scroll, tab_n, self.overflow.get().shown);
        // Lace tabs elide before the row scrolls: each may shrink to its
        // cramped minimum, a lone one down to its stub.
        let factor = if tab_n > 1 { CRAMPED_FACTOR } else { 1. };
        // Lace's tab layout, from the outer edge: 2·spacing before the
        // title, then spacing, the 24px close box and 4/3·spacing after it
        // (spacing = a quarter of the font height). The close box here is
        // 18px, so its 3px a side go into the margins around it.
        let border = if outlined { outline } else { px(0.) };
        let lead = TAB_LEAD - border;
        let close_box_pad = (px(24.) - TAB_CLOSE_BUTTON) / 2.;
        let tail = TAB_TAIL - border;
        let tab_gap = TAB_BEFORE_CLOSE + close_box_pad;
        self.overflow.set(overflow);

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
            let with_close = close && selected;

            // The cramped minimum follows the label text: the tab name, or
            // else the panel name, which a title element is taken to show
            // after a Lace tab icon.
            let handle = PanelHandle::of(&panel);
            let tab_name = handle.as_ref().and_then(|handle| handle.tab_name(cx));
            let icon = if handle.is_some() && tab_name.is_none() { TAB_ICON_ROOM } else { px(0.) };
            let label = tab_name.unwrap_or_else(|| SharedString::from(panel.panel_name(cx).to_string()));
            let title_min = title_minimum(&label, icon, factor, window);
            // The tab keeps its padding, outline and close box whole; only
            // the title gives.
            let pad_right = if with_close {
                tail + close_box_pad
            } else {
                TAB_BEFORE_CLOSE + tail
            };
            let tab_min = lead
                + pad_right
                + border * 2.
                + title_min
                + if with_close { tab_gap + TAB_CLOSE_BUTTON } else { px(0.) };

            let tab = div()
                .id(("lace-edge-tab", ix))
                .relative()
                .flex()
                .flex_row()
                .flex_shrink(1.)
                .min_w(tab_min)
                .items_center()
                .pl(lead)
                .pr(pad_right)
                .text_sm()
                .whitespace_nowrap()
                .cursor_pointer()
                // Tabs reach the strip's top edge, as Lace's title bar has no
                // top padding. The active tab also reaches its bottom and
                // covers the rule there, opening into its panel; the others
                // stop on it.
                .map(|this| match selected {
                    true => this
                        .h(edges.height)
                        .bg(tab_active_bg)
                        .text_color(fg_active),
                    false => this
                        .h(edges.height - rule)
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
                .when(flush_left && Some(ix) == first_visible, |this| {
                    this.rounded_tl(corner)
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
                // The title elides (`…`) as the tab shrinks.
                .child(
                    div()
                        .flex_shrink(1.)
                        .min_w(title_min)
                        .overflow_hidden()
                        .text_ellipsis()
                        .child(title),
                )
                // Lace boxes the ✕ and shows it on the active tab only.
                .when(with_close, |this| {
                    let group = group.clone();
                    let id = panel.panel_id(cx);
                    this.child(
                        chrome_button(("lace-close-tab", ix), DockIcon::CloseTab, TAB_CLOSE_BUTTON)
                            .flex_none()
                            .ml(tab_gap)
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
                            let node = group.node();
                            move |drag: &DragPanel, offset, _, cx| {
                                cx.stop_propagation();
                                // The whole pane lifts out at its docked size.
                                let size = crate::floating::group_size(node)
                                    .unwrap_or(crate::floating::FALLBACK_SIZE);
                                drag.set_drag_offset(offset);
                                drag.set_preview_size(size);
                                cx.new(|_| crate::floating::FloatingPane::new(panel.clone(), size))
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
        // Like Lace's trailing stretch it has no width of its own: it fills
        // what the tabs leave and is gone once they overflow, so the row
        // never scrolls past its last tab.
        tabs.push(
            div()
                .id("lace-edge-empty")
                .h_full()
                .flex_grow(1.)
                .min_w_0()
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

        // Lace's ‹ › tab scroll arrows, after the tabs and before the
        // title-bar buttons.
        let arrows = overflow.shown.then(|| {
            let arrow = |id: &'static str, icon: DockIcon, tip: &'static str, enabled: bool, back: bool| {
                let scroll = self.scroll.clone();
                chrome_button(id, icon, TITLE_BUTTON)
                    .disabled(!enabled)
                    .tooltip(tip)
                    .on_click(move |_, window, _| {
                        scroll_tabs(&scroll, tab_n, back);
                        window.refresh();
                    })
            };
            div()
                .flex()
                .flex_none()
                .items_center()
                .gap(px(2.))
                .pl(px(4.))
                .h(edges.height - rule)
                .child(arrow("lace-tab-scroll-left", DockIcon::ChevronLeft, "Scroll tabs left", overflow.left, true))
                .child(arrow("lace-tab-scroll-right", DockIcon::ChevronRight, "Scroll tabs right", overflow.right, false))
        });

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
            .when(corner > px(0.), |this| this.rounded_t(corner))
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
                    // Lace's tab bar spaces outlined tabs by `TAB.margin`.
                    .when(outlined, |this| this.gap(TAB_SPACING))
                    .when(outlined && !edges.below_title, |this| this.pl(px(4.)))
                    .overflow_x_scroll()
                    .track_scroll(&self.scroll)
                    .children(tabs),
            )
            .children(arrows)
            .children(trailing)
            // The wheel over the tab row scrolls it through gpui's own
            // handler (a vertical wheel turns into horizontal scrolling).
            // The arrows follow the last layout. When this frame's layout
            // changes what they should be (a resize, a tab added or
            // removed), draw once more.
            .child({
                let scroll = self.scroll.clone();
                canvas(
                    move |_, window, _| {
                        if TabOverflow::of(&scroll, tab_n, overflow.shown) != overflow {
                            window.on_next_frame(|window, _| window.refresh());
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .size_0()
            })
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::TabOverflow;
    use gpui_kit::gpui::px;

    #[test]
    fn arrows_follow_lace_overflow_rules() {
        // Tabs that fit: no arrows.
        assert!(!TabOverflow::from_excess(3, px(0.), false, false, false).shown);
        // A lone tab never scrolls, however narrow the strip.
        assert!(!TabOverflow::from_excess(1, px(80.), false, false, true).shown);
        // The first pixel of overflow brings the arrows up.
        assert!(TabOverflow::from_excess(3, px(1.), false, false, true).shown);
        // At the start only the right arrow works, at the end only the left.
        let start = TabOverflow::from_excess(3, px(80.), false, false, true);
        assert_eq!((start.shown, start.left, start.right), (true, false, true));
        let end = TabOverflow::from_excess(3, px(80.), true, true, false);
        assert_eq!((end.left, end.right), (true, false));
        let middle = TabOverflow::from_excess(3, px(80.), true, true, true);
        assert_eq!((middle.left, middle.right), (true, true));
        // Once up, the arrows go as soon as the tabs would fit without them,
        // not only once they fit beside them.
        let fits_without = super::TAB_ARROWS_WIDTH - px(1.);
        assert!(!TabOverflow::from_excess(3, fits_without, true, true, false).shown);
        assert!(TabOverflow::from_excess(3, fits_without, false, true, false).shown);
    }
}
