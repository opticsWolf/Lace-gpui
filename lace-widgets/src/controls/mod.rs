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
pub mod nav;
pub mod range;
pub mod status;
pub mod toggle;
mod interaction;

pub use button::{apply_face, face_div, measure_text, ButtonRow, FoldOrder, Icon, PushButton, ToolButton};
pub use chrome::{frame_line, size_grip, splitter_grip, toolbar_handle, toolbar_separator, Scrollbar, Splitter};
pub use container::{
    frame_box, header_section, item_row_div, menu_separator, popup_panel, tab_div, tool_box_tab, tooltip,
    CollapsibleGroupBox, GroupBox, MenuBarItem, MenuItem, Sort, TabBar, TabClose, TabPosition, TabWidget, ToolBox,
};
pub use check::{check_indicator, radio_indicator, CheckBox, CheckState, RadioButton};
pub use field::{field_div, ComboBox, LineEdit, SpinBox, TextEdit};
pub use interaction::{child_id, Interaction};
pub use margin::Margin;
pub use nav::{Breadcrumb, Pagination, Stepper};
pub use range::{round_rect, Dial, DynamicRange, DynamicSlider, ProgressBar, Slider, Ticks};
pub use status::{Alert, Avatar, Badge, Link, ProgressRing, Separator, Skeleton, Spinner, Tag};
pub use toggle::{nth_id, Rating, Segmented, Switch};
pub use crate::style::tones::Tone;
