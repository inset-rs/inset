//! Host capabilities the framework calls at runtime.
//!
//! Unlike Dart's isolate-global `PlatformDispatcher`, this object is supplied
//! by the embedder and held by `App`. Incoming host events travel through
//! `EmbedderClient`, so this interface contains requests and state only.

use std::any::Any;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use web_time::Instant;

use crate::fonts::FontSource;
use crate::geometry::{Color, Offset, Rect};
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

/// The type of application exit to perform when calling `ServicesBinding::exit_application`.
///
/// Flutter counterpart: `AppExitType` (`dart:ui` `platform_dispatcher.dart`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AppExitType {
    /// Requests that the application start an orderly exit, sending a request back to the
    /// framework through the `WidgetsBinding`. If that responds with [`AppExitResponse::Exit`],
    /// then proceed with the same steps as a [`Required`](AppExitType::Required) exit. If that
    /// responds with [`AppExitResponse::Cancel`], then the exit request is canceled and the
    /// application continues executing normally.
    Cancelable,

    /// A non-cancelable orderly exit request. The engine will shut down the engine and call
    /// the native UI toolkit's exit API.
    ///
    /// If you need an even faster and more dangerous exit, then call
    /// [`std::process::exit`] directly, and even the native toolkit's exit API won't be
    /// called. This is quite dangerous, though, since it's possible that the engine will
    /// crash because it hasn't been properly shut down, causing the app to crash on exit.
    Required,
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

/// The host's threads as any thread reaches them: the two things the framework asks of them.
/// gpui's `PlatformDispatcher`. Flutter's embedder API has the first as `post_task_callback`,
/// with its target time, and keeps the second — the engine's own worker threads — to itself.
pub trait Dispatcher: Send + Sync {
    /// Sets when the host should next wake the application for a timer.
    ///
    /// The host keeps one such time. When it arrives, the host calls
    /// [`EmbedderClient::wake`](crate::EmbedderClient::wake) on the main thread.
    ///
    /// `None` means no timer is waiting, so the host should not wake for one.
    ///
    /// Each call replaces the previous one. The new time can be later than the old one, not
    /// only earlier. Call this only from the thread the application runs on.
    fn wake_at(&self, timer_wakeup: Option<Instant>);

    /// Asks the host to wake the application as soon as it can, because a task became ready
    /// to run.
    ///
    /// The next wake clears this request. It does not change the timer wakeup set by
    /// [`wake_at`](Self::wake_at), which is still waiting for its own time. Call this from
    /// any thread.
    fn wake_now(&self);

    /// Runs `work` off the main thread, on whatever the host has for that, such as a system
    /// queue or a worker thread. The framework's `run_in_background` sends its work here.
    fn dispatch(&self, work: Box<dyn FnOnce() + Send>);
}

/// A dispatcher for tests, which pump the application by hand: it never wakes the application,
/// and work runs at once on the calling thread, so its result is there at the next checkpoint.
pub struct InertDispatcher;

impl Dispatcher for InertDispatcher {
    fn wake_at(&self, _timer_wakeup: Option<Instant>) {}

    fn wake_now(&self) {}

    fn dispatch(&self, work: Box<dyn FnOnce() + Send>) {
        work();
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

    /// Requests one isolate frame at the host's next appropriate opportunity.
    fn request_frame(&self);

    /// The host clock used for frame and timer timestamps.
    fn now(&self) -> Instant;

    /// The host's threads, for waking the application or running work off the main thread: see
    /// [`Dispatcher`].
    fn dispatcher(&self) -> Arc<dyn Dispatcher>;

    /// Every view the app has now. An app may run with none.
    fn views(&self) -> Vec<ViewRef>;

    /// Looks up a current host-provided view.
    fn view(&self, id: ViewId) -> Option<ViewRef>;

    /// The view the platform gave the app at launch, if any. It stays the same view for the
    /// app's whole life, while its window and its surface may come and go: it may be there,
    /// with no size, before its window, and on a desktop it stays, drawing nowhere, once its
    /// window closes. `None` on a desktop configured without one, and when embedded.
    fn implicit_view(&self) -> Option<ViewRef>;

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

    /// The platform's light/dark preference (Flutter
    /// `PlatformDispatcher.platformBrightness`).
    ///
    /// Defaults to [`Brightness::Light`], matching Flutter's view configuration
    /// default. A live host that can see the OS theme overrides this.
    fn platform_brightness(&self) -> Brightness {
        Brightness::Light
    }

    /// The route the application was started with (Flutter
    /// `PlatformDispatcher.defaultRouteName`).
    ///
    /// Defaults to `"/"`, Flutter's value for a host that was not asked to open a
    /// particular route; a host that receives a deep link answers it here.
    fn default_route_name(&self) -> String {
        String::from("/")
    }

    /// The lifecycle state immediately after the application started (Flutter
    /// `PlatformDispatcher.initialLifecycleState`).
    ///
    /// This property will not be updated as the lifecycle changes: a host reports each change
    /// through `EmbedderClient::app_lifecycle_state_changed`.
    ///
    /// It is used to initialize `SchedulerBinding::lifecycle_state` at startup. Defaults to
    /// `None`, Flutter's empty string: a host that has no state to report yet.
    fn initial_lifecycle_state(&self) -> Option<AppLifecycleState> {
        None
    }

    /// The platform's own font lookup: faces by family name and by codepoint, the way
    /// Flutter's engine asks the OS. `None` for a host without one; text then shapes only
    /// against fonts the application registers.
    fn font_source(&self) -> Option<Box<dyn FontSource>> {
        None
    }

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

    /// The host's compositor, where it has one: native surfaces shown beside the
    /// framework's picture. `None` on the web, on X11, on the inert host, and on an
    /// embedder that chose not to have one.
    fn compositor(&self) -> Option<Arc<dyn crate::Compositor>> {
        None
    }

    /// Asking for another view, where the platform allows it: at once on a desktop, through
    /// the system on iPad and Android. `None`, the default, on iPhone and when embedded. The
    /// answer is the host's at the time of asking, so a host that learns only at run time
    /// whether its platform has windows answers `None` there, and an app asks before it offers
    /// a window.
    fn windowing_owner(&self) -> Option<Rc<dyn crate::WindowingOwner>> {
        None
    }

    /// The screens a window is placed on, and where the pointer is on them, or `None` where the
    /// host has no screens to place a window against: a phone, a browser tab, and a desktop
    /// whose system will not say.
    fn screens(&self) -> Option<&dyn Screens> {
        None
    }

    /// The system clipboard, or `None` where the host has none.
    fn clipboard(&self) -> Option<&dyn Clipboard> {
        None
    }

    /// The system UI drawn around the app, or `None` where the host has none.
    fn system_chrome(&self) -> Option<&dyn SystemChrome> {
        None
    }

    /// Haptic feedback on the device, or `None` where the host has none.
    fn haptics(&self) -> Option<&dyn Haptics> {
        None
    }

    /// The system's own menu over a text selection, or `None` where the host has none.
    fn system_context_menu(&self) -> Option<&dyn SystemContextMenu> {
        None
    }

    /// Native pop-up menus shown at the pointer, or `None` where the host has none.
    fn popup_menus(&self) -> Option<&dyn PopupMenus> {
        None
    }

    /// The system services a selection toolbar offers, or `None` where the host has none.
    fn text_services(&self) -> Option<&dyn TextServices> {
        None
    }

    /// The system cursor shown for a pointing device, or `None` where the host has none.
    fn mouse_cursor(&self) -> Option<&dyn MouseCursor> {
        None
    }

    /// Application state kept across relaunches, or `None` where the host has none.
    fn restoration(&self) -> Option<&dyn Restoration> {
        None
    }

    /// Quitting at the application's own request, or `None` where the host cannot quit
    /// itself: a phone, a browser tab.
    fn app_exit(&self) -> Option<&dyn AppExit> {
        None
    }

    /// The files the system asks the app to open, or `None` where the system never asks: it
    /// starts the app with them on its command line instead, or has no files to open.
    fn file_opening(&self) -> Option<&dyn FileOpening> {
        None
    }
}

impl dyn Platform {
    /// The host as its own type, for what it offers beyond the interface.
    pub fn downcast_ref<T: Platform>(&self) -> Option<&T> {
        (self as &dyn Any).downcast_ref::<T>()
    }
}

/// The system clipboard: Flutter's `Clipboard` over the platform channel. Every call answers
/// later, as Flutter's does: a host may have to ask its user, or another process, first.
pub trait Clipboard {
    /// Stores plain text on the system clipboard (Flutter's `Clipboard.setData`), and completes
    /// once it is stored.
    fn set_text(&self, text: &str) -> ClipboardSetTextFuture;

    /// Retrieves plain text from the system clipboard (Flutter's `Clipboard.getData`); `None`
    /// when it holds none, or the host could not read it.
    fn text(&self) -> ClipboardTextFuture;

    /// Whether the clipboard contains string data (Flutter's `Clipboard.hasStrings`).
    fn has_strings(&self) -> ClipboardHasStringsFuture;
}

/// The clipboard's text being stored. Flutter's `Future<void>` from `Clipboard.setData`.
pub type ClipboardSetTextFuture = Pin<Box<dyn Future<Output = ()>>>;

/// The clipboard's text on its way. Flutter's `Future<ClipboardData?>`.
pub type ClipboardTextFuture = Pin<Box<dyn Future<Output = Option<String>>>>;

/// Whether the clipboard holds text, on its way. Flutter's `Future<bool>`.
pub type ClipboardHasStringsFuture = Pin<Box<dyn Future<Output = bool>>>;

/// Flutter's `SystemChrome`: the system UI around the app.
pub trait SystemChrome {
    /// Styles the system overlays the host draws over the app — the status bar, and on
    /// Android the system navigation bar (Flutter `SystemChrome.setSystemUIOverlayStyle`).
    fn set_overlay_style(&self, style: &SystemUiOverlayStyle);

    /// Describes the app in the host's application switcher (Flutter
    /// `SystemChrome.setApplicationSwitcherDescription`).
    fn set_application_switcher_description(&self, description: &ApplicationSwitcherDescription);
}

/// Flutter's `HapticFeedback`.
pub trait Haptics {
    /// Plays haptic feedback on the device (Flutter's `HapticFeedback.vibrate`
    /// message on `SystemChannels.platform`).
    fn feedback(&self, kind: HapticFeedbackType);
}

/// Flutter's `SystemContextMenu`: the system's own menu over a text selection.
pub trait SystemContextMenu {
    /// Shows the system context menu (Flutter's `ContextMenu.showSystemContextMenu`).
    ///
    /// `items` is `None` for the deprecated call that lets the platform pick default
    /// buttons. Built-in items are performed by the host; custom items are reported
    /// through [`crate::EmbedderClient::custom_context_menu_action`].
    fn show(&self, target_rect: Rect, items: Option<&[SystemContextMenuItem]>);

    /// Hides the system context menu (Flutter's `ContextMenu.hideSystemContextMenu`).
    fn hide(&self);
}

/// A native pop-up menu shown at the pointer, answered with the index chosen.
pub trait PopupMenus {
    /// Shows `entries` as the platform's own popup menu at the pointer and waits for it to
    /// close; answers the index of the entry chosen. Not a Flutter call: Flutter's context
    /// menus draw inside the view, and a desktop panel narrower than its menu needs the
    /// system's, which is its own window.
    ///
    /// The host runs the menu's event loop inside this call, so nothing else of the app runs
    /// meanwhile and a native callback that arrives then must post, not borrow.
    fn show(&self, entries: &[PopupMenuEntry]) -> Option<usize>;
}

/// The screens a window is placed on, for an app that places a window against something on them,
/// such as an edge or its own icon. Positions are in the coordinates of
/// [`HostWindow::frame`](crate::HostWindow::frame): logical points from the top-left corner of the
/// main screen, y growing down. Not a Flutter API: Flutter's `Display` has a size, not a place.
pub trait Screens {
    /// The screens, the main one first: the one positions start from.
    fn screens(&self) -> Vec<Screen>;

    /// Where the pointer is now, over a window of the app or not. `None` when the host cannot
    /// say.
    fn pointer_location(&self) -> Option<Offset>;
}

/// A screen, in the coordinates of a window's frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Screen {
    /// All of the screen.
    pub frame: Rect,
    /// What the system's own bars leave free: the menu bar and the Dock, or the taskbar.
    pub visible_frame: Rect,
}

/// The text services a selection toolbar offers: Flutter's LookUp, SearchWeb and Share
/// platform-channel calls.
pub trait TextServices {
    /// Looks up the given text (Flutter's `LookUp.invoke`).
    fn look_up(&self, text: &str);

    /// Searches the web for the given text (Flutter's `SearchWeb.invoke`).
    fn search_web(&self, text: &str);

    /// Shares the given text (Flutter's `Share.invoke`).
    fn share(&self, text: &str);
}

/// Flutter's mouse cursor channel.
pub trait MouseCursor {
    /// Shows a system cursor for a pointing device on `view`, the one the device is over
    /// (Flutter's `activateSystemCursor` message on `SystemChannels.mouseCursor`, which
    /// carries no view: its embedders map the device to one themselves). A host with one
    /// cursor for the whole process ignores the view.
    fn activate_system_cursor(&self, view: ViewId, device: i64, kind: SystemMouseCursorKind);
}

/// Flutter's restoration channel: state kept across relaunches.
pub trait Restoration {
    /// The restoration data the host kept for this application (the `get` message of
    /// Flutter's `SystemChannels.restoration`).
    fn get(&self) -> Option<RestorationUpdate>;

    /// Hands the host the current restoration data (the `put` message of Flutter's
    /// `SystemChannels.restoration`), which keeps it until the operating system asks for it.
    fn put(&self, data: RestorationMap);
}

/// The application asking to quit: the `System.exitApplication` message of Flutter's
/// `SystemChannels.platform`.
pub trait AppExit {
    /// Quits the application the way the system quits one, as Flutter's embedders do.
    ///
    /// For [`AppExitType::Cancelable`] the host first asks the framework through
    /// [`EmbedderClient::request_app_exit`](crate::EmbedderClient::request_app_exit), on a later
    /// turn of its loop, never inside this call, and quits only once the answer is
    /// [`AppExitResponse::Exit`]. For [`AppExitType::Required`] it quits without asking.
    ///
    /// The future answers what the framework said, or `Exit` for a required exit. A host that
    /// has already quit may never answer.
    ///
    /// `exit_code` is the process's exit status where the system takes one; Flutter's macOS
    /// and Linux embedders ignore it, and a host may too.
    fn exit_application(&self, exit_type: AppExitType, exit_code: i32) -> AppExitFuture;
}

/// An answer to a request to exit, on its way: the host's to the application's
/// [`AppExit::exit_application`], and the framework's to the host's
/// [`EmbedderClient::request_app_exit`](crate::EmbedderClient::request_app_exit). Flutter's
/// `Future<AppExitResponse>`.
pub type AppExitFuture = Pin<Box<dyn Future<Output = AppExitResponse>>>;

/// The files the system asks the app to open: documents opened from the system's file browser, or
/// dropped on the app's icon, at launch or while it runs. Flutter's macOS embedder hands them to
/// its plugins (`FlutterAppLifecycleDelegate.handleOpenURLs:`), outside the framework.
pub trait FileOpening {
    /// What happens with the files the system asks the app to open. The host calls `handler`
    /// with each request's files on the main thread, outside every framework call, so it may
    /// post to the app. Files asked for before a handler is set wait for one: a launch asks for
    /// its files before the app has started.
    fn set_open_files_handler(&self, handler: Option<OpenFilesHandler>);
}

/// A handler for the files the system asks the app to open.
pub type OpenFilesHandler = Rc<dyn Fn(Vec<PathBuf>)>;

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

    fn dispatcher(&self) -> Arc<dyn Dispatcher> {
        Arc::new(InertDispatcher)
    }

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
            fn dispatcher(&self) -> std::sync::Arc<dyn super::Dispatcher> {
                std::sync::Arc::new(super::InertDispatcher)
            }
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
