# Geometric Dock Styling for gpui-kit — Design Specification

| | |
|---|---|
| **Author** | GPUI-Kit Dock Working Group |
| **Date** | 2026-10-22 |
| **Baseline** | longbridge/gpui-kit master @ v0.8.0 (pins GPUI via gpui-pre 0.3.9) |
| **Status** | Design review draft — *Spec First*: this document fixes the mechanism, public API, and defaults before any code is written |
| **Verification bar** | `cargo check` (compile-first), extended with the dock's headless UI test harness |

---

## Table of Contents

- [1. Purpose and Scope](#1-purpose-and-scope)
- [2. Verified Upstream Architecture](#2-verified-upstream-architecture)
  - [2.1 The renderer seam](#21-the-renderer-seam)
  - [2.2 Current splitter and gap geometry](#22-current-splitter-and-gap-geometry)
  - [2.3 GPUI primitives available](#23-gpui-primitives-available)
- [3. Geometry Model: Gaps, Radii, Outlines](#3-geometry-model-gaps-radii-outlines)
  - [3.1 Gap width and gap colour](#31-gap-width-and-gap-colour)
  - [3.2 Corner radius over body and tab bar](#32-corner-radius-over-body-and-tab-bar)
  - [3.3 Panel outline](#33-panel-outline)
  - [3.4 Hit-band coverage rule](#34-hit-band-coverage-rule)
- [4. Splitter Handles](#4-splitter-handles)
  - [4.1 Length modes](#41-length-modes)
  - [4.2 Thickness and end caps](#42-thickness-and-end-caps)
  - [4.3 Pointer states and colours](#43-pointer-states-and-colours)
- [5. Panel Effects: Shadow and Glow](#5-panel-effects-shadow-and-glow)
  - [5.1 Layering algorithm](#51-layering-algorithm)
- [6. Public API Sketch](#6-public-api-sketch)
- [7. Defaults and Theme Integration](#7-defaults-and-theme-integration)
- [8. Implementation Plan and Verification](#8-implementation-plan-and-verification)
  - [8.1 Risk register](#81-risk-register)
  - [8.2 Verification plan](#82-verification-plan)
- [9. Open Questions for Review](#9-open-questions-for-review)

---

## 1. Purpose and Scope

This specification defines a geometric styling layer for the dock component of gpui-kit, the Rust desktop UI framework by Longbridge built on GPUI. The layer adds eight visual capabilities that the current dock does not offer: rounded dock panels, an adjustable gap width between panels, a separately adjustable splitter handle width inside the gap, a contrasting gap colour, a panel outline with configurable colour and width, panel shadow and glow effects that live in the gap without covering neighbouring panels, shortened splitter handles that no longer stretch the full length of a panel edge, and customizable rounded splitter end caps. The document is a design review artifact: it fixes the mechanism, the public API, and the defaults before any code is written, so that implementation can proceed mechanically once the design is approved.

Everything proposed here was verified against the upstream master tree of longbridge/gpui-kit at version 0.8.0, which pins GPUI through the gpui-pre 0.3.9 package. The central finding is favourable: the dock already separates layout behavior from appearance through a renderer seam, and all eight requirements are expressible as appearance alone. Seven of the eight therefore require no upstream changes at all; the eighth (gap widths wider than the resize hit band) has an optional, small base enhancement that can ship separately. Table 1 maps each requirement to its mechanism and to the exact hook that will carry it.

**Table 1: Requirement mapping to renderer hooks. No hook signatures change.**

| ID | Requirement | Mechanism | Renderer hook | Upstream change |
|----|-------------|-----------|---------------|-----------------|
| F1 | Rounded corner dock panels | Inner clipper with corner radius | `TabGroupRenderer::frame` | None |
| F2 | Adjustable gap width between panels | Per-edge margins on group frames from split adjacency | `TabGroupRenderer::frame` | None (see 3.4) |
| F3 | Contrasting gap colour | `split_frame` background paint | `DockAreaRenderer::split_frame` | None |
| F4 | Separately adjustable splitter width | Visual thickness in the handle hook, hit band untouched | `DockAreaRenderer::render_split_handle` | None |
| F5 | Panel outline, adjustable colour and width | Border on the inner clipper | `TabGroupRenderer::frame` | None |
| F6 | Shadow / glow in the gap, never covering neighbours | BoxShadow layer beneath group chrome; sibling paint order occludes | `TabGroupRenderer::frame` | None |
| F7 | Shortened splitter handles | Length modes: pixels, percent, insets | `DockAreaRenderer::render_split_handle` | None |
| F8 | Rounded splitter ends, customizable | Cap preset: Flat or Rounded (auto radius = thickness / 2) | `DockAreaRenderer::render_split_handle` | None |

Three design principles govern the layer. First, styling is dock-level: one style object configures every panel and splitter in the dock, with a single, explicitly requested exception — the shadow and glow effect is adjustable both at dock level and per panel. Second, the layer is appearance-only: drag, drop, resizing, focus, serialization, and accessibility behavior stay exactly where they are, because the renderer seam guarantees that base attaches interaction to whatever elements the skin returns. Third, defaults reproduce today's look pixel for pixel, so the feature can merge without any visual regression and the styled look appears only when an application opts in.

Out of scope are changes to the layout tree, the drag-and-drop machinery, dock persistence formats, and the tab interaction model. The specification also does not attempt to restyle the tab bar's internal visuals (active tab colours, close buttons, overflow menus); the tab bar is merely clipped to the new rounded corners. These boundaries keep the change reviewable and make the styled skin a strict superset of the current one.

---

## 2. Verified Upstream Architecture

The gpui-kit workspace splits into three crates. gpui-base carries unstyled behavior and infrastructure; gpui-component carries the complete styled component system; the gpui-kit crate is the umbrella that applications depend on. The dock lives on both sides of that split: the layout engine (splits, tab groups, drag, persistence) is in `crates/base/src/dock`, and the production appearance is in `crates/component/src/dock`. The engine's documentation states the contract plainly: base supplies behavior, the host supplies appearance, and nothing in the engine module paints a colour, a border, or a size. Two unrelated appearances already exist over the same behavior — the component skin and a showcase skin under `crates/base/examples` — which proves the seam is real and exercised.

### 2.1 The renderer seam

Appearance is injected through two traits. `DockAreaRenderer` owns the area frame, each split container's frame, the divider between split slots, one dock's chrome, and the placeholder for unloadable panels; it also hands out the per-group renderer. `TabGroupRenderer` owns a tab group's outer frame, the content region below the tab bar, the tab bar itself, how the active panel view is placed, and the drop indicator. A renderer never sees drag events or mouse positions; base attaches drag sources, hit-testing, focus, and keyboard handling to the elements the renderer returns. Every hook is optional with a sensible default, and returning `None` from the divider hook falls back to base's own one-pixel line. Table 2 lists the hooks this design consumes.

**Table 2: Renderer hooks consumed by the geometric styling layer.**

| Hook | Receives | Role in this design |
|------|----------|---------------------|
| `DockAreaRenderer::frame` | window, cx | Optional outer-edge breathing inset around the whole area |
| `DockAreaRenderer::split_frame` | node id, axis | Paints the contrasting gap colour behind every slot of one split |
| `DockAreaRenderer::render_split_handle` | `ResizeHandleContext` | Paints the visual splitter: thickness, length, caps, state colours |
| `DockAreaRenderer::render_dock` | dock context, content | Unchanged; keeps dock chrome and the dock-edge resize handle |
| `TabGroupRenderer::frame` | `TabGroupContext` | Margins (gap), effect layer, inner rounded clipper, outline |
| `TabGroupRenderer::content_frame` | `TabGroupContext` | Unchanged; keeps drop handling and existing padding rules |
| `TabGroupRenderer::render_tab_bar` | `TabGroupContext` | Unchanged; inherits the top corner radius from the clipper |

The handle hook is the only one that needs detail here. It receives a `ResizeHandleContext` with three accessors: `axis()` (the axis along which resizing happens), `edge()` (`Some` for a handle hugging a dock edge, `None` for a handle straddling a boundary between two slots), and `state()`, one of `Idle`, `Hovered`, `Pressed`, `Dragging`. The hook returns an optional element; whatever it returns is placed inside the hit band, and the hit band itself — its width, its cursor, and its drag behavior — remains base's property. That split between painted appearance and interactive area is exactly what F4, F7, and F8 need.

### 2.2 Current splitter and gap geometry

Today's divider is deliberately minimal. Base positions each resize handle absolutely at minus four pixels from the leading edge of every panel after the first, so the handle straddles the seam between two slots. The hit band is a one-pixel line plus four pixels of transparent padding on each side, giving a nine-pixel-wide interactive strip centred on the seam. The production skin paints that strip as a one-pixel hairline in the theme border colour, and adds an animated pill indicator that grows with pointer engagement: 20 pixels at 0.35 opacity on hover, 28 pixels at 0.6 when pressed, 44 pixels at 0.9 while dragging, three pixels thick, fully rounded, and smoothed through the framework's Transition system. The gap colour that shows through the transparent padding comes from the split frame hook, which currently paints the theme's tab bar token; the panels themselves are adjacent, with no visual gap of their own.

> | **9 px** | **1 px** | **3 px** |
> |:---:|:---:|:---:|
> | resize hit band (1 px line + 4 px padding per side) | painted hairline in the theme border colour | hover pill thickness, 20 / 28 / 44 px by state |

Two consequences follow. Because panels are adjacent today, the only visible gap is the hairline itself, and the split frame's background is invisible in practice — it becomes meaningful only once panels are inset by margins, which is precisely what the gap feature does. And because the hit band straddles the seam symmetrically, any visual gap wider than nine pixels would leave inert strips at its outer edges unless the band grows with it; section 3.4 turns that observation into an explicit coverage rule and a phased plan.

### 2.3 GPUI primitives available

The pinned GPUI version exposes every primitive the design needs, and the component crate already uses them in production paths, which removes API-availability risk:

- **Box shadows:** `Styled::shadow` accepts a `Vec` of `BoxShadow` values with colour, x and y offset, blur radius, and spread radius; dialogs, popovers, and toasts compose multi-layer shadow stacks this way. Shadow and glow are therefore the same mechanism with different parameters, and both can be active simultaneously by concatenation.
- **Per-corner and per-edge styling:** corner radii are a `Corners` value (`rounded`, `rounded_tl`, and friends) and border widths are per-edge (`border_l_1`, `border_b_1`). Rounded bodies with square-adjacent chrome, or later per-corner radii, need no new plumbing.
- **Deferred painting:** `deferred()` paints an element after the rest of the tree, under the window mask. The handle pill already uses it so a hugging edge handle's overhang survives the dock's clip; the splitter design reuses the technique only where the existing code does.
- **Relative sizing and transitions:** proportional child sizes support percent-length handles directly, and the `Transition` helper animates values between frames with theme-provided duration and easing, which the pill already demonstrates.

---

## 3. Geometry Model: Gaps, Radii, Outlines

This chapter fixes the geometric vocabulary of the feature: how wide the gap is and where it lives, how panel rounding is scoped to body and tab bar, how the outline attaches, and how the visual gap relates to the interactive hit band. The recurring idea is that the renderer seam lets the skin reshape what is visible inside each slot without touching the slot algebra that base controls — margins, radii, and borders all live on elements the skin itself returns.

### 3.1 Gap width and gap colour

The gap is created by insetting each tab group inside its own slot. A tab group's outer frame is laid out by base as a column that fills the slot around whatever the renderer returns, so margins on that frame shrink the group's visible box within the slot and leave the freed strip transparent. Each group can discover its position in the layout tree through its `TabGroupContext` node identifier, and the skin holds a shared handle to the dock area, so the skin computes adjacency at render time: for an interior boundary of a horizontal split, the left group takes a trailing margin of `gap / 2` and the right group takes a leading margin of `gap / 2`. The freed strip, plus the nine-pixel handle band between slots, is then uniformly backed by the split frame hook, which paints the user's gap colour. The visible gap between two panels is therefore exactly the configured width, centred on the seam where the splitter handle sits.

```
   panel A slot                handle band (base-owned)              panel B slot
 <------------------------> <-------------------------------------> <------------------------>

 +-----------------..........................................................+-----------------+
 |   +-----------+          m = gap / 2            m = gap / 2               +-----------+   |
 |   |           |                                                          |           |   |
 |   |  panel A   | | 1 px hairline (default) or user splitter visual |     |  panel B   |  |
 |   |  (styled)  |          ^ hit band: 9 px wide, centred on seam ^       |  (styled)  |  |
 |   +-----------+                                                          +-----------+   |
 |_________________.......................... ............................._________________|

        m = gap / 2 margin on the interior edges of each group frame;
        the whole strip between the two visible boxes is painted by
        DockAreaRenderer::split_frame with the user's gap colour.
```

*Figure 1: Cross-section of one interior boundary in a horizontal split.*

The colour itself is a single configurable value on the split frame background. When the application leaves it unset, the hook keeps today's theme token, so the gap recedes into the chrome; when set, the same hue is applied behind every seam of the split, giving the contrasting-channel effect panels appear to float in. Because the colour lives on the split container rather than on individual panels, nested splits compose naturally: each split paints its own channel, and outer splits show through wherever inner margins are absent.

### 3.2 Corner radius over body and tab bar

Rounding applies to the whole group — the content body and the tab bar together — which is the scope agreed for this feature. The group frame is restructured into two nested elements, shown in Figure 2. The outer frame remains unclipped and hosts the effect layer described in chapter 5. The inner clipper carries the corner radius, an overflow clip, and the optional outline; it wraps the background, the tab bar, and the content region. Because the tab bar lives inside the clipper, its top corners follow the radius automatically, and because the clip is on the skin's own element rather than on a base slot, no upstream clipping behavior changes.

```
 TabGroupRenderer::frame(group)            unclipped outer frame
 |
 +-- effect layer (absolute, negative inset, painted first)
 |    BoxShadow vector: shadow, glow, or both concatenated
 |
 +-- inner clipper (rounded r, overflow_hidden, outline border)
      |
      +-- background fill (theme background or user override)
      +-- tab bar / title bar            inherits the top corner radius
      +-- content_frame (drop handling)  existing padding rules unchanged
           +-- active panel view
```

*Figure 2: Group frame structure after the change.*

The two-element structure exists for a reason that will matter in chapter 5: rounding wants a clip, while the effect layer must paint outside the visible box and must not be clipped. Separating them means the same frame can host both a rounded, outlined panel and an unclipped glow that reaches into the surrounding gap. A uniform radius value is exposed now; per-corner radii are a natural extension via GPUI's `Corners` type and are listed as an open question rather than committed API.

### 3.3 Panel outline

The outline is a border drawn on the inner clipper, with configurable colour and width. Drawing it on the clipper rather than the outer frame guarantees the outline follows the rounded silhouette exactly, including the clipped tab bar corners. GPUI's per-edge border widths remain available if a later revision wants, for example, a heavier bottom edge under the tab bar; the committed API is the simple uniform case (colour plus width), with `None` meaning no outline, which is today's behavior. Interaction with the gap is automatic: the outline is painted inside the visible box, so a contrasting gap colour meets a crisp panel edge rather than bleeding into it.

### 3.4 Hit-band coverage rule

The interactive band between two slots is nine pixels wide and stays where base puts it. The rule that follows: a visual gap of nine pixels or less is fully covered by the hit band, so every pixel of the gap starts a resize drag; a wider gap leaves inert strips at its outer edges where the pointer neither grabs nor drags. The recommended default is an eight-pixel gap, which keeps full coverage while producing a clearly visible channel. For applications that want wider channels, a small follow-up enhancement in base — exposing the handle's hit padding as a builder parameter instead of the current crate-private constant — lets the band grow to match; this is Phase 2 in chapter 8 and is the only point in the whole design where an upstream edit is worth discussing. Nothing in the skin-side implementation blocks on it.

---
## 4. Splitter Handles

The splitter handle keeps its interaction contract and changes only its paint. The hook receives the axis (horizontal for a vertical divider between side-by-side panels), the edge (`None` for handles straddling a seam, `Some` for handles hugging a dock edge), and the pointer state. It returns an element that base places inside the nine-pixel hit band; the cursor, the drag threshold, and the resize behavior are base's and are untouched. Everything in this chapter is therefore expressible inside `DockAreaRenderer::render_split_handle` with the same techniques the existing pill indicator already uses — overhang by negative margins, deferred painting for hugging-edge handles, and state-driven sizing.

### 4.1 Length modes

The visual length of the handle is decoupled from the length of the panel edge. Three modes cover the requested customization space, selected by one enum value. **Pixels** gives a fixed-length handle centred on the seam, independent of panel size, which is the classic grabber look. **Percent** gives a handle proportional to the split's current edge length, which scales gracefully across window resizes. **Insets** positions the handle by its distance from the two ends of the edge, which lets an application pin handles toward a corner or keep them clear of content that hugs the panel edge. The wrapper element always spans the full edge (today's behavior), and the visual is a child of that wrapper positioned by the mode, so centering logic never depends on measured pixel lengths for the Pixels and Percent cases.

```
             full edge (h_full wrapper centred on the seam)
 <------------------------------------------------------------->

 Pixels(64):      ................#######................
 Percent(0.40):   .......###############.......                (0.4 x edge, centred)
 Insets(16,48):   ..##########################.....            (16 from lead, 48 from trail)

                  ^ thickness t spans the gap across the seam; the
                    hit band underneath stays 9 px whatever the mode.
```

*Figure 3: Splitter length modes drawn along one panel edge.*

Two implementation notes keep the modes robust. Percent length is expressed as a relative size of the full-length wrapper, which GPUI resolves during layout without the skin measuring anything; if a future GPUI revision restricts relative sizing on the relevant axis, the documented `on_prepaint` pattern for reading painted group bounds provides the fallback measurement path. Insets mode is implemented as leading and trailing spacers inside the wrapper, which compose with any axis and need no measurement at all. A minimum visual length equal to the thickness is enforced so a degenerate configuration cannot render an invisible handle.

### 4.2 Thickness and end caps

The handle's visual thickness across the gap is independent of the one-pixel line the hit band paints. A thicker visual is produced exactly the way the existing pill overhangs the hairline: the visual element is centred on the seam and pulled out sideways by negative margins of half the difference between the desired thickness and the underlying pixel. For handles hugging a dock edge, where the hairline is the container's outermost pixel, the overhanging visual is deferred so it survives the dock's clip — reusing today's pill mechanism unchanged. The hit band and cursor are unaffected by thickness, which is what the separately adjustable requirement asks for.

End caps have exactly two modes, as requested. **Flat** draws square ends (corner radius zero). **Rounded** derives the corner radius automatically from the stroke width: radius equals thickness divided by two, which turns the handle into a perfect pill whose rounding always matches its width no matter how thick the user makes it. Deriving rather than exposing a raw radius is deliberate — it removes a whole class of mismatched configurations where the cap radius exceeds the handle width — while the thickness knob still gives full control over how pronounced the rounding looks. If a future need for per-end radii arises, the enum extends without breaking the two existing modes.

```
   Flat cap (radius 0)                    Rounded cap (radius = thickness / 2)

   ######                                     .######.
   ######        square ends,                 ########        half-circle ends,
   ######        any thickness                ########        auto-fitted pill
   ######                                     ######

   thickness t (across the gap), length per SplitterLength, centred on the seam
```

*Figure 4: The two cap presets. Rounded always uses radius t / 2.*

### 4.3 Pointer states and colours

The style exposes one colour per pointer state — idle, hovered, pressed, dragging — matching the states base already reports. Any colour left unset falls back to the current theme-driven behavior, and the defaults reproduce today's appearance exactly: a full-length one-pixel hairline in the theme border colour when idle, and the growing pill (20, 28, 44 pixels at 0.35, 0.6, 0.9 opacity) on hover, press, and drag. Because the pill already animates through the Transition system, the new visual can adopt the same easing for length, thickness, and colour transitions; whether to animate by default is listed in the open questions, with reuse of the existing system as the recommendation. Length and thickness settings apply independently of state, so a short rounded handle can still brighten while dragged.

The hit-area invariant is worth stating once, plainly: **no splitter setting in this chapter changes what the pointer can grab.** Thickness, length, and caps are paint; the band is behavior. This preserves the muscle memory of the current dock, keeps the automated drag tests in base meaningful, and means a garish or minimal splitter style can never break resizing.

---

## 5. Panel Effects: Shadow and Glow

Effects give panels presence in the gap: a drop shadow that grounds a floating panel, or a glow that tints the channel around it. Both effects are box shadow layers — a shadow is a dark, offset, soft layer, a glow is a saturated colour with zero offset and a wide blur — so both compile to the same GPUI primitive and are independently settable. Both may be active at the same time; the combined shadow vector is simply the shadow layer followed by the glow layer, which is how the framework's own dialog styling composes multiple shadows. Parameters are the natural ones: colour, offset, blur radius, and spread radius for the shadow; colour, blur radius, and spread radius for the glow.

Scope of adjustment follows the agreed exception to dock-level styling: the effect is configured on the dock's style object as the default for every panel, and an application may override it per panel through a registry keyed by panel identity exposed on the dock (a suggested form is a `set_panel_effect` method; see the open questions). Everything else in the geometry style remains dock-level. The override exists because mixed content genuinely wants different presence — a video panel may want a deep shadow while a palette panel wants a soft glow — and because the occlusion mechanism below is per panel, so overrides compose safely.

### 5.1 Layering algorithm

The guarantee that an effect colours the gap but never covers a neighbouring panel comes from paint order, not from clipping. GPUI paints siblings in tree order, so whatever a group paints before its later siblings is covered by them where they draw. The design exploits this with a three-stage order inside each split container, summarized in Figure 5.

```
  paint order inside one split container (bottom to top)
  -----------------------------------------------------------
  3. deferred overhangs (hugging dock-edge handles only)
  2. group chrome, in tree order:   [ group 1 ][ group 2 ][ group 3 ]
        each group paints:  2a. effect layer (unclipped, absolute)
                            2b. inner clipper: bg, tab bar, panel
  1. split_frame background = the gap colour
  -----------------------------------------------------------
  group 2 paints AFTER group 1's effect layer, so any part of
  that layer reaching into group 2's slot is overpainted by
  group 2's own chrome. The effect stays visible only in the
  gap - which is the requested behavior.
```

*Figure 5: Paint order that makes effects gap-safe.*

The effect layer itself is an absolutely positioned child of the unclipped outer frame, inset negatively by the intended spread, painted before the inner clipper. Slots in the resizable group do not clip their children by default — base explicitly warns against adding a clip because it would cut the straddling resize handle — so the layer can reach from the visible box into the margins and the handle band. Any part of the layer that reaches past the gap into a neighbour's visible box is overpainted when that neighbour paints its own chrome later in tree order. The split frame's existing overflow clip confines effects to the split container, so nothing escapes the dock area at its outer boundary. Occlusion is thus structural: no mask bookkeeping, no neighbour-aware geometry, and no way for a style value to violate the guarantee.

**Table 3: Where an effect layer is visible, by zone.**

| Zone | Effect visibility | Why |
|------|-------------------|-----|
| Own margins (inside own slot) | Visible | Layer is unclipped within the slot; nothing repaints it |
| Handle band between slots | Visible | Part of the gap; painted before handle visuals |
| Neighbour's visible box | Occluded | Neighbour paints its chrome later in tree order |
| Outside the split container | Clipped | `split_frame` overflow_hidden bounds the effect |

Two aesthetic caveats complete the picture. Adjacent glows blend in the shared gap through ordinary alpha compositing, which reads as a brighter channel between two glowing panels and is normally the desired effect; applications that dislike it can lower glow alpha or set effects per panel. And because a group paints before its successors, an effect never tints a neighbour's margins after the fact — any tint in a neighbour's margin region comes from the neighbour's own earlier-painted glow, which is again the gap doing its job. Recommended defaults keep the shadow spread within the gap width so the visual weight sits in the channel rather than under neighbouring chrome.

---

## 6. Public API Sketch

The API is one style object with nested sub-styles, constructed with builder methods and handed to the dock once. It lives in a new geometry module of the component crate's dock; nothing in gpui-base changes for the committed scope. Field defaults reproduce the current look, so a `Dock` constructed without a style is visually identical to today's. The sketch below shows the full value surface; naming may shift slightly during review but the shapes are the commitment.

```rust
// crates/component/src/dock/geometry.rs (new module)

pub struct DockGeometryStyle {
    pub gap: GapStyle,
    pub panel: PanelGeometryStyle,
    pub splitter: SplitterStyle,
}

pub struct GapStyle {
    /// Visual gap between neighbouring panels, centred on each seam.
    pub width: Pixels,                 // default px(0.) -> today's look
    /// Background behind the gap; painted by split_frame.
    pub background: Option<Hsla>,      // None -> theme tokens.tab_bar
    /// Breathing inset around the outer edge of the dock area.
    pub outer: Pixels,                 // default px(0.)
}

pub struct PanelGeometryStyle {
    /// Body and tab bar share this radius.
    pub corner_radius: Pixels,         // default px(0.)
    pub outline: Option<OutlineStyle>, // default None
    /// Shadow and glow are independent; both may be active at once.
    pub effect: PanelEffect,           // defaults: both None
}

pub struct OutlineStyle { pub color: Hsla, pub width: Pixels }

pub struct PanelEffect {
    pub shadow: Option<ShadowSpec>,    // dark, offset, soft
    pub glow: Option<GlowSpec>,        // saturated, centred, wide
}

pub struct ShadowSpec {
    pub color: Hsla,
    pub offset: Point,                 // (dx, dy)
    pub blur_radius: Pixels,
    pub spread_radius: Pixels,
}

pub struct GlowSpec {
    pub color: Hsla,
    pub blur_radius: Pixels,
    pub spread_radius: Pixels,
}

pub struct SplitterStyle {
    /// Visual thickness across the gap; hit band stays base-owned.
    pub thickness: Pixels,             // default px(1.) -> today's hairline
    pub length: SplitterLength,        // default Percent(1.0) -> full edge
    pub cap: SplitterCap,              // default Flat
    pub colors: SplitterColors,        // per-state; None -> theme
}

pub enum SplitterLength {
    Pixels(u32),
    Percent(f32),
    Insets { start: Pixels, end: Pixels },
}

pub enum SplitterCap { Flat, Rounded }  // Rounded -> radius = thickness / 2

pub struct SplitterColors {
    pub idle: Option<Hsla>,
    pub hovered: Option<Hsla>,
    pub pressed: Option<Hsla>,
    pub dragging: Option<Hsla>,
}
```

*Listing 1: The geometry style value surface.*

Usage at the application level is a single construction site, shown in Listing 2. The style object is cheap to build and immutable once handed to the dock; changing it at runtime re-renders through the normal invalidation path because the skin reads it during element construction. A convenience constructor per sub-style keeps the builder chain short for the common cases — gap plus radius plus glow is the flagship configuration the feature exists for.

```rust
use gpui_component::dock::{Dock, DockGeometryStyle, SplitterCap, SplitterLength};

let style = DockGeometryStyle::new()
    .gap(px(8.))
    .gap_background(cx.theme().surface.darkened(0.4))
    .panel_radius(px(10.))
    .outline(OutlineStyle { color: cx.theme().border, width: px(1.) })
    .shadow(ShadowSpec {
        color: black().alpha(0.35),
        offset: point(px(0.), px(3.)),
        blur_radius: px(12.),
        spread_radius: px(-2.),
    })
    .glow(GlowSpec {
        color: cx.theme().accent.alpha(0.30),
        blur_radius: px(22.),
        spread_radius: px(0.),
    })
    .splitter(|s| s
        .thickness(px(6.))
        .length(SplitterLength::Percent(0.45))
        .cap(SplitterCap::Rounded));

let dock = Dock::new("main", window, cx).with_geometry_style(style);
```

*Listing 2: Wiring the styled look into an application.*

**Table 4: Every style field lands on an existing hook; no signatures change.**

| Style field | Renderer hook | Element that carries it |
|-------------|---------------|--------------------------|
| `gap.width` | `TabGroupRenderer::frame` | Per-edge margins computed from split adjacency |
| `gap.background` | `DockAreaRenderer::split_frame` | Split wrapper background |
| `gap.outer` | `TabGroupRenderer::frame` | Margins on edge-facing sides |
| `panel.corner_radius` | `TabGroupRenderer::frame` | Inner clipper `rounded()` |
| `panel.outline` | `TabGroupRenderer::frame` | Border colour and width on clipper |
| `panel.effect.*` | `TabGroupRenderer::frame` | Absolute BoxShadow layer, painted first |
| `splitter.thickness` | `DockAreaRenderer::render_split_handle` | Overhang visual centred on the seam |
| `splitter.length` | `DockAreaRenderer::render_split_handle` | Child position inside the full-length wrapper |
| `splitter.cap` | `DockAreaRenderer::render_split_handle` | Corner radius of the visual (0 or t / 2) |
| `splitter.colors` | `DockAreaRenderer::render_split_handle` | Fill per `ResizeHandleState` |

---

## 7. Defaults and Theme Integration

Every colour field is an `Option` that resolves to the active theme when unset, matching the component crate's existing token pattern: unset gap background keeps the tab bar token the split frame paints today, unset splitter colours keep the border hairline and muted pill, unset outline means no border, and radius fields default to zero rather than adopting the theme's general radius, because rounding panels is precisely the opt-in this feature sells. Explicit values always win over tokens, and no style value is ever persisted: the layout tree and dock state that the dock serializes contain no appearance, so a saved layout restores identically under any style, and switching themes re-derives all unset colours on the next frame.

**Table 5: Defaults preserve today's appearance; the styled column shows the flagship configuration.**

| Parameter | Default (current look) | Styled example |
|-----------|------------------------|----------------|
| `gap.width` | 0 (panels adjacent) | 8 px |
| `gap.background` | theme tab bar token | darkened surface tone |
| `gap.outer` | 0 | 0 to 8 px |
| `panel.corner_radius` | 0 (square) | 10 px |
| `panel.outline` | None | 1 px theme border |
| `panel.effect.shadow` | None | y +3, blur 12, alpha 0.35 |
| `panel.effect.glow` | None | accent, blur 22, alpha 0.30 |
| `splitter.thickness` | 1 px hairline | 6 px |
| `splitter.length` | `Percent(1.0)` full edge | `Percent(0.45)` |
| `splitter.cap` | Flat | Rounded (radius 3 px) |
| resize hit band | 9 px, base-owned | 9 px, unchanged |

The backward-compatibility property is structural rather than incidental. Because margins, radius, outline, effects, and splitter paint all default to values that collapse the new structure into the old one, the restructured group frame renders the same pixels as the current frame when no style is set. That makes the change safe to review against the existing visual test suite, and it means applications that never touch the API see nothing but the same dock they ship today.

---

## 8. Implementation Plan and Verification

The work splits into two phases with a hard boundary: Phase 1 is skin-only and touches nothing in gpui-base, while Phase 2 is an optional, separately reviewable base enhancement that only applications wanting gaps wider than nine pixels need. Phase 1 is itself vertically sliceable — gap colour and margins first, then radius and outline, then effects, then splitter modes — so review can land incrementally without ever breaking the default look.

**Table 6: Phased plan. Phase 1 is entirely inside the component crate.**

| Phase | Scope | Files | Upstream change |
|-------|-------|-------|-----------------|
| P1a | Gap width and gap colour | New `crates/component/src/dock/geometry.rs`; wire `split_frame` and group frame margins in `dock.rs` and `tab_panel.rs`; adjacency query via the skin's shared area handle | None |
| P1b | Corner radius, outline | Group frame restructure: unclipped outer frame + inner clipper in `tab_panel.rs` | None |
| P1c | Shadow and glow, per-panel override | Effect layer in `tab_panel.rs`; override registry and setter on the dock | None |
| P1d | Splitter thickness, length modes, caps, state colours | `render_split_handle` in `dock.rs` reworked on the existing overhang and deferred techniques; styled showcase in `examples/dock` | None |
| P2 | Optional: hit-band growth for gaps wider than 9 px | Expose handle hit padding as a builder parameter in `crates/base/src/resizable` (today a crate-private constant) | Small, additive |

### 8.1 Risk register

**Table 7: Risks and their mitigations.**

| Risk | Impact | Mitigation |
|------|--------|------------|
| Percent length needs the measured edge length | Handle shorter or longer than intended | Relative sizing resolves in layout; fallback is the documented `on_prepaint` bounds pattern already referenced by the frame hook |
| A future base change clips slots | Effect layer would cut at slot bounds | Keep spread within own margins; add a paint-order test that fails loudly if slots gain clips |
| Deferred overhang paints above popovers | Handle pixel overlays floating UI | Deferred is used only for hugging dock-edge handles, exactly where today's pill already uses it |
| Gap wider than hit band leaves dead strips | Outer gap edges not draggable | Default gap 8 px stays under the 9 px band; Phase 2 exposes the padding knob for larger gaps |
| Theme coupling of unset colours | Style drifts when tokens change | Unset fields read tokens at render time; explicit values always win |

### 8.2 Verification plan

Verification follows the agreed compile-first bar, extended with the dock's own headless UI test harness. The gates, in order: `cargo check` for the component crate and the dock example, which proves the fork compiles against gpui-pre 0.3.9; the existing base and component dock test suites, which must pass unchanged because default styling collapses to today's look; new gpui test cases using the visual test context that assert each feature mechanically — group frames carry the computed margins, the handle element has the configured thickness and length, the effect layer precedes the clipper in paint order, and defaults produce zero margins and no effect layer; and a manual smoke run of the styled showcase in `examples/dock` on a desktop for the subjective checks the tests cannot make. Linux build dependencies (X11 and Wayland client libraries and the Vulkan loader) are environment concerns only for running windows; the compile gate and the headless tests do not require a display.

---

## 9. Open Questions for Review

Six decisions are left open deliberately, each with a recommendation so review can be a matter of agreeing or overriding rather than inventing. They are ordered by how much they shape the committed API.

1. **Per-corner radii now or later?** The committed API is a uniform radius; GPUI's `Corners` type makes the extension additive. *Recommendation:* uniform now, per-corner as a follow-up once the flagship look is validated.
2. **Should splitter state changes animate through the existing Transition system?** The pill already animates, and the infrastructure is theme-configurable. *Recommendation:* yes, with the same fast duration the pill uses.
3. **Should the dock-edge resize handles (the ones hugging left, right, and bottom docks) adopt the same thickness and cap styling as seam splitters?** Consistency argues yes. *Recommendation:* yes, driven by the same `SplitterStyle`.
4. **Where do per-panel effect overrides live: a new method on the Panel trait, or a registry on the dock keyed by panel identity?** The registry keeps the trait untouched and works for panels the application does not own. *Recommendation:* registry, exposed as a setter on the dock.
5. **Gap and hit-band policy: clamp the accepted gap to nine pixels until Phase 2 lands, or accept any value and document the dead strips?** *Recommendation:* default eight pixels, accept larger values with a documentation note, ship Phase 2 as an immediate follow-up.
6. **Should the outer edge of the dock area get a breathing inset by default (`gap.outer`), or default to zero so the dock hugs the window as it does today?** *Recommendation:* default zero, opt-in per application.
