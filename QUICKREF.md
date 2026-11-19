# lace-gpui quick reference

A cheat sheet for the API. [README.md](README.md) explains what each piece is
for, and [ARCHITECTURE.md](ARCHITECTURE.md) explains how the pieces connect.

## Setup

```toml
lace-gpui = { version = "0.0.3", features = ["gpui-kit"] }
```

```rust
gpui_kit::init(cx);
lace_gpui::skin::apply("midnight", cx);
```

If the app also uses `lace-widgets`, add this:

```rust
lace_widgets::install_preset("midnight", cx);
```

## Themes (no gpui needed)

| Call | Result |
|---|---|
| `lace_gpui::themes::ALL` | `[&ThemeSpec; 37]`, dark to light within each group |
| `lace_gpui::themes::GROUPS` | `[(group name, &[&ThemeSpec])]` for menus |
| `lace_gpui::by_name("Tokyo Night")` | `Option<&'static ThemeSpec>`; ignores case, and `-` or a space counts as `_` |
| `lace_gpui::build(spec)` | `Tokens`, already enforced |
| `lace_gpui::spec_from_hex(..)` | Builds a `ThemeSpec` from hex seeds |
| `lace_gpui::audit(&tokens)` / `enforce(..)` | The contrast rulebook |
| `color::contrast_ratio(a, b)` | The WCAG ratio |
| `color::mix(a, b, t)` | Mixes two colours in OKLab |

Theme names:

| Group | Names |
|---|---|
| Basics | `midnight` `dark` `mocha` `slate` `caramel` `neutral` `cream` `light` |
| Basics Neo | the Basics names with a `_neo` suffix |
| Editor Classics | `dracula` `monokai` `nordic` `catppuccin` `tokyo_night` `solarized_dark` `solarized_light` |
| Neon | `cyberpunk_neon` `neon_dusk` |
| Edge Treatments | `cyberpunk_edge` `violet_haze` `midnight_haze`, each also with `_neutral` and `_light`; `slate_amber_dark` `slate_amber` `slate_amber_light` |

## Applying a theme (`skin`)

| Call | What it does |
|---|---|
| `skin::apply(name, cx) -> bool` | Applies the palette and the dock styles |
| `skin::apply_base(name, cx)` | Also applies LaceStyle's flat widget layer (`WidgetStyle::LaceBase`) |
| `skin::apply_styled(name, WidgetStyle, cx)` | Applies the theme with the widget layer you choose |
| `skin::apply_spec(&spec, cx)` / `apply_spec_styled` | Applies a `ThemeSpec` you own |
| `skin::apply_next(current, cx) -> &str` | Applies the next theme in `ALL` and returns its name |
| `skin::apply_next_styled(current, style, cx)` | `apply_next`, keeping the widget layer |
| `skin::apply_tokens(&mut theme, &t)` | Writes the theme only; installs no globals |
| `skin::tab_edges(&t)` | Derives a `TabEdgeStyle` |
| `skin::sidebar_style(&t)` | Derives a `SidebarStyle` |
| `skin::drop_overlay_style(&t)` | Derives a `DropOverlayStyle` |
| `skin::dock_chrome(&spec)` | Derives gpui-kit's `PanelStyle` |
| `skin::theme_set_json()` | Experimental: exports the presets as Lace-flavoured JSON |
| `skin::hsla(rgba)` | Converts a `Tokens` colour to an `Hsla` |

Every `apply*` call installs the three dock globals and refreshes the windows.
To override one after the theme is applied, call
`TabEdgeStyle::install(..)`, `SidebarStyle::install(..)` or
`DropOverlayStyle::install(..)`. To read the current value, call
`::current(cx)` on the same type.

## The dock (`geometry`)

```rust
let (area, skin) = GeometryDockSkin::dock_area("id", None, style, window, cx);
```

`DockGeometryStyle::new()` takes these builder methods:

| Method | Sets |
|---|---|
| `.gap(px)` | Split seams and the channels between docks |
| `.gap_background(hsla)` | The gap colour (default: the theme background) |
| `.gap_outer(px)` | The inset from the window frame |
| `.panel_radius(px)` | The pane radius; use `cx.theme().radius_lg` for the theme's own |
| `.outline(OutlineStyle::themed(px))` or `OutlineStyle::new(color, px)` | The pane outline |
| `.shadow(ShadowSpec::new(color, offset, blur).spread(px))` | A drop shadow |
| `.glow(GlowSpec::new(color, blur).spread(px))` | A glow |
| `.splitter(\|s\| s.thickness(px).length(SplitterLength::Percent(f) \| pixels(px) \| ..).cap(SplitterCap::Rounded).colors(SplitterColors::default().idle(..).hovered(..).pressed(..).dragging(..)))` | The splitter |

`DockGeometryStyle::default()` gives the stock gpui-kit look.

Methods on the skin:

| Call | What it does |
|---|---|
| `skin.set_style(style, cx)` / `skin.style()` | Restyles live / reads the style |
| `skin.set_panel_effect(panel_id, PanelEffect, cx)` | Gives one panel its own effect |
| `skin.clear_panel_effect(panel_id, cx)` | Removes that effect |
| `skin.set_close_button_visible(true, cx)` | Shows the close box on dock tabs |
| `skin.set_pin_target(..)` | Wires the pin button. `Sidebars::new` calls this for you |

## Sidebars (`sidebar`)

```rust
let sidebars = cx.new(|cx| {
    let mut s = Sidebars::new(area.clone(), &skin, cx);
    s.pinnable(&panel_entity, Some(icon)); // once for each panel that may be pinned
    s
});
// Render `sidebars`, not `area`.
```

| Call | What it does |
|---|---|
| `s.pin(panel_id, at: Option<Point>, window, cx)` | Moves a panel into the sidebar for its dock. A centre panel goes to the sidebar nearest `at` |
| `s.unpin(panel_id, window, cx)` | Puts the panel back into the group it came from |
| `s.open(panel_id, peek, window, cx)` | Slides the panel out. With `peek = true` it hides when the pointer leaves |
| `s.close_overlay(cx)` | Slides the open panel back in |
| `s.is_pinned(panel_id)` | Whether the panel is in a sidebar |
| `s.dock_at(..)` | Docks a pinned panel at a drop target. Dragging a sidebar tab calls this |
| `s.set_focus_behavior(SidebarFocus, cx)` / `s.focus_behavior()` | Sets / reads the focus mode |

`SidebarFocus` modes. `SidebarFocus::ALL` and `.label()` are there for pickers.

| Variant | Label | When a panel opens | When it closes |
|---|---|---|---|
| `TakeAndRestore` (default) | Take Focus & Restore | Focuses the panel | Restores the previous focus, unless the user moved it |
| `TakeOnly` | Take Focus Only | Focuses the panel | Leaves focus alone |
| `NoTransfer` | No Focus Transfer | Leaves focus alone; a click focuses the panel | Leaves focus alone |

Mouse and keyboard:

| Input | Effect |
|---|---|
| Hover a sidebar tab | Peeks the panel |
| Click a sidebar tab | Opens it, or closes it if it is open |
| Escape, or a click outside the panel | Closes it |
| Pin button in the title bar | Pins or unpins the panel |
| Drag a sidebar tab onto the dock | Docks the panel there |

## Tab strip, floating panes, icons

| Item | Notes |
|---|---|
| `edge_tabs::TabEdgeStyle` | Fields: `rule`, `outline`, `indicator` (`TabIndicator::{None, Top, Bottom}`), `indicator_width`, `radius`, `height`, `dimming`, `below_title`, `frame_*`, `panel_bg`. `draws_tabs()` is true when Lace's strip replaces the stock one |
| Tab metrics | lead 8, 4 before the close box, close box 24, tail 5, 3 between tabs (private `TAB_*` consts in `edge_tabs.rs`) |
| `floating::FloatingPane::new(panel, size)` | The drag preview, drawn at `FLOAT_OPACITY` (0.88) |
| `floating::drop_overlay(indicator, cx)` | The five-target cross, styled by `DropOverlayStyle` |
| `dock_icons::DockIcon::*.icon()` | Lace's tabler icons |
| `dock_icons::tab_label(icon, label)` | A tab label with an icon |

## Widgets

| Item | Notes |
|---|---|
| `button::GlowButton` | `Button::new(..).primary().glow("id", disabled, window, cx)` adds hover/press halos. `.glow_hover(..)` adds only the hover halo, with no wrapper |
| `base_style::BaseStyle` | `from_tokens(&t)` / `from_spec(&spec, &t)`, `.with_depth_scale(f)`, `.apply(&mut theme, &t)` |
| `lace_widgets::controls` | `PushButton`, checks and toggles, fields, ranges, containers, nav, status and chrome. Each widget paints from the installed `LaceStyle` |
| `lace_widgets::controls::CollapsibleGroupBox` | `CollapsibleGroupBox::new("id", "Title").default_open(false).child(..)`: a group box that folds away from its title. A click, Space or Enter toggles it. `.open(bool)` + `.on_toggle(\|open, w, cx\| ..)` make it controlled |
| `lace_widgets::controls::DynamicSlider` | `DynamicSlider::new("id", v).range(DynamicRange::new().percent(0.5).min_span(1.).limits(0., 1e3).step(0.).decimals(1)).show_limits(true).on_change(..)`: a slider whose limits follow the value (±50 % by default). The limits hold during a drag and re-centre on release |
| `ProgressBar::fill(rgba)` | Paints the fill in your colour instead of the style's (enabled bars only) |
| `Slider::on_drag(\|pressed, w, cx\| ..)` / `.selected(bool)` | Drag start/end hook; a fill that stays visible on a selected row |
| `lace_widgets::style::install(style, cx)` | Swaps the widget style live |

## Item-view delegates (`lace_widgets::views::Delegate`)

`ItemView::new(&model, Mode::Table, cx).column(ix, Column::fixed(w).delegate(..))`

| Delegate | Cell value | Shows / edits |
|---|---|---|
| `Text` (default) / `ReadOnly` | any | Text; a line edit on double-click or F2 / never edited |
| `number(min, max, step, decimals)` | number | A spin box editor |
| `choice([..])` | text | A combo box editor |
| `Check` | bool | A check box, toggled in place |
| `Switch` | bool | A switch, toggled in place |
| `toggle("label")` | bool | A checkable push button, toggled in place |
| `Radio` | bool | A radio button; choosing a row clears the column in its sibling rows |
| `slider(min, max, step, decimals)` | number | A slider dragged in place, with the value beside it |
| `dynamic_slider(DynamicRange)` | number | A `DynamicSlider` in place, with the value beside it |
| `Progress(max)` | number | A progress bar with a percentage in one colour that reads on fill and track. The fill is the accent toned 10 % toward the theme (darker on dark themes, brighter on light) |
| `button("label", \|node, w, cx\| ..)` | — | A push button in every row |
| `custom(\|ctx, w, cx\| ..)` | any | Anything; `CellCtx` gives the node, column, value, selection and style |

The slider delegates keep an integer cell an integer. Every in-place edit emits `ModelEvent::Edited`.

## Commands

```sh
cargo run --release --features gpui-kit --example lace_workbench -- dark_neo
```

```sh
cargo run --features gpui-kit --example dock_geometry
```

```sh
cargo run --features gpui-kit --example gallery
```

```sh
cargo run --features gpui-kit --example widgets
```

```sh
cargo run -p lace-widgets --example lacestyle
```

```sh
cargo run --example dump_tokens > tokens.json
```

```sh
cargo test --features gpui-kit
```

```sh
cargo test -p lace-widgets
```
