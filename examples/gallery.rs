//! The Lace gallery, as a gpui-kit dock application.
//!
//! Run it with:
//!
//! ```sh
//! cargo run --example gallery --features gpui-kit
//! ```
//!
//! A dock area (left rail 240px, right outline 220px, bottom log 140px,
//! tabbed editor in the center) wearing whichever Lace theme is current.
//! The status bar's button walks all 16 presets in Lace's dark-to-light
//! order — every widget in the window recolours through
//! [`lace4gpui::skin`], no restart, no reload.
//!
//! The center's second tab, **Geometry**, is a per-token geometry lab: it
//! draws each theme's radii, borders, outline strength and focus ring at
//! their real token values (via `themes::by_name` + `build`, the same
//! pipeline `skin` paints the app with), so a theme pass can be judged
//! token by token, not just by vibe.

use std::sync::Arc;

use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::dock::{
    BasePanel, DockArea, DockLayout, DockPlacement, DockSkin, Panel, PanelEvent, PanelView,
};
use gpui_kit::component::theme::ActiveTheme;
use gpui_kit::*; // the glob includes AppContext, needed for cx.new siblings

use lace4gpui::{build as build_tokens, color, skin, themes};

/// Token colour -> gpui paint colour (the same conversion `skin` uses).
fn h(c: color::Rgba) -> Hsla {
    skin::hsla(c)
}

// ---------------------------------------------------------------------------
// A plain dockable panel: a title and a body of text.
// ---------------------------------------------------------------------------

struct InfoPanel {
    label: &'static str,
    body: SharedString,
    focus_handle: FocusHandle,
}

impl InfoPanel {
    fn new(label: &'static str, body: &str, cx: &mut Context<Self>) -> Self {
        Self {
            label,
            body: body.into(),
            focus_handle: cx.focus_handle(),
        }
    }
}

impl EventEmitter<PanelEvent> for InfoPanel {}

impl Focusable for InfoPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl BasePanel for InfoPanel {
    fn panel_name(&self) -> &'static str {
        "lace-gallery-info"
    }
}

impl Panel for InfoPanel {
    fn title(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let text_color = cx.theme().colors.foreground;
        div().text_color(text_color).child(self.label.to_string())
    }
}

impl Render for InfoPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = &cx.theme().colors;
        div()
            .size_full()
            .bg(colors.background)
            .text_color(colors.foreground)
            .p_4()
            .child(self.body.clone())
    }
}

// ---------------------------------------------------------------------------
// The geometry lab: every Lace geometry token, drawn at its real value.
// ---------------------------------------------------------------------------

struct GeoPanel {
    theme_name: &'static str,
    focus_handle: FocusHandle,
}

impl GeoPanel {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            theme_name: "midnight",
            focus_handle: cx.focus_handle(),
        }
    }

    /// Feed the status bar's current theme into the lab.
    fn set_theme(&mut self, name: &'static str) {
        self.theme_name = name;
    }
}

impl EventEmitter<PanelEvent> for GeoPanel {}

impl Focusable for GeoPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl BasePanel for GeoPanel {
    fn panel_name(&self) -> &'static str {
        "lace-gallery-geometry"
    }
}

impl Panel for GeoPanel {
    fn title(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let text_color = cx.theme().colors.foreground;
        div().text_color(text_color).child("Geometry")
    }
}

/// `4` -> "4px", `1.5` -> "1.5px".
fn fmt_px(v: f64) -> String {
    if (v - v.round()).abs() < f64::EPSILON {
        format!("{}px", v.round() as i64)
    } else {
        format!("{}px", v)
    }
}

/// A radius swatch: a filled box cornered at `radius`, labelled underneath.
/// `edge_w == 0` draws no border; `top_only` rounds only the top corners
/// (that is how dock tabs are shaped).
fn swatch(
    label: String,
    radius: f32,
    fill: color::Rgba,
    text: color::Rgba,
    edge: color::Rgba,
    edge_w: f32,
    top_only: bool,
) -> impl IntoElement {
    let mut s = div()
        .w(px(104.0))
        .h(px(64.0))
        .bg(h(fill))
        .text_color(h(text))
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(13.0));
    s = if top_only {
        s.rounded_tl(px(radius)).rounded_tr(px(radius))
    } else {
        s.rounded(px(radius))
    };
    if edge_w > 0.0 {
        s = s.border(px(edge_w)).border_color(h(edge));
    }
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(s)
        .child(div().text_size(px(11.0)).child(label))
}

/// A bordered demo box for the BORDERS & FOCUS row.
fn border_box(label: &str, fill: color::Rgba, edge: color::Rgba, edge_w: f32, radius: f32) -> impl IntoElement {
    div()
        .w(px(104.0))
        .h(px(56.0))
        .bg(h(fill))
        .border(px(edge_w.max(1.0)))
        .border_color(h(edge))
        .rounded(px(radius))
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(13.0))
        .child(label.to_string())
}

/// A raw-token chip: name on the left, formatted value on the right.
fn chip(name: &str, value: String, bg: color::Rgba, fg: color::Rgba, muted: color::Rgba, radius: f32) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap_4()
        .px_2()
        .py_1()
        .bg(h(bg))
        .rounded(px(radius.max(2.0)))
        .text_size(px(12.0))
        .child(div().text_color(h(muted)).child(name.to_string()))
        .child(div().text_color(h(fg)).child(value))
}

/// A section caption: small muted label with a rule running to the edge.
fn section(label: &'static str, rule: color::Rgba, muted: color::Rgba) -> impl IntoElement {
    div()
        .mt(px(28.0))
        .mb(px(14.0))
        .flex()
        .items_center()
        .gap_3()
        .child(div().text_size(px(11.0)).text_color(h(muted)).child(label))
        .child(div().flex_1().h(px(1.0)).bg(h(rule)))
}

impl Render for GeoPanel {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let spec = themes::by_name(self.theme_name).unwrap_or(&themes::ALL[0]);
        let t = build_tokens(spec);
        let chrome = format!("{:?}", skin::dock_chrome(spec));

        // Lace's own outline recipe: text mixed over the control fill.
        let outline_mix = color::mix(t.button_bg, t.text, t.outline_strength);

        let mode = if t.is_light { "light" } else { "dark" };

        // --- header ------------------------------------------------------
        let header = div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .text_size(px(20.0))
                    .child(format!("{} — geometry lab", spec.name)),
            )
            .child(
                div()
                    .flex()
                    .gap_6()
                    .text_size(px(13.0))
                    .text_color(h(t.text_muted))
                    .child(format!("mode: {}", mode))
                    .child(format!(
                        "scrollbar: {}",
                        match t.scrollbar {
                            lace4gpui::engine::Scrollbar::Thin => "thin",
                            lace4gpui::engine::Scrollbar::Expanding => "expanding",
                            lace4gpui::engine::Scrollbar::Fusion => "fusion",
                        }
                    ))
                    .child(format!("title: {}", fmt_px(t.title_height)))
                    .child(format!("dock_chrome: {}", chrome)),
            );

        // --- radii -------------------------------------------------------
        let radii = div()
            .flex()
            .items_end()
            .gap_6()
            .child(swatch(
                format!("control_radius = {}", fmt_px(t.control_radius)),
                t.control_radius as f32,
                t.button_bg,
                t.text,
                outline_mix,
                1.0,
                false,
            ))
            .child(swatch(
                format!("corner_radius = {}", fmt_px(t.corner_radius)),
                t.corner_radius as f32,
                t.panel_bg,
                t.text,
                t.border_color,
                1.0,
                false,
            ))
            .child(
                // A dock tab: recessed strip, accent indicator on top, top-only radius.
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .bg(h(t.input_bg))
                            .rounded(px(t.corner_radius as f32))
                            .p(px(6.0))
                            .child(
                                div()
                                    .w(px(104.0))
                                    .h(px(52.0))
                                    .bg(h(t.tab_bg_active))
                                    .text_color(h(t.tab_text_active))
                                    .border_t(px(2.0))
                                    .border_color(h(t.tab_indicator))
                                    .rounded_tl(px(t.tab_radius as f32))
                                    .rounded_tr(px(t.tab_radius as f32))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_size(px(13.0))
                                    .child("Tab"),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .child(format!("tab_radius = {}", fmt_px(t.tab_radius))),
                    ),
            );

        // --- real controls ------------------------------------------------
        let controls = div()
            .flex()
            .items_center()
            .gap_3()
            .child(Button::new("geo-btn-default").label("Default"))
            .child(Button::new("geo-btn-primary").label("Primary").primary())
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(h(t.text_muted))
                    .child(format!(
                        "gpui-kit's two global knobs: theme.radius = {} (control_radius) · theme.radius_lg = {} (corner_radius)",
                        fmt_px(t.control_radius),
                        fmt_px(t.corner_radius)
                    )),
            );

        // --- borders & focus ----------------------------------------------
        let borders = div()
            .flex()
            .items_center()
            .gap_6()
            .child(border_box("outline", t.button_bg, outline_mix, 1.0, t.control_radius as f32))
            .child(border_box(
                "dock",
                t.panel_bg,
                t.border_color,
                t.border_width as f32,
                t.corner_radius as f32,
            ))
            .child(border_box(
                "focus",
                t.input_bg,
                t.focus_border_color,
                t.focus_width as f32,
                t.control_radius as f32,
            ));

        // --- raw geometry tokens -------------------------------------------
        let chips = div()
            .flex()
            .flex_wrap()
            .gap_2()
            .max_w(px(760.0))
            .child(chip("control_radius", fmt_px(t.control_radius), t.input_bg, t.text, t.text_muted, t.control_radius as f32))
            .child(chip("corner_radius", fmt_px(t.corner_radius), t.input_bg, t.text, t.text_muted, t.control_radius as f32))
            .child(chip("tab_radius", fmt_px(t.tab_radius), t.input_bg, t.text, t.text_muted, t.control_radius as f32))
            .child(chip("border_width", fmt_px(t.border_width), t.input_bg, t.text, t.text_muted, t.control_radius as f32))
            .child(chip("outline_strength", format!("{}", t.outline_strength), t.input_bg, t.text, t.text_muted, t.control_radius as f32))
            .child(chip("focus_width", fmt_px(t.focus_width), t.input_bg, t.text, t.text_muted, t.control_radius as f32))
            .child(chip("title_height", fmt_px(t.title_height), t.input_bg, t.text, t.text_muted, t.control_radius as f32));

        // --- the scrollable lab ---------------------------------------------
        div()
            .id("geo-scroll")
            .size_full()
            .overflow_y_scroll()
            .bg(h(t.canvas_bg))
            .text_color(h(t.text))
            .p(px(24.0))
            .child(header)
            .child(section("RADII", t.border_color, t.text_muted))
            .child(radii)
            .child(section("REAL CONTROLS", t.border_color, t.text_muted))
            .child(controls)
            .child(section("BORDERS & FOCUS", t.border_color, t.text_muted))
            .child(borders)
            .child(section("RAW GEOMETRY TOKENS", t.border_color, t.text_muted))
            .child(chips)
    }
}

// ---------------------------------------------------------------------------
// The gallery window: a dock area plus a status bar that cycles themes.
// ---------------------------------------------------------------------------

struct Gallery {
    area: Entity<DockArea>,
    geo: Entity<GeoPanel>,
    theme_name: SharedString,
}

fn panel(label: &'static str, body: &str, cx: &mut App) -> Arc<dyn PanelView> {
    Arc::new(cx.new(|cx| InfoPanel::new(label, body, cx)) as Entity<InfoPanel>)
}

impl Gallery {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let area = cx.new(|cx| {
            DockArea::new("lace-gallery", Some(1), window, cx)
                .with_renderer(DockSkin::new(cx))
        });

        let editor = panel(
            "Editor",
            "The center of the dock, holding tabs. Switch to the Geometry tab for the per-token geometry lab.",
            cx,
        );
        let files = panel(
            "Files",
            "src/\n  color.rs\n  contrast.rs\n  engine.rs\n  themes.rs\n  skin.rs\n  lib.rs\n\nexamples\n  gallery.rs\n  dump_tokens.rs\n\ntests\n  engine.rs",
            cx,
        );
        let outline = panel(
            "Outline",
            "struct ThemeSpec\nenum Scrollbar\nmod depth_steps\nstruct Tokens\nfn build_unenforced\nfn build\nfn fingerprint",
            cx,
        );
        let logs = panel(
            "Logs",
            "12:00:01  INFO  theme applied\n12:00:02  DEBUG  tokens rebuilt (78 colours · 7 geometry)\n12:00:03  WARN  contrast floor nudged: text_muted +0.4 L\n12:00:04  ERROR  sample: hover separation enforced on input_bg\n12:00:05  OK  fingerprint unique across presets",
            cx,
        );
        let geo = cx.new(GeoPanel::new);

        let center = DockLayout::tabs()
            .panel_view(editor, cx)
            .panel_view(Arc::new(geo.clone()) as Arc<dyn PanelView>, cx)
            .active_index(1);

        area.update(cx, |area, cx| {
            area.set_center(center, window, cx);
            area.set_dock(
                DockPlacement::Left,
                DockLayout::tabs().panel_view(files, cx),
                window,
                cx,
            );
            area.set_dock(
                DockPlacement::Right,
                DockLayout::tabs().panel_view(outline, cx),
                window,
                cx,
            );
            area.set_dock(
                DockPlacement::Bottom,
                DockLayout::tabs().panel_view(logs, cx),
                window,
                cx,
            );
            area.set_dock_size(DockPlacement::Left, px(240.0), window, cx);
            area.set_dock_size(DockPlacement::Right, px(220.0), window, cx);
            area.set_dock_size(DockPlacement::Bottom, px(140.0), window, cx);
        });

        Self {
            area,
            geo,
            theme_name: "midnight".into(),
        }
    }
}

impl Render for Gallery {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = &cx.theme().colors;
        let theme_name = self.theme_name.clone();

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(colors.background)
            .text_color(colors.foreground)
            .font_family(".SystemUIFont")
            // The dock fills everything above the status bar.
            .child(div().flex_1().min_h_0().overflow_hidden().child(self.area.clone()))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_3()
                    .py_1()
                    .bg(colors.status_bar)
                    .border_t_1()
                    .border_color(colors.status_bar_border)
                    .text_size(px(12.0))
                    .child(format!("Lace 0.9.7 · theme: {}", self.theme_name))
                    .child(
                        Button::new("next-theme")
                            .label(format!("Next theme ({})", theme_name))
                            .on_click(cx.listener(move |this, _event, _window, cx| {
                                let name = skin::apply_next(&this.theme_name, cx);
                                this.theme_name = name.into();
                                this.geo.update(cx, |geo, cx| {
                                    geo.set_theme(name);
                                    cx.notify();
                                });
                                cx.notify();
                            })),
                    ),
            )
    }
}

fn main() {
    gpui_kit::application().run(|cx: &mut App| {
        gpui_kit::init(cx);
        // Lace's darkest preset, then the window.
        skin::apply("midnight", cx);

        let bounds = Bounds::centered(None, size(px(1280.0), px(820.0)), cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions {
                title: Some("lace4gpui gallery".into()),
                ..Default::default()
            }),
            ..Default::default()
        };
        gpui_kit::open_window(options, cx, |window, cx| {
            cx.new(|cx| Gallery::new(window, cx))
        })
        .expect("failed to open gallery window");
    });
}
