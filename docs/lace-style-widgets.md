# LaceStyle for gpui-kit widgets

Lace has two layers of theming:

- **The palette** — `ThemeSpec` seeds derived into `Tokens` and enforced by
  `contrast.py`. `lace-gpui`'s `skin` module ports this layer one token at a
  time.
- **`LaceStyle`** — a `QProxyStyle` over Fusion (`lace/lace_style.py`, drawing
  in `lace/style/`). It keeps Fusion's metrics and paints over Fusion's dated
  drawing: flat fills, one stroke, vector glyphs, a single control radius, and
  state changes expressed as lightness steps rather than gradients.

`BaseStyle` (`src/base_style.rs`) is the second layer adapted to gpui-kit. It
is not another colour preset: it is applied on top of any of the sixteen
palettes, and it can be toggled at runtime.

```rust
use lace_gpui::skin::{self, WidgetStyle};

// Palette + LaceStyle's flat widget rules.
skin::apply_styled("midnight", WidgetStyle::LaceBase, cx);
// The same, named for the common case.
skin::apply_base("midnight", cx);
// The palette alone, over gpui-kit's stock widgets.
skin::apply_styled("midnight", WidgetStyle::Theme, cx);
```

`skin::apply` and `skin::apply_spec` keep their old palette-only behaviour.
`skin::apply_next_styled` cycles presets without dropping the chosen layer.

## The rules that carry over

| LaceStyle rule | Source | gpui-kit mapping |
|---|---|---|
| `control_radius` / `corner_radius` | `LaceStyle.set_tokens`, fed by `DockStyleCategory.CORE` | `Theme.radius` / `Theme.radius_lg` |
| Button face, `+0.04` hover, `+0.08` pressed, toward contrast, for every variant | `_paint.HOVER_STEP`, `_paint.PRESS_STEP`, `_buttons.face_colors` | `button_hover`, `button_active`, the primary/secondary pairs, and every status button pair; the depth pass adds the face sheen |
| Progress fill: accent, rounded ends | `_range.progress_contents` | `progress_bar` (accent-to-`accent_bright` sweep) |
| Button outline: text mixed over the button face at `outline_strength`, lifted to the border floor | `_paint.stroke`, `_paint.legible`, `_buttons.face_colors` | Structural `ThemeColor::border` family (`frame_outline_color`, opaque); `ThemeColor::input` carries the indicator value (below) |
| Field outline: text mixed over the field fill at `outline_strength`, lifted to the border floor | `_inputs.field_outline` | `BaseStyle::field_outline_color` (derived and documented); shares the `ThemeColor::input` slot with indicators, so the indicator value wins on screen |
| Unchecked checkbox/radio outline: field stroke lifted to the non-text UI floor (`ui_ratio`), backed off 20% via `INDICATOR_FLOOR_SCALE = 0.8` | `_primitives._outline` (ui floor), backed off so the opaque 1 px line reads as an edge, not a glow | `ThemeColor::input` (`indicator_outline_color`, opaque) |
| Classic chassis holds outlines to `1.15:1`; everything else to the theme's border floor | `_BASIC_CLASSIC` sets `outline_contrast="low"` | `BaseStyle::border_ratio` (inferred from `outline_strength <= 0.1`) |
| Keyboard focus ring: accent, held to `ui_ratio` (3:1), width `focus_width` | `_paint.legible`, `_buttons._face` | `ThemeColor::ring` keeps the full-strength accent for the 1 px tinted edge; `Theme.focus_ring` is off because gpui-kit's hard 3 px band cannot blur — the gallery adds a soft `BoxShadow` halo around the focused field/control; `Theme.drag_border` stays opaque |
| Scrollbar thumb: text over the window at `.32` idle, `.45` hover, `.55` pressed; `.05` track; all held to `ui_ratio` | `_primitives.scrollbar` | `scrollbar_thumb`, `scrollbar_thumb_hover`, `scrollbar` |
| Slider: accent groove fill and button-face handle with an outline | `_range.slider` | `slider_bar` (accent), `slider_thumb` (button face) |
| Checked indicator: accent fill, readable glyph; radio is a dot, not a check | `_primitives.check_box`, `_primitives.radio_button` | Accent wash from `primary`; checked checkbox keeps its glyph plus an app-side halo; checked radio is covered by the gallery's `dot_radio` overlay (14 px disc hides the stock check glyph, 6 px dot sits on top) because stock `Radio` hard-codes `checkbox_check_icon` |
| `field_outline = true` | `LaceStyle` default | Recorded on `BaseStyle`; applied via the shared `input` slot (indicator value wins) |
| `splitter_length = 50`, `focus_width = 2` | `LaceStyle` defaults | `focus_width` → `Theme.focus_ring`; splitter length is advisory (the dock geometry skin owns splitters) |

`BaseStyle::from_tokens` derives everything from a built `Tokens`.
`BaseStyle::from_spec` uses the spec's real contrast level; `from_tokens`
assumes `normal`, which is true of all sixteen curated presets.

## Depth on top of LaceStyle

LaceStyle itself is flat. The Lace demo gets depth from the surrounding Qt and
dock chrome — tonal surfaces, soft popup shadows and the sidebar tab's hover
gradient.
gpui-kit's `ThemeToken` keeps a solid representative colour plus a renderable
`Background`, and `Background` accepts a linear gradient, so `BaseStyle::apply`
can add a shallow relief without touching the solid colours components read
for text and icons:

| Surface | Gradient |
|---|---|
| Neutral button faces at rest | top `+0.010` L, bottom `-0.014` L |
| Neutral hover faces | top `+0.018` L, bottom `-0.022` L |
| Pressed neutral faces | inset: top `-0.014` L, bottom `+0.014` L |
| Accent faces at rest and status faces | text-aware: a white-labelled face keeps its enforced colour at the top and shades down; a dark-labelled face lifts the top and keeps its enforced colour at the bottom (`+0.008`/`-0.010`) |
| Accent hover/pressed and status hover/pressed | same direction with `+0.012`/`-0.016`; checkbox/radio/switch wash uses half steps |
| Tabs, table/list heads, title and status bars | top `+0.006` L, bottom `-0.005` L |
| Canvas, sidebar, panels, cards | top `+0.004` L, bottom `-0.006` L |
| Scrollbar and slider thumbs | top `+0.014` L, bottom `-0.014` L |
| Progress fill | left accent → right `accent_bright` |

Hover faces carry more relief than resting ones and pressed faces invert the
sheen, so the state change reads as a lit surface rather than a flat colour
swap. gpui only offers two-stop linear gradients (no radial/conic, no
multi-stop edge-inward glow), so "inward" relief is expressed as a top-lit
raise on hover and an inset inversion on press, plus blurred `BoxShadow`
halos at the gallery layer where a true outer glow is wanted.

The accent direction is chosen per face from the contrast of white versus
black. Both gradient stops are therefore at least as readable as the flat,
contrast-checked accent: white-labelled accents only darken away from the
top, and dark-labelled accents only lighten above the bottom.

The theme surface has no shadow or blur slot, and gpui-kit's focus and
slider-thumb rings are hard-edged 3 px borders with no blur radius. `BoxShadow`
(with `blur_radius`) is gpui's only blur primitive, so the gallery draws its
own app-side effects where they matter:

- cards, the header, the sidebar and the sidebar footer card carry a soft
  black shadow scaled for dark and light modes;
- primary actions (`New item`, `Create`, `Save changes`) get a low-alpha
  accent halo (blur 18, alpha 0.28);
- a checked checkbox's indicator gets an accent halo (blur 10, alpha 0.32);
- a focused input or textarea gets an accent halo (blur 16, alpha 0.32);
- the slider thumb hides its hard 3 px ring under a surface-coloured
  border-only disc and paints the halo outside it (blur 10, alpha 0.40);
- the selected sidebar row gets a softer accent glow;
- checked radios get the `dot_radio` overlay plus the checkbox-strength halo
  (skipped while disabled).

These effects use `BoxShadow` in `examples/widgets.rs`; they are presentation,
not tokens, so an application can vary them independently of the palette.
The overlays are transparent `div`s with no pointer handlers: the stock
checkbox, radio, slider, input and textarea underneath keep their own click,
drag, focus and keyboard handling.

## What gpui-kit cannot separate

These are structural limits of gpui-kit 0.8's theme surface, not omissions in
the derivation. `BaseStyle` picks the more visible half of each pair.

1. **One outline slot for buttons, fields and unchecked indicators.** `ThemeColor::input` is the border of an input, the border of a default button (and it seeds the dark input fill via `input.mix(transparent, 0.3)`), *and* the outline of an unchecked checkbox/radio. LaceStyle derives field/button outlines at the border floor and indicator outlines at the non-text UI floor. The base layer feeds the backed-off indicator value (`ui_ratio * 0.8`), because a 1 px box cannot afford a 1.15:1 line; button and field outlines are therefore near-matches rather than exact. Only the indicator value is backed off 20%: structural `frame_outline_color` and the derived field/button outlines stay at the border floor so they do not drop below visibility.
2. **One input colour for border and fill.** `ThemeColor::input` has no
   separate field-background token. In dark mode gpui-kit makes the fill from
   30% of that same colour. The fill stays in the right family, but LaceStyle's
   exact recessed `input_bg` is not representable.
3. **One large radius.** `Theme.radius_lg` serves cards, dialogs and popups.
   LaceStyle blends popups between the control and card radius (4px classic,
   6px on the 10px neo chassis), while gpui-kit would need a third radius to
   keep cards at 10px and popups at 6px. `BaseStyle::popup_radius` records the
   value; `radius_lg` keeps the card radius.
4. **One slider colour plus a hard thumb ring.** gpui-kit's `slider_bar` paints both the groove (at
   20% opacity) and its filled portion. LaceStyle fills the groove with the
   accent over a neutral track. The base layer keeps the accent fill. The
   thumb has one fill token and no stroke, so LaceStyle's outlined button-face
   handle keeps the fill but loses the outline; gpui-kit also grows a hard
   3 px ring on hover that cannot blur, so the gallery hides it under a
   surface-coloured disc and paints a blurred halo outside.
5. **One active scrollbar token.** Hover and drag share
   `scrollbar_thumb_hover`. LaceStyle's `.45` hover and `.55` pressed mix
   collapse; the base layer keeps the hover value.
6. **No theme-level splitter length.** `splitter_length = 50` is stored on
   `BaseStyle` for documentation; splitter geometry comes from
   `geometry::DockGeometryStyle` instead.
7. **No per-widget `QStyle` dispatch.** gpui-kit components choose their own
   tokens. Rules without a named slot (e.g. LaceStyle's flat menu popup
   radius, tab-close wash) stay with the component.
8. **No shadow or blur tokens.** `Theme` exposes one boolean shadow switch,
   and each component draws its own shadow. `BaseStyle` keeps `focus_ring`
   off; card, action, checkbox, input, slider and radio halos are app-side
   in the gallery via blurred `BoxShadow`, not part of `ThemeTokens`. Stock
   `Radio` also hard-codes the checkbox check glyph, so the gallery covers a
   checked radio with the `dot_radio` overlay (no pointer handlers; the stock
   control keeps focus and clicks).

## Live toggle

`examples/widgets.rs` starts with `WidgetStyle::LaceBase` and adds a
**Lace style / Theme only** button to the title strip. Switching it re-applies
the current preset with the other layer, and the header's theme arrows keep
the selected layer. The status bar shows it as `widgets: Lace style` or
`widgets: Theme only`. The gallery shell's own shadows and glows stay in both
modes because they are application presentation rather than theme tokens.

```sh
cargo run --example widgets --features gpui-kit
```
