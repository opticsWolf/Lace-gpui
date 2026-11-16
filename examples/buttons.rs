//! Button showcase: every gpui-kit button type in Lace colors.
//!
//! Run it with:
//!
//! ```sh
//! cargo run --example buttons --features gpui-kit
//! ```
//!
//! Styling is the Lace palette plus Lace radii only
//! ([`skin::apply`], i.e. `WidgetStyle::Theme`): no glow helper, no
//! gradients. Hover and pressed states come straight from the stock
//! gpui-kit styles fed by the theme tokens, so every button below is a
//! plain stock button. The header button cycles all 16 Lace presets to
//! show every type recoloring live.

use gpui_kit::component::button::{
    Button, ButtonGroup, ButtonRounded, ButtonVariants, DropdownButton, Toggle, ToggleGroup,
};
use gpui_kit::component::theme::Theme;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::{Disableable, IconName, Selectable};
use gpui_kit::*;

use lace_gpui::skin;
use lace_widgets::DynamicButton;

gpui_kit::actions!(buttons, [DemoAction]);

struct ButtonsApp {
    theme: &'static str,
    /// gpui-kit stock theme, snapshotted before Lace is applied, so the
    /// header toggle can switch back to the default skin.
    stock: Theme,
    /// True while a Lace preset is applied; false for the stock skin.
    lace: bool,
    /// Per-button expanded flags for the dynamic toolbar, same order as
    /// [`DYNAMIC_BAR`]. Updated in `render` before the row is built.
    dynamic: [bool; 5],
    /// Clickable demo state — everything below persists while the app runs.
    s_toggled: bool,
    g_single: usize,
    g_multi: [bool; 3],
    g_seg: usize,
    t_wifi: bool,
    t_bt: bool,
    tg_view: usize,
    tg_align: usize,
}

impl ButtonsApp {
    fn new(stock: Theme, _window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {
            theme: "midnight",
            stock,
            lace: true,
            dynamic: [true; 5],
            s_toggled: true,
            g_single: 0,
            g_multi: [true, false, false],
            g_seg: 1,
            t_wifi: true,
            t_bt: false,
            tg_view: 0,
            tg_align: 1,
        }
    }

    fn next_theme(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.theme = skin::apply_next(self.theme, cx);
        self.lace = true;
        cx.notify();
    }

    /// Header toggle: `on` restores the current Lace preset, `off` puts the
    /// snapshotted stock theme back — same three steps `skin` uses after
    /// editing colors (replace through `global_mut`, rebuild the Base
    /// projection, repaint).
    fn set_skin(&mut self, lace: bool, _window: &mut Window, cx: &mut Context<Self>) {
        self.lace = lace;
        if lace {
            skin::apply(self.theme, cx);
        } else {
            *Theme::global_mut(cx) = self.stock.clone();
            Theme::sync_base(cx);
            cx.refresh_windows();
        }
        cx.notify();
    }
}

/// A section: title line plus a wrapping row of its buttons.
fn section(title: &str, content: impl IntoElement) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(div().text_sm().child(title.to_string()))
        .child(
            div()
                .flex()
                .flex_row()
                .flex_wrap()
                .items_center()
                .gap_2()
                .child(content),
        )
}

fn variant_row() -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .flex_wrap()
        .items_center()
        .gap_2()
        .child(Button::new("v-default").label("Default"))
        .child(Button::new("v-primary").label("Primary").primary())
        .child(Button::new("v-secondary").label("Secondary").secondary())
        .child(Button::new("v-danger").label("Danger").danger())
        .child(Button::new("v-warning").label("Warning").warning())
        .child(Button::new("v-success").label("Success").success())
        .child(Button::new("v-info").label("Info").info())
        .child(Button::new("v-ghost").label("Ghost").ghost())
        .child(Button::new("v-link").label("Link").link())
        .child(Button::new("v-text").label("Text").text())
}

fn outline_row() -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .flex_wrap()
        .items_center()
        .gap_2()
        .child(Button::new("o-default").label("Outline").outline())
        .child(
            Button::new("o-primary")
                .label("Primary")
                .primary()
                .outline(),
        )
        .child(Button::new("o-danger").label("Danger").danger().outline())
        .child(
            Button::new("o-success")
                .label("Success")
                .success()
                .outline(),
        )
}

fn edge_row() -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .flex_wrap()
        .items_center()
        .gap_2()
        .child(
            Button::new("e-none")
                .label("Square")
                .primary()
                .rounded(ButtonRounded::None),
        )
        .child(
            Button::new("e-small")
                .label("Small radius")
                .primary()
                .rounded(ButtonRounded::Small),
        )
        .child(
            Button::new("e-medium")
                .label("Medium radius")
                .primary()
                .rounded(ButtonRounded::Medium),
        )
        .child(
            Button::new("e-large")
                .label("Large radius")
                .primary()
                .rounded(ButtonRounded::Large),
        )
        .child(
            Button::new("e-compact")
                .label("Compact")
                .primary()
                .compact(),
        )
}

fn icon_row() -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .flex_wrap()
        .items_center()
        .gap_2()
        .child(
            Button::new("i-plus")
                .label("Create")
                .primary()
                .icon(IconName::Plus),
        )
        .child(
            Button::new("i-search")
                .label("Search")
                .icon(IconName::Search),
        )
        .child(
            Button::new("i-settings")
                .label("Settings")
                .secondary()
                .icon(IconName::Settings),
        )
        // Icon-only buttons: no label, just the face.
        .child(Button::new("i-only-plus").primary().icon(IconName::Plus))
        .child(Button::new("i-only-bell").icon(IconName::Bell))
        .child(Button::new("i-only-preview").outline().icon(IconName::Eye))
}

/// States row. The Toggled button flips view state and keeps it.
fn state_row(view: &ButtonsApp, cx: &Context<ButtonsApp>) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .flex_wrap()
        .items_center()
        .gap_2()
        .child(
            Button::new("s-disabled")
                .label("Disabled")
                .primary()
                .disabled(true),
        )
        .child(
            Button::new("s-loading")
                .label("Loading")
                .primary()
                .loading(true),
        )
        .child(
            Button::new("s-selected")
                .label("Selected")
                .primary()
                .selected(true),
        )
        .child(
            Button::new("s-toggled")
                .label("Toggled")
                .secondary()
                .toggled(view.s_toggled)
                .on_click(cx.listener(|this, _: &ClickEvent, _, _| {
                    this.s_toggled = !this.s_toggled;
                })),
        )
}

/// Groups are fully controlled: the view owns the selected flags and
/// each click reports the new indices back.
fn group_row(view: &ButtonsApp, cx: &Context<ButtonsApp>) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .flex_wrap()
        .items_center()
        .gap_4()
        .child(
            ButtonGroup::new("g-single")
                .child(Button::new("g-a").label("Day").selected(view.g_single == 0))
                .child(
                    Button::new("g-b")
                        .label("Week")
                        .selected(view.g_single == 1),
                )
                .child(
                    Button::new("g-c")
                        .label("Month")
                        .selected(view.g_single == 2),
                )
                .on_click(cx.listener(|this, ix: &Vec<usize>, _, _| {
                    if let Some(&i) = ix.first() {
                        this.g_single = i;
                    }
                })),
        )
        .child(
            ButtonGroup::new("g-multiple")
                .multiple(true)
                .child(Button::new("gm-a").label("Bold").selected(view.g_multi[0]))
                .child(
                    Button::new("gm-b")
                        .label("Italic")
                        .selected(view.g_multi[1]),
                )
                .child(
                    Button::new("gm-c")
                        .label("Underline")
                        .selected(view.g_multi[2]),
                )
                .on_click(cx.listener(|this, ix: &Vec<usize>, _, _| {
                    // The group reports the full new selection.
                    this.g_multi = [ix.contains(&0), ix.contains(&1), ix.contains(&2)];
                })),
        )
        .child(
            ButtonGroup::new("g-segmented")
                .outline()
                .child(Button::new("gs-a").label("Left").selected(view.g_seg == 0))
                .child(
                    Button::new("gs-b")
                        .label("Center")
                        .selected(view.g_seg == 1),
                )
                .child(Button::new("gs-c").label("Right").selected(view.g_seg == 2))
                .on_click(cx.listener(|this, ix: &Vec<usize>, _, _| {
                    if let Some(&i) = ix.first() {
                        this.g_seg = i;
                    }
                })),
        )
}

/// Switches and segmented toggle groups, all view-driven.
fn toggle_row(view: &ButtonsApp, cx: &Context<ButtonsApp>) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .flex_wrap()
        .items_center()
        .gap_4()
        .child(
            Toggle::new("t-wifi")
                .label("Wi-Fi")
                .icon(IconName::Check)
                .checked(view.t_wifi)
                .on_click(cx.listener(|this, on: &bool, _, _| {
                    this.t_wifi = *on;
                })),
        )
        .child(
            Toggle::new("t-bt")
                .label("Bluetooth")
                .icon(IconName::Bell)
                .checked(view.t_bt)
                .on_click(cx.listener(|this, on: &bool, _, _| {
                    this.t_bt = *on;
                })),
        )
        .child(
            ToggleGroup::new("tg-view")
                .child(
                    Toggle::new("tg-list")
                        .label("List")
                        .checked(view.tg_view == 0)
                        .on_click(cx.listener(|this, _: &bool, _, _| {
                            this.tg_view = 0;
                        })),
                )
                .child(
                    Toggle::new("tg-grid")
                        .label("Grid")
                        .checked(view.tg_view == 1)
                        .on_click(cx.listener(|this, _: &bool, _, _| {
                            this.tg_view = 1;
                        })),
                ),
        )
        .child(
            ToggleGroup::new("tg-align")
                .segmented()
                .child(
                    Toggle::new("tg-left")
                        .label("Left")
                        .checked(view.tg_align == 0)
                        .on_click(cx.listener(|this, _: &bool, _, _| {
                            this.tg_align = 0;
                        })),
                )
                .child(
                    Toggle::new("tg-center")
                        .label("Center")
                        .checked(view.tg_align == 1)
                        .on_click(cx.listener(|this, _: &bool, _, _| {
                            this.tg_align = 1;
                        })),
                )
                .child(
                    Toggle::new("tg-right")
                        .label("Right")
                        .checked(view.tg_align == 2)
                        .on_click(cx.listener(|this, _: &bool, _, _| {
                            this.tg_align = 2;
                        })),
                ),
        )
}

/// The responsive toolbar: rightmost buttons collapse first as the window
/// narrows. Thresholds sit inside the default 1060px window so every
/// state is reachable by resizing.
const DYNAMIC_BAR: [DynamicButton; 5] = [
    DynamicButton {
        id: "dyn-new",
        label: "New",
        icon: IconName::Plus,
        collapse_at: 980.0,
        primary: true,
        tooltip: None,
        fill: None,
    },
    DynamicButton {
        id: "dyn-search",
        label: "Search",
        icon: IconName::Search,
        collapse_at: 860.0,
        primary: false,
        tooltip: None,
        fill: None,
    },
    DynamicButton {
        id: "dyn-star",
        label: "Star",
        icon: IconName::Star,
        collapse_at: 740.0,
        primary: false,
        tooltip: None,
        fill: None,
    },
    DynamicButton {
        id: "dyn-alerts",
        label: "Alerts",
        icon: IconName::Bell,
        collapse_at: 620.0,
        primary: false,
        tooltip: None,
        fill: None,
    },
    DynamicButton {
        id: "dyn-settings",
        label: "Settings",
        icon: IconName::Settings,
        collapse_at: 500.0,
        primary: false,
        // Custom hover text: the collapsed gear alone is cryptic.
        tooltip: Some("Application settings"),
        fill: None,
    },
];

fn dropdown_row() -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .flex_wrap()
        .items_center()
        .gap_2()
        .child(
            DropdownButton::new("d-actions")
                .button(
                    Button::new("d-main")
                        .label("Actions")
                        .primary()
                        .icon(IconName::ChevronDown),
                )
                .dropdown_menu(|menu, _, _| {
                    menu.menu("New file", Box::new(DemoAction))
                        .menu("Open…", Box::new(DemoAction))
                        .separator()
                        .menu("Save", Box::new(DemoAction))
                }),
        )
        .child(
            DropdownButton::new("d-more")
                .button(Button::new("d-main2").label("More").outline())
                .dropdown_menu(|menu, _, _| {
                    menu.menu("Duplicate", Box::new(DemoAction))
                        .menu("Archive", Box::new(DemoAction))
                }),
        )
}

impl Render for ButtonsApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = &cx.theme().colors;
        let theme_label = format!("Theme: {} (click for next)", self.theme);
        // Responsive toolbar: decide every button's shape from the viewport
        // width *before* building the row — a pure function of stored state,
        // so it settles in one pass with no layout feedback loop.
        let width = f32::from(window.viewport_size().width);
        for (i, b) in DYNAMIC_BAR.iter().enumerate() {
            self.dynamic[i] = b.show_label(width, self.dynamic[i]);
        }
        let collapsed = self.dynamic.iter().filter(|e| !**e).count();
        let bar_status = format!("{width:.0}px · {collapsed} collapsed — resize the window");
        // Live proof the radius comes from themes.rs: midnight reports 4px,
        // cyberpunk_neon 10px, and every default button below rounds to it.
        let edge_title = format!(
            "Edges · theme control radius {:.0}px",
            f32::from(cx.theme().radius)
        );
        div()
            .size_full()
            .bg(colors.background)
            .text_color(colors.foreground)
            .p_6()
            .flex()
            .flex_col()
            .gap_5()
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .child(div().text_lg().child("gpui-kit buttons · skins"))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_2()
                            .child(
                                Toggle::new("skin-toggle")
                                    .label("Lace skin")
                                    .checked(self.lace)
                                    .on_click(cx.listener(|this, on: &bool, window, cx| {
                                        this.set_skin(*on, window, cx);
                                    })),
                            )
                            .child(
                                Button::new("next-theme")
                                    .label(theme_label)
                                    .secondary()
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.next_theme(window, cx);
                                    })),
                            ),
                    ),
            )
            .child(section("Variants", variant_row()))
            .child(section("Outline", outline_row()))
            .child(section(&edge_title, edge_row()))
            .child(section("Icons · with label and icon-only", icon_row()))
            .child(section(
                "States · disabled, loading, selected, toggled (click Toggled)",
                state_row(self, cx),
            ))
            .child(section(
                "ButtonGroup · single, multiple, segmented (click to select)",
                group_row(self, cx),
            ))
            .child(section(
                "Toggle · switches and segmented groups (click them)",
                toggle_row(self, cx),
            ))
            .child(section(
                "DropdownButton · split button + menu",
                dropdown_row(),
            ))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .child("Dynamic · collapse to icon with hysteresis".to_string()),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .flex_nowrap()
                            .items_center()
                            .gap_2()
                            .p_2()
                            .children(
                                DYNAMIC_BAR
                                    .iter()
                                    .enumerate()
                                    .map(|(i, b)| b.render(self.dynamic[i], cx).flex_shrink_0()),
                            ),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(colors.muted_foreground)
                            .child(bar_status),
                    ),
            )
    }
}

fn main() {
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(|cx: &mut App| {
            gpui_kit::init(cx);
            cx.on_action(|_: &DemoAction, _: &mut App| {});
            // Snapshot the stock skin first so the header toggle can put it
            // back; then paint Lace on top (palette + radii, no glow, no
            // gradients).
            let stock = Theme::global(cx).clone();
            skin::apply("midnight", cx);

            let bounds = Bounds::centered(None, size(px(1060.), px(760.)), cx);
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("Lace buttons".into()),
                    ..Default::default()
                }),
                ..Default::default()
            };
            gpui_kit::open_window(options, cx, |window, cx| {
                cx.new(|cx| ButtonsApp::new(stock.clone(), window, cx))
            })
            .expect("failed to open buttons window");
        });
}
