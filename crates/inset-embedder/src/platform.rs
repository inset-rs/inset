//! Host capabilities the framework calls at runtime.
//!
//! Unlike Dart's isolate-global `PlatformDispatcher`, this object is supplied
//! by the embedder and held by `App`. Incoming host events travel through
//! `EmbedderClient`, so this interface contains requests and state only.

use std::any::Any;
use std::rc::Rc;
use std::time::Duration;

use web_time::Instant;

use crate::fonts::FontSource;
use crate::geometry::{Color, Rect};
use crate::mouse_cursor::SystemMouseCursorKind;
use crate::restoration::{RestorationMap, RestorationUpdate};
use crate::system_context_menu::SystemContextMenuItem;
use crate::{View, ViewFocusDirection, ViewFocusState, ViewId};

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

/// States that an application can be in once it is running.
///
/// States not supported on a platform are synthesized so the state machine stays the same
/// everywhere. For example, [`Hidden`](AppLifecycleState::Hidden) is synthesized on mobile
/// before [`Paused`](AppLifecycleState::Paused) when coming from
/// [`Inactive`](AppLifecycleState::Inactive), and before [`Inactive`](AppLifecycleState::Inactive)
/// when coming from [`Paused`](AppLifecycleState::Paused).
///
/// The values below are listed in the expected state machine transition order. The initial
/// state is [`Detached`](AppLifecycleState::Detached).
///
/// Flutter counterpart: `AppLifecycleState` (`dart:ui` `platform_dispatcher.dart`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AppLifecycleState {
    /// The application is still hosted by a Flutter engine but is detached from any host
    /// views.
    ///
    /// The application defaults to this state before it initializes, and can be in this
    /// state (applicable on Android, iOS, and web) after all views have been detached.
    ///
    /// When the application is in this state, the engine is running without a view.
    ///
    /// This state is only entered on iOS, Android, and web, although on all platforms it is
    /// the default state before the application begins running.
    Detached,

    /// On all platforms, this state indicates that the application is in the default
    /// running mode for a running application that has input focus and is visible.
    ///
    /// On Android, this state corresponds to the Flutter host view having focus while in
    /// Android's "resumed" state. It is possible for the Flutter app to be in the
    /// [`Inactive`](AppLifecycleState::Inactive) state while still being in Android's
    /// "onResume" state if the app has lost focus but hasn't had `Activity.onPause` called
    /// on it.
    ///
    /// On iOS and macOS, this corresponds to the app running in the foreground active
    /// state.
    Resumed,

    /// At least one view of the application is visible, but none have input focus. The
    /// application is otherwise running normally.
    ///
    /// On non-web desktop platforms, this corresponds to an application that is not in the
    /// foreground, but still has visible windows.
    ///
    /// On the web, this corresponds to an application that is running in a window or tab
    /// that does not have input focus.
    ///
    /// On iOS and macOS, this state corresponds to the Flutter host view running in the
    /// foreground inactive state. Apps transition to this state when in a phone call, when
    /// responding to a TouchID request, when entering the app switcher or the control
    /// center, or when the UIViewController hosting the Flutter app is transitioning.
    ///
    /// On Android, this corresponds to the Flutter host view running in Android's paused
    /// state (i.e. `Activity.onPause` has been called), or in Android's "resumed" state
    /// (i.e. `Activity.onResume` has been called) but does not have window focus.
    ///
    /// On Android and iOS, apps in this state should assume that they may be
    /// [`Hidden`](AppLifecycleState::Hidden) and [`Paused`](AppLifecycleState::Paused) at any
    /// time.
    Inactive,

    /// All views of an application are hidden, either because the application is about to
    /// be paused (on iOS and Android), or because it has been minimized or placed on a
    /// desktop that is no longer visible (on non-web desktop), or is running in a window or
    /// tab that is no longer visible (on the web).
    ///
    /// On iOS and Android, in order to keep the state machine the same on all platforms, a
    /// transition to this state is synthesized before the [`Paused`](AppLifecycleState::Paused)
    /// state is entered when coming from [`Inactive`](AppLifecycleState::Inactive), and
    /// before the [`Inactive`](AppLifecycleState::Inactive) state is entered when coming
    /// from [`Paused`](AppLifecycleState::Paused). This allows cross-platform implementations
    /// that want to know when an app is conceptually "hidden" to only write one handler.
    Hidden,

    /// The application is not currently visible to the user, and not responding to user
    /// input.
    ///
    /// When the application is in this state, the engine will not call
    /// `PlatformDispatcher.onBeginFrame` and `PlatformDispatcher.onDrawFrame`.
    ///
    /// This state is only entered on iOS and Android.
    Paused,
}

/// The possible responses to a request to exit the application.
///
/// The request is typically responded to by creating an `AppLifecycleListener` and supplying
/// an `on_exit_requested` callback, or by overriding
/// `WidgetsBindingObserver::did_request_app_exit`.
///
/// Flutter counterpart: `AppExitResponse` (`dart:ui` `platform_dispatcher.dart`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AppExitResponse {
    /// Exiting the application can proceed.
    Exit,

    /// Cancel the exit: do not exit the application.
    Cancel,
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

/// One line of a popup menu the host shows with its own menu system.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PopupMenuEntry {
    Item {
        label: String,
        enabled: bool,
        /// Drawn with a check mark, for a setting the menu toggles.
        checked: bool,
    },
    Separator,
}

impl PopupMenuEntry {
    pub fn item(label: impl Into<String>) -> PopupMenuEntry {
        PopupMenuEntry::Item {
            label: label.into(),
            enabled: true,
            checked: false,
        }
    }

    pub fn checked(self, checked: bool) -> PopupMenuEntry {
        match self {
            PopupMenuEntry::Item { label, enabled, .. } => PopupMenuEntry::Item {
                label,
                enabled,
                checked,
            },
            separator => separator,
        }
    }

    pub fn enabled(self, enabled: bool) -> PopupMenuEntry {
        match self {
            PopupMenuEntry::Item { label, checked, .. } => PopupMenuEntry::Item {
                label,
                enabled,
                checked,
            },
            separator => separator,
        }
    }
}

/// The long-lived host object held by the application.
///
/// Implementations must queue requests and return. They must not synchronously
/// re-enter the [`EmbedderClient`](crate::EmbedderClient) while `App` is active.
///
/// A host may offer more than this interface — a picture from a buffer only its system
/// has, a handle to its renderer. An app that knows its host reaches that through
/// [`downcast_ref`](dyn Platform::downcast_ref); the interface itself names nothing of any
/// one system.
pub trait Platform: Any {
    /// Which host this is, for behaviour that follows platform convention
    /// (Flutter `defaultTargetPlatform`).
    ///
    /// Required rather than defaulted: an embedder must say what it is, and a
    /// default would let one silently claim the wrong conventions.
    fn target_platform(&self) -> TargetPlatform;

    /// Open an encoded image, answering with a codec for its frames.
    ///
    /// Flutter dart:ui `instantiateImageCodec`. The host decodes with whatever its platform
    /// provides and uploads each frame, so the framework never sees pixels and the set of
    /// readable formats is the platform's rather than the framework's.
    ///
    /// The default answers a failure at once, so a host with no decoder leaves nobody waiting.
    fn open_image_codec(&self, bytes: std::sync::Arc<[u8]>) -> crate::ImageCodecFuture {
        let _ = bytes;
        Box::pin(std::future::ready(Err(crate::ImageDecodeError::NoDecoder)))
    }

    /// An image from pixels the app already holds, with no codec in between: Flutter's
    /// `decodeImageFromPixels`. `None` when the host has no renderer to put it on yet.
    fn import_pixels(&self, pixels: valo::PixelBuffer) -> Option<crate::Image> {
        let _ = pixels;
        None
    }

    /// Whether the host's own text input turns editing keys into edits before the framework
    /// sees them — backspace and delete, caret movement, the line and document ends.
    ///
    /// Flutter has no such question because each of its embedders is written for one host:
    /// its macOS and iOS embedders do interpret those keys, and its text field bindings for
    /// those platforms step aside so a key is not acted on twice.
    ///
    /// Defaults to false, which is what a host built on a windowing library that reports
    /// plain key presses should answer, whichever platform it runs on. A host that hands the
    /// framework the operating system's own editing commands answers true, and the field
    /// then leaves those keys to it.
    fn handles_text_editing_keys(&self) -> bool {
        false
    }

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

    /// Stores plain text on the system clipboard (Flutter's `Clipboard.setData`
    /// message on `SystemChannels.platform`); the default drops it.
    fn clipboard_set_data(&self, text: &str) {
        let _ = text;
    }

    /// Retrieves plain text from the system clipboard (Flutter's `Clipboard.getData`);
    /// the default answers [`None`].
    fn clipboard_get_data(&self) -> Option<String> {
        None
    }

    /// Whether the clipboard contains string data (Flutter's `Clipboard.hasStrings`);
    /// the default answers `false`.
    fn clipboard_has_strings(&self) -> bool {
        false
    }

    /// Whether this host can show a system-rendered context menu (Flutter
    /// `PlatformDispatcher.supportsShowingSystemContextMenu`).
    ///
    /// Defaults to `false`. A host that can show one (iOS 16+ `UIEditMenuInteraction`)
    /// answers `true`.
    fn supports_showing_system_context_menu(&self) -> bool {
        false
    }

    /// Shows the system context menu (Flutter's `ContextMenu.showSystemContextMenu`).
    ///
    /// `items` is `None` for the deprecated call that lets the platform pick default
    /// buttons. Defaults to dropping it. Built-in items are performed by the host;
    /// custom items are reported through [`crate::EmbedderClient::custom_context_menu_action`].
    fn show_system_context_menu(&self, target_rect: Rect, items: Option<&[SystemContextMenuItem]>) {
        let _ = (target_rect, items);
    }

    /// Hides the system context menu (Flutter's `ContextMenu.hideSystemContextMenu`).
    ///
    /// Defaults to dropping it.
    fn hide_system_context_menu(&self) {}

    /// Shows `entries` as the platform's own popup menu at the pointer and waits for it to
    /// close; answers the index of the entry chosen. Not a Flutter call: Flutter's context
    /// menus draw inside the view, and a desktop panel narrower than its menu needs the
    /// system's, which is its own window.
    ///
    /// The host runs the menu's event loop inside this call, so nothing else of the app runs
    /// meanwhile and a native callback that arrives then must post, not borrow. Defaults to
    /// `None`, for a host with no menus.
    fn show_popup_menu(&self, entries: &[PopupMenuEntry]) -> Option<usize> {
        let _ = entries;
        None
    }

    /// Looks up the given text (Flutter's `LookUp.invoke`).
    ///
    /// Defaults to dropping it.
    fn look_up(&self, text: &str) {
        let _ = text;
    }

    /// Searches the web for the given text (Flutter's `SearchWeb.invoke`).
    ///
    /// Defaults to dropping it.
    fn search_web(&self, text: &str) {
        let _ = text;
    }

    /// Shares the given text (Flutter's `Share.invoke`).
    ///
    /// Defaults to dropping it.
    fn share(&self, text: &str) {
        let _ = text;
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

    /// Flutter `PlatformDispatcher.requestViewFocusChange`: asks the host to move view focus.
    /// A host without native focus support ignores the request.
    fn request_view_focus_change(
        &self,
        view_id: ViewId,
        state: ViewFocusState,
        direction: ViewFocusDirection,
    ) {
        let _ = (view_id, state, direction);
    }

    /// The stable implicit view, when this embedding provides one.
    fn implicit_view(&self) -> Option<ViewRef>;

    /// The host's window maker, for an app that opens windows of its own. `None`, the
    /// default, where the implicit view is the only one there can be: mobile and the web.
    fn windowing_owner(&self) -> Option<Rc<dyn crate::WindowingOwner>> {
        None
    }

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

impl dyn Platform {
    /// The host as its own type, for what it offers beyond the interface.
    pub fn downcast_ref<T: Platform>(&self) -> Option<&T> {
        (self as &dyn Any).downcast_ref::<T>()
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

    #[test]
    fn a_platform_downcasts_to_its_own_type_and_no_other() {
        struct Other;
        impl Platform for Other {
            fn target_platform(&self) -> TargetPlatform {
                TargetPlatform::Linux
            }
            fn request_frame(&self) {}
            fn now(&self) -> super::Instant {
                super::Instant::now()
            }
            fn wake_at(&self, _deadline: super::Instant) {}
            fn views(&self) -> Vec<super::ViewRef> {
                Vec::new()
            }
            fn view(&self, _id: super::ViewId) -> Option<super::ViewRef> {
                None
            }
            fn implicit_view(&self) -> Option<super::ViewRef> {
                None
            }
        }
        let platform: super::PlatformRef = std::rc::Rc::new(InertPlatform);
        assert!(platform.downcast_ref::<InertPlatform>().is_some());
        assert!(platform.downcast_ref::<Other>().is_none());
    }
}
