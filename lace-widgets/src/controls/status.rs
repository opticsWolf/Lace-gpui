//! Status and display components: tags, badges, avatars, alerts, links,
//! separators, skeletons, spinners and progress rings. Colours come from
//! [`crate::style::tones`]; shapes follow the control and frame radii.

use std::time::Instant;

use gpui_kit::gpui::{
    canvas, div, prelude::FluentBuilder, px, relative, AnyElement, App, ElementId, FontWeight, InteractiveElement,
    IntoElement, ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window,
};

use super::interaction::Interaction;
use crate::glyph;
use crate::style::tones::{self, Tone};
use crate::style::{hsla, paint, rules, ActiveStyle, State};

thread_local! {
    static EPOCH: Instant = Instant::now();
}

/// Seconds since the first animated component, for looping animations.
fn seconds() -> f32 {
    EPOCH.with(|e| e.elapsed().as_secs_f32())
}

// ---------------------------------------------------------------------------
// Tag
// ---------------------------------------------------------------------------

/// Height of a tag, px.
pub const TAG_H: f32 = 22.0;
/// Height of a count badge, px.
pub const BADGE_H: f32 = 18.0;

/// A tag: a small label on a tone wash, or outlined.
#[derive(IntoElement)]
pub struct Tag {
    label: SharedString,
    tone: Tone,
    outline: bool,
    pill: bool,
    disabled: bool,
}

impl Tag {
    /// A tag in the accent tone.
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self { label: label.into(), tone: Tone::Accent, outline: false, pill: false, disabled: false }
    }

    /// Colour role.
    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    /// Outline only, on the window.
    pub fn outline(mut self) -> Self {
        self.outline = true;
        self
    }

    /// Fully rounded ends.
    pub fn pill(mut self) -> Self {
        self.pill = true;
        self
    }

    /// Disable.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl RenderOnce for Tag {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let style = cx.lace();
        let c = tones::tag(style, State::NORMAL.with_disabled(self.disabled), self.tone, self.outline);
        let radius = if self.pill { 999.0 } else { style.control_radius.min(6.0) };
        div()
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .h(px(TAG_H))
            .px(px(if self.pill { 10. } else { 8. }))
            .rounded(px(radius))
            .when_some(c.fill, |d, f| d.bg(hsla(f)))
            .border_1()
            .border_color(hsla(c.line))
            .text_size(px(style.font_size))
            // A line box as tall as the inner height keeps the glyphs on
            // the vertical centre instead of riding the default leading.
            .line_height(px(TAG_H - 2.0))
            .text_color(hsla(c.text))
            .whitespace_nowrap()
            .child(self.label)
    }
}

// ---------------------------------------------------------------------------
// Badge
// ---------------------------------------------------------------------------

/// A count or dot over the top-right corner of a child (an icon, an avatar,
/// a button), in a solid tone ringed with the window colour.
#[derive(IntoElement)]
pub struct Badge {
    child: Option<AnyElement>,
    count: Option<usize>,
    max: usize,
    dot: bool,
    tone: Tone,
}

impl Badge {
    /// A badge showing `count` (hidden at 0).
    pub fn count(count: usize) -> Self {
        Self { child: None, count: Some(count), max: 99, dot: false, tone: Tone::Danger }
    }

    /// A plain dot.
    pub fn dot() -> Self {
        Self { child: None, count: None, max: 99, dot: true, tone: Tone::Danger }
    }

    /// Counts above `max` show as `max+`.
    pub fn max(mut self, max: usize) -> Self {
        self.max = max;
        self
    }

    /// Colour role (danger by default).
    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    /// What the badge sits on.
    pub fn on(mut self, child: impl IntoElement) -> Self {
        self.child = Some(child.into_any_element());
        self
    }
}

impl RenderOnce for Badge {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let style = cx.lace();
        let c = tones::solid(style, State::NORMAL, self.tone);
        let ring = hsla(style.active.window);
        // Width the mark reaches past the child's right edge, kept as a
        // margin so neighbours in a row are not overlapped.
        let mut reach = 6.0;
        let mark = match (self.dot, self.count) {
            (true, _) => Some(
                div().size(px(12.)).rounded_full().bg(hsla(c.fill.unwrap())).border_2().border_color(ring).into_any_element(),
            ),
            (false, Some(n)) if n > 0 => {
                let text = if n > self.max { format!("{}+", self.max) } else { n.to_string() };
                reach = 6.0 + 7.0 * (text.chars().count() as f32 - 1.0);
                Some(
                    div()
                        .h(px(BADGE_H))
                        .min_w(px(BADGE_H))
                        .px(px(5.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded_full()
                        .bg(hsla(c.fill.unwrap()))
                        .border_1()
                        .border_color(ring)
                        .text_size(px(11.))
                        .line_height(px(BADGE_H - 2.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(hsla(c.text))
                        .child(text)
                        .into_any_element(),
                )
            }
            _ => None,
        };
        // The mark's left edge sits a little inside the child's right edge,
        // so wide counts ("99+") grow outward instead of over the child.
        let mark = mark.map(|m| div().absolute().top(px(-6.)).left(relative(0.72)).child(m));
        let reach = if mark.is_some() { reach } else { 0.0 };
        div()
            .relative()
            .flex_none()
            .mr(px(reach))
            .when_some(self.child, |d, c| d.child(c))
            .when_some(mark, |d, m| d.child(m))
    }
}

// ---------------------------------------------------------------------------
// Avatar
// ---------------------------------------------------------------------------

/// A round avatar with the initials of a name.
#[derive(IntoElement)]
pub struct Avatar {
    name: SharedString,
    size: f32,
    tone: Tone,
}

impl Avatar {
    /// An avatar for `name`.
    pub fn new(name: impl Into<SharedString>) -> Self {
        Self { name: name.into(), size: 32.0, tone: Tone::Accent }
    }

    /// Diameter, px.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Colour role.
    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }
}

/// Up to two initials: the first letters of the first and last words.
pub fn initials(name: &str) -> String {
    let words: Vec<&str> = name.split_whitespace().collect();
    let first = |w: &str| w.chars().next().map(|c| c.to_uppercase().collect::<String>()).unwrap_or_default();
    match words.as_slice() {
        [] => String::new(),
        [one] => first(one),
        [a, .., z] => first(a) + &first(z),
    }
}

impl RenderOnce for Avatar {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let style = cx.lace();
        let c = tones::avatar(style, State::NORMAL, self.tone);
        div()
            .size(px(self.size))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .rounded_full()
            .bg(hsla(c.fill.unwrap()))
            .border_1()
            .border_color(hsla(c.line))
            .text_size(px((self.size * 0.4).max(9.0)))
            .line_height(px(self.size - 2.0))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(hsla(c.text))
            .child(initials(&self.name))
    }
}

// ---------------------------------------------------------------------------
// Alert
// ---------------------------------------------------------------------------

/// A message box inside a page: a tone icon, an optional title and the text,
/// on a quiet tone wash with a tone outline and a stripe down the left edge.
#[derive(IntoElement)]
pub struct Alert {
    tone: Tone,
    title: Option<SharedString>,
    message: SharedString,
}

impl Alert {
    /// An alert in `tone`.
    pub fn new(tone: Tone, message: impl Into<SharedString>) -> Self {
        Self { tone, title: None, message: message.into() }
    }

    /// Bold first line.
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }
}

/// The round tone icon of an alert: a check, a cross, `!` or `i`.
pub fn tone_icon(tone: Tone, fill: gpui_kit::gpui::Hsla, ink: gpui_kit::gpui::Hsla, size: f32) -> impl IntoElement {
    let body = div().size(px(size)).flex_none().rounded_full().bg(fill);
    match tone {
        Tone::Success => body.child(glyph::painter(move |b, window, _| glyph::paint_check(window, b, ink))),
        Tone::Danger => body.child(glyph::painter(move |b, window, _| glyph::paint_cross(window, b, ink, size * 0.4))),
        _ => body.child(glyph::painter(move |b, window, _| {
            // `!` for warnings, `i` for the rest: a bar and a dot, swapped.
            let (cx, cy) = (f32::from(b.center().x), f32::from(b.center().y));
            let h = size * 0.22;
            let w = (size * 0.12).max(1.5);
            let (bar, dot) = if tone == Tone::Warning { ((cy - h * 1.2, cy + h * 0.3), cy + h * 1.1) } else { ((cy - h * 0.3, cy + h * 1.2), cy - h * 1.1) };
            glyph::paint_line(window, (cx, bar.0), (cx, bar.1), w, ink);
            glyph::paint_polyline(window, &[(cx, dot), (cx, dot + 0.01)], w * 1.25, ink);
        })),
    }
}

impl RenderOnce for Alert {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let style = cx.lace();
        let s = State::NORMAL;
        let a = tones::alert(style, s, self.tone);
        let ink = paint::on(style, s, a.accent);
        div()
            .flex()
            .gap(px(8.))
            .w_full()
            .px(px(10.))
            .py(px(8.))
            .rounded(px(style.frame_radius))
            .bg(hsla(a.fill))
            .border_1()
            .border_l(px(3.))
            .border_color(hsla(a.line))
            .text_size(px(style.font_size))
            .text_color(hsla(a.text))
            .child(div().pt(px(1.)).child(tone_icon(self.tone, hsla(a.accent), hsla(ink), 16.0)))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .flex_1()
                    .min_w_0()
                    .when_some(self.title, |d, t| {
                        d.child(div().font_weight(FontWeight::SEMIBOLD).text_color(hsla(a.accent)).child(t))
                    })
                    .child(self.message),
            )
    }
}

// ---------------------------------------------------------------------------
// Link
// ---------------------------------------------------------------------------

type Click = std::rc::Rc<dyn Fn(&mut Window, &mut App)>;

/// A text link: the accent, underlined on hover.
#[derive(IntoElement)]
pub struct Link {
    id: ElementId,
    label: SharedString,
    on_click: Option<Click>,
}

impl Link {
    /// A link reading `label`.
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self { id: id.into(), label: label.into(), on_click: None }
    }

    /// Called on click.
    pub fn on_click(mut self, f: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(std::rc::Rc::new(f));
        self
    }
}

impl RenderOnce for Link {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let ix = Interaction::new(self.id.clone(), window, cx);
        let s = ix.state(window, cx);
        let style = cx.lace().clone();
        let el = div()
            .id(self.id.clone())
            .flex_none()
            .cursor_pointer()
            .text_size(px(style.font_size))
            .text_color(hsla(tones::link(&style, s)))
            .when(s.hovered, |d| d.underline())
            .child(self.label);
        let el = ix.bind(el, false);
        match self.on_click {
            Some(f) => el.on_click(move |_, window, cx| f(window, cx)),
            None => el,
        }
    }
}

// ---------------------------------------------------------------------------
// Separator
// ---------------------------------------------------------------------------

/// A frame line between groups, optionally with a centred caption.
#[derive(IntoElement)]
pub struct Separator {
    vertical: bool,
    label: Option<SharedString>,
}

impl Separator {
    /// A horizontal separator.
    pub fn horizontal() -> Self {
        Self { vertical: false, label: None }
    }

    /// A vertical separator.
    pub fn vertical() -> Self {
        Self { vertical: true, label: None }
    }

    /// Caption in the middle of a horizontal line.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }
}

impl RenderOnce for Separator {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let style = cx.lace();
        let s = State::NORMAL;
        let line = hsla(rules::frame_line(style, s));
        if self.vertical {
            return div().w(px(1.)).h_full().flex_none().bg(line).into_any_element();
        }
        let rule = || div().h(px(1.)).flex_1().bg(line);
        match self.label {
            None => div().h(px(1.)).w_full().flex_none().bg(line).into_any_element(),
            Some(l) => div()
                .flex()
                .items_center()
                .gap(px(8.))
                .w_full()
                .text_size(px(style.small_font_size))
                .text_color(hsla(paint::muted(style, s)))
                .child(rule())
                .child(l)
                .child(rule())
                .into_any_element(),
        }
    }
}

// ---------------------------------------------------------------------------
// Skeleton
// ---------------------------------------------------------------------------

/// Pulse period of a skeleton, s.
const PULSE: f32 = 1.4;

/// A placeholder block that breathes while content loads.
#[derive(IntoElement)]
pub struct Skeleton {
    w: Option<f32>,
    h: f32,
    round: bool,
}

impl Skeleton {
    /// A full-width line of `h` px.
    pub fn line(h: f32) -> Self {
        Self { w: None, h, round: false }
    }

    /// A `w` x `h` block.
    pub fn block(w: f32, h: f32) -> Self {
        Self { w: Some(w), h, round: false }
    }

    /// A circle of diameter `d`.
    pub fn circle(d: f32) -> Self {
        Self { w: Some(d), h: d, round: true }
    }
}

impl RenderOnce for Skeleton {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let style = cx.lace();
        window.request_animation_frame();
        let c = tones::skeleton(style, State::NORMAL, (seconds() % PULSE) / PULSE);
        div()
            .flex_none()
            .h(px(self.h))
            .map(|d| match self.w {
                Some(w) => d.w(px(w)),
                None => d.w_full(),
            })
            .map(|d| if self.round { d.rounded_full() } else { d.rounded(px(style.control_radius.min(self.h / 2.0))) })
            .bg(hsla(c))
    }
}

// ---------------------------------------------------------------------------
// Spinner and progress ring
// ---------------------------------------------------------------------------

/// One spinner turn, s.
const TURN: f32 = 0.9;

/// A busy indicator: an accent arc circling a faint track.
#[derive(IntoElement)]
pub struct Spinner {
    size: f32,
}

impl Spinner {
    /// A spinner `size` px across.
    pub fn new(size: f32) -> Self {
        Self { size }
    }
}

impl Default for Spinner {
    fn default() -> Self {
        Self::new(18.0)
    }
}

impl RenderOnce for Spinner {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let style = cx.lace().clone();
        let size = self.size;
        let paint = canvas(
            |_, _, _| (),
            move |b, (), window, _| {
                window.request_animation_frame();
                let s = State::NORMAL;
                let c = (f32::from(b.center().x), f32::from(b.center().y));
                let pen = (size / 9.0).max(2.0);
                let r = size / 2.0 - pen / 2.0;
                glyph::paint_arc(window, c, r, 0.0, 360.0, pen, hsla(rules::track(&style, s)));
                let start = -360.0 * ((seconds() % TURN) / TURN);
                glyph::paint_arc(window, c, r, start, 100.0, pen, hsla(rules::range_fill(&style, s)));
            },
        )
        .size_full();
        div().size(px(size)).flex_none().child(paint)
    }
}

/// A circular progress indicator, with the percentage in the middle.
#[derive(IntoElement)]
pub struct ProgressRing {
    value: f64,
    size: f32,
    label: bool,
    disabled: bool,
}

impl ProgressRing {
    /// Progress `value` in 0..=1.
    pub fn new(value: f64) -> Self {
        Self { value, size: 40.0, label: true, disabled: false }
    }

    /// Diameter, px.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Hide the percentage.
    pub fn no_label(mut self) -> Self {
        self.label = false;
        self
    }

    /// Disable.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl RenderOnce for ProgressRing {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let style = cx.lace().clone();
        let s = State::NORMAL.with_disabled(self.disabled);
        let size = self.size;
        let frac = self.value.clamp(0.0, 1.0) as f32;
        let text = hsla(style.pal(s).window_text);
        let font = (size * 0.26).clamp(9.0, style.font_size);
        let (track, fill) = (hsla(rules::track(&style, s)), hsla(rules::range_fill(&style, s)));
        let paint = canvas(
            |_, _, _| (),
            move |b, (), window, _| {
                let c = (f32::from(b.center().x), f32::from(b.center().y));
                let pen = (size / 10.0).max(3.0);
                let r = size / 2.0 - pen / 2.0;
                glyph::paint_arc(window, c, r, 0.0, 360.0, pen, track);
                // From 12 o'clock, clockwise.
                glyph::paint_arc(window, c, r, 90.0, -360.0 * frac, pen, fill);
            },
        )
        .absolute()
        .size_full();
        div()
            .relative()
            .size(px(size))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .child(paint)
            .when(self.label, |d| {
                d.child(div().text_size(px(font)).text_color(text).child(format!("{}%", (frac * 100.0).round() as i32)))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::initials;

    #[test]
    fn initials_take_first_and_last_word() {
        assert_eq!(initials("ada lovelace"), "AL");
        assert_eq!(initials("Grace Brewster Hopper"), "GH");
        assert_eq!(initials("Linus"), "L");
        assert_eq!(initials("  "), "");
    }
}
