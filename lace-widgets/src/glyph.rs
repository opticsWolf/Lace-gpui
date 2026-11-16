//! Vector glyphs — the shapes `lace/style/_paint.py` strokes with QPainter.
//!
//! Each glyph comes twice: a `paint_*` function that draws into a bounds
//! during a canvas paint (for widgets that compose several shapes), and an
//! element constructor that fills its parent (`chevron`, `check_mark`, ...).
//! Strokes use round caps and joins and the 1.5 px glyph pen, as Lace does.

use gpui_kit::gpui::{
    canvas, div, point, px, App, Bounds, Hsla, IntoElement, ParentElement, PathBuilder, PathStyle,
    Pixels, Point, Styled, Window,
};
use lyon_tessellation::{LineCap, LineJoin, StrokeOptions};

use crate::style::paint::GLYPH_PEN;

/// Which way a chevron or triangle points.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    /// Up.
    Up,
    /// Down.
    Down,
    /// Left.
    Left,
    /// Right.
    Right,
}

fn round_stroke(width: f32) -> PathBuilder {
    PathBuilder::stroke(px(width)).with_style(PathStyle::Stroke(
        StrokeOptions::default()
            .with_line_width(width)
            .with_line_cap(LineCap::Round)
            .with_line_join(LineJoin::Round),
    ))
}

fn flat_stroke(width: f32) -> PathBuilder {
    PathBuilder::stroke(px(width)).with_style(PathStyle::Stroke(
        StrokeOptions::default()
            .with_line_width(width)
            .with_line_cap(LineCap::Butt)
            .with_line_join(LineJoin::Miter),
    ))
}

fn pt(x: f32, y: f32) -> Point<Pixels> {
    point(px(x), px(y))
}

fn center(b: Bounds<Pixels>) -> (f32, f32) {
    let c = b.center();
    (f32::from(c.x), f32::from(c.y))
}

fn side(b: Bounds<Pixels>) -> f32 {
    f32::from(b.size.width).min(f32::from(b.size.height))
}

/// A `side` x `side` square centred in `b`.
pub fn centered(b: Bounds<Pixels>, side: f32) -> Bounds<Pixels> {
    let (cx, cy) = center(b);
    Bounds::new(pt(cx - side / 2.0, cy - side / 2.0), gpui_kit::gpui::size(px(side), px(side)))
}

/// Stroke an open polyline through `pts` (window coordinates).
pub fn paint_polyline(window: &mut Window, pts: &[(f32, f32)], width: f32, color: Hsla) {
    if pts.len() < 2 {
        return;
    }
    let mut b = round_stroke(width);
    b.move_to(pt(pts[0].0, pts[0].1));
    for &(x, y) in &pts[1..] {
        b.line_to(pt(x, y));
    }
    if let Ok(path) = b.build() {
        window.paint_path(path, color);
    }
}

/// Stroke one straight line with flat caps (frame lines, ticks).
pub fn paint_line(window: &mut Window, a: (f32, f32), b: (f32, f32), width: f32, color: Hsla) {
    let mut p = flat_stroke(width);
    p.move_to(pt(a.0, a.1));
    p.line_to(pt(b.0, b.1));
    if let Ok(path) = p.build() {
        window.paint_path(path, color);
    }
}

/// Fill a closed polygon.
pub fn paint_polygon(window: &mut Window, pts: &[(f32, f32)], color: Hsla) {
    if pts.len() < 3 {
        return;
    }
    let mut b = PathBuilder::fill();
    b.move_to(pt(pts[0].0, pts[0].1));
    for &(x, y) in &pts[1..] {
        b.line_to(pt(x, y));
    }
    b.close();
    if let Ok(path) = b.build() {
        window.paint_path(path, color);
    }
}

/// Stroke a circular arc around `(cx, cy)`. Angles follow Qt's dial: degrees
/// counter-clockwise from 3 o'clock; the arc runs from `start` by `span`.
pub fn paint_arc(
    window: &mut Window,
    c: (f32, f32),
    radius: f32,
    start: f32,
    span: f32,
    width: f32,
    color: Hsla,
) {
    if span.abs() < 0.01 || radius <= 0.0 {
        return;
    }
    // Small segments keep the tessellated arc smooth and avoid arc_to's
    // large-arc ambiguity.
    let steps = ((span.abs() / 6.0).ceil() as usize).max(2);
    let mut b = round_stroke(width);
    for i in 0..=steps {
        let a = (start + span * i as f32 / steps as f32).to_radians();
        let p = pt(c.0 + radius * a.cos(), c.1 - radius * a.sin());
        if i == 0 {
            b.move_to(p);
        } else {
            b.line_to(p);
        }
    }
    if let Ok(path) = b.build() {
        window.paint_path(path, color);
    }
}

/// A point on a circle, Qt-dial angle convention.
pub fn on_circle(c: (f32, f32), radius: f32, degrees: f32) -> (f32, f32) {
    let a = degrees.to_radians();
    (c.0 + radius * a.cos(), c.1 - radius * a.sin())
}

/// A stroked chevron centred in `b`. `size` 0 picks Lace's default
/// (`min(w, h, 14) * 0.55`, at least 4 px).
pub fn paint_chevron(window: &mut Window, b: Bounds<Pixels>, dir: Dir, color: Hsla, size: f32) {
    let s = if size > 0.0 { size } else { side(b).min(14.0) * 0.55 }.max(4.0);
    let (cx, cy) = center(b);
    let (h, q) = (s / 2.0, s / 4.0);
    let pts = match dir {
        Dir::Down => [(cx - h, cy - q), (cx, cy + q), (cx + h, cy - q)],
        Dir::Up => [(cx - h, cy + q), (cx, cy - q), (cx + h, cy + q)],
        Dir::Right => [(cx - q, cy - h), (cx + q, cy), (cx - q, cy + h)],
        Dir::Left => [(cx + q, cy - h), (cx - q, cy), (cx + q, cy + h)],
    };
    paint_polyline(window, &pts, GLYPH_PEN, color);
}

/// A filled triangle centred in `b` (scrollbar step arrows).
pub fn paint_triangle(window: &mut Window, b: Bounds<Pixels>, dir: Dir, color: Hsla, size: f32) {
    let s = if size > 0.0 { size } else { side(b) * 0.5 };
    let (cx, cy) = center(b);
    let (h, q) = (s / 2.0, s / 4.0);
    let pts = match dir {
        Dir::Down => [(cx - h, cy - q), (cx + h, cy - q), (cx, cy + q)],
        Dir::Up => [(cx - h, cy + q), (cx + h, cy + q), (cx, cy - q)],
        Dir::Right => [(cx - q, cy - h), (cx - q, cy + h), (cx + q, cy)],
        Dir::Left => [(cx + q, cy - h), (cx + q, cy + h), (cx - q, cy)],
    };
    paint_polygon(window, &pts, color);
}

/// A stroked tick inside `b`.
pub fn paint_check(window: &mut Window, b: Bounds<Pixels>, color: Hsla) {
    let (x, y) = (f32::from(b.origin.x), f32::from(b.origin.y));
    let (w, h) = (f32::from(b.size.width), f32::from(b.size.height));
    let pts = [
        (x + w * 0.24, y + h * 0.52),
        (x + w * 0.43, y + h * 0.70),
        (x + w * 0.77, y + h * 0.32),
    ];
    paint_polyline(window, &pts, GLYPH_PEN.max(w * 0.12), color);
}

/// The partially-checked bar.
pub fn paint_dash(window: &mut Window, b: Bounds<Pixels>, color: Hsla) {
    let (x, w) = (f32::from(b.origin.x), f32::from(b.size.width));
    let (_, cy) = center(b);
    paint_polyline(
        window,
        &[(x + w * 0.28, cy), (x + w - w * 0.28, cy)],
        GLYPH_PEN.max(w * 0.12),
        color,
    );
}

/// A stroked `x` (close buttons). `size` 0 = 45% of the side.
pub fn paint_cross(window: &mut Window, b: Bounds<Pixels>, color: Hsla, size: f32) {
    let s = if size > 0.0 { size } else { side(b) * 0.45 };
    let h = s / 2.0;
    let (cx, cy) = center(b);
    paint_polyline(window, &[(cx - h, cy - h), (cx + h, cy + h)], GLYPH_PEN, color);
    paint_polyline(window, &[(cx - h, cy + h), (cx + h, cy - h)], GLYPH_PEN, color);
}

/// A `+` or `-` glyph (spin boxes with plus/minus symbols).
pub fn paint_plus_minus(window: &mut Window, b: Bounds<Pixels>, plus: bool, color: Hsla) {
    let s = side(b) * 0.5;
    let (cx, cy) = center(b);
    paint_polyline(window, &[(cx - s / 2.0, cy), (cx + s / 2.0, cy)], GLYPH_PEN, color);
    if plus {
        paint_polyline(window, &[(cx, cy - s / 2.0), (cx, cy + s / 2.0)], GLYPH_PEN, color);
    }
}

// ---------------------------------------------------------------------------
// Elements
// ---------------------------------------------------------------------------

/// An element that fills its parent and paints with `f`.
pub fn painter(
    f: impl FnOnce(Bounds<Pixels>, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    canvas(|_, _, _| (), move |bounds, (), window, cx| f(bounds, window, cx)).size_full()
}

/// A `w` x `h` box holding a glyph painted by `f`.
pub fn glyph_box(
    w: f32,
    h: f32,
    f: impl FnOnce(Bounds<Pixels>, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div().w(px(w)).h(px(h)).flex_none().child(painter(f))
}

/// A chevron element filling its parent.
pub fn chevron(dir: Dir, color: Hsla, size: f32) -> impl IntoElement {
    painter(move |b, window, _| paint_chevron(window, b, dir, color, size))
}

/// A tick element filling its parent.
pub fn check_mark(color: Hsla) -> impl IntoElement {
    painter(move |b, window, _| paint_check(window, b, color))
}

/// A dash element filling its parent.
pub fn dash(color: Hsla) -> impl IntoElement {
    painter(move |b, window, _| paint_dash(window, b, color))
}

/// A cross element filling its parent.
pub fn cross(color: Hsla, size: f32) -> impl IntoElement {
    painter(move |b, window, _| paint_cross(window, b, color, size))
}
