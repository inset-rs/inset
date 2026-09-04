//! Flutter counterpart: `widgets/media_query.dart`.
//!
//! [`MediaQuery::from_view`] derives its data from the view when its dependencies or its
//! widget change: the `WidgetsBindingObserver` hooks (`didChangeMetrics`,
//! `didChangeAccessibilityFeatures`, `didChangeTextScaleFactor`,
//! `didChangePlatformBrightness`) wait with the observer. `DisplayFeature`,
//! `SystemTextScaler`, `debugBrightnessOverride`, and the deprecated `textScaleFactor`
//! members are absent; see `PORTING.md`.

use std::collections::HashSet;
use std::fmt::{self, Debug};
use std::rc::Rc;

use reveal_embedder::{Brightness, Size, View as EmbedderView, ViewPadding, ViewRef};
use reveal_foundation::{App, Handle};
use reveal_gestures::{DeviceGestureSettings, K_TOUCH_SLOP};
use reveal_painting::{BorderRadius, EdgeInsets, TextScaler};

use crate::framework::{
    BuildContext, InheritedModel, InheritedModelKind, InheritedWidget, IntoWidget, KeyRef, State,
    StateData, StatefulWidget, WidgetRef,
};
use crate::widgets::basic::Builder;

/// Whether in portrait or landscape.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Orientation {
    /// Taller than wide.
    Portrait,

    /// Wider than tall.
    Landscape,
}

/// Specifies a part of [`MediaQueryData`] to depend on.
///
/// [`MediaQuery`] contains a large number of related properties. Widgets frequently depend
/// on only a few of these attributes. For example, a widget that needs to rebuild when the
/// [`MediaQueryData::text_scaler`] changes does not need to be notified when the
/// [`MediaQueryData::size`] changes. Specifying an aspect avoids unnecessary rebuilds.
///
/// Dart's private `_MediaQueryAspect`; public here because it is the [`InheritedModel`]
/// aspect type of [`MediaQuery`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MediaQueryAspect {
    /// Specifies the aspect corresponding to [`MediaQueryData::size`].
    Size,

    /// Specifies the aspect corresponding to the width of [`MediaQueryData::size`].
    Width,

    /// Specifies the aspect corresponding to the height of [`MediaQueryData::size`].
    Height,

    /// Specifies the aspect corresponding to [`MediaQueryData::orientation`].
    Orientation,

    /// Specifies the aspect corresponding to [`MediaQueryData::device_pixel_ratio`].
    DevicePixelRatio,

    /// Specifies the aspect corresponding to [`MediaQueryData::text_scaler`].
    TextScaler,

    /// Specifies the aspect corresponding to [`MediaQueryData::platform_brightness`].
    PlatformBrightness,

    /// Specifies the aspect corresponding to [`MediaQueryData::padding`].
    Padding,

    /// Specifies the aspect corresponding to [`MediaQueryData::view_insets`].
    ViewInsets,

    /// Specifies the aspect corresponding to [`MediaQueryData::system_gesture_insets`].
    SystemGestureInsets,

    /// Specifies the aspect corresponding to [`MediaQueryData::view_padding`].
    ViewPadding,

    /// Specifies the aspect corresponding to [`MediaQueryData::always_use_24_hour_format`].
    AlwaysUse24HourFormat,

    /// Specifies the aspect corresponding to [`MediaQueryData::accessible_navigation`].
    AccessibleNavigation,

    /// Specifies the aspect corresponding to [`MediaQueryData::invert_colors`].
    InvertColors,

    /// Specifies the aspect corresponding to [`MediaQueryData::high_contrast`].
    HighContrast,

    /// Specifies the aspect corresponding to [`MediaQueryData::on_off_switch_labels`].
    OnOffSwitchLabels,

    /// Specifies the aspect corresponding to [`MediaQueryData::disable_animations`].
    DisableAnimations,

    /// Specifies the aspect corresponding to [`MediaQueryData::reduce_motion`].
    ReduceMotion,

    /// Specifies the aspect corresponding to [`MediaQueryData::bold_text`].
    BoldText,

    /// Specifies the aspect corresponding to [`MediaQueryData::supports_announce`].
    SupportsAnnounce,

    /// Specifies the aspect corresponding to [`MediaQueryData::navigation_mode`].
    NavigationMode,

    /// Specifies the aspect corresponding to [`MediaQueryData::gesture_settings`].
    GestureSettings,

    /// Specifies the aspect corresponding to
    /// [`MediaQueryData::supports_showing_system_context_menu`].
    SupportsShowingSystemContextMenu,

    /// Specifies the aspect corresponding to
    /// [`MediaQueryData::line_height_scale_factor_override`].
    LineHeightScaleFactorOverride,

    /// Specifies the aspect corresponding to [`MediaQueryData::letter_spacing_override`].
    LetterSpacingOverride,

    /// Specifies the aspect corresponding to [`MediaQueryData::word_spacing_override`].
    WordSpacingOverride,

    /// Specifies the aspect corresponding to [`MediaQueryData::paragraph_spacing_override`].
    ParagraphSpacingOverride,

    /// Specifies the aspect corresponding to [`MediaQueryData::display_corner_radii`].
    DisplayCornerRadii,
}

// ---------------------------------------------------------------------------------------------
// MediaQueryData

/// Information about a piece of media (e.g., a window).
///
/// For example, the [`size`](Self::size) property contains the width and height of the
/// current window.
///
/// To obtain individual attributes in a [`MediaQueryData`], prefer to use the
/// attribute-specific functions of [`MediaQuery`] over obtaining the entire
/// [`MediaQueryData`] and accessing its members. Querying using [`MediaQuery::of`] will cause
/// your widget to rebuild automatically whenever _any_ field of the [`MediaQueryData`]
/// changes (e.g., if the user rotates their device). Therefore, unless you are concerned
/// with the entire [`MediaQueryData`] object changing, prefer using the specific methods
/// (for example: [`MediaQuery::size_of`] and [`MediaQuery::padding_of`]), as it will rebuild
/// more efficiently.
///
/// To obtain the entire current [`MediaQueryData`] for a given [`BuildContext`], use the
/// [`MediaQuery::of`] function. This can be useful if you are going to use
/// [`copy_with`](Self::copy_with) to replace the [`MediaQueryData`] with one with an updated
/// property.
///
/// ## Insets and Padding
///
/// [`padding`](Self::padding) relates to [`view_padding`](Self::view_padding) and
/// [`view_insets`](Self::view_insets), in the simplest configuration, as the difference
/// between the two. In cases when the view insets exceed the view padding, like when a
/// software keyboard is shown below, padding goes to zero rather than a negative value.
/// Therefore, padding is calculated by taking `max(0.0, viewPadding - viewInsets)`.
///
/// [`MediaQueryData`] includes three [`EdgeInsets`] values: [`padding`](Self::padding),
/// [`view_padding`](Self::view_padding), and [`view_insets`](Self::view_insets). These
/// values reflect the configuration of the device and are used and optionally consumed by
/// widgets that position content within these insets. The padding value defines areas that
/// might not be completely visible, like the display "notch" on the iPhone X. The
/// view insets value defines areas that aren't visible at all, typically because they're
/// obscured by the device's keyboard. Similar to view insets, view padding does not
/// differentiate padding in areas that may be obscured. For example, by using the view
/// padding property, padding would defer to the iPhone "safe area" regardless of whether a
/// keyboard is showing.
///
/// The view insets and view padding are independent values, they're measured from the
/// edges of the [`MediaQuery`] widget's bounds. Together they inform the
/// [`padding`](Self::padding) property. The bounds of the top level [`MediaQuery`] created
/// by `WidgetsApp` are the same as the window that contains the app.
///
/// Widgets whose layouts consume space defined by [`view_insets`](Self::view_insets),
/// [`view_padding`](Self::view_padding), or [`padding`](Self::padding) should enclose their
/// children in secondary [`MediaQuery`] widgets that reduce those properties by the same
/// amount. The [`remove_padding`](Self::remove_padding),
/// [`remove_view_padding`](Self::remove_view_padding), and
/// [`remove_view_insets`](Self::remove_view_insets) methods are useful for this.
///
/// Dart's named constructor arguments are the fluent setters
/// (`MediaQueryData::new().size(size).padding(padding)`); [`copy_with`](Self::copy_with)
/// clones for the same chain.
#[derive(Clone)]
pub struct MediaQueryData {
    /// The size of the media in logical pixels (e.g, the size of the screen).
    ///
    /// Logical pixels are roughly the same visual size across devices. Physical pixels are
    /// the size of the actual hardware pixels on the device. The number of physical pixels
    /// per logical pixel is described by the [`device_pixel_ratio`](Self::device_pixel_ratio).
    ///
    /// Prefer using [`MediaQuery::size_of`] over [`MediaQuery::of`]`.size` to get the size,
    /// since the former will only notify of changes in [`size`](Self::size), while the latter
    /// will notify for all [`MediaQueryData`] changes.
    ///
    /// For widgets drawn in an `Overlay`, do not assume that the size of the `Overlay` is
    /// the size of the [`MediaQuery`]'s size. Nested overlays can have different sizes.
    ///
    /// ## Troubleshooting
    ///
    /// It is considered bad practice to cache and later use the size returned by
    /// `MediaQuery::size_of(app, context)`. It will make the application non-responsive and
    /// might lead to unexpected behaviors.
    ///
    /// For instance, during startup, especially in release mode, the first returned size
    /// might be [`Size::ZERO`]. The size will be updated when the native platform reports
    /// the actual resolution. Using [`MediaQuery::size_of`] will ensure that when the size
    /// changes, any widgets depending on the size are automatically rebuilt.
    pub size: Size,

    /// The number of device pixels for each logical pixel of the encompassing view. This
    /// number might not be a power of two. Indeed, it might not even be an integer. For
    /// example, the Nexus 6 has a device pixel ratio of 3.5.
    ///
    /// This property is typically only informational. Overriding this property does not
    /// rescale the app as the framework or its rendering pipeline usually does not read this
    /// value.
    pub device_pixel_ratio: f64,

    /// The font scaling strategy to use for laying out textual contents.
    ///
    /// If this [`MediaQueryData`] is created by the [`from_view`](Self::from_view)
    /// constructor, this property reflects the platform's preferred text scaling strategy,
    /// and may change as the user changes the scaling factor in the operating system's
    /// accessibility settings.
    ///
    /// See also:
    ///
    ///  * [`MediaQuery::text_scaler_of`], a method to find and depend on the
    ///    [`text_scaler`](Self::text_scaler) defined for a [`BuildContext`].
    ///  * `TextPainter`, a class that lays out and paints text.
    pub text_scaler: TextScaler,

    /// The current brightness mode of the host platform.
    ///
    /// For example, starting in Android Pie, battery saver mode asks all apps to render in
    /// a "dark mode".
    ///
    /// Not all platforms necessarily support a concept of brightness mode. Those platforms
    /// will report [`Brightness::Light`] in this property.
    ///
    /// See also:
    ///
    ///  * [`MediaQuery::platform_brightness_of`], a method to find and depend on the
    ///    platform brightness defined for a [`BuildContext`].
    pub platform_brightness: Brightness,

    /// The parts of the display that are completely obscured by system UI, typically by the
    /// device's keyboard.
    ///
    /// When a mobile device's keyboard is visible `view_insets.bottom` corresponds to the
    /// top of the keyboard.
    ///
    /// This value is independent of the [`padding`](Self::padding) and
    /// [`view_padding`](Self::view_padding). View padding is measured from the edges of the
    /// [`MediaQuery`] widget's bounds. Padding is calculated based on the view padding and
    /// view insets. The bounds of the top level [`MediaQuery`] created by `WidgetsApp` are
    /// the same as the window (often the mobile device screen) that contains the app.
    pub view_insets: EdgeInsets,

    /// The parts of the display that are partially obscured by system UI, typically by the
    /// hardware display "notches" or the system status bar.
    ///
    /// If you consumed this padding (e.g. by building a widget that envelops or accounts for
    /// this padding in its layout in such a way that children are no longer exposed to this
    /// padding), you should remove this padding for subsequent descendants in the widget
    /// tree by inserting a new [`MediaQuery`] widget using the [`MediaQuery::remove_padding`]
    /// constructor.
    ///
    /// Padding is derived from the values of [`view_insets`](Self::view_insets) and
    /// [`view_padding`](Self::view_padding).
    ///
    /// See also:
    ///
    ///  * `SafeArea`, a widget that consumes this padding with a `Padding` widget and
    ///    automatically removes it from the [`MediaQuery`] for its child.
    pub padding: EdgeInsets,

    /// The parts of the display that are partially obscured by system UI, typically by the
    /// hardware display "notches" or the system status bar.
    ///
    /// This value remains the same regardless of whether the system is reporting other
    /// obstructions in the same physical area of the screen. For example, a software
    /// keyboard on the bottom of the screen that may cover and consume the same area that
    /// requires bottom padding will not affect this value.
    ///
    /// This value is independent of the [`padding`](Self::padding) and
    /// [`view_insets`](Self::view_insets): their values are measured from the edges of the
    /// [`MediaQuery`] widget's bounds. The bounds of the top level [`MediaQuery`] created by
    /// `WidgetsApp` are the same as the window that contains the app. On mobile devices,
    /// this will typically be the full screen.
    pub view_padding: EdgeInsets,

    /// The areas along the edges of the display where the system consumes certain input
    /// events and blocks delivery of those events to the app.
    ///
    /// Starting with Android Q, simple swipe gestures that start within the
    /// [`system_gesture_insets`](Self::system_gesture_insets) areas are used by the system
    /// for page navigation and may not be delivered to the app. Taps and swipe gestures that
    /// begin with a long-press are delivered to the app, but simple press-drag-release swipe
    /// gestures which begin within the area defined by
    /// [`system_gesture_insets`](Self::system_gesture_insets) may not be.
    ///
    /// Apps should avoid locating gesture detectors within the system gesture insets area.
    /// Apps should feel free to put visual elements within this area.
    ///
    /// This property is currently only expected to be set to a non-default value on Android
    /// starting with version Q.
    pub system_gesture_insets: EdgeInsets,

    /// Whether to use 24-hour format when formatting time.
    ///
    /// The behavior of this flag is different across platforms:
    ///
    /// - On Android this flag is reported directly from the user settings called "Use
    ///   24-hour format". It applies to any locale used by the application, whether it is
    ///   the system-wide locale, or the custom locale set by the application.
    /// - On iOS this flag is set to true when the user setting called "24-Hour Time" is set
    ///   or the system-wide locale's default uses 24-hour formatting.
    /// - On macOS this flag reflects the current system locale's time format, which
    ///   incorporates the "24-Hour Time" preference in System Settings. As on iOS, this only
    ///   takes effect for the system locale; a custom locale passed to the application will
    ///   ignore the 24-hour preference.
    /// - On Windows this flag is derived from the user's "Short time" format in the Region
    ///   settings; it is true when the configured format uses a 24-hour pattern.
    /// - On Linux this flag reflects the desktop environment's clock-format setting where
    ///   available (for example, `org.gnome.desktop.interface.clock-format` on GNOME). On
    ///   desktops that do not expose such a setting, it defaults to true (24-hour).
    /// - On Web this flag is always false.
    pub always_use_24_hour_format: bool,

    /// Whether the user is using an accessibility service like TalkBack or VoiceOver to
    /// interact with the application.
    ///
    /// When this setting is true, features such as timeouts should be disabled or have
    /// minimum durations increased.
    pub accessible_navigation: bool,

    /// Whether the operating system is currently inverting the colors of the platform.
    ///
    /// This flag indicates that the underlying OS is already performing a global color
    /// inversion at the screen level. It does not mean the framework will automatically
    /// invert its own layout painting.
    ///
    /// Instead, this flag allows the application to react to the inversion. For example, by
    /// selectively re-inverting images, maps, or video playback so that they display with
    /// natural colors instead of looking like a film negative.
    ///
    /// This flag is currently only updated on iOS devices.
    pub invert_colors: bool,

    /// Whether the platform is requesting a high contrast between foreground and background
    /// content.
    ///
    /// On iOS, this corresponds to the "Increase Contrast" setting in Settings ->
    /// Accessibility. On Android, this corresponds to the "High contrast text" or similar
    /// accessibility settings.
    ///
    /// This flag indicates that the operating system is already performing high-contrast
    /// adjustments or expects the application to adjust its color palette to meet higher
    /// accessibility standards.
    ///
    /// This flag is currently only updated on iOS devices running iOS 13+ and Android
    /// devices running API 34+.
    pub high_contrast: bool,

    /// Whether the user requested to show on/off labels inside switches on iOS, via Settings
    /// -> Accessibility -> Display & Text Size -> On/Off Labels.
    pub on_off_switch_labels: bool,

    /// Whether the platform is requesting that animations be disabled or reduced as much as
    /// possible.
    ///
    /// This corresponds to Android's "Remove animations" accessibility setting.
    ///
    /// On iOS, reduced motion is exposed separately via
    /// [`reduce_motion`](Self::reduce_motion) and does not set this flag.
    ///
    /// Manually overriding this value in a [`MediaQuery`] widget will not affect framework
    /// animations (for example those driven by `AnimationController`). However, it can still
    /// be useful for testing or for custom widgets that explicitly read
    /// [`disable_animations`](Self::disable_animations).
    ///
    /// When implementing custom explicit animations, you should check this property and
    /// adjust behavior accordingly (for example, by reducing duration or skipping
    /// non-essential animations when it is true).
    pub disable_animations: bool,

    /// Whether the platform is requesting that animations be reduced or replaced with
    /// cross-fades in preference to motion effects.
    ///
    /// This corresponds to the iOS "Reduce Motion" accessibility setting.
    ///
    /// Unlike [`disable_animations`](Self::disable_animations), this flag does not
    /// automatically alter framework animations such as those controlled via
    /// `AnimationController`. Instead, it is intended to be read by widgets that want to
    /// tone down or replace non-essential motion, for example by substituting a cross-fade
    /// for a slide transition.
    ///
    /// When implementing custom animations, you should check this property and adjust
    /// behavior accordingly; for example, by preferring a fade over movement when it is
    /// true.
    pub reduce_motion: bool,

    /// Whether the platform is requesting that text be drawn with a bold font weight.
    pub bold_text: bool,

    /// Whether accessibility announcements (like `SemanticsService.sendAnnouncement`) are
    /// supported on the current platform.
    ///
    /// Returns `false` on platforms where announcements are deprecated or unsupported by the
    /// underlying platform.
    ///
    /// Returns `true` on platforms where such announcements are generally supported without
    /// discouragement. (iOS, web etc)
    pub supports_announce: bool,

    /// Describes the navigation mode requested by the platform.
    ///
    /// Some user interfaces are better navigated using a directional pad (DPAD) or arrow
    /// keys, and for those interfaces, some widgets need to handle these directional events
    /// differently. In order to know when to do that, these widgets will look for the
    /// navigation mode in effect for their context.
    ///
    /// For instance, in a television interface, [`NavigationMode::Directional`] should be
    /// set, so that directional navigation is used to navigate away from a text field using
    /// the DPAD. In contrast, on a regular desktop application with the
    /// [`navigation_mode`](Self::navigation_mode) set to [`NavigationMode::Traditional`],
    /// the arrow keys are used to move the cursor instead of navigating away.
    ///
    /// The [`NavigationMode`] values indicate the type of navigation to be used in a widget
    /// subtree for those widgets sensitive to it.
    pub navigation_mode: NavigationMode,

    /// The gesture settings for the view this media query is derived from.
    ///
    /// This contains platform specific configuration for gesture behavior, such as touch
    /// slop. These settings should be favored for configuring gesture behavior over the
    /// framework constants.
    pub gesture_settings: DeviceGestureSettings,

    /// Whether showing the system context menu is supported.
    ///
    /// For example, on iOS 16.0 and above, the system text selection context menu may be
    /// shown instead of the framework-drawn context menu in order to avoid the iOS clipboard
    /// access notification when the "Paste" button is pressed.
    pub supports_showing_system_context_menu: bool,

    /// Overrides the height of the text, as a multiple of the font size.
    ///
    /// `None` when the platform has not set an override for text height.
    ///
    /// See also:
    ///
    ///  * `Text`, `SelectableText`, and `EditableText`, all of whose `TextStyle.height` and
    ///    `StrutStyle.height` are overridden by this.
    pub line_height_scale_factor_override: Option<f64>,

    /// Overrides the amount of space (in logical pixels) to add between each letter in a
    /// piece of text.
    ///
    /// A negative value can be used to bring the letters closer.
    ///
    /// `None` when the platform has not set an override for text letter spacing.
    pub letter_spacing_override: Option<f64>,

    /// Overrides the amount of space (in logical pixels) to add at each sequence of
    /// white-space (i.e. between each word) in a piece of text.
    ///
    /// A negative value can be used to bring the words closer.
    ///
    /// `None` when the platform has not set an override for text word spacing.
    pub word_spacing_override: Option<f64>,

    /// The amount of space (in logical pixels) to add following each paragraph in a piece of
    /// text.
    ///
    /// `None` when the platform has not set an override for text paragraph spacing.
    pub paragraph_spacing_override: Option<f64>,

    /// The radii of the display corners in logical pixels.
    ///
    /// This is currently populated only on Android API 31+. On earlier Android versions,
    /// iOS, and other platforms, this value is `None`.
    pub display_corner_radii: Option<BorderRadius>,
}

impl MediaQueryData {
    /// Creates data for a media query with explicit values.
    ///
    /// In a typical application, calling this constructor directly is rarely needed.
    /// Consider using [`from_view`](Self::from_view) to create data based on a view, or
    /// [`copy_with`](Self::copy_with) to create a new copy of [`MediaQueryData`] with
    /// updated properties from a base [`MediaQueryData`].
    pub fn new() -> MediaQueryData {
        MediaQueryData {
            size: Size::ZERO,
            device_pixel_ratio: 1.0,
            text_scaler: TextScaler::NO_SCALING,
            platform_brightness: Brightness::Light,
            view_insets: EdgeInsets::ZERO,
            padding: EdgeInsets::ZERO,
            view_padding: EdgeInsets::ZERO,
            system_gesture_insets: EdgeInsets::ZERO,
            always_use_24_hour_format: false,
            accessible_navigation: false,
            invert_colors: false,
            high_contrast: false,
            on_off_switch_labels: false,
            disable_animations: false,
            reduce_motion: false,
            bold_text: false,
            supports_announce: false,
            navigation_mode: NavigationMode::Traditional,
            gesture_settings: DeviceGestureSettings::new(Some(K_TOUCH_SLOP)),
            supports_showing_system_context_menu: false,
            line_height_scale_factor_override: None,
            letter_spacing_override: None,
            word_spacing_override: None,
            paragraph_spacing_override: None,
            display_corner_radii: None,
        }
    }

    /// Creates data for a [`MediaQuery`] based on the given `view`.
    ///
    /// If provided, the `platform_data` is used to fill in the platform-specific aspects of
    /// the newly created [`MediaQueryData`]. If `platform_data` is `None`, the platform
    /// (`app.platform()`, Dart's `view.platformDispatcher`) is consulted to construct the
    /// platform-specific data.
    ///
    /// Data which is exposed directly on the view is considered view-specific. Data which is
    /// only exposed via the platform is considered platform-specific.
    ///
    /// Callers of this method should ensure that they also register for notifications so
    /// that the [`MediaQueryData`] can be updated when any data used to construct it changes
    /// (Dart's `WidgetsBindingObserver.didChangeMetrics`, `didChangeAccessibilityFeatures`,
    /// `didChangeTextScaleFactor`, and `didChangePlatformBrightness`, which wait with the
    /// observer). The last three notifications are only relevant if no `platform_data` is
    /// provided. If `platform_data` is provided, callers should ensure to call this method
    /// again when it changes to keep the constructed [`MediaQueryData`] updated.
    ///
    /// The platform here reports its brightness only: the accessibility features, the
    /// 24-hour format, the text scale factor, the system context menu support, and the text
    /// style overrides, as well as the view's system gesture insets and display corner
    /// radii, keep their defaults until the embedder exposes them (see `PORTING.md`).
    ///
    /// In general, [`MediaQuery::of`], and its associated "...Of" methods, are the
    /// appropriate way to obtain [`MediaQueryData`] from a widget. This `from_view`
    /// constructor is primarily for use in the implementation of the framework itself.
    ///
    /// See also:
    ///
    ///  * [`MediaQuery::from_view`], which constructs [`MediaQueryData`] from a provided
    ///    view, makes it available to descendant widgets, and sets up the appropriate
    ///    notification listeners to keep the data updated.
    pub fn from_view(
        app: &App,
        view: &dyn EmbedderView,
        platform_data: Option<&MediaQueryData>,
    ) -> MediaQueryData {
        let metrics = view.metrics();
        let device_pixel_ratio = metrics.device_pixel_ratio;
        MediaQueryData {
            size: Size::new(metrics.physical_size[0], metrics.physical_size[1])
                / device_pixel_ratio,
            device_pixel_ratio,
            text_scaler: Self::text_scaler_from_view(platform_data),
            platform_brightness: platform_data.map_or_else(
                || app.platform().platform_brightness(),
                |data| data.platform_brightness,
            ),
            padding: edge_insets_from_view_padding(metrics.padding, device_pixel_ratio),
            view_padding: edge_insets_from_view_padding(metrics.view_padding, device_pixel_ratio),
            view_insets: edge_insets_from_view_padding(metrics.view_insets, device_pixel_ratio),
            system_gesture_insets: EdgeInsets::ZERO,
            accessible_navigation: platform_data.is_some_and(|data| data.accessible_navigation),
            invert_colors: platform_data.is_some_and(|data| data.invert_colors),
            disable_animations: platform_data.is_some_and(|data| data.disable_animations),
            reduce_motion: platform_data.is_some_and(|data| data.reduce_motion),
            bold_text: platform_data.is_some_and(|data| data.bold_text),
            supports_announce: platform_data.is_some_and(|data| data.supports_announce),
            high_contrast: platform_data.is_some_and(|data| data.high_contrast),
            on_off_switch_labels: platform_data.is_some_and(|data| data.on_off_switch_labels),
            always_use_24_hour_format: platform_data
                .is_some_and(|data| data.always_use_24_hour_format),
            navigation_mode: platform_data
                .map_or(NavigationMode::Traditional, |data| data.navigation_mode),
            gesture_settings: DeviceGestureSettings::from_view(view),
            supports_showing_system_context_menu: platform_data
                .is_some_and(|data| data.supports_showing_system_context_menu),
            line_height_scale_factor_override: platform_data
                .and_then(|data| data.line_height_scale_factor_override),
            letter_spacing_override: platform_data.and_then(|data| data.letter_spacing_override),
            word_spacing_override: platform_data.and_then(|data| data.word_spacing_override),
            paragraph_spacing_override: platform_data
                .and_then(|data| data.paragraph_spacing_override),
            display_corner_radii: None,
        }
    }

    /// Dart's `_textScalerFromView`: the parent's scaler, else the platform's.
    /// `SystemTextScaler` waits for the embedder's text scale factor; a platform that
    /// reports none scales by 1.0, which is [`TextScaler::NO_SCALING`].
    fn text_scaler_from_view(platform_data: Option<&MediaQueryData>) -> TextScaler {
        platform_data.map_or(TextScaler::NO_SCALING, |data| data.text_scaler.clone())
    }

    /// Dart `MediaQueryData(size:)`.
    pub fn size(mut self, size: Size) -> MediaQueryData {
        self.size = size;
        self
    }

    /// Dart `MediaQueryData(devicePixelRatio:)`.
    pub fn device_pixel_ratio(mut self, device_pixel_ratio: f64) -> MediaQueryData {
        self.device_pixel_ratio = device_pixel_ratio;
        self
    }

    /// Dart `MediaQueryData(textScaler:)`.
    pub fn text_scaler(mut self, text_scaler: TextScaler) -> MediaQueryData {
        self.text_scaler = text_scaler;
        self
    }

    /// Dart `MediaQueryData(platformBrightness:)`.
    pub fn platform_brightness(mut self, platform_brightness: Brightness) -> MediaQueryData {
        self.platform_brightness = platform_brightness;
        self
    }

    /// Dart `MediaQueryData(padding:)`.
    pub fn padding(mut self, padding: EdgeInsets) -> MediaQueryData {
        self.padding = padding;
        self
    }

    /// Dart `MediaQueryData(viewInsets:)`.
    pub fn view_insets(mut self, view_insets: EdgeInsets) -> MediaQueryData {
        self.view_insets = view_insets;
        self
    }

    /// Dart `MediaQueryData(systemGestureInsets:)`.
    pub fn system_gesture_insets(mut self, system_gesture_insets: EdgeInsets) -> MediaQueryData {
        self.system_gesture_insets = system_gesture_insets;
        self
    }

    /// Dart `MediaQueryData(viewPadding:)`.
    pub fn view_padding(mut self, view_padding: EdgeInsets) -> MediaQueryData {
        self.view_padding = view_padding;
        self
    }

    /// Dart `MediaQueryData(alwaysUse24HourFormat:)`.
    pub fn always_use_24_hour_format(mut self, always_use_24_hour_format: bool) -> MediaQueryData {
        self.always_use_24_hour_format = always_use_24_hour_format;
        self
    }

    /// Dart `MediaQueryData(accessibleNavigation:)`.
    pub fn accessible_navigation(mut self, accessible_navigation: bool) -> MediaQueryData {
        self.accessible_navigation = accessible_navigation;
        self
    }

    /// Dart `MediaQueryData(invertColors:)`.
    pub fn invert_colors(mut self, invert_colors: bool) -> MediaQueryData {
        self.invert_colors = invert_colors;
        self
    }

    /// Dart `MediaQueryData(highContrast:)`.
    pub fn high_contrast(mut self, high_contrast: bool) -> MediaQueryData {
        self.high_contrast = high_contrast;
        self
    }

    /// Dart `MediaQueryData(onOffSwitchLabels:)`.
    pub fn on_off_switch_labels(mut self, on_off_switch_labels: bool) -> MediaQueryData {
        self.on_off_switch_labels = on_off_switch_labels;
        self
    }

    /// Dart `MediaQueryData(disableAnimations:)`.
    pub fn disable_animations(mut self, disable_animations: bool) -> MediaQueryData {
        self.disable_animations = disable_animations;
        self
    }

    /// Dart `MediaQueryData(reduceMotion:)`.
    pub fn reduce_motion(mut self, reduce_motion: bool) -> MediaQueryData {
        self.reduce_motion = reduce_motion;
        self
    }

    /// Dart `MediaQueryData(boldText:)`.
    pub fn bold_text(mut self, bold_text: bool) -> MediaQueryData {
        self.bold_text = bold_text;
        self
    }

    /// Dart `MediaQueryData(supportsAnnounce:)`.
    pub fn supports_announce(mut self, supports_announce: bool) -> MediaQueryData {
        self.supports_announce = supports_announce;
        self
    }

    /// Dart `MediaQueryData(navigationMode:)`.
    pub fn navigation_mode(mut self, navigation_mode: NavigationMode) -> MediaQueryData {
        self.navigation_mode = navigation_mode;
        self
    }

    /// Dart `MediaQueryData(gestureSettings:)`.
    pub fn gesture_settings(mut self, gesture_settings: DeviceGestureSettings) -> MediaQueryData {
        self.gesture_settings = gesture_settings;
        self
    }

    /// Dart `MediaQueryData(supportsShowingSystemContextMenu:)`.
    pub fn supports_showing_system_context_menu(
        mut self,
        supports_showing_system_context_menu: bool,
    ) -> MediaQueryData {
        self.supports_showing_system_context_menu = supports_showing_system_context_menu;
        self
    }

    /// Dart `MediaQueryData(lineHeightScaleFactorOverride:)`.
    pub fn line_height_scale_factor_override(
        mut self,
        line_height_scale_factor_override: f64,
    ) -> MediaQueryData {
        self.line_height_scale_factor_override = Some(line_height_scale_factor_override);
        self
    }

    /// Dart `MediaQueryData(letterSpacingOverride:)`.
    pub fn letter_spacing_override(mut self, letter_spacing_override: f64) -> MediaQueryData {
        self.letter_spacing_override = Some(letter_spacing_override);
        self
    }

    /// Dart `MediaQueryData(wordSpacingOverride:)`.
    pub fn word_spacing_override(mut self, word_spacing_override: f64) -> MediaQueryData {
        self.word_spacing_override = Some(word_spacing_override);
        self
    }

    /// Dart `MediaQueryData(paragraphSpacingOverride:)`.
    pub fn paragraph_spacing_override(mut self, paragraph_spacing_override: f64) -> MediaQueryData {
        self.paragraph_spacing_override = Some(paragraph_spacing_override);
        self
    }

    /// Dart `MediaQueryData(displayCornerRadii:)`.
    pub fn display_corner_radii(mut self, display_corner_radii: BorderRadius) -> MediaQueryData {
        self.display_corner_radii = Some(display_corner_radii);
        self
    }

    /// The orientation of the media (e.g., whether the device is in landscape or portrait
    /// mode).
    pub fn orientation(&self) -> Orientation {
        if self.size.width() > self.size.height() {
            Orientation::Landscape
        } else {
            Orientation::Portrait
        }
    }

    /// Creates a copy of this media query data but with the given fields replaced with the
    /// new values: chain the field setters on the result
    /// (`data.copy_with().text_scaler(TextScaler::NO_SCALING)`).
    pub fn copy_with(&self) -> MediaQueryData {
        self.clone()
    }

    /// Creates a copy of this media query data but with the
    /// `line_height_scale_factor_override`, `letter_spacing_override`,
    /// `word_spacing_override`, and `paragraph_spacing_override` replaced with the given
    /// values.
    ///
    /// If an argument is `None`, then this [`MediaQueryData`] is returned with the
    /// corresponding override set to `None`.
    ///
    /// See also:
    ///
    ///  * [`MediaQuery::apply_text_style_overrides`], which uses this method to apply text
    ///    style overrides to the ambient [`MediaQuery`].
    pub fn apply_text_style_overrides(
        &self,
        line_height_scale_factor_override: Option<f64>,
        letter_spacing_override: Option<f64>,
        word_spacing_override: Option<f64>,
        paragraph_spacing_override: Option<f64>,
    ) -> MediaQueryData {
        MediaQueryData {
            line_height_scale_factor_override,
            letter_spacing_override,
            word_spacing_override,
            paragraph_spacing_override,
            ..self.clone()
        }
    }

    /// Creates a copy of this media query data but with the `display_corner_radii` replaced
    /// with the given value.
    ///
    /// If the argument is `None`, then this [`MediaQueryData`] is returned with the
    /// `display_corner_radii` set to `None`.
    pub fn apply_display_corner_radii(
        &self,
        display_corner_radii: Option<BorderRadius>,
    ) -> MediaQueryData {
        MediaQueryData {
            display_corner_radii,
            ..self.clone()
        }
    }

    /// Creates a copy of this media query data but with the given [`padding`](Self::padding)s
    /// replaced with zero.
    ///
    /// If all four of the `remove_left`, `remove_top`, `remove_right`, and `remove_bottom`
    /// arguments are false, then this [`MediaQueryData`] is returned unmodified.
    ///
    /// See also:
    ///
    ///  * [`MediaQuery::remove_padding`], which uses this method to remove
    ///    [`padding`](Self::padding) from the ambient [`MediaQuery`].
    ///  * `SafeArea`, which both removes the padding from the [`MediaQuery`] and adds a
    ///    `Padding` widget.
    ///  * [`remove_view_insets`](Self::remove_view_insets), the same thing but for
    ///    [`view_insets`](Self::view_insets).
    ///  * [`remove_view_padding`](Self::remove_view_padding), the same thing but for
    ///    [`view_padding`](Self::view_padding).
    pub fn remove_padding(
        &self,
        remove_left: bool,
        remove_top: bool,
        remove_right: bool,
        remove_bottom: bool,
    ) -> MediaQueryData {
        if !(remove_left || remove_top || remove_right || remove_bottom) {
            return self.clone();
        }
        self.copy_with()
            .padding(self.padding.copy_with(
                remove_left.then_some(0.0),
                remove_top.then_some(0.0),
                remove_right.then_some(0.0),
                remove_bottom.then_some(0.0),
            ))
            .view_padding(self.view_padding.copy_with(
                remove_left.then(|| (self.view_padding.left - self.padding.left).max(0.0)),
                remove_top.then(|| (self.view_padding.top - self.padding.top).max(0.0)),
                remove_right.then(|| (self.view_padding.right - self.padding.right).max(0.0)),
                remove_bottom.then(|| (self.view_padding.bottom - self.padding.bottom).max(0.0)),
            ))
    }

    /// Creates a copy of this media query data but with the given
    /// [`view_insets`](Self::view_insets) replaced with zero.
    ///
    /// If all four of the `remove_left`, `remove_top`, `remove_right`, and `remove_bottom`
    /// arguments are false, then this [`MediaQueryData`] is returned unmodified.
    ///
    /// See also:
    ///
    ///  * [`MediaQuery::remove_view_insets`], which uses this method to remove
    ///    [`view_insets`](Self::view_insets) from the ambient [`MediaQuery`].
    ///  * [`remove_padding`](Self::remove_padding), the same thing but for
    ///    [`padding`](Self::padding).
    ///  * [`remove_view_padding`](Self::remove_view_padding), the same thing but for
    ///    [`view_padding`](Self::view_padding).
    pub fn remove_view_insets(
        &self,
        remove_left: bool,
        remove_top: bool,
        remove_right: bool,
        remove_bottom: bool,
    ) -> MediaQueryData {
        if !(remove_left || remove_top || remove_right || remove_bottom) {
            return self.clone();
        }
        self.copy_with()
            .view_padding(
                self.view_padding.copy_with(
                    remove_left.then(|| (self.view_padding.left - self.view_insets.left).max(0.0)),
                    remove_top.then(|| (self.view_padding.top - self.view_insets.top).max(0.0)),
                    remove_right
                        .then(|| (self.view_padding.right - self.view_insets.right).max(0.0)),
                    remove_bottom
                        .then(|| (self.view_padding.bottom - self.view_insets.bottom).max(0.0)),
                ),
            )
            .view_insets(self.view_insets.copy_with(
                remove_left.then_some(0.0),
                remove_top.then_some(0.0),
                remove_right.then_some(0.0),
                remove_bottom.then_some(0.0),
            ))
    }

    /// Creates a copy of this media query data but with the given
    /// [`view_padding`](Self::view_padding) replaced with zero.
    ///
    /// If all four of the `remove_left`, `remove_top`, `remove_right`, and `remove_bottom`
    /// arguments are false, then this [`MediaQueryData`] is returned unmodified.
    ///
    /// See also:
    ///
    ///  * [`MediaQuery::remove_view_padding`], which uses this method to remove
    ///    [`view_padding`](Self::view_padding) from the ambient [`MediaQuery`].
    ///  * [`remove_padding`](Self::remove_padding), the same thing but for
    ///    [`padding`](Self::padding).
    ///  * [`remove_view_insets`](Self::remove_view_insets), the same thing but for
    ///    [`view_insets`](Self::view_insets).
    pub fn remove_view_padding(
        &self,
        remove_left: bool,
        remove_top: bool,
        remove_right: bool,
        remove_bottom: bool,
    ) -> MediaQueryData {
        if !(remove_left || remove_top || remove_right || remove_bottom) {
            return self.clone();
        }
        self.copy_with()
            .padding(self.padding.copy_with(
                remove_left.then_some(0.0),
                remove_top.then_some(0.0),
                remove_right.then_some(0.0),
                remove_bottom.then_some(0.0),
            ))
            .view_padding(self.view_padding.copy_with(
                remove_left.then_some(0.0),
                remove_top.then_some(0.0),
                remove_right.then_some(0.0),
                remove_bottom.then_some(0.0),
            ))
    }
}

impl Default for MediaQueryData {
    fn default() -> MediaQueryData {
        MediaQueryData::new()
    }
}

/// Dart's `==`: the scalers compare by their `textScaleFactor`, not as scalers.
impl PartialEq for MediaQueryData {
    fn eq(&self, other: &MediaQueryData) -> bool {
        other.size == self.size
            && other.device_pixel_ratio == self.device_pixel_ratio
            && other.text_scaler.text_scale_factor() == self.text_scaler.text_scale_factor()
            && other.platform_brightness == self.platform_brightness
            && other.padding == self.padding
            && other.view_padding == self.view_padding
            && other.view_insets == self.view_insets
            && other.system_gesture_insets == self.system_gesture_insets
            && other.always_use_24_hour_format == self.always_use_24_hour_format
            && other.high_contrast == self.high_contrast
            && other.on_off_switch_labels == self.on_off_switch_labels
            && other.disable_animations == self.disable_animations
            && other.reduce_motion == self.reduce_motion
            && other.invert_colors == self.invert_colors
            && other.accessible_navigation == self.accessible_navigation
            && other.bold_text == self.bold_text
            && other.supports_announce == self.supports_announce
            && other.navigation_mode == self.navigation_mode
            && other.gesture_settings == self.gesture_settings
            && other.supports_showing_system_context_menu
                == self.supports_showing_system_context_menu
            && other.line_height_scale_factor_override == self.line_height_scale_factor_override
            && other.letter_spacing_override == self.letter_spacing_override
            && other.word_spacing_override == self.word_spacing_override
            && other.paragraph_spacing_override == self.paragraph_spacing_override
            && other.display_corner_radii == self.display_corner_radii
    }
}

impl Debug for MediaQueryData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let properties = [
            format!("size: {:?}", self.size),
            format!("devicePixelRatio: {:.1}", self.device_pixel_ratio),
            format!("textScaler: {:?}", self.text_scaler),
            format!("platformBrightness: {:?}", self.platform_brightness),
            format!("padding: {:?}", self.padding),
            format!("viewPadding: {:?}", self.view_padding),
            format!("viewInsets: {:?}", self.view_insets),
            format!("systemGestureInsets: {:?}", self.system_gesture_insets),
            format!("alwaysUse24HourFormat: {}", self.always_use_24_hour_format),
            format!("accessibleNavigation: {}", self.accessible_navigation),
            format!("highContrast: {}", self.high_contrast),
            format!("onOffSwitchLabels: {}", self.on_off_switch_labels),
            format!("disableAnimations: {}", self.disable_animations),
            format!("reduceMotion: {}", self.reduce_motion),
            format!("invertColors: {}", self.invert_colors),
            format!("boldText: {}", self.bold_text),
            format!("navigationMode: {:?}", self.navigation_mode),
            format!("gestureSettings: {:?}", self.gesture_settings),
            format!(
                "supportsShowingSystemContextMenu: {}",
                self.supports_showing_system_context_menu
            ),
            format!(
                "lineHeightScaleFactorOverride: {}",
                nullable(self.line_height_scale_factor_override.as_ref())
            ),
            format!(
                "letterSpacingOverride: {}",
                nullable(self.letter_spacing_override.as_ref())
            ),
            format!(
                "wordSpacingOverride: {}",
                nullable(self.word_spacing_override.as_ref())
            ),
            format!(
                "paragraphSpacingOverride: {}",
                nullable(self.paragraph_spacing_override.as_ref())
            ),
            format!(
                "displayCornerRadii: {}",
                nullable(self.display_corner_radii.as_ref())
            ),
        ];
        write!(f, "MediaQueryData({})", properties.join(", "))
    }
}

/// Dart's string interpolation of a nullable value.
fn nullable(value: Option<&impl Debug>) -> String {
    value.map_or_else(|| "null".to_owned(), |value| format!("{value:?}"))
}

/// Dart's `EdgeInsets.fromViewPadding`: the physical `padding` scaled into logical pixels.
/// Owed to painting's `EdgeInsets`, whose `Deferred` trigger is this file.
fn edge_insets_from_view_padding(padding: ViewPadding, device_pixel_ratio: f64) -> EdgeInsets {
    EdgeInsets::from_ltrb(
        padding.left / device_pixel_ratio,
        padding.top / device_pixel_ratio,
        padding.right / device_pixel_ratio,
        padding.bottom / device_pixel_ratio,
    )
}

// ---------------------------------------------------------------------------------------------
// MediaQuery

/// Establishes a subtree in which media queries resolve to the given data.
///
/// For example, to learn the size of the current view (e.g., the view containing your app),
/// you can use [`MediaQuery::size_of`]: `MediaQuery::size_of(app, context)`.
///
/// Querying the current media using specific methods (for example, [`MediaQuery::size_of`]
/// or [`MediaQuery::padding_of`]) will cause your widget to rebuild automatically whenever
/// that specific property changes.
///
/// Querying using [`MediaQuery::of`] will cause your widget to rebuild automatically
/// whenever _any_ field of the [`MediaQueryData`] changes (e.g., if the user rotates their
/// device). Therefore, unless you are concerned with the entire [`MediaQueryData`] object
/// changing, prefer using the specific methods (for example: [`MediaQuery::size_of`] and
/// [`MediaQuery::padding_of`]), as it will rebuild more efficiently.
///
/// If no [`MediaQuery`] is in scope then [`MediaQuery::of`] and the "...Of" methods similar
/// to [`MediaQuery::size_of`] will panic. Alternatively, the "maybe-" variant methods (such
/// as [`MediaQuery::maybe_of`] and [`MediaQuery::maybe_size_of`]) can be used, which return
/// `None`, instead of panicking, when no [`MediaQuery`] is in scope.
///
/// See also:
///
///  * `WidgetsApp` and `MaterialApp`, which introduce a [`MediaQuery`] and keep it up to
///    date with the current screen metrics as they change.
///  * [`MediaQueryData`], the data structure that represents the metrics.
#[derive(Debug)]
pub struct MediaQuery {
    /// See [`InheritedWidget::key`].
    pub key: Option<KeyRef>,
    /// Contains information about the current media.
    ///
    /// For example, the [`MediaQueryData::size`] property contains the width and height of
    /// the current window.
    pub data: MediaQueryData,
    /// The widget below this widget in the tree.
    pub child: WidgetRef,
}

impl MediaQuery {
    /// Creates a widget that provides [`MediaQueryData`] to its descendants.
    pub fn new<K>(data: MediaQueryData, child: impl IntoWidget<K>) -> MediaQuery {
        MediaQuery {
            key: None,
            data,
            child: child.into_widget(),
        }
    }

    /// Dart `MediaQuery(key:)`.
    pub fn key(mut self, key: KeyRef) -> MediaQuery {
        self.key = Some(key);
        self
    }

    /// The tree node for this widget.
    ///
    /// A model is both an [`InheritedWidget`] and an [`InheritedModel`], so the kind tag of
    /// [`IntoWidget`] cannot be inferred for it; this names the conversion.
    pub fn into_widget(self) -> WidgetRef {
        IntoWidget::<InheritedModelKind>::into_widget(self)
    }

    /// Creates a new [`MediaQuery`] that inherits from the ambient [`MediaQuery`] from the
    /// given context, but removes the specified padding.
    ///
    /// This should be inserted into the widget tree when the [`MediaQuery`] padding is
    /// consumed by a widget in such a way that the padding is no longer exposed to the
    /// widget's descendants or siblings.
    ///
    /// The `context` argument must have a [`MediaQuery`] in scope.
    ///
    /// If all four of the `remove_left`, `remove_top`, `remove_right`, and `remove_bottom`
    /// arguments are false, then the returned [`MediaQuery`] reuses the ambient
    /// [`MediaQueryData`] unmodified, which is not particularly useful.
    ///
    /// See also:
    ///
    ///  * `SafeArea`, which both removes the padding from the [`MediaQuery`] and adds a
    ///    `Padding` widget.
    ///  * [`MediaQueryData::padding`], the affected property of the [`MediaQueryData`].
    ///  * [`remove_view_insets`](Self::remove_view_insets), the same thing but for
    ///    [`MediaQueryData::view_insets`].
    ///  * [`remove_view_padding`](Self::remove_view_padding), the same thing but for
    ///    [`MediaQueryData::view_padding`].
    pub fn remove_padding<K>(
        app: &mut App,
        context: BuildContext,
        remove_left: bool,
        remove_top: bool,
        remove_right: bool,
        remove_bottom: bool,
        child: impl IntoWidget<K>,
    ) -> MediaQuery {
        MediaQuery::new(
            MediaQuery::of(app, context).remove_padding(
                remove_left,
                remove_top,
                remove_right,
                remove_bottom,
            ),
            child,
        )
    }

    /// Creates a new [`MediaQuery`] that inherits from the ambient [`MediaQuery`] from the
    /// given context, but removes the specified view insets.
    ///
    /// This should be inserted into the widget tree when the [`MediaQuery`] view insets are
    /// consumed by a widget in such a way that the view insets are no longer exposed to the
    /// widget's descendants or siblings.
    ///
    /// The `context` argument must have a [`MediaQuery`] in scope.
    ///
    /// If all four of the `remove_left`, `remove_top`, `remove_right`, and `remove_bottom`
    /// arguments are false, then the returned [`MediaQuery`] reuses the ambient
    /// [`MediaQueryData`] unmodified, which is not particularly useful.
    ///
    /// See also:
    ///
    ///  * [`MediaQueryData::view_insets`], the affected property of the [`MediaQueryData`].
    ///  * [`remove_padding`](Self::remove_padding), the same thing but for
    ///    [`MediaQueryData::padding`].
    ///  * [`remove_view_padding`](Self::remove_view_padding), the same thing but for
    ///    [`MediaQueryData::view_padding`].
    pub fn remove_view_insets<K>(
        app: &mut App,
        context: BuildContext,
        remove_left: bool,
        remove_top: bool,
        remove_right: bool,
        remove_bottom: bool,
        child: impl IntoWidget<K>,
    ) -> MediaQuery {
        MediaQuery::new(
            MediaQuery::of(app, context).remove_view_insets(
                remove_left,
                remove_top,
                remove_right,
                remove_bottom,
            ),
            child,
        )
    }

    /// Creates a new [`MediaQuery`] that inherits from the ambient [`MediaQuery`] from the
    /// given context, but removes the specified view padding.
    ///
    /// This should be inserted into the widget tree when the [`MediaQuery`] view padding is
    /// consumed by a widget in such a way that the view padding is no longer exposed to the
    /// widget's descendants or siblings.
    ///
    /// The `context` argument must have a [`MediaQuery`] in scope.
    ///
    /// If all four of the `remove_left`, `remove_top`, `remove_right`, and `remove_bottom`
    /// arguments are false, then the returned [`MediaQuery`] reuses the ambient
    /// [`MediaQueryData`] unmodified, which is not particularly useful.
    ///
    /// See also:
    ///
    ///  * [`MediaQueryData::view_padding`], the affected property of the
    ///    [`MediaQueryData`].
    ///  * [`remove_padding`](Self::remove_padding), the same thing but for
    ///    [`MediaQueryData::padding`].
    ///  * [`remove_view_insets`](Self::remove_view_insets), the same thing but for
    ///    [`MediaQueryData::view_insets`].
    pub fn remove_view_padding<K>(
        app: &mut App,
        context: BuildContext,
        remove_left: bool,
        remove_top: bool,
        remove_right: bool,
        remove_bottom: bool,
        child: impl IntoWidget<K>,
    ) -> MediaQuery {
        MediaQuery::new(
            MediaQuery::of(app, context).remove_view_padding(
                remove_left,
                remove_top,
                remove_right,
                remove_bottom,
            ),
            child,
        )
    }

    /// Wraps the `child` in a [`MediaQuery`] with its
    /// [`MediaQueryData::line_height_scale_factor_override`],
    /// [`MediaQueryData::letter_spacing_override`],
    /// [`MediaQueryData::word_spacing_override`],
    /// [`MediaQueryData::paragraph_spacing_override`] set to the specified values.
    ///
    /// If a text style override argument is `None`, then the corresponding override in the
    /// updated [`MediaQueryData`] is set to `None`.
    ///
    /// The returned widget must be inserted in a widget tree below an existing
    /// [`MediaQuery`] widget.
    ///
    /// See also:
    ///
    ///  * [`MediaQueryData::line_height_scale_factor_override`],
    ///    [`MediaQueryData::letter_spacing_override`],
    ///    [`MediaQueryData::word_spacing_override`],
    ///    [`MediaQueryData::paragraph_spacing_override`], the affected properties of the
    ///    [`MediaQueryData`].
    pub fn apply_text_style_overrides<K>(
        key: Option<KeyRef>,
        line_height_scale_factor_override: Option<f64>,
        letter_spacing_override: Option<f64>,
        word_spacing_override: Option<f64>,
        paragraph_spacing_override: Option<f64>,
        child: impl IntoWidget<K>,
    ) -> WidgetRef {
        let child = child.into_widget();
        let builder = Builder::new(move |app, context| {
            MediaQuery::new(
                MediaQuery::of(app, context).apply_text_style_overrides(
                    line_height_scale_factor_override,
                    letter_spacing_override,
                    word_spacing_override,
                    paragraph_spacing_override,
                ),
                child.clone(),
            )
            .into_widget()
        });
        Self::keyed(builder, key).into_widget()
    }

    /// Wraps the `child` in a [`MediaQuery`] which is built using data from the provided
    /// `view`.
    ///
    /// The [`MediaQuery`] is constructed using the platform-specific data of the surrounding
    /// [`MediaQuery`] and the view-specific data of the provided `view`. If no surrounding
    /// [`MediaQuery`] exists, the platform-specific data is generated from the platform
    /// associated with the provided `view`. Any information that's exposed via the platform
    /// is considered platform-specific. Data exposed directly on the view is considered
    /// view-specific.
    ///
    /// The injected [`MediaQuery`] updates when the surrounding [`MediaQuery`] or the `view`
    /// changes; updates to the view's own metrics wait with the `WidgetsBindingObserver`
    /// (see `PORTING.md`).
    pub fn from_view<K>(
        key: Option<KeyRef>,
        view: ViewRef,
        child: impl IntoWidget<K>,
    ) -> WidgetRef {
        let from_view = MediaQueryFromView::new(view, child);
        match key {
            Some(key) => from_view.key(key),
            None => from_view,
        }
        .into_widget()
    }

    /// Wraps the `child` in a [`MediaQuery`] with its [`MediaQueryData::text_scaler`] set to
    /// [`TextScaler::NO_SCALING`].
    ///
    /// The returned widget must be inserted in a widget tree below an existing
    /// [`MediaQuery`] widget.
    ///
    /// This can be used to prevent, for example, icon fonts from scaling as the user adjusts
    /// the platform's text scaling value.
    pub fn with_no_text_scaling<K>(key: Option<KeyRef>, child: impl IntoWidget<K>) -> WidgetRef {
        let child = child.into_widget();
        let builder = Builder::new(move |app, context| {
            MediaQuery::new(
                MediaQuery::of(app, context)
                    .copy_with()
                    .text_scaler(TextScaler::NO_SCALING),
                child.clone(),
            )
            .into_widget()
        });
        Self::keyed(builder, key).into_widget()
    }

    /// Wraps the `child` in a [`MediaQuery`] and applies [`TextScaler::clamp`] on the current
    /// [`MediaQueryData::text_scaler`].
    ///
    /// The returned widget must be inserted in a widget tree below an existing
    /// [`MediaQuery`] widget.
    ///
    /// This is a convenience function to restrict the range of the scaled text size to
    /// `[min_scale_factor * font_size, max_scale_factor * font_size]` (to prevent excessive
    /// text scaling that would break the UI, for example). When `min_scale_factor` equals
    /// `max_scale_factor`, the scaler becomes `TextScaler::linear(min_scale_factor)`.
    ///
    /// Dart accepts a `key` here and does not pass it to its `Builder`; neither does this.
    pub fn with_clamped_text_scaling<K>(
        _key: Option<KeyRef>,
        min_scale_factor: f64,
        max_scale_factor: f64,
        child: impl IntoWidget<K>,
    ) -> WidgetRef {
        debug_assert!(max_scale_factor >= min_scale_factor);
        debug_assert!(!max_scale_factor.is_nan());
        debug_assert!(min_scale_factor.is_finite());
        debug_assert!(min_scale_factor >= 0.0);

        let child = child.into_widget();
        Builder::new(move |app, context| {
            let data = MediaQuery::of(app, context);
            let text_scaler = data.text_scaler.clamp(min_scale_factor, max_scale_factor);
            MediaQuery::new(data.copy_with().text_scaler(text_scaler), child.clone()).into_widget()
        })
        .into_widget()
    }

    /// The `Builder` behind a static `MediaQuery` wrapper, under Dart's `Key? key`.
    fn keyed(builder: Builder, key: Option<KeyRef>) -> Builder {
        match key {
            Some(key) => builder.key(key),
            None => builder,
        }
    }

    /// The data from the closest instance of this class that encloses the given context.
    ///
    /// You can use this function to query the entire set of data held in the current
    /// [`MediaQueryData`] object. When any of that information changes, your widget will be
    /// scheduled to be rebuilt, keeping your widget up-to-date.
    ///
    /// Since it is typical that the widget only requires a subset of properties of the
    /// [`MediaQueryData`] object, prefer using the more specific methods (for example:
    /// [`size_of`](Self::size_of) and [`padding_of`](Self::padding_of)), as those methods
    /// will not cause a widget to rebuild when unrelated properties are updated.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// let media = MediaQuery::of(app, context);
    /// ```
    ///
    /// If there is no [`MediaQuery`] in scope, this method panics.
    ///
    /// See also:
    ///
    /// * [`maybe_of`](Self::maybe_of), which doesn't panic if it doesn't find a
    ///   [`MediaQuery`] ancestor. It returns `None` instead.
    /// * [`size_of`](Self::size_of) and other specific methods for retrieving and depending
    ///   on changes of a specific value.
    pub fn of(app: &mut App, context: BuildContext) -> MediaQueryData {
        Self::of_aspect(app, context, None)
    }

    /// Dart's `_of`: the ambient data, depending on `aspect`; panics with
    /// `debugCheckHasMediaQuery`'s summary when there is none.
    fn of_aspect(
        app: &mut App,
        context: BuildContext,
        aspect: Option<MediaQueryAspect>,
    ) -> MediaQueryData {
        Self::maybe_of_aspect(app, context, aspect).expect(
            "No MediaQuery widget ancestor found. No MediaQuery ancestor could be found \
             starting from the context that was passed to MediaQuery.of(). This can happen \
             because the context used is not a descendant of a View widget, which introduces \
             a MediaQuery.",
        )
    }

    /// The data from the closest instance of this class that encloses the given context, if
    /// any.
    ///
    /// Use this function if you want to allow situations where no [`MediaQuery`] is in
    /// scope. Prefer using [`of`](Self::of) in situations where a media query is always
    /// expected to exist.
    ///
    /// If there is no [`MediaQuery`] in scope, then this function will return `None`.
    ///
    /// You can use this function to query the entire set of data held in the current
    /// [`MediaQueryData`] object. When any of that information changes, your widget will be
    /// scheduled to be rebuilt, keeping your widget up-to-date.
    ///
    /// Since it is typical that the widget only requires a subset of properties of the
    /// [`MediaQueryData`] object, prefer using the more specific methods (for example:
    /// [`maybe_size_of`](Self::maybe_size_of) and
    /// [`maybe_padding_of`](Self::maybe_padding_of)), as those methods will not cause a
    /// widget to rebuild when unrelated properties are updated.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// let Some(media_query) = MediaQuery::maybe_of(app, context) else {
    ///     // Do something else instead.
    /// };
    /// ```
    ///
    /// See also:
    ///
    /// * [`of`](Self::of), which will panic if it doesn't find a [`MediaQuery`] ancestor,
    ///   instead of returning `None`.
    /// * [`maybe_size_of`](Self::maybe_size_of) and other specific methods for retrieving
    ///   and depending on changes of a specific value.
    pub fn maybe_of(app: &mut App, context: BuildContext) -> Option<MediaQueryData> {
        Self::maybe_of_aspect(app, context, None)
    }

    /// Dart's `_maybeOf`: the ambient data, depending on `aspect`, if any.
    fn maybe_of_aspect(
        app: &mut App,
        context: BuildContext,
        aspect: Option<MediaQueryAspect>,
    ) -> Option<MediaQueryData> {
        MediaQuery::inherit_from(app, context, aspect).map(|media_query| media_query.data.clone())
    }

    /// Returns [`MediaQueryData::size`] from the nearest [`MediaQuery`] ancestor or panics,
    /// if no such ancestor exists.
    ///
    /// Use of this method will cause the given `context` to rebuild any time that the
    /// [`MediaQueryData::size`] property of the ancestor [`MediaQuery`] changes.
    ///
    /// Prefer using this function over getting the attribute directly from the
    /// [`MediaQueryData`] returned from [`of`](Self::of), because using this function will
    /// only rebuild the `context` when this specific attribute changes, not when _any_
    /// attribute changes.
    pub fn size_of(app: &mut App, context: BuildContext) -> Size {
        Self::of_aspect(app, context, Some(MediaQueryAspect::Size)).size
    }

    /// Returns [`MediaQueryData::size`] from the nearest [`MediaQuery`] ancestor or `None`,
    /// if no such ancestor exists.
    ///
    /// Use of this method will cause the given `context` to rebuild any time that the
    /// [`MediaQueryData::size`] property of the ancestor [`MediaQuery`] changes.
    ///
    /// Prefer using this function over getting the attribute directly from the
    /// [`MediaQueryData`] returned from [`maybe_of`](Self::maybe_of), because using this
    /// function will only rebuild the `context` when this specific attribute changes, not
    /// when _any_ attribute changes.
    pub fn maybe_size_of(app: &mut App, context: BuildContext) -> Option<Size> {
        Self::maybe_of_aspect(app, context, Some(MediaQueryAspect::Size)).map(|data| data.size)
    }

    /// Returns width of [`MediaQueryData::size`] from the nearest [`MediaQuery`] ancestor or
    /// panics, if no such ancestor exists.
    ///
    /// Use of this method will cause the given `context` to rebuild any time that the width
    /// of [`MediaQueryData::size`] property of the ancestor [`MediaQuery`] changes.
    pub fn width_of(app: &mut App, context: BuildContext) -> f64 {
        Self::of_aspect(app, context, Some(MediaQueryAspect::Width))
            .size
            .width()
    }

    /// Returns width of [`MediaQueryData::size`] from the nearest [`MediaQuery`] ancestor or
    /// `None`, if no such ancestor exists.
    ///
    /// Use of this method will cause the given `context` to rebuild any time that the width
    /// of [`MediaQueryData::size`] property of the ancestor [`MediaQuery`] changes.
    pub fn maybe_width_of(app: &mut App, context: BuildContext) -> Option<f64> {
        Self::maybe_of_aspect(app, context, Some(MediaQueryAspect::Width))
            .map(|data| data.size.width())
    }

    /// Returns height of [`MediaQueryData::size`] from the nearest [`MediaQuery`] ancestor
    /// or panics, if no such ancestor exists.
    ///
    /// Use of this method will cause the given `context` to rebuild any time that the height
    /// of [`MediaQueryData::size`] property of the ancestor [`MediaQuery`] changes.
    pub fn height_of(app: &mut App, context: BuildContext) -> f64 {
        Self::of_aspect(app, context, Some(MediaQueryAspect::Height))
            .size
            .height()
    }

    /// Returns height of [`MediaQueryData::size`] from the nearest [`MediaQuery`] ancestor
    /// or `None`, if no such ancestor exists.
    ///
    /// Use of this method will cause the given `context` to rebuild any time that the height
    /// of [`MediaQueryData::size`] property of the ancestor [`MediaQuery`] changes.
    pub fn maybe_height_of(app: &mut App, context: BuildContext) -> Option<f64> {
        Self::maybe_of_aspect(app, context, Some(MediaQueryAspect::Height))
            .map(|data| data.size.height())
    }

    /// Returns [`MediaQueryData::orientation`] for the nearest [`MediaQuery`] ancestor or
    /// panics, if no such ancestor exists.
    ///
    /// Use of this method will cause the given `context` to rebuild any time that the
    /// [`MediaQueryData::orientation`] property of the ancestor [`MediaQuery`] changes.
    pub fn orientation_of(app: &mut App, context: BuildContext) -> Orientation {
        Self::of_aspect(app, context, Some(MediaQueryAspect::Orientation)).orientation()
    }

    /// Returns [`MediaQueryData::orientation`] for the nearest [`MediaQuery`] ancestor or
    /// `None`, if no such ancestor exists.
    ///
    /// Use of this method will cause the given `context` to rebuild any time that the
    /// [`MediaQueryData::orientation`] property of the ancestor [`MediaQuery`] changes.
    pub fn maybe_orientation_of(app: &mut App, context: BuildContext) -> Option<Orientation> {
        Self::maybe_of_aspect(app, context, Some(MediaQueryAspect::Orientation))
            .map(|data| data.orientation())
    }

    /// Returns [`MediaQueryData::device_pixel_ratio`] for the nearest [`MediaQuery`]
    /// ancestor or panics, if no such ancestor exists.
    ///
    /// Use of this method will cause the given `context` to rebuild any time that the
    /// [`MediaQueryData::device_pixel_ratio`] property of the ancestor [`MediaQuery`]
    /// changes.
    pub fn device_pixel_ratio_of(app: &mut App, context: BuildContext) -> f64 {
        Self::of_aspect(app, context, Some(MediaQueryAspect::DevicePixelRatio)).device_pixel_ratio
    }

    /// Returns [`MediaQueryData::device_pixel_ratio`] for the nearest [`MediaQuery`]
    /// ancestor or `None`, if no such ancestor exists.
    ///
    /// Use of this method will cause the given `context` to rebuild any time that the
    /// [`MediaQueryData::device_pixel_ratio`] property of the ancestor [`MediaQuery`]
    /// changes.
    pub fn maybe_device_pixel_ratio_of(app: &mut App, context: BuildContext) -> Option<f64> {
        Self::maybe_of_aspect(app, context, Some(MediaQueryAspect::DevicePixelRatio))
            .map(|data| data.device_pixel_ratio)
    }

    /// Returns the [`MediaQueryData::text_scaler`] for the nearest [`MediaQuery`] ancestor
    /// or [`TextScaler::NO_SCALING`] if no such ancestor exists.
    ///
    /// Use of this method will cause the given `context` to rebuild any time that the
    /// [`MediaQueryData::text_scaler`] property of the ancestor [`MediaQuery`] changes.
    pub fn text_scaler_of(app: &mut App, context: BuildContext) -> TextScaler {
        Self::maybe_text_scaler_of(app, context).unwrap_or(TextScaler::NO_SCALING)
    }

    /// Returns the [`MediaQueryData::text_scaler`] for the nearest [`MediaQuery`] ancestor
    /// or `None` if no such ancestor exists.
    ///
    /// Use of this method will cause the given `context` to rebuild any time that the
    /// [`MediaQueryData::text_scaler`] property of the ancestor [`MediaQuery`] changes.
    pub fn maybe_text_scaler_of(app: &mut App, context: BuildContext) -> Option<TextScaler> {
        Self::maybe_of_aspect(app, context, Some(MediaQueryAspect::TextScaler))
            .map(|data| data.text_scaler)
    }

    /// Returns [`MediaQueryData::platform_brightness`] for the nearest [`MediaQuery`]
    /// ancestor or [`Brightness::Light`], if no such ancestor exists.
    ///
    /// Use of this method will cause the given `context` to rebuild any time that the
    /// [`MediaQueryData::platform_brightness`] property of the ancestor [`MediaQuery`]
    /// changes.
    pub fn platform_brightness_of(app: &mut App, context: BuildContext) -> Brightness {
        Self::maybe_platform_brightness_of(app, context).unwrap_or(Brightness::Light)
    }

    /// Returns [`MediaQueryData::platform_brightness`] for the nearest [`MediaQuery`]
    /// ancestor or `None`, if no such ancestor exists.
    ///
    /// Use of this method will cause the given `context` to rebuild any time that the
    /// [`MediaQueryData::platform_brightness`] property of the ancestor [`MediaQuery`]
    /// changes.
    pub fn maybe_platform_brightness_of(
        app: &mut App,
        context: BuildContext,
    ) -> Option<Brightness> {
        Self::maybe_of_aspect(app, context, Some(MediaQueryAspect::PlatformBrightness))
            .map(|data| data.platform_brightness)
    }

    /// Returns [`MediaQueryData::padding`] for the nearest [`MediaQuery`] ancestor or panics,
    /// if no such ancestor exists.
    ///
    /// Use of this method will cause the given `context` to rebuild any time that the
    /// [`MediaQueryData::padding`] property of the ancestor [`MediaQuery`] changes.
    pub fn padding_of(app: &mut App, context: BuildContext) -> EdgeInsets {
        Self::of_aspect(app, context, Some(MediaQueryAspect::Padding)).padding
    }

    /// Returns [`MediaQueryData::padding`] for the nearest [`MediaQuery`] ancestor or
    /// `None`, if no such ancestor exists.
    ///
    /// Use of this method will cause the given `context` to rebuild any time that the
    /// [`MediaQueryData::padding`] property of the ancestor [`MediaQuery`] changes.
    pub fn maybe_padding_of(app: &mut App, context: BuildContext) -> Option<EdgeInsets> {
        Self::maybe_of_aspect(app, context, Some(MediaQueryAspect::Padding))
            .map(|data| data.padding)
    }

    /// Returns [`MediaQueryData::view_insets`] for the nearest [`MediaQuery`] ancestor or
    /// panics, if no such ancestor exists.
    ///
    /// Use of this method will cause the given `context` to rebuild any time that the
    /// [`MediaQueryData::view_insets`] property of the ancestor [`MediaQuery`] changes.
    pub fn view_insets_of(app: &mut App, context: BuildContext) -> EdgeInsets {
        Self::of_aspect(app, context, Some(MediaQueryAspect::ViewInsets)).view_insets
    }

    /// Returns [`MediaQueryData::view_insets`] for the nearest [`MediaQuery`] ancestor or
    /// `None`, if no such ancestor exists.
    ///
    /// Use of this method will cause the given `context` to rebuild any time that the
    /// [`MediaQueryData::view_insets`] property of the ancestor [`MediaQuery`] changes.
    pub fn maybe_view_insets_of(app: &mut App, context: BuildContext) -> Option<EdgeInsets> {
        Self::maybe_of_aspect(app, context, Some(MediaQueryAspect::ViewInsets))
            .map(|data| data.view_insets)
    }

    /// Returns [`MediaQueryData::system_gesture_insets`] for the nearest [`MediaQuery`]
    /// ancestor or panics, if no such ancestor exists.
    ///
    /// Use of this method will cause the given `context` to rebuild any time that the
    /// [`MediaQueryData::system_gesture_insets`] property of the ancestor [`MediaQuery`]
    /// changes.
    pub fn system_gesture_insets_of(app: &mut App, context: BuildContext) -> EdgeInsets {
        Self::of_aspect(app, context, Some(MediaQueryAspect::SystemGestureInsets))
            .system_gesture_insets
    }

    /// Returns [`MediaQueryData::system_gesture_insets`] for the nearest [`MediaQuery`]
    /// ancestor or `None`, if no such ancestor exists.
    ///
    /// Use of this method will cause the given `context` to rebuild any time that the
    /// [`MediaQueryData::system_gesture_insets`] property of the ancestor [`MediaQuery`]
    /// changes.
    pub fn maybe_system_gesture_insets_of(
        app: &mut App,
        context: BuildContext,
    ) -> Option<EdgeInsets> {
        Self::maybe_of_aspect(app, context, Some(MediaQueryAspect::SystemGestureInsets))
            .map(|data| data.system_gesture_insets)
    }

    /// Returns [`MediaQueryData::view_padding`] for the nearest [`MediaQuery`] ancestor or
    /// panics, if no such ancestor exists.
    ///
    /// Use of this method will cause the given `context` to rebuild any time that the
    /// [`MediaQueryData::view_padding`] property of the ancestor [`MediaQuery`] changes.
    pub fn view_padding_of(app: &mut App, context: BuildContext) -> EdgeInsets {
        Self::of_aspect(app, context, Some(MediaQueryAspect::ViewPadding)).view_padding
    }

    /// Returns [`MediaQueryData::view_padding`] for the nearest [`MediaQuery`] ancestor or
    /// `None`, if no such ancestor exists.
    ///
    /// Use of this method will cause the given `context` to rebuild any time that the
    /// [`MediaQueryData::view_padding`] property of the ancestor [`MediaQuery`] changes.
    pub fn maybe_view_padding_of(app: &mut App, context: BuildContext) -> Option<EdgeInsets> {
        Self::maybe_of_aspect(app, context, Some(MediaQueryAspect::ViewPadding))
            .map(|data| data.view_padding)
    }

    /// Returns [`MediaQueryData::always_use_24_hour_format`] for the nearest [`MediaQuery`]
    /// ancestor or panics, if no such ancestor exists.
    ///
    /// Use of this method will cause the given `context` to rebuild any time that the
    /// [`MediaQueryData::always_use_24_hour_format`] property of the ancestor
    /// [`MediaQuery`] changes.
    pub fn always_use_24_hour_format_of(app: &mut App, context: BuildContext) -> bool {
        Self::of_aspect(app, context, Some(MediaQueryAspect::AlwaysUse24HourFormat))
            .always_use_24_hour_format
    }

    /// Returns [`MediaQueryData::always_use_24_hour_format`] for the nearest [`MediaQuery`]
    /// ancestor or `None`, if no such ancestor exists.
    ///
    /// Use of this method will cause the given `context` to rebuild any time that the
    /// [`MediaQueryData::always_use_24_hour_format`] property of the ancestor
    /// [`MediaQuery`] changes.
    pub fn maybe_always_use_24_hour_format_of(
        app: &mut App,
        context: BuildContext,
    ) -> Option<bool> {
        Self::maybe_of_aspect(app, context, Some(MediaQueryAspect::AlwaysUse24HourFormat))
            .map(|data| data.always_use_24_hour_format)
    }

    /// Returns [`MediaQueryData::accessible_navigation`] for the nearest [`MediaQuery`]
    /// ancestor or panics, if no such ancestor exists.
    ///
    /// Use of this method will cause the given `context` to rebuild any time that the
    /// [`MediaQueryData::accessible_navigation`] property of the ancestor [`MediaQuery`]
    /// changes.
    pub fn accessible_navigation_of(app: &mut App, context: BuildContext) -> bool {
        Self::of_aspect(app, context, Some(MediaQueryAspect::AccessibleNavigation))
            .accessible_navigation
    }

    /// Returns [`MediaQueryData::accessible_navigation`] for the nearest [`MediaQuery`]
    /// ancestor or `None`, if no such ancestor exists.
    ///
    /// Use of this method will cause the given `context` to rebuild any time that the
    /// [`MediaQueryData::accessible_navigation`] property of the ancestor [`MediaQuery`]
    /// changes.
    pub fn maybe_accessible_navigation_of(app: &mut App, context: BuildContext) -> Option<bool> {
        Self::maybe_of_aspect(app, context, Some(MediaQueryAspect::AccessibleNavigation))
            .map(|data| data.accessible_navigation)
    }

    /// Returns [`MediaQueryData::invert_colors`] for the nearest [`MediaQuery`] ancestor or
    /// panics, if no such ancestor exists.
    ///
    /// Use of this method will cause the given `context` to rebuild any time that the
    /// [`MediaQueryData::invert_colors`] property of the ancestor [`MediaQuery`] changes.
    pub fn invert_colors_of(app: &mut App, context: BuildContext) -> bool {
        Self::of_aspect(app, context, Some(MediaQueryAspect::InvertColors)).invert_colors
    }

    /// Returns [`MediaQueryData::invert_colors`] for the nearest [`MediaQuery`] ancestor or
    /// `None`, if no such ancestor exists.
    ///
    /// Use of this method will cause the given `context` to rebuild any time that the
    /// [`MediaQueryData::invert_colors`] property of the ancestor [`MediaQuery`] changes.
    pub fn maybe_invert_colors_of(app: &mut App, context: BuildContext) -> Option<bool> {
        Self::maybe_of_aspect(app, context, Some(MediaQueryAspect::InvertColors))
            .map(|data| data.invert_colors)
    }

    /// Returns [`MediaQueryData::high_contrast`] for the nearest [`MediaQuery`] ancestor or
    /// false, if no such ancestor exists.
    ///
    /// Use of this method will cause the given `context` to rebuild any time that the
    /// [`MediaQueryData::high_contrast`] property of the ancestor [`MediaQuery`] changes.
    pub fn high_contrast_of(app: &mut App, context: BuildContext) -> bool {
        Self::maybe_high_contrast_of(app, context).unwrap_or(false)
    }

    /// Returns [`MediaQueryData::high_contrast`] for the nearest [`MediaQuery`] ancestor or
    /// `None`, if no such ancestor exists.
    ///
    /// Use of this method will cause the given `context` to rebuild any time that the
    /// [`MediaQueryData::high_contrast`] property of the ancestor [`MediaQuery`] changes.
    pub fn maybe_high_contrast_of(app: &mut App, context: BuildContext) -> Option<bool> {
        Self::maybe_of_aspect(app, context, Some(MediaQueryAspect::HighContrast))
            .map(|data| data.high_contrast)
    }

    /// Returns [`MediaQueryData::on_off_switch_labels`] for the nearest [`MediaQuery`]
    /// ancestor or false, if no such ancestor exists.
    ///
    /// Use of this method will cause the given `context` to rebuild any time that the
    /// [`MediaQueryData::on_off_switch_labels`] property of the ancestor [`MediaQuery`]
    /// changes.
    pub fn on_off_switch_labels_of(app: &mut App, context: BuildContext) -> bool {
        Self::maybe_on_off_switch_labels_of(app, context).unwrap_or(false)
    }

    /// Returns [`MediaQueryData::on_off_switch_labels`] for the nearest [`MediaQuery`]
    /// ancestor or `None`, if no such ancestor exists.
    ///
    /// Use of this method will cause the given `context` to rebuild any time that the
    /// [`MediaQueryData::on_off_switch_labels`] property of the ancestor [`MediaQuery`]
    /// changes.
    pub fn maybe_on_off_switch_labels_of(app: &mut App, context: BuildContext) -> Option<bool> {
        Self::maybe_of_aspect(app, context, Some(MediaQueryAspect::OnOffSwitchLabels))
            .map(|data| data.on_off_switch_labels)
    }

    /// Returns [`MediaQueryData::disable_animations`] for the nearest [`MediaQuery`]
    /// ancestor or false, if no such ancestor exists.
    ///
    /// Use of this method will cause the given `context` to rebuild any time that the
    /// [`MediaQueryData::disable_animations`] property of the ancestor [`MediaQuery`]
    /// changes.
    pub fn disable_animations_of(app: &mut App, context: BuildContext) -> bool {
        Self::of_aspect(app, context, Some(MediaQueryAspect::DisableAnimations)).disable_animations
    }

    /// Returns [`MediaQueryData::disable_animations`] for the nearest [`MediaQuery`]
    /// ancestor or `None`, if no such ancestor exists.
    ///
    /// Use of this method will cause the given `context` to rebuild any time that the
    /// [`MediaQueryData::disable_animations`] property of the ancestor [`MediaQuery`]
    /// changes.
    pub fn maybe_disable_animations_of(app: &mut App, context: BuildContext) -> Option<bool> {
        Self::maybe_of_aspect(app, context, Some(MediaQueryAspect::DisableAnimations))
            .map(|data| data.disable_animations)
    }

    /// Returns [`MediaQueryData::reduce_motion`] for the nearest [`MediaQuery`] ancestor or
    /// false, if no such ancestor exists.
    ///
    /// Use of this method will cause the given `context` to rebuild any time that the
    /// [`MediaQueryData::reduce_motion`] property of the ancestor [`MediaQuery`] changes.
    pub fn reduce_motion_of(app: &mut App, context: BuildContext) -> bool {
        Self::of_aspect(app, context, Some(MediaQueryAspect::ReduceMotion)).reduce_motion
    }

    /// Returns [`MediaQueryData::reduce_motion`] for the nearest [`MediaQuery`] ancestor or
    /// `None`, if no such ancestor exists.
    ///
    /// Use of this method will cause the given `context` to rebuild any time that the
    /// [`MediaQueryData::reduce_motion`] property of the ancestor [`MediaQuery`] changes.
    pub fn maybe_reduce_motion_of(app: &mut App, context: BuildContext) -> Option<bool> {
        Self::maybe_of_aspect(app, context, Some(MediaQueryAspect::ReduceMotion))
            .map(|data| data.reduce_motion)
    }

    /// Returns the [`MediaQueryData::bold_text`] accessibility setting for the nearest
    /// [`MediaQuery`] ancestor or false, if no such ancestor exists.
    ///
    /// Use of this method will cause the given `context` to rebuild any time that the
    /// [`MediaQueryData::bold_text`] property of the ancestor [`MediaQuery`] changes.
    pub fn bold_text_of(app: &mut App, context: BuildContext) -> bool {
        Self::maybe_bold_text_of(app, context).unwrap_or(false)
    }

    /// Returns the [`MediaQueryData::bold_text`] accessibility setting for the nearest
    /// [`MediaQuery`] ancestor or `None`, if no such ancestor exists.
    ///
    /// Use of this method will cause the given `context` to rebuild any time that the
    /// [`MediaQueryData::bold_text`] property of the ancestor [`MediaQuery`] changes.
    pub fn maybe_bold_text_of(app: &mut App, context: BuildContext) -> Option<bool> {
        Self::maybe_of_aspect(app, context, Some(MediaQueryAspect::BoldText))
            .map(|data| data.bold_text)
    }

    /// Returns the [`MediaQueryData::supports_announce`] accessibility setting for the
    /// nearest [`MediaQuery`] ancestor or false, if no such ancestor exists.
    ///
    /// Use of this method will cause the given `context` to rebuild any time that the
    /// [`MediaQueryData::supports_announce`] property of the ancestor [`MediaQuery`]
    /// changes. This is especially important for `supports_announce` because it has a low
    /// frequency change rate. The performance difference between rebuilding for all media
    /// query data changes and only rebuilding for `supports_announce` is a dramatic
    /// difference.
    pub fn supports_announce_of(app: &mut App, context: BuildContext) -> bool {
        Self::maybe_supports_announce_of(app, context).unwrap_or(false)
    }

    /// Returns the [`MediaQueryData::supports_announce`] accessibility setting for the
    /// nearest [`MediaQuery`] ancestor or `None`, if no such ancestor exists.
    ///
    /// Use of this method will cause the given `context` to rebuild any time that the
    /// [`MediaQueryData::supports_announce`] property of the ancestor [`MediaQuery`]
    /// changes.
    pub fn maybe_supports_announce_of(app: &mut App, context: BuildContext) -> Option<bool> {
        Self::maybe_of_aspect(app, context, Some(MediaQueryAspect::SupportsAnnounce))
            .map(|data| data.supports_announce)
    }

    /// Returns [`MediaQueryData::navigation_mode`] for the nearest [`MediaQuery`] ancestor
    /// or panics, if no such ancestor exists.
    ///
    /// Use of this method will cause the given `context` to rebuild any time that the
    /// [`MediaQueryData::navigation_mode`] property of the ancestor [`MediaQuery`] changes.
    pub fn navigation_mode_of(app: &mut App, context: BuildContext) -> NavigationMode {
        Self::of_aspect(app, context, Some(MediaQueryAspect::NavigationMode)).navigation_mode
    }

    /// Returns [`MediaQueryData::navigation_mode`] for the nearest [`MediaQuery`] ancestor
    /// or `None`, if no such ancestor exists.
    ///
    /// Use of this method will cause the given `context` to rebuild any time that the
    /// [`MediaQueryData::navigation_mode`] property of the ancestor [`MediaQuery`] changes.
    pub fn maybe_navigation_mode_of(
        app: &mut App,
        context: BuildContext,
    ) -> Option<NavigationMode> {
        Self::maybe_of_aspect(app, context, Some(MediaQueryAspect::NavigationMode))
            .map(|data| data.navigation_mode)
    }

    /// Returns [`MediaQueryData::gesture_settings`] for the nearest [`MediaQuery`] ancestor
    /// or panics, if no such ancestor exists.
    ///
    /// Use of this method will cause the given `context` to rebuild any time that the
    /// [`MediaQueryData::gesture_settings`] property of the ancestor [`MediaQuery`] changes.
    pub fn gesture_settings_of(app: &mut App, context: BuildContext) -> DeviceGestureSettings {
        Self::of_aspect(app, context, Some(MediaQueryAspect::GestureSettings)).gesture_settings
    }

    /// Returns [`MediaQueryData::gesture_settings`] for the nearest [`MediaQuery`] ancestor
    /// or `None`, if no such ancestor exists.
    ///
    /// Use of this method will cause the given `context` to rebuild any time that the
    /// [`MediaQueryData::gesture_settings`] property of the ancestor [`MediaQuery`] changes.
    pub fn maybe_gesture_settings_of(
        app: &mut App,
        context: BuildContext,
    ) -> Option<DeviceGestureSettings> {
        Self::maybe_of_aspect(app, context, Some(MediaQueryAspect::GestureSettings))
            .map(|data| data.gesture_settings)
    }

    /// Returns [`MediaQueryData::supports_showing_system_context_menu`] for the nearest
    /// [`MediaQuery`] ancestor or panics, if no such ancestor exists.
    ///
    /// Use of this method will cause the given `context` to rebuild any time that the
    /// [`MediaQueryData::supports_showing_system_context_menu`] property of the ancestor
    /// [`MediaQuery`] changes.
    pub fn supports_showing_system_context_menu(app: &mut App, context: BuildContext) -> bool {
        Self::of_aspect(
            app,
            context,
            Some(MediaQueryAspect::SupportsShowingSystemContextMenu),
        )
        .supports_showing_system_context_menu
    }

    /// Returns [`MediaQueryData::supports_showing_system_context_menu`] for the nearest
    /// [`MediaQuery`] ancestor or `None`, if no such ancestor exists.
    ///
    /// Use of this method will cause the given `context` to rebuild any time that the
    /// [`MediaQueryData::supports_showing_system_context_menu`] property of the ancestor
    /// [`MediaQuery`] changes.
    pub fn maybe_supports_showing_system_context_menu(
        app: &mut App,
        context: BuildContext,
    ) -> Option<bool> {
        Self::maybe_of_aspect(
            app,
            context,
            Some(MediaQueryAspect::SupportsShowingSystemContextMenu),
        )
        .map(|data| data.supports_showing_system_context_menu)
    }

    /// Returns the [`MediaQueryData::line_height_scale_factor_override`] for the nearest
    /// [`MediaQuery`] ancestor or `None`, if no such ancestor exists or if the platform has
    /// not specified an override.
    ///
    /// Use of this method will cause the given `context` to rebuild any time that the
    /// [`MediaQueryData::line_height_scale_factor_override`] property of the ancestor
    /// [`MediaQuery`] changes.
    pub fn maybe_line_height_scale_factor_override_of(
        app: &mut App,
        context: BuildContext,
    ) -> Option<f64> {
        Self::maybe_of_aspect(
            app,
            context,
            Some(MediaQueryAspect::LineHeightScaleFactorOverride),
        )
        .and_then(|data| data.line_height_scale_factor_override)
    }

    /// Returns the [`MediaQueryData::letter_spacing_override`] for the nearest
    /// [`MediaQuery`] ancestor or `None`, if no such ancestor exists or if the platform has
    /// not specified an override.
    ///
    /// Use of this method will cause the given `context` to rebuild any time that the
    /// [`MediaQueryData::letter_spacing_override`] property of the ancestor [`MediaQuery`]
    /// changes.
    pub fn maybe_letter_spacing_override_of(app: &mut App, context: BuildContext) -> Option<f64> {
        Self::maybe_of_aspect(app, context, Some(MediaQueryAspect::LetterSpacingOverride))
            .and_then(|data| data.letter_spacing_override)
    }

    /// Returns the [`MediaQueryData::word_spacing_override`] for the nearest [`MediaQuery`]
    /// ancestor or `None`, if no such ancestor exists or if the platform has not specified
    /// an override.
    ///
    /// Use of this method will cause the given `context` to rebuild any time that the
    /// [`MediaQueryData::word_spacing_override`] property of the ancestor [`MediaQuery`]
    /// changes.
    pub fn maybe_word_spacing_override_of(app: &mut App, context: BuildContext) -> Option<f64> {
        Self::maybe_of_aspect(app, context, Some(MediaQueryAspect::WordSpacingOverride))
            .and_then(|data| data.word_spacing_override)
    }

    /// Returns the [`MediaQueryData::paragraph_spacing_override`] for the nearest
    /// [`MediaQuery`] ancestor or `None`, if no such ancestor exists or if the platform has
    /// not specified an override.
    ///
    /// Use of this method will cause the given `context` to rebuild any time that the
    /// [`MediaQueryData::paragraph_spacing_override`] property of the ancestor
    /// [`MediaQuery`] changes.
    pub fn maybe_paragraph_spacing_override_of(
        app: &mut App,
        context: BuildContext,
    ) -> Option<f64> {
        Self::maybe_of_aspect(
            app,
            context,
            Some(MediaQueryAspect::ParagraphSpacingOverride),
        )
        .and_then(|data| data.paragraph_spacing_override)
    }

    /// Returns [`MediaQueryData::display_corner_radii`] for the nearest [`MediaQuery`]
    /// ancestor or panics, if no such ancestor exists.
    ///
    /// Use of this method will cause the given `context` to rebuild any time that the
    /// [`MediaQueryData::display_corner_radii`] property of the ancestor [`MediaQuery`]
    /// changes.
    pub fn display_corner_radii_of(app: &mut App, context: BuildContext) -> Option<BorderRadius> {
        Self::of_aspect(app, context, Some(MediaQueryAspect::DisplayCornerRadii))
            .display_corner_radii
    }

    /// Returns [`MediaQueryData::display_corner_radii`] for the nearest [`MediaQuery`]
    /// ancestor or `None`, if no such ancestor exists.
    ///
    /// Use of this method will cause the given `context` to rebuild any time that the
    /// [`MediaQueryData::display_corner_radii`] property of the ancestor [`MediaQuery`]
    /// changes.
    pub fn maybe_display_corner_radii_of(
        app: &mut App,
        context: BuildContext,
    ) -> Option<BorderRadius> {
        Self::maybe_of_aspect(app, context, Some(MediaQueryAspect::DisplayCornerRadii))
            .and_then(|data| data.display_corner_radii)
    }
}

impl InheritedWidget for MediaQuery {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &MediaQuery) -> bool {
        self.data != old_widget.data
    }
}

impl InheritedModel for MediaQuery {
    type Aspect = MediaQueryAspect;

    fn update_should_notify_dependent(
        &self,
        old_widget: &MediaQuery,
        dependencies: &HashSet<MediaQueryAspect>,
    ) -> bool {
        let (data, old) = (&self.data, &old_widget.data);
        dependencies.iter().any(|dependency| match dependency {
            MediaQueryAspect::Size => data.size != old.size,
            MediaQueryAspect::Width => data.size.width() != old.size.width(),
            MediaQueryAspect::Height => data.size.height() != old.size.height(),
            MediaQueryAspect::Orientation => data.orientation() != old.orientation(),
            MediaQueryAspect::DevicePixelRatio => data.device_pixel_ratio != old.device_pixel_ratio,
            MediaQueryAspect::TextScaler => data.text_scaler != old.text_scaler,
            MediaQueryAspect::PlatformBrightness => {
                data.platform_brightness != old.platform_brightness
            }
            MediaQueryAspect::Padding => data.padding != old.padding,
            MediaQueryAspect::ViewInsets => data.view_insets != old.view_insets,
            MediaQueryAspect::ViewPadding => data.view_padding != old.view_padding,
            MediaQueryAspect::InvertColors => data.invert_colors != old.invert_colors,
            MediaQueryAspect::HighContrast => data.high_contrast != old.high_contrast,
            MediaQueryAspect::OnOffSwitchLabels => {
                data.on_off_switch_labels != old.on_off_switch_labels
            }
            MediaQueryAspect::DisableAnimations => {
                data.disable_animations != old.disable_animations
            }
            MediaQueryAspect::ReduceMotion => data.reduce_motion != old.reduce_motion,
            MediaQueryAspect::BoldText => data.bold_text != old.bold_text,
            MediaQueryAspect::SupportsAnnounce => data.supports_announce != old.supports_announce,
            MediaQueryAspect::NavigationMode => data.navigation_mode != old.navigation_mode,
            MediaQueryAspect::GestureSettings => data.gesture_settings != old.gesture_settings,
            MediaQueryAspect::SystemGestureInsets => {
                data.system_gesture_insets != old.system_gesture_insets
            }
            MediaQueryAspect::AccessibleNavigation => {
                data.accessible_navigation != old.accessible_navigation
            }
            MediaQueryAspect::AlwaysUse24HourFormat => {
                data.always_use_24_hour_format != old.always_use_24_hour_format
            }
            MediaQueryAspect::SupportsShowingSystemContextMenu => {
                data.supports_showing_system_context_menu
                    != old.supports_showing_system_context_menu
            }
            MediaQueryAspect::LineHeightScaleFactorOverride => {
                data.line_height_scale_factor_override != old.line_height_scale_factor_override
            }
            MediaQueryAspect::LetterSpacingOverride => {
                data.letter_spacing_override != old.letter_spacing_override
            }
            MediaQueryAspect::WordSpacingOverride => {
                data.word_spacing_override != old.word_spacing_override
            }
            MediaQueryAspect::ParagraphSpacingOverride => {
                data.paragraph_spacing_override != old.paragraph_spacing_override
            }
            MediaQueryAspect::DisplayCornerRadii => {
                data.display_corner_radii != old.display_corner_radii
            }
        })
    }
}

/// Describes the navigation mode to be set by a [`MediaQuery`] widget.
///
/// The different modes indicate the type of navigation to be used in a widget subtree for
/// those widgets sensitive to it.
///
/// Use `MediaQuery::navigation_mode_of(app, context)` to determine the navigation mode in
/// effect for the given context. Use a [`MediaQuery`] widget to set the navigation mode for
/// its descendant widgets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NavigationMode {
    /// This indicates a traditional keyboard-and-mouse navigation modality.
    ///
    /// This navigation mode is where the arrow keys can be used for secondary modification
    /// operations, like moving sliders or cursors, and disabled controls will lose focus and
    /// not be traversable.
    Traditional,

    /// This indicates a directional-based navigation mode.
    ///
    /// This navigation mode indicates that arrow keys should be reserved for navigation
    /// operations, and secondary modifications operations, like moving sliders or cursors,
    /// will use alternative bindings or be disabled.
    ///
    /// Some behaviors are also affected by this mode. For instance, disabled controls will
    /// retain focus when disabled, and will be able to receive focus (although they remain
    /// disabled) when traversed.
    Directional,
}

// ---------------------------------------------------------------------------------------------
// _MediaQueryFromView

/// Dart's `_MediaQueryFromView`: the widget behind [`MediaQuery::from_view`].
struct MediaQueryFromView {
    key: Option<KeyRef>,
    view: ViewRef,
    ignore_parent_data: bool,
    child: WidgetRef,
}

impl MediaQueryFromView {
    /// Creates a `MediaQueryFromView`; Dart's optional named arguments are the setters.
    fn new<K>(view: ViewRef, child: impl IntoWidget<K>) -> MediaQueryFromView {
        MediaQueryFromView {
            key: None,
            view,
            ignore_parent_data: false,
            child: child.into_widget(),
        }
    }

    /// Dart `_MediaQueryFromView(key:)`.
    fn key(mut self, key: KeyRef) -> MediaQueryFromView {
        self.key = Some(key);
        self
    }
}

impl Debug for MediaQueryFromView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MediaQueryFromView")
            .field("view", &self.view.id())
            .field("ignore_parent_data", &self.ignore_parent_data)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for MediaQueryFromView {
    type State = MediaQueryFromViewState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> MediaQueryFromViewState {
        MediaQueryFromViewState {
            state: StateData::new(),
            parent_data: None,
            data: None,
        }
    }
}

/// Dart's `_MediaQueryFromViewState`. Its `WidgetsBindingObserver` registration and hooks
/// wait with the observer: the data is re-derived when a dependency or the widget changes.
struct MediaQueryFromViewState {
    state: StateData<MediaQueryFromView>,
    parent_data: Option<MediaQueryData>,
    data: Option<MediaQueryData>,
}

impl MediaQueryFromViewState {
    fn update_parent_data(self: Handle<Self>, app: &mut App) {
        let parent_data = if self.widget(app).ignore_parent_data {
            None
        } else {
            MediaQuery::maybe_of(app, self.context(app))
        };
        let this = app.get_mut(self);
        this.parent_data = parent_data;
        this.data = None; // _updateData must be called again after changing parent data.
    }

    fn update_data(self: Handle<Self>, app: &mut App) {
        let view = Rc::clone(&self.widget(app).view);
        let parent_data = app.get(self).parent_data.clone();
        let new_data = MediaQueryData::from_view(app, &*view, parent_data.as_ref());
        if app.get(self).data.as_ref() != Some(&new_data) {
            self.set_state(app, |state| state.data = Some(new_data));
        }
    }
}

impl State for MediaQueryFromViewState {
    type Widget = MediaQueryFromView;
    crate::state_accessors!();

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        self.update_parent_data(app);
        self.update_data(app);
        debug_assert!(app.get(self).data.is_some());
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &MediaQueryFromView) {
        let (ignore_parent_data_changed, view_changed) = {
            let widget = self.widget(app);
            (
                widget.ignore_parent_data != old_widget.ignore_parent_data,
                !std::ptr::addr_eq(Rc::as_ptr(&old_widget.view), Rc::as_ptr(&widget.view)),
            )
        };
        if ignore_parent_data_changed {
            self.update_parent_data(app);
        }
        if app.get(self).data.is_none() || view_changed {
            self.update_data(app);
        }
        debug_assert!(app.get(self).data.is_some());
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        // Dart replaces a platform-sourced brightness with `debugBrightnessOverride` in
        // non-release mode; foundation's `debug.dart` waits.
        let effective_data = app
            .get(self)
            .data
            .clone()
            .expect("did_change_dependencies derives the data before the first build");
        MediaQuery::new(effective_data, self.widget(app).child.clone()).into_widget()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;
    use std::time::{Duration, Instant};

    use reveal_embedder::{
        Picture, Platform, PlatformRef, TargetPlatform, ViewConstraints, ViewId, ViewMetrics,
    };
    use reveal_scheduler::SchedulerBinding;

    use super::*;
    use crate::binding::run_widget;
    use crate::framework::StatelessWidget;
    use crate::test_harness::Harness;
    use crate::view::View;
    use crate::widgets::basic::SizedBox;

    /// An 800x600 physical view at 2x with a status bar, a home indicator, and a keyboard.
    struct TestView;

    impl EmbedderView for TestView {
        fn id(&self) -> ViewId {
            ViewId(0)
        }

        fn metrics(&self) -> ViewMetrics {
            let safe_area = ViewPadding {
                left: 0.0,
                top: 40.0,
                right: 0.0,
                bottom: 68.0,
            };
            ViewMetrics {
                physical_size: [800.0, 600.0],
                physical_constraints: ViewConstraints::tight(800.0, 600.0),
                device_pixel_ratio: 2.0,
                padding: safe_area,
                view_padding: safe_area,
                view_insets: ViewPadding {
                    left: 0.0,
                    top: 0.0,
                    right: 0.0,
                    bottom: 20.0,
                },
            }
        }

        fn present(&self, _picture: &Picture) {}
    }

    /// A dark-themed platform with one view, for the binding's frame pipeline.
    struct TestPlatform {
        view: ViewRef,
    }

    impl Platform for TestPlatform {
        fn target_platform(&self) -> TargetPlatform {
            TargetPlatform::IOS
        }

        fn platform_brightness(&self) -> Brightness {
            Brightness::Dark
        }

        fn request_frame(&self) {}

        fn now(&self) -> Instant {
            Instant::now()
        }

        fn wake_at(&self, _deadline: Instant) {}

        fn views(&self) -> Vec<ViewRef> {
            vec![Rc::clone(&self.view)]
        }

        fn view(&self, id: ViewId) -> Option<ViewRef> {
            (self.view.id() == id).then(|| Rc::clone(&self.view))
        }

        fn implicit_view(&self) -> Option<ViewRef> {
            Some(Rc::clone(&self.view))
        }
    }

    /// What a [`Reader`] does with its context on every build.
    type ReadAccessor = Rc<dyn Fn(&mut App, BuildContext)>;

    /// Reads one `MediaQuery` accessor on every build, counts its builds, and builds its
    /// child unchanged, so only its own dependencies decide its rebuilds.
    struct Reader {
        builds: Rc<Cell<u32>>,
        read: ReadAccessor,
        child: Option<WidgetRef>,
    }

    impl Reader {
        /// The erased reader and its build counter.
        fn counting(
            read: impl Fn(&mut App, BuildContext) + 'static,
            child: Option<WidgetRef>,
        ) -> (WidgetRef, Rc<Cell<u32>>) {
            let builds = Rc::new(Cell::new(0));
            let reader = Reader {
                builds: Rc::clone(&builds),
                read: Rc::new(read),
                child,
            }
            .into_widget();
            (reader, builds)
        }
    }

    impl Debug for Reader {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("Reader").finish_non_exhaustive()
        }
    }

    impl StatelessWidget for Reader {
        fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
            self.builds.set(self.builds.get() + 1);
            (self.read)(app, context);
            self.child
                .clone()
                .unwrap_or_else(|| SizedBox::shrink().into_widget())
        }
    }

    /// A reader that records what `MediaQuery::of` returns on each build.
    fn data_reader() -> (WidgetRef, Rc<RefCell<Vec<MediaQueryData>>>) {
        let seen = Rc::new(RefCell::new(Vec::new()));
        let (reader, _) = Reader::counting(
            {
                let seen = Rc::clone(&seen);
                move |app, context| seen.borrow_mut().push(MediaQuery::of(app, context))
            },
            None,
        );
        (reader, seen)
    }

    fn mount(app: &mut App, child: WidgetRef) -> Harness {
        let harness = Harness::mount(app, child);
        harness.pump(app);
        harness
    }

    fn pump_frame(app: &mut App, at: Duration) {
        SchedulerBinding::handle_begin_frame(app, Some(at));
        app.drain_microtasks();
        SchedulerBinding::handle_draw_frame(app);
        app.drain_microtasks();
    }

    #[test]
    fn from_view_computes_logical_size_and_padding() {
        let app = App::new();
        let data = MediaQueryData::from_view(&app, &TestView, None);
        assert_eq!(data.size, Size::new(400.0, 300.0));
        assert_eq!(data.device_pixel_ratio, 2.0);
        assert_eq!(data.padding, EdgeInsets::from_ltrb(0.0, 20.0, 0.0, 34.0));
        assert_eq!(
            data.view_padding,
            EdgeInsets::from_ltrb(0.0, 20.0, 0.0, 34.0)
        );
        assert_eq!(data.view_insets, EdgeInsets::from_ltrb(0.0, 0.0, 0.0, 10.0));
        assert_eq!(data.orientation(), Orientation::Landscape);
        assert_eq!(data.platform_brightness, Brightness::Light);
        assert_eq!(data.text_scaler, TextScaler::NO_SCALING);
        assert_eq!(data.gesture_settings, DeviceGestureSettings::new(None));
    }

    #[test]
    fn from_view_takes_the_platform_specific_data_from_platform_data() {
        let app = App::new();
        let platform_data = MediaQueryData::new()
            .platform_brightness(Brightness::Dark)
            .bold_text(true)
            .text_scaler(TextScaler::linear(2.0))
            .size(Size::new(1.0, 1.0));
        let data = MediaQueryData::from_view(&app, &TestView, Some(&platform_data));
        assert_eq!(data.platform_brightness, Brightness::Dark);
        assert!(data.bold_text);
        assert_eq!(data.text_scaler, TextScaler::linear(2.0));
        assert_eq!(
            data.size,
            Size::new(400.0, 300.0),
            "the size is view-specific"
        );
    }

    #[test]
    fn size_of_depends_only_on_the_size_aspect() {
        let mut app = App::new();
        let (whole, whole_builds) = Reader::counting(
            |app, context| {
                MediaQuery::of(app, context);
            },
            None,
        );
        let (padding, padding_builds) = Reader::counting(
            |app, context| {
                MediaQuery::padding_of(app, context);
            },
            Some(whole),
        );
        let (size, size_builds) = Reader::counting(
            |app, context| {
                MediaQuery::size_of(app, context);
            },
            Some(padding),
        );
        let builds = || (size_builds.get(), padding_builds.get(), whole_builds.get());
        let query = |data: MediaQueryData| MediaQuery::new(data, size.clone()).into_widget();
        let portrait = MediaQueryData::new().size(Size::new(100.0, 200.0));
        let padded = portrait.copy_with().padding(EdgeInsets::all(10.0));

        let harness = mount(&mut app, query(portrait.clone()));
        assert_eq!(builds(), (1, 1, 1));

        harness.set_child(&mut app, query(padded.clone()));
        harness.pump(&mut app);
        assert_eq!(
            builds(),
            (1, 2, 2),
            "a padding-only change spares the size reader"
        );

        let landscape = padded.copy_with().size(Size::new(200.0, 100.0));
        harness.set_child(&mut app, query(landscape.clone()));
        harness.pump(&mut app);
        assert_eq!(
            builds(),
            (2, 2, 3),
            "a size-only change spares the padding reader"
        );

        harness.set_child(&mut app, query(landscape));
        harness.pump(&mut app);
        assert_eq!(builds(), (2, 2, 3), "equal data does not notify");
    }

    #[test]
    fn remove_padding_zeroes_the_requested_sides() {
        let data = MediaQueryData::new()
            .padding(EdgeInsets::from_ltrb(1.0, 2.0, 3.0, 4.0))
            .view_padding(EdgeInsets::from_ltrb(5.0, 6.0, 7.0, 8.0));
        let removed = data.remove_padding(true, false, true, false);
        assert_eq!(removed.padding, EdgeInsets::from_ltrb(0.0, 2.0, 0.0, 4.0));
        assert_eq!(
            removed.view_padding,
            EdgeInsets::from_ltrb(4.0, 6.0, 4.0, 8.0),
            "the view padding loses the removed padding"
        );
        assert_eq!(data.remove_padding(false, false, false, false), data);
    }

    #[test]
    fn remove_view_insets_zeroes_the_requested_sides() {
        let data = MediaQueryData::new()
            .view_insets(EdgeInsets::from_ltrb(1.0, 2.0, 3.0, 4.0))
            .view_padding(EdgeInsets::from_ltrb(5.0, 6.0, 7.0, 8.0));
        let removed = data.remove_view_insets(false, true, false, true);
        assert_eq!(
            removed.view_insets,
            EdgeInsets::from_ltrb(1.0, 0.0, 3.0, 0.0)
        );
        assert_eq!(
            removed.view_padding,
            EdgeInsets::from_ltrb(5.0, 4.0, 7.0, 4.0)
        );
        assert_eq!(data.remove_view_insets(false, false, false, false), data);
    }

    #[test]
    fn remove_view_padding_zeroes_the_requested_sides() {
        let data = MediaQueryData::new()
            .padding(EdgeInsets::from_ltrb(1.0, 2.0, 3.0, 4.0))
            .view_padding(EdgeInsets::from_ltrb(5.0, 6.0, 7.0, 8.0));
        let removed = data.remove_view_padding(true, true, false, false);
        assert_eq!(removed.padding, EdgeInsets::from_ltrb(0.0, 0.0, 3.0, 4.0));
        assert_eq!(
            removed.view_padding,
            EdgeInsets::from_ltrb(0.0, 0.0, 7.0, 8.0)
        );
        assert_eq!(data.remove_view_padding(false, false, false, false), data);
    }

    #[test]
    #[should_panic(expected = "No MediaQuery widget ancestor found")]
    fn of_panics_with_darts_message_when_absent() {
        let mut app = App::new();
        let (reader, _) = data_reader();
        mount(&mut app, reader);
    }

    #[test]
    fn maybe_of_and_the_defaulted_accessors_tolerate_a_missing_media_query() {
        let mut app = App::new();
        let seen = Rc::new(RefCell::new(Vec::new()));
        let (reader, _) = Reader::counting(
            {
                let seen = Rc::clone(&seen);
                move |app, context| {
                    seen.borrow_mut().push((
                        MediaQuery::maybe_of(app, context),
                        MediaQuery::text_scaler_of(app, context),
                        MediaQuery::platform_brightness_of(app, context),
                        MediaQuery::bold_text_of(app, context),
                    ));
                }
            },
            None,
        );
        mount(&mut app, reader);
        assert_eq!(
            seen.borrow().as_slice(),
            [(None, TextScaler::NO_SCALING, Brightness::Light, false)]
        );
    }

    #[test]
    fn with_clamped_text_scaling_and_with_no_text_scaling_wrap_the_ambient_scaler() {
        let mut app = App::new();
        let seen = Rc::new(RefCell::new(Vec::new()));
        let (reader, _) = Reader::counting(
            {
                let seen = Rc::clone(&seen);
                move |app, context| {
                    seen.borrow_mut()
                        .push(MediaQuery::text_scaler_of(app, context));
                }
            },
            None,
        );
        let ambient = |child: WidgetRef| {
            MediaQuery::new(
                MediaQueryData::new().text_scaler(TextScaler::linear(2.0)),
                child,
            )
            .into_widget()
        };

        let harness = mount(
            &mut app,
            ambient(MediaQuery::with_clamped_text_scaling(
                None,
                0.0,
                1.5,
                reader.clone(),
            )),
        );
        assert_eq!(seen.borrow().last(), Some(&TextScaler::linear(1.5)));

        harness.set_child(
            &mut app,
            ambient(MediaQuery::with_no_text_scaling(None, reader)),
        );
        harness.pump(&mut app);
        assert_eq!(seen.borrow().last(), Some(&TextScaler::NO_SCALING));
    }

    #[test]
    fn from_view_yields_the_views_metrics_and_takes_platform_data_from_the_ambient_query() {
        let mut app = App::new();
        let view: ViewRef = Rc::new(TestView);
        let (reader, seen) = data_reader();
        let from_view = MediaQuery::from_view(None, Rc::clone(&view), reader);

        let harness = mount(&mut app, from_view.clone());
        assert_eq!(seen.borrow().len(), 1);
        let data = seen.borrow()[0].clone();
        assert_eq!(data, MediaQueryData::from_view(&app, &*view, None));
        assert_eq!(data.size, Size::new(400.0, 300.0));
        assert_eq!(data.platform_brightness, Brightness::Light);

        let ambient = |brightness: Brightness| {
            MediaQuery::new(
                MediaQueryData::new().platform_brightness(brightness),
                from_view.clone(),
            )
            .into_widget()
        };
        harness.set_child(&mut app, ambient(Brightness::Dark));
        harness.pump(&mut app);
        assert_eq!(seen.borrow().len(), 2);
        let data = seen.borrow()[1].clone();
        assert_eq!(
            data.platform_brightness,
            Brightness::Dark,
            "the ambient query supplies the platform data"
        );
        assert_eq!(
            data.size,
            Size::new(400.0, 300.0),
            "the view's own data stays"
        );

        harness.set_child(&mut app, ambient(Brightness::Light));
        harness.pump(&mut app);
        assert_eq!(seen.borrow().len(), 3, "a changed ambient query re-derives");
        assert_eq!(seen.borrow()[2].platform_brightness, Brightness::Light);
    }

    #[test]
    fn from_view_under_a_view_yields_the_views_metrics() {
        let view: ViewRef = Rc::new(TestView);
        let platform: PlatformRef = Rc::new(TestPlatform {
            view: Rc::clone(&view),
        });
        let mut app = App::with_platform(platform);
        let (reader, seen) = data_reader();
        let from_view = MediaQuery::from_view(None, Rc::clone(&view), reader);
        run_widget(
            &mut app,
            View::new(Rc::clone(&view), from_view).into_widget(),
        );
        app.elapse(Duration::ZERO);
        pump_frame(&mut app, Duration::ZERO);

        assert_eq!(seen.borrow().len(), 1);
        let data = seen.borrow()[0].clone();
        assert_eq!(data, MediaQueryData::from_view(&app, &*view, None));
        assert_eq!(data.size, Size::new(400.0, 300.0));
        assert_eq!(data.padding, EdgeInsets::from_ltrb(0.0, 20.0, 0.0, 34.0));
        assert_eq!(
            data.platform_brightness,
            Brightness::Dark,
            "the platform's brightness"
        );
    }
}
