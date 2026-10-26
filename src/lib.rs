//! lace-gpui — Lace's OKLCH theming engine, ported to gpui-kit.
//!
//! Lace ([opticsWolf/Lace](https://github.com/opticsWolf/Lace), Apache-2.0)
//! derives a complete dock theme from three to five seed colours in OKLCH,
//! then lifts every token that still misses a WCAG contrast or surface
//! separation floor. This crate ports that engine to Rust for
//! [gpui-kit](https://crates.io/crates/gpui-kit) (formerly gpui-component):
//!
//! - [`color`] — the zero-dependency colour core (OKLab/OKLCH, WCAG, gamut
//!   mapping). No gpui, no Qt, no serde: build it anywhere.
//! - [`engine`] — [`ThemeSpec`](engine::ThemeSpec) → [`Tokens`](engine::Tokens)
//!   derivation (Lace's `_DEPTH` steps, selection modes, keep_tint).
//! - [`contrast`] — the rulebook: WCAG floors per role and contrast level,
//!   surface separation per depth, explicit-colour capping, and [`enforce`].
//! - [`themes`] — 16 curated presets, from midnight to cyberpunk_neon.
//! - [`skin`] *(feature `gpui-kit`)* — the bridge: `apply("dracula", cx)`
//!   paints a running gpui-kit app with Lace's tokens.
//! - [`base_style`] *(feature `gpui-kit`)* — LaceStyle's flat widget layer:
//!   derived outlines, state steps, focus ring, scrollbar and slider mixes.
//! - [`geometry`] *(feature `gpui-kit`)* — an application-side dock skin:
//!   gaps, rounded panels, outlines, shadow/glow and configurable splitters.
//!
//! ```
//! // The colour core alone:
//! let tokens = lace_gpui::build(lace_gpui::themes::by_name("dracula").unwrap());
//! assert!(!tokens.is_light);
//!
//! // WCAG body text on the panel, enforced by the engine:
//! let ratio = lace_gpui::color::contrast_ratio(tokens.text, tokens.panel_bg);
//! assert!(ratio > 4.5);
//! ```
//!
//! ## Fidelity
//!
//! Faithful port of Lace 0.9.7's build path with three documented
//! simplifications, all in the README: gpui has no Qt palette roles, so the
//! Fusion bevel tokens ride along unused; the sidebar's gradient hover pair
//! maps to gpui's flat hover fill; and status colours gain a small
//! visibility floor Lace applies at a different layer.

pub mod color;
pub mod contrast;
pub mod engine;
pub mod themes;

#[cfg(feature = "gpui-kit")]
pub mod base_style;

#[cfg(feature = "gpui-kit")]
pub mod skin;

#[cfg(feature = "gpui-kit")]
pub mod geometry;

/// The Lace release this crate was ported from.
pub const LACE_VERSION: &str = "0.9.7";

pub use contrast::{audit, enforce, separation_target, EXPLICIT_MAX_DE, LIGHT_SURFACE_SCALE};
pub use engine::{build, spec_from_hex, ThemeSpec, Tokens};
pub use themes::{by_name, ALL};
