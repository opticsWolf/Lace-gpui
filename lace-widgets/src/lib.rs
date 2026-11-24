//! LaceStyle for gpui — Lace's flat, vector-drawn control layer as custom
//! gpui widgets.
//!
//! - [`style`] — the palette, tokens and colour rules (pure, testable).
//! - [`glyph`] — vector glyphs: chevrons, ticks, crosses, arcs.
//! - [`controls`] — the widgets: buttons, indicators, fields, range,
//!   containers and chrome, each painting itself from the installed
//!   [`style::LaceStyle`].
//!
//! ```ignore
//! lace_widgets::style::install_preset("dracula", cx);
//! // ...
//! PushButton::new("ok", "OK").default_button().on_click(|_, _, _| {})
//! ```
//!
//! [`DynamicButton`] and the [`edge`] helpers are the older gpui-kit-skin
//! helpers the theme gallery uses; they paint stock gpui-kit buttons.

pub mod controls;
pub mod dynamic_button;
pub mod edge;
pub mod glyph;
pub mod icons;
pub mod style;
pub mod views;

pub use dynamic_button::DynamicButton;
pub use edge::{edge_color, edged, hover_fill, on_fill, press_fill};
pub use style::{font_scale, install, install_preset, set_font_scale, with_font_scale, ActiveStyle, LaceStyle, State, FONT_SCALE_RANGE};
