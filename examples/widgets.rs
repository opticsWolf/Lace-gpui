//! A full-window widget gallery for `gpui-kit`, painted by Lace.
//!
//! Run it with:
//!
//! ```sh
//! cargo run --example widgets --features gpui-kit
//! ```
//!
//! The window follows the usual desktop shape: a title strip with theme
//! controls, a navigation sidebar, a scrolling component page, and a status
//! bar. The five pages cover the widgets an application actually reaches for:
//!
//! - **Controls** — buttons, checkboxes, radios, switches, rating, slider,
//!   progress and loading states.
//! - **Forms** — text, password, number and multi-line inputs, a select, a
//!   date picker, a colour picker, and settings rows.
//! - **Data** — a static table, a selectable list, tags, badges, avatars, a
//!   bar chart and a description list.
//! - **Feedback** — alerts, dialogs, sheets, notifications, popovers,
//!   dropdown menus, tooltips, skeleton loaders and an empty state.
//! - **Navigation** — breadcrumbs, tab bars, a stepper, pagination,
//!   keyboard hints, links and an accordion.
//!
//! The arrows in the header walk all sixteen Lace presets live. Everything
//! recolours through [`lace_gpui::skin`], no restart.

use gpui_kit::component::{
    accordion::Accordion,
    alert::Alert,
    avatar::Avatar,
    badge::Badge,
    breadcrumb::{Breadcrumb, BreadcrumbItem},
    button::{Button, ButtonGroup, ButtonVariant, ButtonVariants},
    chart::BarChart,
    checkbox::Checkbox,
    color_picker::{ColorPickerState, ColorSelect},
    date_picker::{DatePicker, DatePickerState},
    description_list::DescriptionList,
    dialog::{DialogClose, DialogFooter},
    empty::{
        Empty, EmptyContent, EmptyDescription, EmptyHeader, EmptyMedia, EmptyMediaVariant,
        EmptyTitle,
    },
    form::{Field, Form},
    input::{Input, InputState, NumberInput, Textarea, TextareaState},
    kbd::Kbd,
    link::Link,
    list::ListItem,
    menu::{DropdownMenu, PopupMenuItem},
    notification::Notification,
    pagination::Pagination,
    popover::Popover,
    progress::{Progress, ProgressCircle},
    radio::{Radio, RadioGroup},
    rating::Rating,
    select::{Select, SelectState},
    separator::Separator,
    skeleton::Skeleton,
    slider::{Slider, SliderState},
    spinner::Spinner,
    stepper::{Stepper, StepperItem},
    switch::Switch,
    tab::{Tab, TabBar},
    table::{Table, TableBody, TableCell, TableHead, TableHeader, TableRow},
    tag::Tag,
    ActiveTheme, Disableable, Icon, IconName, IndexPath, Sizable, WindowExt,
};
use gpui_kit::*;

use lace_gpui::{skin, themes};

// ---------------------------------------------------------------------------
// Layout helpers
// ---------------------------------------------------------------------------

const NAV: [(&str, &str, IconName); 5] = [
    (
        "Controls",
        "Buttons, selections, sliders, progress and loading states.",
        IconName::Settings2,
    ),
    (
        "Forms",
        "Inputs, selects, pickers and the setting rows that wrap them.",
        IconName::FileText,
    ),
    (
        "Data",
        "Tables, lists, tags, avatars, charts and description lists.",
        IconName::ChartPie,
    ),
    (
        "Feedback",
        "Alerts, dialogs, sheets, notifications, popovers and empty states.",
        IconName::Bell,
    ),
    (
        "Navigation",
        "Tabs, breadcrumbs, steppers, pagination, links and disclosure.",
        IconName::Menu,
    ),
];

fn row() -> Div {
    div().flex().items_center()
}

fn col() -> Div {
    div().flex().flex_col()
}

/// The shared card surface for a component group.
fn card(cx: &App) -> Div {
    let theme = cx.theme();
    div()
        .flex()
        .flex_col()
        .gap_4()
        .min_w_0()
        .p_5()
        .bg(theme.colors.list)
        .border_1()
        .border_color(theme.colors.border)
        .rounded(theme.radius_lg)
}

fn card_header(title: &'static str, description: &'static str, cx: &App) -> Div {
    let theme = cx.theme();
    col()
        .gap_1()
        .child(
            div()
                .text_size(px(15.))
                .text_color(theme.foreground)
                .child(title),
        )
        .child(
            div()
                .text_size(px(12.))
                .text_color(theme.muted_foreground)
                .child(description),
        )
}

/// A label + description on the left, a control on the right.
fn setting_row(
    title: &'static str,
    description: &'static str,
    control: impl IntoElement,
    cx: &App,
) -> Div {
    let muted = cx.theme().muted_foreground;
    row()
        .justify_between()
        .gap_6()
        .py(px(6.))
        .child(
            col()
                .flex_1()
                .gap_1()
                .child(div().text_size(px(13.)).child(title))
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(muted)
                        .child(description),
                ),
        )
        .child(control)
}

fn rule(cx: &App) -> Div {
    div().h(px(1.)).w_full().bg(cx.theme().border)
}

// ---------------------------------------------------------------------------
// Overlays shared by the header and the Feedback page
// ---------------------------------------------------------------------------

fn open_about(window: &mut Window, cx: &mut App) {
    window.open_dialog(cx, |dialog, _, _| {
        dialog
            .title("About Lace Widgets")
            .child(
                col()
                    .gap_4()
                    .w(px(440.))
                    .child(div().text_size(px(13.)).child(
                        "A gpui-kit 0.8 component gallery painted by Lace's OKLCH theming engine.",
                    ))
                    .child(
                        DescriptionList::new()
                            .columns(1)
                            .item("Presets", "16 curated themes", 1)
                            .item("Tokens", "78 colours + 7 geometry values", 1)
                            .item("Contrast", "WCAG floors enforced on every build", 1)
                            .item("Bridge", "lace_gpui::skin \u{2192} gpui_kit::ThemeColor", 1),
                    ),
            )
            .footer(
                DialogFooter::new()
                    .child(DialogClose::new().trigger(|button| button.label("Close"))),
            )
    });
}

fn open_create_dialog(window: &mut Window, cx: &mut App) {
    let name = cx.new(|cx| InputState::new(window, cx).placeholder("Launch plan"));
    let note = cx.new(|cx| InputState::new(window, cx).placeholder("What is this for?"));

    window.open_dialog(cx, move |dialog, _, _| {
        dialog
            .title("Create item")
            .child(
                col()
                    .gap_4()
                    .w(px(420.))
                    .child(Field::new().label("Name").child(Input::new(&name)))
                    .child(Field::new().label("Description").child(Input::new(&note))),
            )
            .footer(
                DialogFooter::new()
                    .child(DialogClose::new().trigger(|button| button.label("Cancel")))
                    .child(
                        Button::new("create-item")
                            .label("Create")
                            .primary()
                            .on_click(|_, window, cx| {
                                window.push_notification(
                                    Notification::success("New item created.").title("Created"),
                                    cx,
                                );
                                window.close_dialog(cx);
                            }),
                    ),
            )
    });
}

fn open_inspector(window: &mut Window, cx: &mut App) {
    window.open_sheet(cx, |sheet, _, cx| {
        let muted = cx.theme().muted_foreground;
        sheet
            .title("Inspector")
            .size(px(360.))
            .child(
                col()
                    .gap_5()
                    .child(
                        col()
                            .gap_1()
                            .child(div().text_size(px(13.)).child("Selection"))
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(muted)
                                    .child("Nothing is selected. Pick a row in the Data page to inspect its tokens."),
                            ),
                    )
                    .child(rule(cx))
                    .child(
                        DescriptionList::new()
                            .columns(1)
                            .item("Panel", "DockSkin", 1)
                            .item("Chassis", "classic 4px", 1)
                            .item("Scrollbar", "thin", 1),
                    ),
            )
    });
}

fn open_delete_alert(window: &mut Window, cx: &mut App) {
    window.open_alert_dialog(cx, |alert, _, _| {
        alert
            .title("Delete this project?")
            .description(
                "The project and its local theme overrides will be removed. This cannot be undone.",
            )
            .ok_text("Delete")
            .ok_variant(ButtonVariant::Danger)
            .show_cancel(true)
    });
}

// ---------------------------------------------------------------------------
// The application
// ---------------------------------------------------------------------------

struct WidgetsApp {
    theme_name: SharedString,
    page: usize,

    // Controls page state.
    pinned: bool,
    newsletter: bool,
    analytics: bool,
    density_mode: usize,
    auto_update: bool,
    compact_mode: bool,
    notifications: bool,
    rating: usize,

    // Navigation page state.
    tab_index: usize,
    segmented_index: usize,
    step: usize,
    page_number: usize,

    // Form / input state.
    search: Entity<InputState>,
    name: Entity<InputState>,
    email: Entity<InputState>,
    password: Entity<InputState>,
    budget: Entity<InputState>,
    readonly: Entity<InputState>,
    notes: Entity<TextareaState>,
    density: Entity<SelectState<Vec<&'static str>>>,
    start_date: Entity<DatePickerState>,
    accent: Entity<ColorPickerState>,
    volume: Entity<SliderState>,
}

impl WidgetsApp {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search =
            cx.new(|cx| InputState::new(window, cx).placeholder("Search components\u{2026}"));

        let name = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Ada Lovelace")
                .default_value("Grace Hopper")
        });
        let email = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("name@example.com")
                .default_value("grace@example.com")
        });
        let password = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}")
                .masked(true)
                .default_value("lace-theme-2026")
        });
        let budget = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("0.00")
                .default_value("1280")
        });
        let readonly = cx
            .new(|cx| InputState::new(window, cx).default_value("lace://midnight/contrast?step=1"));
        let notes = cx.new(|cx| {
            TextareaState::new(window, cx)
                .placeholder("Anything else we should know?")
                .default_value("Build the widget showcase with the Lace theme applied.")
        });

        let density = cx.new(|cx| {
            let items: Vec<&'static str> = vec!["Compact", "Comfortable", "Spacious"];
            SelectState::new(items, Some(IndexPath::new(1)), window, cx)
        });
        let start_date = cx.new(|cx| DatePickerState::new(window, cx));
        let accent = cx.new(|cx| {
            let primary = cx.theme().primary;
            ColorPickerState::new(window, cx).default_value(primary)
        });
        let volume = cx.new(|_| {
            SliderState::new()
                .min(0.)
                .max(100.)
                .step(1.)
                .default_value(64.)
        });

        Self {
            theme_name: "midnight".into(),
            page: 0,
            pinned: false,
            newsletter: true,
            analytics: false,
            density_mode: 1,
            auto_update: true,
            compact_mode: false,
            notifications: true,
            rating: 4,
            tab_index: 0,
            segmented_index: 1,
            step: 1,
            page_number: 2,
            search,
            name,
            email,
            password,
            budget,
            readonly,
            notes,
            density,
            start_date,
            accent,
            volume,
        }
    }

    fn cycle_theme(&mut self, step: isize, cx: &mut Context<Self>) {
        let current = themes::by_name(&self.theme_name)
            .map(|spec| spec.name)
            .unwrap_or("midnight");
        let index = themes::ALL
            .iter()
            .position(|spec| spec.name == current)
            .unwrap_or(0);
        let len = themes::ALL.len() as isize;
        let next = themes::ALL[((index as isize + step).rem_euclid(len)) as usize].name;
        skin::apply(next, cx);
        self.theme_name = next.into();
        cx.notify();
    }

    // -- header ------------------------------------------------------------

    fn header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let background = cx.theme().title_bar;
        let border = cx.theme().title_bar_border;
        let muted = cx.theme().muted_foreground;
        let primary = cx.theme().primary;
        let primary_foreground = cx.theme().primary_foreground;
        let radius = cx.theme().radius;
        let radius_lg = cx.theme().radius_lg;
        let panel_bg = cx.theme().background;
        let panel_border = cx.theme().border;
        let is_dark = cx.theme().is_dark();
        let theme_name = self.theme_name.clone();

        div()
            .h(px(58.))
            .flex_none()
            .flex()
            .items_center()
            .justify_between()
            .gap_4()
            .px(px(16.))
            .bg(background)
            .border_b_1()
            .border_color(border)
            .child(
                row()
                    .gap_3()
                    .child(
                        div()
                            .size(px(32.))
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(radius)
                            .bg(primary)
                            .child(
                                Icon::new(IconName::Palette)
                                    .large()
                                    .text_color(primary_foreground),
                            ),
                    )
                    .child(
                        col()
                            .gap_1()
                            .child(div().text_size(px(15.)).child("Lace Widgets"))
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(muted)
                                    .child("A themed gpui-kit component gallery"),
                            ),
                    )
                    .child(Tag::new().child("gpui-kit 0.8").outline()),
            )
            .child(
                row()
                    .gap_2()
                    .child(
                        row()
                            .gap_2()
                            .px(px(8.))
                            .py(px(4.))
                            .rounded(radius_lg)
                            .bg(panel_bg)
                            .border_1()
                            .border_color(panel_border)
                            .child(
                                Icon::new(if is_dark {
                                    IconName::Moon
                                } else {
                                    IconName::Sun
                                })
                                .small()
                                .text_color(primary),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(muted)
                                    .child(if is_dark { "Dark" } else { "Light" }),
                            ),
                    )
                    .child(
                        Button::new("theme-prev")
                            .icon(IconName::ChevronLeft)
                            .ghost()
                            .small()
                            .tooltip("Previous Lace theme")
                            .on_click(cx.listener(|this, _, _, cx| this.cycle_theme(-1, cx))),
                    )
                    .child(
                        div()
                            .min_w(px(122.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .px(px(10.))
                            .py(px(4.))
                            .bg(panel_bg)
                            .border_1()
                            .border_color(panel_border)
                            .rounded(radius)
                            .text_size(px(12.))
                            .child(theme_name),
                    )
                    .child(
                        Button::new("theme-next")
                            .icon(IconName::ChevronRight)
                            .ghost()
                            .small()
                            .tooltip("Next Lace theme")
                            .on_click(cx.listener(|this, _, _, cx| this.cycle_theme(1, cx))),
                    )
                    .child(
                        Button::new("header-new")
                            .label("New item")
                            .primary()
                            .icon(IconName::Plus)
                            .on_click(cx.listener(|_, _, window, cx| {
                                open_create_dialog(window, cx);
                            })),
                    )
                    .child(
                        Button::new("header-about")
                            .icon(IconName::Info)
                            .ghost()
                            .tooltip("About this demo")
                            .on_click(cx.listener(|_, _, window, cx| {
                                open_about(window, cx);
                            })),
                    )
                    .child(
                        Badge::new().count(3).child(
                            Button::new("header-bell")
                                .icon(IconName::Bell)
                                .ghost()
                                .tooltip("Notifications")
                                .on_click(cx.listener(|_, _, window, cx| {
                                    window.push_notification(
                                        Notification::info("3 unread updates in the gallery.")
                                            .title("Inbox"),
                                        cx,
                                    );
                                })),
                        ),
                    ),
            )
    }

    // -- sidebar -----------------------------------------------------------

    fn nav_item(
        &self,
        index: usize,
        label: &'static str,
        icon: IconName,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        ListItem::new(("nav", index))
            .selected(self.page == index)
            .py(px(7.))
            .on_click(cx.listener(move |this, _, _, cx| {
                if this.page != index {
                    this.page = index;
                    cx.notify();
                }
            }))
            .child(
                row()
                    .gap_3()
                    .child(Icon::new(icon).small())
                    .child(div().text_size(px(13.)).child(label)),
            )
    }

    fn sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let background = cx.theme().sidebar;
        let border = cx.theme().sidebar_border;
        let muted = cx.theme().muted_foreground;
        let foreground = cx.theme().foreground;
        let primary = cx.theme().primary;
        let radius = cx.theme().radius;
        let card_bg = cx.theme().colors.list;
        let card_border = cx.theme().border;

        div()
            .w(px(240.))
            .flex_none()
            .h_full()
            .flex()
            .flex_col()
            .gap_2()
            .p_3()
            .bg(background)
            .border_r_1()
            .border_color(border)
            .child(
                Input::new(&self.search)
                    .prefix(Icon::new(IconName::Search).small().text_color(muted))
                    .cleanable(true)
                    .w_full(),
            )
            .child(
                div()
                    .px_2()
                    .pt_2()
                    .text_size(px(10.))
                    .text_color(muted)
                    .child("COMPONENTS"),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(self.nav_item(0, "Controls", IconName::Settings2, cx))
                    .child(self.nav_item(1, "Forms", IconName::FileText, cx))
                    .child(self.nav_item(2, "Data", IconName::ChartPie, cx))
                    .child(self.nav_item(3, "Feedback", IconName::Bell, cx))
                    .child(self.nav_item(4, "Navigation", IconName::Menu, cx)),
            )
            .child(div().flex_1())
            .child(
                col()
                    .gap_2()
                    .p_3()
                    .bg(card_bg)
                    .border_1()
                    .border_color(card_border)
                    .rounded(radius)
                    .child(
                        row()
                            .gap_2()
                            .child(Icon::new(IconName::Palette).small().text_color(primary))
                            .child(div().text_size(px(12.)).child("Lace theme engine")),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(muted)
                            .child("78 colours \u{00B7} 7 geometry tokens \u{00B7} OKLCH"),
                    ),
            )
            .child(
                row()
                    .gap_3()
                    .p_2()
                    .child(Avatar::new().name("Ada Lovelace"))
                    .child(
                        col()
                            .flex_1()
                            .gap_1()
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(foreground)
                                    .child("Ada Lovelace"),
                            )
                            .child(
                                div()
                                    .text_size(px(10.))
                                    .text_color(muted)
                                    .child("Design systems"),
                            ),
                    )
                    .child(
                        Button::new("user-settings")
                            .icon(IconName::Settings)
                            .ghost()
                            .small()
                            .tooltip("Settings")
                            .on_click(cx.listener(|_, _, window, cx| {
                                window.push_notification(
                                    Notification::info("Settings live in the Forms page.")
                                        .title("Settings"),
                                    cx,
                                );
                            })),
                    ),
            )
    }

    // -- content shell -----------------------------------------------------

    fn page(&self, cx: &mut Context<Self>) -> AnyElement {
        match self.page {
            0 => self.controls_page(cx).into_any_element(),
            1 => self.forms_page(cx).into_any_element(),
            2 => self.data_page(cx).into_any_element(),
            3 => self.feedback_page(cx).into_any_element(),
            _ => self.navigation_page(cx).into_any_element(),
        }
    }

    fn content_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let muted = theme.muted_foreground;
        let border = theme.border;
        let (title, description, _) = NAV[self.page];

        div()
            .flex_none()
            .px(px(24.))
            .py(px(18.))
            .border_b_1()
            .border_color(border)
            .flex()
            .items_end()
            .justify_between()
            .gap_4()
            .child(
                col()
                    .gap_2()
                    .child(
                        Breadcrumb::new()
                            .child(BreadcrumbItem::new("Workspace"))
                            .child(BreadcrumbItem::new("Components"))
                            .child(BreadcrumbItem::new(title).disabled(true)),
                    )
                    .child(div().text_size(px(22.)).child(title))
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(muted)
                            .child(description),
                    ),
            )
            .child(
                row()
                    .gap_2()
                    .child(
                        Button::new("header-about-2")
                            .label("About")
                            .outline()
                            .on_click(cx.listener(|_, _, window, cx| {
                                open_about(window, cx);
                            })),
                    )
                    .child(
                        Button::new("header-create-2")
                            .label("Create")
                            .primary()
                            .icon(IconName::Plus)
                            .on_click(cx.listener(|_, _, window, cx| {
                                open_create_dialog(window, cx);
                            })),
                    ),
            )
    }

    fn content(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let background = cx.theme().background;
        div()
            .flex_1()
            .min_w_0()
            .h_full()
            .flex()
            .flex_col()
            .bg(background)
            .child(self.content_header(cx))
            .child(
                div()
                    .id(("widgets-scroll", self.page))
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .p(px(24.))
                    .child(self.page(cx)),
            )
    }

    fn status_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let background = theme.status_bar;
        let border = theme.status_bar_border;
        let muted = theme.muted_foreground;
        let success = theme.success;
        let is_light = !theme.is_dark();

        div()
            .h(px(30.))
            .flex_none()
            .flex()
            .items_center()
            .justify_between()
            .px(px(12.))
            .bg(background)
            .border_t_1()
            .border_color(border)
            .text_size(px(11.))
            .text_color(muted)
            .child(
                row()
                    .gap_2()
                    .child(Icon::new(IconName::CircleCheck).small().text_color(success))
                    .child("Ready")
                    .child(div().h(px(12.)).w(px(1.)).bg(border))
                    .child("All widgets rendered"),
            )
            .child(
                row()
                    .gap_4()
                    .child(format!("theme: {}", self.theme_name))
                    .child(format!("mode: {}", if is_light { "light" } else { "dark" }))
                    .child(format!("{} presets \u{00B7} 78 tokens", themes::ALL.len()))
                    .child("lace-gpui 0.9.7"),
            )
    }

    // -- page: controls ----------------------------------------------------

    fn controls_page(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let muted = theme.muted_foreground;
        let border = theme.border;
        let primary = theme.primary;

        div()
            .flex()
            .flex_col()
            .gap_5()
            // Buttons -----------------------------------------------------
            .child(
                card(cx)
                    .child(card_header(
                        "Buttons",
                        "Variants, sizes, icons, groups and states.",
                        cx,
                    ))
                    .child(
                        row()
                            .flex_wrap()
                            .gap_2()
                            .child(Button::new("btn-default").label("Default"))
                            .child(Button::new("btn-primary").label("Primary").primary())
                            .child(Button::new("btn-secondary").label("Secondary").secondary())
                            .child(Button::new("btn-danger").label("Danger").danger())
                            .child(Button::new("btn-warning").label("Warning").warning())
                            .child(Button::new("btn-success").label("Success").success())
                            .child(Button::new("btn-info").label("Info").info())
                            .child(Button::new("btn-ghost").label("Ghost").ghost())
                            .child(Button::new("btn-link").label("Link").link())
                            .child(Button::new("btn-text").label("Text").text()),
                    )
                    .child(
                        row()
                            .flex_wrap()
                            .gap_2()
                            .child(Button::new("btn-small").label("Small").small())
                            .child(Button::new("btn-medium").label("Medium"))
                            .child(Button::new("btn-large").label("Large").large())
                            .child(
                                Button::new("btn-icon-plus")
                                    .icon(IconName::Plus)
                                    .tooltip("New item"),
                            )
                            .child(
                                Button::new("btn-icon-search")
                                    .icon(IconName::Search)
                                    .tooltip("Search"),
                            )
                            .child(
                                Button::new("btn-loading")
                                    .label("Saving\u{2026}")
                                    .loading(true),
                            )
                            .child(Button::new("btn-disabled").label("Disabled").disabled(true))
                            .child(
                                Button::new("btn-pinned")
                                    .label("Pinned")
                                    .toggled(self.pinned)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.pinned = !this.pinned;
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(
                        row()
                            .flex_wrap()
                            .gap_3()
                            .child(
                                ButtonGroup::new("button-group")
                                    .outline()
                                    .child(Button::new("group-day").label("Day"))
                                    .child(Button::new("group-week").label("Week"))
                                    .child(Button::new("group-month").label("Month")),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(muted)
                                    .child("Button groups share one border and one focus ring."),
                            ),
                    ),
            )
            // Selection ---------------------------------------------------
            .child(
                div()
                    .flex()
                    .gap_5()
                    .items_start()
                    .child(
                        card(cx)
                            .flex_1()
                            .child(card_header(
                                "Checkboxes & radios",
                                "Independent toggles and one-of-many choices.",
                                cx,
                            ))
                            .child(
                                col()
                                    .gap_3()
                                    .child(
                                        Checkbox::new("check-newsletter")
                                            .checked(self.newsletter)
                                            .label("Receive the weekly theme digest")
                                            .on_change(cx.listener(|this, checked, _, cx| {
                                                this.newsletter = *checked;
                                                cx.notify();
                                            })),
                                    )
                                    .child(
                                        Checkbox::new("check-analytics")
                                            .checked(self.analytics)
                                            .label("Share anonymous usage analytics")
                                            .on_change(cx.listener(|this, checked, _, cx| {
                                                this.analytics = *checked;
                                                cx.notify();
                                            })),
                                    )
                                    .child(
                                        Checkbox::new("check-disabled")
                                            .label("Enterprise audit log")
                                            .disabled(true),
                                    )
                                    .child(div().h(px(1.)).bg(border))
                                    .child(
                                        RadioGroup::new("density-group")
                                            .selected_index(Some(self.density_mode))
                                            .on_change(cx.listener(|this, index, _, cx| {
                                                this.density_mode = *index;
                                                cx.notify();
                                            }))
                                            .child(Radio::new("density-compact").label("Compact"))
                                            .child(
                                                Radio::new("density-comfortable")
                                                    .label("Comfortable"),
                                            )
                                            .child(
                                                Radio::new("density-spacious").label("Spacious"),
                                            ),
                                    ),
                            ),
                    )
                    .child(
                        card(cx)
                            .flex_1()
                            .child(card_header(
                                "Switches & rating",
                                "Immediate settings and a compact score picker.",
                                cx,
                            ))
                            .child(
                                col()
                                    .gap_2()
                                    .child(
                                        Switch::new("switch-auto-update")
                                            .checked(self.auto_update)
                                            .label("Install updates automatically")
                                            .on_change(cx.listener(|this, checked, _, cx| {
                                                this.auto_update = *checked;
                                                cx.notify();
                                            })),
                                    )
                                    .child(
                                        Switch::new("switch-compact")
                                            .checked(self.compact_mode)
                                            .label("Compact sidebar")
                                            .on_change(cx.listener(|this, checked, _, cx| {
                                                this.compact_mode = *checked;
                                                cx.notify();
                                            })),
                                    )
                                    .child(
                                        Switch::new("switch-notifications")
                                            .checked(self.notifications)
                                            .label("Desktop notifications")
                                            .on_change(cx.listener(|this, checked, _, cx| {
                                                this.notifications = *checked;
                                                cx.notify();
                                            })),
                                    )
                                    .child(div().h(px(1.)).bg(border))
                                    .child(
                                        row()
                                            .justify_between()
                                            .child(
                                                col()
                                                    .gap_1()
                                                    .child(div().text_size(px(13.)).child("Rating"))
                                                    .child(
                                                        div()
                                                            .text_size(px(11.))
                                                            .text_color(muted)
                                                            .child("Click a star"),
                                                    ),
                                            )
                                            .child(
                                                Rating::new("rating")
                                                    .value(self.rating)
                                                    .max(5)
                                                    .on_click(cx.listener(|this, value, _, cx| {
                                                        this.rating = *value;
                                                        cx.notify();
                                                    })),
                                            ),
                                    ),
                            ),
                    ),
            )
            // Slider, progress and loading ---------------------------------
            .child(
                card(cx)
                    .child(card_header(
                        "Sliders, progress & loading",
                        "The quiet feedback layer: values, completion and waiting.",
                        cx,
                    ))
                    .child(
                        row()
                            .items_start()
                            .gap_8()
                            .child(
                                col()
                                    .gap_2()
                                    .flex_1()
                                    .child(
                                        row()
                                            .justify_between()
                                            .child(div().text_size(px(12.)).child("Volume"))
                                            .child(
                                                div()
                                                    .text_size(px(11.))
                                                    .text_color(muted)
                                                    .child("drag the thumb"),
                                            ),
                                    )
                                    .child(Slider::new(&self.volume).accessibility_label("Volume")),
                            )
                            .child(
                                col()
                                    .gap_3()
                                    .w(px(300.))
                                    .child(
                                        row()
                                            .justify_between()
                                            .text_size(px(11.))
                                            .text_color(muted)
                                            .child("Tokens built")
                                            .child("68%"),
                                    )
                                    .child(Progress::new("progress-tokens").value(68.))
                                    .child(
                                        row()
                                            .justify_between()
                                            .text_size(px(11.))
                                            .text_color(muted)
                                            .child("Contrast pass")
                                            .child("running"),
                                    )
                                    .child(
                                        Progress::new("progress-contrast")
                                            .loading(true)
                                            .color(primary),
                                    )
                                    .child(
                                        row()
                                            .gap_3()
                                            .child(
                                                ProgressCircle::new("circle-progress").value(72.),
                                            )
                                            .child(
                                                col()
                                                    .gap_1()
                                                    .child(div().text_size(px(12.)).child("72%"))
                                                    .child(
                                                        div()
                                                            .text_size(px(10.))
                                                            .text_color(muted)
                                                            .child("geometry coverage"),
                                                    ),
                                            ),
                                    ),
                            ),
                    )
                    .child(div().h(px(1.)).bg(border))
                    .child(
                        row()
                            .items_start()
                            .gap_6()
                            .child(
                                row().gap_2().child(Spinner::new()).child(
                                    div()
                                        .text_size(px(12.))
                                        .text_color(muted)
                                        .child("Loading the next theme\u{2026}"),
                                ),
                            )
                            .child(
                                col().gap_2().flex_1().child(
                                    row()
                                        .gap_3()
                                        .child(Skeleton::new().size(px(32.)).rounded(px(999.)))
                                        .child(
                                            col()
                                                .gap_2()
                                                .flex_1()
                                                .child(Skeleton::new().h(px(10.)).w(px(240.)))
                                                .child(Skeleton::new().h(px(10.)).w(px(160.))),
                                        ),
                                ),
                            ),
                    ),
            )
    }

    // -- page: forms -------------------------------------------------------

    fn forms_page(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;

        div()
            .flex()
            .flex_col()
            .gap_5()
            .child(
                card(cx)
                    .child(card_header(
                        "Account form",
                        "Field labels, help text and the input family.",
                        cx,
                    ))
                    .child(
                        Form::new()
                            .columns(2)
                            .child(
                                Field::new()
                                    .label("Full name")
                                    .description("Shown on your profile.")
                                    .child(Input::new(&self.name)),
                            )
                            .child(
                                Field::new()
                                    .label("Email")
                                    .description("Used for sign-in.")
                                    .child(Input::new(&self.email)),
                            )
                            .child(
                                Field::new()
                                    .label("Password")
                                    .description("Masked by the input state.")
                                    .child(Input::new(&self.password)),
                            )
                            .child(
                                Field::new()
                                    .label("Monthly budget")
                                    .description("A number input with a prefix.")
                                    .child(
                                        NumberInput::new(&self.budget)
                                            .prefix(div().child("$"))
                                            .accessibility_label("Monthly budget"),
                                    ),
                            )
                            .child(
                                Field::new()
                                    .label("Density")
                                    .description("Searchable select.")
                                    .child(
                                        Select::new(&self.density)
                                            .placeholder("Choose a density")
                                            .accessibility_label("Density"),
                                    ),
                            )
                            .child(
                                Field::new()
                                    .label("Start date")
                                    .description("A calendar popover.")
                                    .child(
                                        DatePicker::new(&self.start_date)
                                            .placeholder("Pick a date"),
                                    ),
                            )
                            .child(
                                Field::new()
                                    .label("Accent")
                                    .description("A colour picker with sliders.")
                                    .child(
                                        ColorSelect::new(&self.accent).placeholder("Pick a colour"),
                                    ),
                            )
                            .child(
                                Field::new()
                                    .label("Clipboard")
                                    .description("A read-only field.")
                                    .child(Input::new(&self.readonly).readonly(true)),
                            )
                            .child(
                                Field::new()
                                    .label("Notes")
                                    .description("Multi-line input with a fixed height.")
                                    .col_span(2)
                                    .child(Textarea::new(&self.notes).h(px(104.))),
                            )
                            .footer(
                                row()
                                    .gap_2()
                                    .child(
                                        Button::new("form-reset")
                                            .label("Reset")
                                            .outline()
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.name.update(cx, |state, cx| {
                                                    state.set_value("", window, cx);
                                                });
                                                this.email.update(cx, |state, cx| {
                                                    state.set_value("", window, cx);
                                                });
                                                cx.notify();
                                            })),
                                    )
                                    .child(
                                        Button::new("form-save")
                                            .label("Save changes")
                                            .primary()
                                            .on_click(cx.listener(|_, _, window, cx| {
                                                window.push_notification(
                                                    Notification::success(
                                                        "Profile settings saved for this theme.",
                                                    )
                                                    .title("Saved"),
                                                    cx,
                                                );
                                            })),
                                    ),
                            ),
                    ),
            )
            .child(
                card(cx)
                    .child(card_header(
                        "Preferences",
                        "Switches and settings rows, the everyday settings pattern.",
                        cx,
                    ))
                    .child(setting_row(
                        "Automatic updates",
                        "Install Lace preset updates in the background.",
                        Switch::new("pref-auto")
                            .checked(self.auto_update)
                            .on_change(cx.listener(|this, checked, _, cx| {
                                this.auto_update = *checked;
                                cx.notify();
                            })),
                        cx,
                    ))
                    .child(rule(cx))
                    .child(setting_row(
                        "Compact mode",
                        "Tighten list rows and control heights.",
                        Switch::new("pref-compact")
                            .checked(self.compact_mode)
                            .on_change(cx.listener(|this, checked, _, cx| {
                                this.compact_mode = *checked;
                                cx.notify();
                            })),
                        cx,
                    ))
                    .child(rule(cx))
                    .child(setting_row(
                        "Desktop notifications",
                        "Show a toast when a theme passes its contrast audit.",
                        Switch::new("pref-notifications")
                            .checked(self.notifications)
                            .on_change(cx.listener(|this, checked, _, cx| {
                                this.notifications = *checked;
                                cx.notify();
                            })),
                        cx,
                    ))
                    .child(rule(cx))
                    .child(setting_row(
                        "Search scope",
                        "The sidebar filter searches component names and tokens.",
                        Tag::new().child("All pages"),
                        cx,
                    ))
                    .child(rule(cx))
                    .child(
                        row()
                            .gap_4()
                            .text_size(px(11.))
                            .text_color(muted)
                            .child(
                                Link::new("docs-link")
                                    .href("https://gpui-kit.com")
                                    .child("gpui-kit documentation"),
                            )
                            .child("\u{00B7}")
                            .child(
                                Link::new("lace-link")
                                    .href("https://github.com/opticsWolf/Lace")
                                    .child("Lace on GitHub"),
                            ),
                    ),
            )
    }

    // -- page: data --------------------------------------------------------

    fn data_page(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let muted = theme.muted_foreground;
        let border = theme.border;
        let radius = theme.radius_lg;
        let theme_name = self.theme_name.clone();
        let is_light = !theme.is_dark();

        div()
            .flex()
            .flex_col()
            .gap_5()
            .child(
                card(cx)
                    .child(card_header(
                        "Table",
                        "A static, composable table with headers, rows and cells.",
                        cx,
                    ))
                    .child(
                        Table::new()
                            .small()
                            .w_full()
                            .child(
                                TableHeader::new().child(
                                    TableRow::new()
                                        .child(TableHead::new().child("Widget"))
                                        .child(TableHead::new().child("Group"))
                                        .child(TableHead::new().child("State"))
                                        .child(TableHead::new().child("Tokens")),
                                ),
                            )
                            .child(
                                TableBody::new()
                                    .child(
                                        TableRow::new()
                                            .child(TableCell::new().child("Button"))
                                            .child(TableCell::new().child("Controls"))
                                            .child(
                                                TableCell::new()
                                                    .child(Tag::success().child("Stable")),
                                            )
                                            .child(TableCell::new().child("12")),
                                    )
                                    .child(
                                        TableRow::new()
                                            .child(TableCell::new().child("Input"))
                                            .child(TableCell::new().child("Forms"))
                                            .child(
                                                TableCell::new()
                                                    .child(Tag::success().child("Stable")),
                                            )
                                            .child(TableCell::new().child("9")),
                                    )
                                    .child(
                                        TableRow::new()
                                            .child(TableCell::new().child("Table"))
                                            .child(TableCell::new().child("Data"))
                                            .child(TableCell::new().child(Tag::info().child("New")))
                                            .child(TableCell::new().child("14")),
                                    )
                                    .child(
                                        TableRow::new()
                                            .child(TableCell::new().child("Chart"))
                                            .child(TableCell::new().child("Data"))
                                            .child(
                                                TableCell::new()
                                                    .child(Tag::warning().child("Beta")),
                                            )
                                            .child(TableCell::new().child("8")),
                                    )
                                    .child(
                                        TableRow::new()
                                            .child(TableCell::new().child("Dialog"))
                                            .child(TableCell::new().child("Feedback"))
                                            .child(
                                                TableCell::new()
                                                    .child(Tag::success().child("Stable")),
                                            )
                                            .child(TableCell::new().child("11")),
                                    ),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap_5()
                    .items_start()
                    .child(
                        card(cx)
                            .flex_1()
                            .child(card_header(
                                "List",
                                "Selectable rows with avatars and trailing state.",
                                cx,
                            ))
                            .child(
                                col()
                                    .gap_1()
                                    .border_1()
                                    .border_color(border)
                                    .rounded(radius)
                                    .overflow_hidden()
                                    .child(
                                        ListItem::new("list-ada").py(px(8.)).selected(true).child(
                                            row()
                                                .justify_between()
                                                .gap_3()
                                                .w_full()
                                                .child(
                                                    row()
                                                        .gap_3()
                                                        .child(Avatar::new().name("Ada Lovelace"))
                                                        .child(
                                                            col()
                                                                .gap_1()
                                                                .child(
                                                                    div()
                                                                        .text_size(px(12.))
                                                                        .child("Ada Lovelace"),
                                                                )
                                                                .child(
                                                                    div()
                                                                        .text_size(px(10.))
                                                                        .text_color(muted)
                                                                        .child("Theme author"),
                                                                ),
                                                        ),
                                                )
                                                .child(Tag::success().child("Online")),
                                        ),
                                    )
                                    .child(
                                        ListItem::new("list-grace").py(px(8.)).child(
                                            row()
                                                .justify_between()
                                                .gap_3()
                                                .w_full()
                                                .child(
                                                    row()
                                                        .gap_3()
                                                        .child(Avatar::new().name("Grace Hopper"))
                                                        .child(
                                                            col()
                                                                .gap_1()
                                                                .child(
                                                                    div()
                                                                        .text_size(px(12.))
                                                                        .child("Grace Hopper"),
                                                                )
                                                                .child(
                                                                    div()
                                                                        .text_size(px(10.))
                                                                        .text_color(muted)
                                                                        .child("Compiler"),
                                                                ),
                                                        ),
                                                )
                                                .child(Tag::new().child("Away")),
                                        ),
                                    )
                                    .child(
                                        ListItem::new("list-linus").py(px(8.)).child(
                                            row()
                                                .justify_between()
                                                .gap_3()
                                                .w_full()
                                                .child(
                                                    row()
                                                        .gap_3()
                                                        .child(Avatar::new().name("Linus Torvalds"))
                                                        .child(
                                                            col()
                                                                .gap_1()
                                                                .child(
                                                                    div()
                                                                        .text_size(px(12.))
                                                                        .child("Linus Torvalds"),
                                                                )
                                                                .child(
                                                                    div()
                                                                        .text_size(px(10.))
                                                                        .text_color(muted)
                                                                        .child("Kernel"),
                                                                ),
                                                        ),
                                                )
                                                .child(Tag::danger().child("Offline")),
                                        ),
                                    )
                                    .child(
                                        ListItem::new("list-margaret").py(px(8.)).child(
                                            row()
                                                .justify_between()
                                                .gap_3()
                                                .w_full()
                                                .child(
                                                    row()
                                                        .gap_3()
                                                        .child(
                                                            Avatar::new().name("Margaret Hamilton"),
                                                        )
                                                        .child(
                                                            col()
                                                                .gap_1()
                                                                .child(
                                                                    div()
                                                                        .text_size(px(12.))
                                                                        .child("Margaret Hamilton"),
                                                                )
                                                                .child(
                                                                    div()
                                                                        .text_size(px(10.))
                                                                        .text_color(muted)
                                                                        .child("Flight software"),
                                                                ),
                                                        ),
                                                )
                                                .child(Tag::info().child("Review")),
                                        ),
                                    ),
                            ),
                    )
                    .child(
                        card(cx)
                            .flex_1()
                            .child(card_header(
                                "Tags, badges & avatars",
                                "Small status elements and identity chips.",
                                cx,
                            ))
                            .child(
                                row()
                                    .flex_wrap()
                                    .gap_2()
                                    .child(Tag::primary().child("Primary"))
                                    .child(Tag::secondary().child("Secondary"))
                                    .child(Tag::success().child("Success"))
                                    .child(Tag::warning().child("Warning"))
                                    .child(Tag::danger().child("Danger"))
                                    .child(Tag::info().child("Info"))
                                    .child(Tag::new().outline().child("Outline")),
                            )
                            .child(
                                row()
                                    .gap_6()
                                    .child(
                                        Badge::new().count(8).child(
                                            Button::new("badge-bell")
                                                .icon(IconName::Bell)
                                                .outline()
                                                .tooltip("Eight notifications"),
                                        ),
                                    )
                                    .child(
                                        Badge::new().dot().child(
                                            Button::new("badge-dot")
                                                .icon(IconName::Inbox)
                                                .outline()
                                                .tooltip("One unread item"),
                                        ),
                                    )
                                    .child(Badge::new().count(120).max(99).child(
                                        Button::new("badge-inbox").icon(IconName::Bell).outline(),
                                    )),
                            )
                            .child(
                                row()
                                    .gap_2()
                                    .child(Avatar::new().name("Ada Lovelace"))
                                    .child(Avatar::new().name("Grace Hopper"))
                                    .child(Avatar::new().name("Linus Torvalds"))
                                    .child(Avatar::new().name("Margaret Hamilton"))
                                    .child(div().text_size(px(11.)).text_color(muted).child("+12")),
                            )
                            .child(div().h(px(1.)).bg(border))
                            .child(
                                DescriptionList::new()
                                    .columns(1)
                                    .item("Theme", theme_name, 1)
                                    .item("Mode", if is_light { "Light" } else { "Dark" }, 1)
                                    .item("Contrast", "WCAG AA enforced", 1)
                                    .item("Colour space", "OKLCH \u{2192} sRGB", 1),
                            ),
                    ),
            )
            .child(
                card(cx)
                    .child(card_header(
                        "Chart",
                        "A native bar chart that follows the theme's chart ramp.",
                        cx,
                    ))
                    .child(
                        div().h(px(190.)).w_full().child(
                            BarChart::new(vec![
                                ("Jan", 42.0f64),
                                ("Feb", 58.0),
                                ("Mar", 51.0),
                                ("Apr", 73.0),
                                ("May", 66.0),
                                ("Jun", 84.0),
                                ("Jul", 78.0),
                            ])
                            .id("widgets-bar-chart")
                            .band(|item| item.0)
                            .value(|item| item.1)
                            .value_axis(true)
                            .value_tick_count(4)
                            .grid(true)
                            .name("Token coverage"),
                        ),
                    ),
            )
    }

    // -- page: feedback ----------------------------------------------------

    fn feedback_page(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let muted = theme.muted_foreground;

        div()
            .flex()
            .flex_col()
            .gap_5()
            .child(
                card(cx)
                    .child(card_header(
                        "Alerts",
                        "Inline messages for information, success, warning and error.",
                        cx,
                    ))
                    .child(
                        Alert::info(
                            "alert-info",
                            "The selected theme was rebuilt from its seed colours.",
                        )
                        .title("Theme updated"),
                    )
                    .child(
                        Alert::success(
                            "alert-success",
                            "All text and surface pairs meet their contrast floor.",
                        )
                        .title("Contrast check passed"),
                    )
                    .child(
                        Alert::warning(
                            "alert-warning",
                            "This preset nudges the muted text colour slightly to stay readable.",
                        )
                        .title("Contrast floor adjusted"),
                    )
                    .child(
                        Alert::error(
                            "alert-error",
                            "The theme file could not be parsed. The previous theme is still active.",
                        )
                        .title("Import failed"),
                    ),
            )
            .child(
                card(cx)
                    .child(card_header(
                        "Dialogs, sheets & notifications",
                        "Overlays that sit above the window and dismiss cleanly.",
                        cx,
                    ))
                    .child(
                        row()
                            .flex_wrap()
                            .gap_2()
                            .child(
                                Button::new("open-dialog")
                                    .label("Open dialog")
                                    .primary()
                                    .on_click(cx.listener(|_, _, window, cx| {
                                        open_create_dialog(window, cx);
                                    })),
                            )
                            .child(
                                Button::new("open-alert")
                                    .label("Confirm delete")
                                    .danger()
                                    .on_click(cx.listener(|_, _, window, cx| {
                                        open_delete_alert(window, cx);
                                    })),
                            )
                            .child(
                                Button::new("open-sheet")
                                    .label("Open sheet")
                                    .outline()
                                    .on_click(cx.listener(|_, _, window, cx| {
                                        open_inspector(window, cx);
                                    })),
                            )
                            .child(
                                Button::new("toast-info")
                                    .label("Info toast")
                                    .on_click(cx.listener(|_, _, window, cx| {
                                        window.push_notification(
                                            Notification::info(
                                                "All 16 Lace presets are available.",
                                            )
                                            .title("Heads up"),
                                            cx,
                                        );
                                    })),
                            )
                            .child(
                                Button::new("toast-success")
                                    .label("Success toast")
                                    .on_click(cx.listener(|_, _, window, cx| {
                                        window.push_notification(
                                            Notification::success(
                                                "Tokens rebuilt and applied live.",
                                            )
                                            .title("Theme applied"),
                                            cx,
                                        );
                                    })),
                            )
                            .child(
                                Button::new("toast-error")
                                    .label("Error toast")
                                    .on_click(cx.listener(|_, _, window, cx| {
                                        window.push_notification(
                                            Notification::error(
                                                "A token could not be resolved.",
                                            )
                                            .title("Something went wrong"),
                                            cx,
                                        );
                                    })),
                            ),
                    )
                    .child(rule(cx))
                    .child(
                        row()
                            .flex_wrap()
                            .gap_2()
                            .child(
                                Button::new("tooltip-button")
                                    .label("Hover for tooltip")
                                    .outline()
                                    .tooltip("Tooltips inherit the Lace tooltip pair."),
                            )
                            .child(
                                Popover::new("popover-demo")
                                    .trigger(
                                        Button::new("popover-trigger")
                                            .label("Popover")
                                            .outline(),
                                    )
                                    .content(|_, _, cx| {
                                        let muted = cx.theme().muted_foreground;
                                        col()
                                            .gap_2()
                                            .p_3()
                                            .w(px(250.))
                                            .child(
                                                div()
                                                    .text_size(px(13.))
                                                    .child("Popover content"),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(11.))
                                                    .text_color(muted)
                                                    .child("Any element can live here, including inputs and menus."),
                                            )
                                    }),
                            )
                            .child(
                                Button::new("menu-trigger")
                                    .label("Dropdown menu")
                                    .outline()
                                    .dropdown_menu(|menu, _, _| {
                                        menu.item(
                                            PopupMenuItem::new("Duplicate")
                                                .icon(IconName::Copy),
                                        )
                                        .item(
                                            PopupMenuItem::new("Rename")
                                                .icon(IconName::FileText),
                                        )
                                        .separator()
                                        .item(
                                            PopupMenuItem::new("Delete")
                                                .icon(IconName::Delete),
                                        )
                                    }),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(muted)
                            .child("Dialogs are keyboard dismissible; notifications autohide unless they carry an action."),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap_5()
                    .items_start()
                    .child(
                        card(cx)
                            .flex_1()
                            .child(card_header(
                                "Loading",
                                "Skeletons and spinners for work in progress.",
                                cx,
                            ))
                            .child(
                                row()
                                    .gap_3()
                                    .child(Spinner::new())
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .text_color(muted)
                                            .child("Rebuilding tokens\u{2026}"),
                                    ),
                            )
                            .child(
                                col()
                                    .gap_2()
                                    .child(Skeleton::new().h(px(12.)).w_full())
                                    .child(Skeleton::new().h(px(12.)).w(px(260.)))
                                    .child(Skeleton::new().h(px(12.)).w(px(200.))),
                            ),
                    )
                    .child(
                        card(cx)
                            .flex_1()
                            .child(card_header(
                                "Empty state",
                                "The polite zero-result screen.",
                                cx,
                            ))
                            .child(
                                Empty::new()
                                    .header(
                                        EmptyHeader::new()
                                            .media(
                                                EmptyMedia::new()
                                                    .with_variant(EmptyMediaVariant::Icon)
                                                    .child(
                                                        Icon::new(IconName::Inbox)
                                                            .large()
                                                            .text_color(muted),
                                                    ),
                                            )
                                            .title(EmptyTitle::new().child("Nothing here yet"))
                                            .description(
                                                EmptyDescription::new().child(
                                                    "Create a project or import a theme to fill this space.",
                                                ),
                                            ),
                                    )
                                    .content(
                                        EmptyContent::new().child(
                                            Button::new("empty-create")
                                                .label("Create project")
                                                .primary()
                                                .icon(IconName::Plus)
                                                .on_click(cx.listener(|_, _, window, cx| {
                                                    open_create_dialog(window, cx);
                                                })),
                                        ),
                                    ),
                            ),
                    ),
            )
    }

    // -- page: navigation --------------------------------------------------

    fn navigation_page(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;

        div()
            .flex()
            .flex_col()
            .gap_5()
            .child(
                card(cx)
                    .child(card_header(
                        "Breadcrumbs & tabs",
                        "Location, then mutually exclusive views.",
                        cx,
                    ))
                    .child(
                        Breadcrumb::new()
                            .child(BreadcrumbItem::new("Workspace"))
                            .child(BreadcrumbItem::new("Components"))
                            .child(BreadcrumbItem::new("Navigation")),
                    )
                    .child(
                        TabBar::new("widget-tabs")
                            .pill()
                            .selected_index(self.tab_index)
                            .child(Tab::new().label("Overview"))
                            .child(Tab::new().label("Activity"))
                            .child(Tab::new().label("Settings"))
                            .child(Tab::new().label("Archived").disabled(true))
                            .on_click(cx.listener(|this, index, _, cx| {
                                this.tab_index = *index;
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(muted)
                            .child(format!(
                                "Pill tab {} of 3 selected.",
                                self.tab_index + 1
                            )),
                    )
                    .child(rule(cx))
                    .child(
                        TabBar::new("widget-segmented")
                            .segmented()
                            .selected_index(self.segmented_index)
                            .child(Tab::new().label("Day"))
                            .child(Tab::new().label("Week"))
                            .child(Tab::new().label("Month"))
                            .on_click(cx.listener(|this, index, _, cx| {
                                this.segmented_index = *index;
                                cx.notify();
                            })),
                    ),
            )
            .child(
                card(cx)
                    .child(card_header(
                        "Stepper",
                        "Progress through a sequence, with the current step selected.",
                        cx,
                    ))
                    .child(
                        Stepper::new("widget-stepper")
                            .selected_index(self.step)
                            .items([
                                StepperItem::new().child("Account"),
                                StepperItem::new().child("Profile"),
                                StepperItem::new().child("Theme"),
                                StepperItem::new().child("Review"),
                            ])
                            .on_click(cx.listener(|this, index, _, cx| {
                                this.step = *index;
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap_5()
                    .items_start()
                    .child(
                        card(cx)
                            .flex_1()
                            .child(card_header(
                                "Pagination",
                                "Page through a bounded result set.",
                                cx,
                            ))
                            .child(
                                Pagination::new("widget-pagination")
                                    .total_pages(8)
                                    .current_page(self.page_number)
                                    .on_click(cx.listener(|this, page, _, cx| {
                                        this.page_number = *page;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(muted)
                                    .child(format!("Page {} of 8", self.page_number)),
                            ),
                    )
                    .child(
                        card(cx)
                            .flex_1()
                            .child(card_header(
                                "Links & shortcuts",
                                "Text links and keyboard affordances.",
                                cx,
                            ))
                            .child(
                                row()
                                    .gap_4()
                                    .child(
                                        Link::new("nav-docs")
                                            .href("https://gpui-kit.com")
                                            .child("Component docs"),
                                    )
                                    .child(
                                        Link::new("nav-lace")
                                            .href("https://github.com/opticsWolf/Lace")
                                            .child("Lace"),
                                    ),
                            )
                            .child(rule(cx))
                            .child(
                                col()
                                    .gap_2()
                                    .child(
                                        row()
                                            .justify_between()
                                            .child(div().text_size(px(12.)).child("Search components"))
                                            .child(Kbd::new(
                                                Keystroke::parse("ctrl-k").unwrap_or_default(),
                                            )),
                                    )
                                    .child(
                                        row()
                                            .justify_between()
                                            .child(div().text_size(px(12.)).child("Next theme"))
                                            .child(Kbd::new(
                                                Keystroke::parse("ctrl-shift-t")
                                                    .unwrap_or_default(),
                                            )),
                                    )
                                    .child(
                                        row()
                                            .justify_between()
                                            .child(div().text_size(px(12.)).child("Command palette"))
                                            .child(Kbd::new(
                                                Keystroke::parse("ctrl-shift-p")
                                                    .unwrap_or_default(),
                                            )),
                                    ),
                            ),
                    ),
            )
            .child(
                card(cx)
                    .child(card_header(
                        "Accordion",
                        "Progressive disclosure for secondary detail.",
                        cx,
                    ))
                    .child(
                        Accordion::new("widget-accordion")
                            .bordered(true)
                            .item(|item| {
                                item.title("What is Lace?")
                                    .child("Lace derives a full dock theme from three to five OKLCH seed colours, then lifts every token that misses a contrast floor.")
                            })
                            .item(|item| {
                                item.title("How does the bridge work?")
                                    .child("lace_gpui::skin maps the engine's Tokens onto gpui-kit's ThemeColor, its radii, its scrollbar mode and its panel chrome.")
                            })
                            .item(|item| {
                                item.title("Can I ship my own palette?")
                                    .open(true)
                                    .child("Yes. Build a ThemeSpec, call build, then apply_spec. The curated presets are only a starting point.")
                            }),
                    ),
            )
            .child(
                col()
                    .gap_4()
                    .child(Separator::horizontal())
                    .child(
                        div()
                            .flex()
                            .justify_center()
                            .text_size(px(11.))
                            .text_color(muted)
                            .child("end of gallery"),
                    ),
            )
    }
}

impl Render for WidgetsApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let background = theme.background;
        let foreground = theme.foreground;

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(background)
            .text_color(foreground)
            .font_family(".SystemUIFont")
            .child(self.header(cx))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .child(self.sidebar(cx))
                    .child(self.content(cx)),
            )
            .child(self.status_bar(cx))
    }
}

fn main() {
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(|cx: &mut App| {
            gpui_kit::init(cx);
            // Lace's darkest preset; the header arrows cycle the rest.
            skin::apply("midnight", cx);

            // Fit the gallery to the display, even on small or scaled screens.
            let display = cx
                .primary_display()
                .map(|display| display.visible_bounds())
                .unwrap_or_else(|| Bounds::new(point(px(0.), px(0.)), size(px(1360.), px(880.))));
            let width = px(1360.).min(display.size.width - px(64.)).max(px(900.));
            let height = px(880.).min(display.size.height - px(64.)).max(px(560.));
            let bounds = Bounds::new(
                point(
                    display.origin.x + (display.size.width - width) * 0.5,
                    display.origin.y + (display.size.height - height) * 0.5,
                ),
                size(width, height),
            );
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("Lace Widgets".into()),
                    ..Default::default()
                }),
                ..Default::default()
            };
            gpui_kit::open_window(options, cx, |window, cx| {
                cx.new(|cx| WidgetsApp::new(window, cx))
            })
            .expect("failed to open the widgets window");
        });
}
