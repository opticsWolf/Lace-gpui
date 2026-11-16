//! Sliders, progress bars and dials (`_range.py`).
//!
//! Each is painted in one canvas: a rounded groove in a muted track colour,
//! the accent from the minimum to the value, and a round button-coloured
//! knob. Dragging uses window-level mouse listeners registered at paint time,
//! so a drag keeps tracking outside the widget.

use std::rc::Rc;
use std::time::Instant;

use gpui_kit::gpui::{
    canvas, div, point, px, quad, size, App, BorderStyle, Bounds, DispatchPhase, ElementId, Hsla,
    InteractiveElement, IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    ParentElement, Pixels, RenderOnce, ScrollWheelEvent, Styled, Window,
};

use super::interaction::{child_id, Interaction};
use crate::glyph;
use crate::style::rules::{self, GROOVE};
use crate::style::{hsla, ActiveStyle, LaceStyle, State};

/// Fusion's slider handle length, px; the knob is 2 px smaller.
pub const HANDLE: f32 = 16.0;
/// Length of a slider tick, px.
pub const TICK: f32 = 4.0;
/// Busy segment length (fraction of the bar) and period, s.
pub const BUSY_SPAN: f32 = 0.3;
const BUSY_PERIOD: f32 = 1.6;
/// Room outside a dial's arc for notches, and their length, px.
pub const NOTCH_ROOM: f32 = 6.0;
const NOTCH_LEN: f32 = 3.0;
/// Dial knob diameter, as a multiple of the groove.
const DIAL_KNOB: f32 = 3.0;

type Change = Rc<dyn Fn(f64, &mut Window, &mut App)>;

/// Paint a rounded rect: fill and an optional `(colour, width)` outline.
pub fn round_rect(window: &mut Window, b: Bounds<Pixels>, radius: f32, fill: Hsla, line: Option<(Hsla, f32)>) {
    let (color, width) = line.unwrap_or((gpui_kit::gpui::transparent_black(), 0.0));
    window.paint_quad(quad(b, px(radius), fill, px(width), color, BorderStyle::Solid));
}

fn rect(x: f32, y: f32, w: f32, h: f32) -> Bounds<Pixels> {
    Bounds::new(point(px(x), px(y)), size(px(w.max(0.0)), px(h.max(0.0))))
}

/// Paint a round knob of diameter `d` centred on `c`.
fn paint_knob(window: &mut Window, style: &LaceStyle, s: State, c: (f32, f32), d: f32) {
    let face = rules::knob(style, s);
    let b = rect((c.0 - d / 2.0).round(), (c.1 - d / 2.0).round(), d, d);
    let line = match face.ring {
        Some(ring) => (hsla(ring), style.focus_width),
        None => (hsla(face.line.unwrap_or(face.text)), 1.0),
    };
    round_rect(window, b, d / 2.0, hsla(face.fill.unwrap_or(face.text)), Some(line));
}

#[derive(Clone, Copy, Default)]
struct DragUi {
    dragging: bool,
    over_knob: bool,
}

/// Where slider ticks go (`QSlider::TickPosition`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Ticks {
    /// None.
    #[default]
    None,
    /// Above (left of) the groove.
    Above,
    /// Below (right of) the groove.
    Below,
    /// Both sides.
    Both,
}

/// A `QSlider`.
#[derive(IntoElement)]
pub struct Slider {
    id: ElementId,
    value: f64,
    min: f64,
    max: f64,
    step: f64,
    page: f64,
    vertical: bool,
    ticks: Ticks,
    tick_interval: f64,
    disabled: bool,
    forced: Option<State>,
    on_change: Option<Change>,
}

impl Slider {
    /// A horizontal slider on `value`, 0..=100.
    pub fn new(id: impl Into<ElementId>, value: f64) -> Self {
        Self {
            id: id.into(),
            value,
            min: 0.0,
            max: 100.0,
            step: 1.0,
            page: 10.0,
            vertical: false,
            ticks: Ticks::None,
            tick_interval: 0.0,
            disabled: false,
            forced: None,
            on_change: None,
        }
    }

    /// Range.
    pub fn range(mut self, min: f64, max: f64) -> Self {
        self.min = min;
        self.max = max;
        self
    }

    /// Single and page steps.
    pub fn steps(mut self, step: f64, page: f64) -> Self {
        self.step = step;
        self.page = page;
        self
    }

    /// Vertical (the minimum at the bottom).
    pub fn vertical(mut self) -> Self {
        self.vertical = true;
        self
    }

    /// Tick marks every `interval` (0: every page step).
    pub fn ticks(mut self, ticks: Ticks, interval: f64) -> Self {
        self.ticks = ticks;
        self.tick_interval = interval;
        self
    }

    /// Disable.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Paint in `state` (hover / press apply to the knob).
    pub fn state(mut self, state: State) -> Self {
        self.forced = Some(state);
        self
    }

    /// Called with the new value while dragging or stepping.
    pub fn on_change(mut self, f: impl Fn(f64, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
}

/// The slider's pixel extent across the groove.
fn slider_thickness(ticks: Ticks) -> f32 {
    let extra = match ticks {
        Ticks::None => 0.0,
        Ticks::Above | Ticks::Below => TICK + 1.0,
        Ticks::Both => 2.0 * (TICK + 1.0),
    };
    HANDLE + 4.0 + extra
}

impl RenderOnce for Slider {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let ui = window.use_keyed_state(child_id(&self.id, "drag"), cx, |_, _| DragUi::default());
        let ix = Interaction::new(self.id.clone(), window, cx)
            .focusable(&self.id, window, cx)
            .force(self.forced);
        let tracked = ix.state(window, cx).with_disabled(self.disabled);
        let u = *ui.read(cx);
        // Hover and press belong to the knob (`activeSubControls`).
        let knob_state = if self.forced.is_some() {
            tracked
        } else {
            State {
                hovered: u.over_knob || u.dragging,
                pressed: u.dragging,
                ..tracked
            }
        };
        let s = State { hovered: false, pressed: false, ..tracked };
        let style = cx.lace().clone();
        let (value, min, max, step, page) = (self.value, self.min, self.max, self.step, self.page);
        let span = (max - min).max(f64::EPSILON);
        let clamp = move |v: f64| v.clamp(min, max);
        let vertical = self.vertical;
        let ticks = self.ticks;
        let interval = if self.tick_interval > 0.0 { self.tick_interval } else { page.max(step) };
        let thick = slider_thickness(ticks);
        let on_change = self.on_change.clone().filter(|_| !s.disabled && self.forced.is_none());
        let drag = ui.clone();

        // Value at a pointer position along the groove.
        let value_at = move |b: Bounds<Pixels>, p: gpui_kit::gpui::Point<Pixels>| -> f64 {
            let (pos, origin, len) = if vertical {
                (f32::from(p.y), f32::from(b.origin.y), f32::from(b.size.height))
            } else {
                (f32::from(p.x), f32::from(b.origin.x), f32::from(b.size.width))
            };
            let travel = (len - HANDLE).max(1.0);
            let mut frac = ((pos - origin - HANDLE / 2.0) / travel).clamp(0.0, 1.0) as f64;
            if vertical {
                frac = 1.0 - frac;
            }
            let v = min + frac * span;
            clamp((v / step).round() * step)
        };

        let paint = canvas(
            |_, _, _| (),
            move |b, (), window, _cx| {
                let (x0, y0) = (f32::from(b.origin.x), f32::from(b.origin.y));
                let (w, h) = (f32::from(b.size.width), f32::from(b.size.height));
                let len = if vertical { h } else { w };
                let travel = (len - HANDLE).max(0.0);
                let frac = ((value - min) / span).clamp(0.0, 1.0) as f32;
                let along = |f: f32| if vertical { travel * (1.0 - f) } else { travel * f } + HANDLE / 2.0;
                // The groove sits centred in the space the ticks leave.
                let before = matches!(ticks, Ticks::Above | Ticks::Both) as u8 as f32 * (TICK + 1.0);
                let after = matches!(ticks, Ticks::Below | Ticks::Both) as u8 as f32 * (TICK + 1.0);
                let across_len = if vertical { w } else { h };
                let mid = before + (across_len - before - after) / 2.0;
                let kc = along(frac);
                let track = hsla(rules::track(&style, s));
                let fill = hsla(rules::range_fill(&style, s));
                if vertical {
                    let gx = x0 + (mid - GROOVE / 2.0).round();
                    round_rect(window, rect(gx, y0 + HANDLE / 2.0 - 2.0, GROOVE, len - HANDLE + 4.0), GROOVE / 2.0, track, None);
                    round_rect(window, rect(gx, y0 + kc, GROOVE, len - HANDLE / 2.0 + 2.0 - kc), GROOVE / 2.0, fill, None);
                } else {
                    let gy = y0 + (mid - GROOVE / 2.0).round();
                    round_rect(window, rect(x0 + HANDLE / 2.0 - 2.0, gy, len - HANDLE + 4.0, GROOVE), GROOVE / 2.0, track, None);
                    round_rect(window, rect(x0 + HANDLE / 2.0 - 2.0, gy, kc - HANDLE / 2.0 + 2.0, GROOVE), GROOVE / 2.0, fill, None);
                }
                if ticks != Ticks::None && max > min {
                    let color = hsla(rules::ticks(&style, s));
                    let mut v = min;
                    while v <= max + 1e-9 {
                        let c = along(((v - min) / span) as f32).floor() + 0.5;
                        for (on, at) in [
                            (before > 0.0, 0.0),
                            (after > 0.0, across_len - TICK),
                        ] {
                            if !on {
                                continue;
                            }
                            if vertical {
                                glyph::paint_line(window, (x0 + at, y0 + c), (x0 + at + TICK, y0 + c), 1.0, color);
                            } else {
                                glyph::paint_line(window, (x0 + c, y0 + at), (x0 + c, y0 + at + TICK), 1.0, color);
                            }
                        }
                        v += interval;
                    }
                }
                let center = if vertical { (x0 + mid, y0 + kc) } else { (x0 + kc, y0 + mid) };
                let d = HANDLE - 2.0;
                paint_knob(window, &style, knob_state, center, d);

                // Pointer: knob hover, click-to-jump and drag.
                let knob_box = rect(center.0 - d / 2.0, center.1 - d / 2.0, d, d);
                let hover = drag.clone();
                window.on_mouse_event(move |ev: &MouseMoveEvent, phase, _window, cx| {
                    if phase != DispatchPhase::Bubble {
                        return;
                    }
                    let over = knob_box.contains(&ev.position);
                    hover.update(cx, |u, cx| {
                        if u.over_knob != over {
                            u.over_knob = over;
                            cx.notify();
                        }
                    });
                });
                if let Some(f) = on_change.clone() {
                    let down = drag.clone();
                    let f2 = f.clone();
                    window.on_mouse_event(move |ev: &MouseDownEvent, phase, window, cx| {
                        if phase == DispatchPhase::Bubble && ev.button == MouseButton::Left && b.contains(&ev.position) {
                            down.update(cx, |u, cx| {
                                u.dragging = true;
                                cx.notify();
                            });
                            let v = value_at(b, ev.position);
                            if v != value {
                                f2(v, window, cx);
                            }
                        }
                    });
                    let mv = drag.clone();
                    window.on_mouse_event(move |ev: &MouseMoveEvent, phase, window, cx| {
                        if phase == DispatchPhase::Bubble && mv.read(cx).dragging && ev.dragging() {
                            let v = value_at(b, ev.position);
                            if v != value {
                                f(v, window, cx);
                            }
                        }
                    });
                }
                let up = drag.clone();
                window.on_mouse_event(move |_: &MouseUpEvent, _, _window, cx| {
                    up.update(cx, |u, cx| {
                        if u.dragging {
                            u.dragging = false;
                            cx.notify();
                        }
                    });
                });
            },
        )
        .size_full();

        let el = div().id(self.id.clone()).flex_none().child(paint);
        let el = if vertical { el.w(px(thick)).h_full().min_h(px(60.)) } else { el.h(px(thick)).w_full().min_w(px(60.)) };
        let el = ix.bind(el, s.disabled);
        match self.on_change.filter(|_| !s.disabled) {
            Some(f) => {
                let key = f.clone();
                el.on_scroll_wheel(move |ev: &ScrollWheelEvent, window, cx| {
                    let dy = f32::from(ev.delta.pixel_delta(px(20.)).y);
                    if dy.abs() > 0.0 {
                        f(clamp(if dy > 0.0 { value + step } else { value - step }), window, cx);
                    }
                })
                .on_key_down(move |ev, window, cx| {
                    let v = match ev.keystroke.key.as_str() {
                        "right" | "up" => value + step,
                        "left" | "down" => value - step,
                        "pageup" => value + page,
                        "pagedown" => value - page,
                        "home" => min,
                        "end" => max,
                        _ => return,
                    };
                    key(clamp(v), window, cx);
                    cx.stop_propagation();
                })
            }
            None => el,
        }
    }
}

// ---------------------------------------------------------------------------
// Progress bar
// ---------------------------------------------------------------------------

/// A `QProgressBar`: a track with the accent filled to the value; `busy`
/// sweeps a segment across instead.
#[derive(IntoElement)]
pub struct ProgressBar {
    value: f64,
    min: f64,
    max: f64,
    busy: bool,
    vertical: bool,
    thickness: f32,
    disabled: bool,
    animate: bool,
}

impl ProgressBar {
    /// A bar at `value` of 0..=100.
    pub fn new(value: f64) -> Self {
        Self { value, min: 0.0, max: 100.0, busy: false, vertical: false, thickness: 8.0, disabled: false, animate: true }
    }

    /// Range.
    pub fn range(mut self, min: f64, max: f64) -> Self {
        self.min = min;
        self.max = max;
        self
    }

    /// Indeterminate (`minimum == maximum == 0`).
    pub fn busy(mut self) -> Self {
        self.busy = true;
        self
    }

    /// Hold a busy bar still, mid-track (renders, galleries).
    pub fn still(mut self) -> Self {
        self.animate = false;
        self
    }

    /// Vertical, filling from the bottom.
    pub fn vertical(mut self) -> Self {
        self.vertical = true;
        self
    }

    /// Bar thickness, px.
    pub fn thickness(mut self, t: f32) -> Self {
        self.thickness = t;
        self
    }

    /// Disable.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

thread_local! {
    static EPOCH: Instant = Instant::now();
}

impl RenderOnce for ProgressBar {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let style = cx.lace().clone();
        let s = State::NORMAL.with_disabled(self.disabled);
        let me = self;
        let paint = canvas(
            |_, _, _| (),
            move |b, (), window, _| {
                let (x0, y0) = (f32::from(b.origin.x), f32::from(b.origin.y));
                let (w, h) = (f32::from(b.size.width), f32::from(b.size.height));
                let radius = style.control_radius.min(w.min(h) / 2.0);
                round_rect(window, b, radius, hsla(rules::track(&style, s)), None);
                let length = if me.vertical { h } else { w };
                let (start, len) = if me.busy {
                    let t = if me.animate {
                        window.request_animation_frame();
                        EPOCH.with(|e| (e.elapsed().as_secs_f32() % BUSY_PERIOD) / BUSY_PERIOD)
                    } else {
                        0.5
                    };
                    let seg = length * BUSY_SPAN;
                    (-seg + t * (length + seg), seg)
                } else {
                    let span = me.max - me.min;
                    let frac = if span <= 0.0 { 0.0 } else { ((me.value - me.min) / span).clamp(0.0, 1.0) as f32 };
                    let len = length * frac;
                    (if me.vertical { length - len } else { 0.0 }, len)
                };
                let (a, z) = (start.max(0.0), (start + len).min(length));
                if z <= a {
                    return;
                }
                let bar = if me.vertical { rect(x0, y0 + a, w, z - a) } else { rect(x0 + a, y0, z - a, h) };
                round_rect(window, bar, radius, hsla(rules::range_fill(&style, s)), None);
            },
        )
        .size_full();
        let el = div().flex_none().child(paint);
        if me.vertical {
            el.w(px(me.thickness)).h_full()
        } else {
            el.h(px(me.thickness)).w_full()
        }
    }
}

// ---------------------------------------------------------------------------
// Dial
// ---------------------------------------------------------------------------

/// A `QDial`: a round groove, accent from the minimum to the value, and the
/// slider's knob riding on it; optional notches outside.
#[derive(IntoElement)]
pub struct Dial {
    id: ElementId,
    value: f64,
    min: f64,
    max: f64,
    step: f64,
    notches: bool,
    notch_interval: f64,
    diameter: f32,
    disabled: bool,
    forced: Option<State>,
    on_change: Option<Change>,
}

impl Dial {
    /// A dial on `value`, 0..=100.
    pub fn new(id: impl Into<ElementId>, value: f64) -> Self {
        Self {
            id: id.into(),
            value,
            min: 0.0,
            max: 100.0,
            step: 1.0,
            notches: false,
            notch_interval: 10.0,
            diameter: 64.0,
            disabled: false,
            forced: None,
            on_change: None,
        }
    }

    /// Range.
    pub fn range(mut self, min: f64, max: f64) -> Self {
        self.min = min;
        self.max = max;
        self
    }

    /// Show notches every `interval`.
    pub fn notches(mut self, interval: f64) -> Self {
        self.notches = true;
        self.notch_interval = interval;
        self
    }

    /// Diameter, px.
    pub fn diameter(mut self, d: f32) -> Self {
        self.diameter = d;
        self
    }

    /// Disable.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Paint in `state`.
    pub fn state(mut self, state: State) -> Self {
        self.forced = Some(state);
        self
    }

    /// Called with the new value.
    pub fn on_change(mut self, f: impl Fn(f64, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
}

/// Qt's dial angle for `frac` of the range: 240 deg at the minimum to -60 at
/// the maximum, counter-clockwise from 3 o'clock.
fn dial_angle(frac: f32) -> f32 {
    240.0 - frac * 300.0
}

impl RenderOnce for Dial {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let ui = window.use_keyed_state(child_id(&self.id, "drag"), cx, |_, _| DragUi::default());
        let ix = Interaction::new(self.id.clone(), window, cx)
            .focusable(&self.id, window, cx)
            .force(self.forced);
        let mut s = ix.state(window, cx).with_disabled(self.disabled);
        if self.forced.is_none() && ui.read(cx).dragging {
            s.pressed = true;
            s.hovered = true;
        }
        let style = cx.lace().clone();
        let (value, min, max, step) = (self.value, self.min, self.max, self.step);
        let span = (max - min).max(f64::EPSILON);
        let clamp = move |v: f64| v.clamp(min, max);
        let (notches, interval) = (self.notches, self.notch_interval.max(step));
        let on_change = self.on_change.clone().filter(|_| !s.disabled && self.forced.is_none());
        let drag = ui.clone();
        let value_at = move |c: (f32, f32), p: gpui_kit::gpui::Point<Pixels>| -> f64 {
            let (dx, dy) = (f32::from(p.x) - c.0, c.1 - f32::from(p.y));
            let mut a = dy.atan2(dx).to_degrees();
            if a < -90.0 {
                a += 360.0;
            }
            let frac = ((240.0 - a) / 300.0).clamp(0.0, 1.0) as f64;
            clamp(((min + frac * span) / step).round() * step)
        };
        let paint = canvas(
            |_, _, _| (),
            move |b, (), window, _| {
                let side = f32::from(b.size.width).min(f32::from(b.size.height));
                let knob_d = (GROOVE * DIAL_KNOB).min(side * 0.2);
                let room = if notches { NOTCH_ROOM } else { 0.0 };
                let radius = side / 2.0 - room - knob_d / 2.0 - 1.0;
                if radius <= GROOVE {
                    return;
                }
                let c = (
                    f32::from(b.origin.x) + f32::from(b.size.width) / 2.0,
                    f32::from(b.origin.y) + f32::from(b.size.height) / 2.0,
                );
                let start = dial_angle(0.0);
                let at = dial_angle(((value - min) / span).clamp(0.0, 1.0) as f32);
                let plain = State { hovered: false, pressed: false, ..s };
                glyph::paint_arc(window, c, radius, start, -300.0, GROOVE, hsla(rules::track(&style, plain)));
                if (at - start).abs() > 0.01 {
                    glyph::paint_arc(window, c, radius, start, at - start, GROOVE, hsla(rules::range_fill(&style, plain)));
                }
                if notches {
                    let color = hsla(rules::ticks(&style, plain));
                    let inner = radius + knob_d / 2.0 + 1.0;
                    let mut v = min;
                    while v <= max + 1e-9 {
                        let a = dial_angle(((v - min) / span) as f32);
                        glyph::paint_line(
                            window,
                            glyph::on_circle(c, inner, a),
                            glyph::on_circle(c, inner + NOTCH_LEN, a),
                            1.0,
                            color,
                        );
                        v += interval;
                    }
                }
                paint_knob(window, &style, s, glyph::on_circle(c, radius, at), knob_d);

                if let Some(f) = on_change.clone() {
                    let down = drag.clone();
                    let f2 = f.clone();
                    window.on_mouse_event(move |ev: &MouseDownEvent, phase, window, cx| {
                        if phase == DispatchPhase::Bubble && ev.button == MouseButton::Left && b.contains(&ev.position) {
                            down.update(cx, |u, cx| {
                                u.dragging = true;
                                cx.notify();
                            });
                            let v = value_at(c, ev.position);
                            if v != value {
                                f2(v, window, cx);
                            }
                        }
                    });
                    let mv = drag.clone();
                    window.on_mouse_event(move |ev: &MouseMoveEvent, phase, window, cx| {
                        if phase == DispatchPhase::Bubble && mv.read(cx).dragging && ev.dragging() {
                            let v = value_at(c, ev.position);
                            if v != value {
                                f(v, window, cx);
                            }
                        }
                    });
                }
                let up = drag.clone();
                window.on_mouse_event(move |_: &MouseUpEvent, _, _window, cx| {
                    up.update(cx, |u, cx| {
                        if u.dragging {
                            u.dragging = false;
                            cx.notify();
                        }
                    });
                });
            },
        )
        .size_full();
        let el = div().id(self.id.clone()).flex_none().size(px(self.diameter)).child(paint);
        let el = ix.bind(el, s.disabled);
        match self.on_change.filter(|_| !s.disabled) {
            Some(f) => el.on_key_down(move |ev, window, cx| {
                let v = match ev.keystroke.key.as_str() {
                    "right" | "up" => value + step,
                    "left" | "down" => value - step,
                    "pageup" => value + step * 10.0,
                    "pagedown" => value - step * 10.0,
                    _ => return,
                };
                f(clamp(v), window, cx);
                cx.stop_propagation();
            }),
            None => el,
        }
    }
}
