//! A geometric dock, built on `lace_gpui::geometry`.
//!
//! Run it with:
//!
//! ```sh
//! cargo run --example dock_geometry --features gpui-kit
//! ```
//!
//! The centre is an `h_split` of three tab groups, so the two interior seams
//! show every feature at once: the 8 px gap, the channel colour behind it, the
//! rounded and outlined panel frames, the glow, and a 6 px rounded splitter.
//! The status bar switches between three presets and cycles the Lace themes,
//! and the per-panel button gives the editor tab its own glow to show the
//! effect registry.
//!
//! Vertical gaps are not available in gpui-kit 0.8; see
//! `docs/gpui-kit-0.8-feasibility.md`. The layout here is horizontal on
//! purpose so every seam in the showcase gets a gap.

use std::sync::Arc;

use gpui_kit::base::dock::PanelView as BasePanelView;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::dock::{
    BasePanel, DockArea, DockLayout, DockPlacement, Panel, PanelEvent, PanelHandle, PanelId,
};
use gpui_kit::component::theme::ActiveTheme;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use lace_gpui::geometry::{
    DockGeometryStyle, GeometryDockSkin, GlowSpec, OutlineStyle, PanelEffect, ShadowSpec,
    SplitterCap, SplitterLength,
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
        "lace-dock-geometry-info"
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
                .glow(GlowSpec::new(theme.colors.accent.alpha(0.28), px(22.)))
                .splitter(|s| {
                    s.thickness(px(6.))
                        .length(SplitterLength::Percent(0.45))
                        .cap(SplitterCap::Rounded)
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
    editor: Entity<InfoPanel>,
    editor_glow: bool,
    preset: Preset,
    theme_name: SharedString,
}

fn panel_view<P: Panel>(panel: Entity<P>) -> Arc<dyn BasePanelView> {
    Arc::new(PanelHandle::new(panel))
}

impl DockGeometryDemo {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let preset = Preset::Flagship;
        let (area, skin) = GeometryDockSkin::dock_area(
            "lace-dock-geometry",
            Some(1),
            preset.style(cx),
            window,
            cx,
        );

        let files = cx.new(|cx| {
            InfoPanel::new(
                "Files",
                "src/\n  color.rs\n  contrast.rs\n  engine.rs\n  themes.rs\n  skin.rs\n  geometry.rs\n\nexamples\n  dock_geometry.rs",
                cx,
            )
        });
        let editor = cx.new(|cx| {
            InfoPanel::new(
                "Editor",
                "The centre is an h_split, so both interior seams get a real 8 px gap.\n\nThe channel behind them is the split frame's background; the panels are inset by half the gap on each interior side, which is why the two neighbours share one centred channel.",
                cx,
            )
        });
        let outline = cx.new(|cx| {
            InfoPanel::new(
                "Outline",
                "DockGeometryStyle\n  gap: 8px, outer 6px\n  radius: 10px\n  outline: 1px theme border\n  shadow: y+3 blur 12 alpha .35\n  glow: accent blur 22 alpha .28\n  splitter: 6px, 45%, rounded",
                cx,
            )
        });
        let logs = cx.new(|cx| {
            InfoPanel::new(
                "Logs",
                "12:00:01  INFO  geometry skin installed\n12:00:02  DEBUG  gap 8px · radius 10px · splitter 6px\n12:00:03  WARN  vertical gaps unavailable on gpui-kit 0.8\n12:00:04  INFO  theme cycled; unset colours re-derived",
                cx,
            )
        });

        let editor_view = panel_view(editor.clone());
        let center = DockLayout::h_split()
            .child(
                DockLayout::tabs().panel_view(panel_view(files), cx),
                Some(px(240.)),
            )
            .child(
                DockLayout::h_split()
                    .child(DockLayout::tabs().panel_view(editor_view, cx), None)
                    .child(
                        DockLayout::tabs().panel_view(panel_view(outline), cx),
                        Some(px(280.)),
                    ),
                None,
            );

        area.update(cx, |area, cx| {
            area.set_center(center, window, cx);
            area.set_dock(
                DockPlacement::Bottom,
                DockLayout::tabs().panel_view(panel_view(logs), cx),
                window,
                cx,
            );
            area.set_dock_size(DockPlacement::Bottom, px(150.), window, cx);
        });

        Self {
            area,
            skin,
            editor,
            editor_glow: false,
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
        let editor_panel = PanelId::from(self.editor.entity_id());
        let editor_glow = self.editor_glow;
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
                        Button::new("geo-editor-effect")
                            .label(if editor_glow {
                                "Editor glow: on"
                            } else {
                                "Editor glow: off"
                            })
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.editor_glow = !this.editor_glow;
                                if this.editor_glow {
                                    let accent = cx.theme().colors.accent.alpha(0.5);
                                    this.skin.set_panel_effect(
                                        editor_panel,
                                        PanelEffect {
                                            shadow: None,
                                            glow: Some(GlowSpec::new(accent, px(30.))),
                                        },
                                        cx,
                                    );
                                } else {
                                    this.skin.clear_panel_effect(editor_panel, cx);
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
                    .text_size(px(11.))
                    .child(format!(
                        "Lace 0.9.7 · preset: {} · theme: {} · gaps: horizontal only",
                        preset.label(),
                        self.theme_name
                    ))
                    .child("gpui-kit 0.8 · GeometryDockSkin"),
            )
    }
}

fn main() {
    gpui_kit::application().run(|cx: &mut App| {
        gpui_kit::init(cx);
        skin::apply("midnight", cx);

        let bounds = Bounds::centered(None, size(px(1280.0), px(820.0)), cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions {
                title: Some("lace-gpui geometric dock".into()),
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
