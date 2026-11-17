//! The "Kit" page: gpui-kit's stock components (gpui-component) as they
//! render under the Lace skin, untouched, so it is easy to see which ones
//! still need a LaceStyle widget or more theme tokens.

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
use gpui_kit::gpui::{div, prelude::*, px, AnyElement, App, Context, Entity, SharedString, Window};

/// The page's own state: what the stock components bind to.
pub struct Kit {
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
        Self {
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

impl Render for Kit {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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

        div()
            .p(px(12.))
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(
                div()
                    .text_size(px(12.))
                    .child("gpui-kit / gpui-component stock components, unstyled by LaceStyle: they follow the skin's theme colours only."),
            )
            .child(div().grid().grid_cols(2).gap(px(10.)).children([
                card("Buttons", buttons),
                card("Check box, switch, radio, rating", toggles),
                card("Inputs, number input, select", inputs),
                card("Slider, progress, spinner", ranges),
                card("Tags, badges, avatars, labels, skeleton", display),
                card("Alerts", feedback),
                card("Tabs, pagination, stepper", navigation),
                card("Accordion and tree", div().flex().flex_col().gap(px(10.)).child(accordion).child(kit_tree)),
            ]))
    }
}
