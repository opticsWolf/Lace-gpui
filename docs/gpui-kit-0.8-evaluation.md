# Evaluation record: gpui-kit 0.8 geometric dock feasibility

This is the working record behind
[`gpui-kit-0.8-feasibility.md`](gpui-kit-0.8-feasibility.md): what was
evaluated, in what order, what was verified by execution rather than reading,
where the dead ends were, and how confident each conclusion is.

It is deliberately separate from the findings so the report stays readable and
this file can keep the receipts.

---

## 1. Scope and baseline

| | |
|---|---|
| Subject | `gpui-kit-geometric-dock-styling-spec.md` (design draft, 2026-10-22) |
| Question | How much can be built against the API gpui-kit 0.8 ships, and what are the gaps? |
| `gpui-kit` | 0.8.0 |
| `gpui-base` | 0.8.0 |
| `gpui-component` | 0.8.0 |
| `gpui-pre` | 0.3.9 (pinned by gpui-kit) |
| Host | Windows, `cargo`/`rustc` 1.99.0 (2026-08-27 / 2026-09-28) |
| Date | 2026-10-26 |
| Crate under test | `lace-gpui` 0.0.2 (`gpui-kit` behind an optional feature) |

Sources were read from the vendored registry tree:

```
~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-kit-0.8.0
~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-base-0.8.0
~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-component-0.8.0
~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-pre-0.3.9
```

The spec's claims were checked against those files, not against the spec's own
description of them.

---

## 2. Method

Three passes, in order:

1. **API inventory.** Enumerate every hook, context type and constant the spec
   relies on, and confirm it is public, has the claimed shape, and is composed
   the way the spec assumes.
2. **Mechanism audit.** Follow each proposed mechanism to the code that would
   have to support it: margins, clipping, paint order, hit band, handle
   appearance, layout-tree queries.
3. **Executable probe.** For the one assumption that decides whether the gap
   feature works at all — "margins on the group frame shrink its visible box" —
   build a temporary window and read the resolved bounds. Source reading gave a
   prediction; the probe confirmed it.

The probe was necessary because the margin question is a layout-engine
semantics question, not an API question, and the answer differs by axis.

---

## 3. Chronology and dead ends

### 3.1 Source audit — the seam is real

Confirmed early:

- Two traits carry all appearance: `DockAreaRenderer` and `TabGroupRenderer`.
- The handle hook receives axis, edge and state, and the band is fixed.
- The stock skin already implements everything the spec wants to extend
  (hairline + pill, gap background, dock chrome).
- Defaults in 0.8 do reproduce today's look.

No surprises in this pass.

### 3.2 Source audit — three structural mismatches

Three assumptions failed on reading:

1. **Frame composition.** The spec assumes an unclipped outer frame plus an
   inner clipper. Base takes the hook's single element, applies the column,
   fill and clip to it, then appends the tab bar and content. The returned
   element is the clipper.
2. **Vertical margins.** The spec assumes margins shrink the group within its
   slot. `ResizablePanel` is always a row, and the group is forced to
   `size_full()` after the hook, so vertical margins are cross-axis and do not
   shrink.
3. **Per-panel effects and handle identity.** The frame belongs to a group, not
   a panel, and `ResizeHandleContext` carries no id.

### 3.3 Probe attempt 1 — headless test harness (blocked)

The candidate was a `#[gpui_kit::test]` test using `VisualTestContext` and
`debug_bounds`, mirroring the base crate's own dock tests. This required
`gpui-kit/test-support`.

Three attempts, all blocked at dependency resolution:

| Attempt | Result |
|---------|--------|
| Add `gpui-kit` as a dev-dependency with `features = ["test-support"]` | `cocoa` conflict: `gpui-fast-platform` wants `=0.26.0`, lock has `0.26.1` |
| Same, `--offline` | `gpui-fast-apple` not cached |
| Add a `gpui-kit-test` feature forwarding to `gpui-kit/test-support` | Same `cocoa` conflict |

Trying to fix the lock made it worse:

```
cargo update cocoa@0.26.1 --precise 0.26.0
→ unicode-properties conflict: gpui-fast-platform wants =0.1.3,
  gpui-pre-web has 0.1.4
```

Pinning both would have been whack-a-mole against optional `gpui-fast-*`
packages that are not even used on Windows. Abandoned.

**Side discovery:** the trigger is activating `gpui-kit/test-support`, not
editing the manifest in general. After restoring the manifest I added a trivial
extra `[[example]]` target and it still built against the committed lock. The
headless harness is what pulls the optional `gpui-fast-*` graph into
resolution; ordinary targets and features that do not touch `test-support` are
fine. This is recorded as a build-environment gap in the report because it
blocks the spec's own headless test plan.

### 3.4 Probe attempt 2 — real window, file logging (worked)

To avoid touching the manifest, the probe temporarily replaced
`examples/gallery.rs` (an existing target). A watchdog thread wrote to
`%TEMP%\layout_probe.log` and force-exited after 15 seconds, because the first
run hung and stdout was lost when the command timed out.

Two small compile fixes:

- `size(px(900., 600.))` is not valid; `size()` takes a single value. Used
  `.w(px(900.)).h(px(600.))`.
- `on_prepaint` is `gpui_base::ElementExt`, not a GPUI core method. Imported
  `gpui_kit::base::ElementExt as _`.

Result (second frame, after layout settled):

```
panel-h   origin=(0.0,0.0)   size=(400.0,200.0)
group-h   origin=(4.0,0.0)   size=(392.0,200.0)
panel-v   origin=(0.0,200.0) size=(400.0,200.0)
group-v   origin=(0.0,204.0) size=(400.0,200.0)
```

Interpretation:

- Horizontal: `mx(4)` reduced width by exactly 8 and kept the box inside the
  slot. **The gap mechanism works.**
- Vertical: `my(4)` moved the top down by 4 but kept full height, so the bottom
  landed at 404 — 4 px past the slot. **No gap; just overflow.**

That single result is the most consequential discovery in the evaluation. It
turns a "probably fine" into a definite axis split.

### 3.5 Probe 2 — the proposed fix, simulated

A second temporary run made the vertical panel `.flex().flex_col()` — the
shape the one-line base fix would produce — and kept the same `my(4)` group:

```
panel-v-col   origin=(0.0,200.0) size=(400.0,200.0)
group-v-col   origin=(0.0,204.0) size=(400.0,192.0)   # bottom 396, gap works
```

So the diagnosis in D4 and the fix in D6 are both measured, not inferred. The
remaining uncertainty is only whether the real panel's other users tolerate the
column direction for vertical splits.

### 3.6 Cleanup

- `examples/gallery.rs` restored from backup.
- `Cargo.toml` and `Cargo.lock` restored; `cargo check --features gpui-kit`
  passes again.
- Probe test/example files deleted.
- Both probe log files removed.
- Working tree contains only the new `docs/` directory and the pre-existing
  untracked `palette-parity.png`.

---

## 4. Discovery catalogue

Each entry states how it was established.

### D1 — The group frame is the clipper, not an outer shell

**Verified by source.** `TabGroup::render` calls `frame(...)`, then applies
`.flex().flex_col().size_full().overflow_hidden()`, then `.child(tab bar)` and
`.child(content)`. `gpui-base/src/dock/tab_group.rs:691-755`. The stock skin
says the same: `gpui-component/src/dock/tab_panel.rs:709-710`.

**Impact:** F1 and F5 still work. The spec's two-element frame (section 3.2)
does not exist.

### D2 — An unclipped effect layer is impossible

**Verified by source.** Any child of the frame is clipped by the frame's own
`overflow_hidden`. An absolutely positioned layer painted before the content
would also be a child. There is no hook that returns the frame's parent.

**Impact:** F6 must use `Styled::shadow` on the frame. The requirement still
holds by paint order; the mechanism in section 5.1 does not.

### D3 — Shadow is painted before the background and outside its own mask

**Verified by source.** `gpui-pre/src/style.rs:711-742` paints drop shadows,
then background, then inset shadows, then children. The content mask is applied
only around the child continuation (`gpui-pre/src/elements/div.rs:2545-2547`).

**Impact:** a frame's shadow can occupy the gap and still be overpainted by a
later sibling's chrome. Confirms F6's gap-safety with the element-shadow
approach.

### D4 — Horizontal margins shrink; vertical margins overflow

**Verified by execution** (probe, section 3.4).

**Impact:** F2 works for `h_split` and fails for `v_split`.

### D5 — Root cause of D4: panels are always rows, groups are always full-size

**Verified by source.** `ResizablePanel::render` calls `.flex()` unconditionally
(`gpui-base/src/resizable/panel.rs:338-343`); base applies `.size_full()` to the
hook's frame (`tab_group.rs:703-706`); flex-shrink defaults to 1.0
(`gpui-pre/src/style.rs:802`); panels do not clip (`panel.rs:236-241`).

**Impact:** a skin cannot work around it reliably. `size_full()` is forced and
absolute positioning conflicts with it. Measuring the slot height from a
previous frame has no stable source across resizes.

### D6 — The smallest fix is a flex-direction change, not the spec's Phase 2

**Verified by execution in a synthetic panel; not yet in the real tree.**
Making the panel a column for `Axis::Vertical` puts the group's height on the
main axis, where margins shrink exactly as width does today. The probe's
vertical branch with `.flex().flex_col()` produced:

```
panel-v-col   origin=(0.0,200.0) size=(400.0,200.0)
group-v-col   origin=(0.0,204.0) size=(400.0,192.0)   # bottom 396, gap works
```

The resize handle is absolutely positioned, so it is unaffected. What remains
is applying the conditional direction in `gpui-base` and regression-testing the
non-dock uses of `resizable_panel` in both axes.

### D7 — The hit band is fixed at 9 px

**Verified by source.** `HANDLE_SIZE = 1`, `HANDLE_PADDING = 4`, both
`pub(crate)` (`gpui-base/src/resizable/resize_handle.rs:12-13`); the band sizes
are derived at `resize_handle.rs:275-354`. Phase 2 is real.

### D8 — No handle identity

**Verified by source.** `ResizeHandleContext` exposes axis, edge and state only
(`resize_handle.rs:61-93`). The stock indicator's transition is keyed by a
global string, so every divider already shares one animation state
(`gpui-component/src/resizable.rs:85-101`).

**Impact:** per-handle animation is not addressable; shared animation is.

### D9 — Adjacency is derivable but not provided

**Verified by source.** `TabGroupContext` has no parent/axis/index
(`tab_group.rs:768-892`). `DockArea::layout(placement)` is public
(`dock_area.rs:189-194`), and `PaneTree`/`PaneNode` expose enough to walk it
(`layout/tree.rs:64-105`, `layout/node.rs:84-118`).

**Impact:** the app can compute interior sides, but only if it keeps its own
weak handle to the area. `DockSkin` has one, but `shared()` is `pub(crate)`
(`gpui-component/src/dock/mod.rs:214-225`).

### D10 — `gap.outer` is area padding, not group margins

**Verified by source + D4.** Base applies `size_full`, `flex_row` and
`overflow_hidden` to the area frame (`dock_area.rs:1547-1560`), so padding
shrinks the content box. A margin on a dock-edge group would reveal the split
frame's gap colour, not the window.

### D11 — Per-panel effect is per-group in practice

**Verified by source.** The styled surface is the group frame (D1). A registry
keyed by the active `PanelId` changes the frame when the visible tab changes;
it cannot give two tabs in one group simultaneous effects. Only one is visible,
so the practical loss is small.

### D12 — The proposed API does not exist

**Verified by source.** `DockGeometryStyle`, `OutlineStyle`, `PanelEffect`,
`ShadowSpec`, `GlowSpec`, `SplitterStyle`, `SplitterLength`, `SplitterCap`,
`SplitterColors` are absent. `Dock` is deliberately not exported by
`gpui-component` (`gpui-component/src/dock/mod.rs:42-49`), so even
`use gpui_component::dock::Dock` in Listing 2 does not resolve. The area type is
`DockArea`; configuration lives on the `Rc<dyn DockAreaRenderer>` the app
constructs.

### D13 — The spec's border and radius assumptions hold

**Verified by source.** The border is painted after the children
(`gpui-pre/src/style.rs:744-756`), so an outline on the frame sits above the tab
bar and content and follows the radius. The clip is on the frame, so the tab
bar inherits the top corners.

### D14 — Defaults are safe

**Verified by source and existing code.** Zero margin/radius/outline/effect
collapses the new structure to today's look, and no appearance is persisted.
This was the spec's strongest claim and it holds.

### D15 — Enabling gpui-kit 0.8's `test-support` currently fails to resolve

**Verified by execution.** Enabling `test-support` — which pulls the optional
`gpui-fast-*` graph into resolution — fails on:

- `cocoa`: `gpui-fast-platform 0.1.10` pins `=0.26.0`, lock has `0.26.1`.
- `unicode-properties`: `gpui-fast-platform` pins `=0.1.3`, `gpui-pre-web` has
  `0.1.4`.

The committed lock hides it. Adding a plain target does not trigger it; the
feature activation does. This is not a design issue in the spec, but it blocks
the spec's headless verification plan in this workspace. The geometry module
itself needs no new dependency.

### D16 — `on_prepaint` is a base extension, not GPUI core

**Verified by execution.** `gpui_base::ElementExt::on_prepaint` adds an
absolutely positioned canvas child and passes the element's resolved bounds.
GPUI core has no such method. Minor, but relevant to anyone reproducing the
probe.

### D17 — `tab_group_renderer()` runs once per group entity, not per frame

**Verified by source.** `DockArea::sync_groups` calls
`self.renderer.tab_group_renderer()` when it creates a `TabGroup` entity and
stores the `Rc` in that entity (`dock_area.rs:1131-1147`). It is not called
again for a cached group.

**Impact on the implementation.** The first version of `GeometryDockSkin`
built a fresh `GeometryTabGroup` with a *copy* of the style, so `set_style`
would never reach groups already on screen. The skin now shares one
`Rc<RefCell<DockGeometryStyle>>` (and one effect registry) with every group
renderer. Any future runtime style knob has to go through that shared cell.

### D18 — Dock-edge resize handles use the stock appearance

**Verified by source and execution.** `DockSkin::render_dock` draws the
dock-edge handle with `crate::resizable::resize_handle`, which injects
`resize_handle_appearance()` — not the `DockAreaRenderer::render_split_handle`
hook. That chrome and its `SkinShared` are private
(`gpui-component/src/dock/dock.rs:69-91, 114-153`).

**Impact:** seam splitters follow `SplitterStyle`; dock-edge handles stay
stock. The region gap described in D20 is a separate channel and does not
change that handle. The showcase screenshot confirmed the visible bar at the
bottom seam is the stock hover pill, not the geometric splitter. Spec open
question 3 (dock-edge handles adopt the splitter style) cannot be answered
without a base change or reimplementing the dock chrome.

### D19 — Percent and inset splitter lengths do resolve in 0.8

**Verified by execution.** A temporary probe with the exact wrapper shape the
skin builds measured:

```
percent-wrap     0.8 x 588.8
percent-visual   5.6 x 265.6   (0.451 of the band height)
inset-visual     5.6 x 524.8   (588.8 - 16 - 48)
```

**Impact:** F7's percent and inset modes are not just expressible, they work.
The probe ran at a 0.933 device scale; the 45% ratio and the 64 px subtraction
are exact. This closes the last medium-confidence item in the original audit.

### D20 — Inter-dock gaps are skin-only, via flex `gap` on base's frames

**Verified by execution.** The area frame is base's flex row (left/centre/right)
and the centre frame is its flex column (centre tree/bottom dock). Applying
`gap(width)` to those frames spaces the dock regions, and `padding(outer)` on
the area frame makes the outside inset. Painting both frames with the gap
colour backs the channels. A temporary run with a left dock open and closed and
the bottom Logs dock showed:

- a vertical channel between the left dock and the centre;
- a horizontal channel between the centre and Logs;
- the outer inset around the whole arrangement;
- no stray gap when the left dock was closed (the empty closed-dock child did
  not contribute a visible flex gap).

**Impact:** the earlier “gaps are per split seam” limitation was too narrow.
Only split seams *inside* a region need the `ResizablePanel` axis fix; dock
region boundaries, including the Logs dock, do not. `/src/geometry.rs` now uses
the area and centre frame hooks for this.

---

## 5. Confidence

| Claim | Basis | Confidence |
|-------|-------|------------|
| Horizontal margin gap works | Measured | High |
| Vertical margin gap fails | Measured | High |
| Inter-dock gaps via frame flex gap work | Measured, left dock open/closed + Logs | High |
| Panel-row + forced `size_full` is the cause | Source + measurement | High |
| Axis-aligned panel direction fixes it | Verified in a synthetic panel | High; still needs a real-tree two-axis test |
| Effect layer is impossible | Source composition | High |
| Element shadow is gap-safe | Source paint order | High |
| Hit band is fixed at 9 px | Source constants | High |
| `Dock`/geometry API absent | Source export list | High |
| Percent/inset splitter lengths work | Measured in the implementation probe | High |
| Per-panel override granularity is the group | Source composition | High |
| `test-support` resolution fails | Reproduced three ways | High (time-specific) |

The two items that should be checked before committing to a plan are the
percent/inset handle modes and the real-tree application of D6 (the synthetic
probe covers the direction change, not the other users of `resizable_panel`).

The percent/inset item is now closed by D19. Only the real-tree `resizable_panel`
change remains open, and the implementation deliberately does not depend on it.

---

## 6. Reproduction

### Read-only checks

```text
gpui-base/src/dock/tab_group.rs          691-963
gpui-base/src/dock/dock_area.rs          189-194, 1436-1450, 1547-1604, 1911-2004
gpui-base/src/resizable/panel.rs         236-241, 338-395
gpui-base/src/resizable/resize_handle.rs 12-13, 61-119, 275-388
gpui-base/src/dock/layout/tree.rs        64-105
gpui-base/src/dock/layout/node.rs        84-118
gpui-component/src/dock/dock.rs          40-153
gpui-component/src/dock/mod.rs           42-57, 185-258
gpui-component/src/dock/tab_panel.rs     700-807
gpui-component/src/resizable.rs          10-151
gpui-pre/src/style.rs                    688-762, 771-802
gpui-pre/src/elements/div.rs             2545-2547
```

### Probe

The probe source is in the appendix of the feasibility report. To rerun
without touching the manifest, replace `examples/gallery.rs` temporarily, run
`cargo run --example gallery --features gpui-kit`, and restore the file. Do
**not** add a test-support dev-dependency until D15 is resolved.

---

## 7. What would change the conclusions

- **D6 fails in the real tree:** vertical gaps would need a base wrapper or a
  new group-inset hook, a larger upstream change. The synthetic probe only
  proves the mechanism; `resizable_panel`'s other users still have to accept a
  column for vertical splits.
- **`on_prepaint`-based measurement made to work across resizes:** vertical
  gaps could be skin-only, but with a one-frame lag and fragile slot-size
  inference.
- **Base changes the group frame composition:** the spec's effect layer and
  per-panel granularity become available; none of the visual findings change.
- **gpui-kit 0.8 publishes a fix or a new lock:** D15 disappears; the rest
  stands.
- **A 0.9 API appears:** re-run the API inventory first; the renderer seam is
  likely stable, but the frame composition and panel direction are exactly the
  kind of thing that could change.

---

## 8. Implementation and verification

The findings were implemented as `src/geometry.rs` (feature `gpui-kit`) and
`examples/dock_geometry.rs`. The implementation follows section 7 of the
feasibility report exactly: an application-side skin over `DockSkin`, with the
stock renderer delegated for everything the geometry does not own.

### 8.1 What was exercised

| Check | How |
|-------|-----|
| Style surface and defaults | 5 unit tests in `src/geometry.rs` |
| Compiles across all targets | `cargo check --features gpui-kit --all-targets` |
| Horizontal gap, outer padding | Temporary layout probe |
| Inter-dock gaps | Demo run with a temporary left dock, open and closed, plus the Logs dock |
| Percent length | Temporary layout probe (D19) |
| Insets length | Temporary layout probe (D19) |
| Real window renders without panic | Ran `dock_geometry` for 8 s |
| Visual result | Window captured with `PrintWindow` and a full-screen grab |

The probe was a third temporary example, added as a plain `[[example]]` target.
Consistent with D15, it resolved against the committed lock; `test-support` was
never enabled.

### 8.2 What the running window showed

- Three groups with rounded corners and a 1 px themed outline.
- Two interior seams with real gaps, each centred on the splitter band.
- A vertical channel above the Logs dock, created by the centre frame's flex
  gap rather than by split margins.
- A vertical channel between a temporary left dock and the centre; removing
  the left dock left no stray gap.
- An outer inset between the whole arrangement and the window frame, painted
  with the gap colour.
- A 45%-length, 6 px, rounded splitter at the seam in the Flagship preset.
- The same splitter at 60% and 3 px in Minimal.
- The capture caught the live switch from Flagship to Minimal, which is
  direct evidence that `set_style` reaches groups created before the change
  (D17's shared-`Rc` fix).
- Per-panel glow via `set_panel_effect` (registry path compiled; the visible
  capture used the dock-wide glow).
- Dock-edge handle still stock: the bar visible where the bottom dock meets the
  centre is the stock hover pill, which is how D18 was noticed.

### 8.3 What the implementation changed in the findings

- D9 (adjacency boilerplate) is confirmed in practice: the skin walks
  `DockArea::layout` for all four regions on every group frame. It works, and
  it is visible boilerplate.
- D10 (`gap.outer` as area padding) is confirmed: the probe measured the split
  frame at `900 - 2 * 5.6` with `p(6)` on the area.
- New D17 and D18 came out of writing the skin, not out of the source audit.
- D19 closed the percent/inset question.
- D20 overturned the earlier “gaps are per split seam” limitation: dock-region
  gaps are flex space on base's own frames and need no panel patch. Only a
  vertical split *inside* a region still needs the panel-axis fix.

### 8.4 Cleanup

Probe target and file removed; logs deleted; `Cargo.toml` keeps only the
`dock_geometry` example; `cargo check` and `cargo test --lib` pass.

---

## 9. Artifacts

| Artifact | Status |
|----------|--------|
| `docs/gpui-kit-0.8-feasibility.md` | Findings and recommendations |
| `docs/gpui-kit-0.8-evaluation.md` | This record |
| `src/geometry.rs` | The geometric skin and style surface |
| `examples/dock_geometry.rs` | The showcase |
| `README.md`, `src/lib.rs` | Documented the module and example |
| `examples/gallery.rs` | Restored to the committed version |
| `Cargo.toml`, `Cargo.lock` | Only the new example target added; check passes |
| Probe examples/tests, `%TEMP%\layout_probe*.log`, `%TEMP%\geometry_probe.log` | Deleted |
| `palette-parity.png` | Pre-existing untracked file, untouched |
