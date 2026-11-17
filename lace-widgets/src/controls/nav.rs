//! Navigation components: breadcrumbs, pagination and steppers.

use std::rc::Rc;

use gpui_kit::gpui::{
    div, prelude::FluentBuilder, px, relative, App, ElementId, FontWeight, InteractiveElement, IntoElement,
    ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window,
};

use super::button::ToolButton;
use super::interaction::Interaction;
use super::toggle::nth_id;
use crate::glyph::{self, Dir};
use crate::style::{hsla, paint, rules, tones, ActiveStyle, State};

type Pick = Rc<dyn Fn(usize, &mut Window, &mut App)>;

// ---------------------------------------------------------------------------
// Breadcrumb
// ---------------------------------------------------------------------------

/// A path of places: muted, hoverable crumbs split by chevrons, the last
/// (current) one in the text colour.
#[derive(IntoElement)]
pub struct Breadcrumb {
    id: ElementId,
    items: Vec<SharedString>,
    on_select: Option<Pick>,
}

impl Breadcrumb {
    /// A breadcrumb over `items`, outermost first.
    pub fn new(id: impl Into<ElementId>, items: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        Self { id: id.into(), items: items.into_iter().map(Into::into).collect(), on_select: None }
    }

    /// Called with the crumb clicked (never the last).
    pub fn on_select(mut self, f: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for Breadcrumb {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let style = cx.lace().clone();
        let n = self.items.len();
        let mut row = div().flex().flex_none().items_center().gap(px(2.)).text_size(px(style.font_size));
        for (i, label) in self.items.into_iter().enumerate() {
            let last = i + 1 == n;
            let id = nth_id(&self.id, "crumb", i);
            let ix = Interaction::new(id.clone(), window, cx);
            let s = ix.state(window, cx);
            let color = if last {
                style.active.window_text
            } else if s.hovered {
                tones::link(&style, s)
            } else {
                paint::muted(&style, s)
            };
            let el = div()
                .id(id)
                .px(px(3.))
                .rounded(px(style.control_radius.min(4.0)))
                .text_color(hsla(color))
                .when(last, |d| d.font_weight(FontWeight::SEMIBOLD))
                .when(!last, |d| d.cursor_pointer())
                .when(s.hovered && !last, |d| d.underline())
                .child(label);
            let el = ix.bind(el, last);
            let el = match self.on_select.clone() {
                Some(f) if !last => el.on_click(move |_, window, cx| f(i, window, cx)),
                _ => el,
            };
            row = row.child(el);
            if !last {
                let c = hsla(paint::muted(&style, State::NORMAL));
                row = row.child(glyph::glyph_box(12.0, 12.0, move |b, window, _| glyph::paint_chevron(window, b, Dir::Right, c, 6.0)));
            }
        }
        row
    }
}

// ---------------------------------------------------------------------------
// Pagination
// ---------------------------------------------------------------------------

/// Page buttons with previous / next and ellipses: `‹ 1 … 4 5 6 … 20 ›`.
#[derive(IntoElement)]
pub struct Pagination {
    id: ElementId,
    page: usize,
    pages: usize,
    on_select: Option<Pick>,
}

impl Pagination {
    /// Page `page` (0-based) of `pages`.
    pub fn new(id: impl Into<ElementId>, page: usize, pages: usize) -> Self {
        let pages = pages.max(1);
        Self { id: id.into(), page: page.min(pages - 1), pages, on_select: None }
    }

    /// Called with the page to show.
    pub fn on_select(mut self, f: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(f));
        self
    }
}

/// The page numbers to show, `None` for an ellipsis: the first and last
/// pages, and the current one with a neighbour each side.
pub fn page_slots(page: usize, pages: usize) -> Vec<Option<usize>> {
    if pages <= 7 {
        return (0..pages).map(Some).collect();
    }
    let lo = page.saturating_sub(1).max(1);
    let hi = (page + 1).min(pages - 2);
    let (lo, hi) = if page < 3 { (1, 3) } else if page + 3 >= pages { (pages - 4, pages - 2) } else { (lo, hi) };
    let mut out = vec![Some(0)];
    if lo > 1 {
        out.push(None);
    }
    out.extend((lo..=hi).map(Some));
    if hi < pages - 2 {
        out.push(None);
    }
    out.push(Some(pages - 1));
    out
}

impl RenderOnce for Pagination {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let style = cx.lace().clone();
        let muted = hsla(paint::muted(&style, State::NORMAL));
        let ink = hsla(style.active.button_text);
        let button = |id: ElementId, label: String, target: Option<usize>, checked: bool| {
            let b = ToolButton::new(id, label).auto_raise().checked(checked).disabled(target.is_none());
            match (self.on_select.clone(), target) {
                (Some(f), Some(t)) => b.on_click(move |window, cx| f(t, window, cx)),
                _ => b,
            }
        };
        let arrow = |dir: Dir| glyph::glyph_box(12.0, 12.0, move |b, window, _| glyph::paint_chevron(window, b, dir, ink, 7.0));
        let prev = button(nth_id(&self.id, "prev", 0), String::new(), self.page.checked_sub(1), false).icon(arrow(Dir::Left));
        let next = button(nth_id(&self.id, "next", 0), String::new(), (self.page + 1 < self.pages).then_some(self.page + 1), false)
            .icon(arrow(Dir::Right));
        let mut row = div().flex().flex_none().items_center().gap(px(2.)).child(prev);
        for (k, slot) in page_slots(self.page, self.pages).into_iter().enumerate() {
            row = match slot {
                Some(p) => row.child(
                    div().min_w(px(24.)).child(button(nth_id(&self.id, "page", p), (p + 1).to_string(), Some(p), p == self.page)),
                ),
                None => row.child(
                    div()
                        .id(nth_id(&self.id, "gap", k))
                        .w(px(18.))
                        .flex()
                        .justify_center()
                        .text_size(px(style.font_size))
                        .text_color(muted)
                        .child("…"),
                ),
            };
        }
        row.child(next)
    }
}

// ---------------------------------------------------------------------------
// Stepper
// ---------------------------------------------------------------------------

/// Numbered steps joined by a line: done steps show a check, the current
/// one its number on the accent, the rest a groove.
#[derive(IntoElement)]
pub struct Stepper {
    id: ElementId,
    steps: Vec<SharedString>,
    current: usize,
    on_select: Option<Pick>,
}

impl Stepper {
    /// Steps `steps`, `current` being worked on.
    pub fn new(id: impl Into<ElementId>, steps: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        Self { id: id.into(), steps: steps.into_iter().map(Into::into).collect(), current: 0, on_select: None }
    }

    /// The current step; `len()` marks every step done.
    pub fn current(mut self, ix: usize) -> Self {
        self.current = ix;
        self
    }

    /// Called with a step clicked.
    pub fn on_select(mut self, f: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(f));
        self
    }
}

/// Diameter of a stepper node, px.
const NODE: f32 = 22.0;

impl RenderOnce for Stepper {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let style = cx.lace().clone();
        let n = self.steps.len();
        let mut row = div().flex().w_full();
        for (i, label) in self.steps.into_iter().enumerate() {
            let id = nth_id(&self.id, "step", i);
            let ix = Interaction::new(id.clone(), window, cx);
            let s = ix.state(window, cx);
            let (done, here) = (i < self.current, i == self.current);
            let c = tones::step_node(&style, s, done || here);
            let fill = c.fill.unwrap();
            let fill = if s.hovered { paint::state_fill(s, fill) } else { fill };
            let text = hsla(c.text);
            let node = div()
                .size(px(NODE))
                .flex()
                .items_center()
                .justify_center()
                .rounded_full()
                .bg(hsla(fill))
                .border_1()
                .border_color(hsla(c.line))
                .text_size(px(style.small_font_size))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(text)
                .map(|d| {
                    if done {
                        d.child(glyph::painter(move |b, window, _| glyph::paint_check(window, b, text)))
                    } else {
                        d.child((i + 1).to_string())
                    }
                });
            // The line to the next node starts at this node's centre and
            // runs one column across; the nodes paint over its ends.
            let line_color = if done { rules::range_fill(&style, State::NORMAL) } else { rules::track(&style, State::NORMAL) };
            let connector = (i + 1 < n).then(|| {
                div().absolute().top(px(NODE / 2.0 - 1.0)).left(relative(0.5)).w(relative(1.)).h(px(2.)).bg(hsla(line_color))
            });
            let caption = if here || done { style.active.window_text } else { paint::muted(&style, State::NORMAL) };
            let col = div()
                .id(id)
                .relative()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .items_center()
                .gap(px(4.))
                .when(self.on_select.is_some(), |d| d.cursor_pointer())
                .when_some(connector, |d, l| d.child(l))
                .child(node)
                .child(
                    div()
                        .text_size(px(style.small_font_size))
                        .text_color(hsla(caption))
                        .when(here, |d| d.font_weight(FontWeight::SEMIBOLD))
                        .child(label),
                );
            let col = ix.bind(col, false);
            let col = match self.on_select.clone() {
                Some(f) => col.on_click(move |_, window, cx| f(i, window, cx)),
                None => col,
            };
            row = row.child(col);
        }
        row
    }
}

#[cfg(test)]
mod tests {
    use super::page_slots;

    fn show(page: usize, pages: usize) -> String {
        page_slots(page, pages)
            .into_iter()
            .map(|s| s.map_or("…".to_string(), |p| (p + 1).to_string()))
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn slots_compress_long_ranges() {
        assert_eq!(show(0, 5), "1 2 3 4 5");
        assert_eq!(show(0, 20), "1 2 3 4 … 20");
        assert_eq!(show(9, 20), "1 … 9 10 11 … 20");
        assert_eq!(show(19, 20), "1 … 17 18 19 20");
        assert_eq!(page_slots(5, 20).len(), 7);
    }
}
