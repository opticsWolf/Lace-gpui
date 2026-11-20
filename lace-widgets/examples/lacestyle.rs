//! LaceStyle in gpui: every primitive in every state (the Qt gallery's
//! sheet), the S5 widget showcase, and the item-view / splitter / dial page.
//!
//! ```text
//! cargo run --release --example lacestyle -- [theme] [gallery|widgets|views]
//! ```

use gpui_kit::component::input::{InputState, Textarea, TextareaState};
use gpui_kit::component::Sizable;
use gpui_kit::gpui::{
    div, point, prelude::*, px, size, AnyElement, App, Bounds, Context, Entity, SharedString,
    TitlebarOptions, WeakEntity, Window, WindowBounds, WindowOptions,
};
use lace_widgets::controls::*;
use lace_widgets::glyph::{self, Dir};
use lace_widgets::style::paint::focus_ring;
use lace_widgets::style::rules;
use lace_widgets::style::{hsla, ActiveStyle, LaceStyle, State};
use lace_widgets::views::{Column, Delegate, ItemModel, ItemView, Mode, SelectionMode, Value};

#[path = "lacestyle_parts/kit.rs"]
mod kit;

/// Every Lace preset: Basics, Basics Neo, Editor Classics, Neon, Edge Treatments.
const THEMES: &[&lace_gpui::engine::ThemeSpec] = &lace_gpui::themes::ALL;
const PAGES: [&str; 5] = ["Gallery", "Widgets", "Views", "Splitters", "Kit"];

const LABEL_W: f32 = 120.0;
const CELL_W: f32 = 76.0;
const CELL_H: f32 = 40.0;

const ROWS: [&str; 48] = [
    "push button", "default button", "flat button", "tool button", "tool auto-raise", "tool split",
    "line edit", "combo box", "combo editable", "spin box", "slider", "slider ticks", "progress",
    "progress busy", "tab", "group box", "header", "menu item", "menu bar item", "item view",
    "tooltip", "tool box", "check box", "partial check", "radio", "item check", "menu check",
    "branch open", "branch shut", "arrow down", "arrow right", "spin up", "spin plus", "frame",
    "line edit frame", "focus rect", "scrollbar h", "scrollbar v", "expanding h", "splitter", "dial",
    "tab close", "frame line", "frame box", "toolbar handle", "toolbar separator", "size grip", "",
];

fn columns() -> [(&'static str, State); 6] {
    [
        ("normal", State::NORMAL),
        ("hover", State::hover()),
        ("pressed", State::pressed()),
        ("focused", State::focused()),
        ("checked", State::checked()),
        ("disabled", State::disabled()),
    ]
}

/// The tree page's rows: (label, depth, parent index, mode, size).
const TREE: [(&str, usize, Option<usize>, &str, &str); 11] = [
    ("src", 0, None, "read_write", "3 kB"),
    ("src/file_0.rs", 1, Some(0), "read", "1 kB"),
    ("src/file_1.rs", 1, Some(0), "read", "2 kB"),
    ("src/file_2.rs", 1, Some(0), "read", "3 kB"),
    ("docs", 0, None, "read_write", "4 kB"),
    ("docs/file_0.rs", 1, Some(4), "read", "1 kB"),
    ("docs/file_1.rs", 1, Some(4), "read", "2 kB"),
    ("docs/file_2.rs", 1, Some(4), "read", "3 kB"),
    ("tests", 0, None, "read_write", "5 kB"),
    ("tests/file_0.rs", 1, Some(8), "read", "1 kB"),
    ("tests/file_1.rs", 1, Some(8), "read", "2 kB"),
];

struct Gallery {
    theme: usize,
    page: usize,
    g_edits: Vec<Entity<InputState>>,
    g_combos: Vec<Entity<InputState>>,
    line: Entity<InputState>,
    read_only: Entity<InputState>,
    disabled_edit: Entity<InputState>,
    editable: Entity<InputState>,
    plain: Entity<TextareaState>,
    rich: Entity<TextareaState>,
    toggle: bool,
    checks: [CheckState; 3],
    radio: usize,
    combo: usize,
    spins: [f64; 4],
    sliders: [f64; 3],
    dials: [f64; 3],
    tab: usize,
    south: usize,
    west: usize,
    tool_page: usize,
    table_scroll: f64,
    tree_checks: [bool; 11],
    expanded: [bool; 3],
    tree_sel: Option<usize>,
    list_sel: usize,
    tree_view: Entity<ItemView>,
    table_view: Entity<ItemView>,
    list_view: Entity<ItemView>,
    /// Live overrides of the style's frame radius and scrollbar width,
    /// kept across theme switches.
    frame_radius: f64,
    bar_width: f64,
    alternating: bool,
    kit: Entity<kit::Kit>,
}

/// A checkable project tree: folders over files, 3 levels.
fn tree_view(cx: &mut Context<Gallery>) -> Entity<ItemView> {
    let model = cx.new(|_| {
        let mut m = ItemModel::new(["Name", "Size", "Kind"]);
        let folders = [("src", 6), ("tests", 4), ("examples", 3), ("docs", 5), ("assets", 8)];
        for (fi, (folder, files)) in folders.into_iter().enumerate() {
            let f = m.add_row(None, vec![folder.into(), Value::Empty, "folder".into()]);
            m.set_checkable(f, Some(CheckState::Unchecked));
            for i in 0..files {
                let name = format!("{folder}_{i}.rs");
                let size = ((fi * 7 + i * 13) % 40 + 1) as i64;
                let r = m.add_row(Some(f), vec![name.into(), Value::Int(size), "file".into()]);
                m.set_checkable(r, Some(if (i + fi) % 3 == 0 { CheckState::Checked } else { CheckState::Unchecked }));
                if i == 1 {
                    for k in 0..3 {
                        let leaf = m.add_row(Some(r), vec![format!("item_{k}").into(), Value::Int(k as i64 + 1), "symbol".into()]);
                        m.set_checkable(leaf, Some(CheckState::Unchecked));
                    }
                }
            }
            m.set_expanded(f, fi < 2);
        }
        m.sync_tristate();
        m
    });
    cx.new(|cx| {
        ItemView::new(&model, Mode::Tree, cx)
            .column(0, Column::interactive(300.))
            .column(1, Column::interactive(70.).delegate(Delegate::number(0., 1000., 1., 0)))
            .column(2, Column::stretch().delegate(Delegate::choice(["folder", "file", "symbol"])))
            .selection_mode(SelectionMode::Extended)
            .sorting(true)
            .alternating(true)
            .with_selected(&[1, 2], cx)
    })
}

/// A 2000-row table with a widget delegate in every column.
fn table_view(cx: &mut Context<Gallery>) -> Entity<ItemView> {
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
    cx.new(|cx| {
        ItemView::new(&model, Mode::Table, cx)
            .column(0, Column::interactive(110.))
            .column(1, Column::interactive(64.).delegate(Delegate::number(0., 100., 1., 0)))
            .column(2, Column::interactive(90.).delegate(Delegate::choice(LEVELS)))
            .column(3, Column::fixed(48.).delegate(Delegate::Check))
            .column(4, Column::stretch().delegate(Delegate::Progress(100.)))
            .column(5, Column::fixed(76.).delegate(Delegate::button("Run", |id, _, _| println!("run row {id}"))))
            .selection_mode(SelectionMode::Extended)
            .sorting(true)
            .with_selected(&[2], cx)
    })
}

fn list_view(cx: &mut Context<Gallery>) -> Entity<ItemView> {
    let items = ["Inbox", "Drafts", "Sent", "Archive", "Spam", "Trash", "Projects", "Receipts", "Travel", "Family"];
    let model = cx.new(|_| ItemModel::list(items));
    cx.new(|cx| ItemView::new(&model, Mode::List, cx).alternating(true).with_selected(&[2], cx))
}

fn input(window: &mut Window, cx: &mut App, text: &str) -> Entity<InputState> {
    let text = SharedString::from(text.to_string());
    cx.new(|cx| InputState::new(window, cx).default_value(text))
}

impl Gallery {
    fn new(theme: usize, page: usize, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let g_edits = (0..6).map(|_| input(window, cx, "Text")).collect();
        let g_combos = (0..6).map(|_| input(window, cx, "Edit")).collect();
        Self {
            theme,
            page,
            g_edits,
            g_combos,
            line: input(window, cx, "Line edit"),
            read_only: input(window, cx, "Read-only"),
            disabled_edit: input(window, cx, "Disabled"),
            editable: input(window, cx, "Editable"),
            plain: cx.new(|cx| TextareaState::new(window, cx).default_value("Plain text edit\nsecond line")),
            rich: cx.new(|cx| TextareaState::new(window, cx).default_value("Rich text edit")),
            toggle: true,
            checks: [CheckState::Unchecked, CheckState::Checked, CheckState::Partial],
            radio: 1,
            combo: 0,
            spins: [0.0, 0.0, 0.0, 99.0],
            sliders: [40.0, 70.0, 30.0],
            dials: [15.0, 80.0, 50.0],
            tab: 0,
            south: 0,
            west: 0,
            tool_page: 0,
            table_scroll: 0.0,
            tree_checks: [false, true, false, true, false, true, false, true, false, true, true],
            expanded: [true, true, false],
            tree_sel: Some(1),
            list_sel: 1,
            tree_view: tree_view(cx),
            table_view: table_view(cx),
            list_view: list_view(cx),
            frame_radius: cx.lace().frame_radius as f64,
            bar_width: cx.lace().scrollbar_width as f64,
            alternating: true,
            kit: cx.new(|cx| kit::Kit::new(window, cx)),
        }
    }

    fn set_theme(&mut self, ix: usize, cx: &mut App) {
        self.theme = ix % THEMES.len();
        let name = THEMES[self.theme].name;
        lace_gpui::skin::apply(name, cx);
        lace_widgets::install_preset(name, cx);
        // Each theme brings its own radii; the header fields follow them.
        self.frame_radius = cx.lace().frame_radius as f64;
        self.apply_tokens(cx);
    }

    /// Alternating row colours on every item view of the page.
    fn set_alternating(&mut self, on: bool, cx: &mut App) {
        self.alternating = on;
        for v in [&self.tree_view, &self.table_view, &self.list_view] {
            v.update(cx, |v, cx| v.set_alternating(on, cx));
        }
    }

    /// Reinstall the style with the header's radius and bar width.
    fn apply_tokens(&self, cx: &mut App) {
        let mut style = cx.lace().clone();
        style.frame_radius = self.frame_radius as f32;
        style.scrollbar_width = self.bar_width as f32;
        style.scrollbar_rest = (self.bar_width as f32 / 3.0).round().max(2.0);
        lace_widgets::style::install(style, cx);
    }
}

/// A value callback that updates the gallery with access to the app.
fn act_v<T: 'static>(
    w: &WeakEntity<Gallery>,
    f: impl Fn(&mut Gallery, T, &mut App) + 'static,
) -> impl Fn(T, &mut Window, &mut App) + 'static {
    let w = w.clone();
    move |v, _, cx| {
        let _ = w.update(cx, |g, cx| {
            f(g, v, cx);
            cx.notify();
        });
    }
}

/// A spin-box callback that sets a style token and reinstalls the style.
fn token(w: &WeakEntity<Gallery>, f: impl Fn(&mut Gallery, f64) + 'static) -> impl Fn(f64, &mut Window, &mut App) + 'static {
    let w = w.clone();
    move |v, _, cx| {
        let _ = w.update(cx, |g, cx| {
            f(g, v);
            g.apply_tokens(cx);
            cx.notify();
        });
    }
}

/// A value callback that updates the gallery and repaints.
fn set<T: 'static>(
    w: &WeakEntity<Gallery>,
    f: impl Fn(&mut Gallery, T) + 'static,
) -> impl Fn(T, &mut Window, &mut App) + 'static {
    let w = w.clone();
    move |v, _, cx| {
        let _ = w.update(cx, |g, cx| {
            f(g, v);
            cx.notify();
        });
    }
}

/// A click callback that updates the gallery and repaints.
fn act(w: &WeakEntity<Gallery>, f: impl Fn(&mut Gallery, &mut App) + 'static) -> impl Fn(&mut Window, &mut App) + 'static {
    let w = w.clone();
    move |_, cx| {
        let _ = w.update(cx, |g, cx| {
            f(g, cx);
            cx.notify();
        });
    }
}

fn boxed(w: f32, h: f32) -> gpui_kit::gpui::Div {
    div().w(px(w)).h(px(h)).flex().items_center().justify_center()
}

fn glyph_cell(f: impl FnOnce(Bounds<gpui_kit::gpui::Pixels>, &mut Window, &mut App) + 'static) -> AnyElement {
    glyph::glyph_box(14.0, 14.0, f).into_any_element()
}

// ---------------------------------------------------------------------------
// Gallery page
// ---------------------------------------------------------------------------

impl Gallery {
    fn cell(&self, row: &str, col: usize, s: State, style: &LaceStyle) -> AnyElement {
        let id = SharedString::from(format!("g-{row}-{col}"));
        let sel = State { selected: s.checked, checked: false, ..s };
        match row {
            "push button" => PushButton::new(id, "Button").state(s).min_width(66.0).into_any_element(),
            "default button" => PushButton::new(id, "OK").default_button().state(s).min_width(66.0).into_any_element(),
            "flat button" => PushButton::new(id, "Flat").flat().state(s).min_width(66.0).into_any_element(),
            "tool button" => ToolButton::new(id, "T").state(s).into_any_element(),
            "tool auto-raise" => ToolButton::new(id, "T").auto_raise().state(s).into_any_element(),
            "tool split" => ToolButton::new(id, "T").split().state(s).into_any_element(),
            "line edit" => div()
                .w(px(66.))
                .child(LineEdit::new(id, &self.g_edits[col]).state(s).disabled(s.disabled))
                .into_any_element(),
            "combo box" => div()
                .w(px(66.))
                .child(ComboBox::new(id, ["Item"]).selected(Some(0)).state(s))
                .into_any_element(),
            "combo editable" => div()
                .w(px(66.))
                .child(ComboBox::new(id, ["Edit"]).editable(&self.g_combos[col]).state(s).disabled(s.disabled))
                .into_any_element(),
            "spin box" => div().w(px(66.)).child(SpinBox::new(id, 42.0).state(s)).into_any_element(),
            "slider" => div().w(px(66.)).child(Slider::new(id, 40.0).state(s)).into_any_element(),
            "slider ticks" => div()
                .w(px(66.))
                .child(Slider::new(id, 40.0).ticks(Ticks::Below, 10.0).state(s))
                .into_any_element(),
            "progress" => div()
                .w(px(66.))
                .child(ProgressBar::new(60.0).thickness(6.0).disabled(s.disabled))
                .into_any_element(),
            "progress busy" => div()
                .w(px(66.))
                .child(ProgressBar::new(0.0).busy().still().thickness(6.0).disabled(s.disabled))
                .into_any_element(),
            "tab" => tab_div(style, sel, TabPosition::North).w(px(66.)).px_0().child("Tab").into_any_element(),
            "group box" => frame_box(style, s).w(px(66.)).h(px(16.)).into_any_element(),
            "header" => header_section(style, s, "Name".into(), Some(Sort::Descending)).w(px(66.)).into_any_element(),
            "menu item" => div().w(px(72.)).child(MenuItem::new(id, "Open").state(s)).into_any_element(),
            "menu bar item" => MenuBarItem::new(id, "File").state(s).into_any_element(),
            "item view" => item_row_div(style, sel).w(px(66.)).h(px(22.)).px(px(4.)).child("Row").into_any_element(),
            "tooltip" => tooltip(style, s, "Tip").into_any_element(),
            "tool box" => tool_box_tab(style, s, "Pa...".into()).w(px(66.)).into_any_element(),
            "check box" | "item check" => check_indicator(style, s, check::INDICATOR).into_any_element(),
            "partial check" => check_indicator(style, s.with_partial(true), check::INDICATOR).into_any_element(),
            "radio" => radio_indicator(style, s, check::INDICATOR).into_any_element(),
            "menu check" => {
                let c = hsla(rules::menu_check(style, s));
                glyph_cell(move |b, window, _| glyph::paint_check(window, glyph::centered(b, 12.0), c))
            }
            "branch open" | "branch shut" => {
                let c = hsla(rules::branch(style, s));
                let dir = if row == "branch open" { Dir::Down } else { Dir::Right };
                glyph_cell(move |b, window, _| glyph::paint_chevron(window, b, dir, c, 0.0))
            }
            "arrow down" | "arrow right" => {
                let c = hsla(rules::arrow(style, s));
                let dir = if row == "arrow down" { Dir::Down } else { Dir::Right };
                glyph_cell(move |b, window, _| glyph::paint_chevron(window, b, dir, c, 0.0))
            }
            "spin up" => {
                let c = hsla(rules::field_glyph(style, s, false, true));
                glyph_cell(move |b, window, _| glyph::paint_chevron(window, b, Dir::Up, c, 7.0))
            }
            "spin plus" => {
                let c = hsla(rules::field_glyph(style, s, false, true));
                glyph_cell(move |b, window, _| glyph::paint_plus_minus(window, b, true, c))
            }
            "frame" => frame_box(style, s).w(px(52.)).h(px(26.)).into_any_element(),
            "line edit frame" => field_div(style, s).w(px(52.)).h(px(26.)).into_any_element(),
            "focus rect" => {
                let f = State { focused: true, ..s };
                div()
                    .w(px(48.))
                    .h(px(22.))
                    .rounded(px(style.control_radius + 2.0))
                    .border(px(style.focus_width))
                    .border_color(hsla(focus_ring(style, f)))
                    .into_any_element()
            }
            "scrollbar h" | "expanding h" => div()
                .w(px(68.))
                .child(Scrollbar::new(id, 30.0, 100.0, 40.0).state(s, s.hovered || s.pressed))
                .into_any_element(),
            "scrollbar v" => div()
                .h(px(36.))
                .child(Scrollbar::new(id, 30.0, 100.0, 40.0).vertical().state(s, s.hovered || s.pressed))
                .into_any_element(),
            "splitter" => div().w(px(11.)).h(px(36.)).child(splitter_grip(style, s, true)).into_any_element(),
            "dial" => Dial::new(id, 30.0).diameter(36.0).notches(10.0).state(s).disabled(s.disabled).into_any_element(),
            "tab close" => TabClose::new(id).state(s).disabled(s.disabled).into_any_element(),
            "frame line" => div().w(px(52.)).child(frame_line(style, s, false)).into_any_element(),
            "frame box" => frame_box(style, s).w(px(52.)).h(px(22.)).into_any_element(),
            "toolbar handle" => toolbar_handle(style, s, false).h(px(24.)).into_any_element(),
            "toolbar separator" => toolbar_separator(style, s, false).h(px(24.)).into_any_element(),
            "size grip" => size_grip(style, s).into_any_element(),
            _ => div().into_any_element(),
        }
    }

    fn gallery(&self, style: &LaceStyle) -> AnyElement {
        let muted = hsla(lace_widgets::style::paint::muted(style, State::NORMAL));
        let column = |rows: &[&str]| {
            let header = div()
                .flex()
                .h(px(22.))
                .child(div().w(px(LABEL_W)))
                .children(columns().map(|(name, _)| boxed(CELL_W, 22.0).text_color(muted).child(name)));
            let body = rows.iter().filter(|r| !r.is_empty()).map(|row| {
                div()
                    .flex()
                    .h(px(CELL_H))
                    .child(div().w(px(LABEL_W)).pl(px(6.)).flex().items_center().child(row.to_string()))
                    .children(
                        columns()
                            .into_iter()
                            .enumerate()
                            .map(|(i, (_, s))| boxed(CELL_W, CELL_H).child(self.cell(row, i, s, style))),
                    )
            });
            div().flex().flex_col().child(header).children(body)
        };
        div()
            .flex()
            .flex_col()
            .gap(px(4.))
            .p(px(8.))
            .child(format!("{} - LaceStyle (gpui)", style.name))
            .child(
                div()
                    .flex()
                    .gap(px(24.))
                    .child(column(&ROWS[0..16]))
                    .child(column(&ROWS[16..32]))
                    .child(column(&ROWS[32..48])),
            )
            .into_any_element()
    }
}

// ---------------------------------------------------------------------------
// Widgets page
// ---------------------------------------------------------------------------

impl Gallery {
    fn widgets(&self, style: &LaceStyle, w: &WeakEntity<Self>) -> AnyElement {
        let buttons = GroupBox::new("Buttons").child(
            div()
                .flex()
                .flex_col()
                .gap(px(14.))
                .py(px(8.))
                .child(
                    div()
                        .flex()
                        .gap(px(12.))
                        .child(PushButton::new("push", "Push").min_width(100.))
                        .child(PushButton::new("default", "Default").default_button().min_width(100.))
                        .child(PushButton::new("flat", "Flat").flat().min_width(100.))
                        .child(
                            PushButton::new("toggle", "Toggle")
                                .checked(self.toggle)
                                .min_width(100.)
                                .on_click(act(w, |g, _| g.toggle = !g.toggle)),
                        )
                        .child(PushButton::new("pdis", "Disabled").disabled(true).min_width(100.)),
                )
                .child(
                    div()
                        .flex()
                        .gap(px(12.))
                        .child(ToolButton::new("tool", "Tool"))
                        .child(div().w(px(80.)).child(ToolButton::new("tool-raise", "Tool").auto_raise()))
                        .child(ToolButton::new("tool-split", "Tool").split())
                        .child(ToolButton::new("tool-checked", "Tool").checked(true)),
                ),
        );

        let combo_w = w.clone();
        let inputs = GroupBox::new("Inputs").child(
            div()
                .grid()
                .grid_cols(3)
                .gap(px(8.))
                .child(LineEdit::new("le", &self.line))
                .child(LineEdit::new("le-ro", &self.read_only).read_only(true))
                .child(LineEdit::new("le-dis", &self.disabled_edit).disabled(true))
                .child(
                    ComboBox::new("combo", ["Combo", "Second", "Third"])
                        .selected(Some(self.combo))
                        .on_select(set(&combo_w, |g, i| g.combo = i)),
                )
                .child(ComboBox::new("combo-ed", ["Editable", "Other"]).editable(&self.editable))
                .child(SpinBox::new("spin0", self.spins[0]).on_change(set(w, |g, v| g.spins[0] = v)))
                .child(
                    SpinBox::new("spin1", self.spins[1])
                        .decimals(2)
                        .step(0.1)
                        .plus_minus()
                        .on_change(set(w, |g, v| g.spins[1] = v)),
                )
                .child(SpinBox::new("spin2", self.spins[2]).no_buttons().on_change(set(w, |g, v| g.spins[2] = v)))
                .child(
                    SpinBox::new("spin3", self.spins[3])
                        .range(0., 3600.)
                        .suffix(" s")
                        .on_change(set(w, |g, v| g.spins[3] = v)),
                )
                .child(SpinBox::new("spin-ro", self.spins[0]).read_only(true).on_change(set(w, |_, _| {}))),
        );

        let check = |i: usize, label: &'static str| {
            CheckBox::new(SharedString::from(format!("chk{i}")))
                .label(label)
                .value(self.checks[i])
                .tristate(i == 2)
                .on_change(set(w, move |g, v| g.checks[i] = v))
        };
        let radio = |i: usize, label: &'static str| {
            RadioButton::new(SharedString::from(format!("rad{i}")))
                .label(label)
                .checked(self.radio == i)
                .on_select(act(w, move |g, _| g.radio = i))
        };
        let checks = GroupBox::new("Checks").child(
            div()
                .flex()
                .flex_col()
                .gap(px(30.))
                .py(px(20.))
                .child(
                    div()
                        .flex()
                        .gap(px(40.))
                        .child(check(0, "Check"))
                        .child(check(1, "Checked"))
                        .child(check(2, "Partial"))
                        .child(CheckBox::new("chk-dis").label("Disabled").disabled(true)),
                )
                .child(
                    div()
                        .flex()
                        .gap(px(40.))
                        .child(radio(0, "Radio"))
                        .child(radio(1, "Selected"))
                        .child(div().w(px(80.)))
                        .child(RadioButton::new("rad-dis").label("Disabled").disabled(true)),
                ),
        );

        let pct = self.sliders[0];
        let range = GroupBox::new("Range").child(
            div()
                .flex()
                .flex_col()
                .gap(px(6.))
                .child(Slider::new("sl0", self.sliders[0]).on_change(set(w, |g, v| g.sliders[0] = v)))
                .child(
                    Slider::new("sl1", self.sliders[1])
                        .ticks(Ticks::Below, 10.0)
                        .on_change(set(w, |g, v| g.sliders[1] = v)),
                )
                .child(Slider::new("sl2", self.sliders[2]).disabled(true))
                .child(
                    div()
                        .relative()
                        .h(px(28.))
                        .child(ProgressBar::new(pct).thickness(28.0))
                        .child(
                            div()
                                .absolute()
                                .inset_0()
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(format!("{}%", pct.round())),
                        ),
                )
                .child(ProgressBar::new(0.0).busy().thickness(28.0)),
        );

        let tabs = TabWidget::new(
            TabBar::new("tabs", ["Table", "Tree", "Tool box", "Off"])
                .current(self.tab)
                .disable_tab(3)
                .on_select(set(w, |g, i| g.tab = i)),
            match self.tab {
                0 => self.table(style, w).into_any_element(),
                1 => self.tree(style, w, 6).into_any_element(),
                _ => ToolBox::new("toolbox")
                    .page("Page one", div().child("First page"))
                    .page("Page two", div().child("Second page"))
                    .page("Page three", div().child("Third page"))
                    .current(self.tool_page)
                    .on_select(set(w, |g, i| g.tool_page = i))
                    .into_any_element(),
            },
        );

        let south = TabWidget::new(
            TabBar::new("south", ["One", "Two"])
                .position(TabPosition::South)
                .current(self.south)
                .on_select(set(w, |g, i| g.south = i)),
            div().size_full().flex().items_center().child("South"),
        );
        let west = TabWidget::new(
            TabBar::new("west", ["One", "Two"])
                .position(TabPosition::West)
                .current(self.west)
                .on_select(set(w, |g, i| g.west = i)),
            div().size_full().flex().items_center().child("West"),
        );

        div()
            .flex()
            .flex_col()
            .gap(px(10.))
            .p(px(12.))
            .child(div().flex().gap(px(10.)).child(div().flex_1().child(buttons)).child(div().flex_1().child(inputs)))
            .child(div().flex().gap(px(10.)).child(div().flex_1().child(checks)).child(div().flex_1().child(range)))
            .child(div().h(px(200.)).child(tabs))
            .child(
                div()
                    .flex()
                    .gap(px(10.))
                    .h(px(150.))
                    .child(div().flex_1().child(south))
                    .child(div().flex_1().child(west)),
            )
            .into_any_element()
    }

    /// A small table: header, numbered rows, a vertical scrollbar.
    fn table(&self, style: &LaceStyle, w: &WeakEntity<Self>) -> impl IntoElement {
        let pal = style.pal(State::NORMAL).clone();
        let line = hsla(rules::frame_line(style, State::NORMAL));
        let total = 6usize;
        let visible = 4.0;
        let first = self.table_scroll.round() as usize;
        let rows = (first..total.min(first + 5)).map(|r| {
            let v = 5 - r;
            div()
                .flex()
                .h(px(36.))
                .border_b_1()
                .border_color(line)
                .child(
                    div()
                        .w(px(16.))
                        .h_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(hsla(pal.button))
                        .child(format!("{}", r + 1)),
                )
                .children((0..3).map(move |c| {
                    div()
                        .flex_1()
                        .h_full()
                        .px(px(4.))
                        .flex()
                        .items_center()
                        .border_r_1()
                        .border_color(line)
                        .child(format!("{v}.{c}"))
                }))
        });
        div()
            .size_full()
            .flex()
            .bg(hsla(pal.base))
            .text_color(hsla(pal.text))
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .child(
                        div()
                            .flex()
                            .child(header_section(style, State::NORMAL, "".into(), None).w(px(16.)))
                            .child(header_section(style, State::NORMAL, "Name".into(), Some(Sort::Ascending)).flex_1().justify_center())
                            .child(header_section(style, State::NORMAL, "Size".into(), None).flex_1())
                            .child(header_section(style, State::NORMAL, "Type".into(), None).flex_1()),
                    )
                    .children(rows),
            )
            .child(
                Scrollbar::new("table-sb", self.table_scroll, total as f64 - visible, visible)
                    .vertical()
                    .on_scroll(set(w, |g, v| g.table_scroll = v)),
            )
    }

    /// The checkbox tree, `rows` rows tall at least.
    fn tree(&self, style: &LaceStyle, w: &WeakEntity<Self>, min_rows: usize) -> impl IntoElement {
        let pal = style.pal(State::NORMAL).clone();
        let row_h = 20.0;
        let mut body = div().flex().flex_col();
        let mut shown = 0;
        for (i, &(label, depth, parent, mode, sz)) in TREE.iter().enumerate() {
            if let Some(p) = parent {
                let top = [0, 4, 8].iter().position(|&t| t == p).unwrap_or(0);
                if !self.expanded[top] {
                    continue;
                }
            }
            shown += 1;
            let check = if depth == 0 {
                let kids: Vec<bool> =
                    TREE.iter().enumerate().filter(|(_, r)| r.2 == Some(i)).map(|(k, _)| self.tree_checks[k]).collect();
                if kids.iter().all(|&c| c) {
                    CheckState::Checked
                } else if kids.iter().any(|&c| c) {
                    CheckState::Partial
                } else {
                    CheckState::Unchecked
                }
            } else if self.tree_checks[i] {
                CheckState::Checked
            } else {
                CheckState::Unchecked
            };
            let s = State {
                selected: self.tree_sel == Some(i),
                checked: check == CheckState::Checked,
                partial: check == CheckState::Partial,
                ..State::NORMAL
            };
            let rid = SharedString::from(format!("tree{i}"));
            let branch = hsla(rules::branch(style, State::NORMAL));
            let top = [0, 4, 8].iter().position(|&t| t == i);
            let open = top.map(|t| self.expanded[t]).unwrap_or(false);
            let toggle_w = w.clone();
            let check_w = w.clone();
            let sel_w = w.clone();
            let row = item_row_div(style, State { checked: false, partial: false, ..s })
                .id(rid)
                .h(px(row_h))
                .child(
                    div()
                        .w(px(18.0 + depth as f32 * 22.0))
                        .h_full()
                        .flex()
                        .justify_end()
                        .when_some(top, |d, t| {
                            d.child(
                                div()
                                    .id(SharedString::from(format!("branch{t}")))
                                    .w(px(18.))
                                    .h_full()
                                    .child(glyph::chevron(if open { Dir::Down } else { Dir::Right }, branch, 0.0))
                                    .on_click(move |_, _, cx| {
                                        let _ = toggle_w.update(cx, |g, cx| {
                                            g.expanded[t] = !g.expanded[t];
                                            cx.notify();
                                        });
                                    }),
                            )
                        }),
                )
                .child(
                    div()
                        .id(SharedString::from(format!("tcheck{i}")))
                        .px(px(4.))
                        .child(check_indicator(style, s, check::INDICATOR))
                        .on_click(move |_, _, cx| {
                            let _ = check_w.update(cx, |g, cx| {
                                if depth == 0 {
                                    let on = check != CheckState::Checked;
                                    for k in 0..TREE.len() {
                                        if TREE[k].2 == Some(i) {
                                            g.tree_checks[k] = on;
                                        }
                                    }
                                } else {
                                    g.tree_checks[i] = !g.tree_checks[i];
                                }
                                cx.notify();
                            });
                        }),
                )
                .child(div().w(px(180.0 - depth as f32 * 22.0)).child(label))
                .child(div().w(px(120.)).child(mode))
                .child(div().w(px(80.)).child(sz))
                .on_click(move |_, _, cx| {
                    let _ = sel_w.update(cx, |g, cx| {
                        g.tree_sel = Some(i);
                        cx.notify();
                    });
                });
            body = body.child(row);
        }
        let _ = (shown, min_rows);
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(hsla(pal.base))
            .border_1()
            .border_color(hsla(rules::frame_line(style, State::NORMAL)))
            .rounded(px(style.frame_radius))
            .p(px(style.frame_inset(style.frame_radius)))
            .overflow_hidden()
            .child(
                div()
                    .flex()
                    .child(header_section(style, State::NORMAL, "Path".into(), None).w(px(18.0 + 22.0 + 180.0)))
                    .child(header_section(style, State::NORMAL, "Mode".into(), None).w(px(120.)))
                    .child(header_section(style, State::NORMAL, "Size".into(), None).flex_1()),
            )
            .child(body)
    }
}

// ---------------------------------------------------------------------------
// Views page
// ---------------------------------------------------------------------------

impl Gallery {
    fn list(&self, style: &LaceStyle, w: &WeakEntity<Self>) -> impl IntoElement {
        let pal = style.pal(State::NORMAL).clone();
        let rows = ["Alpha", "Beta", "Gamma", "Delta", "Epsilon"].into_iter().enumerate().map(|(i, label)| {
            let rid = SharedString::from(format!("list{i}"));
            let s = State { selected: self.list_sel == i, disabled: i == 3, ..State::NORMAL };
            let pick = w.clone();
            item_row_div(style, s)
                .id(rid)
                .h(px(20.))
                .px(px(4.))
                .child(label)
                .when(i != 3, |d| {
                    d.on_click(move |_, _, cx| {
                        let _ = pick.update(cx, |g, cx| {
                            g.list_sel = i;
                            cx.notify();
                        });
                    })
                })
        });
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(hsla(pal.base))
            .border_1()
            .border_color(hsla(rules::frame_line(style, State::NORMAL)))
            .rounded(px(style.frame_radius))
            .p(px(style.frame_inset(style.frame_radius)))
            .py(px(2.0f32.max(style.frame_inset(style.frame_radius))))
            .children(rows)
    }

    fn text_pane(&self, style: &LaceStyle, state: &Entity<TextareaState>) -> impl IntoElement {
        let pal = style.pal(State::NORMAL).clone();
        div()
            .size_full()
            .p(px(4.0f32.max(style.frame_inset(style.frame_radius))))
            .bg(hsla(pal.base))
            .text_color(hsla(pal.text))
            .border_1()
            .border_color(hsla(rules::frame_line(style, State::NORMAL)))
            .rounded(px(style.frame_radius))
            .child(Textarea::new(state).xsmall().appearance(false).h_full())
    }

    fn views(&self, style: &LaceStyle, w: &WeakEntity<Self>) -> AnyElement {
        let dials = div()
            .flex()
            .justify_around()
            .items_center()
            .size_full()
            .child(Dial::new("dial0", self.dials[0]).diameter(180.).notches(2.0).on_change(set(w, |g, v| g.dials[0] = v)))
            .child(Dial::new("dial1", self.dials[1]).diameter(180.).notches(2.0).on_change(set(w, |g, v| g.dials[1] = v)))
            .child(Dial::new("dial2", self.dials[2]).diameter(180.).notches(2.0).disabled(true));
        let splitter = Splitter::new("split")
            .pane(div().size_full().child(self.text_pane(style, &self.plain)), 1.0)
            .pane(div().size_full().child(self.text_pane(style, &self.rich)), 1.0);
        let alternating = self.alternating;
        div()
            .flex()
            .flex_col()
            .gap(px(10.))
            .p(px(12.))
            .child(
                div().flex().gap(px(16.)).child(
                    CheckBox::new("alternating")
                        .label("Alternating row colours (tree, table, list)")
                        .checked(alternating)
                        .on_change(act_v(w, |g, v: CheckState, cx| g.set_alternating(v == CheckState::Checked, cx))),
                ),
            )
            .child(
                div()
                    .flex()
                    .gap(px(10.))
                    .h(px(420.))
                    .child(div().w(px(560.)).flex_none().child(GroupBox::new("Tree view: checks, sort, edit").child(self.tree_view.clone())))
                    .child(div().flex_1().min_w_0().child(GroupBox::new("Table view: 2000 rows, widget delegates").child(self.table_view.clone()))),
            )
            .child(
                div()
                    .flex()
                    .gap(px(10.))
                    .h(px(280.))
                    .child(div().w(px(200.)).child(GroupBox::new("List view").child(self.list_view.clone())))
                    .child(div().flex_1().child(GroupBox::new("Splitter + text edits").child(splitter)))
                    .child(div().flex_1().child(GroupBox::new("Dial").child(dials))),
            )
            .child(
                div().h(px(110.)).child(
                    GroupBox::new("Collapsible buttons: drag the splitter (row folds one by one, last first)").child(
                        Splitter::new("collapse-split")
                            .pane(div().size_full().flex().flex_col().justify_center().gap(px(8.)).child(self.button_row(style)).child(
                                div().flex().gap(px(6.)).child(
                                    PushButton::new("alone", "Collapses alone")
                                        .icon(|c| glyph::chevron(Dir::Right, c, 0.0).into_any_element())
                                        .collapsible(),
                                ),
                            ), 1.0)
                            .pane(div().size_full().flex().items_center().justify_center().child("drag the handle left"), 1.0),
                    ),
                ),
            )
            .into_any_element()
    }

    /// A Weave-style button row: when the row cannot fit every label, all
    /// of its buttons drop to their icons together (tooltips keep the text).
    fn button_row(&self, _style: &LaceStyle) -> impl IntoElement {
        ButtonRow::new("actions")
            .button(PushButton::new("apply", "Apply changes").icon(|c| glyph::check_mark(c).into_any_element()))
            .button(PushButton::new("remove", "Remove").icon(|c| glyph::dash(c).into_any_element()))
            .button(PushButton::new("expand", "Expand all").icon(|c| glyph::chevron(Dir::Down, c, 0.0).into_any_element()))
            .button(PushButton::new("close", "Close").icon(|c| glyph::cross(c, 8.0).into_any_element()))
    }
}

// ---------------------------------------------------------------------------
// Splitters page
// ---------------------------------------------------------------------------

/// A labelled pane for the splitter demos.
fn demo_pane(style: &LaceStyle, title: &'static str, note: &'static str) -> impl IntoElement {
    let pal = style.pal(State::NORMAL).clone();
    div()
        .size_full()
        .p(px(4.))
        .child(
            div()
                .size_full()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px(4.))
                .bg(hsla(pal.base))
                .border_1()
                .border_color(hsla(rules::frame_line(style, State::NORMAL)))
                .rounded(px(style.frame_radius))
                .child(title)
                .child(div().text_size(px(style.small_font_size)).text_color(hsla(pal.placeholder)).child(note)),
        )
}

/// The room left beside a demo: drag the outer handle into it.
fn room(style: &LaceStyle, text: &'static str) -> impl IntoElement {
    let pal = style.pal(State::NORMAL).clone();
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .text_color(hsla(pal.placeholder))
        .child(text)
}

impl Gallery {
    fn splitters(&self, style: &LaceStyle) -> AnyElement {
        // 1. Side by side, stacking when narrower than 560 px.
        let three = Splitter::new("rs-three")
            .restack(560.)
            .pane(demo_pane(style, "Properties", "side by side >= 560 px"), 1.0)
            .pane(demo_pane(style, "Preview", "stacked below 560 px"), 1.0)
            .pane(demo_pane(style, "Log", "back at 600 px (40 px hysteresis)"), 1.0);
        let first = Splitter::new("rs-outer1")
            .pane(three, 3.0)
            .pane(room(style, "<- drag this handle left"), 1.0);

        // 2. Stacked, turning side by side when shorter than 240 px.
        let stacked = Splitter::new("rs-stack")
            .vertical()
            .restack(240.)
            .pane(demo_pane(style, "Top", "stacked >= 240 px tall"), 1.0)
            .pane(demo_pane(style, "Bottom", "side by side below 240 px"), 1.0);
        let second = Splitter::new("rs-outer2")
            .vertical()
            .pane(stacked, 3.0)
            .pane(room(style, "drag this handle up"), 1.0);

        // 3. A nested layout: a sidebar beside an editor over a console;
        // the whole turns stacked when narrow, the inner one when short.
        let editor = Splitter::new("rs-editor")
            .vertical()
            .restack(220.)
            .pane(demo_pane(style, "Editor", "inner: stacked >= 220 px tall"), 2.0)
            .pane(demo_pane(style, "Console", ""), 1.0);
        let nested = Splitter::new("rs-nested")
            .restack(420.)
            .pane(demo_pane(style, "Sidebar", "outer: side by side >= 420 px"), 1.0)
            .pane(editor, 2.5);
        let third = Splitter::new("rs-outer3")
            .pane(Splitter::new("rs-outer3v").vertical().pane(nested, 3.0).pane(room(style, "drag this handle up"), 1.0), 3.0)
            .pane(room(style, "<- and this one left"), 1.0);

        div()
            .flex()
            .flex_col()
            .gap(px(10.))
            .p(px(12.))
            .child("Restacking splitters (Weave's WeaveSplitter(restack=px)): a splitter shorter than its threshold along its own axis turns its panes the other way, and turns back only with 40 px to spare. Each way keeps its own pane sizes.")
            .child(div().h(px(260.)).child(GroupBox::new("Side by side -> stacked").child(first)))
            .child(
                div()
                    .flex()
                    .gap(px(10.))
                    .h(px(460.))
                    .child(div().w(px(360.)).flex_none().child(GroupBox::new("Stacked -> side by side").child(second)))
                    .child(div().flex_1().min_w_0().child(GroupBox::new("Nested layout").child(third))),
            )
            .into_any_element()
    }
}

// ---------------------------------------------------------------------------
// Shell
// ---------------------------------------------------------------------------

impl Render for Gallery {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let style = cx.lace().clone();
        let pal = style.pal(State::NORMAL).clone();
        let w = cx.entity().downgrade();
        let theme = self.theme;
        let header = div()
            .flex()
            .items_center()
            .gap(px(8.))
            .px(px(8.))
            .h(px(36.))
            .border_b_1()
            .border_color(hsla(rules::frame_line(&style, State::NORMAL)))
            .child(
                ToolButton::new("prev-theme", "<")
                    .auto_raise()
                    .on_click(act(&w, move |g, cx| g.set_theme(theme + THEMES.len() - 1, cx))),
            )
            .child(
                div().w(px(150.)).child(
                    ComboBox::new("theme-pick", THEMES.iter().map(|t| t.name))
                        .selected(Some(theme))
                        .on_select(act_v(&w, |g, i, cx| g.set_theme(i, cx))),
                ),
            )
            .child(ToolButton::new("next-theme", ">").auto_raise().on_click(act(&w, move |g, cx| g.set_theme(theme + 1, cx))))
            .child(div().w(px(24.)))
            .child(TabBar::new("pages", PAGES).current(self.page).on_select(set(&w, |g, i| g.page = i)))
            .child(div().flex_1())
            .child("Frame radius")
            .child(div().w(px(64.)).child(SpinBox::new("tok-radius", self.frame_radius).range(0., 16.).on_change(token(&w, |g, v| g.frame_radius = v))))
            .child("Scrollbar")
            .child(div().w(px(64.)).child(SpinBox::new("tok-bar", self.bar_width).range(4., 24.).on_change(token(&w, |g, v| g.bar_width = v))))
            .child(div().w(px(12.)))
            .child(MenuBarItem::new("mb-file", "File"))
            .child(MenuBarItem::new("mb-edit", "Edit"))
            .child(MenuBarItem::new("mb-view", "View"));
        let page = match self.page {
            0 => self.gallery(&style),
            1 => self.widgets(&style, &w),
            2 => self.views(&style, &w),
            3 => self.splitters(&style),
            _ => self.kit.clone().into_any_element(),
        };
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(hsla(pal.window))
            .text_color(hsla(pal.window_text))
            .text_size(px(style.font_size))
            .font_family("Segoe UI")
            .child(header)
            .child(div().id("page").flex_1().min_h_0().overflow_y_scroll().child(page))
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let theme = args
        .first()
        .and_then(|t| THEMES.iter().position(|s| s.name == t))
        .or_else(|| THEMES.iter().position(|s| s.name == "dracula"))
        .unwrap_or(0);
    let page = args
        .get(1)
        .and_then(|p| PAGES.iter().position(|n| n.eq_ignore_ascii_case(p)))
        .unwrap_or(0);
    gpui_kit::application().with_assets(gpui_kit::assets::Assets).run(move |cx: &mut App| {
        gpui_kit::init(cx);
        lace_gpui::skin::apply(THEMES[theme].name, cx);
        lace_widgets::install_preset(THEMES[theme].name, cx);
        let (w, h) = if page == 0 { (1820.0, 760.0) } else { (1100.0, 1000.0) };
        let display = cx
            .primary_display()
            .map(|d| d.visible_bounds())
            .unwrap_or_else(|| Bounds::new(point(px(0.), px(0.)), size(px(w), px(h))));
        let width = px(w).min(display.size.width - px(32.));
        let height = px(h).min(display.size.height - px(32.));
        let bounds = Bounds::new(
            point(display.origin.x + px(16.), display.origin.y + px(16.)),
            size(width, height),
        );
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions { title: Some("LaceStyle gpui".into()), ..Default::default() }),
            ..Default::default()
        };
        gpui_kit::open_window(options, cx, |window, cx| cx.new(|cx| Gallery::new(theme, page, window, cx)))
            .expect("failed to open the LaceStyle window");
    });
}
