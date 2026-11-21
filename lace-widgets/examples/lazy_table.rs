//! A lazy table: ten million rows, none held until a view draws them.
//!
//! The model knows only its size. As the view scrolls it asks for the rows
//! it is about to draw (`ModelEvent::FetchRows`); the owner computes them
//! here (an app would read them from its data) and hands them back with
//! `fill_rows`. The model keeps a bounded cache.
//!
//! ```text
//! cargo run -p lace-widgets --example lazy_table [theme]
//! ```

use gpui_kit::gpui::{
    div, point, px, size, App, AppContext, Bounds, Context, Entity, IntoElement, ParentElement, Render, Styled,
    Subscription, TitlebarOptions, Window, WindowBounds, WindowOptions,
};
use lace_widgets::views::{ItemModel, ItemView, Mode, ModelEvent, SelectionMode, Value};

const ROWS: usize = 10_000_000;

struct Demo {
    view: Entity<ItemView>,
    _fetch: Subscription,
}

/// Row `r`'s cells.
fn row(r: usize) -> Vec<Value> {
    let x = r as f64 * 1e-3;
    vec![Value::Int(r as i64), Value::Float(x.sin()), Value::Float(x.cos()), Value::from(format!("sample {r}"))]
}

impl Demo {
    fn new(cx: &mut Context<Self>) -> Self {
        let model = cx.new(|_| ItemModel::lazy_table(["row", "sin", "cos", "label"], ROWS));
        let fetch = cx.subscribe(&model, |_, model, ev: &ModelEvent, cx| {
            if let ModelEvent::FetchRows { first, count } = *ev {
                println!("fetch rows {first}..{}", first + count);
                model.update(cx, |m, cx| {
                    m.fill_rows(first, (first..first + count).map(row).collect());
                    cx.notify();
                });
            }
        });
        let view = cx.new(|cx| ItemView::new(&model, Mode::Table, cx).selection_mode(SelectionMode::Extended));
        Self { view, _fetch: fetch }
    }
}

impl Render for Demo {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().p(px(12.)).child(self.view.clone())
    }
}

fn main() {
    let theme = std::env::args().nth(1).unwrap_or_else(|| "dracula".into());
    gpui_kit::application().with_assets(gpui_kit::assets::Assets).run(move |cx: &mut App| {
        gpui_kit::init(cx);
        lace_gpui::skin::apply(&theme, cx);
        lace_widgets::install_preset(&theme, cx);
        let bounds = Bounds::new(point(px(40.), px(40.)), size(px(720.), px(560.)));
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions { title: Some("Lace lazy table".into()), ..Default::default() }),
            ..Default::default()
        };
        gpui_kit::open_window(options, cx, |_, cx| cx.new(Demo::new)).expect("failed to open the window");
    });
}
