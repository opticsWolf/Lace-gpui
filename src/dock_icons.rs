//! Lace's dock icons: the tabler outlines PySide Lace draws its tab close
//! box and title-bar buttons with, embedded so no asset source is needed.
//!
//! gpui renders an SVG as a mask in the element's text colour, so each icon
//! takes the colour of the button it sits in, as Lace's tinted icons do.

use gpui_kit::component::Icon;
use gpui_kit::gpui::{div, prelude::FluentBuilder as _, px, IntoElement, ParentElement as _, Styled as _};

/// One of Lace's dock icons (`lace/resources/lace_icons`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DockIcon {
    /// The boxed ✕ on a tab.
    CloseTab,
    /// The title bar's close.
    Close,
    /// Two bars: the area menu.
    TabsMenu,
    /// Pin to a sidebar.
    Pin,
    /// Unpin from a sidebar.
    Unpin,
    /// Pin every tab of the group.
    PinAll,
    /// Float out into a window.
    Float,
    /// Dock a floating window back.
    Dock,
    /// Maximize the area.
    Maximize,
    /// Restore a maximized area.
    Restore,
    /// The list of a group's tabs.
    TabList,
    ChevronLeft,
    ChevronRight,
}

impl DockIcon {
    /// Every dock icon.
    pub const ALL: [DockIcon; 13] = [
        Self::CloseTab,
        Self::Close,
        Self::TabsMenu,
        Self::Pin,
        Self::Unpin,
        Self::PinAll,
        Self::Float,
        Self::Dock,
        Self::Maximize,
        Self::Restore,
        Self::TabList,
        Self::ChevronLeft,
        Self::ChevronRight,
    ];

    /// Its stem: the file name in `lace/resources/lace_icons`, and its name
    /// in the icon provider's [`DOCK`](crate::icons::DOCK) set.
    pub fn stem(self) -> &'static str {
        match self {
            Self::CloseTab => "close_tab",
            Self::Close => "close",
            Self::TabsMenu => "tabs_menu",
            Self::Pin => "pin",
            Self::Unpin => "unpin",
            Self::PinAll => "pin_all",
            Self::Float => "float",
            Self::Dock => "dock",
            Self::Maximize => "maximize",
            Self::Restore => "restore",
            Self::TabList => "tab_list",
            Self::ChevronLeft => "chevron_left",
            Self::ChevronRight => "chevron_right",
        }
    }

    /// The SVG source.
    pub fn svg(self) -> &'static [u8] {
        match self {
            Self::CloseTab => include_bytes!("icons/close_tab.svg"),
            Self::Close => include_bytes!("icons/close.svg"),
            Self::TabsMenu => include_bytes!("icons/tabs_menu.svg"),
            Self::Pin => include_bytes!("icons/pin.svg"),
            Self::Unpin => include_bytes!("icons/unpin.svg"),
            Self::PinAll => include_bytes!("icons/pin_all.svg"),
            Self::Float => include_bytes!("icons/float.svg"),
            Self::Dock => include_bytes!("icons/dock.svg"),
            Self::Maximize => include_bytes!("icons/maximize.svg"),
            Self::Restore => include_bytes!("icons/restore.svg"),
            Self::TabList => include_bytes!("icons/tab_list.svg"),
            Self::ChevronLeft => include_bytes!("icons/chevron_left.svg"),
            Self::ChevronRight => include_bytes!("icons/chevron_right.svg"),
        }
    }

    /// As a gpui-kit icon, for a `Button` or on its own.
    pub fn icon(self) -> Icon {
        Icon::empty().data(self.svg())
    }
}

impl From<DockIcon> for Icon {
    fn from(icon: DockIcon) -> Self {
        icon.icon()
    }
}

/// A tab label with Lace's leading icon: `icon` at 16px, 6px before the text.
///
/// gpui-kit panels have no icon of their own, so a panel that wants one
/// returns this from its `Panel::title`. The text may shrink, so a cramped
/// tab elides it after the icon.
pub fn tab_label(icon: Option<Icon>, label: impl IntoElement) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(6.))
        .when_some(icon, |this, icon| this.child(icon.size(px(16.)).flex_none()))
        .child(div().flex_shrink(1.).min_w_0().overflow_hidden().child(label))
}

#[cfg(test)]
mod tests {
    use super::DockIcon;

    #[test]
    fn every_icon_is_an_svg() {
        for icon in DockIcon::ALL {
            let text = std::str::from_utf8(icon.svg()).unwrap();
            assert!(text.contains("<svg"), "{icon:?}");
        }
    }
}
