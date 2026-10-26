# gpui-kit 0.8 feasibility check — geometric dock styling

**Question:** how much of `gpui-kit-geometric-dock-styling-spec.md` can be built
against the API gpui-kit 0.8 actually ships?

**Baseline checked:** `gpui-kit 0.8.0`, `gpui-base 0.8.0`,
`gpui-component 0.8.0`, `gpui-pre 0.3.9` (the vendored sources under
`~/.cargo/registry/src/...`), cargo/rustc 1.99.0, 2026-10-26.

**Method:** read the dock, resizable and painting paths in the pinned sources,
then ran a headless-ish layout probe to check the one assumption the whole gap
design rests on (margins shrinking a `size_full()` group inside a resizable
panel). Source references below are crate-relative.

The working record — chronology, dead ends, confidence levels and reproduction
steps — is in [`gpui-kit-0.8-evaluation.md`](gpui-kit-0.8-evaluation.md).

**Short answer:**

- **The capabilities are mostly buildable**, but only as an **application-side
  skin**, not as the component-crate API the spec proposes.
- **5 of 8 requirements are exact** with a custom `DockAreaRenderer` +
  `TabGroupRenderer` (F1, F3, F4, F5, F8).
- **F7 is proven for fixed pixel lengths**; percent and insets are expressible
  but unexercised by 0.8's own skin and need a short spike.
- **F6 works, but not as specified:** effects must be the group frame's own
  `BoxShadow`; there is no unclipped "effect layer" available.
- **F2 is half-blocked:** horizontal gaps work today; vertical gaps do not,
  because `ResizablePanel` is unconditionally a row. A very small base change
  fixes it.
- **The public API in chapter 6 does not exist at all** — no `Dock`, no
  `DockGeometryStyle`, no `with_geometry_style`. `Dock` is deliberately not
  re-exported by `gpui-component`.
- **The hit-band growth (Phase 2) is confirmed necessary**; the constants are
  `pub(crate)`.

---

## 1. Verdict by requirement

| ID | Requirement | Status against 0.8 | Mechanism actually available |
|----|-------------|--------------------|------------------------------|
| F1 | Rounded dock panels | **Yes** | `.rounded(r)` on `TabGroupRenderer::frame`; base already applies `overflow_hidden`, so the tab bar and content clip |
| F2 | Adjustable gap width | **Horizontal only** | `mx(gap/2)` on the group frame shrinks (main axis). `my(...)` overflows (cross axis). Needs a one-line base fix for vertical splits |
| F3 | Contrasting gap colour | **Yes** | `DockAreaRenderer::split_frame` background |
| F4 | Separately adjustable splitter width | **Yes** | Custom appearance child with negative margin overhang, exactly as the stock pill does |
| F5 | Panel outline, colour + width | **Yes** | Border on the group frame; painted after children and follows the radius |
| F6 | Shadow / glow in the gap | **Yes, reduced form** | `Styled::shadow(vec![...])` on the group frame. Gap-safety comes from sibling paint order, but there is no separate layer and the granularity is the group |
| F7 | Shortened splitters (pixels / percent / insets) | **Yes, with a spike** | Fixed pixels are proven; percent/insets are expressible inside the full-length band, but 0.8's own skin only exercises fixed lengths |
| F8 | Rounded splitter caps | **Yes** | Radius computed by the skin as `thickness / 2` |
| — | Hit band wider than 9 px | **No** | `HANDLE_SIZE` and `HANDLE_PADDING` are `pub(crate)`; Phase 2 is real |
| — | Per-panel effect overrides | **Group granularity** | A registry in the skin can key by active `PanelId`, but the styled surface is the group frame |
| — | Chapter 6 public API | **None of it exists** | `DockGeometryStyle`, `OutlineStyle`, `SplitterLength`, `SplitterCap`, `with_geometry_style` are all new |

If the spec's exact architecture is the acceptance criterion, the count is
**0 of 8 hooks changed / 0 of the proposed types present**. If the acceptance
criterion is the visual result, it is **5 exact + 1 mostly + 1 reduced + 1
half**, with one tiny base patch needed for the vertical half of F2.

---

## 2. Claims in the spec that check out

These are accurate against 0.8 and should not be re-litigated:

- The renderer seam is real and is the only styling hook: `DockAreaRenderer`
  (`gpui-base/src/dock/dock_area.rs:1911-2004`) and `TabGroupRenderer`
  (`gpui-base/src/dock/tab_group.rs:902-963`).
- The handle hook receives axis, edge and state, and nothing else:
  `ResizeHandleContext` (`gpui-base/src/resizable/resize_handle.rs:61-93`).
  The four states and the `is_active` helper are as described
  (`resize_handle.rs:101-119`).
- The hit band is a 1 px line plus 4 px of padding per side (9 px total), and
  the painted appearance is a child inside it:
  `resize_handle.rs:12-13, 275-388`.
- The stock divider is a 1 px border-coloured line plus a 3 px pill that grows
  to 20 / 28 / 44 px at 0.35 / 0.6 / 0.9 opacity:
  `gpui-component/src/resizable.rs:60-151`.
- `split_frame` is an ordinary wrapper around the resizable group, so painting
  a background there does put colour behind every panel:
  `dock_area.rs:1436-1450`, `gpui-component/src/dock/dock.rs:61-67`.
- Base applies the column, the fill and the clip around the group frame, and
  the skin cannot accidentally break that:
  `gpui-base/src/dock/tab_group.rs:691-707`.
- Defaults (zero radius, no outline, no effect, zero margin, 1 px hairline)
  reproduce today's pixels; nothing needs persisting.
- The coverage rule is correct: a gap wider than 9 px leaves inert strips
  because the band is fixed.

---

## 3. The three structural mismatches

### 3.1 There is no unclipped outer frame / inner clipper split

The spec (section 3.2, Figure 2; also 5.1 and P1b/P1c) assumes the renderer
can return an unclipped outer frame that hosts an effect layer, and put the tab
bar and content inside an inner clipper.

0.8 does the opposite: it takes the single element the hook returns, applies
the structure to **that element**, and appends the tab bar and content to it:

```rust
renderer
    .frame(&context, window, cx)   // the skin's element
    .flex().flex_col().size_full().overflow_hidden()
    .child(renderer.render_tab_bar(...))
    .child(renderer.content_frame(...))
```

Source: `gpui-base/src/dock/tab_group.rs:691-755`. The stock skin says the same
in a comment ("The column, the fill and the clip are base's now, applied around
this", `gpui-component/src/dock/tab_panel.rs:709-710`).

Consequences:

- **F1 and F5 still work**, because radius and border belong on exactly the
  element base clips.
- **The effect layer cannot exist.** A child of the frame is clipped by the
  frame's own `overflow_hidden`; an absolutely positioned layer painted before
  the clipper has nowhere to live. Use `Styled::shadow` on the frame itself.
- **F6's guarantee still holds, by a different route.** GPUI paints drop
  shadows before the background and outside the element's own content mask
  (`gpui-pre/src/style.rs:711-742`; the mask wraps only the children in
  `gpui-pre/src/elements/div.rs:2545-2547`), and later siblings paint over it.
  So a shadow reaches into the gap, is clipped by `split_frame` at the split
  boundary, and is overpainted by a neighbour's chrome. That is the requested
  behaviour, without a dedicated layer.
- **The effect is per group, not per panel.** The frame belongs to a tab group;
  multiple tabs share one frame. A registry keyed by the active panel id is the
  closest equivalent, and it is owned by the skin, not by the dock.
- The spec's risk register should gain: *"base owns the frame's clip; an
  unclipped effect layer requires a base change."*

### 3.2 Vertical gaps do not work through margins

This is the one that needs a code change, and it is not in the spec's risk
register.

The gap mechanism assumes margins on the group frame shrink its visible box.
That is true on the **main axis** and false on the **cross axis**:

- `ResizablePanel` is always `.flex()` — a row — regardless of the split axis
  (`gpui-base/src/resizable/panel.rs:338-343`).
- Base applies `.size_full()` to the group frame **after** the hook returns
  (`tab_group.rs:703-706`), so the skin cannot opt out of `height: 100%`.
- In a row, a child with `width: 100%` and horizontal margins is shrunk by
  flex-shrink (default 1.0, `gpui-pre/src/style.rs:802`): available space minus
  margins. A child with `height: 100%` and vertical margins is on the cross
  axis, where flex-shrink does not apply: it keeps 100% height and overflows.
- `ResizablePanel` deliberately has no `overflow_hidden`
  (`panel.rs:236-241` warns callers not to add one, because it would clip the
  straddling handle), so the overflow is visible rather than clipped.

Probe result (temporary example, since removed; numbers are real):

```
panel-h   origin=(0.0,0.0)   size=(400.0,200.0)
group-h   origin=(4.0,0.0)   size=(392.0,200.0)   # mx(4): 8 px gap, works
panel-v   origin=(0.0,200.0) size=(400.0,200.0)
group-v   origin=(0.0,204.0) size=(400.0,200.0)   # my(4): bottom at 404, overflow
```

So an `h_split` gets real gaps; a `v_split` (including splits inside the
bottom dock) gets a shifted, overflowing panel and **no gap at all**.

There is no reliable skin-only workaround. `size_full()` is forced, absolute
positioning conflicts with it, and a measured `max_h` + margin hack has no
stable source for the slot height across window resizes. This is a base fix.

**Smallest fix:** make the panel's flex direction follow its axis, i.e. a
column for `Axis::Vertical`, so the group's height becomes the main axis and
shrinks exactly like width does today:

```rust
// gpui-base/src/resizable/panel.rs:338
div()
    .flex()
    .map(|this| if self.axis.is_vertical() { this.flex_col() } else { this })
    .flex_grow_1()
    ...
```

The resize handle is absolutely positioned, so it is unaffected. The stock
uses of `resizable_panel` should be exercised in both axes before shipping.
With that one change, the spec's margin model works vertically and F2 becomes
fully implementable.

The vertical branch was verified in a synthetic panel (`.flex().flex_col()`
instead of `.flex()`) with the same probe:

```
panel-v-col   origin=(0.0,200.0) size=(400.0,200.0)
group-v-col   origin=(0.0,204.0) size=(400.0,192.0)   # bottom 396, gap works
```

The remaining work is applying the conditional direction in the real
`gpui-base` tree and regression-testing the non-dock uses of
`resizable_panel`.

### 3.3 Adjacency is not in the context

`TabGroupContext` exposes the node id, panels, active index, zoom, closability,
drop indicator and callbacks (`tab_group.rs:768-892`). It does **not** expose
the parent split, the child index, the split axis, or which sides are interior.

The spec's claim that the skin "holds a shared handle to the dock area" is true
of the *stock* `DockSkin` — it stores `cx.weak_entity()`
(`gpui-component/src/dock/mod.rs:214-225`) — but `shared()` is `pub(crate)`,
so an application cannot reach it. An app-side skin has to repeat the
bootstrap dance itself (create the renderer with a weak entity, then install it
with `with_renderer`).

Adjacency *is* derivable from public API:

- `DockArea::layout(placement) -> Option<&PaneTree>`
  (`dock_area.rs:189-194`).
- `PaneTree::{root, find_node, node_ids, panels}`
  (`gpui-base/src/dock/layout/tree.rs:64-105`).
- `PaneNode::{id, kind, walk}`, with `PaneRef::Split { axis, children, .. }`
  (`gpui-base/src/dock/layout/node.rs:84-118`).

So the app can walk the tree, find the group's `node()`, and decide which
sides are interior. This is work the spec pushes into the component crate; at
0.8 it is per-application boilerplate. A small upstream addition — parent,
axis and index on `TabGroupContext` — would remove it.

Note also that `gap.outer` is better implemented as **padding on the area
frame** (`DockAreaRenderer::frame`), not as margins on edge groups: a margin on
the outer side of a dock's group would reveal the split frame's gap colour, not
the window. Base already applies `size_full`, `flex_row` and `overflow_hidden`
to the area frame (`dock_area.rs:1547-1560`), so padding shrinks the content
box without fighting the layout.

---

## 4. What each achievable piece looks like in a 0.8 skin

### F1 radius, F5 outline

```rust
fn frame(&self, group: &TabGroupContext, _: &mut Window, cx: &mut App) -> Stateful<Div> {
    let s = &self.style.panel;
    div()
        .id(("tab-panel", group.node().as_u64()))
        .bg(self.theme.background)
        .rounded(s.corner_radius)
        .when_some(s.outline, |this, outline| {
            this.border_1().border_color(outline.color)
        })
        // base adds flex_col, size_full and overflow_hidden after this
}
```

Base paints the border after the children, so the outline sits on top of the
tab bar and content and follows the radius (`gpui-pre/src/style.rs:744-756`).
The tab bar inherits the top corners because the clip is on this frame.

### F6 shadow and glow

```rust
.shadow(vec![
    BoxShadow {
        color: black().alpha(0.35),
        offset: point(px(0.), px(3.)),
        blur_radius: px(12.),
        spread_radius: px(-2.),
        inset: false,
    },
    BoxShadow {
        color: accent.alpha(0.30),
        offset: point(px(0.), px(0.)),
        blur_radius: px(22.),
        spread_radius: px(0.),
        inset: false,
    },
])
```

This is the same primitive the dialogs use. The shadow paints before the
background, so it never tints the panel's own content, and its spill into the
gap is covered only where the next panel actually paints.

### F3 gap colour

```rust
fn split_frame(&self, node: NodeId, _: Axis, _: &mut Window, cx: &mut App) -> Stateful<Div> {
    div()
        .id(("dock-split-frame", node.as_u64()))
        .bg(self.style.gap.background.unwrap_or(cx.theme().tokens.tab_bar))
}
```

### F4, F7, F8 splitter

```rust
fn render_split_handle(&self, h: &ResizeHandleContext, window: &mut Window, cx: &mut App)
    -> Option<AnyElement>
{
    let t = self.style.splitter.thickness;         // e.g. 6 px
    let color = match h.state() { ... };
    let visual = div()
        .flex_none()
        .bg(color)
        .map(|d| match h.axis() {
            Axis::Horizontal => d.w(t).h(self.length(h)).ml((t - px(1.)) * -0.5),
            Axis::Vertical   => d.h(t).w(self.length(h)).mt((t - px(1.)) * -0.5),
        })
        .when(self.style.splitter.cap == SplitterCap::Rounded, |d| d.rounded(t * 0.5));

    Some(match h.edge() {
        // Hugging a dock edge: the overhang would be clipped by dock_frame.
        Some(_) => deferred(visual).into_any_element(),
        None => visual.into_any_element(),
    })
}
```

Thickness works exactly as the stock pill does
(`gpui-component/src/resizable.rs:118-148`). Percent length is a relative size
of the full-length band; insets are absolute offsets inside it (the band is
already positioned). `deferred` is public (`resizable.rs:12`), and the
deferral-only-for-hugging-edges rule is the one the existing code documents.

**Caveat:** 0.8's own skin never uses percent or insets, so those two modes
need a small spike; fixed pixels and caps are proven.

### Per-panel effect registry

```rust
fn effect_for(&self, group: &TabGroupContext, cx: &App) -> PanelEffect {
    group.active_panel()
        .map(|p| p.panel_id(cx))
        .and_then(|id| self.panel_effects.borrow().get(&id).cloned())
        .unwrap_or_else(|| self.style.panel.effect)
}
```

This gives the intended "one panel deep shadow, another a soft glow" for groups
that hold one visible panel. It cannot give *different tabs in one group*
different effects at the same time — only one is visible anyway — and it is
owned by the skin, not by a `Dock` method.

---

## 5. Gaps that need an upstream change

| Priority | Gap | Minimum change |
|----------|-----|----------------|
| **P0** | Vertical gaps | Make `ResizablePanel`'s flex direction follow its axis (`panel.rs:338`). One line, but needs both-axis regression tests |
| **P1** | Hit band wider than 9 px | Expose `HANDLE_PADDING` / `HANDLE_SIZE`, or add a builder on `resize_handle` (`resize_handle.rs:12-13`). This is the spec's Phase 2 and is still correct |
| **P2** | Adjacency boilerplate | Add `parent: Option<{ node, axis, index, count }>` to `TabGroupContext`, or a `GroupSides` helper. Without it every skin re-walks `DockArea::layout` |
| **P2** | Per-handle animation identity | `ResizeHandleContext` has no id. The stock indicator already uses one global transition key for every divider (`gpui-component/src/resizable.rs:85-101`). Add a node/handle id if independent animation is required |
| **P3** | Effect layer / inner clipper | Only needed to match the spec's *implementation* shape. Requirements F1/F5/F6 are satisfiable on the frame. Doing it exactly needs base to stop clipping the hook's element, or to accept an outer+inner pair |
| **P3** | `DockGeometryStyle` API | Entirely new surface. If it is wanted, it must be added in `gpui-component`; `Dock` is deliberately not exported (`gpui-component/src/dock/mod.rs:42-49`), so the name in Listing 2 is unavailable even as an alias |

Two corrections to the spec's risk register:

1. Add *"vertical margins cannot shrink a `size_full` group; panels are rows"*
   — this is a certainty, not a risk.
2. Rephrase *"A future base change clips slots"* — base **already clips the
   group frame**; the effect layer risk runs the other way.

---

## 6. What is not testable today (build-environment gap)

The spec's verification plan wants headless `VisualTestContext` tests. In this
workspace, enabling `gpui-kit/test-support` currently fails at dependency
resolution:

- `gpui-fast-platform 0.1.10` (an optional dep of `gpui-kit`) requires
  `cocoa =0.26.0`, while `gpui-pre-macos 0.3.9` resolves `cocoa 0.26.1`.
- The same graph pins `unicode-properties =0.1.3`, while `gpui-pre-web` resolves
  `0.1.4`.

The committed `Cargo.lock` avoids this because it was resolved before those
exact pins mattered, but **activating `gpui-kit/test-support`** (through a
dev-dependency or a forwarding feature) forces a fresh resolution and hits it.
A plain new target does not: I verified that adding a trivial extra
`[[example]]` still built against the committed lock. I hit the failure while
trying to add the probe test and worked around it by running the probe as a
temporary example instead.

That is not a design flaw in the spec, but it means:

- The spec's headless test gate cannot be stood up in this repo as-is.
- The geometry API itself can be added without `test-support`; it is the
  verification plan that is blocked.

---

## 7. Recommended shape if this is built now

**Status:** built as `src/geometry.rs` and `examples/dock_geometry.rs`; see
section 8 for what landed and what the implementation probe measured.

1. **Do not plan a new component module yet.** Build the geometry as an
   application-side skin first; it exercises the real constraints and needs no
   fork.
2. **Implement F1, F3, F4, F5, F7, F8 and frame-level F6** in that skin. That
   is most of the visual payoff.
3. **Patch `ResizablePanel`'s flex direction** (or vendor the one file) if
   vertical gaps are required. The vertical branch was verified in a synthetic
   panel; without the change, document the gap as horizontal-only.
4. **Keep the 8 px default.** It is under the 9 px band and needs no base
   change for coverage.
5. **Use area padding for `gap.outer`**, not group margins.
6. **Leave the hit-band growth and the `DockGeometryStyle` API for a later
   upstream pass**; neither is needed to prove the look.

If the goal is to land this in gpui-kit itself, the order that minimises risk
is: panel-direction fix → skin-side geometry → adjacency in the context →
hit-band parameter → (only if the layer abstraction is still wanted) frame
composition.

---

## 8. Implementation status

Built as `src/geometry.rs` plus `examples/dock_geometry.rs`. It is an
application-side skin, exactly the shape section 7 recommended: `DockSkin`
keeps every non-geometric hook, and `GeometryDockSkin` overrides the area frame,
split frame, splitter and tab-group frame.

| Requirement | State | Notes |
|---|---|---|
| F1 rounded panels | Built | Radius on the group frame; base already clips it |
| F2 gap width | Horizontal only | 8 px gap = `ml/mr(4)`; vertical sides stay computed but unapplied |
| F3 gap colour | Built | `split_frame` background, theme tab-bar token when unset |
| F4 splitter thickness | Built | Overhang with negative margin, stock band untouched |
| F5 panel outline | Built | Border on the group frame; themed colour by default |
| F6 shadow / glow | Built, reduced form | `Styled::shadow` on the frame; no separate effect layer exists |
| F7 splitter length | Built, all three modes | Pixels, percent and insets all measured (below) |
| F8 rounded caps | Built | Radius is `thickness / 2` |
| Per-panel effect | Built | Registry keyed by `PanelId`, group-granularity |
| Hit band > 9 px | Not built | `HANDLE_SIZE`/`HANDLE_PADDING` still `pub(crate)` |
| Dock-edge handles | Not built | `DockSkin::render_dock` owns them and `shared()` is private, so the stock appearance is retained |

Defaults reproduce the stock skin. An untouched `SplitterStyle` delegates to
the stock renderer, which keeps the animated hairline/pill; the first explicit
splitter setting opts into the fixed geometric bar. This is the one place where
"defaults reproduce today's look" is implemented by delegation rather than by
collapsing the new structure.

New measurements from the implementation probe (the probe was a temporary
example, since removed). The probe ran at a 0.933 device scale, so absolute
numbers are scaled; the ratios and the subtractions are exact:

```
area            900.0 x 600.0   (padding 6 applied to the content box)
split-frame     888.8 x 588.8
left-group      396.0 wide       (400 slot - 4 margin)
right-group     396.0 wide       (400 slot - 4 margin)
percent-wrap     0.8 x 588.8     (1 px band content, full height)
percent-visual  5.6 x 265.6      (45% of 588.8; 6 px overhang)
inset-visual    5.6 x 524.8      (588.8 - 16 - 48)
```

So percent resolves against the full edge and insets compose as padding
without measuring anything, which was the one open question in section 1.

Two implementation-time findings:

1. **The dock-edge handle cannot be restyled through the area renderer.**
   `DockSkin::render_dock` builds it with `resize_handle()` and the stock
   appearance, and the `SkinShared` needed to reimplement that chrome is
   `pub(crate)`. Seam splitters are styled; dock edges are not. Open question 3
   in the spec therefore needs a base change or a full chrome reimplementation.
2. **A group's style must be shared through an `Rc`, not copied.**
   `tab_group_renderer()` is called when a group entity is created, not on
   every frame, so a value snapshot freezes the style for groups already on
   screen. `GeometryDockSkin` shares the style and effect registry through
   `Rc<RefCell<…>>`, which is what makes `set_style` repaint existing groups.

---

## Appendix: reproducing the gap probe

`on_prepaint` is `gpui_base::ElementExt`, not a GPUI core method; the probe
used it to read each element's resolved bounds on the second frame.

```rust
use gpui_kit::base::ElementExt as _;

div()
    .flex()
    .flex_col()
    .w(px(900.))
    .h(px(600.))
    .child(
        div().flex().w(px(400.)).h(px(200.))
            .on_prepaint(|bounds, _, _| log("panel-h", bounds))
            .child(
                div().size_full().mx(px(4.))
                    .on_prepaint(|bounds, _, _| log("group-h", bounds)),
            ),
    )
    .child(
        div().flex().w(px(400.)).h(px(200.))
            .on_prepaint(|bounds, _, _| log("panel-v", bounds))
            .child(
                div().size_full().my(px(4.))
                    .on_prepaint(|bounds, _, _| log("group-v", bounds)),
            ),
    )
```

Measured:

```
panel-h   origin=(0.0,0.0)   size=(400.0,200.0)
group-h   origin=(4.0,0.0)   size=(392.0,200.0)   # shrunk by 8, correctly
panel-v   origin=(0.0,200.0) size=(400.0,200.0)
group-v   origin=(0.0,204.0) size=(400.0,200.0)   # bottom 404, overflowed
```

Re-running the vertical half with `.flex().flex_col()` on the panel — the
shape the proposed base fix produces — gives the expected result:

```
panel-v-col   origin=(0.0,200.0) size=(400.0,200.0)
group-v-col   origin=(0.0,204.0) size=(400.0,192.0)   # bottom 396, gap works
```

The probe was a temporary replacement for `examples/gallery.rs`, run twice, and
restored. It needs no test-support feature (which is why it worked; see
section 6).
