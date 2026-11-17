//! Colour rules for the components Qt has no class for: switches, tags,
//! badges, alerts, avatars, ratings, steppers, skeletons.
//!
//! They follow LaceStyle's own logic so they sit with the Qt families: a
//! wash of the colour over the window for fills, the colour lifted to the
//! `ui` floor for lines, text lifted to the `text` floor on its fill, the
//! hover / press lightness steps, and the disabled group fading everything
//! toward the window.

use lace_gpui::color::Rgba;

use super::paint::{self as p, State};
use super::rules::TRACK_MIX;
use super::LaceStyle;

/// A colour role for the status components.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tone {
    /// The theme accent.
    #[default]
    Accent,
    /// Text-coloured, for neutral tags and secondary content.
    Neutral,
    /// The theme's success colour.
    Success,
    /// The theme's warning colour.
    Warning,
    /// The theme's error colour.
    Danger,
    /// The theme's info colour.
    Info,
}

/// Share of the tone washed over the window for a tag or alert fill.
pub const WASH: f64 = 0.20;
/// Share for an alert's larger, quieter fill.
pub const ALERT_WASH: f64 = 0.10;

/// The raw colour of `tone`, faded for a disabled control.
pub fn tone_color(style: &LaceStyle, s: State, tone: Tone) -> Rgba {
    let pal = style.pal(s);
    let c = match tone {
        Tone::Accent => return pal.accent,
        Tone::Neutral => pal.text,
        Tone::Success => style.status.success,
        Tone::Warning => style.status.warning,
        Tone::Danger => style.status.error,
        Tone::Info => style.status.info,
    };
    if s.disabled {
        p::mix(pal.window, c, 0.6)
    } else {
        c
    }
}

/// A filled face: fill, outline and the text on it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Chip {
    /// Fill; `None` for an outline-only chip.
    pub fill: Option<Rgba>,
    /// Outline.
    pub line: Rgba,
    /// Text and glyphs.
    pub text: Rgba,
}

/// A tag: the tone washed over the window, or, `outline`, the window with
/// a tone outline. Either way the text is the tone, held to the text floor.
pub fn tag(style: &LaceStyle, s: State, tone: Tone, outline: bool) -> Chip {
    let pal = style.pal(s);
    let c = tone_color(style, s, tone);
    let wash = if tone == Tone::Neutral { WASH * 0.6 } else { WASH };
    let fill = p::state_fill(s, if outline { pal.window } else { p::mix(pal.window, c, wash) });
    let line = p::legible(style, s, if outline { c } else { p::mix(fill, c, 0.5) }, style.ui_ratio(), Some(fill));
    let text = p::legible(style, s, c, style.text_ratio(), Some(fill));
    Chip { fill: (!outline).then_some(fill), line, text }
}

/// A solid face in the tone (a toned button, a badge, a count bubble).
///
/// The label picks between the text and the window colour, so a bright
/// warning or success fill gets dark ink on a dark theme too, and is held
/// to the body-text floor.
pub fn solid(style: &LaceStyle, s: State, tone: Tone) -> Chip {
    let pal = style.pal(s);
    let fill = p::state_fill(s, tone_color(style, s, tone));
    let ratio = if s.disabled { p::DISABLED_RATIO } else { style.text_ratio() };
    let text = lace_gpui::color::on_color(fill, [pal.text, pal.window], ratio);
    Chip { fill: Some(fill), line: fill, text }
}

/// An alert box: a quiet wash, a tone outline, the tone for its icon and
/// title, and the body text in the window's text colour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Alert {
    /// Box fill.
    pub fill: Rgba,
    /// Box outline.
    pub line: Rgba,
    /// Icon and title.
    pub accent: Rgba,
    /// Message text.
    pub text: Rgba,
}

/// An alert in `tone`.
pub fn alert(style: &LaceStyle, s: State, tone: Tone) -> Alert {
    let pal = style.pal(s);
    let c = tone_color(style, s, tone);
    let fill = p::mix(pal.window, c, ALERT_WASH);
    Alert {
        fill,
        line: p::legible(style, s, p::mix(fill, c, 0.55), style.border_ratio(), Some(fill)),
        accent: p::legible(style, s, c, style.text_ratio(), Some(fill)),
        text: p::legible(style, s, pal.window_text, style.text_ratio(), Some(fill)),
    }
}

/// A switch: the track, its outline, the knob and the knob's outline.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Switch {
    /// Track fill.
    pub track: Rgba,
    /// Track outline.
    pub line: Rgba,
    /// Knob fill.
    pub knob: Rgba,
    /// Knob outline.
    pub knob_line: Rgba,
}

/// A switch in state `s`; `checked` turns it on. On, the track is the
/// accent (a checked check box's fill) and the knob the text colour that
/// reads on it; off, the track is a groove and the knob a button face.
pub fn switch(style: &LaceStyle, s: State) -> Switch {
    let pal = style.pal(s);
    if s.checked {
        let track = p::state_fill(s, pal.accent);
        let knob = p::on(style, s, track);
        Switch {
            track,
            line: p::legible(style, s, track, style.ui_ratio(), None),
            knob,
            knob_line: knob,
        }
    } else {
        let track = p::state_fill(s, p::mix(pal.window, pal.text, TRACK_MIX));
        let line = if s.active_hover() {
            p::legible(style, s, pal.accent, style.ui_ratio(), None)
        } else {
            p::legible(style, s, p::stroke(style, s, track), style.border_ratio(), None)
        };
        let knob = pal.button;
        Switch { track, line, knob, knob_line: p::legible(style, s, p::stroke(style, s, knob), style.ui_ratio(), Some(track)) }
    }
}

/// An avatar disc: the accent washed over the button colour, initials in
/// the text colour that reads on it.
pub fn avatar(style: &LaceStyle, s: State, tone: Tone) -> Chip {
    let pal = style.pal(s);
    let fill = p::mix(pal.button, tone_color(style, s, tone), 0.45);
    Chip { fill: Some(fill), line: p::stroke(style, s, fill), text: p::on(style, s, fill) }
}

/// A rating star: filled stars take the warning gold, empty ones the
/// track; a hovered (previewed) star the gold at half strength.
pub fn star(style: &LaceStyle, s: State, on: bool, preview: bool) -> Rgba {
    let pal = style.pal(s);
    let gold = tone_color(style, s, Tone::Warning);
    let empty = p::legible(style, s, p::mix(pal.window, pal.text, 0.30), style.border_ratio(), None);
    if on {
        gold
    } else if preview {
        p::mix(empty, gold, 0.55)
    } else {
        empty
    }
}

/// A stepper node: done steps and the current one take the accent, the
/// rest a groove; the number on it in a colour that reads.
pub fn step_node(style: &LaceStyle, s: State, reached: bool) -> Chip {
    let pal = style.pal(s);
    if reached {
        solid(style, s, Tone::Accent)
    } else {
        let fill = p::mix(pal.window, pal.text, TRACK_MIX);
        Chip { fill: Some(fill), line: p::stroke(style, s, fill), text: p::legible(style, s, pal.text, style.text_ratio(), Some(fill)) }
    }
}

/// A skeleton block at pulse phase `t` (0..1): the track, breathing.
pub fn skeleton(style: &LaceStyle, s: State, t: f32) -> Rgba {
    let pal = style.pal(s);
    let lo = p::mix(pal.window, pal.text, TRACK_MIX * 0.6);
    let hi = p::mix(pal.window, pal.text, TRACK_MIX * 1.3);
    let k = 0.5 - 0.5 * (t * std::f32::consts::TAU).cos();
    color_lerp(lo, hi, k as f64)
}

fn color_lerp(a: Rgba, b: Rgba, t: f64) -> Rgba {
    let t = t.clamp(0.0, 1.0);
    let ch = |x: u8, y: u8| (x as f64 + (y as f64 - x as f64) * t).round() as u8;
    [ch(a[0], b[0]), ch(a[1], b[1]), ch(a[2], b[2]), ch(a[3], b[3])]
}

/// A link: the accent at the text floor, the pressed step while pressed.
pub fn link(style: &LaceStyle, s: State) -> Rgba {
    let c = p::legible(style, s, style.pal(s).accent, style.text_ratio(), None);
    if s.pressed {
        p::step(c, p::PRESS_STEP)
    } else {
        c
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lace_gpui::color::contrast_ratio;

    fn styles() -> Vec<LaceStyle> {
        lace_gpui::themes::ALL.iter().map(|s| LaceStyle::new(s, &lace_gpui::build(s))).collect()
    }

    #[test]
    fn tag_text_reads_on_its_fill() {
        for st in styles() {
            for tone in [Tone::Accent, Tone::Neutral, Tone::Success, Tone::Warning, Tone::Danger, Tone::Info] {
                let c = tag(&st, State::NORMAL, tone, false);
                let r = contrast_ratio(c.text, c.fill.unwrap());
                assert!(r >= 4.4, "{} {tone:?} tag text {r:.2}", st.name);
            }
        }
    }

    #[test]
    fn alert_body_reads() {
        for st in styles() {
            for tone in [Tone::Success, Tone::Warning, Tone::Danger, Tone::Info] {
                let a = alert(&st, State::NORMAL, tone);
                assert!(contrast_ratio(a.text, a.fill) >= 4.4, "{} {tone:?}", st.name);
                assert!(contrast_ratio(a.accent, a.fill) >= 4.4, "{} {tone:?} title", st.name);
            }
        }
    }

    #[test]
    fn solid_label_reads() {
        for st in styles() {
            for tone in [Tone::Accent, Tone::Success, Tone::Warning, Tone::Danger, Tone::Info] {
                let c = solid(&st, State::NORMAL, tone);
                let r = contrast_ratio(c.text, c.fill.unwrap());
                assert!(r >= 4.4, "{} {tone:?} solid label {r:.2}", st.name);
            }
        }
    }

    #[test]
    fn switch_on_differs_from_off() {
        for st in styles() {
            let off = switch(&st, State::NORMAL);
            let on = switch(&st, State::checked());
            assert_ne!(off.track, on.track, "{}", st.name);
            assert!(contrast_ratio(on.knob, on.track) >= 2.9, "{} knob on track", st.name);
        }
    }
}
