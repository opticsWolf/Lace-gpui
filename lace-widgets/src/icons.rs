//! Named icons for the widgets: Lace's icon provider
//! ([`lace_gpui::icons`]) plus a builder the widgets take.
//!
//! An app registers its sets once; a button or menu item then asks for an
//! icon by name:
//!
//! ```ignore
//! lace_widgets::icons::register(IconSet::from_static("app", APP_ICONS), cx);
//! MenuItem::new("undo", "Undo").icon_rc(lace_widgets::icons::named("undo", cx));
//! ```

use std::rc::Rc;

use gpui_kit::gpui::{App, IntoElement as _};

pub use lace_gpui::icons::*;

use crate::controls::Icon;

/// The size widgets draw an icon at, px (Qt's 16 px small icon).
pub const SIZE: f32 = 16.0;

/// The registered icon `name` (see [`Icons::lookup`]) as a widget icon,
/// drawn [`SIZE`] px in the colour the widget hands it.
pub fn named(name: &str, cx: &App) -> Option<Icon> {
    lookup(name, cx).map(|svg| icon(svg, SIZE))
}

/// `svg` as a widget icon, `size` px.
pub fn icon(svg: Svg, size: f32) -> Icon {
    Rc::new(move |color| element(&svg, size, color).into_any_element())
}
