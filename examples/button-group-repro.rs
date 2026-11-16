//! ButtonGroup-adjacent flex nesting repro.
//!
//! No env: three bare `Button`s in a flex row (control, alive).
//! `REPRO_IDENT=1`: each button wrapped as
//! `div().flex().flex_row().child(div().child(button_with_group_hover))`
//! — the gallery's glow wrapper shape. This crashes the gallery build.

use gpui_kit::component::button::Button;
use gpui_kit::gpui::{
    App, Bounds, BoxShadow, Context, Div, Hsla, InteractiveElement, ParentElement, Render,
    SharedString, Styled, Window, WindowBounds, WindowOptions, point, px, red, size,
};
use gpui_kit::*;
use lace_gpui::skin::{self, WidgetStyle};

fn halo(glow: Hsla) -> BoxShadow {
    BoxShadow::new(px(0.), px(0.), glow.opacity(0.30)).blur_radius(px(8.))
}

/// The gallery glow-wrapper shape: flex-row outer, hugging inner div, and a
/// group_hover halo owned by the button's own group.
fn wrap(button: Button, id: &str) -> Div {
    let glow: Hsla = red();
    let group: SharedString = format!("repro-{id}").into();
    let button = button
        .group(group.clone())
        .group_hover(group, move |s| s.shadow(vec![halo(glow)]));
    div().flex().flex_row().child(div().child(button))
}

struct Root;

impl Render for Root {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        eprintln!(
            "RENDER #{} ident={}",
            N.fetch_add(1, std::sync::atomic::Ordering::SeqCst),
            std::env::var("REPRO_IDENT").is_ok()
        );
        let row = div().flex().flex_row().gap_2();
        let row = if std::env::var("REPRO_IDENT").is_ok() {
            row.children(
                (0..20).map(|i| wrap(Button::new(("b", i)).label(format!("B{i}")), &format!("b{i}"))),
            )
        } else {
            row.children((0..20).map(|i| Button::new(("b", i)).label(format!("B{i}"))))
        };
        div().flex().flex_col().gap_2().child(row)
    }
}

fn main() {
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(|cx: &mut App| {
            gpui_kit::init(cx);
            skin::apply_styled("midnight", WidgetStyle::LaceBase, cx);
            let bounds = Bounds::new(point(px(100.), px(100.)), size(px(640.), px(480.)));
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            };
            gpui_kit::open_window(options, cx, |window, cx| {
                let _ = window;
                cx.new(|_cx| Root)
            })
            .expect("failed to open repro window");
        });
}
