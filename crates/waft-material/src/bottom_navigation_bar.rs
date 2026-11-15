//! Flutter counterpart: material_ui's `bottom_navigation_bar.dart`.

use std::any::Any;
use std::collections::VecDeque;
use std::fmt::{self, Debug};
use std::rc::Rc;

use waft_animation::{
    Animatable, Animation as _, AnimationBehavior, AnimationController, AnimationStatusListener,
    AnyAnimation, ColorTween, CurvedAnimation, Curves, Tween,
};
use waft_embedder::{Brightness, Canvas, Color, Offset, Paint, Rect, Size};
use waft_foundation::{App, Handle, Listener, ValueChanged};
use waft_painting::{
    AlignmentGeometry, EdgeInsetsGeometry, TextDirection, TextOverflow, TextStyle,
};
use waft_rendering::{BoxConstraints, CustomPainter, MainAxisAlignment, MainAxisSize};
use waft_scheduler::{Ticker, TickerCallback, TickerProviderObject};
use waft_services::MouseCursorRef;
use waft_widgets::{
    Align, BottomNavigationBarItem, BuildContext, Column, ConstrainedBox, CustomPaint,
    DefaultTextStyle, Directionality, Expanded, FadeTransition, Flexible, IconTheme, IconThemeData,
    IntoWidget, IntrinsicWidth, KeyRef, Matrix4, MediaQuery, Orientation, Padding, Row, Semantics,
    SizedBox, Stack, State, StateData, StatefulWidget, StatelessWidget, Text,
    TickerProviderStateMixin, TickerProviderStateMixinData, Transform, Visibility, WidgetRef,
    WidgetState, WidgetStateMouseCursor, WidgetStateProperty, WidgetStates,
    debug_check_has_directionality, debug_check_has_media_query, debug_check_has_overlay,
};

use crate::bottom_navigation_bar_theme::BottomNavigationBarTheme;
use crate::constants::K_BOTTOM_NAVIGATION_BAR_HEIGHT;
use crate::debug::debug_check_has_material_localizations;
use crate::ink_well::InkResponse;
use crate::material::{Material, MaterialType, destroy_lingering};
use crate::material_localizations::MaterialLocalizations;
use crate::theme::{K_THEME_ANIMATION_DURATION, Theme};
use crate::tooltip::Tooltip;

/// Defines the layout and behavior of a `BottomNavigationBar`.
///
/// For a sample on how to use these, please see `BottomNavigationBar`.
/// See also:
///
///  * `BottomNavigationBar`
///  * `BottomNavigationBarItem`
///  * <https://material.io/design/components/bottom-navigation.html#specs>
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BottomNavigationBarType {
    /// The `BottomNavigationBar`'s `BottomNavigationBarItem`s have fixed width.
    Fixed,

    /// The location and size of the `BottomNavigationBar` `BottomNavigationBarItem`s
    /// animate and labels fade in when they are tapped.
    Shifting,
}

/// Refines the layout of a `BottomNavigationBar` when the enclosing
/// `MediaQueryData::orientation` is `Orientation::Landscape`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BottomNavigationBarLandscapeLayout {
    /// If the enclosing `MediaQueryData::orientation` is
    /// `Orientation::Landscape` then the navigation bar's items are
    /// evenly spaced and spread out across the available width. Each
    /// item's label and icon are arranged in a column.
    Spread,

    /// If the enclosing `MediaQueryData::orientation` is
    /// `Orientation::Landscape` then the navigation bar's items are
    /// evenly spaced in a row but only consume as much width as they
    /// would in portrait orientation. The row of items is centered within
    /// the available width. Each item's label and icon are arranged
    /// in a column.
    Centered,

    /// If the enclosing `MediaQueryData::orientation` is
    /// `Orientation::Landscape` then the navigation bar's items are
    /// evenly spaced and each item's icon and label are lined up in a
    /// row instead of a column.
    Linear,
}

/// A material widget that's displayed at the bottom of an app for selecting
/// among a small number of views, typically between three and five.
///
/// There is an updated version of this component, `NavigationBar`, that's
/// preferred for new applications and applications that are configured
/// for Material 3 (see `ThemeData::use_material3`).
///
/// The bottom navigation bar consists of multiple items in the form of
/// text labels, icons, or both, laid out on top of a piece of material. It
/// provides quick navigation between the top-level views of an app. For larger
/// screens, side navigation may be a better fit.
///
/// A bottom navigation bar is usually used in conjunction with a `Scaffold`,
/// where it is provided as the `Scaffold::bottom_navigation_bar` argument.
///
/// The bottom navigation bar's [`type`](Self::type) changes how its [`items`](Self::items)
/// are displayed. If not specified, then it's automatically set to
/// [`BottomNavigationBarType::Fixed`] when there are less than four items, and
/// [`BottomNavigationBarType::Shifting`] otherwise.
///
/// The length of [`items`](Self::items) must be at least two and each item's icon and
/// label must not be null.
///
///  * [`BottomNavigationBarType::Fixed`], the default when there are less than
///    four [`items`](Self::items). The selected item is rendered with the
///    [`selected_item_color`](Self::selected_item_color) if it's non-null, otherwise the
///    theme's `ColorScheme::primary` color is used for `Brightness::Light` themes
///    and `ColorScheme::secondary` for `Brightness::Dark` themes.
///    If [`background_color`](Self::background_color) is null, The
///    navigation bar's background color defaults to the [`Material`] background
///    color, `ThemeData::canvas_color` (essentially opaque white).
///  * [`BottomNavigationBarType::Shifting`], the default when there are four
///    or more [`items`](Self::items). If [`selected_item_color`](Self::selected_item_color)
///    is null, all items are rendered in white. The navigation bar's background color is
///    the same as the `BottomNavigationBarItem::background_color` of the selected item. In
///    this case it's assumed that each item will have a different background color
///    and that background color will contrast well with white.
///
/// ## Updating to `NavigationBar`
///
/// The `NavigationBar` widget's visuals
/// are a little bit different, see the Material 3 spec at
/// <https://m3.material.io/components/navigation-bar/overview> for
/// more details.
///
/// The `NavigationBar` widget's API is also slightly different.
/// To update from [`BottomNavigationBar`] to `NavigationBar`, you will
/// need to make the following changes.
///
/// 1. Instead of using [`BottomNavigationBar::items`], which
///    takes a list of [`BottomNavigationBarItem`]s, use
///    `NavigationBar::destinations`, which takes a list of widgets.
///    Usually, you use a list of `NavigationDestination` widgets.
///    Just like [`BottomNavigationBarItem`]s, `NavigationDestination`s
///    have a label and icon field.
///
/// 2. Instead of using [`BottomNavigationBar::on_tap`],
///    use `NavigationBar::on_destination_selected`, which is also
///    a callback that is called when the user taps on a
///    navigation bar item.
///
/// 3. Instead of using [`BottomNavigationBar::current_index`],
///    use `NavigationBar::selected_index`, which is also an integer
///    that represents the index of the selected destination.
///
/// 4. You may also need to make changes to the styling of the
///    `NavigationBar`, see the properties in the `NavigationBar`
///    constructor for more details.
///
/// See also:
///
///  * [`BottomNavigationBarItem`]
///  * `Scaffold`
///  * <https://material.io/design/components/bottom-navigation.html>
///  * `NavigationBar`, this widget's replacement in Material Design 3.
pub struct BottomNavigationBar {
    pub key: Option<KeyRef>,

    /// Defines the appearance of the button items that are arrayed within the
    /// bottom navigation bar.
    pub items: Vec<BottomNavigationBarItem>,

    /// Called when one of the [`items`](Self::items) is tapped.
    ///
    /// The stateful widget that creates the bottom navigation bar needs to keep
    /// track of the index of the selected [`BottomNavigationBarItem`] and call
    /// `set_state` to rebuild the bottom navigation bar with the new
    /// [`current_index`](Self::current_index).
    pub on_tap: Option<ValueChanged<usize>>,

    /// The index into [`items`](Self::items) for the current active
    /// [`BottomNavigationBarItem`].
    pub current_index: usize,

    /// The z-coordinate of this [`BottomNavigationBar`].
    ///
    /// If null, defaults to `8.0`.
    ///
    /// {@macro material_ui.material.elevation}
    pub elevation: Option<f64>,

    /// Defines the layout and behavior of a [`BottomNavigationBar`].
    ///
    /// See documentation for [`BottomNavigationBarType`] for information on the
    /// meaning of different types.
    pub r#type: Option<BottomNavigationBarType>,

    /// What the constructor was given as `fixedColor`; Dart's getter is
    /// [`effective_fixed_color`](Self::effective_fixed_color).
    pub fixed_color: Option<Color>,

    /// The color of the [`BottomNavigationBar`] itself.
    ///
    /// If [`type`](Self::type) is [`BottomNavigationBarType::Shifting`] and the
    /// [`items`](Self::items) have `BottomNavigationBarItem::background_color` set, the
    /// items' background_color will splash and overwrite this color.
    pub background_color: Option<Color>,

    /// The size of all of the [`BottomNavigationBarItem`] icons.
    ///
    /// See `BottomNavigationBarItem::icon` for more information.
    pub icon_size: f64,

    /// What the constructor was given as `selectedItemColor`; Dart's field, which falls
    /// back to `fixedColor`, is
    /// [`effective_selected_item_color`](Self::effective_selected_item_color).
    ///
    /// The color of the selected `BottomNavigationBarItem::icon` and
    /// `BottomNavigationBarItem::label`.
    ///
    /// If null then the ambient `BottomNavigationBarThemeData::selected_item_color`
    /// is used. If that is also null, `ColorScheme::primary` is used when
    /// `ThemeData::brightness` is `Brightness::Light`, and `ColorScheme::secondary`
    /// is used when `ThemeData::brightness` is `Brightness::Dark`.
    pub selected_item_color: Option<Color>,

    /// The color of the unselected `BottomNavigationBarItem::icon` and
    /// `BottomNavigationBarItem::label`s.
    ///
    /// If null then the ambient `BottomNavigationBarThemeData::unselected_item_color`
    /// is used. If that is also null, `ThemeData::unselected_widget_color` is used.
    pub unselected_item_color: Option<Color>,

    /// The size, opacity, and color of the icon in the currently selected
    /// `BottomNavigationBarItem::icon`.
    ///
    /// If this is not provided, the size will default to [`icon_size`](Self::icon_size),
    /// the color will default to [`selected_item_color`](Self::selected_item_color).
    ///
    /// It this field is provided, it must contain non-null `IconThemeData::size`
    /// and `IconThemeData::color` properties. Also, if this field is supplied,
    /// [`unselected_icon_theme`](Self::unselected_icon_theme) must be provided.
    pub selected_icon_theme: Option<IconThemeData>,

    /// The size, opacity, and color of the icon in the currently unselected
    /// `BottomNavigationBarItem::icon`s.
    ///
    /// If this is not provided, the size will default to [`icon_size`](Self::icon_size),
    /// the color will default to [`unselected_item_color`](Self::unselected_item_color).
    ///
    /// It this field is provided, it must contain non-null `IconThemeData::size`
    /// and `IconThemeData::color` properties. Also, if this field is supplied,
    /// [`selected_icon_theme`](Self::selected_icon_theme) must be provided.
    pub unselected_icon_theme: Option<IconThemeData>,

    /// The [`TextStyle`] of the [`BottomNavigationBarItem`] labels when they are
    /// selected.
    pub selected_label_style: Option<TextStyle>,

    /// The [`TextStyle`] of the [`BottomNavigationBarItem`] labels when they are not
    /// selected.
    pub unselected_label_style: Option<TextStyle>,

    /// The font size of the [`BottomNavigationBarItem`] labels when they are selected.
    ///
    /// If `TextStyle::font_size` of [`selected_label_style`](Self::selected_label_style)
    /// is non-null, it will be used instead of this.
    ///
    /// Defaults to `14.0`.
    pub selected_font_size: f64,

    /// The font size of the [`BottomNavigationBarItem`] labels when they are not
    /// selected.
    ///
    /// If `TextStyle::font_size` of
    /// [`unselected_label_style`](Self::unselected_label_style) is non-null, it will be
    /// used instead of this.
    ///
    /// Defaults to `12.0`.
    pub unselected_font_size: f64,

    /// Whether the labels are shown for the unselected [`BottomNavigationBarItem`]s.
    pub show_unselected_labels: Option<bool>,

    /// Whether the labels are shown for the selected [`BottomNavigationBarItem`].
    pub show_selected_labels: Option<bool>,

    /// The cursor for a mouse pointer when it enters or is hovering over the
    /// items.
    ///
    /// If [`mouse_cursor`](Self::mouse_cursor) is a [`WidgetStateMouseCursor`], its
    /// `resolve` method can define the appearance of the cursor depending on whether
    /// [`WidgetState::Selected`] is active.
    ///
    /// If null, then the value of `BottomNavigationBarThemeData::mouse_cursor` is used.
    /// If that is also null, then [`WidgetStateMouseCursor::clickable`] is used.
    pub mouse_cursor: Option<MouseCursorRef>,

    /// Whether detected gestures should provide acoustic and/or haptic feedback.
    ///
    /// For example, on Android a tap will produce a clicking sound and a
    /// long-press will produce a short vibration, when feedback is enabled.
    ///
    /// See also:
    ///
    ///  * `Feedback` for providing platform-specific feedback to certain actions.
    pub enable_feedback: Option<bool>,

    /// The arrangement of the bar's [`items`](Self::items) when the enclosing
    /// `MediaQueryData::orientation` is `Orientation::Landscape`.
    ///
    /// The following alternatives are supported:
    ///
    /// * [`BottomNavigationBarLandscapeLayout::Spread`] - the items are
    ///   evenly spaced and spread out across the available width. Each
    ///   item's label and icon are arranged in a column.
    /// * [`BottomNavigationBarLandscapeLayout::Centered`] - the items are
    ///   evenly spaced in a row but only consume as much width as they
    ///   would in portrait orientation. The row of items is centered within
    ///   the available width. Each item's label and icon are arranged
    ///   in a column.
    /// * [`BottomNavigationBarLandscapeLayout::Linear`] - the items are
    ///   evenly spaced and each item's icon and label are lined up in a
    ///   row instead of a column.
    ///
    /// If this property is null, then the value of the enclosing
    /// `BottomNavigationBarThemeData::landscape_layout` is used. If that
    /// property is also null, then
    /// [`BottomNavigationBarLandscapeLayout::Spread`] is used.
    ///
    /// This property is null by default.
    ///
    /// See also:
    ///
    ///  * `ThemeData::bottom_navigation_bar_theme` - which can be used to specify
    ///    bottom navigation bar defaults for an entire application.
    ///  * [`BottomNavigationBarTheme`] - which can be used to specify
    ///    bottom navigation bar defaults for a widget subtree.
    ///  * [`MediaQuery::orientation_of`] - which can be used to determine the current
    ///    orientation.
    pub landscape_layout: Option<BottomNavigationBarLandscapeLayout>,

    /// This flag is controlling how [`BottomNavigationBar`] is going to use
    /// the colors provided by the [`selected_icon_theme`](Self::selected_icon_theme),
    /// [`unselected_icon_theme`](Self::unselected_icon_theme),
    /// [`selected_item_color`](Self::selected_item_color),
    /// [`unselected_item_color`](Self::unselected_item_color).
    /// The default value is `true` as the new theming logic is a breaking change.
    /// To opt-in the new theming logic set the flag to `false`
    pub use_legacy_color_scheme: bool,
}

impl BottomNavigationBar {
    /// Creates a bottom navigation bar which is typically used as a
    /// `Scaffold`'s `Scaffold::bottom_navigation_bar` argument.
    ///
    /// The length of [`items`](Self::items) must be at least two and each item's icon and
    /// label must not be null.
    ///
    /// If [`type`](Self::type) is null then [`BottomNavigationBarType::Fixed`] is used when
    /// there are two or three [`items`](Self::items), [`BottomNavigationBarType::Shifting`]
    /// otherwise.
    ///
    /// The [`icon_size`](Self::icon_size), [`selected_font_size`](Self::selected_font_size),
    /// [`unselected_font_size`](Self::unselected_font_size), and
    /// [`elevation`](Self::elevation) arguments must be non-negative.
    ///
    /// If [`selected_label_style`](Self::selected_label_style).color and
    /// [`unselected_label_style`](Self::unselected_label_style).color values
    /// are non-null, they will be used instead of
    /// [`selected_item_color`](Self::selected_item_color) and
    /// [`unselected_item_color`](Self::unselected_item_color).
    ///
    /// If custom [`IconThemeData`]s are used, you must provide both
    /// [`selected_icon_theme`](Self::selected_icon_theme) and
    /// [`unselected_icon_theme`](Self::unselected_icon_theme), and both
    /// `IconThemeData::color` and `IconThemeData::size` must be set.
    ///
    /// If [`use_legacy_color_scheme`](Self::use_legacy_color_scheme) is set to `false`
    /// [`selected_icon_theme`](Self::selected_icon_theme) values will be used instead of
    /// [`icon_size`](Self::icon_size) and [`selected_item_color`](Self::selected_item_color)
    /// for selected icons.
    /// [`unselected_icon_theme`](Self::unselected_icon_theme) values will be used instead of
    /// [`icon_size`](Self::icon_size) and
    /// [`unselected_item_color`](Self::unselected_item_color) for unselected icons.
    ///
    /// If both [`selected_label_style`](Self::selected_label_style).font_size and
    /// [`selected_font_size`](Self::selected_font_size) are set,
    /// [`selected_label_style`](Self::selected_label_style).font_size will be used.
    ///
    /// Only one of [`selected_item_color`](Self::selected_item_color) and
    /// [`fixed_color`](Self::fixed_color) can be specified. The former is preferred,
    /// [`fixed_color`](Self::fixed_color) only exists for the sake of backwards
    /// compatibility.
    ///
    /// If [`show_selected_labels`](Self::show_selected_labels) is `null`,
    /// `BottomNavigationBarThemeData::show_selected_labels` is used. If
    /// `BottomNavigationBarThemeData::show_selected_labels` is null, then
    /// [`show_selected_labels`](Self::show_selected_labels) defaults to `true`.
    ///
    /// If [`show_unselected_labels`](Self::show_unselected_labels) is `null`,
    /// `BottomNavigationBarThemeData::show_unselected_labels` is used. If
    /// `BottomNavigationBarThemeData::show_selected_labels` is null, then
    /// [`show_unselected_labels`](Self::show_unselected_labels) defaults to `true` when
    /// [`type`](Self::type) is [`BottomNavigationBarType::Fixed`] and `false` when
    /// [`type`](Self::type) is [`BottomNavigationBarType::Shifting`].
    pub fn new(items: Vec<BottomNavigationBarItem>) -> BottomNavigationBar {
        BottomNavigationBar {
            key: None,
            items,
            on_tap: None,
            current_index: 0,
            elevation: None,
            r#type: None,
            fixed_color: None,
            background_color: None,
            icon_size: 24.0,
            selected_item_color: None,
            unselected_item_color: None,
            selected_icon_theme: None,
            unselected_icon_theme: None,
            selected_font_size: 14.0,
            unselected_font_size: 12.0,
            selected_label_style: None,
            unselected_label_style: None,
            show_selected_labels: None,
            show_unselected_labels: None,
            mouse_cursor: None,
            enable_feedback: None,
            landscape_layout: None,
            use_legacy_color_scheme: true,
        }
    }

    /// Dart `BottomNavigationBar(key:)`.
    pub fn key(mut self, key: KeyRef) -> BottomNavigationBar {
        self.key = Some(key);
        self
    }

    /// Dart `BottomNavigationBar(onTap:)`.
    pub fn on_tap(mut self, on_tap: impl Fn(&mut App, usize) + 'static) -> BottomNavigationBar {
        self.on_tap = Some(Rc::new(on_tap));
        self
    }

    /// Dart `BottomNavigationBar(currentIndex:)`.
    pub fn current_index(mut self, current_index: usize) -> BottomNavigationBar {
        self.current_index = current_index;
        self
    }

    /// Dart `BottomNavigationBar(elevation:)`.
    pub fn elevation(mut self, elevation: f64) -> BottomNavigationBar {
        debug_assert!(elevation >= 0.0);
        self.elevation = Some(elevation);
        self
    }

    /// Dart `BottomNavigationBar(type:)`.
    pub fn r#type(mut self, r#type: BottomNavigationBarType) -> BottomNavigationBar {
        self.r#type = Some(r#type);
        self
    }

    /// Dart `BottomNavigationBar(fixedColor:)`.
    pub fn fixed_color(mut self, fixed_color: Color) -> BottomNavigationBar {
        self.fixed_color = Some(fixed_color);
        self
    }

    /// Dart `BottomNavigationBar(backgroundColor:)`.
    pub fn background_color(mut self, background_color: Color) -> BottomNavigationBar {
        self.background_color = Some(background_color);
        self
    }

    /// Dart `BottomNavigationBar(iconSize:)`.
    pub fn icon_size(mut self, icon_size: f64) -> BottomNavigationBar {
        debug_assert!(icon_size >= 0.0);
        self.icon_size = icon_size;
        self
    }

    /// Dart `BottomNavigationBar(selectedItemColor:)`.
    pub fn selected_item_color(mut self, selected_item_color: Color) -> BottomNavigationBar {
        self.selected_item_color = Some(selected_item_color);
        self
    }

    /// Dart `BottomNavigationBar(unselectedItemColor:)`.
    pub fn unselected_item_color(mut self, unselected_item_color: Color) -> BottomNavigationBar {
        self.unselected_item_color = Some(unselected_item_color);
        self
    }

    /// Dart `BottomNavigationBar(selectedIconTheme:)`.
    pub fn selected_icon_theme(
        mut self,
        selected_icon_theme: IconThemeData,
    ) -> BottomNavigationBar {
        self.selected_icon_theme = Some(selected_icon_theme);
        self
    }

    /// Dart `BottomNavigationBar(unselectedIconTheme:)`.
    pub fn unselected_icon_theme(
        mut self,
        unselected_icon_theme: IconThemeData,
    ) -> BottomNavigationBar {
        self.unselected_icon_theme = Some(unselected_icon_theme);
        self
    }

    /// Dart `BottomNavigationBar(selectedFontSize:)`.
    pub fn selected_font_size(mut self, selected_font_size: f64) -> BottomNavigationBar {
        debug_assert!(selected_font_size >= 0.0);
        self.selected_font_size = selected_font_size;
        self
    }

    /// Dart `BottomNavigationBar(unselectedFontSize:)`.
    pub fn unselected_font_size(mut self, unselected_font_size: f64) -> BottomNavigationBar {
        debug_assert!(unselected_font_size >= 0.0);
        self.unselected_font_size = unselected_font_size;
        self
    }

    /// Dart `BottomNavigationBar(selectedLabelStyle:)`.
    pub fn selected_label_style(mut self, selected_label_style: TextStyle) -> BottomNavigationBar {
        self.selected_label_style = Some(selected_label_style);
        self
    }

    /// Dart `BottomNavigationBar(unselectedLabelStyle:)`.
    pub fn unselected_label_style(
        mut self,
        unselected_label_style: TextStyle,
    ) -> BottomNavigationBar {
        self.unselected_label_style = Some(unselected_label_style);
        self
    }

    /// Dart `BottomNavigationBar(showSelectedLabels:)`.
    pub fn show_selected_labels(mut self, show_selected_labels: bool) -> BottomNavigationBar {
        self.show_selected_labels = Some(show_selected_labels);
        self
    }

    /// Dart `BottomNavigationBar(showUnselectedLabels:)`.
    pub fn show_unselected_labels(mut self, show_unselected_labels: bool) -> BottomNavigationBar {
        self.show_unselected_labels = Some(show_unselected_labels);
        self
    }

    /// Dart `BottomNavigationBar(mouseCursor:)`.
    pub fn mouse_cursor(mut self, mouse_cursor: impl Into<MouseCursorRef>) -> BottomNavigationBar {
        self.mouse_cursor = Some(mouse_cursor.into());
        self
    }

    /// Dart `BottomNavigationBar(enableFeedback:)`.
    pub fn enable_feedback(mut self, enable_feedback: bool) -> BottomNavigationBar {
        self.enable_feedback = Some(enable_feedback);
        self
    }

    /// Dart `BottomNavigationBar(landscapeLayout:)`.
    pub fn landscape_layout(
        mut self,
        landscape_layout: BottomNavigationBarLandscapeLayout,
    ) -> BottomNavigationBar {
        self.landscape_layout = Some(landscape_layout);
        self
    }

    /// Dart `BottomNavigationBar(useLegacyColorScheme:)`.
    pub fn use_legacy_color_scheme(mut self, use_legacy_color_scheme: bool) -> BottomNavigationBar {
        self.use_legacy_color_scheme = use_legacy_color_scheme;
        self
    }

    /// Dart's `selectedItemColor` field, as its constructor fills it:
    /// `selectedItemColor ?? fixedColor`.
    pub fn effective_selected_item_color(&self) -> Option<Color> {
        self.selected_item_color.or(self.fixed_color)
    }

    /// The value of [`selected_item_color`](Self::selected_item_color).
    ///
    /// This getter only exists for backwards compatibility, the
    /// [`selected_item_color`](Self::selected_item_color) property is preferred.
    pub fn effective_fixed_color(&self) -> Option<Color> {
        self.effective_selected_item_color()
    }
}

impl Debug for BottomNavigationBar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BottomNavigationBar")
            .field("items", &self.items)
            .field("current_index", &self.current_index)
            .field("type", &self.r#type)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for BottomNavigationBar {
    type State = BottomNavigationBarState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn debug_check_constructor(&self) {
        debug_assert!(self.items.len() >= 2);
        debug_assert!(
            self.items.iter().all(|item| item.label.is_some()),
            "Every item must have a non-null label"
        );
        debug_assert!(self.current_index < self.items.len());
        debug_assert!(
            self.selected_item_color.is_none() || self.fixed_color.is_none(),
            "Either selectedItemColor or fixedColor can be specified, but not both"
        );
    }

    fn create_state(&self) -> BottomNavigationBarState {
        BottomNavigationBarState {
            state: StateData::new(),
            ticker_provider: TickerProviderStateMixinData::new(),
            controllers: Vec::new(),
            animations: Vec::new(),
            fade_out_animations: Vec::new(),
            circles: VecDeque::new(),
            background_color: None,
            flex_tween: None,
            color_tweens: None,
        }
    }
}

/// Dart's `Tween<double>(begin: begin, end: end).evaluate(animation)`.
fn evaluate_tween(app: &mut App, begin: f64, end: f64, animation: AnyAnimation<f64>) -> f64 {
    let tween = Tween::new(app, Some(begin), Some(end));
    let value = tween.evaluate(app, animation);
    app.destroy(tween);
    value
}

/// Dart's `Tween<double>(begin: begin, end: end).transform(t)`.
fn transform_tween(app: &mut App, begin: f64, end: f64, t: f64) -> f64 {
    let tween = Tween::new(app, Some(begin), Some(end));
    let value = tween.transform(app, t);
    app.destroy(tween);
    value
}

// This represents a single tile in the bottom navigation bar. It is intended
// to go into a flex container.
struct BottomNavigationTile {
    r#type: BottomNavigationBarType,
    item: BottomNavigationBarItem,
    animation: AnyAnimation<f64>,
    /// The state's `Tween<double>(begin: 1.0, end: 0.0).animate(animation)`, which `_Label`
    /// fades selected labels out with.
    fade_out_animation: AnyAnimation<f64>,
    icon_size: f64,
    key: Option<KeyRef>,
    on_tap: Option<Listener>,
    label_color_tween: Option<Handle<ColorTween>>,
    icon_color_tween: Option<Handle<ColorTween>>,
    flex: Option<f64>,
    selected: bool,
    selected_icon_theme: Option<IconThemeData>,
    unselected_icon_theme: Option<IconThemeData>,
    selected_label_style: TextStyle,
    unselected_label_style: TextStyle,
    index_label: Option<String>,
    show_selected_labels: bool,
    show_unselected_labels: bool,
    mouse_cursor: MouseCursorRef,
    enable_feedback: bool,
    layout: BottomNavigationBarLandscapeLayout,
}

impl Debug for BottomNavigationTile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_BottomNavigationTile")
            .field("type", &self.r#type)
            .field("item", &self.item)
            .field("selected", &self.selected)
            .finish_non_exhaustive()
    }
}

impl StatelessWidget for BottomNavigationTile {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, _context: BuildContext) -> WidgetRef {
        let selected_font_size = self
            .selected_label_style
            .font_size
            .expect("the bar gives its label styles a font size");

        let selected_icon_size = self
            .selected_icon_theme
            .as_ref()
            .and_then(|theme| theme.size)
            .unwrap_or(self.icon_size);
        let unselected_icon_size = self
            .unselected_icon_theme
            .as_ref()
            .and_then(|theme| theme.size)
            .unwrap_or(self.icon_size);

        // The amount that the selected icon is bigger than the unselected icons,
        // (or zero if the selected icon is not bigger than the unselected icons).
        let selected_icon_diff = (selected_icon_size - unselected_icon_size).max(0.0);
        // The amount that the unselected icons are bigger than the selected icon,
        // (or zero if the unselected icons are not any bigger than the selected icon).
        let unselected_icon_diff = (unselected_icon_size - selected_icon_size).max(0.0);

        // The effective tool tip message to be shown on the BottomNavigationBarItem.
        let effective_tooltip = self
            .item
            .tooltip
            .clone()
            .filter(|tooltip| !tooltip.is_empty());

        // Defines the padding for the animating icons + labels.
        //
        // The animations go from "Unselected":
        // =======
        // |      <-- Padding equal to the text height + 1/2 selectedIconDiff.
        // |  ☆
        // | text <-- Invisible text + padding equal to 1/2 selectedIconDiff.
        // =======
        //
        // To "Selected":
        //
        // =======
        // |      <-- Padding equal to 1/2 text height + 1/2 unselectedIconDiff.
        // |  ☆
        // | text
        // |      <-- Padding equal to 1/2 text height + 1/2 unselectedIconDiff.
        // =======
        let (bottom_padding, top_padding) =
            if self.show_selected_labels && !self.show_unselected_labels {
                (
                    evaluate_tween(
                        app,
                        selected_icon_diff / 2.0,
                        selected_font_size / 2.0 - unselected_icon_diff / 2.0,
                        self.animation,
                    ),
                    evaluate_tween(
                        app,
                        selected_font_size + selected_icon_diff / 2.0,
                        selected_font_size / 2.0 - unselected_icon_diff / 2.0,
                        self.animation,
                    ),
                )
            } else if !self.show_selected_labels && !self.show_unselected_labels {
                (
                    evaluate_tween(
                        app,
                        selected_icon_diff / 2.0,
                        unselected_icon_diff / 2.0,
                        self.animation,
                    ),
                    evaluate_tween(
                        app,
                        selected_font_size + selected_icon_diff / 2.0,
                        selected_font_size + unselected_icon_diff / 2.0,
                        self.animation,
                    ),
                )
            } else {
                (
                    evaluate_tween(
                        app,
                        selected_font_size / 2.0 + selected_icon_diff / 2.0,
                        selected_font_size / 2.0 + unselected_icon_diff / 2.0,
                        self.animation,
                    ),
                    evaluate_tween(
                        app,
                        selected_font_size / 2.0 + selected_icon_diff / 2.0,
                        selected_font_size / 2.0 + unselected_icon_diff / 2.0,
                        self.animation,
                    ),
                )
            };

        // In order to use the flex container to grow the tile during animation, we
        // need to divide the changes in flex allotment into smaller pieces to
        // produce smooth animation. We do this by multiplying the flex value
        // (which is an integer) by a large number.
        let size: i32 = match self.r#type {
            BottomNavigationBarType::Fixed => 1,
            BottomNavigationBarType::Shifting => {
                (self.flex.expect("a shifting tile has a flex") * 1000.0).round() as i32
            }
        };

        let mut ink_response = InkResponse::new()
            .mouse_cursor(self.mouse_cursor.clone())
            .enable_feedback(self.enable_feedback)
            .child(
                Padding::new(EdgeInsetsGeometry::only(
                    0.0,
                    top_padding,
                    0.0,
                    bottom_padding,
                ))
                .child(Tile {
                    layout: self.layout,
                    icon: TileIcon {
                        color_tween: self
                            .icon_color_tween
                            .expect("the bar gives each tile an icon color tween"),
                        animation: self.animation,
                        icon_size: self.icon_size,
                        selected: self.selected,
                        item: self.item.clone(),
                        selected_icon_theme: self.selected_icon_theme.clone(),
                        unselected_icon_theme: self.unselected_icon_theme.clone(),
                    }
                    .into_widget(),
                    label: Label {
                        color_tween: self
                            .label_color_tween
                            .expect("the bar gives each tile a label color tween"),
                        animation: self.animation,
                        fade_out_animation: self.fade_out_animation,
                        item: self.item.clone(),
                        selected_label_style: self.selected_label_style.clone(),
                        unselected_label_style: self.unselected_label_style.clone(),
                        show_selected_labels: self.show_selected_labels,
                        show_unselected_labels: self.show_unselected_labels,
                    }
                    .into_widget(),
                }),
            );
        if let Some(on_tap) = &self.on_tap {
            ink_response = ink_response.on_tap(on_tap.clone());
        }
        let mut result = ink_response.into_widget();

        if let Some(effective_tooltip) = effective_tooltip {
            result = Tooltip::new()
                .message(effective_tooltip)
                .prefer_below(false)
                .vertical_offset(selected_icon_size + selected_font_size)
                .exclude_from_semantics(true)
                .child(result)
                .into_widget();
        }

        let mut index_semantics = Semantics::new();
        if let Some(index_label) = &self.index_label {
            index_semantics = index_semantics.label(index_label.clone());
        }
        result = Semantics::new()
            .selected(self.selected)
            .button(true)
            .container(true)
            .child(Stack::new().children([result, index_semantics.into_widget()]))
            .into_widget();

        Expanded::new(result).flex(size).into_widget()
    }
}

// If the orientation is landscape and layout is
// BottomNavigationBarLandscapeLayout.linear then return a
// icon-space-label row, where space is 8 pixels. Otherwise return a
// icon-label column.
#[derive(Debug)]
struct Tile {
    layout: BottomNavigationBarLandscapeLayout,
    icon: WidgetRef,
    label: WidgetRef,
}

impl StatelessWidget for Tile {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        if MediaQuery::orientation_of(app, context) == Orientation::Landscape
            && self.layout == BottomNavigationBarLandscapeLayout::Linear
        {
            return Align::new()
                .height_factor(1.0)
                .child(
                    Row::new()
                        .main_axis_size(MainAxisSize::Min)
                        .spacing(8.0)
                        .children([
                            self.icon.clone(),
                            // Flexible lets the overflow property of
                            // label to work and IntrinsicWidth gives label a
                            // reasonable width preventing extra space before it.
                            Flexible::new(IntrinsicWidth::new().child(self.label.clone()))
                                .into_widget(),
                        ]),
                )
                .into_widget();
        }
        Column::new()
            .main_axis_alignment(MainAxisAlignment::SpaceBetween)
            .main_axis_size(MainAxisSize::Min)
            .children([self.icon.clone(), self.label.clone()])
            .into_widget()
    }
}

struct TileIcon {
    color_tween: Handle<ColorTween>,
    animation: AnyAnimation<f64>,
    icon_size: f64,
    selected: bool,
    item: BottomNavigationBarItem,
    selected_icon_theme: Option<IconThemeData>,
    unselected_icon_theme: Option<IconThemeData>,
}

impl Debug for TileIcon {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_TileIcon")
            .field("icon_size", &self.icon_size)
            .field("selected", &self.selected)
            .field("item", &self.item)
            .finish_non_exhaustive()
    }
}

impl StatelessWidget for TileIcon {
    fn build(&self, app: &mut App, _context: BuildContext) -> WidgetRef {
        let icon_color = self.color_tween.evaluate(app, self.animation);
        let mut default_icon_theme = IconThemeData::new().size(self.icon_size);
        default_icon_theme.color = icon_color.map(Into::into);
        let icon_theme_data = IconThemeData::lerp(
            Some(&default_icon_theme.merge(self.unselected_icon_theme.as_ref())),
            Some(&default_icon_theme.merge(self.selected_icon_theme.as_ref())),
            self.animation.value(app),
        );

        Align::new()
            .alignment(AlignmentGeometry::TOP_CENTER)
            .height_factor(1.0)
            .child(IconTheme::new(
                icon_theme_data,
                if self.selected {
                    self.item.active_icon.clone()
                } else {
                    self.item.icon.clone()
                },
            ))
            .into_widget()
    }
}

struct Label {
    color_tween: Handle<ColorTween>,
    animation: AnyAnimation<f64>,
    /// The state's `Tween<double>(begin: 1.0, end: 0.0).animate(animation)`.
    fade_out_animation: AnyAnimation<f64>,
    item: BottomNavigationBarItem,
    selected_label_style: TextStyle,
    unselected_label_style: TextStyle,
    show_selected_labels: bool,
    show_unselected_labels: bool,
}

impl Debug for Label {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_Label")
            .field("item", &self.item)
            .field("show_selected_labels", &self.show_selected_labels)
            .field("show_unselected_labels", &self.show_unselected_labels)
            .finish_non_exhaustive()
    }
}

impl StatelessWidget for Label {
    fn build(&self, app: &mut App, _context: BuildContext) -> WidgetRef {
        let selected_font_size = self.selected_label_style.font_size;
        let unselected_font_size = self.unselected_label_style.font_size;

        let custom_style = TextStyle::lerp(
            Some(&self.unselected_label_style),
            Some(&self.selected_label_style),
            self.animation.value(app),
        )
        .expect("both styles are given");
        let mut style = custom_style.copy_with();
        if let Some(selected_font_size) = selected_font_size {
            style = style.font_size(selected_font_size);
        }
        if let Some(color) = self.color_tween.evaluate(app, self.animation) {
            style = style.color(color);
        }
        let scale = evaluate_tween(
            app,
            unselected_font_size.expect("the bar gives its label styles a font size")
                / selected_font_size.expect("the bar gives its label styles a font size"),
            1.0,
            self.animation,
        );
        let mut label = Text::new(self.item.label.clone().expect("every item has a label"));
        if let Some(semantics_label) = &self.item.semantics_label {
            label = label.semantics_label(semantics_label.clone());
        }
        let mut text = DefaultTextStyle::merge(
            // The font size should grow here when active, but because of the way
            // font rendering works, it doesn't grow smoothly if we just animate
            // the font size, so we use a transform instead.
            Transform::new(diagonal3_all(scale))
                .alignment(AlignmentGeometry::BOTTOM_CENTER)
                .child(label),
        )
        .style(style)
        .into_widget();

        if !self.show_unselected_labels && !self.show_selected_labels {
            // Never show any labels.
            text = Visibility::maintain(text).visible(false).into_widget();
        } else if !self.show_unselected_labels {
            // Fade selected labels in.
            text = FadeTransition::new(self.animation)
                .child(text)
                .into_widget();
        } else if !self.show_selected_labels {
            // Fade selected labels out.
            text = FadeTransition::new(self.fade_out_animation)
                .child(text)
                .into_widget();
        }

        text = Align::new()
            .alignment(AlignmentGeometry::BOTTOM_CENTER)
            .height_factor(1.0)
            .child(text)
            .into_widget();

        if self.item.label.is_some() {
            // Do not grow text in bottom navigation bar when we can show a tooltip
            // instead.
            text = MediaQuery::with_clamped_text_scaling(None, 0.0, 1.0, text);
        }

        text
    }
}

/// Dart's `Matrix4.diagonal3(Vector3.all(scale))`.
fn diagonal3_all(scale: f64) -> Matrix4 {
    let scale = scale as f32;
    Matrix4::from_flutter_array(&[
        scale, 0.0, 0.0, 0.0, //
        0.0, scale, 0.0, 0.0, //
        0.0, 0.0, scale, 0.0, //
        0.0, 0.0, 0.0, 1.0,
    ])
}

/// The three `ColorTween`s a build of the bar hands its tiles: the legacy one for both
/// labels and icons, and the label's and icon's of the newer colour scheme.
struct ColorTweens {
    color_tween: Handle<ColorTween>,
    label_color_tween: Handle<ColorTween>,
    icon_color_tween: Handle<ColorTween>,
}

/// Dart's `_BottomNavigationBarState`.
pub struct BottomNavigationBarState {
    state: StateData<BottomNavigationBar>,
    ticker_provider: TickerProviderStateMixinData,
    controllers: Vec<Handle<AnimationController>>,
    animations: Vec<Handle<CurvedAnimation>>,
    /// One `Tween<double>(begin: 1.0, end: 0.0).animate(animation)` per animation, which
    /// Dart's `_Label` makes on every build.
    fade_out_animations: Vec<(Handle<Tween<f64>>, AnyAnimation<f64>)>,

    // A queue of color splashes currently being animated.
    circles: VecDeque<Rc<Circle>>,

    // Last splash circle's color, and the final color of the control after
    // animation is complete.
    background_color: Option<Color>,

    /// Dart's `static final Animatable<double> _flexTween`, one per state.
    flex_tween: Option<Handle<Tween<f64>>>,

    /// The colour tweens `_createTiles` sets for each build, made in `init_state`.
    color_tweens: Option<ColorTweens>,
}

impl BottomNavigationBarState {
    fn dispose_animations(self: Handle<Self>, app: &mut App) {
        let state = app.get_mut(self);
        let controllers = std::mem::take(&mut state.controllers);
        let circles = std::mem::take(&mut state.circles);
        let animations = std::mem::take(&mut state.animations);
        let fade_out_animations = std::mem::take(&mut state.fade_out_animations);
        for controller in controllers {
            controller.dispose(app);
            app.destroy(controller);
        }
        for circle in circles {
            circle.dispose(app);
            circle.destroy(app);
        }
        for animation in animations {
            animation.dispose(app);
            app.destroy(animation);
        }
        for (tween, animation) in fade_out_animations {
            app.destroy(animation.id());
            app.destroy(tween);
        }
    }

    fn reset_state(self: Handle<Self>, app: &mut App) {
        self.dispose_animations(app);

        let items_length = self.widget(app).items.len();
        let controllers: Vec<Handle<AnimationController>> = (0..items_length)
            .map(|_| {
                let controller = AnimationController::create(
                    app,
                    None,
                    Some(K_THEME_ANIMATION_DURATION),
                    None,
                    0.0,
                    1.0,
                    AnimationBehavior::Normal,
                    self,
                );
                controller.add_listener(app, Listener::handle_method(self, Self::rebuild));
                controller
            })
            .collect();
        let animations: Vec<Handle<CurvedAnimation>> = controllers
            .iter()
            .map(|controller| {
                CurvedAnimation::create(
                    app,
                    controller.as_animation(),
                    Curves::fast_out_slow_in(),
                    Some(Rc::new(Curves::fast_out_slow_in().flipped())),
                )
            })
            .collect();
        let fade_out_animations = animations
            .iter()
            .map(|animation| {
                let tween = Tween::new(app, Some(1.0), Some(0.0));
                (tween, tween.animate(app, animation.as_animation()))
            })
            .collect();
        let current_index = self.widget(app).current_index;
        controllers[current_index].set_value(app, 1.0);
        let background_color = self.widget(app).items[current_index]
            .background_color
            .as_ref()
            .map(|color| color.color());
        let state = app.get_mut(self);
        state.controllers = controllers;
        state.animations = animations;
        state.fade_out_animations = fade_out_animations;
        state.background_color = background_color;
    }

    // Computes the default value for the [type] parameter.
    //
    // If type is provided, it is returned. Next, if the bottom navigation bar
    // theme provides a type, it is used. Finally, the default behavior will be
    // [BottomNavigationBarType.fixed] for 3 or fewer items, and
    // [BottomNavigationBarType.shifting] is used for 4+ items.
    fn effective_type(self: Handle<Self>, app: &mut App) -> BottomNavigationBarType {
        if let Some(r#type) = self.widget(app).r#type {
            return r#type;
        }
        let context = self.context(app);
        BottomNavigationBarTheme::of(app, context).r#type.unwrap_or(
            if self.widget(app).items.len() <= 3 {
                BottomNavigationBarType::Fixed
            } else {
                BottomNavigationBarType::Shifting
            },
        )
    }

    // Computes the default value for the [showUnselected] parameter.
    //
    // Unselected labels are shown by default for [BottomNavigationBarType.fixed],
    // and hidden by default for [BottomNavigationBarType.shifting].
    fn default_show_unselected(self: Handle<Self>, app: &mut App) -> bool {
        match self.effective_type(app) {
            BottomNavigationBarType::Shifting => false,
            BottomNavigationBarType::Fixed => true,
        }
    }

    fn rebuild(self: Handle<Self>, app: &mut App) {
        self.set_state(app, |_| {
            // Rebuilding when any of the controllers tick, i.e. when the items are
            // animated.
        });
    }

    fn evaluate_flex(self: Handle<Self>, app: &App, animation: AnyAnimation<f64>) -> f64 {
        app.get(self)
            .flex_tween
            .expect("made in init_state")
            .evaluate(app, animation)
    }

    fn push_circle(self: Handle<Self>, app: &mut App, index: usize) {
        let Some(color) = self.widget(app).items[index]
            .background_color
            .as_ref()
            .map(|color| color.color())
        else {
            return;
        };
        let circle = Rc::new(Circle::new(app, self, index, color, self));
        circle.controller.add_status_listener(
            app,
            AnimationStatusListener::new(move |status, app| {
                if status.is_completed() {
                    let mut removed = None;
                    self.set_state(app, |state| {
                        let circle = state
                            .circles
                            .pop_front()
                            .expect("a completed circle is queued");
                        state.background_color = Some(circle.color);
                        removed = Some(circle);
                    });
                    let circle = removed.expect("set_state runs its callback");
                    circle.dispose(app);
                    circle.destroy_lingering(app);
                }
            }),
        );
        app.get_mut(self).circles.push_back(circle);
    }

    // If the given [TextStyle] has a non-null `fontSize`, it should be used.
    // Otherwise, the [selectedFontSize] parameter should be used.
    fn effective_text_style(text_style: Option<TextStyle>, font_size: f64) -> TextStyle {
        let text_style = text_style.unwrap_or_default();
        // Prefer the font size on textStyle if present.
        if text_style.font_size.is_none() {
            text_style.copy_with().font_size(font_size)
        } else {
            text_style
        }
    }

    // If [IconThemeData] is provided, it should be used.
    // Otherwise, the [IconThemeData]'s color should be selectedItemColor
    // or unselectedItemColor.
    fn effective_icon_theme(
        icon_theme: Option<IconThemeData>,
        item_color: Option<Color>,
    ) -> IconThemeData {
        // Prefer the iconTheme over itemColor if present.
        icon_theme.unwrap_or_else(|| {
            let mut icon_theme = IconThemeData::new();
            icon_theme.color = item_color.map(Into::into);
            icon_theme
        })
    }

    fn create_tiles(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        layout: BottomNavigationBarLandscapeLayout,
    ) -> Vec<WidgetRef> {
        let localizations = <dyn MaterialLocalizations>::of(app, context);

        let theme_data = Theme::of(app, context);
        let bottom_theme = BottomNavigationBarTheme::of(app, context);

        let theme_color = match theme_data.brightness() {
            Brightness::Light => theme_data.color_scheme.primary,
            Brightness::Dark => theme_data.color_scheme.secondary,
        };

        let widget = self.widget(app);
        let effective_selected_label_style = Self::effective_text_style(
            widget
                .selected_label_style
                .clone()
                .or(bottom_theme.selected_label_style.clone()),
            widget.selected_font_size,
        );

        let effective_unselected_label_style = Self::effective_text_style(
            widget
                .unselected_label_style
                .clone()
                .or(bottom_theme.unselected_label_style.clone()),
            widget.unselected_font_size,
        );

        let effective_selected_icon_theme = Self::effective_icon_theme(
            widget
                .selected_icon_theme
                .clone()
                .or(bottom_theme.selected_icon_theme.clone()),
            Some(
                widget
                    .effective_selected_item_color()
                    .or(bottom_theme.selected_item_color)
                    .unwrap_or(theme_color),
            ),
        );

        let effective_unselected_icon_theme = Self::effective_icon_theme(
            widget
                .unselected_icon_theme
                .clone()
                .or(bottom_theme.unselected_icon_theme.clone()),
            Some(
                widget
                    .unselected_item_color
                    .or(bottom_theme.unselected_item_color)
                    .unwrap_or(theme_data.unselected_widget_color),
            ),
        );

        let icon_theme_color =
            |icon_theme: &IconThemeData| icon_theme.color.as_ref().map(|color| color.color());
        let label_style_color = |style: &TextStyle| style.color.as_ref().map(|color| color.color());
        let effective_type = self.effective_type(app);
        let widget = self.widget(app);

        let color_tween_ends = match effective_type {
            BottomNavigationBarType::Fixed => (
                widget
                    .unselected_item_color
                    .or(bottom_theme.unselected_item_color)
                    .unwrap_or(theme_data.unselected_widget_color),
                widget
                    .effective_selected_item_color()
                    .or(bottom_theme.selected_item_color)
                    .or(widget.effective_fixed_color())
                    .unwrap_or(theme_color),
            ),
            BottomNavigationBarType::Shifting => (
                widget
                    .unselected_item_color
                    .or(bottom_theme.unselected_item_color)
                    .unwrap_or(theme_data.color_scheme.surface),
                widget
                    .effective_selected_item_color()
                    .or(bottom_theme.selected_item_color)
                    .unwrap_or(theme_data.color_scheme.surface),
            ),
        };

        let label_color_tween_ends = match effective_type {
            BottomNavigationBarType::Fixed => (
                label_style_color(&effective_unselected_label_style)
                    .or(widget.unselected_item_color)
                    .or(bottom_theme.unselected_item_color)
                    .unwrap_or(theme_data.unselected_widget_color),
                label_style_color(&effective_selected_label_style)
                    .or(widget.effective_selected_item_color())
                    .or(bottom_theme.selected_item_color)
                    .or(widget.effective_fixed_color())
                    .unwrap_or(theme_color),
            ),
            BottomNavigationBarType::Shifting => (
                label_style_color(&effective_unselected_label_style)
                    .or(widget.unselected_item_color)
                    .or(bottom_theme.unselected_item_color)
                    .unwrap_or(theme_data.color_scheme.surface),
                label_style_color(&effective_selected_label_style)
                    .or(widget.effective_selected_item_color())
                    .or(bottom_theme.selected_item_color)
                    .unwrap_or(theme_color),
            ),
        };

        let icon_color_tween_ends = match effective_type {
            BottomNavigationBarType::Fixed => (
                icon_theme_color(&effective_selected_icon_theme)
                    .or(widget.unselected_item_color)
                    .or(bottom_theme.unselected_item_color)
                    .unwrap_or(theme_data.unselected_widget_color),
                icon_theme_color(&effective_unselected_icon_theme)
                    .or(widget.effective_selected_item_color())
                    .or(bottom_theme.selected_item_color)
                    .or(widget.effective_fixed_color())
                    .unwrap_or(theme_color),
            ),
            BottomNavigationBarType::Shifting => (
                icon_theme_color(&effective_unselected_icon_theme)
                    .or(widget.unselected_item_color)
                    .or(bottom_theme.unselected_item_color)
                    .unwrap_or(theme_data.color_scheme.surface),
                icon_theme_color(&effective_selected_icon_theme)
                    .or(widget.effective_selected_item_color())
                    .or(bottom_theme.selected_item_color)
                    .unwrap_or(theme_color),
            ),
        };

        let tweens = app
            .get(self)
            .color_tweens
            .as_ref()
            .expect("made in init_state");
        let (color_tween, label_color_tween, icon_color_tween) = (
            tweens.color_tween,
            tweens.label_color_tween,
            tweens.icon_color_tween,
        );
        for (tween, (begin, end)) in [
            (color_tween, color_tween_ends),
            (label_color_tween, label_color_tween_ends),
            (icon_color_tween, icon_color_tween_ends),
        ] {
            tween.set_begin(app, Some(begin));
            tween.set_end(app, Some(end));
        }

        let show_unselected_labels_default = self.default_show_unselected(app);
        let mut tiles = Vec::new();
        let items_length = self.widget(app).items.len();
        for i in 0..items_length {
            let widget = self.widget(app);
            let mut states = WidgetStates::new();
            if i == widget.current_index {
                states.insert(WidgetState::Selected);
            }

            let effective_mouse_cursor: MouseCursorRef =
                <dyn WidgetStateProperty<MouseCursorRef>>::resolve_as(
                    &widget.mouse_cursor,
                    &states,
                )
                .or_else(|| {
                    bottom_theme
                        .mouse_cursor
                        .as_ref()
                        .and_then(|mouse_cursor| mouse_cursor.resolve(&states))
                })
                .unwrap_or_else(|| WidgetStateMouseCursor::clickable().resolve(&states));

            let state = app.get(self);
            let animation = state.animations[i].as_animation();
            let fade_out_animation = state.fade_out_animations[i].1;
            let flex = self.evaluate_flex(app, animation);
            let widget = self.widget(app);
            tiles.push(
                BottomNavigationTile {
                    r#type: effective_type,
                    item: widget.items[i].clone(),
                    animation,
                    fade_out_animation,
                    icon_size: widget.icon_size,
                    key: widget.items[i].key.clone(),
                    selected_icon_theme: if widget.use_legacy_color_scheme {
                        widget
                            .selected_icon_theme
                            .clone()
                            .or(bottom_theme.selected_icon_theme.clone())
                    } else {
                        Some(effective_selected_icon_theme.clone())
                    },
                    unselected_icon_theme: if widget.use_legacy_color_scheme {
                        widget
                            .unselected_icon_theme
                            .clone()
                            .or(bottom_theme.unselected_icon_theme.clone())
                    } else {
                        Some(effective_unselected_icon_theme.clone())
                    },
                    selected_label_style: effective_selected_label_style.clone(),
                    unselected_label_style: effective_unselected_label_style.clone(),
                    enable_feedback: widget
                        .enable_feedback
                        .or(bottom_theme.enable_feedback)
                        .unwrap_or(true),
                    on_tap: Some(Listener::new(move |app| {
                        if let Some(on_tap) = self.widget(app).on_tap.clone() {
                            on_tap(app, i);
                        }
                    })),
                    label_color_tween: Some(if widget.use_legacy_color_scheme {
                        color_tween
                    } else {
                        label_color_tween
                    }),
                    icon_color_tween: Some(if widget.use_legacy_color_scheme {
                        color_tween
                    } else {
                        icon_color_tween
                    }),
                    flex: Some(flex),
                    selected: i == widget.current_index,
                    show_selected_labels: widget
                        .show_selected_labels
                        .or(bottom_theme.show_selected_labels)
                        .unwrap_or(true),
                    show_unselected_labels: widget
                        .show_unselected_labels
                        .or(bottom_theme.show_unselected_labels)
                        .unwrap_or(show_unselected_labels_default),
                    index_label: Some(
                        localizations.tab_label(i as i32 + 1, widget.items.len() as i32),
                    ),
                    mouse_cursor: effective_mouse_cursor,
                    layout,
                }
                .into_widget(),
            );
        }
        tiles
    }
}

impl TickerProviderStateMixin for BottomNavigationBarState {
    fn ticker_provider_data(self: Handle<Self>, app: &App) -> &TickerProviderStateMixinData {
        &app.get(self).ticker_provider
    }

    fn ticker_provider_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut TickerProviderStateMixinData {
        &mut app.get_mut(self).ticker_provider
    }
}

/// Dart's `vsync: this`.
impl TickerProviderObject for BottomNavigationBarState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
        TickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl State for BottomNavigationBarState {
    type Widget = BottomNavigationBar;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let flex_tween = Tween::new(app, Some(1.0), Some(1.5));
        let color_tweens = ColorTweens {
            color_tween: ColorTween::new(app, None, None),
            label_color_tween: ColorTween::new(app, None, None),
            icon_color_tween: ColorTween::new(app, None, None),
        };
        let state = app.get_mut(self);
        state.flex_tween = Some(flex_tween);
        state.color_tweens = Some(color_tweens);
        self.reset_state(app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.dispose_animations(app);
        let state = app.get_mut(self);
        let flex_tween = state.flex_tween.take();
        let color_tweens = state.color_tweens.take();
        if let Some(flex_tween) = flex_tween {
            app.destroy(flex_tween);
        }
        if let Some(color_tweens) = color_tweens {
            app.destroy(color_tweens.color_tween);
            app.destroy(color_tweens.label_color_tween);
            app.destroy(color_tweens.icon_color_tween);
        }
        TickerProviderStateMixin::dispose(self, app);
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        TickerProviderStateMixin::activate(self, app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &BottomNavigationBar) {
        // No animated segue if the length of the items list changes.
        if self.widget(app).items.len() != old_widget.items.len() {
            self.reset_state(app);
            return;
        }

        let current_index = self.widget(app).current_index;
        if current_index != old_widget.current_index {
            match self.effective_type(app) {
                BottomNavigationBarType::Fixed => {}
                BottomNavigationBarType::Shifting => self.push_circle(app, current_index),
            }
            let state = app.get(self);
            let old_controller = state.controllers[old_widget.current_index];
            let controller = state.controllers[current_index];
            old_controller.reverse(app, None);
            controller.forward(app, None);
        } else {
            let item_background_color = self.widget(app).items[current_index]
                .background_color
                .as_ref()
                .map(|color| color.color());
            if app.get(self).background_color != item_background_color {
                app.get_mut(self).background_color = item_background_color;
            }
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(debug_check_has_directionality(
            app, context, None, None, None
        ));
        debug_assert!(debug_check_has_material_localizations(app, context));
        debug_assert!(debug_check_has_media_query(app, context));
        debug_assert!(debug_check_has_overlay(app, context));

        let bottom_theme = BottomNavigationBarTheme::of(app, context);
        let layout = self
            .widget(app)
            .landscape_layout
            .or(bottom_theme.landscape_layout)
            .unwrap_or(BottomNavigationBarLandscapeLayout::Spread);
        let additional_bottom_padding = MediaQuery::view_padding_of(app, context).bottom;

        let background_color = match self.effective_type(app) {
            BottomNavigationBarType::Fixed => self
                .widget(app)
                .background_color
                .or(bottom_theme.background_color),
            BottomNavigationBarType::Shifting => app.get(self).background_color,
        };

        let elevation = self
            .widget(app)
            .elevation
            .or(bottom_theme.elevation)
            .unwrap_or(8.0);
        let circles: Rc<[Rc<Circle>]> = app.get(self).circles.iter().cloned().collect();
        let text_direction = Directionality::of(app, context);
        let tiles = self.create_tiles(app, context, layout);
        let row = DefaultTextStyle::merge(
            Row::new()
                .main_axis_alignment(MainAxisAlignment::SpaceBetween)
                .children(tiles),
        )
        .overflow(TextOverflow::Ellipsis);
        let without_bottom_padding =
            MediaQuery::remove_padding(app, context, false, false, false, true, row).into_widget();
        Semantics::new()
            .explicit_child_nodes(true)
            .child(Bar {
                layout,
                elevation,
                color: background_color,
                child: ConstrainedBox::new(
                    BoxConstraints::new()
                        .min_height(K_BOTTOM_NAVIGATION_BAR_HEIGHT + additional_bottom_padding),
                )
                .child(
                    CustomPaint::new()
                        .painter(RadialPainter {
                            circles,
                            text_direction,
                        })
                        .child(
                            Material::new()
                                // Splashes.
                                .r#type(MaterialType::Transparency)
                                .child(
                                    Padding::new(EdgeInsetsGeometry::only(
                                        0.0,
                                        0.0,
                                        0.0,
                                        additional_bottom_padding,
                                    ))
                                    .child(without_bottom_padding),
                                ),
                        ),
                )
                .into_widget(),
            })
            .into_widget()
    }
}

// Optionally center a Material child for landscape layouts when layout is
// BottomNavigationBarLandscapeLayout.centered
#[derive(Debug)]
struct Bar {
    child: WidgetRef,
    layout: BottomNavigationBarLandscapeLayout,
    elevation: f64,
    color: Option<Color>,
}

impl StatelessWidget for Bar {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let mut aligned_child = self.child.clone();
        if MediaQuery::orientation_of(app, context) == Orientation::Landscape
            && self.layout == BottomNavigationBarLandscapeLayout::Centered
        {
            aligned_child = Align::new()
                .alignment(AlignmentGeometry::BOTTOM_CENTER)
                .height_factor(1.0)
                .child(
                    SizedBox::new()
                        .width(MediaQuery::height_of(app, context))
                        .child(self.child.clone()),
                )
                .into_widget();
        }
        let mut material = Material::new().elevation(self.elevation);
        if let Some(color) = self.color {
            material = material.color(color);
        }
        material.child(aligned_child).into_widget()
    }
}

// Describes an animating color splash circle.
struct Circle {
    state: Handle<BottomNavigationBarState>,
    index: usize,
    color: Color,
    controller: Handle<AnimationController>,
    animation: Handle<CurvedAnimation>,
}

impl Circle {
    fn new(
        app: &mut App,
        state: Handle<BottomNavigationBarState>,
        index: usize,
        color: Color,
        vsync: impl waft_scheduler::TickerProvider,
    ) -> Circle {
        let controller = AnimationController::create(
            app,
            None,
            Some(K_THEME_ANIMATION_DURATION),
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            vsync,
        );
        let animation = CurvedAnimation::create(
            app,
            controller.as_animation(),
            Curves::fast_out_slow_in(),
            None,
        );
        controller.forward(app, None);
        Circle {
            state,
            index,
            color,
            controller,
            animation,
        }
    }

    fn horizontal_leading_offset(&self, app: &App) -> f64 {
        let weight_sum = |animations: &[Handle<CurvedAnimation>]| -> f64 {
            // We're adding flex values instead of animation values to produce correct
            // ratios.
            animations
                .iter()
                .map(|animation| self.state.evaluate_flex(app, animation.as_animation()))
                .fold(0.0, |sum, value| sum + value)
        };

        let animations = &app.get(self.state).animations;
        let all_weights = weight_sum(animations);
        // These weights sum to the start edge of the indexed item.
        let leading_weights = weight_sum(&animations[0..self.index]);

        // Add half of its flex value in order to get to the center.
        (leading_weights
            + self
                .state
                .evaluate_flex(app, animations[self.index].as_animation())
                / 2.0)
            / all_weights
    }

    fn dispose(&self, app: &mut App) {
        self.controller.dispose(app);
        self.animation.dispose(app);
    }

    /// Frees what [`new`](Self::new) made, once [`dispose`](Self::dispose) has run.
    fn destroy(&self, app: &mut App) {
        app.destroy(self.animation);
        app.destroy(self.controller);
    }

    /// [`destroy`](Self::destroy) from inside the controller's own status listener: both
    /// stay readable until the next checkpoint.
    fn destroy_lingering(&self, app: &mut App) {
        destroy_lingering(app, self.animation);
        destroy_lingering(app, self.controller);
    }
}

// Paints the animating color splash circles.
struct RadialPainter {
    circles: Rc<[Rc<Circle>]>,
    text_direction: TextDirection,
}

impl RadialPainter {
    // Computes the maximum radius attainable such that at least one of the
    // bounding rectangle's corners touches the edge of the circle. Drawing a
    // circle larger than this radius is not needed, since there is no perceivable
    // difference within the cropped rectangle.
    fn max_radius(center: Offset, size: Size) -> f64 {
        let max_x = center.dx().max(size.width() - center.dx());
        let max_y = center.dy().max(size.height() - center.dy());
        (max_x * max_x + max_y * max_y).sqrt()
    }
}

impl CustomPainter for RadialPainter {
    fn should_repaint(&self, _app: &App, old_painter: &dyn CustomPainter) -> bool {
        let old_painter = old_painter
            .as_any()
            .downcast_ref::<RadialPainter>()
            .expect("compared with a painter of its own type");
        if self.text_direction != old_painter.text_direction {
            return true;
        }
        if Rc::ptr_eq(&self.circles, &old_painter.circles) {
            return false;
        }
        if self.circles.len() != old_painter.circles.len() {
            return true;
        }
        for i in 0..self.circles.len() {
            if !Rc::ptr_eq(&self.circles[i], &old_painter.circles[i]) {
                return true;
            }
        }
        false
    }

    fn paint(&self, app: &mut App, canvas: &mut Canvas, size: Size) {
        for circle in self.circles.iter() {
            let paint = Paint::from_color(circle.color.into());
            let rect = Rect::from_ltwh(0.0, 0.0, size.width(), size.height());
            canvas.clip_rect(rect, waft_embedder::ClipOp::Intersect, true);
            let left_fraction = match self.text_direction {
                TextDirection::Rtl => 1.0 - circle.horizontal_leading_offset(app),
                TextDirection::Ltr => circle.horizontal_leading_offset(app),
            };
            let center = Offset::new(left_fraction * size.width(), size.height() / 2.0);
            let radius = transform_tween(
                app,
                0.0,
                RadialPainter::max_radius(center, size),
                circle.animation.as_animation().value(app),
            );
            canvas.draw_circle(center, radius as f32, &paint);
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
pub(crate) mod tests {
    //! The cases of `test/bottom_navigation_bar_test.dart`.

    use std::cell::Cell;
    use std::time::Duration;

    use waft_embedder::FontWeight;
    use waft_painting::EdgeInsets;
    use waft_widgets::{AnyElement, Icon, IconData, MediaQueryData, Opacity};

    use super::*;
    use crate::color_scheme::ColorScheme;
    use crate::colors::Colors;
    use crate::icons::Icons;
    use crate::scaffold::Scaffold;
    use crate::test_support::{WidgetTester, with_main_thread_stack};
    use crate::theme_data::ThemeData;
    use crate::tooltip::tests::{m2, m3, pump_app, tester};

    /// Dart's `Matrix4.diagonal3(Vector3.all(scale))`.
    pub(crate) fn diagonal3(scale: f64) -> Matrix4 {
        diagonal3_all(scale)
    }

    pub(crate) fn item(icon: IconData, label: &str) -> BottomNavigationBarItem {
        BottomNavigationBarItem::new(Icon::new(Some(icon))).label(label)
    }

    pub(crate) fn ac_alarm() -> Vec<BottomNavigationBarItem> {
        vec![
            item(Icons::ac_unit(), "AC"),
            item(Icons::access_alarm(), "Alarm"),
        ]
    }

    /// `tester.pumpWidget(MaterialApp(theme:, home: Scaffold(bottomNavigationBar: bar)))`.
    pub(crate) fn pump_bar(
        tester: &mut WidgetTester,
        theme: impl FnOnce(&App) -> ThemeData,
        bar: impl FnOnce() -> BottomNavigationBar,
    ) {
        pump_app(tester, theme, |_app| {
            Scaffold::new().bottom_navigation_bar(bar()).into_widget()
        });
    }

    pub(crate) fn first_text(tester: &WidgetTester, text: &str) -> AnyElement {
        *tester
            .find_text(text)
            .first()
            .expect("a Text showing the text")
    }

    /// `tester.renderObject<RenderParagraph>(find.text(text)).text.style!`.
    pub(crate) fn paragraph_style(tester: &WidgetTester, text: &str) -> TextStyle {
        let render = tester.render_box_of(first_text(tester, text));
        let app = tester.cell.borrow();
        let paragraph = render
            .as_object()
            .downcast::<waft_rendering::RenderParagraph>(&app)
            .expect("a Text renders a paragraph");
        paragraph
            .text(&app)
            .style()
            .cloned()
            .expect("the paragraph's text has a style")
    }

    pub(crate) fn color_of(style: &TextStyle) -> Option<Color> {
        style.color.as_ref().map(|color| color.color())
    }

    /// Dart's `_iconStyle(tester, icon)`.
    pub(crate) fn icon_style(tester: &WidgetTester, icon: IconData) -> TextStyle {
        tester.glyph_style(tester.find_by_icon(&icon))
    }

    /// Dart's `_itemPadding(tester, icon)`.
    fn item_padding(tester: &WidgetTester, icon: IconData) -> EdgeInsets {
        let ink_response = tester.ancestor_of_type::<InkResponse>(tester.find_by_icon(&icon));
        let padding = tester.descendants_of_type::<Padding>(ink_response)[0];
        tester.widget(padding, |padding: &Padding| {
            padding.padding.resolve(Some(TextDirection::Ltr))
        })
    }

    /// Dart's `_getOpacity(tester, text)`.
    fn opacity_of(tester: &WidgetTester, text: &str) -> f64 {
        let fade = tester.ancestor_of_type::<FadeTransition>(first_text(tester, text));
        let opacity = tester.widget(fade, |fade: &FadeTransition| fade.opacity);
        opacity.value(&tester.cell.borrow())
    }

    /// Dart's `_getMaterial(tester)`.
    pub(crate) fn material_of<R>(tester: &WidgetTester, read: impl FnOnce(&Material) -> R) -> R {
        let bar = tester.find_by_type::<BottomNavigationBar>()[0];
        let material = tester.descendants_of_type::<Material>(bar)[0];
        tester.widget(material, read)
    }

    /// The first `Transform` above the `Text` showing `text`.
    pub(crate) fn label_transform(tester: &WidgetTester, text: &str) -> Matrix4 {
        let transform = tester.ancestor_of_type::<Transform>(first_text(tester, text));
        tester.widget(transform, |transform: &Transform| transform.transform)
    }

    pub(crate) fn descendants_of_bar<W: 'static>(tester: &WidgetTester) -> usize {
        let bar = tester.find_by_type::<BottomNavigationBar>()[0];
        tester.descendants_of_type::<W>(bar).len()
    }

    fn bar_box(tester: &WidgetTester) -> waft_rendering::AnyRenderBox {
        tester.render_box_of(tester.find_by_type::<BottomNavigationBar>()[0])
    }

    #[test]
    fn bottom_navigation_bar_callback_test() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let mutated_index = Rc::new(Cell::new(None));
            let seen = Rc::clone(&mutated_index);
            pump_bar(&mut tester, m3, move || {
                BottomNavigationBar::new(ac_alarm())
                    .on_tap(move |_app, index| seen.set(Some(index)))
            });

            let at = tester.global_rect(tester.render_box_of(first_text(&tester, "Alarm")));
            tester.tap(at.center(), 1);

            assert_eq!(mutated_index.get(), Some(1));
        });
    }

    #[test]
    fn material2_bottom_navigation_bar_content_test() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_bar(&mut tester, m2, || BottomNavigationBar::new(ac_alarm()));

            let size = bar_box(&tester).size(&tester.cell.borrow());
            assert_eq!(size.height(), K_BOTTOM_NAVIGATION_BAR_HEIGHT);
            assert_eq!(tester.find_text("AC").len(), 1);
            assert_eq!(tester.find_text("Alarm").len(), 1);
        });
    }

    #[test]
    fn material3_bottom_navigation_bar_content_test() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_bar(&mut tester, m3, || BottomNavigationBar::new(ac_alarm()));

            let size = bar_box(&tester).size(&tester.cell.borrow());
            // kBottomNavigationBarHeight is a minimum dimension.
            assert!(size.height() >= K_BOTTOM_NAVIGATION_BAR_HEIGHT);
            assert_eq!(tester.find_text("AC").len(), 1);
            assert_eq!(tester.find_text("Alarm").len(), 1);
        });
    }

    fn fixed_bottom_navigation_bar_defaults(
        theme: impl FnOnce(&App, ColorScheme, Color) -> ThemeData,
        expected_height: Option<f64>,
    ) {
        let primary_color = Color::new(0xFF000001);
        let unselected_widget_color = Color::new(0xFF000002);
        let mut tester = tester();
        pump_bar(
            &mut tester,
            |app| {
                theme(
                    app,
                    ColorScheme::light().copy_with().with_primary(primary_color),
                    unselected_widget_color,
                )
            },
            || BottomNavigationBar::new(ac_alarm()).r#type(BottomNavigationBarType::Fixed),
        );

        let selected_font_size = 14.0;
        let unselected_font_size = 12.0;
        let selected_font_style = paragraph_style(&tester, "AC");
        let unselected_font_style = paragraph_style(&tester, "Alarm");
        let selected_icon = icon_style(&tester, Icons::ac_unit());
        let unselected_icon = icon_style(&tester, Icons::access_alarm());
        assert_eq!(color_of(&selected_font_style), Some(primary_color));
        assert_eq!(selected_font_style.font_size, Some(selected_font_size));
        assert_eq!(selected_font_style.font_weight, Some(FontWeight::W400));
        assert_eq!(selected_font_style.height, expected_height);
        assert_eq!(
            color_of(&unselected_font_style),
            Some(unselected_widget_color)
        );
        assert_eq!(unselected_font_style.font_weight, Some(FontWeight::W400));
        assert_eq!(unselected_font_style.height, expected_height);
        // Unselected label has a font size of 14 but is scaled down to be font size 12.
        assert_eq!(
            label_transform(&tester, "Alarm"),
            diagonal3_all(unselected_font_size / selected_font_size)
        );
        assert_eq!(color_of(&selected_icon), Some(primary_color));
        assert_eq!(selected_icon.font_size, Some(24.0));
        assert_eq!(color_of(&unselected_icon), Some(unselected_widget_color));
        assert_eq!(unselected_icon.font_size, Some(24.0));
        // There should not be any [Opacity] or [FadeTransition] widgets
        // since showUnselectedLabels and showSelectedLabels are true.
        assert_eq!(descendants_of_bar::<Opacity>(&tester), 0);
        assert_eq!(descendants_of_bar::<FadeTransition>(&tester), 0);
        assert_eq!(material_of(&tester, |material| material.elevation), 8.0);
    }

    #[test]
    fn material2_fixed_bottom_navigation_bar_defaults() {
        with_main_thread_stack(|| {
            fixed_bottom_navigation_bar_defaults(
                |app, color_scheme, unselected_widget_color| {
                    ThemeData::light(app, Some(false))
                        .copy_with()
                        .color_scheme(color_scheme)
                        .unselected_widget_color(unselected_widget_color)
                        .build()
                },
                None,
            );
        });
    }

    #[test]
    fn material3_fixed_bottom_navigation_bar_defaults() {
        with_main_thread_stack(|| {
            fixed_bottom_navigation_bar_defaults(
                |app, color_scheme, unselected_widget_color| {
                    ThemeData::builder()
                        .color_scheme(color_scheme)
                        .unselected_widget_color(unselected_widget_color)
                        .build(app)
                },
                Some(1.43),
            );
        });
    }

    #[test]
    fn custom_selected_and_unselected_font_styles() {
        with_main_thread_stack(|| {
            let selected_text_style = TextStyle::new()
                .font_weight(FontWeight::W200)
                .font_size(18.0);
            let unselected_text_style = TextStyle::new()
                .font_weight(FontWeight::W600)
                .font_size(12.0);
            let mut tester = tester();
            let (selected, unselected) =
                (selected_text_style.clone(), unselected_text_style.clone());
            pump_bar(&mut tester, m3, move || {
                BottomNavigationBar::new(ac_alarm())
                    .r#type(BottomNavigationBarType::Fixed)
                    .selected_label_style(selected)
                    .unselected_label_style(unselected)
            });

            let selected_font_style = paragraph_style(&tester, "AC");
            let unselected_font_style = paragraph_style(&tester, "Alarm");
            assert_eq!(selected_font_style.font_size, selected_text_style.font_size);
            assert_eq!(
                selected_font_style.font_weight,
                selected_text_style.font_weight
            );
            assert_eq!(
                label_transform(&tester, "Alarm"),
                diagonal3_all(
                    unselected_text_style.font_size.unwrap()
                        / selected_text_style.font_size.unwrap()
                )
            );
            assert_eq!(
                unselected_font_style.font_weight,
                unselected_text_style.font_weight
            );
        });
    }

    #[test]
    fn font_size_on_text_styles_overrides_font_size_params() {
        with_main_thread_stack(|| {
            let selected_text_style = TextStyle::new().font_size(18.0);
            let unselected_text_style = TextStyle::new().font_size(12.0);
            let mut tester = tester();
            let (selected, unselected) =
                (selected_text_style.clone(), unselected_text_style.clone());
            pump_bar(&mut tester, m3, move || {
                BottomNavigationBar::new(ac_alarm())
                    .r#type(BottomNavigationBarType::Fixed)
                    .selected_label_style(selected)
                    .unselected_label_style(unselected)
                    .selected_font_size(17.0)
                    .unselected_font_size(11.0)
            });

            let selected_font_style = paragraph_style(&tester, "AC");
            assert_eq!(selected_font_style.font_size, selected_text_style.font_size);
            assert_eq!(
                label_transform(&tester, "Alarm"),
                diagonal3_all(
                    unselected_text_style.font_size.unwrap()
                        / selected_text_style.font_size.unwrap()
                )
            );
        });
    }

    #[test]
    fn custom_selected_and_unselected_icon_themes() {
        with_main_thread_stack(|| {
            let selected_icon_theme = IconThemeData::new()
                .size(36.0)
                .color(Color::new(0x00000001));
            let unselected_icon_theme = IconThemeData::new()
                .size(18.0)
                .color(Color::new(0x00000002));
            let mut tester = tester();
            let (selected, unselected) =
                (selected_icon_theme.clone(), unselected_icon_theme.clone());
            pump_bar(&mut tester, m3, move || {
                BottomNavigationBar::new(ac_alarm())
                    .r#type(BottomNavigationBarType::Fixed)
                    .selected_icon_theme(selected)
                    .unselected_icon_theme(unselected)
            });

            let selected_icon = icon_style(&tester, Icons::ac_unit());
            let unselected_icon = icon_style(&tester, Icons::access_alarm());
            assert_eq!(selected_icon.color, selected_icon_theme.color);
            assert_eq!(selected_icon.font_size, selected_icon_theme.size);
            assert_eq!(unselected_icon.color, unselected_icon_theme.color);
            assert_eq!(unselected_icon.font_size, unselected_icon_theme.size);
        });
    }

    #[test]
    fn color_on_icon_theme_overrides_selected_and_unselected_item_colors() {
        with_main_thread_stack(|| {
            let selected_icon_theme = IconThemeData::new()
                .size(36.0)
                .color(Color::new(0x00000001));
            let unselected_icon_theme = IconThemeData::new()
                .size(18.0)
                .color(Color::new(0x00000002));
            let selected_item_color = Color::new(0x00000003);
            let unselected_item_color = Color::new(0x00000004);
            let mut tester = tester();
            let (selected, unselected) =
                (selected_icon_theme.clone(), unselected_icon_theme.clone());
            pump_bar(&mut tester, m3, move || {
                BottomNavigationBar::new(ac_alarm())
                    .r#type(BottomNavigationBarType::Fixed)
                    .selected_icon_theme(selected)
                    .unselected_icon_theme(unselected)
                    .selected_item_color(selected_item_color)
                    .unselected_item_color(unselected_item_color)
            });

            let selected_font_style = paragraph_style(&tester, "AC");
            let unselected_font_style = paragraph_style(&tester, "Alarm");
            let selected_icon = icon_style(&tester, Icons::ac_unit());
            let unselected_icon = icon_style(&tester, Icons::access_alarm());
            assert_eq!(selected_icon.color, selected_icon_theme.color);
            assert_eq!(unselected_icon.color, unselected_icon_theme.color);
            assert_eq!(color_of(&selected_font_style), Some(selected_item_color));
            assert_eq!(
                color_of(&unselected_font_style),
                Some(unselected_item_color)
            );
        });
    }

    fn padded_bar(show_selected_labels: bool, show_unselected_labels: bool) -> WidgetTester {
        let mut tester = tester();
        pump_bar(&mut tester, m3, move || {
            BottomNavigationBar::new(ac_alarm())
                .r#type(BottomNavigationBarType::Fixed)
                .show_selected_labels(show_selected_labels)
                .show_unselected_labels(show_unselected_labels)
                .selected_font_size(16.0)
                .selected_icon_theme(IconThemeData::new().size(36.0))
                .unselected_icon_theme(IconThemeData::new().size(20.0))
        });
        tester
    }

    #[test]
    fn padding_is_calculated_properly_on_items_all_labels() {
        with_main_thread_stack(|| {
            let (selected_font_size, selected_icon_size, unselected_icon_size) = (16.0, 36.0, 20.0);
            let tester = padded_bar(true, true);

            let selected_item_padding = item_padding(&tester, Icons::ac_unit());
            assert_eq!(selected_item_padding.top, selected_font_size / 2.0);
            assert_eq!(selected_item_padding.bottom, selected_font_size / 2.0);
            let unselected_item_padding = item_padding(&tester, Icons::access_alarm());
            let expected_unselected_padding =
                (selected_icon_size - unselected_icon_size) / 2.0 + selected_font_size / 2.0;
            assert_eq!(unselected_item_padding.top, expected_unselected_padding);
            assert_eq!(unselected_item_padding.bottom, expected_unselected_padding);
        });
    }

    #[test]
    fn padding_is_calculated_properly_on_items_selected_labels_only() {
        with_main_thread_stack(|| {
            let (selected_font_size, selected_icon_size, unselected_icon_size) = (16.0, 36.0, 20.0);
            let tester = padded_bar(true, false);

            let selected_item_padding = item_padding(&tester, Icons::ac_unit());
            assert_eq!(selected_item_padding.top, selected_font_size / 2.0);
            assert_eq!(selected_item_padding.bottom, selected_font_size / 2.0);
            let unselected_item_padding = item_padding(&tester, Icons::access_alarm());
            assert_eq!(
                unselected_item_padding.top,
                (selected_icon_size - unselected_icon_size) / 2.0 + selected_font_size
            );
            assert_eq!(
                unselected_item_padding.bottom,
                (selected_icon_size - unselected_icon_size) / 2.0
            );
        });
    }

    #[test]
    fn padding_is_calculated_properly_on_items_no_labels() {
        with_main_thread_stack(|| {
            let (selected_font_size, selected_icon_size, unselected_icon_size) = (16.0, 36.0, 20.0);
            let tester = padded_bar(false, false);

            let selected_item_padding = item_padding(&tester, Icons::ac_unit());
            assert_eq!(selected_item_padding.top, selected_font_size);
            assert_eq!(selected_item_padding.bottom, 0.0);
            let unselected_item_padding = item_padding(&tester, Icons::access_alarm());
            assert_eq!(
                unselected_item_padding.top,
                (selected_icon_size - unselected_icon_size) / 2.0 + selected_font_size
            );
            assert_eq!(
                unselected_item_padding.bottom,
                (selected_icon_size - unselected_icon_size) / 2.0
            );
        });
    }

    #[test]
    fn material2_shifting_bottom_navigation_bar_defaults() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_bar(&mut tester, m2, || {
                BottomNavigationBar::new(ac_alarm()).r#type(BottomNavigationBarType::Shifting)
            });

            let selected_font_size = 14.0;
            assert_eq!(
                paragraph_style(&tester, "AC").font_size,
                Some(selected_font_size)
            );
            assert_eq!(
                color_of(&paragraph_style(&tester, "AC")),
                Some(Colors::WHITE)
            );
            assert_eq!(opacity_of(&tester, "Alarm"), 0.0);
            assert_eq!(material_of(&tester, |material| material.elevation), 8.0);
        });
    }

    #[test]
    fn material3_shifting_bottom_navigation_bar_defaults() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_bar(&mut tester, m3, || {
                BottomNavigationBar::new(ac_alarm()).r#type(BottomNavigationBarType::Shifting)
            });

            let selected_font_size = 14.0;
            assert_eq!(
                paragraph_style(&tester, "AC").font_size,
                Some(selected_font_size)
            );
            let surface = {
                let element = first_text(&tester, "AC");
                let mut app = tester.cell.borrow_mut();
                Theme::of(&mut app, element).color_scheme.surface
            };
            assert_eq!(color_of(&paragraph_style(&tester, "AC")), Some(surface));
            assert_eq!(opacity_of(&tester, "Alarm"), 0.0);
            assert_eq!(material_of(&tester, |material| material.elevation), 8.0);
        });
    }

    /// `ThemeData(primaryColor: 0xFF000000, unselectedWidgetColor: 0xFFD501FF)`.
    fn primary_and_unselected_theme(app: &App) -> ThemeData {
        ThemeData::builder()
            .primary_color(Color::new(0xFF000000))
            .unselected_widget_color(Color::new(0xFFD501FF))
            .build(app)
    }

    const SELECTED_COLOR: Color = Color::new(0xFF0004FF);
    const UNSELECTED_COLOR: Color = Color::new(0xFFE5FF00);
    const SELECTED_LABEL_COLOR: Color = Color::new(0xFFFF9900);
    const UNSELECTED_LABEL_COLOR: Color = Color::new(0xFF92F74E);
    const SELECTED_ICON_THEME_COLOR: Color = Color::new(0xFF1E7723);
    const UNSELECTED_ICON_THEME_COLOR: Color = Color::new(0xFF009688);

    #[test]
    fn fixed_bottom_navigation_bar_custom_font_size_color() {
        with_main_thread_stack(|| {
            let (selected_font_size, unselected_font_size) = (18.0, 14.0);
            let mut tester = tester();
            pump_bar(&mut tester, primary_and_unselected_theme, move || {
                BottomNavigationBar::new(ac_alarm())
                    .r#type(BottomNavigationBarType::Fixed)
                    .selected_font_size(selected_font_size)
                    .unselected_font_size(unselected_font_size)
                    .selected_item_color(SELECTED_COLOR)
                    .unselected_item_color(UNSELECTED_COLOR)
            });

            let selected_icon = icon_style(&tester, Icons::ac_unit());
            let unselected_icon = icon_style(&tester, Icons::access_alarm());

            assert_eq!(
                paragraph_style(&tester, "AC").font_size,
                Some(selected_font_size)
            );
            // Unselected label has a font size of 18 but is scaled down to be font size 14.
            assert_eq!(
                paragraph_style(&tester, "Alarm").font_size,
                Some(selected_font_size)
            );
            assert_eq!(
                label_transform(&tester, "Alarm"),
                diagonal3_all(unselected_font_size / selected_font_size)
            );
            assert_eq!(
                color_of(&paragraph_style(&tester, "AC")),
                Some(SELECTED_COLOR)
            );
            assert_eq!(
                color_of(&paragraph_style(&tester, "Alarm")),
                Some(UNSELECTED_COLOR)
            );
            assert_eq!(color_of(&selected_icon), Some(SELECTED_COLOR));
            assert_eq!(color_of(&unselected_icon), Some(UNSELECTED_COLOR));
            // There should not be any [Opacity] or [FadeTransition] widgets
            // since showUnselectedLabels and showSelectedLabels are true.
            assert_eq!(descendants_of_bar::<Opacity>(&tester), 0);
            assert_eq!(descendants_of_bar::<FadeTransition>(&tester), 0);
        });
    }

    #[test]
    fn shifting_bottom_navigation_bar_custom_font_size_color() {
        with_main_thread_stack(|| {
            let (selected_font_size, unselected_font_size) = (18.0, 14.0);
            let mut tester = tester();
            pump_bar(&mut tester, primary_and_unselected_theme, move || {
                BottomNavigationBar::new(ac_alarm())
                    .r#type(BottomNavigationBarType::Shifting)
                    .selected_font_size(selected_font_size)
                    .unselected_font_size(unselected_font_size)
                    .selected_item_color(SELECTED_COLOR)
                    .unselected_item_color(UNSELECTED_COLOR)
            });

            let selected_icon = icon_style(&tester, Icons::ac_unit());
            let unselected_icon = icon_style(&tester, Icons::access_alarm());

            assert_eq!(
                paragraph_style(&tester, "AC").font_size,
                Some(selected_font_size)
            );
            assert_eq!(
                color_of(&paragraph_style(&tester, "AC")),
                Some(SELECTED_COLOR)
            );
            assert_eq!(opacity_of(&tester, "Alarm"), 0.0);

            assert_eq!(color_of(&selected_icon), Some(SELECTED_COLOR));
            assert_eq!(color_of(&unselected_icon), Some(UNSELECTED_COLOR));
        });
    }

    fn label_style_color_overrides_item_color_only_for_the_label(r#type: BottomNavigationBarType) {
        let mut tester = tester();
        pump_bar(&mut tester, primary_and_unselected_theme, move || {
            BottomNavigationBar::new(ac_alarm())
                .r#type(r#type)
                .selected_label_style(TextStyle::new().color(SELECTED_LABEL_COLOR))
                .unselected_label_style(TextStyle::new().color(UNSELECTED_LABEL_COLOR))
                .selected_item_color(SELECTED_COLOR)
                .unselected_item_color(UNSELECTED_COLOR)
                .use_legacy_color_scheme(false)
        });

        let selected_icon = icon_style(&tester, Icons::ac_unit());
        let unselected_icon = icon_style(&tester, Icons::access_alarm());

        assert_eq!(color_of(&selected_icon), Some(SELECTED_COLOR));
        assert_eq!(color_of(&unselected_icon), Some(UNSELECTED_COLOR));
        assert_eq!(
            color_of(&paragraph_style(&tester, "AC")),
            Some(SELECTED_LABEL_COLOR)
        );
        assert_eq!(
            color_of(&paragraph_style(&tester, "Alarm")),
            Some(UNSELECTED_LABEL_COLOR)
        );
    }

    #[test]
    fn label_style_color_should_override_item_color_only_for_the_label_for_fixed() {
        with_main_thread_stack(|| {
            label_style_color_overrides_item_color_only_for_the_label(
                BottomNavigationBarType::Fixed,
            );
        });
    }

    #[test]
    fn label_style_color_should_override_item_color_only_for_the_label_for_shifting() {
        with_main_thread_stack(|| {
            label_style_color_overrides_item_color_only_for_the_label(
                BottomNavigationBarType::Shifting,
            );
        });
    }

    fn icon_theme_color_overrides_item_color(
        r#type: BottomNavigationBarType,
        label_styles: bool,
        item_colors: bool,
    ) {
        let mut tester = tester();
        pump_bar(&mut tester, primary_and_unselected_theme, move || {
            let mut bar = BottomNavigationBar::new(ac_alarm())
                .r#type(r#type)
                .selected_icon_theme(
                    IconThemeData::new()
                        .size(20.0)
                        .color(SELECTED_ICON_THEME_COLOR),
                )
                .unselected_icon_theme(
                    IconThemeData::new()
                        .size(18.0)
                        .color(UNSELECTED_ICON_THEME_COLOR),
                )
                .use_legacy_color_scheme(false);
            if label_styles {
                bar = bar
                    .selected_label_style(
                        TextStyle::new().font_size(18.0).color(SELECTED_LABEL_COLOR),
                    )
                    .unselected_label_style(
                        TextStyle::new()
                            .font_size(18.0)
                            .color(UNSELECTED_LABEL_COLOR),
                    );
            }
            if item_colors {
                bar = bar
                    .selected_item_color(SELECTED_COLOR)
                    .unselected_item_color(UNSELECTED_COLOR);
            }
            bar
        });

        let selected_icon = icon_style(&tester, Icons::ac_unit());
        let unselected_icon = icon_style(&tester, Icons::access_alarm());

        assert_eq!(color_of(&selected_icon), Some(SELECTED_ICON_THEME_COLOR));
        assert_eq!(
            color_of(&unselected_icon),
            Some(UNSELECTED_ICON_THEME_COLOR)
        );
    }

    #[test]
    fn icon_theme_color_should_override_item_color_for_fixed() {
        with_main_thread_stack(|| {
            icon_theme_color_overrides_item_color(BottomNavigationBarType::Fixed, true, true);
        });
    }

    #[test]
    fn icon_theme_color_should_override_item_color_for_shifted() {
        with_main_thread_stack(|| {
            icon_theme_color_overrides_item_color(BottomNavigationBarType::Shifting, true, false);
        });
    }

    #[test]
    fn icon_theme_color_should_override_item_color_color_for_fixed() {
        with_main_thread_stack(|| {
            icon_theme_color_overrides_item_color(BottomNavigationBarType::Fixed, false, false);
        });
    }

    #[test]
    fn icon_theme_color_should_override_item_color_for_shifted_with_item_colors() {
        with_main_thread_stack(|| {
            icon_theme_color_overrides_item_color(BottomNavigationBarType::Shifting, false, true);
        });
    }

    #[test]
    fn fixed_bottom_navigation_bar_can_hide_unselected_labels() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_bar(&mut tester, m3, || {
                BottomNavigationBar::new(ac_alarm())
                    .r#type(BottomNavigationBarType::Fixed)
                    .show_unselected_labels(false)
            });

            assert_eq!(opacity_of(&tester, "AC"), 1.0);
            assert_eq!(opacity_of(&tester, "Alarm"), 0.0);
        });
    }

    #[test]
    fn fixed_bottom_navigation_bar_can_update_background_color() {
        with_main_thread_stack(|| {
            let color = Colors::YELLOW.primary();
            let mut tester = tester();
            pump_bar(&mut tester, m3, move || {
                BottomNavigationBar::new(ac_alarm())
                    .r#type(BottomNavigationBarType::Fixed)
                    .background_color(color)
            });

            assert_eq!(material_of(&tester, |material| material.color), Some(color));
        });
    }

    #[test]
    fn shifting_bottom_navigation_bar_background_color_is_overridden_by_item_color() {
        with_main_thread_stack(|| {
            let item_color = Colors::YELLOW.primary();
            let background_color = Colors::BLUE.primary();
            let mut tester = tester();
            pump_bar(&mut tester, m3, move || {
                BottomNavigationBar::new(vec![
                    item(Icons::ac_unit(), "AC").background_color(item_color),
                    item(Icons::access_alarm(), "Alarm"),
                ])
                .r#type(BottomNavigationBarType::Shifting)
                .background_color(background_color)
            });

            assert_eq!(
                material_of(&tester, |material| material.color),
                Some(item_color)
            );
        });
    }

    #[test]
    #[should_panic(expected = "Either selectedItemColor or fixedColor can be specified")]
    fn specifying_both_selected_item_color_and_fixed_color_asserts() {
        BottomNavigationBar::new(ac_alarm())
            .selected_item_color(Colors::BLACK)
            .fixed_color(Colors::BLACK)
            .into_widget();
    }

    #[test]
    fn fixed_bottom_navigation_bar_uses_fixed_color_when_selected_item_color_not_provided() {
        with_main_thread_stack(|| {
            let fixed_color = Colors::BLACK;
            let mut tester = tester();
            pump_bar(&mut tester, m3, move || {
                BottomNavigationBar::new(ac_alarm())
                    .r#type(BottomNavigationBarType::Fixed)
                    .fixed_color(fixed_color)
            });

            assert_eq!(color_of(&paragraph_style(&tester, "AC")), Some(fixed_color));
        });
    }

    #[test]
    fn setting_selected_font_size_to_zero_hides_all_labels() {
        with_main_thread_stack(|| {
            let custom_elevation = 3.0;
            let mut tester = tester();
            pump_bar(&mut tester, m3, move || {
                BottomNavigationBar::new(ac_alarm())
                    .r#type(BottomNavigationBarType::Fixed)
                    .elevation(custom_elevation)
            });

            assert_eq!(
                material_of(&tester, |material| material.elevation),
                custom_elevation
            );
        });
    }

    /// `MaterialApp(theme:, home: MediaQuery(data: data, child: Scaffold(bottomNavigationBar:)))`.
    fn pump_bar_under_media_query(
        tester: &mut WidgetTester,
        theme: impl FnOnce(&App) -> ThemeData,
        data: MediaQueryData,
        bar: BottomNavigationBar,
    ) {
        pump_app(tester, theme, move |_app| {
            MediaQuery::new(data, Scaffold::new().bottom_navigation_bar(bar)).into_widget()
        });
    }

    fn bar_size(tester: &WidgetTester) -> Size {
        bar_box(tester).size(&tester.cell.borrow())
    }

    #[test]
    fn material2_bottom_navigation_bar_adds_bottom_padding_to_height() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_bar_under_media_query(
                &mut tester,
                m2,
                MediaQueryData::new().view_padding(EdgeInsets::only(0.0, 0.0, 0.0, 40.0)),
                BottomNavigationBar::new(ac_alarm()),
            );

            let expected_height = K_BOTTOM_NAVIGATION_BAR_HEIGHT + 40.0;
            assert_eq!(bar_size(&tester).height(), expected_height);
        });
    }

    #[test]
    fn material3_bottom_navigation_bar_adds_bottom_padding_to_height() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_bar_under_media_query(
                &mut tester,
                m3,
                MediaQueryData::new().view_padding(EdgeInsets::only(0.0, 0.0, 0.0, 40.0)),
                BottomNavigationBar::new(ac_alarm()),
            );

            let expected_min_height = K_BOTTOM_NAVIGATION_BAR_HEIGHT + 40.0;
            assert!(bar_size(&tester).height() >= expected_min_height);
        });
    }

    #[test]
    fn bottom_navigation_bar_adds_bottom_padding_to_height_with_a_custom_font_size() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_bar_under_media_query(
                &mut tester,
                m3,
                MediaQueryData::new().view_padding(EdgeInsets::only(0.0, 0.0, 0.0, 40.0)),
                BottomNavigationBar::new(ac_alarm()).selected_font_size(8.0),
            );

            let expected_height = K_BOTTOM_NAVIGATION_BAR_HEIGHT + 40.0;
            assert_eq!(bar_size(&tester).height(), expected_height);
        });
    }

    #[test]
    fn bottom_navigation_bar_height_will_not_change_when_toggle_keyboard() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            // Test the bar height is correct when not showing the keyboard.
            pump_bar_under_media_query(
                &mut tester,
                m3,
                MediaQueryData::new()
                    .view_padding(EdgeInsets::only(0.0, 0.0, 0.0, 40.0))
                    .padding(EdgeInsets::only(0.0, 0.0, 0.0, 40.0)),
                BottomNavigationBar::new(ac_alarm()).selected_font_size(8.0),
            );

            // Expect the height is the correct.
            let expected_height = K_BOTTOM_NAVIGATION_BAR_HEIGHT + 40.0;
            assert_eq!(bar_size(&tester).height(), expected_height);

            // Now we show the keyboard.
            pump_bar_under_media_query(
                &mut tester,
                m3,
                MediaQueryData::new()
                    .view_padding(EdgeInsets::only(0.0, 0.0, 0.0, 40.0))
                    .view_insets(EdgeInsets::only(0.0, 0.0, 0.0, 336.0)),
                BottomNavigationBar::new(ac_alarm()).selected_font_size(8.0),
            );

            // Expect the height is the same.
            assert_eq!(bar_size(&tester).height(), expected_height);
        });
    }

    /// Dart's `tester.renderObjectList(find.byType(InkResponse))`.
    fn actions(tester: &WidgetTester) -> Vec<waft_rendering::AnyRenderBox> {
        tester
            .find_by_type::<InkResponse>()
            .into_iter()
            .map(|element| tester.render_box_of(element))
            .collect()
    }

    fn origin_of(tester: &WidgetTester, render: waft_rendering::AnyRenderBox) -> Offset {
        render.local_to_global(&tester.cell.borrow(), Offset::ZERO, None)
    }

    pub(crate) fn text_center(tester: &WidgetTester, text: &str) -> Offset {
        tester
            .global_rect(tester.render_box_of(first_text(tester, text)))
            .center()
    }

    #[test]
    fn bottom_navigation_bar_action_size_test() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_bar(&mut tester, m3, || {
                BottomNavigationBar::new(ac_alarm()).r#type(BottomNavigationBarType::Shifting)
            });

            let found = actions(&tester);
            assert_eq!(found.len(), 2);
            assert_eq!(found[0].size(&tester.cell.borrow()).width(), 480.0);
            assert_eq!(found[1].size(&tester.cell.borrow()).width(), 320.0);

            pump_bar(&mut tester, m3, || {
                BottomNavigationBar::new(ac_alarm())
                    .current_index(1)
                    .r#type(BottomNavigationBarType::Shifting)
            });

            tester.pump(Duration::from_millis(200));

            let found = actions(&tester);
            assert_eq!(found.len(), 2);
            assert_eq!(found[0].size(&tester.cell.borrow()).width(), 320.0);
            assert_eq!(found[1].size(&tester.cell.borrow()).width(), 480.0);
        });
    }

    fn four_items() -> Vec<BottomNavigationBarItem> {
        vec![
            item(Icons::ac_unit(), "AC"),
            item(Icons::access_alarm(), "Alarm"),
            item(Icons::access_time(), "Time"),
            item(Icons::add(), "Add"),
        ]
    }

    #[test]
    fn bottom_navigation_bar_multiple_taps_test() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_bar(&mut tester, m3, || {
                BottomNavigationBar::new(four_items()).r#type(BottomNavigationBarType::Shifting)
            });

            // We want to make sure that the last label does not get displaced,
            // irrespective of how many taps happen on the first N - 1 labels and how
            // they grow.

            let original_origin = origin_of(&tester, actions(&tester)[3]);

            for (pointer, label) in [(1, "AC"), (2, "Alarm"), (3, "Time")] {
                let at = text_center(&tester, label);
                tester.tap(at, pointer);
                tester.pump(Duration::ZERO);
                tester.pump(Duration::from_millis(100));

                assert_eq!(origin_of(&tester, actions(&tester)[3]), original_origin);
            }
        });
    }

    fn inherits_shadowed_app_theme(r#type: BottomNavigationBarType) {
        let mut tester = tester();
        pump_app(
            &mut tester,
            |app| {
                ThemeData::builder()
                    .brightness(Brightness::Light)
                    .build(app)
            },
            move |app| {
                Theme::new(
                    ThemeData::builder().brightness(Brightness::Dark).build(app),
                    Scaffold::new().bottom_navigation_bar(
                        BottomNavigationBar::new(four_items()).r#type(r#type),
                    ),
                )
                .into_widget()
            },
        );

        let at = text_center(&tester, "Alarm");
        tester.tap(at, 1);
        tester.pump(Duration::from_secs(1));
        let element = first_text(&tester, "Alarm");
        let brightness = Theme::of(&mut tester.cell.borrow_mut(), element).brightness();
        assert_eq!(brightness, Brightness::Dark);
    }

    #[test]
    fn bottom_navigation_bar_inherits_shadowed_app_theme_for_shifting_navbar() {
        with_main_thread_stack(|| inherits_shadowed_app_theme(BottomNavigationBarType::Shifting));
    }

    #[test]
    fn bottom_navigation_bar_inherits_shadowed_app_theme_for_fixed_navbar() {
        with_main_thread_stack(|| inherits_shadowed_app_theme(BottomNavigationBarType::Fixed));
    }

    #[test]
    fn bottom_navigation_bar_icon_size_test() {
        with_main_thread_stack(|| {
            let builder_icon_size = Rc::new(Cell::new(None));
            let mut tester = tester();
            let seen = Rc::clone(&builder_icon_size);
            pump_bar(&mut tester, m3, move || {
                BottomNavigationBar::new(vec![
                    item(Icons::ac_unit(), "A"),
                    BottomNavigationBarItem::new(waft_widgets::Builder::new(
                        move |app, context| {
                            let size = IconTheme::of(app, context).size.expect("an icon size");
                            seen.set(Some(size));
                            SizedBox::new().width(size).height(size).into_widget()
                        },
                    ))
                    .label("B"),
                ])
                .icon_size(12.0)
            });

            let icon = tester.find_by_type::<Icon>()[0];
            let size = tester.render_box_of(icon).size(&tester.cell.borrow());
            assert_eq!(size.width(), 12.0);
            assert_eq!(size.height(), 12.0);
            assert_eq!(builder_icon_size.get(), Some(12.0));
        });
    }

    fn ab_items() -> Vec<BottomNavigationBarItem> {
        vec![
            item(Icons::ac_unit(), "A"),
            item(Icons::battery_alert(), "B"),
        ]
    }

    #[test]
    fn material2_bottom_navigation_bar_responds_to_text_scale_factor() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_bar(&mut tester, m2, || {
                BottomNavigationBar::new(ab_items()).r#type(BottomNavigationBarType::Fixed)
            });
            assert_eq!(bar_size(&tester).height(), K_BOTTOM_NAVIGATION_BAR_HEIGHT);

            pump_bar(&mut tester, m3, || {
                BottomNavigationBar::new(ab_items()).r#type(BottomNavigationBarType::Shifting)
            });
            assert_eq!(bar_size(&tester).height(), K_BOTTOM_NAVIGATION_BAR_HEIGHT);

            pump_app(&mut tester, m3, |_app| {
                MediaQuery::with_clamped_text_scaling(
                    None,
                    2.0,
                    2.0,
                    Scaffold::new().bottom_navigation_bar(BottomNavigationBar::new(ab_items())),
                )
            });
            assert_eq!(bar_size(&tester).height(), 56.0);
        });
    }

    #[test]
    fn material3_bottom_navigation_bar_responds_to_text_scale_factor() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_bar(&mut tester, m3, || {
                BottomNavigationBar::new(ab_items()).r#type(BottomNavigationBarType::Fixed)
            });
            // kBottomNavigationBarHeight is a minimum dimension.
            assert!(bar_size(&tester).height() >= K_BOTTOM_NAVIGATION_BAR_HEIGHT);

            pump_bar(&mut tester, m3, || {
                BottomNavigationBar::new(ab_items()).r#type(BottomNavigationBarType::Shifting)
            });
            // kBottomNavigationBarHeight is a minimum dimension.
            assert!(bar_size(&tester).height() >= K_BOTTOM_NAVIGATION_BAR_HEIGHT);

            pump_app(&mut tester, m3, |_app| {
                MediaQuery::with_clamped_text_scaling(
                    None,
                    2.0,
                    2.0,
                    Scaffold::new().bottom_navigation_bar(BottomNavigationBar::new(ab_items())),
                )
            });
            // kBottomNavigationBarHeight is a minimum dimension.
            assert!(bar_size(&tester).height() >= K_BOTTOM_NAVIGATION_BAR_HEIGHT);
        });
    }

    fn does_not_grow_with_text_scale_factor(theme: fn(&App) -> ThemeData, material3: bool) {
        let mut tester = tester();
        pump_bar(&mut tester, theme, || {
            BottomNavigationBar::new(ab_items()).r#type(BottomNavigationBarType::Fixed)
        });
        let default_height = bar_size(&tester).height();
        if material3 {
            // kBottomNavigationBarHeight is a minimum dimension.
            assert!(default_height >= K_BOTTOM_NAVIGATION_BAR_HEIGHT);
        } else {
            assert_eq!(default_height, K_BOTTOM_NAVIGATION_BAR_HEIGHT);
        }

        pump_bar(&mut tester, m3, || {
            BottomNavigationBar::new(ab_items()).r#type(BottomNavigationBarType::Shifting)
        });
        let shifting_height = bar_size(&tester).height();
        if material3 {
            // kBottomNavigationBarHeight is a minimum dimension.
            assert!(shifting_height >= K_BOTTOM_NAVIGATION_BAR_HEIGHT);
        } else {
            assert_eq!(shifting_height, K_BOTTOM_NAVIGATION_BAR_HEIGHT);
        }

        pump_bar_under_media_query(
            &mut tester,
            m3,
            MediaQueryData::new().text_scaler(waft_painting::TextScaler::linear(2.0)),
            BottomNavigationBar::new(ab_items()),
        );
        let height = bar_size(&tester).height();
        if material3 {
            assert_eq!(height, default_height);
            assert_eq!(height, shifting_height);
        } else {
            assert_eq!(height, K_BOTTOM_NAVIGATION_BAR_HEIGHT);
        }
    }

    #[test]
    fn material2_bottom_navigation_bar_does_not_grow_with_text_scale_factor_when_labels_are_provided()
     {
        with_main_thread_stack(|| does_not_grow_with_text_scale_factor(m2, false));
    }

    #[test]
    fn material3_bottom_navigation_bar_does_not_grow_with_text_scale_factor_when_labels_are_provided()
     {
        with_main_thread_stack(|| does_not_grow_with_text_scale_factor(m3, true));
    }

    /// The case's `buildApp`: a `MaterialApp` under a `MediaQuery` whose text scaler is
    /// `text_scaler`, with a bar whose first item has the tooltip "Foo".
    fn pump_tooltip_app(
        tester: &mut WidgetTester,
        theme: fn(&App) -> ThemeData,
        text_scaler: waft_painting::TextScaler,
    ) {
        let root = {
            let app = tester.cell.borrow();
            MediaQuery::new(
                MediaQueryData::new().text_scaler(text_scaler),
                crate::test_support::material_app(
                    theme(&app),
                    Scaffold::new()
                        .bottom_navigation_bar(BottomNavigationBar::new(vec![
                            item(Icons::ac_unit(), "Foo").tooltip("Foo"),
                            item(Icons::battery_alert(), "B"),
                        ]))
                        .into_widget(),
                ),
            )
            .into_widget()
        };
        tester.pump_widget(root);
    }

    fn last_text_size(tester: &WidgetTester, text: &str) -> Size {
        let found = tester.find_text(text);
        let last = *found.last().expect("a Text showing the text");
        tester.render_box_of(last).size(&tester.cell.borrow())
    }

    #[test]
    fn material2_bottom_navigation_bar_shows_tool_tips_with_text_scaling_on_long_press_when_labels_are_provided()
     {
        with_main_thread_stack(|| {
            let label = "Foo";
            let mut tester = tester();
            pump_tooltip_app(&mut tester, m2, waft_painting::TextScaler::linear(1.0));
            assert_eq!(tester.find_text(label).len(), 1);
            let at = text_center(&tester, label);
            tester.long_press(at, 1);
            assert_eq!(tester.find_text(label).len(), 2);
            assert_eq!(last_text_size(&tester, label), Size::new(42.0, 14.0));
            tester.pump_and_settle_by(Duration::from_secs(2));

            pump_tooltip_app(&mut tester, m2, waft_painting::TextScaler::linear(4.0));
            assert_eq!(tester.find_text(label).len(), 1);
            let at = text_center(&tester, label);
            tester.long_press(at, 2);
            assert_eq!(last_text_size(&tester, label), Size::new(168.0, 56.0));
        });
    }

    #[test]
    fn material3_bottom_navigation_bar_shows_tool_tips_with_text_scaling_on_long_press_when_labels_are_provided()
     {
        with_main_thread_stack(|| {
            let label = "Foo";
            let mut tester = tester();
            pump_tooltip_app(&mut tester, m3, waft_painting::TextScaler::NO_SCALING);
            assert_eq!(tester.find_text(label).len(), 1);
            let at = text_center(&tester, label);
            tester.long_press(at, 1);
            assert_eq!(tester.find_text(label).len(), 2);
            assert_eq!(last_text_size(&tester, label).height(), 20.0);
            tester.pump_and_settle_by(Duration::from_secs(2));

            pump_tooltip_app(&mut tester, m3, waft_painting::TextScaler::linear(4.0));
            assert_eq!(tester.find_text(label).len(), 1);
            let at = text_center(&tester, label);
            tester.long_press(at, 2);
            assert_eq!(last_text_size(&tester, label).height(), 80.0);
        });
    }

    /// Dart's `findByTooltip(message)`: the tooltips showing `message`.
    fn tooltips_with(tester: &WidgetTester, message: &str) -> usize {
        tester
            .find_by_type::<Tooltip>()
            .into_iter()
            .filter(|tooltip| {
                tester.widget(*tooltip, |tooltip: &Tooltip| {
                    tooltip.message.as_deref() == Some(message)
                })
            })
            .count()
    }

    #[test]
    fn different_behaviour_of_tool_tip_in_bottom_navigation_bar_item() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_bar(&mut tester, m3, || {
                BottomNavigationBar::new(vec![
                    item(Icons::ac_unit(), "A").tooltip("A tooltip"),
                    item(Icons::battery_alert(), "B"),
                    item(Icons::cake(), "C").tooltip(""),
                ])
            });

            assert_eq!(tester.find_text("A").len(), 1);
            let at = text_center(&tester, "A");
            tester.long_press(at, 1);
            assert_eq!(tooltips_with(&tester, "A tooltip"), 1);

            assert_eq!(tester.find_text("B").len(), 1);
            let at = text_center(&tester, "B");
            tester.long_press(at, 2);
            assert_eq!(tooltips_with(&tester, "B"), 0);

            assert_eq!(tester.find_text("C").len(), 1);
            let at = text_center(&tester, "C");
            tester.long_press(at, 3);
            assert_eq!(tooltips_with(&tester, "C"), 0);
        });
    }

    #[test]
    fn bottom_navigation_bar_limits_width_of_tiles_with_long_labels() {
        with_main_thread_stack(|| {
            let long_text = |letter: &str| format!("[{}]", vec![letter; 100].join(", "));
            let (long_text_a, long_text_b) = (long_text("A"), long_text("B"));
            let mut tester = tester();
            let (a, b) = (long_text_a.clone(), long_text_b.clone());
            pump_bar(&mut tester, m3, move || {
                BottomNavigationBar::new(vec![
                    item(Icons::ac_unit(), &a),
                    item(Icons::battery_alert(), &b),
                ])
            });

            assert!(bar_size(&tester).height() >= K_BOTTOM_NAVIGATION_BAR_HEIGHT);

            let item_box_a = tester.render_box_of(first_text(&tester, &long_text_a));
            assert_eq!(item_box_a.size(&tester.cell.borrow()).width(), 400.0);
            let item_box_b = tester.render_box_of(first_text(&tester, &long_text_b));
            assert_eq!(item_box_b.size(&tester.cell.borrow()).width(), 400.0);
        });
    }

    fn favorite_bar(
        selected_item: usize,
        filled: &KeyRef,
        stroked: &KeyRef,
    ) -> BottomNavigationBar {
        BottomNavigationBar::new(vec![
            BottomNavigationBarItem::new(
                Icon::new(Some(Icons::favorite_border())).key(stroked.clone()),
            )
            .active_icon(Icon::new(Some(Icons::favorite())).key(filled.clone()))
            .label("Favorite"),
            item(Icons::access_alarm(), "Alarm"),
        ])
        .current_index(selected_item)
    }

    #[test]
    fn bottom_navigation_bar_inactive_icon_shown() {
        with_main_thread_stack(|| {
            let filled: KeyRef = waft_foundation::ValueKey::new("filled".to_owned()).into();
            let stroked: KeyRef = waft_foundation::ValueKey::new("stroked".to_owned()).into();
            let mut tester = tester();
            let (f, s) = (filled.clone(), stroked.clone());
            pump_bar(&mut tester, m3, move || favorite_bar(0, &f, &s));

            assert_eq!(tester.find_onstage_by_key(&filled).len(), 1);
            assert_eq!(tester.find_onstage_by_key(&stroked).len(), 0);

            let (f, s) = (filled.clone(), stroked.clone());
            pump_bar(&mut tester, m3, move || favorite_bar(1, &f, &s));

            assert_eq!(tester.find_onstage_by_key(&filled).len(), 0);
            assert_eq!(tester.find_onstage_by_key(&stroked).len(), 1);
        });
    }

    #[test]
    fn bottom_navigation_bar_handles_items_length_changes() {
        with_main_thread_stack(|| {
            // Regression test for https://github.com/flutter/flutter/issues/10322
            let build_frame = |tester: &mut WidgetTester, item_count: usize| {
                pump_bar(tester, m3, move || {
                    BottomNavigationBar::new(
                        (0..item_count)
                            .map(|item_index| item(Icons::android(), &format!("item {item_index}")))
                            .collect(),
                    )
                    .r#type(BottomNavigationBarType::Fixed)
                });
            };
            let found = |tester: &WidgetTester, text: &str| tester.find_onstage_text(text).len();

            let mut tester = tester();
            build_frame(&mut tester, 3);
            assert_eq!(found(&tester, "item 0"), 1);
            assert_eq!(found(&tester, "item 1"), 1);
            assert_eq!(found(&tester, "item 2"), 1);
            assert_eq!(found(&tester, "item 3"), 0);

            build_frame(&mut tester, 4);
            assert_eq!(found(&tester, "item 0"), 1);
            assert_eq!(found(&tester, "item 1"), 1);
            assert_eq!(found(&tester, "item 2"), 1);
            assert_eq!(found(&tester, "item 3"), 1);

            build_frame(&mut tester, 2);
            assert_eq!(found(&tester, "item 0"), 1);
            assert_eq!(found(&tester, "item 1"), 1);
            assert_eq!(found(&tester, "item 2"), 0);
            assert_eq!(found(&tester, "item 3"), 0);
        });
    }

    #[test]
    fn bottom_navigation_bar_change_background_color_test() {
        with_main_thread_stack(|| {
            // Regression test for: https://github.com/flutter/flutter/issues/19653
            let red = Colors::RED.primary();
            let green = Colors::GREEN.primary();
            let background_color = Rc::new(Cell::new(red));
            let mut tester = tester();
            let shared = Rc::clone(&background_color);
            pump_app(&mut tester, m3, move |_app| {
                waft_widgets::StatefulBuilder::new(move |_app, _context, set_state| {
                    let background_color = shared.get();
                    let pressed = Rc::clone(&shared);
                    Scaffold::new()
                        .body(waft_widgets::Center::new().child(
                            crate::elevated_button::ElevatedButton::new(
                                Some(Listener::new(move |app| {
                                    let pressed = Rc::clone(&pressed);
                                    set_state(app, &mut |_app| pressed.set(green));
                                })),
                                Some(Text::new("green")),
                            ),
                        ))
                        .bottom_navigation_bar(
                            BottomNavigationBar::new(vec![
                                item(Icons::dashboard(), "Page 1")
                                    .background_color(background_color),
                                item(Icons::menu(), "Page 2").background_color(background_color),
                            ])
                            .r#type(BottomNavigationBarType::Shifting),
                        )
                        .into_widget()
                })
                .into_widget()
            });

            let background_material_color = |tester: &WidgetTester| {
                let bar = tester.find_by_type::<BottomNavigationBar>()[0];
                tester
                    .descendants_of_type::<Material>(bar)
                    .into_iter()
                    .find_map(|material| {
                        tester.widget(material, |material: &Material| {
                            (material.r#type == MaterialType::Canvas).then_some(material.color)
                        })
                    })
                    .expect("a canvas material")
            };

            assert_eq!(background_color.get(), red);
            assert_eq!(background_material_color(&tester), Some(red));
            let at = text_center(&tester, "green");
            tester.tap(at, 1);
            tester.pump_and_settle();
            assert_eq!(background_color.get(), green);
            assert_eq!(background_material_color(&tester), Some(green));
        });
    }

    #[test]
    #[should_panic(expected = "Every item must have a non-null label")]
    fn bottom_navigation_bar_item_label_should_not_be_nullable() {
        BottomNavigationBar::new(vec![
            item(Icons::ac_unit(), "AC"),
            BottomNavigationBarItem::new(Icon::new(Some(Icons::access_alarm()))),
        ])
        .r#type(BottomNavigationBarType::Shifting)
        .into_widget();
    }

    fn no_labels_render_no_text(r#type: BottomNavigationBarType) {
        let mut tester = tester();
        pump_bar(&mut tester, m3, move || {
            BottomNavigationBar::new(vec![
                item(Icons::dashboard(), "Red").background_color(Colors::RED.primary()),
                item(Icons::menu(), "Green").background_color(Colors::GREEN.primary()),
            ])
            .show_selected_labels(false)
            .show_unselected_labels(false)
            .r#type(r#type)
        });
        assert_eq!(tester.find_onstage_text("Red").len(), 1);
        assert_eq!(tester.find_onstage_text("Green").len(), 1);
        let visibilities = tester.find_onstage_by_type::<Visibility>();
        let visible = |element: AnyElement| {
            tester.widget(element, |visibility: &Visibility| visibility.visible)
        };
        assert!(!visible(*visibilities.first().expect("a Visibility")));
        assert!(!visible(*visibilities.last().expect("a Visibility")));
    }

    #[test]
    fn bottom_navigation_bar_show_no_labels_for_shifting_navbar_renders_no_text() {
        with_main_thread_stack(|| no_labels_render_no_text(BottomNavigationBarType::Shifting));
    }

    #[test]
    fn bottom_navigation_bar_show_no_labels_for_fixed_navbar_renders_no_text() {
        with_main_thread_stack(|| no_labels_render_no_text(BottomNavigationBarType::Fixed));
    }

    #[test]
    fn bottom_navigation_bar_changes_mouse_cursor_when_the_tile_is_hovered_over() {
        with_main_thread_stack(|| {
            let text: MouseCursorRef = waft_services::SystemMouseCursors::TEXT.into();
            let click: MouseCursorRef = waft_services::SystemMouseCursors::CLICK.into();
            let mut tester = tester();
            // Test BottomNavigationBar() constructor
            let pump = |tester: &mut WidgetTester, mouse_cursor: Option<MouseCursorRef>| {
                pump_app(tester, m3, move |_app| {
                    let mut bar = BottomNavigationBar::new(ac_alarm());
                    if let Some(mouse_cursor) = mouse_cursor {
                        bar = bar.mouse_cursor(mouse_cursor);
                    }
                    Scaffold::new()
                        .bottom_navigation_bar(
                            waft_widgets::MouseRegion::new()
                                .cursor(waft_services::SystemMouseCursors::FORBIDDEN.into())
                                .child(bar),
                        )
                        .into_widget()
                });
            };
            pump(&mut tester, Some(text.clone()));

            let mut gesture = WidgetTester::mouse(1, 0);
            let at = text_center(&tester, "AC");
            tester.add_pointer(&mut gesture, at);

            tester.pump_and_settle();

            assert!(*tester.active_cursor(1) == *text);

            // Test default cursor
            pump(&mut tester, None);

            assert!(*tester.active_cursor(1) == *click);
        });
    }

    /// A bar of two items whose icons are `SizedBox`es of `icon_size` keyed by the two keys,
    /// labelled "Title0" and "Title1".
    fn titled_bar(
        icon_size: Size,
        icon0: &KeyRef,
        icon1: &KeyRef,
        landscape_layout: Option<BottomNavigationBarLandscapeLayout>,
    ) -> BottomNavigationBar {
        let icon = |key: &KeyRef| {
            SizedBox::new()
                .key(key.clone())
                .width(icon_size.width())
                .height(icon_size.height())
        };
        let mut bar = BottomNavigationBar::new(vec![
            BottomNavigationBarItem::new(icon(icon0)).label("Title0"),
            BottomNavigationBarItem::new(icon(icon1)).label("Title1"),
        ]);
        if let Some(landscape_layout) = landscape_layout {
            bar = bar.landscape_layout(landscape_layout);
        }
        bar
    }

    fn unique_key() -> KeyRef {
        waft_foundation::UniqueKey::new().into()
    }

    fn rect_of_key(tester: &WidgetTester, key: &KeyRef) -> Rect {
        let element = tester.find_onstage_by_key(key)[0];
        tester.get_rect(tester.render_box_of(element))
    }

    /// Dart's `tester.getSize(find.text(text))`: the size of the text's box, before the
    /// transform that scales an unselected label.
    fn text_size(tester: &WidgetTester, text: &str) -> Size {
        tester
            .render_box_of(first_text(tester, text))
            .size(&tester.cell.borrow())
    }

    fn rect_of_text(tester: &WidgetTester, text: &str) -> Rect {
        tester.get_rect(tester.render_box_of(first_text(tester, text)))
    }

    fn pump_titled_bar(
        tester: &mut WidgetTester,
        theme: fn(&App) -> ThemeData,
        icon_size: Size,
        landscape_layout: Option<BottomNavigationBarLandscapeLayout>,
    ) -> (KeyRef, KeyRef) {
        let (icon0, icon1) = (unique_key(), unique_key());
        let (key0, key1) = (icon0.clone(), icon1.clone());
        pump_bar(tester, theme, move || {
            titled_bar(icon_size, &key0, &key1, landscape_layout)
        });
        assert_eq!(
            bar_size(tester),
            Size::new(800.0, K_BOTTOM_NAVIGATION_BAR_HEIGHT)
        );
        assert_eq!(
            tester.get_rect(bar_box(tester)),
            Rect::from_ltrb(0.0, 600.0 - K_BOTTOM_NAVIGATION_BAR_HEIGHT, 800.0, 600.0)
        );
        (icon0, icon1)
    }

    /// The M3 cases' `iconTop` and `labelBottom` for a tile `content_height` high without its
    /// label, and the label's height.
    fn m3_tile_extents(content_height: impl Fn(f64) -> f64) -> (f64, f64, f64) {
        let navigation_bar_top = 600.0 - K_BOTTOM_NAVIGATION_BAR_HEIGHT; // 544
        let selected_font_size: f64 = 14.0;
        let m3_line_height = 1.43;
        let label_height = (selected_font_size * m3_line_height).floor(); // 20
        let navigation_tile_vertical_padding = selected_font_size / 2.0; // 7.0
        let navigation_tile_height =
            content_height(label_height) + 2.0 * navigation_tile_vertical_padding;

        // Navigation tiles parent is a Row with crossAxisAlignment set to center.
        let navigation_tile_vertical_offset =
            (K_BOTTOM_NAVIGATION_BAR_HEIGHT - navigation_tile_height) / 2.0;

        let icon_top =
            navigation_bar_top + navigation_tile_vertical_offset + navigation_tile_vertical_padding;
        let label_bottom =
            600.0 - (navigation_tile_vertical_offset + navigation_tile_vertical_padding);
        (icon_top, label_bottom, label_height)
    }

    #[test]
    fn material2_bottom_navigation_bar_default_layout() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let (icon0, icon1) = pump_titled_bar(&mut tester, m2, Size::new(200.0, 10.0), None);

            // The height of the navigation bar is kBottomNavigationBarHeight = 56
            // The top of the navigation bar is 600 - 56 = 544
            // The top and bottom of the selected item is defined by its centered icon/label column:
            //   top = 544 + ((56 - (10 + 10)) / 2) = 562
            //   bottom = top + 10 + 10 = 582
            assert_eq!(rect_of_key(&tester, &icon0).top, 560.0);
            assert_eq!(rect_of_text(&tester, "Title0").bottom, 584.0);

            // The items are padded horizontally according to
            // MainAxisAlignment.spaceAround. Left/right padding is:
            // 800 - (200 * 2) / 4 = 100
            // The layout of the unselected item's label is slightly different; not
            // checking that here.
            assert_eq!(
                rect_of_text(&tester, "Title0"),
                Rect::from_ltrb(158.0, 570.0, 242.0, 584.0)
            );
            assert_eq!(
                rect_of_key(&tester, &icon0),
                Rect::from_ltrb(100.0, 560.0, 300.0, 570.0)
            );
            assert_eq!(
                rect_of_key(&tester, &icon1),
                Rect::from_ltrb(500.0, 560.0, 700.0, 570.0)
            );
        });
    }

    #[test]
    fn material3_bottom_navigation_bar_default_layout() {
        with_main_thread_stack(|| {
            let icon_height = 10.0;
            let mut tester = tester();
            let (icon0, icon1) =
                pump_titled_bar(&mut tester, m3, Size::new(200.0, icon_height), None);

            let (icon_top, label_bottom, label_height) =
                m3_tile_extents(|label_height| icon_height + label_height);

            assert_eq!(rect_of_key(&tester, &icon0).top, icon_top);
            assert_eq!(rect_of_text(&tester, "Title0").bottom, label_bottom);

            // The items are padded horizontally according to
            // MainAxisAlignment.spaceAround. Left/right padding is:
            // 800 - (200 * 2) / 4 = 100
            // The layout of the unselected item's label is slightly different; not
            // checking that here.
            let first_label_width = text_size(&tester, "Title0").width();
            let items_width = 800.0 / 2.0; // 2 items.
            let first_label_center = items_width / 2.0;
            assert_eq!(
                rect_of_text(&tester, "Title0"),
                Rect::from_ltrb(
                    first_label_center - first_label_width / 2.0,
                    label_bottom - label_height,
                    first_label_center + first_label_width / 2.0,
                    label_bottom,
                )
            );
            assert_eq!(
                rect_of_key(&tester, &icon0),
                Rect::from_ltrb(100.0, icon_top, 300.0, icon_top + icon_height)
            );
            assert_eq!(
                rect_of_key(&tester, &icon1),
                Rect::from_ltrb(500.0, icon_top, 700.0, icon_top + icon_height)
            );
        });
    }

    #[test]
    fn material2_bottom_navigation_bar_centered_landscape_layout() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let (icon0, icon1) = pump_titled_bar(
                &mut tester,
                m2,
                Size::new(200.0, 10.0),
                Some(BottomNavigationBarLandscapeLayout::Centered),
            );

            // The items are laid out as in the default case, within width = 600
            // (the "portrait" width) and the result is centered with the
            // landscape width = 800.
            // So item 0's left edges are:
            // ((800 - 600) / 2) + ((600 - 400) / 4) = 150.
            // Item 1's right edge is:
            // 800 - 150 = 650
            // The layout of the unselected item's label is slightly different; not
            // checking that here.
            assert_eq!(
                rect_of_text(&tester, "Title0"),
                Rect::from_ltrb(208.0, 570.0, 292.0, 584.0)
            );
            assert_eq!(
                rect_of_key(&tester, &icon0),
                Rect::from_ltrb(150.0, 560.0, 350.0, 570.0)
            );
            assert_eq!(
                rect_of_key(&tester, &icon1),
                Rect::from_ltrb(450.0, 560.0, 650.0, 570.0)
            );
        });
    }

    #[test]
    fn material3_bottom_navigation_bar_centered_landscape_layout() {
        with_main_thread_stack(|| {
            let (icon_width, icon_height) = (200.0, 10.0);
            let mut tester = tester();
            let (icon0, icon1) = pump_titled_bar(
                &mut tester,
                m3,
                Size::new(icon_width, icon_height),
                Some(BottomNavigationBarLandscapeLayout::Centered),
            );

            let (icon_top, label_bottom, label_height) =
                m3_tile_extents(|label_height| icon_height + label_height);

            // The items are laid out as in the default case, within width = 600
            // (the "portrait" width) and the result is centered with the
            // landscape width = 800.
            // So item 0's left edges are:
            // ((800 - 600) / 2) + ((600 - 400) / 4) = 150.
            // Item 1's right edge is:
            // 800 - 150 = 650
            // The layout of the unselected item's label is slightly different; not
            // checking that here.
            let first_label_width = text_size(&tester, "Title0").width();
            let item_width = icon_width; // 200
            let first_item_left = 150.0;
            let first_label_center = first_item_left + item_width / 2.0; // 250

            assert_eq!(
                rect_of_text(&tester, "Title0"),
                Rect::from_ltrb(
                    first_label_center - first_label_width / 2.0,
                    label_bottom - label_height,
                    first_label_center + first_label_width / 2.0,
                    label_bottom,
                )
            );
            assert_eq!(
                rect_of_key(&tester, &icon0),
                Rect::from_ltrb(150.0, icon_top, 350.0, icon_top + icon_height)
            );
            assert_eq!(
                rect_of_key(&tester, &icon1),
                Rect::from_ltrb(450.0, icon_top, 650.0, icon_top + icon_height)
            );
        });
    }

    #[test]
    fn material2_bottom_navigation_bar_linear_landscape_layout() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let (icon0, icon1) = pump_titled_bar(
                &mut tester,
                m2,
                Size::new(100.0, 20.0),
                Some(BottomNavigationBarLandscapeLayout::Linear),
            );

            // The items are laid out as in the default case except each
            // item's icon/label is arranged in a row, with 8 pixels in
            // between the icon and label.  The layout of the unselected
            // item's label is slightly different; not checking that here.
            assert_eq!(
                rect_of_text(&tester, "Title0"),
                Rect::from_ltrb(212.0, 565.0, 296.0, 579.0)
            );
            assert_eq!(
                rect_of_key(&tester, &icon0),
                Rect::from_ltrb(104.0, 562.0, 204.0, 582.0)
            );
            assert_eq!(
                rect_of_key(&tester, &icon1),
                Rect::from_ltrb(504.0, 562.0, 604.0, 582.0)
            );
        });
    }

    #[test]
    fn material3_bottom_navigation_bar_linear_landscape_layout() {
        with_main_thread_stack(|| {
            let (icon_width, icon_height) = (100.0, 20.0);
            let mut tester = tester();
            let (icon0, icon1) = pump_titled_bar(
                &mut tester,
                m3,
                Size::new(icon_width, icon_height),
                Some(BottomNavigationBarLandscapeLayout::Linear),
            );

            // Icon and label are in the same row.
            let (icon_top, label_bottom, label_height) =
                m3_tile_extents(|label_height: f64| icon_height.max(label_height));

            // The items are laid out as in the default case except each
            // item's icon/label is arranged in a row, with 8 pixels in
            // between the icon and label.  The layout of the unselected
            // item's label is slightly different; not checking that here.
            let item_full_with = 800.0 / 2.0; // Two items in the navigation bar.
            let separator_width = 8.0;
            let first_label_width = text_size(&tester, "Title0").width();
            let first_item_content_width = icon_width + separator_width + first_label_width;
            let first_item_left = item_full_with / 2.0 - first_item_content_width / 2.0;
            let second_label_width = text_size(&tester, "Title1").width();
            let second_item_content_width = icon_width + separator_width + second_label_width;
            let second_item_left =
                item_full_with + item_full_with / 2.0 - second_item_content_width / 2.0;

            assert_eq!(
                rect_of_text(&tester, "Title0"),
                Rect::from_ltrb(
                    first_item_left + icon_width + separator_width,
                    label_bottom - label_height,
                    first_item_left + icon_width + separator_width + first_label_width,
                    label_bottom,
                )
            );
            assert_eq!(
                rect_of_key(&tester, &icon0),
                Rect::from_ltrb(
                    first_item_left,
                    icon_top,
                    first_item_left + icon_width,
                    icon_top + icon_height
                )
            );
            assert_eq!(
                rect_of_key(&tester, &icon1),
                Rect::from_ltrb(
                    second_item_left,
                    icon_top,
                    second_item_left + icon_width,
                    icon_top + icon_height
                )
            );
        });
    }

    #[test]
    fn bottom_navigation_bar_keys_passed_through() {
        with_main_thread_stack(|| {
            let key1: KeyRef = waft_foundation::ValueKey::new("key1".to_owned()).into();
            let key2: KeyRef = waft_foundation::ValueKey::new("key2".to_owned()).into();
            let mut tester = tester();
            let (k1, k2) = (key1.clone(), key2.clone());
            pump_bar(&mut tester, m3, move || {
                BottomNavigationBar::new(vec![
                    item(Icons::favorite_border(), "Favorite").key(k1),
                    item(Icons::access_alarm(), "Alarm").key(k2),
                ])
            });

            assert_eq!(tester.find_onstage_by_key(&key1).len(), 1);
            assert_eq!(tester.find_onstage_by_key(&key2).len(), 1);
        });
    }

    #[test]
    fn bottom_navigation_bar_and_bottom_navigation_bar_item_render_at_zero_size() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(&mut tester, m3, |_app| {
                waft_widgets::Center::new()
                    .child(
                        SizedBox::shrink().child(Scaffold::new().bottom_navigation_bar(
                            BottomNavigationBar::new(vec![
                                item(Icons::favorite_border(), "X"),
                                item(Icons::access_alarm(), "Y"),
                            ]),
                        )),
                    )
                    .into_widget()
            });
            assert!(rect_of_text(&tester, "X").size().is_empty());
        });
    }

    /// The non-golden part of Dart's group 'BottomNavigationBar shifting backgroundColor with
    /// transition': the bar keeps the old item's colour while the new item's circle grows,
    /// then takes the new item's colour once the circle completes.
    fn shifting_background_color_with_transition(theme: fn(&App) -> ThemeData) {
        let red = Colors::RED.primary();
        let green = Colors::GREEN.primary();
        let mut tester = tester();
        pump_app(&mut tester, theme, move |_app| {
            let current_index = Rc::new(Cell::new(0));
            waft_widgets::StatefulBuilder::new(move |_app, _context, set_state| {
                let tapped = Rc::clone(&current_index);
                Scaffold::new()
                    .bottom_navigation_bar(
                        waft_widgets::RepaintBoundary::new().child(
                            BottomNavigationBar::new(vec![
                                item(Icons::dashboard(), "Red").background_color(red),
                                item(Icons::menu(), "Green").background_color(green),
                            ])
                            .r#type(BottomNavigationBarType::Shifting)
                            .current_index(current_index.get())
                            .on_tap(move |app, index| {
                                let tapped = Rc::clone(&tapped);
                                set_state(app, &mut |_app| tapped.set(index));
                            }),
                        ),
                    )
                    .into_widget()
            })
            .into_widget()
        });
        let at = text_center(&tester, "Green");
        tester.tap(at, 1);

        for _ in 0..3 {
            tester.pump(Duration::from_millis(30));
            assert_eq!(material_of(&tester, |material| material.color), Some(red));
        }
        tester.pump_and_settle();
        assert_eq!(material_of(&tester, |material| material.color), Some(green));
    }

    #[test]
    fn material2_bottom_navigation_bar_shifting_background_color_with_transition() {
        with_main_thread_stack(|| shifting_background_color_with_transition(m2));
    }

    #[test]
    fn material3_bottom_navigation_bar_shifting_background_color_with_transition() {
        with_main_thread_stack(|| shifting_background_color_with_transition(m3));
    }
}
