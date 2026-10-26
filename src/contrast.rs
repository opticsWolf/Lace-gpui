//! What each theme token is drawn on, and how far it must stand out.
//!
//! Rust port of Lace's `lace/theme_contrast.py` (Apache-2.0, (c) opticsWolf).
//! Two kinds of rule, both measured on a finished [`Tokens`]:
//!
//! - **contrast** (WCAG ratio): a text or UI token against the surface it is
//!   painted on, with a floor per [`Contrast`](crate::engine::Contrast) level.
//! - **separation** (OKLCH lightness): a surface against the one it sits on,
//!   so that a title bar, input field or hover is visible at all. The floor
//!   is set by the [`Depth`](crate::engine::Depth) level.
//!
//! [`enforce`] fixes a theme in place; it runs at the end of
//! [`build`](crate::build). Colours the spec set explicitly may only move
//! within [`EXPLICIT_MAX_DE`], and only when that improves the rule they
//! miss — derived colours move as far as they need to.
//!
//! One structural note: in Lace several style categories share a single
//! colour list (the title background is both `TAB.bg_normal` and
//! `TITLE_BAR.bg_normal`; the panel is `PANEL.bg_normal`, `TAB.bg_active` and
//! `SIDEPANEL.bg_normal` at once), so a fix written into the list follows
//! every token that uses it. This port flattens the theme into fields, and
//! [`Ref::set`] resyncs the mirrors wherever a rule writes — same net effect.

use crate::color::{self, Rgba};
use crate::engine::{Contrast, Depth, Explicit, ThemeSpec, Tokens};

/// How far (OKLab ΔE) the fixer may move a colour the theme set explicitly.
/// Enough to lift a near-miss, small enough that the theme still looks like
/// its author's choice.
pub const EXPLICIT_MAX_DE: f64 = 0.04;

/// The "surface" floor on a light canvas, as a share of the one above.
/// Near white there is only ~0.02 of lightness left above a light panel, and
/// a light theme wants three steps in it — a white field, the panel, a title
/// bar barely off it — set apart by their frames and rules rather than by
/// fill alone. Hovers keep their full floor.
pub const LIGHT_SURFACE_SCALE: f64 = 0.75;

/// WCAG floors per role and contrast level: [low, normal, high].
mod targets {
    pub const TEXT: [f64; 3] = [4.5, 7.0, 10.0];
    pub const MUTED: [f64; 3] = [3.0, 4.5, 7.0];
    pub const DISABLED: [f64; 3] = [1.8, 2.3, 3.0];
    pub const UI: [f64; 3] = [1.5, 3.0, 4.5];
    pub const BORDER: [f64; 3] = [1.15, 1.3, 1.6];
    /// Text on a mid-tone accent can't reach body-text floors (the best any
    /// colour manages on such a fill is ~4.6:1), so selections get their own.
    pub const ON_ACCENT: [f64; 3] = [3.0, 4.5, 7.0];
}

/// The role a contrast rule plays; picks the floor from `targets`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Text,
    Muted,
    Disabled,
    Ui,
    Border,
    OnAccent,
}

pub fn contrast_target(role: Role, level: Contrast) -> f64 {
    let table = match role {
        Role::Text => targets::TEXT,
        Role::Muted => targets::MUTED,
        Role::Disabled => targets::DISABLED,
        Role::Ui => targets::UI,
        Role::Border => targets::BORDER,
        Role::OnAccent => targets::ON_ACCENT,
    };
    table[level.index()]
}

/// Minimum OKLCH lightness gap between a surface and its parent, per depth.
mod sep_targets {
    pub const SURFACE: [f64; 3] = [0.012, 0.02, 0.035];
    pub const HOVER: [f64; 3] = [0.03, 0.04, 0.06];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SepRole {
    Surface,
    Hover,
}

/// The lightness floor for `role` at `depth` on `canvas`.
pub fn separation_target(role: SepRole, depth: Depth, canvas: Rgba) -> f64 {
    let mut t = match role {
        SepRole::Surface => sep_targets::SURFACE,
        SepRole::Hover => sep_targets::HOVER,
    }[depth.index()];
    if role == SepRole::Surface && !color::is_dark(canvas) {
        t *= LIGHT_SURFACE_SCALE;
    }
    t
}

/// The canvas floor for [`TOUCH_PAIRS`]: smaller than the parent floor, so
/// only surfaces that all but vanish into the canvas move (midnight's input
/// at ΔL 0.001), not every theme that sits a little close.
pub fn touch_target(depth: Depth) -> f64 {
    [0.008, 0.012, 0.02][depth.index()]
}

/// A reference to one colour token (canonical field) in [`Tokens`].
///
/// `get` reads the canonical field; `set` writes it **and** every mirror
/// that shared the same colour list in Lace. This is what makes a fix
/// written to one token follow every token that means the same colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ref {
    CanvasBg,
    PanelBg,
    TabBgNormal,
    InputBg,
    ButtonBg,
    TooltipBg,
    Highlight,
    TabBgHover,
    TitleButtonHover,
    SidepanelButtonHover,
    Text,
    TextMuted,
    TextActive,
    DisabledText,
    /// The shared action-button glyph colour (`_btn_disabled`): side-panel
    /// buttons, tab close buttons and title-bar buttons all use it.
    ButtonDisabled,
    TooltipText,
    TabCloseBtn,
    FocusBorder,
    AccentColor,
    AccentBright,
    BorderColor,
    HighlightedText,
    // Status colours (visibility pass).
    Success,
    Warning,
    Error,
    Info,
}

impl Ref {
    pub fn get(self, t: &Tokens) -> Rgba {
        match self {
            Ref::CanvasBg => t.canvas_bg,
            Ref::PanelBg => t.panel_bg,
            Ref::TabBgNormal => t.tab_bg_normal,
            Ref::InputBg => t.input_bg,
            Ref::ButtonBg => t.button_bg,
            Ref::TooltipBg => t.tooltip_bg,
            Ref::Highlight => t.highlight,
            Ref::TabBgHover => t.tab_bg_hover,
            Ref::TitleButtonHover => t.title_button_hover,
            Ref::SidepanelButtonHover => t.sidepanel_button_hover,
            Ref::Text => t.text,
            Ref::TextMuted => t.text_muted,
            Ref::TextActive => t.text_active,
            Ref::DisabledText => t.disabled_text,
            Ref::ButtonDisabled => t.title_button_disabled,
            Ref::TooltipText => t.tooltip_text,
            Ref::TabCloseBtn => t.tab_close_btn,
            Ref::FocusBorder => t.focus_border_color,
            Ref::AccentColor => t.accent_color,
            Ref::AccentBright => t.accent_bright,
            Ref::BorderColor => t.border_color,
            Ref::HighlightedText => t.highlighted_text,
            Ref::Success => t.success,
            Ref::Warning => t.warning,
            Ref::Error => t.error,
            Ref::Info => t.info,
        }
    }

    pub fn set(self, t: &mut Tokens, c: Rgba, explicit: &Explicit) {
        match self {
            Ref::CanvasBg => {
                t.canvas_bg = c;
                t.sidebar_bg = c;
                t.splitter_handle = c;
            }
            Ref::PanelBg => {
                t.panel_bg = c;
                t.tab_bg_active = c;
                t.sidepanel_bg = c;
                t.overlay_background = c;
                t.sidebar_tab_bg_active = c;
            }
            Ref::TabBgNormal => t.tab_bg_normal = c,
            Ref::InputBg => t.input_bg = c,
            Ref::ButtonBg => t.button_bg = c,
            Ref::TooltipBg => t.tooltip_bg = c,
            Ref::Highlight => t.highlight = c,
            Ref::TabBgHover => {
                t.tab_bg_hover = c;
                t.sidebar_tab_bg_hover_start = c;
            }
            Ref::TitleButtonHover => t.title_button_hover = c,
            Ref::SidepanelButtonHover => {
                t.sidepanel_button_hover = c;
                t.tab_close_btn_hover = c;
            }
            Ref::Text => {
                t.text = c;
                t.sidepanel_title_text = c;
                t.overlay_arrow = c;
                // Lace's tooltip text defaults to the same list as `text`;
                // an explicit tooltip text is the only thing that opts out.
                if !explicit.tooltip_text {
                    t.tooltip_text = c;
                }
            }
            Ref::TextMuted => {
                t.text_muted = c;
                t.tab_text_normal = c;
                t.title_text_normal = c;
                t.title_button_color = c;
                t.sidepanel_button_color = c;
                t.sidebar_tab_text_normal = c;
                t.sidebar_badge_text = c;
            }
            Ref::TextActive => {
                t.text_active = c;
                t.tab_text_active = c;
                t.title_text_active = c;
                t.sidebar_tab_text_active = c;
            }
            Ref::DisabledText => {
                t.disabled_text = c;
                t.sidebar_tab_text_disabled = c;
            }
            Ref::ButtonDisabled => {
                // One list in Lace (`_btn_disabled`), shared by all three.
                t.title_button_disabled = c;
                t.sidepanel_button_disabled = c;
                t.tab_close_btn_disabled = c;
            }
            Ref::TooltipText => t.tooltip_text = c,
            Ref::TabCloseBtn => t.tab_close_btn = c,
            Ref::FocusBorder => {
                t.focus_border_color = c;
                t.sidepanel_focus_border = c;
                t.title_focus_border = c;
                // Lace's `_focus_border` *is* the border list when the spec
                // leaves it unset, so a focus fix moves the edges too.
                if !explicit.focus_border_color && explicit.border_color {
                    t.border_color = c;
                    t.sidebar_tab_border_normal = c;
                    t.sidepanel_border = c;
                    t.tab_border_normal = c;
                    t.title_border_color = c;
                }
            }
            Ref::AccentColor => {
                t.accent_color = c;
                // Lace seeds `PANEL.highlight` with the same list as the
                // accent, so contrast fixes to the accent carry into the
                // selection fill (and from there into its selected text).
                t.highlight = c;
                t.sidebar_tab_border_active = c;
                t.sidebar_indicator = c;
                t.sidebar_badge_bg = c;
                t.tab_border_active = c;
                t.tab_indicator = c;
                t.splitter_handle_hover = c;
                // `_accent_dim` is a follower: it takes the accent's new RGB
                // but keeps its own 25% alpha.
                let overlay_alpha = t.overlay_color[3];
                t.overlay_color = [c[0], c[1], c[2], overlay_alpha];
                let dim_alpha = t.accent_dim[3];
                t.accent_dim = [c[0], c[1], c[2], dim_alpha];
            }
            Ref::AccentBright => {
                t.accent_bright = c;
                t.title_active_edge = c;
                t.overlay_frame = c;
            }
            Ref::BorderColor => {
                t.border_color = c;
                t.sidebar_tab_border_normal = c;
                t.sidepanel_border = c;
                // Lace's `_neutral_border` list is also the dock-tab outline.
                t.tab_border_normal = c;
                t.title_border_color = c;
                // ...and the focus border when it was derived from the
                // border list (spec had a border but no focus colour).
                if !explicit.focus_border_color && explicit.border_color {
                    t.focus_border_color = c;
                    t.sidepanel_focus_border = c;
                    t.title_focus_border = c;
                }
            }
            Ref::HighlightedText => t.highlighted_text = c,
            Ref::Success => t.success = c,
            Ref::Warning => t.warning = c,
            Ref::Error => t.error = c,
            Ref::Info => t.info = c,
        }
    }

    /// The `Explicit` flag governing this token, if any.
    fn is_explicit(self, e: &Explicit) -> bool {
        match self {
            Ref::CanvasBg => e.canvas_bg,
            Ref::PanelBg => e.panel_bg,
            Ref::TabBgNormal => e.tab_bg_normal,
            Ref::TooltipBg => e.tooltip_bg,
            Ref::Text => e.text,
            Ref::TooltipText => e.tooltip_text,
            Ref::FocusBorder => e.focus_border_color,
            Ref::AccentColor => e.accent_color,
            Ref::BorderColor => e.border_color,
            Ref::Success => e.success,
            Ref::Warning => e.warning,
            Ref::Error => e.error,
            Ref::Info => e.info,
            _ => false,
        }
    }
}

/// (role, foreground, surface). Order matters: enforced top to bottom, and
/// the accent's `on_accent` rule comes last because the `ui` rules may have
/// nudged the accent it sits on.
const CONTRAST_PAIRS: &[(Role, Ref, Ref)] = &[
    (Role::Text, Ref::Text, Ref::CanvasBg),
    (Role::Text, Ref::Text, Ref::PanelBg),
    // Panel text is also drawn on buttons and fields.
    (Role::Text, Ref::Text, Ref::ButtonBg),
    (Role::Text, Ref::Text, Ref::InputBg),
    // Active tab/title/sidebar text is one colour in Lace.
    (Role::Text, Ref::TextActive, Ref::PanelBg),
    (Role::Text, Ref::TextActive, Ref::TabBgNormal),
    (Role::Text, Ref::TooltipText, Ref::TooltipBg),
    (Role::Muted, Ref::TextMuted, Ref::TabBgNormal),
    (Role::Muted, Ref::TextMuted, Ref::PanelBg),
    (Role::Muted, Ref::TextMuted, Ref::CanvasBg),
    (Role::Muted, Ref::TabCloseBtn, Ref::PanelBg),
    (Role::Disabled, Ref::DisabledText, Ref::PanelBg),
    (Role::Disabled, Ref::DisabledText, Ref::CanvasBg),
    // Lace pairs `_btn_disabled` with the title strip, not with the disabled
    // text; the fix then reaches its side-panel and tab mirrors.
    (Role::Disabled, Ref::ButtonDisabled, Ref::TabBgNormal),
    (Role::Ui, Ref::FocusBorder, Ref::CanvasBg),
    (Role::Ui, Ref::AccentColor, Ref::TabBgNormal),
    (Role::Ui, Ref::AccentColor, Ref::CanvasBg),
    (Role::Ui, Ref::AccentBright, Ref::TabBgNormal),
    (Role::Border, Ref::BorderColor, Ref::PanelBg),
    // Last: the accent it sits on may have been nudged by the "ui" rules.
    (Role::OnAccent, Ref::HighlightedText, Ref::Highlight),
];

/// (role, surface, parent). The surface is the one moved when too close.
const SEPARATION_PAIRS: &[(SepRole, Ref, Ref)] = &[
    (SepRole::Surface, Ref::PanelBg, Ref::CanvasBg),
    (SepRole::Surface, Ref::TabBgNormal, Ref::PanelBg),
    (SepRole::Surface, Ref::InputBg, Ref::PanelBg),
    (SepRole::Surface, Ref::ButtonBg, Ref::PanelBg),
    (SepRole::Surface, Ref::TooltipBg, Ref::PanelBg),
    (SepRole::Hover, Ref::TabBgHover, Ref::TabBgNormal),
    (SepRole::Hover, Ref::TabBgHover, Ref::CanvasBg),
    (SepRole::Hover, Ref::TitleButtonHover, Ref::TabBgNormal),
    (SepRole::Hover, Ref::SidepanelButtonHover, Ref::PanelBg),
];

/// (surface, far side, own parent): a derived surface that also touches the
/// canvas (a title bar in a gap, an input at a card's edge). Kept apart from
/// it by the same lightness floor, but only by moving *further from its own
/// parent*. Never flipped across the parent or the canvas.
const TOUCH_PAIRS: &[(Ref, Ref, Ref)] = &[
    (Ref::TabBgNormal, Ref::CanvasBg, Ref::PanelBg),
    (Ref::InputBg, Ref::CanvasBg, Ref::PanelBg),
];

fn lightness_gap(a: Rgba, b: Rgba) -> f64 {
    (color::to_oklch(a).0 - color::to_oklch(b).0).abs()
}

fn opaque(bg: Rgba, canvas: Rgba) -> Rgba {
    if bg[3] < 255 {
        color::composite(bg, canvas)
    } else {
        bg
    }
}

/// Snapshot of the explicit tokens' original values, so a capped colour's
/// allowed moves are measured against where it started.
struct Origin {
    values: Vec<(Ref, Rgba)>,
}

impl Origin {
    fn new(tokens: &Tokens, explicit: &Explicit) -> Self {
        let refs = [
            Ref::CanvasBg,
            Ref::PanelBg,
            Ref::TabBgNormal,
            Ref::TooltipBg,
            Ref::Text,
            Ref::TooltipText,
            Ref::FocusBorder,
            Ref::AccentColor,
            Ref::BorderColor,
            Ref::Success,
            Ref::Warning,
            Ref::Error,
            Ref::Info,
        ];
        let mut values: Vec<(Ref, Rgba)> = refs
            .iter()
            .filter(|r| r.is_explicit(explicit))
            .map(|r| (*r, r.get(tokens)))
            .collect();
        // When the spec omits a focus border, Lace derives it from the
        // border list and both tokens share that list's explicit origin.
        if !explicit.focus_border_color && explicit.border_color {
            values.push((Ref::FocusBorder, tokens.border_color));
        }
        Origin { values }
    }

    fn of(&self, r: Ref) -> Option<Rgba> {
        self.values.iter().find(|(o, _)| *o == r).map(|(_, c)| *c)
    }
}

/// Where `value` may go on its way to `want`: free for derived colours,
/// capped to [`EXPLICIT_MAX_DE`] (straight toward the fix, plus the two pure
/// lightness ends of its hue) for explicit ones.
fn options(origin: &Origin, r: Ref, want: Rgba, alpha: u8) -> Vec<Rgba> {
    match origin.of(r) {
        None => vec![want],
        Some(o) => {
            let (_, c, h) = color::to_oklch(o);
            vec![
                color::toward(o, want, EXPLICIT_MAX_DE),
                color::toward(o, color::from_oklch_a(1.0, c, h, alpha), EXPLICIT_MAX_DE),
                color::toward(o, color::from_oklch_a(0.0, c, h, alpha), EXPLICIT_MAX_DE),
            ]
        }
    }
}

/// `s` moved just past `target` from `far`, away from `own`; or `None`.
///
/// `None` when the move would cross `far` (it lies on the other side of `s`
/// from `own`; getting past it would be a flip) or rounds to no change at
/// black or white.
fn touch_fix(s: Rgba, far: Rgba, own: Rgba, target: f64) -> Option<Rgba> {
    let (ls, cs, hs) = color::to_oklch(s);
    let lf = color::to_oklch(far).0;
    let lo = color::to_oklch(own).0;
    let sign: f64 = if ls > lo { 1.0 } else { -1.0 };
    // `far` must not sit clearly further along the way we move: getting past
    // it would be a flip. Level with it (under the floor) is the case this
    // rule exists for.
    if (lf - ls) * sign > target {
        return None;
    }
    let base = if sign > 0.0 { ls.max(lf) } else { ls.min(lf) };
    let mut aim = target + 0.004 - (base - lf).abs();
    for _ in 0..40 {
        let want = color::from_oklch_a(base + sign * aim.max(0.0), cs, hs, s[3]);
        if lightness_gap(want, far) >= target {
            return Some(want);
        }
        aim += 0.004;
        let next = base + sign * aim;
        if !(0.0 < next && next < 1.0) {
            return None;
        }
    }
    None
}

/// Fix `tokens` in place so it meets the rules.
///
/// Surfaces are separated first, then foregrounds are fixed against the
/// final surfaces. Explicit colours stay within [`EXPLICIT_MAX_DE`] of their
/// original value however many rules touch them, and only move when that
/// improves the rule they miss; they may still fall short.
pub fn enforce(tokens: &mut Tokens, spec: &ThemeSpec, explicit: &Explicit) {
    let canvas = tokens.canvas_bg;
    let origin = Origin::new(tokens, explicit);
    let depth = spec.depth;

    // Touching the canvas: a small nudge, derived surfaces only, never a
    // flip. First, so the parent rules below see the result.
    for (surface, far, own) in TOUCH_PAIRS {
        let s = surface.get(tokens);
        if surface.is_explicit(explicit) || s[3] < 255 {
            continue;
        }
        let f = far.get(tokens);
        let o = own.get(tokens);
        let target = touch_target(depth);
        if lightness_gap(s, f) >= target {
            continue;
        }
        if let Some(want) = touch_fix(s, f, o, target) {
            surface.set(tokens, want, explicit);
        }
    }

    // Separation: group the pairs by surface (a shared hover sits on the
    // title bar and the sidebar at once, and a fix for one side must not
    // undo the other).
    for group in separation_groups() {
        let (role, s_ref, parent_ref) = group[0];
        let s = s_ref.get(tokens);
        if s[3] < 255 {
            continue;
        }
        let target = separation_target(role, depth, canvas);
        let p = parent_ref.get(tokens);
        let gap = lightness_gap(s, p);
        if gap >= target {
            continue;
        }
        // Every separation rule this surface takes part in.
        let rules: Vec<(f64, Rgba)> = group
            .iter()
            .map(|(r, _, par)| (separation_target(*r, depth, canvas), par.get(tokens)))
            .collect();
        let failing = |c: Rgba| -> usize {
            rules.iter().filter(|(t, q)| lightness_gap(c, *q) < *t).count()
        };
        let (ls, cs, hs) = color::to_oklch(s);
        let lp = color::to_oklch(p).0;
        let first: f64 = if gap > 0.002 {
            if ls > lp { 1.0 } else { -1.0 }
        } else if lp < color::DARK_L {
            1.0
        } else {
            -1.0
        };
        // Nearest colour past the floor on `sign`'s side of the parent. A
        // little past, so 8-bit rounding can't land it short; and near black
        // small OKLCH steps round back to the same colour, so widen until
        // the rounded colour clears.
        let reach = |sign: f64| -> Rgba {
            let mut aim = target + 0.004;
            let mut want = color::from_oklch_a(lp + sign * aim, cs, hs, s[3]);
            while lightness_gap(want, p) < target && aim < 0.3 {
                aim += 0.004;
                want = color::from_oklch_a(lp + sign * aim, cs, hs, s[3]);
            }
            want
        };
        // The side it is already on first; the other side when that clamps
        // at black or white, or breaks a sibling rule.
        let cands = [reach(first), reach(-first)];
        let want = cands
            .into_iter()
            .min_by(|a, b| {
                let ka = (
                    (lightness_gap(*a, p) < target) as u8,
                    failing(*a),
                    (color::delta_e(*a, s) * 1e6) as u64,
                );
                let kb = (
                    (lightness_gap(*b, p) < target) as u8,
                    failing(*b),
                    (color::delta_e(*b, s) * 1e6) as u64,
                );
                ka.cmp(&kb)
            })
            .unwrap();
        let best = options(&origin, s_ref, want, s[3])
            .into_iter()
            .max_by_key(|c| (lightness_gap(*c, p) * 1e9) as u64)
            .unwrap();
        if lightness_gap(best, p) > gap {
            s_ref.set(tokens, best, explicit);
        }
    }

    // Contrast: group the pairs by foreground (several tokens share one
    // colour in Lace; a fix must hold on every surface it is drawn on).
    for group in contrast_groups() {
        let fg_ref = group.fg;
        let fg = fg_ref.get(tokens);
        if fg[3] == 0 {
            continue;
        }
        let rules: Vec<(f64, Rgba)> = group
            .pairs
            .iter()
            .map(|(role, _, surface)| {
                (
                    contrast_target(*role, spec.contrast),
                    opaque(surface.get(tokens), canvas),
                )
            })
            .collect();
        if rules.iter().all(|(t, q)| color::contrast_ratio(fg, *q) >= *t) {
            continue;
        }
        // Anchor on the first rule the colour currently misses (Lace loops
        // the pairs one at a time; the first failing one drives the fix).
        let anchor = rules
            .iter()
            .find(|(t, q)| color::contrast_ratio(fg, *q) < *t)
            .copied()
            .unwrap();
        let unreadable = |c: Rgba| -> usize {
            rules.iter().filter(|(t, q)| color::contrast_ratio(c, *q) < *t).count()
        };
        let (anchor_target, anchor_bg) = anchor;
        let now = color::contrast_ratio(fg, anchor_bg);
        // A hair past each floor, so the audit's rounding can't read it short.
        let mut wants: Vec<Rgba> = rules
            .iter()
            .map(|(t, q)| color::ensure_contrast(fg, *q, t + 0.02))
            .collect();
        // The nearest fix for one surface can land in the other's dead zone:
        // chain the fixes so each is held against every surface.
        for _ in 0..2 {
            for (t, q) in &rules {
                let last = *wants.last().unwrap();
                wants.push(color::ensure_contrast(last, *q, t + 0.02));
            }
        }
        // The two ends of its hue too: on mid-tone surfaces the one colour
        // readable on all of them is often near black or near white.
        let (_, cf, hf) = color::to_oklch(fg);
        for end in [1.0_f64, 0.0] {
            wants.push(color::from_oklch_a(end, cf, hf, fg[3]));
        }
        let cands: Vec<Rgba> = wants
            .iter()
            .flat_map(|w| options(&origin, fg_ref, *w, fg[3]))
            .collect();
        // Among the allowed moves, the one closest to the target that passes
        // everywhere, else the one that fails fewest and gets furthest here.
        let best = cands
            .into_iter()
            .min_by(|a, b| {
                let key = |c: Rgba| -> (usize, bool, i64) {
                    let unread = unreadable(c);
                    let third = if unread == 0 {
                        (color::delta_e(c, fg) * 1e9) as i64
                    } else {
                        -((color::contrast_ratio(c, anchor_bg) * 1e9) as i64)
                    };
                    (unread, color::contrast_ratio(c, anchor_bg) < anchor_target, third)
                };
                key(*a).cmp(&key(*b))
            })
            .unwrap();
        if color::contrast_ratio(best, anchor_bg) > now && unreadable(best) <= unreadable(fg) {
            fg_ref.set(tokens, best, explicit);
        }
    }

    // Lace's tooltip text defaults to the same list as `text`, so it inherits
    // the text colour's explicit cap rather than being fixed independently.
    // An explicit `tooltip_text` is the only case that keeps its own value.
    if !explicit.tooltip_text {
        tokens.tooltip_text = tokens.text;
    }

    status_pass(tokens, explicit);
}

/// Port-specific floor (documented in the README's fidelity notes): every
/// status colour must stay visible on the canvas — 2.9:1 on a dark canvas,
/// 1.2:1 on a light one, where mid-tone grounds leave saturated colours
/// little room. Moved only when the move helps, and capped for explicit
/// colours like everything else.
fn status_pass(tokens: &mut Tokens, explicit: &Explicit) {
    let canvas = tokens.canvas_bg;
    let panel = tokens.panel_bg;
    let bgs = [canvas, panel];
    let target = if color::is_dark(canvas) { 2.9 } else { 1.2 };
    for r in [Ref::Success, Ref::Warning, Ref::Error, Ref::Info] {
        let orig = r.get(tokens);
        let orig_min = bgs
            .iter()
            .map(|b| color::contrast_ratio(orig, *b))
            .fold(f64::INFINITY, f64::min);
        let orig_failing = bgs
            .iter()
            .filter(|b| color::contrast_ratio(orig, **b) < target)
            .count();
        if orig_failing == 0 {
            continue;
        }
        let is_explicit = r.is_explicit(explicit);
        let mut cands = vec![orig];
        for k in 1..=30 {
            let stepped = color::step_contrast(orig, 0.02 * k as f64);
            cands.push(if is_explicit {
                color::toward(orig, stepped, EXPLICIT_MAX_DE)
            } else {
                stepped
            });
        }
        let score = |c: &Rgba| -> (usize, u8, u64) {
            let failing = bgs
                .iter()
                .filter(|b| color::contrast_ratio(*c, **b) < target)
                .count();
            let min_ratio = bgs
                .iter()
                .map(|b| color::contrast_ratio(*c, *b))
                .fold(f64::INFINITY, f64::min);
            // An explicit colour never ends up worse than the spec left it.
            let worse = (min_ratio < orig_min - 1e-9) as u8;
            (failing, worse, (color::delta_e(orig, *c) * 1e6) as u64)
        };
        if let Some(best) = cands.iter().copied().min_by_key(|c| score(c)) {
            let (failing, _, _) = score(&best);
            if failing < orig_failing {
                r.set(tokens, best, explicit);
            }
        }
    }
}

/// Separation pairs grouped by the surface they move (Lace groups by shared
/// colour list).
fn separation_groups() -> Vec<&'static [(SepRole, Ref, Ref)]> {
    // TabBgHover appears twice (title bar + sidebar canvas); everything else
    // is its own group in this port.
    vec![
        &SEPARATION_PAIRS[0..1], // panel vs canvas
        &SEPARATION_PAIRS[1..2], // title vs panel
        &SEPARATION_PAIRS[2..3], // input vs panel
        &SEPARATION_PAIRS[3..4], // button vs panel
        &SEPARATION_PAIRS[4..5], // tooltip vs panel
        &SEPARATION_PAIRS[5..7], // hover vs title, vs sidebar canvas
        &SEPARATION_PAIRS[7..8], // title button hover
        &SEPARATION_PAIRS[8..9], // sidepanel button hover
    ]
}

/// Contrast pairs grouped by foreground token.
struct ContrastGroup {
    fg: Ref,
    pairs: &'static [(Role, Ref, Ref)],
}

fn contrast_groups() -> Vec<ContrastGroup> {
    let mut groups: Vec<ContrastGroup> = Vec::new();
    for (i, (_, fg, _)) in CONTRAST_PAIRS.iter().enumerate() {
        if let Some(existing) = groups.iter_mut().find(|grp| grp.fg == *fg) {
            // Same foreground again: extend its slice of pairs to here.
            let start = CONTRAST_PAIRS
                .iter()
                .position(|(_, f, _)| *f == existing.fg)
                .unwrap();
            existing.pairs = &CONTRAST_PAIRS[start..=i];
        } else {
            groups.push(ContrastGroup { fg: *fg, pairs: &CONTRAST_PAIRS[i..=i] });
        }
    }
    groups
}

/// Audit: every rule the theme misses at the given levels, as `(kind, token,
/// value, target)` rows. Reports only — `enforce` is what fixes.
pub fn audit(tokens: &Tokens, spec: &ThemeSpec) -> Vec<(&'static str, String, f64, f64)> {
    let canvas = tokens.canvas_bg;
    let mut out = Vec::new();
    for (role, fg_ref, surface) in CONTRAST_PAIRS {
        let fg = fg_ref.get(tokens);
        if fg[3] == 0 {
            continue;
        }
        let bg = opaque(surface.get(tokens), canvas);
        let target = contrast_target(*role, spec.contrast);
        let ratio = color::contrast_ratio(fg, bg);
        if ratio < target - 1e-6 {
            out.push(("contrast", format!("{:?}", fg_ref), ratio, target));
        }
    }
    for (role, surface, parent) in SEPARATION_PAIRS {
        let s = opaque(surface.get(tokens), canvas);
        let p = opaque(parent.get(tokens), canvas);
        let target = separation_target(*role, spec.depth, canvas);
        let gap = lightness_gap(s, p);
        if gap < target - 1e-6 {
            out.push(("separation", format!("{:?}", surface), gap, target));
        }
    }
    out
}
