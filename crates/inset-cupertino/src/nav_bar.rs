//! Flutter counterpart: `cupertino/nav_bar.dart`.

use std::any::Any;
use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use inset_animation::{
    Animatable, Animation, AnimationBehavior, AnimationController, AnimationStatus,
    AnimationStatusListener, AnimationWithParent, AnyAnimation, Cubic, Curve, CurvedAnimation,
    Curves, Interval, RectTween, TweenLerp,
};
use inset_embedder::{
    Brightness, Color, ImageFilter, Matrix4, Offset, Rect, Size, SystemUiOverlayStyle,
    TextBaseline, TextDirection, clamp_double,
};
use inset_foundation::{App, Handle, Listenable, Listener, ValueChanged, ValueListenable};
use inset_painting::{
    Alignment, AlignmentGeometry, AnyColor, AxisDirection, Border, BorderSide, BorderStyle,
    BoxDecoration, EdgeInsetsDirectional, EdgeInsetsGeometry, TextOverflow, TextScaler, TextSpan,
    TextStyle,
};
use inset_rendering::{
    AnyRenderBox, AnyRenderObject, BoxConstraints, BoxHitTestResult, BoxParentData, ContainerLayer,
    MainAxisSize, OverScrollHeaderStretchConfiguration, PaintingContext, RelativeRect,
    RenderAnimatedOpacity, RenderAnimatedOpacityMixin, RenderBox, RenderBoxData, RenderHandle,
    RenderObject, RenderObjectData, RenderObjectWithChildData, RenderObjectWithChildMixin,
    RenderShiftedBox, TransformLayer,
};
use inset_scheduler::{Ticker, TickerCallback, TickerProviderObject};
use inset_widgets::{
    AbsorbPointer, Align, AnimatedBuilder, AnimatedOpacity, AnimatedWidget, AnnotatedRegion,
    AnyModalRoute, BackdropFilter, BuildContext, Builder, ClipRect, Column, ConstrainedBox,
    DecoratedBox, DefaultTextStyle, DefaultTextStyleTransition, Directionality, Expanded,
    FadeTransition, Flexible, FocusableActionDetector, GestureDetector, GlobalKey, Hero,
    HeroFlightDirection, HeroTagRef, IconTheme, IconThemeData, IntoWidget, KeyRef, KeyedSubtree,
    LayoutBuilder, MediaQuery, NavigationToolbar, Navigator, NavigatorState, Opacity, Orientation,
    Padding, PositionedTransition, PreferredSizeWidget, PreferredSizeWidgetRef, RectTweenObject,
    RelativeRectTween, RenderObjectWidget, Row, SafeArea, ScaleTransition, ScrollNotification,
    ScrollNotificationCallback, ScrollNotificationObserver, ScrollNotificationObserverState,
    ScrollUpdateNotification, Scrollable, ScrollableState, SingleChildRenderObjectWidget, SizedBox,
    SliverPersistentHeader, SliverPersistentHeaderDelegate, Stack, StandardComponentType, State,
    StateData, StatefulWidget, StatelessWidget, Text, TextStyleTween, TickerProviderStateMixin,
    TickerProviderStateMixinData, Transform, ValueListenableBuilder, ViewportNotificationMixin,
    Visibility, WidgetRef, downcast_widget,
};

use crate::button::CupertinoButton;
use crate::colors::CupertinoDynamicColor;
use crate::constants::K_MIN_INTERACTIVE_DIMENSION_CUPERTINO;
use crate::icons::CupertinoIcons;
use crate::localizations::CupertinoLocalizations;
use crate::page_scaffold::{CupertinoPageScaffoldBackgroundColor, ObstructingPreferredSizeWidget};
use crate::route::CupertinoRouteTransition;
use crate::sheet::CupertinoSheetRoute;
use crate::theme::CupertinoTheme;

// ---------------------------------------------------------------------------------------------
// Constants and file-level helpers

/// Modes that determine how to display the navigation bar's bottom in relation to scroll events.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavigationBarBottomMode {
    /// Enable hiding the bottom in response to scrolling.
    ///
    /// As scrolling starts, the large title stays pinned while the bottom resizes until it is
    /// completely consumed. Then, the large title scrolls under the persistent navigation bar.
    Automatic,

    /// Always display the bottom regardless of the scroll activity.
    ///
    /// When scrolled, the bottom stays pinned while the large title scrolls under the
    /// persistent navigation bar.
    Always,
}

/// Standard iOS navigation bar height without the status bar.
///
/// This height is constant and independent of accessibility as it is in iOS.
const K_NAV_BAR_PERSISTENT_HEIGHT: f64 = K_MIN_INTERACTIVE_DIMENSION_CUPERTINO;

/// Size increase from expanding the navigation bar into an iOS-11-style large title
/// configuration in a `CustomScrollView`.
const K_NAV_BAR_LARGE_TITLE_HEIGHT_EXTENSION: f64 = 52.0;

/// Number of logical pixels scrolled down before the title text is transferred from the normal
/// navigation bar to a big title below the navigation bar.
const K_NAV_BAR_SHOW_LARGE_TITLE_THRESHOLD: f64 = 10.0;

/// Number of logical pixels scrolled during which the navigation bar's background fades in or
/// out.
///
/// Eyeballed on the native Settings app on an iPhone 15 simulator running iOS 17.4.
const K_NAV_BAR_SCROLL_UNDER_ANIMATION_EXTENT: f64 = 10.0;

const K_NAV_BAR_EDGE_PADDING: f64 = 16.0;

const K_NAV_BAR_BOTTOM_PADDING: f64 = 8.0;

const K_NAV_BAR_BACK_BUTTON_TAP_WIDTH: f64 = 50.0;

/// The minimum text scale to apply to contents of the nav bar which can scale to a size less
/// than the default, such as the large title.
///
/// Eyeballed on an iPhone 15 simulator running iOS 17.5.
const K_MIN_SCALE_FACTOR: f64 = 0.9;

/// The maximum text scale to apply to contents of the nav bar, except the large title which can
/// grow larger but is damped.
///
/// Calculated on an iPhone 15 simulator running iOS 17.5.
const K_MAX_SCALE_FACTOR: f64 = 1.235;

/// The damping ratio applied to reduce the rate at which the large title scales.
///
/// Eyeballed on an iPhone 15 simulator running iOS 17.5.
const K_LARGE_TITLE_SCALE_DAMPING_RATIO: f64 = 3.0;

/// The width of the 'Cancel' button if the search field in a
/// [`CupertinoSliverNavigationBar::search`] is active.
///
/// Eyeballed on an iPhone 15 simulator running iOS 17.5.
const K_SEARCH_FIELD_CANCEL_BUTTON_WIDTH: f64 = 67.0;

/// The height of the unscaled search field used in a [`CupertinoSliverNavigationBar::search`].
const K_SEARCH_FIELD_HEIGHT: f64 = 36.0;

/// The duration of the animation when the search field in
/// [`CupertinoSliverNavigationBar::search`] is tapped.
const K_NAV_BAR_SEARCH_DURATION: Duration = Duration::from_millis(300);

/// The curve of the animation when the search field in [`CupertinoSliverNavigationBar::search`]
/// is tapped.
fn k_nav_bar_search_curve() -> Rc<dyn Curve> {
    Curves::ease_in_out()
}

/// Title text transfer fade.
const K_NAV_BAR_TITLE_FADE_DURATION: Duration = Duration::from_millis(150);

const K_DEFAULT_NAV_BAR_BORDER_COLOR: Color = Color::new(0x4D000000);

/// Dart's `Border(bottom:)`: only the bottom side is drawn.
const fn nav_bar_border(color: Color) -> Border {
    Border {
        top: BorderSide::NONE,
        right: BorderSide::NONE,
        bottom: BorderSide {
            color,
            // 0.0 means one physical pixel
            width: 0.0,
            style: BorderStyle::Solid,
            stroke_align: BorderSide::STROKE_ALIGN_INSIDE,
        },
        left: BorderSide::NONE,
    }
}

const K_DEFAULT_NAV_BAR_BORDER: Border = nav_bar_border(K_DEFAULT_NAV_BAR_BORDER_COLOR);

const K_TRANSPARENT_NAV_BAR_BORDER: Border = nav_bar_border(Color::new(0x00000000));

/// The curve of the animation of the top nav bar regardless of push/pop direction in the hero
/// transition between two nav bars.
///
/// Eyeballed on an iPhone 15 Pro simulator running iOS 17.5.
const K_TOP_NAV_BAR_HEADER_TRANSITION_CURVE: Cubic = Cubic::new(0.0, 0.45, 0.45, 0.98);

/// The curve of the animation of the bottom nav bar regardless of push/pop direction in the hero
/// transition between two nav bars.
///
/// Eyeballed on an iPhone 15 Pro simulator running iOS 17.5.
const K_BOTTOM_NAV_BAR_HEADER_TRANSITION_CURVE: Cubic = Cubic::new(0.05, 0.90, 0.90, 0.95);

/// There's a single tag for all instances of navigation bars because they can all transition
/// between each other (per `Navigator`) via `Hero` transitions.
///
/// Dart's `const _HeroTag _defaultHeroTag`; a shared value here, so `Rc::ptr_eq` reproduces
/// Dart's `identical` as well as its `==`.
fn default_hero_tag() -> HeroTagRef {
    thread_local! {
        static TAG: HeroTagRef = Rc::new(HeroTag { navigator: None });
    }
    TAG.with(Rc::clone)
}

/// Dart's `_HeroTag`: the identity all cupertino navigation bars of one `Navigator` share.
#[derive(Debug, PartialEq, Eq, Hash)]
struct HeroTag {
    navigator: Option<Handle<NavigatorState>>,
}

/// An `AnimatedWidget` that imposes a fixed size on its child widget, and shifts the child
/// widget in the parent stack, driven by its `offset_animation` property.
#[derive(Debug)]
struct FixedSizeSlidingTransition {
    /// Whether the writing direction used in the navigation bar transition is left-to-right.
    is_ltr: bool,

    /// The animated offset from the top-leading corner of the stack.
    ///
    /// When `is_ltr` is true, the `Offset` is the position of the child widget in the stack
    /// render box's regular coordinate space.
    ///
    /// When `is_ltr` is false, the coordinate system is flipped around the horizontal axis and
    /// the origin is set to the top right corner of the render boxes. In other words, this
    /// parameter describes the offset from the top right corner of the stack, to the top right
    /// corner of the child widget, and the x-axis runs right to left.
    offset_animation: AnyAnimation<Offset>,

    /// The fixed width to impose on `child`.
    width: f64,

    /// The fixed height to impose on `child`.
    height: f64,

    child: WidgetRef,
}

impl AnimatedWidget for FixedSizeSlidingTransition {
    fn listenable(&self) -> Rc<dyn inset_foundation::Listenable> {
        Rc::new(self.offset_animation)
    }

    fn build(&self, app: &mut App, _context: BuildContext) -> WidgetRef {
        let value = self.offset_animation.value(app);
        let mut positioned = inset_widgets::Positioned::new(self.child.clone())
            .top(value.dy())
            .width(self.width)
            .height(self.height);
        positioned = if self.is_ltr {
            positioned.left(value.dx())
        } else {
            positioned.right(value.dx())
        };
        positioned.into_widget()
    }
}

/// Returns `child` wrapped with background and a bottom border if background color is opaque.
/// Otherwise, also blur with [`BackdropFilter`].
///
/// When `update_system_ui_overlay` is true, the nav bar will update the OS status bar's color
/// theme based on the background color of the nav bar.
fn wrap_with_background(
    border: Option<Border>,
    background_color: AnyColor,
    brightness: Option<Brightness>,
    child: WidgetRef,
    update_system_ui_overlay: bool,
    enable_background_filter_blur: bool,
) -> WidgetRef {
    let mut result = child;
    if update_system_ui_overlay {
        let is_dark = background_color.compute_luminance() < 0.179;
        let new_brightness = brightness.unwrap_or(if is_dark {
            Brightness::Dark
        } else {
            Brightness::Light
        });
        let overlay_style = match new_brightness {
            Brightness::Dark => SystemUiOverlayStyle::LIGHT,
            Brightness::Light => SystemUiOverlayStyle::DARK,
        };
        // `SystemUiOverlayStyle::LIGHT` and `SystemUiOverlayStyle::DARK` set some system
        // navigation bar properties, which are used if there is no `AnnotatedRegion` on the
        // bottom of the screen. For backward compatibility, create a `SystemUiOverlayStyle`
        // without the system navigation bar properties.
        result = AnnotatedRegion::new(
            result,
            SystemUiOverlayStyle {
                status_bar_color: overlay_style.status_bar_color,
                status_bar_brightness: overlay_style.status_bar_brightness,
                status_bar_icon_brightness: overlay_style.status_bar_icon_brightness,
                system_status_bar_contrast_enforced: overlay_style
                    .system_status_bar_contrast_enforced,
                ..SystemUiOverlayStyle::new()
            },
        )
        .into_widget();
    }
    let child_with_background = DecoratedBox::new({
        let mut decoration = BoxDecoration::new().color(background_color.clone());
        if let Some(border) = border {
            decoration = decoration.border(border);
        }
        decoration
    })
    .child(result);

    ClipRect::new()
        .child(
            BackdropFilter::filter(ImageFilter::blur(10.0, 10.0))
                .enabled(alpha(&background_color) != 0xFF && enable_background_filter_blur)
                .child(child_with_background),
        )
        .into_widget()
}

/// Dart's `Color.alpha`, which nav_bar.dart still reads although it is deprecated upstream.
#[expect(deprecated, reason = "the getter Dart's `backgroundColor.alpha` reads")]
fn alpha(color: &AnyColor) -> i32 {
    color.alpha()
}

fn damp_scale_factor(scaled_font_size: f64, unscaled_font_size: f64, damping_ratio: f64) -> f64 {
    let scale_factor = scaled_font_size / unscaled_font_size;
    if scale_factor < 1.0 {
        K_MIN_SCALE_FACTOR.max(scale_factor)
    } else {
        1.0 + ((scale_factor - 1.0) / damping_ratio)
    }
}

/// Whether the current route supports nav bar hero transitions from or to.
fn is_transitionable(app: &mut App, context: BuildContext) -> bool {
    let route = AnyModalRoute::of(app, context);

    // Fullscreen dialogs never transitions their nav bar with other push-style pages' nav bars
    // or with other fullscreen dialog pages on the way in or on the way out.
    route.is_some_and(|route| {
        route.as_route().as_page_route(app).is_some() && !route.fullscreen_dialog(app)
    }) && !CupertinoSheetRoute::has_parent_sheet(app, context)
}

// ---------------------------------------------------------------------------------------------
// CupertinoNavigationBar

/// An iOS-styled navigation bar.
///
/// The navigation bar is a toolbar that minimally consists of a widget, normally a page title.
///
/// It also supports [`leading`](Self::leading) and [`trailing`](Self::trailing) widgets on
/// either end of the toolbar, typically for actions and navigation.
///
/// The [`leading`](Self::leading) widget will automatically be a back chevron icon button (or a
/// cancel button in case of a fullscreen dialog) to pop the current route if none is provided
/// and [`automatically_imply_leading`](Self::automatically_imply_leading) is true (true by
/// default).
///
/// This toolbar should be placed at top of the screen where it will automatically account for
/// the OS's status bar.
///
/// If the given [`background_color`](Self::background_color)'s opacity is not 1.0 (which is the
/// case by default), it will produce a blurring effect to the content behind it.
///
/// ### Layout options
///
/// While the [`CupertinoSliverNavigationBar`] can dynamically change size and layout in response
/// to scrolling, this static version can reflect the same large (expanded) layout, or the small
/// (collapsed) layout.
///
/// [`CupertinoNavigationBar::new`] will display the collapsed version of the
/// [`CupertinoSliverNavigationBar`]. The [`middle`](Self::middle) widget will automatically be a
/// title text from the current [`CupertinoPageRoute`](crate::CupertinoPageRoute) if none is
/// provided and [`automatically_imply_middle`](Self::automatically_imply_middle) is true (true
/// by default).
///
/// Using [`CupertinoNavigationBar::large`] will display the expanded version of
/// [`CupertinoSliverNavigationBar`]. The [`large_title`](Self::large_title) widget will
/// automatically be a title text from the current
/// [`CupertinoPageRoute`](crate::CupertinoPageRoute) if none is provided and
/// [`automatically_imply_title`](Self::automatically_imply_title) is true (true by default).
///
/// ### Transitions
///
/// When [`transition_between_routes`](Self::transition_between_routes) is true, this navigation
/// bar will transition on top of the routes instead of inside them if the route being
/// transitioned to also has a [`CupertinoNavigationBar`] or a [`CupertinoSliverNavigationBar`]
/// with `transition_between_routes` set to true. If `transition_between_routes` is true, none of
/// the widget parameters can contain a key in its subtree since that widget will exist in
/// multiple places in the tree simultaneously.
///
/// By default, only one [`CupertinoNavigationBar`] or [`CupertinoSliverNavigationBar`] should be
/// present in each `PageRoute` to support the default transitions. Use
/// [`transition_between_routes`](Self::transition_between_routes) or
/// [`hero_tag`](Self::hero_tag) to customize the transition behavior for multiple navigation
/// bars per route.
///
/// When used in a [`CupertinoPageScaffold`](crate::CupertinoPageScaffold), its `navigation_bar`
/// disables text scaling to match the native iOS behavior. To override this behavior, wrap each
/// of the navigation bar's components inside a `MediaQuery` with the desired `TextScaler`.
///
/// See also:
///
///  * [`CupertinoPageScaffold`](crate::CupertinoPageScaffold), a page layout helper typically
///    hosting the [`CupertinoNavigationBar`].
///  * [`CupertinoSliverNavigationBar`] for a navigation bar to be placed in a scrolling list and
///    that supports iOS-11-style large titles.
#[derive(Debug)]
pub struct CupertinoNavigationBar {
    key: Option<KeyRef>,

    /// The navigation bar's title, when using [`CupertinoNavigationBar::large`].
    ///
    /// If null and [`automatically_imply_title`](Self::automatically_imply_title) is true, an
    /// appropriate `Text` title will be created if the current route is a
    /// [`CupertinoPageRoute`](crate::CupertinoPageRoute) and has a `title`.
    ///
    /// This property is null for [`CupertinoNavigationBar::new`], which shows a collapsed
    /// navigation bar and uses [`middle`](Self::middle) for the title instead.
    pub large_title: Option<WidgetRef>,

    /// Widget to place at the start of the navigation bar. Normally a back button for a normal
    /// page or a cancel button for full page dialogs.
    ///
    /// If null and [`automatically_imply_leading`](Self::automatically_imply_leading) is true,
    /// an appropriate button will be automatically created.
    pub leading: Option<WidgetRef>,

    /// Controls whether we should try to imply the leading widget if null.
    ///
    /// If true and [`leading`](Self::leading) is null, automatically try to deduce what the
    /// leading widget should be. If the leading widget is not null, this parameter has no
    /// effect.
    ///
    /// Specifically this navigation bar will:
    ///
    /// 1. Show a 'Cancel' button if the current route is a `fullscreen_dialog`.
    /// 2. Show a back chevron with [`previous_page_title`](Self::previous_page_title) if
    ///    `previous_page_title` is not null.
    /// 3. Show a back chevron with the previous route's `title` if the current route is a
    ///    [`CupertinoPageRoute`](crate::CupertinoPageRoute) and the previous route is also a
    ///    [`CupertinoPageRoute`](crate::CupertinoPageRoute).
    pub automatically_imply_leading: bool,

    /// Controls whether we should try to imply the middle widget if null.
    ///
    /// If true and [`middle`](Self::middle) is null, automatically fill in a `Text` widget with
    /// the current route's `title` if the route is a
    /// [`CupertinoPageRoute`](crate::CupertinoPageRoute). If the middle widget is not null, this
    /// parameter has no effect.
    pub automatically_imply_middle: bool,

    /// Manually specify the previous route's title when automatically implying the leading back
    /// button.
    ///
    /// Overrides the text shown with the back chevron instead of automatically showing the
    /// previous [`CupertinoPageRoute`](crate::CupertinoPageRoute)'s `title` when
    /// [`automatically_imply_leading`](Self::automatically_imply_leading) is true.
    ///
    /// Has no effect when [`leading`](Self::leading) is not null or if
    /// `automatically_imply_leading` is false.
    pub previous_page_title: Option<String>,

    /// The navigation bar's default title.
    ///
    /// If null and [`automatically_imply_middle`](Self::automatically_imply_middle) is true, an
    /// appropriate `Text` title will be created if the current route is a
    /// [`CupertinoPageRoute`](crate::CupertinoPageRoute) and has a `title`.
    ///
    /// This property is null for [`CupertinoNavigationBar::large`], which shows an expanded
    /// navigation bar and uses [`large_title`](Self::large_title) instead.
    pub middle: Option<WidgetRef>,

    /// Widget to place at the end of the navigation bar. Normally additional actions taken on
    /// the page such as a search or edit function.
    pub trailing: Option<WidgetRef>,

    /// The background color of the navigation bar. If it contains transparency, the tab bar will
    /// automatically produce a blurring effect to the content behind it. This behavior can be
    /// disabled by setting
    /// [`enable_background_filter_blur`](Self::enable_background_filter_blur) to false.
    ///
    /// By default, the navigation bar's background is visible only when scrolled under. This
    /// behavior can be controlled with
    /// [`automatic_background_visibility`](Self::automatic_background_visibility).
    ///
    /// Defaults to [`CupertinoTheme`]'s `bar_background_color` if null.
    pub background_color: Option<AnyColor>,

    /// Whether the navigation bar appears transparent when no content is scrolled under.
    ///
    /// If this is true, the navigation bar's background color will be transparent until the
    /// content scrolls under it. If false, the navigation bar will always use
    /// [`background_color`](Self::background_color) as its background color.
    ///
    /// If the navigation bar is not a child of a
    /// [`CupertinoPageScaffold`](crate::CupertinoPageScaffold), this has no effect.
    ///
    /// This value defaults to true.
    pub automatic_background_visibility: bool,

    /// The brightness of the specified [`background_color`](Self::background_color).
    ///
    /// Setting this value changes the style of the system status bar. Typically used to increase
    /// the contrast ratio of the system status bar over `background_color`.
    ///
    /// If set to null, the value of the property will be inferred from the relative luminance of
    /// `background_color`.
    pub brightness: Option<Brightness>,

    /// Padding for the contents of the navigation bar.
    ///
    /// If null, the navigation bar will adopt the following defaults:
    ///
    ///  * Vertically, contents will be sized to the same height as the navigation bar itself
    ///    minus the status bar.
    ///  * Horizontally, padding will be 16 pixels according to iOS specifications unless the
    ///    leading widget is an automatically inserted back button, in which case the padding will
    ///    be 0.
    ///
    /// Vertical padding won't change the height of the nav bar.
    pub padding: Option<EdgeInsetsDirectional>,

    /// The border of the navigation bar. By default renders a single pixel bottom border side.
    ///
    /// If a border is null, the navigation bar will not display a border.
    pub border: Option<Border>,

    /// Whether to transition between navigation bars.
    ///
    /// When [`transition_between_routes`](Self::transition_between_routes) is true, this
    /// navigation bar will transition on top of the routes instead of inside it if the route
    /// being transitioned to also has a [`CupertinoNavigationBar`] or a
    /// [`CupertinoSliverNavigationBar`] with `transition_between_routes` set to true.
    ///
    /// This transition will also occur on edge back swipe gestures like on iOS but only if the
    /// previous page below has `maintain_state` set to true on the `PageRoute`.
    ///
    /// When set to true, only one navigation bar can be present per route unless
    /// [`hero_tag`](Self::hero_tag) is also set.
    ///
    /// This value defaults to true.
    pub transition_between_routes: bool,

    /// Whether to have a blur effect when a non-opaque background color is used.
    ///
    /// When [`enable_background_filter_blur`](Self::enable_background_filter_blur) is set to
    /// false, the blur effect will be disabled. The behaviour of `enable_background_filter_blur`
    /// will only be respected when
    /// [`automatic_background_visibility`](Self::automatic_background_visibility) is false or
    /// until content scrolls under the navbar.
    ///
    /// This value defaults to true.
    pub enable_background_filter_blur: bool,

    /// Tag for the navigation bar's `Hero` widget if
    /// [`transition_between_routes`](Self::transition_between_routes) is true.
    ///
    /// Defaults to a common tag between all [`CupertinoNavigationBar`] and
    /// [`CupertinoSliverNavigationBar`] instances of the same `Navigator`. With the default tag,
    /// all navigation bars of the same navigator can transition between each other as long as
    /// there's only one navigation bar per route.
    ///
    /// This tag can be overridden to manually handle having multiple navigation bars per route
    /// or to transition between multiple `Navigator`s.
    ///
    /// To disable `Hero` transitions for this navigation bar, set `transition_between_routes` to
    /// false.
    pub hero_tag: HeroTagRef,

    /// A widget to place at the bottom of the navigation bar.
    ///
    /// Only widgets that implement `PreferredSizeWidget` can be used at the bottom of a
    /// navigation bar.
    ///
    /// See also:
    ///
    ///  * `PreferredSize`, which can be used to give an arbitrary widget a preferred size.
    pub bottom: Option<PreferredSizeWidgetRef>,
}

impl CupertinoNavigationBar {
    /// Creates a static iOS style navigation bar, with a centered [`middle`](Self::middle) title.
    ///
    /// Similar to the collapsed state of [`CupertinoSliverNavigationBar`], which can dynamically
    /// change size in response to scrolling.
    ///
    /// See also:
    ///
    ///   * [`CupertinoNavigationBar::large`], which creates a static iOS style navigation bar
    ///     with a [`large_title`](Self::large_title), similar to the expanded state of
    ///     [`CupertinoSliverNavigationBar`].
    pub fn new() -> CupertinoNavigationBar {
        CupertinoNavigationBar {
            key: None,
            large_title: None,
            leading: None,
            automatically_imply_leading: true,
            automatically_imply_middle: true,
            previous_page_title: None,
            middle: None,
            trailing: None,
            border: Some(K_DEFAULT_NAV_BAR_BORDER),
            background_color: None,
            automatic_background_visibility: true,
            enable_background_filter_blur: true,
            brightness: None,
            padding: None,
            transition_between_routes: true,
            hero_tag: default_hero_tag(),
            bottom: None,
        }
    }

    /// Creates a static iOS style navigation bar, with a left aligned
    /// [`large_title`](Self::large_title).
    ///
    /// Similar to the expanded state of [`CupertinoSliverNavigationBar`], which can dynamically
    /// change size in response to scrolling.
    ///
    /// See also:
    ///
    ///   * [`CupertinoNavigationBar::new`], which creates a static iOS style navigation bar with
    ///     [`middle`](Self::middle), similar to the collapsed state of
    ///     [`CupertinoSliverNavigationBar`].
    pub fn large() -> CupertinoNavigationBar {
        CupertinoNavigationBar::new()
    }

    /// Dart `CupertinoNavigationBar(key:)`.
    pub fn key(mut self, key: KeyRef) -> CupertinoNavigationBar {
        self.key = Some(key);
        self
    }

    /// Dart `CupertinoNavigationBar.large(largeTitle:)`.
    pub fn large_title<K>(mut self, large_title: impl IntoWidget<K>) -> CupertinoNavigationBar {
        debug_assert!(self.middle.is_none());
        self.large_title = Some(large_title.into_widget());
        self
    }

    /// Dart `CupertinoNavigationBar(leading:)`.
    pub fn leading<K>(mut self, leading: impl IntoWidget<K>) -> CupertinoNavigationBar {
        self.leading = Some(leading.into_widget());
        self
    }

    /// Dart `CupertinoNavigationBar(automaticallyImplyLeading:)`.
    pub fn automatically_imply_leading(
        mut self,
        automatically_imply_leading: bool,
    ) -> CupertinoNavigationBar {
        self.automatically_imply_leading = automatically_imply_leading;
        self
    }

    /// Dart `CupertinoNavigationBar(automaticallyImplyMiddle:)`.
    pub fn automatically_imply_middle(
        mut self,
        automatically_imply_middle: bool,
    ) -> CupertinoNavigationBar {
        self.automatically_imply_middle = automatically_imply_middle;
        self
    }

    /// Dart `CupertinoNavigationBar.large(automaticallyImplyTitle:)`, which the large
    /// constructor forwards to `automaticallyImplyMiddle`.
    pub fn automatically_imply_title(
        self,
        automatically_imply_title: bool,
    ) -> CupertinoNavigationBar {
        self.automatically_imply_middle(automatically_imply_title)
    }

    /// Dart `CupertinoNavigationBar(previousPageTitle:)`.
    pub fn previous_page_title(
        mut self,
        previous_page_title: impl Into<String>,
    ) -> CupertinoNavigationBar {
        self.previous_page_title = Some(previous_page_title.into());
        self
    }

    /// Dart `CupertinoNavigationBar(middle:)`.
    pub fn middle<K>(mut self, middle: impl IntoWidget<K>) -> CupertinoNavigationBar {
        debug_assert!(self.large_title.is_none());
        self.middle = Some(middle.into_widget());
        self
    }

    /// Dart `CupertinoNavigationBar(trailing:)`.
    pub fn trailing<K>(mut self, trailing: impl IntoWidget<K>) -> CupertinoNavigationBar {
        self.trailing = Some(trailing.into_widget());
        self
    }

    /// Dart `CupertinoNavigationBar(border:)`.
    pub fn border(mut self, border: Option<Border>) -> CupertinoNavigationBar {
        self.border = border;
        self
    }

    /// Dart `CupertinoNavigationBar(backgroundColor:)`.
    pub fn background_color(
        mut self,
        background_color: impl Into<AnyColor>,
    ) -> CupertinoNavigationBar {
        self.background_color = Some(background_color.into());
        self
    }

    /// Dart `CupertinoNavigationBar(automaticBackgroundVisibility:)`.
    pub fn automatic_background_visibility(
        mut self,
        automatic_background_visibility: bool,
    ) -> CupertinoNavigationBar {
        self.automatic_background_visibility = automatic_background_visibility;
        self
    }

    /// Dart `CupertinoNavigationBar(enableBackgroundFilterBlur:)`.
    pub fn enable_background_filter_blur(
        mut self,
        enable_background_filter_blur: bool,
    ) -> CupertinoNavigationBar {
        self.enable_background_filter_blur = enable_background_filter_blur;
        self
    }

    /// Dart `CupertinoNavigationBar(brightness:)`.
    pub fn brightness(mut self, brightness: Brightness) -> CupertinoNavigationBar {
        self.brightness = Some(brightness);
        self
    }

    /// Dart `CupertinoNavigationBar(padding:)`.
    pub fn padding(mut self, padding: EdgeInsetsDirectional) -> CupertinoNavigationBar {
        self.padding = Some(padding);
        self
    }

    /// Dart `CupertinoNavigationBar(transitionBetweenRoutes:)`.
    pub fn transition_between_routes(
        mut self,
        transition_between_routes: bool,
    ) -> CupertinoNavigationBar {
        self.transition_between_routes = transition_between_routes;
        debug_assert!(
            !self.transition_between_routes || Rc::ptr_eq(&self.hero_tag, &default_hero_tag()),
            "Cannot specify a heroTag override if this navigation bar does not transition due to \
             transitionBetweenRoutes = false."
        );
        self
    }

    /// Dart `CupertinoNavigationBar(heroTag:)`.
    pub fn hero_tag(mut self, hero_tag: HeroTagRef) -> CupertinoNavigationBar {
        self.hero_tag = hero_tag;
        debug_assert!(
            !self.transition_between_routes || Rc::ptr_eq(&self.hero_tag, &default_hero_tag()),
            "Cannot specify a heroTag override if this navigation bar does not transition due to \
             transitionBetweenRoutes = false."
        );
        self
    }

    /// Dart `CupertinoNavigationBar(bottom:)`.
    pub fn bottom(mut self, bottom: PreferredSizeWidgetRef) -> CupertinoNavigationBar {
        self.bottom = Some(bottom);
        self
    }
}

impl Default for CupertinoNavigationBar {
    fn default() -> CupertinoNavigationBar {
        CupertinoNavigationBar::new()
    }
}

impl PreferredSizeWidget for CupertinoNavigationBar {
    fn preferred_size(&self) -> Size {
        let bottom_height = self
            .bottom
            .as_ref()
            .map_or(0.0, |bottom| bottom.preferred_size().height());

        let effective_large_height = if self.large_title.is_some() {
            K_NAV_BAR_LARGE_TITLE_HEIGHT_EXTENSION
        } else {
            0.0
        };

        Size::from_height(K_NAV_BAR_PERSISTENT_HEIGHT + bottom_height + effective_large_height)
    }
}

impl ObstructingPreferredSizeWidget for CupertinoNavigationBar {
    /// True if the navigation bar's background color has no transparency.
    fn should_fully_obstruct(&self, app: &mut App, context: BuildContext) -> bool {
        let background_color =
            CupertinoDynamicColor::maybe_resolve(self.background_color.as_ref(), app, context)
                .unwrap_or_else(|| CupertinoTheme::of(app, context).bar_background_color());
        alpha(&background_color) == 0xFF
    }
}

impl StatefulWidget for CupertinoNavigationBar {
    type State = CupertinoNavigationBarState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> CupertinoNavigationBarState {
        CupertinoNavigationBarState {
            state: StateData::new(),
            keys: NavigationBarStaticComponentsKeys::new(),
            scroll_notification_observer: None,
            scroll_notification_listener: None,
            scroll_animation_value: 0.0,
        }
    }
}

/// Dart's `_CupertinoNavigationBarState`.
///
/// A state class exists for the nav bar so that the keys of its sub-components don't change when
/// rebuilding the nav bar, causing the sub-components to lose their own states.
pub struct CupertinoNavigationBarState {
    state: StateData<CupertinoNavigationBar>,
    keys: Rc<NavigationBarStaticComponentsKeys>,
    scroll_notification_observer: Option<Handle<ScrollNotificationObserverState>>,
    scroll_notification_listener: Option<ScrollNotificationCallback>,
    scroll_animation_value: f64,
}

impl CupertinoNavigationBarState {
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
        let Some(notification) = (notification.as_any()).downcast_ref::<ScrollUpdateNotification>()
        else {
            return;
        };
        if notification.depth() != 0 {
            return;
        }
        let metrics = notification.metrics();
        let old_scroll_animation_value = app.get(self).scroll_animation_value;
        let mut scroll_extent = 0.0;
        match metrics.axis_direction() {
            // Scroll view is reversed
            AxisDirection::Up => scroll_extent = metrics.extent_after(),
            AxisDirection::Down => scroll_extent = metrics.extent_before(),
            // Scrolled under is only supported in the vertical axis, and should not be altered
            // based on horizontal notifications of the same predicate since it could be a 2D
            // scroller.
            AxisDirection::Right | AxisDirection::Left => {}
        }

        if (0.0..K_NAV_BAR_SCROLL_UNDER_ANIMATION_EXTENT).contains(&scroll_extent) {
            self.set_state(app, |state| {
                state.scroll_animation_value = clamp_double(
                    scroll_extent / K_NAV_BAR_SCROLL_UNDER_ANIMATION_EXTENT,
                    0.0,
                    1.0,
                );
            });
        } else if scroll_extent > K_NAV_BAR_SCROLL_UNDER_ANIMATION_EXTENT
            && old_scroll_animation_value != 1.0
        {
            self.set_state(app, |state| state.scroll_animation_value = 1.0);
        } else if scroll_extent <= 0.0 && old_scroll_animation_value != 0.0 {
            self.set_state(app, |state| state.scroll_animation_value = 0.0);
        }
    }
}

impl State for CupertinoNavigationBarState {
    type Widget = CupertinoNavigationBar;
    inset_widgets::state_accessors!();

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        let listener = self.scroll_notification_listener(app);
        if let Some(observer) = app.get(self).scroll_notification_observer {
            observer.remove_listener(app, &listener);
        }
        let context = self.context(app);
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
        // The static navigation bar does not expand or collapse (see
        // `CupertinoSliverNavigationBar`), it will either display the collapsed nav bar with
        // middle, or the expanded with large_title.
        debug_assert!(self.widget(app).middle.is_none() || self.widget(app).large_title.is_none());

        let widget_background_color = self.widget(app).background_color.clone();
        let background_color =
            CupertinoDynamicColor::maybe_resolve(widget_background_color.as_ref(), app, context)
                .unwrap_or_else(|| CupertinoTheme::of(app, context).bar_background_color());

        let parent_page_scaffold_background_color =
            CupertinoPageScaffoldBackgroundColor::maybe_of(app, context);

        let scroll_animation_value = app.get(self).scroll_animation_value;
        let automatic_background_visibility = self.widget(app).automatic_background_visibility;
        let border = self.widget(app).border;

        let initial_border =
            if automatic_background_visibility && parent_page_scaffold_background_color.is_some() {
                Some(K_TRANSPARENT_NAV_BAR_BORDER)
            } else {
                border
            };
        let effective_border = border
            .and_then(|border| Border::lerp(initial_border, Some(border), scroll_animation_value));

        let effective_background_color = match &parent_page_scaffold_background_color {
            Some(scaffold_color) if automatic_background_visibility => AnyColor::lerp(
                Some(scaffold_color),
                Some(&background_color),
                scroll_animation_value,
            )
            .map_or_else(|| background_color.clone(), AnyColor::from),
            _ => background_color.clone(),
        };

        let bottom = self.widget(app).bottom.clone();
        let bottom_height = bottom
            .as_ref()
            .map_or(0.0, |bottom| bottom.preferred_size().height());
        let persistent_height =
            K_NAV_BAR_PERSISTENT_HEIGHT + bottom_height + MediaQuery::padding_of(app, context).top;
        let large_height = persistent_height + K_NAV_BAR_LARGE_TITLE_HEIGHT_EXTENSION;

        let widget = self.widget(app);
        let large_title = widget.large_title.clone();
        let padding = widget.padding;
        let brightness = widget.brightness;
        let enable_background_filter_blur = widget.enable_background_filter_blur;
        let transition_between_routes = widget.transition_between_routes;
        let hero_tag = Rc::clone(&widget.hero_tag);
        let has_user_middle = widget.middle.is_some();
        let route = AnyModalRoute::of(app, context);
        let widget = self.widget(app);
        let arguments = NavigationBarStaticComponentsArguments {
            keys: Rc::clone(&app.get(self).keys),
            route,
            user_leading: widget.leading.clone(),
            automatically_imply_leading: widget.automatically_imply_leading,
            automatically_imply_title: widget.automatically_imply_middle,
            previous_page_title: widget.previous_page_title.clone(),
            user_middle: widget.middle.clone(),
            user_trailing: widget.trailing.clone(),
            padding,
            user_large_title: large_title.clone(),
            user_bottom: bottom.clone().map(IntoWidget::into_widget),
            large: large_title.is_some(),
            // This one does not scroll
            static_bar: true,
        };
        let components = Rc::new(NavigationBarStaticComponents::new(app, context, arguments));

        // Standard persistent components
        let mut nav_bar: WidgetRef = PersistentNavigationBar {
            components: Rc::clone(&components),
            padding,
            middle_visible: Some(large_title.is_none()),
        }
        .into_widget();

        nav_bar = if large_title.is_some() {
            // Large nav bar
            let mut children = vec![
                nav_bar,
                Expanded::new(
                    Padding::new(EdgeInsetsGeometry::directional(
                        K_NAV_BAR_EDGE_PADDING,
                        0.0,
                        0.0,
                        K_NAV_BAR_BOTTOM_PADDING,
                    ))
                    .child(
                        DefaultTextStyle::new(
                            CupertinoTheme::of(app, context)
                                .text_theme()
                                .nav_large_title_text_style(),
                            LargeTitle {
                                height: K_NAV_BAR_LARGE_TITLE_HEIGHT_EXTENSION,
                                child: components.large_title.clone(),
                            },
                        )
                        .max_lines(1)
                        .overflow(TextOverflow::Ellipsis),
                    ),
                )
                .into_widget(),
            ];
            if bottom.is_some() {
                children.push(
                    SizedBox::new()
                        .height(bottom_height)
                        .child(components.nav_bar_bottom.clone())
                        .into_widget(),
                );
            }
            ConstrainedBox::new(BoxConstraints::new().max_height(large_height))
                .child(Column::new().children(children))
                .into_widget()
        } else {
            // Small nav bar
            let mut children = vec![nav_bar];
            if bottom.is_some() {
                children.push(
                    SizedBox::new()
                        .height(bottom_height)
                        .child(components.nav_bar_bottom.clone())
                        .into_widget(),
                );
            }
            ConstrainedBox::new(BoxConstraints::new().max_height(persistent_height))
                .child(Column::new().children(children))
                .into_widget()
        };

        nav_bar = wrap_with_background(
            effective_border,
            effective_background_color.clone(),
            brightness,
            DefaultTextStyle::new(
                CupertinoTheme::of(app, context).text_theme().text_style(),
                nav_bar,
            )
            .into_widget(),
            true,
            enable_background_filter_blur,
        );

        if !transition_between_routes || !is_transitionable(app, context) {
            return nav_bar;
        }

        let keys = Rc::clone(&app.get(self).keys);
        // Get the context that might have a possibly changed `CupertinoTheme`.
        Builder::new(move |app: &mut App, context: BuildContext| {
            let text_theme = CupertinoTheme::of(app, context).text_theme();
            let tag: HeroTagRef = if Rc::ptr_eq(&hero_tag, &default_hero_tag()) {
                Rc::new(HeroTag {
                    navigator: Some(Navigator::of(app, context, false)),
                })
            } else {
                Rc::clone(&hero_tag)
            };
            Hero::new(
                tag,
                TransitionableNavigationBar {
                    key: Rc::new(keys.nav_bar_box_key.clone()),
                    components_keys: Rc::clone(&keys),
                    background_color: Some(effective_background_color.clone()),
                    back_button_text_style: text_theme.nav_action_text_style(),
                    title_text_style: text_theme.nav_title_text_style(),
                    large_title_text_style: Some(text_theme.nav_large_title_text_style()),
                    border: effective_border,
                    has_user_middle,
                    large_expanded: large_title.is_some(),
                    searchable: false,
                    automatic_background_visibility,
                    child: nav_bar.clone(),
                },
            )
            .create_rect_tween(Rc::new(linear_translate_with_largest_rect_size_tween))
            .placeholder_builder(Rc::new(nav_bar_hero_launch_pad_builder))
            .flight_shuttle_builder(Rc::new(nav_bar_hero_flight_shuttle_builder))
            .transition_on_user_gestures(true)
            .into_widget()
        })
        .into_widget()
    }
}

impl Debug for CupertinoNavigationBarState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CupertinoNavigationBarState")
    }
}

// ---------------------------------------------------------------------------------------------
// CupertinoSliverNavigationBar

/// An iOS-styled navigation bar with iOS-11-style large titles using slivers.
///
/// The [`CupertinoSliverNavigationBar`] must be placed in a sliver group such as the
/// `CustomScrollView`.
///
/// This navigation bar consists of two sections, a pinned static section on top and a sliding
/// section containing iOS-11-style large title below it.
///
/// It should be placed at top of the screen and automatically accounts for the iOS status bar.
///
/// This navigation bar is expanded only in portrait orientation. In landscape mode, the
/// navigation bar remains permanently collapsed. The navigation bar also collapses when
/// scrolling in portrait mode.
///
/// Minimally, a [`large_title`](Self::large_title) widget will appear in the middle of the app
/// bar when the sliver is collapsed and transfer to the area below in larger font when the
/// sliver is expanded. This expanded view will only trigger in portrait orientation, while in
/// landscape mode the bar will stay in its collapsed view.
///
/// For advanced uses, an optional [`middle`](Self::middle) widget can be supplied to show a
/// different widget in the middle of the navigation bar when the sliver is collapsed.
///
/// Like [`CupertinoNavigationBar`], it also supports a [`leading`](Self::leading) and
/// [`trailing`](Self::trailing) widget on the static section on top that remains while
/// scrolling.
///
/// The [`stretch`](Self::stretch) parameter determines whether the nav bar should stretch to
/// fill the over-scroll area. The nav bar can still expand and contract as the user scrolls, but
/// it will also stretch when the user over-scrolls if the `stretch` value is true. Defaults to
/// false.
///
/// See also:
///
///  * [`CupertinoNavigationBar`], an iOS navigation bar for use on non-scrolling pages.
pub struct CupertinoSliverNavigationBar {
    key: Option<KeyRef>,

    /// The navigation bar's title.
    ///
    /// This text will appear in the top static navigation bar when collapsed and below the
    /// navigation bar, in a larger font, when expanded.
    ///
    /// A suitable `DefaultTextStyle` is provided around this widget as it is moved around, to
    /// change its font size.
    ///
    /// If [`middle`](Self::middle) is null, then the `large_title` widget will be inserted into
    /// the tree in two places when transitioning from the collapsed state to the expanded state.
    /// It is therefore imperative that this subtree not contain any `GlobalKey`s, and that it not
    /// rely on maintaining state.
    ///
    /// If null and [`automatically_imply_title`](Self::automatically_imply_title) is true, an
    /// appropriate `Text` title will be created if the current route is a
    /// [`CupertinoPageRoute`](crate::CupertinoPageRoute) and has a `title`.
    pub large_title: Option<WidgetRef>,

    /// Widget to place at the start of the navigation bar. Normally a back button for a normal
    /// page or a cancel button for full page dialogs.
    ///
    /// This widget is visible in both collapsed and expanded states.
    pub leading: Option<WidgetRef>,

    /// Controls whether we should try to imply the leading widget if null.
    pub automatically_imply_leading: bool,

    /// Controls whether we should try to imply the [`large_title`](Self::large_title) widget if
    /// null.
    ///
    /// If true and `large_title` is null, automatically fill in a `Text` widget with the current
    /// route's `title` if the route is a [`CupertinoPageRoute`](crate::CupertinoPageRoute).
    pub automatically_imply_title: bool,

    /// Controls whether [`middle`](Self::middle) widget should always be visible (even in
    /// expanded state).
    ///
    /// If true (default) and `middle` is not null, the middle widget is always visible. If
    /// false, the middle widget is visible only in collapsed state if it is provided.
    ///
    /// This should be set to false if you only want to show `large_title` in expanded state and
    /// `middle` in collapsed state.
    pub always_show_middle: bool,

    /// Manually specify the previous route's title when automatically implying the leading back
    /// button.
    pub previous_page_title: Option<String>,

    /// A widget to place in the middle of the static navigation bar instead of the
    /// [`large_title`](Self::large_title).
    ///
    /// If [`always_show_middle`](Self::always_show_middle) is true, this widget is visible in
    /// both the collapsed and expanded states of the navigation bar. Else, it is visible only in
    /// the collapsed state.
    pub middle: Option<WidgetRef>,

    /// Widget to place at the end of the navigation bar.
    ///
    /// This widget is visible in both collapsed and expanded states.
    pub trailing: Option<WidgetRef>,

    /// The background color of the navigation bar.
    pub background_color: Option<AnyColor>,

    /// Whether the navigation bar appears transparent when no content is scrolled under.
    pub automatic_background_visibility: bool,

    /// Whether to have a blur effect when a non-opaque background color is used.
    pub enable_background_filter_blur: bool,

    /// The brightness of the specified [`background_color`](Self::background_color).
    pub brightness: Option<Brightness>,

    /// Padding for the contents of the navigation bar.
    pub padding: Option<EdgeInsetsDirectional>,

    /// The border of the navigation bar. By default renders a single pixel bottom border side.
    pub border: Option<Border>,

    /// Whether to transition between navigation bars.
    pub transition_between_routes: bool,

    /// Tag for the navigation bar's `Hero` widget if
    /// [`transition_between_routes`](Self::transition_between_routes) is true.
    pub hero_tag: HeroTagRef,

    /// A widget to place at the bottom of the large title or static navigation bar if there is
    /// no large title.
    ///
    /// Only widgets that implement `PreferredSizeWidget` can be used at the bottom of a
    /// navigation bar.
    pub bottom: Option<PreferredSizeWidgetRef>,

    /// Modes that determine how to display the navigation bar's [`bottom`](Self::bottom), or the
    /// search field in a [`CupertinoSliverNavigationBar::search`].
    ///
    /// If null, defaults to [`NavigationBarBottomMode::Automatic`] if either a `bottom` is
    /// provided or this is a [`CupertinoSliverNavigationBar::search`].
    pub bottom_mode: Option<NavigationBarBottomMode>,

    /// Called when the search field in [`CupertinoSliverNavigationBar::search`] is tapped,
    /// toggling between an active and an inactive search state.
    pub on_searchable_bottom_tap: Option<ValueChanged<bool>>,

    /// Whether the nav bar should stretch to fill the over-scroll area.
    ///
    /// The nav bar can still expand and contract as the user scrolls, but it will also stretch
    /// when the user over-scrolls if the `stretch` value is true.
    ///
    /// When set to true, the nav bar will prevent subsequent slivers from accessing overscrolls.
    /// This may be undesirable for using overscroll-based widgets like the
    /// `CupertinoSliverRefreshControl`.
    ///
    /// Defaults to false.
    pub stretch: bool,

    /// The search field used in [`CupertinoSliverNavigationBar::search`].
    ///
    /// The provided search field is constrained to a fixed height of 35 pixels in its inactive
    /// state, and [`K_MIN_INTERACTIVE_DIMENSION_CUPERTINO`] pixels in its active state.
    pub search_field: Option<WidgetRef>,

    /// True if [`CupertinoSliverNavigationBar::search`] was used.
    searchable: bool,
}

impl CupertinoSliverNavigationBar {
    /// Creates a navigation bar for scrolling lists.
    ///
    /// If [`automatically_imply_title`](Self::automatically_imply_title) is false, then the
    /// [`large_title`](Self::large_title) argument is required.
    pub fn new() -> CupertinoSliverNavigationBar {
        CupertinoSliverNavigationBar {
            key: None,
            large_title: None,
            leading: None,
            automatically_imply_leading: true,
            automatically_imply_title: true,
            always_show_middle: true,
            previous_page_title: None,
            middle: None,
            trailing: None,
            border: Some(K_DEFAULT_NAV_BAR_BORDER),
            background_color: None,
            automatic_background_visibility: true,
            enable_background_filter_blur: true,
            brightness: None,
            padding: None,
            transition_between_routes: true,
            hero_tag: default_hero_tag(),
            stretch: false,
            bottom: None,
            bottom_mode: None,
            on_searchable_bottom_tap: None,
            search_field: None,
            searchable: false,
        }
    }

    /// A navigation bar for scrolling lists that integrates a provided search field directly into
    /// the navigation bar.
    ///
    /// This search-enabled navigation bar is functionally equivalent to
    /// [`CupertinoSliverNavigationBar::new`], but with the addition of
    /// [`search_field`](Self::search_field), which sits at the bottom of the navigation bar.
    ///
    /// When the search field is tapped, [`leading`](Self::leading),
    /// [`trailing`](Self::trailing), [`middle`](Self::middle), and
    /// [`large_title`](Self::large_title) all collapse, causing the search field to animate to
    /// the 'top' of the navigation bar. A 'Cancel' button is presented next to the active search
    /// field, which when tapped, closes the search view, bringing the navigation bar back to its
    /// initial state.
    pub fn search<K>(search_field: impl IntoWidget<K>) -> CupertinoSliverNavigationBar {
        CupertinoSliverNavigationBar {
            search_field: Some(search_field.into_widget()),
            bottom_mode: Some(NavigationBarBottomMode::Automatic),
            searchable: true,
            ..CupertinoSliverNavigationBar::new()
        }
    }

    /// Dart `CupertinoSliverNavigationBar(key:)`.
    pub fn key(mut self, key: KeyRef) -> CupertinoSliverNavigationBar {
        self.key = Some(key);
        self
    }

    /// Dart `CupertinoSliverNavigationBar(largeTitle:)`.
    pub fn large_title<K>(
        mut self,
        large_title: impl IntoWidget<K>,
    ) -> CupertinoSliverNavigationBar {
        self.large_title = Some(large_title.into_widget());
        self
    }

    /// Dart `CupertinoSliverNavigationBar(leading:)`.
    pub fn leading<K>(mut self, leading: impl IntoWidget<K>) -> CupertinoSliverNavigationBar {
        self.leading = Some(leading.into_widget());
        self
    }

    /// Dart `CupertinoSliverNavigationBar(automaticallyImplyLeading:)`.
    pub fn automatically_imply_leading(
        mut self,
        automatically_imply_leading: bool,
    ) -> CupertinoSliverNavigationBar {
        self.automatically_imply_leading = automatically_imply_leading;
        self
    }

    /// Dart `CupertinoSliverNavigationBar(automaticallyImplyTitle:)`.
    pub fn automatically_imply_title(
        mut self,
        automatically_imply_title: bool,
    ) -> CupertinoSliverNavigationBar {
        self.automatically_imply_title = automatically_imply_title;
        debug_assert!(
            self.automatically_imply_title || self.large_title.is_some(),
            "No largeTitle has been provided but automaticallyImplyTitle is also false. Either \
             provide a largeTitle or set automaticallyImplyTitle to true."
        );
        self
    }

    /// Dart `CupertinoSliverNavigationBar(alwaysShowMiddle:)`.
    pub fn always_show_middle(mut self, always_show_middle: bool) -> CupertinoSliverNavigationBar {
        self.always_show_middle = always_show_middle;
        self
    }

    /// Dart `CupertinoSliverNavigationBar(previousPageTitle:)`.
    pub fn previous_page_title(
        mut self,
        previous_page_title: impl Into<String>,
    ) -> CupertinoSliverNavigationBar {
        self.previous_page_title = Some(previous_page_title.into());
        self
    }

    /// Dart `CupertinoSliverNavigationBar(middle:)`.
    pub fn middle<K>(mut self, middle: impl IntoWidget<K>) -> CupertinoSliverNavigationBar {
        self.middle = Some(middle.into_widget());
        self
    }

    /// Dart `CupertinoSliverNavigationBar(trailing:)`.
    pub fn trailing<K>(mut self, trailing: impl IntoWidget<K>) -> CupertinoSliverNavigationBar {
        self.trailing = Some(trailing.into_widget());
        self
    }

    /// Dart `CupertinoSliverNavigationBar(border:)`.
    pub fn border(mut self, border: Option<Border>) -> CupertinoSliverNavigationBar {
        self.border = border;
        self
    }

    /// Dart `CupertinoSliverNavigationBar(backgroundColor:)`.
    pub fn background_color(
        mut self,
        background_color: impl Into<AnyColor>,
    ) -> CupertinoSliverNavigationBar {
        self.background_color = Some(background_color.into());
        self
    }

    /// Dart `CupertinoSliverNavigationBar(automaticBackgroundVisibility:)`.
    pub fn automatic_background_visibility(
        mut self,
        automatic_background_visibility: bool,
    ) -> CupertinoSliverNavigationBar {
        self.automatic_background_visibility = automatic_background_visibility;
        self
    }

    /// Dart `CupertinoSliverNavigationBar(enableBackgroundFilterBlur:)`.
    pub fn enable_background_filter_blur(
        mut self,
        enable_background_filter_blur: bool,
    ) -> CupertinoSliverNavigationBar {
        self.enable_background_filter_blur = enable_background_filter_blur;
        self
    }

    /// Dart `CupertinoSliverNavigationBar(brightness:)`.
    pub fn brightness(mut self, brightness: Brightness) -> CupertinoSliverNavigationBar {
        self.brightness = Some(brightness);
        self
    }

    /// Dart `CupertinoSliverNavigationBar(padding:)`.
    pub fn padding(mut self, padding: EdgeInsetsDirectional) -> CupertinoSliverNavigationBar {
        self.padding = Some(padding);
        self
    }

    /// Dart `CupertinoSliverNavigationBar(transitionBetweenRoutes:)`.
    pub fn transition_between_routes(
        mut self,
        transition_between_routes: bool,
    ) -> CupertinoSliverNavigationBar {
        self.transition_between_routes = transition_between_routes;
        self
    }

    /// Dart `CupertinoSliverNavigationBar(heroTag:)`.
    pub fn hero_tag(mut self, hero_tag: HeroTagRef) -> CupertinoSliverNavigationBar {
        self.hero_tag = hero_tag;
        self
    }

    /// Dart `CupertinoSliverNavigationBar(stretch:)`.
    pub fn stretch(mut self, stretch: bool) -> CupertinoSliverNavigationBar {
        self.stretch = stretch;
        self
    }

    /// Dart `CupertinoSliverNavigationBar(bottom:)`; not for
    /// [`CupertinoSliverNavigationBar::search`], which owns the bottom.
    pub fn bottom(mut self, bottom: PreferredSizeWidgetRef) -> CupertinoSliverNavigationBar {
        debug_assert!(!self.searchable);
        self.bottom = Some(bottom);
        self
    }

    /// Dart `CupertinoSliverNavigationBar(bottomMode:)`.
    pub fn bottom_mode(
        mut self,
        bottom_mode: NavigationBarBottomMode,
    ) -> CupertinoSliverNavigationBar {
        self.bottom_mode = Some(bottom_mode);
        debug_assert!(
            self.searchable || self.bottom.is_some(),
            "A bottomMode was provided without a corresponding bottom."
        );
        self
    }

    /// Dart `CupertinoSliverNavigationBar.search(onSearchableBottomTap:)`.
    pub fn on_searchable_bottom_tap(
        mut self,
        on_searchable_bottom_tap: impl Fn(&mut App, bool) + 'static,
    ) -> CupertinoSliverNavigationBar {
        debug_assert!(self.searchable);
        self.on_searchable_bottom_tap = Some(Rc::new(on_searchable_bottom_tap));
        self
    }

    /// True if the navigation bar's background color has no transparency.
    pub fn opaque(&self) -> bool {
        self.background_color
            .as_ref()
            .is_some_and(|color| alpha(color) == 0xFF)
    }
}

impl Debug for CupertinoSliverNavigationBar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CupertinoSliverNavigationBar")
    }
}

impl Default for CupertinoSliverNavigationBar {
    fn default() -> CupertinoSliverNavigationBar {
        CupertinoSliverNavigationBar::new()
    }
}

impl StatefulWidget for CupertinoSliverNavigationBar {
    type State = CupertinoSliverNavigationBarState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> CupertinoSliverNavigationBarState {
        CupertinoSliverNavigationBarState {
            state: StateData::new(),
            ticker_provider: TickerProviderStateMixinData::new(),
            keys: NavigationBarStaticComponentsKeys::new(),
            scrollable_state: None,
            effective_middle: None,
            animation_controller: None,
            search_animation: None,
            persistent_height_animation: None,
            large_title_height_animation: None,
            scaled_search_field_height: 0.0,
            scaled_large_title_height: 0.0,
            search_is_active: false,
            is_portrait: true,
            scroll_change_listener: None,
        }
    }
}

/// Dart's `_CupertinoSliverNavigationBarState`.
///
/// A state class exists for the nav bar so that the keys of its sub-components don't change when
/// rebuilding the nav bar, causing the sub-components to lose their own states.
pub struct CupertinoSliverNavigationBarState {
    state: StateData<CupertinoSliverNavigationBar>,
    ticker_provider: TickerProviderStateMixinData,
    keys: Rc<NavigationBarStaticComponentsKeys>,
    scrollable_state: Option<Handle<ScrollableState>>,
    effective_middle: Option<WidgetRef>,
    animation_controller: Option<Handle<AnimationController>>,
    search_animation: Option<Handle<CurvedAnimation>>,
    persistent_height_animation: Option<AnyAnimation<f64>>,
    large_title_height_animation: Option<AnyAnimation<f64>>,
    scaled_search_field_height: f64,
    scaled_large_title_height: f64,
    search_is_active: bool,
    is_portrait: bool,
    scroll_change_listener: Option<Listener>,
}

impl CupertinoSliverNavigationBarState {
    fn animation_controller(self: Handle<Self>, app: &App) -> Handle<AnimationController> {
        app.get(self)
            .animation_controller
            .expect("init_state created the controller")
    }

    /// Dart's `_handleScrollChange` tear-off, which compares equal across calls.
    fn scroll_change_listener(self: Handle<Self>, app: &mut App) -> Listener {
        if let Some(listener) = app.get(self).scroll_change_listener.clone() {
            return listener;
        }
        let listener = Listener::handle_method(
            self,
            CupertinoSliverNavigationBarState::handle_scroll_change,
        );
        app.get_mut(self).scroll_change_listener = Some(listener.clone());
        listener
    }

    fn bottom_height(self: Handle<Self>, app: &App) -> f64 {
        debug_assert!(!self.widget(app).searchable || self.widget(app).bottom.is_none());
        if self.widget(app).searchable {
            return app.get(self).scaled_search_field_height + K_NAV_BAR_BOTTOM_PADDING;
        }
        match &self.widget(app).bottom {
            Some(bottom) => bottom.preferred_size().height(),
            None => 0.0,
        }
    }

    fn update_effective_middle(self: Handle<Self>, app: &mut App) {
        let is_portrait = app.get(self).is_portrait;
        let widget = self.widget(app);
        let effective_middle = widget.middle.clone().or_else(|| {
            if is_portrait {
                None
            } else {
                widget.large_title.clone()
            }
        });
        app.get_mut(self).effective_middle = effective_middle;
    }

    fn compute_scaled_heights(self: Handle<Self>, app: &mut App) {
        let context = self.context(app);
        let text_scaler = MediaQuery::text_scaler_of(app, context);
        let scaled_search_field_height = K_SEARCH_FIELD_HEIGHT
            * damp_scale_factor(
                text_scaler.scale(K_SEARCH_FIELD_HEIGHT),
                K_SEARCH_FIELD_HEIGHT,
                K_MAX_SCALE_FACTOR,
            );
        let scaled_large_title_height = if app.get(self).is_portrait {
            K_NAV_BAR_LARGE_TITLE_HEIGHT_EXTENSION
                * damp_scale_factor(
                    text_scaler.scale(K_NAV_BAR_LARGE_TITLE_HEIGHT_EXTENSION),
                    K_NAV_BAR_LARGE_TITLE_HEIGHT_EXTENSION,
                    K_LARGE_TITLE_SCALE_DAMPING_RATIO,
                )
        } else {
            0.0
        };
        let this = app.get_mut(self);
        this.scaled_search_field_height = scaled_search_field_height;
        this.scaled_large_title_height = scaled_large_title_height;
    }

    fn setup_searchable_animation(self: Handle<Self>, app: &mut App) {
        let controller = self.animation_controller(app).view();
        let persistent_height_tween = DoubleTween::new(K_NAV_BAR_PERSISTENT_HEIGHT, 0.0);
        let persistent_height_animation = persistent_height_tween.animate(app, controller);
        persistent_height_animation.add_status_listener(
            app,
            AnimationStatusListener::handle_method(
                self,
                CupertinoSliverNavigationBarState::handle_search_field_status_changed,
            ),
        );
        let large_title_height_tween =
            DoubleTween::new(app.get(self).scaled_large_title_height, 0.0);
        let large_title_height_animation = large_title_height_tween.animate(app, controller);
        let this = app.get_mut(self);
        this.persistent_height_animation = Some(persistent_height_animation);
        this.large_title_height_animation = Some(large_title_height_animation);
    }

    fn handle_scroll_change(self: Handle<Self>, app: &mut App) {
        let Some(position) = app
            .get(self)
            .scrollable_state
            .map(|state| state.position(app))
        else {
            return;
        };
        if !position.has_pixels(app) || position.pixels(app) <= 0.0 {
            return;
        }

        let mut target = None;
        let bottom_scroll_offset =
            if self.widget(app).bottom_mode == Some(NavigationBarBottomMode::Always) {
                0.0
            } else {
                self.bottom_height(app)
            };
        let can_scroll_bottom = (self.widget(app).searchable || self.widget(app).bottom.is_some())
            && bottom_scroll_offset > 0.0;

        let pixels = position.pixels(app);
        let scaled_large_title_height = app.get(self).scaled_large_title_height;
        // Snap the scroll view to a target determined by the navigation bar's position.
        if can_scroll_bottom && pixels < bottom_scroll_offset {
            target = Some(if pixels > bottom_scroll_offset / 2.0 {
                bottom_scroll_offset
            } else {
                0.0
            });
        } else if pixels > bottom_scroll_offset
            && pixels < bottom_scroll_offset + scaled_large_title_height
        {
            target = Some(
                if pixels > bottom_scroll_offset + (scaled_large_title_height / 2.0) {
                    bottom_scroll_offset + scaled_large_title_height
                } else {
                    bottom_scroll_offset
                },
            );
        }

        // If the target is not null and within the scrollable range, animate to it.
        if let Some(target) = target
            && target <= position.max_scroll_extent(app)
        {
            position.animate_to(
                app,
                target,
                // Eyeballed on an iPhone 16 simulator running iOS 18.
                Duration::from_millis(300),
                Curves::fast_ease_in_to_slow_ease_out(),
            );
        }
    }

    fn handle_search_field_status_changed(
        self: Handle<Self>,
        app: &mut App,
        status: AnimationStatus,
    ) {
        // If the search animation is stopped, rebuild so that the leading, middle, and trailing
        // widgets that were collapsed while the search field was active are re-expanded.
        // Otherwise, rebuild to update this widget with the animation controller's values.
        self.set_state(app, |state| match status {
            AnimationStatus::Forward => state.search_is_active = true,
            AnimationStatus::Reverse => state.search_is_active = false,
            AnimationStatus::Completed | AnimationStatus::Dismissed => {}
        });
    }

    fn on_search_field_tap(self: Handle<Self>, app: &mut App) {
        if let Some(on_searchable_bottom_tap) = self.widget(app).on_searchable_bottom_tap.clone() {
            let search_is_active = app.get(self).search_is_active;
            on_searchable_bottom_tap(app, !search_is_active);
        }
        self.animation_controller(app).toggle(app, None);
    }
}

impl TickerProviderStateMixin for CupertinoSliverNavigationBarState {
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

impl TickerProviderObject for CupertinoSliverNavigationBarState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
        TickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl State for CupertinoSliverNavigationBarState {
    type Widget = CupertinoSliverNavigationBar;
    inset_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let animation_controller = AnimationController::create(
            app,
            None,
            Some(K_NAV_BAR_SEARCH_DURATION),
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            self,
        );
        app.get_mut(self).animation_controller = Some(animation_controller);
        let search_animation = CurvedAnimation::create(
            app,
            animation_controller.view(),
            k_nav_bar_search_curve(),
            None,
        );
        app.get_mut(self).search_animation = Some(search_animation);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &Self::Widget) {
        if !widget_ref_eq(&self.widget(app).middle, &old_widget.middle) {
            self.update_effective_middle(app);
        }
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        let context = self.context(app);
        let is_portrait = MediaQuery::orientation_of(app, context) == Orientation::Portrait;
        app.get_mut(self).is_portrait = is_portrait;
        self.update_effective_middle(app);
        self.compute_scaled_heights(app);
        self.setup_searchable_animation(app);
        let listener = self.scroll_change_listener(app);
        if let Some(scrollable_state) = app.get(self).scrollable_state {
            scrollable_state
                .position(app)
                .is_scrolling_notifier(app)
                .remove_listener(app, &listener);
        }
        let scrollable_state = Scrollable::maybe_of(app, context, None);
        app.get_mut(self).scrollable_state = scrollable_state;
        if let Some(scrollable_state) = scrollable_state {
            scrollable_state
                .position(app)
                .is_scrolling_notifier(app)
                .add_listener(app, listener);
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(scrollable_state) = app.get(self).scrollable_state {
            let listener = self.scroll_change_listener(app);
            scrollable_state
                .position(app)
                .is_scrolling_notifier(app)
                .remove_listener(app, &listener);
        }
        if let Some(search_animation) = app.get(self).search_animation {
            search_animation.dispose(app);
        }
        self.animation_controller(app).dispose(app);
        TickerProviderStateMixin::dispose(self, app);
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        TickerProviderStateMixin::activate(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let controller = self.animation_controller(app);
        let search_is_active = app.get(self).search_is_active;
        let effective_middle = app.get(self).effective_middle.clone();
        let scaled_search_field_height = app.get(self).scaled_search_field_height;
        let persistent_height_animation = app
            .get(self)
            .persistent_height_animation
            .expect("did_change_dependencies set up the animations");
        let widget = self.widget(app);
        let searchable = widget.searchable;
        let search_field = widget.search_field.clone();
        let user_leading = widget.leading.clone().map(|leading| {
            Visibility::new(leading)
                .visible(!search_is_active)
                .into_widget()
        });
        let user_trailing = widget.trailing.clone().map(|trailing| {
            Visibility::new(trailing)
                .visible(!search_is_active)
                .into_widget()
        });
        let user_middle = if controller.is_animating(app) {
            Some(Text::new("").into_widget())
        } else {
            effective_middle.clone()
        };
        let widget = self.widget(app);
        let user_bottom = if searchable {
            let searchable_bottom = SearchableBottom {
                animation_controller: controller,
                animation: persistent_height_animation,
                search_field,
                search_field_height: scaled_search_field_height,
                on_search_field_tap: Listener::handle_method(
                    self,
                    CupertinoSliverNavigationBarState::on_search_field_tap,
                ),
            };
            Some(if search_is_active {
                ActiveSearchableBottom(searchable_bottom).into_widget()
            } else {
                InactiveSearchableBottom(searchable_bottom).into_widget()
            })
        } else {
            widget.bottom.clone().map(IntoWidget::into_widget)
        }
        .unwrap_or_else(|| SizedBox::shrink().into_widget());

        let route = AnyModalRoute::of(app, context);
        let widget = self.widget(app);
        let arguments = NavigationBarStaticComponentsArguments {
            keys: Rc::clone(&app.get(self).keys),
            route,
            user_leading,
            automatically_imply_leading: widget.automatically_imply_leading,
            automatically_imply_title: widget.automatically_imply_title,
            previous_page_title: widget.previous_page_title.clone(),
            user_middle,
            user_trailing,
            user_large_title: widget.large_title.clone(),
            user_bottom: Some(user_bottom),
            padding: widget.padding,
            large: app.get(self).is_portrait,
            // This one scrolls.
            static_bar: false,
        };
        let components = Rc::new(NavigationBarStaticComponents::new(app, context, arguments));

        let widget = self.widget(app);
        let automatic_background_visibility = widget.automatic_background_visibility;
        let brightness = widget.brightness;
        let border = widget.border;
        let padding = widget.padding;
        let transition_between_routes = widget.transition_between_routes;
        let hero_tag = Rc::clone(&widget.hero_tag);
        let always_show_middle = widget.always_show_middle && effective_middle.is_some();
        let enable_background_filter_blur = widget.enable_background_filter_blur;
        let bottom_mode = if search_is_active {
            NavigationBarBottomMode::Always
        } else {
            widget
                .bottom_mode
                .unwrap_or(NavigationBarBottomMode::Automatic)
        };
        let stretch = widget.stretch && !search_is_active;
        let background_color_source = widget.background_color.clone();
        let bottom_height = self.bottom_height(app);
        let keys = Rc::clone(&app.get(self).keys);
        let search_animation = app
            .get(self)
            .search_animation
            .expect("init_state created the curve");
        let large_title_height_animation = app
            .get(self)
            .large_title_height_animation
            .expect("did_change_dependencies set up the animations");

        MediaQuery::with_no_text_scaling(
            None,
            AnimatedBuilder::new(
                Rc::new(search_animation.as_animation()),
                move |app: &mut App, context: BuildContext, _child: Option<&WidgetRef>| {
                    let theme = CupertinoTheme::of(app, context);
                    let delegate = LargeTitleNavigationBarSliverDelegate {
                        keys: Rc::clone(&keys),
                        components: Rc::clone(&components),
                        user_middle: effective_middle.clone(),
                        background_color: CupertinoDynamicColor::maybe_resolve(
                            background_color_source.as_ref(),
                            app,
                            context,
                        )
                        .unwrap_or_else(|| theme.bar_background_color()),
                        automatic_background_visibility,
                        brightness,
                        border,
                        padding,
                        actions_foreground_color: theme.primary_color(),
                        transition_between_routes,
                        hero_tag: Rc::clone(&hero_tag),
                        persistent_height: persistent_height_animation.value(app)
                            + MediaQuery::padding_of(app, context).top,
                        large_title_height: large_title_height_animation.value(app),
                        always_show_middle,
                        stretch_configuration: stretch
                            .then(OverScrollHeaderStretchConfiguration::new),
                        enable_background_filter_blur,
                        bottom_mode,
                        bottom_height,
                        controller,
                        searchable,
                    };
                    SliverPersistentHeader::new(Rc::new(delegate))
                        // iOS navigation bars are always pinned.
                        .pinned(true)
                        .into_widget()
                },
            ),
        )
    }
}

impl Debug for CupertinoSliverNavigationBarState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CupertinoSliverNavigationBarState")
    }
}

// ---------------------------------------------------------------------------------------------
// _LargeTitleNavigationBarSliverDelegate

/// Dart's `_LargeTitleNavigationBarSliverDelegate`.
struct LargeTitleNavigationBarSliverDelegate {
    keys: Rc<NavigationBarStaticComponentsKeys>,
    components: Rc<NavigationBarStaticComponents>,
    user_middle: Option<WidgetRef>,
    background_color: AnyColor,
    automatic_background_visibility: bool,
    brightness: Option<Brightness>,
    border: Option<Border>,
    padding: Option<EdgeInsetsDirectional>,
    actions_foreground_color: AnyColor,
    transition_between_routes: bool,
    hero_tag: HeroTagRef,
    persistent_height: f64,
    large_title_height: f64,
    always_show_middle: bool,
    stretch_configuration: Option<OverScrollHeaderStretchConfiguration>,
    enable_background_filter_blur: bool,
    bottom_mode: NavigationBarBottomMode,
    bottom_height: f64,
    controller: Handle<AnimationController>,
    searchable: bool,
}

impl Debug for LargeTitleNavigationBarSliverDelegate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("LargeTitleNavigationBarSliverDelegate")
    }
}

impl SliverPersistentHeaderDelegate for LargeTitleNavigationBarSliverDelegate {
    fn min_extent(&self) -> f64 {
        self.persistent_height
            + if self.bottom_mode == NavigationBarBottomMode::Always {
                self.bottom_height
            } else {
                0.0
            }
    }

    fn max_extent(&self) -> f64 {
        self.persistent_height + self.large_title_height + self.bottom_height
    }

    fn stretch_configuration(&self) -> Option<OverScrollHeaderStretchConfiguration> {
        self.stretch_configuration.clone()
    }

    fn build(
        &self,
        app: &mut App,
        context: BuildContext,
        shrink_offset: f64,
        _overlaps_content: bool,
    ) -> WidgetRef {
        let large_title_threshold =
            self.max_extent() - self.min_extent() - K_NAV_BAR_SHOW_LARGE_TITLE_THRESHOLD;
        let show_large_title = shrink_offset < large_title_threshold;

        // Calculate how much the bottom should shrink.
        let bottom_shrink_factor = clamp_double(shrink_offset / self.bottom_height, 0.0, 1.0);

        let shrink_animation_value = clamp_double(
            (shrink_offset - large_title_threshold - K_NAV_BAR_SCROLL_UNDER_ANIMATION_EXTENT)
                / K_NAV_BAR_SCROLL_UNDER_ANIMATION_EXTENT,
            0.0,
            1.0,
        );

        let persistent_navigation_bar = PersistentNavigationBar {
            components: Rc::clone(&self.components),
            padding: self.padding,
            // If a user specified middle exists, always show it. Otherwise, show title when
            // sliver is collapsed.
            middle_visible: (!self.always_show_middle).then_some(!show_large_title),
        };

        let parent_page_scaffold_background_color =
            CupertinoPageScaffoldBackgroundColor::maybe_of(app, context);

        let initial_border = if self.automatic_background_visibility
            && parent_page_scaffold_background_color.is_some()
        {
            Some(K_TRANSPARENT_NAV_BAR_BORDER)
        } else {
            self.border
        };
        let effective_border = self
            .border
            .and_then(|border| Border::lerp(initial_border, Some(border), shrink_animation_value));

        let effective_background_color = match &parent_page_scaffold_background_color {
            Some(scaffold_color) if self.automatic_background_visibility => AnyColor::lerp(
                Some(scaffold_color),
                Some(&self.background_color),
                shrink_animation_value,
            )
            .map_or_else(|| self.background_color.clone(), AnyColor::from),
            _ => self.background_color.clone(),
        };

        let theme_text_theme = CupertinoTheme::of(app, context).text_theme();
        let mut stack_children = vec![
            inset_widgets::Positioned::new(
                ClipRect::new().child(
                    Padding::new(EdgeInsetsGeometry::directional(
                        K_NAV_BAR_EDGE_PADDING,
                        0.0,
                        0.0,
                        K_NAV_BAR_BOTTOM_PADDING,
                    ))
                    .child(
                        SafeArea::new(
                            AnimatedOpacity::new(
                                // Fade the large title as the search field animates from its
                                // expanded to its collapsed state.
                                if show_large_title
                                    && !Animation::is_forward_or_completed(self.controller, app)
                                {
                                    1.0
                                } else {
                                    0.0
                                },
                                K_NAV_BAR_TITLE_FADE_DURATION,
                            )
                            .child(
                                DefaultTextStyle::new(
                                    theme_text_theme.nav_large_title_text_style(),
                                    LargeTitle {
                                        height: self.large_title_height,
                                        child: self.components.large_title.clone(),
                                    },
                                )
                                .max_lines(1)
                                .overflow(TextOverflow::Ellipsis),
                            ),
                        )
                        .top(false)
                        .bottom(false),
                    ),
                ),
            )
            .top(self.persistent_height)
            .left(0.0)
            .right(0.0)
            .bottom(if self.bottom_mode == NavigationBarBottomMode::Automatic {
                self.bottom_height * (1.0 - bottom_shrink_factor)
            } else {
                0.0
            })
            .into_widget(),
            inset_widgets::Positioned::new(persistent_navigation_bar)
                .left(0.0)
                .right(0.0)
                .top(0.0)
                .into_widget(),
        ];
        if self.bottom_mode == NavigationBarBottomMode::Automatic {
            stack_children.push(
                inset_widgets::Positioned::new(
                    SizedBox::new()
                        .height(self.bottom_height * (1.0 - bottom_shrink_factor))
                        .child(ClipRect::new().child(self.components.nav_bar_bottom.clone())),
                )
                .left(0.0)
                .right(0.0)
                .bottom(0.0)
                .into_widget(),
            );
        }

        let mut column_children =
            vec![Expanded::new(Stack::new().children(stack_children)).into_widget()];
        if self.bottom_mode == NavigationBarBottomMode::Always {
            column_children.push(
                SizedBox::new()
                    .height(self.bottom_height)
                    .child(self.components.nav_bar_bottom.clone())
                    .into_widget(),
            );
        }

        let nav_bar = wrap_with_background(
            effective_border,
            effective_background_color.clone(),
            self.brightness,
            DefaultTextStyle::new(
                theme_text_theme.text_style(),
                Column::new().children(column_children),
            )
            .into_widget(),
            true,
            self.enable_background_filter_blur,
        );

        if !self.transition_between_routes || !is_transitionable(app, context) {
            return nav_bar;
        }

        let tag: HeroTagRef = if Rc::ptr_eq(&self.hero_tag, &default_hero_tag()) {
            Rc::new(HeroTag {
                navigator: Some(Navigator::of(app, context, false)),
            })
        } else {
            Rc::clone(&self.hero_tag)
        };
        Hero::new(
            tag,
            // This is all the way down here instead of being at the top level of
            // `CupertinoSliverNavigationBar` like `CupertinoNavigationBar` because it needs to
            // wrap the top level `RenderBox` rather than a `RenderSliver`.
            TransitionableNavigationBar {
                key: Rc::new(self.keys.nav_bar_box_key.clone()),
                components_keys: Rc::clone(&self.keys),
                background_color: Some(effective_background_color),
                back_button_text_style: theme_text_theme.nav_action_text_style(),
                title_text_style: theme_text_theme.nav_title_text_style(),
                large_title_text_style: Some(theme_text_theme.nav_large_title_text_style()),
                border: effective_border,
                has_user_middle: self.user_middle.is_some()
                    && (self.always_show_middle || !show_large_title),
                large_expanded: show_large_title,
                searchable: self.searchable,
                automatic_background_visibility: self.automatic_background_visibility,
                child: nav_bar,
            },
        )
        .create_rect_tween(Rc::new(linear_translate_with_largest_rect_size_tween))
        .flight_shuttle_builder(Rc::new(nav_bar_hero_flight_shuttle_builder))
        .placeholder_builder(Rc::new(nav_bar_hero_launch_pad_builder))
        .transition_on_user_gestures(true)
        .into_widget()
    }

    fn should_rebuild(&self, old_delegate: &dyn SliverPersistentHeaderDelegate) -> bool {
        let Some(old_delegate) = old_delegate
            .as_any()
            .downcast_ref::<LargeTitleNavigationBarSliverDelegate>()
        else {
            return true;
        };
        !Rc::ptr_eq(&self.components, &old_delegate.components)
            || !widget_ref_eq(&self.user_middle, &old_delegate.user_middle)
            || self.background_color != old_delegate.background_color
            || self.automatic_background_visibility != old_delegate.automatic_background_visibility
            || self.border != old_delegate.border
            || self.padding != old_delegate.padding
            || self.actions_foreground_color != old_delegate.actions_foreground_color
            || self.transition_between_routes != old_delegate.transition_between_routes
            || self.persistent_height != old_delegate.persistent_height
            || self.large_title_height != old_delegate.large_title_height
            || self.always_show_middle != old_delegate.always_show_middle
            || *self.hero_tag != *old_delegate.hero_tag
            || self.enable_background_filter_blur != old_delegate.enable_background_filter_blur
            || self.bottom_mode != old_delegate.bottom_mode
            || self.bottom_height != old_delegate.bottom_height
            || self.controller != old_delegate.controller
            || self.searchable != old_delegate.searchable
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------------------------
// _LargeTitle and _RenderLargeTitle

/// The large title of the navigation bar.
///
/// Magnifies on over-scroll when [`CupertinoSliverNavigationBar::stretch`] is true.
#[derive(Debug)]
struct LargeTitle {
    child: Option<WidgetRef>,
    height: f64,
}

impl RenderObjectWidget for LargeTitle {
    type RenderObject = RenderLargeTitle;

    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        let alignment =
            AlignmentGeometry::BOTTOM_START.resolve(Some(Directionality::of(app, context)));
        RenderLargeTitle::new(app, alignment, self.height).as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        context: BuildContext,
        render_object: RenderHandle<RenderLargeTitle>,
    ) {
        let alignment =
            AlignmentGeometry::BOTTOM_START.resolve(Some(Directionality::of(app, context)));
        render_object.set_alignment(app, alignment);
        render_object.set_height(app, self.height);
    }
}

impl SingleChildRenderObjectWidget for LargeTitle {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

/// Dart's `_RenderLargeTitle`.
pub struct RenderLargeTitle {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    alignment: Alignment,
    height: f64,
    scale: f64,
}

impl RenderLargeTitle {
    fn new(app: &mut App, alignment: Alignment, height: f64) -> RenderHandle<RenderLargeTitle> {
        RenderHandle::new_box(
            app,
            RenderLargeTitle {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                alignment,
                height,
                scale: 1.0,
            },
        )
    }

    /// How the title is positioned inside the box it magnifies in.
    pub fn alignment(self: RenderHandle<Self>, app: &App) -> Alignment {
        self.get(app).alignment
    }

    /// Sets [`alignment`](Self::alignment).
    pub fn set_alignment(self: RenderHandle<Self>, app: &mut App, value: Alignment) {
        if self.get(app).alignment == value {
            return;
        }
        self.get_mut(app).alignment = value;

        self.mark_needs_layout(app);
    }

    /// The unstretched height of the large title.
    pub fn height(self: RenderHandle<Self>, app: &App) -> f64 {
        self.get(app).height
    }

    /// Sets [`height`](Self::height).
    pub fn set_height(self: RenderHandle<Self>, app: &mut App, value: f64) {
        if self.get(app).height == value {
            return;
        }
        self.get_mut(app).height = value;

        self.mark_needs_layout(app);
    }

    fn compute_title_scale(child_size: Size, constraints: BoxConstraints, height: f64) -> f64 {
        let max_height = height - K_NAV_BAR_BOTTOM_PADDING;
        let scale = 1.0 + 0.03 * (constraints.max_height - max_height) / max_height;
        let max_scale = if child_size.width() != 0.0 {
            clamp_double(constraints.max_width / child_size.width(), 1.0, 1.1)
        } else {
            1.1
        };
        clamp_double(scale, 1.0, max_scale)
    }
}

impl RenderObjectWithChildMixin for RenderLargeTitle {
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

impl RenderShiftedBox for RenderLargeTitle {}

impl RenderObject for RenderLargeTitle {
    inset_rendering::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        self.set_size(app, constraints.biggest());

        let Some(child) = self.child(app) else {
            return;
        };

        let child_constraints = constraints.width_constraints().loosen();
        child.layout(app, child_constraints, true);
        let child_size = child.size(app);
        let height = self.height(app);
        let scale = RenderLargeTitle::compute_title_scale(child_size, constraints, height);
        self.get_mut(app).scale = scale;
        let size = self.size(app);
        let alignment = self.alignment(app);
        child.parent_data_of_mut::<BoxParentData>(app).offset =
            alignment.along_offset(size - (child_size * scale));
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        let Some(child) = self.child(app) else {
            self.as_object().set_layer(app, None);
            return;
        };
        let child_offset = child.box_parent_data(app).offset;
        let scale = self.get(app).scale as f32;
        let old = self.as_object().layer_as::<TransformLayer>(app);
        let layer = context.push_transform(
            app,
            self.as_object().needs_compositing(app),
            offset + child_offset,
            Matrix4::scale(scale, scale),
            |app, context, offset| context.paint_child(app, child.as_object(), offset),
            old,
        );
        self.as_object()
            .set_layer(app, layer.map(|layer| layer.as_container_layer()));
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

impl RenderBox for RenderLargeTitle {
    inset_rendering::render_box_accessors!();

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        debug_assert_eq!(Some(child), self.child(app).map(AnyRenderBox::as_object));

        // Dart's `super.applyPaintTransform`, which `RenderBox` cannot reach from an override.
        let offset = child.parent_data_of::<BoxParentData>(app).offset;
        let scale = self.get(app).scale as f32;
        *transform = transform
            .then(&Matrix4::translation(
                offset.dx() as f32,
                offset.dy() as f32,
            ))
            .then(&Matrix4::from_flutter_array(&[
                scale, 0.0, 0.0, 0.0, //
                0.0, scale, 0.0, 0.0, //
                0.0, 0.0, scale, 0.0, //
                0.0, 0.0, 0.0, 1.0,
            ]));
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        let child = self.child(app)?;
        let distance = child.get_distance_to_actual_baseline(app, baseline)?;
        let offset = child.box_parent_data(app).offset;
        Some(offset.dy() + distance * self.get(app).scale)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        let child = self.child(app)?;
        let child_constraints = constraints.width_constraints().loosen();
        let result = child.get_dry_baseline(app, child_constraints, baseline)?;
        let child_size = child.get_dry_layout(app, child_constraints);
        let height = self.height(app);
        let scale = RenderLargeTitle::compute_title_scale(child_size, constraints, height);
        let scaled_child_size = child_size * scale;
        Some(
            result * scale
                + self
                    .alignment(app)
                    .along_offset(constraints.biggest() - scaled_child_size)
                    .dy(),
        )
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        let Some(child) = self.child(app) else {
            return false;
        };

        let child_offset = child.box_parent_data(app).offset;
        let scale = 1.0 / self.get(app).scale;
        let transform = Matrix4::scale(scale as f32, scale as f32).then(&Matrix4::translation(
            -child_offset.dx() as f32,
            -child_offset.dy() as f32,
        ));

        result.add_with_raw_transform(Some(transform), position, |result, transformed| {
            child.hit_test(app, result, transformed)
        })
    }
}

// ---------------------------------------------------------------------------------------------
// _PersistentNavigationBar

/// The top part of the navigation bar that's never scrolled away.
///
/// Consists of the entire navigation bar without background and border when used without large
/// titles. With large titles, it's the top static half that doesn't scroll.
#[derive(Debug)]
struct PersistentNavigationBar {
    components: Rc<NavigationBarStaticComponents>,

    padding: Option<EdgeInsetsDirectional>,

    /// Whether the middle widget has a visible animated opacity. A `None` value means the middle
    /// opacity will not be animated.
    middle_visible: Option<bool>,
}

impl StatelessWidget for PersistentNavigationBar {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let middle = self.components.middle.clone().map(|middle| {
            let middle = DefaultTextStyle::new(
                CupertinoTheme::of(app, context)
                    .text_theme()
                    .nav_title_text_style(),
                middle,
            )
            .into_widget();
            // When the middle's visibility can change on the fly like with large title slivers,
            // wrap with animated opacity.
            match self.middle_visible {
                None => middle,
                Some(middle_visible) => AnimatedOpacity::new(
                    if middle_visible { 1.0 } else { 0.0 },
                    K_NAV_BAR_TITLE_FADE_DURATION,
                )
                .child(middle)
                .into_widget(),
            }
        });

        let leading = self.components.leading.clone();
        let back_chevron = self.components.back_chevron.clone();
        let back_label = self.components.back_label.clone();

        let leading = match (leading, back_chevron, back_label) {
            (None, Some(back_chevron), Some(back_label))
                if !CupertinoSheetRoute::has_parent_sheet(app, context) =>
            {
                CupertinoNavigationBarBackButton::assemble(back_chevron, back_label).into_widget()
            }
            (leading, _, _) => {
                let align = Align::new().width_factor(1.0);
                match leading {
                    Some(leading) => align.child(leading).into_widget(),
                    None => align.into_widget(),
                }
            }
        };

        let mut toolbar = NavigationToolbar::new()
            .leading(leading)
            .middle_spacing(6.0);
        if let Some(middle) = middle {
            toolbar = toolbar.middle(middle);
        }
        if let Some(trailing) = self.components.trailing.clone() {
            toolbar = toolbar.trailing(trailing);
        }
        let mut padded_toolbar: WidgetRef = toolbar.into_widget();

        if let Some(padding) = self.padding {
            padded_toolbar = Padding::new(EdgeInsetsGeometry::only(
                0.0,
                padding.top,
                0.0,
                padding.bottom,
            ))
            .child(padded_toolbar)
            .into_widget();
        }

        SizedBox::new()
            .height(K_NAV_BAR_PERSISTENT_HEIGHT + MediaQuery::padding_of(app, context).top)
            .child(
                SafeArea::new(padded_toolbar)
                    .top(!CupertinoSheetRoute::has_parent_sheet(app, context))
                    .bottom(false),
            )
            .into_widget()
    }
}

// ---------------------------------------------------------------------------------------------
// _NavigationBarStaticComponentsKeys and _NavigationBarStaticComponents

/// A collection of keys always used when building static routes' nav bars's components with
/// [`NavigationBarStaticComponents`] and read in [`NavigationBarTransition`] in `Hero` flights in
/// order to reference the components' render boxes for their positions.
///
/// These keys should never re-appear inside the `Hero` flights.
#[derive(Debug)]
struct NavigationBarStaticComponentsKeys {
    nav_bar_box_key: GlobalKey,
    leading_key: GlobalKey,
    back_chevron_key: GlobalKey,
    back_label_key: GlobalKey,
    middle_key: GlobalKey,
    trailing_key: GlobalKey,
    large_title_key: GlobalKey,
    nav_bar_bottom_key: GlobalKey,
}

impl NavigationBarStaticComponentsKeys {
    fn new() -> Rc<NavigationBarStaticComponentsKeys> {
        Rc::new(NavigationBarStaticComponentsKeys {
            nav_bar_box_key: GlobalKey::labeled("Navigation bar render box"),
            leading_key: GlobalKey::labeled("Leading"),
            back_chevron_key: GlobalKey::labeled("Back chevron"),
            back_label_key: GlobalKey::labeled("Back label"),
            middle_key: GlobalKey::labeled("Middle"),
            trailing_key: GlobalKey::labeled("Trailing"),
            large_title_key: GlobalKey::labeled("Large title"),
            nav_bar_bottom_key: GlobalKey::labeled("Navigation bar bottom"),
        })
    }
}

/// Dart's named arguments of the [`NavigationBarStaticComponents`] constructor.
struct NavigationBarStaticComponentsArguments {
    keys: Rc<NavigationBarStaticComponentsKeys>,
    route: Option<AnyModalRoute>,
    user_leading: Option<WidgetRef>,
    automatically_imply_leading: bool,
    automatically_imply_title: bool,
    previous_page_title: Option<String>,
    user_middle: Option<WidgetRef>,
    user_trailing: Option<WidgetRef>,
    user_large_title: Option<WidgetRef>,
    user_bottom: Option<WidgetRef>,
    padding: Option<EdgeInsetsDirectional>,
    large: bool,
    static_bar: bool,
}

/// Based on various user widgets and other parameters, construct `KeyedSubtree` components that
/// are used in common by the [`CupertinoNavigationBar`] and the [`CupertinoSliverNavigationBar`].
/// The keyed subtrees are inserted into static routes and their children are reused in the
/// `Hero` flights.
#[derive(Debug)]
struct NavigationBarStaticComponents {
    leading: Option<WidgetRef>,
    back_chevron: Option<WidgetRef>,
    /// This widget is not decorated with a font since the font style could animate during
    /// transitions.
    back_label: Option<WidgetRef>,
    /// This widget is not decorated with a font since the font style could animate during
    /// transitions.
    middle: Option<WidgetRef>,
    trailing: Option<WidgetRef>,
    /// This widget is not decorated with a font since the font style could animate during
    /// transitions.
    large_title: Option<WidgetRef>,
    nav_bar_bottom: WidgetRef,
}

impl NavigationBarStaticComponents {
    fn new(
        app: &mut App,
        context: BuildContext,
        arguments: NavigationBarStaticComponentsArguments,
    ) -> NavigationBarStaticComponents {
        let NavigationBarStaticComponentsArguments {
            keys,
            route,
            user_leading,
            automatically_imply_leading,
            automatically_imply_title,
            previous_page_title,
            user_middle,
            user_trailing,
            user_large_title,
            user_bottom,
            padding,
            large,
            static_bar,
        } = arguments;
        NavigationBarStaticComponents {
            leading: NavigationBarStaticComponents::create_leading(
                app,
                context,
                &keys.leading_key,
                user_leading.clone(),
                route,
                automatically_imply_leading,
                padding,
            ),
            back_chevron: NavigationBarStaticComponents::create_back_chevron(
                app,
                context,
                &keys.back_chevron_key,
                user_leading.clone(),
                route,
                automatically_imply_leading,
            ),
            back_label: NavigationBarStaticComponents::create_back_label(
                app,
                context,
                &keys.back_label_key,
                user_leading,
                route,
                previous_page_title,
                automatically_imply_leading,
            ),
            middle: NavigationBarStaticComponents::create_middle(
                app,
                context,
                &keys.middle_key,
                user_middle,
                user_large_title.clone(),
                large,
                static_bar,
                automatically_imply_title,
                route,
            ),
            trailing: NavigationBarStaticComponents::create_trailing(
                app,
                context,
                &keys.trailing_key,
                user_trailing,
                padding,
            ),
            large_title: NavigationBarStaticComponents::create_large_title(
                app,
                context,
                &keys.large_title_key,
                user_large_title,
                large,
                automatically_imply_title,
                route,
            ),
            nav_bar_bottom: NavigationBarStaticComponents::create_nav_bar_bottom(
                app,
                context,
                &keys.nav_bar_bottom_key,
                user_bottom,
            ),
        }
    }

    fn derived_title(
        app: &App,
        automatically_imply_title: bool,
        current_route: Option<AnyModalRoute>,
    ) -> Option<WidgetRef> {
        // Auto use the `CupertinoPageRoute`'s title if middle not provided.
        if automatically_imply_title {
            let title = current_route
                .and_then(|route| {
                    route
                        .as_route()
                        .interface::<Rc<dyn CupertinoRouteTransition>>()
                })
                .and_then(|route| route.title(app));
            if let Some(title) = title {
                return Some(Text::new(title).into_widget());
            }
        }

        None
    }

    fn create_leading(
        app: &mut App,
        context: BuildContext,
        leading_key: &GlobalKey,
        user_leading: Option<WidgetRef>,
        route: Option<AnyModalRoute>,
        automatically_imply_leading: bool,
        padding: Option<EdgeInsetsDirectional>,
    ) -> Option<WidgetRef> {
        let mut leading_content = None;

        if let Some(user_leading) = user_leading {
            leading_content = Some(user_leading);
        } else if automatically_imply_leading
            && let Some(route) = route
            && route.as_route().as_page_route(app).is_some()
            && route.can_pop(app)
            && route.fullscreen_dialog(app)
        {
            leading_content = Some(
                CupertinoButton::new(
                    Text::new(<dyn CupertinoLocalizations>::of(app, context).cancel_button_label())
                        .into_widget(),
                    Some(Listener::new(move |app: &mut App| {
                        route
                            .as_route()
                            .navigator(app)
                            .expect("an installed route has a navigator")
                            .maybe_pop(app, None);
                    })),
                )
                .padding(EdgeInsetsGeometry::ZERO)
                .into_widget(),
            );
        }

        let leading_content = leading_content?;

        Some(
            KeyedSubtree::new(
                Padding::new(EdgeInsetsGeometry::directional(
                    padding.map_or(K_NAV_BAR_EDGE_PADDING, |padding| padding.start),
                    0.0,
                    0.0,
                    0.0,
                ))
                .child(
                    MediaQuery::new(
                        MediaQuery::of(app, context)
                            .copy_with()
                            .text_scaler(clamped_text_scaler(app, context)),
                        IconTheme::merge(None, IconThemeData::new().size(32.0), leading_content),
                    )
                    .into_widget(),
                ),
            )
            .key(Rc::new(leading_key.clone()))
            .into_widget(),
        )
    }

    fn create_back_chevron(
        app: &mut App,
        context: BuildContext,
        back_chevron_key: &GlobalKey,
        user_leading: Option<WidgetRef>,
        route: Option<AnyModalRoute>,
        automatically_imply_leading: bool,
    ) -> Option<WidgetRef> {
        if user_leading.is_some() || !automatically_imply_leading {
            return None;
        }
        let route = route?;
        if !route.can_pop(app)
            || route
                .as_route()
                .as_page_route(app)
                .is_some_and(|_| route.fullscreen_dialog(app))
        {
            return None;
        }

        Some(
            KeyedSubtree::new(
                MediaQuery::new(
                    MediaQuery::of(app, context)
                        .copy_with()
                        .text_scaler(clamped_text_scaler(app, context)),
                    BackChevron,
                )
                .into_widget(),
            )
            .key(Rc::new(back_chevron_key.clone()))
            .into_widget(),
        )
    }

    fn create_back_label(
        app: &mut App,
        context: BuildContext,
        back_label_key: &GlobalKey,
        user_leading: Option<WidgetRef>,
        route: Option<AnyModalRoute>,
        previous_page_title: Option<String>,
        automatically_imply_leading: bool,
    ) -> Option<WidgetRef> {
        if user_leading.is_some() || !automatically_imply_leading {
            return None;
        }
        let route = route?;
        if !route.can_pop(app)
            || route
                .as_route()
                .as_page_route(app)
                .is_some_and(|_| route.fullscreen_dialog(app))
        {
            return None;
        }

        Some(
            KeyedSubtree::new(
                MediaQuery::new(
                    MediaQuery::of(app, context)
                        .copy_with()
                        .text_scaler(clamped_text_scaler(app, context)),
                    BackLabel {
                        specified_previous_title: previous_page_title,
                        route: Some(route),
                    },
                )
                .into_widget(),
            )
            .key(Rc::new(back_label_key.clone()))
            .into_widget(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn create_middle(
        app: &mut App,
        context: BuildContext,
        middle_key: &GlobalKey,
        user_middle: Option<WidgetRef>,
        user_large_title: Option<WidgetRef>,
        large: bool,
        static_bar: bool,
        automatically_imply_title: bool,
        route: Option<AnyModalRoute>,
    ) -> Option<WidgetRef> {
        let mut middle_content = user_middle;

        if large && static_bar {
            // Static bar only displays the middle, or the large, not both. A scrolling bar
            // creates both middle and large to transition between.
            return None;
        }

        if large {
            middle_content = middle_content.or(user_large_title);
        }

        middle_content = middle_content.or_else(|| {
            NavigationBarStaticComponents::derived_title(app, automatically_imply_title, route)
        });

        let middle_content = middle_content?;

        Some(
            KeyedSubtree::new(
                MediaQuery::new(
                    MediaQuery::of(app, context)
                        .copy_with()
                        .text_scaler(clamped_text_scaler(app, context)),
                    middle_content,
                )
                .into_widget(),
            )
            .key(Rc::new(middle_key.clone()))
            .into_widget(),
        )
    }

    fn create_trailing(
        app: &mut App,
        context: BuildContext,
        trailing_key: &GlobalKey,
        user_trailing: Option<WidgetRef>,
        padding: Option<EdgeInsetsDirectional>,
    ) -> Option<WidgetRef> {
        let user_trailing = user_trailing?;

        Some(
            KeyedSubtree::new(
                Padding::new(EdgeInsetsGeometry::directional(
                    0.0,
                    0.0,
                    padding.map_or(K_NAV_BAR_EDGE_PADDING, |padding| padding.end),
                    0.0,
                ))
                .child(
                    MediaQuery::new(
                        MediaQuery::of(app, context)
                            .copy_with()
                            .text_scaler(clamped_text_scaler(app, context)),
                        IconTheme::merge(None, IconThemeData::new().size(32.0), user_trailing),
                    )
                    .into_widget(),
                ),
            )
            .key(Rc::new(trailing_key.clone()))
            .into_widget(),
        )
    }

    fn create_large_title(
        app: &mut App,
        context: BuildContext,
        large_title_key: &GlobalKey,
        user_large_title: Option<WidgetRef>,
        large: bool,
        automatic_imply_title: bool,
        route: Option<AnyModalRoute>,
    ) -> Option<WidgetRef> {
        if !large {
            return None;
        }

        let large_title_content = user_large_title.or_else(|| {
            NavigationBarStaticComponents::derived_title(app, automatic_imply_title, route)
        });

        debug_assert!(
            large_title_content.is_some(),
            "largeTitle was not provided and there was no title from the route."
        );

        let large_title_content = large_title_content?;
        let text_scaler = TextScaler::linear(damp_scale_factor(
            MediaQuery::text_scaler_of(app, context).scale(K_NAV_BAR_LARGE_TITLE_HEIGHT_EXTENSION),
            K_NAV_BAR_LARGE_TITLE_HEIGHT_EXTENSION,
            K_LARGE_TITLE_SCALE_DAMPING_RATIO,
        ));

        Some(
            KeyedSubtree::new(
                MediaQuery::new(
                    MediaQuery::of(app, context)
                        .copy_with()
                        .text_scaler(text_scaler),
                    large_title_content,
                )
                .into_widget(),
            )
            .key(Rc::new(large_title_key.clone()))
            .into_widget(),
        )
    }

    fn create_nav_bar_bottom(
        app: &mut App,
        context: BuildContext,
        nav_bar_bottom_key: &GlobalKey,
        user_bottom: Option<WidgetRef>,
    ) -> WidgetRef {
        let text_scaler = MediaQuery::text_scaler_of(app, context);
        KeyedSubtree::new(
            MediaQuery::new(
                MediaQuery::of(app, context)
                    .copy_with()
                    .text_scaler(text_scaler),
                user_bottom.unwrap_or_else(|| SizedBox::shrink().into_widget()),
            )
            .into_widget(),
        )
        .key(Rc::new(nav_bar_bottom_key.clone()))
        .into_widget()
    }
}

/// Dart's `_NavigationBarStaticComponents._clampedTextScaler`.
fn clamped_text_scaler(app: &mut App, context: BuildContext) -> TextScaler {
    MediaQuery::text_scaler_of(app, context).clamp(1.0, K_MAX_SCALE_FACTOR)
}

// ---------------------------------------------------------------------------------------------
// CupertinoNavigationBarBackButton, _BackChevron, _BackLabel

/// A nav bar back button typically used in [`CupertinoNavigationBar`].
///
/// This is automatically inserted into [`CupertinoNavigationBar`] and
/// [`CupertinoSliverNavigationBar`]'s `leading` slot when `automatically_imply_leading` is true.
///
/// When manually inserted, the [`CupertinoNavigationBarBackButton`] should only be used in routes
/// that can be popped unless a custom [`on_pressed`](Self::on_pressed) is provided.
///
/// Shows a back chevron and the previous route's title when available from the previous
/// [`CupertinoPageRoute`](crate::CupertinoPageRoute)'s title. If
/// [`previous_page_title`](Self::previous_page_title) is specified, it will be shown instead.
#[derive(Debug)]
pub struct CupertinoNavigationBarBackButton {
    key: Option<KeyRef>,

    /// The color of the back button.
    ///
    /// Can be used to override the color of the back button chevron and label.
    ///
    /// Defaults to [`CupertinoTheme`]'s `primary_color` if null.
    pub color: Option<AnyColor>,

    /// An override for showing the previous route's title. If null, it will be automatically
    /// derived from [`CupertinoPageRoute`](crate::CupertinoPageRoute)'s title if the current and
    /// previous routes are both [`CupertinoPageRoute`](crate::CupertinoPageRoute)s.
    pub previous_page_title: Option<String>,

    /// An override callback to perform instead of the default behavior which is to pop the
    /// `Navigator`.
    ///
    /// It can, for instance, be used to pop the platform's navigation stack instead of the
    /// framework's `Navigator` in add-to-app situations.
    ///
    /// Defaults to null.
    pub on_pressed: Option<Listener>,

    back_chevron: Option<WidgetRef>,

    back_label: Option<WidgetRef>,
}

impl CupertinoNavigationBarBackButton {
    /// Construct a [`CupertinoNavigationBarBackButton`] that can be used to pop the current
    /// route.
    pub fn new() -> CupertinoNavigationBarBackButton {
        CupertinoNavigationBarBackButton {
            key: None,
            color: None,
            previous_page_title: None,
            on_pressed: None,
            back_chevron: None,
            back_label: None,
        }
    }

    /// Dart's `CupertinoNavigationBarBackButton._assemble`, which lets the back chevron and label
    /// be separately created (and keyed) because they animate separately during page transitions.
    fn assemble(
        back_chevron: WidgetRef,
        back_label: WidgetRef,
    ) -> CupertinoNavigationBarBackButton {
        CupertinoNavigationBarBackButton {
            key: None,
            color: None,
            previous_page_title: None,
            on_pressed: None,
            back_chevron: Some(back_chevron),
            back_label: Some(back_label),
        }
    }

    /// Dart `CupertinoNavigationBarBackButton(key:)`.
    pub fn key(mut self, key: KeyRef) -> CupertinoNavigationBarBackButton {
        self.key = Some(key);
        self
    }

    /// Dart `CupertinoNavigationBarBackButton(color:)`.
    pub fn color(mut self, color: impl Into<AnyColor>) -> CupertinoNavigationBarBackButton {
        self.color = Some(color.into());
        self
    }

    /// Dart `CupertinoNavigationBarBackButton(previousPageTitle:)`.
    pub fn previous_page_title(
        mut self,
        previous_page_title: impl Into<String>,
    ) -> CupertinoNavigationBarBackButton {
        self.previous_page_title = Some(previous_page_title.into());
        self
    }

    /// Dart `CupertinoNavigationBarBackButton(onPressed:)`.
    pub fn on_pressed(mut self, on_pressed: Listener) -> CupertinoNavigationBarBackButton {
        self.on_pressed = Some(on_pressed);
        self
    }
}

impl Default for CupertinoNavigationBarBackButton {
    fn default() -> CupertinoNavigationBarBackButton {
        CupertinoNavigationBarBackButton::new()
    }
}

impl StatelessWidget for CupertinoNavigationBarBackButton {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let current_route = AnyModalRoute::of(app, context);
        if self.on_pressed.is_none() {
            debug_assert!(
                current_route.is_some_and(|route| route.can_pop(app)),
                "CupertinoNavigationBarBackButton should only be used in routes that can be popped"
            );
        }

        let mut action_text_style = CupertinoTheme::of(app, context)
            .text_theme()
            .nav_action_text_style();
        if let Some(color) = &self.color {
            action_text_style = action_text_style.copy_with().color(
                CupertinoDynamicColor::maybe_resolve(Some(color), app, context)
                    .expect("a color was provided"),
            );
        }

        let on_pressed = self.on_pressed.clone();
        CupertinoButton::new(
            DefaultTextStyle::new(
                action_text_style,
                ConstrainedBox::new(
                    BoxConstraints::new().min_width(K_NAV_BAR_BACK_BUTTON_TAP_WIDTH),
                )
                .child(Row::new().main_axis_size(MainAxisSize::Min).children(
                    vec![
                            Padding::new(EdgeInsetsGeometry::directional(8.0, 0.0, 0.0, 0.0))
                                .into_widget(),
                            self.back_chevron
                                .clone()
                                .unwrap_or_else(|| BackChevron.into_widget()),
                            Padding::new(EdgeInsetsGeometry::directional(6.0, 0.0, 0.0, 0.0))
                                .into_widget(),
                            Flexible::new(self.back_label.clone().unwrap_or_else(|| {
                                BackLabel {
                                    specified_previous_title: self.previous_page_title.clone(),
                                    route: current_route,
                                }
                                .into_widget()
                            }))
                            .into_widget(),
                        ],
                )),
            )
            .into_widget(),
            Some(Listener::new(move |app: &mut App| match &on_pressed {
                Some(on_pressed) => on_pressed.call(app),
                None => {
                    Navigator::maybe_pop(app, context, None);
                }
            })),
        )
        .padding(EdgeInsetsGeometry::ZERO)
        .into_widget()
    }
}

/// Dart's `_BackChevron`.
#[derive(Debug)]
struct BackChevron;

impl StatelessWidget for BackChevron {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let text_direction = Directionality::of(app, context);
        let text_style = DefaultTextStyle::of(app, context).style;

        // Replicate the `Icon` logic here to get a tightly sized icon and add custom non-square
        // padding.
        let back = CupertinoIcons::back();
        let mut icon_style = TextStyle::new().inherit(false).font_size(30.0);
        if let Some(color) = text_style.color.clone() {
            icon_style = icon_style.color(color);
        }
        if let Some(font_family) = back.font_family.clone() {
            icon_style = icon_style.font_family(font_family);
        }
        if let Some(font_package) = back.font_package.clone() {
            icon_style = icon_style.package(font_package);
        }
        let mut icon_widget: WidgetRef =
            Padding::new(EdgeInsetsGeometry::directional(6.0, 0.0, 2.0, 0.0))
                .child(Text::rich(Rc::new(
                    TextSpan::new()
                        .text(
                            char::from_u32(back.code_point)
                                .expect("a codepoint")
                                .to_string(),
                        )
                        .style(icon_style),
                )))
                .into_widget();
        match text_direction {
            TextDirection::Rtl => {
                icon_widget = Transform::new(Matrix4::scale(-1.0, 1.0))
                    .alignment(AlignmentGeometry::CENTER)
                    .transform_hit_tests(false)
                    .child(icon_widget)
                    .into_widget();
            }
            TextDirection::Ltr => {}
        }

        KeyedSubtree::new(icon_widget)
            .key(StandardComponentType::BackButton.key())
            .into_widget()
    }
}

/// A widget that shows next to the back chevron when `automatically_imply_leading` is true.
#[derive(Debug)]
struct BackLabel {
    specified_previous_title: Option<String>,
    route: Option<AnyModalRoute>,
}

impl BackLabel {
    /// Dart's `_buildPreviousTitleWidget`; Dart's unused `child` parameter is dropped.
    fn build_previous_title_widget(
        app: &mut App,
        context: BuildContext,
        previous_title: Option<String>,
    ) -> WidgetRef {
        let Some(previous_title) = previous_title else {
            return SizedBox::shrink().into_widget();
        };

        let text_widget = if previous_title.chars().count() > 12 {
            Text::new(<dyn CupertinoLocalizations>::of(app, context).back_button_label())
        } else {
            Text::new(previous_title)
                .max_lines(1)
                .overflow(TextOverflow::Ellipsis)
        };

        Align::new()
            .alignment(AlignmentGeometry::CENTER_START)
            .width_factor(1.0)
            .child(text_widget)
            .into_widget()
    }
}

impl StatelessWidget for BackLabel {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        if self.specified_previous_title.is_some() {
            return BackLabel::build_previous_title_widget(
                app,
                context,
                self.specified_previous_title.clone(),
            );
        }
        if let Some(route) = self.route
            && let Some(cupertino_route) = route
                .as_route()
                .interface::<Rc<dyn CupertinoRouteTransition>>()
            && !route.as_route().is_first(app)
        {
            // There is no timing issue because the previousTitle Listenable changes
            // happen during route modifications before the ValueListenableBuilder
            // is built.
            let previous_title: Rc<dyn ValueListenable<Option<String>>> =
                Rc::new(cupertino_route.previous_title(app));
            return ValueListenableBuilder::new(previous_title, |app, context, title, _child| {
                BackLabel::build_previous_title_widget(app, context, title.clone())
            })
            .into_widget();
        }
        SizedBox::shrink().into_widget()
    }
}

// ---------------------------------------------------------------------------------------------
// The searchable bottoms

/// The 'Cancel' button next to the search field in a [`CupertinoSliverNavigationBar::search`].
#[derive(Debug)]
struct CancelButton {
    opacity: f64,
    on_pressed: Option<Listener>,
}

impl StatelessWidget for CancelButton {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let label = <dyn CupertinoLocalizations>::of(app, context).cancel_button_label();
        MediaQuery::with_no_text_scaling(
            None,
            Align::new()
                .alignment(AlignmentGeometry::CENTER_LEFT)
                .child(
                    Opacity::new(self.opacity).child(
                        CupertinoButton::new(
                            Text::new(label)
                                .max_lines(1)
                                .overflow(TextOverflow::Clip)
                                .into_widget(),
                            self.on_pressed.clone(),
                        )
                        .padding(EdgeInsetsGeometry::ZERO),
                    ),
                ),
        )
    }
}

/// The parameters both searchable bottoms carry.
#[derive(Debug)]
struct SearchableBottom {
    animation_controller: Handle<AnimationController>,
    search_field: Option<WidgetRef>,
    animation: AnyAnimation<f64>,
    search_field_height: f64,
    on_search_field_tap: Listener,
}

/// The bottom of a [`CupertinoSliverNavigationBar::search`] when the search field is inactive.
#[derive(Debug)]
struct InactiveSearchableBottom(SearchableBottom);

impl StatelessWidget for InactiveSearchableBottom {
    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        let mut search_field_box = SizedBox::new().height(self.0.search_field_height);
        if let Some(search_field) = self.0.search_field.clone() {
            search_field_box = search_field_box.child(search_field);
        }
        let child: WidgetRef = GestureDetector::new()
            .on_tap(self.0.on_search_field_tap.clone())
            .child(
                AbsorbPointer::new().child(
                    FocusableActionDetector::new(
                        Padding::new(EdgeInsetsGeometry::directional(
                            K_NAV_BAR_EDGE_PADDING,
                            0.0,
                            K_NAV_BAR_EDGE_PADDING,
                            K_NAV_BAR_BOTTOM_PADDING,
                        ))
                        .child(search_field_box),
                    )
                    .descendants_are_focusable(false),
                ),
            )
            .into_widget();
        let animation_controller = self.0.animation_controller;
        AnimatedBuilder::new(
            Rc::new(self.0.animation),
            move |_app: &mut App, _context: BuildContext, child: Option<&WidgetRef>| {
                let child = child.cloned();
                LayoutBuilder::new(
                    move |app: &mut App, _context, constraints: BoxConstraints| {
                        let value = AnimationController::value(animation_controller, app);
                        let mut leading = SizedBox::new().width(
                            constraints.max_width - (K_SEARCH_FIELD_CANCEL_BUTTON_WIDTH * value),
                        );
                        if let Some(child) = child.clone() {
                            leading = leading.child(child);
                        }
                        Row::new()
                            .children(vec![
                                leading.into_widget(),
                                // A decoy 'Cancel' button used in the collapsed-to-expanded animation.
                                SizedBox::new()
                                    .width(value * K_SEARCH_FIELD_CANCEL_BUTTON_WIDTH)
                                    .child(
                                        Padding::new(EdgeInsetsGeometry::only(
                                            0.0,
                                            0.0,
                                            0.0,
                                            K_NAV_BAR_BOTTOM_PADDING,
                                        ))
                                        .child(
                                            CancelButton {
                                                opacity: 0.4,
                                                on_pressed: Some(Listener::new(
                                                    |_app: &mut App| {},
                                                )),
                                            },
                                        ),
                                    )
                                    .into_widget(),
                            ])
                            .into_widget()
                    },
                )
                .into_widget()
            },
        )
        .child(child)
        .into_widget()
    }
}

/// The bottom of a [`CupertinoSliverNavigationBar::search`] when the search field is active.
#[derive(Debug)]
struct ActiveSearchableBottom(SearchableBottom);

impl StatelessWidget for ActiveSearchableBottom {
    fn build(&self, app: &mut App, _context: BuildContext) -> WidgetRef {
        let search_field = self
            .0
            .search_field
            .clone()
            .unwrap_or_else(|| SizedBox::shrink().into_widget());
        let animation_controller = self.0.animation_controller;
        let cancel_opacity = DoubleTween::new(0.0, 1.0).animate(app, animation_controller.view());
        let cancel = FadeTransition::new(cancel_opacity).child(CancelButton {
            opacity: 1.0,
            on_pressed: Some(self.0.on_search_field_tap.clone()),
        });
        Padding::new(EdgeInsetsGeometry::directional(
            K_NAV_BAR_EDGE_PADDING,
            0.0,
            0.0,
            K_NAV_BAR_BOTTOM_PADDING,
        ))
        .child(
            Row::new()
                // Eyeballed on an iPhone 15 simulator running iOS 17.5.
                .spacing(12.0)
                .children(vec![
                    Expanded::new(
                        SizedBox::new()
                            .height(self.0.search_field_height)
                            .child(search_field),
                    )
                    .into_widget(),
                    AnimatedBuilder::new(
                        Rc::new(self.0.animation),
                        move |app: &mut App, _context: BuildContext, child: Option<&WidgetRef>| {
                            let value = AnimationController::value(animation_controller, app);
                            let mut sized =
                                SizedBox::new().width(value * K_SEARCH_FIELD_CANCEL_BUTTON_WIDTH);
                            if let Some(child) = child.cloned() {
                                sized = sized.child(child);
                            }
                            sized.into_widget()
                        },
                    )
                    .child(cancel)
                    .into_widget(),
                ]),
        )
        .into_widget()
    }
}

// ---------------------------------------------------------------------------------------------
// _TransitionableNavigationBar, _NavigationBarTransition

/// This should always be the first child of `Hero` widgets.
///
/// This class helps each `Hero` transition obtain the start or end navigation bar's box size and
/// the inner components of the navigation bar that will move around.
///
/// It should be wrapped around the biggest render box of the static navigation bar in each route.
#[derive(Clone, Debug)]
struct TransitionableNavigationBar {
    /// Dart's `super(key: componentsKeys.navBarBoxKey)`.
    key: KeyRef,
    components_keys: Rc<NavigationBarStaticComponentsKeys>,
    background_color: Option<AnyColor>,
    back_button_text_style: TextStyle,
    title_text_style: TextStyle,
    large_title_text_style: Option<TextStyle>,
    border: Option<Border>,
    has_user_middle: bool,
    large_expanded: bool,
    searchable: bool,
    automatic_background_visibility: bool,
    child: WidgetRef,
}

impl TransitionableNavigationBar {
    fn render_box(&self, app: &mut App) -> AnyRenderBox {
        let box_ = self
            .components_keys
            .nav_bar_box_key
            .current_context(app)
            .expect("the nav bar box is mounted")
            .find_render_object(app)
            .expect("the nav bar box has a render object")
            .as_box()
            .expect("the nav bar box is a box");
        debug_assert!(
            box_.as_object().attached(app),
            "TransitionableNavigationBar::render_box should be called when building hero flight \
             shuttles when the from and the to nav bar boxes are already laid out and painted."
        );
        box_
    }

    fn user_gesture_in_progress(&self, app: &mut App) -> bool {
        let context = self
            .components_keys
            .nav_bar_box_key
            .current_context(app)
            .expect("the nav bar box is mounted");
        Navigator::of(app, context, false).user_gesture_in_progress(app)
    }
}

impl StatelessWidget for TransitionableNavigationBar {
    fn key(&self) -> Option<&KeyRef> {
        Some(&self.key)
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        self.child.clone()
    }
}

/// This class represents the widget that will be in the `Hero` flight instead of the 2 static
/// navigation bars by taking inner components from both.
///
/// The `top_nav_bar` field is the nav bar that was on top regardless of push/pop direction.
///
/// Similarly, the `bottom_nav_bar` field is the nav bar that was at the bottom regardless of the
/// push/pop direction.
///
/// If `MediaQueryData::padding` is still present in this widget's `BuildContext`, that padding
/// will become part of the transitional navigation bar as well.
#[derive(Debug)]
struct NavigationBarTransition {
    animation: AnyAnimation<f64>,
    top_nav_bar: Rc<TransitionableNavigationBar>,
    bottom_nav_bar: Rc<TransitionableNavigationBar>,
    height_tween: DoubleTween,
}

impl NavigationBarTransition {
    fn new(
        app: &mut App,
        animation: AnyAnimation<f64>,
        top_nav_bar: Rc<TransitionableNavigationBar>,
        bottom_nav_bar: Rc<TransitionableNavigationBar>,
    ) -> NavigationBarTransition {
        let height_tween = DoubleTween::new(
            bottom_nav_bar.render_box(app).size(app).height(),
            top_nav_bar.render_box(app).size(app).height(),
        );
        NavigationBarTransition {
            animation,
            top_nav_bar,
            bottom_nav_bar,
            height_tween,
        }
    }
}

impl StatelessWidget for NavigationBarTransition {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let directionality = Directionality::of(app, context);
        let components_transition = NavigationBarComponentsTransition::new(
            app,
            self.animation,
            Rc::clone(&self.bottom_nav_bar),
            Rc::clone(&self.top_nav_bar),
            directionality,
        );

        let children: Vec<WidgetRef> = [
            components_transition.bottom_nav_bar_background(app),
            components_transition.bottom_back_chevron(app),
            components_transition.bottom_back_label(app),
            components_transition.bottom_leading(app),
            components_transition.bottom_middle(app),
            components_transition.bottom_large_title(app),
            components_transition.bottom_trailing(app),
            components_transition.bottom_nav_bar_bottom(app),
            // Draw top components on top of the bottom components.
            components_transition.top_nav_bar_background(app),
            components_transition.top_leading(app),
            components_transition.top_back_chevron(app),
            components_transition.top_back_label(app),
            components_transition.top_middle(app),
            components_transition.top_large_title(app),
            components_transition.top_trailing(app),
            components_transition.top_nav_bar_bottom(app),
        ]
        .into_iter()
        .flatten()
        .collect();

        // The text scaling is disabled to avoid odd transitions between pages.
        MediaQuery::with_no_text_scaling(
            None,
            SizedBox::new()
                .height(
                    self.height_tween.begin.max(self.height_tween.end)
                        + MediaQuery::padding_of(app, context).top,
                )
                .width(f64::INFINITY)
                .child(Stack::new().children(children)),
        )
    }
}

// ---------------------------------------------------------------------------------------------
// _NavigationBarComponentsTransition

/// This class helps create widgets that are in transition based on static components from the
/// bottom and top navigation bars.
///
/// It animates these transitional components both in terms of position and their appearance.
///
/// Instead of running the transitional components through their normal static navigation bar
/// layout logic, this creates transitional widgets that are based on these widgets' existing
/// render objects' layout and position.
///
/// This is possible because this widget is only used during `Hero` transitions where both the
/// from and to routes are already built and laid out.
///
/// The components' existing layout constraints and positions are then replicated using
/// `Positioned` or [`PositionedTransition`] wrappers.
///
/// This class never returns the `KeyedSubtree`s created by [`NavigationBarStaticComponents`]
/// directly. Since those widgets are still present in the widget tree during the hero
/// transitions, it would cause global key duplications. Instead, only the keyed subtrees'
/// children are returned.
struct NavigationBarComponentsTransition {
    animation: AnyAnimation<f64>,
    bottom_components: Rc<NavigationBarStaticComponentsKeys>,
    top_components: Rc<NavigationBarStaticComponentsKeys>,

    // These render boxes that are the ancestors of all the bottom and top components are used to
    // determine the components' relative positions inside their respective navigation bars.
    bottom_nav_bar_box: AnyRenderBox,
    top_nav_bar_box: AnyRenderBox,

    bottom_back_button_text_style: TextStyle,
    top_back_button_text_style: TextStyle,
    bottom_title_text_style: TextStyle,
    top_title_text_style: TextStyle,
    bottom_large_title_text_style: Option<TextStyle>,
    top_large_title_text_style: Option<TextStyle>,

    bottom_has_user_middle: bool,
    top_has_user_middle: bool,
    bottom_large_expanded: bool,
    top_large_expanded: bool,
    user_gesture_in_progress: bool,
    searchable: bool,
    bottom_automatic_background_visibility: bool,

    bottom_background_color: Option<AnyColor>,
    top_background_color: Option<AnyColor>,
    #[expect(
        dead_code,
        reason = "carried as Dart carries it; Dart reads only `topBorder`"
    )]
    bottom_border: Option<Border>,
    top_border: Option<Border>,

    /// This is the outer box in which all the components will be fitted. The sizing component of
    /// `RelativeRect`s will be based on this rect's size.
    transition_box: Rect,

    /// x-axis unity number representing the direction of growth for text.
    forward_direction: f64,
}

/// Dart's `_NavigationBarComponentsTransition.fadeOut`.
const FADE_OUT: DoubleTween = DoubleTween::new(1.0, 0.0);

/// Dart's `_NavigationBarComponentsTransition.fadeIn`.
const FADE_IN: DoubleTween = DoubleTween::new(0.0, 1.0);

impl NavigationBarComponentsTransition {
    fn new(
        app: &mut App,
        animation: AnyAnimation<f64>,
        bottom_nav_bar: Rc<TransitionableNavigationBar>,
        top_nav_bar: Rc<TransitionableNavigationBar>,
        directionality: TextDirection,
    ) -> NavigationBarComponentsTransition {
        let bottom_nav_bar_box = bottom_nav_bar.render_box(app);
        let top_nav_bar_box = top_nav_bar.render_box(app);
        NavigationBarComponentsTransition {
            animation,
            bottom_components: Rc::clone(&bottom_nav_bar.components_keys),
            top_components: Rc::clone(&top_nav_bar.components_keys),
            bottom_nav_bar_box,
            top_nav_bar_box,
            bottom_back_button_text_style: bottom_nav_bar.back_button_text_style.clone(),
            top_back_button_text_style: top_nav_bar.back_button_text_style.clone(),
            bottom_title_text_style: bottom_nav_bar.title_text_style.clone(),
            top_title_text_style: top_nav_bar.title_text_style.clone(),
            bottom_large_title_text_style: bottom_nav_bar.large_title_text_style.clone(),
            top_large_title_text_style: top_nav_bar.large_title_text_style.clone(),
            bottom_has_user_middle: bottom_nav_bar.has_user_middle,
            top_has_user_middle: top_nav_bar.has_user_middle,
            bottom_large_expanded: bottom_nav_bar.large_expanded,
            top_large_expanded: top_nav_bar.large_expanded,
            bottom_background_color: bottom_nav_bar.background_color.clone(),
            top_background_color: top_nav_bar.background_color.clone(),
            bottom_border: bottom_nav_bar.border,
            top_border: top_nav_bar.border,
            bottom_automatic_background_visibility: bottom_nav_bar.automatic_background_visibility,
            user_gesture_in_progress: top_nav_bar.user_gesture_in_progress(app)
                || bottom_nav_bar.user_gesture_in_progress(app),
            searchable: top_nav_bar.searchable && bottom_nav_bar.searchable,
            // paint_bounds are based on offset zero so it's ok to expand the rects.
            transition_box: bottom_nav_bar_box
                .as_object()
                .paint_bounds(app)
                .expand_to_include(top_nav_bar_box.as_object().paint_bounds(app)),
            forward_direction: if directionality == TextDirection::Ltr {
                1.0
            } else {
                -1.0
            },
        }
    }

    /// Take a widget in its original ancestor navigation bar render box and translate it into a
    /// relative box in the transition navigation bar box.
    fn position_in_transition_box(
        &self,
        app: &mut App,
        key: &GlobalKey,
        from: AnyRenderBox,
    ) -> RelativeRect {
        let component_box = keyed_render_box(app, key);
        debug_assert!(component_box.as_object().attached(app));

        RelativeRect::from_rect(
            component_box.local_to_global(app, Offset::ZERO, Some(from.as_object()))
                & component_box.size(app),
            self.transition_box,
        )
    }

    /// Create an animated widget that moves the given child widget between its original position
    /// in its ancestor navigation bar to another widget's position in that widget's navigation
    /// bar.
    ///
    /// Anchor their positions based on the vertical middle of their respective render boxes'
    /// leading edge.
    ///
    /// This method assumes there's no other transforms other than translations when converting a
    /// rect from the original navigation bar's coordinate space to the other navigation bar's
    /// coordinate space, to avoid performing floating point operations on the size of the child
    /// widget, so that the incoming constraints used for sizing the child widget will be exactly
    /// the same.
    #[allow(clippy::too_many_arguments)]
    fn slide_from_leading_edge(
        &self,
        app: &mut App,
        from_key: &GlobalKey,
        from_nav_bar_box: AnyRenderBox,
        to_key: &GlobalKey,
        to_nav_bar_box: AnyRenderBox,
        curve: Rc<dyn Curve>,
        child: WidgetRef,
    ) -> FixedSizeSlidingTransition {
        let from_box = keyed_render_box(app, from_key);
        let to_box = keyed_render_box(app, to_key);

        let is_ltr = self.forward_direction > 0.0;

        // The animation moves the from_box so its anchor (left-center or right-center depending
        // on the writing direction) aligns with to_box's anchor.
        let from_size = from_box.size(app);
        let to_size = to_box.size(app);
        let from_anchor_local = Offset::new(
            if is_ltr { 0.0 } else { from_size.width() },
            from_size.height() / 2.0,
        );
        let to_anchor_local = Offset::new(
            if is_ltr { 0.0 } else { to_size.width() },
            to_size.height() / 2.0,
        );
        let from_anchor_in_from_box =
            from_box.local_to_global(app, from_anchor_local, Some(from_nav_bar_box.as_object()));
        let to_anchor_in_to_box =
            to_box.local_to_global(app, to_anchor_local, Some(to_nav_bar_box.as_object()));

        // We can't get ahold of the render box of the stack (i.e., `transition_box`) we place
        // components on yet, but we know the stack needs to be top-leading aligned with both
        // from_nav_bar_box and to_nav_bar_box to make the transition look smooth. Also use the
        // top-leading point as the origin for ease of calculation.

        // The offset to move from_anchor to to_anchor, in transition_box's top-leading
        // coordinates.
        let translation = if is_ltr {
            to_anchor_in_to_box - from_anchor_in_from_box
        } else {
            Offset::new(
                to_nav_bar_box.size(app).width() - to_anchor_in_to_box.dx(),
                to_anchor_in_to_box.dy(),
            ) - Offset::new(
                from_nav_bar_box.size(app).width() - from_anchor_in_from_box.dx(),
                from_anchor_in_from_box.dy(),
            )
        };

        let from_box_margin = self.position_in_transition_box(app, from_key, from_nav_bar_box);
        let from_origin_in_transition_box = Offset::new(
            if is_ltr {
                from_box_margin.left
            } else {
                from_box_margin.right
            },
            from_box_margin.top,
        );

        let anchor_movement_in_transition_box = OffsetTween::new(
            from_origin_in_transition_box,
            from_origin_in_transition_box + translation,
        );

        let offset_animation = self
            .animation
            .drive(app, CurveTween::new(curve))
            .drive(app, anchor_movement_in_transition_box);
        FixedSizeSlidingTransition {
            is_ltr,
            offset_animation,
            width: from_nav_bar_box.size(app).width(),
            height: from_box.size(app).height(),
            child,
        }
    }

    fn fade_in_from(&self, app: &mut App, t: f64, curve: Rc<dyn Curve>) -> AnyAnimation<f64> {
        self.animation.drive(
            app,
            FADE_IN.chain(CurveTween::new(Rc::new(Interval::new(t, 1.0, curve)))),
        )
    }

    fn fade_out_by(&self, app: &mut App, t: f64, curve: Rc<dyn Curve>) -> AnyAnimation<f64> {
        self.animation.drive(
            app,
            FADE_OUT.chain(CurveTween::new(Rc::new(Interval::new(0.0, t, curve)))),
        )
    }

    /// The parent of the hero animation, which is the route animation.
    fn route_animation(&self, app: &App) -> AnyAnimation<f64> {
        // The hero animation is a `CurvedAnimation`.
        let curved = self
            .animation
            .downcast::<CurvedAnimation>(app)
            .expect("the hero flight drives a CurvedAnimation");
        AnimationWithParent::parent(curved, app)
    }

    fn bottom_nav_bar_background(&self, app: &mut App) -> Option<WidgetRef> {
        let bottom_background_color = self.bottom_background_color.clone()?;
        if self.bottom_large_expanded && self.bottom_automatic_background_visibility {
            return None;
        }
        let animation_curve = if self.animation.status(app) == AnimationStatus::Forward {
            Curves::fast_ease_in_to_slow_ease_out()
        } else {
            Rc::new(Curves::fast_ease_in_to_slow_ease_out().flipped()) as Rc<dyn Curve>
        };

        let curve = if self.user_gesture_in_progress {
            Curves::linear()
        } else {
            animation_curve
        };
        let page_transition_animation =
            self.route_animation(app).drive(app, CurveTween::new(curve));

        let from = self.position_in_transition_box(
            app,
            &self.bottom_components.nav_bar_box_key,
            self.bottom_nav_bar_box,
        );

        let width = self.bottom_nav_bar_box.size(app).width();
        let position_tween = RelativeRectTween::new(
            Some(from),
            Some(from.shift(Offset::new(self.forward_direction * -width, 0.0))),
        );

        let height = self.bottom_nav_bar_box.size(app).height();
        Some(
            PositionedTransition::new(
                page_transition_animation.drive(app, position_tween),
                wrap_with_background(
                    self.top_border,
                    bottom_background_color,
                    None,
                    SizedBox::new()
                        .height(height)
                        .width(f64::INFINITY)
                        .into_widget(),
                    // Don't update the system status bar color mid-flight.
                    false,
                    true,
                ),
            )
            .into_widget(),
        )
    }

    fn bottom_leading(&self, app: &mut App) -> Option<WidgetRef> {
        let bottom_leading = keyed_subtree_child(app, &self.bottom_components.leading_key)?;

        let rect = self.position_in_transition_box(
            app,
            &self.bottom_components.leading_key,
            self.bottom_nav_bar_box,
        );
        let opacity = self.fade_out_by(app, 0.4, Curves::ease_out());
        Some(
            inset_widgets::Positioned::from_relative_rect(
                rect,
                FadeTransition::new(opacity).child(bottom_leading),
            )
            .into_widget(),
        )
    }

    fn bottom_back_chevron(&self, app: &mut App) -> Option<WidgetRef> {
        let bottom_back_chevron =
            keyed_subtree_child(app, &self.bottom_components.back_chevron_key)?;

        let rect = self.position_in_transition_box(
            app,
            &self.bottom_components.back_chevron_key,
            self.bottom_nav_bar_box,
        );
        let opacity = self.fade_out_by(app, 0.6, Curves::ease_out());
        Some(
            inset_widgets::Positioned::from_relative_rect(
                rect,
                FadeTransition::new(opacity).child(DefaultTextStyle::new(
                    self.bottom_back_button_text_style.clone(),
                    bottom_back_chevron,
                )),
            )
            .into_widget(),
        )
    }

    fn bottom_back_label(&self, app: &mut App) -> Option<WidgetRef> {
        let bottom_back_label = keyed_subtree_child(app, &self.bottom_components.back_label_key)?;

        let from = self.position_in_transition_box(
            app,
            &self.bottom_components.back_label_key,
            self.bottom_nav_bar_box,
        );

        // Transition away by sliding horizontally to the leading edge off of the screen.
        let width = self.bottom_nav_bar_box.size(app).width();
        let position_tween = RelativeRectTween::new(
            Some(from),
            Some(from.shift(Offset::new(self.forward_direction * (-width / 2.0), 0.0))),
        );
        let rect = self.animation.drive(app, position_tween);
        let opacity = self.fade_out_by(app, 0.2, Curves::ease_out());

        Some(
            PositionedTransition::new(
                rect,
                FadeTransition::new(opacity).child(DefaultTextStyle::new(
                    self.bottom_back_button_text_style.clone(),
                    bottom_back_label,
                )),
            )
            .into_widget(),
        )
    }

    fn bottom_middle(&self, app: &mut App) -> Option<WidgetRef> {
        let bottom_middle = keyed_subtree_child(app, &self.bottom_components.middle_key);
        let top_back_label = keyed_subtree_child(app, &self.top_components.back_label_key);
        let top_leading = keyed_subtree_child(app, &self.top_components.leading_key);

        // The middle component is non-null when the nav bar is a large title nav bar but would be
        // invisible when expanded, therefore don't show it here.
        if !self.bottom_has_user_middle && self.bottom_large_expanded {
            return None;
        }

        let fade_out_by = if self.bottom_has_user_middle {
            0.4
        } else {
            0.7
        };

        if let (Some(bottom_middle), true) = (bottom_middle.clone(), top_back_label.is_some()) {
            // Move from current position to the top page's back label position.
            let opacity = self.fade_out_by(app, fade_out_by, Curves::ease_out());
            let style = self.animation.drive(
                app,
                TextStyleTween::new(
                    Some(self.bottom_title_text_style.clone()),
                    Some(self.top_back_button_text_style.clone()),
                ),
            );
            let child = FadeTransition::new(opacity)
                .child(
                    // As the text shrinks, make sure it's still anchored to the leading edge of a
                    // constantly sized outer box.
                    Align::new()
                        .alignment(AlignmentGeometry::CENTER_START)
                        .child(DefaultTextStyleTransition::new(style, bottom_middle)),
                )
                .into_widget();
            return Some(
                self.slide_from_leading_edge(
                    app,
                    &self.bottom_components.middle_key,
                    self.bottom_nav_bar_box,
                    &self.top_components.back_label_key,
                    self.top_nav_bar_box,
                    Rc::new(Interval::new(0.0, 1.0, Curves::linear())),
                    child,
                )
                .into_widget(),
            );
        }

        // When the top page has a leading widget override (one of the few ways to not have a top
        // back label), don't move the bottom middle widget and just fade.
        if let (Some(bottom_middle), true) = (bottom_middle, top_leading.is_some()) {
            let rect = self.position_in_transition_box(
                app,
                &self.bottom_components.middle_key,
                self.bottom_nav_bar_box,
            );
            let opacity = self.fade_out_by(app, fade_out_by, Curves::ease_out());
            return Some(
                inset_widgets::Positioned::from_relative_rect(
                    rect,
                    FadeTransition::new(opacity).child(
                        // Keep the font when transitioning into a non-back label leading.
                        DefaultTextStyle::new(self.bottom_title_text_style.clone(), bottom_middle),
                    ),
                )
                .into_widget(),
            );
        }

        None
    }

    fn bottom_large_title(&self, app: &mut App) -> Option<WidgetRef> {
        let bottom_large_title = keyed_subtree_child(app, &self.bottom_components.large_title_key)?;
        let top_back_label = keyed_subtree_child(app, &self.top_components.back_label_key);

        if !self.bottom_large_expanded {
            return None;
        }

        if top_back_label.is_some() {
            // Move from current position to the top page's back label position.
            let interval_end = if self.animation.status(app) == AnimationStatus::Forward {
                0.7
            } else {
                1.0
            };
            let opacity = self.fade_out_by(app, 0.6, Curves::ease_out());
            let style = self.animation.drive(
                app,
                TextStyleTween::new(
                    self.bottom_large_title_text_style.clone(),
                    Some(self.top_back_button_text_style.clone()),
                ),
            );
            let child = FadeTransition::new(opacity)
                .child(
                    // As the text shrinks, make sure it's still anchored to the leading edge of a
                    // constantly sized outer box.
                    Align::new()
                        .alignment(AlignmentGeometry::CENTER_START)
                        .child(
                            DefaultTextStyleTransition::new(style, bottom_large_title)
                                .max_lines(1)
                                .overflow(TextOverflow::Ellipsis),
                        ),
                )
                .into_widget();
            return Some(
                self.slide_from_leading_edge(
                    app,
                    &self.bottom_components.large_title_key,
                    self.bottom_nav_bar_box,
                    &self.top_components.back_label_key,
                    self.top_nav_bar_box,
                    Rc::new(Interval::new(0.0, interval_end, Curves::linear())),
                    child,
                )
                .into_widget(),
            );
        }

        // Unlike bottom middle, the bottom large title moves when it can't transition to the top
        // back label position.
        let from = self.position_in_transition_box(
            app,
            &self.bottom_components.large_title_key,
            self.bottom_nav_bar_box,
        );

        let width = self.bottom_nav_bar_box.size(app).width();
        let position_tween = RelativeRectTween::new(
            Some(from),
            Some(from.shift(Offset::new(self.forward_direction * width / 4.0, 0.0))),
        );
        let rect = self.animation.drive(app, position_tween);
        let opacity = self.fade_out_by(app, 0.4, Curves::ease_out());

        // Just shift slightly towards the trailing edge instead of moving to the back label
        // position.
        Some(
            PositionedTransition::new(
                rect,
                FadeTransition::new(opacity).child(
                    // Keep the font when transitioning into a non-back-label leading.
                    DefaultTextStyle::new(
                        self.bottom_large_title_text_style
                            .clone()
                            .expect("a large expanded bar has a large title style"),
                        bottom_large_title,
                    ),
                ),
            )
            .into_widget(),
        )
    }

    fn bottom_trailing(&self, app: &mut App) -> Option<WidgetRef> {
        let bottom_trailing = keyed_subtree_child(app, &self.bottom_components.trailing_key)?;

        let rect = self.position_in_transition_box(
            app,
            &self.bottom_components.trailing_key,
            self.bottom_nav_bar_box,
        );
        let opacity = self.fade_out_by(app, 0.6, Curves::ease_out());
        Some(
            inset_widgets::Positioned::from_relative_rect(
                rect,
                FadeTransition::new(opacity).child(bottom_trailing),
            )
            .into_widget(),
        )
    }

    fn bottom_nav_bar_bottom(&self, app: &mut App) -> Option<WidgetRef> {
        let bottom_nav_bar_bottom =
            keyed_subtree_child(app, &self.bottom_components.nav_bar_bottom_key)?;

        let from = self.position_in_transition_box(
            app,
            &self.bottom_components.nav_bar_bottom_key,
            self.bottom_nav_bar_box,
        );
        // Shift in from the leading edge of the screen.
        let width = self.bottom_nav_bar_box.size(app).width();
        let position_tween = RelativeRectTween::new(
            Some(from),
            Some(from.shift(Offset::new(self.forward_direction * -width, 0.0))),
        );

        let animation_curve: Rc<dyn Curve> = if self.animation.status(app)
            == AnimationStatus::Forward
        {
            Rc::new(K_BOTTOM_NAV_BAR_HEADER_TRANSITION_CURVE)
        } else {
            Rc::new((Rc::new(K_BOTTOM_NAV_BAR_HEADER_TRANSITION_CURVE) as Rc<dyn Curve>).flipped())
        };

        // Fade out only if this is not a `CupertinoSliverNavigationBar::search` to
        // `CupertinoSliverNavigationBar::search` transition.
        let child: WidgetRef = if self.searchable {
            bottom_nav_bar_bottom
        } else {
            let opacity = self.fade_out_by(app, 0.8, Rc::clone(&animation_curve));
            FadeTransition::new(opacity)
                .child(bottom_nav_bar_bottom)
                .into_widget()
        };

        // The bottom widget animates linearly during a backswipe by a user gesture.
        let rect = if self.user_gesture_in_progress {
            self.route_animation(app)
                .drive(app, CurveTween::new(Curves::linear()))
                .drive(app, position_tween)
        } else {
            self.animation
                .drive(app, CurveTween::new(animation_curve))
                .drive(app, position_tween)
        };

        Some(PositionedTransition::new(rect, ClipRect::new().child(child)).into_widget())
    }

    fn top_nav_bar_background(&self, app: &mut App) -> Option<WidgetRef> {
        let top_background_color = self.top_background_color.clone()?;
        let animation_curve = if self.animation.status(app) == AnimationStatus::Forward {
            Curves::fast_ease_in_to_slow_ease_out()
        } else {
            Rc::new(Curves::fast_ease_in_to_slow_ease_out().flipped()) as Rc<dyn Curve>
        };

        let curve = if self.user_gesture_in_progress {
            Curves::linear()
        } else {
            animation_curve
        };
        let page_transition_animation =
            self.route_animation(app).drive(app, CurveTween::new(curve));

        let to = self.position_in_transition_box(
            app,
            &self.top_components.nav_bar_box_key,
            self.top_nav_bar_box,
        );

        let width = self.top_nav_bar_box.size(app).width();
        let position_tween = RelativeRectTween::new(
            Some(to.shift(Offset::new(self.forward_direction * width, 0.0))),
            Some(to),
        );

        let height = self.top_nav_bar_box.size(app).height();
        Some(
            PositionedTransition::new(
                page_transition_animation.drive(app, position_tween),
                wrap_with_background(
                    self.top_border,
                    top_background_color,
                    None,
                    SizedBox::new()
                        .height(height)
                        .width(f64::INFINITY)
                        .into_widget(),
                    // Don't update the system status bar color mid-flight.
                    false,
                    true,
                ),
            )
            .into_widget(),
        )
    }

    fn top_leading(&self, app: &mut App) -> Option<WidgetRef> {
        let top_leading = keyed_subtree_child(app, &self.top_components.leading_key)?;

        let rect = self.position_in_transition_box(
            app,
            &self.top_components.leading_key,
            self.top_nav_bar_box,
        );
        let opacity = self.fade_in_from(app, 0.6, Curves::ease_in());
        Some(
            inset_widgets::Positioned::from_relative_rect(
                rect,
                FadeTransition::new(opacity).child(top_leading),
            )
            .into_widget(),
        )
    }

    fn top_back_chevron(&self, app: &mut App) -> Option<WidgetRef> {
        let top_back_chevron = keyed_subtree_child(app, &self.top_components.back_chevron_key)?;
        let bottom_back_chevron =
            keyed_subtree_child(app, &self.bottom_components.back_chevron_key);

        let to = self.position_in_transition_box(
            app,
            &self.top_components.back_chevron_key,
            self.top_nav_bar_box,
        );
        let mut from = to;

        let mut child = top_back_chevron;
        // Values eyeballed from an iPhone 15 simulator running iOS 17.5.
        let forward = self.animation.status(app) == AnimationStatus::Forward;
        let effective_scale_curve: Rc<dyn Curve> = if forward {
            Rc::new(Interval::new(0.0, 0.2, Curves::linear()))
        } else {
            Rc::new(Interval::new(0.8, 1.0, Curves::linear()))
        };
        let effective_position_curve: Rc<dyn Curve> = if forward {
            Rc::new(Interval::new(0.0, 0.5, Curves::linear()))
        } else {
            Rc::new(Interval::new(0.5, 1.0, Curves::linear()))
        };

        // If it's the first page with a back chevron, shrink and shift in slightly from the right.
        if bottom_back_chevron.is_none() {
            let top_back_chevron_box = keyed_render_box(app, &self.top_components.back_chevron_key);
            let width = top_back_chevron_box.size(app).width();
            from = to.shift(Offset::new(self.forward_direction * width * 2.0, 0.0));
            let scale = self
                .route_animation(app)
                .drive(app, CurveTween::new(effective_scale_curve));
            child = ScaleTransition::new(scale).child(child).into_widget();
        }

        let position_tween = RelativeRectTween::new(Some(from), Some(to));
        let rect = self
            .route_animation(app)
            .drive(app, CurveTween::new(effective_position_curve))
            .drive(app, position_tween);

        // Fades faster going back from the first page with a back chevron.
        let fade_start = if bottom_back_chevron.is_none() && !forward {
            0.9
        } else {
            0.4
        };
        let opacity = self.route_animation(app).drive(
            app,
            CurveTween::new(Rc::new(Interval::new(fade_start, 1.0, Curves::linear()))),
        );

        Some(
            PositionedTransition::new(
                rect,
                FadeTransition::new(opacity).child(DefaultTextStyle::new(
                    self.top_back_button_text_style.clone(),
                    child,
                )),
            )
            .into_widget(),
        )
    }

    fn top_back_label(&self, app: &mut App) -> Option<WidgetRef> {
        let bottom_middle = keyed_subtree_child(app, &self.bottom_components.middle_key);
        let bottom_large_title = keyed_subtree_child(app, &self.bottom_components.large_title_key);
        let top_back_label = keyed_subtree_child(app, &self.top_components.back_label_key)?;

        let top_back_label_opacity = self
            .top_components
            .back_label_key
            .current_context(app)
            .and_then(|context| {
                context.find_ancestor_render_object_of_type::<RenderAnimatedOpacity>(app)
            });

        let mid_click_opacity = match top_back_label_opacity {
            Some(render) if render.opacity(app).value(app) < 1.0 => {
                let end = render.opacity(app).value(app);
                Some(self.animation.drive(app, DoubleTween::new(0.0, end)))
            }
            _ => None,
        };

        // Pick up from an incoming transition from the large title. This is duplicated here from
        // the bottom large title transition widget because the content text might be different.
        // For instance, if the bottom large title text is too long, the top back label will say
        // 'Back' instead of the original text.
        if bottom_large_title.is_some() && self.bottom_large_expanded {
            let interval_end = if self.animation.status(app) == AnimationStatus::Forward {
                0.7
            } else {
                1.0
            };
            let opacity = match mid_click_opacity {
                Some(opacity) => opacity,
                None => self.fade_in_from(app, 0.4, Curves::ease_in()),
            };
            let style = self.animation.drive(
                app,
                TextStyleTween::new(
                    self.bottom_large_title_text_style.clone(),
                    Some(self.top_back_button_text_style.clone()),
                ),
            );
            let child = FadeTransition::new(opacity)
                .child(
                    DefaultTextStyleTransition::new(style, top_back_label)
                        .max_lines(1)
                        .overflow(TextOverflow::Ellipsis),
                )
                .into_widget();
            return Some(
                self.slide_from_leading_edge(
                    app,
                    &self.bottom_components.large_title_key,
                    self.bottom_nav_bar_box,
                    &self.top_components.back_label_key,
                    self.top_nav_bar_box,
                    Rc::new(Interval::new(0.0, interval_end, Curves::linear())),
                    child,
                )
                .into_widget(),
            );
        }

        // The top back label always comes from the large title first if available and expanded
        // instead of middle.
        if bottom_middle.is_some() {
            let opacity = match mid_click_opacity {
                Some(opacity) => opacity,
                None => self.fade_in_from(app, 0.3, Curves::ease_in()),
            };
            let style = self.animation.drive(
                app,
                TextStyleTween::new(
                    Some(self.bottom_title_text_style.clone()),
                    Some(self.top_back_button_text_style.clone()),
                ),
            );
            let child = FadeTransition::new(opacity)
                .child(DefaultTextStyleTransition::new(style, top_back_label))
                .into_widget();
            return Some(
                self.slide_from_leading_edge(
                    app,
                    &self.bottom_components.middle_key,
                    self.bottom_nav_bar_box,
                    &self.top_components.back_label_key,
                    self.top_nav_bar_box,
                    Rc::new(Interval::new(0.0, 1.0, Curves::linear())),
                    child,
                )
                .into_widget(),
            );
        }

        None
    }

    fn top_middle(&self, app: &mut App) -> Option<WidgetRef> {
        let top_middle = keyed_subtree_child(app, &self.top_components.middle_key)?;

        // The middle component is non-null when the nav bar is a large title nav bar but would be
        // invisible when expanded, therefore don't show it here.
        if !self.top_has_user_middle && self.top_large_expanded {
            return None;
        }

        let to = self.position_in_transition_box(
            app,
            &self.top_components.middle_key,
            self.top_nav_bar_box,
        );
        let to_box = keyed_render_box(app, &self.top_components.middle_key);

        let is_ltr = self.forward_direction > 0.0;

        // Anchor is the top-leading point of to_box, in transition box's top-leading coordinate
        // space.
        let to_anchor_in_transition_box =
            Offset::new(if is_ltr { to.left } else { to.right }, to.top);

        let to_size = to_box.size(app);
        // Shift in from the trailing edge of the screen.
        let anchor_movement_in_transition_box = OffsetTween::new(
            Offset::new(
                // the "width / 2" here makes the middle widget's horizontal center on the
                // trailing edge of the top nav bar.
                self.top_nav_bar_box.size(app).width() - to_size.width() / 2.0,
                to.top,
            ),
            to_anchor_in_transition_box,
        );

        let offset_animation = self.animation.drive(app, anchor_movement_in_transition_box);
        let opacity = self.fade_in_from(app, 0.25, Curves::ease_in());
        Some(
            FixedSizeSlidingTransition {
                is_ltr,
                offset_animation,
                width: to_size.width(),
                height: to_size.height(),
                child: FadeTransition::new(opacity)
                    .child(DefaultTextStyle::new(
                        self.top_title_text_style.clone(),
                        top_middle,
                    ))
                    .into_widget(),
            }
            .into_widget(),
        )
    }

    fn top_trailing(&self, app: &mut App) -> Option<WidgetRef> {
        let top_trailing = keyed_subtree_child(app, &self.top_components.trailing_key)?;

        let rect = self.position_in_transition_box(
            app,
            &self.top_components.trailing_key,
            self.top_nav_bar_box,
        );
        let opacity = self.fade_in_from(app, 0.4, Curves::ease_in());
        Some(
            inset_widgets::Positioned::from_relative_rect(
                rect,
                FadeTransition::new(opacity).child(top_trailing),
            )
            .into_widget(),
        )
    }

    fn top_large_title(&self, app: &mut App) -> Option<WidgetRef> {
        let top_large_title = keyed_subtree_child(app, &self.top_components.large_title_key)?;

        if !self.top_large_expanded {
            return None;
        }

        let to = self.position_in_transition_box(
            app,
            &self.top_components.large_title_key,
            self.top_nav_bar_box,
        );

        // Shift in from the trailing edge of the screen.
        let width = self.top_nav_bar_box.size(app).width();
        let position_tween = RelativeRectTween::new(
            Some(to.shift(Offset::new(self.forward_direction * width, 0.0))),
            Some(to),
        );

        let animation_curve: Rc<dyn Curve> =
            if self.animation.status(app) == AnimationStatus::Forward {
                Rc::new(K_TOP_NAV_BAR_HEADER_TRANSITION_CURVE)
            } else {
                Rc::new((Rc::new(K_TOP_NAV_BAR_HEADER_TRANSITION_CURVE) as Rc<dyn Curve>).flipped())
            };

        // The large title animates linearly during a backswipe by a user gesture.
        let rect = if self.user_gesture_in_progress {
            self.route_animation(app)
                .drive(app, CurveTween::new(Curves::linear()))
                .drive(app, position_tween)
        } else {
            self.animation
                .drive(app, CurveTween::new(Rc::clone(&animation_curve)))
                .drive(app, position_tween)
        };
        let opacity = self.fade_in_from(app, 0.0, animation_curve);

        Some(
            PositionedTransition::new(
                rect,
                FadeTransition::new(opacity).child(
                    DefaultTextStyle::new(
                        self.top_large_title_text_style
                            .clone()
                            .expect("a large expanded bar has a large title style"),
                        top_large_title,
                    )
                    .max_lines(1)
                    .overflow(TextOverflow::Ellipsis),
                ),
            )
            .into_widget(),
        )
    }

    fn top_nav_bar_bottom(&self, app: &mut App) -> Option<WidgetRef> {
        let top_nav_bar_bottom = keyed_subtree_child(app, &self.top_components.nav_bar_bottom_key)?;

        let to = self.position_in_transition_box(
            app,
            &self.top_components.nav_bar_bottom_key,
            self.top_nav_bar_box,
        );
        // Shift in from the trailing edge of the screen.
        let width = self.top_nav_bar_box.size(app).width();
        let position_tween = RelativeRectTween::new(
            Some(to.shift(Offset::new(self.forward_direction * width, 0.0))),
            Some(to),
        );

        let animation_curve: Rc<dyn Curve> =
            if self.animation.status(app) == AnimationStatus::Forward {
                Rc::new(K_TOP_NAV_BAR_HEADER_TRANSITION_CURVE)
            } else {
                Rc::new((Rc::new(K_TOP_NAV_BAR_HEADER_TRANSITION_CURVE) as Rc<dyn Curve>).flipped())
            };

        // Fade in only if this is not a `CupertinoSliverNavigationBar::search` to
        // `CupertinoSliverNavigationBar::search` transition.
        let child: WidgetRef = if self.searchable {
            top_nav_bar_bottom
        } else {
            let opacity = self.fade_in_from(app, 0.0, Rc::clone(&animation_curve));
            FadeTransition::new(opacity)
                .child(top_nav_bar_bottom)
                .into_widget()
        };

        // The bottom widget animates linearly during a backswipe by a user gesture.
        let rect = if self.user_gesture_in_progress {
            self.route_animation(app)
                .drive(app, CurveTween::new(Curves::linear()))
                .drive(app, position_tween)
        } else {
            self.animation
                .drive(app, CurveTween::new(animation_curve))
                .drive(app, position_tween)
        };

        Some(PositionedTransition::new(rect, ClipRect::new().child(child)).into_widget())
    }
}

// ---------------------------------------------------------------------------------------------
// The hero plumbing and the file's private tweens

/// The render box of the widget the global `key` is on.
fn keyed_render_box(app: &mut App, key: &GlobalKey) -> AnyRenderBox {
    key.current_context(app)
        .expect("the keyed component is mounted")
        .find_render_object(app)
        .expect("the keyed component has a render object")
        .as_box()
        .expect("the keyed component is a box")
}

/// Dart's `key.currentWidget as KeyedSubtree?` followed by `.child`: the component the static
/// nav bar built, without the `KeyedSubtree` whose global key must not appear twice.
fn keyed_subtree_child(app: &mut App, key: &GlobalKey) -> Option<WidgetRef> {
    let widget = key.current_widget(app)?;
    Some(
        downcast_widget::<KeyedSubtree>(&*widget)
            .expect("a nav bar component is a KeyedSubtree")
            .child
            .clone(),
    )
}

/// Navigation bars' hero rect tween that will move between the static bars but keep a constant
/// size that's the bigger of both navigation bars.
fn linear_translate_with_largest_rect_size_tween(
    app: &mut App,
    begin: Option<Rect>,
    end: Option<Rect>,
) -> Rc<dyn RectTweenObject> {
    let begin = begin.expect("a hero flight has a begin rect");
    let end = end.expect("a hero flight has an end rect");
    let largest_size = Size::new(
        begin.size().width().max(end.size().width()),
        begin.size().height().max(end.size().height()),
    );
    Rc::new(RectTween::new(
        app,
        Some(begin.top_left() & largest_size),
        Some(end.top_left() & largest_size),
    ))
}

fn nav_bar_hero_launch_pad_builder(
    _app: &mut App,
    _context: BuildContext,
    _hero_size: Size,
    child: WidgetRef,
) -> WidgetRef {
    debug_assert!(downcast_widget::<TransitionableNavigationBar>(&*child).is_some());
    // Tree reshaping is fine here because the heroes' child is always a
    // `TransitionableNavigationBar` which has a global key.

    // Keeping the hero subtree here is needed (instead of just swapping out the anchor nav bars
    // for fixed size boxes during flights) because the nav bar and their specific component
    // children may serve as anchor points again if another mid-transition flight diversion is
    // triggered.

    // This is ok performance-wise because static nav bars are generally cheap to build and lay
    // out but expensive to GPU render (due to clips and blurs) which we're skipping here.
    Visibility::new(child)
        .maintain_size(true)
        .maintain_animation(true)
        .maintain_state(true)
        .visible(false)
        .into_widget()
}

/// Navigation bars' hero flight shuttle builder.
fn nav_bar_hero_flight_shuttle_builder(
    app: &mut App,
    _flight_context: BuildContext,
    animation: AnyAnimation<f64>,
    flight_direction: HeroFlightDirection,
    from_hero_context: BuildContext,
    to_hero_context: BuildContext,
) -> WidgetRef {
    let from_nav_bar = hero_nav_bar(app, from_hero_context);
    let to_nav_bar = hero_nav_bar(app, to_hero_context);

    debug_assert!(
        from_nav_bar
            .components_keys
            .nav_bar_box_key
            .current_context(app)
            .is_some(),
        "The from nav bar to Hero must have been mounted in the previous frame"
    );
    debug_assert!(
        to_nav_bar
            .components_keys
            .nav_bar_box_key
            .current_context(app)
            .is_some(),
        "The to nav bar to Hero must have been mounted in the previous frame"
    );

    let (bottom_nav_bar, top_nav_bar) = match flight_direction {
        HeroFlightDirection::Push => (from_nav_bar, to_nav_bar),
        HeroFlightDirection::Pop => (to_nav_bar, from_nav_bar),
    };
    NavigationBarTransition::new(app, animation, top_nav_bar, bottom_nav_bar).into_widget()
}

/// Dart's `(heroContext.widget as Hero).child as _TransitionableNavigationBar`.
fn hero_nav_bar(app: &mut App, hero_context: BuildContext) -> Rc<TransitionableNavigationBar> {
    let hero_widget = hero_context.widget(app).clone();
    let hero = downcast_widget::<Hero>(&*hero_widget).expect("a hero context holds a Hero");
    Rc::new(
        downcast_widget::<TransitionableNavigationBar>(&*hero.child)
            .expect("a nav bar Hero's child is a TransitionableNavigationBar")
            .clone(),
    )
}

/// Dart's `Tween<double>` as a value; the driven animations outlive the build that creates them,
/// so an arena tween's slot could never be freed. See `PORTING.md`.
#[derive(Clone, Copy, Debug)]
struct DoubleTween {
    begin: f64,
    end: f64,
}

impl DoubleTween {
    const fn new(begin: f64, end: f64) -> DoubleTween {
        DoubleTween { begin, end }
    }
}

impl Animatable<f64> for DoubleTween {
    fn transform(&self, _app: &App, t: f64) -> f64 {
        if t == 0.0 {
            return self.begin;
        }
        if t == 1.0 {
            return self.end;
        }
        f64::lerp(&self.begin, &self.end, t)
    }
}

/// Dart's `Tween<Offset>` as a value, for the reason [`DoubleTween`] gives.
#[derive(Clone, Copy, Debug)]
struct OffsetTween {
    begin: Offset,
    end: Offset,
}

impl OffsetTween {
    const fn new(begin: Offset, end: Offset) -> OffsetTween {
        OffsetTween { begin, end }
    }
}

impl Animatable<Offset> for OffsetTween {
    fn transform(&self, _app: &App, t: f64) -> Offset {
        if t == 0.0 {
            return self.begin;
        }
        if t == 1.0 {
            return self.end;
        }
        Offset::lerp(Some(self.begin), Some(self.end), t).expect("both ends are set")
    }
}

/// Dart's `CurveTween` as a value, for the reason [`DoubleTween`] gives.
#[derive(Clone)]
struct CurveTween {
    curve: Rc<dyn Curve>,
}

impl CurveTween {
    fn new(curve: Rc<dyn Curve>) -> CurveTween {
        CurveTween { curve }
    }
}

impl Animatable<f64> for CurveTween {
    fn transform(&self, _app: &App, t: f64) -> f64 {
        if t == 0.0 || t == 1.0 {
            debug_assert_eq!(self.curve.transform(t).round(), t);
            return t;
        }
        self.curve.transform(t)
    }
}

/// Dart's `widget != oldWidget` on two optional widgets, which compares object identity.
fn widget_ref_eq(a: &Option<WidgetRef>, b: &Option<WidgetRef>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => Rc::ptr_eq(a, b),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use inset_foundation::AppCell;
    use std::cell::Cell;

    use inset_embedder::{
        Locale, PointerChange, PointerData, PointerDataPacket, PointerDeviceKind,
    };
    use inset_gestures::GestureBinding;
    use inset_painting::EdgeInsets;
    use inset_rendering::{AnyRenderObject, RenderDecoratedBox, RenderParagraph};
    use inset_widgets::{
        AnyElement, CustomScrollView, DefaultWidgetsLocalizations, FixedScrollMetrics,
        Localizations, MediaQueryData, NavigatorState, Notification, PreferredSize, Route,
        ScrollController, ScrollControllerLeaf, ScrollMetrics, SliverToBoxAdapter, WidgetBuilder,
        WidgetsBinding,
    };

    use super::*;
    use crate::app::CupertinoApp;
    use crate::icons::install_cupertino_icon_font;
    use crate::localizations::DefaultCupertinoLocalizations;
    use crate::page_scaffold::CupertinoPageScaffold;
    use crate::route::CupertinoPageRoute;
    use crate::test_support::{build, pump, test_cell};

    /// The test view is 800x600 physical at 2x.
    const VIEW_WIDTH: f64 = 400.0;

    /// What the shell does at start-up: the app-wide fonts, plus the icon font the chevron
    /// shapes from.
    fn app_with_fonts() -> Rc<AppCell> {
        let cell = test_cell();
        let mut app = cell.borrow_mut();
        let binding = inset_painting::PaintingBinding::instance(&mut app);
        if !binding.has_fonts(&app) {
            binding.install_fonts(&mut app, |fonts| {
                fonts.add_source(valo_system_fonts::SystemFonts::load());
            });
        }
        install_cupertino_icon_font(&mut app);
        drop(app);
        cell
    }

    fn root_element(app: &mut App) -> AnyElement {
        WidgetsBinding::instance(app)
            .root_element(app)
            .expect("a mounted app")
    }

    /// Every element of the mounted tree, parents before children.
    fn elements(app: &App, root: AnyElement) -> Vec<AnyElement> {
        let mut all = vec![root];
        let mut visited = 0;
        while visited < all.len() {
            let element = all[visited];
            element.visit_children(app, &mut |child| all.push(child));
            visited += 1;
        }
        all
    }

    fn has_widget<W: 'static>(app: &mut App) -> bool {
        let root = root_element(app);
        elements(app, root)
            .into_iter()
            .any(|element| downcast_widget::<W>(&**element.widget(app)).is_some())
    }

    /// Every render object of the mounted tree, parents before children.
    fn render_objects(app: &mut App) -> Vec<AnyRenderObject> {
        let root = root_element(app)
            .find_render_object(app)
            .expect("a mounted view has a render object");
        let mut all = vec![root];
        let mut visited = 0;
        while visited < all.len() {
            let object = all[visited];
            object.visit_children(app, &mut |child| all.push(child));
            visited += 1;
        }
        all
    }

    fn paragraphs(app: &mut App) -> Vec<RenderHandle<RenderParagraph>> {
        render_objects(app)
            .into_iter()
            .filter_map(|object| object.downcast::<RenderParagraph>(app))
            .collect()
    }

    fn texts(app: &mut App) -> Vec<String> {
        paragraphs(app)
            .into_iter()
            .map(|paragraph| paragraph.text(app).to_plain_text(true, true))
            .collect()
    }

    /// The paragraph showing `text`, if the tree has one.
    fn paragraph_of(app: &mut App, text: &str) -> Option<RenderHandle<RenderParagraph>> {
        paragraphs(app)
            .into_iter()
            .find(|paragraph| paragraph.text(app).to_plain_text(true, true) == text)
    }

    /// The render box of the widget the global `key` is on.
    fn keyed_box(app: &mut App, key: &GlobalKey) -> AnyRenderBox {
        key.current_context(app)
            .expect("the keyed widget is mounted")
            .find_render_object(app)
            .expect("the keyed widget is laid out")
            .as_box()
            .expect("a box")
    }

    fn global_rect(app: &mut App, object: AnyRenderBox) -> Rect {
        object.local_to_global(app, Offset::ZERO, None) & object.size(app)
    }

    /// Runs frames until every route transition has settled.
    fn settle(app: &mut App) -> Duration {
        let mut at = Duration::ZERO;
        for _ in 0..60 {
            at += Duration::from_millis(20);
            pump(app, at);
        }
        at
    }

    /// Mounts a bare navigation bar under the chrome it reads: a media query with `padding`,
    /// the Cupertino localizations, and a text direction.
    fn mount_bar(cell: &AppCell, padding: EdgeInsets, bar: CupertinoNavigationBar) -> GlobalKey {
        let key = GlobalKey::new();
        let bar = bar.key(Rc::new(key.clone()));
        // A scaffold `Positioned`s the bar at the top, which leaves its height loose.
        let tree = MediaQuery::new(
            MediaQueryData::new().padding(padding),
            Align::new()
                .alignment(AlignmentGeometry::TOP_CENTER)
                .child(bar),
        )
        .into_widget();
        build(
            cell,
            Localizations::new(
                Locale::new("en"),
                vec![
                    DefaultWidgetsLocalizations::delegate(),
                    DefaultCupertinoLocalizations::delegate(),
                ],
            )
            .child(Directionality::new(TextDirection::Ltr, tree))
            .into_widget(),
        );
        key
    }

    /// An iOS app whose routes are built by `on_generate_route`, settled.
    fn mount_app(configure: impl FnOnce(CupertinoApp) -> CupertinoApp) -> Rc<AppCell> {
        let cell = app_with_fonts();
        build(&cell, configure(CupertinoApp::new()).into_widget());
        let mut app = cell.borrow_mut();
        settle(&mut app);
        drop(app);
        cell
    }

    /// Sends one pointer packet; the test view is 2x, so logical coordinates double.
    fn send(app: &mut App, change: PointerChange, at_point: Offset, at: Duration) {
        GestureBinding::instance(app).handle_pointer_data_packet(
            app,
            PointerDataPacket::new(vec![PointerData {
                change,
                kind: PointerDeviceKind::Touch,
                time_stamp: at,
                pointer_identifier: 1,
                physical_x: at_point.dx() * 2.0,
                physical_y: at_point.dy() * 2.0,
                ..PointerData::default()
            }]),
        );
        app.drain_microtasks();
    }

    fn tap(app: &mut App, at_point: Offset, at: Duration) {
        send(app, PointerChange::Down, at_point, at);
        send(
            app,
            PointerChange::Up,
            at_point,
            at + Duration::from_millis(20),
        );
    }

    /// A page whose scaffold carries `bar`.
    fn scaffold_page(bar: impl Fn() -> CupertinoNavigationBar + 'static) -> WidgetBuilder {
        Rc::new(move |_app: &mut App, _context: BuildContext| {
            CupertinoPageScaffold::new(SizedBox::expand())
                .navigation_bar(crate::page_scaffold::ObstructingPreferredSizeWidgetRef::new(bar()))
                .into_widget()
        })
    }

    #[test]
    fn a_navigation_bar_centres_its_middle_and_reports_its_persistent_height() {
        let cell = app_with_fonts();
        let bar = CupertinoNavigationBar::new().middle(Text::new("Title"));
        assert_eq!(
            bar.preferred_size(),
            Size::from_height(K_NAV_BAR_PERSISTENT_HEIGHT),
            "a bar without a bottom or a large title prefers the persistent height"
        );
        let key = mount_bar(&cell, EdgeInsets::from_ltrb(0.0, 20.0, 0.0, 0.0), bar);
        let mut app = cell.borrow_mut();

        let middle = paragraph_of(&mut app, "Title").expect("the middle is shown");
        let rect = global_rect(&mut app, middle.as_box());
        assert!(
            (rect.left + rect.width() / 2.0 - VIEW_WIDTH / 2.0).abs() < 0.5,
            "the middle is horizontally centred: {rect:?}"
        );
        assert!(
            rect.top >= 20.0,
            "the middle sits below the status bar padding: {rect:?}"
        );

        let bar_box = keyed_box(&mut app, &key);
        assert_eq!(
            bar_box.size(&app).height(),
            K_NAV_BAR_PERSISTENT_HEIGHT + 20.0,
            "the persistent bar takes the top padding on top of its own height"
        );
    }

    #[test]
    fn a_large_navigation_bar_adds_the_large_title_extension_to_its_preferred_size() {
        let bar = CupertinoNavigationBar::large().large_title(Text::new("Title"));
        assert_eq!(
            bar.preferred_size(),
            Size::from_height(K_NAV_BAR_PERSISTENT_HEIGHT + K_NAV_BAR_LARGE_TITLE_HEIGHT_EXTENSION)
        );
    }

    #[test]
    fn a_navigation_bar_fully_obstructs_only_when_its_background_is_opaque() {
        let cell = test_cell();
        let seen = Rc::new(Cell::new(None));
        let probe = Rc::clone(&seen);
        let opaque = CupertinoNavigationBar::new().background_color(Color::new(0xFF00FF00));
        let translucent = CupertinoNavigationBar::new().background_color(Color::new(0x8000FF00));
        build(
            &cell,
            Directionality::new(
                TextDirection::Ltr,
                Builder::new(move |app: &mut App, context: BuildContext| {
                    probe.set(Some((
                        opaque.should_fully_obstruct(app, context),
                        translucent.should_fully_obstruct(app, context),
                    )));
                    SizedBox::shrink().into_widget()
                }),
            )
            .into_widget(),
        );
        assert_eq!(
            seen.get().expect("the probe built"),
            (true, false),
            "only an opaque background fully obstructs"
        );
    }

    #[test]
    fn an_implied_back_label_reads_the_previous_route_title() {
        let cell = mount_app(|cupertino_app| {
            cupertino_app.on_generate_route(move |app, settings| {
                let second = settings.name.as_deref() == Some("/second");
                let route =
                    CupertinoPageRoute::new(app, scaffold_page(CupertinoNavigationBar::new));
                let route = route.title(app, if second { "Second" } else { "Start" }.to_string());
                Some(Route::as_route(route))
            })
        });
        let mut app = cell.borrow_mut();
        let count = |app: &mut App, text: &str| texts(app).iter().filter(|t| *t == text).count();
        assert_eq!(count(&mut app, "Start"), 1, "the first route's middle");

        let root = root_element(&mut app);
        let navigator = elements(&app, root)
            .into_iter()
            .find_map(|element| element.state_handle::<NavigatorState>(&app))
            .expect("the app has a navigator");
        navigator.push_named(&mut app, "/second", None);
        settle(&mut app);

        assert_eq!(
            count(&mut app, "Start"),
            2,
            "the previous route's title is read off the route as the back label: {:?}",
            texts(&mut app)
        );
    }

    #[test]
    fn a_second_route_implies_its_title_as_the_middle_and_a_back_chevron_that_pops() {
        let cell = mount_app(|cupertino_app| {
            cupertino_app.on_generate_route(move |app, settings| {
                let second = settings.name.as_deref() == Some("/second");
                let route = CupertinoPageRoute::new(
                    app,
                    scaffold_page(move || {
                        CupertinoNavigationBar::new().previous_page_title("Home")
                    }),
                );
                let route = route.title(app, if second { "Second" } else { "Home" }.to_string());
                Some(Route::as_route(route))
            })
        });
        let mut app = cell.borrow_mut();

        assert!(
            texts(&mut app).contains(&"Home".to_string()),
            "the first route's title is implied as the middle: {:?}",
            texts(&mut app)
        );

        let root = root_element(&mut app);
        let navigator = elements(&app, root)
            .into_iter()
            .find_map(|element| element.state_handle::<NavigatorState>(&app))
            .expect("the app has a navigator");
        navigator.push_named(&mut app, "/second", None);
        settle(&mut app);

        let shown = texts(&mut app);
        assert!(
            shown.contains(&"Second".to_string()),
            "the pushed route's title is implied as the middle: {shown:?}"
        );
        assert!(
            shown.contains(&"Home".to_string()),
            "the previous page title is shown as the back label: {shown:?}"
        );
        let chevron = char::from_u32(CupertinoIcons::back().code_point)
            .expect("a codepoint")
            .to_string();
        assert!(
            shown.contains(&chevron),
            "the back chevron is shown: {shown:?}"
        );

        let chevron_box = paragraph_of(&mut app, &chevron)
            .expect("the back chevron is laid out")
            .as_box();
        let rect = global_rect(&mut app, chevron_box);
        let at = Duration::from_millis(1500);
        tap(&mut app, rect.center(), at);
        settle(&mut app);

        assert!(
            !texts(&mut app).contains(&"Second".to_string()),
            "tapping the back button popped the route"
        );
    }

    #[test]
    fn automatically_imply_leading_and_middle_can_be_turned_off() {
        let cell = mount_app(|cupertino_app| {
            cupertino_app.on_generate_route(move |app, _settings| {
                let route = CupertinoPageRoute::new(
                    app,
                    scaffold_page(|| {
                        CupertinoNavigationBar::new()
                            .automatically_imply_leading(false)
                            .automatically_imply_middle(false)
                    }),
                );
                Some(Route::as_route(route.title(app, "Home".to_string())))
            })
        });
        let mut app = cell.borrow_mut();
        assert!(
            !texts(&mut app).contains(&"Home".to_string()),
            "automaticallyImplyMiddle false leaves the middle empty"
        );
    }

    #[test]
    fn a_scroll_update_notification_fades_the_bar_background_in() {
        const SCAFFOLD: Color = Color::new(0xFF102030);
        const BAR: Color = Color::new(0xFF405060);

        let cell = app_with_fonts();
        let key = GlobalKey::new();
        let bar_key = Rc::new(key.clone());
        let inner = Rc::new(Cell::new(None));
        let probe = Rc::clone(&inner);
        let tree = MediaQuery::new(
            MediaQueryData::new(),
            ScrollNotificationObserver::new(CupertinoPageScaffoldBackgroundColor::new(
                AnyColor::new(SCAFFOLD),
                Align::new()
                    .alignment(AlignmentGeometry::TOP_CENTER)
                    .child(Builder::new(
                        move |_app: &mut App, context: BuildContext| {
                            probe.set(Some(context));
                            CupertinoNavigationBar::new()
                                .key(bar_key.clone())
                                .background_color(BAR)
                                .into_widget()
                        },
                    )),
            )),
        )
        .into_widget();
        build(
            &cell,
            Localizations::new(
                Locale::new("en"),
                vec![
                    DefaultWidgetsLocalizations::delegate(),
                    DefaultCupertinoLocalizations::delegate(),
                ],
            )
            .child(Directionality::new(TextDirection::Ltr, tree))
            .into_widget(),
        );
        let mut app = cell.borrow_mut();

        assert_eq!(
            bar_background(&mut app, &key),
            SCAFFOLD,
            "with nothing scrolled under, the bar wears the scaffold's colour"
        );

        let context = inner.get().expect("the bar built");
        notify_scrolled(&mut app, context, 40.0);
        pump(&mut app, Duration::from_millis(20));

        assert_eq!(
            bar_background(&mut app, &key),
            BAR,
            "past the animation extent the bar wears its own colour"
        );
    }

    /// The colour of the bar's own `DecoratedBox`, the one `wrap_with_background` builds.
    fn bar_background(app: &mut App, key: &GlobalKey) -> Color {
        let bar = keyed_box(app, key).as_object();
        let mut found = None;
        let mut stack = vec![bar];
        while let Some(object) = stack.pop() {
            if let Some(decorated) = object.downcast::<RenderDecoratedBox>(app) {
                found = Some(decorated);
                break;
            }
            object.visit_children(app, &mut |child| stack.push(child));
        }
        found
            .expect("the bar decorates its background")
            .decoration(app)
            .as_any()
            .downcast_ref::<BoxDecoration>()
            .expect("a box decoration")
            .color
            .clone()
            .expect("the bar's background colour")
            .color()
    }

    /// Dispatches a `ScrollUpdateNotification` whose metrics say `pixels` have scrolled past.
    fn notify_scrolled(app: &mut App, context: BuildContext, pixels: f64) {
        let metrics: Rc<dyn ScrollMetrics> = Rc::new(FixedScrollMetrics::new(
            Some(0.0),
            Some(1000.0),
            Some(pixels),
            Some(600.0),
            AxisDirection::Down,
            2.0,
        ));
        let notification = ScrollUpdateNotification::new(metrics, context).scroll_delta(pixels);
        notification.dispatch(app, Some(context));
        app.drain_microtasks();
    }

    #[test]
    fn a_sliver_navigation_bar_shows_the_large_title_and_collapses_it_when_scrolled() {
        let cell = app_with_fonts();
        let mut app = cell.borrow_mut();
        let controller = ScrollController::default(&mut app);
        let scroll_view = CustomScrollView::new()
            .controller(controller.as_controller())
            .slivers([
                CupertinoSliverNavigationBar::new()
                    .large_title(Text::new("Large"))
                    .middle(Text::new("Middle"))
                    .always_show_middle(false)
                    .into_widget(),
                SliverToBoxAdapter::new()
                    .child(SizedBox::new().height(2000.0))
                    .into_widget(),
            ]);
        let tree = MediaQuery::new(MediaQueryData::new(), scroll_view).into_widget();
        drop(app);
        build(
            &cell,
            Localizations::new(
                Locale::new("en"),
                vec![
                    DefaultWidgetsLocalizations::delegate(),
                    DefaultCupertinoLocalizations::delegate(),
                ],
            )
            .child(Directionality::new(TextDirection::Ltr, tree))
            .into_widget(),
        );
        let mut app = cell.borrow_mut();

        let large = paragraph_of(&mut app, "Large").expect("the large title is built");
        let expanded = global_rect(&mut app, large.as_box());
        assert!(
            expanded.top > K_NAV_BAR_PERSISTENT_HEIGHT,
            "at scroll 0 the large title sits below the persistent bar: {expanded:?}"
        );
        assert_eq!(
            large_title_opacity(&mut app),
            1.0,
            "the large title is fully visible at scroll 0"
        );

        controller.jump_to(&mut app, K_NAV_BAR_LARGE_TITLE_HEIGHT_EXTENSION + 20.0);
        settle(&mut app);

        assert_eq!(
            large_title_opacity(&mut app),
            0.0,
            "past the extension the large title has faded out"
        );
        let middle = paragraph_of(&mut app, "Middle").expect("the middle is built");
        assert!(
            global_rect(&mut app, middle.as_box()).top < K_NAV_BAR_PERSISTENT_HEIGHT,
            "the middle takes the persistent bar"
        );
    }

    /// The opacity the sliver bar's large title fades with.
    fn large_title_opacity(app: &mut App) -> f64 {
        let large_title = render_objects(app)
            .into_iter()
            .find(|object| object.downcast::<RenderLargeTitle>(app).is_some())
            .expect("the large title is in the tree");
        let mut ancestor = large_title.parent(app);
        while let Some(object) = ancestor {
            if let Some(opacity) = object.downcast::<RenderAnimatedOpacity>(app) {
                return opacity.opacity(app).value(app);
            }
            ancestor = object.parent(app);
        }
        panic!("the large title is wrapped in an animated opacity");
    }

    #[test]
    fn a_hero_flight_between_two_bars_builds_the_navigation_bar_transition() {
        let cell = mount_app(|cupertino_app| {
            cupertino_app.on_generate_route(move |app, settings| {
                let second = settings.name.as_deref() == Some("/second");
                let route =
                    CupertinoPageRoute::new(app, scaffold_page(CupertinoNavigationBar::new));
                let route = route.title(app, if second { "Second" } else { "Home" }.to_string());
                Some(Route::as_route(route))
            })
        });
        let mut app = cell.borrow_mut();

        let root = root_element(&mut app);
        let navigator = elements(&app, root)
            .into_iter()
            .find_map(|element| element.state_handle::<NavigatorState>(&app))
            .expect("the app has a navigator");
        navigator.push_named(&mut app, "/second", None);
        // Two frames: one to mount the incoming route, one for the hero controller to start the
        // flight from the two mounted nav bars.
        pump(&mut app, Duration::from_millis(1220));
        pump(&mut app, Duration::from_millis(1340));

        assert!(
            has_widget::<NavigationBarTransition>(&mut app),
            "the flight shuttle is the navigation bar transition"
        );
        let shown = texts(&mut app);
        assert!(
            shown.iter().filter(|text| *text == "Home").count() >= 1
                && shown.contains(&"Second".to_string()),
            "the transition carries both routes' components: {shown:?}"
        );
    }

    #[test]
    fn a_navigation_bar_bottom_adds_its_preferred_height() {
        let bar = CupertinoNavigationBar::new().bottom(PreferredSizeWidgetRef::new(
            PreferredSize::new(Size::from_height(30.0), SizedBox::shrink()),
        ));
        assert_eq!(
            bar.preferred_size(),
            Size::from_height(K_NAV_BAR_PERSISTENT_HEIGHT + 30.0)
        );

        let cell = test_cell();
        let key = mount_bar(&cell, EdgeInsets::ZERO, bar);
        let mut app = cell.borrow_mut();
        let bar_box = keyed_box(&mut app, &key);
        assert_eq!(
            bar_box.size(&app).height(),
            K_NAV_BAR_PERSISTENT_HEIGHT + 30.0,
            "the bottom sits under the persistent bar"
        );
    }
}
