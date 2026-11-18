//! A full Lace workbench: the Lace dock (geometry skin, edge tabs,
//! auto-hide sidebars) filled with panes built from LaceStyle widgets.
//!
//! ```sh
//! cargo run --release --features gpui-kit --example lace_workbench -- [theme]
//! ```
//!
//! Layout:
//!
//! - Left dock: Explorer (tree view), Search (form plus result list).
//! - Centre: Editor, Controls, Data (2000-row table), Layout (splitters and
//!   tab widgets), beside an Overview group (rings, tags, badges, stepper).
//! - Right dock: Properties (form), Appearance (live style tokens, palette).
//! - Bottom dock: Console (command line plus log), Problems (alerts).
//! - Sidebars: Navigation (right) and Mixer (bottom) start pinned.
//!
//! The top bar switches the theme and the dock preset, so every pane can be
//! checked against every theme while the panes are dragged, split, pinned and
//! resized.

use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::base::dock::PanelView as BasePanelView;
use gpui_kit::component::dock::{
    BasePanel, DockLayout, DockPlacement, Panel, PanelEvent, PanelHandle, PanelId,
};
use gpui_kit::component::input::{InputState, TextareaState};
use gpui_kit::component::theme::ActiveTheme;
use gpui_kit::component::Icon;
use gpui_kit::*;

use lace_gpui::dock_icons::tab_label;
use lace_gpui::geometry::{
    DockGeometryStyle, GeometryDockSkin, OutlineStyle, SplitterCap, SplitterColors,
    SplitterLength,
};
use lace_gpui::sidebar::Sidebars;
use lace_gpui::skin;
use lace_widgets::controls as lw;
use lace_widgets::controls::{CheckState, Tone};
use lace_widgets::glyph::{self, Dir};
use lace_widgets::style::{hsla, rules, ActiveStyle, LaceStyle, State};
use lace_widgets::views::{Column, Delegate, ItemModel, ItemView, Mode, SelectionMode, Value};

const THEMES: &[&lace_gpui::engine::ThemeSpec] = &lace_gpui::themes::ALL;

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// A Lucide icon from gpui-kit's bundled assets, by file name.
fn lucide(name: &str) -> Icon {
    Icon::empty().path(format!("icons/{name}.svg"))
}

/// A widget callback that updates view `V` and repaints it.
fn on<V: 'static, T: 'static>(
    w: &WeakEntity<V>,
    f: impl Fn(&mut V, T, &mut Window, &mut Context<V>) + 'static,
) -> impl Fn(T, &mut Window, &mut App) + 'static {
    let w = w.clone();
    move |v, window, cx| {
        let _ = w.update(cx, |this, cx| {
            f(this, v, window, cx);
            cx.notify();
        });
    }
}

/// A click callback that updates view `V` and repaints it.
fn tap<V: 'static>(
    w: &WeakEntity<V>,
    f: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static,
) -> impl Fn(&mut Window, &mut App) + 'static {
    let f = on(w, move |this, (), window, cx| f(this, window, cx));
    move |window, cx| f((), window, cx)
}

/// A pane's body: window colour, LaceStyle text, scrolling when tall.
fn scroll_pane(id: &'static str, style: &LaceStyle) -> Stateful<Div> {
    pane(style).id(id).overflow_y_scroll()
}

/// A pane's body that fills its panel without scrolling.
fn pane(style: &LaceStyle) -> Div {
    let pal = style.pal(State::NORMAL);
    div()
        .size_full()
        .flex()
        .flex_col()
        .gap(px(10.))
        .p(px(10.))
        .bg(hsla(pal.window))
        .text_color(hsla(pal.window_text))
        .text_size(px(style.font_size))
}

/// A framed view surface: base colour, frame line, frame radius.
fn framed(style: &LaceStyle) -> Div {
    let pal = style.pal(State::NORMAL);
    div()
        .bg(hsla(pal.base))
        .text_color(hsla(pal.text))
        .border_1()
        .border_color(hsla(rules::frame_line(style, State::NORMAL)))
        .rounded(px(style.frame_radius))
}

/// A group box at its natural height: `GroupBox` fills a stretched parent,
/// which in a scrolling column would share out the whole pane.
fn natural(group: lw::GroupBox) -> Div {
    div().flex_none().child(group)
}

fn row() -> Div {
    div().flex().flex_wrap().items_center().gap(px(8.))
}

/// A form row: a fixed label column and the control.
fn field(label: &'static str, control: impl IntoElement) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(8.))
        .child(div().w(px(84.)).flex_none().child(label))
        .child(div().flex_1().min_w_0().child(control))
}

fn muted(style: &LaceStyle) -> Hsla {
    hsla(style.pal(State::NORMAL).placeholder)
}

fn input(window: &mut Window, cx: &mut App, text: &str) -> Entity<InputState> {
    let text = SharedString::from(text.to_string());
    cx.new(|cx| InputState::new(window, cx).default_value(text))
}

fn placeholder(window: &mut Window, cx: &mut App, text: &str) -> Entity<InputState> {
    let text = SharedString::from(text.to_string());
    cx.new(|cx| InputState::new(window, cx).placeholder(text))
}

/// The dock-panel boilerplate: focus, name and an icon-plus-label tab.
macro_rules! dock_panel {
    ($ty:ident, $title:literal, $icon:literal) => {
        impl EventEmitter<PanelEvent> for $ty {}

        impl Focusable for $ty {
            fn focus_handle(&self, _: &App) -> FocusHandle {
                self.focus.clone()
            }
        }

        impl BasePanel for $ty {
            // The sidebar rails label pinned panels by name.
            fn panel_name(&self) -> &'static str {
                $title
            }
        }

        impl Panel for $ty {
            fn title(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
                div()
                    .text_color(cx.theme().colors.foreground)
                    .child(tab_label(Some(lucide($icon)), $title))
            }
        }

        impl $ty {
            const ICON: &'static str = $icon;
        }
    };
}

// ---------------------------------------------------------------------------
// Explorer: a project tree
// ---------------------------------------------------------------------------

struct Explorer {
    focus: FocusHandle,
    filter: Entity<InputState>,
    tree: Entity<ItemView>,
    hidden: bool,
}

dock_panel!(Explorer, "Explorer", "folder");

impl Explorer {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let model = cx.new(|_| {
            let mut m = ItemModel::new(["Name", "Size", "Kind"]);
            let tree: [(&str, &[&str]); 5] = [
                ("src", &["color.rs", "contrast.rs", "engine.rs", "geometry.rs", "sidebar.rs", "skin.rs"]),
                ("examples", &["dock_geometry.rs", "lace_workbench.rs", "widgets.rs"]),
                ("lace-widgets", &["controls", "style", "views", "glyph.rs"]),
                ("docs", &["README.md", "feasibility.md"]),
                ("assets", &["icons", "fonts"]),
            ];
            for (fi, (folder, files)) in tree.into_iter().enumerate() {
                let f = m.add_row(None, vec![folder.into(), Value::Empty, "folder".into()]);
                for (i, file) in files.iter().enumerate() {
                    let size = ((fi * 7 + i * 13) % 40 + 1) as i64;
                    let kind = if file.contains('.') { "file" } else { "folder" };
                    let r = m.add_row(Some(f), vec![(*file).into(), Value::Int(size), kind.into()]);
                    if kind == "folder" {
                        for k in 0..3 {
                            m.add_row(Some(r), vec![format!("mod_{k}.rs").into(), Value::Int(k as i64 + 2), "file".into()]);
                        }
                    }
                }
                m.set_expanded(f, fi < 2);
            }
            m
        });
        let tree = cx.new(|cx| {
            ItemView::new(&model, Mode::Tree, cx)
                .column(0, Column::interactive(170.))
                .column(1, Column::interactive(48.))
                .column(2, Column::stretch())
                .selection_mode(SelectionMode::Extended)
                .sorting(true)
                .alternating(true)
                .with_selected(&[2], cx)
        });
        Self {
            focus: cx.focus_handle(),
            filter: placeholder(window, cx, "Filter files"),
            tree,
            hidden: false,
        }
    }
}

impl Render for Explorer {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let style = cx.lace().clone();
        let w = cx.entity().downgrade();
        pane(&style)
            .child(
                row()
                    .gap(px(4.))
                    .child(lw::ToolButton::new("ex-new", "New").auto_raise())
                    .child(lw::ToolButton::new("ex-folder", "Folder").auto_raise())
                    .child(lw::ToolButton::new("ex-refresh", "Refresh").auto_raise())
                    .child(div().flex_1())
                    .child(
                        lw::ToolButton::new("ex-hidden", "Hidden")
                            .checked(self.hidden)
                            .on_click(tap(&w, |e: &mut Explorer, _, _| e.hidden = !e.hidden)),
                    ),
            )
            .child(lw::LineEdit::new("ex-filter", &self.filter))
            .child(div().flex_1().min_h_0().child(self.tree.clone()))
    }
}

// ---------------------------------------------------------------------------
// Search: a find form and a result list
// ---------------------------------------------------------------------------

struct Search {
    focus: FocusHandle,
    query: Entity<InputState>,
    replace: Entity<InputState>,
    options: [bool; 3],
    scope: usize,
    results: Entity<ItemView>,
}

dock_panel!(Search, "Search", "search");

impl Search {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let hits = [
            "geometry.rs:412  splitter_bar",
            "geometry.rs:988  render_dock",
            "edge_tabs.rs:732  lace-edge-strip",
            "sidebar.rs:120  pin",
            "sidebar.rs:244  unpin",
            "skin.rs:405  tab_edges",
            "skin.rs:610  sidebar_style",
            "themes.rs:318  frame_radius",
        ];
        let model = cx.new(|_| ItemModel::list(hits));
        Self {
            focus: cx.focus_handle(),
            query: input(window, cx, "splitter"),
            replace: placeholder(window, cx, "Replace"),
            options: [false, true, false],
            scope: 0,
            results: cx.new(|cx| ItemView::new(&model, Mode::List, cx).alternating(true).with_selected(&[1], cx)),
        }
    }
}

impl Render for Search {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let style = cx.lace().clone();
        let w = cx.entity().downgrade();
        let option = |i: usize, label: &'static str| {
            lw::CheckBox::new(SharedString::from(format!("se-opt{i}")))
                .label(label)
                .checked(self.options[i])
                .on_change(on(&w, move |s: &mut Search, v: CheckState, _, _| {
                    s.options[i] = v == CheckState::Checked
                }))
        };
        pane(&style)
            .child(lw::LineEdit::new("se-query", &self.query))
            .child(lw::LineEdit::new("se-replace", &self.replace))
            .child(
                row()
                    .gap(px(14.))
                    .child(option(0, "Case"))
                    .child(option(1, "Word"))
                    .child(option(2, "Regex")),
            )
            .child(
                row()
                    .child(
                        div().flex_1().child(
                            lw::ComboBox::new("se-scope", ["Workspace", "Open files", "Selection"])
                                .selected(Some(self.scope))
                                .on_select(on(&w, |s: &mut Search, i: usize, _, _| s.scope = i)),
                        ),
                    )
                    .child(lw::PushButton::new("se-find", "Find").default_button()),
            )
            .child(
                div()
                    .text_size(px(style.small_font_size))
                    .text_color(muted(&style))
                    .child("8 matches in 5 files"),
            )
            .child(div().flex_1().min_h_0().child(self.results.clone()))
    }
}

// ---------------------------------------------------------------------------
// Editor: a text edit under a formatting tool bar
// ---------------------------------------------------------------------------

struct Editor {
    focus: FocusHandle,
    text: Entity<TextareaState>,
    format: [bool; 3],
    font: usize,
    size: f64,
    wrap: bool,
}

dock_panel!(Editor, "Editor", "file-code");

impl Editor {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let text = "//! The Lace workbench.\n\
                    \n\
                    fn main() {\n    \
                        let tokens = lace_gpui::build(spec);\n    \
                        skin::apply(\"violet_haze\", cx);\n    \
                        lace_widgets::install_preset(\"violet_haze\", cx);\n\
                    }\n\
                    \n\
                    // Drag this tab into another group, split the centre,\n\
                    // or pin it to a sidebar from the title bar.\n";
        Self {
            focus: cx.focus_handle(),
            text: cx.new(|cx| TextareaState::new(window, cx).default_value(text)),
            format: [true, false, false],
            font: 0,
            size: 13.0,
            wrap: true,
        }
    }
}

impl Render for Editor {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let style = cx.lace().clone();
        let w = cx.entity().downgrade();
        let fmt = |i: usize, label: &'static str| {
            lw::ToolButton::new(SharedString::from(format!("ed-fmt{i}")), label)
                .auto_raise()
                .checked(self.format[i])
                .on_click(tap(&w, move |e: &mut Editor, _, _| e.format[i] = !e.format[i]))
        };
        pane(&style)
            .child(
                row()
                    .gap(px(4.))
                    .child(fmt(0, "B"))
                    .child(fmt(1, "I"))
                    .child(fmt(2, "U"))
                    .child(div().h(px(22.)).child(lw::toolbar_separator(&style, State::NORMAL, false)))
                    .child(
                        div().w(px(150.)).child(
                            lw::ComboBox::new("ed-font", ["Cascadia Mono", "Consolas", "Segoe UI"])
                                .selected(Some(self.font))
                                .on_select(on(&w, |e: &mut Editor, i: usize, _, _| e.font = i)),
                        ),
                    )
                    .child(
                        div().w(px(70.)).child(
                            lw::SpinBox::new("ed-size", self.size)
                                .range(8., 32.)
                                .on_change(on(&w, |e: &mut Editor, v: f64, _, _| e.size = v)),
                        ),
                    )
                    .child(div().flex_1())
                    .child(
                        lw::Switch::new("ed-wrap")
                            .label("Wrap")
                            .checked(self.wrap)
                            .on_change(on(&w, |e: &mut Editor, v: bool, _, _| e.wrap = v)),
                    ),
            )
            .child(div().flex_1().min_h_0().child(lw::TextEdit::new("ed-text", &self.text)))
            .child(
                row()
                    .text_size(px(style.small_font_size))
                    .child(lw::Breadcrumb::new("ed-crumbs", ["lace-gpui", "examples", "lace_workbench.rs"]))
                    .child(div().flex_1())
                    .child(lw::Tag::new("Rust").tone(Tone::Info))
                    .child(lw::Tag::new("UTF-8").tone(Tone::Neutral))
                    .child(lw::Tag::new("Saved").tone(Tone::Success)),
            )
    }
}

// ---------------------------------------------------------------------------
// Controls: every family of LaceStyle control in group boxes
// ---------------------------------------------------------------------------

struct Controls {
    focus: FocusHandle,
    toggle: bool,
    checks: [CheckState; 3],
    radio: usize,
    switch: bool,
    rating: usize,
    segment: usize,
    combo: usize,
    spins: [f64; 2],
    sliders: [f64; 2],
    line: Entity<InputState>,
    password: Entity<InputState>,
    editable: Entity<InputState>,
}

dock_panel!(Controls, "Controls", "sliders-horizontal");

impl Controls {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus: cx.focus_handle(),
            toggle: true,
            checks: [CheckState::Unchecked, CheckState::Checked, CheckState::Partial],
            radio: 1,
            switch: true,
            rating: 3,
            segment: 1,
            combo: 0,
            spins: [42.0, 0.5],
            sliders: [40.0, 70.0],
            line: input(window, cx, "Line edit"),
            password: cx.new(|cx| InputState::new(window, cx).masked(true).default_value("secret")),
            editable: input(window, cx, "Editable"),
        }
    }
}

impl Render for Controls {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let style = cx.lace().clone();
        let w = cx.entity().downgrade();

        let buttons = lw::GroupBox::new("Buttons").child(
            div()
                .flex()
                .flex_col()
                .gap(px(10.))
                .child(
                    row()
                        .child(lw::PushButton::new("c-push", "Push"))
                        .child(lw::PushButton::new("c-default", "Default").default_button())
                        .child(lw::PushButton::new("c-flat", "Flat").flat())
                        .child(
                            lw::PushButton::new("c-toggle", "Toggle")
                                .checked(self.toggle)
                                .on_click(tap(&w, |c: &mut Controls, _, _| c.toggle = !c.toggle)),
                        )
                        .child(lw::PushButton::new("c-dis", "Disabled").disabled(true)),
                )
                .child(
                    row()
                        .child(lw::PushButton::new("c-danger", "Danger").tone(Tone::Danger))
                        .child(lw::PushButton::new("c-warning", "Warning").tone(Tone::Warning))
                        .child(lw::PushButton::new("c-success", "Success").tone(Tone::Success))
                        .child(lw::PushButton::new("c-info", "Info").tone(Tone::Info)),
                )
                .child(
                    row()
                        .child(lw::ToolButton::new("c-tool", "Tool"))
                        .child(lw::ToolButton::new("c-raise", "Raise").auto_raise())
                        .child(lw::ToolButton::new("c-split", "Split").split())
                        .child(
                            lw::Segmented::new("c-seg", ["Left", "Middle", "Right"])
                                .selected(Some(self.segment))
                                .on_select(on(&w, |c: &mut Controls, i: usize, _, _| c.segment = i)),
                        ),
                )
                .child(
                    lw::ButtonRow::new("c-row")
                        .button(lw::PushButton::new("c-apply", "Apply changes").icon(|c| glyph::check_mark(c).into_any_element()))
                        .button(lw::PushButton::new("c-remove", "Remove").icon(|c| glyph::dash(c).into_any_element()))
                        .button(lw::PushButton::new("c-expand", "Expand all").icon(|c| glyph::chevron(Dir::Down, c, 0.0).into_any_element()))
                        .button(lw::PushButton::new("c-close", "Close").icon(|c| glyph::cross(c, 8.0).into_any_element())),
                ),
        );

        let check = |i: usize, label: &'static str| {
            lw::CheckBox::new(SharedString::from(format!("c-chk{i}")))
                .label(label)
                .value(self.checks[i])
                .tristate(i == 2)
                .on_change(on(&w, move |c: &mut Controls, v: CheckState, _, _| c.checks[i] = v))
        };
        let radio = |i: usize, label: &'static str| {
            lw::RadioButton::new(SharedString::from(format!("c-rad{i}")))
                .label(label)
                .checked(self.radio == i)
                .on_select(tap(&w, move |c: &mut Controls, _, _| c.radio = i))
        };
        let toggles = lw::GroupBox::new("Checks, radios, switches").child(
            div()
                .flex()
                .flex_col()
                .gap(px(12.))
                .child(
                    row()
                        .gap(px(18.))
                        .child(check(0, "Check"))
                        .child(check(1, "Checked"))
                        .child(check(2, "Partial"))
                        .child(lw::CheckBox::new("c-chk-dis").label("Disabled").disabled(true)),
                )
                .child(
                    row()
                        .gap(px(18.))
                        .child(radio(0, "Radio"))
                        .child(radio(1, "Selected"))
                        .child(radio(2, "Third"))
                        .child(lw::RadioButton::new("c-rad-dis").label("Disabled").disabled(true)),
                )
                .child(
                    row()
                        .gap(px(18.))
                        .child(
                            lw::Switch::new("c-switch")
                                .label("Switch")
                                .checked(self.switch)
                                .on_change(on(&w, |c: &mut Controls, v: bool, _, _| c.switch = v)),
                        )
                        .child(lw::Switch::new("c-switch-dis").label("Disabled").disabled(true))
                        .child(
                            lw::Rating::new("c-rating", self.rating)
                                .on_change(on(&w, |c: &mut Controls, v: usize, _, _| c.rating = v)),
                        ),
                ),
        );

        let inputs = lw::GroupBox::new("Inputs").child(
            div()
                .grid()
                .grid_cols(2)
                .gap(px(8.))
                .child(lw::LineEdit::new("c-line", &self.line))
                .child(lw::LineEdit::new("c-password", &self.password))
                .child(
                    lw::ComboBox::new("c-combo", ["Combo", "Second", "Third"])
                        .selected(Some(self.combo))
                        .on_select(on(&w, |c: &mut Controls, i: usize, _, _| c.combo = i)),
                )
                .child(lw::ComboBox::new("c-combo-ed", ["Editable", "Other"]).editable(&self.editable))
                .child(
                    lw::SpinBox::new("c-spin0", self.spins[0])
                        .range(0., 100.)
                        .on_change(on(&w, |c: &mut Controls, v: f64, _, _| c.spins[0] = v)),
                )
                .child(
                    lw::SpinBox::new("c-spin1", self.spins[1])
                        .decimals(2)
                        .step(0.05)
                        .plus_minus()
                        .on_change(on(&w, |c: &mut Controls, v: f64, _, _| c.spins[1] = v)),
                ),
        );

        let pct = self.sliders[0];
        let ranges = lw::GroupBox::new("Range").child(
            div()
                .flex()
                .flex_col()
                .gap(px(10.))
                .child(lw::Slider::new("c-sl0", self.sliders[0]).on_change(on(&w, |c: &mut Controls, v: f64, _, _| c.sliders[0] = v)))
                .child(
                    lw::Slider::new("c-sl1", self.sliders[1])
                        .ticks(lw::Ticks::Below, 10.0)
                        .on_change(on(&w, |c: &mut Controls, v: f64, _, _| c.sliders[1] = v)),
                )
                .child(lw::Slider::new("c-sl2", 30.0).disabled(true))
                .child(lw::ProgressBar::new(pct))
                .child(lw::ProgressBar::new(0.0).busy())
                .child(
                    row()
                        .gap(px(14.))
                        .child(lw::ProgressRing::new(pct / 100.0))
                        .child(lw::ProgressRing::new(self.sliders[1] / 100.0).size(28.0).no_label())
                        .child(lw::Spinner::default())
                        .child(div().text_color(muted(&style)).child(format!("{}%", pct.round()))),
                ),
        );

        scroll_pane("controls-scroll", &style).child(
            div()
                .grid()
                .grid_cols(2)
                .gap(px(10.))
                .child(buttons)
                .child(toggles)
                .child(inputs)
                .child(ranges),
        )
    }
}

// ---------------------------------------------------------------------------
// Data: a 2000-row table with widget delegates
// ---------------------------------------------------------------------------

struct Data {
    focus: FocusHandle,
    filter: Entity<InputState>,
    table: Entity<ItemView>,
    alternating: bool,
    page: usize,
}

dock_panel!(Data, "Data", "table");

impl Data {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        const LEVELS: [&str; 3] = ["low", "medium", "high"];
        let rows = (0..2000).map(|i| {
            vec![
                Value::from(format!("Sample {i:04}")),
                Value::Int((i * 37 % 100) as i64),
                Value::from(LEVELS[i % 3]),
                Value::Bool(i % 4 == 0),
                Value::Float((i * 13 % 101) as f64),
            ]
        });
        let model = cx.new(|_| ItemModel::table(["Name", "Qty", "Level", "Done", "Progress", "Action"], rows));
        let table = cx.new(|cx| {
            ItemView::new(&model, Mode::Table, cx)
                .column(0, Column::interactive(110.))
                .column(1, Column::interactive(64.).delegate(Delegate::number(0., 100., 1., 0)))
                .column(2, Column::interactive(90.).delegate(Delegate::choice(LEVELS)))
                .column(3, Column::fixed(48.).delegate(Delegate::Check))
                .column(4, Column::stretch().delegate(Delegate::Progress(100.)))
                .column(5, Column::fixed(76.).delegate(Delegate::button("Run", |_, _, _| {})))
                .selection_mode(SelectionMode::Extended)
                .sorting(true)
                .alternating(true)
                .with_selected(&[2], cx)
        });
        Self {
            focus: cx.focus_handle(),
            filter: placeholder(window, cx, "Filter rows"),
            table,
            alternating: true,
            page: 0,
        }
    }
}

impl Render for Data {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let style = cx.lace().clone();
        let w = cx.entity().downgrade();
        pane(&style)
            .child(
                row()
                    .child(div().w(px(200.)).child(lw::LineEdit::new("da-filter", &self.filter)))
                    .child(
                        lw::CheckBox::new("da-alt")
                            .label("Alternating rows")
                            .checked(self.alternating)
                            .on_change(on(&w, |d: &mut Data, v: CheckState, _, cx| {
                                d.alternating = v == CheckState::Checked;
                                let on = d.alternating;
                                d.table.update(cx, |t, cx| t.set_alternating(on, cx));
                            })),
                    )
                    .child(div().flex_1())
                    .child(lw::PushButton::new("da-export", "Export"))
                    .child(lw::PushButton::new("da-run", "Run all").default_button()),
            )
            .child(div().flex_1().min_h_0().child(self.table.clone()))
            .child(
                row()
                    .child(lw::Pagination::new("da-pages", self.page, 20).on_select(on(&w, |d: &mut Data, p: usize, _, _| d.page = p)))
                    .child(div().flex_1())
                    .child(div().text_size(px(style.small_font_size)).text_color(muted(&style)).child("2000 rows")),
            )
    }
}

// ---------------------------------------------------------------------------
// Layout: splitters, restacking and tab widgets inside a dock panel
// ---------------------------------------------------------------------------

struct Layout {
    focus: FocusHandle,
    south: usize,
    west: usize,
    tool: usize,
}

dock_panel!(Layout, "Layout", "layout-dashboard");

impl Layout {
    fn new(cx: &mut Context<Self>) -> Self {
        Self { focus: cx.focus_handle(), south: 0, west: 1, tool: 0 }
    }
}

fn demo_pane(style: &LaceStyle, title: &'static str, note: &'static str) -> impl IntoElement {
    div().size_full().p(px(3.)).child(
        framed(style)
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(4.))
            .child(title)
            .child(div().text_size(px(style.small_font_size)).text_color(muted(style)).child(note)),
    )
}

impl Render for Layout {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let style = cx.lace().clone();
        let w = cx.entity().downgrade();
        let editor = lw::Splitter::new("ly-editor")
            .vertical()
            .restack(200.)
            .pane(demo_pane(&style, "Editor", "stacked >= 200 px tall"), 2.0)
            .pane(demo_pane(&style, "Console", ""), 1.0);
        let nested = lw::Splitter::new("ly-nested")
            .restack(380.)
            .pane(demo_pane(&style, "Sidebar", "side by side >= 380 px"), 1.0)
            .pane(editor, 2.5);
        let south = lw::TabWidget::new(
            lw::TabBar::new("ly-south", ["Output", "Debug"])
                .position(lw::TabPosition::South)
                .current(self.south)
                .on_select(on(&w, |l: &mut Layout, i: usize, _, _| l.south = i)),
            div().size_full().flex().items_center().justify_center().child(["Output tab", "Debug tab"][self.south]),
        );
        let west = lw::TabWidget::new(
            lw::TabBar::new("ly-west", ["One", "Two", "Three"])
                .position(lw::TabPosition::West)
                .current(self.west)
                .on_select(on(&w, |l: &mut Layout, i: usize, _, _| l.west = i)),
            div().size_full().flex().items_center().justify_center().child(format!("West tab {}", self.west + 1)),
        );
        let toolbox = lw::ToolBox::new("ly-toolbox")
            .page("General", div().child("Tool box pages fold like Qt's QToolBox."))
            .page("Advanced", div().child("Only one page is open at a time."))
            .page("About", div().child("LaceStyle chrome inside a Lace dock."))
            .current(self.tool)
            .on_select(on(&w, |l: &mut Layout, i: usize, _, _| l.tool = i));
        scroll_pane("layout-scroll", &style)
            .child(
                div()
                    .text_color(muted(&style))
                    .child("Content splitters live inside the dock: drag them, then narrow the panel to see them restack."),
            )
            .child(div().flex_none().h(px(280.)).child(lw::GroupBox::new("Nested restacking splitter").child(nested)))
            .child(
                div()
                    .flex_none()
                    .flex()
                    .gap(px(10.))
                    .h(px(170.))
                    .child(div().flex_1().child(south))
                    .child(div().flex_1().child(west)),
            )
            .child(div().flex_none().h(px(200.)).child(toolbox))
    }
}

// ---------------------------------------------------------------------------
// Overview: status widgets
// ---------------------------------------------------------------------------

struct Overview {
    focus: FocusHandle,
    step: usize,
}

dock_panel!(Overview, "Overview", "gauge");

impl Overview {
    fn new(cx: &mut Context<Self>) -> Self {
        Self { focus: cx.focus_handle(), step: 1 }
    }
}

fn stat(style: &LaceStyle, label: &'static str, value: f64, tone: Tone) -> impl IntoElement {
    framed(style)
        .flex()
        .items_center()
        .gap(px(10.))
        .p(px(8.))
        .child(lw::ProgressRing::new(value).size(40.0))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(4.))
                .child(label)
                .child(lw::Tag::new(format!("{:.0}%", value * 100.0)).tone(tone).pill()),
        )
}

impl Render for Overview {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let style = cx.lace().clone();
        let w = cx.entity().downgrade();
        scroll_pane("overview-scroll", &style)
            .child(stat(&style, "Build", 0.92, Tone::Success))
            .child(stat(&style, "Coverage", 0.64, Tone::Warning))
            .child(stat(&style, "Contrast", 0.31, Tone::Danger))
            .child(lw::Separator::horizontal().label("Team"))
            .child(
                row()
                    .gap(px(14.))
                    .child(lw::Badge::count(7).on(lw::Avatar::new("Lace Style")))
                    .child(lw::Badge::dot().tone(Tone::Success).on(lw::Avatar::new("Gpui Kit").tone(Tone::Info)))
                    .child(lw::Badge::count(120).on(lw::Avatar::new("Weave").tone(Tone::Neutral))),
            )
            .child(
                row()
                    .child(lw::Tag::new("dock"))
                    .child(lw::Tag::new("widgets").tone(Tone::Info))
                    .child(lw::Tag::new("themes").tone(Tone::Neutral).outline())
                    .child(lw::Tag::new("v0.9.7").pill()),
            )
            .child(lw::Separator::horizontal().label("Release"))
            .child(
                lw::Stepper::new("ov-steps", ["Plan", "Build", "Test", "Ship"])
                    .current(self.step)
                    .on_select(on(&w, |o: &mut Overview, i: usize, _, _| o.step = i)),
            )
            .child(lw::Separator::horizontal().label("Loading"))
            .child(
                row()
                    .child(lw::Skeleton::circle(28.0))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(6.))
                            .child(lw::Skeleton::block(150.0, 10.0))
                            .child(lw::Skeleton::block(100.0, 10.0)),
                    ),
            )
            .child(lw::Link::new("ov-link", "Open the release notes"))
    }
}

// ---------------------------------------------------------------------------
// Properties: a form
// ---------------------------------------------------------------------------

struct Properties {
    focus: FocusHandle,
    name: Entity<InputState>,
    width: f64,
    height: f64,
    opacity: f64,
    rotation: f64,
    visible: bool,
    locked: bool,
    blend: usize,
    align: usize,
}

dock_panel!(Properties, "Properties", "settings-2");

impl Properties {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus: cx.focus_handle(),
            name: input(window, cx, "Layer 1"),
            width: 320.0,
            height: 180.0,
            opacity: 85.0,
            rotation: 30.0,
            visible: true,
            locked: false,
            blend: 0,
            align: 1,
        }
    }
}

impl Render for Properties {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let style = cx.lace().clone();
        let w = cx.entity().downgrade();
        scroll_pane("props-scroll", &style)
            .child(natural(lw::GroupBox::new("Layer").child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(field("Name", lw::LineEdit::new("pr-name", &self.name)))
                    .child(field(
                        "Width",
                        lw::SpinBox::new("pr-w", self.width).range(0., 4096.).on_change(on(&w, |p: &mut Properties, v: f64, _, _| p.width = v)),
                    ))
                    .child(field(
                        "Height",
                        lw::SpinBox::new("pr-h", self.height).range(0., 4096.).on_change(on(&w, |p: &mut Properties, v: f64, _, _| p.height = v)),
                    ))
                    .child(field(
                        "Blend",
                        lw::ComboBox::new("pr-blend", ["Normal", "Multiply", "Screen", "Overlay"])
                            .selected(Some(self.blend))
                            .on_select(on(&w, |p: &mut Properties, i: usize, _, _| p.blend = i)),
                    ))
                    .child(field(
                        "Align",
                        lw::Segmented::new("pr-align", ["Left", "Centre", "Right"])
                            .selected(Some(self.align))
                            .on_select(on(&w, |p: &mut Properties, i: usize, _, _| p.align = i)),
                    )),
            )))
            .child(natural(lw::GroupBox::new("Appearance").child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(field(
                        "Opacity",
                        lw::Slider::new("pr-opacity", self.opacity).on_change(on(&w, |p: &mut Properties, v: f64, _, _| p.opacity = v)),
                    ))
                    .child(field(
                        "Visible",
                        lw::Switch::new("pr-visible").checked(self.visible).on_change(on(&w, |p: &mut Properties, v: bool, _, _| p.visible = v)),
                    ))
                    .child(field(
                        "Locked",
                        lw::CheckBox::new("pr-locked")
                            .label("Prevent edits")
                            .checked(self.locked)
                            .on_change(on(&w, |p: &mut Properties, v: CheckState, _, _| p.locked = v == CheckState::Checked)),
                    ))
                    .child(field(
                        "Rotation",
                        div().flex().items_center().gap(px(10.)).child(
                            lw::Dial::new("pr-rot", self.rotation)
                                .range(0., 360.)
                                .diameter(56.)
                                .notches(30.)
                                .on_change(on(&w, |p: &mut Properties, v: f64, _, _| p.rotation = v)),
                        )
                        .child(format!("{:.0}°", self.rotation)),
                    )),
            )))
            .child(
                row()
                    .child(div().flex_1())
                    .child(lw::PushButton::new("pr-reset", "Reset").flat())
                    .child(lw::PushButton::new("pr-apply", "Apply").default_button()),
            )
    }
}

// ---------------------------------------------------------------------------
// Appearance: live LaceStyle tokens and the palette
// ---------------------------------------------------------------------------

struct Appearance {
    focus: FocusHandle,
    skin: Rc<GeometryDockSkin>,
    close_buttons: bool,
}

dock_panel!(Appearance, "Appearance", "palette");

impl Appearance {
    fn new(skin: Rc<GeometryDockSkin>, cx: &mut Context<Self>) -> Self {
        Self { focus: cx.focus_handle(), skin, close_buttons: true }
    }
}

/// Reinstall the active style with one token changed; every widget repaints.
fn retoken(window: &mut Window, cx: &mut App, f: impl FnOnce(&mut LaceStyle)) {
    let mut style = cx.lace().clone();
    f(&mut style);
    lace_widgets::style::install(style, cx);
    window.refresh();
}

impl Render for Appearance {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let style = cx.lace().clone();
        let w = cx.entity().downgrade();
        let pal = style.pal(State::NORMAL).clone();
        let swatches = [
            ("window", pal.window),
            ("text", pal.window_text),
            ("base", pal.base),
            ("base text", pal.text),
            ("button", pal.button),
            ("placeholder", pal.placeholder),
        ];
        scroll_pane("appearance-scroll", &style)
            .child(natural(lw::GroupBox::new("Style tokens").child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(field(
                        "Radius",
                        lw::SpinBox::new("ap-radius", style.frame_radius as f64)
                            .range(0., 16.)
                            .on_change(|v, window, cx| retoken(window, cx, |s| s.frame_radius = v as f32)),
                    ))
                    .child(field(
                        "Scrollbar",
                        lw::SpinBox::new("ap-bar", style.scrollbar_width as f64)
                            .range(4., 24.)
                            .on_change(|v, window, cx| {
                                retoken(window, cx, |s| {
                                    s.scrollbar_width = v as f32;
                                    s.scrollbar_rest = (v as f32 / 3.0).round().max(2.0);
                                })
                            }),
                    ))
                    .child(field(
                        "Tabs",
                        lw::CheckBox::new("ap-close")
                            .label("Close button on the active tab")
                            .checked(self.close_buttons)
                            .on_change(on(&w, |a: &mut Appearance, v: CheckState, _, cx| {
                                a.close_buttons = v == CheckState::Checked;
                                a.skin.set_close_button_visible(a.close_buttons, cx);
                            })),
                    )),
            )))
            .child(natural(lw::GroupBox::new(format!("Palette: {}", style.name)).child(
                div().grid().grid_cols(3).gap(px(8.)).children(swatches.into_iter().map(|(label, color)| {
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(4.))
                        .child(
                            div()
                                .h(px(28.))
                                .rounded(px(style.control_radius))
                                .border_1()
                                .border_color(hsla(rules::frame_line(&style, State::NORMAL)))
                                .bg(hsla(color)),
                        )
                        .child(div().text_size(px(style.small_font_size)).text_color(muted(&style)).child(label))
                })),
            )))
    }
}

// ---------------------------------------------------------------------------
// Navigation: tab widget, breadcrumb, pagination, stepper
// ---------------------------------------------------------------------------

struct Navigation {
    focus: FocusHandle,
    tab: usize,
    page: usize,
    step: usize,
}

dock_panel!(Navigation, "Navigation", "compass");

impl Navigation {
    fn new(cx: &mut Context<Self>) -> Self {
        Self { focus: cx.focus_handle(), tab: 0, page: 3, step: 2 }
    }
}

impl Render for Navigation {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let style = cx.lace().clone();
        let w = cx.entity().downgrade();
        let body = ["General settings", "Appearance settings", "Advanced settings"][self.tab];
        scroll_pane("nav-scroll", &style)
            .child(
                div().flex_none().h(px(120.)).child(lw::TabWidget::new(
                    lw::TabBar::new("nv-tabs", ["General", "Appearance", "Advanced"])
                        .current(self.tab)
                        .on_select(on(&w, |n: &mut Navigation, i: usize, _, _| n.tab = i)),
                    div().size_full().flex().items_center().justify_center().child(body),
                )),
            )
            .child(lw::Breadcrumb::new("nv-crumbs", ["Home", "Projects", "Lace", "Dock"]))
            .child(lw::Pagination::new("nv-pages", self.page, 9).on_select(on(&w, |n: &mut Navigation, p: usize, _, _| n.page = p)))
            .child(
                lw::Stepper::new("nv-steps", ["Seed", "Derive", "Enforce"])
                    .current(self.step)
                    .on_select(on(&w, |n: &mut Navigation, i: usize, _, _| n.step = i)),
            )
    }
}

// ---------------------------------------------------------------------------
// Console: a command line over a log
// ---------------------------------------------------------------------------

struct Console {
    focus: FocusHandle,
    command: Entity<InputState>,
    lines: Vec<(Tone, String)>,
    level: usize,
}

dock_panel!(Console, "Console", "square-terminal");

impl Console {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let lines = [
            (Tone::Info, "12:00:01  INFO   geometry skin installed"),
            (Tone::Neutral, "12:00:02  DEBUG  LaceStyle preset installed"),
            (Tone::Info, "12:00:03  INFO   13 panes in 6 groups, 2 pinned"),
            (Tone::Info, "12:00:04  INFO   v_split gaps painted as inset cards"),
            (Tone::Success, "12:00:05  OK     every pane rendered"),
        ];
        Self {
            focus: cx.focus_handle(),
            command: placeholder(window, cx, "Type a command and press Run"),
            lines: lines.into_iter().map(|(t, l)| (t, l.to_string())).collect(),
            level: 0,
        }
    }

    fn run(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.command.read(cx).value().to_string();
        if text.trim().is_empty() {
            return;
        }
        self.lines.push((Tone::Neutral, format!("> {text}")));
        self.lines.push((Tone::Success, format!("ok: {} chars", text.len())));
        self.command.update(cx, |c, cx| c.set_value("", window, cx));
    }
}

impl Render for Console {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let style = cx.lace().clone();
        let w = cx.entity().downgrade();
        let level = self.level;
        let lines = self
            .lines
            .iter()
            .filter(|(tone, _)| level == 0 || (level == 1 && *tone == Tone::Warning) || (level == 2 && *tone == Tone::Info))
            .map(|(_, line)| div().child(line.clone()));
        pane(&style)
            .child(
                row()
                    .child(div().flex_1().min_w_0().child(lw::LineEdit::new("co-cmd", &self.command)))
                    .child(lw::PushButton::new("co-run", "Run").default_button().on_click(tap(&w, |c: &mut Console, window, cx| c.run(window, cx))))
                    .child(lw::PushButton::new("co-clear", "Clear").flat().on_click(tap(&w, |c: &mut Console, _, _| c.lines.clear())))
                    .child(
                        lw::Segmented::new("co-level", ["All", "Warn", "Info"])
                            .selected(Some(self.level))
                            .on_select(on(&w, |c: &mut Console, i: usize, _, _| c.level = i)),
                    ),
            )
            .child(
                framed(&style)
                    .id("co-log")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .p(px(6.))
                    .font_family("Cascadia Mono")
                    .text_size(px(12.))
                    .children(lines),
            )
    }
}

// ---------------------------------------------------------------------------
// Problems: alerts
// ---------------------------------------------------------------------------

struct Problems {
    focus: FocusHandle,
    filter: usize,
}

dock_panel!(Problems, "Problems", "triangle-alert");

impl Problems {
    fn new(cx: &mut Context<Self>) -> Self {
        Self { focus: cx.focus_handle(), filter: 0 }
    }
}

impl Render for Problems {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let style = cx.lace().clone();
        let w = cx.entity().downgrade();
        let all = [
            (Tone::Danger, "Errors", "contrast.rs: accent misses the 3:1 floor on light_neo."),
            (Tone::Warning, "Warnings", "geometry.rs: v_split gaps stay unapplied until the axis fix."),
            (Tone::Warning, "Warnings", "sidebar.rs: rail label clipped below 60 px."),
            (Tone::Info, "Info", "Two panes start pinned to sidebars."),
            (Tone::Success, "Info", "All 43 library tests pass."),
        ];
        let filter = ["All", "Errors", "Warnings", "Info"][self.filter];
        let alerts = all
            .into_iter()
            .filter(|(_, kind, _)| filter == "All" || *kind == filter)
            .map(|(tone, kind, text)| lw::Alert::new(tone, text).title(kind));
        scroll_pane("problems-scroll", &style)
            .child(
                row()
                    .child(
                        lw::Segmented::new("pb-filter", ["All", "Errors", "Warnings", "Info"])
                            .selected(Some(self.filter))
                            .on_select(on(&w, |p: &mut Problems, i: usize, _, _| p.filter = i)),
                    )
                    .child(lw::Badge::count(1).tone(Tone::Danger).on(div().px(px(4.)).child("errors")))
                    .child(lw::Badge::count(2).tone(Tone::Warning).on(div().px(px(4.)).child("warnings"))),
            )
            .children(alerts)
    }
}

// ---------------------------------------------------------------------------
// Mixer: vertical sliders and dials
// ---------------------------------------------------------------------------

struct Mixer {
    focus: FocusHandle,
    levels: [f64; 4],
    pans: [f64; 4],
    mutes: [bool; 4],
}

dock_panel!(Mixer, "Mixer", "audio-lines");

impl Mixer {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus: cx.focus_handle(),
            levels: [70.0, 45.0, 85.0, 30.0],
            pans: [50.0, 20.0, 80.0, 50.0],
            mutes: [false, false, true, false],
        }
    }
}

impl Render for Mixer {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let style = cx.lace().clone();
        let w = cx.entity().downgrade();
        let channels = ["Drums", "Bass", "Keys", "Vocal"].into_iter().enumerate().map(|(i, name)| {
            framed(&style)
                .flex()
                .flex_col()
                .items_center()
                .gap(px(8.))
                .p(px(8.))
                .w(px(96.))
                .child(name)
                .child(
                    lw::Dial::new(SharedString::from(format!("mx-pan{i}")), self.pans[i])
                        .diameter(44.)
                        .notches(25.)
                        .on_change(on(&w, move |m: &mut Mixer, v: f64, _, _| m.pans[i] = v)),
                )
                .child(
                    div()
                        .h(px(130.))
                        .flex()
                        .gap(px(8.))
                        .child(
                            lw::Slider::new(SharedString::from(format!("mx-lvl{i}")), self.levels[i])
                                .vertical()
                                .disabled(self.mutes[i])
                                .on_change(on(&w, move |m: &mut Mixer, v: f64, _, _| m.levels[i] = v)),
                        )
                        .child(lw::ProgressBar::new(if self.mutes[i] { 0.0 } else { self.levels[i] }).vertical().thickness(6.0)),
                )
                .child(
                    lw::PushButton::new(SharedString::from(format!("mx-mute{i}")), "Mute")
                        .checked(self.mutes[i])
                        .on_click(tap(&w, move |m: &mut Mixer, _, _| m.mutes[i] = !m.mutes[i])),
                )
        });
        scroll_pane("mixer-scroll", &style).child(row().items_start().gap(px(10.)).children(channels))
    }
}

// ---------------------------------------------------------------------------
// The workbench
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
enum Preset {
    Flagship,
    Minimal,
    Stock,
}

impl Preset {
    const ALL: [Preset; 3] = [Preset::Flagship, Preset::Minimal, Preset::Stock];

    fn style(self, cx: &App) -> DockGeometryStyle {
        let theme = cx.theme();
        match self {
            Preset::Flagship => DockGeometryStyle::new()
                .gap(px(7.))
                .gap_outer(px(6.))
                .panel_radius(px(10.))
                .outline(OutlineStyle::themed(px(1.)))
                .splitter(|s| {
                    s.thickness(px(3.))
                        .length(SplitterLength::Percent(1.0))
                        .cap(SplitterCap::Rounded)
                        .colors(
                            SplitterColors::default()
                                .idle(theme.colors.background)
                                .hovered(theme.colors.primary)
                                .dragging(theme.colors.primary),
                        )
                }),
            Preset::Minimal => DockGeometryStyle::new()
                .gap(px(4.))
                .panel_radius(px(6.))
                .splitter(|s| s.thickness(px(3.)).length(SplitterLength::Percent(0.6)).cap(SplitterCap::Rounded)),
            Preset::Stock => DockGeometryStyle::default(),
        }
    }
}

struct Workbench {
    sidebars: Entity<Sidebars>,
    skin: Rc<GeometryDockSkin>,
    preset: Preset,
    theme: usize,
}

fn panel_view<P: Panel>(panel: Entity<P>) -> Arc<dyn BasePanelView> {
    Arc::new(PanelHandle::new(panel))
}

impl Workbench {
    fn new(theme: usize, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let preset = Preset::Flagship;
        let (area, skin) = GeometryDockSkin::dock_area("lace-workbench", None, preset.style(cx), window, cx);
        skin.set_close_button_visible(true, cx);

        let explorer = cx.new(|cx| Explorer::new(window, cx));
        let search = cx.new(|cx| Search::new(window, cx));
        let editor = cx.new(|cx| Editor::new(window, cx));
        let controls = cx.new(|cx| Controls::new(window, cx));
        let data = cx.new(|cx| Data::new(window, cx));
        let layout = cx.new(Layout::new);
        let overview = cx.new(Overview::new);
        let properties = cx.new(|cx| Properties::new(window, cx));
        let appearance = cx.new(|cx| Appearance::new(skin.clone(), cx));
        let navigation = cx.new(Navigation::new);
        let console = cx.new(|cx| Console::new(window, cx));
        let problems = cx.new(Problems::new);
        let mixer = cx.new(Mixer::new);

        let left = DockLayout::tabs()
            .panel_view(panel_view(explorer.clone()), cx)
            .panel_view(panel_view(search.clone()), cx)
            .active_index(0);
        let workbench = DockLayout::tabs()
            .panel_view(panel_view(editor.clone()), cx)
            .panel_view(panel_view(controls.clone()), cx)
            .panel_view(panel_view(data.clone()), cx)
            .panel_view(panel_view(layout.clone()), cx)
            .active_index(1);
        let side = DockLayout::tabs().panel_view(panel_view(overview.clone()), cx).active_index(0);
        let center = DockLayout::h_split().child(workbench, None).child(side, Some(px(280.)));
        let right = DockLayout::tabs()
            .panel_view(panel_view(properties.clone()), cx)
            .panel_view(panel_view(appearance.clone()), cx)
            .panel_view(panel_view(navigation.clone()), cx)
            .active_index(0);
        let bottom = DockLayout::tabs()
            .panel_view(panel_view(console.clone()), cx)
            .panel_view(panel_view(problems.clone()), cx)
            .panel_view(panel_view(mixer.clone()), cx)
            .active_index(0);

        area.update(cx, |area, cx| {
            area.set_center(center, window, cx);
            area.set_dock(DockPlacement::Left, left, window, cx);
            area.set_dock_size(DockPlacement::Left, px(270.), window, cx);
            area.set_dock(DockPlacement::Right, right, window, cx);
            area.set_dock_size(DockPlacement::Right, px(300.), window, cx);
            area.set_dock(DockPlacement::Bottom, bottom, window, cx);
            area.set_dock_size(DockPlacement::Bottom, px(220.), window, cx);
        });

        let pinned = [PanelId::from(navigation.entity_id()), PanelId::from(mixer.entity_id())];
        let sidebars = cx.new(|cx| {
            let mut s = Sidebars::new(area.clone(), &skin, cx);
            s.pinnable(&explorer, Some(lucide(Explorer::ICON)));
            s.pinnable(&search, Some(lucide(Search::ICON)));
            s.pinnable(&editor, Some(lucide(Editor::ICON)));
            s.pinnable(&controls, Some(lucide(Controls::ICON)));
            s.pinnable(&data, Some(lucide(Data::ICON)));
            s.pinnable(&layout, Some(lucide(Layout::ICON)));
            s.pinnable(&overview, Some(lucide(Overview::ICON)));
            s.pinnable(&properties, Some(lucide(Properties::ICON)));
            s.pinnable(&appearance, Some(lucide(Appearance::ICON)));
            s.pinnable(&navigation, Some(lucide(Navigation::ICON)));
            s.pinnable(&console, Some(lucide(Console::ICON)));
            s.pinnable(&problems, Some(lucide(Problems::ICON)));
            s.pinnable(&mixer, Some(lucide(Mixer::ICON)));
            s
        });
        sidebars.update(cx, |s, cx| {
            for id in pinned {
                s.pin(id, None, window, cx);
            }
        });

        Self { sidebars, skin, preset, theme }
    }

    fn set_theme(&mut self, ix: usize, window: &mut Window, cx: &mut App) {
        self.theme = ix % THEMES.len();
        let name = THEMES[self.theme].name;
        skin::apply(name, cx);
        lace_widgets::install_preset(name, cx);
        // Explicit splitter colours are captured when the style is built.
        self.skin.set_style(self.preset.style(cx), cx);
        window.refresh();
    }

    fn set_preset(&mut self, preset: Preset, cx: &mut App) {
        self.preset = preset;
        self.skin.set_style(preset.style(cx), cx);
    }
}

impl Render for Workbench {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let style = cx.lace().clone();
        let pal = style.pal(State::NORMAL).clone();
        let w = cx.entity().downgrade();
        let theme = self.theme;
        let preset = Preset::ALL.iter().position(|p| *p == self.preset).unwrap_or(0);

        let bar = div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(8.))
            .px(px(8.))
            .h(px(40.))
            .border_b_1()
            .border_color(hsla(rules::frame_line(&style, State::NORMAL)))
            .child(lw::MenuBarItem::new("mb-file", "File"))
            .child(lw::MenuBarItem::new("mb-edit", "Edit"))
            .child(lw::MenuBarItem::new("mb-view", "View"))
            .child(lw::MenuBarItem::new("mb-window", "Window"))
            .child(div().w(px(16.)))
            .child(
                lw::ToolButton::new("wb-prev", "<")
                    .auto_raise()
                    .on_click(tap(&w, move |b: &mut Workbench, window, cx| b.set_theme(theme + THEMES.len() - 1, window, cx))),
            )
            .child(
                div().w(px(170.)).child(
                    lw::ComboBox::new("wb-theme", THEMES.iter().map(|t| t.name))
                        .selected(Some(theme))
                        .on_select(on(&w, |b: &mut Workbench, i: usize, window, cx| b.set_theme(i, window, cx))),
                ),
            )
            .child(
                lw::ToolButton::new("wb-next", ">")
                    .auto_raise()
                    .on_click(tap(&w, move |b: &mut Workbench, window, cx| b.set_theme(theme + 1, window, cx))),
            )
            .child(div().w(px(16.)))
            .child(
                lw::Segmented::new("wb-preset", ["Flagship", "Minimal", "Stock"])
                    .selected(Some(preset))
                    .on_select(on(&w, |b: &mut Workbench, i: usize, _, cx| b.set_preset(Preset::ALL[i], cx))),
            )
            .child(div().flex_1())
            .child(
                div()
                    .text_size(px(style.small_font_size))
                    .text_color(muted(&style))
                    .child("13 panes · drag tabs, split, pin to the sidebars"),
            );

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(hsla(pal.window))
            .text_color(hsla(pal.window_text))
            .text_size(px(style.font_size))
            .font_family("Segoe UI")
            .child(bar)
            .child(div().flex_1().min_h_0().overflow_hidden().child(self.sidebars.clone()))
    }
}

fn main() {
    let theme = std::env::args()
        .nth(1)
        .and_then(|name| THEMES.iter().position(|t| t.name == name))
        .or_else(|| THEMES.iter().position(|t| t.name == "violet_haze"))
        .unwrap_or(0);
    gpui_kit::application().with_assets(gpui_kit::assets::AllAssets).run(move |cx: &mut App| {
        gpui_kit::init(cx);
        skin::apply(THEMES[theme].name, cx);
        lace_widgets::install_preset(THEMES[theme].name, cx);

        let bounds = Bounds::centered(None, size(px(1500.0), px(960.0)), cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions {
                title: Some("Lace workbench".into()),
                ..Default::default()
            }),
            ..Default::default()
        };
        gpui_kit::open_window(options, cx, |window, cx| cx.new(|cx| Workbench::new(theme, window, cx)))
            .expect("failed to open the workbench window");
    });
}
