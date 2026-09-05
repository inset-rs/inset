//! Host capabilities the framework calls at runtime.
//!
//! Unlike Dart's isolate-global `PlatformDispatcher`, this object is supplied
//! by the embedder and held by `App`. Incoming host events travel through
//! `EmbedderClient`, so this interface contains requests and state only.

use std::rc::Rc;
use std::time::{Duration, Instant};

use crate::fonts::FontSource;
use crate::geometry::Color;
use crate::mouse_cursor::SystemMouseCursorKind;
use crate::restoration::{RestorationMap, RestorationUpdate};
use crate::{View, ViewId};

pub type PlatformRef = Rc<dyn Platform>;
pub type ViewRef = Rc<dyn View>;

/// Specifies a description of the application that is pertinent to the
/// embedder's application switcher (also known as "recent tasks") user
/// interface.
///
/// Flutter counterpart: `ApplicationSwitcherDescription` (`services/system_chrome.dart`).
/// It lives here because [`Platform`] names it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ApplicationSwitcherDescription {
    /// A label and description of the current state of the application.
    pub label: Option<String>,
    /// The application's primary color, as a 32-bit ARGB value.
    ///
    /// This may influence the color that the operating system uses to represent
    /// the application.
    pub primary_color: Option<u32>,
}

/// Specifies a system overlay style for a portion of the app.
///
/// Flutter counterpart: `SystemUiOverlayStyle` (`services/system_chrome.dart`).
/// It lives here because [`Platform`] names it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SystemUiOverlayStyle {
    /// The color of the system bottom navigation bar.
    ///
    /// Only honored in Android versions O and greater.
    pub system_navigation_bar_color: Option<Color>,

    /// The color of the divider between the system's bottom navigation bar and the app's
    /// content.
    ///
    /// Only honored in Android versions P and greater.
    pub system_navigation_bar_divider_color: Option<Color>,

    /// The brightness of the system navigation bar icons.
    ///
    /// Only honored in Android versions O and greater.
    /// When set to [`Brightness::Light`], the system navigation bar icons are light.
    /// When set to [`Brightness::Dark`], the system navigation bar icons are dark.
    pub system_navigation_bar_icon_brightness: Option<Brightness>,

    /// Overrides the contrast enforcement when setting a transparent navigation bar.
    ///
    /// When setting a transparent navigation bar in SDK 29+, or Android 10 and up, a
    /// translucent body scrim may be applied behind the button navigation bar to ensure
    /// contrast with buttons and the background of the application.
    ///
    /// SDK 28-, or Android P and lower, will not apply this body scrim.
    ///
    /// Setting this to false overrides the default body scrim.
    pub system_navigation_bar_contrast_enforced: Option<bool>,

    /// The color of top status bar.
    ///
    /// Only honored in Android version M and greater.
    pub status_bar_color: Option<Color>,

    /// The brightness of top status bar.
    ///
    /// Only honored in iOS.
    pub status_bar_brightness: Option<Brightness>,

    /// The brightness of the top status bar icons.
    ///
    /// Only honored in Android version M and greater.
    pub status_bar_icon_brightness: Option<Brightness>,

    /// Overrides the contrast enforcement when setting a transparent status bar.
    ///
    /// When setting a transparent status bar in SDK 29+, or Android 10 and up, a translucent
    /// body scrim may be applied to ensure contrast with icons and the background of the
    /// application.
    ///
    /// SDK 28-, or Android P and lower, will not apply this body scrim.
    ///
    /// Setting this to false overrides the default body scrim.
    pub system_status_bar_contrast_enforced: Option<bool>,
}

impl SystemUiOverlayStyle {
    /// System overlays should be drawn with a light color. Intended for applications with a
    /// dark background.
    pub const LIGHT: SystemUiOverlayStyle = SystemUiOverlayStyle {
        system_navigation_bar_color: Some(Color::new(0xFF000000)),
        system_navigation_bar_icon_brightness: Some(Brightness::Light),
        status_bar_icon_brightness: Some(Brightness::Light),
        status_bar_brightness: Some(Brightness::Dark),
        system_navigation_bar_divider_color: None,
        system_navigation_bar_contrast_enforced: None,
        status_bar_color: None,
        system_status_bar_contrast_enforced: None,
    };

    /// System overlays should be drawn with a dark color. Intended for applications with a
    /// light background.
    pub const DARK: SystemUiOverlayStyle = SystemUiOverlayStyle {
        system_navigation_bar_color: Some(Color::new(0xFF000000)),
        system_navigation_bar_icon_brightness: Some(Brightness::Light),
        status_bar_icon_brightness: Some(Brightness::Dark),
        status_bar_brightness: Some(Brightness::Light),
        system_navigation_bar_divider_color: None,
        system_navigation_bar_contrast_enforced: None,
        status_bar_color: None,
        system_status_bar_contrast_enforced: None,
    };

    /// Creates a new [`SystemUiOverlayStyle`]; Dart's named arguments are the setters.
    pub const fn new() -> SystemUiOverlayStyle {
        SystemUiOverlayStyle {
            system_navigation_bar_color: None,
            system_navigation_bar_divider_color: None,
            system_navigation_bar_icon_brightness: None,
            system_navigation_bar_contrast_enforced: None,
            status_bar_color: None,
            status_bar_brightness: None,
            status_bar_icon_brightness: None,
            system_status_bar_contrast_enforced: None,
        }
    }

    /// Dart `SystemUiOverlayStyle(systemNavigationBarColor:)`.
    pub fn system_navigation_bar_color(mut self, color: Color) -> SystemUiOverlayStyle {
        self.system_navigation_bar_color = Some(color);
        self
    }

    /// Dart `SystemUiOverlayStyle(systemNavigationBarDividerColor:)`.
    pub fn system_navigation_bar_divider_color(mut self, color: Color) -> SystemUiOverlayStyle {
        self.system_navigation_bar_divider_color = Some(color);
        self
    }

    /// Dart `SystemUiOverlayStyle(systemNavigationBarIconBrightness:)`.
    pub fn system_navigation_bar_icon_brightness(
        mut self,
        brightness: Brightness,
    ) -> SystemUiOverlayStyle {
        self.system_navigation_bar_icon_brightness = Some(brightness);
        self
    }

    /// Dart `SystemUiOverlayStyle(systemNavigationBarContrastEnforced:)`.
    pub fn system_navigation_bar_contrast_enforced(
        mut self,
        enforced: bool,
    ) -> SystemUiOverlayStyle {
        self.system_navigation_bar_contrast_enforced = Some(enforced);
        self
    }

    /// Dart `SystemUiOverlayStyle(statusBarColor:)`.
    pub fn status_bar_color(mut self, color: Color) -> SystemUiOverlayStyle {
        self.status_bar_color = Some(color);
        self
    }

    /// Dart `SystemUiOverlayStyle(statusBarBrightness:)`.
    pub fn status_bar_brightness(mut self, brightness: Brightness) -> SystemUiOverlayStyle {
        self.status_bar_brightness = Some(brightness);
        self
    }

    /// Dart `SystemUiOverlayStyle(statusBarIconBrightness:)`.
    pub fn status_bar_icon_brightness(mut self, brightness: Brightness) -> SystemUiOverlayStyle {
        self.status_bar_icon_brightness = Some(brightness);
        self
    }

    /// Dart `SystemUiOverlayStyle(systemStatusBarContrastEnforced:)`.
    pub fn system_status_bar_contrast_enforced(mut self, enforced: bool) -> SystemUiOverlayStyle {
        self.system_status_bar_contrast_enforced = Some(enforced);
        self
    }

    /// Creates a copy of this theme with the given fields replaced with new values.
    pub fn copy_with(&self) -> SystemUiOverlayStyle {
        *self
    }
}

/// The kind of haptic feedback to play.
///
/// Flutter counterpart: the `HapticFeedbackType.*` argument Dart sends with the
/// `HapticFeedback.vibrate` message on `SystemChannels.platform`. It lives here
/// because [`Platform`] names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HapticFeedbackType {
    /// Vibration for a short duration, the argument-less `HapticFeedback.vibrate`.
    Vibrate,
    /// A collision impact with a light mass.
    LightImpact,
    /// A collision impact with a medium mass.
    MediumImpact,
    /// A collision impact with a heavy mass.
    HeavyImpact,
    /// A selection changing through discrete values.
    SelectionClick,
    /// A task or action completed successfully.
    SuccessNotification,
    /// A task or action produced a warning.
    WarningNotification,
    /// A task or action failed.
    ErrorNotification,
}

/// Describes the contrast of a theme or color palette.
///
/// Flutter counterpart: `Brightness` (`dart:ui` `window.dart`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Brightness {
    /// The color is dark and will require a light text color to achieve readable
    /// contrast.
    ///
    /// For example, the color might be dark grey, requiring white text.
    Dark,

    /// The color is light and will require a dark text color to achieve readable
    /// contrast.
    ///
    /// For example, the color might be bright white, requiring black text.
    Light,
}

/// The platform that user interaction should adapt to target.
///
/// Flutter counterpart: `TargetPlatform` (`foundation/platform.dart`). Flutter
/// reads the current one from a library global that consults `dart:io`. Here it
/// is a plain value type, and the App-owned [`Platform`] reports which one it
/// is, so two Apps in one process can differ.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TargetPlatform {
    /// Android: <https://www.android.com/>
    Android,
    /// Fuchsia: <https://fuchsia.dev/fuchsia-src/concepts>
    Fuchsia,
    /// iOS: <https://www.apple.com/ios/>
    IOS,
    /// Linux: <https://www.linux.org>
    Linux,
    /// macOS: <https://www.apple.com/macos>
    MacOS,
    /// Windows: <https://www.windows.com>
    Windows,
}

/// The long-lived host object held by the application.
///
/// Implementations must queue requests and return. They must not synchronously
/// re-enter the [`EmbedderClient`](crate::EmbedderClient) while `App` is active.
pub trait Platform: 'static {
    /// Which host this is, for behaviour that follows platform convention
    /// (Flutter `defaultTargetPlatform`).
    ///
    /// Required rather than defaulted: an embedder must say what it is, and a
    /// default would let one silently claim the wrong conventions.
    fn target_platform(&self) -> TargetPlatform;

    /// The full system-reported supported locales of the device (Flutter
    /// `PlatformDispatcher.locales`), in order of preference; empty until the host
    /// reports them.
    fn locales(&self) -> Vec<crate::Locale> {
        Vec::new()
    }

    /// Tells the host which locale the application resolved to (Flutter
    /// `PlatformDispatcher.setApplicationLocale`); the default drops it.
    fn set_application_locale(&self, locale: &crate::Locale) {
        let _ = locale;
    }

    /// The route the application was started with (Flutter
    /// `PlatformDispatcher.defaultRouteName`).
    ///
    /// Defaults to `"/"`, Flutter's value for a host that was not asked to open a
    /// particular route; a host that receives a deep link answers it here.
    fn default_route_name(&self) -> String {
        String::from("/")
    }

    /// Describes the app in the host's application switcher (Flutter
    /// `SystemChrome.setApplicationSwitcherDescription`); the default drops it.
    fn set_application_switcher_description(&self, description: &ApplicationSwitcherDescription) {
        let _ = description;
    }

    /// Styles the system overlays the host draws over the app — the status bar, and on
    /// Android the system navigation bar (Flutter
    /// `SystemChrome.setSystemUIOverlayStyle`); the default drops it.
    fn set_system_ui_overlay_style(&self, style: &SystemUiOverlayStyle) {
        let _ = style;
    }

    /// Plays haptic feedback on the device (Flutter's `HapticFeedback.vibrate`
    /// message on `SystemChannels.platform`); the default drops it.
    fn haptic_feedback(&self, kind: HapticFeedbackType) {
        let _ = kind;
    }

    /// The platform's light/dark preference (Flutter
    /// `PlatformDispatcher.platformBrightness`).
    ///
    /// Defaults to [`Brightness::Light`], matching Flutter's view configuration
    /// default. A live host that can see the OS theme overrides this.
    fn platform_brightness(&self) -> Brightness {
        Brightness::Light
    }

    /// Requests one isolate frame at the host's next appropriate opportunity.
    fn request_frame(&self);

    /// The host clock used for frame and timer timestamps.
    fn now(&self) -> Instant;

    /// Asks the host to wake the application at `deadline`.
    fn wake_at(&self, deadline: Instant);

    /// Current host-provided views.
    fn views(&self) -> Vec<ViewRef>;

    /// Looks up a current host-provided view.
    fn view(&self, id: ViewId) -> Option<ViewRef>;

    /// The stable implicit view, when this embedding provides one.
    fn implicit_view(&self) -> Option<ViewRef>;

    /// The platform's own font lookup: faces by family name and by codepoint, the way
    /// Flutter's engine asks the OS. `None` for a host without one; text then shapes only
    /// against fonts the application registers.
    fn font_source(&self) -> Option<Box<dyn FontSource>> {
        None
    }

    /// Shows a system cursor for a pointing device (Flutter's `activateSystemCursor`
    /// message on `SystemChannels.mouseCursor`).
    ///
    /// Defaults to nothing: a host without a system cursor ignores the request.
    fn activate_system_cursor(&self, device: i64, kind: SystemMouseCursorKind) {
        let _ = (device, kind);
    }

    /// The restoration data the host kept for this application (the `get` message of
    /// Flutter's `SystemChannels.restoration`).
    ///
    /// Defaults to `None`, Dart's null reply: a host that cannot store restoration data
    /// leaves state restoration turned off.
    fn restoration_get(&self) -> Option<RestorationUpdate> {
        None
    }

    /// Hands the host the current restoration data (the `put` message of Flutter's
    /// `SystemChannels.restoration`), which keeps it until the operating system asks for
    /// it.
    ///
    /// Defaults to dropping it: a host without state restoration has nowhere to put it.
    fn restoration_put(&self, data: RestorationMap) {
        let _ = data;
    }
}

/// Platform for hand-pumped tests.
pub struct InertPlatform;

impl Platform for InertPlatform {
    /// Android, which is what Flutter's own test binding defaults
    /// `debugDefaultTargetPlatform` to: a bare App has no host to speak of,
    /// and a deterministic answer beats the machine's.
    fn target_platform(&self) -> TargetPlatform {
        TargetPlatform::Android
    }

    fn request_frame(&self) {}

    fn now(&self) -> Instant {
        Instant::now()
    }

    fn wake_at(&self, _deadline: Instant) {}

    fn views(&self) -> Vec<ViewRef> {
        Vec::new()
    }

    fn view(&self, _id: ViewId) -> Option<ViewRef> {
        None
    }

    fn implicit_view(&self) -> Option<ViewRef> {
        None
    }
}

/// One complete engine frame delivered to the client.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Frame {
    pub elapsed: Duration,
}

#[cfg(test)]
mod tests {
    use super::{Brightness, InertPlatform, Platform, TargetPlatform};

    #[test]
    fn inert_platform_is_android() {
        assert_eq!(InertPlatform.target_platform(), TargetPlatform::Android);
    }

    #[test]
    fn inert_platform_brightness_is_light() {
        assert_eq!(InertPlatform.platform_brightness(), Brightness::Light);
    }
}
