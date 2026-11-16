//! Hover / press / keyboard-focus tracking for LaceStyle widgets.

use std::sync::Arc;

use gpui_kit::gpui::{
    App, Div, ElementId, Entity, FocusHandle, InteractiveElement, MouseButton, SharedString,
    Stateful, StatefulInteractiveElement, Window,
};

use crate::style::State;

#[derive(Clone, Copy, Debug, Default)]
struct Track {
    hovered: bool,
    pressed: bool,
}

/// A widget's interaction state, kept across frames under its element id.
///
/// ```ignore
/// let ix = Interaction::new(self.id.clone(), window, cx).focusable(window, cx).force(self.forced);
/// let s = ix.state(window, cx).with_disabled(self.disabled);
/// ix.bind(div().id(self.id), self.disabled) // hover, press, focus wired
/// ```
#[derive(Clone)]
pub struct Interaction {
    track: Entity<Track>,
    focus: Option<FocusHandle>,
    forced: Option<State>,
}

/// `id` with a `label` child, for a widget's secondary parts.
pub fn child_id(id: &ElementId, label: &'static str) -> ElementId {
    ElementId::NamedChild(Arc::new(id.clone()), SharedString::new_static(label))
}

impl Interaction {
    /// The tracked state for `id`, created on first use.
    pub fn new(id: impl Into<ElementId>, window: &mut Window, cx: &mut App) -> Self {
        let track = window.use_keyed_state(id.into(), cx, |_, _| Track::default());
        Self { track, focus: None, forced: None }
    }

    /// Give the widget a focus handle (kept with its state), so it takes
    /// keyboard focus and Tab reaches it.
    pub fn focusable(mut self, id: &ElementId, window: &mut Window, cx: &mut App) -> Self {
        let handle = window.use_keyed_state(child_id(id, "focus"), cx, |_, cx| cx.focus_handle());
        self.focus = Some(handle.read(cx).clone());
        self
    }

    /// Use an existing focus handle (a text field's, say) instead.
    pub fn with_focus(mut self, handle: FocusHandle) -> Self {
        self.focus = Some(handle);
        self
    }

    /// Replace the tracked state with `state` (gallery rendering).
    pub fn force(mut self, state: Option<State>) -> Self {
        self.forced = state;
        self
    }

    /// The focus handle, if any.
    pub fn focus_handle(&self) -> Option<&FocusHandle> {
        self.focus.as_ref()
    }

    /// Whether the widget holds focus (mouse or keyboard).
    pub fn is_focused(&self, window: &Window) -> bool {
        self.forced.map(|s| s.focused).unwrap_or_else(|| {
            self.focus.as_ref().is_some_and(|f| f.is_focused(window))
        })
    }

    /// The state to paint: forced, or hover / press / keyboard focus as tracked.
    pub fn state(&self, window: &Window, cx: &App) -> State {
        if let Some(s) = self.forced {
            return s;
        }
        let t = *self.track.read(cx);
        let focused = self.focus.as_ref().is_some_and(|f| f.is_focused(window))
            && window.last_input_was_keyboard();
        State { hovered: t.hovered, pressed: t.pressed && t.hovered, focused, ..State::NORMAL }
    }

    /// Wire hover, press and focus tracking onto `el`. A disabled or forced
    /// widget gets no tracking (and no focus), so it never changes state.
    pub fn bind(&self, el: Stateful<Div>, disabled: bool) -> Stateful<Div> {
        if disabled || self.forced.is_some() {
            return el;
        }
        let hover = self.track.clone();
        let down = self.track.clone();
        let up = self.track.clone();
        let out = self.track.clone();
        let el = el
            .on_hover(move |h, _, cx| {
                hover.update(cx, |t, cx| {
                    if t.hovered != *h {
                        t.hovered = *h;
                        cx.notify();
                    }
                })
            })
            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                down.update(cx, |t, cx| {
                    t.pressed = true;
                    cx.notify();
                })
            })
            .on_mouse_up(MouseButton::Left, move |_, _, cx| {
                up.update(cx, |t, cx| {
                    t.pressed = false;
                    cx.notify();
                })
            })
            .on_mouse_up_out(MouseButton::Left, move |_, _, cx| {
                out.update(cx, |t, cx| {
                    if t.pressed {
                        t.pressed = false;
                        cx.notify();
                    }
                })
            });
        match &self.focus {
            Some(f) => el.track_focus(f).tab_stop(true),
            None => el,
        }
    }
}
