//! The "Kit" page: gpui-kit's stock components (gpui-component) as they
//! render under the Lace skin, and, switched to LaceStyle, the same groups
//! built from this crate's widgets, so the two can be compared directly.

use gpui_kit::component::accordion::Accordion;
use gpui_kit::component::alert::Alert;
use gpui_kit::component::avatar::Avatar;
use gpui_kit::component::badge::Badge;
use gpui_kit::component::breadcrumb::Breadcrumb;
use gpui_kit::component::button::{Button, ButtonGroup, ButtonVariants};
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::group_box::{GroupBox as KitGroupBox, GroupBoxVariants};
use gpui_kit::component::input::{Input, InputState, NumberInput, Textarea, TextareaState};
use gpui_kit::component::label::Label;
use gpui_kit::component::link::Link;
use gpui_kit::component::list::ListItem;
use gpui_kit::component::pagination::Pagination;
use gpui_kit::component::progress::{Progress, ProgressCircle};
use gpui_kit::component::radio::{Radio, RadioGroup};
use gpui_kit::component::rating::Rating;
use gpui_kit::component::select::{Select, SelectState};
use gpui_kit::component::separator::Separator;
use gpui_kit::component::skeleton::Skeleton;
use gpui_kit::component::slider::{Slider, SliderState};
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::stepper::{Stepper, StepperItem};
use gpui_kit::component::switch::Switch;
use gpui_kit::component::tab::TabBar as KitTabBar;
use gpui_kit::component::tag::Tag;
use gpui_kit::component::tree::tree;
use gpui_kit::component::{Disableable, IconName, IndexPath, Sizable};
use gpui_kit::component::tree::{TreeItem, TreeState};
use gpui_kit::gpui::{div, prelude::*, px, AnyElement, App, Context, Entity, SharedString, WeakEntity, Window};
use lace_widgets::controls as lw;
use lace_widgets::controls::{CheckState, Tone};
use lace_widgets::glyph;
use lace_widgets::style::{hsla, ActiveStyle};
use lace_widgets::views::{ItemModel, ItemView, Mode};

/// The page's own state: what the stock components bind to.
pub struct Kit {
    /// Show the LaceStyle widgets instead of the stock ones.
    lace: bool,
    l_slider: f64,
    l_number: f64,
    l_select: usize,
    l_segment: usize,
    l_tool: usize,
    l_step: usize,
    l_tree: Entity<ItemView>,
    text: Entity<InputState>,
    password: Entity<InputState>,
    number: Entity<InputState>,
    area: Entity<TextareaState>,
    slider: Entity<SliderState>,
    select: Entity<SelectState<Vec<SharedString>>>,
    tree: Entity<TreeState>,
    check: bool,
    switch: bool,
    radio: usize,
    rating: usize,
    page: usize,
    tab: usize,
    open: Vec<usize>,
}

impl Kit {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let items: Vec<SharedString> = ["Apple", "Banana", "Cherry", "Damson", "Elderberry"].into_iter().map(Into::into).collect();
        let tree_items = vec![
            TreeItem::new("src", "src")
                .expanded(true)
                .child(TreeItem::new("src/main.rs", "main.rs"))
                .child(TreeItem::new("src/lib.rs", "lib.rs"))
                .child(TreeItem::new("src/ui", "ui").child(TreeItem::new("src/ui/view.rs", "view.rs"))),
            TreeItem::new("docs", "docs").child(TreeItem::new("docs/readme.md", "readme.md")),
            TreeItem::new("Cargo.toml", "Cargo.toml"),
        ];
        let l_model = cx.new(|_| {
            let mut m = ItemModel::new(["Name"]);
            let src = m.add_row(None, vec!["src".into()]);
            for f in ["main.rs", "lib.rs"] {
                m.add_row(Some(src), vec![f.into()]);
            }
            let ui = m.add_row(Some(src), vec!["ui".into()]);
            m.add_row(Some(ui), vec!["view.rs".into()]);
            let docs = m.add_row(None, vec!["docs".into()]);
            m.add_row(Some(docs), vec!["readme.md".into()]);
            m.add_row(None, vec!["Cargo.toml".into()]);
            m.set_expanded(src, true);
            m
        });
        Self {
            lace: true,
            l_slider: 40.0,
            l_number: 42.0,
            l_select: 1,
            l_segment: 1,
            l_tool: 0,
            l_step: 1,
            l_tree: cx.new(|cx| ItemView::new(&l_model, Mode::Tree, cx).with_selected(&[1], cx)),
            text: cx.new(|cx| InputState::new(window, cx).placeholder("Input")),
            password: cx.new(|cx| InputState::new(window, cx).masked(true).default_value("secret")),
            number: cx.new(|cx| InputState::new(window, cx).default_value("42")),
            area: cx.new(|cx| TextareaState::new(window, cx).default_value("Multi-line input\nsecond line")),
            slider: cx.new(|_| SliderState::new().min(0.).max(100.).step(1.).default_value(40.)),
            select: cx.new(|cx| SelectState::new(items, Some(IndexPath::new(1)), window, cx)),
            tree: cx.new(|cx| TreeState::new(cx).items(tree_items)),
            check: true,
            switch: true,
            radio: 1,
            rating: 3,
            page: 2,
            tab: 0,
            open: vec![0],
        }
    }
}

/// A titled card for one group of components.
fn card(title: &'static str, body: impl IntoElement) -> AnyElement {
    KitGroupBox::new().outline().title(title).child(div().flex().flex_col().gap(px(10.)).child(body)).into_any_element()
}

fn row() -> gpui_kit::gpui::Div {
    div().flex().flex_wrap().items_center().gap(px(8.))
}

/// A widget callback that updates the page and repaints it.
fn upd<T: 'static>(w: &WeakEntity<Kit>, f: impl Fn(&mut Kit, T) + 'static) -> impl Fn(T, &mut Window, &mut App) + 'static {
    let w = w.clone();
    move |v, _, cx| {
        let _ = w.update(cx, |k, cx| {
            f(k, v);
            cx.notify();
        });
    }
}

/// A LaceStyle group box card.
fn lace_card(title: &'static str, body: impl IntoElement) -> AnyElement {
    lw::GroupBox::new(title).child(div().flex().flex_col().gap(px(10.)).child(body)).into_any_element()
}

impl Render for Kit {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let w = cx.entity().downgrade();
        let (cards, note) = if self.lace {
            (self.lace_cards(window, cx), "The same groups built from LaceStyle widgets: Qt's families plus switches, tags, alerts, rings and steppers in the Lace look.")
        } else {
            (self.stock_cards(cx), "gpui-kit / gpui-component stock components, unstyled by LaceStyle: they follow the skin's theme colours only.")
        };
        div()
            .p(px(12.))
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(
                row()
                    .child(
                        lw::Segmented::new("kit-mode", ["Default", "LaceStyle"])
                            .selected(Some(self.lace as usize))
                            .on_select(upd(&w, |k, i| k.lace = i == 1)),
                    )
                    .child(div().text_size(px(12.)).child(note)),
            )
            .child(div().grid().grid_cols(2).gap(px(10.)).children(cards))
    }
}

impl Kit {
    /// The eight groups in LaceStyle widgets.
    fn lace_cards(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let w = cx.entity().downgrade();
        let style = cx.lace().clone();
        let ink = hsla(style.active.button_text);
        let plus = move || {
            glyph::glyph_box(12.0, 12.0, move |b, window, _| glyph::paint_plus_minus(window, b, true, ink)).into_any_element()
        };

        let buttons = div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(
                row()
                    .child(lw::PushButton::new("l-default", "Default"))
                    .child(lw::PushButton::new("l-primary", "Primary").default_button())
                    .child(lw::PushButton::new("l-flat", "Flat").flat())
                    .child(lw::PushButton::new("l-danger", "Danger").tone(Tone::Danger))
                    .child(lw::PushButton::new("l-warning", "Warning").tone(Tone::Warning))
                    .child(lw::PushButton::new("l-success", "Success").tone(Tone::Success))
                    .child(lw::PushButton::new("l-info", "Info").tone(Tone::Info)),
            )
            .child(
                row()
                    .child(lw::PushButton::new("l-icon", "With icon").icon(move |_| plus()))
                    .child(lw::ToolButton::new("l-tool", "Tool").auto_raise())
                    .child(lw::ToolButton::new("l-tool-on", "Checked").checked(true))
                    .child(lw::PushButton::new("l-disabled", "Disabled").disabled(true)),
            )
            .child(
                row().child(
                    lw::Segmented::new("l-group", ["Left", "Middle", "Right"])
                        .selected(Some(self.l_segment))
                        .on_select(upd(&w, |k, i| k.l_segment = i)),
                ),
            );

        let toggles = div()
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(
                row()
                    .child(
                        lw::CheckBox::new("l-check")
                            .label("Checkbox")
                            .checked(self.check)
                            .on_change(upd(&w, |k, v: CheckState| k.check = v == CheckState::Checked)),
                    )
                    .child(lw::CheckBox::new("l-check-off").label("Disabled").disabled(true))
                    .child(lw::Switch::new("l-switch").label("Switch").checked(self.switch).on_change(upd(&w, |k, v| k.switch = v)))
                    .child(lw::Switch::new("l-switch-off").label("Disabled").disabled(true)),
            )
            .child(row().children(["One", "Two", "Three"].into_iter().enumerate().map(|(i, label)| {
                lw::RadioButton::new(SharedString::from(format!("l-r{i}")))
                    .label(label)
                    .checked(self.radio == i)
                    .on_select({
                        let f = upd(&w, move |k, ()| k.radio = i);
                        move |window, cx| f((), window, cx)
                    })
                    .into_any_element()
            })))
            .child(row().child(lw::Rating::new("l-rating", self.rating).on_change(upd(&w, |k, v| k.rating = v))));

        let inputs = div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(lw::LineEdit::new("l-text", &self.text))
            .child(lw::LineEdit::new("l-password", &self.password))
            .child(lw::SpinBox::new("l-number", self.l_number).range(0., 100.).on_change(upd(&w, |k, v| k.l_number = v)))
            .child(
                lw::ComboBox::new("l-select", ["Apple", "Banana", "Cherry", "Damson", "Elderberry"])
                    .selected(Some(self.l_select))
                    .on_select(upd(&w, |k, i| k.l_select = i)),
            )
            .child(div().h(px(70.)).child(lw::TextEdit::new("l-area", &self.area)));

        let ranges = div()
            .flex()
            .flex_col()
            .gap(px(12.))
            .child(lw::Slider::new("l-slider", self.l_slider).on_change(upd(&w, |k, v| k.l_slider = v)))
            .child(lw::ProgressBar::new(self.l_slider))
            .child(lw::ProgressBar::new(0.0).busy())
            .child(
                row()
                    .child(lw::ProgressRing::new(self.l_slider / 100.0))
                    .child(lw::ProgressRing::new(0.62).size(28.0).no_label())
                    .child(lw::Spinner::default()),
            );

        let display = div()
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(
                row()
                    .child(lw::Tag::new("Accent"))
                    .child(lw::Tag::new("Neutral").tone(Tone::Neutral))
                    .child(lw::Tag::new("Success").tone(Tone::Success))
                    .child(lw::Tag::new("Warning").tone(Tone::Warning))
                    .child(lw::Tag::new("Danger").tone(Tone::Danger))
                    .child(lw::Tag::new("Outline").tone(Tone::Info).outline())
                    .child(lw::Tag::new("Pill").pill()),
            )
            .child(
                row()
                    .gap(px(14.))
                    .child(lw::Badge::count(7).on(lw::Avatar::new("Lace Style")))
                    .child(lw::Badge::dot().tone(Tone::Success).on(lw::Avatar::new("Gpui Kit").tone(Tone::Info)))
                    .child(lw::Badge::count(120).on(lw::Avatar::new("Weave").size(24.0).tone(Tone::Neutral)))
                    .child(lw::Link::new("l-link", "Link")),
            )
            .child(lw::Separator::horizontal().label("Separator"))
            .child(lw::Breadcrumb::new("l-crumbs", ["Home", "Projects", "Lace"]))
            .child(
                row()
                    .child(lw::Skeleton::circle(28.0))
                    .child(div().flex().flex_col().gap(px(6.)).child(lw::Skeleton::block(220.0, 12.0)).child(lw::Skeleton::block(160.0, 12.0))),
            );

        let feedback = div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(lw::Alert::new(Tone::Info, "An informational alert.").title("Info"))
            .child(lw::Alert::new(Tone::Success, "Saved."))
            .child(lw::Alert::new(Tone::Warning, "Check the settings."))
            .child(lw::Alert::new(Tone::Danger, "Something failed."));

        let navigation = div()
            .flex()
            .flex_col()
            .gap(px(12.))
            .child(
                lw::TabBar::new("l-tabs", ["General", "Appearance", "Advanced"])
                    .current(self.tab)
                    .on_select(upd(&w, |k, i| k.tab = i)),
            )
            .child(
                lw::Segmented::new("l-tabs-seg", ["Day", "Week", "Month"])
                    .selected(Some(self.tab))
                    .on_select(upd(&w, |k, i| k.tab = i)),
            )
            .child(lw::Pagination::new("l-pages", self.page, 12).on_select(upd(&w, |k, p| k.page = p)))
            .child(
                lw::Stepper::new("l-stepper", ["Plan", "Build", "Test", "Ship"])
                    .current(self.l_step)
                    .on_select(upd(&w, |k, i| k.l_step = i)),
            );

        let accordion = lw::ToolBox::new("l-toolbox")
            .page("What is this page?", div().child("The kit groups, drawn by LaceStyle."))
            .page("Why?", div().child("To compare them with the stock components."))
            .page("Tree", div().h(px(150.)).child(self.l_tree.clone()))
            .current(self.l_tool)
            .on_select(upd(&w, |k, i| k.l_tool = i));

        vec![
            lace_card("Buttons", buttons),
            lace_card("Check box, switch, radio, rating", toggles),
            lace_card("Line edit, spin box, combo box, text edit", inputs),
            lace_card("Slider, progress, ring, spinner", ranges),
            lace_card("Tags, badges, avatars, link, skeleton", display),
            lace_card("Alerts", feedback),
            lace_card("Tabs, segmented, pagination, stepper", navigation),
            lace_card("Tool box (accordion) and tree", div().h(px(260.)).child(accordion)),
        ]
    }

    /// The eight groups in gpui-kit's stock components.
    fn stock_cards(&mut self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let buttons = div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(
                row()
                    .child(Button::new("k-default").label("Default"))
                    .child(Button::new("k-primary").primary().label("Primary"))
                    .child(Button::new("k-secondary").secondary().label("Secondary"))
                    .child(Button::new("k-danger").danger().label("Danger"))
                    .child(Button::new("k-warning").warning().label("Warning"))
                    .child(Button::new("k-success").success().label("Success"))
                    .child(Button::new("k-info").info().label("Info")),
            )
            .child(
                row()
                    .child(Button::new("k-ghost").ghost().label("Ghost"))
                    .child(Button::new("k-link").link().label("Link"))
                    .child(Button::new("k-outline").outline().label("Outline"))
                    .child(Button::new("k-icon").icon(IconName::Plus).label("With icon"))
                    .child(Button::new("k-iconly").icon(IconName::Search).tooltip("Icon only"))
                    .child(Button::new("k-loading").loading(true).label("Loading"))
                    .child(Button::new("k-disabled").label("Disabled").disabled(true)),
            )
            .child(
                row()
                    .child(Button::new("k-small").small().label("Small"))
                    .child(Button::new("k-large").large().label("Large"))
                    .child(
                        ButtonGroup::new("k-group")
                            .outline()
                            .child(Button::new("k-g1").label("Left"))
                            .child(Button::new("k-g2").label("Middle"))
                            .child(Button::new("k-g3").label("Right")),
                    ),
            );

        let toggles = div()
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(
                row()
                    .child(Checkbox::new("k-check").label("Checkbox").checked(self.check).on_click(cx.listener(|this, v: &bool, _, cx| {
                        this.check = *v;
                        cx.notify();
                    })))
                    .child(Checkbox::new("k-check-off").label("Disabled").disabled(true))
                    .child(Switch::new("k-switch").label("Switch").checked(self.switch).on_click(cx.listener(|this, v: &bool, _, cx| {
                        this.switch = *v;
                        cx.notify();
                    }))),
            )
            .child(
                RadioGroup::horizontal("k-radio")
                    .selected_index(Some(self.radio))
                    .on_click(cx.listener(|this, i: &usize, _, cx| {
                        this.radio = *i;
                        cx.notify();
                    }))
                    .child(Radio::new("k-r0").label("One"))
                    .child(Radio::new("k-r1").label("Two"))
                    .child(Radio::new("k-r2").label("Three")),
            )
            .child(row().child(Rating::new("k-rating").value(self.rating).on_click(cx.listener(|this, v: &usize, _, cx| {
                this.rating = *v;
                cx.notify();
            }))));

        let inputs = div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(Input::new(&self.text).cleanable(true))
            .child(Input::new(&self.password).mask_toggle())
            .child(NumberInput::new(&self.number))
            .child(Select::new(&self.select).placeholder("Select a fruit"))
            .child(div().h(px(70.)).child(Textarea::new(&self.area).h_full()));

        let ranges = div()
            .flex()
            .flex_col()
            .gap(px(12.))
            .child(Slider::new(&self.slider))
            .child(Progress::new("k-progress").value(62.))
            .child(
                row()
                    .child(ProgressCircle::new("k-circle").value(62.))
                    .child(ProgressCircle::new("k-busy").loading(true))
                    .child(Spinner::new()),
            );

        let display = div()
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(
                row()
                    .child(Tag::primary().child("Primary"))
                    .child(Tag::secondary().child("Secondary"))
                    .child(Tag::success().child("Success"))
                    .child(Tag::warning().child("Warning"))
                    .child(Tag::danger().child("Danger"))
                    .child(Tag::info().outline().child("Outline")),
            )
            .child(
                row()
                    .child(Badge::new().count(7).child(Avatar::new().name("Lace Style")))
                    .child(Badge::new().dot().child(Avatar::new().name("Gpui Kit")))
                    .child(Label::new("Label").secondary("secondary text"))
                    .child(Link::new("k-link2").href("https://example.com").child("Link")),
            )
            .child(Separator::horizontal().label("Separator"))
            .child(Breadcrumb::new().child("Home").child("Projects").child("Lace"))
            .child(div().flex().flex_col().gap(px(6.)).child(Skeleton::new().w(px(220.)).h(px(12.))).child(Skeleton::new().secondary().w(px(160.)).h(px(12.))));

        let feedback = div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(Alert::info("k-a1", "An informational alert.").title("Info"))
            .child(Alert::success("k-a2", "Saved."))
            .child(Alert::warning("k-a3", "Check the settings."))
            .child(Alert::error("k-a4", "Something failed."));

        let navigation = div()
            .flex()
            .flex_col()
            .gap(px(12.))
            .child(
                KitTabBar::new("k-tabs")
                    .selected_index(self.tab)
                    .on_click(cx.listener(|this, i: &usize, _, cx| {
                        this.tab = *i;
                        cx.notify();
                    }))
                    .child("General")
                    .child("Appearance")
                    .child("Advanced"),
            )
            .child(
                KitTabBar::new("k-tabs-seg")
                    .segmented()
                    .selected_index(self.tab)
                    .on_click(cx.listener(|this, i: &usize, _, cx| {
                        this.tab = *i;
                        cx.notify();
                    }))
                    .child("Day")
                    .child("Week")
                    .child("Month"),
            )
            .child(Pagination::new("k-pages").current_page(self.page).total_pages(12).on_click(cx.listener(|this, p: &usize, _, cx| {
                this.page = *p;
                cx.notify();
            })))
            .child(
                Stepper::new("k-stepper")
                    .selected_index(1)
                    .item(StepperItem::new().child("Plan"))
                    .item(StepperItem::new().child("Build"))
                    .item(StepperItem::new().child("Ship")),
            );

        let open = self.open.clone();
        let accordion = Accordion::new("k-accordion")
            .bordered(true)
            .on_toggle_click(cx.listener(|this, open: &[usize], _, cx| {
                this.open = open.to_vec();
                cx.notify();
            }))
            .item(|i| i.title("What is this page?").open(open.contains(&0)).child("gpui-kit's stock components under the Lace skin."))
            .item(|i| i.title("Why?").open(open.contains(&1)).child("To see which ones still need LaceStyle work."))
            .item(|i| i.title("Disabled item").disabled(true));

        let kit_tree = div().h(px(170.)).child(tree(&self.tree, |ix, entry, selected, _, _| {
            let depth = entry.depth();
            let folder = entry.is_folder();
            let open = entry.is_expanded();
            ListItem::new(ix).selected(selected).pl(px(8. + 16. * depth as f32)).child(
                div()
                    .flex()
                    .gap(px(6.))
                    .child(if folder { if open { "▾" } else { "▸" } } else { " " })
                    .child(entry.item().label.clone()),
            )
        }));

        vec![
            card("Buttons", buttons),
            card("Check box, switch, radio, rating", toggles),
            card("Inputs, number input, select", inputs),
            card("Slider, progress, spinner", ranges),
            card("Tags, badges, avatars, labels, skeleton", display),
            card("Alerts", feedback),
            card("Tabs, pagination, stepper", navigation),
            card("Accordion and tree", div().flex().flex_col().gap(px(10.)).child(accordion).child(kit_tree)),
        ]
    }
}
