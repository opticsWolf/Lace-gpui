# Architecture

lace-gpui ports PySide6 Lace in two layers:

- a pure colour engine that knows nothing about any GUI toolkit;
- a gpui-kit layer that paints a running app with the engine's output and
  rebuilds Lace's dock look on top of gpui-kit's dock.

This document explains how data flows between the layers and where each
responsibility lives. For the API, see [QUICKREF.md](QUICKREF.md).

## The two layers

```
                ┌────────────────────── no dependencies ──────────────────────┐
  themes.rs     │  ThemeSpec (seeds, chassis, geometry knobs)                 │
     │          │      │                                                      │
     ▼          │      ▼  engine::build                                       │
  engine.rs ────┼─▶ derive Tokens  (_DEPTH steps, selection modes, keep_tint) │
     │          │      │                                                      │
     ▼          │      ▼  contrast::enforce                                   │
  contrast.rs ──┼─▶ enforced Tokens (WCAG, separation, touch; explicit cap)   │
  color.rs      │   (OKLab/OKLCH, gamut mapping, contrast ratio, mix)         │
                └──────────────────────────────┬──────────────────────────────┘
                                               │ feature "gpui-kit"
                ┌──────────────────────────────▼──────────────────────────────┐
  skin.rs       │  apply_spec_styled(spec, style, cx)                         │
                │   ├─ apply_tokens        → gpui_kit Theme / ThemeColor      │
                │   ├─ BaseStyle::apply    → LaceStyle widget layer (opt.)    │
                │   ├─ TabEdgeStyle   ─┐                                      │
                │   ├─ SidebarStyle    ├─ installed as gpui Globals           │
                │   ├─ DropOverlayStyle┘                                      │
                │   └─ Theme::sync_base + refresh_windows                     │
                └──────────────────────────────┬──────────────────────────────┘
                                               │ read every frame
     geometry.rs ─ edge_tabs.rs ─ floating.rs ─ sidebar.rs  (the Lace dock)
```

### Colour engine (`color`, `engine`, `contrast`, `themes`)

**`ThemeSpec`** is a `const` struct that holds three kinds of input:

- the seed colours;
- the chassis: depth tables, hover direction, outline strength;
- the geometry and tab knobs: corner, control and tab radii, the tab indicator
  and its width, the title rule, border flags.

The chassis bases are `ThemeSpec::DEFAULT`, `BASIC_CLASSIC`, `EDITOR_CLASSIC`
and `BASIC_NEO`. The curated themes are struct updates over one of them, so a
theme only states where it differs from its family.

**`engine::build`** derives about 80 colour tokens plus geometry passthrough
into **`Tokens`**, then runs **`contrast::enforce`**. The rulebook is applied
in a fixed order: touch targets, then surface separation, then text contrast.
Explicit seed colours may only move within `EXPLICIT_MAX_DE`.

The output is plain `[u8; 4]` RGBA, so the whole engine is testable without a
display. `tests/engine.rs` and `tests/classic_parity.rs` pin it against Lace
0.9.7.

### Bridge (`skin`)

`skin` is the only place where the engine meets gpui-kit. Each `apply*` entry
point resolves a spec, builds tokens and then:

1. **Writes the gpui-kit `Theme`.**
   - `apply_to_theme_color` maps about 125 `ThemeColor` fields.
   - `apply_tokens` also sets the radii, scrollbar mode and light/dark mode.
2. **Adds the widget layer**, if one was requested. With
   `WidgetStyle::LaceBase`, `BaseStyle` derives LaceStyle's flat outlines,
   state steps and focus ring from the same tokens and writes the ones
   gpui-kit has slots for.
3. **Installs the dock globals:**
   - `TabEdgeStyle`, derived by `tab_edges`;
   - `SidebarStyle`, derived by `sidebar_style`;
   - `DropOverlayStyle`, derived by `drop_overlay_style`.

   gpui-kit's `Theme` has no fields for Lace's dock chrome, so these carry it
   as typed globals. Examples of that chrome: tab outlines, the accent stripe,
   the pane body colour, sidebar rings and the hover gradient.
4. **Refreshes.** `Theme::sync_base` rebuilds gpui-base's projection of the
   theme, and `refresh_windows` repaints.

The dock components read the globals with `::current(cx)` every frame, so a
theme switch needs no rewiring.

## The Lace dock

gpui-kit 0.8's dock (in gpui-base) owns the layout tree: splits, tab groups,
docks, drag and drop, and the drop decision. It exposes renderer seams:
`DockAreaRenderer`, `TabGroupRenderer` and `ResizeHandleRenderer`. Everything
Lace-specific is built behind those seams and from public dock API.

```
Sidebars (view; the window's root content)
 ├─ rails (left / right / bottom): sidebar tabs, drag sources
 ├─ overlay: the open or peeked pinned pane (track_focus, Escape)
 └─ DockArea (gpui-base)  ── renderer = GeometryDockSkin
      ├─ area frame / centre frame      gaps between dock regions, gap_outer
      ├─ split handles                  SplitterStyle bars
      └─ GeometryTabGroup per group     (TabGroupRenderer)
           ├─ frame                     rounded card, outline, effect, panel_bg,
           │                            inset for vertical split gaps (GroupSides)
           ├─ render_tab_bar ─▶ EdgeTabBar  Lace tabs, overflow, close / pin
           │                                └─ on drag: FloatingPane preview
           ├─ content_frame             padding that keeps content on the card
           └─ render_drop_indicator ─▶ floating::drop_overlay
```

### `geometry` — `GeometryDockSkin`

`GeometryDockSkin` implements `DockAreaRenderer`. It holds a
`DockGeometryStyle` (`Copy`) in a cell, together with the per-panel effect
overrides. Anything it does not restyle is delegated to gpui-kit's stock
`DockSkin`:

- the stock tab strip, for themes without outlines or a stripe;
- dock chrome;
- dock-edge resize handles;
- placeholders.

Gaps are drawn at two levels:

- **Dock regions.** The area frame is a flex row and the centre frame a flex
  column, so their `gap` separates left, centre, right and bottom without
  touching any panel geometry.
- **Split seams.** `GroupSides::for_child` and `GroupSides::nested` work out
  which sides of a group face a seam. Horizontal seams are real margins.
  `ResizablePanel` is always a row, so a vertical seam keeps the slot and
  paints the card inset by half the gap.

`GeometryTabGroup::frame` fills the card with `TabEdgeStyle::panel_bg`, which
is Lace's `PANEL.bg_normal`. The gap behind the cards shows the theme
background. Shadows and glows are `BoxShadow`s on the same frame.

### `edge_tabs` — `EdgeTabBar`

gpui-kit's stock tab has fixed 1 px side borders, and the stock strip gives a
skin no control over where tabs land. When `TabEdgeStyle::draws_tabs()` is
true (the theme has an outline or an indicator), the skin renders
`EdgeTabBar` instead.

`EdgeTabBar` rebuilds the strip from public API:

- select, close and reorder, plus drag out and merge;
- zoom and the panel toolbar;
- the dock collapse buttons;
- overflow arrows (`TabOverflow`).

It also reproduces Lace's look:

- measured paddings;
- the top-corner radius and outline;
- the accent stripe (`TabIndicator::{Top, Bottom}`);
- the title rule;
- unfocused dimming;
- `below_title` frames;
- the close box, and a pin button that reaches `Sidebars` through the
  `PinTarget` trait.

### `floating` — in-flight panes

When a tab drag starts, `EdgeTabBar` (or a sidebar tab) creates a
`FloatingPane`. This is the drag preview: the whole pane, with a one-tab strip
over a second live render of the panel's view, at `FLOAT_OPACITY`. gpui
cannot snapshot an element into an image, so the panel is rendered twice.
Each group records its last bounds in `GROUP_BOUNDS`, so the preview opens at
the docked size.

gpui-base decides where a drop lands and passes a `DropIndicator` to
`render_drop_indicator`. `drop_overlay` then draws Lace's `DockOverlay`: the
translucent region the pane would take, and a cross of five targets with the
hovered one lit. Its colours come from `DropOverlayStyle`.

### `sidebar` — `Sidebars`

gpui-kit's dock has no auto-hide sidebars, so `Sidebars` wraps the
`DockArea` in its own view and is rendered in its place.

- **Pin and unpin.** Pinning removes a panel from the dock but keeps its view
  alive. Unpinning adds it back to the group it came from, when that group
  still exists. `DockArea::remove_panel` is generic over the panel type, so
  the host names each pinnable panel once with `pinnable`.
- **Rails.** Each rail holds vertical tabs with rotated labels. gpui lays out
  text horizontally only, so a vertical label is drawn as SVG text and tinted
  like an icon. The tabs carry the theme's rings, the indicator and the
  left-to-right hover gradient.
- **Overlay.**
  - `open(panel, peek)` slides the panel out.
  - A **peek** (hover) hides again after the pointer leaves, with a short
    delay.
  - An **open** (click) stays until a click outside, Escape or another click
    on the tab.
  - The overlay calls `track_focus` on the panel's own focus handle, so
    clicking into it and pressing Escape both work.
- **Focus modes** (`SidebarFocus`):
  - `open` records the window's focus when it lies outside every pinned pane.
    It then focuses the pane, unless the mode is `NoTransfer`.
  - `close_overlay` remembers the pane's handle.
  - On the next render, `TakeAndRestore` returns focus to the recorded handle,
    but only if focus is still on the closed pane or nowhere. A user who has
    clicked somewhere else keeps that focus.
- **Drag to dock.** Dragging a sidebar tab shows a `FloatingPane`. Dropping it
  calls `dock_at`, which puts the panel into the target group or splits it
  off one side.

## The `lace-widgets` crate

`lace-widgets` is a workspace member that depends on `lace-gpui`. It
re-implements LaceStyle as custom gpui widgets rather than restyling gpui-kit's
stock ones:

- `style` holds the palette, tokens and rules, all pure and testable. Its
  `LaceStyle` is installed as a global with `install` or `install_preset`.
- `glyph` holds vector glyphs (chevrons, ticks, crosses, arcs), stroked
  through lyon with round caps.
- `controls` holds the widgets. Each paints itself from the installed style.
- `views` holds item views and models.

`base_style` in the main crate is the other route: it keeps gpui-kit's widgets
and changes only what gpui-kit's theme can express. The workbench example uses
`lace-widgets` for its panes. The `gallery` and `widgets` examples use
`base_style`. Both routes are described in
[docs/lace-style-widgets.md](docs/lace-style-widgets.md).

## Design rules

- **The engine has no dependencies.** Anything that touches gpui is behind
  the `gpui-kit` feature.
- **Theme-driven by default.** The dock chrome reads globals that `skin`
  installs, so a hand-built `TabEdgeStyle` or `SidebarStyle` can override a
  theme without forking the skin.
- **Stock behaviour is kept.** Every replacement (the tab strip, sidebars, the
  drop overlay) keeps gpui-kit's interactions and changes only the drawing.
  `DockGeometryStyle::default()` reproduces the stock look exactly.
- **Parity is measured.** Paddings, colours and stripe widths come from PySide
  Lace's source and are checked against screenshots of the same theme.
- **Divergences are written down.** Wherever gpui-kit 0.8 cannot express
  something, the module doc and
  [docs/gpui-kit-0.8-feasibility.md](docs/gpui-kit-0.8-feasibility.md) say so.

## Tests

| Where | What |
|---|---|
| `src/color.rs` | Matrix round trips, gamut mapping, WCAG reference values, hex, mix and step |
| `tests/engine.rs` | Every theme derives with a unique fingerprint; the contrast and separation floors; `keep_tint`; explicit-colour caps; sidebar ring overrides |
| `tests/classic_parity.rs` | The classic Basics token for token against Lace 0.9.7 |
| `src/skin.rs` | The theme-to-dock mapping: which themes draw Lace tabs, and the stripe side and width |
| `src/geometry.rs` | Stock-look defaults, splitter defaults, side ownership, nested seams, effect order |
| `src/edge_tabs.rs`, `src/floating.rs`, `src/sidebar.rs` | Overflow, preview sizing, rail layout |
| `lace-widgets` | Style tones, rules and paint |

Interactive behaviour is checked by running `lace_workbench` against
screenshots of PySide Lace: drag and drop, the focus modes, peek and pin. The
example's toolbar can switch the theme, the dock preset and the sidebar focus
mode live.

## Other documents

| File | Contents |
|---|---|
| [gpui-kit-geometric-dock-styling-spec.md](gpui-kit-geometric-dock-styling-spec.md) | The original geometric dock spec (features F1–F8) |
| [docs/gpui-kit-0.8-feasibility.md](docs/gpui-kit-0.8-feasibility.md) | What gpui-kit 0.8 can express, item by item |
| [docs/gpui-kit-0.8-evaluation.md](docs/gpui-kit-0.8-evaluation.md) | The gpui-kit 0.8 evaluation |
| [docs/lace-style-widgets.md](docs/lace-style-widgets.md) | How LaceStyle maps onto gpui-kit widgets, and its limits |
