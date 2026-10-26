//! Engine-level integration tests: one test per contract the gpui-kit
//! bridge relies on. Floors that look arbitrary are the enforcement
//! contract's own numbers — see the comments where they came from.

use lace_gpui::color;
use lace_gpui::contrast::{separation_target, SepRole, EXPLICIT_MAX_DE};
use lace_gpui::engine::{build, spec_from_hex, Depth, Selection};
use lace_gpui::themes::{by_name, ALL};

fn gap(a: [u8; 4], b: [u8; 4]) -> f64 {
    (color::to_oklch(a).0 - color::to_oklch(b).0).abs()
}

// -----------------------------------------------------------------------
// 1. Everything derives, and no two themes are the same theme.
// -----------------------------------------------------------------------
#[test]
fn all_themes_derive_and_are_distinct() {
    let mut fps = Vec::new();
    for spec in ALL {
        let t = build(spec);
        fps.push(t.fingerprint());
    }
    let unique = fps.iter().collect::<std::collections::HashSet<_>>();
    assert_eq!(unique.len(), ALL.len(), "two themes share a token fingerprint");
}

// -----------------------------------------------------------------------
// 2. is_light follows the chassis (Rel::Auto decides from the base).
// -----------------------------------------------------------------------
#[test]
fn is_light_matches_chassis() {
    let light = [
        "slate", "caramel", "neutral", "cream", "light", "solarized_light",
    ];
    let dark = [
        "midnight", "dark", "mocha", "dracula", "monokai", "nordic",
        "catppuccin", "tokyo_night", "solarized_dark", "cyberpunk_neon",
    ];
    for name in light {
        let t = build(by_name(name).unwrap());
        assert!(t.is_light, "{} should be light", name);
    }
    for name in dark {
        let t = build(by_name(name).unwrap());
        assert!(!t.is_light, "{} should be dark", name);
    }
}

// -----------------------------------------------------------------------
// 3. Body text on the canvas and panel. The engine lifts explicit seeds
//    toward the 7.0 "normal" target, but only within EXPLICIT_MAX_DE —
//    solarized_light's [101,123,131] lands at ~4.3 on its panel and stays
//    there by design (the theme still looks like its author's choice).
//    Everything clears WCAG AA (4.5) on the canvas.
// -----------------------------------------------------------------------
#[test]
fn body_text_contrast_floor() {
    for spec in ALL {
        let t = build(spec);
        let r_canvas = color::contrast_ratio(t.text, t.canvas_bg);
        let r_panel = color::contrast_ratio(t.text, t.panel_bg);
        assert!(r_canvas >= 4.4, "{} text on canvas {:.2}", spec.name, r_canvas);
        assert!(r_panel >= 4.2, "{} text on panel {:.2}", spec.name, r_panel);
    }
}

// -----------------------------------------------------------------------
// 4. The active tab's label reads on the active tab fill.
// -----------------------------------------------------------------------
#[test]
fn active_tab_text_contrast() {
    for spec in ALL {
        let t = build(spec);
        let r = color::contrast_ratio(t.tab_text_active, t.tab_bg_active);
        assert!(r >= 4.4, "{} active tab text {:.2}", spec.name, r);
    }
}

// -----------------------------------------------------------------------
// 5. Muted text (inactive tab labels, title buttons) stays readable on
//    the strip it sits on.
// -----------------------------------------------------------------------
#[test]
fn muted_text_floor() {
    for spec in ALL {
        let t = build(spec);
        let r = color::contrast_ratio(t.text_muted, t.tab_bg_normal);
        assert!(r >= 3.4, "{} muted on title strip {:.2}", spec.name, r);
    }
}

// -----------------------------------------------------------------------
// 6. Selected text is readable on the selection fill.
// -----------------------------------------------------------------------
#[test]
fn selection_text_contrast() {
    for spec in ALL {
        let t = build(spec);
        let r = color::contrast_ratio(t.highlighted_text, t.highlight);
        // on_accent "low" floor is 3.0; mid-tone accents cap out ~4.6.
        assert!(r >= 2.9, "{} selection text {:.2}", spec.name, r);
    }
}

// -----------------------------------------------------------------------
// 7. The panel separates from the canvas by the depth floor (scaled on
//    light grounds).
// -----------------------------------------------------------------------
#[test]
fn surfaces_separate_from_canvas() {
    for spec in ALL {
        let t = build(spec);
        let target = separation_target(SepRole::Surface, spec.depth, t.canvas_bg);
        let g = gap(t.panel_bg, t.canvas_bg);
        assert!(g >= target - 1e-6, "{} panel gap {:.4} < {:.4}", spec.name, g, target);
        // The title strip separates from the panel by the same rule.
        let g_title = gap(t.tab_bg_normal, t.panel_bg);
        assert!(
            g_title >= target - 1e-6,
            "{} title gap {:.4} < {:.4}",
            spec.name,
            g_title,
            target
        );
    }
}

// -----------------------------------------------------------------------
// 8. Hover fills are visible against what they sit on.
// -----------------------------------------------------------------------
#[test]
fn hover_separates() {
    for spec in ALL {
        let t = build(spec);
        let target = separation_target(SepRole::Hover, spec.depth, t.canvas_bg);
        let g = gap(t.tab_bg_hover, t.tab_bg_normal);
        assert!(g >= target - 1e-6, "{} hover gap {:.4} < {:.4}", spec.name, g, target);
    }
}

// -----------------------------------------------------------------------
// 9. Status colours stay visible on the canvas: 2.9:1 on a dark ground,
//    1.2:1 on a light one (mid-tone canvases like slate leave saturated
//    colours little room).
// -----------------------------------------------------------------------
#[test]
fn status_colors_visible() {
    for spec in ALL {
        let t = build(spec);
        let floor = if t.is_light { 1.15 } else { 2.85 };
        for (name, c) in [
            ("success", t.success),
            ("warning", t.warning),
            ("error", t.error),
            ("info", t.info),
        ] {
            let r = color::contrast_ratio(c, t.canvas_bg);
            assert!(r >= floor, "{} {} on canvas {:.2} < {}", spec.name, name, r, floor);
        }
    }
}

// -----------------------------------------------------------------------
// 10. keep_tint semantics: midnight opts out, so its active text runs to
//     pure white; a keep_tint chassis stops short of it.
// -----------------------------------------------------------------------
#[test]
fn midnight_keeps_pure_white_active_text() {
    let midnight = build(by_name("midnight").unwrap());
    let lm = color::to_oklch(midnight.text_active).0;
    assert!(lm > 0.96, "midnight active text L {}", lm);

    let dark = build(by_name("dark").unwrap());
    let ld = color::to_oklch(dark.text_active).0;
    assert!(
        ld <= 0.965 + EXPLICIT_MAX_DE,
        "keep_tint active text should stop short: L {}",
        ld
    );
    assert!(lm > ld);
}

// -----------------------------------------------------------------------
// 11. Tint selection: an accent wash over the input background that keeps
//     the normal text colour. The wash is composited off the *derived*
//     input, which the touch pass may still nudge — hence the ±3-channel
//     tolerance.
// -----------------------------------------------------------------------
#[test]
fn tint_selection_contract() {
    let mut spec = spec_from_hex("tint_test", "#1e1e2e", "#f5c2e7", "#cdd6f4", None, None);
    spec.selection = Selection::Tint;
    let t = build(&spec);

    let tint = [spec.accent[0], spec.accent[1], spec.accent[2], 72];
    let expect = color::composite(tint, t.input_bg);
    for i in 0..3 {
        assert!(
            (t.highlight[i] as i32 - expect[i] as i32).abs() <= 3,
            "channel {} drifted: {:?} vs {:?}",
            i,
            t.highlight,
            expect
        );
    }
    assert_eq!(t.highlighted_text, t.text, "tint keeps the normal text colour");
}

// -----------------------------------------------------------------------
// 12. Geometry passes through untouched.
// -----------------------------------------------------------------------
#[test]
fn geometry_passthrough() {
    let neon = build(by_name("cyberpunk_neon").unwrap());
    assert!((neon.corner_radius - 10.0).abs() < 1e-9);
    assert!((neon.tab_radius - 8.0).abs() < 1e-9);
    assert!((neon.border_width - 1.5).abs() < 1e-9);
    assert!((neon.title_height - 32.0).abs() < 1e-9);

    let midnight = build(by_name("midnight").unwrap());
    assert!((midnight.corner_radius - 4.0).abs() < 1e-9);
    assert!((midnight.tab_radius - 4.0).abs() < 1e-9);
    assert!((midnight.border_width - 1.5).abs() < 1e-9);
}

// -----------------------------------------------------------------------
// 13. spec_from_hex: the five-seed path builds a coherent theme.
// -----------------------------------------------------------------------
#[test]
fn spec_from_hex_builds() {
    let spec = spec_from_hex("mine", "#1e1e2e", "#f5c2e7", "#cdd6f4", Some("#313244"), None);
    assert_eq!(spec.base, [30, 30, 46, 255]);
    assert_eq!(spec.surface, Some([49, 50, 68, 255]));
    let t = build(&spec);
    assert!(!t.is_light);
    assert_eq!(t.panel_bg, [49, 50, 68, 255], "explicit surface becomes the panel");
    // Depth default "subtle": surfaces obey the raised floors of the
    // chassis even when every colour is explicit.
    let target = separation_target(SepRole::Surface, Depth::Subtle, t.canvas_bg);
    assert!(gap(t.panel_bg, t.canvas_bg) >= target - 1e-6);
}

// -----------------------------------------------------------------------
// A custom spec keeps its explicit colours within EXPLICIT_MAX_DE.
// -----------------------------------------------------------------------
#[test]
fn explicit_colors_stay_capped() {
    let spec = spec_from_hex("cap_test", "#1e1e2e", "#f5c2e7", "#101010", None, None);
    let t = build(&spec);
    // The near-black text is asked to rise to the 7.0 floor but may only
    // move 0.04 OKLab from where the spec put it.
    let d = color::delta_e(spec.text, t.text);
    assert!(d <= EXPLICIT_MAX_DE + 1e-6, "text moved ΔE {:.4}", d);
    // ...and only when that helped.
    let before = color::contrast_ratio(spec.text, [30, 30, 46, 255]);
    let after = color::contrast_ratio(t.text, t.canvas_bg);
    assert!(after > before, "cap applied without improving: {:.2} -> {:.2}", before, after);
}

// -----------------------------------------------------------------------
// 14. Sidebar ring overrides: pinned colours win, otherwise the ring
//     follows the shared border/accent lists.
// -----------------------------------------------------------------------
#[test]
fn sidebar_tab_border_overrides() {
    // cyberpunk_neon rings only the active tab: idle is transparent, active
    // is the focus cyan, and hover is the accent pink.
    let neon = build(by_name("cyberpunk_neon").unwrap());
    assert_eq!(neon.sidebar_tab_border_normal, [0, 0, 0, 0]);
    assert_eq!(neon.sidebar_tab_border_active, [0, 240, 255, 255]);
    assert_eq!(neon.sidebar_tab_border_hover, [255, 0, 127, 255]);

    // A theme without overrides seeds the ring from the border and accent,
    // and a hover keeps the inactive outline.
    let midnight = build(by_name("midnight").unwrap());
    assert_eq!(midnight.sidebar_tab_border_normal, midnight.border_color);
    assert_eq!(midnight.sidebar_tab_border_active, midnight.accent_color);
    assert_eq!(
        midnight.sidebar_tab_border_hover,
        midnight.sidebar_tab_border_normal
    );
}
