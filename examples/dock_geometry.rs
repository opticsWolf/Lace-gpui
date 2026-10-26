//! A geometric dock lab, built on `lace_gpui::geometry`.
//!
//! Run it with:
//!
//! ```sh
//! cargo run --example dock_geometry --features gpui-kit
//! ```
//!
//! The area has four dock regions: a left Navigator, a centre workbench, a
//! right Inspector and a bottom Output. The left, right and bottom docks show
//! the inter-dock gaps from the area/centre frames; the workbench is an
//! `h_split` so its centre and Outline groups show the geometric splitter.
//!
//! Each region is a tab group, not a single panel: Navigator has Files/Search,
//! Workbench has Editor/Split lab/Preview, Outline has Outline/Symbols,
//! Inspector has Properties/History, and Output has Logs/Problems/Terminal.
//! The **Split lab** tab is active by default and holds stock
//! `h_resizable`/`v_resizable` groups, so the inner splitters are the regular
//! hairline/pill with no dock gap. That is the contrast the skin is meant to
//! make: custom dock seams and gaps outside, ordinary content splitters inside.
//!
//! The top bar switches between three presets and cycles the Lace themes;
//! the shadow button gives the active Split lab panel its own effect.
//!
//! Vertical gaps **inside a `v_split`** still need the gpui-base panel-axis fix;
//! inter-dock vertical gaps, including the Output dock, do not. See
//! `docs/gpui-kit-0.8-feasibility.md`.

use std::sync::Arc;

use gpui_kit::base::dock::PanelView as BasePanelView;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::dock::{
    BasePanel, DockArea, DockLayout, DockPlacement, Panel, PanelEvent, PanelHandle, PanelId,
};
use gpui_kit::component::resizable::{h_resizable, resizable_panel, v_resizable};
use gpui_kit::component::theme::ActiveTheme;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use lace_gpui::geometry::{
    DockGeometryStyle, GeometryDockSkin, OutlineStyle, PanelEffect, ShadowSpec, SplitterCap,
    SplitterColors, SplitterLength,
};
use lace_gpui::skin;

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
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl BasePanel for InfoPanel {
    fn panel_name(&self) -> &'static str {
        self.label
    }
}

impl Panel for InfoPanel {
    fn title(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .text_color(cx.theme().colors.foreground)
            .child(self.label)
    }
}

impl Render for InfoPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = &cx.theme().colors;
        div()
            .size_full()
            .bg(colors.background)
            .text_color(colors.foreground)
            .p_4()
            .child(self.body.clone())
    }
}

/// A tab whose content is two nested stock resizable groups.
///
/// The `h_resizable`/`v_resizable` helpers come from gpui-component, not from
/// `GeometryDockSkin`, so their handles keep the stock hairline/pill and the
/// panels stay adjacent. This is what "regular splitters inside dock panels"
/// means in the demo.
struct SplitLabPanel {
    focus_handle: FocusHandle,
}

impl SplitLabPanel {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }
}

impl EventEmitter<PanelEvent> for SplitLabPanel {}

impl Focusable for SplitLabPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl BasePanel for SplitLabPanel {
    fn panel_name(&self) -> &'static str {
        "split-lab"
    }
}

impl Panel for SplitLabPanel {
    fn title(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .text_color(cx.theme().colors.foreground)
            .child("Split lab")
    }
}

fn inner_pane(label: &'static str, bg: Hsla, fg: Hsla) -> impl IntoElement {
    div()
        .size_full()
        .bg(bg)
        .text_color(fg)
        .p_3()
        .text_size(px(12.))
        .child(label)
}

impl Render for SplitLabPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = &cx.theme().colors;
        let muted = colors.muted_foreground;
        let title_bar = colors.title_bar;
        let background = colors.background;
        let status_bar = colors.status_bar;
        let foreground = colors.foreground;

        div()
            .size_full()
            .bg(background)
            .text_color(foreground)
            .flex()
            .flex_col()
            .child(
                div()
                    .px_3()
                    .py_2()
                    .text_size(px(11.))
                    .text_color(muted)
                    .child(
                        "Stock h_resizable / v_resizable inside a dock panel — no dock gap, no geometric splitter.",
                    ),
            )
            .child(
                div().flex_1().min_h_0().child(
                    h_resizable("split-lab-h")
                        .child(resizable_panel().child(
                            v_resizable("split-lab-v")
                                .child(resizable_panel().child(inner_pane(
                                    "Inner A",
                                    title_bar,
                                    foreground,
                                )))
                                .child(resizable_panel().child(inner_pane(
                                    "Inner B",
                                    background,
                                    foreground,
                                ))),
                        ))
                        .child(resizable_panel().child(inner_pane(
                            "Inner C",
                            status_bar,
                            foreground,
                        ))),
                ),
            )
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Preset {
    Flagship,
    Minimal,
    Stock,
}

impl Preset {
    fn label(self) -> &'static str {
        match self {
            Preset::Flagship => "flagship",
            Preset::Minimal => "minimal",
            Preset::Stock => "stock",
        }
    }

    fn style(self, cx: &App) -> DockGeometryStyle {
        let theme = cx.theme();
        match self {
            Preset::Flagship => DockGeometryStyle::new()
                .gap(px(8.))
                .gap_outer(px(6.))
                .panel_radius(px(10.))
                .outline(OutlineStyle::themed(px(1.)))
                .shadow(
                    ShadowSpec::new(black().alpha(0.35), point(px(0.), px(3.)), px(12.))
                        .spread(px(-2.)),
                )
                .splitter(|s| {
                    s.thickness(px(6.))
                        .length(SplitterLength::Percent(0.45))
                        .cap(SplitterCap::Rounded)
                        .colors(
                            SplitterColors::default()
                                .idle(theme.colors.muted_foreground)
                                .hovered(theme.colors.accent)
                                .dragging(theme.colors.accent),
                        )
                }),
            Preset::Minimal => DockGeometryStyle::new()
                .gap(px(4.))
                .panel_radius(px(6.))
                .splitter(|s| {
                    s.thickness(px(3.))
                        .length(SplitterLength::Percent(0.6))
                        .cap(SplitterCap::Rounded)
                }),
            Preset::Stock => DockGeometryStyle::default(),
        }
    }
}

struct DockGeometryDemo {
    area: Entity<DockArea>,
    skin: std::rc::Rc<GeometryDockSkin>,
    split_lab: Entity<SplitLabPanel>,
    split_lab_shadow: bool,
    preset: Preset,
    theme_name: SharedString,
}

fn panel_view<P: Panel>(panel: Entity<P>) -> Arc<dyn BasePanelView> {
    Arc::new(PanelHandle::new(panel))
}

impl DockGeometryDemo {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let preset = Preset::Flagship;
        let (area, skin) =
            GeometryDockSkin::dock_area("lace-dock-geometry", None, preset.style(cx), window, cx);

        let files = cx.new(|cx| {
            InfoPanel::new(
                "Files",
                "src/\n  color.rs\n  contrast.rs\n  engine.rs\n  themes.rs\n  skin.rs\n  geometry.rs\n\nexamples\n  dock_geometry.rs",
                cx,
            )
        });
        let search = cx.new(|cx| {
            InfoPanel::new(
                "Search",
                "Search results\n\n12 matches in 4 files\n  geometry.rs  gap, splitter\n  skin.rs      apply\n  themes.rs    presets\n  dock_geometry.rs  demo",
                cx,
            )
        });
        let editor = cx.new(|cx| {
            InfoPanel::new(
                "Editor",
                "The workbench is an h_split, so the seam to Outline gets a real 8 px gap.\n\nThe channel behind it is the split frame's background; the two groups are inset by half the gap on the seam side, which is why they share one centred channel.",
                cx,
            )
        });
        let preview = cx.new(|cx| {
            InfoPanel::new(
                "Preview",
                "Preview\n\nThis tab is a plain panel in the Workbench group. Switch back to Split lab to drag the stock inner splitters.",
                cx,
            )
        });
        let split_lab = cx.new(SplitLabPanel::new);
        let outline = cx.new(|cx| {
            InfoPanel::new(
                "Outline",
                "DockGeometryStyle\n  gap: 8px, outer 6px\n  radius: 10px\n  outline: 1px theme border\n  shadow: y+3 blur 12 alpha .35\n  effect: per-panel shadow\n  splitter: 6px, 45%, rounded",
                cx,
            )
        });
        let symbols = cx.new(|cx| {
            InfoPanel::new(
                "Symbols",
                "Symbols\n\nSplitLabPanel\nInnerPanel::render\nGeometryDockSkin::dock_area\nDockGeometryStyle::splitter",
                cx,
            )
        });
        let properties = cx.new(|cx| {
            InfoPanel::new(
                "Properties",
                "Properties\n\nactive tab: Split lab\ndock: workbench (centre)\ninner split: stock\ninner gap: none",
                cx,
            )
        });
        let history = cx.new(|cx| {
            InfoPanel::new(
                "History",
                "History\n\n1. geometry skin installed\n2. dock regions spaced\n3. tab groups populated\n4. Split lab opened",
                cx,
            )
        });
        let logs = cx.new(|cx| {
            InfoPanel::new(
                "Logs",
                "12:00:01  INFO  geometry skin installed\n12:00:02  DEBUG  gap 8px · radius 10px · splitter 6px\n12:00:03  WARN  v_split gaps need the gpui-base panel-axis fix\n12:00:04  INFO  four dock regions, five tab groups\n12:00:05  INFO  Split lab uses stock inner splitters",
                cx,
            )
        });
        let problems = cx.new(|cx| {
            InfoPanel::new(
                "Problems",
                "0 errors, 0 warnings\n\nInner resizable groups are the stock gpui-component ones. Dock splitters are the geometric ones. The two layers do not interfere.",
                cx,
            )
        });
        let terminal = cx.new(|cx| {
            InfoPanel::new(
                "Terminal",
                "$ cargo run --example dock_geometry --features gpui-kit\n    Finished dev profile\n     Running `target/debug/examples/dock_geometry.exe`",
                cx,
            )
        });

        let navigator = DockLayout::tabs()
            .panel_view(panel_view(files), cx)
            .panel_view(panel_view(search), cx)
            .active_index(0);
        let workbench = DockLayout::tabs()
            .panel_view(panel_view(editor), cx)
            .panel_view(panel_view(split_lab.clone()), cx)
            .panel_view(panel_view(preview), cx)
            .active_index(1);
        let outline_group = DockLayout::tabs()
            .panel_view(panel_view(outline), cx)
            .panel_view(panel_view(symbols), cx)
            .active_index(0);
        let center = DockLayout::h_split()
            .child(workbench, None)
            .child(outline_group, Some(px(280.)));
        let inspector = DockLayout::tabs()
            .panel_view(panel_view(properties), cx)
            .panel_view(panel_view(history), cx)
            .active_index(0);
        let output = DockLayout::tabs()
            .panel_view(panel_view(logs), cx)
            .panel_view(panel_view(problems), cx)
            .panel_view(panel_view(terminal), cx)
            .active_index(0);

        area.update(cx, |area, cx| {
            area.set_center(center, window, cx);
            area.set_dock(DockPlacement::Left, navigator, window, cx);
            area.set_dock_size(DockPlacement::Left, px(220.), window, cx);
            area.set_dock(DockPlacement::Right, inspector, window, cx);
            area.set_dock_size(DockPlacement::Right, px(260.), window, cx);
            area.set_dock(DockPlacement::Bottom, output, window, cx);
            area.set_dock_size(DockPlacement::Bottom, px(150.), window, cx);
        });

        Self {
            area,
            skin,
            split_lab,
            split_lab_shadow: false,
            preset,
            theme_name: "midnight".into(),
        }
    }

    fn apply_preset(&mut self, preset: Preset, cx: &mut App) {
        self.preset = preset;
        self.skin.set_style(preset.style(cx), cx);
    }
}

impl Render for DockGeometryDemo {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = &cx.theme().colors;
        let preset = self.preset;
        let split_lab_panel = PanelId::from(self.split_lab.entity_id());
        let split_lab_shadow = self.split_lab_shadow;
        let theme_name = self.theme_name.clone();

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(colors.background)
            .text_color(colors.foreground)
            .child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .py_2()
                    .bg(colors.title_bar)
                    .border_b_1()
                    .border_color(colors.title_bar_border)
                    .text_size(px(12.))
                    .child(
                        Button::new("geo-theme")
                            .label(format!("Theme: {}", theme_name))
                            .on_click(cx.listener(|this, _, _, cx| {
                                let name = skin::apply_next(&this.theme_name, cx);
                                this.theme_name = name.into();
                                // Explicit colours are captured at style-build
                                // time, so rebuild the preset against the new
                                // theme rather than keeping stale accents.
                                this.apply_preset(this.preset, cx);
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("geo-flagship")
                            .label("Flagship")
                            .when(preset == Preset::Flagship, |b| b.primary())
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.apply_preset(Preset::Flagship, cx);
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("geo-minimal")
                            .label("Minimal")
                            .when(preset == Preset::Minimal, |b| b.primary())
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.apply_preset(Preset::Minimal, cx);
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("geo-stock")
                            .label("Stock")
                            .when(preset == Preset::Stock, |b| b.primary())
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.apply_preset(Preset::Stock, cx);
                                cx.notify();
                            })),
                    )
                    .child(div().flex_1())
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(colors.muted_foreground)
                            .child(format!("{} · 12 panels · 5 tab groups", preset.label())),
                    )
                    .child(
                        Button::new("geo-split-shadow")
                            .label(if split_lab_shadow {
                                "Split lab shadow: on"
                            } else {
                                "Split lab shadow: off"
                            })
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.split_lab_shadow = !this.split_lab_shadow;
                                if this.split_lab_shadow {
                                    let shadow = ShadowSpec::new(
                                        black().alpha(0.55),
                                        point(px(0.), px(6.)),
                                        px(24.),
                                    )
                                    .spread(px(-4.));
                                    this.skin.set_panel_effect(
                                        split_lab_panel,
                                        PanelEffect {
                                            shadow: Some(shadow),
                                            glow: None,
                                        },
                                        cx,
                                    );
                                } else {
                                    this.skin.clear_panel_effect(split_lab_panel, cx);
                                }
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .child(self.area.clone()),
            )
    }
}

fn main() {
    gpui_kit::application().run(|cx: &mut App| {
        gpui_kit::init(cx);
        skin::apply("midnight", cx);

        let bounds = Bounds::centered(None, size(px(1360.0), px(900.0)), cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions {
                title: Some("lace-gpui geometric dock lab".into()),
                ..Default::default()
            }),
            ..Default::default()
        };
        gpui_kit::open_window(options, cx, |window, cx| {
            cx.new(|cx| DockGeometryDemo::new(window, cx))
        })
        .expect("failed to open geometry window");
    });
}
