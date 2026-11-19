//! `ItemView`: the tree / table / list view over an [`ItemModel`].
//!
//! ```ignore
//! let model = cx.new(|_| ItemModel::table(["Name", "Qty", "Kind"], rows));
//! let view = cx.new(|cx| {
//!     ItemView::new(&model, Mode::Table, cx)
//!         .column(0, Column::stretch())
//!         .column(1, Column::fixed(80.).delegate(Delegate::number(0., 100., 1., 0)))
//!         .column(2, Column::contents().delegate(Delegate::choice(["low", "high"])))
//! });
//! ```
//!
//! Rows are virtualised (`uniform_list`), so a view holds hundreds of
//! thousands of rows; only visible rows are built each frame.

use std::collections::BTreeSet;
use std::ops::Range;
use std::rc::Rc;

use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::AppContext;
use gpui_kit::gpui::{
    canvas, div, point, ClickEvent, prelude::FluentBuilder, px, uniform_list, AnyElement, App, Context,
    DispatchPhase, ElementId, Entity, EventEmitter, FocusHandle, InteractiveElement, IntoElement,
    KeyDownEvent, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, Render,
    ScrollStrategy, ScrollWheelEvent, SharedString, StatefulInteractiveElement, Styled, Subscription,
    TextRun, UniformListScrollHandle, Window,
};

use super::model::{ItemModel, ModelEvent, NodeId, Value};
use crate::controls::{
    check_indicator, header_section, item_row_div, radio_indicator, ComboBox, DynamicRange, DynamicSlider,
    Interaction, LineEdit, Margin, ProgressBar, PushButton, Scrollbar, Slider, Sort, SpinBox, Switch,
};
use crate::glyph::{self, Dir};
use crate::style::rules;
use crate::style::{hsla, ActiveStyle, LaceStyle, State};

/// Indent per tree level, px.
pub const INDENT: f32 = 20.0;
/// Width of the expand / collapse chevron, px.
pub const BRANCH: f32 = 16.0;
/// Cell padding, px.
pub const CELL_PAD: f32 = 4.0;
/// Width of a resize grip at a header section's right edge, px.
const GRIP: f32 = 6.0;
/// Smallest column a drag leaves, px.
const MIN_COLUMN: f32 = 24.0;

/// How the view lays out rows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    /// Indented rows with expand chevrons (`QTreeView`).
    #[default]
    Tree,
    /// Flat rows with grid lines (`QTableView`).
    Table,
    /// Flat rows, one column, no header (`QListView`).
    List,
}

/// How a column takes its width (`QHeaderView::ResizeMode`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Resize {
    /// Fixed width.
    Fixed(f32),
    /// Starts at the width; the user drags the header edge.
    Interactive(f32),
    /// Shares the space the other columns leave.
    Stretch,
    /// Fits the header and cells.
    Contents,
}

/// Selection behaviour (`QAbstractItemView::SelectionMode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SelectionMode {
    /// No selection.
    None,
    /// One row.
    #[default]
    Single,
    /// Ctrl toggles, Shift extends.
    Extended,
}

/// Horizontal alignment of a column's cells.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Align {
    /// Numbers right, everything else left.
    #[default]
    Auto,
    /// Left.
    Left,
    /// Centre.
    Center,
    /// Right.
    Right,
}

/// What a custom delegate gets to paint a cell.
pub struct CellCtx<'a> {
    /// Row.
    pub node: NodeId,
    /// Column.
    pub column: usize,
    /// The cell.
    pub value: &'a Value,
    /// The row is selected.
    pub selected: bool,
    /// The row is enabled.
    pub enabled: bool,
    /// The style.
    pub style: &'a LaceStyle,
}

type CustomCell = Rc<dyn Fn(&CellCtx, &mut Window, &mut App) -> AnyElement>;
type RowAction = Rc<dyn Fn(NodeId, &mut Window, &mut App)>;

/// How a column shows and edits its cells (`QStyledItemDelegate`).
#[derive(Clone, Default)]
pub enum Delegate {
    /// Text; edited in a line edit.
    #[default]
    Text,
    /// Text, never edited.
    ReadOnly,
    /// A number; edited with a spin box.
    Number {
        /// Minimum.
        min: f64,
        /// Maximum.
        max: f64,
        /// Step.
        step: f64,
        /// Decimals shown.
        decimals: usize,
    },
    /// One of `choices`; edited with a combo box.
    Choice(Rc<[SharedString]>),
    /// A flag shown as a check box, toggled in place.
    Check,
    /// A flag shown as a switch, toggled in place.
    Switch,
    /// A flag shown as a checkable push button labelled `label`, toggled in
    /// place.
    Toggle(SharedString),
    /// One flag per sibling group shown as radio buttons: choosing a row
    /// sets it and clears the column in the rows that share its parent.
    Radio,
    /// A number dragged on a slider in place, its value beside it.
    Slider {
        /// Minimum.
        min: f64,
        /// Maximum.
        max: f64,
        /// Step.
        step: f64,
        /// Decimals shown.
        decimals: usize,
    },
    /// A number dragged on a [`DynamicSlider`] in place: its limits follow
    /// the value (±50 % by default).
    DynamicSlider(DynamicRange),
    /// A progress bar over `0..=max`.
    Progress(f64),
    /// A button in every row, labelled `label`.
    Button(SharedString, RowAction),
    /// Anything: the closure builds the cell.
    Custom(CustomCell),
}

impl Delegate {
    /// A number editor.
    pub fn number(min: f64, max: f64, step: f64, decimals: usize) -> Self {
        Delegate::Number { min, max, step, decimals }
    }

    /// A choice editor.
    pub fn choice(choices: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        Delegate::Choice(choices.into_iter().map(Into::into).collect())
    }

    /// A toggle button labelled `label`.
    pub fn toggle(label: impl Into<SharedString>) -> Self {
        Delegate::Toggle(label.into())
    }

    /// A slider over `min..=max`.
    pub fn slider(min: f64, max: f64, step: f64, decimals: usize) -> Self {
        Delegate::Slider { min, max, step, decimals }
    }

    /// A slider whose limits follow the value.
    pub fn dynamic_slider(range: DynamicRange) -> Self {
        Delegate::DynamicSlider(range)
    }

    /// A button.
    pub fn button(label: impl Into<SharedString>, f: impl Fn(NodeId, &mut Window, &mut App) + 'static) -> Self {
        Delegate::Button(label.into(), Rc::new(f))
    }

    /// A custom cell.
    pub fn custom(f: impl Fn(&CellCtx, &mut Window, &mut App) -> AnyElement + 'static) -> Self {
        Delegate::Custom(Rc::new(f))
    }

    fn editable(&self) -> bool {
        matches!(self, Delegate::Text | Delegate::Number { .. } | Delegate::Choice(_))
    }
}

/// A column's layout and delegate.
#[derive(Clone)]
pub struct Column {
    /// Width rule.
    pub resize: Resize,
    /// Delegate.
    pub delegate: Delegate,
    /// Alignment.
    pub align: Align,
    /// Margin of the editor or button delegate inside the cell; `None`
    /// takes the view's (see [`ItemView::delegate_margins`]).
    pub margin: Option<Margin>,
}

impl Default for Column {
    fn default() -> Self {
        Self { resize: Resize::Interactive(120.0), delegate: Delegate::Text, align: Align::Auto, margin: None }
    }
}

impl Column {
    /// A user-resizable column, `w` px to start.
    pub fn interactive(w: f32) -> Self {
        Self { resize: Resize::Interactive(w), ..Default::default() }
    }
    /// A fixed column.
    pub fn fixed(w: f32) -> Self {
        Self { resize: Resize::Fixed(w), ..Default::default() }
    }
    /// A stretching column.
    pub fn stretch() -> Self {
        Self { resize: Resize::Stretch, ..Default::default() }
    }
    /// A column fitted to its contents.
    pub fn contents() -> Self {
        Self { resize: Resize::Contents, ..Default::default() }
    }
    /// With `delegate`.
    pub fn delegate(mut self, delegate: Delegate) -> Self {
        self.delegate = delegate;
        self
    }
    /// With `align`.
    pub fn align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }
    /// The editor / button delegate keeps `margin` from the cell edges:
    /// one number for all sides, `(h, v)`, or `[left, top, right, bottom]`.
    pub fn margin(mut self, margin: impl Into<Margin>) -> Self {
        self.margin = Some(margin.into());
        self
    }
}

/// What a view tells its owner.
#[derive(Clone, Debug, PartialEq)]
pub enum ViewEvent {
    /// The selection changed.
    SelectionChanged,
    /// The current row changed.
    CurrentChanged(Option<NodeId>),
    /// A row was activated (double-click, or Enter on a row with nothing to edit).
    Activated(NodeId),
}

#[derive(Clone)]
struct RowData {
    id: NodeId,
    ix: usize,
    depth: usize,
    cells: Vec<Value>,
    check: Option<crate::controls::CheckState>,
    has_children: bool,
    expanded: bool,
    enabled: bool,
    editable: bool,
}

/// The view.
pub struct ItemView {
    model: Entity<ItemModel>,
    mode: Mode,
    columns: Vec<Column>,
    user_widths: Vec<Option<f32>>,
    widths: Vec<f32>,
    content_widths: (u64, Vec<f32>),
    rows: Vec<(NodeId, usize)>,
    rows_rev: Option<u64>,
    selection: BTreeSet<NodeId>,
    current: Option<NodeId>,
    anchor: Option<usize>,
    selection_mode: SelectionMode,
    sort: Option<(usize, bool)>,
    sorting: bool,
    scroll: UniformListScrollHandle,
    hscroll: f32,
    viewport: (f32, f32),
    editing: Option<(NodeId, usize)>,
    editor: Option<Entity<InputState>>,
    editor_sub: Option<Subscription>,
    resizing: Option<(usize, f32, f32)>,
    hovered: Option<NodeId>,
    focus: FocusHandle,
    row_height: f32,
    alternating: bool,
    header: bool,
    grid: bool,
    root_decorated: bool,
    radius: Option<f32>,
    bar_width: Option<f32>,
    overlay: Option<bool>,
    editor_margin: Margin,
    button_margin: Margin,
    id: ElementId,
    _observe: Subscription,
}

impl EventEmitter<ViewEvent> for ItemView {}

impl ItemView {
    /// A view over `model`.
    pub fn new(model: &Entity<ItemModel>, mode: Mode, cx: &mut Context<Self>) -> Self {
        let observe = cx.observe(model, |_, _, cx| cx.notify());
        let id: ElementId = SharedString::from(format!("item-view-{}", cx.entity_id())).into();
        Self {
            model: model.clone(),
            mode,
            columns: Vec::new(),
            user_widths: Vec::new(),
            widths: Vec::new(),
            content_widths: (u64::MAX, Vec::new()),
            rows: Vec::new(),
            rows_rev: None,
            selection: BTreeSet::new(),
            current: None,
            anchor: None,
            selection_mode: SelectionMode::Single,
            sort: None,
            sorting: false,
            scroll: UniformListScrollHandle::new(),
            hscroll: 0.0,
            viewport: (0.0, 0.0),
            editing: None,
            editor: None,
            editor_sub: None,
            resizing: None,
            hovered: None,
            focus: cx.focus_handle(),
            row_height: if mode == Mode::Table { 26.0 } else { 22.0 },
            alternating: mode == Mode::Table,
            header: mode != Mode::List,
            grid: mode == Mode::Table,
            root_decorated: true,
            radius: None,
            bar_width: None,
            overlay: None,
            editor_margin: Margin::all(1.0),
            button_margin: Margin::new(4.0, 2.0, 4.0, 2.0),
            id,
            _observe: observe,
        }
    }

    /// Configure column `ix`.
    pub fn column(mut self, ix: usize, column: Column) -> Self {
        if self.columns.len() <= ix {
            self.columns.resize(ix + 1, Column::default());
        }
        self.columns[ix] = column;
        self
    }

    /// Selection behaviour.
    pub fn selection_mode(mut self, mode: SelectionMode) -> Self {
        self.selection_mode = mode;
        self
    }

    /// Click a header to sort.
    pub fn sorting(mut self, on: bool) -> Self {
        self.sorting = on;
        self
    }

    /// Row height, px.
    pub fn row_height(mut self, h: f32) -> Self {
        self.row_height = h;
        self
    }

    /// Alternate row colours (`QAbstractItemView::setAlternatingRowColors`;
    /// on by default for tables, off for trees and lists).
    pub fn alternating(mut self, on: bool) -> Self {
        self.alternating = on;
        self
    }

    /// Switch alternating row colours on a live view.
    pub fn set_alternating(&mut self, on: bool, cx: &mut Context<Self>) {
        if self.alternating != on {
            self.alternating = on;
            cx.notify();
        }
    }

    /// Whether rows alternate their colours.
    pub fn is_alternating(&self) -> bool {
        self.alternating
    }

    /// Show the header.
    pub fn header(mut self, on: bool) -> Self {
        self.header = on;
        self
    }

    /// Grid lines between cells.
    pub fn grid(mut self, on: bool) -> Self {
        self.grid = on;
        self
    }

    /// Chevrons on top-level rows (`rootIsDecorated`).
    pub fn root_decorated(mut self, on: bool) -> Self {
        self.root_decorated = on;
        self
    }

    /// Corner radius of this view's frame, px (default: the style's
    /// `frame_radius`).
    pub fn corner_radius(mut self, r: f32) -> Self {
        self.radius = Some(r.max(0.0));
        self
    }

    /// Width of this view's scrollbars, px (default: the style's
    /// `scrollbar_width`).
    pub fn scrollbar_width(mut self, w: f32) -> Self {
        self.bar_width = Some(w.max(2.0));
        self
    }

    /// Default margins inside a cell: of an open editor (1 px; Qt gives it
    /// the whole cell, 0) and of a button delegate (`[4, 2, 4, 2]`).
    /// A column's [`Column::margin`] overrides both.
    pub fn delegate_margins(mut self, editor: impl Into<Margin>, button: impl Into<Margin>) -> Self {
        self.editor_margin = editor.into();
        self.button_margin = button.into();
        self
    }

    /// Lay the scrollbars over the content (default: the style's
    /// `scrollbar_overlay`), keeping only their slim resting width free.
    pub fn overlay_scrollbars(mut self, on: bool) -> Self {
        self.overlay = Some(on);
        self
    }

    fn bar_extent(&self, style: &LaceStyle) -> f32 {
        self.bar_width.unwrap_or(style.scrollbar_width)
    }

    /// Width a bar takes from the layout: its slim resting width when it
    /// overlays the content, else its full width.
    fn bar_gutter(&self, style: &LaceStyle) -> f32 {
        let extent = self.bar_extent(style);
        let overlay = self.overlay.unwrap_or(style.scrollbar_overlay);
        if overlay && style.scrollbar == crate::style::ScrollbarMode::Expanding {
            (style.scrollbar_rest + 2.0).min(extent)
        } else {
            extent
        }
    }

    /// Whether the rows overflow the viewport, so a vertical bar shows.
    fn needs_vbar(&self) -> bool {
        self.rows.len() as f32 * self.row_height > self.viewport.1 + 0.5
    }

    /// The model.
    pub fn model(&self) -> &Entity<ItemModel> {
        &self.model
    }

    /// The selected rows.
    pub fn selection(&self) -> Vec<NodeId> {
        self.selection.iter().copied().collect()
    }

    /// The current row.
    pub fn current(&self) -> Option<NodeId> {
        self.current
    }

    /// Start with `ids` selected; the first is current.
    pub fn with_selected(mut self, ids: &[NodeId], cx: &mut Context<Self>) -> Self {
        if self.selection_mode != SelectionMode::None {
            let take = if self.selection_mode == SelectionMode::Single { 1 } else { ids.len() };
            self.selection.extend(ids.iter().take(take).copied());
        }
        self.set_current(ids.first().copied(), cx);
        self
    }

    /// Select `id` alone and make it current.
    pub fn select(&mut self, id: NodeId, cx: &mut Context<Self>) {
        self.refresh_rows(cx);
        let ix = self.rows.iter().position(|r| r.0 == id);
        self.selection.clear();
        if self.selection_mode != SelectionMode::None {
            self.selection.insert(id);
        }
        self.anchor = ix;
        self.set_current(Some(id), cx);
        if let Some(ix) = ix {
            self.scroll.scroll_to_item(ix, ScrollStrategy::Top);
        }
        cx.emit(ViewEvent::SelectionChanged);
        cx.notify();
    }

    fn set_current(&mut self, id: Option<NodeId>, cx: &mut Context<Self>) {
        if self.current != id {
            self.current = id;
            cx.emit(ViewEvent::CurrentChanged(id));
        }
    }

    fn column_spec(&self, ix: usize) -> Column {
        self.columns.get(ix).cloned().unwrap_or_else(|| {
            if self.mode == Mode::List {
                Column::stretch()
            } else {
                Column::default()
            }
        })
    }

    fn refresh_rows(&mut self, cx: &App) {
        let m = self.model.read(cx);
        if self.rows_rev != Some(m.revision()) {
            self.rows = m.visible(self.mode != Mode::Tree);
            self.rows_rev = Some(m.revision());
            let live: BTreeSet<NodeId> = self.rows.iter().map(|r| r.0).collect();
            self.selection.retain(|id| m.node(*id).is_some());
            if self.current.is_some_and(|c| m.node(c).is_none()) {
                self.current = None;
            }
            let _ = live;
        }
    }

    // ---------------------------------------------------------------- layout

    fn measure(&self, text: &str, window: &mut Window, style: &LaceStyle) -> f32 {
        if text.is_empty() {
            return 0.0;
        }
        let font = window.text_style().font();
        let run = TextRun {
            len: text.len(),
            font,
            color: gpui_kit::gpui::black(),
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        f32::from(window.text_system().layout_line(text, px(style.font_size), &[run], None).width)
    }

    /// Widths of `Contents` columns, cached per model revision.
    fn contents_widths(&mut self, n: usize, window: &mut Window, style: &LaceStyle, cx: &App) -> Vec<f32> {
        let rev = self.model.read(cx).revision();
        if self.content_widths.0 == rev && self.content_widths.1.len() == n {
            return self.content_widths.1.clone();
        }
        let m = self.model.read(cx).clone();
        let mut out = vec![0.0; n];
        for (c, w) in out.iter_mut().enumerate() {
            if self.column_spec(c).resize != Resize::Contents {
                continue;
            }
            let head = m.headers().get(c).map(|h| h.to_string()).unwrap_or_default();
            *w = self.measure(&head, window, style) + 2.0 * CELL_PAD + 14.0;
            let decimals = match self.column_spec(c).delegate {
                Delegate::Number { decimals, .. } => Some(decimals),
                _ => None,
            };
            // A sample keeps huge models cheap.
            for &(id, depth) in self.rows.iter().take(400) {
                let text = m.value(id, c).display(decimals);
                let mut cw = self.measure(&text, window, style) + 2.0 * CELL_PAD;
                if c == 0 && self.mode == Mode::Tree {
                    cw += self.lead_width(depth, m.node(id).is_some_and(|n| n.check.is_some()));
                }
                *w = w.max(cw);
            }
            if let Delegate::Choice(_) | Delegate::Number { .. } = self.column_spec(c).delegate {
                *w += 20.0;
            }
        }
        self.content_widths = (rev, out.clone());
        out
    }

    /// Indent, chevron and check box before a tree's first cell.
    fn lead_width(&self, depth: usize, checkable: bool) -> f32 {
        let mut w = depth as f32 * INDENT;
        if self.mode == Mode::Tree && self.root_decorated {
            w += BRANCH;
        }
        if checkable {
            w += crate::controls::check::INDICATOR + 4.0;
        }
        w
    }

    fn layout_columns(&mut self, n: usize, window: &mut Window, style: &LaceStyle, cx: &App) {
        let contents = self.contents_widths(n, window, style, cx);
        if self.user_widths.len() != n {
            self.user_widths.resize(n, None);
        }
        // Nothing to scroll, nothing to reserve.
        let avail = (self.viewport.0 - if self.needs_vbar() { self.bar_gutter(style) } else { 0.0 }).max(0.0);
        let mut widths = vec![0.0; n];
        let mut stretch = 0;
        for (c, w) in widths.iter_mut().enumerate() {
            *w = match self.column_spec(c).resize {
                Resize::Fixed(w) => w,
                Resize::Interactive(w) => self.user_widths[c].unwrap_or(w),
                Resize::Contents => contents[c],
                Resize::Stretch => {
                    stretch += 1;
                    0.0
                }
            };
        }
        if stretch > 0 {
            let used: f32 = widths.iter().sum();
            let share = ((avail - used) / stretch as f32).max(60.0);
            for (c, w) in widths.iter_mut().enumerate() {
                if self.column_spec(c).resize == Resize::Stretch {
                    *w = share;
                }
            }
        }
        self.widths = widths;
    }

    // ------------------------------------------------------------- selection

    fn click_row(&mut self, id: NodeId, ix: usize, ctrl: bool, shift: bool, cx: &mut Context<Self>) {
        match self.selection_mode {
            SelectionMode::None => {}
            SelectionMode::Single => {
                self.selection.clear();
                self.selection.insert(id);
                self.anchor = Some(ix);
            }
            SelectionMode::Extended => {
                if shift {
                    let a = self.anchor.unwrap_or(ix);
                    let (lo, hi) = (a.min(ix), a.max(ix));
                    if !ctrl {
                        self.selection.clear();
                    }
                    self.selection.extend(self.rows[lo..=hi].iter().map(|r| r.0));
                } else if ctrl {
                    if !self.selection.remove(&id) {
                        self.selection.insert(id);
                    }
                    self.anchor = Some(ix);
                } else {
                    self.selection.clear();
                    self.selection.insert(id);
                    self.anchor = Some(ix);
                }
            }
        }
        self.set_current(Some(id), cx);
        cx.emit(ViewEvent::SelectionChanged);
        cx.notify();
    }

    fn move_current(&mut self, to: isize, relative: bool, shift: bool, cx: &mut Context<Self>) {
        if self.rows.is_empty() {
            return;
        }
        let here = self.current.and_then(|c| self.rows.iter().position(|r| r.0 == c));
        let target = if relative { here.map(|h| h as isize + to).unwrap_or(0) } else { to };
        let ix = target.clamp(0, self.rows.len() as isize - 1) as usize;
        let id = self.rows[ix].0;
        self.click_row(id, ix, false, shift && self.selection_mode == SelectionMode::Extended, cx);
        self.scroll.scroll_to_item(ix, ScrollStrategy::Nearest);
    }

    fn page_rows(&self) -> isize {
        ((self.viewport.1 / self.row_height).floor() as isize - 1).max(1)
    }

    // ----------------------------------------------------------- tree, checks

    fn toggle_expanded(&mut self, id: NodeId, open: Option<bool>, cx: &mut Context<Self>) {
        self.model.update(cx, |m, cx| {
            let Some(n) = m.node(id) else { return };
            let to = open.unwrap_or(!n.expanded);
            if n.expanded != to && !n.children.is_empty() {
                m.set_expanded(id, to);
                cx.notify();
            }
        });
    }

    fn toggle_check(&mut self, id: NodeId, cx: &mut Context<Self>) {
        use crate::controls::CheckState as C;
        self.model.update(cx, |m, cx| {
            let Some(state) = m.node(id).and_then(|n| n.check) else { return };
            let to = if state == C::Checked { C::Unchecked } else { C::Checked };
            for (node, state) in m.set_check(id, to) {
                cx.emit(ModelEvent::Checked { node, state });
            }
            cx.notify();
        });
    }

    // ---------------------------------------------------------------- editing

    fn edit_cell(&mut self, id: NodeId, column: usize, value: Value, cx: &mut Context<Self>) {
        self.model.update(cx, |m, cx| {
            if m.value(id, column) != value {
                m.set(id, column, value);
                cx.emit(ModelEvent::Edited { node: id, column });
                cx.notify();
            }
        });
    }

    /// Write a number from a slider, keeping an integer cell an integer.
    fn edit_number(&mut self, id: NodeId, column: usize, v: f64, cx: &mut Context<Self>) {
        let old = self.model.read(cx).value(id, column);
        let nv = if matches!(old, Value::Int(_)) { Value::Int(v.round() as i64) } else { Value::Float(v) };
        self.edit_cell(id, column, nv, cx);
    }

    /// Set `column` on `id` and clear it on the rows sharing its parent: a
    /// radio group per sibling set.
    fn choose_radio(&mut self, id: NodeId, column: usize, cx: &mut Context<Self>) {
        let siblings: Vec<NodeId> = {
            let m = self.model.read(cx);
            let parent = m.node(id).and_then(|n| n.parent);
            m.children(parent).to_vec()
        };
        for other in siblings {
            if other != id && self.model.read(cx).value(other, column).as_bool() {
                self.edit_cell(other, column, Value::Bool(false), cx);
            }
        }
        self.edit_cell(id, column, Value::Bool(true), cx);
    }

    /// A per-cell element id.
    fn cell_id(&self, kind: &str, column: usize, id: NodeId) -> ElementId {
        ElementId::NamedInteger(SharedString::from(format!("{}-{kind}{column}", self.id_name())), id as u64)
    }

    /// A slider delegate: the slider, then the value right-aligned.
    fn slider_cell(&self, slider: impl IntoElement, text: SharedString, style: &LaceStyle) -> AnyElement {
        div()
            .flex_1()
            .min_w_0()
            .flex()
            .items_center()
            .gap(px(6.))
            .child(div().flex_1().min_w_0().child(slider))
            .child(
                div()
                    .flex_none()
                    .min_w(px(style.font_size * 2.5))
                    .flex()
                    .justify_end()
                    .whitespace_nowrap()
                    .child(text),
            )
            .into_any_element()
    }

    fn can_edit(&self, id: NodeId, column: usize, cx: &App) -> bool {
        let editable = self.model.read(cx).node(id).is_some_and(|n| n.enabled && n.editable);
        editable && self.column_spec(column).delegate.editable()
    }

    /// Start editing `column` of `id` (F2, Enter, double-click).
    pub fn start_edit(&mut self, id: NodeId, column: usize, window: &mut Window, cx: &mut Context<Self>) {
        if !self.can_edit(id, column, cx) {
            return;
        }
        self.commit_edit(window, cx);
        self.editing = Some((id, column));
        if let Delegate::Text = self.column_spec(column).delegate {
            let text = self.model.read(cx).value(id, column).display(None);
            let editor = cx.new(|cx| InputState::new(window, cx).default_value(text));
            editor.update(cx, |e, cx| e.focus(window, cx));
            self.editor_sub = Some(cx.subscribe_in(&editor, window, |this, _, ev: &InputEvent, window, cx| match ev {
                InputEvent::PressEnter { .. } | InputEvent::Blur => this.commit_edit(window, cx),
                _ => {}
            }));
            self.editor = Some(editor);
        }
        cx.notify();
    }

    /// Write the open editor back and close it.
    pub fn commit_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((id, column)) = self.editing.take() else { return };
        if let Some(editor) = self.editor.take() {
            let text = editor.read(cx).value().to_string();
            let value = self.model.read(cx).value(id, column).parse_like(&text);
            self.edit_cell(id, column, value, cx);
        }
        self.editor_sub = None;
        self.focus.focus(window, cx);
        cx.notify();
    }

    /// Close the editor without writing.
    pub fn cancel_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.editing = None;
        self.editor = None;
        self.editor_sub = None;
        self.focus.focus(window, cx);
        cx.notify();
    }

    fn on_key(&mut self, ev: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = ev.keystroke.key.as_str();
        let m = ev.keystroke.modifiers;
        if self.editing.is_some() {
            if key == "escape" {
                self.cancel_edit(window, cx);
                cx.stop_propagation();
            }
            return;
        }
        let page = self.page_rows();
        match key {
            "up" => self.move_current(-1, true, m.shift, cx),
            "down" => self.move_current(1, true, m.shift, cx),
            "pageup" => self.move_current(-page, true, m.shift, cx),
            "pagedown" => self.move_current(page, true, m.shift, cx),
            "home" => self.move_current(0, false, m.shift, cx),
            "end" => self.move_current(isize::MAX / 2, false, m.shift, cx),
            "left" | "right" if self.mode == Mode::Tree => {
                let Some(id) = self.current else { return };
                let (open, parent, has_kids) = {
                    let n = self.model.read(cx).node(id);
                    (n.is_some_and(|n| n.expanded), n.and_then(|n| n.parent), n.is_some_and(|n| !n.children.is_empty()))
                };
                if key == "right" {
                    if has_kids && !open {
                        self.toggle_expanded(id, Some(true), cx);
                    } else if has_kids {
                        self.move_current(1, true, false, cx);
                    }
                } else if open {
                    self.toggle_expanded(id, Some(false), cx);
                } else if let Some(p) = parent {
                    self.select(p, cx);
                }
            }
            "space" => {
                if let Some(id) = self.current {
                    self.toggle_check(id, cx);
                }
            }
            "a" if (m.control || m.platform) && self.selection_mode == SelectionMode::Extended => {
                self.selection = self.rows.iter().map(|r| r.0).collect();
                cx.emit(ViewEvent::SelectionChanged);
                cx.notify();
            }
            "enter" | "f2" => {
                let Some(id) = self.current else { return };
                let col = (0..self.widths.len()).find(|&c| self.can_edit(id, c, cx));
                match col {
                    Some(c) => self.start_edit(id, c, window, cx),
                    None if key == "enter" => cx.emit(ViewEvent::Activated(id)),
                    None => {}
                }
            }
            _ => return,
        }
        cx.stop_propagation();
    }

    // -------------------------------------------------------------- painting

    fn row_data(&self, range: Range<usize>, cx: &App) -> Vec<RowData> {
        let m = self.model.read(cx);
        self.rows[range.start.min(self.rows.len())..range.end.min(self.rows.len())]
            .iter()
            .enumerate()
            .filter_map(|(k, &(id, depth))| {
                let n = m.node(id)?;
                Some(RowData {
                    id,
                    ix: range.start + k,
                    depth,
                    cells: n.cells.clone(),
                    check: n.check,
                    has_children: !n.children.is_empty(),
                    expanded: n.expanded,
                    enabled: n.enabled,
                    editable: n.editable,
                })
            })
            .collect()
    }

    fn render_rows(&mut self, range: Range<usize>, window: &mut Window, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let style = cx.lace().clone();
        let pal = style.pal(State::NORMAL).clone();
        let grid = hsla(rules::frame_line(&style, State::NORMAL));
        let focused = self.focus.contains_focused(window, cx) && window.last_input_was_keyboard();
        let total: f32 = self.widths.iter().sum();
        let data = self.row_data(range, cx);
        let mut out = Vec::with_capacity(data.len());
        for r in data {
            let selected = self.selection.contains(&r.id);
            let s = State {
                selected,
                hovered: self.hovered == Some(r.id),
                focused: focused && self.current == Some(r.id) && self.editing.is_none(),
                disabled: !r.enabled,
                ..State::NORMAL
            };
            let (fill, _) = rules::item_row(&style, s);
            let mut row = item_row_div(&style, s)
                .id(ElementId::NamedInteger(self.id_name(), r.id as u64))
                .h(px(self.row_height))
                .w(px(total.max(self.viewport.0)))
                .flex_none()
                .relative()
                .left(px(-self.hscroll))
                .when(fill.is_none() && self.alternating && r.ix % 2 == 1, |d| d.bg(hsla(pal.alternate_base)))
                .on_hover(cx.listener(move |this, h: &bool, _, cx| {
                    if *h {
                        this.hovered = Some(r.id);
                    } else if this.hovered == Some(r.id) {
                        this.hovered = None;
                    }
                    cx.notify();
                }));
            for (c, w) in self.widths.clone().into_iter().enumerate() {
                row = row.child(self.render_cell(&r, c, w, s, &style, grid, window, cx));
            }
            out.push(row.into_any_element());
        }
        out
    }

    fn id_name(&self) -> SharedString {
        match &self.id {
            ElementId::Name(n) => n.clone(),
            _ => "item-view".into(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_cell(
        &mut self,
        r: &RowData,
        c: usize,
        w: f32,
        s: State,
        style: &LaceStyle,
        grid: gpui_kit::gpui::Hsla,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let spec = self.column_spec(c);
        let value = r.cells.get(c).cloned().unwrap_or_default();
        let (id, ix) = (r.id, r.ix);
        let editing = self.editing == Some((id, c));
        let align = match spec.align {
            Align::Auto
                if (value.is_number() || matches!(spec.delegate, Delegate::Number { .. }))
                    && !matches!(spec.delegate, Delegate::Slider { .. } | Delegate::DynamicSlider(_)) =>
            {
                Align::Right
            }
            Align::Auto => Align::Left,
            a => a,
        };
        let is_button = matches!(spec.delegate, Delegate::Button(..));
        let editor_margin = spec.margin.unwrap_or(self.editor_margin);
        let button_margin = spec.margin.unwrap_or(self.button_margin);
        let editor_open =
            editing && matches!(spec.delegate, Delegate::Text | Delegate::Number { .. } | Delegate::Choice(_));
        let mut cell = div()
            .id(ElementId::NamedInteger(SharedString::from(format!("{}-c{c}", self.id_name())), id as u64))
            .w(px(w))
            .h_full()
            .flex_none()
            .flex()
            .items_center()
            // An open editor fills the cell, as Qt's updateEditorGeometry
            // sets it to the whole option.rect.
            .px(px(if editor_open || is_button { 0.0 } else { CELL_PAD }))
            .gap(px(4.))
            .overflow_hidden()
            .when(self.grid, |d| d.border_r_1().border_b_1().border_color(grid));

        // Tree lead: indent, chevron, check box.
        if c == 0 && self.mode == Mode::Tree {
            let branch = hsla(rules::branch(style, s));
            cell = cell.child(div().w(px(r.depth as f32 * INDENT)).flex_none());
            if self.root_decorated || r.depth > 0 {
                let open = r.expanded;
                let chevron = div()
                    .id(ElementId::NamedInteger(SharedString::from(format!("{}-br", self.id_name())), id as u64))
                    .w(px(BRANCH))
                    .h_full()
                    .flex_none()
                    .when(r.has_children, |d| {
                        d.child(glyph::chevron(if open { Dir::Down } else { Dir::Right }, branch, 0.0))
                            .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                this.toggle_expanded(id, None, cx);
                            }))
                    });
                cell = cell.child(chevron);
            }
            if let Some(check) = r.check {
                let cs = State {
                    checked: check == crate::controls::CheckState::Checked,
                    partial: check == crate::controls::CheckState::Partial,
                    disabled: s.disabled,
                    ..State::NORMAL
                };
                cell = cell.child(
                    div()
                        .id(ElementId::NamedInteger(SharedString::from(format!("{}-ck", self.id_name())), id as u64))
                        .flex_none()
                        .child(check_indicator(style, cs, crate::controls::check::INDICATOR))
                        .when(r.enabled, |d| {
                            d.on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                this.toggle_check(id, cx);
                            }))
                        }),
                );
            }
        }

        let content: AnyElement = match (&spec.delegate, editing) {
            (Delegate::Text, true) => match &self.editor {
                Some(e) => div().flex_1().h_full().child(LineEdit::new(ElementId::Name("cell-editor".into()), e).fill_with(editor_margin)).into_any_element(),
                None => div().into_any_element(),
            },
            (Delegate::Number { min, max, step, decimals }, true) => div()
                .flex_1()
                .h_full()
                .child(
                    SpinBox::new(ElementId::Name("cell-spin".into()), value.as_f64().unwrap_or(0.0))
                        .range(*min, *max)
                        .step(*step)
                        .decimals(*decimals)
                        .fill_with(editor_margin)
                        .on_change(cx.processor(move |this, v: f64, _, cx| {
                            let old = this.model.read(cx).value(id, c);
                            let nv = if matches!(old, Value::Int(_)) { Value::Int(v.round() as i64) } else { Value::Float(v) };
                            this.edit_cell(id, c, nv, cx);
                        })),
                )
                .into_any_element(),
            (Delegate::Choice(choices), true) => {
                let current = choices.iter().position(|ch| *ch == value.display(None));
                let list = choices.clone();
                div()
                    .flex_1()
                    .h_full()
                    .child(
                        ComboBox::new(ElementId::Name("cell-combo".into()), choices.iter().cloned())
                            .selected(current)
                            .fill_with(editor_margin)
                            .on_select(cx.processor(move |this, i: usize, window, cx| {
                                this.edit_cell(id, c, Value::Text(list[i].clone()), cx);
                                this.editing = None;
                                this.focus.focus(window, cx);
                            })),
                    )
                    .into_any_element()
            }
            (Delegate::Check, _) => {
                let on = value.as_bool();
                let cs = State { checked: on, disabled: s.disabled || !r.editable, ..State::NORMAL };
                div()
                    .id(ElementId::NamedInteger(SharedString::from(format!("{}-bk{c}", self.id_name())), id as u64))
                    .flex_1()
                    .flex()
                    .justify_center()
                    .child(check_indicator(style, cs, crate::controls::check::INDICATOR))
                    .when(r.enabled && r.editable, |d| {
                        d.on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.edit_cell(id, c, Value::Bool(!on), cx);
                        }))
                    })
                    .into_any_element()
            }
            (Delegate::Switch, _) => {
                let on = value.as_bool();
                div()
                    .flex_1()
                    .flex()
                    .justify_center()
                    .child(
                        Switch::new(self.cell_id("sw", c, id))
                            .checked(on)
                            .disabled(s.disabled || !r.editable)
                            .on_change(cx.processor(move |this, v: bool, _, cx| this.edit_cell(id, c, Value::Bool(v), cx))),
                    )
                    .into_any_element()
            }
            (Delegate::Toggle(label), _) => {
                let on = value.as_bool();
                let view = cx.entity().downgrade();
                div()
                    .flex_1()
                    .h_full()
                    .child(
                        PushButton::new(self.cell_id("tg", c, id), label.clone())
                            .checked(on)
                            .fill_with(button_margin)
                            .disabled(s.disabled || !r.editable)
                            .on_click(move |_, cx| {
                                view.update(cx, |this, cx| this.edit_cell(id, c, Value::Bool(!on), cx)).ok();
                            }),
                    )
                    .into_any_element()
            }
            (Delegate::Radio, _) => {
                let on = value.as_bool();
                let cs = State { checked: on, disabled: s.disabled || !r.editable, ..State::NORMAL };
                div()
                    .id(self.cell_id("rd", c, id))
                    .flex_1()
                    .flex()
                    .justify_center()
                    .child(radio_indicator(style, cs, crate::controls::check::INDICATOR))
                    .when(r.enabled && r.editable, |d| {
                        d.on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.choose_radio(id, c, cx);
                        }))
                    })
                    .into_any_element()
            }
            (Delegate::Slider { min, max, step, decimals }, _) => {
                let v = value.as_f64().unwrap_or(*min);
                self.slider_cell(
                    Slider::new(self.cell_id("sl", c, id), v)
                        .range(*min, *max)
                        .steps(*step, *step * 10.0)
                        .selected(s.selected)
                        .disabled(s.disabled || !r.editable)
                        .on_change(cx.processor(move |this, v: f64, _, cx| this.edit_number(id, c, v, cx))),
                    value.display(Some(*decimals)),
                    style,
                )
            }
            (Delegate::DynamicSlider(range), _) => {
                let v = value.as_f64().unwrap_or(0.0);
                self.slider_cell(
                    DynamicSlider::new(self.cell_id("ds", c, id), v)
                        .range(*range)
                        .selected(s.selected)
                        .disabled(s.disabled || !r.editable)
                        .on_change(cx.processor(move |this, v: f64, _, cx| this.edit_number(id, c, v, cx))),
                    value.display(Some(range.decimals)),
                    style,
                )
            }
            (Delegate::Progress(max), _) => {
                let v = value.as_f64().unwrap_or(0.0);
                let frac = (v / max.max(f64::EPSILON)).clamp(0.0, 1.0) as f32;
                let label: SharedString = format!("{:.0}%", 100.0 * frac).into();
                let h = (self.row_height - 8.0).max(4.0);
                let ps = State { disabled: s.disabled, ..State::NORMAL };
                let (fill, ink) = progress_colors(style, ps);
                // One label colour that reads on both the fill and the track,
                // so no clip edge can cut through the digits.
                div()
                    .flex_1()
                    .relative()
                    .h(px(h))
                    .child(ProgressBar::new(v).range(0.0, *max).thickness(h).disabled(s.disabled).fill(fill))
                    .child(
                        div()
                            .absolute()
                            .inset_0()
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_color(hsla(ink))
                            .child(label),
                    )
                    .into_any_element()
            }
            (Delegate::Button(label, f), _) => {
                let f = f.clone();
                // Qt paints a button delegate into option.rect less a small margin.
                div()
                    .flex_1()
                    .h_full()
                    .child(
                        PushButton::new(
                            ElementId::NamedInteger(SharedString::from(format!("{}-bt{c}", self.id_name())), id as u64),
                            label.clone(),
                        )
                        .fill_with(button_margin)
                        .disabled(s.disabled)
                        .on_click(move |window, cx| f(id, window, cx)),
                    )
                    .into_any_element()
            }
            (Delegate::Custom(f), _) => {
                let ctx = CellCtx { node: id, column: c, value: &value, selected: s.selected, enabled: r.enabled, style };
                div().flex_1().min_w_0().child(f(&ctx, window, cx)).into_any_element()
            }
            (d, _) => {
                let decimals = match d {
                    Delegate::Number { decimals, .. } => Some(*decimals),
                    _ => None,
                };
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .when(align == Align::Right, |d| d.justify_end())
                    .when(align == Align::Center, |d| d.justify_center())
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .child(value.display(decimals))
                    .into_any_element()
            }
        };
        cell = cell.child(content);

        // Press selects; a double press edits or activates.
        cell.on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, ev: &MouseDownEvent, window, cx| {
                this.focus.focus(window, cx);
                if this.editing.is_some_and(|e| e != (id, c)) {
                    this.commit_edit(window, cx);
                }
                if ev.click_count >= 2 {
                    if this.can_edit(id, c, cx) {
                        this.start_edit(id, c, window, cx);
                        // Keeps the view's own focus-on-press from taking focus back.
                        window.prevent_default();
                    } else {
                        if this.mode == Mode::Tree {
                            this.toggle_expanded(id, None, cx);
                        }
                        cx.emit(ViewEvent::Activated(id));
                    }
                } else {
                    let m = ev.modifiers;
                    this.click_row(id, ix, m.control || m.platform, m.shift, cx);
                }
            }),
        )
        .into_any_element()
    }

    fn render_header(&mut self, style: &LaceStyle, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let headers = self.model.read(cx).headers().to_vec();
        let mut row = div().flex().flex_none().relative().left(px(-self.hscroll));
        for (c, w) in self.widths.clone().into_iter().enumerate() {
            let sid = ElementId::NamedInteger(SharedString::from(format!("{}-h", self.id_name())), c as u64);
            let ix = Interaction::new(sid.clone(), window, cx);
            let mut s = ix.state(window, cx);
            s.hovered &= self.sorting;
            let sort = self.sort.filter(|(sc, _)| *sc == c).map(|(_, asc)| if asc { Sort::Ascending } else { Sort::Descending });
            let title = headers.get(c).cloned().unwrap_or_default();
            let resizable = matches!(self.column_spec(c).resize, Resize::Interactive(_));
            let section = header_section(style, s, title, sort)
                .id(sid)
                .w(px(w))
                .flex_none()
                .relative()
                .when(self.sorting, |d| {
                    d.on_click(cx.listener(move |this, ev: &ClickEvent, _, cx| {
                        // The release after a resize drag is no click.
                        if let ClickEvent::Mouse(m) = ev {
                            if f32::from(m.up.position.x - m.down.position.x).abs() > 3.0 {
                                return;
                            }
                        }
                        let asc = !matches!(this.sort, Some((sc, true)) if sc == c);
                        this.sort = Some((c, asc));
                        this.model.update(cx, |m, cx| {
                            m.sort(c, asc);
                            cx.emit(ModelEvent::Sorted { column: c, ascending: asc });
                            cx.notify();
                        });
                    }))
                })
                .when(resizable, |d| {
                    d.child(
                        div()
                            .id(ElementId::NamedInteger(SharedString::from(format!("{}-grip", self.id_name())), c as u64))
                            .absolute()
                            .top_0()
                            .bottom_0()
                            .right(px(-GRIP / 2.0))
                            .w(px(GRIP))
                            .cursor_col_resize()
                            .on_mouse_down(MouseButton::Left, cx.listener(move |this, ev: &MouseDownEvent, _, cx| {
                                cx.stop_propagation();
                                let w = this.widths.get(c).copied().unwrap_or(100.0);
                                this.resizing = Some((c, f32::from(ev.position.x), w));
                            })),
                    )
                });
            row = row.child(ix.bind(section, !self.sorting));
        }
        // An empty section runs on to the right edge, over the scrollbar's
        // gutter too, so the header leaves no hole above the bar.
        let filler = header_section(style, State::NORMAL, SharedString::default(), None).flex_none().w(px(4000.));
        div().overflow_hidden().flex_none().child(row.child(filler)).into_any_element()
    }
}

impl Render for ItemView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let style = cx.lace().clone();
        let pal = style.pal(State::NORMAL).clone();
        self.refresh_rows(cx);
        let n = self.model.read(cx).column_count();
        self.layout_columns(n, window, &style, cx);
        let total: f32 = self.widths.iter().sum();
        let extent = self.bar_extent(&style);
        let gutter = self.bar_gutter(&style);
        let v_gutter = if self.needs_vbar() { gutter } else { 0.0 };
        let max_h = (total - (self.viewport.0 - v_gutter)).max(0.0);
        self.hscroll = self.hscroll.clamp(0.0, max_h);

        let rows = self.rows.len();
        let list = uniform_list(self.id.clone(), rows, cx.processor(|this, range, window, cx| this.render_rows(range, window, cx)))
            .track_scroll(&self.scroll)
            .size_full();

        let base = self.scroll.0.borrow().base_handle.clone();
        let (off, max) = (base.offset(), base.max_offset());
        let vbar = Scrollbar::new(
            ElementId::Name(format!("{}-vsb", self.id_name()).into()),
            f64::from(-f32::from(off.y)),
            f64::from(f32::from(max.y)),
            f64::from(self.viewport.1),
        )
        .vertical()
        .width(extent)
        .on_scroll({
            let base = base.clone();
            cx.processor(move |_, v: f64, _, cx| {
                base.set_offset(point(base.offset().x, px(-(v as f32))));
                cx.notify();
            })
        });
        let hbar = (max_h > 0.5).then(|| {
            Scrollbar::new(
                ElementId::Name(format!("{}-hsb", self.id_name()).into()),
                self.hscroll as f64,
                max_h as f64,
                self.viewport.0 as f64,
            )
            .width(extent)
            .on_scroll(cx.processor(|this, v: f64, _, cx| {
                this.hscroll = v as f32;
                cx.notify();
            }))
        });
        // Each bar sits in a strip as wide as its gutter, anchored to the
        // far edge and reaching back over the content as it grows.
        let hbar = hbar.map(|bar| {
            div()
                .relative()
                .flex_none()
                .h(px(gutter))
                .child(div().absolute().left_0().right_0().bottom_0().h(px(extent)).child(bar))
        });
        let vbar = div()
            .relative()
            .flex_none()
            .w(px(gutter))
            .h_full()
            .child(div().absolute().top_0().bottom_0().right_0().w(px(extent)).child(vbar));

        let header = (self.header && self.mode != Mode::List).then(|| self.render_header(&style, window, cx));
        let measure = cx.entity().downgrade();
        let body = div()
            .relative()
            .flex_1()
            .min_h_0()
            .flex()
            .child(div().flex_1().min_w_0().h_full().child(list))
            .when(self.needs_vbar() && f32::from(max.y) > 0.5, |d| d.child(vbar))
            .child(
                canvas(
                    move |b, window, cx| {
                        let size = (f32::from(b.size.width), f32::from(b.size.height));
                        let changed = measure
                            .update(cx, |this, cx| {
                                let changed =
                                    (this.viewport.0 - size.0).abs() > 0.5 || (this.viewport.1 - size.1).abs() > 0.5;
                                if changed {
                                    this.viewport = size;
                                    cx.notify();
                                }
                                changed
                            })
                            .unwrap_or(false);
                        // A notify raised mid-draw is dropped; ask for the next
                        // frame so the new column widths show without input.
                        if changed {
                            window.request_animation_frame();
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full(),
            )
            .on_scroll_wheel(cx.listener(move |this, ev: &ScrollWheelEvent, _, cx| {
                let d = ev.delta.pixel_delta(px(20.));
                let dx = f32::from(d.x);
                if dx.abs() > 0.0 {
                    this.hscroll = (this.hscroll - dx).clamp(0.0, max_h);
                    cx.notify();
                }
            }));

        // Column resize drags anywhere in the window.
        let drag = cx.entity().downgrade();
        let tracker = canvas(
            |_, _, _| (),
            move |_, (), window, _| {
                let mv = drag.clone();
                window.on_mouse_event(move |ev: &MouseMoveEvent, phase, _, cx| {
                    if phase != DispatchPhase::Bubble {
                        return;
                    }
                    let _ = mv.update(cx, |this, cx| {
                        if let Some((c, x0, w0)) = this.resizing {
                            if !ev.dragging() {
                                this.resizing = None;
                                return;
                            }
                            let w = (w0 + f32::from(ev.position.x) - x0).max(MIN_COLUMN);
                            if this.user_widths.len() <= c {
                                this.user_widths.resize(c + 1, None);
                            }
                            this.user_widths[c] = Some(w);
                            cx.notify();
                        }
                    });
                });
                let up = drag.clone();
                window.on_mouse_event(move |_: &MouseUpEvent, _, _, cx| {
                    let _ = up.update(cx, |this, _| this.resizing = None);
                });
            },
        )
        .absolute()
        .size_full();

        let radius = self.radius.unwrap_or(style.frame_radius);
        let focused = self.focus.contains_focused(window, cx);
        let line = if focused && window.last_input_was_keyboard() {
            rules::field(&style, State::focused()).line
        } else {
            Some(rules::frame_line(&style, State::NORMAL))
        };
        div()
            .id(self.id.clone())
            .key_context("ItemView")
            .track_focus(&self.focus)
            .tab_stop(true)
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .bg(hsla(pal.base))
            .text_color(hsla(pal.text))
            .text_size(px(style.font_size))
            .border_1()
            .border_color(hsla(line.unwrap_or(pal.base)))
            .rounded(px(radius))
            // Keeps rows, header and bars out of the rounded corners.
            .p(px(style.frame_inset((radius - 1.0).max(0.0))))
            .on_key_down(cx.listener(Self::on_key))
            .children(header)
            .child(body)
            .children(hbar)
            .child(tracker)
    }
}

/// OKLCH lightness step that tones an embedded progress fill.
const PROGRESS_TONE: f64 = 0.10;

/// The toned fill of an embedded progress bar, and one label colour that
/// reads on both that fill and the track.
///
/// The accent sinks 10 % toward a dark theme's background (darker) or a
/// light theme's (brighter), flipping when that would push it past a
/// usable lightness.
fn progress_colors(style: &LaceStyle, s: State) -> (lace_gpui::color::Rgba, lace_gpui::color::Rgba) {
    use lace_gpui::color;
    let pal = style.pal(s);
    let accent = crate::style::paint::accent(style, s);
    let l = color::to_oklch(accent).0;
    let mut dl = if color::is_dark(pal.window) { -PROGRESS_TONE } else { PROGRESS_TONE };
    if !(0.25..=0.92).contains(&(l + dl)) {
        dl = -dl;
    }
    let fill = color::step(accent, dl);
    let track = rules::track(style, s);
    let worst = |c| color::contrast_ratio(c, fill).min(color::contrast_ratio(c, track));
    let ink = [pal.text, pal.highlighted_text]
        .into_iter()
        .max_by(|a, b| worst(*a).total_cmp(&worst(*b)))
        .unwrap_or(pal.text);
    (fill, ink)
}
