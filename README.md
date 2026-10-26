# lace-gpui

Lace's OKLCH theming engine, ported to [gpui-kit](https://crates.io/crates/gpui-kit)
(formerly `gpui-component`).

[Lace](https://github.com/opticsWolf/Lace) (Apache-2.0, © opticsWolf) derives a
complete dock theme from three to five seed colours in OKLCH — Björn
Ottosson's OKLab in polar form — and then lifts every token that still misses
a WCAG contrast floor or a surface-separation floor. This crate ports that
engine to Rust so a gpui-kit application can wear the same sixteen themes, and
cycle them live.

```toml
[dependencies]
lace-gpui = { version = "0.0.2", features = ["gpui-kit"] }
```

## Layout

| Module | What it is | Depends on |
|---|---|---|
| `color` | OKLab/OKLCH math, WCAG contrast, gamut mapping, hex — port of `lace/color_science.py` | nothing |
| `engine` | `ThemeSpec` → `Tokens` derivation — port of `build_theme` (`_DEPTH` steps, selection modes, `keep_tint`) | `color` |
| `contrast` | The rulebook and its fixer — port of `lace/theme_contrast.py` (`enforce`) | `color`, `engine` |
| `themes` | 16 curated presets, dark-to-light, from `midnight` to `cyberpunk_neon` | `engine` |
| `skin` | The bridge: `apply("dracula", cx)` paints a running gpui-kit app | gpui-kit 0.8 |

The colour core and the engine have **zero dependencies**; only the `gpui-kit`
feature pulls a crate.

## Using it

```rust
// Anywhere — no gpui in sight:
let tokens = lace_gpui::build(lace_gpui::themes::by_name("dracula").unwrap());
assert_eq!(tokens.panel_bg, [68, 71, 90, 255]);

// In a gpui-kit app, after `gpui_kit::init(cx)`:
lace_gpui::skin::apply("midnight", cx);          // paint once
let now = lace_gpui::skin::apply_next(&now, cx); // cycle all 16 live
```

`examples/gallery.rs` is a full dock application — left/right/bottom docks,
tabbed center, and a status-bar button that walks the theme set. The center's
**Geometry** tab is a per-token geometry lab: radii swatches, real gpui-kit
buttons, Lace's outline recipe (`color::mix(button_bg, text,
outline_strength)`), the dock border at `border_width`, the focus ring at
`focus_width`, and the raw geometry tokens as chips — every figure drawn at
its real token value for the theme on screen:

```sh
cargo run --example gallery --features gpui-kit
```

`examples/widgets.rs` is the component catalogue: a title strip with live theme
cycling, a navigation sidebar, five scrolling pages (Controls, Forms, Data,
Feedback, Navigation) and a status bar. It puts the common gpui-kit widgets
through their paces — buttons, checkboxes, radios, switches, sliders, a rating,
inputs, a select, a date picker, a colour picker, a table, a list, tags,
avatars, charts, alerts, dialogs, sheets, notifications, popovers, menus,
tooltips, skeletons, an empty state, tabs, a stepper, pagination, breadcrumbs
and an accordion:

```sh
cargo run --example widgets --features gpui-kit
```

`examples/dump_tokens.rs` needs no features and no display: it prints all 16
themes' enforced `Tokens` (78 colours + 7 geometry values + spec knobs) as
JSON, e.g. for docs or diffing:

```sh
cargo run --example dump_tokens > tokens.json
```

## The sixteen themes

| Group | Themes |
|---|---|
| Basics (classic 4px chassis, darker hovers, faint outlines) | midnight, dark, mocha, slate, caramel, neutral, cream, light |
| Editor Classics (stock chassis, lighter hovers) | dracula, monokai, nordic, catppuccin, tokyo_night, solarized_dark, solarized_light |
| Neon (10px cards, 8px tab tops) | cyberpunk_neon |

Palettes are ported one for one from `lace/dock_custom_theme.py`; the Basics
keep the classic chassis (`corner_radius` 4, `outline_strength` 0.08, inactive
borders at alpha 85) and midnight opts out of `keep_tint` so its active tab
text stays pure white.

## Fidelity notes

Faithful port of Lace 0.9.7's build path — the same `_DEPTH` tables, the same
contrast/separation/touch rules, the same `EXPLICIT_MAX_DE = 0.04` cap on
explicit colours, the same fix order (touch, then separation, then contrast).
Three documented simplifications:

1. **No Qt palette roles.** gpui has no Fusion bevel; `color_light` /
   `color_mid` / `color_dark` ride along in `Tokens` for downstream use but
   no gpui-kit field consumes them.
2. **Flat hover fills.** Lace's sidebar hover is a two-stop gradient
   (`tab_bg_hover_start`/`end`); gpui-kit takes one colour, so the bridge
   maps the start stop. Both tokens remain in `Tokens`.
3. **Status visibility floor.** `enforce` gains one small rule Lace applies
   at a different layer: a status colour must reach 2.9:1 on a dark canvas
   (1.2:1 on a light one, where mid-tone grounds like slate leave saturated
   colours little room). Explicit colours only move when the move helps, and
   stay within the same `EXPLICIT_MAX_DE` cap.

The eight classic Basics are also checked token-for-token against Lace
0.9.7 in `tests/classic_parity.rs`, including the shared-list behaviour
(border/focus mirrors, disabled action glyphs, accent/highlight/overlay and
tooltip text).

A few second-order differences remain outside the classic Basics:
catppuccin and tokyo_night's shared text is solved as one grouped contrast
pass (a few RGB steps from Lace's sequential pass); solarized_dark keeps the
port's status floor; and cyberpunk_neon's sidebar tab rings fall back to the
border/accent because `ThemeSpec` does not carry Lace's
`sidebar_tab_border_*` overrides yet.

`enforce` also never lets an explicit colour end worse than the spec left it
(the `worse` tiebreak in candidate scoring), and only moves it when a rule it
misses improves.

## The bridge

`skin::apply_to_theme_color` maps ~125 of gpui-kit 0.8's `ThemeColor` fields
from the `Tokens` — backgrounds, text pairs, tabs (Lace's tab strip *is* the
title strip), sidebar, lists and tables (zebra rows take Lace's
`alternate_base`), popovers (Lace's tooltip pair), status colours with
readable foregrounds, chart ramp, and the base-16 stand-ins. Fields Lace
doesn't model keep whatever the initialized theme already had.

Non-colour knobs: `radius` ← `control_radius`, `radius_lg` ← `corner_radius`,
`scrollbar_mode` ← the spec's scrollbar choice, `mode` ← `is_light`.
`skin::dock_chrome` picks `PanelStyle` from the chassis geometry.

## Tests

- `src/color.rs` — 14 unit tests: matrix round trips, gamut mapping keeping L
  and hue, WCAG reference values, hex parsing, mix/step/ensure_contrast.
- `tests/engine.rs` — 14 integration tests: every theme derives and has a
  unique token fingerprint; `is_light` per chassis; body / active-tab /
  muted / selection text floors; surface and hover separation; status
  visibility; `keep_tint` semantics; the tint-selection contract (±3
  channels around the derived input); geometry passthrough; explicit colours
  stay capped.
- `tests/classic_parity.rs` — the eight classic Basics' shared-token layer
  pinned to Lace 0.9.7's exact output: the 85-alpha idle border, its tab /
  side-panel / title mirrors, the shared disabled action glyph, the
  accent-derived selection fill and overlay, and tooltip text.

```sh
cargo test                              # core: runs anywhere
cargo test --features gpui-kit          # + skin tests; needs GUI dev libs on Linux
cargo check --all-targets --features gpui-kit
```

On a headless Linux box the feature build links gpui's windowing stack
(`libxkbcommon`, `libxkbcommon-x11`, …) and will need those packages
installed; `cargo check` needs none of them.

## License

Apache-2.0. The OKLab constants are Björn Ottosson's (MIT-licensed
reference); the theme engine, the sixteen palettes and the chassis are
opticsWolf's Lace, ported with the license and attribution intact. See
`LICENSE` and `NOTICE`.
