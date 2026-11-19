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
//! - [`themes`] — 37 curated presets in Lace's five menu groups.
//! - [`modern`] — the modern look both paint layers share: radius floors
//!   and soft elevation shadows, applied at paint time over the tokens.
//! - [`skin`] *(feature `gpui-kit`)* — the bridge: `apply("dracula", cx)`
//!   paints a running gpui-kit app with Lace's tokens and installs the dock
//!   styles below.
//! - [`base_style`] *(feature `gpui-kit`)* — LaceStyle's flat widget layer:
//!   derived outlines, state steps, focus ring, scrollbar and slider mixes.
//! - [`button`] *(feature `gpui-kit`)* — theme-driven button halos: an accent
//!   hover glow plus a denser press glow from one call, no view state needed.
//! - [`geometry`] *(feature `gpui-kit`)* — an application-side dock skin:
//!   gaps, rounded panels, outlines, shadow/glow and configurable splitters.
//! - [`edge_tabs`] *(feature `gpui-kit`)* — Lace's dock tab strip: outlines,
//!   the accent stripe, overflow, close and pin buttons.
//! - [`sidebar`] *(feature `gpui-kit`)* — auto-hide sidebars with peek,
//!   pin/unpin, drag-to-dock and Lace's three focus modes.
//! - [`floating`] *(feature `gpui-kit`)* — the whole-pane drag preview and
//!   Lace's five-target drop overlay.
//! - [`dock_icons`] *(feature `gpui-kit`)* — Lace's tabler dock icons.
//!
//! `QUICKREF.md` and `ARCHITECTURE.md` at the repository root give the API
//! at a glance and how the pieces fit together.
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
//! Faithful port of Lace 0.9.7's build path with two documented
//! simplifications, both in the README: gpui has no Qt palette roles, so the
//! Fusion bevel tokens ride along unused; and status colours gain a small
//! visibility floor Lace applies at a different layer.

pub mod color;
pub mod contrast;
pub mod engine;
pub mod modern;
pub mod themes;

#[cfg(feature = "gpui-kit")]
pub mod base_style;

#[cfg(feature = "gpui-kit")]
pub mod button;

#[cfg(feature = "gpui-kit")]
pub mod skin;

#[cfg(feature = "gpui-kit")]
pub mod geometry;

#[cfg(feature = "gpui-kit")]
pub mod edge_tabs;
#[cfg(feature = "gpui-kit")]
pub mod dock_icons;
#[cfg(feature = "gpui-kit")]
pub mod sidebar;
#[cfg(feature = "gpui-kit")]
pub mod floating;

/// The Lace release this crate was ported from.
pub const LACE_VERSION: &str = "0.9.7";

pub use contrast::{audit, enforce, separation_target, EXPLICIT_MAX_DE, LIGHT_SURFACE_SCALE};
pub use engine::{build, spec_from_hex, ThemeSpec, Tokens};
pub use themes::{by_name, ALL};
