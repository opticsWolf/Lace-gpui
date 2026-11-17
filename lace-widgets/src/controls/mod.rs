//! LaceStyle widgets.
//!
//! Every widget is a `RenderOnce` component with a stable id. It tracks its
//! own hover / press / keyboard focus through keyed state, so the view that
//! uses it needs no fields for them, and paints from the installed
//! [`LaceStyle`](crate::style::LaceStyle). [`Interaction::force`] overrides
//! the tracked state — that is how the gallery shows every state at once.

pub mod button;
pub mod check;
pub mod chrome;
pub mod container;
pub mod field;
pub mod margin;
pub mod range;
mod interaction;

pub use button::{apply_face, face_div, measure_text, ButtonRow, FoldOrder, Icon, PushButton, ToolButton};
pub use chrome::{frame_line, size_grip, splitter_grip, toolbar_handle, toolbar_separator, Scrollbar, Splitter};
pub use container::{
    frame_box, header_section, item_row_div, menu_separator, popup_panel, tab_div, tool_box_tab, tooltip,
    GroupBox, MenuBarItem, MenuItem, Sort, TabBar, TabClose, TabPosition, TabWidget, ToolBox,
};
pub use check::{check_indicator, radio_indicator, CheckBox, CheckState, RadioButton};
pub use field::{field_div, ComboBox, LineEdit, SpinBox};
pub use interaction::{child_id, Interaction};
pub use margin::Margin;
pub use range::{round_rect, Dial, ProgressBar, Slider, Ticks};
