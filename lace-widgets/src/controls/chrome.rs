//! Splitters, scrollbars, frame lines, toolbar handles and size grips
//! (`_chrome.py`, `_primitives.scrollbar`).

use std::rc::Rc;

use gpui_kit::gpui::{
    canvas, div, point, prelude::FluentBuilder, px, size, AnyElement, App, Bounds, DispatchPhase,
    Div, ElementId, InteractiveElement, IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, ParentElement, Pixels, RenderOnce, ScrollWheelEvent, SharedString, Styled, Window,
};

use super::interaction::{child_id, Interaction};
use super::range::round_rect;
use crate::glyph::{self, Dir};
use crate::style::rules::{self, EXPANDING_REST, SPLITTER_WIDTH};
use crate::style::{hsla, ActiveStyle, LaceStyle, ScrollbarMode, State};

fn rect(x: f32, y: f32, w: f32, h: f32) -> Bounds<Pixels> {
    Bounds::new(point(px(x), px(y)), size(px(w.max(0.0)), px(h.max(0.0))))
}

fn nth(id: &ElementId, label: &str, i: usize) -> ElementId {
    ElementId::NamedChild(std::sync::Arc::new(id.clone()), SharedString::from(format!("{label}{i}")))
}

// ---------------------------------------------------------------------------
// Splitter
// ---------------------------------------------------------------------------

/// Smallest pane a drag leaves, px.
pub const MIN_PANE: f32 = 24.0;

#[derive(Clone, Default)]
struct SplitUi {
    fracs: Vec<f32>,
    /// Handle being dragged, pointer start, fractions at the start.
    drag: Option<(usize, f32, Vec<f32>)>,
    /// Space shared by the panes (container minus handles), px.
    avail: f32,
}

/// A `QSplitter` over any number of panes: each handle is
/// [`SPLITTER_WIDTH`] px with a rounded grip that turns accent and thickens
/// on hover or drag.
#[derive(IntoElement)]
pub struct Splitter {
    id: ElementId,
    vertical: bool,
    panes: Vec<(AnyElement, f32)>,
    forced: Option<State>,
}

impl Splitter {
    /// Panes side by side (`Qt::Horizontal`).
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self { id: id.into(), vertical: false, panes: Vec::new(), forced: None }
    }

    /// Panes stacked (`Qt::Vertical`).
    pub fn vertical(mut self) -> Self {
        self.vertical = true;
        self
    }

    /// Add a pane with an initial stretch (relative size).
    pub fn pane(mut self, el: impl IntoElement, stretch: f32) -> Self {
        self.panes.push((el.into_any_element(), stretch.max(0.01)));
        self
    }

    /// Paint every handle in `state`.
    pub fn state(mut self, state: State) -> Self {
        self.forced = Some(state);
        self
    }
}

/// A splitter handle's grip painted for `s`, filling a handle box.
pub fn splitter_grip(style: &LaceStyle, s: State, vertical_grip: bool) -> impl IntoElement {
    let (color, thick) = rules::splitter_grip(style, s);
    let color = hsla(color);
    let length = style.splitter_length;
    glyph::painter(move |b, window, _| {
        let (w, h) = (f32::from(b.size.width), f32::from(b.size.height));
        let c = b.center();
        let (cx, cy) = (f32::from(c.x), f32::from(c.y));
        let g = if vertical_grip {
            let l = length.min(h);
            rect((cx - thick / 2.0).round(), cy - l / 2.0, thick.min(w), l)
        } else {
            let l = length.min(w);
            rect(cx - l / 2.0, (cy - thick / 2.0).round(), l, thick.min(h))
        };
        round_rect(window, g, thick / 2.0, color, None);
    })
}

impl RenderOnce for Splitter {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let n = self.panes.len();
        let stretches: Vec<f32> = self.panes.iter().map(|p| p.1).collect();
        let ui = window.use_keyed_state(child_id(&self.id, "split"), cx, move |_, _| {
            let sum: f32 = stretches.iter().sum();
            SplitUi { fracs: stretches.iter().map(|s| s / sum).collect(), drag: None, avail: 0.0 }
        });
        if ui.read(cx).fracs.len() != n {
            ui.update(cx, |u, _| u.fracs = vec![1.0 / n.max(1) as f32; n]);
        }
        let style = cx.lace().clone();
        let vertical = self.vertical;
        let fracs = ui.read(cx).fracs.clone();
        let dragging = ui.read(cx).drag.as_ref().map(|d| d.0);

        let mut el = div().id(self.id.clone()).relative().flex().size_full();
        el = if vertical { el.flex_col() } else { el.flex_row() };
        for (i, (pane, _)) in self.panes.into_iter().enumerate() {
            let mut p = div().relative().min_w_0().min_h_0().overflow_hidden().child(pane);
            {
                let st = p.style();
                st.flex_grow = Some(fracs[i]);
                st.flex_shrink = Some(1.0);
                st.flex_basis = Some(px(0.).into());
            }
            el = el.child(p);
            if i + 1 < n {
                let hid = nth(&self.id, "handle", i);
                let ix = Interaction::new(hid.clone(), window, cx).force(self.forced);
                let mut s = ix.state(window, cx);
                if dragging == Some(i) {
                    s.pressed = true;
                    s.hovered = true;
                }
                let start = ui.clone();
                let handle = div()
                    .id(hid)
                    .flex_none()
                    .when(vertical, |d| d.h(px(SPLITTER_WIDTH)).w_full().cursor_row_resize())
                    .when(!vertical, |d| d.w(px(SPLITTER_WIDTH)).h_full().cursor_col_resize())
                    .child(splitter_grip(&style, s, !vertical))
                    .on_mouse_down(MouseButton::Left, move |ev: &MouseDownEvent, _, cx| {
                        let at = if vertical { f32::from(ev.position.y) } else { f32::from(ev.position.x) };
                        start.update(cx, |u, cx| {
                            u.drag = Some((i, at, u.fracs.clone()));
                            cx.notify();
                        });
                    });
                el = el.child(ix.bind(handle, false));
            }
        }

        // Measure, and follow drags anywhere in the window.
        let handles = SPLITTER_WIDTH * n.saturating_sub(1) as f32;
        let track = ui.clone();
        el.child(
            canvas(
                |_, _, _| (),
                move |b, (), window, cx| {
                    let total = if vertical { f32::from(b.size.height) } else { f32::from(b.size.width) };
                    let avail = (total - handles).max(1.0);
                    if (track.read(cx).avail - avail).abs() > 0.5 {
                        track.update(cx, |u, _| u.avail = avail);
                    }
                    let mv = track.clone();
                    window.on_mouse_event(move |ev: &MouseMoveEvent, phase, _, cx| {
                        if phase != DispatchPhase::Bubble {
                            return;
                        }
                        let Some((i, from, start)) = mv.read(cx).drag.clone() else { return };
                        if !ev.dragging() {
                            return;
                        }
                        let at = if vertical { f32::from(ev.position.y) } else { f32::from(ev.position.x) };
                        let min = (MIN_PANE / avail).min(0.45);
                        let pair = start[i] + start[i + 1];
                        let a = (start[i] + (at - from) / avail).clamp(min.min(pair / 2.0), (pair - min).max(pair / 2.0));
                        mv.update(cx, |u, cx| {
                            u.fracs[i] = a;
                            u.fracs[i + 1] = pair - a;
                            cx.notify();
                        });
                    });
                    let up = track.clone();
                    window.on_mouse_event(move |_: &MouseUpEvent, _, _, cx| {
                        if up.read(cx).drag.is_some() {
                            up.update(cx, |u, cx| {
                                u.drag = None;
                                cx.notify();
                            });
                        }
                    });
                },
            )
            .absolute()
            .size_full(),
        )
    }
}

// ---------------------------------------------------------------------------
// Scrollbar
// ---------------------------------------------------------------------------

/// Shortest scrollbar handle, px.
pub const HANDLE_MIN: f32 = 20.0;

#[derive(Clone, Copy, Default)]
struct BarUi {
    on_handle: bool,
    drag: Option<(f32, f64)>,
}

type Scroll = Rc<dyn Fn(f64, &mut Window, &mut App)>;

/// A `QScrollBar`: `value` in `0..=max`, `page` the visible span. `thin`
/// bars are a handle on a faint track; `expanding` bars rest as a slim
/// handle and grow, with step arrows, on hover.
#[derive(IntoElement)]
pub struct Scrollbar {
    id: ElementId,
    value: f64,
    max: f64,
    page: f64,
    vertical: bool,
    disabled: bool,
    forced: Option<(State, bool)>,
    on_scroll: Option<Scroll>,
}

impl Scrollbar {
    /// A horizontal bar.
    pub fn new(id: impl Into<ElementId>, value: f64, max: f64, page: f64) -> Self {
        Self { id: id.into(), value, max, page, vertical: false, disabled: false, forced: None, on_scroll: None }
    }

    /// Vertical.
    pub fn vertical(mut self) -> Self {
        self.vertical = true;
        self
    }

    /// Disable.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Paint in `state`, `on_handle` saying whether the pointer is on the handle.
    pub fn state(mut self, state: State, on_handle: bool) -> Self {
        self.forced = Some((state, on_handle));
        self
    }

    /// Called with the new value.
    pub fn on_scroll(mut self, f: impl Fn(f64, &mut Window, &mut App) + 'static) -> Self {
        self.on_scroll = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for Scrollbar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let ui = window.use_keyed_state(child_id(&self.id, "bar"), cx, |_, _| BarUi::default());
        let ix = Interaction::new(self.id.clone(), window, cx).force(self.forced.map(|f| f.0));
        let mut s = ix.state(window, cx).with_disabled(self.disabled);
        let u = *ui.read(cx);
        let on_handle = match self.forced {
            Some((_, h)) => h,
            None => u.on_handle || u.drag.is_some(),
        };
        if self.forced.is_none() && u.drag.is_some() {
            s.pressed = true;
            s.hovered = true;
        }
        let style = cx.lace().clone();
        let mode = style.scrollbar;
        let extent = mode.extent();
        let buttons = mode == ScrollbarMode::Expanding;
        let (value, max, page, vertical) = (self.value, self.max.max(0.0), self.page.max(0.0), self.vertical);
        let on_scroll = self.on_scroll.clone().filter(|_| !s.disabled && self.forced.is_none());
        let state = ui.clone();

        let paint = canvas(
            |_, _, _| (),
            move |b, (), window, _| {
                let (x0, y0) = (f32::from(b.origin.x), f32::from(b.origin.y));
                let (w, h) = (f32::from(b.size.width), f32::from(b.size.height));
                let thick_full = if vertical { w } else { h };
                let along = if vertical { h } else { w };
                let btn = if buttons { thick_full } else { 0.0 };
                let length = (along - 2.0 * btn).max(0.0);
                let handle_len = if max <= 0.0 {
                    length
                } else {
                    (length * (page / (max + page)) as f32).max(HANDLE_MIN).min(length)
                };
                let travel = (length - handle_len).max(0.0);
                let pos = if max <= 0.0 { 0.0 } else { travel * (value / max).clamp(0.0, 1.0) as f32 };
                let colors = rules::scrollbar(&style, s, on_handle);
                let expanded = s.active_hover() || s.pressed;
                // At rest an expanding bar hugs its far edge, slim.
                let thick = if buttons && !expanded { thick_full.min(EXPANDING_REST + 2.0) } else { thick_full };
                let off = thick_full - thick;
                if let Some(track) = colors.track {
                    round_rect(window, b, 0.0, hsla(track), None);
                }
                if buttons && expanded {
                    let pal = style.pal(s);
                    for (i, dir, at_end) in [(0, if vertical { Dir::Up } else { Dir::Left }, value <= 0.0), (1, if vertical { Dir::Down } else { Dir::Right }, value >= max)] {
                        let r = if vertical {
                            rect(x0, if i == 0 { y0 } else { y0 + h - btn }, w, btn)
                        } else {
                            rect(if i == 0 { x0 } else { x0 + w - btn }, y0, btn, h)
                        };
                        let (_, ink) = rules::scroll_step(&style, s, false, at_end);
                        let _ = pal;
                        glyph::paint_triangle(window, r, dir, hsla(ink), btn * 0.5 + 2.0);
                    }
                }
                let pad = if thick >= 7.0 { 2.0 } else { 1.0 };
                let hb = if vertical {
                    rect(x0 + off + pad, y0 + btn + pos + pad, thick - 2.0 * pad, handle_len - 2.0 * pad)
                } else {
                    rect(x0 + btn + pos + pad, y0 + off + pad, handle_len - 2.0 * pad, thick - 2.0 * pad)
                };
                if max > 0.0 {
                    let r = (thick - 2.0 * pad).min(handle_len - 2.0 * pad) / 2.0;
                    round_rect(window, hb, r, hsla(colors.handle), None);
                }

                // Pointer: handle hover, drag, page clicks, step buttons.
                let full_handle = if vertical {
                    rect(x0, y0 + btn + pos, w, handle_len)
                } else {
                    rect(x0 + btn + pos, y0, handle_len, h)
                };
                let hover = state.clone();
                window.on_mouse_event(move |ev: &MouseMoveEvent, phase, _, cx| {
                    if phase != DispatchPhase::Bubble {
                        return;
                    }
                    let over = full_handle.contains(&ev.position);
                    hover.update(cx, |u, cx| {
                        if u.on_handle != over {
                            u.on_handle = over;
                            cx.notify();
                        }
                    });
                });
                let Some(f) = on_scroll.clone() else { return };
                let along_of = move |p: gpui_kit::gpui::Point<Pixels>| if vertical { f32::from(p.y) } else { f32::from(p.x) };
                let origin = if vertical { y0 } else { x0 };
                let down = state.clone();
                let f_down = f.clone();
                window.on_mouse_event(move |ev: &MouseDownEvent, phase, window, cx| {
                    if phase != DispatchPhase::Bubble || ev.button != MouseButton::Left || !b.contains(&ev.position) {
                        return;
                    }
                    let at = along_of(ev.position);
                    if full_handle.contains(&ev.position) {
                        down.update(cx, |u, cx| {
                            u.drag = Some((at, value));
                            cx.notify();
                        });
                    } else if btn > 0.0 && at < origin + btn {
                        f_down((value - page / 10.0).max(0.0), window, cx);
                    } else if btn > 0.0 && at > origin + along - btn {
                        f_down((value + page / 10.0).min(max), window, cx);
                    } else if at < origin + btn + pos {
                        f_down((value - page).max(0.0), window, cx);
                    } else {
                        f_down((value + page).min(max), window, cx);
                    }
                });
                let mv = state.clone();
                window.on_mouse_event(move |ev: &MouseMoveEvent, phase, window, cx| {
                    if phase != DispatchPhase::Bubble || !ev.dragging() {
                        return;
                    }
                    if let Some((from, v0)) = mv.read(cx).drag {
                        if travel > 0.0 {
                            let v = (v0 + ((along_of(ev.position) - from) / travel) as f64 * max).clamp(0.0, max);
                            if v != value {
                                f(v, window, cx);
                            }
                        }
                    }
                });
                let up = state.clone();
                window.on_mouse_event(move |_: &MouseUpEvent, _, _, cx| {
                    if up.read(cx).drag.is_some() {
                        up.update(cx, |u, cx| {
                            u.drag = None;
                            cx.notify();
                        });
                    }
                });
            },
        )
        .size_full();

        let el = div().id(self.id.clone()).flex_none().child(paint);
        let el = if vertical { el.w(px(extent)).h_full() } else { el.h(px(extent)).w_full() };
        let el = ix.bind(el, s.disabled);
        match self.on_scroll.filter(|_| !s.disabled) {
            Some(f) => el.on_scroll_wheel(move |ev: &ScrollWheelEvent, window, cx| {
                let d = ev.delta.pixel_delta(px(20.));
                let dv = f32::from(if vertical { d.y } else { d.x }) as f64;
                if dv != 0.0 {
                    f((value - dv).clamp(0.0, max), window, cx);
                }
            }),
            None => el,
        }
    }
}

// ---------------------------------------------------------------------------
// Lines and grips
// ---------------------------------------------------------------------------

/// A `QFrame::HLine` / `VLine`.
pub fn frame_line(style: &LaceStyle, s: State, vertical: bool) -> Div {
    let c = hsla(rules::frame_line(style, s));
    let el = div().flex_none().bg(c);
    if vertical {
        el.w(px(1.)).h_full()
    } else {
        el.h(px(1.)).w_full()
    }
}

const HANDLE_SHARE: f32 = 0.45;
const HANDLE_GAP: f32 = 3.0;
const SEPARATOR_INSET: f32 = 4.0;
/// Size-grip diagonal lines, as distances from the corner.
const GRIP_LINES: [f32; 3] = [4.0, 8.0, 12.0];

/// A toolbar's drag handle: two short parallel lines (toolbar horizontal
/// unless `vertical`).
pub fn toolbar_handle(style: &LaceStyle, s: State, vertical: bool) -> Div {
    let c = hsla(rules::chrome_ink(style, s));
    let el = div().flex_none().child(glyph::painter(move |b, window, _| {
        let ctr = b.center();
        let (cx, cy) = (f32::from(ctr.x), f32::from(ctr.y));
        if !vertical {
            let half = f32::from(b.size.height) * HANDLE_SHARE / 2.0;
            for dx in [-HANDLE_GAP / 2.0, HANDLE_GAP / 2.0] {
                let x = (cx + dx).round() + 0.5;
                glyph::paint_line(window, (x, cy - half), (x, cy + half), 1.0, c);
            }
        } else {
            let half = f32::from(b.size.width) * HANDLE_SHARE / 2.0;
            for dy in [-HANDLE_GAP / 2.0, HANDLE_GAP / 2.0] {
                let y = (cy + dy).round() + 0.5;
                glyph::paint_line(window, (cx - half, y), (cx + half, y), 1.0, c);
            }
        }
    }));
    if vertical {
        el.h(px(10.)).w_full()
    } else {
        el.w(px(10.)).h_full()
    }
}

/// A toolbar separator: one line across, inset from the edges.
pub fn toolbar_separator(style: &LaceStyle, s: State, vertical: bool) -> Div {
    let c = hsla(rules::chrome_ink(style, s));
    let el = div().flex_none().child(glyph::painter(move |b, window, _| {
        let (x0, y0) = (f32::from(b.origin.x), f32::from(b.origin.y));
        let (w, h) = (f32::from(b.size.width), f32::from(b.size.height));
        if !vertical {
            let x = (x0 + w / 2.0).floor() + 0.5;
            window.paint_quad(gpui_kit::gpui::fill(rect(x - 0.5, y0 + SEPARATOR_INSET, 1.0, h - 2.0 * SEPARATOR_INSET), c));
        } else {
            let y = (y0 + h / 2.0).floor() + 0.5;
            window.paint_quad(gpui_kit::gpui::fill(rect(x0 + SEPARATOR_INSET, y - 0.5, w - 2.0 * SEPARATOR_INSET, 1.0), c));
        }
    }));
    if vertical {
        el.h(px(8.)).w_full()
    } else {
        el.w(px(8.)).h_full()
    }
}

/// A size grip: three diagonal lines in the bottom-right corner.
pub fn size_grip(style: &LaceStyle, s: State) -> Div {
    let c = hsla(rules::chrome_ink(style, s));
    div().size(px(16.)).flex_none().child(glyph::painter(move |b, window, _| {
        let x0 = f32::from(b.origin.x + b.size.width) - 1.0;
        let y0 = f32::from(b.origin.y + b.size.height) - 1.0;
        let reach = f32::from(b.size.width).min(f32::from(b.size.height)) - 1.0;
        for k in GRIP_LINES {
            if k > reach {
                break;
            }
            glyph::paint_line(window, (x0 - k, y0 - 1.0), (x0 - 1.0, y0 - k), 1.0, c);
        }
    }))
}
