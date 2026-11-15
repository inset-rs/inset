//! Flutter counterpart: material_ui's `app_bar.dart`.
//!
//! The `Semantics` wrappers Dart puts around the title, the flexible space and the whole bar
//! wait with accessibility (see `PORTING.md`); `excludeHeaderSemantics` and
//! `useDefaultSemanticsOrder` are kept and read by nothing yet.

use std::any::Any;
use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use waft_animation::{Cubic, Curve, Curves, Interval};
use waft_embedder::{
    Clip, Color, Offset, Size, SystemUiOverlayStyle, TargetPlatform, TextBaseline, TextDirection,
    clamp_double,
};
use waft_foundation::{App, AsyncCallback, Handle};
use waft_painting::{
    Alignment, AlignmentDirectional, AlignmentGeometry, AnyColor, EdgeInsets, EdgeInsetsGeometry,
    ShapeBorder, TextOverflow, TextStyle,
};
use waft_rendering::{
    AnyRenderBox, AnyRenderObject, BaselineOffset, BoxConstraints, BoxHitTestResult, BoxParentData,
    FloatingHeaderSnapConfiguration, OrdinalSortKey, OverScrollHeaderStretchConfiguration,
    PaintingContext, PersistentHeaderShowOnScreenConfiguration, RenderAligningShiftedBox,
    RenderAligningShiftedBoxData, RenderBox, RenderBoxData, RenderHandle, RenderObject,
    RenderObjectData, RenderObjectWithChildData, RenderObjectWithChildMixin, RenderShiftedBox,
    SingleChildLayoutDelegate, TickerProviderRef,
};
use waft_scheduler::{Ticker, TickerCallback, TickerProviderObject};
use waft_widgets::{
    Align, AnimatedOpacity, AnnotatedRegion, AnyModalRoute, AnySize, AxisDirection, BuildContext,
    Center, ClipRect, Column, ConstrainedBox, CrossAxisAlignment, CustomSingleChildLayout,
    DefaultTextStyle, Directionality, Flexible, IconTheme, IconThemeData, IntoWidget, KeyRef,
    MainAxisAlignment, MainAxisSize, MediaQuery, NavigationToolbar, Opacity, Padding,
    PreferredSizeWidget, PreferredSizeWidgetRef, RenderObjectWidget, Row, SafeArea,
    ScrollNotification, ScrollNotificationCallback, ScrollNotificationObserver,
    ScrollNotificationObserverState, ScrollNotificationPredicate, ScrollUpdateNotification,
    Semantics, SingleChildRenderObjectWidget, SizeExtension, SliverPersistentHeader,
    SliverPersistentHeaderDelegate, Stack, StackFit, State, StateData, StatefulWidget,
    StatelessWidget, TickerProviderStateMixin, TickerProviderStateMixinData, WidgetRef,
    WidgetState, WidgetStateProperty, WidgetStates, default_scroll_notification_predicate,
    downcast_widget,
};

use crate::action_buttons::{BackButton, CloseButton, DrawerButton, EndDrawerButton};
use crate::app_bar_theme::{AppBarTheme, AppBarThemeData};
use crate::button_style::ButtonStyle;
use crate::constants::K_TOOLBAR_HEIGHT;
use crate::debug::debug_check_has_material_localizations;
use crate::flexible_space_bar::{FlexibleSpaceBar, FlexibleSpaceBarSettings};
use crate::generated::app_bar_defaults_m3::{
    LargeScrollUnderFlexibleConfig, MediumScrollUnderFlexibleConfig, app_bar_defaults_m3,
};
use crate::icon_button::IconButton;
use crate::icon_button_theme::{IconButtonTheme, IconButtonThemeData};
use crate::material::{Material, MaterialType};
use crate::scaffold::Scaffold;
use crate::theme::Theme;
use crate::theme_data::ThemeData;

/// Dart's `_FlexibleConfigBuilder`.
type FlexibleConfigBuilder = fn(&mut App, BuildContext) -> Box<dyn ScrollUnderFlexibleConfig>;

const K_LEADING_WIDTH: f64 = K_TOOLBAR_HEIGHT; // So the leading button is square.
const K_MAX_TITLE_TEXT_SCALE_FACTOR: f64 = 1.34; // TODO(perc): Add link to Material spec when available, https://github.com/flutter/flutter/issues/58769.

/// Dart's `_SliverAppVariant`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SliverAppVariant {
    Small,
    Medium,
    Large,
}

// Bottom justify the toolbarHeight child which may overflow the top.
/// Dart's `_ToolbarContainerLayout`.
#[derive(Debug)]
struct ToolbarContainerLayout {
    toolbar_height: f64,
}

impl SingleChildLayoutDelegate for ToolbarContainerLayout {
    fn get_constraints_for_child(&self, constraints: BoxConstraints) -> BoxConstraints {
        constraints.tighten(None, Some(self.toolbar_height))
    }

    fn get_size(&self, constraints: BoxConstraints) -> Size {
        Size::new(constraints.max_width, self.toolbar_height)
    }

    fn get_position_for_child(&self, size: Size, child_size: Size) -> Offset {
        Offset::new(0.0, size.height() - child_size.height())
    }

    fn should_relayout(&self, old_delegate: &dyn SingleChildLayoutDelegate) -> bool {
        old_delegate
            .as_any()
            .downcast_ref::<ToolbarContainerLayout>()
            .is_none_or(|old_delegate| self.toolbar_height != old_delegate.toolbar_height)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Dart's `_PreferredAppBarSize extends Size`: the heights an [`AppBar`]'s preferred size was
/// made from, carried by that [`AnySize`] for [`AppBar::preferred_height_for`] to read.
#[derive(Debug)]
struct PreferredAppBarSize {
    toolbar_height: Option<f64>,
    bottom_height: Option<f64>,
}

impl PreferredAppBarSize {
    fn new(toolbar_height: Option<f64>, bottom_height: Option<f64>) -> PreferredAppBarSize {
        PreferredAppBarSize {
            toolbar_height,
            bottom_height,
        }
    }

    /// This size as an [`AnySize`], whose plain value is Dart's `super.fromHeight(..)`.
    fn into_any(self) -> AnySize {
        let size = Size::from_height(
            self.toolbar_height.unwrap_or(K_TOOLBAR_HEIGHT) + self.bottom_height.unwrap_or(0.0),
        );
        AnySize::with_shared(size, Rc::new(self))
    }
}

impl SizeExtension for PreferredAppBarSize {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A Material Design app bar.
///
/// An app bar consists of a toolbar and potentially other widgets, such as a
/// `TabBar` and a [`FlexibleSpaceBar`]. App bars typically expose one or more
/// common [`actions`](Self::actions) with [`IconButton`]s which are optionally followed by a
/// `PopupMenuButton` for less common operations (sometimes called the "overflow
/// menu").
///
/// App bars are typically used in the `Scaffold::app_bar` property, which places
/// the app bar as a fixed-height widget at the top of the screen. For a scrollable
/// app bar, see [`SliverAppBar`], which embeds an [`AppBar`] in a sliver for use in
/// a `CustomScrollView`.
///
/// The AppBar displays the toolbar widgets, [`leading`](Self::leading), [`title`](Self::title),
/// and [`actions`](Self::actions), above the [`bottom`](Self::bottom) (if any). The `bottom`
/// is usually used for a `TabBar`. If a [`flexible_space`](Self::flexible_space) widget is
/// specified then it is stacked behind the toolbar and the bottom widget. When the writing
/// language is left-to-right (e.g. English), the leading widget is in the top left, the
/// actions are in the top right, the title is between them, the bottom is at the bottom, and
/// the flexible space is behind all of them.
///
/// If the [`leading`](Self::leading) widget is omitted, but the [`AppBar`] is in a [`Scaffold`]
/// with a `Drawer`, then a button will be inserted to open the drawer. Otherwise, if
/// the nearest `Navigator` has any previous routes, a [`BackButton`] is inserted
/// instead. This behavior can be turned off by setting the
/// [`automatically_imply_leading`](Self::automatically_imply_leading) to false. In that case a
/// null leading widget will result in the middle/title widget stretching to start.
///
/// If the [`actions`](Self::actions) widget list is omitted or empty, but the [`AppBar`] is in a
/// [`Scaffold`] with an end `Drawer`, then a button will be inserted to open the end drawer.
/// This behavior can be turned off by setting the
/// [`automatically_imply_actions`](Self::automatically_imply_actions) to false.
///
/// The [`AppBar`] insets its content based on the ambient [`MediaQuery`]'s padding,
/// to avoid system UI intrusions. It's taken care of by [`Scaffold`] when used in
/// the `Scaffold::app_bar` property. When animating an [`AppBar`], unexpected
/// [`MediaQuery`] changes (as is common in `Hero` animations) may cause the content
/// to suddenly jump. Wrap the [`AppBar`] in a [`MediaQuery`] widget, and adjust its
/// padding such that the animation is smooth.
///
/// ## Troubleshooting
///
/// ### Why don't my TextButton actions appear?
///
/// If the app bar's [`actions`](Self::actions) contains `TextButton`s, they will not
/// be visible if their foreground (text) color is the same as the
/// app bar's background color.
///
/// In Material v2 (i.e., when `ThemeData::use_material3` is false),
/// the default app bar [`background_color`](Self::background_color) is the overall theme's
/// `ColorScheme::primary` if the overall theme's brightness is
/// `Brightness::Light`. Unfortunately this is the same as the default
/// `ButtonStyle::foreground_color` for `TextButton` for light themes.
/// In this case a preferable text button foreground color is
/// `ColorScheme::on_primary`, a color that contrasts nicely with
/// `ColorScheme::primary`. To remedy the problem, override
/// `TextButton::style`.
///
/// See also:
///
///  * [`Scaffold`], which displays the [`AppBar`] in its `Scaffold::app_bar` slot.
///  * [`SliverAppBar`], which uses [`AppBar`] to provide a flexible app bar that
///    can be used in a `CustomScrollView`.
///  * `TabBar`, which is typically placed in the [`bottom`](Self::bottom) slot of the [`AppBar`]
///    if the screen has multiple pages arranged in tabs.
///  * [`IconButton`], which is used with [`actions`](Self::actions) to show buttons on the app
///    bar.
///  * `PopupMenuButton`, to show a popup menu on the app bar, via [`actions`](Self::actions).
///  * [`FlexibleSpaceBar`], which is used with [`flexible_space`](Self::flexible_space) when the
///    app bar can expand and collapse.
///  * <https://material.io/design/components/app-bars-top.html>
///  * <https://m3.material.io/components/top-app-bar>
///  * Cookbook: [Place a floating app bar above a list](https://docs.flutter.dev/cookbook/lists/floating-app-bar)
pub struct AppBar {
    pub key: Option<KeyRef>,

    /// A widget to display before the toolbar's [`title`](Self::title).
    ///
    /// Typically the [`leading`](Self::leading) widget is an `Icon` or an [`IconButton`].
    ///
    /// Becomes the leading component of the [`NavigationToolbar`] built
    /// by this widget. The [`leading`](Self::leading) widget's width and height are constrained
    /// to be no bigger than [`leading_width`](Self::leading_width) and
    /// [`toolbar_height`](Self::toolbar_height) respectively.
    ///
    /// If this is null and [`automatically_imply_leading`](Self::automatically_imply_leading) is
    /// set to true, the [`AppBar`] will imply an appropriate widget. For example, if the
    /// [`AppBar`] is in a [`Scaffold`] that also has a `Drawer`, the [`Scaffold`] will fill this
    /// widget with an [`IconButton`] that opens the drawer (using `Icons::menu`). If
    /// there's no `Drawer` and the parent `Navigator` can go back, the [`AppBar`]
    /// will use a [`BackButton`] that calls `Navigator::maybe_pop`.
    ///
    /// See also:
    ///
    ///  * `Scaffold::app_bar`, in which an [`AppBar`] is usually placed.
    ///  * `Scaffold::drawer`, in which the `Drawer` is usually placed.
    pub leading: Option<WidgetRef>,

    /// Controls whether we should try to imply the leading widget if null.
    ///
    /// If true and [`AppBar::leading`] is null, automatically try to deduce what the leading
    /// widget should be. If false and [`AppBar::leading`] is null, leading space is given to
    /// [`AppBar::title`]. If leading widget is not null, this parameter has no effect.
    pub automatically_imply_leading: bool,

    /// The primary widget displayed in the app bar.
    ///
    /// Becomes the middle component of the [`NavigationToolbar`] built by this widget.
    ///
    /// Typically a `Text` widget that contains a description of the current
    /// contents of the app.
    ///
    /// The [`title`](Self::title)'s width is constrained to fit within the remaining space
    /// between the toolbar's [`leading`](Self::leading) and [`actions`](Self::actions) widgets.
    /// Its height is _not_ constrained. The [`title`](Self::title) is vertically centered and
    /// clipped to fit within the toolbar, whose height is [`toolbar_height`](Self::toolbar_height).
    /// Typically this isn't noticeable because a simple `Text` [`title`](Self::title) will fit
    /// within the toolbar by default. On the other hand, it is noticeable when a
    /// widget with an intrinsic height that is greater than [`toolbar_height`](Self::toolbar_height)
    /// is used as the [`title`](Self::title). For example, when the height of an Image used
    /// as the [`title`](Self::title) exceeds [`toolbar_height`](Self::toolbar_height), it will be
    /// centered and clipped (top and bottom), which may be undesirable. In cases like this
    /// the height of the [`title`](Self::title) widget can be constrained.
    pub title: Option<WidgetRef>,

    /// A list of Widgets to display in a row after the [`title`](Self::title) widget.
    ///
    /// Typically these widgets are [`IconButton`]s representing common operations.
    /// For less common operations, consider using a `PopupMenuButton` as the
    /// last action.
    ///
    /// The [`actions`](Self::actions) become the trailing component of the [`NavigationToolbar`]
    /// built by this widget. The height of each action is constrained to be no bigger
    /// than the [`toolbar_height`](Self::toolbar_height).
    ///
    /// To avoid having the last action covered by the debug banner, you may want
    /// to set the `MaterialApp::debug_show_checked_mode_banner` to false.
    ///
    /// If this is null or empty and [`automatically_imply_actions`](Self::automatically_imply_actions)
    /// is set to true, the [`AppBar`] will imply an appropriate widget. For example, if the
    /// [`AppBar`] is in a [`Scaffold`] that also has an end `Drawer`, the [`Scaffold`] will fill
    /// this widget with an [`IconButton`] that opens the end drawer (using `Icons::menu`).
    pub actions: Option<Vec<WidgetRef>>,

    /// Controls whether we should try to imply the actions widget if null.
    ///
    /// If true and [`AppBar::actions`] is null or empty, automatically try to deduce what the
    /// actions widget should be. If false and [`AppBar::actions`] is null or empty, the actions
    /// widget list is kept empty. If [`AppBar::actions`] is not null, this parameter has no
    /// effect.
    pub automatically_imply_actions: bool,

    /// This widget is stacked behind the toolbar and the tab bar. Its height will
    /// be the same as the app bar's overall height.
    ///
    /// A flexible space isn't actually flexible unless the [`AppBar`]'s container
    /// changes the [`AppBar`]'s size. A [`SliverAppBar`] in a `CustomScrollView`
    /// changes the [`AppBar`]'s height when scrolled.
    ///
    /// Typically a [`FlexibleSpaceBar`]. See [`FlexibleSpaceBar`] for details.
    pub flexible_space: Option<WidgetRef>,

    /// This widget appears across the bottom of the app bar.
    ///
    /// Typically a `TabBar`. Only widgets that implement [`PreferredSizeWidget`] can
    /// be used at the bottom of an app bar.
    ///
    /// See also:
    ///
    ///  * `PreferredSize`, which can be used to give an arbitrary widget a preferred size.
    pub bottom: Option<PreferredSizeWidgetRef>,

    /// The z-coordinate at which to place this app bar relative to its parent.
    ///
    /// This property controls the size of the shadow below the app bar if
    /// [`shadow_color`](Self::shadow_color) is not null.
    ///
    /// If [`surface_tint_color`](Self::surface_tint_color) is not null then it will apply a
    /// surface tint overlay to the background color (see `Material::surface_tint_color` for
    /// more detail).
    ///
    /// The value must be non-negative.
    ///
    /// If this property is null, then the ambient [`AppBarThemeData::elevation`]
    /// is used. If that is also null, the default value is 4.
    ///
    /// See also:
    ///
    ///  * [`scrolled_under_elevation`](Self::scrolled_under_elevation), which will be used when
    ///    the app bar has something scrolled underneath it.
    ///  * [`shadow_color`](Self::shadow_color), which is the color of the shadow below the app
    ///    bar.
    ///  * [`surface_tint_color`](Self::surface_tint_color), which determines the elevation
    ///    overlay that will be applied to the background of the app bar.
    ///  * [`shape`](Self::shape), which defines the shape of the app bar's [`Material`] and its
    ///    shadow.
    pub elevation: Option<f64>,

    /// The elevation that will be used if this app bar has something
    /// scrolled underneath it.
    ///
    /// If this property is null, then the ambient
    /// [`AppBarThemeData::scrolled_under_elevation`] is used. If that is also null then
    /// [`elevation`](Self::elevation) is used.
    ///
    /// The value must be non-negative.
    pub scrolled_under_elevation: Option<f64>,

    /// A check that specifies which child's [`ScrollNotification`]s should be
    /// listened to.
    ///
    /// By default, checks whether `notification.depth == 0`. Set it to something
    /// else for more complicated layouts.
    pub notification_predicate: ScrollNotificationPredicate,

    /// The color of the shadow below the app bar.
    ///
    /// If this property is null, then the ambient [`AppBarThemeData::shadow_color`]
    /// is used. If that is also null, the default value is fully opaque black.
    pub shadow_color: Option<Color>,

    /// The color of the surface tint overlay applied to the app bar's
    /// background color to indicate elevation.
    ///
    /// If null no overlay will be applied.
    pub surface_tint_color: Option<Color>,

    /// The shape of the app bar's [`Material`] as well as its shadow.
    ///
    /// If this property is null, then the ambient [`AppBarThemeData::shape`]
    /// is used. Both properties default to null.
    /// If both properties are null then the shape of the app bar's [`Material`]
    /// is just a simple rectangle.
    ///
    /// A shadow is only displayed if the [`elevation`](Self::elevation) is greater than
    /// zero.
    pub shape: Option<Box<dyn ShapeBorder>>,

    /// The fill color to use for an app bar's [`Material`].
    ///
    /// If null, then the [`AppBarTheme`] background color is used. If that value is also
    /// null:
    /// In Material v2 (i.e., when `ThemeData::use_material3` is false),
    /// then [`AppBar`] uses the overall theme's `ColorScheme::primary` if the
    /// overall theme's brightness is `Brightness::Light`, and `ColorScheme::surface`
    /// if the overall theme's brightness is `Brightness::Dark`.
    /// In Material v3 (i.e., when `ThemeData::use_material3` is true),
    /// then [`AppBar`] uses the overall theme's `ColorScheme::surface`
    ///
    /// If this color is a `WidgetStateColor` it will be resolved against
    /// [`WidgetState::ScrolledUnder`] when the content of the app's
    /// primary scrollable overlaps the app bar.
    pub background_color: Option<AnyColor>,

    /// The default color for `Text` and `Icon`s within the app bar.
    ///
    /// If null, then the [`AppBarTheme`] foreground color is used. If that
    /// value is also null:
    /// In Material v2 (i.e., when `ThemeData::use_material3` is false),
    /// then [`AppBar`] uses the overall theme's `ColorScheme::on_primary` if the
    /// overall theme's brightness is `Brightness::Light`, and `ColorScheme::on_surface`
    /// if the overall theme's brightness is `Brightness::Dark`.
    /// In Material v3 (i.e., when `ThemeData::use_material3` is true),
    /// then [`AppBar`] uses the overall theme's `ColorScheme::on_surface`.
    ///
    /// This color is used to configure [`DefaultTextStyle`] that contains
    /// the toolbar's children, and the default [`IconTheme`] widgets that
    /// are created if [`icon_theme`](Self::icon_theme) and
    /// [`actions_icon_theme`](Self::actions_icon_theme) are null.
    pub foreground_color: Option<Color>,

    /// The color, opacity, and size to use for toolbar icons.
    ///
    /// If this property is null, then a copy of `ThemeData::icon_theme`
    /// is used, with the `IconThemeData::color` set to the
    /// app bar's [`foreground_color`](Self::foreground_color).
    ///
    /// See also:
    ///
    ///  * [`actions_icon_theme`](Self::actions_icon_theme), which defines the appearance of
    ///    icons in the [`actions`](Self::actions) list.
    pub icon_theme: Option<IconThemeData>,

    /// The color, opacity, and size to use for the icons that appear in the app
    /// bar's [`actions`](Self::actions).
    ///
    /// This property should only be used when the [`actions`](Self::actions) should be
    /// themed differently than the icon that appears in the app bar's [`leading`](Self::leading)
    /// widget.
    ///
    /// If this property is null, then the ambient [`AppBarThemeData::actions_icon_theme`]
    /// is used. If that is also null, then the value of [`icon_theme`](Self::icon_theme) is used.
    ///
    /// See also:
    ///
    ///  * [`icon_theme`](Self::icon_theme), which defines the appearance of all of the toolbar
    ///    icons.
    pub actions_icon_theme: Option<IconThemeData>,

    /// Whether this app bar is being displayed at the top of the screen.
    ///
    /// If true, the app bar's toolbar elements and [`bottom`](Self::bottom) widget will be
    /// padded on top by the height of the system status bar. The layout
    /// of the [`flexible_space`](Self::flexible_space) is not affected by the
    /// [`primary`](Self::primary) property.
    pub primary: bool,

    /// Whether the title should be centered.
    ///
    /// If this property is null, then [`AppBarThemeData::center_title`] of
    /// `ThemeData::app_bar_theme` is used. If that is also null, then value is
    /// adapted to the current [`TargetPlatform`].
    pub center_title: Option<bool>,

    /// Whether the title should be wrapped with header `Semantics`.
    ///
    /// If false, the title will be used as `SemanticsProperties::names_route`
    /// for Android, Fuchsia, Linux, and Windows platform. This means the title is
    /// announced by screen reader when transition to this route.
    ///
    /// The accessibility behavior is platform adaptive, based on the device's
    /// actual platform rather than the theme's platform setting. This ensures that
    /// assistive technologies like VoiceOver on iOS and macOS receive the correct
    /// `names_route` semantic information, even when the app's theme is configured
    /// to mimic a different platform's appearance.
    ///
    /// Defaults to false.
    pub exclude_header_semantics: bool,

    /// The spacing around [`title`](Self::title) content on the horizontal axis. This spacing
    /// is applied even if there is no [`leading`](Self::leading) content or
    /// [`actions`](Self::actions). If you want [`title`](Self::title) to take all the space
    /// available, set this value to 0.0.
    ///
    /// If this property is null, then [`AppBarThemeData::title_spacing`] of
    /// `ThemeData::app_bar_theme` is used. If that is also null, then the
    /// default value is [`NavigationToolbar::K_MIDDLE_SPACING`].
    pub title_spacing: Option<f64>,

    /// How opaque the toolbar part of the app bar is.
    ///
    /// A value of 1.0 is fully opaque, and a value of 0.0 is fully transparent.
    ///
    /// Typically, this value is not changed from its default value (1.0). It is
    /// used by [`SliverAppBar`] to animate the opacity of the toolbar when the app
    /// bar is scrolled.
    pub toolbar_opacity: f64,

    /// How opaque the bottom part of the app bar is.
    ///
    /// A value of 1.0 is fully opaque, and a value of 0.0 is fully transparent.
    ///
    /// Typically, this value is not changed from its default value (1.0). It is
    /// used by [`SliverAppBar`] to animate the opacity of the toolbar when the app
    /// bar is scrolled.
    pub bottom_opacity: f64,

    /// Defines the height of the toolbar component of an [`AppBar`].
    ///
    /// By default, the value of [`toolbar_height`](Self::toolbar_height) is [`K_TOOLBAR_HEIGHT`].
    pub toolbar_height: Option<f64>,

    /// Defines the width of [`AppBar::leading`] widget.
    ///
    /// By default, the value of [`AppBar::leading_width`] is 56.0.
    pub leading_width: Option<f64>,

    /// The default text style for the AppBar's [`leading`](Self::leading), and
    /// [`actions`](Self::actions) widgets, but not its [`title`](Self::title).
    ///
    /// If this property is null, then [`AppBarThemeData::toolbar_text_style`] of
    /// `ThemeData::app_bar_theme` is used. If that is also null, the default
    /// value is a copy of the overall theme's `TextTheme::body_medium`
    /// [`TextStyle`], with color set to the app bar's [`foreground_color`](Self::foreground_color).
    ///
    /// See also:
    ///
    ///  * [`title_text_style`](Self::title_text_style), which overrides the default text style
    ///    for the [`title`](Self::title).
    ///  * [`DefaultTextStyle`], which overrides the default text style for all of the
    ///    widgets in a subtree.
    pub toolbar_text_style: Option<TextStyle>,

    /// The default text style for the AppBar's [`title`](Self::title) widget.
    ///
    /// If this property is null, then [`AppBarThemeData::title_text_style`] of
    /// `ThemeData::app_bar_theme` is used. If that is also null, the default
    /// value is a copy of the overall theme's `TextTheme::title_large`
    /// [`TextStyle`], with color set to the app bar's [`foreground_color`](Self::foreground_color).
    ///
    /// See also:
    ///
    ///  * [`toolbar_text_style`](Self::toolbar_text_style), which is the default text style for
    ///    the AppBar's [`title`](Self::title), [`leading`](Self::leading), and
    ///    [`actions`](Self::actions) widgets, also known as the AppBar's "toolbar".
    ///  * [`DefaultTextStyle`], which overrides the default text style for all of the
    ///    widgets in a subtree.
    pub title_text_style: Option<TextStyle>,

    /// Specifies the style to use for the system overlays (e.g. the status bar on
    /// Android or iOS, the system navigation bar on Android).
    ///
    /// If this property is null, then [`AppBarThemeData::system_overlay_style`] of
    /// `ThemeData::app_bar_theme` is used. If that is also null, an appropriate
    /// [`SystemUiOverlayStyle`] is calculated based on the
    /// [`background_color`](Self::background_color).
    ///
    /// The AppBar's descendants are built within a
    /// `AnnotatedRegion<SystemUiOverlayStyle>` widget, which causes
    /// `SystemChrome::set_system_ui_overlay_style` to be called
    /// automatically. Apps should not enclose an AppBar with their
    /// own [`AnnotatedRegion`].
    ///
    /// See also:
    ///
    ///  * [`AnnotatedRegion`], for placing [`SystemUiOverlayStyle`] in the layer tree.
    ///  * `SystemChrome::set_system_ui_overlay_style`, the imperative API for setting
    ///    system overlays style.
    pub system_overlay_style: Option<SystemUiOverlayStyle>,

    /// Forces the AppBar's Material widget type to be [`MaterialType::Transparency`]
    /// (instead of Material's default type).
    ///
    /// This will remove the visual display of [`background_color`](Self::background_color) and
    /// [`elevation`](Self::elevation), and affect other characteristics of the AppBar's
    /// Material widget.
    ///
    /// Provided for cases where the app bar is to be transparent, and gestures
    /// must pass through the app bar to widgets beneath the app bar (i.e. with
    /// `Scaffold::extend_body_behind_app_bar` set to true).
    ///
    /// Defaults to false.
    pub force_material_transparency: bool,

    /// Whether to use the default semantic ordering for the app bar's children for
    /// accessibility traversal order.
    ///
    /// If this is set to true, the app bar will use the default semantic ordering,
    /// which places the flexible space after the main content in the semantics tree.
    /// This affects how screen readers and other assistive technologies navigate the app bar's
    /// content.
    ///
    /// Set this to false if you want to customize semantics traversal order in the app bar.
    /// You can then assign `SemanticsSortKey`s to app bar's children to control the order.
    ///
    /// Defaults to true.
    pub use_default_semantics_order: bool,

    /// The content will be clipped (or not) according to this option.
    ///
    /// See the enum [`Clip`] for details of all possible options and their common use cases.
    pub clip_behavior: Option<Clip>,

    /// The padding between the [`actions`](Self::actions) and the end of the AppBar.
    ///
    /// Defaults to zero.
    pub actions_padding: Option<EdgeInsetsGeometry>,

    /// Whether the color should be animated.
    pub animate_color: bool,
}

impl AppBar {
    /// Creates a Material Design app bar.
    ///
    /// If [`elevation`](Self::elevation) is specified, it must be non-negative.
    ///
    /// Typically used in the `Scaffold::app_bar` property.
    pub fn new() -> AppBar {
        AppBar {
            key: None,
            leading: None,
            automatically_imply_leading: true,
            title: None,
            actions: None,
            automatically_imply_actions: true,
            flexible_space: None,
            bottom: None,
            elevation: None,
            scrolled_under_elevation: None,
            notification_predicate: Rc::new(default_scroll_notification_predicate),
            shadow_color: None,
            surface_tint_color: None,
            shape: None,
            background_color: None,
            foreground_color: None,
            icon_theme: None,
            actions_icon_theme: None,
            primary: true,
            center_title: None,
            exclude_header_semantics: false,
            title_spacing: None,
            toolbar_opacity: 1.0,
            bottom_opacity: 1.0,
            toolbar_height: None,
            leading_width: None,
            toolbar_text_style: None,
            title_text_style: None,
            system_overlay_style: None,
            force_material_transparency: false,
            use_default_semantics_order: true,
            clip_behavior: None,
            actions_padding: None,
            animate_color: false,
        }
    }

    /// Dart `AppBar(key:)`.
    pub fn key(mut self, key: KeyRef) -> AppBar {
        self.key = Some(key);
        self
    }

    /// Dart `AppBar(leading:)`.
    pub fn leading<K>(mut self, leading: impl IntoWidget<K>) -> AppBar {
        self.leading = Some(leading.into_widget());
        self
    }

    /// Dart `AppBar(automaticallyImplyLeading:)`.
    pub fn automatically_imply_leading(mut self, automatically_imply_leading: bool) -> AppBar {
        self.automatically_imply_leading = automatically_imply_leading;
        self
    }

    /// Dart `AppBar(title:)`.
    pub fn title<K>(mut self, title: impl IntoWidget<K>) -> AppBar {
        self.title = Some(title.into_widget());
        self
    }

    /// Dart `AppBar(actions:)`.
    pub fn actions(mut self, actions: Vec<WidgetRef>) -> AppBar {
        self.actions = Some(actions);
        self
    }

    /// Dart `AppBar(automaticallyImplyActions:)`.
    pub fn automatically_imply_actions(mut self, automatically_imply_actions: bool) -> AppBar {
        self.automatically_imply_actions = automatically_imply_actions;
        self
    }

    /// Dart `AppBar(flexibleSpace:)`.
    pub fn flexible_space<K>(mut self, flexible_space: impl IntoWidget<K>) -> AppBar {
        self.flexible_space = Some(flexible_space.into_widget());
        self
    }

    /// Dart `AppBar(bottom:)`.
    pub fn bottom(mut self, bottom: PreferredSizeWidgetRef) -> AppBar {
        self.bottom = Some(bottom);
        self
    }

    /// Dart `AppBar(elevation:)`.
    pub fn elevation(mut self, elevation: f64) -> AppBar {
        debug_assert!(elevation >= 0.0);
        self.elevation = Some(elevation);
        self
    }

    /// Dart `AppBar(scrolledUnderElevation:)`.
    pub fn scrolled_under_elevation(mut self, scrolled_under_elevation: f64) -> AppBar {
        self.scrolled_under_elevation = Some(scrolled_under_elevation);
        self
    }

    /// Dart `AppBar(notificationPredicate:)`.
    pub fn notification_predicate(
        mut self,
        notification_predicate: ScrollNotificationPredicate,
    ) -> AppBar {
        self.notification_predicate = notification_predicate;
        self
    }

    /// Dart `AppBar(shadowColor:)`.
    pub fn shadow_color(mut self, shadow_color: Color) -> AppBar {
        self.shadow_color = Some(shadow_color);
        self
    }

    /// Dart `AppBar(surfaceTintColor:)`.
    pub fn surface_tint_color(mut self, surface_tint_color: Color) -> AppBar {
        self.surface_tint_color = Some(surface_tint_color);
        self
    }

    /// Dart `AppBar(shape:)`.
    pub fn shape(mut self, shape: impl Into<Box<dyn ShapeBorder>>) -> AppBar {
        self.shape = Some(shape.into());
        self
    }

    /// Dart `AppBar(backgroundColor:)`.
    pub fn background_color(mut self, background_color: impl Into<AnyColor>) -> AppBar {
        self.background_color = Some(background_color.into());
        self
    }

    /// Dart `AppBar(foregroundColor:)`.
    pub fn foreground_color(mut self, foreground_color: Color) -> AppBar {
        self.foreground_color = Some(foreground_color);
        self
    }

    /// Dart `AppBar(iconTheme:)`.
    pub fn icon_theme(mut self, icon_theme: IconThemeData) -> AppBar {
        self.icon_theme = Some(icon_theme);
        self
    }

    /// Dart `AppBar(actionsIconTheme:)`.
    pub fn actions_icon_theme(mut self, actions_icon_theme: IconThemeData) -> AppBar {
        self.actions_icon_theme = Some(actions_icon_theme);
        self
    }

    /// Dart `AppBar(primary:)`.
    pub fn primary(mut self, primary: bool) -> AppBar {
        self.primary = primary;
        self
    }

    /// Dart `AppBar(centerTitle:)`.
    pub fn center_title(mut self, center_title: bool) -> AppBar {
        self.center_title = Some(center_title);
        self
    }

    /// Dart `AppBar(excludeHeaderSemantics:)`.
    pub fn exclude_header_semantics(mut self, exclude_header_semantics: bool) -> AppBar {
        self.exclude_header_semantics = exclude_header_semantics;
        self
    }

    /// Dart `AppBar(titleSpacing:)`.
    pub fn title_spacing(mut self, title_spacing: f64) -> AppBar {
        self.title_spacing = Some(title_spacing);
        self
    }

    /// Dart `AppBar(toolbarOpacity:)`.
    pub fn toolbar_opacity(mut self, toolbar_opacity: f64) -> AppBar {
        self.toolbar_opacity = toolbar_opacity;
        self
    }

    /// Dart `AppBar(bottomOpacity:)`.
    pub fn bottom_opacity(mut self, bottom_opacity: f64) -> AppBar {
        self.bottom_opacity = bottom_opacity;
        self
    }

    /// Dart `AppBar(toolbarHeight:)`.
    pub fn toolbar_height(mut self, toolbar_height: f64) -> AppBar {
        self.toolbar_height = Some(toolbar_height);
        self
    }

    /// Dart `AppBar(leadingWidth:)`.
    pub fn leading_width(mut self, leading_width: f64) -> AppBar {
        self.leading_width = Some(leading_width);
        self
    }

    /// Dart `AppBar(toolbarTextStyle:)`.
    pub fn toolbar_text_style(mut self, toolbar_text_style: TextStyle) -> AppBar {
        self.toolbar_text_style = Some(toolbar_text_style);
        self
    }

    /// Dart `AppBar(titleTextStyle:)`.
    pub fn title_text_style(mut self, title_text_style: TextStyle) -> AppBar {
        self.title_text_style = Some(title_text_style);
        self
    }

    /// Dart `AppBar(systemOverlayStyle:)`.
    pub fn system_overlay_style(mut self, system_overlay_style: SystemUiOverlayStyle) -> AppBar {
        self.system_overlay_style = Some(system_overlay_style);
        self
    }

    /// Dart `AppBar(forceMaterialTransparency:)`.
    pub fn force_material_transparency(mut self, force_material_transparency: bool) -> AppBar {
        self.force_material_transparency = force_material_transparency;
        self
    }

    /// Dart `AppBar(useDefaultSemanticsOrder:)`.
    pub fn use_default_semantics_order(mut self, use_default_semantics_order: bool) -> AppBar {
        self.use_default_semantics_order = use_default_semantics_order;
        self
    }

    /// Dart `AppBar(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> AppBar {
        self.clip_behavior = Some(clip_behavior);
        self
    }

    /// Dart `AppBar(actionsPadding:)`.
    pub fn actions_padding(mut self, actions_padding: EdgeInsetsGeometry) -> AppBar {
        self.actions_padding = Some(actions_padding);
        self
    }

    /// Dart `AppBar(animateColor:)`.
    pub fn animate_color(mut self, animate_color: bool) -> AppBar {
        self.animate_color = animate_color;
        self
    }

    /// Used by [`Scaffold`] to compute its [`AppBar`]'s overall height. The returned value is
    /// the same `preferred_size.height` unless [`AppBar::toolbar_height`] was null and
    /// `AppBarTheme::of(context).toolbar_height` is non-null. In that case the
    /// return value is the sum of the theme's toolbar height and the height of
    /// the app bar's [`AppBar::bottom`] widget.
    pub fn preferred_height_for(
        app: &mut App,
        context: BuildContext,
        preferred_size: &AnySize,
    ) -> f64 {
        if let Some(app_bar_size) = preferred_size.extension::<PreferredAppBarSize>()
            && app_bar_size.toolbar_height.is_none()
        {
            return AppBarTheme::of(app, context)
                .toolbar_height
                .unwrap_or(K_TOOLBAR_HEIGHT)
                + app_bar_size.bottom_height.unwrap_or(0.0);
        }
        preferred_size.height()
    }

    /// One of the two `Semantics` around the flexible space's `Stack` children: `order` is the
    /// child's `OrdinalSortKey` when the bar uses the default semantics order.
    fn semantics_in_order(use_default_semantics_order: bool, order: f64) -> Semantics {
        let semantics = Semantics::new().explicit_child_nodes(true);
        if use_default_semantics_order {
            return semantics.sort_key(Rc::new(OrdinalSortKey::new(order)));
        }
        semantics
    }

    fn get_effective_center_title(
        &self,
        theme: &ThemeData,
        appbar_theme: &AppBarThemeData,
    ) -> bool {
        let platform_center = || match theme.platform {
            TargetPlatform::IOS | TargetPlatform::MacOS => self
                .actions
                .as_ref()
                .is_none_or(|actions| actions.len() < 2),
            TargetPlatform::Android
            | TargetPlatform::Fuchsia
            | TargetPlatform::Linux
            | TargetPlatform::Windows => false,
        };

        self.center_title
            .or(appbar_theme.center_title)
            .unwrap_or_else(platform_center)
    }
}

impl Default for AppBar {
    fn default() -> AppBar {
        AppBar::new()
    }
}

impl PreferredSizeWidget for AppBar {
    /// A size whose height is the sum of [`toolbar_height`](AppBar::toolbar_height) and the
    /// [`bottom`](AppBar::bottom) widget's preferred height.
    ///
    /// [`Scaffold`] uses this size to set its app bar's height.
    ///
    /// Dart fills this field in the constructor; a fluent chain has no such point, so it is
    /// worked out from the fields when it is read.
    fn preferred_size(&self) -> AnySize {
        PreferredAppBarSize::new(
            self.toolbar_height,
            self.bottom
                .as_ref()
                .map(|bottom| bottom.preferred_size().height()),
        )
        .into_any()
    }
}

impl Debug for AppBar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppBar")
            .field("key", &self.key)
            .field("title", &self.title)
            .field("toolbar_height", &self.toolbar_height)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for AppBar {
    type State = AppBarState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn as_preferred_size_widget(&self) -> Option<&dyn PreferredSizeWidget> {
        Some(self)
    }

    fn create_state(&self) -> AppBarState {
        AppBarState {
            state: StateData::new(),
            scroll_notification_observer: None,
            scrolled_under: false,
            scroll_notification_listener: None,
        }
    }
}

/// Dart's `_AppBarState`.
pub struct AppBarState {
    state: StateData<AppBar>,
    scroll_notification_observer: Option<Handle<ScrollNotificationObserverState>>,
    scrolled_under: bool,
    /// Dart's `_handleScrollNotification` tear-off, made once so that it compares equal.
    scroll_notification_listener: Option<ScrollNotificationCallback>,
}

impl AppBarState {
    /// Dart's `_handleScrollNotification` tear-off, which compares equal across calls.
    fn scroll_notification_listener(
        self: Handle<Self>,
        app: &mut App,
    ) -> ScrollNotificationCallback {
        if let Some(listener) = app.get(self).scroll_notification_listener.clone() {
            return listener;
        }
        let listener: ScrollNotificationCallback =
            Rc::new(move |app, notification| self.handle_scroll_notification(app, notification));
        app.get_mut(self).scroll_notification_listener = Some(Rc::clone(&listener));
        listener
    }

    fn handle_scroll_notification(
        self: Handle<Self>,
        app: &mut App,
        notification: &dyn ScrollNotification,
    ) {
        let Some(update) = notification
            .as_any()
            .downcast_ref::<ScrollUpdateNotification>()
        else {
            return;
        };
        if !(self.widget(app).notification_predicate)(notification) {
            return;
        }
        let old_scrolled_under = app.get(self).scrolled_under;
        let metrics = update.metrics();
        match metrics.axis_direction() {
            AxisDirection::Up => {
                // Scroll view is reversed
                app.get_mut(self).scrolled_under = metrics.extent_after() > 0.0;
            }
            AxisDirection::Down => {
                app.get_mut(self).scrolled_under = metrics.extent_before() > 0.0;
            }
            AxisDirection::Right | AxisDirection::Left => {
                // Scrolled under is only supported in the vertical axis, and should
                // not be altered based on horizontal notifications of the same
                // predicate since it could be a 2D scroller.
            }
        }

        if app.get(self).scrolled_under != old_scrolled_under {
            self.set_state(app, |_| {
                // React to a change in WidgetState.scrolledUnder
            });
        }
    }

    fn resolve_color(
        states: &WidgetStates,
        widget_color: Option<&AnyColor>,
        theme_color: Option<Color>,
        default_color: Color,
    ) -> Color {
        <dyn WidgetStateProperty<Color>>::resolve_as(&widget_color.cloned(), states)
            .or_else(|| {
                <dyn WidgetStateProperty<Color>>::resolve_as(
                    &theme_color.map(AnyColor::from),
                    states,
                )
            })
            .unwrap_or_else(|| {
                <dyn WidgetStateProperty<Color>>::resolve_as(&AnyColor::from(default_color), states)
            })
            .color()
    }

    fn system_overlay_style_for_brightness(
        brightness: waft_embedder::Brightness,
        background_color: Option<Color>,
    ) -> SystemUiOverlayStyle {
        let style = if brightness == waft_embedder::Brightness::Dark {
            SystemUiOverlayStyle::LIGHT
        } else {
            SystemUiOverlayStyle::DARK
        };
        // For backward compatibility, create an overlay style without system navigation bar settings.
        SystemUiOverlayStyle {
            status_bar_color: background_color,
            status_bar_brightness: style.status_bar_brightness,
            status_bar_icon_brightness: style.status_bar_icon_brightness,
            system_status_bar_contrast_enforced: style.system_status_bar_contrast_enforced,
            ..SystemUiOverlayStyle::new()
        }
    }
}

impl State for AppBarState {
    type Widget = AppBar;
    waft_widgets::state_accessors!();

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        let listener = self.scroll_notification_listener(app);
        if let Some(observer) = app.get(self).scroll_notification_observer {
            observer.remove_listener(app, &listener);
        }
        let context = self.context(app);
        let scaffold_state = Scaffold::maybe_of(app, context);

        if let Some(scaffold_state) = scaffold_state
            && (scaffold_state.is_drawer_open(app) || scaffold_state.is_end_drawer_open(app))
        {
            return;
        }
        let observer = ScrollNotificationObserver::maybe_of(app, context);
        app.get_mut(self).scroll_notification_observer = observer;
        if let Some(observer) = observer {
            observer.add_listener(app, listener);
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(observer) = app.get(self).scroll_notification_observer {
            let listener = self.scroll_notification_listener(app);
            observer.remove_listener(app, &listener);
            app.get_mut(self).scroll_notification_observer = None;
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(
            !self.widget(app).primary || waft_widgets::debug_check_has_media_query(app, context)
        );
        debug_assert!(debug_check_has_material_localizations(app, context));
        let theme = Theme::of(app, context);
        let icon_button_theme = IconButtonTheme::of(app, context);
        let app_bar_theme = AppBarTheme::of(app, context);
        let defaults = if theme.use_material3 {
            app_bar_defaults_m3(app, context)
        } else {
            app_bar_defaults_m2(app, context)
        };
        let scaffold = Scaffold::maybe_of(app, context);
        let parent_route = AnyModalRoute::of(app, context);

        let settings_is_scrolled_under = context
            .depend_on_inherited_widget_of_exact_type::<FlexibleSpaceBarSettings>(app)
            .and_then(|settings| settings.is_scrolled_under);
        let mut states = WidgetStates::new();
        if settings_is_scrolled_under.unwrap_or(app.get(self).scrolled_under) {
            states.insert(WidgetState::ScrolledUnder);
        }

        let has_drawer = scaffold.is_some_and(|scaffold| scaffold.has_drawer(app));
        let has_end_drawer = scaffold.is_some_and(|scaffold| scaffold.has_end_drawer(app));
        let use_close_button = parent_route.is_some_and(|route| route.fullscreen_dialog(app));

        let widget = self.widget(app);
        let toolbar_height = widget
            .toolbar_height
            .or(app_bar_theme.toolbar_height)
            .unwrap_or(K_TOOLBAR_HEIGHT);

        let background_color = AppBarState::resolve_color(
            &states,
            widget.background_color.as_ref(),
            app_bar_theme.background_color,
            defaults
                .background_color
                .expect("the defaults give a background color"),
        );

        let scrolled_under_background = AppBarState::resolve_color(
            &states,
            widget.background_color.as_ref(),
            app_bar_theme.background_color,
            theme.color_scheme.surface_container(),
        );

        let effective_background_color = if states.contains(&WidgetState::ScrolledUnder) {
            scrolled_under_background
        } else {
            background_color
        };

        let foreground_color = widget
            .foreground_color
            .or(app_bar_theme.foreground_color)
            .or(defaults.foreground_color)
            .expect("the defaults give a foreground color");

        let elevation = widget
            .elevation
            .or(app_bar_theme.elevation)
            .or(defaults.elevation)
            .expect("the defaults give an elevation");

        let effective_elevation = if states.contains(&WidgetState::ScrolledUnder) {
            widget
                .scrolled_under_elevation
                .or(app_bar_theme.scrolled_under_elevation)
                .or(defaults.scrolled_under_elevation)
                .unwrap_or(elevation)
        } else {
            elevation
        };

        let mut overall_icon_theme = widget
            .icon_theme
            .clone()
            .or_else(|| app_bar_theme.icon_theme.clone())
            .unwrap_or_else(|| {
                defaults
                    .icon_theme
                    .as_ref()
                    .expect("the defaults give an icon theme")
                    .copy_with()
                    .color(foreground_color)
            });

        let action_foreground_color = widget.foreground_color.or(app_bar_theme.foreground_color);
        let mut actions_icon_theme = widget
            .actions_icon_theme
            .clone()
            .or_else(|| app_bar_theme.actions_icon_theme.clone())
            .or_else(|| widget.icon_theme.clone())
            .or_else(|| app_bar_theme.icon_theme.clone())
            .or_else(|| {
                defaults
                    .actions_icon_theme
                    .as_ref()
                    .map(|actions_icon_theme| match action_foreground_color {
                        Some(color) => actions_icon_theme.copy_with().color(color),
                        None => actions_icon_theme.copy_with(),
                    })
            })
            .unwrap_or_else(|| overall_icon_theme.clone());

        let actions_padding = widget
            .actions_padding
            .or(app_bar_theme.actions_padding)
            .or(defaults.actions_padding)
            .expect("the defaults give an actions padding");

        let mut toolbar_text_style = widget
            .toolbar_text_style
            .clone()
            .or_else(|| app_bar_theme.toolbar_text_style.clone())
            .or_else(|| {
                defaults
                    .toolbar_text_style
                    .as_ref()
                    .map(|style| style.copy_with().color(foreground_color))
            });

        let mut title_text_style = widget
            .title_text_style
            .clone()
            .or_else(|| app_bar_theme.title_text_style.clone())
            .or_else(|| {
                defaults
                    .title_text_style
                    .as_ref()
                    .map(|style| style.copy_with().color(foreground_color))
            });

        #[expect(deprecated, reason = "Dart calls the deprecated `Color.withOpacity`")]
        if widget.toolbar_opacity != 1.0 {
            let opacity = Interval::new(0.25, 1.0, Curves::fast_out_slow_in())
                .transform(widget.toolbar_opacity);
            if let Some(style) = &title_text_style
                && let Some(color) = &style.color
            {
                title_text_style = Some(style.copy_with().color(color.with_opacity(opacity)));
            }
            if let Some(style) = &toolbar_text_style
                && let Some(color) = &style.color
            {
                toolbar_text_style = Some(style.copy_with().color(color.with_opacity(opacity)));
            }
            overall_icon_theme = overall_icon_theme
                .copy_with()
                .opacity(opacity * overall_icon_theme.opacity.unwrap_or(1.0));
            actions_icon_theme = actions_icon_theme
                .copy_with()
                .opacity(opacity * actions_icon_theme.opacity.unwrap_or(1.0));
        }

        let mut leading = widget.leading.clone();
        if leading.is_none() && widget.automatically_imply_leading {
            if has_drawer {
                leading = Some(
                    DrawerButton::new()
                        .style(
                            IconButton::style_from()
                                .icon_size(overall_icon_theme.size.unwrap_or(24.0))
                                .build(),
                        )
                        .into_widget(),
                );
            } else if parent_route.is_some_and(|route| route.implies_app_bar_dismissal(app)) {
                leading = Some(if use_close_button {
                    CloseButton::new().into_widget()
                } else {
                    BackButton::new().into_widget()
                });
            }
        }
        let widget = self.widget(app);
        let leading_width = widget
            .leading_width
            .or(app_bar_theme.leading_width)
            .unwrap_or(K_LEADING_WIDTH);
        let leading = leading.map(|mut leading| {
            if theme.use_material3 {
                // This comparison is to check if there is a custom [overallIconTheme]. If true, it
                // means that no custom [overallIconTheme] is provided, so [iconButtonTheme] is
                // applied. Otherwise, we generate a new [IconButtonThemeData] based on the values
                // from [overallIconTheme]. If [iconButtonTheme] only has null values, the default
                // [overallIconTheme] will be applied below by [IconTheme.merge]
                let effective_icon_button_theme =
                    if Some(&overall_icon_theme) == defaults.icon_theme.as_ref() {
                        icon_button_theme.clone()
                    } else {
                        // The [IconButton.styleFrom] method is used to generate a correct
                        // [overlayColor] based on the [foregroundColor].
                        icon_button_theme_for(&icon_button_theme, &overall_icon_theme)
                    };

                leading = IconButtonTheme::new(
                    effective_icon_button_theme,
                    if is_icon_button(&leading) {
                        Center::new().child(leading).into_widget()
                    } else {
                        leading
                    },
                )
                .into_widget();

                // Based on the Material Design 3 specs, the leading IconButton should have
                // a size of 48x48, and a highlight size of 40x40. Users can also put other
                // type of widgets on leading with the original config.
                ConstrainedBox::new(BoxConstraints::tight_for(Some(leading_width), None))
                    .child(leading)
                    .into_widget()
            } else {
                ConstrainedBox::new(BoxConstraints::tight_for(Some(leading_width), None))
                    .child(leading)
                    .into_widget()
            }
        });

        let names_route = match app.platform().target_platform() {
            TargetPlatform::Android
            | TargetPlatform::Fuchsia
            | TargetPlatform::Linux
            | TargetPlatform::Windows => Some(true),
            TargetPlatform::IOS | TargetPlatform::MacOS => None,
        };
        let title = widget.title.clone().map(|title| {
            let mut title = AppBarTitleBox { child: title }.into_widget();
            if !widget.exclude_header_semantics {
                let mut semantics = Semantics::new().header(true);
                if let Some(names_route) = names_route {
                    semantics = semantics.names_route(names_route);
                }
                title = semantics.child(title).into_widget();
            }
            let title = DefaultTextStyle::new(
                title_text_style
                    .clone()
                    .expect("the defaults give a title text style"),
                title,
            )
            .soft_wrap(false)
            .overflow(TextOverflow::Ellipsis);

            // Set maximum text scale factor to [_kMaxTitleTextScaleFactor] for the
            // title to keep the visual hierarchy the same even with larger font
            // sizes. To opt out, wrap the [title] widget in a [MediaQuery] widget
            // with a different `TextScaler`.
            MediaQuery::with_clamped_text_scaling(None, 0.0, K_MAX_TITLE_TEXT_SCALE_FACTOR, title)
        });

        let mut actions = None;
        if let Some(widget_actions) = widget
            .actions
            .as_ref()
            .filter(|actions| !actions.is_empty())
        {
            actions = Some(
                Padding::new(actions_padding)
                    .child(
                        Row::new()
                            .main_axis_size(MainAxisSize::Min)
                            .cross_axis_alignment(if theme.use_material3 {
                                CrossAxisAlignment::Center
                            } else {
                                CrossAxisAlignment::Stretch
                            })
                            .children(widget_actions.iter().cloned()),
                    )
                    .into_widget(),
            );
        } else if has_end_drawer && widget.automatically_imply_actions {
            actions = Some(
                EndDrawerButton::new()
                    .style(
                        IconButton::style_from()
                            .icon_size(overall_icon_theme.size.unwrap_or(24.0))
                            .build(),
                    )
                    .into_widget(),
            );
        }

        // Allow the trailing actions to have their own theme if necessary.
        let actions = actions.map(|actions| {
            let effective_actions_icon_button_theme =
                if Some(&actions_icon_theme) == defaults.actions_icon_theme.as_ref() {
                    icon_button_theme.clone()
                } else {
                    icon_button_theme_for(&icon_button_theme, &actions_icon_theme)
                };

            IconButtonTheme::new(
                effective_actions_icon_button_theme,
                IconTheme::merge(None, actions_icon_theme.clone(), actions),
            )
            .into_widget()
        });

        let mut toolbar = NavigationToolbar::new()
            .center_middle(widget.get_effective_center_title(&theme, &app_bar_theme))
            .middle_spacing(
                widget
                    .title_spacing
                    .or(app_bar_theme.title_spacing)
                    .unwrap_or(NavigationToolbar::K_MIDDLE_SPACING),
            );
        toolbar.leading = leading;
        toolbar.middle = title;
        toolbar.trailing = actions;

        // If the toolbar is allocated less than toolbarHeight make it
        // appear to scroll upwards within its shrinking container.
        let mut app_bar = ClipRect::new()
            .clip_behavior(widget.clip_behavior.unwrap_or(Clip::HardEdge))
            .child(
                CustomSingleChildLayout::new(Rc::new(ToolbarContainerLayout { toolbar_height }))
                    .child(IconTheme::merge(
                        None,
                        overall_icon_theme,
                        DefaultTextStyle::new(
                            toolbar_text_style.expect("the defaults give a toolbar text style"),
                            toolbar,
                        ),
                    )),
            )
            .into_widget();
        if let Some(bottom) = &widget.bottom {
            let bottom = if widget.bottom_opacity == 1.0 {
                bottom.widget().clone()
            } else {
                Opacity::new(
                    Interval::new(0.25, 1.0, Curves::fast_out_slow_in())
                        .transform(widget.bottom_opacity),
                )
                .child(bottom.clone())
                .into_widget()
            };
            app_bar = Column::new()
                .main_axis_alignment(MainAxisAlignment::SpaceBetween)
                .children([
                    Flexible::new(
                        ConstrainedBox::new(BoxConstraints::new().max_height(toolbar_height))
                            .child(app_bar),
                    )
                    .into_widget(),
                    bottom,
                ])
                .into_widget();
        }

        // The padding applies to the toolbar and tabbar, not the flexible space.
        if widget.primary {
            app_bar = SafeArea::new(app_bar).bottom(false).into_widget();
        }

        app_bar = Align::new()
            .alignment(AlignmentGeometry::TOP_CENTER)
            .child(app_bar)
            .into_widget();

        if let Some(flexible_space) = &widget.flexible_space {
            app_bar = Stack::new()
                .fit(StackFit::Passthrough)
                .children([
                    AppBar::semantics_in_order(widget.use_default_semantics_order, 1.0)
                        .child(flexible_space.clone())
                        .into_widget(),
                    AppBar::semantics_in_order(widget.use_default_semantics_order, 0.0)
                        // Creates a material widget to prevent the flexibleSpace from
                        // obscuring the ink splashes produced by appBar children.
                        .child(
                            Material::new()
                                .r#type(MaterialType::Transparency)
                                .child(app_bar),
                        )
                        .into_widget(),
                ])
                .into_widget();
        }

        let overlay_style = widget
            .system_overlay_style
            .or(app_bar_theme.system_overlay_style)
            .or(defaults.system_overlay_style)
            .unwrap_or_else(|| {
                AppBarState::system_overlay_style_for_brightness(
                    ThemeData::estimate_brightness_for_color(effective_background_color),
                    // Make the status bar transparent for M3 so the elevation overlay
                    // color is picked up by the statusbar.
                    theme.use_material3.then_some(Color::new(0x00000000)),
                )
            });

        let mut material = Material::new()
            .color(if theme.use_material3 {
                effective_background_color
            } else {
                background_color
            })
            .elevation(effective_elevation)
            .r#type(if widget.force_material_transparency {
                MaterialType::Transparency
            } else {
                MaterialType::Canvas
            })
            .animate_color(widget.animate_color);
        if let Some(shadow_color) = widget
            .shadow_color
            .or(app_bar_theme.shadow_color)
            .or(defaults.shadow_color)
        {
            material = material.shadow_color(shadow_color);
        }
        // M3 `defaults.surfaceTint` is Colors.transparent now. It is not used
        // here because otherwise, it will cause breaking change for
        // `scrolledUnderElevation`.
        if let Some(surface_tint_color) = widget
            .surface_tint_color
            .or(app_bar_theme.surface_tint_color)
            .or_else(|| {
                theme
                    .use_material3
                    .then(|| theme.color_scheme.surface_tint())
            })
        {
            material = material.surface_tint_color(surface_tint_color);
        }
        if let Some(shape) = widget
            .shape
            .clone()
            .or_else(|| app_bar_theme.shape.clone())
            .or_else(|| defaults.shape.clone())
        {
            material = material.shape(shape);
        }
        Semantics::new()
            .container(true)
            .child(AnnotatedRegion::new(
                material.child(Semantics::new().explicit_child_nodes(true).child(app_bar)),
                overlay_style,
            ))
            .into_widget()
    }
}

impl Debug for AppBarState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppBarState")
            .field("scrolled_under", &self.scrolled_under)
            .finish_non_exhaustive()
    }
}

/// Dart's `leading is IconButton`: an [`IconButton`] or one of the action buttons, the four
/// widgets Dart declares as its subclasses (`_ActionButton extends IconButton`).
fn is_icon_button(widget: &WidgetRef) -> bool {
    let widget = &**widget;
    downcast_widget::<IconButton>(widget).is_some()
        || downcast_widget::<BackButton>(widget).is_some()
        || downcast_widget::<CloseButton>(widget).is_some()
        || downcast_widget::<DrawerButton>(widget).is_some()
        || downcast_widget::<EndDrawerButton>(widget).is_some()
}

/// Dart's `IconButtonThemeData(style: iconButtonTheme.style?.copyWith(foregroundColor:,
/// overlayColor:, iconSize:))` from `IconButton.styleFrom(foregroundColor: iconTheme.color,
/// iconSize: iconTheme.size)`, which [`AppBar`] gives its leading widget and its actions.
fn icon_button_theme_for(
    icon_button_theme: &IconButtonThemeData,
    icon_theme: &IconThemeData,
) -> IconButtonThemeData {
    let mut style_from = IconButton::style_from();
    if let Some(color) = &icon_theme.color {
        style_from = style_from.foreground_color(color.color());
    }
    if let Some(size) = icon_theme.size {
        style_from = style_from.icon_size(size);
    }
    let icon_button_style = style_from.build();

    let mut data = IconButtonThemeData::new();
    data.style = icon_button_theme
        .style
        .as_ref()
        .map(|style| copy_with_icon_button_style(style, icon_button_style));
    data
}

/// Dart's `style.copyWith(foregroundColor:, overlayColor:, iconSize:)` with the given style's
/// three properties, each of which replaces `style`'s only when it is not null.
fn copy_with_icon_button_style(style: &ButtonStyle, icon_button_style: ButtonStyle) -> ButtonStyle {
    let mut style = style.copy_with();
    if let Some(foreground_color) = icon_button_style.foreground_color {
        style.foreground_color = Some(foreground_color);
    }
    if let Some(overlay_color) = icon_button_style.overlay_color {
        style.overlay_color = Some(overlay_color);
    }
    if let Some(icon_size) = icon_button_style.icon_size {
        style.icon_size = Some(icon_size);
    }
    style
}

/// Dart's `widget != oldWidget` on two optional widgets, which compares object identity.
fn same_optional_widget(a: &Option<WidgetRef>, b: &Option<WidgetRef>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => Rc::ptr_eq(a, b),
        _ => false,
    }
}

/// Dart's `actions != oldDelegate.actions`, which compares the lists by identity; a `Vec` has
/// none, so two lists are the same when they hold the same widgets.
fn same_actions(a: &Option<Vec<WidgetRef>>, b: &Option<Vec<WidgetRef>>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => a.len() == b.len() && a.iter().zip(b).all(|(a, b)| Rc::ptr_eq(a, b)),
        _ => false,
    }
}

/// Dart's `bottom != oldDelegate.bottom`, which compares the widgets by identity.
fn same_bottom(a: &Option<PreferredSizeWidgetRef>, b: &Option<PreferredSizeWidgetRef>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => Rc::ptr_eq(a.widget(), b.widget()),
        _ => false,
    }
}

/// Dart's `!=` on two configurations, which compares them by identity.
fn same_configuration<T>(a: &Option<Rc<T>>, b: &Option<Rc<T>>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => Rc::ptr_eq(a, b),
        _ => false,
    }
}

/// Dart's `const Cubic(0.2, 0.0, 0.0, 1.0)` that fades a medium or large app bar's collapsed
/// title, one per thread as Dart canonicalizes the constant.
fn collapsed_title_fade_curve() -> Rc<dyn Curve> {
    thread_local! {
        static CURVE: Rc<dyn Curve> = Rc::new(Cubic::new(0.2, 0.0, 0.0, 1.0));
    }
    CURVE.with(Rc::clone)
}

/// Dart's `_SliverAppBarDelegate`.
struct SliverAppBarDelegate {
    leading: Option<WidgetRef>,
    automatically_imply_leading: bool,
    title: Option<WidgetRef>,
    actions: Option<Vec<WidgetRef>>,
    automatically_imply_actions: bool,
    flexible_space: Option<WidgetRef>,
    bottom: Option<PreferredSizeWidgetRef>,
    elevation: Option<f64>,
    scrolled_under_elevation: Option<f64>,
    shadow_color: Option<Color>,
    surface_tint_color: Option<Color>,
    force_elevated: bool,
    background_color: Option<AnyColor>,
    foreground_color: Option<Color>,
    icon_theme: Option<IconThemeData>,
    actions_icon_theme: Option<IconThemeData>,
    primary: bool,
    center_title: Option<bool>,
    exclude_header_semantics: bool,
    title_spacing: Option<f64>,
    expanded_height: Option<f64>,
    collapsed_height: f64,
    top_padding: f64,
    floating: bool,
    pinned: bool,
    shape: Option<Box<dyn ShapeBorder>>,
    toolbar_height: Option<f64>,
    leading_width: Option<f64>,
    toolbar_text_style: Option<TextStyle>,
    title_text_style: Option<TextStyle>,
    system_overlay_style: Option<SystemUiOverlayStyle>,
    bottom_height: f64,
    force_material_transparency: bool,
    use_default_semantics_order: bool,
    clip_behavior: Option<Clip>,
    variant: SliverAppVariant,
    accessible_navigation: bool,
    actions_padding: Option<EdgeInsetsGeometry>,
    vsync: TickerProviderRef,
    snap_configuration: Option<Rc<FloatingHeaderSnapConfiguration>>,
    stretch_configuration: Option<Rc<OverScrollHeaderStretchConfiguration>>,
    show_on_screen_configuration: Option<PersistentHeaderShowOnScreenConfiguration>,
}

impl SliverPersistentHeaderDelegate for SliverAppBarDelegate {
    fn min_extent(&self) -> f64 {
        self.collapsed_height
    }

    fn max_extent(&self) -> f64 {
        f64::max(
            self.top_padding
                + self.expanded_height.unwrap_or(
                    self.toolbar_height.unwrap_or(K_TOOLBAR_HEIGHT) + self.bottom_height,
                ),
            self.min_extent(),
        )
    }

    fn vsync(&self) -> Option<TickerProviderRef> {
        Some(self.vsync.clone())
    }

    fn snap_configuration(&self) -> Option<FloatingHeaderSnapConfiguration> {
        self.snap_configuration.as_deref().cloned()
    }

    fn stretch_configuration(&self) -> Option<OverScrollHeaderStretchConfiguration> {
        self.stretch_configuration.as_deref().cloned()
    }

    fn show_on_screen_configuration(&self) -> Option<PersistentHeaderShowOnScreenConfiguration> {
        self.show_on_screen_configuration
    }

    fn build(
        &self,
        _app: &mut App,
        _context: BuildContext,
        shrink_offset: f64,
        overlaps_content: bool,
    ) -> WidgetRef {
        let toolbar_height = self.toolbar_height.unwrap_or(K_TOOLBAR_HEIGHT);
        let visible_main_height = self.max_extent() - shrink_offset - self.top_padding;
        let extra_toolbar_height = f64::max(
            self.min_extent() - self.bottom_height - self.top_padding - toolbar_height,
            0.0,
        );
        let visible_toolbar_height =
            visible_main_height - self.bottom_height - extra_toolbar_height;

        let is_scrolled_under = overlaps_content
            || self.force_elevated
            || (self.pinned && shrink_offset > self.max_extent() - self.min_extent());
        let is_pinned_with_opacity_fade =
            self.pinned && self.floating && self.bottom.is_some() && extra_toolbar_height == 0.0;
        let toolbar_opacity =
            if !self.accessible_navigation && (!self.pinned || is_pinned_with_opacity_fade) {
                clamp_double(visible_toolbar_height / toolbar_height, 0.0, 1.0)
            } else {
                1.0
            };
        let effective_title = match self.variant {
            SliverAppVariant::Small => self.title.clone(),
            SliverAppVariant::Medium | SliverAppVariant::Large => {
                let mut fade = AnimatedOpacity::new(
                    if is_scrolled_under { 1.0 } else { 0.0 },
                    Duration::from_millis(500),
                )
                .curve(collapsed_title_fade_curve());
                fade.child = self.title.clone();
                Some(fade.into_widget())
            }
        };

        let mut app_bar = AppBar::new()
            .automatically_imply_leading(self.automatically_imply_leading)
            .automatically_imply_actions(self.automatically_imply_actions)
            .primary(self.primary)
            .exclude_header_semantics(self.exclude_header_semantics)
            .toolbar_opacity(toolbar_opacity)
            .bottom_opacity(if self.pinned {
                1.0
            } else {
                clamp_double(visible_main_height / self.bottom_height, 0.0, 1.0)
            })
            .force_material_transparency(self.force_material_transparency)
            .use_default_semantics_order(self.use_default_semantics_order);
        app_bar.elevation = if is_scrolled_under {
            self.elevation
        } else {
            Some(0.0)
        };
        debug_assert!(app_bar.elevation.is_none_or(|elevation| elevation >= 0.0));
        app_bar.clip_behavior = self.clip_behavior;
        app_bar.leading = self.leading.clone();
        app_bar.title = effective_title;
        app_bar.actions = self.actions.clone();
        app_bar.flexible_space = match &self.flexible_space {
            Some(flexible_space) if self.title.is_none() && !self.exclude_header_semantics => Some(
                Semantics::new()
                    .header(true)
                    .child(flexible_space.clone())
                    .into_widget(),
            ),
            flexible_space => flexible_space.clone(),
        };
        app_bar.bottom = self.bottom.clone();
        app_bar.scrolled_under_elevation = self.scrolled_under_elevation;
        app_bar.shadow_color = self.shadow_color;
        app_bar.surface_tint_color = self.surface_tint_color;
        app_bar.background_color = self.background_color.clone();
        app_bar.foreground_color = self.foreground_color;
        app_bar.icon_theme = self.icon_theme.clone();
        app_bar.actions_icon_theme = self.actions_icon_theme.clone();
        app_bar.center_title = self.center_title;
        app_bar.title_spacing = self.title_spacing;
        app_bar.shape = self.shape.clone();
        app_bar.toolbar_height = self.toolbar_height;
        app_bar.leading_width = self.leading_width;
        app_bar.toolbar_text_style = self.toolbar_text_style.clone();
        app_bar.title_text_style = self.title_text_style.clone();
        app_bar.system_overlay_style = self.system_overlay_style;
        app_bar.actions_padding = self.actions_padding;

        FlexibleSpaceBar::create_settings(
            f64::max(self.min_extent(), self.max_extent() - shrink_offset),
            app_bar,
        )
        .min_extent(self.min_extent())
        .max_extent(self.max_extent())
        .toolbar_opacity(toolbar_opacity)
        .is_scrolled_under(is_scrolled_under)
        .has_leading(self.leading.is_some() || self.automatically_imply_leading)
        .build()
        .into_widget()
    }

    fn should_rebuild(&self, old_delegate: &dyn SliverPersistentHeaderDelegate) -> bool {
        let Some(old_delegate) = old_delegate.as_any().downcast_ref::<SliverAppBarDelegate>()
        else {
            return true;
        };
        !same_optional_widget(&self.leading, &old_delegate.leading)
            || self.automatically_imply_leading != old_delegate.automatically_imply_leading
            || !same_optional_widget(&self.title, &old_delegate.title)
            || !same_actions(&self.actions, &old_delegate.actions)
            || self.automatically_imply_actions != old_delegate.automatically_imply_actions
            || !same_optional_widget(&self.flexible_space, &old_delegate.flexible_space)
            || !same_bottom(&self.bottom, &old_delegate.bottom)
            || self.bottom_height != old_delegate.bottom_height
            || self.elevation != old_delegate.elevation
            || self.shadow_color != old_delegate.shadow_color
            || self.background_color != old_delegate.background_color
            || self.foreground_color != old_delegate.foreground_color
            || self.icon_theme != old_delegate.icon_theme
            || self.actions_icon_theme != old_delegate.actions_icon_theme
            || self.primary != old_delegate.primary
            || self.center_title != old_delegate.center_title
            || self.title_spacing != old_delegate.title_spacing
            || self.expanded_height != old_delegate.expanded_height
            || self.top_padding != old_delegate.top_padding
            || self.pinned != old_delegate.pinned
            || self.floating != old_delegate.floating
            || self.vsync != old_delegate.vsync
            || !same_configuration(&self.snap_configuration, &old_delegate.snap_configuration)
            || !same_configuration(
                &self.stretch_configuration,
                &old_delegate.stretch_configuration,
            )
            || self.show_on_screen_configuration != old_delegate.show_on_screen_configuration
            || self.force_elevated != old_delegate.force_elevated
            || self.toolbar_height != old_delegate.toolbar_height
            || self.leading_width != old_delegate.leading_width
            || self.toolbar_text_style != old_delegate.toolbar_text_style
            || self.title_text_style != old_delegate.title_text_style
            || self.system_overlay_style != old_delegate.system_overlay_style
            || self.force_material_transparency != old_delegate.force_material_transparency
            || self.use_default_semantics_order != old_delegate.use_default_semantics_order
            || self.accessible_navigation != old_delegate.accessible_navigation
            || self.actions_padding != old_delegate.actions_padding
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Debug for SliverAppBarDelegate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SliverAppBarDelegate(topPadding: {:.1}, bottomHeight: {:.1}, ...)",
            self.top_padding, self.bottom_height
        )
    }
}

/// A Material Design app bar that integrates with a `CustomScrollView`.
///
/// An app bar consists of a toolbar and potentially other widgets, such as a
/// `TabBar` and a [`FlexibleSpaceBar`]. App bars typically expose one or more
/// common actions with [`IconButton`]s which are optionally followed by a
/// `PopupMenuButton` for less common operations.
///
/// Sliver app bars are typically used as the first child of a
/// `CustomScrollView`, which lets the app bar integrate with the scroll view so
/// that it can vary in height according to the scroll offset or float above the
/// other content in the scroll view. For a fixed-height app bar at the top of
/// the screen see [`AppBar`], which is used in the `Scaffold::app_bar` slot.
///
/// The AppBar displays the toolbar widgets, [`leading`](Self::leading), [`title`](Self::title),
/// and [`actions`](Self::actions), above the [`bottom`](Self::bottom) (if any). If a
/// [`flexible_space`](Self::flexible_space) widget is specified then it is stacked behind the
/// toolbar and the bottom widget.
///
/// The property [`snap`](Self::snap) can only be set to true if [`floating`](Self::floating) is
/// also true.
///
/// See also:
///
///  * `CustomScrollView`, which integrates the [`SliverAppBar`] into its
///    scrolling.
///  * [`AppBar`], which is a fixed-height app bar for use in `Scaffold::app_bar`.
///  * `TabBar`, which is typically placed in the [`bottom`](Self::bottom) slot of the
///    [`AppBar`] if the screen has multiple pages arranged in tabs.
///  * [`IconButton`], which is used with [`actions`](Self::actions) to show buttons on the app
///    bar.
///  * `PopupMenuButton`, to show a popup menu on the app bar, via [`actions`](Self::actions).
///  * [`FlexibleSpaceBar`], which is used with [`flexible_space`](Self::flexible_space) when the
///    app bar can expand and collapse.
///  * <https://material.io/design/components/app-bars-top.html>
pub struct SliverAppBar {
    pub key: Option<KeyRef>,

    /// See [`AppBar::leading`].
    ///
    /// This property is used to configure an [`AppBar`].
    pub leading: Option<WidgetRef>,

    /// See [`AppBar::automatically_imply_leading`].
    ///
    /// This property is used to configure an [`AppBar`].
    pub automatically_imply_leading: bool,

    /// See [`AppBar::title`].
    ///
    /// This property is used to configure an [`AppBar`].
    pub title: Option<WidgetRef>,

    /// See [`AppBar::actions`].
    ///
    /// This property is used to configure an [`AppBar`].
    pub actions: Option<Vec<WidgetRef>>,

    /// See [`AppBar::automatically_imply_actions`].
    ///
    /// This property is used to configure an [`AppBar`].
    pub automatically_imply_actions: bool,

    /// See [`AppBar::flexible_space`].
    ///
    /// This property is used to configure an [`AppBar`].
    pub flexible_space: Option<WidgetRef>,

    /// See [`AppBar::bottom`].
    ///
    /// This property is used to configure an [`AppBar`].
    pub bottom: Option<PreferredSizeWidgetRef>,

    /// See [`AppBar::elevation`].
    ///
    /// This property is used to configure an [`AppBar`].
    pub elevation: Option<f64>,

    /// See [`AppBar::scrolled_under_elevation`].
    ///
    /// This property is used to configure an [`AppBar`].
    pub scrolled_under_elevation: Option<f64>,

    /// See [`AppBar::shadow_color`].
    ///
    /// This property is used to configure an [`AppBar`].
    pub shadow_color: Option<Color>,

    /// See [`AppBar::surface_tint_color`].
    ///
    /// This property is used to configure an [`AppBar`].
    pub surface_tint_color: Option<Color>,

    /// Whether to show the shadow appropriate for the [`elevation`](Self::elevation) even if the
    /// content is not scrolled under the [`AppBar`].
    ///
    /// Defaults to false, meaning that the [`elevation`](Self::elevation) is only applied when
    /// the [`AppBar`] is being displayed over content that is scrolled under it.
    ///
    /// When set to true, the [`elevation`](Self::elevation) is applied regardless.
    ///
    /// Ignored when [`elevation`](Self::elevation) is zero.
    pub force_elevated: bool,

    /// See [`AppBar::background_color`].
    ///
    /// This property is used to configure an [`AppBar`].
    pub background_color: Option<AnyColor>,

    /// See [`AppBar::foreground_color`].
    ///
    /// This property is used to configure an [`AppBar`].
    pub foreground_color: Option<Color>,

    /// See [`AppBar::icon_theme`].
    ///
    /// This property is used to configure an [`AppBar`].
    pub icon_theme: Option<IconThemeData>,

    /// See [`AppBar::actions_icon_theme`].
    ///
    /// This property is used to configure an [`AppBar`].
    pub actions_icon_theme: Option<IconThemeData>,

    /// See [`AppBar::primary`].
    ///
    /// This property is used to configure an [`AppBar`].
    pub primary: bool,

    /// See [`AppBar::center_title`].
    ///
    /// This property is used to configure an [`AppBar`].
    pub center_title: Option<bool>,

    /// See [`AppBar::exclude_header_semantics`].
    ///
    /// This property is used to configure an [`AppBar`].
    pub exclude_header_semantics: bool,

    /// See [`AppBar::title_spacing`].
    ///
    /// This property is used to configure an [`AppBar`].
    pub title_spacing: Option<f64>,

    /// Defines the height of the app bar when it is collapsed.
    ///
    /// By default, the collapsed height is [`toolbar_height`](Self::toolbar_height). If
    /// [`bottom`](Self::bottom) widget is specified, then its height from
    /// [`PreferredSizeWidget::preferred_size`] is added to the height. If
    /// [`primary`](Self::primary) is true, then the [`MediaQuery`] top padding,
    /// `EdgeInsets::top` of `MediaQueryData::padding`, is added as well.
    ///
    /// If [`pinned`](Self::pinned) and [`floating`](Self::floating) are true, with
    /// [`bottom`](Self::bottom) set, the default collapsed height is only the height of
    /// [`PreferredSizeWidget::preferred_size`] with the [`MediaQuery`] top padding.
    pub collapsed_height: Option<f64>,

    /// The size of the app bar when it is fully expanded.
    ///
    /// By default, the total height of the toolbar and the bottom widget (if
    /// any). If a [`flexible_space`](Self::flexible_space) widget is specified this height should
    /// be big enough to accommodate whatever that widget contains.
    ///
    /// This does not include the status bar height (which will be automatically
    /// included if [`primary`](Self::primary) is true).
    pub expanded_height: Option<f64>,

    /// Whether the app bar should become visible as soon as the user scrolls
    /// towards the app bar.
    ///
    /// Otherwise, the user will need to scroll near the top of the scroll view to
    /// reveal the app bar.
    ///
    /// If [`snap`](Self::snap) is true then a scroll that exposes the app bar will trigger an
    /// animation that slides the entire app bar into view. Similarly if a scroll
    /// dismisses the app bar, the animation will slide it completely out of view.
    pub floating: bool,

    /// Whether the app bar should remain visible at the start of the scroll view.
    ///
    /// The app bar can still expand and contract as the user scrolls, but it will
    /// remain visible rather than being scrolled out of view.
    pub pinned: bool,

    /// See [`AppBar::shape`].
    ///
    /// This property is used to configure an [`AppBar`].
    pub shape: Option<Box<dyn ShapeBorder>>,

    /// If [`snap`](Self::snap) and [`floating`](Self::floating) are true then the floating app
    /// bar will "snap" into view.
    ///
    /// If [`snap`](Self::snap) is true then a scroll that exposes the floating app bar will
    /// trigger an animation that slides the entire app bar into view. Similarly
    /// if a scroll dismisses the app bar, the animation will slide the app bar
    /// completely out of view. Additionally, setting [`snap`](Self::snap) to true will fully
    /// expand the floating app bar when the framework tries to reveal the
    /// contents of the app bar by calling `RenderObject::show_on_screen`. For
    /// example, when a `TextField` in the floating app bar gains focus, if [`snap`](Self::snap)
    /// is true, the framework will always fully expand the floating app bar, in
    /// order to reveal the focused `TextField`.
    ///
    /// Snapping only applies when the app bar is floating, not when the app bar
    /// appears at the top of its scroll view.
    pub snap: bool,

    /// Whether the app bar should stretch to fill the over-scroll area.
    ///
    /// The app bar can still expand and contract as the user scrolls, but it will
    /// also stretch when the user over-scrolls.
    pub stretch: bool,

    /// The offset of overscroll required to activate [`on_stretch_trigger`](Self::on_stretch_trigger).
    ///
    /// This defaults to 100.0.
    pub stretch_trigger_offset: f64,

    /// The callback function to be executed when a user over-scrolls to the
    /// offset specified by [`stretch_trigger_offset`](Self::stretch_trigger_offset).
    pub on_stretch_trigger: Option<AsyncCallback>,

    /// See [`AppBar::toolbar_height`].
    ///
    /// This property is used to configure an [`AppBar`].
    pub toolbar_height: f64,

    /// See [`AppBar::leading_width`].
    ///
    /// This property is used to configure an [`AppBar`].
    pub leading_width: Option<f64>,

    /// See [`AppBar::toolbar_text_style`].
    ///
    /// This property is used to configure an [`AppBar`].
    pub toolbar_text_style: Option<TextStyle>,

    /// See [`AppBar::title_text_style`].
    ///
    /// This property is used to configure an [`AppBar`].
    pub title_text_style: Option<TextStyle>,

    /// See [`AppBar::system_overlay_style`].
    ///
    /// This property is used to configure an [`AppBar`].
    pub system_overlay_style: Option<SystemUiOverlayStyle>,

    /// See [`AppBar::force_material_transparency`].
    ///
    /// This property is used to configure an [`AppBar`].
    pub force_material_transparency: bool,

    /// See [`AppBar::use_default_semantics_order`].
    ///
    /// This property is used to configure an [`AppBar`].
    pub use_default_semantics_order: bool,

    /// The content will be clipped (or not) according to this option.
    pub clip_behavior: Option<Clip>,

    /// See [`AppBar::actions_padding`].
    ///
    /// This property is used to configure an [`AppBar`].
    pub actions_padding: Option<EdgeInsetsGeometry>,

    variant: SliverAppVariant,
}

impl SliverAppBar {
    /// Creates a Material Design app bar that can be placed in a `CustomScrollView`.
    pub fn new() -> SliverAppBar {
        SliverAppBar::with_variant(SliverAppVariant::Small, false, K_TOOLBAR_HEIGHT)
    }

    /// Creates a Material Design medium top app bar that can be placed
    /// in a `CustomScrollView`.
    ///
    /// Returns a [`SliverAppBar`] configured with appropriate defaults
    /// for a medium top app bar as defined in Material 3. It starts fully
    /// expanded with the title in an area underneath the main row of icons.
    /// When the `CustomScrollView` is scrolled, the title will be scrolled
    /// under the main row. When it is fully collapsed, a smaller version of the
    /// title will fade in on the main row. The reverse will happen if it is
    /// expanded again.
    ///
    /// See also:
    ///
    ///   * [`AppBar`], for a small or center-aligned top app bar.
    ///   * [`SliverAppBar::large`], for a large top app bar.
    ///   * <https://m3.material.io/components/top-app-bar/overview>, the Material 3
    ///     app bar specification.
    pub fn medium() -> SliverAppBar {
        SliverAppBar::with_variant(
            SliverAppVariant::Medium,
            true,
            MediumScrollUnderFlexibleConfig::COLLAPSED_HEIGHT,
        )
    }

    /// Creates a Material Design large top app bar that can be placed
    /// in a `CustomScrollView`.
    ///
    /// Returns a [`SliverAppBar`] configured with appropriate defaults
    /// for a large top app bar as defined in Material 3. It starts fully
    /// expanded with the title in an area underneath the main row of icons.
    /// When the `CustomScrollView` is scrolled, the title will be scrolled
    /// under the main row. When it is fully collapsed, a smaller version of the
    /// title will fade in on the main row. The reverse will happen if it is
    /// expanded again.
    ///
    /// See also:
    ///
    ///   * [`AppBar`], for a small or center-aligned top app bar.
    ///   * [`SliverAppBar::medium`], for a medium top app bar.
    ///   * <https://m3.material.io/components/top-app-bar/overview>, the Material 3
    ///     app bar specification.
    pub fn large() -> SliverAppBar {
        SliverAppBar::with_variant(
            SliverAppVariant::Large,
            true,
            LargeScrollUnderFlexibleConfig::COLLAPSED_HEIGHT,
        )
    }

    /// The defaults the three Dart constructors share; they differ in `pinned`,
    /// `toolbarHeight` and the variant.
    fn with_variant(variant: SliverAppVariant, pinned: bool, toolbar_height: f64) -> SliverAppBar {
        SliverAppBar {
            key: None,
            leading: None,
            automatically_imply_leading: true,
            title: None,
            actions: None,
            automatically_imply_actions: true,
            flexible_space: None,
            bottom: None,
            elevation: None,
            scrolled_under_elevation: None,
            shadow_color: None,
            surface_tint_color: None,
            force_elevated: false,
            background_color: None,
            foreground_color: None,
            icon_theme: None,
            actions_icon_theme: None,
            primary: true,
            center_title: None,
            exclude_header_semantics: false,
            title_spacing: None,
            collapsed_height: None,
            expanded_height: None,
            floating: false,
            pinned,
            shape: None,
            snap: false,
            stretch: false,
            stretch_trigger_offset: 100.0,
            on_stretch_trigger: None,
            toolbar_height,
            leading_width: None,
            toolbar_text_style: None,
            title_text_style: None,
            system_overlay_style: None,
            force_material_transparency: false,
            use_default_semantics_order: true,
            clip_behavior: None,
            actions_padding: None,
            variant,
        }
    }

    /// Dart `SliverAppBar(key:)`.
    pub fn key(mut self, key: KeyRef) -> SliverAppBar {
        self.key = Some(key);
        self
    }

    /// Dart `SliverAppBar(leading:)`.
    pub fn leading<K>(mut self, leading: impl IntoWidget<K>) -> SliverAppBar {
        self.leading = Some(leading.into_widget());
        self
    }

    /// Dart `SliverAppBar(automaticallyImplyLeading:)`.
    pub fn automatically_imply_leading(
        mut self,
        automatically_imply_leading: bool,
    ) -> SliverAppBar {
        self.automatically_imply_leading = automatically_imply_leading;
        self
    }

    /// Dart `SliverAppBar(title:)`.
    pub fn title<K>(mut self, title: impl IntoWidget<K>) -> SliverAppBar {
        self.title = Some(title.into_widget());
        self
    }

    /// Dart `SliverAppBar(actions:)`.
    pub fn actions(mut self, actions: Vec<WidgetRef>) -> SliverAppBar {
        self.actions = Some(actions);
        self
    }

    /// Dart `SliverAppBar(automaticallyImplyActions:)`.
    pub fn automatically_imply_actions(
        mut self,
        automatically_imply_actions: bool,
    ) -> SliverAppBar {
        self.automatically_imply_actions = automatically_imply_actions;
        self
    }

    /// Dart `SliverAppBar(flexibleSpace:)`.
    pub fn flexible_space<K>(mut self, flexible_space: impl IntoWidget<K>) -> SliverAppBar {
        self.flexible_space = Some(flexible_space.into_widget());
        self
    }

    /// Dart `SliverAppBar(bottom:)`.
    pub fn bottom(mut self, bottom: PreferredSizeWidgetRef) -> SliverAppBar {
        self.bottom = Some(bottom);
        self
    }

    /// Dart `SliverAppBar(elevation:)`.
    pub fn elevation(mut self, elevation: f64) -> SliverAppBar {
        self.elevation = Some(elevation);
        self
    }

    /// Dart `SliverAppBar(scrolledUnderElevation:)`.
    pub fn scrolled_under_elevation(mut self, scrolled_under_elevation: f64) -> SliverAppBar {
        self.scrolled_under_elevation = Some(scrolled_under_elevation);
        self
    }

    /// Dart `SliverAppBar(shadowColor:)`.
    pub fn shadow_color(mut self, shadow_color: Color) -> SliverAppBar {
        self.shadow_color = Some(shadow_color);
        self
    }

    /// Dart `SliverAppBar(surfaceTintColor:)`.
    pub fn surface_tint_color(mut self, surface_tint_color: Color) -> SliverAppBar {
        self.surface_tint_color = Some(surface_tint_color);
        self
    }

    /// Dart `SliverAppBar(forceElevated:)`.
    pub fn force_elevated(mut self, force_elevated: bool) -> SliverAppBar {
        self.force_elevated = force_elevated;
        self
    }

    /// Dart `SliverAppBar(backgroundColor:)`.
    pub fn background_color(mut self, background_color: impl Into<AnyColor>) -> SliverAppBar {
        self.background_color = Some(background_color.into());
        self
    }

    /// Dart `SliverAppBar(foregroundColor:)`.
    pub fn foreground_color(mut self, foreground_color: Color) -> SliverAppBar {
        self.foreground_color = Some(foreground_color);
        self
    }

    /// Dart `SliverAppBar(iconTheme:)`.
    pub fn icon_theme(mut self, icon_theme: IconThemeData) -> SliverAppBar {
        self.icon_theme = Some(icon_theme);
        self
    }

    /// Dart `SliverAppBar(actionsIconTheme:)`.
    pub fn actions_icon_theme(mut self, actions_icon_theme: IconThemeData) -> SliverAppBar {
        self.actions_icon_theme = Some(actions_icon_theme);
        self
    }

    /// Dart `SliverAppBar(primary:)`.
    pub fn primary(mut self, primary: bool) -> SliverAppBar {
        self.primary = primary;
        self
    }

    /// Dart `SliverAppBar(centerTitle:)`.
    pub fn center_title(mut self, center_title: bool) -> SliverAppBar {
        self.center_title = Some(center_title);
        self
    }

    /// Dart `SliverAppBar(excludeHeaderSemantics:)`.
    pub fn exclude_header_semantics(mut self, exclude_header_semantics: bool) -> SliverAppBar {
        self.exclude_header_semantics = exclude_header_semantics;
        self
    }

    /// Dart `SliverAppBar(titleSpacing:)`.
    pub fn title_spacing(mut self, title_spacing: f64) -> SliverAppBar {
        self.title_spacing = Some(title_spacing);
        self
    }

    /// Dart `SliverAppBar(collapsedHeight:)`.
    pub fn collapsed_height(mut self, collapsed_height: f64) -> SliverAppBar {
        self.collapsed_height = Some(collapsed_height);
        self
    }

    /// Dart `SliverAppBar(expandedHeight:)`.
    pub fn expanded_height(mut self, expanded_height: f64) -> SliverAppBar {
        self.expanded_height = Some(expanded_height);
        self
    }

    /// Dart `SliverAppBar(floating:)`.
    pub fn floating(mut self, floating: bool) -> SliverAppBar {
        self.floating = floating;
        self
    }

    /// Dart `SliverAppBar(pinned:)`.
    pub fn pinned(mut self, pinned: bool) -> SliverAppBar {
        self.pinned = pinned;
        self
    }

    /// Dart `SliverAppBar(shape:)`.
    pub fn shape(mut self, shape: impl Into<Box<dyn ShapeBorder>>) -> SliverAppBar {
        self.shape = Some(shape.into());
        self
    }

    /// Dart `SliverAppBar(snap:)`.
    pub fn snap(mut self, snap: bool) -> SliverAppBar {
        self.snap = snap;
        self
    }

    /// Dart `SliverAppBar(stretch:)`.
    pub fn stretch(mut self, stretch: bool) -> SliverAppBar {
        self.stretch = stretch;
        self
    }

    /// Dart `SliverAppBar(stretchTriggerOffset:)`.
    pub fn stretch_trigger_offset(mut self, stretch_trigger_offset: f64) -> SliverAppBar {
        debug_assert!(stretch_trigger_offset > 0.0);
        self.stretch_trigger_offset = stretch_trigger_offset;
        self
    }

    /// Dart `SliverAppBar(onStretchTrigger:)`.
    pub fn on_stretch_trigger(mut self, on_stretch_trigger: AsyncCallback) -> SliverAppBar {
        self.on_stretch_trigger = Some(on_stretch_trigger);
        self
    }

    /// Dart `SliverAppBar(toolbarHeight:)`.
    pub fn toolbar_height(mut self, toolbar_height: f64) -> SliverAppBar {
        self.toolbar_height = toolbar_height;
        self
    }

    /// Dart `SliverAppBar(leadingWidth:)`.
    pub fn leading_width(mut self, leading_width: f64) -> SliverAppBar {
        self.leading_width = Some(leading_width);
        self
    }

    /// Dart `SliverAppBar(toolbarTextStyle:)`.
    pub fn toolbar_text_style(mut self, toolbar_text_style: TextStyle) -> SliverAppBar {
        self.toolbar_text_style = Some(toolbar_text_style);
        self
    }

    /// Dart `SliverAppBar(titleTextStyle:)`.
    pub fn title_text_style(mut self, title_text_style: TextStyle) -> SliverAppBar {
        self.title_text_style = Some(title_text_style);
        self
    }

    /// Dart `SliverAppBar(systemOverlayStyle:)`.
    pub fn system_overlay_style(
        mut self,
        system_overlay_style: SystemUiOverlayStyle,
    ) -> SliverAppBar {
        self.system_overlay_style = Some(system_overlay_style);
        self
    }

    /// Dart `SliverAppBar(forceMaterialTransparency:)`.
    pub fn force_material_transparency(
        mut self,
        force_material_transparency: bool,
    ) -> SliverAppBar {
        self.force_material_transparency = force_material_transparency;
        self
    }

    /// Dart `SliverAppBar(useDefaultSemanticsOrder:)`.
    pub fn use_default_semantics_order(
        mut self,
        use_default_semantics_order: bool,
    ) -> SliverAppBar {
        self.use_default_semantics_order = use_default_semantics_order;
        self
    }

    /// Dart `SliverAppBar(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> SliverAppBar {
        self.clip_behavior = Some(clip_behavior);
        self
    }

    /// Dart `SliverAppBar(actionsPadding:)`.
    pub fn actions_padding(mut self, actions_padding: EdgeInsetsGeometry) -> SliverAppBar {
        self.actions_padding = Some(actions_padding);
        self
    }
}

impl Default for SliverAppBar {
    fn default() -> SliverAppBar {
        SliverAppBar::new()
    }
}

impl Debug for SliverAppBar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SliverAppBar")
            .field("key", &self.key)
            .field("title", &self.title)
            .field("variant", &self.variant)
            .field("floating", &self.floating)
            .field("pinned", &self.pinned)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for SliverAppBar {
    type State = SliverAppBarState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn debug_check_constructor(&self) {
        debug_assert!(
            self.floating || !self.snap,
            "The \"snap\" argument only makes sense for floating app bars."
        );
        debug_assert!(
            self.collapsed_height
                .is_none_or(|collapsed_height| collapsed_height >= self.toolbar_height),
            "The \"collapsedHeight\" argument has to be larger than or equal to [toolbarHeight]."
        );
    }

    fn create_state(&self) -> SliverAppBarState {
        SliverAppBarState {
            state: StateData::new(),
            ticker_provider: TickerProviderStateMixinData::new(),
            snap_configuration: None,
            stretch_configuration: None,
            show_on_screen_configuration: None,
        }
    }
}

// This class is only Stateful because it owns the TickerProvider used
// by the floating appbar snap animation (via FloatingHeaderSnapConfiguration).
/// Dart's `_SliverAppBarState`.
pub struct SliverAppBarState {
    state: StateData<SliverAppBar>,
    ticker_provider: TickerProviderStateMixinData,
    /// Compared by identity, as Dart's delegate compares it.
    snap_configuration: Option<Rc<FloatingHeaderSnapConfiguration>>,
    /// Compared by identity, as Dart's delegate compares it.
    stretch_configuration: Option<Rc<OverScrollHeaderStretchConfiguration>>,
    show_on_screen_configuration: Option<PersistentHeaderShowOnScreenConfiguration>,
}

impl SliverAppBarState {
    fn update_snap_configuration(self: Handle<Self>, app: &mut App) {
        let widget = self.widget(app);
        let snap_configuration = (widget.snap && widget.floating).then(|| {
            Rc::new(
                FloatingHeaderSnapConfiguration::new()
                    .curve(Curves::ease_out())
                    .duration(Duration::from_millis(200)),
            )
        });

        let show_on_screen_configuration = (widget.floating & widget.snap).then(|| {
            PersistentHeaderShowOnScreenConfiguration::new()
                .min_show_on_screen_extent(f64::INFINITY)
        });
        let this = app.get_mut(self);
        this.snap_configuration = snap_configuration;
        this.show_on_screen_configuration = show_on_screen_configuration;
    }

    fn update_stretch_configuration(self: Handle<Self>, app: &mut App) {
        let widget = self.widget(app);
        let stretch_configuration = widget.stretch.then(|| {
            let mut configuration = OverScrollHeaderStretchConfiguration::new()
                .stretch_trigger_offset(widget.stretch_trigger_offset);
            configuration.on_stretch_trigger = widget.on_stretch_trigger.clone();
            Rc::new(configuration)
        });
        app.get_mut(self).stretch_configuration = stretch_configuration;
    }
}

impl TickerProviderStateMixin for SliverAppBarState {
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

impl TickerProviderObject for SliverAppBarState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
        TickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl State for SliverAppBarState {
    type Widget = SliverAppBar;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        self.update_snap_configuration(app);
        self.update_stretch_configuration(app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &Self::Widget) {
        let widget = self.widget(app);
        let snap_changed = widget.snap != old_widget.snap || widget.floating != old_widget.floating;
        let stretch_changed = widget.stretch != old_widget.stretch;
        if snap_changed {
            self.update_snap_configuration(app);
        }
        if stretch_changed {
            self.update_stretch_configuration(app);
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        TickerProviderStateMixin::dispose(self, app);
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        TickerProviderStateMixin::activate(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(
            !self.widget(app).primary || waft_widgets::debug_check_has_media_query(app, context)
        );
        let bottom_height = self
            .widget(app)
            .bottom
            .as_ref()
            .map_or(0.0, |bottom| bottom.preferred_size().height());
        let top_padding = if self.widget(app).primary {
            MediaQuery::padding_of(app, context).top
        } else {
            0.0
        };
        let widget = self.widget(app);
        let collapsed_height = if widget.pinned && widget.floating && widget.bottom.is_some() {
            widget.collapsed_height.unwrap_or(0.0) + bottom_height + top_padding
        } else {
            widget.collapsed_height.unwrap_or(widget.toolbar_height) + bottom_height + top_padding
        };
        let (effective_expanded_height, effective_collapsed_height, effective_flexible_space) =
            match widget.variant {
                SliverAppVariant::Small => (
                    widget.expanded_height,
                    collapsed_height,
                    widget.flexible_space.clone(),
                ),
                SliverAppVariant::Medium => (
                    Some(widget.expanded_height.unwrap_or(
                        MediumScrollUnderFlexibleConfig::EXPANDED_HEIGHT + bottom_height,
                    )),
                    widget.collapsed_height.unwrap_or(
                        top_padding
                            + MediumScrollUnderFlexibleConfig::COLLAPSED_HEIGHT
                            + bottom_height,
                    ),
                    Some(widget.flexible_space.clone().unwrap_or_else(|| {
                        ScrollUnderFlexibleSpace {
                            title: widget.title.clone(),
                            foreground_color: widget.foreground_color,
                            config_builder: |app, context| {
                                Box::new(MediumScrollUnderFlexibleConfig::new(app, context))
                            },
                            title_text_style: widget.title_text_style.clone(),
                            bottom_height,
                        }
                        .into_widget()
                    })),
                ),
                SliverAppVariant::Large => (
                    Some(widget.expanded_height.unwrap_or(
                        LargeScrollUnderFlexibleConfig::EXPANDED_HEIGHT + bottom_height,
                    )),
                    widget.collapsed_height.unwrap_or(
                        top_padding
                            + LargeScrollUnderFlexibleConfig::COLLAPSED_HEIGHT
                            + bottom_height,
                    ),
                    Some(widget.flexible_space.clone().unwrap_or_else(|| {
                        ScrollUnderFlexibleSpace {
                            title: widget.title.clone(),
                            foreground_color: widget.foreground_color,
                            config_builder: |app, context| {
                                Box::new(LargeScrollUnderFlexibleConfig::new(app, context))
                            },
                            title_text_style: widget.title_text_style.clone(),
                            bottom_height,
                        }
                        .into_widget()
                    })),
                ),
            };

        let accessible_navigation = MediaQuery::of(app, context).accessible_navigation;
        let this = app.get(self);
        let snap_configuration = this.snap_configuration.clone();
        let stretch_configuration = this.stretch_configuration.clone();
        let show_on_screen_configuration = this.show_on_screen_configuration;
        let widget = self.widget(app);
        debug_assert!(widget.primary || top_padding == 0.0);
        let delegate = SliverAppBarDelegate {
            vsync: TickerProviderRef::new(self),
            leading: widget.leading.clone(),
            automatically_imply_leading: widget.automatically_imply_leading,
            title: widget.title.clone(),
            actions: widget.actions.clone(),
            automatically_imply_actions: widget.automatically_imply_actions,
            flexible_space: effective_flexible_space,
            bottom: widget.bottom.clone(),
            elevation: widget.elevation,
            scrolled_under_elevation: widget.scrolled_under_elevation,
            shadow_color: widget.shadow_color,
            surface_tint_color: widget.surface_tint_color,
            force_elevated: widget.force_elevated,
            background_color: widget.background_color.clone(),
            foreground_color: widget.foreground_color,
            icon_theme: widget.icon_theme.clone(),
            actions_icon_theme: widget.actions_icon_theme.clone(),
            primary: widget.primary,
            center_title: widget.center_title,
            exclude_header_semantics: widget.exclude_header_semantics,
            title_spacing: widget.title_spacing,
            expanded_height: effective_expanded_height,
            collapsed_height: effective_collapsed_height,
            top_padding,
            floating: widget.floating,
            pinned: widget.pinned,
            shape: widget.shape.clone(),
            snap_configuration,
            stretch_configuration,
            show_on_screen_configuration,
            toolbar_height: Some(widget.toolbar_height),
            leading_width: widget.leading_width,
            toolbar_text_style: widget.toolbar_text_style.clone(),
            title_text_style: widget.title_text_style.clone(),
            system_overlay_style: widget.system_overlay_style,
            force_material_transparency: widget.force_material_transparency,
            use_default_semantics_order: widget.use_default_semantics_order,
            clip_behavior: widget.clip_behavior,
            variant: widget.variant,
            accessible_navigation,
            actions_padding: widget.actions_padding,
            // Dart's delegate constructor fills `_bottomHeight` the same way.
            bottom_height,
        };
        let floating = widget.floating;
        let pinned = widget.pinned;

        MediaQuery::remove_padding(
            app,
            context,
            false,
            false,
            false,
            true,
            SliverPersistentHeader::new(Rc::new(delegate))
                .floating(floating)
                .pinned(pinned),
        )
        .into_widget()
    }
}

impl Debug for SliverAppBarState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SliverAppBarState")
    }
}

// Layout the AppBar's title with unconstrained height, vertically
// center it within its (NavigationToolbar) parent, and allow the
// parent to constrain the title's actual height.
/// Dart's `_AppBarTitleBox`.
#[derive(Debug)]
struct AppBarTitleBox {
    child: WidgetRef,
}

impl RenderObjectWidget for AppBarTitleBox {
    type RenderObject = RenderAppBarTitleBox;

    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        let text_direction = Directionality::of(app, context);
        RenderAppBarTitleBox::new(app, Some(text_direction)).as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        context: BuildContext,
        render_object: RenderHandle<RenderAppBarTitleBox>,
    ) {
        let text_direction = Directionality::of(app, context);
        render_object.set_text_direction(app, Some(text_direction));
    }
}

impl SingleChildRenderObjectWidget for AppBarTitleBox {
    fn child(&self) -> Option<&WidgetRef> {
        Some(&self.child)
    }
}

/// Dart's `_RenderAppBarTitleBox`.
struct RenderAppBarTitleBox {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    aligning: RenderAligningShiftedBoxData,
}

impl RenderAppBarTitleBox {
    fn new(
        app: &mut App,
        text_direction: Option<TextDirection>,
    ) -> RenderHandle<RenderAppBarTitleBox> {
        RenderHandle::new_box(
            app,
            RenderAppBarTitleBox {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                aligning: RenderAligningShiftedBoxData::new(
                    AlignmentGeometry::CENTER,
                    text_direction,
                ),
            },
        )
    }
}

impl RenderObjectWithChildMixin for RenderAppBarTitleBox {
    type ChildType = AnyRenderBox;

    fn child_data(self: RenderHandle<Self>, app: &App) -> &RenderObjectWithChildData<AnyRenderBox> {
        &self.get(app).child
    }

    fn child_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RenderObjectWithChildData<AnyRenderBox> {
        &mut self.get_mut(app).child
    }
}

impl RenderShiftedBox for RenderAppBarTitleBox {}

impl RenderAligningShiftedBox for RenderAppBarTitleBox {
    fn aligning_data(self: RenderHandle<Self>, app: &App) -> &RenderAligningShiftedBoxData {
        &self.get(app).aligning
    }

    fn aligning_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RenderAligningShiftedBoxData {
        &mut self.get_mut(app).aligning
    }
}

impl RenderObject for RenderAppBarTitleBox {
    waft_rendering::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let inner_constraints = constraints.copy_with().max_height(f64::INFINITY);
        let child = self.child(app).expect("the title box has a child");
        child.layout(app, inner_constraints, true);
        let size = constraints.constrain(child.size(app));
        self.set_size(app, size);
        self.align_child(app);
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        RenderShiftedBox::paint(self, app, context, offset);
    }

    fn visit_children(
        self: RenderHandle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(AnyRenderObject),
    ) {
        if let Some(child) = self.child(app) {
            visitor(child.as_object());
        }
    }
}

impl RenderBox for RenderAppBarTitleBox {
    waft_rendering::render_box_accessors!();

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderShiftedBox::compute_min_intrinsic_width(self, app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderShiftedBox::compute_max_intrinsic_width(self, app, height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderShiftedBox::compute_min_intrinsic_height(self, app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderShiftedBox::compute_max_intrinsic_height(self, app, width)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        let inner_constraints = constraints.copy_with().max_height(f64::INFINITY);
        let child = self.child(app).expect("the title box has a child");
        let child_size = child.get_dry_layout(app, inner_constraints);
        constraints.constrain(child_size)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        let inner_constraints = constraints.copy_with().max_height(f64::INFINITY);
        let child = self.child(app)?;
        let result = child.get_dry_baseline(app, inner_constraints, baseline)?;
        let child_size = child.get_dry_layout(app, inner_constraints);
        let size = self.get_dry_layout(app, constraints);
        Some(
            result
                + self
                    .resolved_alignment(app)
                    .along_offset(size - child_size)
                    .dy(),
        )
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderShiftedBox::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderShiftedBox::hit_test_children(self, app, result, position)
    }
}

/// Dart's `_ScrollUnderFlexibleSpace`.
struct ScrollUnderFlexibleSpace {
    title: Option<WidgetRef>,
    foreground_color: Option<Color>,
    config_builder: FlexibleConfigBuilder,
    title_text_style: Option<TextStyle>,
    bottom_height: f64,
}

impl StatelessWidget for ScrollUnderFlexibleSpace {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let settings = context
            .depend_on_inherited_widget_of_exact_type::<FlexibleSpaceBarSettings>(app)
            .expect("a FlexibleSpaceBarSettings above the flexible space")
            .clone();
        let config = (self.config_builder)(app, context);
        debug_assert!(
            config.expanded_title_padding().is_non_negative(),
            "The _ExpandedTitleWithPadding widget assumes that the expanded title padding is \
             non-negative. Update its implementation to handle negative padding."
        );

        let expanded_text_style = match &self.title_text_style {
            Some(title_text_style) => Some(title_text_style.clone()),
            None => {
                let app_bar_theme = AppBarTheme::of(app, context);
                match app_bar_theme.title_text_style {
                    Some(title_text_style) => Some(title_text_style),
                    None => {
                        let foreground_color =
                            match self.foreground_color.or(app_bar_theme.foreground_color) {
                                Some(foreground_color) => foreground_color,
                                None => app_bar_defaults(app, context)
                                    .foreground_color
                                    .expect("the defaults give a foreground color"),
                            };
                        config
                            .expanded_text_style()
                            .map(|style| style.copy_with().color(foreground_color))
                    }
                }
            }
        };

        let expanded_title = match (&self.title, expanded_text_style) {
            (None, _) => None,
            (Some(title), None) => Some(title.clone()),
            (Some(title), Some(text_style)) => {
                Some(DefaultTextStyle::new(text_style, title.clone()).into_widget())
            }
        };

        let resolved_title_padding = config
            .expanded_title_padding()
            .resolve(Some(Directionality::of(app, context)));
        let expanded_title_padding: EdgeInsetsGeometry = if self.bottom_height > 0.0 {
            resolved_title_padding
                .copy_with(None, None, None, Some(0.0))
                .into()
        } else {
            resolved_title_padding.into()
        };

        let mut children = vec![
            Padding::new(EdgeInsetsGeometry::only(
                0.0,
                settings.min_extent - self.bottom_height,
                0.0,
                0.0,
            ))
            .into_widget(),
            Flexible::new(ClipRect::new().child(ExpandedTitleWithPadding {
                padding: expanded_title_padding,
                max_extent: settings.max_extent - settings.min_extent,
                child: expanded_title,
            }))
            .into_widget(),
        ];
        // Reserve space for AppBar.bottom, which is a sibling of this widget,
        // on the parent Stack.
        if self.bottom_height > 0.0 {
            children.push(
                Padding::new(EdgeInsetsGeometry::only(0.0, 0.0, 0.0, self.bottom_height))
                    .into_widget(),
            );
        }

        // Set maximum text scale factor to [_kMaxTitleTextScaleFactor] for the
        // title to keep the visual hierarchy the same even with larger font
        // sizes. To opt out, wrap the [title] widget in a [MediaQuery] widget
        // with a different TextScaler.
        // TODO(tahatesser): Add link to Material spec when available, https://github.com/flutter/flutter/issues/58769.
        MediaQuery::with_clamped_text_scaling(
            None,
            0.0,
            K_MAX_TITLE_TEXT_SCALE_FACTOR,
            // This column will assume the full height of the parent Stack.
            Column::new().children(children),
        )
    }
}

impl Debug for ScrollUnderFlexibleSpace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ScrollUnderFlexibleSpace")
            .field("title", &self.title)
            .field("bottom_height", &self.bottom_height)
            .finish_non_exhaustive()
    }
}

// A widget that bottom-start aligns its child (the expanded title widget), and
// insets the child according to the specified padding.
//
// This widget gives the child an infinite max height constraint, and will also
// attempt to vertically limit the child's bounding box (not including the
// padding) to within the y range [0, maxExtent], to make sure the child is
// visible when the AppBar is fully expanded.
/// Dart's `_ExpandedTitleWithPadding`.
#[derive(Debug)]
struct ExpandedTitleWithPadding {
    padding: EdgeInsetsGeometry,
    max_extent: f64,
    child: Option<WidgetRef>,
}

impl RenderObjectWidget for ExpandedTitleWithPadding {
    type RenderObject = RenderExpandedTitleBox;

    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        let text_direction = Some(Directionality::of(app, context));
        RenderExpandedTitleBox::new(
            app,
            self.padding.resolve(text_direction),
            AlignmentDirectional::BOTTOM_START.resolve(text_direction),
            self.max_extent,
            None,
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        context: BuildContext,
        render_object: RenderHandle<RenderExpandedTitleBox>,
    ) {
        let text_direction = Some(Directionality::of(app, context));
        render_object.set_padding(app, self.padding.resolve(text_direction));
        render_object.set_title_alignment(
            app,
            AlignmentDirectional::BOTTOM_START.resolve(text_direction),
        );
        render_object.set_max_extent(app, self.max_extent);
    }
}

impl SingleChildRenderObjectWidget for ExpandedTitleWithPadding {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

/// Dart's `_RenderExpandedTitleBox`.
struct RenderExpandedTitleBox {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    padding: EdgeInsets,
    title_alignment: Alignment,
    max_extent: f64,
}

impl RenderExpandedTitleBox {
    fn new(
        app: &mut App,
        padding: EdgeInsets,
        title_alignment: Alignment,
        max_extent: f64,
        child: Option<AnyRenderBox>,
    ) -> RenderHandle<RenderExpandedTitleBox> {
        let this = RenderHandle::new_box(
            app,
            RenderExpandedTitleBox {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                padding,
                title_alignment,
                max_extent,
            },
        );
        this.set_child(app, child);
        this
    }

    fn padding(self: RenderHandle<Self>, app: &App) -> EdgeInsets {
        self.get(app).padding
    }

    fn set_padding(self: RenderHandle<Self>, app: &mut App, value: EdgeInsets) {
        if self.get(app).padding == value {
            return;
        }
        debug_assert!(value.is_non_negative());
        self.get_mut(app).padding = value;
        self.mark_needs_layout(app);
    }

    fn title_alignment(self: RenderHandle<Self>, app: &App) -> Alignment {
        self.get(app).title_alignment
    }

    fn set_title_alignment(self: RenderHandle<Self>, app: &mut App, value: Alignment) {
        if self.get(app).title_alignment == value {
            return;
        }
        self.get_mut(app).title_alignment = value;
        self.mark_needs_layout(app);
    }

    fn max_extent(self: RenderHandle<Self>, app: &App) -> f64 {
        self.get(app).max_extent
    }

    fn set_max_extent(self: RenderHandle<Self>, app: &mut App, value: f64) {
        if self.get(app).max_extent == value {
            return;
        }
        self.get_mut(app).max_extent = value;
        self.mark_needs_layout(app);
    }

    fn child_offset_from_size(
        self: RenderHandle<Self>,
        app: &App,
        child_size: Size,
        size: Size,
    ) -> Offset {
        debug_assert!(self.child(app).is_some());
        let padding = self.padding(app);
        let title_alignment = self.title_alignment(app);
        debug_assert!(padding.is_non_negative());
        debug_assert_eq!(title_alignment.y, 1.0);
        // yAdjustment is the minimum additional y offset to shift the child in
        // the visible vertical space when AppBar is fully expanded. The goal is to
        // prevent the expanded title from being clipped when the expanded title
        // widget + the bottom padding is too tall to fit in the flexible space (the
        // top padding is basically ignored since the expanded title is
        // bottom-aligned).
        let y_adjustment = clamp_double(
            child_size.height() + padding.bottom - self.max_extent(app),
            0.0,
            padding.bottom,
        );
        let offset_x = (title_alignment.x + 1.0) / 2.0
            * (size.width() - padding.horizontal() - child_size.width())
            + padding.left;
        let offset_y = size.height() - child_size.height() - padding.bottom + y_adjustment;
        Offset::new(offset_x, offset_y)
    }
}

impl RenderObjectWithChildMixin for RenderExpandedTitleBox {
    type ChildType = AnyRenderBox;

    fn child_data(self: RenderHandle<Self>, app: &App) -> &RenderObjectWithChildData<AnyRenderBox> {
        &self.get(app).child
    }

    fn child_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RenderObjectWithChildData<AnyRenderBox> {
        &mut self.get_mut(app).child
    }
}

impl RenderShiftedBox for RenderExpandedTitleBox {}

impl RenderObject for RenderExpandedTitleBox {
    waft_rendering::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let Some(child) = self.child(app) else {
            self.set_size(app, constraints.smallest());
            return;
        };
        let size = constraints.biggest();
        self.set_size(app, size);
        let padding = self.padding(app);
        child.layout(
            app,
            constraints.width_constraints().deflate(padding.into()),
            true,
        );
        let offset = self.child_offset_from_size(app, child.size(app), size);
        child
            .as_object()
            .parent_data_of_mut::<BoxParentData>(app)
            .offset = offset;
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        RenderShiftedBox::paint(self, app, context, offset);
    }

    fn visit_children(
        self: RenderHandle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(AnyRenderObject),
    ) {
        if let Some(child) = self.child(app) {
            visitor(child.as_object());
        }
    }
}

impl RenderBox for RenderExpandedTitleBox {
    waft_rendering::render_box_accessors!();

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        let padding = self.padding(app);
        match self.child(app) {
            None => 0.0,
            Some(child) => {
                child.get_max_intrinsic_height(app, f64::max(0.0, width - padding.horizontal()))
                    + padding.vertical()
            }
        }
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, _height: f64) -> f64 {
        let padding = self.padding(app);
        match self.child(app) {
            None => 0.0,
            Some(child) => child.get_max_intrinsic_width(app, f64::INFINITY) + padding.horizontal(),
        }
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        let padding = self.padding(app);
        match self.child(app) {
            None => 0.0,
            Some(child) => {
                child.get_min_intrinsic_height(app, f64::max(0.0, width - padding.horizontal()))
                    + padding.vertical()
            }
        }
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, _height: f64) -> f64 {
        let padding = self.padding(app);
        match self.child(app) {
            None => 0.0,
            Some(child) => child.get_min_intrinsic_width(app, f64::INFINITY) + padding.horizontal(),
        }
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        if self.child(app).is_none() {
            Size::ZERO
        } else {
            constraints.biggest()
        }
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        let child = self.child(app)?;
        let child_constraints = constraints
            .width_constraints()
            .deflate(self.padding(app).into());
        let child_baseline = child.get_dry_baseline(app, child_constraints, baseline);
        let child_size = child.get_dry_layout(app, child_constraints);
        let size = self.get_dry_layout(app, constraints);
        let result = BaselineOffset(child_baseline)
            + self.child_offset_from_size(app, child_size, size).dy();
        result.offset()
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderShiftedBox::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderShiftedBox::hit_test_children(self, app, result, position)
    }
}

/// Dart's `_ScrollUnderFlexibleConfig`: what a medium or large [`SliverAppBar`] reads to lay
/// out its expanded title.
pub(crate) trait ScrollUnderFlexibleConfig {
    #[expect(dead_code, reason = "Dart declares it and reads it nowhere either")]
    fn collapsed_text_style(&self) -> Option<TextStyle>;
    fn expanded_text_style(&self) -> Option<TextStyle>;
    fn expanded_title_padding(&self) -> EdgeInsetsGeometry;
}

/// Dart's `theme.useMaterial3 ? _AppBarDefaultsM3(context) : _AppBarDefaultsM2(context)`.
fn app_bar_defaults(app: &mut App, context: BuildContext) -> AppBarThemeData {
    if Theme::of(app, context).use_material3 {
        app_bar_defaults_m3(app, context)
    } else {
        app_bar_defaults_m2(app, context)
    }
}

/// Hand coded defaults based on Material Design 2.
///
/// Dart's `_AppBarDefaultsM2(context)`: the [`AppBarThemeData`] whose fields are the
/// constructor's arguments and the values of that subclass's getters.
fn app_bar_defaults_m2(app: &mut App, context: BuildContext) -> AppBarThemeData {
    let theme = Theme::of(app, context);
    let colors = &theme.color_scheme;
    let dark = colors.brightness == waft_embedder::Brightness::Dark;
    AppBarThemeData::new()
        .elevation(4.0)
        .shadow_color(Color::new(0xFF000000))
        .title_spacing(NavigationToolbar::K_MIDDLE_SPACING)
        .toolbar_height(K_TOOLBAR_HEIGHT)
        .background_color(if dark { colors.surface } else { colors.primary })
        .foreground_color(if dark {
            colors.on_surface
        } else {
            colors.on_primary
        })
        .icon_theme(theme.icon_theme.clone())
        .toolbar_text_style(
            theme
                .text_theme
                .body_medium
                .clone()
                .expect("a theme's text theme has a body medium style"),
        )
        .title_text_style(
            theme
                .text_theme
                .title_large
                .clone()
                .expect("a theme's text theme has a title large style"),
        )
        .actions_padding(EdgeInsets::ZERO.into())
}

#[cfg(test)]
mod tests {
    //! The behaviour cases of `test/app_bar_test.dart`, `test/app_bar_sliver_test.dart`, the
    //! component cases of `test/app_bar_theme_test.dart`, and the `SliverAppBar` stretch cases
    //! of Flutter's former `test/material/sliver_app_bar_test.dart`.

    use std::cell::Cell;

    use waft_embedder::{Brightness, Radius, Rect};
    use waft_foundation::{Listener, Task};
    use waft_painting::{Axis, BorderRadiusGeometry, BorderSide, RoundedRectangleBorder};
    use waft_services::SystemChrome;
    use waft_widgets::{
        AnyElement, Builder, Container, CustomScrollView, GlobalKey, Icon, IconData, ListView,
        MediaQueryData, PreferredSize, ScrollController, ScrollControllerLeaf,
        SingleChildScrollView, SizedBox, SliverToBoxAdapter, Text, WidgetStateColor,
    };

    use super::*;
    use crate::drawer::Drawer;
    use crate::icons::Icons;
    use crate::tab_controller::DefaultTabController;
    use crate::tabs::{Tab, TabBar};
    use crate::test_support::{WidgetTester, material_app, with_main_thread_stack};

    /// `kTextTabBarHeight`: the height of the `TabBar` Dart's sliver cases put in `bottom`.
    const K_TEXT_TAB_BAR_HEIGHT: f64 = 48.0;

    fn tester_on(platform: TargetPlatform) -> WidgetTester {
        let tester = WidgetTester::on(platform);
        tester.install_fonts();
        tester
    }

    fn tester() -> WidgetTester {
        tester_on(TargetPlatform::Android)
    }

    /// `tester.pumpWidget(MaterialApp(theme: theme(app), home: home(app)))`.
    fn pump_app(
        tester: &mut WidgetTester,
        theme: impl FnOnce(&App) -> ThemeData,
        home: impl FnOnce(&mut App) -> WidgetRef,
    ) {
        let root = {
            let mut app = tester.cell.borrow_mut();
            let theme = theme(&app);
            material_app(theme, home(&mut app))
        };
        tester.pump_widget(root);
    }

    fn m3(app: &App) -> ThemeData {
        ThemeData::new(app)
    }

    fn m2(app: &App) -> ThemeData {
        ThemeData::builder().use_material3(false).build(app)
    }

    /// Dart's `Scaffold(appBar: appBar)`.
    fn scaffold_with(app_bar: AppBar) -> Scaffold {
        Scaffold::new().app_bar(PreferredSizeWidgetRef::new(app_bar))
    }

    fn rect_of(tester: &WidgetTester, element: AnyElement) -> Rect {
        tester.global_rect(tester.render_box_of(element))
    }

    fn key_rect(tester: &WidgetTester, key: &GlobalKey) -> Rect {
        tester.global_rect(tester.render_object(key).as_box().expect("a render box"))
    }

    /// Dart's `tester.getRect(find.text(text))`.
    fn text_rect(tester: &WidgetTester, text: &str) -> Rect {
        let element = *tester
            .find_onstage_text(text)
            .first()
            .expect("a Text showing the text");
        rect_of(tester, element)
    }

    /// Dart's `tester.getRect(find.byType(AppBar, skipOffstage: false))`.
    fn app_bar_rect(tester: &WidgetTester) -> Rect {
        let app_bar = *tester.find_by_type::<AppBar>().first().expect("an AppBar");
        rect_of(tester, app_bar)
    }

    /// Dart's `tester.widget<Material>(find.descendant(of: find.byType(AppBar), matching:
    /// find.byType(Material)).first)`.
    fn app_bar_material<R>(tester: &WidgetTester, read: impl FnOnce(&Material) -> R) -> R {
        let app_bar = *tester.find_by_type::<AppBar>().first().expect("an AppBar");
        let material = tester
            .find_by_type::<Material>()
            .into_iter()
            .find(|material| tester.ancestors(*material).contains(&app_bar))
            .expect("the app bar's Material");
        tester.widget(material, read)
    }

    fn app_bar_elevation(tester: &WidgetTester) -> f64 {
        app_bar_material(tester, |material| material.elevation)
    }

    fn app_bar_background_color(tester: &WidgetTester) -> Option<Color> {
        app_bar_material(tester, |material| material.color)
    }

    /// Dart's `find.byIcon(icon)` count.
    fn icon_count(tester: &WidgetTester, icon: &IconData) -> usize {
        tester
            .find_by_type::<Icon>()
            .into_iter()
            .filter(|element| {
                tester.widget(*element, |widget: &Icon| widget.icon.as_ref() == Some(icon))
            })
            .count()
    }

    /// Dart's `_iconStyle(tester, icon)?.color`.
    fn icon_color(tester: &WidgetTester, icon: &IconData) -> Option<Color> {
        tester
            .glyph_style(tester.find_by_icon(icon))
            .color
            .map(|color| color.color())
    }

    fn icon_size(tester: &WidgetTester, icon: &IconData) -> Size {
        rect_of(tester, tester.find_by_icon(icon)).size()
    }

    /// Dart's `SystemChrome.latestStyle`.
    fn latest_style(tester: &WidgetTester) -> Option<SystemUiOverlayStyle> {
        let mut app = tester.cell.borrow_mut();
        let view = app
            .platform()
            .views()
            .into_iter()
            .next()
            .expect("one view")
            .id();
        SystemChrome::latest_style(&mut app, view)
    }

    /// Dart's `tester.widget<NavigationToolbar>(find.byType(NavigationToolbar))`.
    fn navigation_toolbar<R>(
        tester: &WidgetTester,
        read: impl FnOnce(&NavigationToolbar) -> R,
    ) -> R {
        let toolbar = *tester
            .find_by_type::<NavigationToolbar>()
            .first()
            .expect("a NavigationToolbar");
        tester.widget(toolbar, read)
    }

    /// Dart's `Placeholder(key:)`: a `LimitedBox` of 400 that fills what it is given.
    fn placeholder(key: &GlobalKey) -> WidgetRef {
        waft_widgets::LimitedBox::new()
            .max_width(400.0)
            .max_height(400.0)
            .child(SizedBox::expand())
            .key(key.clone().into())
            .into_widget()
    }

    // ---------------------------------------------------------------------------------------
    // app_bar_test.dart

    #[test]
    fn the_title_is_centered_on_ios_and_macos_unless_there_are_two_actions() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(&mut tester, m3, |_app| {
                scaffold_with(AppBar::new().title(Text::new("X"))).into_widget()
            });
            let title = text_rect(&tester, "X");
            assert!(title.center().dx() < 400.0 - title.width() / 2.0);

            for platform in [TargetPlatform::IOS, TargetPlatform::MacOS] {
                let theme = move |app: &App| ThemeData::builder().platform(platform).build(app);
                // Clear the widget tree to avoid animating between platforms.
                tester.pump_widget(Container::new().into_widget());

                pump_app(&mut tester, theme, |_app| {
                    scaffold_with(AppBar::new().title(Text::new("X"))).into_widget()
                });
                let title = text_rect(&tester, "X");
                assert!(
                    title.center().dx() > 400.0 - title.width() / 2.0,
                    "on {platform:?}"
                );
                assert!(
                    title.center().dx() < 400.0 + title.width() / 2.0,
                    "on {platform:?}"
                );

                // One action is still centered.
                pump_app(&mut tester, theme, |_app| {
                    scaffold_with(
                        AppBar::new()
                            .title(Text::new("X"))
                            .actions(vec![Icon::new(Some(Icons::thumb_up())).into_widget()]),
                    )
                    .into_widget()
                });
                let title = text_rect(&tester, "X");
                assert!(
                    title.center().dx() > 400.0 - title.width() / 2.0,
                    "on {platform:?}"
                );
                assert!(
                    title.center().dx() < 400.0 + title.width() / 2.0,
                    "on {platform:?}"
                );

                // Two actions is left aligned again.
                pump_app(&mut tester, theme, |_app| {
                    scaffold_with(AppBar::new().title(Text::new("X")).actions(vec![
                        Icon::new(Some(Icons::thumb_up())).into_widget(),
                        Icon::new(Some(Icons::thumb_up())).into_widget(),
                    ]))
                    .into_widget()
                });
                let title = text_rect(&tester, "X");
                assert!(
                    title.center().dx() < 400.0 - title.width() / 2.0,
                    "on {platform:?}"
                );
            }
        });
    }

    #[test]
    fn center_title_true_centers_the_title_on_android() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(&mut tester, m3, |_app| {
                scaffold_with(AppBar::new().center_title(true).title(Text::new("X"))).into_widget()
            });
            let title = text_rect(&tester, "X");
            assert!(title.center().dx() > 400.0 - title.width() / 2.0);
            assert!(title.center().dx() < 400.0 + title.width() / 2.0);
        });
    }

    fn start_aligned_title_rect(direction: TextDirection, title_spacing: Option<f64>) -> Rect {
        let mut rect = Rect::ZERO;
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            pump_app(&mut tester, m3, |_app| {
                let mut app_bar = AppBar::new().center_title(false).title(placeholder(&key));
                if let Some(title_spacing) = title_spacing {
                    app_bar = app_bar.title_spacing(title_spacing);
                }
                Directionality::new(direction, scaffold_with(app_bar).into_widget()).into_widget()
            });
            rect = key_rect(&tester, &key);
        });
        rect
    }

    #[test]
    fn a_start_aligned_title_starts_16_from_the_edge_left_to_right() {
        let title = start_aligned_title_rect(TextDirection::Ltr, None);
        assert_eq!(title.left, 16.0);
        assert_eq!(title.right, 800.0 - 16.0);
    }

    #[test]
    fn a_start_aligned_title_starts_16_from_the_edge_right_to_left() {
        let title = start_aligned_title_rect(TextDirection::Rtl, None);
        assert_eq!(title.right, 800.0 - 16.0);
        assert_eq!(title.left, 16.0);
    }

    #[test]
    fn a_title_spacing_of_32_starts_the_title_32_from_the_edge_left_to_right() {
        let title = start_aligned_title_rect(TextDirection::Ltr, Some(32.0));
        assert_eq!(title.left, 32.0);
        assert_eq!(title.right, 800.0 - 32.0);
    }

    #[test]
    fn a_title_spacing_of_32_starts_the_title_32_from_the_edge_right_to_left() {
        let title = start_aligned_title_rect(TextDirection::Rtl, Some(32.0));
        assert_eq!(title.right, 800.0 - 32.0);
        assert_eq!(title.left, 32.0);
    }

    fn title_rect_after_drawer_button(direction: TextDirection) -> Rect {
        let mut rect = Rect::ZERO;
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(&mut tester, m3, |_app| {
                Directionality::new(
                    direction,
                    scaffold_with(AppBar::new().center_title(false).title(Text::new("X")))
                        // A drawer causes a leading hamburger.
                        .drawer(Drawer::new())
                        .into_widget(),
                )
                .into_widget()
            });
            rect = text_rect(&tester, "X");
        });
        rect
    }

    #[test]
    fn a_drawer_button_starts_the_title_at_72_left_to_right() {
        assert_eq!(
            title_rect_after_drawer_button(TextDirection::Ltr).left,
            72.0
        );
    }

    #[test]
    fn a_drawer_button_starts_the_title_at_72_right_to_left() {
        assert_eq!(
            title_rect_after_drawer_button(TextDirection::Rtl).right,
            800.0 - 72.0
        );
    }

    #[test]
    fn an_app_bar_builds_without_a_scaffold() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(&mut tester, m3, |_app| {
                SizedBox::new()
                    .height(K_TOOLBAR_HEIGHT)
                    .child(
                        AppBar::new()
                            .leading(Text::new("L"))
                            .title(Text::new("No Scaffold"))
                            .actions(vec![
                                Text::new("A1").into_widget(),
                                Text::new("A2").into_widget(),
                            ]),
                    )
                    .into_widget()
            });
            for text in ["L", "No Scaffold", "A1", "A2"] {
                assert_eq!(tester.find_onstage_text(text).len(), 1, "{text}");
            }
        });
    }

    #[test]
    fn an_app_bar_renders_at_zero_size() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(&mut tester, m3, |_app| {
                waft_widgets::Center::new()
                    .child(
                        SizedBox::shrink()
                            .child(scaffold_with(AppBar::new().title(Text::new("X")))),
                    )
                    .into_widget()
            });
            assert!(text_rect(&tester, "X").size().is_empty());
        });
    }

    #[test]
    fn leading_title_and_actions_are_vertically_centered() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let app_bar_key = GlobalKey::new();
            let leading_key = GlobalKey::new();
            let title_key = GlobalKey::new();
            let action0_key = GlobalKey::new();
            let action1_key = GlobalKey::new();
            pump_app(&mut tester, m3, |_app| {
                let sized = |key: &GlobalKey, height: f64| {
                    SizedBox::new()
                        .height(height)
                        .key(key.clone().into())
                        .into_widget()
                };
                scaffold_with(
                    AppBar::new()
                        .key(app_bar_key.clone().into())
                        .leading(sized(&leading_key, 50.0))
                        .title(sized(&title_key, 40.0))
                        .actions(vec![sized(&action0_key, 20.0), sized(&action1_key, 30.0)]),
                )
                .into_widget()
            });
            // The vertical center of the widget with key, in global coordinates.
            let y_center = |key: &GlobalKey| key_rect(&tester, key).center().dy();
            assert_eq!(y_center(&app_bar_key), y_center(&leading_key));
            assert_eq!(y_center(&app_bar_key), y_center(&title_key));
            assert_eq!(y_center(&app_bar_key), y_center(&action0_key));
            assert_eq!(y_center(&app_bar_key), y_center(&action1_key));
        });
    }

    /// Dart's drawer-icon cases: a `Scaffold` with an app bar titled 'Howdy!' and a drawer at
    /// the start or the end; `read` looks at the menu icon.
    fn drawer_icon_case<R: Send>(
        end: bool,
        theme: fn(&App) -> ThemeData,
        icon_theme: fn() -> Option<IconThemeData>,
        read: impl FnOnce(&WidgetTester) -> R + Send,
    ) -> R {
        let mut result = None;
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(&mut tester, theme, |_app| {
                let mut app_bar = AppBar::new().title(Text::new("Howdy!"));
                if let Some(icon_theme) = icon_theme() {
                    app_bar = app_bar.icon_theme(icon_theme);
                }
                let scaffold = scaffold_with(app_bar);
                if end {
                    scaffold.end_drawer(Drawer::new())
                } else {
                    scaffold.drawer(Drawer::new())
                }
                .into_widget()
            });
            result = Some(read(&tester));
        });
        result.expect("the case ran")
    }

    fn light_scheme_theme(app: &App) -> ThemeData {
        ThemeData::from(crate::color_scheme::ColorScheme::light()).build(app)
    }

    #[test]
    fn the_drawer_icon_has_the_default_size() {
        let size = drawer_icon_case(
            false,
            m3,
            || None,
            |tester| icon_size(tester, &Icons::menu()),
        );
        let icon_size = IconThemeData::fallback().size.expect("a size");
        assert_eq!(size, Size::new(icon_size, icon_size));
    }

    #[test]
    fn material3_the_drawer_icon_has_the_default_color() {
        let (color, expected) = drawer_icon_case(
            false,
            light_scheme_theme,
            || None,
            |tester| {
                let app = tester.cell.borrow();
                let expected = light_scheme_theme(&app).color_scheme.on_surface_variant();
                drop(app);
                (icon_color(tester, &Icons::menu()), expected)
            },
        );
        assert_eq!(color, Some(expected));
    }

    #[test]
    fn the_drawer_icon_is_sized_by_the_icon_theme() {
        let size = drawer_icon_case(
            false,
            m3,
            || Some(IconThemeData::new().size(30.0)),
            |tester| icon_size(tester, &Icons::menu()),
        );
        assert_eq!(size, Size::new(30.0, 30.0));
    }

    #[test]
    fn the_drawer_icon_is_colored_by_the_icon_theme() {
        const COLOR: Color = Color::new(0xFF2196F3);
        let color = drawer_icon_case(
            false,
            light_scheme_theme,
            || Some(IconThemeData::new().color(COLOR)),
            |tester| icon_color(tester, &Icons::menu()),
        );
        assert_eq!(color, Some(COLOR));
    }

    #[test]
    fn the_end_drawer_icon_has_the_default_size() {
        let size = drawer_icon_case(
            true,
            m3,
            || None,
            |tester| icon_size(tester, &Icons::menu()),
        );
        let icon_size = IconThemeData::fallback().size.expect("a size");
        assert_eq!(size, Size::new(icon_size, icon_size));
    }

    #[test]
    fn material3_the_end_drawer_icon_has_the_default_color() {
        let (color, expected) = drawer_icon_case(
            true,
            light_scheme_theme,
            || None,
            |tester| {
                let app = tester.cell.borrow();
                let expected = light_scheme_theme(&app).color_scheme.on_surface_variant();
                drop(app);
                (icon_color(tester, &Icons::menu()), expected)
            },
        );
        assert_eq!(color, Some(expected));
    }

    #[test]
    fn the_end_drawer_icon_is_sized_by_the_icon_theme() {
        let size = drawer_icon_case(
            true,
            m3,
            || Some(IconThemeData::new().size(30.0)),
            |tester| icon_size(tester, &Icons::menu()),
        );
        assert_eq!(size, Size::new(30.0, 30.0));
    }

    #[test]
    fn the_end_drawer_icon_is_colored_by_the_icon_theme() {
        const COLOR: Color = Color::new(0xFF2196F3);
        let color = drawer_icon_case(
            true,
            light_scheme_theme,
            || Some(IconThemeData::new().color(COLOR)),
            |tester| icon_color(tester, &Icons::menu()),
        );
        assert_eq!(color, Some(COLOR));
    }

    #[test]
    fn material3_the_leading_widget_extends_to_the_edge_and_is_square() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(&mut tester, m3, |_app| {
                scaffold_with(
                    AppBar::new()
                        .leading(IconButton::new(
                            Some(Listener::new(|_| {})),
                            Icon::new(Some(Icons::menu())),
                        ))
                        .title(Text::new("X")),
                )
                // Doesn't really matter. Triggers a hamburger regardless.
                .drawer(waft_widgets::Column::new())
                .into_widget()
            });
            // Default IconButton has a size of (48x48).
            let hamburger = rect_of(&tester, tester.find_by_type::<IconButton>()[0]);
            assert_eq!(hamburger.top_left(), Offset::new(4.0, 4.0));
            assert_eq!(hamburger.size(), Size::new(48.0, 48.0));

            let leading_key = GlobalKey::new();
            pump_app(&mut tester, m3, |_app| {
                scaffold_with(
                    AppBar::new()
                        .leading(Container::new().key(leading_key.clone().into()))
                        .title(Text::new("X")),
                )
                .into_widget()
            });
            // Default leading widget has a size of (56x56).
            let leading_box = key_rect(&tester, &leading_key);
            assert_eq!(leading_box.top_left(), Offset::ZERO);
            assert_eq!(leading_box.size(), Size::new(56.0, 56.0));

            // The custom leading widget should still be 56x56 even if its size is smaller.
            let sized_key = GlobalKey::new();
            pump_app(&mut tester, m3, |_app| {
                scaffold_with(
                    AppBar::new()
                        .leading(
                            SizedBox::new()
                                .height(36.0)
                                .width(36.0)
                                .key(sized_key.clone().into()),
                        )
                        .title(Text::new("X")),
                )
                .into_widget()
            });
            let leading = key_rect(&tester, &sized_key);
            assert_eq!(leading.top_left(), Offset::ZERO);
            assert_eq!(leading.size(), Size::new(56.0, 56.0));
        });
    }

    #[test]
    fn material3_an_action_is_4_from_the_edge_and_at_least_48() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(&mut tester, m3, |_app| {
                scaffold_with(AppBar::new().title(Text::new("X")).actions(vec![
                    IconButton::new(None, Icon::new(Some(Icons::share())))
                        .tooltip("Share")
                        .icon_size(20.0)
                        .into_widget(),
                    IconButton::new(None, Icon::new(Some(Icons::add())))
                        .tooltip("Add")
                        .icon_size(60.0)
                        .into_widget(),
                ]))
                .into_widget()
            });
            let add_button = rect_of(
                &tester,
                tester.ancestor_of_type::<IconButton>(tester.find_by_icon(&Icons::add())),
            );
            assert_eq!(add_button.top_right(), Offset::new(800.0, 0.0));
            // It's still the size it was plus the 2 * 8dp padding from IconButton.
            assert_eq!(add_button.size(), Size::new(60.0 + 2.0 * 8.0, 56.0));

            let share_button = rect_of(
                &tester,
                tester.ancestor_of_type::<IconButton>(tester.find_by_icon(&Icons::share())),
            );
            // The 20dp icon is expanded to fill the IconButton's touch target to 48dp.
            assert_eq!(share_button.size(), Size::new(48.0, 48.0));
        });
    }

    fn elevation_case(theme: fn(&App) -> ThemeData, elevation: Option<f64>) -> f64 {
        let mut result = 0.0;
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(&mut tester, theme, |_app| {
                let mut app_bar = AppBar::new().title(Text::new("Title"));
                if let Some(elevation) = elevation {
                    app_bar = app_bar.elevation(elevation);
                }
                scaffold_with(app_bar).into_widget()
            });
            result = app_bar_elevation(&tester);
        });
        result
    }

    #[test]
    fn material3_the_app_bar_uses_the_given_elevation_or_0() {
        // Default elevation should be used for the material.
        assert_eq!(elevation_case(m3, None), 0.0);
        // AppBar should use the specified elevation.
        assert_eq!(elevation_case(m3, Some(8.0)), 8.0);
    }

    #[test]
    fn material2_the_app_bar_uses_the_given_elevation_or_4() {
        assert_eq!(elevation_case(m2, None), 4.0);
        assert_eq!(elevation_case(m2, Some(8.0)), 8.0);
    }

    /// Dart's `ListView.builder(itemCount: 100, itemBuilder: ListTile(title: Text('Item $index')))`
    /// with the tile's height (56) and its text.
    fn hundred_items() -> WidgetRef {
        ListView::builder(|_app, _context, index| {
            Some(
                SizedBox::new()
                    .height(56.0)
                    .child(Text::new(format!("Item {index}")))
                    .into_widget(),
            )
        })
        .item_count(100)
        .into_widget()
    }

    #[test]
    fn the_scrolled_under_elevation_is_used_once_content_scrolls_under() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(&mut tester, m3, |_app| {
                scaffold_with(
                    AppBar::new()
                        .title(Text::new("Title"))
                        .elevation(2.0)
                        .scrolled_under_elevation(10.0),
                )
                .body(hundred_items())
                .into_widget()
            });
            // Starts with the base elevation.
            assert_eq!(app_bar_elevation(&tester), 2.0);

            let start = text_rect(&tester, "Item 2").center();
            tester.fling_from(start, Offset::new(0.0, -600.0), 2000.0, 1);
            tester.pump_and_settle();

            // After scrolling it should be the scrolledUnderElevation.
            assert_eq!(app_bar_elevation(&tester), 10.0);
        });
    }

    #[test]
    fn material3_the_scrolled_under_elevation_follows_a_nested_scroll_view_its_predicate_picks() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(&mut tester, m3, |_app| {
                scaffold_with(
                    AppBar::new()
                        .title(Text::new("Title"))
                        .scrolled_under_elevation(10.0)
                        .notification_predicate(Rc::new(|notification| notification.depth() == 1)),
                )
                .body(
                    // Dart's `ListView.builder(scrollDirection: Axis.horizontal, itemCount: 4)`:
                    // waft-widgets' builder has no `scrollDirection` yet, and four items lay out
                    // the same either way.
                    ListView::new()
                        .scroll_direction(Axis::Horizontal)
                        .children((0..4).map(|_| {
                            SizedBox::new()
                                .height(600.0)
                                .width(800.0)
                                .child(hundred_items())
                                .into_widget()
                        })),
                )
                .into_widget()
            });
            // Starts with the base elevation.
            assert_eq!(app_bar_elevation(&tester), 0.0);

            let start = text_rect(&tester, "Item 2").center();
            tester.fling_from(start, Offset::new(0.0, -600.0), 2000.0, 1);
            tester.pump_and_settle();

            // After scrolling it should be the scrolledUnderElevation.
            assert_eq!(app_bar_elevation(&tester), 10.0);
        });
    }

    /// Dart's `Localizations(.., Directionality(ltr, MediaQuery(data: padding top 100, child:)))`.
    fn top_padded(child: WidgetRef, padding: EdgeInsets) -> WidgetRef {
        waft_widgets::Localizations::new(
            waft_embedder::Locale::new("en").country_code("US"),
            vec![
                crate::material_localizations::DefaultMaterialLocalizations::delegate(),
                waft_widgets::DefaultWidgetsLocalizations::delegate(),
            ],
        )
        .child(Directionality::new(
            TextDirection::Ltr,
            MediaQuery::new(MediaQueryData::new().padding(padding), child).into_widget(),
        ))
        .into_widget()
    }

    fn container_bottom(height: f64) -> PreferredSizeWidgetRef {
        PreferredSizeWidgetRef::new(PreferredSize::new(
            Size::from_height(height),
            Container::new(),
        ))
    }

    #[test]
    fn the_app_bar_height_adds_the_bottom_and_the_top_padding_when_primary() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let top_padding_100 = EdgeInsets::only(0.0, 100.0, 0.0, 0.0);

            tester.pump_widget(top_padded(
                scaffold_with(AppBar::new()).primary(false).into_widget(),
                top_padding_100,
            ));
            assert_eq!(app_bar_rect(&tester).top, 0.0);
            assert_eq!(app_bar_rect(&tester).height(), K_TOOLBAR_HEIGHT);

            tester.pump_widget(top_padded(
                scaffold_with(AppBar::new().title(Text::new("title"))).into_widget(),
                top_padding_100,
            ));
            assert_eq!(app_bar_rect(&tester).top, 0.0);
            assert!(text_rect(&tester, "title").top > 100.0);
            assert_eq!(app_bar_rect(&tester).height(), K_TOOLBAR_HEIGHT + 100.0);

            tester.pump_widget(top_padded(
                scaffold_with(AppBar::new().bottom(container_bottom(200.0)))
                    .primary(false)
                    .into_widget(),
                top_padding_100,
            ));
            assert_eq!(app_bar_rect(&tester).top, 0.0);
            assert_eq!(app_bar_rect(&tester).height(), K_TOOLBAR_HEIGHT + 200.0);

            tester.pump_widget(top_padded(
                scaffold_with(AppBar::new().bottom(container_bottom(200.0))).into_widget(),
                top_padding_100,
            ));
            assert_eq!(app_bar_rect(&tester).top, 0.0);
            assert_eq!(
                app_bar_rect(&tester).height(),
                K_TOOLBAR_HEIGHT + 100.0 + 200.0
            );

            tester.pump_widget(top_padded(
                AppBar::new()
                    .primary(false)
                    .title(Text::new("title"))
                    .into_widget(),
                top_padding_100,
            ));
            assert_eq!(app_bar_rect(&tester).top, 0.0);
            assert!(text_rect(&tester, "title").top < 100.0);
        });
    }

    #[test]
    fn an_app_bar_in_the_body_leaves_out_the_bottom_safe_area_padding() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.pump_widget(top_padded(
                Scaffold::new()
                    .body(
                        waft_widgets::Column::new()
                            .children([AppBar::new().title(Text::new("title")).into_widget()]),
                    )
                    .into_widget(),
                EdgeInsets::symmetric(100.0, 0.0),
            ));
            assert_eq!(app_bar_rect(&tester).top, 0.0);
            assert_eq!(app_bar_rect(&tester).height(), K_TOOLBAR_HEIGHT + 100.0);
        });
    }

    #[test]
    fn the_title_sees_no_padding_from_the_media_query() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let title_built = Rc::new(Cell::new(false));
            let seen_padding = Rc::new(Cell::new(None));
            let built = Rc::clone(&title_built);
            let seen = Rc::clone(&seen_padding);
            tester.pump_widget(top_padded(
                scaffold_with(AppBar::new().title(Builder::new(move |app, context| {
                    built.set(true);
                    seen.set(Some(MediaQuery::padding_of(app, context)));
                    Text::new("heh").into_widget()
                })))
                .into_widget(),
                EdgeInsets::from_ltrb(12.0, 34.0, 56.0, 78.0),
            ));
            assert!(title_built.get());
            assert_eq!(seen_padding.get(), Some(EdgeInsets::ZERO));
        });
    }

    #[test]
    fn the_app_bar_shows_a_menu_button_once_a_drawer_is_added() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(&mut tester, m3, |_app| {
                scaffold_with(AppBar::new()).into_widget()
            });
            assert_eq!(icon_count(&tester, &Icons::menu()), 0);
            pump_app(&mut tester, m3, |_app| {
                scaffold_with(AppBar::new())
                    .drawer(Drawer::new())
                    .into_widget()
            });
            assert_eq!(icon_count(&tester, &Icons::menu()), 1);
        });
    }

    #[test]
    fn no_menu_button_for_the_drawer_without_automatically_imply_leading() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(&mut tester, m3, |_app| {
                scaffold_with(AppBar::new().automatically_imply_leading(false))
                    .drawer(Drawer::new())
                    .into_widget()
            });
            assert_eq!(icon_count(&tester, &Icons::menu()), 0);
        });
    }

    #[test]
    fn no_menu_button_for_the_end_drawer_without_automatically_imply_actions() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(&mut tester, m3, |_app| {
                scaffold_with(AppBar::new().automatically_imply_actions(false))
                    .end_drawer(Drawer::new())
                    .into_widget()
            });
            assert_eq!(icon_count(&tester, &Icons::menu()), 0);
        });
    }

    #[test]
    fn a_menu_button_for_the_end_drawer_when_actions_are_implied() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(&mut tester, m3, |_app| {
                scaffold_with(AppBar::new())
                    .end_drawer(Drawer::new())
                    .into_widget()
            });
            assert_eq!(icon_count(&tester, &Icons::menu()), 1);
        });
    }

    #[test]
    fn given_actions_replace_the_end_drawer_menu_button() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(&mut tester, m3, |_app| {
                scaffold_with(
                    AppBar::new().actions(vec![Icon::new(Some(Icons::settings())).into_widget()]),
                )
                .end_drawer(Drawer::new())
                .into_widget()
            });
            assert_eq!(icon_count(&tester, &Icons::menu()), 0);
            assert_eq!(icon_count(&tester, &Icons::settings()), 1);
        });
    }

    #[test]
    fn the_app_bar_uses_its_toolbar_height() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(&mut tester, m3, |_app| {
                scaffold_with(AppBar::new().title(Text::new("Title")).toolbar_height(48.0))
                    .body(Container::new())
                    .into_widget()
            });
            assert_eq!(app_bar_rect(&tester).height(), 48.0);
        });
    }

    #[test]
    fn the_app_bar_uses_its_leading_width() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            pump_app(&mut tester, m3, |_app| {
                scaffold_with(
                    AppBar::new()
                        .leading(placeholder(&key))
                        .leading_width(100.0)
                        .title(Text::new("Title")),
                )
                .into_widget()
            });
            // By default toolbarHeight is 56.0.
            assert_eq!(
                key_rect(&tester, &key),
                Rect::from_ltrb(0.0, 0.0, 100.0, 56.0)
            );
        });
    }

    #[test]
    fn an_end_drawer_gives_the_app_bar_no_leading() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(&mut tester, m3, |_app| {
                scaffold_with(AppBar::new())
                    .end_drawer(Drawer::new())
                    .into_widget()
            });
            let menu = rect_of(&tester, tester.find_by_icon(&Icons::menu())).center();
            tester.tap(menu, 1);
            tester.pump(Duration::ZERO);

            assert!(navigation_toolbar(&tester, |toolbar| toolbar
                .leading
                .is_none()));
        });
    }

    #[test]
    fn the_title_spacing_defaults_to_the_navigation_toolbars_middle_spacing() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(&mut tester, m3, |_app| {
                scaffold_with(AppBar::new().title(Text::new("Title"))).into_widget()
            });
            assert_eq!(
                navigation_toolbar(&tester, |toolbar| toolbar.middle_spacing),
                NavigationToolbar::K_MIDDLE_SPACING
            );
        });
    }

    #[test]
    fn the_foreground_and_background_colors_reach_the_material_texts_and_icons() {
        with_main_thread_stack(|| {
            const FOREGROUND_COLOR: Color = Color::new(0xff00ff00);
            const BACKGROUND_COLOR: Color = Color::new(0xff00ffff);
            let mut tester = tester();
            let leading_icon_key = GlobalKey::new();
            let action_icon_key = GlobalKey::new();
            pump_app(&mut tester, m3, |_app| {
                scaffold_with(
                    AppBar::new()
                        .foreground_color(FOREGROUND_COLOR)
                        .background_color(BACKGROUND_COLOR)
                        .leading(
                            Icon::new(Some(Icons::add_circle()))
                                .key(leading_icon_key.clone().into()),
                        )
                        .title(Text::new("title"))
                        .actions(vec![
                            Icon::new(Some(Icons::ac_unit()))
                                .key(action_icon_key.clone().into())
                                .into_widget(),
                            Text::new("action").into_widget(),
                        ]),
                )
                .into_widget()
            });

            assert_eq!(app_bar_background_color(&tester), Some(BACKGROUND_COLOR));

            let title = tester.find_onstage_text("title")[0];
            let title_style = tester.ancestor_of_type::<DefaultTextStyle>(title);
            assert_eq!(
                tester.widget(title_style, |style: &DefaultTextStyle| style
                    .style
                    .color
                    .clone()),
                Some(AnyColor::from(FOREGROUND_COLOR))
            );

            let icon_theme_color = |key: &GlobalKey| {
                let element = key
                    .current_context(&tester.cell.borrow_mut())
                    .expect("mounted");
                let theme = tester.ancestor_of_type::<IconTheme>(element);
                tester.widget(theme, |theme: &IconTheme| theme.data.color.clone())
            };
            assert_eq!(
                icon_theme_color(&leading_icon_key),
                Some(AnyColor::from(FOREGROUND_COLOR))
            );
            assert_eq!(
                icon_theme_color(&action_icon_key),
                Some(AnyColor::from(FOREGROUND_COLOR))
            );

            // Test icon color
            assert_eq!(
                icon_color(&tester, &Icons::add_circle()),
                Some(FOREGROUND_COLOR)
            );
            assert_eq!(
                icon_color(&tester, &Icons::ac_unit()),
                Some(FOREGROUND_COLOR)
            );
        });
    }

    fn overlay_style_case(
        theme: fn(&App) -> ThemeData,
        app_bar: fn() -> AppBar,
    ) -> Option<SystemUiOverlayStyle> {
        let mut result = None;
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(&mut tester, theme, |_app| {
                scaffold_with(app_bar()).into_widget()
            });
            result = latest_style(&tester);
        });
        result
    }

    #[test]
    fn material3_a_dark_background_draws_a_light_system_bar() {
        let style = overlay_style_case(
            |app| ThemeData::dark(app, None),
            || AppBar::new().title(Text::new("test")),
        );
        assert_eq!(
            style,
            Some(SystemUiOverlayStyle {
                status_bar_color: Some(Color::new(0x00000000)),
                status_bar_brightness: Some(Brightness::Dark),
                status_bar_icon_brightness: Some(Brightness::Light),
                ..SystemUiOverlayStyle::new()
            })
        );
    }

    #[test]
    fn material3_a_light_background_draws_a_dark_system_bar() {
        let style = overlay_style_case(m3, || AppBar::new().title(Text::new("test")));
        assert_eq!(
            style,
            Some(SystemUiOverlayStyle {
                status_bar_color: Some(Color::new(0x00000000)),
                status_bar_brightness: Some(Brightness::Light),
                status_bar_icon_brightness: Some(Brightness::Dark),
                ..SystemUiOverlayStyle::new()
            })
        );
    }

    #[test]
    fn material2_a_dark_background_draws_a_light_system_bar() {
        let style = overlay_style_case(
            |app| ThemeData::dark(app, Some(false)),
            || AppBar::new().title(Text::new("test")),
        );
        assert_eq!(
            style,
            Some(SystemUiOverlayStyle {
                status_bar_brightness: Some(Brightness::Dark),
                status_bar_icon_brightness: Some(Brightness::Light),
                ..SystemUiOverlayStyle::new()
            })
        );
    }

    #[test]
    fn material2_a_light_background_draws_a_dark_system_bar() {
        let style = overlay_style_case(
            |app| {
                ThemeData::builder()
                    .primary_swatch(crate::colors::Colors::LIGHT_BLUE)
                    .use_material3(false)
                    .build(app)
            },
            || AppBar::new().title(Text::new("test")),
        );
        assert_eq!(
            style,
            Some(SystemUiOverlayStyle {
                status_bar_brightness: Some(Brightness::Light),
                status_bar_icon_brightness: Some(Brightness::Dark),
                ..SystemUiOverlayStyle::new()
            })
        );
    }

    #[test]
    fn material2_the_default_status_bar_color_is_none() {
        let style = overlay_style_case(
            |app| {
                ThemeData::builder()
                    .use_material3(false)
                    .app_bar_theme(AppBarThemeData::new())
                    .build(app)
            },
            || AppBar::new().title(Text::new("title")),
        );
        assert_eq!(style.expect("a style").status_bar_color, None);
    }

    #[test]
    fn the_system_overlay_style_styles_the_status_bar_and_the_navigation_bar() {
        let style = overlay_style_case(m3, || {
            AppBar::new()
                .title(Text::new("test"))
                .system_overlay_style(SystemUiOverlayStyle {
                    status_bar_color: Some(crate::colors::Colors::RED.primary()),
                    system_navigation_bar_color: Some(crate::colors::Colors::GREEN.primary()),
                    ..SystemUiOverlayStyle::LIGHT
                })
        })
        .expect("a style");
        assert_eq!(
            style.status_bar_color,
            Some(crate::colors::Colors::RED.primary())
        );
        assert_eq!(
            style.system_navigation_bar_color,
            Some(crate::colors::Colors::GREEN.primary())
        );
    }

    #[test]
    fn the_app_bar_has_no_shape_by_default() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(&mut tester, m3, |_app| {
                AppBar::new()
                    .leading(Text::new("L"))
                    .title(Text::new("No Scaffold"))
                    .actions(vec![
                        Text::new("A1").into_widget(),
                        Text::new("A2").into_widget(),
                    ])
                    .into_widget()
            });
            let app_bar = tester.find_by_type::<AppBar>()[0];
            assert!(tester.widget(app_bar, |app_bar: &AppBar| app_bar.shape.is_none()));
            assert!(app_bar_material(&tester, |material| material
                .shape
                .is_none()));
        });
    }

    #[test]
    fn the_app_bar_gives_its_shape_to_its_material() {
        with_main_thread_stack(|| {
            let rounded_rectangle_border = || {
                RoundedRectangleBorder::new(
                    BorderSide::NONE,
                    BorderRadiusGeometry::all(Radius::circular(15.0)),
                )
            };
            let mut tester = tester();
            pump_app(&mut tester, m3, |_app| {
                AppBar::new()
                    .leading(Text::new("L"))
                    .title(Text::new("No Scaffold"))
                    .actions(vec![
                        Text::new("A1").into_widget(),
                        Text::new("A2").into_widget(),
                    ])
                    .shape(rounded_rectangle_border())
                    .into_widget()
            });
            let expected: Box<dyn ShapeBorder> = Box::new(rounded_rectangle_border());
            let app_bar = tester.find_by_type::<AppBar>()[0];
            assert!(
                tester.widget(app_bar, |app_bar: &AppBar| app_bar.shape.as_deref()
                    == Some(&*expected))
            );
            assert!(app_bar_material(&tester, |material| material
                .shape
                .as_deref()
                == Some(&*expected)));
        });
    }

    #[test]
    fn preferred_height_for_reads_the_themes_toolbar_height_when_the_app_bar_gives_none() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let preferred_height = Rc::new(Cell::new(0.0));
            let preferred_size = Rc::new(Cell::new(Size::ZERO));
            let frame = |tester: &mut WidgetTester,
                         theme_toolbar_height: Option<f64>,
                         app_bar_toolbar_height: Option<f64>| {
                let preferred_height = Rc::clone(&preferred_height);
                let preferred_size = Rc::clone(&preferred_size);
                pump_app(
                    tester,
                    |app| {
                        let mut theme = AppBarThemeData::new();
                        theme.toolbar_height = theme_toolbar_height;
                        ThemeData::builder().app_bar_theme(theme).build(app)
                    },
                    move |_app| {
                        Builder::new(move |app, context| {
                            let mut app_bar = AppBar::new();
                            app_bar.toolbar_height = app_bar_toolbar_height;
                            let app_bar = PreferredSizeWidgetRef::new(app_bar);
                            preferred_height.set(AppBar::preferred_height_for(
                                app,
                                context,
                                &app_bar.preferred_size(),
                            ));
                            preferred_size.set(app_bar.preferred_size().size());
                            Scaffold::new()
                                .app_bar(app_bar)
                                .body(SizedBox::expand())
                                .into_widget()
                        })
                        .into_widget()
                    },
                );
                tester.pump_and_settle();
            };

            frame(&mut tester, None, None);
            assert_eq!(app_bar_rect(&tester).height(), K_TOOLBAR_HEIGHT);
            assert_eq!(preferred_height.get(), K_TOOLBAR_HEIGHT);
            assert_eq!(preferred_size.get().height(), K_TOOLBAR_HEIGHT);

            frame(&mut tester, Some(96.0), None);
            assert_eq!(app_bar_rect(&tester).height(), 96.0);
            assert_eq!(preferred_height.get(), 96.0);
            // Special case: AppBarTheme.toolbarHeight specified,
            // AppBar.theme.toolbarHeight is null.
            assert_eq!(preferred_size.get().height(), K_TOOLBAR_HEIGHT);

            frame(&mut tester, None, Some(64.0));
            assert_eq!(app_bar_rect(&tester).height(), 64.0);
            assert_eq!(preferred_height.get(), 64.0);
            assert_eq!(preferred_size.get().height(), 64.0);

            frame(&mut tester, Some(96.0), Some(64.0));
            assert_eq!(app_bar_rect(&tester).height(), 64.0);
            assert_eq!(preferred_height.get(), 64.0);
            assert_eq!(preferred_size.get().height(), 64.0);
        });
    }

    #[test]
    fn a_preferred_size_that_reuses_an_app_bars_size_still_reads_the_themes_toolbar_height() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let preferred_height = Rc::new(Cell::new(0.0));
            let reused_height = Rc::clone(&preferred_height);
            pump_app(
                &mut tester,
                |app| {
                    let mut theme = AppBarThemeData::new();
                    theme.toolbar_height = Some(96.0);
                    ThemeData::builder().app_bar_theme(theme).build(app)
                },
                move |_app| {
                    Builder::new(move |app, context| {
                        let reused =
                            PreferredSize::new(AppBar::new().preferred_size(), SizedBox::shrink());
                        reused_height.set(AppBar::preferred_height_for(
                            app,
                            context,
                            &reused.preferred_size(),
                        ));
                        SizedBox::expand().into_widget()
                    })
                    .into_widget()
                },
            );
            assert_eq!(preferred_height.get(), 96.0);
        });
    }

    #[test]
    fn the_actions_padding_insets_the_actions_from_the_end() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let app_bar_key = GlobalKey::new();
            let action_key = GlobalKey::new();
            let frame = |tester: &mut WidgetTester, actions_padding: Option<EdgeInsets>| {
                pump_app(tester, m3, |_app| {
                    let mut app_bar = AppBar::new().key(app_bar_key.clone().into()).actions(vec![
                        SizedBox::square(Some(40.0))
                            .key(action_key.clone().into())
                            .into_widget(),
                    ]);
                    if let Some(actions_padding) = actions_padding {
                        app_bar = app_bar.actions_padding(actions_padding.into());
                    }
                    scaffold_with(app_bar).into_widget()
                });
            };

            frame(&mut tester, None);
            // Actions padding default to zero padding.
            let app_bar_right = key_rect(&tester, &app_bar_key).right;
            assert_eq!(app_bar_right - key_rect(&tester, &action_key).right, 0.0);

            let actions_padding = EdgeInsets::only(0.0, 0.0, 8.0, 0.0);
            frame(&mut tester, Some(actions_padding));
            assert_eq!(
                key_rect(&tester, &action_key).right,
                app_bar_right - actions_padding.right
            );
        });
    }

    /// Dart's `WidgetStateColor scrolledUnder` group's `buildAppBar`.
    fn scrolled_under_app(
        content_height: f64,
        reverse: bool,
        include_flexible_space: bool,
    ) -> impl FnOnce(&mut App) -> WidgetRef {
        move |_app| {
            let mut app_bar = AppBar::new()
                .elevation(0.0)
                .background_color(scrolled_under_background())
                .title(Text::new("AppBar"));
            if include_flexible_space {
                app_bar = app_bar
                    .flexible_space(FlexibleSpaceBar::new().title(Text::new("FlexibleSpace")));
            }
            scaffold_with(app_bar)
                .body(
                    ListView::new().reverse(reverse).children([Container::new()
                        .height(content_height)
                        .color(crate::colors::Colors::TEAL.primary())
                        .into_widget()]),
                )
                .into_widget()
        }
    }

    const SCROLLED_COLOR: Color = Color::new(0xff00ff00);
    const DEFAULT_COLOR: Color = Color::new(0xff0000ff);

    fn scrolled_under_background() -> AnyColor {
        WidgetStateColor::resolve_with(|states| {
            if states.contains(&WidgetState::ScrolledUnder) {
                SCROLLED_COLOR
            } else {
                DEFAULT_COLOR
            }
        })
        .into_any()
    }

    /// Dart's `gesture.moveBy(by)`.
    fn move_by(
        tester: &mut WidgetTester,
        gesture: &mut crate::test_support::TestPointer,
        by: Offset,
    ) {
        let at = gesture.position + by;
        tester.move_to(gesture, at, true);
    }

    /// Dart's `startGesture(at)`, `moveBy(by)`, `up()`, then `pumpAndSettle`.
    fn drag(tester: &mut WidgetTester, at: Offset, by: Offset, pointer: i64) {
        let mut gesture = WidgetTester::touch(pointer);
        tester.down(&mut gesture, at);
        tester.move_to(&mut gesture, at + by, true);
        tester.up(&gesture);
        tester.pump_and_settle();
    }

    fn app_bar_material_height(tester: &WidgetTester) -> f64 {
        let app_bar = tester.find_by_type::<AppBar>()[0];
        let material = tester
            .find_by_type::<Material>()
            .into_iter()
            .find(|material| tester.ancestors(*material).contains(&app_bar))
            .expect("the app bar's Material");
        rect_of(tester, material).height()
    }

    /// Dart's 'backgroundColor' and 'backgroundColor with FlexibleSpace'.
    fn scrolled_under_background_case(include_flexible_space: bool) {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(
                &mut tester,
                m3,
                scrolled_under_app(1200.0, false, include_flexible_space),
            );

            assert_eq!(app_bar_background_color(&tester), Some(DEFAULT_COLOR));
            assert_eq!(app_bar_material_height(&tester), K_TOOLBAR_HEIGHT);

            drag(
                &mut tester,
                Offset::new(50.0, 400.0),
                Offset::new(0.0, -K_TOOLBAR_HEIGHT),
                1,
            );
            assert_eq!(app_bar_background_color(&tester), Some(SCROLLED_COLOR));
            assert_eq!(app_bar_material_height(&tester), K_TOOLBAR_HEIGHT);

            drag(
                &mut tester,
                Offset::new(50.0, 300.0),
                Offset::new(0.0, K_TOOLBAR_HEIGHT),
                2,
            );
            assert_eq!(app_bar_background_color(&tester), Some(DEFAULT_COLOR));
            assert_eq!(app_bar_material_height(&tester), K_TOOLBAR_HEIGHT);
        });
    }

    #[test]
    fn a_widget_state_background_color_resolves_against_scrolled_under() {
        scrolled_under_background_case(false);
    }

    #[test]
    fn a_widget_state_background_color_resolves_against_scrolled_under_with_a_flexible_space() {
        scrolled_under_background_case(true);
    }

    /// Dart's 'backgroundColor - reverse' and 'backgroundColor with FlexibleSpace - reverse'.
    fn reversed_scrolled_under_background_case(include_flexible_space: bool) {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(
                &mut tester,
                m3,
                scrolled_under_app(1200.0, true, include_flexible_space),
            );
            tester.pump(Duration::ZERO);

            // In this test case, the content always extends under the AppBar, so it
            // should always be the scrolledColor.
            assert_eq!(app_bar_background_color(&tester), Some(SCROLLED_COLOR));
            assert_eq!(app_bar_material_height(&tester), K_TOOLBAR_HEIGHT);

            drag(
                &mut tester,
                Offset::new(50.0, 400.0),
                Offset::new(0.0, K_TOOLBAR_HEIGHT),
                1,
            );
            assert_eq!(app_bar_background_color(&tester), Some(SCROLLED_COLOR));
            assert_eq!(app_bar_material_height(&tester), K_TOOLBAR_HEIGHT);

            drag(
                &mut tester,
                Offset::new(50.0, 300.0),
                Offset::new(0.0, -K_TOOLBAR_HEIGHT),
                2,
            );
            assert_eq!(app_bar_background_color(&tester), Some(SCROLLED_COLOR));
            assert_eq!(app_bar_material_height(&tester), K_TOOLBAR_HEIGHT);
        });
    }

    #[test]
    fn a_widget_state_background_color_stays_scrolled_under_in_a_long_reversed_list() {
        reversed_scrolled_under_background_case(false);
    }

    #[test]
    fn a_widget_state_background_color_stays_scrolled_under_in_a_long_reversed_list_with_a_flexible_space()
     {
        reversed_scrolled_under_background_case(true);
    }

    /// Dart's 'backgroundColor - not triggered in reverse for short content' and its
    /// flexible-space twin.
    fn short_reversed_scrolled_under_background_case(include_flexible_space: bool) {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(
                &mut tester,
                m3,
                scrolled_under_app(200.0, true, include_flexible_space),
            );
            tester.pump(Duration::ZERO);

            // In reverse, the content here is not long enough to scroll under the app
            // bar.
            assert_eq!(app_bar_background_color(&tester), Some(DEFAULT_COLOR));
            assert_eq!(app_bar_material_height(&tester), K_TOOLBAR_HEIGHT);

            drag(
                &mut tester,
                Offset::new(50.0, 400.0),
                Offset::new(0.0, K_TOOLBAR_HEIGHT),
                1,
            );
            assert_eq!(app_bar_background_color(&tester), Some(DEFAULT_COLOR));
            assert_eq!(app_bar_material_height(&tester), K_TOOLBAR_HEIGHT);
        });
    }

    #[test]
    fn a_widget_state_background_color_is_not_scrolled_under_in_reverse_for_short_content() {
        short_reversed_scrolled_under_background_case(false);
    }

    #[test]
    fn a_widget_state_background_color_is_not_scrolled_under_in_reverse_for_short_content_with_a_flexible_space()
     {
        short_reversed_scrolled_under_background_case(true);
    }

    #[test]
    fn horizontal_scrolling_does_not_change_the_scrolled_under_background_color() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(&mut tester, m3, |_app| {
                scaffold_with(
                    AppBar::new()
                        .elevation(0.0)
                        .background_color(scrolled_under_background())
                        .title(Text::new("AppBar"))
                        .notification_predicate(Rc::new(|notification| {
                            // Represents both scroll views below being treated as a
                            // single viewport.
                            notification.depth() <= 1
                        })),
                )
                .body(
                    SingleChildScrollView::new().child(
                        SingleChildScrollView::new()
                            .scroll_direction(Axis::Horizontal)
                            .child(
                                Container::new()
                                    .height(1200.0)
                                    .width(1200.0)
                                    .color(crate::colors::Colors::TEAL.primary()),
                            ),
                    ),
                )
                .into_widget()
            });
            assert_eq!(app_bar_background_color(&tester), Some(DEFAULT_COLOR));
            assert_eq!(app_bar_material_height(&tester), K_TOOLBAR_HEIGHT);

            let mut gesture = WidgetTester::touch(1);
            tester.down(&mut gesture, Offset::new(50.0, 400.0));
            tester.move_to(
                &mut gesture,
                Offset::new(50.0, 400.0 - K_TOOLBAR_HEIGHT),
                true,
            );
            tester.pump(Duration::ZERO);
            tester.move_to(
                &mut gesture,
                Offset::new(50.0, 400.0 - 2.0 * K_TOOLBAR_HEIGHT),
                true,
            );
            tester.up(&gesture);
            tester.pump_and_settle();
            assert_eq!(app_bar_background_color(&tester), Some(SCROLLED_COLOR));
            assert_eq!(app_bar_material_height(&tester), K_TOOLBAR_HEIGHT);

            let mut gesture = WidgetTester::touch(2);
            tester.down(&mut gesture, Offset::new(50.0, 300.0));
            // Scroll horizontally
            tester.move_to(
                &mut gesture,
                Offset::new(50.0 - K_TOOLBAR_HEIGHT, 300.0),
                true,
            );
            tester.pump(Duration::ZERO);
            tester.move_to(
                &mut gesture,
                Offset::new(50.0 - 2.0 * K_TOOLBAR_HEIGHT, 300.0),
                true,
            );
            tester.up(&gesture);
            tester.pump_and_settle();
            // The app bar is still scrolled under vertically, so it should not have
            // changed back in response to horizontal scrolling.
            assert_eq!(app_bar_background_color(&tester), Some(SCROLLED_COLOR));
            assert_eq!(app_bar_material_height(&tester), K_TOOLBAR_HEIGHT);
        });
    }

    #[test]
    fn a_toolbar_opacity_fades_the_title_and_the_icons() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(&mut tester, m3, |_app| {
                scaffold_with(
                    AppBar::new()
                        .title(Text::new("title"))
                        .icon_theme(IconThemeData::new().color(Color::new(0xFF000000)))
                        .actions(vec![Icon::new(Some(Icons::add())).into_widget()])
                        .toolbar_opacity(0.5),
                )
                .into_widget()
            });
            let opacity = Interval::new(0.25, 1.0, Curves::fast_out_slow_in()).transform(0.5);
            let title = tester.find_onstage_text("title")[0];
            let title_style = tester.ancestor_of_type::<DefaultTextStyle>(title);
            let title_alpha = tester.widget(title_style, |style: &DefaultTextStyle| {
                style.style.color.as_ref().expect("a color").a
            });
            let app = tester.cell.borrow();
            let on_surface = ThemeData::new(&app).color_scheme.on_surface;
            drop(app);
            #[expect(
                deprecated,
                reason = "the app bar fades with the deprecated `withOpacity`"
            )]
            let expected_alpha = on_surface.with_opacity(opacity).a;
            assert_eq!(title_alpha, expected_alpha);

            let add = tester.find_by_icon(&Icons::add());
            let icon_theme = tester.ancestor_of_type::<IconTheme>(add);
            let icon_opacity = tester.widget(icon_theme, |theme: &IconTheme| theme.data.opacity);
            assert_eq!(icon_opacity, Some(opacity));
        });
    }

    #[test]
    fn a_bottom_opacity_fades_the_bottom() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(&mut tester, m3, |_app| {
                scaffold_with(
                    AppBar::new()
                        .bottom(container_bottom(48.0))
                        .bottom_opacity(0.5),
                )
                .into_widget()
            });
            let opacity = tester.find_by_type::<Opacity>();
            assert_eq!(opacity.len(), 1);
            assert_eq!(
                tester.widget(opacity[0], |opacity: &Opacity| opacity.opacity),
                Interval::new(0.25, 1.0, Curves::fast_out_slow_in()).transform(0.5)
            );
        });
    }

    // ---------------------------------------------------------------------------------------
    // app_bar_sliver_test.dart

    /// What Dart's `buildSliverAppBarApp` takes.
    #[derive(Clone, Copy)]
    struct SliverAppBarApp {
        floating: bool,
        pinned: bool,
        collapsed_height: Option<f64>,
        expanded_height: Option<f64>,
        snap: bool,
        toolbar_height: f64,
    }

    impl Default for SliverAppBarApp {
        fn default() -> SliverAppBarApp {
            SliverAppBarApp {
                floating: false,
                pinned: false,
                collapsed_height: None,
                expanded_height: None,
                snap: false,
                toolbar_height: K_TOOLBAR_HEIGHT,
            }
        }
    }

    /// The scroll view's controller and the key of the `TabBar`, which Dart's cases measure.
    struct SliverAppBarAppHandles {
        controller: Handle<ScrollController>,
        tab_bar_key: GlobalKey,
    }

    /// Dart's `buildSliverAppBarApp`, with the scroll view's own controller for its primary
    /// one.
    fn pump_sliver_app_bar_app(
        tester: &mut WidgetTester,
        arguments: SliverAppBarApp,
    ) -> SliverAppBarAppHandles {
        let controller = ScrollController::default(&mut tester.cell.borrow_mut());
        let tab_bar_key = GlobalKey::new();
        let bottom_key = tab_bar_key.clone();
        pump_app(tester, m3, move |_app| {
            let mut sliver_app_bar = SliverAppBar::new()
                .title(Text::new("AppBar Title"))
                .floating(arguments.floating)
                .pinned(arguments.pinned)
                .toolbar_height(arguments.toolbar_height)
                .snap(arguments.snap)
                .bottom(PreferredSizeWidgetRef::new(
                    TabBar::new(
                        ["A", "B", "C"].map(|t| Tab::new().text(format!("TAB {t}")).into_widget()),
                    )
                    .key(bottom_key.into()),
                ));
            sliver_app_bar.collapsed_height = arguments.collapsed_height;
            sliver_app_bar.expanded_height = arguments.expanded_height;
            Scaffold::new()
                .body(DefaultTabController::new(
                    3,
                    CustomScrollView::new()
                        .controller(controller.as_controller())
                        .slivers([
                            sliver_app_bar.into_widget(),
                            SliverToBoxAdapter::new()
                                .child(
                                    Container::new()
                                        .height(1200.0)
                                        .color(crate::colors::Colors::ORANGE.shade400()),
                                )
                                .into_widget(),
                        ]),
                ))
                .into_widget()
        });
        SliverAppBarAppHandles {
            controller,
            tab_bar_key,
        }
    }

    fn jump_to(tester: &mut WidgetTester, controller: Handle<ScrollController>, value: f64) {
        controller.jump_to(&mut tester.cell.borrow_mut(), value);
        tester.pump(Duration::ZERO);
    }

    fn scroll_offset(tester: &WidgetTester, controller: Handle<ScrollController>) -> f64 {
        controller.as_controller().offset(&tester.cell.borrow())
    }

    /// The geometry of the `SliverAppBar`'s sliver.
    fn sliver_app_bar_geometry(tester: &WidgetTester) -> waft_rendering::SliverGeometry {
        let sliver_app_bar = tester.find_by_type::<SliverAppBar>()[0];
        let app = tester.cell.borrow();
        sliver_app_bar
            .find_render_object(&app)
            .and_then(|render| render.as_sliver())
            .expect("a SliverAppBar renders a sliver")
            .geometry(&app)
    }

    /// Dart's `find.byType(SliverAppBar)` with its default `skipOffstage`: a viewport's
    /// element visits only the slivers whose geometry is visible.
    fn sliver_app_bar_is_onstage(tester: &WidgetTester) -> bool {
        sliver_app_bar_geometry(tester).visible
    }

    fn tab_bar_height(tester: &WidgetTester, handles: &SliverAppBarAppHandles) -> f64 {
        key_rect(tester, &handles.tab_bar_key).height()
    }

    #[test]
    fn the_default_sliver_app_bar_scrolls_out_of_view_at_its_height() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let handles = pump_sliver_app_bar_app(&mut tester, SliverAppBarApp::default());
            assert_eq!(scroll_offset(&tester, handles.controller), 0.0);
            assert!(sliver_app_bar_is_onstage(&tester));

            let initial_app_bar_height = app_bar_rect(&tester).height();
            let initial_tab_bar_height = tab_bar_height(&tester, &handles);

            // Scroll the not-pinned appbar partially out of view
            jump_to(&mut tester, handles.controller, 50.0);
            assert!(sliver_app_bar_is_onstage(&tester));
            assert_eq!(app_bar_rect(&tester).height(), initial_app_bar_height);
            assert_eq!(tab_bar_height(&tester, &handles), initial_tab_bar_height);

            // Scroll the not-pinned appbar out of view
            jump_to(&mut tester, handles.controller, 600.0);
            assert!(!sliver_app_bar_is_onstage(&tester));
            assert_eq!(app_bar_rect(&tester).height(), initial_app_bar_height);
            assert_eq!(tab_bar_height(&tester, &handles), initial_tab_bar_height);

            // Scroll the not-pinned appbar back into view
            jump_to(&mut tester, handles.controller, 0.0);
            assert!(sliver_app_bar_is_onstage(&tester));
            assert_eq!(app_bar_rect(&tester).height(), initial_app_bar_height);
            assert_eq!(tab_bar_height(&tester, &handles), initial_tab_bar_height);
        });
    }

    #[test]
    fn a_pinned_sliver_app_bar_collapses_its_expanded_height_but_keeps_the_toolbar() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let handles = pump_sliver_app_bar_app(
                &mut tester,
                SliverAppBarApp {
                    pinned: true,
                    expanded_height: Some(128.0),
                    ..SliverAppBarApp::default()
                },
            );
            assert_eq!(scroll_offset(&tester, handles.controller), 0.0);
            assert!(sliver_app_bar_is_onstage(&tester));
            assert_eq!(app_bar_rect(&tester).height(), 128.0);

            const INITIAL_APP_BAR_HEIGHT: f64 = 128.0;
            let initial_tab_bar_height = tab_bar_height(&tester, &handles);

            // Scroll the not-pinned appbar, collapsing the expanded height. At this
            // point both the toolbar and the tabbar are visible.
            jump_to(&mut tester, handles.controller, 600.0);
            assert!(sliver_app_bar_is_onstage(&tester));
            assert_eq!(tab_bar_height(&tester, &handles), initial_tab_bar_height);
            assert!(app_bar_rect(&tester).height() < INITIAL_APP_BAR_HEIGHT);
            assert!(app_bar_rect(&tester).height() > initial_tab_bar_height);

            // Scroll the not-pinned appbar back into view
            jump_to(&mut tester, handles.controller, 0.0);
            assert!(sliver_app_bar_is_onstage(&tester));
            assert_eq!(app_bar_rect(&tester).height(), INITIAL_APP_BAR_HEIGHT);
            assert_eq!(tab_bar_height(&tester, &handles), initial_tab_bar_height);
        });
    }

    #[test]
    fn a_pinned_and_floating_sliver_app_bar_collapses_to_its_bottom() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let handles = pump_sliver_app_bar_app(
                &mut tester,
                SliverAppBarApp {
                    floating: true,
                    pinned: true,
                    expanded_height: Some(128.0),
                    ..SliverAppBarApp::default()
                },
            );
            assert_eq!(scroll_offset(&tester, handles.controller), 0.0);
            assert!(sliver_app_bar_is_onstage(&tester));
            assert_eq!(app_bar_rect(&tester).height(), 128.0);

            const INITIAL_APP_BAR_HEIGHT: f64 = 128.0;
            let initial_tab_bar_height = tab_bar_height(&tester, &handles);

            // Scroll the floating-pinned appbar, collapsing the expanded height. At this
            // point only the tabBar is visible.
            jump_to(&mut tester, handles.controller, 600.0);
            assert!(sliver_app_bar_is_onstage(&tester));
            assert_eq!(tab_bar_height(&tester, &handles), initial_tab_bar_height);
            assert!(app_bar_rect(&tester).height() < INITIAL_APP_BAR_HEIGHT);
            assert_eq!(app_bar_rect(&tester).height(), initial_tab_bar_height);

            // Scroll the floating-pinned appbar back into view
            jump_to(&mut tester, handles.controller, 0.0);
            assert!(sliver_app_bar_is_onstage(&tester));
            assert_eq!(app_bar_rect(&tester).height(), INITIAL_APP_BAR_HEIGHT);
            assert_eq!(tab_bar_height(&tester, &handles), initial_tab_bar_height);
        });
    }

    #[test]
    fn a_floating_sliver_app_bar_with_snap_snaps_open_and_closed() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let handles = pump_sliver_app_bar_app(
                &mut tester,
                SliverAppBarApp {
                    floating: true,
                    snap: true,
                    expanded_height: Some(128.0),
                    ..SliverAppBarApp::default()
                },
            );
            assert!(sliver_app_bar_is_onstage(&tester));
            assert_eq!(app_bar_rect(&tester).top, 0.0);
            assert_eq!(app_bar_rect(&tester).height(), 128.0);
            assert_eq!(app_bar_rect(&tester).bottom, 128.0);

            // Scroll to the middle of the list. The (floating) appbar is no longer visible.
            handles
                .controller
                .jump_to(&mut tester.cell.borrow_mut(), 256.0);
            tester.pump_and_settle();
            assert!(!sliver_app_bar_is_onstage(&tester));
            assert!(app_bar_rect(&tester).top <= -128.0);

            // Drag the scrollable up and down. The app bar should not snap open, its
            // height should just track the drag offset.
            let mut gesture = WidgetTester::touch(1);
            tester.down(&mut gesture, Offset::new(50.0, 256.0));
            // drag the appbar all the way open
            move_by(&mut tester, &mut gesture, Offset::new(0.0, 128.0));
            tester.pump(Duration::ZERO);
            assert_eq!(app_bar_rect(&tester).top, 0.0);
            assert_eq!(app_bar_rect(&tester).height(), 128.0);

            move_by(&mut tester, &mut gesture, Offset::new(0.0, -50.0));
            tester.pump(Duration::ZERO);
            assert_eq!(app_bar_rect(&tester).bottom, 78.0); // 78 == 128 - 50

            // Trigger the snap open animation: drag down and release
            move_by(&mut tester, &mut gesture, Offset::new(0.0, 10.0));
            tester.up(&gesture);

            // Now verify that the appbar is animating open
            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_millis(50));
            let bottom = app_bar_rect(&tester).bottom;
            assert!(bottom > 88.0); // 88 = 78 + 10

            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_millis(50));
            assert!(app_bar_rect(&tester).bottom > bottom);

            // The animation finishes when the appbar is full height.
            tester.pump_and_settle();
            assert_eq!(app_bar_rect(&tester).height(), 128.0);

            // Now that the app bar is open, perform the same drag scenario
            // in reverse: drag the appbar up and down and then trigger the
            // snap closed animation.
            let mut gesture = WidgetTester::touch(2);
            tester.down(&mut gesture, Offset::new(50.0, 256.0));
            // drag the appbar closed
            move_by(&mut tester, &mut gesture, Offset::new(0.0, -128.0));
            tester.pump(Duration::ZERO);
            assert_eq!(app_bar_rect(&tester).bottom, 0.0);

            move_by(&mut tester, &mut gesture, Offset::new(0.0, 100.0));
            tester.pump(Duration::ZERO);
            assert_eq!(app_bar_rect(&tester).bottom, 100.0);

            // Trigger the snap close animation: drag upwards and release
            move_by(&mut tester, &mut gesture, Offset::new(0.0, -10.0));
            tester.up(&gesture);

            // Now verify that the appbar is animating closed
            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_millis(50));
            let bottom = app_bar_rect(&tester).bottom;
            assert!(bottom < 90.0);

            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_millis(50));
            assert!(app_bar_rect(&tester).bottom < bottom);

            // The animation finishes when the appbar is off screen.
            tester.pump_and_settle();
            assert!(app_bar_rect(&tester).top <= 0.0);
            assert!(app_bar_rect(&tester).bottom <= 0.0);
        });
    }

    #[test]
    fn a_floating_and_pinned_sliver_app_bar_with_snap_snaps_to_its_pinned_height() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let handles = pump_sliver_app_bar_app(
                &mut tester,
                SliverAppBarApp {
                    floating: true,
                    pinned: true,
                    snap: true,
                    expanded_height: Some(128.0),
                    ..SliverAppBarApp::default()
                },
            );
            assert!(sliver_app_bar_is_onstage(&tester));
            assert_eq!(app_bar_rect(&tester).top, 0.0);
            assert_eq!(app_bar_rect(&tester).height(), 128.0);
            assert_eq!(app_bar_rect(&tester).bottom, 128.0);

            // Scroll to the middle of the list. The only the tab bar is visible
            // because this is a pinned appbar.
            handles
                .controller
                .jump_to(&mut tester.cell.borrow_mut(), 256.0);
            tester.pump_and_settle();
            assert!(sliver_app_bar_is_onstage(&tester));
            assert_eq!(app_bar_rect(&tester).top, 0.0);
            assert_eq!(app_bar_rect(&tester).height(), K_TEXT_TAB_BAR_HEIGHT);

            // Drag the scrollable up and down. The app bar should not snap open, the
            // bottom of the appbar should just track the drag offset.
            let mut gesture = WidgetTester::touch(1);
            tester.down(&mut gesture, Offset::new(50.0, 200.0));
            move_by(&mut tester, &mut gesture, Offset::new(0.0, 100.0));
            tester.pump(Duration::ZERO);
            assert_eq!(app_bar_rect(&tester).height(), 100.0);

            move_by(&mut tester, &mut gesture, Offset::new(0.0, -25.0));
            tester.pump(Duration::ZERO);
            assert_eq!(app_bar_rect(&tester).height(), 75.0);

            // Trigger the snap animation: drag down and release
            move_by(&mut tester, &mut gesture, Offset::new(0.0, 10.0));
            tester.up(&gesture);

            // Now verify that the appbar is animating open
            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_millis(50));
            let height = app_bar_rect(&tester).height();
            assert!(height > 85.0);
            assert!(height < 128.0);

            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_millis(50));
            assert!(app_bar_rect(&tester).height() > height);
            assert!(app_bar_rect(&tester).height() < 128.0);

            // The animation finishes when the appbar is fully expanded
            tester.pump_and_settle();
            assert_eq!(app_bar_rect(&tester).top, 0.0);
            assert_eq!(app_bar_rect(&tester).height(), 128.0);
            assert_eq!(app_bar_rect(&tester).bottom, 128.0);

            // Now that the appbar is fully expanded, Perform the same drag
            // scenario in reverse: drag the appbar up and down and then trigger
            // the snap closed animation.
            let mut gesture = WidgetTester::touch(2);
            tester.down(&mut gesture, Offset::new(50.0, 256.0));
            move_by(&mut tester, &mut gesture, Offset::new(0.0, -128.0));
            tester.pump(Duration::ZERO);
            assert_eq!(app_bar_rect(&tester).bottom, K_TEXT_TAB_BAR_HEIGHT);

            move_by(&mut tester, &mut gesture, Offset::new(0.0, 100.0));
            tester.pump(Duration::ZERO);
            assert_eq!(app_bar_rect(&tester).bottom, 100.0);

            // Trigger the snap close animation: drag upwards and release
            move_by(&mut tester, &mut gesture, Offset::new(0.0, -10.0));
            tester.up(&gesture);

            // Now verify that the appbar is animating closed
            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_millis(50));
            let bottom = app_bar_rect(&tester).bottom;
            assert!(bottom < 90.0);

            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_millis(50));
            assert!(app_bar_rect(&tester).bottom < bottom);

            // The animation finishes when the appbar shrinks back to its pinned height
            tester.pump_and_settle();
            assert!(app_bar_rect(&tester).top <= 0.0);
            assert_eq!(app_bar_rect(&tester).bottom, K_TEXT_TAB_BAR_HEIGHT);
        });
    }

    #[test]
    fn a_sliver_app_bar_scrolls_from_its_expanded_height_to_its_collapsed_height() {
        with_main_thread_stack(|| {
            const EXPANDED_APP_BAR_HEIGHT: f64 = 400.0;
            const COLLAPSED_APP_BAR_HEIGHT: f64 = 200.0;
            let mut tester = tester();
            let handles = pump_sliver_app_bar_app(
                &mut tester,
                SliverAppBarApp {
                    collapsed_height: Some(COLLAPSED_APP_BAR_HEIGHT),
                    expanded_height: Some(EXPANDED_APP_BAR_HEIGHT),
                    ..SliverAppBarApp::default()
                },
            );
            assert_eq!(scroll_offset(&tester, handles.controller), 0.0);
            assert!(sliver_app_bar_is_onstage(&tester));
            assert_eq!(app_bar_rect(&tester).height(), EXPANDED_APP_BAR_HEIGHT);

            let initial_tab_bar_height = tab_bar_height(&tester, &handles);

            // Scroll the not-pinned appbar partially out of view.
            jump_to(&mut tester, handles.controller, 50.0);
            assert!(sliver_app_bar_is_onstage(&tester));
            assert_eq!(
                app_bar_rect(&tester).height(),
                EXPANDED_APP_BAR_HEIGHT - 50.0
            );
            assert_eq!(tab_bar_height(&tester, &handles), initial_tab_bar_height);

            // Scroll the not-pinned appbar out of view, to its collapsed height.
            jump_to(&mut tester, handles.controller, 600.0);
            assert!(!sliver_app_bar_is_onstage(&tester));
            assert_eq!(
                app_bar_rect(&tester).height(),
                COLLAPSED_APP_BAR_HEIGHT + initial_tab_bar_height
            );
            assert_eq!(tab_bar_height(&tester, &handles), initial_tab_bar_height);

            // Scroll the not-pinned appbar back into view.
            jump_to(&mut tester, handles.controller, 0.0);
            assert!(sliver_app_bar_is_onstage(&tester));
            assert_eq!(app_bar_rect(&tester).height(), EXPANDED_APP_BAR_HEIGHT);
            assert_eq!(tab_bar_height(&tester, &handles), initial_tab_bar_height);
        });
    }

    fn collapsed_height_case(arguments: SliverAppBarApp, expected_without_bottom: f64) {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let handles = pump_sliver_app_bar_app(&mut tester, arguments);
            let initial_tab_bar_height = tab_bar_height(&tester, &handles);

            // Scroll the not-pinned appbar out of view, to its collapsed height.
            jump_to(&mut tester, handles.controller, 300.0);
            assert!(!sliver_app_bar_is_onstage(&tester));
            assert_eq!(
                app_bar_rect(&tester).height(),
                expected_without_bottom + initial_tab_bar_height
            );
        });
    }

    #[test]
    fn the_default_collapsed_height_is_the_toolbar_height_and_the_bottom() {
        // By default, the collapsedHeight is toolbarHeight + bottom.preferredSize.height,
        // in this case initialTabBarHeight.
        collapsed_height_case(
            SliverAppBarApp {
                toolbar_height: 100.0,
                ..SliverAppBarApp::default()
            },
            100.0,
        );
    }

    #[test]
    fn a_collapsed_height_with_a_toolbar_height_collapses_to_the_collapsed_height() {
        collapsed_height_case(
            SliverAppBarApp {
                toolbar_height: 100.0,
                collapsed_height: Some(150.0),
                ..SliverAppBarApp::default()
            },
            150.0,
        );
    }

    #[test]
    fn a_collapsed_height_collapses_to_itself_and_the_bottom() {
        collapsed_height_case(
            SliverAppBarApp {
                collapsed_height: Some(56.0),
                ..SliverAppBarApp::default()
            },
            56.0,
        );
    }

    /// Dart's 'Material3 - SliverAppBar.medium defaults' and '.large defaults'.
    ///
    /// `expanded_title_bottom` answers from the title's rectangle where Dart's case expects
    /// its bottom left corner.
    fn medium_or_large_defaults_case(
        sliver_app_bar: fn() -> SliverAppBar,
        expanded_app_bar_height: f64,
        expanded_title_bottom: fn(&WidgetTester, Rect) -> f64,
    ) {
        with_main_thread_stack(|| {
            const COLLAPSED_APP_BAR_HEIGHT: f64 = 64.0;
            let mut tester = tester();
            let controller = ScrollController::default(&mut tester.cell.borrow_mut());
            pump_app(&mut tester, m3, move |_app| {
                Scaffold::new()
                    .body(
                        CustomScrollView::new()
                            .controller(controller.as_controller())
                            .slivers([
                                sliver_app_bar()
                                    .title(Text::new("AppBar Title"))
                                    .into_widget(),
                                SliverToBoxAdapter::new()
                                    .child(
                                        Container::new()
                                            .height(1200.0)
                                            .color(crate::colors::Colors::ORANGE.shade400()),
                                    )
                                    .into_widget(),
                            ]),
                    )
                    .into_widget()
            });

            // There are two widgets for the title. The first title is a larger version
            // that is shown at the bottom when the app bar is expanded. It scrolls under
            // the main row until it is completely hidden and then the first title is
            // faded in. The last is the title on the mainrow with the icons. It is
            // transparent when the app bar is expanded, and opaque when it is collapsed.
            let titles = tester.find_onstage_text("AppBar Title");
            let expanded_title = titles[0];
            let collapsed_title = *titles.last().expect("two titles");
            let expanded_title_clip = tester.ancestor_of_type::<ClipRect>(expanded_title);
            let collapsed_title_opacity =
                tester.ancestor_of_type::<AnimatedOpacity>(collapsed_title);
            let opacity = |tester: &WidgetTester| {
                tester.widget(collapsed_title_opacity, |opacity: &AnimatedOpacity| {
                    opacity.opacity
                })
            };
            let clip_height = |tester: &WidgetTester| rect_of(tester, expanded_title_clip).height();

            // Default, fully expanded app bar.
            assert_eq!(scroll_offset(&tester, controller), 0.0);
            assert!(sliver_app_bar_is_onstage(&tester));
            assert_eq!(app_bar_rect(&tester).height(), expanded_app_bar_height);
            assert_eq!(opacity(&tester), 0.0);
            assert_eq!(
                clip_height(&tester),
                expanded_app_bar_height - COLLAPSED_APP_BAR_HEIGHT
            );

            // Test the expanded title is positioned correctly.
            let title_rect = rect_of(&tester, expanded_title);
            assert_eq!(title_rect.left, 16.0);
            let expected_bottom = expanded_title_bottom(&tester, title_rect);
            // Valo's paragraph metrics are f32.
            assert!(
                (title_rect.bottom - expected_bottom).abs() < 1e-4,
                "{} is not {expected_bottom}",
                title_rect.bottom
            );

            // verifyTextNotClipped
            let clip_rect = rect_of(&tester, expanded_title_clip);
            assert!((clip_rect.top..=clip_rect.bottom).contains(&title_rect.top));
            assert!((clip_rect.top..=clip_rect.bottom).contains(&title_rect.bottom));
            assert!((clip_rect.left..=clip_rect.right).contains(&title_rect.left));
            assert!((clip_rect.left..=clip_rect.right).contains(&title_rect.right));

            // Test the expanded title default color.
            let on_surface = ThemeData::new(&tester.cell.borrow())
                .color_scheme
                .on_surface;
            assert_eq!(tester.text_color("AppBar Title"), on_surface);

            // Scroll the expanded app bar partially out of view.
            jump_to(&mut tester, controller, 45.0);
            assert!(sliver_app_bar_is_onstage(&tester));
            assert_eq!(
                app_bar_rect(&tester).height(),
                expanded_app_bar_height - 45.0
            );
            assert_eq!(opacity(&tester), 0.0);
            assert_eq!(
                clip_height(&tester),
                expanded_app_bar_height - COLLAPSED_APP_BAR_HEIGHT - 45.0
            );

            // Scroll so that it is completely collapsed.
            jump_to(&mut tester, controller, 600.0);
            assert!(sliver_app_bar_is_onstage(&tester));
            assert_eq!(app_bar_rect(&tester).height(), COLLAPSED_APP_BAR_HEIGHT);
            assert_eq!(opacity(&tester), 1.0);
            assert_eq!(clip_height(&tester), 0.0);

            // Scroll back to fully expanded.
            controller.jump_to(&mut tester.cell.borrow_mut(), 0.0);
            tester.pump_and_settle();
            assert!(sliver_app_bar_is_onstage(&tester));
            assert_eq!(app_bar_rect(&tester).height(), expanded_app_bar_height);
            assert_eq!(opacity(&tester), 0.0);
            assert_eq!(
                clip_height(&tester),
                expanded_app_bar_height - COLLAPSED_APP_BAR_HEIGHT
            );
        });
    }

    #[test]
    fn material3_a_medium_sliver_app_bar_expands_its_title_below_the_toolbar() {
        // The 64 high toolbar and the title, which the headline small style makes 24 * 1.33 =
        // 31.92 high and the paragraph rounds to 32, as skparagraph's line height does.
        medium_or_large_defaults_case(SliverAppBar::medium, 112.0, |_tester, _title| 96.0);
    }

    /// app_bar_sliver_test's `SliverAppBar.medium with bottom widget` and `.large`'s twin.
    fn medium_or_large_with_bottom_case(
        sliver_app_bar: fn() -> SliverAppBar,
        expanded_app_bar_height: f64,
        title: &'static str,
        collapse_to: f64,
    ) {
        with_main_thread_stack(|| {
            // This is a regression test for https://github.com/flutter/flutter/issues/115091
            const COLLAPSED_APP_BAR_HEIGHT: f64 = 64.0;
            const BOTTOM_HEIGHT: f64 = 48.0;
            let mut tester = tester();
            let controller = ScrollController::default(&mut tester.cell.borrow_mut());
            let tab_bar_key = GlobalKey::new();
            let bottom_key = tab_bar_key.clone();
            pump_app(&mut tester, m3, move |_app| {
                DefaultTabController::new(
                    3,
                    Scaffold::new().body(
                        CustomScrollView::new()
                            .controller(controller.as_controller())
                            .slivers([
                                sliver_app_bar()
                                    .leading(IconButton::new(
                                        Some(Listener::new(|_app| {})),
                                        Icon::new(Some(Icons::menu())),
                                    ))
                                    .title(Text::new(title))
                                    .bottom(PreferredSizeWidgetRef::new(
                                        TabBar::new(
                                            ["Tab 1", "Tab 2", "Tab 3"]
                                                .map(|tab| Tab::new().text(tab).into_widget()),
                                        )
                                        .key(bottom_key.into()),
                                    ))
                                    .into_widget(),
                                SliverToBoxAdapter::new()
                                    .child(
                                        Container::new()
                                            .height(1200.0)
                                            .color(crate::colors::Colors::ORANGE.shade400()),
                                    )
                                    .into_widget(),
                            ]),
                    ),
                )
                .into_widget()
            });

            assert_eq!(tester.find_by_type::<SliverAppBar>().len(), 1);
            assert_eq!(
                app_bar_rect(&tester).height(),
                expanded_app_bar_height + BOTTOM_HEIGHT
            );

            let expanded_title = text_rect(&tester, title);
            let tab_offset = key_rect(&tester, &tab_bar_key).top_left();
            // Valo's paragraph metrics are f32.
            assert!(
                (expanded_title.bottom_left().dy() - tab_offset.dy()).abs() < 1e-4,
                "{expanded_title:?} {tab_offset:?}"
            );

            // Scroll CustomScrollView to collapse SliverAppBar.
            controller.jump_to(&mut tester.cell.borrow_mut(), collapse_to);
            tester.pump_and_settle();

            assert_eq!(
                app_bar_rect(&tester).height(),
                COLLAPSED_APP_BAR_HEIGHT + BOTTOM_HEIGHT
            );
        });
    }

    // 'SliverAppBar.medium with bottom widget'.
    #[test]
    fn a_medium_sliver_app_bar_puts_its_expanded_title_on_its_tab_bar() {
        medium_or_large_with_bottom_case(SliverAppBar::medium, 112.0, "Medium App Bar", 160.0);
    }

    // 'SliverAppBar.large with bottom widget'.
    #[test]
    fn a_large_sliver_app_bar_puts_its_expanded_title_on_its_tab_bar() {
        medium_or_large_with_bottom_case(SliverAppBar::large, 152.0, "Large App Bar", 200.0);
    }

    #[test]
    fn material3_a_large_sliver_app_bar_expands_its_title_28_above_its_bottom() {
        medium_or_large_defaults_case(SliverAppBar::large, 152.0, |tester, _title| {
            // The expanded title and the bottom padding fits in the flexible space.
            sliver_app_bar_geometry(tester).scroll_extent - 28.0
        });
    }

    /// Dart's `SliverAppBar elevation` group's `buildSliverAppBar`.
    fn pump_elevated_sliver_app_bar(
        tester: &mut WidgetTester,
        force_elevated: bool,
        elevation: Option<f64>,
        theme_elevation: Option<f64>,
    ) {
        pump_app(
            tester,
            move |app| {
                let mut app_bar_theme = AppBarThemeData::new();
                app_bar_theme.elevation = theme_elevation;
                app_bar_theme.scrolled_under_elevation = theme_elevation;
                ThemeData::builder().app_bar_theme(app_bar_theme).build(app)
            },
            move |_app| {
                let mut sliver_app_bar = SliverAppBar::new()
                    .title(Text::new("Title"))
                    .force_elevated(force_elevated);
                sliver_app_bar.elevation = elevation;
                sliver_app_bar.scrolled_under_elevation = elevation;
                CustomScrollView::new()
                    .slivers([sliver_app_bar.into_widget()])
                    .into_widget()
            },
        );
    }

    #[test]
    fn a_sliver_app_bar_is_elevated_only_when_forced_or_scrolled_under() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let app_bar_elevation_field = |tester: &WidgetTester| {
                let app_bar = tester.find_by_type::<AppBar>()[0];
                tester.widget(app_bar, |app_bar: &AppBar| app_bar.elevation)
            };

            // When forceElevated is off, SliverAppBar should not be elevated.
            pump_elevated_sliver_app_bar(&mut tester, false, None, None);
            assert_eq!(app_bar_elevation(&tester), 0.0);

            // Default elevation should be used by the material, but
            // the AppBar's elevation should not be specified by SliverAppBar.
            // When useMaterial3 is true, and forceElevated is true, the default elevation
            // should be the value of `scrolledUnderElevation` which is 3.0
            pump_elevated_sliver_app_bar(&mut tester, true, None, None);
            assert_eq!(app_bar_elevation(&tester), 3.0);
            assert_eq!(app_bar_elevation_field(&tester), None);

            // SliverAppBar should use the specified elevation.
            pump_elevated_sliver_app_bar(&mut tester, true, Some(8.0), None);
            assert_eq!(app_bar_elevation(&tester), 8.0);
        });
    }

    #[test]
    fn a_sliver_app_bar_takes_its_elevation_from_the_app_bar_theme() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_elevated_sliver_app_bar(&mut tester, false, None, Some(12.0));
            assert_eq!(app_bar_elevation(&tester), 0.0);

            pump_elevated_sliver_app_bar(&mut tester, true, None, Some(12.0));
            assert_eq!(app_bar_elevation(&tester), 12.0);

            pump_elevated_sliver_app_bar(&mut tester, true, Some(8.0), Some(12.0));
            assert_eq!(app_bar_elevation(&tester), 8.0);
        });
    }

    #[test]
    fn changing_snap_from_true_to_false_keeps_the_floating_bar_working() {
        with_main_thread_stack(|| {
            const APP_BAR_HEIGHT: f64 = 256.0;
            let mut tester = tester();
            let snap = Rc::new(Cell::new(true));
            let button_key = GlobalKey::new();
            let snap_for_build = Rc::clone(&snap);
            let key = button_key.clone();
            pump_app(&mut tester, m3, move |_app| {
                waft_widgets::StatefulBuilder::new(move |_app, _context, set_state| {
                    let snap = Rc::clone(&snap_for_build);
                    let set_state = set_state.clone();
                    let button = crate::text_button::TextButton::new(
                        Some(Listener::new(move |app| {
                            let snap = Rc::clone(&snap);
                            set_state(app, &mut move |_app| snap.set(false));
                        })),
                        Some(Text::new("snap=false")),
                    )
                    .key(key.clone().into());
                    Scaffold::new()
                        .body(
                            CustomScrollView::new().slivers([
                                SliverAppBar::new()
                                    .expanded_height(APP_BAR_HEIGHT)
                                    .floating(true)
                                    .snap(snap_for_build.get())
                                    .actions(vec![button.into_widget()])
                                    .flexible_space(
                                        FlexibleSpaceBar::new().background(
                                            Container::new()
                                                .height(APP_BAR_HEIGHT)
                                                .color(crate::colors::Colors::ORANGE.primary()),
                                        ),
                                    )
                                    .into_widget(),
                                SliverToBoxAdapter::new()
                                    .child(
                                        Container::new()
                                            .height(1200.0)
                                            .color(crate::colors::Colors::TEAL.primary()),
                                    )
                                    .into_widget(),
                            ]),
                        )
                        .into_widget()
                })
                .into_widget()
            });

            let mut gesture = WidgetTester::touch(1);
            tester.down(&mut gesture, Offset::new(50.0, 400.0));
            tester.move_to(&mut gesture, Offset::new(50.0, 300.0), true);
            tester.up(&gesture);

            let at = tester.center_of(&button_key);
            tester.tap(at, 2);
            tester.pump_and_settle();
            assert!(!snap.get());

            let mut gesture = WidgetTester::touch(3);
            tester.down(&mut gesture, Offset::new(50.0, 400.0));
            tester.move_to(&mut gesture, Offset::new(50.0, 300.0), true);
            tester.up(&gesture);
            tester.pump(Duration::ZERO);
        });
    }

    #[test]
    fn a_sliver_app_bar_gives_its_delegate_a_show_on_screen_configuration_only_when_it_snaps() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            for (pinned, floating, snap) in [
                (false, true, false),
                (false, true, true),
                (true, true, false),
                (true, true, true),
            ] {
                pump_app(&mut tester, m3, move |_app| {
                    CustomScrollView::new()
                        .slivers([SliverAppBar::new()
                            .title(Text::new("Jumbo"))
                            .pinned(pinned)
                            .floating(floating)
                            .snap(snap)
                            .into_widget()])
                        .into_widget()
                });
                let header = tester.find_by_type::<SliverPersistentHeader>()[0];
                let (vsync, snap_configuration, show_on_screen_configuration) =
                    tester.widget(header, |header: &SliverPersistentHeader| {
                        (
                            header.delegate.vsync().is_some(),
                            header.delegate.snap_configuration().is_some(),
                            header.delegate.show_on_screen_configuration().is_some(),
                        )
                    });

                // Ensure we have a non-null vsync when it's needed.
                if !floating || (!snap_configuration && !show_on_screen_configuration) {
                    assert!(vsync);
                }
                assert_eq!(show_on_screen_configuration, snap && floating);
            }
        });
    }

    #[test]
    fn a_sliver_app_bar_uses_its_leading_width() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            pump_app(&mut tester, m3, |_app| {
                CustomScrollView::new()
                    .slivers([SliverAppBar::new()
                        .leading(placeholder(&key))
                        .leading_width(100.0)
                        .title(Text::new("Title"))
                        .into_widget()])
                    .into_widget()
            });
            // By default toolbarHeight is 56.0.
            assert_eq!(
                key_rect(&tester, &key),
                Rect::from_ltrb(0.0, 0.0, 100.0, 56.0)
            );
        });
    }

    #[test]
    fn a_sliver_app_bars_title_spacing_defaults_to_the_middle_spacing() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_sliver_app_bar_app(&mut tester, SliverAppBarApp::default());
            assert_eq!(
                navigation_toolbar(&tester, |toolbar| toolbar.middle_spacing),
                NavigationToolbar::K_MIDDLE_SPACING
            );
        });
    }

    // ---------------------------------------------------------------------------------------
    // Flutter's former test/material/sliver_app_bar_test.dart, 'SliverAppBar - Stretch'

    /// Dart's `slowDrag`: a touch down at the centre of `key`'s box, a move by `offset`, ten
    /// milliseconds, and an up.
    fn slow_drag(tester: &mut WidgetTester, key: &GlobalKey, offset: Offset, pointer: i64) {
        let target = tester.center_of(key);
        let mut gesture = WidgetTester::touch(pointer);
        tester.down(&mut gesture, target);
        tester.move_to(&mut gesture, target + offset, true);
        tester.pump(Duration::from_millis(10));
        tester.up(&gesture);
    }

    /// A stretching `SliverAppBar` 100 high over two 800 high boxes, the first keyed
    /// `anchor`, whose `onStretchTrigger` sets the returned flag.
    fn pump_stretching_sliver_app_bar(
        tester: &mut WidgetTester,
        anchor: &GlobalKey,
        physics: waft_widgets::ScrollPhysicsRef,
        stretch_trigger_offset: Option<f64>,
    ) -> Rc<Cell<bool>> {
        let did_trigger = Rc::new(Cell::new(false));
        let triggered = Rc::clone(&did_trigger);
        let anchor = anchor.clone();
        pump_app(tester, m3, move |_app| {
            let mut sliver_app_bar = SliverAppBar::new()
                .stretch(true)
                .expanded_height(100.0)
                .on_stretch_trigger(Rc::new(move |_app| {
                    triggered.set(true);
                    Task::ready(())
                }));
            if let Some(stretch_trigger_offset) = stretch_trigger_offset {
                sliver_app_bar = sliver_app_bar.stretch_trigger_offset(stretch_trigger_offset);
            }
            CustomScrollView::new()
                .physics(physics)
                .slivers([
                    sliver_app_bar.into_widget(),
                    SliverToBoxAdapter::new()
                        .child(Container::new().key(anchor.into()).height(800.0))
                        .into_widget(),
                    SliverToBoxAdapter::new()
                        .child(Container::new().height(800.0))
                        .into_widget(),
                ])
                .into_widget()
        });
        did_trigger
    }

    #[test]
    fn a_stretching_sliver_app_bar_triggers_past_the_default_offset() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let anchor = GlobalKey::new();
            let did_trigger = pump_stretching_sliver_app_bar(
                &mut tester,
                &anchor,
                Rc::new(waft_widgets::BouncingScrollPhysics::new()),
                None,
            );

            slow_drag(&mut tester, &anchor, Offset::new(0.0, 50.0), 1);
            assert!(!did_trigger.get());
            tester.pump_and_settle();
            slow_drag(&mut tester, &anchor, Offset::new(0.0, 150.0), 2);
            assert!(did_trigger.get());
        });
    }

    #[test]
    fn a_stretching_sliver_app_bar_triggers_past_a_custom_offset() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let anchor = GlobalKey::new();
            let did_trigger = pump_stretching_sliver_app_bar(
                &mut tester,
                &anchor,
                Rc::new(waft_widgets::BouncingScrollPhysics::new()),
                Some(150.0),
            );

            slow_drag(&mut tester, &anchor, Offset::new(0.0, 100.0), 1);
            tester.pump_and_settle();
            assert!(!did_trigger.get());
            slow_drag(&mut tester, &anchor, Offset::new(0.0, 300.0), 2);
            assert!(did_trigger.get());
        });
    }

    #[test]
    fn a_stretching_sliver_app_bar_does_not_trigger_without_overscroll_physics() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let anchor = GlobalKey::new();
            let did_trigger = pump_stretching_sliver_app_bar(
                &mut tester,
                &anchor,
                Rc::new(waft_widgets::ClampingScrollPhysics::new()),
                Some(150.0),
            );

            slow_drag(&mut tester, &anchor, Offset::new(0.0, 100.0), 1);
            tester.pump_and_settle();
            assert!(!did_trigger.get());
            slow_drag(&mut tester, &anchor, Offset::new(0.0, 300.0), 2);
            assert!(!did_trigger.get());
        });
    }

    // ---------------------------------------------------------------------------------------
    // app_bar_theme_test.dart

    /// Dart's `_appBarTheme()`.
    fn app_bar_theme() -> AppBarThemeData {
        use crate::colors::Colors;
        AppBarThemeData::new()
            .actions_icon_theme(IconThemeData::new().color(Colors::PINK.primary()))
            .system_overlay_style(SystemUiOverlayStyle::DARK)
            .background_color(Colors::LIGHT_BLUE.primary())
            .elevation(6.0)
            .shadow_color(Colors::RED.primary())
            .surface_tint_color(Colors::GREEN.primary())
            .shape(waft_painting::StadiumBorder::default())
            .icon_theme(IconThemeData::new().color(Colors::BLACK))
            .toolbar_height(96.0)
            .toolbar_text_style(TextStyle::new().color(Colors::YELLOW.primary()))
            .title_text_style(TextStyle::new().color(Colors::PINK.primary()))
    }

    /// The first `W` below the first `A`: Dart's
    /// `find.descendant(of: find.byType(A), matching: find.byType(W)).first`.
    fn first_below<A: 'static, W: 'static>(tester: &WidgetTester) -> AnyElement {
        let ancestor = *tester.find_by_type::<A>().first().expect("an ancestor");
        tester
            .find_by_type::<W>()
            .into_iter()
            .find(|element| tester.ancestors(*element).contains(&ancestor))
            .expect("a descendant of that type")
    }

    /// Dart's `_getAppBarIconTheme(tester).data`.
    fn app_bar_icon_theme(tester: &WidgetTester) -> IconThemeData {
        tester.widget(
            first_below::<AppBar, IconTheme>(tester),
            |theme: &IconTheme| theme.data.clone(),
        )
    }

    /// Dart's `_getAppBarActionsIconTheme(tester).data`.
    fn app_bar_actions_icon_theme(tester: &WidgetTester) -> IconThemeData {
        tester.widget(
            first_below::<NavigationToolbar, IconTheme>(tester),
            |theme: &IconTheme| theme.data.clone(),
        )
    }

    /// Dart's `_getAppBarIconRichText(tester).text.style!.color`.
    fn app_bar_icon_color(tester: &WidgetTester) -> Option<Color> {
        tester
            .glyph_style(*tester.find_by_type::<Icon>().first().expect("an Icon"))
            .color
            .map(|color| color.color())
    }

    /// Dart's `_getAppBarText(tester).style`.
    fn app_bar_text_style(tester: &WidgetTester) -> TextStyle {
        tester.widget(
            first_below::<CustomSingleChildLayout, DefaultTextStyle>(tester),
            |style: &DefaultTextStyle| style.style.clone(),
        )
    }

    fn share_action() -> Vec<WidgetRef> {
        vec![
            IconButton::new(Some(Listener::new(|_| {})), Icon::new(Some(Icons::share())))
                .into_widget(),
        ]
    }

    fn status_bar_brightness(tester: &WidgetTester) -> Option<Brightness> {
        latest_style(tester).expect("a style").status_bar_brightness
    }

    #[test]
    fn the_app_bar_uses_the_values_of_the_app_bar_theme() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(
                &mut tester,
                |app| {
                    ThemeData::builder()
                        .app_bar_theme(app_bar_theme())
                        .build(app)
                },
                |_app| {
                    scaffold_with(
                        AppBar::new()
                            .title(Text::new("App Bar Title"))
                            .actions(share_action()),
                    )
                    .into_widget()
                },
            );
            let theme = app_bar_theme();

            assert_eq!(status_bar_brightness(&tester), Some(Brightness::Light));
            app_bar_material(&tester, |material| {
                assert_eq!(material.color, theme.background_color);
                assert_eq!(Some(material.elevation), theme.elevation);
                assert_eq!(material.shadow_color, theme.shadow_color);
                assert_eq!(material.surface_tint_color, theme.surface_tint_color);
                assert!(
                    material.shape.as_deref()
                        == Some(&waft_painting::StadiumBorder::default() as &dyn ShapeBorder)
                );
            });
            assert_eq!(Some(app_bar_icon_theme(&tester)), theme.icon_theme);
            assert_eq!(
                Some(app_bar_actions_icon_theme(&tester)),
                theme.actions_icon_theme
            );
            assert_eq!(
                app_bar_icon_color(&tester),
                theme
                    .actions_icon_theme
                    .as_ref()
                    .and_then(|icon_theme| icon_theme.color.as_ref())
                    .map(|color| color.color())
            );
            assert_eq!(Some(app_bar_text_style(&tester)), theme.toolbar_text_style);
            assert_eq!(Some(app_bar_rect(&tester).height()), theme.toolbar_height);
            assert_eq!(app_bar_rect(&tester).width(), 800.0);
        });
    }

    #[test]
    fn the_app_bars_properties_take_priority_over_the_theme() {
        with_main_thread_stack(|| {
            use crate::colors::Colors;
            let mut tester = tester();
            let shape = || RoundedRectangleBorder::default();
            pump_app(
                &mut tester,
                |app| {
                    ThemeData::from(crate::color_scheme::ColorScheme::light())
                        .build(app)
                        .copy_with()
                        .app_bar_theme(app_bar_theme())
                        .build()
                },
                move |_app| {
                    scaffold_with(
                        AppBar::new()
                            .background_color(Colors::ORANGE.primary())
                            .system_overlay_style(SystemUiOverlayStyle::LIGHT)
                            .elevation(3.0)
                            .shadow_color(Colors::PURPLE.primary())
                            .surface_tint_color(Colors::BROWN.primary())
                            .shape(shape())
                            .icon_theme(IconThemeData::new().color(Colors::GREEN.primary()))
                            .actions_icon_theme(
                                IconThemeData::new().color(Colors::LIGHT_BLUE.primary()),
                            )
                            .toolbar_text_style(TextStyle::new().color(Colors::PINK.primary()))
                            .title_text_style(TextStyle::new().color(Colors::ORANGE.primary()))
                            .actions(share_action()),
                    )
                    .into_widget()
                },
            );

            assert_eq!(status_bar_brightness(&tester), Some(Brightness::Dark));
            app_bar_material(&tester, |material| {
                assert_eq!(material.color, Some(Colors::ORANGE.primary()));
                assert_eq!(material.elevation, 3.0);
                assert_eq!(material.shadow_color, Some(Colors::PURPLE.primary()));
                assert_eq!(material.surface_tint_color, Some(Colors::BROWN.primary()));
                assert!(material.shape.as_deref() == Some(&shape() as &dyn ShapeBorder));
            });
            assert_eq!(
                app_bar_icon_theme(&tester),
                IconThemeData::new().color(Colors::GREEN.primary())
            );
            assert_eq!(
                app_bar_actions_icon_theme(&tester),
                IconThemeData::new().color(Colors::LIGHT_BLUE.primary())
            );
            assert_eq!(
                app_bar_icon_color(&tester),
                Some(Colors::LIGHT_BLUE.primary())
            );
            assert_eq!(
                app_bar_text_style(&tester),
                TextStyle::new().color(Colors::PINK.primary())
            );
        });
    }

    #[test]
    fn an_icon_buttons_color_takes_priority_over_every_icon_theme() {
        with_main_thread_stack(|| {
            use crate::colors::Colors;
            let mut tester = tester();
            pump_app(&mut tester, light_scheme_theme, |_app| {
                scaffold_with(
                    AppBar::new()
                        .icon_theme(IconThemeData::new().color(Colors::GREEN.primary()))
                        .actions_icon_theme(
                            IconThemeData::new().color(Colors::LIGHT_BLUE.primary()),
                        )
                        .actions(vec![
                            IconButton::new(
                                Some(Listener::new(|_| {})),
                                Icon::new(Some(Icons::share())),
                            )
                            .color(Colors::LIME.primary())
                            .into_widget(),
                        ]),
                )
                .into_widget()
            });
            assert_eq!(app_bar_icon_color(&tester), Some(Colors::LIME.primary()));
        });
    }

    #[test]
    fn the_app_bar_theme_takes_priority_over_the_theme_datas_colors() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(
                &mut tester,
                |app| {
                    ThemeData::from(crate::color_scheme::ColorScheme::light())
                        .build(app)
                        .copy_with()
                        .app_bar_theme(app_bar_theme())
                        .build()
                },
                |_app| scaffold_with(AppBar::new().actions(share_action())).into_widget(),
            );
            let theme = app_bar_theme();

            assert_eq!(status_bar_brightness(&tester), Some(Brightness::Light));
            app_bar_material(&tester, |material| {
                assert_eq!(material.color, theme.background_color);
                assert_eq!(Some(material.elevation), theme.elevation);
                assert_eq!(material.shadow_color, theme.shadow_color);
                assert_eq!(material.surface_tint_color, theme.surface_tint_color);
            });
            assert_eq!(Some(app_bar_icon_theme(&tester)), theme.icon_theme);
            assert_eq!(
                Some(app_bar_actions_icon_theme(&tester)),
                theme.actions_icon_theme
            );
            assert_eq!(
                app_bar_icon_color(&tester),
                theme
                    .actions_icon_theme
                    .as_ref()
                    .and_then(|icon_theme| icon_theme.color.as_ref())
                    .map(|color| color.color())
            );
            assert_eq!(Some(app_bar_text_style(&tester)), theme.toolbar_text_style);
        });
    }

    fn themed_toolbar<R>(
        theme: fn() -> AppBarThemeData,
        platform: TargetPlatform,
        app_bar: fn() -> AppBar,
        read: fn(&NavigationToolbar) -> R,
    ) -> R
    where
        R: Send,
    {
        let mut result = None;
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(
                &mut tester,
                move |app| {
                    ThemeData::builder()
                        .platform(platform)
                        .app_bar_theme(theme())
                        .build(app)
                },
                move |_app| scaffold_with(app_bar()).into_widget(),
            );
            result = Some(navigation_toolbar(&tester, read));
        });
        result.expect("the case ran")
    }

    #[test]
    fn the_app_bar_uses_the_themes_center_title_when_it_gives_none() {
        let center_middle = themed_toolbar(
            || AppBarThemeData::new().center_title(true),
            TargetPlatform::Android,
            || AppBar::new().title(Text::new("Title")),
            |toolbar| toolbar.center_middle,
        );
        assert!(center_middle);
    }

    #[test]
    fn the_app_bars_center_title_takes_priority_over_the_themes() {
        let center_middle = themed_toolbar(
            || AppBarThemeData::new().center_title(true),
            TargetPlatform::Android,
            || AppBar::new().title(Text::new("Title")).center_title(false),
            |toolbar| toolbar.center_middle,
        );
        // The AppBar.centerTitle should be used instead of AppBarThemeData.centerTitle.
        assert!(!center_middle);
    }

    #[test]
    fn center_title_adapts_to_the_platform_without_one_in_the_theme() {
        let center_middle = themed_toolbar(
            AppBarThemeData::new,
            TargetPlatform::IOS,
            || AppBar::new().title(Text::new("Title")),
            |toolbar| toolbar.center_middle,
        );
        // When ThemeData.platform is TargetPlatform.iOS, and AppBarThemeData is null,
        // the value of NavigationToolBar.centerMiddle should be true.
        assert!(center_middle);
    }

    #[test]
    fn the_app_bar_uses_the_themes_title_spacing() {
        let middle_spacing = themed_toolbar(
            || AppBarThemeData::new().title_spacing(10.0),
            TargetPlatform::Android,
            || AppBar::new().title(Text::new("Title")),
            |toolbar| toolbar.middle_spacing,
        );
        assert_eq!(middle_spacing, 10.0);
    }

    #[test]
    fn the_app_bars_title_spacing_takes_priority_over_the_themes() {
        let middle_spacing = themed_toolbar(
            || AppBarThemeData::new().title_spacing(10.0),
            TargetPlatform::Android,
            || AppBar::new().title(Text::new("Title")).title_spacing(40.0),
            |toolbar| toolbar.middle_spacing,
        );
        assert_eq!(middle_spacing, 40.0);
    }

    /// The leading widget's `ConstrainedBox` constraints, Dart's
    /// `(navToolBar.leading! as ConstrainedBox).constraints`.
    fn leading_constraints(toolbar: &NavigationToolbar) -> (f64, f64) {
        let leading = toolbar.leading.as_ref().expect("a leading widget");
        let constraints = downcast_widget::<ConstrainedBox>(&**leading)
            .expect("a ConstrainedBox")
            .constraints;
        (constraints.min_width, constraints.max_width)
    }

    #[test]
    fn the_app_bar_uses_the_themes_leading_width() {
        let (min_width, max_width) = themed_toolbar(
            || AppBarThemeData::new().leading_width(80.0),
            TargetPlatform::Android,
            || AppBar::new().leading(Icon::new(Some(Icons::chevron_left()))),
            leading_constraints,
        );
        assert_eq!(max_width, 80.0);
        assert_eq!(min_width, 80.0);
    }

    #[test]
    fn the_app_bars_leading_width_takes_priority_over_the_themes() {
        let (min_width, max_width) = themed_toolbar(
            || AppBarThemeData::new().leading_width(80.0),
            TargetPlatform::Android,
            || {
                AppBar::new()
                    .leading(Icon::new(Some(Icons::chevron_left())))
                    .leading_width(40.0)
            },
            leading_constraints,
        );
        assert_eq!(max_width, 40.0);
        assert_eq!(min_width, 40.0);
    }

    /// Dart's 'AppBar.leading size with custom IconButton', '.. BackButton', '.. CloseButton'
    /// and '.. DrawerButton': `leading` built with `key` and a press callback, and `W` the
    /// widget Dart finds by type.
    fn leading_size_case<W: 'static>(leading: fn(KeyRef) -> WidgetRef) {
        with_main_thread_stack(|| {
            const TITLE_SPACING: f64 = 16.0;
            let mut tester = tester();
            let leading_key = GlobalKey::new();
            let title_key = GlobalKey::new();
            pump_app(&mut tester, m3, |_app| {
                scaffold_with(
                    AppBar::new()
                        .leading(leading(leading_key.clone().into()))
                        .center_title(false)
                        .title(Text::new("Title").key(title_key.clone().into())),
                )
                .into_widget()
            });

            let button = tester.find_by_type::<W>()[0];
            let button_rect = rect_of(&tester, button);
            assert_eq!(button_rect.size(), Size::new(48.0, 48.0));

            let mut gesture = WidgetTester::mouse(1, 0);
            tester.add_pointer(&mut gesture, Offset::ZERO);
            tester.move_to(&mut gesture, button_rect.center(), false);
            tester.pump_and_settle();
            let on_surface = ThemeData::new(&tester.cell.borrow())
                .color_scheme
                .on_surface;
            #[expect(deprecated, reason = "Dart's case calls the deprecated `withOpacity`")]
            let hover_color = on_surface.with_opacity(0.08);
            let painted = tester.painted(tester.render_box_of(button).as_object());
            assert!(
                painted.contains(&crate::test_support::Drawn::Rect(
                    hover_color.to_argb32(),
                    Rect::from_ltrb(0.0, 0.0, 40.0, 40.0),
                )),
                "{painted:?}"
            );

            // Get the offset of the Center widget that wraps the IconButton.
            let center = tester.ancestor_of_type::<waft_widgets::Center>(button);
            let back_button_right = rect_of(&tester, center).right;
            let title_left = key_rect(&tester, &title_key).left;
            assert_eq!(title_left, back_button_right + TITLE_SPACING);
        });
    }

    #[test]
    fn a_custom_icon_button_leading_is_48_square_and_centered() {
        leading_size_case::<IconButton>(|key| {
            IconButton::new(Some(Listener::new(|_| {})), Icon::new(Some(Icons::menu())))
                .key(key)
                .into_widget()
        });
    }

    #[test]
    fn a_custom_back_button_leading_is_48_square_and_centered() {
        leading_size_case::<BackButton>(|key| {
            BackButton::new()
                .key(key)
                .on_pressed(Listener::new(|_| {}))
                .into_widget()
        });
    }

    #[test]
    fn a_custom_close_button_leading_is_48_square_and_centered() {
        leading_size_case::<CloseButton>(|key| {
            CloseButton::new()
                .key(key)
                .on_pressed(Listener::new(|_| {}))
                .into_widget()
        });
    }

    #[test]
    fn a_custom_drawer_button_leading_is_48_square_and_centered() {
        leading_size_case::<DrawerButton>(|key| {
            DrawerButton::new()
                .key(key)
                .on_pressed(Listener::new(|_| {}))
                .into_widget()
        });
    }
}
