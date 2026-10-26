//! The 16 curated themes.
//!
//! Palettes ported one for one from Lace's `lace/dock_custom_theme.py`
//! (Apache-2.0, (c) opticsWolf): the eight Basics in the classic 4px chassis
//! (dark to light: midnight, dark, mocha, slate, caramel, neutral, cream,
//! light), the seven Editor Classics on Lace's stock chassis, and
//! cyberpunk_neon with its 10px/8px neon geometry.

use crate::color::rgba;
use crate::engine::{Contrast, Depth, HoverMode, Rel, Scrollbar, Selection, ThemeSpec, TitleMode};

/// Lace's classic Basics hold inactive edges at this alpha
/// (`_BASIC_CLASSIC_IDLE_ALPHA`), so the focused area's accent frame is the
/// line the eye finds.
const CLASSIC_IDLE_ALPHA: u8 = 85;

/// Lace's classic Basics chassis (`_BASIC_CLASSIC` + `_basic_spec`): 4px
/// corners, faint outlines at 0.08, darker hovers, and every inactive edge
/// held at alpha 85 (the idle alpha) so the focused area's accent frame is
/// the line the eye finds.
pub const BASIC_CLASSIC: ThemeSpec = ThemeSpec {
    name: "",
    contrast: Contrast::Normal,
    depth: Depth::Subtle,
    selection: Selection::Solid,
    scrollbar: Scrollbar::Expanding,
    is_light: Rel::Auto,
    title_mode: TitleMode::Darker,
    hover_mode: HoverMode::Darker,
    keep_tint: true,
    control_radius: 4.0,
    corner_radius: 4.0,
    tab_radius: 4.0,
    border_width: 1.5,
    outline_strength: 0.08,
    focus_width: 2.0,
    title_height: 30.0,
    ..ThemeSpec::DEFAULT
};

/// Editor Classics ride Lace's stock chassis: the colours are theirs and the
/// geometry is Lace's default — tabs highlight lighter, keep_tint off, the
/// 2px card outline off.
const EDITOR_CLASSIC: ThemeSpec = ThemeSpec {
    hover_mode: HoverMode::Lighter,
    ..ThemeSpec::DEFAULT
};

// ===========================================================================
// BASICS (classic chassis)
// ===========================================================================

/// Deep blue-black; its active tab and title text stay pure white
/// (`keep_tint = false`).
pub const MIDNIGHT: ThemeSpec = ThemeSpec {
    name: "midnight",
    base: rgba(11, 13, 20, 255),
    surface: Some(rgba(17, 21, 31, 255)),
    border: Some(rgba(25, 30, 42, CLASSIC_IDLE_ALPHA)),
    accent: rgba(55, 95, 226, 255),
    text: rgba(206, 211, 224, 255),
    focus_border: Some(rgba(55, 95, 226, 255)),
    keep_tint: false,
    ..BASIC_CLASSIC
};

/// After VS Code Dark Modern; dark, neutral and light share one blue.
pub const DARK: ThemeSpec = ThemeSpec {
    name: "dark",
    base: rgba(18, 19, 20, 255),
    surface: Some(rgba(25, 26, 27, 255)),
    border: Some(rgba(37, 38, 39, CLASSIC_IDLE_ALPHA)),
    accent: rgba(0, 120, 212, 255),
    text: rgba(202, 204, 207, 255),
    focus_border: Some(rgba(0, 120, 212, 255)),
    title_mode: TitleMode::Lighter,
    ..BASIC_CLASSIC
};

/// The warm family's dark end, with its elevated title bar.
pub const MOCHA: ThemeSpec = ThemeSpec {
    name: "mocha",
    base: rgba(36, 30, 27, 255),
    surface: Some(rgba(46, 39, 35, 255)),
    border: Some(rgba(60, 51, 46, CLASSIC_IDLE_ALPHA)),
    accent: rgba(200, 110, 60, 255),
    text: rgba(235, 225, 210, 255),
    focus_border: Some(rgba(200, 110, 60, 255)),
    title_mode: TitleMode::Lighter,
    ..BASIC_CLASSIC
};

/// Mid tone, cool blue-grey, with a title bar lifted off the panel.
pub const SLATE: ThemeSpec = ThemeSpec {
    name: "slate",
    base: rgba(150, 156, 166, 255),
    surface: Some(rgba(168, 174, 184, 255)),
    border: Some(rgba(132, 138, 148, CLASSIC_IDLE_ALPHA)),
    accent: rgba(22, 70, 150, 255),
    text: rgba(22, 26, 34, 255),
    focus_border: Some(rgba(22, 70, 150, 255)),
    is_light: Rel::Light,
    title_mode: TitleMode::Lighter,
    ..BASIC_CLASSIC
};

/// Slate amber's machine grey and burnt amber, its greys and text given
/// about a third more chroma at the same lightness.
pub const CARAMEL: ThemeSpec = ThemeSpec {
    name: "caramel",
    base: rgba(197, 194, 189, 255),
    surface: Some(rgba(217, 214, 207, 255)),
    border: Some(rgba(167, 162, 151, CLASSIC_IDLE_ALPHA)),
    accent: rgba(186, 98, 0, 255),
    text: rgba(39, 34, 28, 255),
    focus_border: Some(rgba(186, 98, 0, 255)),
    title_bg: Some(rgba(207, 204, 197, 255)),
    is_light: Rel::Light,
    ..BASIC_CLASSIC
};

/// The former neutral with its blue cast taken out.
pub const NEUTRAL: ThemeSpec = ThemeSpec {
    name: "neutral",
    base: rgba(190, 191, 194, 255),
    surface: Some(rgba(210, 211, 214, 255)),
    border: Some(rgba(170, 171, 175, CLASSIC_IDLE_ALPHA)),
    accent: rgba(6, 108, 196, 255),
    text: rgba(30, 33, 40, 255),
    focus_border: Some(rgba(6, 108, 196, 255)),
    title_bg: Some(rgba(199, 200, 203, 255)),
    is_light: Rel::Light,
    ..BASIC_CLASSIC
};

/// The warm family's light end.
pub const CREAM: ThemeSpec = ThemeSpec {
    name: "cream",
    base: rgba(240, 235, 225, 255),
    surface: Some(rgba(252, 249, 243, 255)),
    border: Some(rgba(226, 219, 207, CLASSIC_IDLE_ALPHA)),
    accent: rgba(176, 92, 0, 255),
    text: rgba(48, 40, 32, 255),
    focus_border: Some(rgba(176, 92, 0, 255)),
    is_light: Rel::Light,
    ..BASIC_CLASSIC
};

/// After VS Code Light Modern, and nearly neutral: three light steps held
/// apart by the frame line rather than by fill alone.
pub const LIGHT: ThemeSpec = ThemeSpec {
    name: "light",
    base: rgba(238, 238, 241, 255),
    surface: Some(rgba(249, 249, 252, 255)),
    border: Some(rgba(225, 225, 228, CLASSIC_IDLE_ALPHA)),
    accent: rgba(0, 102, 192, 255),
    text: rgba(45, 47, 52, 255),
    focus_border: Some(rgba(0, 102, 192, 255)),
    title_bg: Some(rgba(243, 243, 246, 255)),
    is_light: Rel::Light,
    ..BASIC_CLASSIC
};

// ===========================================================================
// EDITOR CLASSICS (stock chassis)
// ===========================================================================

/// High-contrast dark theme with vibrant purple highlights.
pub const DRACULA: ThemeSpec = ThemeSpec {
    name: "dracula",
    base: rgba(40, 42, 54, 255),
    accent: rgba(189, 147, 249, 255),
    text: rgba(248, 248, 242, 255),
    surface: Some(rgba(68, 71, 90, 255)),
    border: Some(rgba(35, 36, 43, 255)),
    focus_border: Some(rgba(189, 147, 249, 255)),
    success: Some(rgba(80, 250, 123, 255)),
    warning: Some(rgba(241, 250, 140, 255)),
    error: Some(rgba(255, 85, 85, 255)),
    info: Some(rgba(139, 233, 253, 255)),
    ..EDITOR_CLASSIC
};

/// Classic dev look, high pop. Intense focus on the code/content area:
/// darker hovers, like the Basics.
pub const MONOKAI: ThemeSpec = ThemeSpec {
    name: "monokai",
    base: rgba(28, 26, 29, 255),
    accent: rgba(255, 216, 102, 255),
    text: rgba(248, 248, 242, 255),
    surface: Some(rgba(39, 40, 34, 255)),
    border: Some(rgba(20, 19, 21, 255)),
    hover_mode: HoverMode::Darker,
    ..EDITOR_CLASSIC
};

/// Frosty and crisp.
pub const NORDIC: ThemeSpec = ThemeSpec {
    name: "nordic",
    base: rgba(40, 46, 58, 255),
    accent: rgba(136, 192, 208, 255),
    text: rgba(236, 239, 244, 255),
    surface: Some(rgba(46, 52, 64, 255)),
    border: Some(rgba(31, 35, 43, 255)),
    border_width: 0.0,
    ..EDITOR_CLASSIC
};

/// Soothing pastel dark aesthetic.
pub const CATPPUCCIN: ThemeSpec = ThemeSpec {
    name: "catppuccin",
    base: rgba(30, 30, 46, 255),
    accent: rgba(203, 166, 247, 255),
    text: rgba(205, 214, 244, 255),
    surface: Some(rgba(49, 50, 68, 255)),
    border: Some(rgba(30, 30, 46, 128)),
    focus_border: Some(rgba(203, 166, 247, 128)),
    success: Some(rgba(166, 227, 161, 255)),
    warning: Some(rgba(249, 226, 175, 255)),
    error: Some(rgba(243, 139, 168, 255)),
    info: Some(rgba(137, 180, 250, 255)),
    ..EDITOR_CLASSIC
};

/// Clean neon-accented dark theme.
pub const TOKYO_NIGHT: ThemeSpec = ThemeSpec {
    name: "tokyo_night",
    base: rgba(26, 27, 38, 255),
    accent: rgba(122, 162, 247, 255),
    text: rgba(192, 202, 245, 255),
    surface: Some(rgba(36, 40, 59, 255)),
    border: Some(rgba(26, 27, 38, 255)),
    focus_border: Some(rgba(82, 122, 182, 255)),
    success: Some(rgba(158, 206, 106, 255)),
    warning: Some(rgba(224, 175, 104, 255)),
    error: Some(rgba(247, 118, 142, 255)),
    info: Some(rgba(125, 207, 255, 255)),
    ..EDITOR_CLASSIC
};

/// Precision low-eye-strain teal/cyan dark palette.
pub const SOLARIZED_DARK: ThemeSpec = ThemeSpec {
    name: "solarized_dark",
    base: rgba(0, 43, 54, 255),
    accent: rgba(38, 139, 210, 255),
    text: rgba(131, 148, 150, 255),
    surface: Some(rgba(7, 54, 66, 255)),
    border: Some(rgba(0, 30, 38, 255)),
    success: Some(rgba(133, 153, 0, 255)),
    warning: Some(rgba(181, 137, 0, 255)),
    error: Some(rgba(220, 50, 47, 255)),
    info: Some(rgba(42, 161, 152, 255)),
    border_width: 0.0,
    ..EDITOR_CLASSIC
};

/// Precision low-eye-strain warm cream light palette.
pub const SOLARIZED_LIGHT: ThemeSpec = ThemeSpec {
    name: "solarized_light",
    base: rgba(253, 246, 227, 255),
    accent: rgba(38, 139, 210, 255),
    text: rgba(101, 123, 131, 255),
    surface: Some(rgba(238, 232, 213, 255)),
    border: Some(rgba(211, 203, 183, 255)),
    success: Some(rgba(133, 153, 0, 255)),
    warning: Some(rgba(181, 137, 0, 255)),
    error: Some(rgba(220, 50, 47, 255)),
    info: Some(rgba(42, 161, 152, 255)),
    is_light: Rel::Light,
    hover_mode: HoverMode::Darker,
    border_width: 0.0,
    ..EDITOR_CLASSIC
};

// ===========================================================================
// NEON
// ===========================================================================

/// Vibrant, ultra-contrasty: deep cyber indigo, electric neon pink, glowing
/// cyan structure, and the only geometry that breaks the stock chassis —
/// 10px cards, 8px tab tops.
pub const CYBERPUNK_NEON: ThemeSpec = ThemeSpec {
    name: "cyberpunk_neon",
    base: rgba(14, 11, 28, 255),
    accent: rgba(255, 0, 127, 255),
    text: rgba(245, 245, 255, 255),
    surface: Some(rgba(24, 19, 44, 255)),
    border: Some(rgba(0, 180, 205, 205)),
    focus_border: Some(rgba(0, 240, 255, 255)),
    sidebar_tab_border: Some(rgba(0, 0, 0, 0)),
    sidebar_tab_border_active: Some(rgba(0, 240, 255, 255)),
    sidebar_tab_border_hover: Some(rgba(255, 0, 127, 255)),
    success: Some(rgba(57, 255, 20, 255)),
    warning: Some(rgba(255, 215, 0, 255)),
    error: Some(rgba(255, 42, 109, 255)),
    info: Some(rgba(5, 217, 232, 255)),
    corner_radius: 10.0,
    tab_radius: 8.0,
    border_width: 1.5,
    title_height: 32.0,
    ..EDITOR_CLASSIC
};

/// Every curated theme, in Lace's dark-to-light order.
pub const ALL: [&ThemeSpec; 16] = [
    &MIDNIGHT,
    &DARK,
    &MOCHA,
    &SLATE,
    &CARAMEL,
    &NEUTRAL,
    &CREAM,
    &LIGHT,
    &DRACULA,
    &MONOKAI,
    &NORDIC,
    &CATPPUCCIN,
    &TOKYO_NIGHT,
    &SOLARIZED_DARK,
    &SOLARIZED_LIGHT,
    &CYBERPUNK_NEON,
];

/// Look a theme up by name, normalizing case, `-` and spaces to `_`.
/// Returns the first match in [`ALL`] order.
pub fn by_name(name: &str) -> Option<&'static ThemeSpec> {
    let norm: String = name
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|c| if c == '-' || c == ' ' { '_' } else { c })
        .collect();
    ALL.iter().copied().find(|t| t.name == norm)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_normalizes() {
        assert!(by_name("midnight").is_some());
        assert!(by_name("Midnight").is_some());
        assert!(by_name("tokyo-night").is_some());
        assert!(by_name("Tokyo Night").is_some());
        assert!(by_name("solarized_dark").is_some());
        assert!(by_name("nope").is_none());
    }
}
