//! Geometric dock styling: rounded panels, gaps, outlines, panel glow and a
//! configurable splitter, built on the gpui-kit 0.8 renderer seam.
//!
//! This is an **application-side skin**, not a component-crate module. The
//! design is [`gpui-kit-geometric-dock-styling-spec.md`] ported to what
//! gpui-kit 0.8 can actually express; [`docs/gpui-kit-0.8-feasibility.md`]
//! records every place the implementation had to diverge. Two matter to a
//! caller:
//!
//! 1. **Effects are the group frame's own [`BoxShadow`].** gpui-kit 0.8 has no
//!    unclipped outer frame to host a separate effect layer: base takes the
//!    element `TabGroupRenderer::frame` returns and applies the column, fill
//!    and `overflow_hidden` to it. A shadow still reaches into the gap (GPUI
//!    paints drop shadows outside the element's own content mask) and is
//!    overpainted by the next sibling's chrome, so the visual guarantee holds;
//!    only the layer plumbing differs.
//! 2. **Split margins are horizontal.** `ResizablePanel` is a row in 0.8
//!    regardless of the split axis, and base forces the group frame to
//!    `size_full()` after the hook. Horizontal margins therefore shrink
//!    (flex-shrink), vertical ones overflow. Margins are applied to the
//!    left/right of a group whose parent is a horizontal split; vertical split
//!    seams keep the splitter paint and lose only the inset until gpui-base
//!    makes the panel direction follow the axis.
//!
//! Gaps exist at two levels. **Split seams** inside one region are inset by
//! group margins, so their vertical half is blocked by the panel direction.
//! **Dock regions** — left/right/centre and the bottom dock — are children of
//! the area row and the centre column; those frames are flex containers, so
//! their own `gap` creates both horizontal and vertical channels without
//! touching panel geometry. The same colour and width back both, and
//! [`DockGeometryStyle::gap_outer`] pads the area frame for the inset between
//! the docks and the window frame.
//!
//! Everything else is a direct port: rounded frames (F1), gap colour (F3),
//! splitter thickness (F4), panel outline (F5), shortened splitters (F7) and
//! rounded caps (F8). Defaults reproduce the stock gpui-kit skin: zero gap,
//! zero radius, no outline, no effect, and the stock animated hairline and pill
//! for an untouched splitter.
//!
//! ```ignore
//! use lace_gpui::geometry::{DockGeometryStyle, GeometryDockSkin};
//!
//! let style = DockGeometryStyle::new()
//!     .gap(px(8.))
//!     .panel_radius(px(10.))
//!     .glow(GlowSpec::new(cx.theme().colors.accent.alpha(0.30), px(22.)))
//!     .splitter(|s| s.thickness(px(6.)).length(SplitterLength::Percent(0.45)));
//!
//! let (area, skin) = GeometryDockSkin::dock_area("main", None, style, window, cx);
//! ```
//!
//! [`gpui-kit-geometric-dock-styling-spec.md`]: ../../gpui-kit-geometric-dock-styling-spec.md
//! [`docs/gpui-kit-0.8-feasibility.md`]: ../../docs/gpui-kit-0.8-feasibility.md

use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};

use crate::edge_tabs::{self, EdgeTabBar, PinTarget, StripOptions, TabEdgeStyle};

use gpui_kit::base::{
    dock::{
        DockArea, DockAreaRenderer, DockContext, DockEvent, DockPlacement, DropIndicator, NodeId,
        PaneNode,
        PaneRef, PanelId, PanelState, TabGroupContext, TabGroupRenderer,
    },
    ResizeHandleContext, ResizeHandleRenderer, ResizeHandleState,
};
use gpui_kit::component::dock::{
    BasePanelView, ClosePanel, DockSkin, PanelHandle, PanelStyle, ToggleZoom,
};
use gpui_kit::component::theme::ActiveTheme;
use gpui_kit::gpui::{
    canvas, deferred, div, point, prelude::FluentBuilder as _, px, relative, AnyElement, App,
    AppContext as _, Axis, BoxShadow, Div, Entity, Hsla, InteractiveElement as _, IntoElement,
    MouseButton, MouseMoveEvent, MouseUpEvent, ParentElement as _, Pixels, Point, SharedString,
    Stateful, Styled as _, WeakEntity, Window,
};

// ---------------------------------------------------------------------------
// The style value surface (spec chapter 6)
// ---------------------------------------------------------------------------

/// The whole geometric appearance, handed to the dock once.
///
/// Cheap to build and `Copy`; the skin keeps one and reads it while building
/// elements, so changing it and invalidating the area re-renders the new look.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DockGeometryStyle {
    pub gap: GapStyle,
    pub panel: PanelGeometryStyle,
    pub splitter: SplitterStyle,
}

impl DockGeometryStyle {
    pub fn new() -> Self {
        Self::default()
    }

    /// Visual gap between neighbouring panels and between dock regions, in
    /// pixels. A split seam insets each side by half of this; dock-region
    /// channels use the full value through the area and centre frames. The
    /// same width applies on both axes. Default `px(0.)`.
    pub fn gap(mut self, width: Pixels) -> Self {
        self.gap.width = width;
        self
    }

    /// Colour behind every gap. `None` takes the window background (Lace's
    /// `canvas_bg`), which is what Lace paints behind its splitters.
    pub fn gap_background(mut self, color: Hsla) -> Self {
        self.gap.background = Some(color);
        self
    }

    /// Breathing inset around the whole dock area, between the outer dock
    /// panels and the window frame. Applied as padding on the area frame, so
    /// the same gap colour backs it. Default `px(0.)`.
    pub fn gap_outer(mut self, inset: Pixels) -> Self {
        self.gap.outer = inset;
        self
    }

    /// Radius shared by a group's body and tab bar. Default `px(0.)`.
    pub fn panel_radius(mut self, radius: Pixels) -> Self {
        self.panel.corner_radius = radius;
        self
    }

    /// A border around the visible panel box. Default none.
    pub fn outline(mut self, outline: OutlineStyle) -> Self {
        self.panel.outline = Some(outline);
        self
    }

    /// A drop shadow in the gap. Independent of [`Self::glow`]; both may be on.
    pub fn shadow(mut self, shadow: ShadowSpec) -> Self {
        self.panel.effect.shadow = Some(shadow);
        self
    }

    /// A centred glow in the gap. Independent of [`Self::shadow`].
    pub fn glow(mut self, glow: GlowSpec) -> Self {
        self.panel.effect.glow = Some(glow);
        self
    }

    /// Configure the splitter without naming the sub-style.
    pub fn splitter(mut self, f: impl FnOnce(SplitterStyle) -> SplitterStyle) -> Self {
        self.splitter = f(self.splitter);
        self
    }
}

/// Gap geometry. A split uses half of `width` on each interior side; a
/// dock-region boundary uses the whole `width` as a flex gap; `outer` is the
/// padding between the outer docks and the window frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GapStyle {
    pub width: Pixels,
    pub background: Option<Hsla>,
    pub outer: Pixels,
}

impl Default for GapStyle {
    fn default() -> Self {
        Self {
            width: px(0.),
            background: None,
            outer: px(0.),
        }
    }
}

/// Geometry of one panel. Radius, outline and effect all land on the group
/// frame, which is also the element base clips.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct PanelGeometryStyle {
    pub corner_radius: Pixels,
    pub outline: Option<OutlineStyle>,
    pub effect: PanelEffect,
}

impl PanelGeometryStyle {
    pub fn radius(mut self, radius: Pixels) -> Self {
        self.corner_radius = radius;
        self
    }

    pub fn outline(mut self, outline: OutlineStyle) -> Self {
        self.outline = Some(outline);
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OutlineStyle {
    /// `None` resolves to the active theme's border colour at render time, so
    /// a theme switch does not leave a stale ring behind.
    pub color: Option<Hsla>,
    pub width: Pixels,
}

impl OutlineStyle {
    /// An explicit colour. It always wins over the theme.
    pub fn new(color: Hsla, width: Pixels) -> Self {
        Self {
            color: Some(color),
            width,
        }
    }

    /// A ring in the active theme's border colour and Lace border width
    /// (`CORE.border_width`); `width` is the fallback for a theme without one.
    pub fn themed(width: Pixels) -> Self {
        Self { color: None, width }
    }
}

/// Shadow and glow are independent and may both be active.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct PanelEffect {
    pub shadow: Option<ShadowSpec>,
    pub glow: Option<GlowSpec>,
}

impl PanelEffect {
    pub fn is_empty(&self) -> bool {
        self.shadow.is_none() && self.glow.is_none()
    }

    /// Shadow first, then glow, which is how the framework's dialogs compose a
    /// multi-layer stack.
    pub fn shadows(&self) -> Vec<BoxShadow> {
        let mut out = Vec::with_capacity(2);
        if let Some(shadow) = self.shadow {
            out.push(shadow.to_box_shadow());
        }
        if let Some(glow) = self.glow {
            out.push(glow.to_box_shadow());
        }
        out
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShadowSpec {
    pub color: Hsla,
    pub offset: Point<Pixels>,
    pub blur_radius: Pixels,
    pub spread_radius: Pixels,
}

impl ShadowSpec {
    pub fn new(color: Hsla, offset: Point<Pixels>, blur_radius: Pixels) -> Self {
        Self {
            color,
            offset,
            blur_radius,
            spread_radius: px(0.),
        }
    }

    pub fn spread(mut self, spread_radius: Pixels) -> Self {
        self.spread_radius = spread_radius;
        self
    }

    fn to_box_shadow(self) -> BoxShadow {
        BoxShadow {
            color: self.color,
            offset: self.offset,
            blur_radius: self.blur_radius,
            spread_radius: self.spread_radius,
            inset: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlowSpec {
    pub color: Hsla,
    pub blur_radius: Pixels,
    pub spread_radius: Pixels,
}

impl GlowSpec {
    pub fn new(color: Hsla, blur_radius: Pixels) -> Self {
        Self {
            color,
            blur_radius,
            spread_radius: px(0.),
        }
    }

    pub fn spread(mut self, spread_radius: Pixels) -> Self {
        self.spread_radius = spread_radius;
        self
    }

    fn to_box_shadow(self) -> BoxShadow {
        BoxShadow {
            color: self.color,
            offset: point(px(0.), px(0.)),
            blur_radius: self.blur_radius,
            spread_radius: self.spread_radius,
            inset: false,
        }
    }
}

/// Splitter paint. The hit band, cursor and drag remain base's; this only
/// changes what is drawn inside the band.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SplitterStyle {
    pub thickness: Pixels,
    pub length: SplitterLength,
    pub cap: SplitterCap,
    pub colors: SplitterColors,
}

impl Default for SplitterStyle {
    fn default() -> Self {
        Self {
            thickness: px(1.),
            length: SplitterLength::Percent(1.0),
            cap: SplitterCap::Flat,
            colors: SplitterColors::default(),
        }
    }
}

impl SplitterStyle {
    pub fn thickness(mut self, thickness: Pixels) -> Self {
        self.thickness = thickness;
        self
    }

    pub fn length(mut self, length: SplitterLength) -> Self {
        self.length = length;
        self
    }

    pub fn cap(mut self, cap: SplitterCap) -> Self {
        self.cap = cap;
        self
    }

    pub fn colors(mut self, colors: SplitterColors) -> Self {
        self.colors = colors;
        self
    }

    /// True when every field is at the stock value.
    ///
    /// The stock renderer animates a growing pill, which a fixed-length bar
    /// cannot reproduce exactly, so an untouched splitter delegates to it and
    /// keeps today's pixels and motion. The first explicit change opts into
    /// the geometric bar.
    pub fn is_default(&self) -> bool {
        self.thickness == px(1.)
            && matches!(self.length, SplitterLength::Percent(p) if (p - 1.0).abs() < f32::EPSILON)
            && self.cap == SplitterCap::Flat
            && self.colors == SplitterColors::default()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SplitterLength {
    /// A fixed length along the edge, centred on the seam.
    Pixels(Pixels),
    /// A fraction of the edge, `0.0..=1.0`, centred on the seam.
    Percent(f32),
    /// Pin the two ends: distances from the leading and trailing edge.
    Insets { start: Pixels, end: Pixels },
}

impl SplitterLength {
    /// Kept from the spec's spelling so examples read the same.
    pub fn pixels(pixels: Pixels) -> Self {
        Self::Pixels(pixels)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SplitterCap {
    Flat,
    /// Radius is derived as `thickness / 2`, so the cap always matches the bar.
    Rounded,
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct SplitterColors {
    pub idle: Option<Hsla>,
    pub hovered: Option<Hsla>,
    pub pressed: Option<Hsla>,
    pub dragging: Option<Hsla>,
}

impl SplitterColors {
    pub fn idle(mut self, color: Hsla) -> Self {
        self.idle = Some(color);
        self
    }

    pub fn hovered(mut self, color: Hsla) -> Self {
        self.hovered = Some(color);
        self
    }

    pub fn pressed(mut self, color: Hsla) -> Self {
        self.pressed = Some(color);
        self
    }

    pub fn dragging(mut self, color: Hsla) -> Self {
        self.dragging = Some(color);
        self
    }

    /// The colour for one pointer state.
    ///
    /// Unset non-idle states fall back to the pointer's previous level, then to
    /// the theme: idle is the border hairline colour, hover is muted text,
    /// press is body text and a drag is the accent. Explicit values always win.
    fn resolve(self, state: ResizeHandleState, cx: &App) -> Hsla {
        let theme = cx.theme();
        match state {
            ResizeHandleState::Idle => self.idle.unwrap_or(theme.border),
            ResizeHandleState::Hovered => {
                self.hovered.or(self.idle).unwrap_or(theme.muted_foreground)
            }
            ResizeHandleState::Pressed => self
                .pressed
                .or(self.hovered)
                .or(self.idle)
                .unwrap_or(theme.foreground),
            ResizeHandleState::Dragging => self
                .dragging
                .or(self.pressed)
                .or(self.hovered)
                .or(self.idle)
                .unwrap_or(theme.accent),
        }
    }
}

// ---------------------------------------------------------------------------
// Adjacency
// ---------------------------------------------------------------------------

/// Which sides of a group sit on an interior seam.
///
/// `top`/`bottom` are computed but not yet applied to split margins: gpui-kit
/// 0.8 cannot shrink a `size_full()` group on the cross axis (see the module
/// docs). Dock-region gaps do not use these sides; they come from the area and
/// centre frames' own flex gaps. Keeping them on the type means the split-margin
/// pass is a one-line change once base makes `ResizablePanel` follow its axis.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GroupSides {
    pub left: bool,
    pub right: bool,
    pub top: bool,
    pub bottom: bool,
}

impl GroupSides {
    /// The sides a child at `index` of a `count`-slot split owns.
    pub fn for_child(axis: Axis, index: usize, count: usize) -> Self {
        let has_before = index > 0;
        let has_after = index + 1 < count;
        match axis {
            Axis::Horizontal => Self {
                left: has_before,
                right: has_after,
                top: false,
                bottom: false,
            },
            Axis::Vertical => Self {
                left: false,
                right: false,
                top: has_before,
                bottom: has_after,
            },
        }
    }

    /// True when a vertical split would want a margin that 0.8 cannot honour.
    pub fn has_unapplied_vertical(&self) -> bool {
        self.top || self.bottom
    }
}

/// Which sides of `target` are interior in this tree, if it is here.
fn split_sides(node: &PaneNode, target: NodeId) -> Option<GroupSides> {
    match node.kind() {
        PaneRef::Tabs { .. } => (node.id() == target).then(GroupSides::default),
        PaneRef::Split { axis, children, .. } => {
            if let Some(index) = children.iter().position(|child| child.id() == target) {
                return Some(GroupSides::for_child(axis, index, children.len()));
            }
            children.iter().find_map(|child| split_sides(child, target))
        }
    }
}

// ---------------------------------------------------------------------------
// The skin
// ---------------------------------------------------------------------------

/// A dock skin wearing a [`DockGeometryStyle`].
///
/// It delegates the rest of the appearance — tab bar, content padding, dock
/// chrome, dock-edge resize handles, invalid-panel placeholder — to gpui-kit's
/// stock [`DockSkin`], so the geometric layer is a strict superset rather than
/// a reimplementation.
pub struct GeometryDockSkin {
    inner: Rc<DockSkin>,
    inner_group: Rc<dyn TabGroupRenderer>,
    stock_handle: ResizeHandleRenderer,
    area: WeakEntity<DockArea>,
    // Shared, not snapshotted: `tab_group_renderer()` is called once when a
    // group entity is created, so a value copy here would freeze the style for
    // every group already on screen. The `Rc` is what makes `set_style` reach
    // them through the normal invalidation path.
    style: Rc<RefCell<DockGeometryStyle>>,
    panel_effects: Rc<RefCell<HashMap<PanelId, PanelEffect>>>,
    // The stock skin keeps this write-only, so Lace's tab strip needs its own
    // copy to know whether to draw close buttons.
    close_visible: Rc<Cell<bool>>,
    // Where the strip's pin button sends a panel; set by `Sidebars`.
    pin: PinSlot,
    // The dock whose seam is being dragged, for the Lace dock edge.
    dock_resizing: Rc<Cell<Option<DockPlacement>>>,
}

/// The installed pin target, shared with every group's strip.
type PinSlot = Rc<RefCell<Option<Rc<dyn PinTarget>>>>;

impl GeometryDockSkin {
    /// Build the skin with the default (stock) appearance.
    ///
    /// Like [`DockSkin::new`], this must run inside the `DockArea`'s own
    /// constructor so the area's weak handle is available.
    pub fn new(cx: &mut gpui_kit::gpui::Context<DockArea>) -> Rc<Self> {
        Self::new_with_style(DockGeometryStyle::default(), cx)
    }

    /// Build the skin with a style already set.
    pub fn new_with_style(
        style: DockGeometryStyle,
        cx: &mut gpui_kit::gpui::Context<DockArea>,
    ) -> Rc<Self> {
        let inner = DockSkin::new(cx);
        let inner_group = inner.tab_group_renderer();
        Rc::new(Self {
            inner,
            inner_group,
            stock_handle: gpui_kit::component::resize_handle_appearance(),
            area: cx.weak_entity(),
            style: Rc::new(RefCell::new(style)),
            panel_effects: Rc::new(RefCell::new(HashMap::new())),
            close_visible: Rc::new(Cell::new(false)),
            pin: Rc::new(RefCell::new(None)),
            dock_resizing: Rc::new(Cell::new(None)),
        })
    }

    /// Install (or with `None` remove) where the title bar's pin button
    /// sends a panel. [`crate::sidebar::Sidebars`] calls this itself.
    pub fn set_pin_target(&self, target: Option<Rc<dyn PinTarget>>, cx: &mut App) {
        *self.pin.borrow_mut() = target;
        if let Some(area) = self.area.upgrade() {
            area.update(cx, |_, cx| cx.notify());
        }
    }

    /// The stock skin underneath, for its own settings (panel style, dock
    /// collapse affordances).
    pub fn dock_skin(&self) -> &Rc<DockSkin> {
        &self.inner
    }

    /// Show close buttons on closable tabs, in the stock strip and in
    /// Lace's edge strip alike. Hidden by default.
    pub fn set_close_button_visible(&self, visible: bool, cx: &mut App) {
        self.close_visible.set(visible);
        self.inner.set_close_button_visible(visible, cx);
    }

    /// Build a [`DockArea`] and its styling handle in one call, mirroring
    /// [`DockSkin::dock_area`].
    pub fn dock_area(
        id: impl Into<gpui_kit::gpui::SharedString>,
        version: Option<usize>,
        style: DockGeometryStyle,
        window: &mut Window,
        cx: &mut App,
    ) -> (Entity<DockArea>, Rc<Self>) {
        let mut skin = None;
        let area = cx.new(|cx| {
            let this = Self::new_with_style(style, cx);
            skin = Some(this.clone());
            DockArea::new(id, version, window, cx).with_renderer(this)
        });
        (
            area,
            skin.expect("skin installed while constructing the area"),
        )
    }

    /// The current style.
    pub fn style(&self) -> DockGeometryStyle {
        *self.style.borrow()
    }

    /// Replace the style and repaint the area.
    pub fn set_style(&self, style: DockGeometryStyle, cx: &mut App) {
        *self.style.borrow_mut() = style;
        self.notify(cx);
    }

    /// Give one panel its own effect. The rendered surface is still the tab
    /// group frame, so this applies while that panel is the visible tab; a
    /// group showing several tabs cannot give each a different effect at once.
    pub fn set_panel_effect(&self, panel: PanelId, effect: PanelEffect, cx: &mut App) {
        self.panel_effects.borrow_mut().insert(panel, effect);
        self.notify(cx);
    }

    /// Drop a panel override, returning it to the dock-wide effect.
    pub fn clear_panel_effect(&self, panel: PanelId, cx: &mut App) {
        self.panel_effects.borrow_mut().remove(&panel);
        self.notify(cx);
    }

    fn notify(&self, cx: &mut App) {
        if let Some(area) = self.area.upgrade() {
            area.update(cx, |_, cx| cx.notify());
        }
    }
}

impl DockAreaRenderer for GeometryDockSkin {
    fn frame(&self, window: &mut Window, cx: &mut App) -> Stateful<Div> {
        // `gap.outer` is padding on the area frame: base applies `size_full`,
        // `flex_row` and `overflow_hidden` around this element, so padding
        // shrinks the content box without fighting the layout. The frame is
        // also the row that holds left/center/right, so its own flex gap is
        // the inter-dock channel; painting it with the gap colour backs both
        // that channel and the outer padding.
        let style = *self.style.borrow();
        let frame = self
            .inner
            .frame(window, cx)
            .gap(dock_channel(&style))
            .p(style.gap.outer);
        match style.gap.background {
            Some(color) => frame.bg(color),
            None => frame.bg(cx.theme().tokens.background),
        }
    }

    fn split_frame(
        &self,
        node: NodeId,
        axis: Axis,
        window: &mut Window,
        cx: &mut App,
    ) -> Stateful<Div> {
        let background = self.style.borrow().gap.background;
        let frame = self.inner.split_frame(node, axis, window, cx);
        match background {
            Some(color) => frame.bg(color),
            None => frame.bg(cx.theme().tokens.background),
        }
    }

    fn center_frame(&self, window: &mut Window, cx: &mut App) -> Stateful<Div> {
        // Base lays this out as a column: the centre tree, then the bottom
        // dock. A flex gap here is the bottom-dock channel; the centre tree
        // absorbs it because it carries `flex_1`.
        let style = *self.style.borrow();
        let frame = self.inner.center_frame(window, cx).gap(dock_channel(&style));
        match style.gap.background {
            Some(color) => frame.bg(color),
            None => frame.bg(cx.theme().tokens.background),
        }
    }

    fn render_split_handle(
        &self,
        handle: &ResizeHandleContext,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement> {
        let splitter = self.style.borrow().splitter;
        if splitter.is_default() {
            // Keep the stock animated hairline/pill: a fixed-length bar cannot
            // reproduce its per-state growth, and an app that never touches
            // the splitter should see exactly today's dock.
            let stock = self.stock_handle.clone();
            return stock(handle, window, cx);
        }

        let axis = handle.axis();
        let thickness = splitter.thickness.max(px(1.));
        let color = splitter.colors.resolve(handle.state(), cx);

        let visual = div()
            .flex_none()
            .bg(color)
            .when(splitter.cap == SplitterCap::Rounded, |bar| {
                bar.rounded(thickness * 0.5)
            })
            .map(|bar| match (axis, splitter.length) {
                (Axis::Horizontal, SplitterLength::Pixels(length)) => bar
                    .w(thickness)
                    .h(length.max(thickness))
                    .ml((thickness - px(1.)) * -0.5),
                (Axis::Vertical, SplitterLength::Pixels(length)) => bar
                    .h(thickness)
                    .w(length.max(thickness))
                    .mt((thickness - px(1.)) * -0.5),
                (Axis::Horizontal, SplitterLength::Percent(fraction)) => bar
                    .w(thickness)
                    .h(relative(fraction.clamp(0.0, 1.0)))
                    .ml((thickness - px(1.)) * -0.5),
                (Axis::Vertical, SplitterLength::Percent(fraction)) => bar
                    .h(thickness)
                    .w(relative(fraction.clamp(0.0, 1.0)))
                    .mt((thickness - px(1.)) * -0.5),
                // Insets are spacers on the full-length wrapper; the bar itself
                // fills what is left, so no measurement is involved.
                (Axis::Horizontal, SplitterLength::Insets { .. }) => {
                    bar.w(thickness).h_full().ml((thickness - px(1.)) * -0.5)
                }
                (Axis::Vertical, SplitterLength::Insets { .. }) => {
                    bar.h(thickness).w_full().mt((thickness - px(1.)) * -0.5)
                }
            })
            // A degenerate configuration must still leave something to grab
            // visually, so the bar is never shorter than it is thick.
            .map(|bar| match axis {
                Axis::Horizontal => bar.min_h(thickness),
                Axis::Vertical => bar.min_w(thickness),
            });

        let mut wrapper = div().flex_none().flex();
        wrapper = match axis {
            Axis::Horizontal => wrapper.w(px(1.)).h_full().items_center(),
            Axis::Vertical => wrapper.h(px(1.)).w_full().justify_center(),
        };
        if let SplitterLength::Insets { start, end } = splitter.length {
            wrapper = match axis {
                Axis::Horizontal => wrapper.pt(start).pb(end),
                Axis::Vertical => wrapper.pl(start).pr(end),
            };
        }

        // A hugging handle's line is the container's own outermost pixel, so an
        // overhang needs deferral to survive the dock's clip — the same rule
        // and the same technique as the stock pill.
        let visual = match handle.edge() {
            Some(_) => deferred(visual).into_any_element(),
            None => visual.into_any_element(),
        };

        Some(wrapper.child(visual).into_any_element())
    }

    fn render_dock(
        &self,
        dock: &DockContext,
        content: AnyElement,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let style = *self.style.borrow();
        if style.splitter.is_default() {
            // The stock edge: gpui-component's hairline and its own tracker.
            return self.inner.render_dock(dock, content, window, cx);
        }

        // Lace's dock edge. The seam between this dock and the centre is
        // the dock's own padding (see `dock_channel`), so the whole seam is
        // the grab strip, with the same bar the splits wear centred in it.
        // gpui-component's dock handle cannot do this: its band is a fixed
        // few pixels hugging the dock edge, and its bar is the stock pill.
        let placement = dock.placement();
        let seam = style.gap.width;
        let splitter = style.splitter;
        let resizing = self.dock_resizing.clone();
        let dragging = resizing.get() == Some(placement);
        let axis = match placement {
            DockPlacement::Bottom => Axis::Vertical,
            _ => Axis::Horizontal,
        };
        let group = SharedString::from(format!("lace-dock-seam:{placement:?}"));
        let idle = splitter.colors.resolve(ResizeHandleState::Idle, cx);
        let hovered = splitter.colors.resolve(ResizeHandleState::Hovered, cx);
        let active = splitter.colors.resolve(ResizeHandleState::Dragging, cx);

        let bar = seam_bar(splitter, axis)
            .bg(if dragging { active } else { idle })
            .when(!dragging, |bar| bar.group_hover(group.clone(), |s| s.bg(hovered)));

        let strip = div()
            .id(SharedString::from(format!("lace-dock-seam-strip:{placement:?}")))
            .group(group)
            .absolute()
            .flex()
            .items_center()
            .justify_center()
            .map(|strip| match placement {
                DockPlacement::Left => strip.top_0().bottom_0().right_0().w(seam),
                DockPlacement::Right => strip.top_0().bottom_0().left_0().w(seam),
                _ => strip.left_0().right_0().top_0().h(seam),
            })
            .map(|strip| match axis {
                Axis::Horizontal => strip.cursor_col_resize(),
                Axis::Vertical => strip.cursor_row_resize(),
            })
            .on_mouse_down(MouseButton::Left, {
                let resizing = resizing.clone();
                move |_, window, cx| {
                    resizing.set(Some(placement));
                    window.refresh();
                    cx.stop_propagation();
                }
            })
            .child(bar);

        // A drag runs on window-level events: the pointer leaves the strip
        // within a pixel or two.
        let tracker = canvas(|_, _, _| {}, {
            let dock = dock.clone();
            let area = self.area.clone();
            move |_, _, window, _| {
                window.on_mouse_event({
                    let dock = dock.clone();
                    let resizing = resizing.clone();
                    move |event: &MouseMoveEvent, phase, window, cx| {
                        if phase.bubble() && resizing.get() == Some(placement) {
                            dock.resize_to(event.position, window, cx);
                        }
                    }
                });
                window.on_mouse_event(move |_: &MouseUpEvent, phase, window, cx| {
                    if !phase.bubble() || resizing.get() != Some(placement) {
                        return;
                    }
                    resizing.set(None);
                    dock.end_resize(window, cx);
                    _ = area.update(cx, |_, cx| cx.emit(DockEvent::LayoutChanged));
                    window.refresh();
                });
            }
        })
        .absolute()
        .size_0();

        div()
            .flex()
            .size_full()
            .relative()
            .map(|this| match placement {
                DockPlacement::Left => this.pr(seam),
                DockPlacement::Right => this.pl(seam),
                _ => this.pt(seam),
            })
            .child(content)
            .child(strip)
            .child(tracker)
            .into_any_element()
    }

    fn build_placeholder(
        &self,
        state: &PanelState,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<std::sync::Arc<dyn BasePanelView>> {
        self.inner.build_placeholder(state, window, cx)
    }

    fn tab_group_renderer(&self) -> Rc<dyn TabGroupRenderer> {
        Rc::new(GeometryTabGroup {
            inner: self.inner_group.clone(),
            dock: self.inner.clone(),
            strip: EdgeTabBar::default(),
            close_visible: self.close_visible.clone(),
            pin: self.pin.clone(),
            area: self.area.clone(),
            style: self.style.clone(),
            panel_effects: self.panel_effects.clone(),
        })
    }
}

/// The flex gap between the docks and the centre. The stock dock edge
/// leaves the seam to this gap; Lace's edge carries it inside each dock (as
/// padding its handle can cover), so the frames add none.
fn dock_channel(style: &DockGeometryStyle) -> Pixels {
    if style.splitter.is_default() {
        style.gap.width
    } else {
        px(0.)
    }
}

/// A splitter bar of the configured thickness, length and cap, for a seam
/// strip that centres it. `axis` is the resize axis: `Horizontal` for a
/// vertical bar between side-by-side regions.
fn seam_bar(splitter: SplitterStyle, axis: Axis) -> Div {
    let thickness = splitter.thickness.max(px(1.));
    let bar = div()
        .flex_none()
        .when(splitter.cap == SplitterCap::Rounded, |bar| bar.rounded(thickness * 0.5));
    let bar = match (axis, splitter.length) {
        (Axis::Horizontal, SplitterLength::Pixels(length)) => bar.w(thickness).h(length),
        (Axis::Vertical, SplitterLength::Pixels(length)) => bar.h(thickness).w(length),
        (Axis::Horizontal, SplitterLength::Percent(f)) => {
            bar.w(thickness).h(relative(f.clamp(0.0, 1.0)))
        }
        (Axis::Vertical, SplitterLength::Percent(f)) => {
            bar.h(thickness).w(relative(f.clamp(0.0, 1.0)))
        }
        (Axis::Horizontal, SplitterLength::Insets { start, end }) => {
            bar.w(thickness).h_full().mt(start).mb(end)
        }
        (Axis::Vertical, SplitterLength::Insets { start, end }) => {
            bar.h(thickness).w_full().ml(start).mr(end)
        }
    };
    match axis {
        Axis::Horizontal => bar.min_h(thickness),
        Axis::Vertical => bar.min_w(thickness),
    }
}

/// The tab-group half of the skin. State is shared through `Rc`, so a style
/// changed after construction is seen by the next frame.
struct GeometryTabGroup {
    inner: Rc<dyn TabGroupRenderer>,
    dock: Rc<DockSkin>,
    strip: EdgeTabBar,
    close_visible: Rc<Cell<bool>>,
    pin: PinSlot,
    area: WeakEntity<DockArea>,
    style: Rc<RefCell<DockGeometryStyle>>,
    panel_effects: Rc<RefCell<HashMap<PanelId, PanelEffect>>>,
}

impl GeometryTabGroup {
    fn sides_for(&self, node: NodeId, cx: &App) -> GroupSides {
        let Some(area) = self.area.upgrade() else {
            return GroupSides::default();
        };
        let area = area.read(cx);
        for placement in [
            DockPlacement::Center,
            DockPlacement::Left,
            DockPlacement::Right,
            DockPlacement::Bottom,
        ] {
            let Some(tree) = area.layout(placement) else {
                continue;
            };
            if let Some(sides) = split_sides(tree.root(), node) {
                return sides;
            }
        }
        GroupSides::default()
    }

    fn effect_for(&self, group: &TabGroupContext, cx: &App) -> PanelEffect {
        group
            .active_panel()
            .map(|panel| panel.panel_id(cx))
            .and_then(|id| self.panel_effects.borrow().get(&id).copied())
            .unwrap_or(self.style.borrow().panel.effect)
    }
}

impl TabGroupRenderer for GeometryTabGroup {
    fn frame(&self, group: &TabGroupContext, window: &mut Window, cx: &mut App) -> Stateful<Div> {
        let style = *self.style.borrow();
        let edges = TabEdgeStyle::current(cx);
        let focused = edge_tabs::group_focused(group, window, cx);
        // Under `below_title` the content frame draws the outline instead,
        // starting under the strip. Otherwise the focused group's outline
        // takes the focus colour, as Lace's `paint_panel_border` does.
        let outline = if edges
            .below_title_frame(focused, cx.theme().tab_active)
            .is_some()
        {
            None
        } else {
            style.panel.outline.map(|mut outline| {
                if outline.color.is_none() && edges.frame_width > px(0.) {
                    outline.width = edges.frame_width;
                }
                if focused && edges.frame_focus.a > 0. {
                    outline.color = Some(edges.frame_focus);
                }
                outline
            })
        };
        let effect = self.effect_for(group, cx);
        let sides = self.sides_for(group.node(), cx);

        // Keep the stock action handlers: keyboard zoom out is always allowed,
        // keyboard zoom in only when the active panel offers a control, and
        // close acts on the active panel.
        let zoom_control = group
            .active_panel()
            .filter(|panel| panel.zoomable(cx))
            .and_then(|panel| PanelHandle::of(panel))
            .and_then(|handle| handle.zoom_control(cx));

        let half = style.gap.width * 0.5;
        let shadows = effect.shadows();

        div()
            .id(("lace-tab-panel", group.node().as_u64()))
            .bg(cx.theme().tokens.background)
            // A collapsed group is a strip of tabs with no content; the
            // actions act on content, so gate them the same way the stock does.
            .when(!group.is_collapsed(), |this| {
                this.on_action({
                    let group = group.clone();
                    move |_: &ToggleZoom, window, cx| {
                        if !group.is_zoomed() && zoom_control.is_none() {
                            return;
                        }
                        group.toggle_zoom(window, cx);
                    }
                })
                .on_action({
                    let group = group.clone();
                    move |_: &ClosePanel, window, cx| {
                        let Some(panel) = group.active_panel() else {
                            return;
                        };
                        let panel = panel.panel_id(cx);
                        group.close(panel, window, cx);
                    }
                })
            })
            .when(style.panel.corner_radius > px(0.), |this| {
                this.rounded(style.panel.corner_radius)
            })
            .when_some(outline, |this, outline| {
                this.border(outline.width)
                    .border_color(outline.color.unwrap_or(cx.theme().border))
            })
            .when(!shadows.is_empty(), |this| this.shadow(shadows))
            // Horizontal margins shrink inside a row panel. Vertical margins
            // would overflow a `size_full()` group in 0.8, so `top`/`bottom`
            // stay computed but unapplied until the panel axis fix lands.
            .when(sides.left, |this| this.ml(half))
            .when(sides.right, |this| this.mr(half))
    }

    fn content_frame(
        &self,
        group: &TabGroupContext,
        window: &mut Window,
        cx: &mut App,
    ) -> Stateful<Div> {
        let edges = TabEdgeStyle::current(cx);
        let focused = edge_tabs::group_focused(group, window, cx);
        let frame = edges.below_title_frame(focused, cx.theme().tab_active);
        let content = self.inner.content_frame(group, window, cx);
        // Left, right and bottom: the strip's rule is the fourth side.
        match frame {
            Some(color) if !group.is_collapsed() => content
                .border_l(edges.frame_width)
                .border_r(edges.frame_width)
                .border_b(edges.frame_width)
                .border_color(color)
                .rounded_b(edges.frame_radius),
            _ => content,
        }
    }

    fn render_tab_bar(
        &self,
        group: &TabGroupContext,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let edges = TabEdgeStyle::current(cx);
        // The pin button lives in Lace's strip, so installed sidebars take
        // every theme onto it, the classic flat ones included.
        let pin = self.pin.borrow().clone();
        let lace_strip = pin.is_some();
        if !edges.draws_tabs() && !edges.has_rule() && !lace_strip {
            return self.inner.render_tab_bar(group, window, cx);
        }

        let visible: Vec<usize> = group
            .panels()
            .iter()
            .enumerate()
            .filter(|(_, panel)| panel.visible(cx))
            .map(|(ix, _)| ix)
            .collect();
        // The cases the stock skin draws as a plain title (or nothing): keep
        // them, and lay the rule under a title that is actually drawn.
        let title_only = match visible.as_slice() {
            [] => return self.inner.render_tab_bar(group, window, cx),
            [ix] if self.dock.panel_style() == PanelStyle::Auto => {
                let panel = &group.panels()[*ix];
                if PanelHandle::of(panel).is_some_and(|handle| !handle.title_bar(cx)) {
                    return self.inner.render_tab_bar(group, window, cx);
                }
                !lace_strip
            }
            _ => false,
        };

        if title_only || (!edges.draws_tabs() && !lace_strip) {
            let focused = edge_tabs::group_focused(group, window, cx);
            let color = edges.rule_for(focused, cx.theme().tab_active);
            let stock = self.inner.render_tab_bar(group, window, cx);
            return div()
                .relative()
                .flex()
                .flex_col()
                .flex_none()
                .child(stock)
                .when(edges.has_rule(), |this| {
                    this.child(edge_tabs::rule_line(edges.rule, color))
                })
                .into_any_element();
        }

        self.strip.render(
            group,
            edges,
            StripOptions {
                area: &self.area,
                toggles_visible: self.dock.is_toggle_button_visible(),
                close_visible: self.close_visible.get(),
                pin,
            },
            window,
            cx,
        )
    }

    fn render_active_panel(
        &self,
        panel: gpui_kit::gpui::AnyView,
        group: &TabGroupContext,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        self.inner.render_active_panel(panel, group, window, cx)
    }

    fn render_drop_indicator(
        &self,
        indicator: DropIndicator,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement> {
        self.inner.render_drop_indicator(indicator, window, cx)
    }

    fn render_empty(
        &self,
        group: &TabGroupContext,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement> {
        self.inner.render_empty(group, window, cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::gpui::{hsla, px};

    #[test]
    fn defaults_are_the_stock_look() {
        let style = DockGeometryStyle::default();
        assert_eq!(style.gap.width, px(0.));
        assert_eq!(style.gap.outer, px(0.));
        assert!(style.gap.background.is_none());
        assert_eq!(style.panel.corner_radius, px(0.));
        assert!(style.panel.outline.is_none());
        assert!(style.panel.effect.is_empty());
        assert!(style.splitter.is_default());
    }

    #[test]
    fn splitter_leaves_default_on_any_explicit_change() {
        let base = SplitterStyle::default();
        assert!(base.is_default());
        assert!(!base.thickness(px(6.)).is_default());
        assert!(!base.length(SplitterLength::Percent(0.45)).is_default());
        assert!(!base.cap(SplitterCap::Rounded).is_default());
        assert!(!base
            .colors(SplitterColors::default().idle(hsla(0.0, 0.0, 1.0, 1.0)))
            .is_default());
    }

    #[test]
    fn side_ownership_follows_the_parent_axis() {
        assert_eq!(
            GroupSides::for_child(Axis::Horizontal, 0, 3),
            GroupSides {
                left: false,
                right: true,
                top: false,
                bottom: false
            }
        );
        assert_eq!(
            GroupSides::for_child(Axis::Horizontal, 1, 3),
            GroupSides {
                left: true,
                right: true,
                top: false,
                bottom: false
            }
        );
        assert_eq!(
            GroupSides::for_child(Axis::Horizontal, 2, 3),
            GroupSides {
                left: true,
                right: false,
                top: false,
                bottom: false
            }
        );
        assert_eq!(
            GroupSides::for_child(Axis::Vertical, 1, 3),
            GroupSides {
                left: false,
                right: false,
                top: true,
                bottom: true
            }
        );
    }

    #[test]
    fn effect_stack_is_shadow_then_glow() {
        let shadow = ShadowSpec::new(hsla(0.0, 0.0, 0.0, 0.5), point(px(0.), px(3.)), px(12.));
        let glow = GlowSpec::new(hsla(0.5, 1.0, 0.6, 0.3), px(22.));
        let effect = PanelEffect {
            shadow: Some(shadow),
            glow: Some(glow),
        };
        let shadows = effect.shadows();
        assert_eq!(shadows.len(), 2);
        assert_eq!(shadows[0].color, shadow.color);
        assert_eq!(shadows[0].offset, point(px(0.), px(3.)));
        assert_eq!(shadows[1].color, glow.color);
        assert_eq!(shadows[1].offset, point(px(0.), px(0.)));
        assert!(!shadows[0].inset && !shadows[1].inset);
    }

    #[test]
    fn empty_effect_has_no_shadows() {
        assert!(PanelEffect::default().shadows().is_empty());
    }
}
