//! Panes in flight, and where they would land.
//!
//! Lace lifts a dragged pane out into a floating window the moment the drag
//! starts, so the whole pane — its tab strip and its content — follows the
//! pointer, and an overlay over the hovered dock area shows where it would
//! land. gpui-kit's dock decides the landing (it hands the skin a
//! [`DropIndicator`]) and leaves the drawing to the skin; this module draws
//! both halves:
//!
//! - [`FloatingPane`] is the drag preview: the pane at its docked size, its
//!   tab strip holding the one dragged tab over the panel's live content,
//!   slightly see-through so the drop targets under it still show.
//! - [`drop_overlay`] is Lace's `DockOverlay`: the region the drop would
//!   take, filled with the theme's translucent overlay colour inside a frame,
//!   and the cross of five targets with the hovered one lit.
//!
//! The preview renders the panel's view a second time rather than a snapshot
//! of its pixels: gpui has no way to capture an element as an image, and a
//! second render costs one more layout of the pane while the drag lasts. The
//! copy gets its own element state, so it opens scrolled to the top.

use std::{cell::RefCell, collections::HashMap, sync::Arc};

use gpui_kit::base::dock::{DropIndicator, NodeId, PanelView};
use gpui_kit::base::Placement;
use gpui_kit::component::theme::ActiveTheme as _;
use gpui_kit::gpui::{
    div, point, prelude::FluentBuilder as _, px, AnyElement, App, Bounds, Context, Global, Hsla,
    IntoElement, ParentElement as _, Pixels, Point, Render, Size, Styled as _, Window,
};

use crate::edge_tabs::{panel_title, TabEdgeStyle};
use crate::engine::TabIndicator;

/// The drop overlay colours of the installed theme: Lace's `OVERLAY`
/// category. [`crate::skin`] installs it with every theme.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DropOverlayStyle {
    /// The drop region's outline, and the lit target.
    pub frame: Hsla,
    /// The drop region's fill; translucent, so the panes under it show.
    pub fill: Hsla,
    /// A target's face.
    pub cell: Hsla,
    /// The area glyph drawn on each target.
    pub glyph: Hsla,
}

impl Default for DropOverlayStyle {
    fn default() -> Self {
        // Lace's fallbacks: a steel-blue frame over a quarter-opaque fill.
        let blue = gpui_kit::gpui::hsla(0.6, 0.6, 0.31, 1.);
        Self {
            frame: blue,
            fill: blue.opacity(0.25),
            cell: gpui_kit::gpui::hsla(0., 0., 0.16, 1.),
            glyph: gpui_kit::gpui::hsla(0., 0., 0.9, 1.),
        }
    }
}

impl Global for DropOverlayStyle {}

impl DropOverlayStyle {
    /// The style the running app installed, or the default.
    pub fn current(cx: &App) -> Self {
        cx.try_global::<Self>().copied().unwrap_or_default()
    }

    /// Install `style` for every dock in the app.
    pub fn install(style: Self, cx: &mut App) {
        cx.set_global(style);
    }
}

/// How opaque a pane in flight is: enough to read it, not so much that it
/// hides the drop targets under it.
pub const FLOAT_OPACITY: f32 = 0.88;

/// The size of a pane in flight whose docked size is unknown.
pub const FALLBACK_SIZE: Size<Pixels> = Size {
    width: px(360.),
    height: px(260.),
};

thread_local! {
    /// Where each tab group was last painted, so a drag can lift the pane
    /// out at the size it had in the dock. The dock's groups live on the UI
    /// thread, as does every reader.
    static GROUP_BOUNDS: RefCell<HashMap<u64, Bounds<Pixels>>> = RefCell::new(HashMap::new());
}

/// Note where `node`'s group was painted.
pub(crate) fn record_group(node: NodeId, bounds: Bounds<Pixels>) {
    GROUP_BOUNDS.with(|map| {
        map.borrow_mut().insert(node.as_u64(), bounds);
    });
}

/// The size `node`'s group was last painted at.
pub(crate) fn group_size(node: NodeId) -> Option<Size<Pixels>> {
    GROUP_BOUNDS.with(|map| map.borrow().get(&node.as_u64()).map(|bounds| bounds.size))
}

/// A pane in flight: the drag preview for a dock tab or a sidebar tab.
///
/// gpui places a drag preview so that the pointer keeps the offset it had
/// inside the element the drag started on. The preview's first tab stands
/// where that element was, so the pane hangs from the tab under the pointer.
pub struct FloatingPane {
    panel: Arc<dyn PanelView>,
    size: Size<Pixels>,
}

impl FloatingPane {
    pub fn new(panel: Arc<dyn PanelView>, size: Size<Pixels>) -> Self {
        Self { panel, size }
    }
}

impl Render for FloatingPane {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let edges = TabEdgeStyle::current(cx);
        let overlay = DropOverlayStyle::current(cx);
        let title = panel_title(&self.panel, window, cx);
        let theme = cx.theme();
        let strip_height = if edges.height > px(0.) { edges.height } else { px(32.) };
        let outline = edges.outline.max(px(0.));
        let bar = edges.indicator_width.max(px(1.));
        let radius = edges.frame_radius.max(px(0.));

        let tab = div()
            .relative()
            .flex()
            .flex_none()
            .items_center()
            // The docked tab's padding without its close box.
            .pl(px(8.) - outline)
            .pr(px(9.) - outline)
            .h_full()
            .text_sm()
            .whitespace_nowrap()
            .bg(theme.tab_active)
            .text_color(theme.tab_active_foreground)
            .when(edges.radius > px(0.), |this| this.rounded_t(edges.radius))
            .when(outline > px(0.), |this| {
                this.border_l(outline)
                    .border_t(outline)
                    .border_r(outline)
                    .border_color(edges.outline_active)
            })
            .when(edges.indicator == TabIndicator::Top, |this| {
                this.child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .right_0()
                        .h(bar)
                        .bg(edges.indicator_color),
                )
            })
            .when(edges.indicator == TabIndicator::Bottom, |this| {
                this.child(
                    div()
                        .absolute()
                        .bottom_0()
                        .left_0()
                        .right_0()
                        .h(bar)
                        .bg(edges.indicator_color),
                )
            })
            .child(title);

        div()
            .w(self.size.width)
            .h(self.size.height)
            .flex()
            .flex_col()
            .overflow_hidden()
            .opacity(FLOAT_OPACITY)
            .bg(match edges.panel_bg {
                Some(color) => color.into(),
                None => theme.tokens.background.background,
            })
            .rounded(radius)
            .border_1()
            .border_color(overlay.frame)
            .shadow_lg()
            .child(
                div()
                    .flex()
                    .flex_none()
                    .items_end()
                    .h(strip_height)
                    .bg(theme.tab_bar)
                    .when(radius > px(0.), |this| this.rounded_t(radius))
                    .child(tab),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .child(self.panel.view()),
            )
    }
}

/// One of the five drop targets on Lace's overlay cross.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Area {
    Left,
    Right,
    Top,
    Bottom,
    Center,
}

impl Area {
    const ALL: [Area; 5] = [Area::Left, Area::Right, Area::Top, Area::Bottom, Area::Center];

    fn of(placement: Option<Placement>) -> Self {
        match placement {
            Some(Placement::Left) => Area::Left,
            Some(Placement::Right) => Area::Right,
            Some(Placement::Top) => Area::Top,
            Some(Placement::Bottom) => Area::Bottom,
            None => Area::Center,
        }
    }

    /// The target's place on the cross, in cells from the centre one.
    fn grid(self) -> (f32, f32) {
        match self {
            Area::Left => (-1., 0.),
            Area::Right => (1., 0.),
            Area::Top => (0., -1.),
            Area::Bottom => (0., 1.),
            Area::Center => (0., 0.),
        }
    }
}

/// A target's side.
const CELL: Pixels = px(30.);
/// Between targets.
const CELL_GAP: Pixels = px(4.);
/// The glyph box inside a target.
const GLYPH: Pixels = px(18.);

/// Where a target sits within a region of `size`: the cross is centred.
fn cell_origin(size: Size<Pixels>, area: Area) -> Point<Pixels> {
    let (gx, gy) = area.grid();
    let step = CELL + CELL_GAP;
    point(
        size.width * 0.5 - CELL * 0.5 + step * gx,
        size.height * 0.5 - CELL * 0.5 + step * gy,
    )
}

/// Whether a region of `size` has room for the cross with a margin around it.
fn cross_fits(size: Size<Pixels>) -> bool {
    let span = CELL * 3. + CELL_GAP * 2. + px(16.);
    size.width >= span && size.height >= span
}

/// The glyph on a target: a box with the part a drop there would take
/// filled in.
fn glyph(area: Area, color: Hsla) -> impl IntoElement {
    let half = (GLYPH - px(2.)) * 0.5;
    let fill = div().absolute().bg(color);
    let fill = match area {
        Area::Left => fill.left_0().top_0().bottom_0().w(half),
        Area::Right => fill.right_0().top_0().bottom_0().w(half),
        Area::Top => fill.left_0().right_0().top_0().h(half),
        Area::Bottom => fill.left_0().right_0().bottom_0().h(half),
        Area::Center => fill.left(px(2.)).right(px(2.)).top(px(2.)).bottom(px(2.)),
    };
    div()
        .relative()
        .size(GLYPH)
        .border_1()
        .border_color(color)
        .rounded(px(2.))
        .child(fill)
}

/// Lace's drop overlay for a hovered group: where the drop would land,
/// filled translucent inside a frame, and the target cross with the hovered
/// target lit. Positioned in the group's content region, which the dock
/// lays it over; it takes no pointer input, so the drop still reaches the
/// group under it.
pub fn drop_overlay(indicator: DropIndicator, cx: &App) -> AnyElement {
    let style = DropOverlayStyle::current(cx);
    let region = indicator.bounds().size;
    let to = indicator.to();
    let hovered = Area::of(indicator.placement());

    let target = div()
        .absolute()
        .left(to.origin().x)
        .top(to.origin().y)
        .w(to.size().width)
        .h(to.size().height)
        .bg(style.fill)
        .border_1()
        .border_color(style.frame)
        .rounded(px(3.));

    let cells = cross_fits(region).then(|| {
        Area::ALL.into_iter().map(move |area| {
            let origin = cell_origin(region, area);
            let lit = area == hovered;
            div()
                .absolute()
                .left(origin.x)
                .top(origin.y)
                .size(CELL)
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(4.))
                .border_1()
                .border_color(style.frame)
                .bg(if lit { style.frame } else { style.cell })
                .shadow_md()
                .child(glyph(area, if lit { style.cell } else { style.glyph }))
        })
    });

    div()
        .absolute()
        .left_0()
        .top_0()
        .w(region.width)
        .h(region.height)
        .child(target)
        .children(cells.into_iter().flatten())
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::gpui::size;

    #[test]
    fn the_cross_centres_on_the_region() {
        let region = size(px(400.), px(300.));
        let centre = cell_origin(region, Area::Center);
        assert_eq!(centre, point(px(185.), px(135.)));
        // Neighbours sit one cell and one gap away.
        assert_eq!(cell_origin(region, Area::Left), point(px(151.), px(135.)));
        assert_eq!(cell_origin(region, Area::Bottom), point(px(185.), px(169.)));
    }

    #[test]
    fn a_small_region_drops_the_cross_but_keeps_the_target() {
        assert!(cross_fits(size(px(200.), px(200.))));
        assert!(!cross_fits(size(px(90.), px(400.))));
    }

    #[test]
    fn placements_map_to_their_target() {
        assert_eq!(Area::of(None), Area::Center);
        assert_eq!(Area::of(Some(Placement::Left)), Area::Left);
        assert_eq!(Area::of(Some(Placement::Bottom)), Area::Bottom);
    }
}
