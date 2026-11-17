//! Switches, segmented button groups and star ratings: toggles Qt has no
//! class for, painted with LaceStyle's indicator and button rules.

use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

use gpui_kit::gpui::{
    div, prelude::FluentBuilder, px, App, ElementId, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window,
};

use super::check::LABEL_GAP;
use super::interaction::{child_id, Interaction};
use crate::glyph;
use crate::style::rules::{self, ButtonKind};
use crate::style::{hsla, paint, tones, ActiveStyle, State};

/// `id` with an indexed child, for repeated parts (segments, stars, pages).
pub fn nth_id(id: &ElementId, part: &str, i: usize) -> ElementId {
    ElementId::NamedChild(Arc::new(id.clone()), SharedString::from(format!("{part}{i}")))
}

// ---------------------------------------------------------------------------
// Switch
// ---------------------------------------------------------------------------

/// Track size of a switch, px.
pub const SWITCH_W: f32 = 36.0;
/// Track height of a switch, px.
pub const SWITCH_H: f32 = 20.0;
/// Knob diameter, px. Track, knob, inset and travel are all multiples of
/// 4px, so they stay whole device pixels at 100%, 125% and 150% scale and
/// the knob cannot snap to one side.
const KNOB: f32 = 12.0;
/// Gap between the knob and the track's outer edge, px (border included).
const KNOB_INSET: f32 = (SWITCH_H - KNOB) / 2.0;
/// Time the knob takes to slide across, ms.
const SLIDE_MS: f32 = 110.0;

#[derive(Clone, Copy)]
struct SlideUi {
    pos: f32,
    last: Option<Instant>,
}

type Flip = Rc<dyn Fn(bool, &mut Window, &mut App)>;

/// An on/off switch: a pill track and a sliding knob, the accent when on.
#[derive(IntoElement)]
pub struct Switch {
    id: ElementId,
    label: Option<SharedString>,
    on: bool,
    disabled: bool,
    forced: Option<State>,
    on_change: Option<Flip>,
}

impl Switch {
    /// A switch.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self { id: id.into(), label: None, on: false, disabled: false, forced: None, on_change: None }
    }

    /// Label after the switch.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// On or off.
    pub fn checked(mut self, on: bool) -> Self {
        self.on = on;
        self
    }

    /// Disable.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Paint in `state` regardless of the pointer.
    pub fn state(mut self, state: State) -> Self {
        self.forced = Some(state);
        self
    }

    /// Called with the new value on click or Space.
    pub fn on_change(mut self, f: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for Switch {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let ix = Interaction::new(self.id.clone(), window, cx)
            .focusable(&self.id, window, cx)
            .force(self.forced);
        let mut s = ix.state(window, cx).with_disabled(self.disabled).with_checked(self.on);
        if let Some(f) = self.forced {
            s = s.union(f);
        }
        let style = cx.lace().clone();
        let c = tones::switch(&style, s);

        // Slide the knob toward where the value puts it.
        let target = if s.checked { 1.0 } else { 0.0 };
        let slide = window.use_keyed_state(child_id(&self.id, "slide"), cx, move |_, _| SlideUi { pos: target, last: None });
        let pos = if self.forced.is_some() || style.scrollbar_grow_ms <= 0.0 {
            target
        } else {
            let now = Instant::now();
            let ui = *slide.read(cx);
            let dt = ui.last.map_or(16.0, |t| now.duration_since(t).as_secs_f32() * 1000.0).min(50.0);
            let step = dt / SLIDE_MS;
            let pos = if (ui.pos - target).abs() <= step { target } else { ui.pos + step * (target - ui.pos).signum() };
            let moving = pos != target;
            slide.update(cx, |u, _| {
                u.pos = pos;
                u.last = moving.then_some(now);
            });
            if moving {
                window.request_animation_frame();
            }
            pos
        };
        let eased = 1.0 - (1.0 - pos) * (1.0 - pos);

        // The knob sits in flow: the track centres it vertically, and its
        // left margin slides it. Measured from inside the 1px border, so the
        // inset is the same on all four sides at either end.
        let travel = SWITCH_W - 2.0 * KNOB_INSET - KNOB;
        let track = div()
            .flex()
            .flex_none()
            .items_center()
            .w(px(SWITCH_W))
            .h(px(SWITCH_H))
            .rounded_full()
            .bg(hsla(c.track))
            .border_1()
            .border_color(hsla(c.line))
            .child(
                div()
                    .flex_none()
                    .ml(px(KNOB_INSET - 1.0 + travel * eased))
                    .size(px(KNOB))
                    .rounded_full()
                    .bg(hsla(c.knob))
                    .border_1()
                    .border_color(hsla(c.knob_line)),
            );
        let text = style.pal(s).window_text;
        let ring = s.has_focus().then(|| paint::focus_ring(&style, s));
        let el = div()
            .id(self.id.clone())
            .flex()
            .flex_none()
            .items_center()
            .gap(px(LABEL_GAP))
            .h(px(style.control_height - 4.0))
            .px(px(2.))
            .rounded(px(style.control_radius))
            .text_size(px(style.font_size))
            .text_color(hsla(text))
            .child(track)
            .when_some(self.label, |d, l| d.child(l))
            .when_some(ring, |d, r| d.border(px(1.)).border_color(hsla(r)));
        let el = ix.bind(el, s.disabled);
        match self.on_change {
            Some(f) if !s.disabled => {
                let on = self.on;
                let key = f.clone();
                el.on_click(move |_, window, cx| f(!on, window, cx)).on_key_down(move |ev, window, cx| {
                    if ev.keystroke.key == "space" {
                        key(!on, window, cx);
                        cx.stop_propagation();
                    }
                })
            }
            _ => el,
        }
    }
}

// ---------------------------------------------------------------------------
// Segmented
// ---------------------------------------------------------------------------

type Pick = Rc<dyn Fn(usize, &mut Window, &mut App)>;

/// A segmented control (a button group of exclusive choices): joined button
/// faces, the outer corners rounded, the selected one checked.
#[derive(IntoElement)]
pub struct Segmented {
    id: ElementId,
    items: Vec<SharedString>,
    selected: Option<usize>,
    disabled: bool,
    fill: bool,
    forced: Option<State>,
    on_select: Option<Pick>,
}

impl Segmented {
    /// A segmented control over `items`.
    pub fn new(id: impl Into<ElementId>, items: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        Self {
            id: id.into(),
            items: items.into_iter().map(Into::into).collect(),
            selected: None,
            disabled: false,
            fill: false,
            forced: None,
            on_select: None,
        }
    }

    /// The selected segment.
    pub fn selected(mut self, ix: impl Into<Option<usize>>) -> Self {
        self.selected = ix.into();
        self
    }

    /// Stretch across the parent, segments sharing the width.
    pub fn full_width(mut self) -> Self {
        self.fill = true;
        self
    }

    /// Disable.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Paint every segment in `state`.
    pub fn state(mut self, state: State) -> Self {
        self.forced = Some(state);
        self
    }

    /// Called with the segment clicked.
    pub fn on_select(mut self, f: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for Segmented {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let style = cx.lace().clone();
        let r = style.control_radius;
        let n = self.items.len();
        let mut row = div().flex().flex_none().h(px(style.control_height)).when(self.fill, |d| d.w_full());
        for (i, label) in self.items.iter().enumerate() {
            let id = nth_id(&self.id, "seg", i);
            let ix = Interaction::new(id.clone(), window, cx).focusable(&id, window, cx).force(self.forced);
            let s = ix.state(window, cx).union(State {
                checked: self.selected == Some(i),
                disabled: self.disabled,
                ..State::NORMAL
            });
            let face = rules::button_face(&style, s, ButtonKind::Normal);
            let line = face.ring.or(face.line).unwrap_or(style.pal(s).button);
            let (first, last) = (i == 0, i + 1 == n);
            let el = div()
                .id(id)
                .h_full()
                .px(px(10.))
                .flex()
                .items_center()
                .justify_center()
                .whitespace_nowrap()
                .text_size(px(style.font_size))
                .text_color(hsla(face.text))
                .when_some(face.fill, |d, f| d.bg(hsla(f)))
                .border_y_1()
                .border_r_1()
                .when(first, |d| d.border_l_1().rounded_l(px(r)))
                .when(last, |d| d.rounded_r(px(r)))
                // Each segment draws its right edge; the one before a checked
                // segment hands it the shared line.
                .border_color(hsla(line))
                .when(self.fill, |d| d.flex_1().min_w_0())
                .child(label.clone());
            let el = ix.bind(el, s.disabled);
            let el = match self.on_select.clone() {
                Some(f) if !s.disabled => {
                    let key = f.clone();
                    el.on_click(move |_, window, cx| f(i, window, cx)).on_key_down(move |ev, window, cx| {
                        if matches!(ev.keystroke.key.as_str(), "space" | "enter") {
                            key(i, window, cx);
                            cx.stop_propagation();
                        }
                    })
                }
                _ => el,
            };
            row = row.child(el);
        }
        row
    }
}

// ---------------------------------------------------------------------------
// Rating
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Default)]
struct RateUi {
    hovered: Option<usize>,
}

/// A star rating: click a star to rate, hover to preview.
#[derive(IntoElement)]
pub struct Rating {
    id: ElementId,
    value: usize,
    max: usize,
    size: f32,
    disabled: bool,
    on_change: Option<Pick>,
}

impl Rating {
    /// `value` stars out of 5.
    pub fn new(id: impl Into<ElementId>, value: usize) -> Self {
        Self { id: id.into(), value, max: 5, size: 16.0, disabled: false, on_change: None }
    }

    /// Number of stars.
    pub fn max(mut self, max: usize) -> Self {
        self.max = max.max(1);
        self
    }

    /// Star size, px.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Disable.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Called with the new rating (clicking the current one clears it).
    pub fn on_change(mut self, f: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
}

/// The ten points of a five-pointed star centred on `c`.
fn star_points(c: (f32, f32), outer: f32) -> Vec<(f32, f32)> {
    let inner = outer * 0.45;
    (0..10)
        .map(|k| {
            let r = if k % 2 == 0 { outer } else { inner };
            glyph::on_circle(c, r, 90.0 + 36.0 * k as f32)
        })
        .collect()
}

impl RenderOnce for Rating {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let style = cx.lace().clone();
        let ui = window.use_keyed_state(child_id(&self.id, "rate"), cx, |_, _| RateUi::default());
        let hovered = if self.disabled { None } else { ui.read(cx).hovered };
        let s = State::NORMAL.with_disabled(self.disabled);
        let mut row = div().id(self.id.clone()).flex().flex_none().items_center().gap(px(2.));
        for i in 0..self.max {
            let on = hovered.map_or(i < self.value, |h| i <= h && i < self.value);
            let preview = hovered.is_some_and(|h| i <= h);
            let color = hsla(tones::star(&style, s, on, preview));
            let size = self.size;
            let star = div()
                .id(nth_id(&self.id, "star", i))
                .size(px(size))
                .flex_none()
                .child(glyph::painter(move |b, window, _| {
                    let c = b.center();
                    glyph::paint_polygon(window, &star_points((f32::from(c.x), f32::from(c.y)), size * 0.5), color);
                }));
            let star = if self.disabled {
                star
            } else {
                let hover = ui.clone();
                let star = star.on_hover(move |h, _, cx| {
                    hover.update(cx, |u, cx| {
                        if *h {
                            u.hovered = Some(i);
                        } else if u.hovered == Some(i) {
                            u.hovered = None;
                        }
                        cx.notify();
                    })
                });
                match self.on_change.clone() {
                    Some(f) => {
                        let value = self.value;
                        star.on_click(move |_, window, cx| f(if value == i + 1 { 0 } else { i + 1 }, window, cx))
                    }
                    None => star,
                }
            };
            row = row.child(star);
        }
        row
    }
}
