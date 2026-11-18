//! The 37 curated themes.
//!
//! Palettes ported one for one from Lace's `lace/dock_custom_theme.py`
//! (Apache-2.0, (c) opticsWolf): the eight Basics in the classic 4px chassis
//! (dark to light: midnight, dark, mocha, slate, caramel, neutral, cream,
//! light) and again on the 10px "Neo" chassis, the seven Editor Classics on
//! Lace's stock chassis, and the two Neons (cyberpunk_neon, neon_dusk) with
//! their 10px/8px geometry, and the twelve Edge Treatments (cyberpunk_edge,
//! violet_haze, midnight_haze, slate_amber, each dark / neutral / light).
//! Neo, Neon and the 10px Edge families also round their controls and
//! frames further (6px / 8px), which Lace's presets do not.

use crate::color::rgba;
use crate::engine::{
    Contrast, Depth, HoverMode, Rel, Scrollbar, Selection, SideTabFlat, TabIndicator, ThemeSpec,
    TitleMode,
};

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
    tab_dimming: false,
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
    tab_dimming: false,
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
    tab_dimming: false,
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
    tab_dimming: false,
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
    tab_indicator: TabIndicator::None,
    tab_dimming: false,
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
    tab_dimming: false,
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
    tab_dimming: false,
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
    sidebar_tab_outline: 1.5,
    sidebar_tab_flat: SideTabFlat::None,
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
    control_radius: NEO_CONTROL_RADIUS,
    frame_radius: Some(NEO_FRAME_RADIUS),
    ..EDITOR_CLASSIC
};

/// cyberpunk_neon's neon on dracula's softer indigo: no card outline, a
/// darker tab strip so the outlines read, the same 10px/8px geometry.
pub const NEON_DUSK: ThemeSpec = ThemeSpec {
    name: "neon_dusk",
    tab_indicator: TabIndicator::None,
    sidebar_tab_outline: 2.0,
    sidebar_tab_flat: SideTabFlat::Outward,
    base: rgba(34, 36, 48, 255),
    accent: rgba(255, 92, 170, 255),
    text: rgba(240, 240, 248, 255),
    surface: Some(rgba(50, 53, 70, 255)),
    title_bg: Some(rgba(24, 26, 36, 255)),
    border: Some(rgba(98, 114, 164, 200)),
    focus_border: Some(rgba(139, 233, 253, 255)),
    sidebar_tab_border: Some(rgba(0, 0, 0, 0)),
    sidebar_tab_border_hover: Some(rgba(98, 114, 164, 160)),
    success: Some(rgba(80, 250, 123, 255)),
    warning: Some(rgba(241, 250, 140, 255)),
    error: Some(rgba(255, 85, 85, 255)),
    info: Some(rgba(139, 233, 253, 255)),
    corner_radius: 8.0,
    tab_radius: 8.0,
    border_width: 0.0,
    title_height: 32.0,
    control_radius: NEO_CONTROL_RADIUS,
    frame_radius: Some(NEO_FRAME_RADIUS),
    title_rule: 2.0,
    tab_outline: 2.0,
    tab_outline_color: Some(rgba(98, 114, 164, 160)),
    tab_outline_active: Some(rgba(255, 92, 170, 255)),
    ..EDITOR_CLASSIC
};

// ===========================================================================
// BASICS NEO (the 10px card chassis)
// ===========================================================================

/// Control radius of the large-card chassis (Neo, Neon), px. Lace keeps
/// every preset's controls at 4px; the gpui port rounds them with the cards.
pub const NEO_CONTROL_RADIUS: f64 = 6.0;
/// Frame radius of the large-card chassis, px.
pub const NEO_FRAME_RADIUS: f64 = 8.0;

/// Lace's `_BASIC_NEO`: the Basics' colours on 10px cards and 8px tabs, a
/// full-strength unfocused frame, lighter hovers, the stock outline weight.
pub const BASIC_NEO: ThemeSpec = ThemeSpec {
    name: "",
    title_mode: TitleMode::Darker,
    hover_mode: HoverMode::Lighter,
    keep_tint: true,
    control_radius: NEO_CONTROL_RADIUS,
    frame_radius: Some(NEO_FRAME_RADIUS),
    corner_radius: 10.0,
    tab_radius: 8.0,
    border_width: 1.5,
    title_height: 32.0,
    tab_indicator_width: 1.5,
    ..ThemeSpec::DEFAULT
};

/// Midnight on the Neo chassis.
pub const MIDNIGHT_NEO: ThemeSpec = ThemeSpec {
    name: "midnight_neo",
    sidebar_tab_outline: 1.5,
    sidebar_tab_flat: SideTabFlat::None,
    base: rgba(11, 13, 20, 255),
    surface: Some(rgba(17, 21, 31, 255)),
    border: Some(rgba(25, 30, 42, 255)),
    accent: rgba(55, 95, 226, 255),
    text: rgba(206, 211, 224, 255),
    focus_border: Some(rgba(55, 95, 226, 255)),
    sidebar_tab_border: Some(rgba(25, 30, 42, 255)),
    sidebar_tab_border_active: Some(rgba(55, 95, 226, 255)),
    sidebar_tab_border_hover: Some(rgba(55, 95, 226, 160)),
    keep_tint: false,
    ..BASIC_NEO
};

/// Dark on the Neo chassis.
pub const DARK_NEO: ThemeSpec = ThemeSpec {
    name: "dark_neo",
    sidebar_tab_outline: 1.5,
    sidebar_tab_flat: SideTabFlat::None,
    base: rgba(18, 19, 20, 255),
    surface: Some(rgba(25, 26, 27, 255)),
    border: Some(rgba(37, 38, 39, 255)),
    accent: rgba(0, 120, 212, 255),
    text: rgba(202, 204, 207, 255),
    focus_border: Some(rgba(0, 120, 212, 255)),
    sidebar_tab_border: Some(rgba(37, 38, 39, 255)),
    sidebar_tab_border_active: Some(rgba(0, 120, 212, 255)),
    sidebar_tab_border_hover: Some(rgba(0, 120, 212, 160)),
    title_mode: TitleMode::Lighter,
    ..BASIC_NEO
};

/// Mocha on the Neo chassis.
pub const MOCHA_NEO: ThemeSpec = ThemeSpec {
    name: "mocha_neo",
    sidebar_tab_outline: 1.5,
    sidebar_tab_flat: SideTabFlat::None,
    base: rgba(36, 30, 27, 255),
    surface: Some(rgba(46, 39, 35, 255)),
    border: Some(rgba(60, 51, 46, 255)),
    accent: rgba(200, 110, 60, 255),
    text: rgba(235, 225, 210, 255),
    focus_border: Some(rgba(200, 110, 60, 255)),
    sidebar_tab_border: Some(rgba(60, 51, 46, 255)),
    sidebar_tab_border_active: Some(rgba(200, 110, 60, 255)),
    sidebar_tab_border_hover: Some(rgba(200, 110, 60, 160)),
    title_mode: TitleMode::Lighter,
    ..BASIC_NEO
};

/// Slate on the Neo chassis.
pub const SLATE_NEO: ThemeSpec = ThemeSpec {
    name: "slate_neo",
    sidebar_tab_outline: 1.5,
    sidebar_tab_flat: SideTabFlat::None,
    base: rgba(150, 156, 166, 255),
    surface: Some(rgba(168, 174, 184, 255)),
    border: Some(rgba(132, 138, 148, 255)),
    accent: rgba(22, 70, 150, 255),
    text: rgba(22, 26, 34, 255),
    focus_border: Some(rgba(22, 70, 150, 255)),
    sidebar_tab_border: Some(rgba(132, 138, 148, 255)),
    sidebar_tab_border_active: Some(rgba(22, 70, 150, 255)),
    sidebar_tab_border_hover: Some(rgba(22, 70, 150, 160)),
    is_light: Rel::Light,
    title_mode: TitleMode::Lighter,
    ..BASIC_NEO
};

/// Caramel on the Neo chassis.
pub const CARAMEL_NEO: ThemeSpec = ThemeSpec {
    name: "caramel_neo",
    sidebar_tab_outline: 1.5,
    sidebar_tab_flat: SideTabFlat::None,
    base: rgba(197, 194, 189, 255),
    surface: Some(rgba(217, 214, 207, 255)),
    border: Some(rgba(167, 162, 151, 255)),
    accent: rgba(186, 98, 0, 255),
    text: rgba(39, 34, 28, 255),
    focus_border: Some(rgba(186, 98, 0, 255)),
    sidebar_tab_border: Some(rgba(167, 162, 151, 255)),
    sidebar_tab_border_active: Some(rgba(186, 98, 0, 255)),
    sidebar_tab_border_hover: Some(rgba(186, 98, 0, 160)),
    title_bg: Some(rgba(207, 204, 197, 255)),
    is_light: Rel::Light,
    ..BASIC_NEO
};

/// Neutral on the Neo chassis.
pub const NEUTRAL_NEO: ThemeSpec = ThemeSpec {
    name: "neutral_neo",
    sidebar_tab_outline: 1.5,
    sidebar_tab_flat: SideTabFlat::None,
    base: rgba(190, 191, 194, 255),
    surface: Some(rgba(210, 211, 214, 255)),
    border: Some(rgba(170, 171, 175, 255)),
    accent: rgba(6, 108, 196, 255),
    text: rgba(30, 33, 40, 255),
    focus_border: Some(rgba(6, 108, 196, 255)),
    sidebar_tab_border: Some(rgba(170, 171, 175, 255)),
    sidebar_tab_border_active: Some(rgba(6, 108, 196, 255)),
    sidebar_tab_border_hover: Some(rgba(6, 108, 196, 160)),
    title_bg: Some(rgba(199, 200, 203, 255)),
    is_light: Rel::Light,
    ..BASIC_NEO
};

/// Cream on the Neo chassis.
pub const CREAM_NEO: ThemeSpec = ThemeSpec {
    name: "cream_neo",
    sidebar_tab_outline: 1.5,
    sidebar_tab_flat: SideTabFlat::None,
    base: rgba(240, 235, 225, 255),
    surface: Some(rgba(252, 249, 243, 255)),
    border: Some(rgba(226, 219, 207, 255)),
    accent: rgba(176, 92, 0, 255),
    text: rgba(48, 40, 32, 255),
    focus_border: Some(rgba(176, 92, 0, 255)),
    sidebar_tab_border: Some(rgba(226, 219, 207, 255)),
    sidebar_tab_border_active: Some(rgba(176, 92, 0, 255)),
    sidebar_tab_border_hover: Some(rgba(176, 92, 0, 160)),
    is_light: Rel::Light,
    ..BASIC_NEO
};

/// Light on the Neo chassis.
pub const LIGHT_NEO: ThemeSpec = ThemeSpec {
    name: "light_neo",
    sidebar_tab_outline: 1.5,
    sidebar_tab_flat: SideTabFlat::None,
    base: rgba(238, 238, 241, 255),
    surface: Some(rgba(249, 249, 252, 255)),
    border: Some(rgba(225, 225, 228, 255)),
    accent: rgba(0, 102, 192, 255),
    text: rgba(45, 47, 52, 255),
    focus_border: Some(rgba(0, 102, 192, 255)),
    sidebar_tab_border: Some(rgba(225, 225, 228, 255)),
    sidebar_tab_border_active: Some(rgba(0, 102, 192, 255)),
    sidebar_tab_border_hover: Some(rgba(0, 102, 192, 160)),
    title_bg: Some(rgba(243, 243, 246, 255)),
    is_light: Rel::Light,
    ..BASIC_NEO
};

// ===========================================================================
// EDGE TREATMENTS (the outline carries the meaning)
// ===========================================================================

// Lace's four outline designs, each a family of dark, neutral and light.
// The colours, radii, tab outlines and title rules port one for one; the
// sidebar-edge geometry they also set has no ThemeSpec field here yet.

/// Night-city amber on near-black plum: a violet frame and title rule that burn amber on focus.
pub const CYBERPUNK_EDGE: ThemeSpec = ThemeSpec {
    name: "cyberpunk_edge",
    sidebar_tab_outline: 1.5,
    sidebar_tab_flat: SideTabFlat::None,
    base: rgba(17, 13, 20, 255),
    accent: rgba(255, 154, 0, 255),
    text: rgba(242, 232, 224, 255),
    surface: Some(rgba(31, 24, 36, 255)),
    title_bg: Some(rgba(10, 7, 13, 255)),
    border: Some(rgba(110, 72, 148, 190)),
    focus_border: Some(rgba(255, 154, 0, 255)),
    sidebar_tab_border: Some(rgba(110, 72, 148, 190)),
    sidebar_tab_border_active: Some(rgba(255, 154, 0, 255)),
    success: Some(rgba(120, 224, 143, 255)),
    warning: Some(rgba(255, 196, 61, 255)),
    error: Some(rgba(255, 84, 84, 255)),
    info: Some(rgba(186, 137, 255, 255)),
    title_mode: TitleMode::Darker,
    hover_mode: HoverMode::Lighter,
    corner_radius: 10.0,
    tab_radius: 8.0,
    border_width: 1.5,
    title_height: 32.0,
    control_radius: NEO_CONTROL_RADIUS,
    frame_radius: Some(NEO_FRAME_RADIUS),
    title_rule: 1.5,
    title_rule_focus: Some(rgba(255, 154, 0, 255)),
    tab_indicator: TabIndicator::Bottom,
    tab_indicator_width: 1.5,
    ..EDITOR_CLASSIC
};

/// cyberpunk_edge at dusk: grey grounds with the plum cast, burnt amber and violet kept.
pub const CYBERPUNK_EDGE_NEUTRAL: ThemeSpec = ThemeSpec {
    name: "cyberpunk_edge_neutral",
    sidebar_tab_outline: 1.5,
    sidebar_tab_flat: SideTabFlat::None,
    base: rgba(164, 160, 166, 255),
    accent: rgba(148, 72, 0, 255),
    text: rgba(38, 32, 42, 255),
    surface: Some(rgba(186, 182, 188, 255)),
    title_bg: Some(rgba(150, 147, 152, 255)),
    border: Some(rgba(108, 76, 146, 215)),
    focus_border: Some(rgba(148, 72, 0, 255)),
    sidebar_tab_border: Some(rgba(108, 76, 146, 215)),
    sidebar_tab_border_active: Some(rgba(148, 72, 0, 255)),
    success: Some(rgba(26, 137, 70, 255)),
    warning: Some(rgba(176, 116, 0, 255)),
    error: Some(rgba(196, 42, 42, 255)),
    info: Some(rgba(108, 61, 178, 255)),
    is_light: Rel::Light,
    title_mode: TitleMode::Darker,
    hover_mode: HoverMode::Lighter,
    corner_radius: 10.0,
    tab_radius: 8.0,
    border_width: 1.5,
    title_height: 32.0,
    control_radius: NEO_CONTROL_RADIUS,
    frame_radius: Some(NEO_FRAME_RADIUS),
    title_rule: 1.5,
    title_rule_focus: Some(rgba(148, 72, 0, 255)),
    tab_indicator: TabIndicator::Bottom,
    tab_indicator_width: 1.5,
    ..EDITOR_CLASSIC
};

/// cyberpunk_edge in daylight: lilac-white panels, a darker amber, the same violet frame.
pub const CYBERPUNK_EDGE_LIGHT: ThemeSpec = ThemeSpec {
    name: "cyberpunk_edge_light",
    sidebar_tab_outline: 1.5,
    sidebar_tab_flat: SideTabFlat::None,
    base: rgba(231, 226, 235, 255),
    accent: rgba(186, 98, 0, 255),
    text: rgba(40, 32, 44, 255),
    surface: Some(rgba(250, 247, 251, 255)),
    title_bg: Some(rgba(219, 212, 224, 255)),
    border: Some(rgba(140, 104, 170, 210)),
    focus_border: Some(rgba(186, 98, 0, 255)),
    sidebar_tab_border: Some(rgba(140, 104, 170, 210)),
    sidebar_tab_border_active: Some(rgba(186, 98, 0, 255)),
    success: Some(rgba(26, 137, 70, 255)),
    warning: Some(rgba(176, 116, 0, 255)),
    error: Some(rgba(196, 42, 42, 255)),
    info: Some(rgba(108, 61, 178, 255)),
    is_light: Rel::Light,
    title_mode: TitleMode::Darker,
    hover_mode: HoverMode::Darker,
    corner_radius: 10.0,
    tab_radius: 8.0,
    border_width: 1.5,
    title_height: 32.0,
    control_radius: NEO_CONTROL_RADIUS,
    frame_radius: Some(NEO_FRAME_RADIUS),
    title_rule: 1.5,
    title_rule_focus: Some(rgba(186, 98, 0, 255)),
    tab_indicator: TabIndicator::Bottom,
    tab_indicator_width: 1.5,
    ..EDITOR_CLASSIC
};

/// Dracula's palette on cyberpunk_edge's geometry: every tab outlined, the active one in violet.
pub const VIOLET_HAZE: ThemeSpec = ThemeSpec {
    name: "violet_haze",
    tab_indicator: TabIndicator::None,
    sidebar_tab_outline: 2.0,
    sidebar_tab_flat: SideTabFlat::None,
    border_below_title: true,
    base: rgba(40, 42, 54, 255),
    accent: rgba(189, 147, 249, 255),
    text: rgba(248, 248, 242, 255),
    surface: Some(rgba(58, 61, 78, 255)),
    title_bg: Some(rgba(30, 31, 40, 255)),
    border: Some(rgba(68, 71, 90, 255)),
    focus_border: Some(rgba(189, 147, 249, 255)),
    sidebar_tab_border: Some(rgba(0, 0, 0, 0)),
    sidebar_tab_border_hover: Some(rgba(189, 147, 249, 130)),
    success: Some(rgba(80, 250, 123, 255)),
    warning: Some(rgba(241, 250, 140, 255)),
    error: Some(rgba(255, 85, 85, 255)),
    info: Some(rgba(139, 233, 253, 255)),
    title_mode: TitleMode::Darker,
    hover_mode: HoverMode::Lighter,
    corner_radius: 10.0,
    tab_radius: 8.0,
    border_width: 2.0,
    title_height: 32.0,
    control_radius: NEO_CONTROL_RADIUS,
    frame_radius: Some(NEO_FRAME_RADIUS),
    title_rule: 2.0,
    tab_outline: 2.0,
    tab_outline_color: Some(rgba(98, 114, 164, 200)),
    tab_outline_active: Some(rgba(189, 147, 249, 255)),
    ..EDITOR_CLASSIC
};

/// violet_haze on grey grounds with a cool trace, the violet deepened to read on them.
pub const VIOLET_HAZE_NEUTRAL: ThemeSpec = ThemeSpec {
    name: "violet_haze_neutral",
    tab_indicator: TabIndicator::None,
    sidebar_tab_outline: 2.0,
    sidebar_tab_flat: SideTabFlat::None,
    border_below_title: true,
    base: rgba(164, 164, 169, 255),
    accent: rgba(104, 60, 176, 255),
    text: rgba(36, 34, 44, 255),
    surface: Some(rgba(186, 186, 191, 255)),
    title_bg: Some(rgba(150, 150, 155, 255)),
    border: Some(rgba(144, 144, 150, 255)),
    focus_border: Some(rgba(104, 60, 176, 255)),
    sidebar_tab_border: Some(rgba(0, 0, 0, 0)),
    sidebar_tab_border_hover: Some(rgba(104, 60, 176, 150)),
    success: Some(rgba(22, 128, 66, 255)),
    warning: Some(rgba(166, 110, 0, 255)),
    error: Some(rgba(193, 40, 40, 255)),
    info: Some(rgba(20, 116, 148, 255)),
    is_light: Rel::Light,
    title_mode: TitleMode::Darker,
    hover_mode: HoverMode::Lighter,
    corner_radius: 10.0,
    tab_radius: 8.0,
    border_width: 2.0,
    title_height: 32.0,
    control_radius: NEO_CONTROL_RADIUS,
    frame_radius: Some(NEO_FRAME_RADIUS),
    title_rule: 2.0,
    tab_outline: 2.0,
    tab_outline_color: Some(rgba(96, 108, 150, 215)),
    tab_outline_active: Some(rgba(104, 60, 176, 255)),
    ..EDITOR_CLASSIC
};

/// violet_haze on lavender-white, a deeper violet for the active outline.
pub const VIOLET_HAZE_LIGHT: ThemeSpec = ThemeSpec {
    name: "violet_haze_light",
    tab_indicator: TabIndicator::None,
    sidebar_tab_outline: 2.0,
    sidebar_tab_flat: SideTabFlat::None,
    border_below_title: true,
    base: rgba(230, 228, 238, 255),
    accent: rgba(124, 77, 196, 255),
    text: rgba(40, 36, 52, 255),
    surface: Some(rgba(250, 249, 253, 255)),
    title_bg: Some(rgba(217, 214, 230, 255)),
    border: Some(rgba(201, 197, 216, 255)),
    focus_border: Some(rgba(124, 77, 196, 255)),
    sidebar_tab_border: Some(rgba(0, 0, 0, 0)),
    sidebar_tab_border_hover: Some(rgba(124, 77, 196, 130)),
    success: Some(rgba(22, 128, 66, 255)),
    warning: Some(rgba(166, 110, 0, 255)),
    error: Some(rgba(193, 40, 40, 255)),
    info: Some(rgba(20, 116, 148, 255)),
    is_light: Rel::Light,
    title_mode: TitleMode::Darker,
    hover_mode: HoverMode::Darker,
    corner_radius: 10.0,
    tab_radius: 8.0,
    border_width: 2.0,
    title_height: 32.0,
    control_radius: NEO_CONTROL_RADIUS,
    frame_radius: Some(NEO_FRAME_RADIUS),
    title_rule: 2.0,
    tab_outline: 2.0,
    tab_outline_color: Some(rgba(163, 157, 190, 210)),
    tab_outline_active: Some(rgba(124, 77, 196, 255)),
    ..EDITOR_CLASSIC
};

/// Blue-black with a periwinkle accent: only the active tab is outlined.
pub const MIDNIGHT_HAZE: ThemeSpec = ThemeSpec {
    name: "midnight_haze",
    tab_indicator: TabIndicator::None,
    sidebar_tab_outline: 2.0,
    sidebar_tab_flat: SideTabFlat::None,
    border_below_title: true,
    tab_outline_unfocused: Some(rgba(0, 0, 0, 0)),
    base: rgba(24, 26, 35, 255),
    accent: rgba(125, 124, 252, 255),
    text: rgba(229, 232, 236, 255),
    surface: Some(rgba(36, 40, 52, 255)),
    title_bg: Some(rgba(16, 18, 25, 255)),
    border: Some(rgba(42, 46, 60, 255)),
    focus_border: Some(rgba(125, 124, 252, 255)),
    sidebar_tab_border: Some(rgba(0, 0, 0, 0)),
    sidebar_tab_border_active: Some(rgba(125, 124, 252, 255)),
    success: Some(rgba(80, 250, 123, 255)),
    warning: Some(rgba(241, 250, 140, 255)),
    error: Some(rgba(255, 85, 85, 255)),
    info: Some(rgba(139, 233, 253, 255)),
    title_mode: TitleMode::Darker,
    hover_mode: HoverMode::Lighter,
    corner_radius: 10.0,
    tab_radius: 8.0,
    border_width: 2.0,
    title_height: 32.0,
    control_radius: NEO_CONTROL_RADIUS,
    frame_radius: Some(NEO_FRAME_RADIUS),
    title_rule: 2.0,
    tab_outline: 2.0,
    tab_outline_color: Some(rgba(0, 0, 0, 0)),
    tab_outline_active: Some(rgba(125, 124, 252, 255)),
    ..EDITOR_CLASSIC
};

/// midnight_haze on cool grey grounds, the periwinkle deepened to indigo.
pub const MIDNIGHT_HAZE_NEUTRAL: ThemeSpec = ThemeSpec {
    name: "midnight_haze_neutral",
    tab_indicator: TabIndicator::None,
    sidebar_tab_outline: 2.0,
    sidebar_tab_flat: SideTabFlat::None,
    border_below_title: true,
    tab_outline_unfocused: Some(rgba(0, 0, 0, 0)),
    base: rgba(163, 163, 168, 255),
    accent: rgba(66, 65, 192, 255),
    text: rgba(28, 30, 40, 255),
    surface: Some(rgba(185, 185, 190, 255)),
    title_bg: Some(rgba(149, 149, 154, 255)),
    border: Some(rgba(146, 146, 152, 255)),
    focus_border: Some(rgba(66, 65, 192, 255)),
    sidebar_tab_border: Some(rgba(0, 0, 0, 0)),
    sidebar_tab_border_active: Some(rgba(66, 65, 192, 255)),
    success: Some(rgba(22, 128, 66, 255)),
    warning: Some(rgba(166, 110, 0, 255)),
    error: Some(rgba(193, 40, 40, 255)),
    info: Some(rgba(20, 116, 148, 255)),
    is_light: Rel::Light,
    title_mode: TitleMode::Darker,
    hover_mode: HoverMode::Lighter,
    corner_radius: 10.0,
    tab_radius: 8.0,
    border_width: 2.0,
    title_height: 32.0,
    control_radius: NEO_CONTROL_RADIUS,
    frame_radius: Some(NEO_FRAME_RADIUS),
    title_rule: 2.0,
    tab_outline: 2.0,
    tab_outline_color: Some(rgba(0, 0, 0, 0)),
    tab_outline_active: Some(rgba(66, 65, 192, 255)),
    ..EDITOR_CLASSIC
};

/// midnight_haze on blue-white, an indigo outline on the active tab.
pub const MIDNIGHT_HAZE_LIGHT: ThemeSpec = ThemeSpec {
    name: "midnight_haze_light",
    tab_indicator: TabIndicator::None,
    sidebar_tab_outline: 2.0,
    sidebar_tab_flat: SideTabFlat::None,
    border_below_title: true,
    tab_outline_unfocused: Some(rgba(0, 0, 0, 0)),
    base: rgba(227, 229, 237, 255),
    accent: rgba(83, 82, 214, 255),
    text: rgba(28, 30, 40, 255),
    surface: Some(rgba(247, 248, 252, 255)),
    title_bg: Some(rgba(213, 216, 226, 255)),
    border: Some(rgba(206, 209, 220, 255)),
    focus_border: Some(rgba(83, 82, 214, 255)),
    sidebar_tab_border: Some(rgba(0, 0, 0, 0)),
    sidebar_tab_border_active: Some(rgba(83, 82, 214, 255)),
    success: Some(rgba(22, 128, 66, 255)),
    warning: Some(rgba(166, 110, 0, 255)),
    error: Some(rgba(193, 40, 40, 255)),
    info: Some(rgba(20, 116, 148, 255)),
    is_light: Rel::Light,
    title_mode: TitleMode::Darker,
    hover_mode: HoverMode::Darker,
    corner_radius: 10.0,
    tab_radius: 8.0,
    border_width: 2.0,
    title_height: 32.0,
    control_radius: NEO_CONTROL_RADIUS,
    frame_radius: Some(NEO_FRAME_RADIUS),
    title_rule: 2.0,
    tab_outline: 2.0,
    tab_outline_color: Some(rgba(0, 0, 0, 0)),
    tab_outline_active: Some(rgba(83, 82, 214, 255)),
    ..EDITOR_CLASSIC
};

/// slate_amber's warm slate and amber in the dark, on the 4px chassis.
pub const SLATE_AMBER_DARK: ThemeSpec = ThemeSpec {
    name: "slate_amber_dark",
    sidebar_tab_outline: 1.5,
    sidebar_tab_flat: SideTabFlat::None,
    base: rgba(42, 40, 37, 255),
    accent: rgba(230, 150, 45, 255),
    text: rgba(232, 228, 222, 255),
    surface: Some(rgba(54, 51, 47, 255)),
    border: Some(rgba(74, 70, 65, 255)),
    focus_border: Some(rgba(230, 150, 45, 255)),
    sidebar_tab_border: Some(rgba(0, 0, 0, 0)),
    sidebar_tab_border_active: Some(rgba(230, 150, 45, 255)),
    sidebar_tab_border_hover: Some(rgba(230, 150, 45, 160)),
    success: Some(rgba(120, 224, 143, 255)),
    warning: Some(rgba(255, 196, 61, 255)),
    error: Some(rgba(255, 96, 84, 255)),
    info: Some(rgba(138, 180, 248, 255)),
    title_mode: TitleMode::Darker,
    hover_mode: HoverMode::Lighter,
    corner_radius: 4.0,
    tab_radius: 4.0,
    border_width: 1.5,
    title_height: 30.0,
    title_rule: 1.5,
    title_rule_focus: Some(rgba(230, 150, 45, 255)),
    tab_indicator: TabIndicator::Bottom,
    tab_indicator_width: 1.5,
    ..EDITOR_CLASSIC
};

/// Machine-grey slate with burnt amber: a 4px chassis whose title rule turns amber on focus.
pub const SLATE_AMBER: ThemeSpec = ThemeSpec {
    name: "slate_amber",
    sidebar_tab_outline: 1.5,
    sidebar_tab_flat: SideTabFlat::None,
    base: rgba(196, 194, 190, 255),
    accent: rgba(186, 98, 0, 255),
    text: rgba(38, 34, 30, 255),
    surface: Some(rgba(216, 214, 209, 255)),
    border: Some(rgba(166, 162, 154, 255)),
    focus_border: Some(rgba(186, 98, 0, 255)),
    sidebar_tab_border: Some(rgba(0, 0, 0, 0)),
    sidebar_tab_border_active: Some(rgba(186, 98, 0, 255)),
    sidebar_tab_border_hover: Some(rgba(186, 98, 0, 160)),
    is_light: Rel::Light,
    title_mode: TitleMode::Darker,
    hover_mode: HoverMode::Lighter,
    corner_radius: 4.0,
    tab_radius: 4.0,
    border_width: 1.5,
    title_height: 30.0,
    title_rule: 1.5,
    title_rule_focus: Some(rgba(186, 98, 0, 255)),
    tab_indicator: TabIndicator::Bottom,
    tab_indicator_width: 1.5,
    ..EDITOR_CLASSIC
};

/// slate_amber on paler, warmer grounds, the amber a shade deeper.
pub const SLATE_AMBER_LIGHT: ThemeSpec = ThemeSpec {
    name: "slate_amber_light",
    sidebar_tab_outline: 1.5,
    sidebar_tab_flat: SideTabFlat::None,
    base: rgba(230, 227, 222, 255),
    accent: rgba(176, 92, 0, 255),
    text: rgba(42, 37, 32, 255),
    surface: Some(rgba(249, 247, 244, 255)),
    border: Some(rgba(201, 196, 188, 255)),
    focus_border: Some(rgba(176, 92, 0, 255)),
    sidebar_tab_border: Some(rgba(0, 0, 0, 0)),
    sidebar_tab_border_active: Some(rgba(176, 92, 0, 255)),
    sidebar_tab_border_hover: Some(rgba(176, 92, 0, 160)),
    is_light: Rel::Light,
    title_mode: TitleMode::Darker,
    hover_mode: HoverMode::Lighter,
    corner_radius: 4.0,
    tab_radius: 4.0,
    border_width: 1.5,
    title_height: 30.0,
    title_rule: 1.5,
    title_rule_focus: Some(rgba(176, 92, 0, 255)),
    tab_indicator: TabIndicator::Bottom,
    tab_indicator_width: 1.5,
    ..EDITOR_CLASSIC
};
/// Every curated theme, in Lace's menu order: Basics, Basics Neo, Editor
/// Classics, Neon, Edge Treatments.
pub const ALL: [&ThemeSpec; 37] = [
    &MIDNIGHT,
    &DARK,
    &MOCHA,
    &SLATE,
    &CARAMEL,
    &NEUTRAL,
    &CREAM,
    &LIGHT,
    &MIDNIGHT_NEO,
    &DARK_NEO,
    &MOCHA_NEO,
    &SLATE_NEO,
    &CARAMEL_NEO,
    &NEUTRAL_NEO,
    &CREAM_NEO,
    &LIGHT_NEO,
    &DRACULA,
    &MONOKAI,
    &NORDIC,
    &CATPPUCCIN,
    &TOKYO_NIGHT,
    &SOLARIZED_DARK,
    &SOLARIZED_LIGHT,
    &CYBERPUNK_NEON,
    &NEON_DUSK,
    &CYBERPUNK_EDGE,
    &CYBERPUNK_EDGE_NEUTRAL,
    &CYBERPUNK_EDGE_LIGHT,
    &VIOLET_HAZE,
    &VIOLET_HAZE_NEUTRAL,
    &VIOLET_HAZE_LIGHT,
    &MIDNIGHT_HAZE,
    &MIDNIGHT_HAZE_NEUTRAL,
    &MIDNIGHT_HAZE_LIGHT,
    &SLATE_AMBER_DARK,
    &SLATE_AMBER,
    &SLATE_AMBER_LIGHT,
];

/// [`ALL`] in Lace's named groups (`THEME_GROUPS`), for menus.
pub const GROUPS: [(&str, &[&ThemeSpec]); 5] = [
    ("Basics", &[&MIDNIGHT, &DARK, &MOCHA, &SLATE, &CARAMEL, &NEUTRAL, &CREAM, &LIGHT]),
    (
        "Basics Neo",
        &[&MIDNIGHT_NEO, &DARK_NEO, &MOCHA_NEO, &SLATE_NEO, &CARAMEL_NEO, &NEUTRAL_NEO, &CREAM_NEO, &LIGHT_NEO],
    ),
    (
        "Editor Classics",
        &[&DRACULA, &MONOKAI, &NORDIC, &CATPPUCCIN, &TOKYO_NIGHT, &SOLARIZED_DARK, &SOLARIZED_LIGHT],
    ),
    ("Neon", &[&CYBERPUNK_NEON, &NEON_DUSK]),
    (
        "Edge Treatments",
        &[
            &CYBERPUNK_EDGE,
            &CYBERPUNK_EDGE_NEUTRAL,
            &CYBERPUNK_EDGE_LIGHT,
            &VIOLET_HAZE,
            &VIOLET_HAZE_NEUTRAL,
            &VIOLET_HAZE_LIGHT,
            &MIDNIGHT_HAZE,
            &MIDNIGHT_HAZE_NEUTRAL,
            &MIDNIGHT_HAZE_LIGHT,
            &SLATE_AMBER_DARK,
            &SLATE_AMBER,
            &SLATE_AMBER_LIGHT,
        ],
    ),
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
