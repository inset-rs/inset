//! Flutter counterpart: `widgets/bottom_navigation_bar_item.dart`.

use waft_painting::AnyColor;

use crate::framework::{IntoWidget, KeyRef, WidgetRef};

/// An interactive button within either material's `BottomNavigationBar` or the iOS themed
/// `CupertinoTabBar` with an icon and title.
///
/// This class is rarely used in isolation. It is typically embedded in one of the bottom
/// navigation widgets above.
///
/// See also:
///
///  * `BottomNavigationBar`
///  * <https://material.io/design/components/bottom-navigation.html>
///  * `CupertinoTabBar`
///  * <https://developer.apple.com/design/human-interface-guidelines/tab-bars/>
#[derive(Clone, Debug)]
pub struct BottomNavigationBarItem {
    /// A key to be passed through to the resultant widget.
    ///
    /// This allows the identification of different [`BottomNavigationBarItem`]s through their
    /// keys.
    ///
    /// When changing the number of bar items in response to a bar item being tapped, giving
    /// each item a key will allow the inkwell / splash animation to be correctly positioned.
    pub key: Option<KeyRef>,

    /// The icon of the item.
    ///
    /// Typically the icon is an `Icon` or an `ImageIcon` widget. If another type of widget is
    /// provided then it should configure itself to match the current `IconTheme` size and
    /// color.
    ///
    /// If [`active_icon`](Self::active_icon) is provided, this will only be displayed when the
    /// item is not selected.
    ///
    /// To make the bottom navigation bar more accessible, consider choosing an icon with a
    /// stroked and filled version, such as `Icons.cloud` and `Icons.cloud_queue`.
    /// [`icon`](Self::icon) should be set to the stroked version and
    /// [`active_icon`](Self::active_icon) to the filled version.
    ///
    /// If a particular icon doesn't have a stroked or filled version, then don't pair
    /// unrelated icons. Instead, make sure to use a `BottomNavigationBarType.shifting`.
    pub icon: WidgetRef,

    /// An alternative icon displayed when this bottom navigation item is selected.
    ///
    /// If this icon is not provided, the bottom navigation bar will display
    /// [`icon`](Self::icon) in either state.
    pub active_icon: WidgetRef,

    /// The text label for this [`BottomNavigationBarItem`].
    ///
    /// This will be used to create a `Text` widget to put in the bottom navigation bar.
    pub label: Option<String>,

    /// The color of the background radial animation for material `BottomNavigationBar`.
    ///
    /// If the navigation bar's type is `BottomNavigationBarType.shifting`, then the entire bar
    /// is flooded with the [`background_color`](Self::background_color) when this item is
    /// tapped. This will override `BottomNavigationBar.backgroundColor`.
    ///
    /// Not used for `CupertinoTabBar`. Control the invariant bar color directly via
    /// `CupertinoTabBar.backgroundColor`.
    pub background_color: Option<AnyColor>,

    /// The text to display in the `Tooltip` for this [`BottomNavigationBarItem`].
    ///
    /// A `Tooltip` will only appear on this item if [`tooltip`](Self::tooltip) is set to a
    /// non-empty string.
    ///
    /// Defaults to `None`, in which case the tooltip is not shown.
    pub tooltip: Option<String>,
}

impl BottomNavigationBarItem {
    /// Creates an item that is used with `BottomNavigationBar.items`; Dart's optional named
    /// arguments are the setters.
    ///
    /// The argument `icon` should not be `None` and the argument `label` should not be `None`
    /// when used in a Material Design's `BottomNavigationBar`.
    pub fn new<K>(icon: impl IntoWidget<K>) -> BottomNavigationBarItem {
        let icon = icon.into_widget();
        BottomNavigationBarItem {
            key: None,
            active_icon: icon.clone(),
            icon,
            label: None,
            background_color: None,
            tooltip: None,
        }
    }

    /// Dart `BottomNavigationBarItem(key:)`.
    pub fn key(mut self, key: KeyRef) -> BottomNavigationBarItem {
        self.key = Some(key);
        self
    }

    /// Dart `BottomNavigationBarItem(label:)`.
    pub fn label(mut self, label: impl Into<String>) -> BottomNavigationBarItem {
        self.label = Some(label.into());
        self
    }

    /// Dart `BottomNavigationBarItem(activeIcon:)`.
    pub fn active_icon<K>(mut self, active_icon: impl IntoWidget<K>) -> BottomNavigationBarItem {
        self.active_icon = active_icon.into_widget();
        self
    }

    /// Dart `BottomNavigationBarItem(backgroundColor:)`.
    pub fn background_color(
        mut self,
        background_color: impl Into<AnyColor>,
    ) -> BottomNavigationBarItem {
        self.background_color = Some(background_color.into());
        self
    }

    /// Dart `BottomNavigationBarItem(tooltip:)`.
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> BottomNavigationBarItem {
        self.tooltip = Some(tooltip.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framework::same_widget;
    use crate::widgets::basic::SizedBox;

    /// Without an active icon the item shows its icon in either state.
    #[test]
    fn the_active_icon_defaults_to_the_icon() {
        let item = BottomNavigationBarItem::new(SizedBox::shrink());
        assert!(same_widget(&item.icon, &item.active_icon));
        let active = SizedBox::expand().into_widget();
        let item = item.active_icon(active.clone());
        assert!(same_widget(&item.active_icon, &active));
        assert!(!same_widget(&item.icon, &item.active_icon));
    }
}
