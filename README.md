# lace-gpui

Lace's OKLCH theming engine and dock look, ported to
[gpui-kit](https://crates.io/crates/gpui-kit) 0.8 (formerly `gpui-component`).

[Lace](https://github.com/opticsWolf/Lace) (Apache-2.0, © opticsWolf) is a
PySide6 docking framework. It derives a complete dock theme from three to five
seed colours in OKLCH — Björn Ottosson's OKLab in polar form — and then lifts
every token that still misses a WCAG contrast floor or a surface-separation
floor. This crate ports:

- the **engine**: 37 curated themes, the contrast rulebook, and a
  zero-dependency colour core;
- the **bridge** onto gpui-kit's `Theme`, plus LaceStyle's flat widget layer;
- the **Lace dock**: rounded, gapped panes, Lace's tab strip, auto-hide
  sidebars, floating drag previews and the drop overlay.

The aim is screenshot parity with PySide Lace on the same theme.

```toml
[dependencies]
lace-gpui = { version = "0.0.5", features = ["gpui-kit"] }
```

See [QUICKREF.md](QUICKREF.md) for the API at a glance and
[ARCHITECTURE.md](ARCHITECTURE.md) for how the pieces fit together.

## Layout

| Module | What it is | Needs |
|---|---|---|
| `color` | OKLab/OKLCH math, WCAG contrast, gamut mapping, hex. Port of `lace/color_science.py` | nothing |
| `engine` | `ThemeSpec` → `Tokens` derivation. Port of `build_theme` (`_DEPTH` steps, selection modes, `keep_tint`) | nothing |
| `contrast` | The rulebook and its fixer. Port of `lace/theme_contrast.py` (`enforce`) | nothing |
| `themes` | 37 curated presets in five groups (`ALL`, `GROUPS`, `by_name`) | nothing |
| `skin` | The bridge: `apply("dracula", cx)` paints a running gpui-kit app and installs the dock styles | `gpui-kit` |
| `base_style` | LaceStyle's flat widget layer over gpui-kit's stock widgets | `gpui-kit` |
| `button` | `GlowButton`: theme-driven hover/press halos on stock buttons | `gpui-kit` |
| `geometry` | `GeometryDockSkin`: gaps, rounded panes, outlines, shadow/glow, splitters | `gpui-kit` |
| `edge_tabs` | `EdgeTabBar`: Lace's dock tab strip (outlines, accent stripe, overflow, close and pin buttons) | `gpui-kit` |
| `sidebar` | `Sidebars`: auto-hide rails with peek, pin/unpin, drag-to-dock and three focus modes | `gpui-kit` |
| `floating` | `FloatingPane` drag preview and Lace's five-target `drop_overlay` | `gpui-kit` |
| `dock_icons` | The tabler icons Lace draws its close and title-bar buttons with | `gpui-kit` |

The colour core, the engine, the contrast pass and the themes have **zero
dependencies**. Only the `gpui-kit` feature pulls in a crate.

The workspace also holds **`lace-widgets`**, which draws LaceStyle as custom
gpui widgets (push buttons, checks, fields, ranges, containers, chrome, item
views) with vector glyphs. The workbench example fills its panes with them.

## Using it

```rust
// Anywhere — no gpui in sight:
let tokens = lace_gpui::build(lace_gpui::themes::by_name("dracula").unwrap());
assert_eq!(tokens.panel_bg, [68, 71, 90, 255]);

// In a gpui-kit app, after `gpui_kit::init(cx)`:
lace_gpui::skin::apply("midnight", cx);          // palette + dock styles
lace_gpui::skin::apply_base("midnight", cx);     // + LaceStyle's flat controls
let now = lace_gpui::skin::apply_next(&now, cx); // cycle all themes live
```

Every `apply*` call does three things:

- writes the gpui-kit `Theme`;
- installs the three dock-style globals: `TabEdgeStyle`, `SidebarStyle` and
  `DropOverlayStyle`;
- refreshes the windows.

A dock built with `GeometryDockSkin` picks up a new theme on the next frame.

## The Lace dock

```rust
use lace_gpui::geometry::{DockGeometryStyle, GeometryDockSkin, OutlineStyle, SplitterCap, SplitterLength};
use lace_gpui::sidebar::{SidebarFocus, Sidebars};

let style = DockGeometryStyle::new()
    .gap(px(7.))
    .gap_outer(px(6.))
    .panel_radius(cx.theme().radius_lg)        // the theme's corner_radius
    .outline(OutlineStyle::themed(px(1.)))
    .splitter(|s| s.thickness(px(3.)).length(SplitterLength::Percent(1.0)).cap(SplitterCap::Rounded));

let (area, skin) = GeometryDockSkin::dock_area("main", None, style, window, cx);
skin.set_close_button_visible(true, cx);
// ... area.update(cx, |a, cx| { a.set_center(..); a.set_dock(..); });

let sidebars = cx.new(|cx| {
    let mut s = Sidebars::new(area.clone(), &skin, cx);
    s.pinnable(&console, Some(icon));          // name every panel that may be pinned
    s
});
sidebars.update(cx, |s, cx| s.set_focus_behavior(SidebarFocus::TakeAndRestore, cx));
// Render `sidebars` as the window's root content instead of `area`.
```

What follows the theme:

- **Pane body**: filled with Lace's `PANEL.bg_normal` (`Tokens::panel_bg`).
  The gap between panes shows the theme background.
- **Tab strip**: Lace's measured paddings (lead 8, 4 before the close box,
  24 px close box, tail 5, 3 px between tabs), the top-corner radius and the
  outline. The active tab's accent stripe sits on the bottom edge (2 px on
  the classics, 1.5 px on the Neo themes). Some themes have no stripe:
  tokyo_night, neon_dusk and the haze family. When a group loses focus, its
  active tab dims.
- **Sidebars**: rails along the left, right and bottom edges. Tabs are 28 px
  wide with rounded outlines and a left-to-right hover gradient.
  - Hovering a tab *peeks* its panel, which hides when the pointer leaves.
  - Clicking a tab *opens* the panel; Escape or a click elsewhere closes it.
  - The title bar's pin button unpins a panel back into its old group.
  - Dragging a sidebar tab onto the dock docks it there.
- **Focus modes** (`SidebarFocus`), Lace's three:
  - **Take Focus & Restore** (default): opening focuses the panel, and
    closing gives focus back to where it was.
  - **Take Focus Only**: opening focuses the panel; nothing is restored.
  - **No Focus Transfer**: opening leaves focus where it is. Clicking into
    the panel still focuses it.
- **Drag and drop**: a dragged pane floats whole, with its tab strip and live
  content, at 88 % opacity. The hovered area shows Lace's translucent overlay
  and the cross of five drop targets.

What gpui-kit 0.8 can and cannot express is audited in
[docs/gpui-kit-0.8-feasibility.md](docs/gpui-kit-0.8-feasibility.md). The
short version:

- **Exact:** rounded panes, gap colour and width (across split seams, dock
  regions and the outer frame), splitter thickness, length and caps, and
  outlines.
- **Effects** (shadow and glow) are the group frame's own `BoxShadow`.
  gpui-kit has no separate unclipped layer, but paint order still keeps them
  in the gap.
- **Vertical split gaps are painted, not laid out.** `ResizablePanel` is
  always a row, so a group in a `v_split` paints its card inset by half the
  gap.
- **Gaps wider than 9 px** leave inert strips at split seams, because the hit
  band is a crate-private constant in gpui-base.
- **Dock-edge resize handles** keep the stock look; `DockSkin` owns them.

## Examples

All but `dump_tokens` need `--features gpui-kit`.

| Example | What it shows |
|---|---|
| `lace_workbench` | The full Lace look. The dock, edge tabs, sidebars, floating panes and drop overlay are filled with `lace-widgets` panes. The toolbar has theme and preset pickers and the sidebar focus mode. Takes an optional theme name. |
| `dock_geometry` | The geometry skin on stock gpui-kit panels: three presets, a Split lab, and a per-panel shadow toggle. |
| `gallery` | A dock application with a per-token Geometry lab and the Lace style / Theme only toggle. |
| `widgets` | The gpui-kit component catalogue under a Lace theme. |
| `buttons` | The `GlowButton` halos. |
| `button-group-repro` | A reproduction of an upstream `ButtonGroup` stack overflow. |
| `dump_tokens` | Every theme's enforced `Tokens` as JSON. Needs no display. |
| `lace-widgets`: `lacestyle` | The LaceStyle widget catalogue. |

```sh
cargo run --release --features gpui-kit --example lace_workbench -- violet_haze
```

```sh
cargo run --features gpui-kit --example dock_geometry
```

```sh
cargo run --example dump_tokens > tokens.json
```

```sh
cargo run -p lace-widgets --example lacestyle
```

## The themes

| Group | Themes |
|---|---|
| Basics (classic 4 px chassis, darker hovers, faint outlines) | midnight, dark, mocha, slate, caramel, neutral, cream, light |
| Basics Neo (rounder Neo chassis, 1.5 px tab stripe) | midnight_neo, dark_neo, mocha_neo, slate_neo, caramel_neo, neutral_neo, cream_neo, light_neo |
| Editor Classics (stock chassis, lighter hovers) | dracula, monokai, nordic, catppuccin, tokyo_night, solarized_dark, solarized_light |
| Neon | cyberpunk_neon, neon_dusk |
| Edge Treatments (outlined tabs and frames) | cyberpunk_edge(_neutral, _light), violet_haze(_neutral, _light), midnight_haze(_neutral, _light), slate_amber_dark, slate_amber, slate_amber_light |

The palettes are ported one for one from `lace/dock_custom_theme.py`.
`themes::GROUPS` lists them in Lace's menu groups. `themes::by_name` ignores
case and treats `-` and spaces as `_`.

## Fidelity notes

This is a faithful port of Lace 0.9.7's build path:

- the same `_DEPTH` tables;
- the same contrast, separation and touch rules;
- the same `EXPLICIT_MAX_DE = 0.04` cap on explicit colours;
- the same fix order: touch, then separation, then contrast.

There are two documented simplifications:

1. **No Qt palette roles.** gpui has no Fusion bevel. `color_light`,
   `color_mid` and `color_dark` ride along in `Tokens`, but no gpui-kit field
   consumes them.
2. **Status visibility floor.** `enforce` gains one small rule that Lace
   applies at a different layer: a status colour must reach 2.9:1 on a dark
   canvas, or 1.2:1 on a light one.

Lace's sidebar hover gradient (`tab_bg_hover_start` → `end`) is drawn as a
gradient by `Sidebars`. gpui-kit's own `sidebar_accent` field takes the start
stop.

`tests/classic_parity.rs` checks the eight classic Basics token for token
against Lace 0.9.7. A few second-order differences remain outside them:

- catppuccin and tokyo_night solve their shared text as one grouped contrast
  pass, which lands a few RGB steps from Lace's sequential pass;
- solarized_dark keeps the port's status floor.

## The bridge

`skin::apply_to_theme_color` maps about 125 of gpui-kit 0.8's `ThemeColor`
fields from the `Tokens`:

- backgrounds and text pairs;
- tabs (Lace's tab strip *is* the title strip) and the sidebar;
- lists and tables (zebra rows take Lace's `alternate_base`);
- popovers (Lace's tooltip pair);
- status colours with readable foregrounds;
- the chart ramp and the base-16 stand-ins.

Fields Lace doesn't model keep whatever the initialized theme already had.

The non-colour knobs:

- `radius` ← `control_radius`;
- `radius_lg` ← `corner_radius`;
- `scrollbar_mode` ← the spec's scrollbar choice;
- `mode` ← `is_light`.

`skin::tab_edges`, `skin::sidebar_style` and `skin::drop_overlay_style` derive
the dock globals. `skin::dock_chrome` picks gpui-kit's `PanelStyle`.

LaceStyle's widget layer and its limits are described in
[docs/lace-style-widgets.md](docs/lace-style-widgets.md).

## Tests

```sh
cargo test
```

```sh
cargo test --features gpui-kit
```

```sh
cargo test -p lace-widgets
```

```sh
cargo check --all-targets --features gpui-kit
```

| Suite | Tests | Covers |
|---|---|---|
| lib (`--features gpui-kit`) | 48 | Colour math, `BaseStyle`, the skin's theme-to-dock mapping (tab stripes, sidebar rings), geometry defaults and side ownership, overflow, floating sizes and sidebar layout |
| `tests/engine.rs` | 16 | Every theme derives with a unique fingerprint. Text, surface and status floors, `keep_tint`, geometry passthrough, explicit-colour caps |
| `tests/classic_parity.rs` | 1 | The classic Basics' shared tokens pinned to Lace 0.9.7 |
| `lace-widgets` | 29 | LaceStyle tones, rules and paint |

The `gpui-kit` feature build links gpui's windowing stack. On a headless Linux
box it needs `libxkbcommon`, `libxkbcommon-x11` and related packages;
`cargo check` needs none of them.

## License

Apache-2.0. The OKLab constants are Björn Ottosson's (MIT-licensed
reference). The theme engine, the palettes, the chassis and the dock design
are opticsWolf's Lace, ported with the license and attribution intact. See
`LICENSE` and `NOTICE`.
