//! Flutter counterpart: `widgets/app.dart`.
//!
//! [`WidgetsApp`], the widget an application is built from: the navigator, the localizations,
//! the default shortcuts and actions, and the debug banner. `WidgetsApp.router` waits with
//! `router.dart`.

use std::any::TypeId;
use std::collections::HashMap;
use std::fmt::{self, Debug};
use std::ptr;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};

use inset_embedder::{Clip, Color, Locale, TargetPlatform};
use inset_foundation::{App, Handle, K_IS_WEB, Listenable};
use inset_painting::{AxisDirection, TextStyle};

use crate::binding::{WidgetsBinding, WidgetsBindingObserverObject, WidgetsBindingObserverRef};
use crate::framework::{
    BuildContext, GlobalKey, IntoWidget, KeyRef, State, StateData, StatefulWidget, WidgetRef,
};
use crate::widgets::actions::{
    Action, Actions, ActivateIntent, AnyAction, ButtonActivateIntent, DismissIntent,
    DoNothingAction, DoNothingAndStopPropagationIntent, DoNothingIntent, PrioritizedAction,
    PrioritizedIntents, VoidCallbackAction, VoidCallbackIntent,
};
use crate::widgets::banner::CheckedModeBanner;
use crate::widgets::basic::{Builder, WidgetBuilder};
use crate::widgets::default_text_editing_shortcuts::{DefaultTextEditingShortcuts, shortcut_map};
use crate::widgets::focus_manager::TraversalEdgeBehavior;
use crate::widgets::focus_scope::{Focus, FocusScope};
use crate::widgets::focus_traversal::{
    DirectionalFocusAction, DirectionalFocusIntent, FocusTraversalGroup, FocusTraversalPolicy,
    NextFocusAction, NextFocusIntent, PreviousFocusAction, PreviousFocusIntent,
    ReadingOrderTraversalPolicy, RequestFocusAction, RequestFocusIntent, TraversalDirection,
};
use crate::widgets::localizations::{
    Localizations, LocalizationsDelegateRef, LocalizationsResolver,
};
use crate::widgets::navigator::{
    AnyNavigatorObserver, AnyRoute, NavigationNotification, Navigator, NavigatorState,
    RouteFactory, RouteSettings,
};
use crate::widgets::notification_listener::{NotificationListener, NotificationListenerCallback};
use crate::widgets::pages::AnyPageRoute;
use crate::widgets::restoration::RootRestorationScope;
use crate::widgets::scrollable_helpers::{ScrollAction, ScrollIncrementType, ScrollIntent};
use crate::widgets::shared_app_data::SharedAppData;
use crate::widgets::shortcuts::{ShortcutMap, ShortcutRegistrar, Shortcuts};
use crate::widgets::tap_region::TapRegionSurface;
use crate::widgets::text::DefaultTextStyle;
use crate::widgets::title::Title;
use crate::widgets::transitions::{ListenableBuilder, TransitionBuilder};

/// The signature of `WidgetsApp.localeListResolutionCallback`.
///
/// A [`LocaleListResolutionCallback`] is responsible for computing the locale of the app's
/// `Localizations` object when the app starts and when user changes the list of
/// locales for the device.
///
/// The `locales` list is the device's preferred locales when the app started, or the
/// device's preferred locales the user selected after the app was started. This list
/// is in order of preference. If this list is null or empty, then Flutter has not yet
/// received the locale information from the platform. The `supported_locales` parameter
/// is just the value of `WidgetsApp.supportedLocales`.
///
/// See also:
///
///  * [`LocaleResolutionCallback`], which takes only one default locale (instead of a list)
///    and is attempted only after this callback fails or is null. [`LocaleListResolutionCallback`]
///    is recommended over [`LocaleResolutionCallback`].
pub type LocaleListResolutionCallback = Rc<dyn Fn(Option<&[Locale]>, &[Locale]) -> Option<Locale>>;

/// The signature of `WidgetsApp.localeResolutionCallback`.
///
/// It is recommended to provide a [`LocaleListResolutionCallback`] instead of a
/// [`LocaleResolutionCallback`] when possible, as [`LocaleResolutionCallback`] only
/// receives a subset of the information provided in [`LocaleListResolutionCallback`].
///
/// A [`LocaleResolutionCallback`] is responsible for computing the locale of the app's
/// `Localizations` object when the app starts and when user changes the default
/// locale for the device after [`LocaleListResolutionCallback`] fails or is not provided.
///
/// This callback is also used if the app is created with a specific locale using
/// the `WidgetsApp.new` `locale` parameter.
///
/// The `locale` is either the value of `WidgetsApp.locale`, or the device's default
/// locale when the app started, or the device locale the user selected after the app
/// was started. The default locale is the first locale in the list of preferred
/// locales. If `locale` is null, then Flutter has not yet received the locale
/// information from the platform. The `supported_locales` parameter is just the value of
/// `WidgetsApp.supportedLocales`.
///
/// See also:
///
///  * [`LocaleListResolutionCallback`], which takes a list of preferred locales (instead of one locale).
///    Resolutions by [`LocaleListResolutionCallback`] take precedence over [`LocaleResolutionCallback`].
pub type LocaleResolutionCallback = Rc<dyn Fn(Option<&Locale>, &[Locale]) -> Option<Locale>>;

/// The default locale resolution algorithm.
///
/// Custom resolution algorithms can be provided through
/// `WidgetsApp.localeListResolutionCallback` or
/// `WidgetsApp.localeResolutionCallback`.
///
/// When no custom locale resolution algorithms are provided or if both fail
/// to resolve, Flutter will default to calling this algorithm.
///
/// This algorithm prioritizes speed at the cost of slightly less appropriate
/// resolutions for edge cases.
///
/// This algorithm will resolve to the earliest preferred locale that
/// matches the most fields, prioritizing in the order of perfect match,
/// languageCode+countryCode, languageCode+scriptCode, languageCode-only.
///
/// In the case where a locale is matched by languageCode-only and is not the
/// default (first) locale, the next preferred locale with a
/// perfect match can supersede the languageCode-only match if it exists.
///
/// When a preferredLocale matches more than one supported locale, it will
/// resolve to the first matching locale listed in the supportedLocales.
///
/// When all preferred locales have been exhausted without a match, the first
/// countryCode only match will be returned.
///
/// When no match at all is found, the first (default) locale in
/// `supported_locales` will be returned.
///
/// To summarize, the main matching priority is:
///
///  1. `Locale.languageCode`, `Locale.scriptCode`, and `Locale.countryCode`
///  2. `Locale.languageCode` and `Locale.scriptCode` only
///  3. `Locale.languageCode` and `Locale.countryCode` only
///  4. `Locale.languageCode` only (with caveats, see above)
///  5. `Locale.countryCode` only when all `preferred_locales` fail to match
///  6. Returns the first element of `supported_locales` as a fallback
///
/// This algorithm does not take language distance (how similar languages are to each other)
/// into account, and will not handle edge cases such as resolving `de` to `fr` rather than `zh`
/// when `de` is not supported and `zh` is listed before `fr` (German is closer to French
/// than Chinese).
pub fn basic_locale_list_resolution(
    preferred_locales: Option<&[Locale]>,
    supported_locales: &[Locale],
) -> Locale {
    let first_supported = supported_locales
        .first()
        .expect("supportedLocales must not be empty")
        .clone();
    let Some(preferred_locales) = preferred_locales.filter(|locales| !locales.is_empty()) else {
        return first_supported;
    };
    // The hashmaps below are the 'flattened' representation of the supported locales;
    // only the first supported locale with the same key is kept.
    let mut all_supported_locales: HashMap<String, &Locale> = HashMap::new();
    let mut language_and_country_locales: HashMap<String, &Locale> = HashMap::new();
    let mut language_and_script_locales: HashMap<String, &Locale> = HashMap::new();
    let mut language_locales: HashMap<&str, &Locale> = HashMap::new();
    let mut country_locales: HashMap<Option<&str>, &Locale> = HashMap::new();
    for locale in supported_locales {
        all_supported_locales
            .entry(full_key(locale))
            .or_insert(locale);
        language_and_script_locales
            .entry(language_and_script_key(locale))
            .or_insert(locale);
        language_and_country_locales
            .entry(language_and_country_key(locale))
            .or_insert(locale);
        language_locales
            .entry(&locale.language_code)
            .or_insert(locale);
        country_locales
            .entry(locale.country_code.as_deref())
            .or_insert(locale);
    }

    // Since languageCode-only matches are possibly low quality, we don't return
    // it instantly when we find such a match. We check to see if the next
    // preferred locale in the list has a high accuracy match, and only return
    // the languageCode-only match when a higher accuracy match in the next
    // preferred locale cannot be found.
    let mut matches_language_code: Option<&Locale> = None;
    let mut matches_country_code: Option<&Locale> = None;
    // Loop over user's preferred locales
    for (locale_index, user_locale) in preferred_locales.iter().enumerate() {
        // Look for perfect match.
        if all_supported_locales.contains_key(&full_key(user_locale)) {
            return user_locale.clone();
        }
        // Look for language+script match.
        if user_locale.script_code.is_some()
            && let Some(matched) =
                language_and_script_locales.get(&language_and_script_key(user_locale))
        {
            return (*matched).clone();
        }
        // Look for language+country match.
        if user_locale.country_code.is_some()
            && let Some(matched) =
                language_and_country_locales.get(&language_and_country_key(user_locale))
        {
            return (*matched).clone();
        }
        // If there was a languageCode-only match in the previous iteration's higher
        // ranked preferred locale, we return it if the current userLocale does not
        // have a better match.
        if let Some(matched) = matches_language_code {
            return matched.clone();
        }
        // Look and store language-only match.
        if let Some(&matched) = language_locales.get(user_locale.language_code.as_str()) {
            matches_language_code = Some(matched);
            // Since first (default) locale is usually highly preferred, we will allow
            // a languageCode-only match to be instantly matched. If the next preferred
            // languageCode is the same, we defer hastily returning until the next iteration
            // since at worst it is the same and at best it has a strong match.
            let next_prefers_the_same_language = preferred_locales
                .get(locale_index + 1)
                .is_some_and(|next| next.language_code == user_locale.language_code);
            if locale_index == 0 && !next_prefers_the_same_language {
                return matched.clone();
            }
        }
        // countryCode-only match. When all else except default supported locale fails,
        // attempt to match by country only, as a user is likely to be familiar with a
        // language from their listed country.
        if matches_country_code.is_none()
            && user_locale.country_code.is_some()
            && let Some(&matched) = country_locales.get(&user_locale.country_code.as_deref())
        {
            matches_country_code = Some(matched);
        }
    }
    // When there is no languageCode-only match. Fallback to matching countryCode only. Country
    // fallback only applies on iOS. When there is no countryCode-only match, we return first
    // supported locale.
    matches_language_code
        .or(matches_country_code)
        .cloned()
        .unwrap_or(first_supported)
}

fn full_key(locale: &Locale) -> String {
    format!(
        "{}_{:?}_{:?}",
        locale.language_code, locale.script_code, locale.country_code
    )
}

fn language_and_script_key(locale: &Locale) -> String {
    format!("{}_{:?}", locale.language_code, locale.script_code)
}

fn language_and_country_key(locale: &Locale) -> String {
    format!("{}_{:?}", locale.language_code, locale.country_code)
}

/// The signature of [`WidgetsApp::on_generate_title`].
///
/// Used to generate a value for the app's [`Title::title`], which the device uses
/// to identify the app for the user. The `context` includes the [`WidgetsApp`]'s
/// `Localizations` widget so that this method can be used to produce a
/// localized title.
pub type GenerateAppTitle = Rc<dyn Fn(&mut App, BuildContext) -> String>;

/// The signature of [`WidgetsApp::page_route_builder`].
///
/// Creates a `PageRoute` using the given [`RouteSettings`] and [`WidgetBuilder`].
pub type PageRouteFactory = Rc<dyn Fn(&mut App, &RouteSettings, WidgetBuilder) -> AnyPageRoute>;

/// The signature of [`WidgetsApp::on_generate_initial_routes`].
///
/// Creates a series of one or more initial routes.
pub type InitialRouteListFactory = Rc<dyn Fn(&mut App, &str) -> Vec<AnyRoute>>;

/// Dart's `WidgetsApp.showPerformanceOverlayOverride`.
static SHOW_PERFORMANCE_OVERLAY_OVERRIDE: AtomicBool = AtomicBool::new(false);

/// Dart's `WidgetsApp.debugAllowBannerOverride`.
static DEBUG_ALLOW_BANNER_OVERRIDE: AtomicBool = AtomicBool::new(true);

/// A convenience widget that wraps a number of widgets that are commonly
/// required for an application.
///
/// One of the primary roles that [`WidgetsApp`] provides is binding the system
/// back button to popping the [`Navigator`] or quitting the application.
///
/// It is used by both `MaterialApp` and `CupertinoApp` to implement base
/// functionality for an app.
///
/// Find references to many of the widgets that [`WidgetsApp`] wraps in the "See
/// also" section.
///
/// See also:
///
///  * [`CheckedModeBanner`], which displays a `Banner` saying "DEBUG" when
///    running in debug mode.
///  * [`DefaultTextStyle`], the text style to apply to descendant `Text` widgets
///    without an explicit style.
///  * `MediaQuery`, which establishes a subtree in which media queries resolve
///    to a `MediaQueryData`.
///  * [`Localizations`], which defines the [`Locale`] for its `child`.
///  * [`Title`], a widget that describes this app in the operating system.
///  * [`Navigator`], a widget that manages a set of child widgets with a stack
///    discipline.
///  * `Overlay`, a widget that manages a `Stack` of entries that can be managed
///    independently.
pub struct WidgetsApp {
    /// See [`Widget::key`](crate::Widget::key).
    pub key: Option<KeyRef>,

    /// A key to use when building the [`Navigator`].
    ///
    /// If a [`navigator_key`](Self::navigator_key) is specified, the [`Navigator`] can be
    /// directly manipulated without first obtaining it from a [`BuildContext`] via
    /// [`Navigator::of`]: from the [`navigator_key`](Self::navigator_key), use the
    /// [`GlobalKey::current_state`] getter.
    ///
    /// If this is changed, a new [`Navigator`] will be created, losing all the
    /// application state in the process; in that case, the
    /// [`navigator_observers`](Self::navigator_observers) must also be changed, since the
    /// previous observers will be attached to the previous navigator.
    ///
    /// The [`Navigator`] is only built if [`on_generate_route`](Self::on_generate_route) is not
    /// `None`; if it is `None`, [`navigator_key`](Self::navigator_key) must also be `None`.
    pub navigator_key: Option<GlobalKey>,

    /// The route generator callback used when the app is navigated to a
    /// named route.
    ///
    /// If this returns `None` when building the routes to handle the specified
    /// [`initial_route`](Self::initial_route), then all the routes are discarded and
    /// [`Navigator::DEFAULT_ROUTE_NAME`] is used instead (`/`). See
    /// [`initial_route`](Self::initial_route).
    ///
    /// During normal app operation, the [`on_generate_route`](Self::on_generate_route) callback
    /// will only be applied to route names pushed by the application, and so should never
    /// return `None`.
    ///
    /// This is used if [`routes`](Self::routes) does not contain the requested route.
    ///
    /// The [`Navigator`] is only built if routes are provided (either via
    /// [`home`](Self::home), [`routes`](Self::routes),
    /// [`on_generate_route`](Self::on_generate_route), or
    /// [`on_unknown_route`](Self::on_unknown_route)); if they are not,
    /// [`builder`](Self::builder) must not be `None`.
    ///
    /// If this property is not set, either the [`routes`](Self::routes) or
    /// [`home`](Self::home) properties must be set, and the
    /// [`page_route_builder`](Self::page_route_builder) must also be set so that the
    /// default handler will know what routes and `PageRoute`s to build.
    pub on_generate_route: Option<RouteFactory>,

    /// The routes generator callback used for generating initial routes if
    /// [`initial_route`](Self::initial_route) is provided.
    ///
    /// If this property is not set, the underlying
    /// [`Navigator::on_generate_initial_routes`] will default to
    /// [`Navigator::default_generate_initial_routes`].
    pub on_generate_initial_routes: Option<InitialRouteListFactory>,

    /// The `PageRoute` generator callback used when the app is navigated to a
    /// named route.
    ///
    /// A `PageRoute` represents the page in a [`Navigator`], so that it can
    /// correctly animate between pages, and to represent the "return value" of
    /// a route (e.g. which button a user selected in a modal dialog).
    ///
    /// This callback can be used, for example, to specify that a `MaterialPageRoute`
    /// or a `CupertinoPageRoute` should be used for building page transitions.
    pub page_route_builder: Option<PageRouteFactory>,

    /// The widget for the default route of the app ([`Navigator::DEFAULT_ROUTE_NAME`],
    /// which is `/`).
    ///
    /// This is the route that is displayed first when the application is started
    /// normally, unless [`initial_route`](Self::initial_route) is specified. It's also the
    /// route that's displayed if the [`initial_route`](Self::initial_route) can't be
    /// displayed.
    ///
    /// If [`home`](Self::home) is specified, then [`routes`](Self::routes) must not include
    /// an entry for `/`, as [`home`](Self::home) takes its place.
    ///
    /// The [`Navigator`] is only built if routes are provided (either via
    /// [`home`](Self::home), [`routes`](Self::routes),
    /// [`on_generate_route`](Self::on_generate_route), or
    /// [`on_unknown_route`](Self::on_unknown_route)); if they are not,
    /// [`builder`](Self::builder) must not be `None`.
    ///
    /// The difference between using [`home`](Self::home) and using
    /// [`builder`](Self::builder) is that the [`home`](Self::home) subtree is inserted into
    /// the application below a [`Navigator`] (and thus below an `Overlay`, which
    /// [`Navigator`] uses). With [`home`](Self::home), therefore, dialog boxes will work
    /// automatically, the [`routes`](Self::routes) table will be used, and APIs such as
    /// [`Navigator::push`] and [`Navigator::pop`] will work as expected. In contrast, the
    /// widget returned from [`builder`](Self::builder) is inserted _above_ the app's
    /// [`Navigator`] (if any).
    ///
    /// If this property is set, the [`page_route_builder`](Self::page_route_builder) property
    /// must also be set so that the default route handler will know what kind of `PageRoute`s
    /// to build.
    pub home: Option<WidgetRef>,

    /// The application's top-level routing table.
    ///
    /// When a named route is pushed with [`Navigator::push_named`], the route name is
    /// looked up in this map. If the name is present, the associated
    /// [`WidgetBuilder`] is used to construct a `PageRoute` specified by
    /// [`page_route_builder`](Self::page_route_builder) to perform an appropriate transition,
    /// including `Hero` animations, to the new route.
    ///
    /// If the app only has one page, then you can specify it using [`home`](Self::home)
    /// instead.
    ///
    /// If [`home`](Self::home) is specified, then it implies an entry in this table for the
    /// [`Navigator::DEFAULT_ROUTE_NAME`] route (`/`), and it is an error to
    /// redundantly provide such a route in the [`routes`](Self::routes) table.
    ///
    /// If a route is requested that is not specified in this table (or by
    /// [`home`](Self::home)), then the [`on_generate_route`](Self::on_generate_route) callback
    /// is called to build the page instead.
    ///
    /// The [`Navigator`] is only built if routes are provided (either via
    /// [`home`](Self::home), [`routes`](Self::routes),
    /// [`on_generate_route`](Self::on_generate_route), or
    /// [`on_unknown_route`](Self::on_unknown_route)); if they are not,
    /// [`builder`](Self::builder) must not be `None`.
    ///
    /// If the routes map is not empty, the [`page_route_builder`](Self::page_route_builder)
    /// property must be set so that the default route handler will know what kind of
    /// `PageRoute`s to build.
    pub routes: HashMap<String, WidgetBuilder>,

    /// Called when [`on_generate_route`](Self::on_generate_route) fails to generate a route,
    /// except for the [`initial_route`](Self::initial_route).
    ///
    /// This callback is typically used for error handling. For example, this
    /// callback might always generate a "not found" page that describes the route
    /// that wasn't found.
    ///
    /// Unknown routes can arise either from errors in the app or from external
    /// requests to push routes, such as from Android intents.
    ///
    /// The [`Navigator`] is only built if routes are provided (either via
    /// [`home`](Self::home), [`routes`](Self::routes),
    /// [`on_generate_route`](Self::on_generate_route), or
    /// [`on_unknown_route`](Self::on_unknown_route)); if they are not,
    /// [`builder`](Self::builder) must not be `None`.
    pub on_unknown_route: Option<RouteFactory>,

    /// The callback to use when receiving a [`NavigationNotification`].
    ///
    /// By default this updates the engine with the navigation status and stops
    /// bubbling the notification.
    ///
    /// See also:
    ///
    ///  * [`NotificationListener::on_notification`], which uses this callback.
    pub on_navigation_notification: Option<NotificationListenerCallback<NavigationNotification>>,

    /// The name of the first route to show, if a [`Navigator`] is built.
    ///
    /// Defaults to [`Platform::default_route_name`](inset_embedder::Platform::default_route_name),
    /// which may be overridden by the code that launched the application.
    ///
    /// If the route name starts with a slash, then it is treated as a "deep link",
    /// and before this route is pushed, the routes leading to this one are pushed
    /// also. For example, if the route was `/a/b/c`, then the app would start
    /// with the four routes `/`, `/a`, `/a/b`, and `/a/b/c` loaded, in that order.
    /// Even if the route was just `/a`, the app would start with `/` and `/a`
    /// loaded. You can use the
    /// [`on_generate_initial_routes`](Self::on_generate_initial_routes) property to override
    /// this behavior.
    ///
    /// Intermediate routes aren't required to exist. In the example above, `/a`
    /// and `/a/b` could be skipped if they have no matching route. But `/a/b/c` is
    /// required to have a route, else [`initial_route`](Self::initial_route) is ignored and
    /// [`Navigator::DEFAULT_ROUTE_NAME`] is used instead (`/`). This can happen if the
    /// app is started with an intent that specifies a non-existent route.
    ///
    /// The [`Navigator`] is only built if routes are provided (either via
    /// [`home`](Self::home), [`routes`](Self::routes),
    /// [`on_generate_route`](Self::on_generate_route), or
    /// [`on_unknown_route`](Self::on_unknown_route)); if they are not,
    /// [`initial_route`](Self::initial_route) must be `None` and
    /// [`builder`](Self::builder) must not be `None`.
    ///
    /// Changing the [`initial_route`](Self::initial_route) will have no effect, as it only
    /// controls the _initial_ route. To change the route while the application is running,
    /// use the [`Navigator`] APIs.
    pub initial_route: Option<String>,

    /// The list of observers for the [`Navigator`] created for this app.
    ///
    /// This list must be replaced by a list of newly-created observers if the
    /// [`navigator_key`](Self::navigator_key) is changed.
    ///
    /// The [`Navigator`] is only built if routes are provided (either via
    /// [`home`](Self::home), [`routes`](Self::routes),
    /// [`on_generate_route`](Self::on_generate_route), or
    /// [`on_unknown_route`](Self::on_unknown_route)); if they are not,
    /// [`navigator_observers`](Self::navigator_observers) must be the empty list and
    /// [`builder`](Self::builder) must not be `None`.
    pub navigator_observers: Vec<AnyNavigatorObserver>,

    /// A builder for inserting widgets above the [`Navigator`] but below the
    /// other widgets created by the [`WidgetsApp`] widget, or for replacing the
    /// [`Navigator`] entirely.
    ///
    /// For example, from the [`BuildContext`] passed to this method, the
    /// `Directionality`, [`Localizations`], [`DefaultTextStyle`], `MediaQuery`, etc,
    /// are all available. They can also be overridden in a way that impacts all
    /// the routes in the [`Navigator`].
    ///
    /// This is rarely useful, but can be used in applications that wish to
    /// override those defaults, e.g. to force the application into right-to-left
    /// mode despite being in English, or to override the `MediaQuery` metrics
    /// (e.g. to leave a gap for advertisements shown by a plugin from OEM code).
    ///
    /// For specifically overriding the [`title`](Self::title) with a value based on the
    /// [`Localizations`], consider [`on_generate_title`](Self::on_generate_title) instead.
    ///
    /// The [`builder`](Self::builder) callback is passed two arguments, the [`BuildContext`]
    /// and a [`Navigator`] widget (as `child`).
    ///
    /// If no routes are provided using [`home`](Self::home), [`routes`](Self::routes),
    /// [`on_generate_route`](Self::on_generate_route), or
    /// [`on_unknown_route`](Self::on_unknown_route), the `child` will be `None`, and it is
    /// the responsibility of the [`builder`](Self::builder) to provide the application's
    /// routing machinery.
    ///
    /// If routes _are_ provided using one or more of those properties, then `child` is not
    /// `None`, and the returned value should include the `child` in the widget subtree; if it
    /// does not, then the application will have no [`Navigator`] and the routing related
    /// properties are ignored.
    ///
    /// If [`builder`](Self::builder) is `None`, it is as if a builder was specified that
    /// returned the `child` directly. If it is `None`, routes must be provided using one of
    /// the other properties listed above.
    ///
    /// Unless a [`Navigator`] is provided, either implicitly from
    /// [`builder`](Self::builder) being `None`, or by a [`builder`](Self::builder) including
    /// its `child` argument, or by a [`builder`](Self::builder) explicitly providing a
    /// [`Navigator`] of its own, widgets and APIs such as `Hero`, [`Navigator::push`] and
    /// [`Navigator::pop`], will not function.
    pub builder: Option<TransitionBuilder>,

    /// A one-line description used by the device to identify the app for the user.
    ///
    /// On Android the titles appear above the task manager's app snapshots which are
    /// displayed when the user presses the "recent apps" button. On iOS this
    /// value cannot be used. `CFBundleDisplayName` from the app's `Info.plist` is
    /// referred to instead whenever present, `CFBundleName` otherwise.
    /// On the web it is used as the page title, which shows up in the browser's list of open
    /// tabs.
    ///
    /// To provide a localized title instead, use
    /// [`on_generate_title`](Self::on_generate_title).
    pub title: Option<String>,

    /// If non-`None` this callback function is called to produce the app's
    /// title string, otherwise [`title`](Self::title) is used.
    ///
    /// The [`on_generate_title`](Self::on_generate_title) `context` parameter includes the
    /// [`WidgetsApp`]'s [`Localizations`] widget so that this callback can be used to produce
    /// a localized title.
    ///
    /// The [`on_generate_title`](Self::on_generate_title) callback is called each time the
    /// [`WidgetsApp`] rebuilds.
    pub on_generate_title: Option<GenerateAppTitle>,

    /// The default text style for `Text` in the application.
    pub text_style: Option<TextStyle>,

    /// The primary color to use for the application in the operating system
    /// interface.
    ///
    /// For example, on Android this is the color used for the application in the
    /// application switcher.
    pub color: Color,

    /// The initial locale for this app's [`Localizations`] widget is based
    /// on this value.
    ///
    /// If the `locale` is `None` then the system's locale value is used.
    ///
    /// The value of [`Localizations::locale_of`] will equal this locale if
    /// it matches one of the [`supported_locales`](Self::supported_locales). Otherwise it
    /// will be the first element of [`supported_locales`](Self::supported_locales).
    ///
    /// See also:
    ///
    ///  * [`locale_resolution_callback`](Self::locale_resolution_callback), which can override
    ///    the default [`supported_locales`](Self::supported_locales) matching algorithm.
    ///  * [`localizations_delegates`](Self::localizations_delegates), which collectively define
    ///    all of the localized resources used by this app.
    pub locale: Option<Locale>,

    /// The delegates for this app's [`Localizations`] widget.
    ///
    /// The delegates collectively define all of the localized resources
    /// for this application's [`Localizations`] widget.
    pub localizations_delegates: Option<Vec<LocalizationsDelegateRef>>,

    /// This callback is responsible for choosing the app's locale
    /// when the app is started, and when the user changes the
    /// device's locale.
    ///
    /// When a [`locale_list_resolution_callback`](Self::locale_list_resolution_callback) is
    /// provided, the resolver will first attempt to resolve the locale with it. If the
    /// callback or result is `None`, it will fall back to trying the
    /// [`locale_resolution_callback`](Self::locale_resolution_callback). If both are left
    /// `None` or fail to resolve, the basic fallback algorithm will be used.
    ///
    /// The priority of each available fallback is:
    ///
    ///  1. [`locale_list_resolution_callback`](Self::locale_list_resolution_callback) is
    ///     attempted.
    ///  2. [`locale_resolution_callback`](Self::locale_resolution_callback) is attempted.
    ///  3. [`basic_locale_list_resolution`], the basic resolution algorithm, is attempted last.
    ///
    /// This callback considers the entire list of preferred locales.
    ///
    /// This algorithm should be able to handle a `None` or empty list of preferred locales,
    /// which indicates the platform has not reported locale information yet.
    pub locale_list_resolution_callback: Option<LocaleListResolutionCallback>,

    /// See [`locale_list_resolution_callback`](Self::locale_list_resolution_callback).
    ///
    /// This callback considers only the default locale, which is the first locale
    /// in the preferred locales list. It is preferred to set
    /// [`locale_list_resolution_callback`](Self::locale_list_resolution_callback) over
    /// [`locale_resolution_callback`](Self::locale_resolution_callback) as it provides the
    /// full preferred locales list.
    pub locale_resolution_callback: Option<LocaleResolutionCallback>,

    /// The list of locales that this app has been localized for.
    ///
    /// By default only the American English locale is supported. Apps should
    /// configure this list to match the locales they support.
    ///
    /// The order of the list matters. The default locale resolution algorithm,
    /// [`basic_locale_list_resolution`], attempts to match by the following priority:
    ///
    ///  1. `Locale::language_code`, `Locale::script_code`, and `Locale::country_code`
    ///  2. `Locale::language_code` and `Locale::script_code` only
    ///  3. `Locale::language_code` and `Locale::country_code` only
    ///  4. `Locale::language_code` only
    ///  5. `Locale::country_code` only when all preferred locales fail to match
    ///  6. Returns the first element of [`supported_locales`](Self::supported_locales) as a
    ///     fallback
    ///
    /// When more than one supported locale matches one of these criteria, only
    /// the first matching locale is returned.
    ///
    /// When supporting languages with more than one script, it is recommended
    /// to specify the `Locale::script_code` explicitly. Locales may also be defined without
    /// `Locale::country_code` to specify a generic fallback for a particular script.
    pub supported_locales: Vec<Locale>,

    /// Turns on a performance overlay.
    ///
    /// See also:
    ///
    ///  * <https://flutter.dev/to/performance-overlay>
    pub show_performance_overlay: bool,

    /// Turns on an overlay that shows the accessibility information
    /// reported by the framework.
    pub show_semantics_debugger: bool,

    /// Turns on an overlay that enables inspecting the widget tree.
    ///
    /// The inspector is only available in debug mode as it depends on
    /// `RenderObject.debugDescribeChildren` which should not be called outside of
    /// debug mode.
    pub debug_show_widget_inspector: bool,

    /// Turns on a little "DEBUG" banner in debug mode to indicate
    /// that the app is in debug mode. This is on by default (in
    /// debug mode), to turn it off, set the setter argument to
    /// false. In release mode this has no effect.
    ///
    /// To get this banner in your application if you're not using
    /// [`WidgetsApp`], include a [`CheckedModeBanner`] widget in your app.
    ///
    /// This banner is intended to deter people from complaining that your
    /// app is slow when it's in debug mode. In debug mode, a large number of
    /// expensive diagnostics are enabled to aid in development, and so performance in debug
    /// mode is not representative of what will happen in release mode.
    pub debug_show_checked_mode_banner: bool,

    /// The default map of keyboard shortcuts to intents for the application.
    ///
    /// By default, this is set to [`WidgetsApp::default_shortcuts`].
    ///
    /// Passing this will not replace [`DefaultTextEditingShortcuts`]. These can be
    /// overridden by using a [`Shortcuts`] widget lower in the widget tree.
    ///
    /// See also:
    ///
    ///  * `SingleActivator`, which defines shortcut key combination of a single
    ///    key and modifiers, such as "Delete" or "Control+C".
    ///  * The [`Shortcuts`] widget, which defines a keyboard mapping.
    ///  * The [`Actions`] widget, which defines the mapping from intent to action.
    ///  * The `Intent` and `Action` traits, which allow definition of new actions.
    pub shortcuts: Option<ShortcutMap>,

    /// The default map of intent keys to actions for the application.
    ///
    /// By default, this is the output of [`WidgetsApp::default_actions`]. Specifying
    /// [`actions`](Self::actions) for an app overrides the default, so if you wish to modify
    /// the default [`actions`](Self::actions), you can call [`WidgetsApp::default_actions`]
    /// and modify the resulting map, passing it as the [`actions`](Self::actions) for this
    /// app. You may also add to the bindings, or override specific bindings for a widget
    /// subtree, by adding your own [`Actions`] widget.
    ///
    /// See also:
    ///
    ///  * The [`shortcuts`](Self::shortcuts) parameter, which defines the default set of
    ///    shortcuts for the application.
    ///  * The [`Shortcuts`] widget, which defines a keyboard mapping.
    ///  * The [`Actions`] widget, which defines the mapping from intent to action.
    pub actions: Option<HashMap<TypeId, AnyAction>>,

    /// The identifier to use for state restoration of this app.
    ///
    /// Providing a restoration ID inserts a [`RootRestorationScope`] into the
    /// widget hierarchy, which enables state restoration for descendant widgets.
    ///
    /// Providing a restoration ID also enables the [`Navigator`] built
    /// by the [`WidgetsApp`] to restore its state (i.e. to restore the history
    /// stack of active `Route`s). See the documentation on [`Navigator`] for more
    /// details around state restoration of `Route`s.
    ///
    /// See also:
    ///
    ///  * `RestorationManager`, which explains how state restoration works in
    ///    Flutter.
    pub restoration_scope_id: Option<String>,

    /// Dart's deprecated `useInheritedMediaQuery`. This setting is ignored.
    ///
    /// The widget never introduces its own `MediaQuery`; the `View` widget takes
    /// care of that.
    pub use_inherited_media_query: bool,
}

impl WidgetsApp {
    /// Creates a widget that wraps a number of widgets that are commonly
    /// required for an application.
    ///
    /// Most callers will want to use the [`home`](Self::home) or [`routes`](Self::routes)
    /// setters, or both. The [`home`](Self::home) setter is a convenience for the following
    /// [`routes`](Self::routes) map:
    ///
    /// ```text
    /// {"/": |app, context| my_widget}
    /// ```
    ///
    /// It is possible to specify both [`home`](Self::home) and [`routes`](Self::routes), but
    /// only if [`routes`](Self::routes) does _not_ contain an entry for `"/"`. Conversely, if
    /// [`home`](Self::home) is omitted, [`routes`](Self::routes) _must_ contain an entry for
    /// `"/"`.
    ///
    /// If [`home`](Self::home) or [`routes`](Self::routes) are set, the routing implementation
    /// needs to know how to appropriately build `PageRoute`s. This can be achieved by
    /// supplying the [`page_route_builder`](Self::page_route_builder) setter.
    ///
    /// The [`builder`](Self::builder) setter is designed to provide the ability to wrap the
    /// visible content of the app in some other widget. It is recommended that you use
    /// [`home`](Self::home) rather than [`builder`](Self::builder) if you intend to only
    /// display a single route in your app.
    ///
    /// [`WidgetsApp`] is also able to provide a custom implementation of routing via the
    /// [`on_generate_route`](Self::on_generate_route) and
    /// [`on_unknown_route`](Self::on_unknown_route) setters. These correspond to
    /// [`Navigator::on_generate_route`] and [`Navigator::on_unknown_route`]. If
    /// [`home`](Self::home), [`routes`](Self::routes), and [`builder`](Self::builder) are
    /// unset, or if they fail to create a requested route,
    /// [`on_generate_route`](Self::on_generate_route) will be invoked. If that fails,
    /// [`on_unknown_route`](Self::on_unknown_route) will be invoked.
    ///
    /// The [`supported_locales`](Self::supported_locales) list must have one or more elements.
    /// By default it is `[Locale::new("en").country_code("US")]`.
    ///
    /// The configuration is checked when the widget is inflated: Dart's constructor asserts
    /// run there, because a fluent setter cannot see the fields that are still to come.
    pub fn new(color: Color) -> WidgetsApp {
        WidgetsApp {
            key: None,
            navigator_key: None,
            on_generate_route: None,
            on_generate_initial_routes: None,
            page_route_builder: None,
            home: None,
            routes: HashMap::new(),
            on_unknown_route: None,
            on_navigation_notification: None,
            initial_route: None,
            navigator_observers: Vec::new(),
            builder: None,
            title: None,
            on_generate_title: None,
            text_style: None,
            color,
            locale: None,
            localizations_delegates: None,
            locale_list_resolution_callback: None,
            locale_resolution_callback: None,
            supported_locales: vec![Locale::new("en").country_code("US")],
            show_performance_overlay: false,
            show_semantics_debugger: false,
            debug_show_widget_inspector: false,
            debug_show_checked_mode_banner: true,
            shortcuts: None,
            actions: None,
            restoration_scope_id: None,
            use_inherited_media_query: false,
        }
    }

    /// Dart `WidgetsApp(key:)`.
    pub fn key(mut self, key: KeyRef) -> WidgetsApp {
        self.key = Some(key);
        self
    }

    /// Dart `WidgetsApp(navigatorKey:)`.
    pub fn navigator_key(mut self, navigator_key: GlobalKey) -> WidgetsApp {
        self.navigator_key = Some(navigator_key);
        self
    }

    /// Dart `WidgetsApp(onGenerateRoute:)`.
    pub fn on_generate_route(
        mut self,
        on_generate_route: impl Fn(&mut App, &RouteSettings) -> Option<AnyRoute> + 'static,
    ) -> WidgetsApp {
        self.on_generate_route = Some(Rc::new(on_generate_route));
        self
    }

    /// Dart `WidgetsApp(onGenerateInitialRoutes:)`.
    pub fn on_generate_initial_routes(
        mut self,
        on_generate_initial_routes: impl Fn(&mut App, &str) -> Vec<AnyRoute> + 'static,
    ) -> WidgetsApp {
        self.on_generate_initial_routes = Some(Rc::new(on_generate_initial_routes));
        self
    }

    /// Dart `WidgetsApp(pageRouteBuilder:)`.
    pub fn page_route_builder(
        mut self,
        page_route_builder: impl Fn(&mut App, &RouteSettings, WidgetBuilder) -> AnyPageRoute + 'static,
    ) -> WidgetsApp {
        self.page_route_builder = Some(Rc::new(page_route_builder));
        self
    }

    /// Dart `WidgetsApp(home:)`.
    pub fn home<K>(mut self, home: impl IntoWidget<K>) -> WidgetsApp {
        self.home = Some(home.into_widget());
        self
    }

    /// Dart `WidgetsApp(routes:)`.
    pub fn routes(
        mut self,
        routes: impl IntoIterator<Item = (String, WidgetBuilder)>,
    ) -> WidgetsApp {
        self.routes = routes.into_iter().collect();
        self
    }

    /// Dart `WidgetsApp(onUnknownRoute:)`.
    pub fn on_unknown_route(
        mut self,
        on_unknown_route: impl Fn(&mut App, &RouteSettings) -> Option<AnyRoute> + 'static,
    ) -> WidgetsApp {
        self.on_unknown_route = Some(Rc::new(on_unknown_route));
        self
    }

    /// Dart `WidgetsApp(onNavigationNotification:)`.
    pub fn on_navigation_notification(
        mut self,
        on_navigation_notification: impl Fn(&mut App, &NavigationNotification) -> bool + 'static,
    ) -> WidgetsApp {
        self.on_navigation_notification = Some(Rc::new(on_navigation_notification));
        self
    }

    /// Dart `WidgetsApp(initialRoute:)`.
    pub fn initial_route(mut self, initial_route: impl Into<String>) -> WidgetsApp {
        self.initial_route = Some(initial_route.into());
        self
    }

    /// Dart `WidgetsApp(navigatorObservers:)`.
    pub fn navigator_observers(
        mut self,
        navigator_observers: impl IntoIterator<Item = AnyNavigatorObserver>,
    ) -> WidgetsApp {
        self.navigator_observers = navigator_observers.into_iter().collect();
        self
    }

    /// Dart `WidgetsApp(builder:)`.
    pub fn builder(
        mut self,
        builder: impl Fn(&mut App, BuildContext, Option<&WidgetRef>) -> WidgetRef + 'static,
    ) -> WidgetsApp {
        self.builder = Some(Rc::new(builder));
        self
    }

    /// Dart `WidgetsApp(title:)`.
    pub fn title(mut self, title: impl Into<String>) -> WidgetsApp {
        self.title = Some(title.into());
        self
    }

    /// Dart `WidgetsApp(onGenerateTitle:)`.
    pub fn on_generate_title(
        mut self,
        on_generate_title: impl Fn(&mut App, BuildContext) -> String + 'static,
    ) -> WidgetsApp {
        self.on_generate_title = Some(Rc::new(on_generate_title));
        self
    }

    /// Dart `WidgetsApp(textStyle:)`.
    pub fn text_style(mut self, text_style: TextStyle) -> WidgetsApp {
        self.text_style = Some(text_style);
        self
    }

    /// Dart `WidgetsApp(locale:)`.
    pub fn locale(mut self, locale: Locale) -> WidgetsApp {
        self.locale = Some(locale);
        self
    }

    /// Dart `WidgetsApp(localizationsDelegates:)`.
    pub fn localizations_delegates(
        mut self,
        localizations_delegates: impl IntoIterator<Item = LocalizationsDelegateRef>,
    ) -> WidgetsApp {
        self.localizations_delegates = Some(localizations_delegates.into_iter().collect());
        self
    }

    /// Dart `WidgetsApp(localeListResolutionCallback:)`.
    pub fn locale_list_resolution_callback(
        mut self,
        locale_list_resolution_callback: impl Fn(Option<&[Locale]>, &[Locale]) -> Option<Locale>
        + 'static,
    ) -> WidgetsApp {
        self.locale_list_resolution_callback = Some(Rc::new(locale_list_resolution_callback));
        self
    }

    /// Dart `WidgetsApp(localeResolutionCallback:)`.
    pub fn locale_resolution_callback(
        mut self,
        locale_resolution_callback: impl Fn(Option<&Locale>, &[Locale]) -> Option<Locale> + 'static,
    ) -> WidgetsApp {
        self.locale_resolution_callback = Some(Rc::new(locale_resolution_callback));
        self
    }

    /// Dart `WidgetsApp(supportedLocales:)`.
    pub fn supported_locales(
        mut self,
        supported_locales: impl IntoIterator<Item = Locale>,
    ) -> WidgetsApp {
        self.supported_locales = supported_locales.into_iter().collect();
        debug_assert!(!self.supported_locales.is_empty());
        self
    }

    /// Dart `WidgetsApp(showPerformanceOverlay:)`.
    pub fn show_performance_overlay(mut self, show_performance_overlay: bool) -> WidgetsApp {
        self.show_performance_overlay = show_performance_overlay;
        self
    }

    /// Dart `WidgetsApp(showSemanticsDebugger:)`.
    pub fn show_semantics_debugger(mut self, show_semantics_debugger: bool) -> WidgetsApp {
        self.show_semantics_debugger = show_semantics_debugger;
        self
    }

    /// Dart `WidgetsApp(debugShowWidgetInspector:)`.
    pub fn debug_show_widget_inspector(mut self, debug_show_widget_inspector: bool) -> WidgetsApp {
        self.debug_show_widget_inspector = debug_show_widget_inspector;
        self
    }

    /// Dart `WidgetsApp(debugShowCheckedModeBanner:)`.
    pub fn debug_show_checked_mode_banner(
        mut self,
        debug_show_checked_mode_banner: bool,
    ) -> WidgetsApp {
        self.debug_show_checked_mode_banner = debug_show_checked_mode_banner;
        self
    }

    /// Dart `WidgetsApp(shortcuts:)`.
    pub fn shortcuts(mut self, shortcuts: ShortcutMap) -> WidgetsApp {
        self.shortcuts = Some(shortcuts);
        self
    }

    /// Dart `WidgetsApp(actions:)`.
    pub fn actions(mut self, actions: HashMap<TypeId, AnyAction>) -> WidgetsApp {
        self.actions = Some(actions);
        self
    }

    /// Dart `WidgetsApp(restorationScopeId:)`.
    pub fn restoration_scope_id(mut self, restoration_scope_id: impl Into<String>) -> WidgetsApp {
        self.restoration_scope_id = Some(restoration_scope_id.into());
        self
    }

    /// Dart `WidgetsApp(useInheritedMediaQuery:)`, which is ignored.
    pub fn use_inherited_media_query(mut self, use_inherited_media_query: bool) -> WidgetsApp {
        self.use_inherited_media_query = use_inherited_media_query;
        self
    }

    /// If true, forces the performance overlay to be visible in all instances.
    ///
    /// Used by the `showPerformanceOverlay` VM service extension.
    pub fn show_performance_overlay_override() -> bool {
        SHOW_PERFORMANCE_OVERLAY_OVERRIDE.load(Ordering::Relaxed)
    }

    /// Sets [`show_performance_overlay_override`](Self::show_performance_overlay_override).
    pub fn set_show_performance_overlay_override(value: bool) {
        SHOW_PERFORMANCE_OVERLAY_OVERRIDE.store(value, Ordering::Relaxed);
    }

    /// If false, prevents the debug banner from being visible.
    ///
    /// Used by the `debugAllowBanner` VM service extension.
    ///
    /// This is how `flutter run` turns off the banner when you take a screen shot
    /// with "s".
    pub fn debug_allow_banner_override() -> bool {
        DEBUG_ALLOW_BANNER_OVERRIDE.load(Ordering::Relaxed)
    }

    /// Sets [`debug_allow_banner_override`](Self::debug_allow_banner_override).
    pub fn set_debug_allow_banner_override(value: bool) {
        DEBUG_ALLOW_BANNER_OVERRIDE.store(value, Ordering::Relaxed);
    }

    /// Generates the default shortcut key bindings based on the platform.
    ///
    /// Used by [`WidgetsApp`] to assign a default value to
    /// [`shortcuts`](Self::shortcuts).
    pub fn default_shortcuts(app: &App) -> ShortcutMap {
        if K_IS_WEB {
            return default_web_shortcuts();
        }
        match app.platform().target_platform() {
            TargetPlatform::Android
            | TargetPlatform::Fuchsia
            | TargetPlatform::Linux
            | TargetPlatform::Windows => default_shortcuts_table(),
            TargetPlatform::IOS | TargetPlatform::MacOS => default_apple_os_shortcuts(),
        }
    }

    /// The default value of [`actions`](Self::actions).
    pub fn default_actions(app: &mut App) -> HashMap<TypeId, AnyAction> {
        let do_nothing = DoNothingAction::new(app);
        let do_nothing_and_stop_propagation = DoNothingAction::new(app);
        do_nothing_and_stop_propagation.set_consumes_key(app, false);
        let request_focus = RequestFocusAction::new(app);
        let next_focus = NextFocusAction::new(app);
        let previous_focus = PreviousFocusAction::new(app);
        let directional_focus = DirectionalFocusAction::new(app);
        let scroll = ScrollAction::new(app);
        let prioritized = PrioritizedAction::new(app);
        let void_callback = VoidCallbackAction::new(app);
        HashMap::from([
            (
                TypeId::of::<DoNothingIntent>(),
                Action::as_action(do_nothing),
            ),
            (
                TypeId::of::<DoNothingAndStopPropagationIntent>(),
                Action::as_action(do_nothing_and_stop_propagation),
            ),
            (
                TypeId::of::<RequestFocusIntent>(),
                Action::as_action(request_focus),
            ),
            (
                TypeId::of::<NextFocusIntent>(),
                Action::as_action(next_focus),
            ),
            (
                TypeId::of::<PreviousFocusIntent>(),
                Action::as_action(previous_focus),
            ),
            (
                TypeId::of::<DirectionalFocusIntent>(),
                Action::as_action(directional_focus),
            ),
            (TypeId::of::<ScrollIntent>(), Action::as_action(scroll)),
            (
                TypeId::of::<PrioritizedIntents>(),
                Action::as_action(prioritized),
            ),
            (
                TypeId::of::<VoidCallbackIntent>(),
                Action::as_action(void_callback),
            ),
        ])
    }

    /// Dart's constructor asserts, which run when the widget is inflated.
    fn debug_check_configuration(&self) {
        debug_assert!(
            self.home.is_none() || self.on_generate_initial_routes.is_none(),
            "If onGenerateInitialRoutes is specified, the home argument will be redundant."
        );
        debug_assert!(
            self.home.is_none() || !self.routes.contains_key(Navigator::DEFAULT_ROUTE_NAME),
            "If the home property is specified, the routes table cannot include an entry for \
             \"/\", since it would be redundant."
        );
        debug_assert!(
            self.builder.is_some()
                || self.home.is_some()
                || self.routes.contains_key(Navigator::DEFAULT_ROUTE_NAME)
                || self.on_generate_route.is_some()
                || self.on_unknown_route.is_some(),
            "Either the home property must be specified, or the routes table must include an \
             entry for \"/\", or there must be on onGenerateRoute callback specified, or there \
             must be an onUnknownRoute callback specified, or the builder property must be \
             specified, because otherwise there is nothing to fall back on if the app is \
             started with an intent that specifies an unknown route."
        );
        debug_assert!(
            (self.home.is_some()
                || !self.routes.is_empty()
                || self.on_generate_route.is_some()
                || self.on_unknown_route.is_some())
                || (self.builder.is_some()
                    && self.navigator_key.is_none()
                    && self.initial_route.is_none()
                    && self.navigator_observers.is_empty()),
            "If no route is provided using home, routes, onGenerateRoute, or onUnknownRoute, a \
             non-null callback for the builder property must be provided, and the other \
             navigator-related properties, navigatorKey, initialRoute, and navigatorObservers, \
             must have their initial values (null, null, and the empty list, respectively)."
        );
        debug_assert!(
            self.builder.is_some()
                || self.on_generate_route.is_some()
                || self.page_route_builder.is_some(),
            "If neither builder nor onGenerateRoute are provided, the pageRouteBuilder must be \
             specified so that the default handler will know what kind of PageRoute transition \
             to build."
        );
        debug_assert!(!self.supported_locales.is_empty());
    }
}

impl Debug for WidgetsApp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WidgetsApp")
            .field("title", &self.title)
            .field("color", &self.color)
            .field("locale", &self.locale)
            .field("restorationScopeId", &self.restoration_scope_id)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for WidgetsApp {
    type State = WidgetsAppState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> WidgetsAppState {
        self.debug_check_configuration();
        WidgetsAppState {
            state: StateData::new(),
            navigator: None,
            localizations_resolver: None,
            observer: None,
        }
    }
}

/// Dart's `_defaultShortcuts`, whose name the public getter takes.
fn default_shortcuts_table() -> ShortcutMap {
    shortcut_map![
        // Activation
        ENTER => ActivateIntent::new(),
        NUMPAD_ENTER => ActivateIntent::new(),
        SPACE => ActivateIntent::new(),
        GAME_BUTTON_A => ActivateIntent::new(),
        SELECT => ActivateIntent::new(),

        // Dismissal
        ESCAPE => DismissIntent::new(),

        // Keyboard traversal.
        TAB => NextFocusIntent::new(),
        TAB.shift(true) => PreviousFocusIntent::new(),
        ARROW_LEFT => DirectionalFocusIntent::new(TraversalDirection::Left),
        ARROW_RIGHT => DirectionalFocusIntent::new(TraversalDirection::Right),
        ARROW_DOWN => DirectionalFocusIntent::new(TraversalDirection::Down),
        ARROW_UP => DirectionalFocusIntent::new(TraversalDirection::Up),

        // Scrolling
        ARROW_UP.control(true) => ScrollIntent::new(AxisDirection::Up),
        ARROW_DOWN.control(true) => ScrollIntent::new(AxisDirection::Down),
        ARROW_LEFT.control(true) => ScrollIntent::new(AxisDirection::Left),
        ARROW_RIGHT.control(true) => ScrollIntent::new(AxisDirection::Right),
        PAGE_UP => ScrollIntent::new(AxisDirection::Up).r#type(ScrollIncrementType::Page),
        PAGE_DOWN => ScrollIntent::new(AxisDirection::Down).r#type(ScrollIncrementType::Page),
    ]
}

/// Default shortcuts for the web platform.
fn default_web_shortcuts() -> ShortcutMap {
    shortcut_map![
        // Activation
        SPACE => PrioritizedIntents::new(vec![
            Rc::new(ActivateIntent::new()),
            Rc::new(ScrollIntent::new(AxisDirection::Down).r#type(ScrollIncrementType::Page)),
        ]),
        // On the web, enter activates buttons, but not other controls.
        ENTER => ButtonActivateIntent::new(),
        NUMPAD_ENTER => ButtonActivateIntent::new(),

        // Dismissal
        ESCAPE => DismissIntent::new(),

        // Keyboard traversal.
        TAB => NextFocusIntent::new(),
        TAB.shift(true) => PreviousFocusIntent::new(),

        // Scrolling
        ARROW_UP => ScrollIntent::new(AxisDirection::Up),
        ARROW_DOWN => ScrollIntent::new(AxisDirection::Down),
        ARROW_LEFT => ScrollIntent::new(AxisDirection::Left),
        ARROW_RIGHT => ScrollIntent::new(AxisDirection::Right),
        PAGE_UP => ScrollIntent::new(AxisDirection::Up).r#type(ScrollIncrementType::Page),
        PAGE_DOWN => ScrollIntent::new(AxisDirection::Down).r#type(ScrollIncrementType::Page),
    ]
}

/// Default shortcuts for the macOS and iOS platforms.
fn default_apple_os_shortcuts() -> ShortcutMap {
    shortcut_map![
        // Activation
        ENTER => ActivateIntent::new(),
        NUMPAD_ENTER => ActivateIntent::new(),
        SPACE => ActivateIntent::new(),

        // Dismissal
        ESCAPE => DismissIntent::new(),

        // Keyboard traversal
        TAB => NextFocusIntent::new(),
        TAB.shift(true) => PreviousFocusIntent::new(),
        ARROW_LEFT => DirectionalFocusIntent::new(TraversalDirection::Left),
        ARROW_RIGHT => DirectionalFocusIntent::new(TraversalDirection::Right),
        ARROW_DOWN => DirectionalFocusIntent::new(TraversalDirection::Down),
        ARROW_UP => DirectionalFocusIntent::new(TraversalDirection::Up),

        // Scrolling
        ARROW_UP.meta(true) => ScrollIntent::new(AxisDirection::Up),
        ARROW_DOWN.meta(true) => ScrollIntent::new(AxisDirection::Down),
        ARROW_LEFT.meta(true) => ScrollIntent::new(AxisDirection::Left),
        ARROW_RIGHT.meta(true) => ScrollIntent::new(AxisDirection::Right),
        PAGE_UP => ScrollIntent::new(AxisDirection::Up).r#type(ScrollIncrementType::Page),
        PAGE_DOWN => ScrollIntent::new(AxisDirection::Down).r#type(ScrollIncrementType::Page),
    ]
}

/// Dart's `_WidgetsAppState`.
pub struct WidgetsAppState {
    state: StateData<WidgetsApp>,
    navigator: Option<GlobalKey>,
    localizations_resolver: Option<Handle<LocalizationsResolver>>,
    observer: Option<WidgetsBindingObserverRef>,
}

impl WidgetsAppState {
    /// The route the navigator starts with.
    ///
    /// If the platform's default route name isn't `/`, it was set intentionally by the host,
    /// and it overrides whatever is in [`WidgetsApp::initial_route`].
    fn initial_route_name(self: Handle<Self>, app: &App) -> String {
        let default_route_name = app.platform().default_route_name();
        if default_route_name != Navigator::DEFAULT_ROUTE_NAME {
            return default_route_name;
        }
        self.widget(app)
            .initial_route
            .clone()
            .unwrap_or(default_route_name)
    }

    /// The default value for [`WidgetsApp::on_navigation_notification`].
    ///
    /// Does nothing and stops bubbling while the app is not ready — which, until the binding
    /// raises the lifecycle callbacks, it never is; see PORTING.md.
    fn default_on_navigation_notification(
        self: Handle<Self>,
        app: &mut App,
        notification: &NavigationNotification,
    ) -> bool {
        let _ = (app, notification);
        // Avoid updating the engine when the app isn't ready.
        true
    }

    fn clear_navigator_resource(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).navigator = None;
    }

    fn update_routing(self: Handle<Self>, app: &mut App, old_widget: Option<&WidgetsApp>) {
        if self.uses_navigator(app) {
            let navigator_key = self.widget(app).navigator_key.clone();
            let key_changed = old_widget.is_some_and(|old_widget| {
                navigator_key.as_ref().map(GlobalKey::identity)
                    != old_widget.navigator_key.as_ref().map(GlobalKey::identity)
            });
            if app.get(self).navigator.is_none() || key_changed {
                app.get_mut(self).navigator = Some(navigator_key.unwrap_or_else(GlobalKey::new));
            }
            debug_assert!(app.get(self).navigator.is_some());
        } else {
            debug_assert!(self.widget(app).builder.is_some());
            self.clear_navigator_resource(app);
        }
        // If we use a navigator, we have a navigator key.
        debug_assert!(self.uses_navigator(app) == app.get(self).navigator.is_some());
    }

    fn uses_navigator(self: Handle<Self>, app: &App) -> bool {
        let widget = self.widget(app);
        widget.home.is_some()
            || !widget.routes.is_empty()
            || widget.on_generate_route.is_some()
            || widget.on_unknown_route.is_some()
    }

    fn on_generate_route(
        self: Handle<Self>,
        app: &mut App,
        settings: &RouteSettings,
    ) -> Option<AnyRoute> {
        let widget = self.widget(app);
        let name = settings.name.clone();
        let home = widget.home.clone();
        let page_content_builder: Option<WidgetBuilder> = match (name.as_deref(), home) {
            (Some(Navigator::DEFAULT_ROUTE_NAME), Some(home)) => {
                Some(Rc::new(move |_app: &mut App, _context: BuildContext| {
                    home.clone()
                }))
            }
            _ => name
                .as_ref()
                .and_then(|name| widget.routes.get(name))
                .cloned(),
        };

        if let Some(page_content_builder) = page_content_builder {
            let page_route_builder = self.widget(app).page_route_builder.clone().expect(
                "The default onGenerateRoute handler for WidgetsApp must have a \
                 pageRouteBuilder set if the home or routes properties are set.",
            );
            let route = page_route_builder(app, settings, page_content_builder);
            return Some(route.as_route());
        }
        let on_generate_route = self.widget(app).on_generate_route.clone();
        if let Some(on_generate_route) = on_generate_route {
            return on_generate_route(app, settings);
        }
        None
    }

    fn on_unknown_route(
        self: Handle<Self>,
        app: &mut App,
        settings: &RouteSettings,
    ) -> Option<AnyRoute> {
        let on_unknown_route = self
            .widget(app)
            .on_unknown_route
            .clone()
            .unwrap_or_else(|| {
                panic!(
                    "Could not find a generator for route {settings:?} in the WidgetsApp.\n\
                 Make sure your root app widget has provided a way to generate\n\
                 this route.\n\
                 Generators for routes are searched for in the following order:\n\
                 1. For the \"/\" route, the \"home\" property, if non-null, is used.\n\
                 2. Otherwise, the \"routes\" table is used, if it has an entry for the route.\n\
                 3. Otherwise, onGenerateRoute is called. It should return a non-null value \
                 for any valid route not handled by \"home\" and \"routes\".\n\
                 4. Finally if all else fails onUnknownRoute is called.\n\
                 Unfortunately, onUnknownRoute was not set."
                )
            });
        let result = on_unknown_route(app, settings);
        Some(result.expect(
            "The onUnknownRoute callback returned null.\nWhen the WidgetsApp requested a route \
             from its onUnknownRoute callback, the callback returned null. Such callbacks must \
             never return null.",
        ))
    }

    /// The resolver that keeps the app's locale and delegates up to date.
    fn localizations_resolver(self: Handle<Self>, app: &App) -> Handle<LocalizationsResolver> {
        app.get(self)
            .localizations_resolver
            .expect("init_state creates the resolver")
    }

    fn should_update_localizations(self: Handle<Self>, app: &App, old_widget: &WidgetsApp) -> bool {
        let widget = self.widget(app);
        widget.locale != old_widget.locale
            || !same_callback(
                &widget.locale_list_resolution_callback,
                &old_widget.locale_list_resolution_callback,
            )
            || !same_callback(
                &widget.locale_resolution_callback,
                &old_widget.locale_resolution_callback,
            )
            || widget.supported_locales != old_widget.supported_locales
            || !same_delegates(
                &widget.localizations_delegates,
                &old_widget.localizations_delegates,
            )
    }

    fn update_localizations(self: Handle<Self>, app: &mut App, old_widget: &WidgetsApp) {
        if self.should_update_localizations(app, old_widget) {
            let widget = self.widget(app);
            let locale = widget.locale.clone();
            let locale_list_resolution_callback = widget.locale_list_resolution_callback.clone();
            let locale_resolution_callback = widget.locale_resolution_callback.clone();
            let localizations_delegates = widget.localizations_delegates.clone();
            let supported_locales = widget.supported_locales.clone();
            self.localizations_resolver(app).update(
                app,
                locale,
                locale_list_resolution_callback,
                locale_resolution_callback,
                localizations_delegates,
                supported_locales,
            );
        }
    }

    /// Dart's `_usesNavigator` branch of `build`: the navigator and the focus scope over it.
    fn build_navigator(self: Handle<Self>, app: &mut App) -> WidgetRef {
        let navigator = app.get(self).navigator.clone().expect("a navigator key");
        let widget = self.widget(app);
        let observers = widget.navigator_observers.clone();
        let on_generate_initial_routes = widget.on_generate_initial_routes.clone();
        let initial_route = self.initial_route_name(app);
        let mut navigator_widget = Navigator::new()
            .clip_behavior(Clip::None)
            .restoration_scope_id("nav")
            .key(Rc::new(navigator))
            .initial_route(initial_route)
            .on_generate_route(move |app, settings| self.on_generate_route(app, settings))
            .on_unknown_route(move |app, settings| self.on_unknown_route(app, settings))
            .observers(observers)
            .route_traversal_edge_behavior(if K_IS_WEB {
                TraversalEdgeBehavior::LeaveFlutterView
            } else {
                TraversalEdgeBehavior::ParentScope
            })
            .reports_route_update_to_engine(true);
        navigator_widget = match on_generate_initial_routes {
            None => navigator_widget
                .on_generate_initial_routes(Navigator::default_generate_initial_routes),
            Some(on_generate_initial_routes) => navigator_widget.on_generate_initial_routes(
                move |app, _navigator, initial_route_name| {
                    on_generate_initial_routes(app, initial_route_name)
                },
            ),
        };
        FocusScope::new(navigator_widget)
            .debug_label("Navigator Scope")
            .autofocus(true)
            .into_widget()
    }
}

/// Dart's `==` on two optional callbacks: the same `Rc` allocation, or both absent.
fn same_callback<T: ?Sized>(one: &Option<Rc<T>>, other: &Option<Rc<T>>) -> bool {
    match (one, other) {
        (None, None) => true,
        (Some(one), Some(other)) => Rc::ptr_eq(one, other),
        _ => false,
    }
}

/// Dart's `==` on two optional delegate lists: pairwise identity, as Dart compares the lists.
fn same_delegates(
    one: &Option<Vec<LocalizationsDelegateRef>>,
    other: &Option<Vec<LocalizationsDelegateRef>>,
) -> bool {
    match (one, other) {
        (None, None) => true,
        (Some(one), Some(other)) => {
            one.len() == other.len()
                && one
                    .iter()
                    .zip(other)
                    .all(|(one, other)| ptr::addr_eq(&raw const **one, &raw const **other))
        }
        _ => false,
    }
}

impl WidgetsBindingObserverObject for WidgetsAppState {
    /// Dart's `WidgetsApp.didPopRoute`: the app's own navigator goes back, if it has one
    /// and it has something to go back to.
    fn did_pop_route(self: Handle<Self>, app: &mut App) -> bool {
        let Some(key) = app.get(self).navigator.clone() else {
            return false;
        };
        let Some(navigator) = key.current_state::<NavigatorState>(app) else {
            return false;
        };
        navigator.maybe_pop(app, None)
    }
}

impl State for WidgetsAppState {
    type Widget = WidgetsApp;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let widget = self.widget(app);
        let locale = widget.locale.clone();
        let locale_list_resolution_callback = widget.locale_list_resolution_callback.clone();
        let locale_resolution_callback = widget.locale_resolution_callback.clone();
        let localizations_delegates = widget.localizations_delegates.clone();
        let supported_locales = widget.supported_locales.clone();
        let resolver = LocalizationsResolver::new(
            app,
            locale,
            locale_list_resolution_callback,
            locale_resolution_callback,
            localizations_delegates,
            supported_locales,
        );
        app.get_mut(self).localizations_resolver = Some(resolver);
        self.update_routing(app, None);
        let observer: WidgetsBindingObserverRef = Rc::new(self);
        WidgetsBinding::instance(app).add_observer(app, Rc::clone(&observer));
        app.get_mut(self).observer = Some(observer);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &WidgetsApp) {
        self.update_routing(app, Some(old_widget));
        self.update_localizations(app, old_widget);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(observer) = app.get_mut(self).observer.take() {
            WidgetsBinding::instance(app).remove_observer(app, &observer);
        }
        self.localizations_resolver(app).dispose(app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let routing: Option<WidgetRef> =
            self.uses_navigator(app).then(|| self.build_navigator(app));

        let mut result = match self.widget(app).builder.clone() {
            Some(builder) => {
                Builder::new(move |app, context| builder(app, context, routing.as_ref()))
                    .into_widget()
            }
            None => routing.expect("either a builder or a navigator"),
        };

        if let Some(text_style) = self.widget(app).text_style.clone() {
            result = DefaultTextStyle::new(text_style, result).into_widget();
        }

        // The performance overlay, the semantics debugger, and the widget inspector wait; see
        // PORTING.md.

        if cfg!(debug_assertions)
            && self.widget(app).debug_show_checked_mode_banner
            && WidgetsApp::debug_allow_banner_override()
        {
            result = CheckedModeBanner::new(result).into_widget();
        }

        result = Focus::new(result).can_request_focus(false).into_widget();

        let color = self.widget(app).color;
        let title: Option<WidgetRef> = match self.widget(app).on_generate_title.clone() {
            Some(on_generate_title) => {
                let child = result.clone();
                // This Builder exists to provide a context below the Localizations widget.
                // The on_generate_title callback can refer to Localizations via its context
                // parameter.
                Some(
                    Builder::new(move |app, context| {
                        let title = on_generate_title(app, context);
                        Title::new(
                            color.with_values(Some(1.0), None, None, None, None),
                            child.clone(),
                        )
                        .title(title)
                        .into_widget()
                    })
                    .into_widget(),
                )
            }
            // Updating the title element in the DOM is problematic in embedded and multiview
            // modes as the title should be managed by host apps.
            None if self.widget(app).title.is_none() && K_IS_WEB => None,
            None => {
                let title = self.widget(app).title.clone().unwrap_or_default();
                Some(
                    Title::new(
                        color.with_values(Some(1.0), None, None, None, None),
                        result.clone(),
                    )
                    .title(title)
                    .into_widget(),
                )
            }
        };

        let resolver = self.localizations_resolver(app);
        let localized = title.unwrap_or(result);
        let localizations = ListenableBuilder::new(
            Rc::new(resolver) as Rc<dyn Listenable>,
            move |app, _context, _child| {
                let locale = resolver.locale(app);
                let delegates = resolver.localizations_delegates(app);
                Localizations::new(locale, delegates)
                    .is_application_level(true)
                    .child(localized.clone())
                    .into_widget()
            },
        );

        let actions = match self.widget(app).actions.clone() {
            Some(actions) => actions,
            None => {
                let mut actions = WidgetsApp::default_actions(app);
                let scroll = ScrollAction::new(app);
                actions.insert(
                    TypeId::of::<ScrollIntent>(),
                    AnyAction::overridable(app, Action::as_action(scroll), context),
                );
                actions
            }
        };
        let policy = ReadingOrderTraversalPolicy::new(app).as_policy();
        let shortcuts = match self.widget(app).shortcuts.clone() {
            Some(shortcuts) => shortcuts,
            None => WidgetsApp::default_shortcuts(app),
        };
        let on_navigation_notification = self.widget(app).on_navigation_notification.clone();
        let restoration_scope_id = self.widget(app).restoration_scope_id.clone();

        RootRestorationScope::new(
            restoration_scope_id,
            SharedAppData::new(
                NotificationListener::<NavigationNotification>::new(
                    Shortcuts::new(
                        shortcuts,
                        DefaultTextEditingShortcuts::new(Actions::new(
                            actions,
                            FocusTraversalGroup::new(TapRegionSurface::new(
                                ShortcutRegistrar::new(localizations),
                            ))
                            .policy(policy),
                        )),
                    )
                    .debug_label("<Default WidgetsApp Shortcuts>"),
                )
                .on_notification(move |app, notification| match &on_navigation_notification {
                    Some(on_navigation_notification) => {
                        on_navigation_notification(app, notification)
                    }
                    None => self.default_on_navigation_notification(app, notification),
                })
                .into_widget(),
            ),
        )
        .into_widget()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use inset_foundation::AppCell;

    fn locale(language: &str, country: Option<&str>) -> Locale {
        let locale = Locale::new(language);
        match country {
            Some(country) => locale.country_code(country),
            None => locale,
        }
    }

    #[test]
    fn a_perfect_match_wins_over_everything_later_in_the_list() {
        let supported = [
            locale("en", Some("US")),
            locale("fr", Some("FR")),
            locale("zh", Some("CN")),
        ];
        let preferred = [locale("fr", Some("FR")), locale("en", Some("US"))];
        assert_eq!(
            basic_locale_list_resolution(Some(&preferred), &supported),
            locale("fr", Some("FR"))
        );
    }

    #[test]
    fn a_language_and_script_match_beats_a_language_and_country_match() {
        let supported = [
            locale("zh", Some("CN")),
            Locale::new("zh").script_code("Hant"),
            Locale::new("zh").script_code("Hans"),
        ];
        let preferred = [Locale::new("zh").script_code("Hant").country_code("CN")];
        assert_eq!(
            basic_locale_list_resolution(Some(&preferred), &supported),
            Locale::new("zh").script_code("Hant")
        );
    }

    #[test]
    fn a_language_only_match_defers_to_a_better_match_for_the_same_language() {
        let supported = [locale("en", Some("US")), locale("en", Some("GB"))];
        // The first preferred locale only matches by language, but the second is a perfect
        // match for the same language, so it supersedes.
        let preferred = [locale("en", Some("AU")), locale("en", Some("GB"))];
        assert_eq!(
            basic_locale_list_resolution(Some(&preferred), &supported),
            locale("en", Some("GB"))
        );
        // A different language next: the language-only match stands.
        let preferred = [locale("en", Some("AU")), locale("fr", Some("FR"))];
        assert_eq!(
            basic_locale_list_resolution(Some(&preferred), &supported),
            locale("en", Some("US"))
        );
    }

    #[test]
    fn a_country_only_match_is_the_last_resort_before_the_first_supported_locale() {
        let supported = [locale("en", Some("US")), locale("fr", Some("CA"))];
        let preferred = [locale("de", Some("CA"))];
        assert_eq!(
            basic_locale_list_resolution(Some(&preferred), &supported),
            locale("fr", Some("CA"))
        );
        let preferred = [locale("de", Some("DE"))];
        assert_eq!(
            basic_locale_list_resolution(Some(&preferred), &supported),
            locale("en", Some("US"))
        );
        assert_eq!(
            basic_locale_list_resolution(None, &supported),
            locale("en", Some("US"))
        );
        assert_eq!(
            basic_locale_list_resolution(Some(&[]), &supported),
            locale("en", Some("US"))
        );
    }

    // ---- WidgetsApp ----

    use std::cell::Cell;
    use std::time::Duration;

    use inset_embedder::TextDirection;
    use inset_painting::PaintingBinding;
    use inset_scheduler::SchedulerBinding;
    use inset_services::LogicalKeyboardKey;
    use inset_test::{TestPlatform, TestView};

    use crate::framework::{AnyElement, StatelessWidget, downcast_widget};
    use crate::widgets::actions::IntentRef;
    use crate::widgets::basic::SizedBox;
    use crate::widgets::focus_manager::tests::{app_with_view, mount};
    use crate::widgets::focus_manager::{AnyFocusNode, FocusManager, FocusNodeLeaf, primary_focus};
    use crate::widgets::focus_scope::Focus;
    use crate::widgets::localizations::WidgetsLocalizations;
    use crate::widgets::navigator::RouteSettingsRef;
    use crate::widgets::pages::{PageRoute, PageRouteBuilder};
    use crate::widgets::routes::RoutePageBuilder;

    /// The `color` every test app is given.
    const COLOR: Color = Color::new(0xFF2196F3);

    /// A leaf that is easy to find in the element tree.
    #[derive(Debug)]
    struct Marker;

    impl StatelessWidget for Marker {
        fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
            SizedBox::new().width(10.0).height(10.0).into_widget()
        }
    }

    /// An [`App`] on a host with no views, for the tables that only read the target platform.
    fn app_of(platform: TargetPlatform) -> Rc<AppCell> {
        AppCell::with_platform(Rc::new(TestPlatform::new().on(platform)))
    }

    /// A platform with one 800x600 view whose host answers `route` as its default, the way a
    /// deep link reaches an app.
    fn app_opening_at(route: &str) -> Rc<AppCell> {
        let platform = TestPlatform::new()
            .on(TargetPlatform::MacOS)
            .with_view(Rc::new(TestView::new(800.0, 600.0)))
            .with_default_route(route);
        AppCell::with_platform(Rc::new(platform))
    }

    /// What the shell does at start-up: the app-wide fonts, here the OS fonts, which the
    /// debug banner needs to paint its text.
    fn install_fonts(app: &mut App) {
        let binding = PaintingBinding::instance(app);
        if !binding.has_fonts(app) {
            binding.install_fonts(app, |fonts| {
                fonts.add_source(valo_system_fonts::SystemFonts::load());
            });
        }
    }

    /// Runs frames until the route transitions have settled.
    fn settle(app: &mut App) {
        let mut at = Duration::ZERO;
        for _ in 0..20 {
            at += Duration::from_millis(20);
            SchedulerBinding::handle_begin_frame(app, Some(at));
            app.drain_microtasks();
            SchedulerBinding::handle_draw_frame(app);
            app.drain_microtasks();
        }
    }

    /// The `pageRouteBuilder` a test app hands to [`WidgetsApp`].
    fn page_route(app: &mut App, settings: &RouteSettings, builder: WidgetBuilder) -> AnyPageRoute {
        let page_builder: RoutePageBuilder =
            Rc::new(move |app, context, _animation, _secondary| builder(app, context));
        let route = PageRouteBuilder::new(app, page_builder)
            .settings(app, RouteSettingsRef::Settings(settings.clone()));
        PageRoute::as_page_route(route)
    }

    fn widgets_app() -> WidgetsApp {
        WidgetsApp::new(COLOR).page_route_builder(page_route)
    }

    fn root_element(app: &mut App) -> AnyElement {
        WidgetsBinding::instance(app)
            .root_element(app)
            .expect("a mounted app")
    }

    /// Every element of the mounted tree, parents before children.
    fn descendants(app: &App, root: AnyElement) -> Vec<AnyElement> {
        let mut all = vec![root];
        let mut visited = 0;
        while visited < all.len() {
            let element = all[visited];
            element.visit_children(app, &mut |child| all.push(child));
            visited += 1;
        }
        all
    }

    fn find_widget<W: 'static>(app: &App, root: AnyElement) -> Option<&W> {
        for element in descendants(app, root) {
            if let Some(widget) = downcast_widget::<W>(&**element.widget(app)) {
                return Some(widget);
            }
        }
        None
    }

    fn has_widget<W: 'static>(app: &App, root: AnyElement) -> bool {
        find_widget::<W>(app, root).is_some()
    }

    /// The intents the shortcuts bind to `key`, in the table's order.
    fn intents_for(shortcuts: &ShortcutMap, key: LogicalKeyboardKey) -> Vec<IntentRef> {
        shortcuts
            .iter()
            .filter(|(activator, _)| {
                activator
                    .triggers()
                    .is_some_and(|triggers| triggers.contains(&key))
            })
            .map(|(_, intent)| Rc::clone(intent))
            .collect()
    }

    #[test]
    fn home_is_shown_by_the_first_route_of_the_navigator_the_app_builds() {
        let cell = app_with_view();
        let mut app = cell.borrow_mut();
        install_fonts(&mut app);
        drop(app);
        mount(&cell, widgets_app().home(Marker).into_widget());
        let mut app = cell.borrow_mut();
        settle(&mut app);
        let root = root_element(&mut app);
        assert!(has_widget::<Navigator>(&app, root), "the app builds one");
        assert!(
            has_widget::<Marker>(&app, root),
            "the home route shows home"
        );
    }

    #[test]
    fn a_routes_table_entry_for_the_default_route_is_the_first_route() {
        let cell = app_with_view();
        let mut app = cell.borrow_mut();
        install_fonts(&mut app);
        let builder: WidgetBuilder = Rc::new(|_app, _context| Marker.into_widget());
        drop(app);
        mount(
            &cell,
            widgets_app()
                .routes([(String::from("/"), builder)])
                .into_widget(),
        );
        let mut app = cell.borrow_mut();
        settle(&mut app);
        let root = root_element(&mut app);
        assert!(has_widget::<Marker>(&app, root));
    }

    #[test]
    fn a_builder_replaces_the_navigator_and_is_handed_no_child() {
        let cell = app_with_view();
        let mut app = cell.borrow_mut();
        install_fonts(&mut app);
        let had_child = Rc::new(Cell::new(true));
        let sink = Rc::clone(&had_child);
        drop(app);
        mount(
            &cell,
            WidgetsApp::new(COLOR)
                .builder(move |_app, _context, child| {
                    sink.set(child.is_some());
                    Marker.into_widget()
                })
                .into_widget(),
        );
        let mut app = cell.borrow_mut();
        let root = root_element(&mut app);
        assert!(!had_child.get(), "no routes, so the builder gets no child");
        assert!(has_widget::<Marker>(&app, root));
        assert!(!has_widget::<Navigator>(&app, root));
    }

    #[test]
    fn a_builder_wraps_the_navigator_when_the_app_has_routes() {
        let cell = app_with_view();
        let mut app = cell.borrow_mut();
        install_fonts(&mut app);
        let had_child = Rc::new(Cell::new(false));
        let sink = Rc::clone(&had_child);
        drop(app);
        mount(
            &cell,
            widgets_app()
                .home(SizedBox::shrink())
                .builder(move |_app, _context, child| {
                    sink.set(child.is_some());
                    child.expect("the navigator").clone()
                })
                .into_widget(),
        );
        let mut app = cell.borrow_mut();
        settle(&mut app);
        let root = root_element(&mut app);
        assert!(had_child.get());
        assert!(has_widget::<Navigator>(&app, root));
    }

    /// Android's back button and back gesture both arrive here. The app goes back while it
    /// has somewhere to go, and says it did not once it is at the first route, which is how
    /// the host knows to close the activity instead.
    #[test]
    fn a_back_press_pops_the_app_navigator_until_it_is_at_the_first_route() {
        let cell = app_with_view();
        let mut app = cell.borrow_mut();
        install_fonts(&mut app);
        let captured: Rc<Cell<Option<BuildContext>>> = Rc::default();
        let sink = Rc::clone(&captured);
        drop(app);
        mount(
            &cell,
            widgets_app()
                .home(Builder::new(move |_app, context| {
                    sink.set(Some(context));
                    Marker.into_widget()
                }))
                .into_widget(),
        );
        let mut app = cell.borrow_mut();
        settle(&mut app);
        let binding = WidgetsBinding::instance(&mut app);
        assert!(
            !binding.handle_pop_route(&mut app),
            "at the first route there is nowhere to go back to"
        );

        let context = captured.get().expect("the home built");
        let settings = RouteSettings::new().name("second");
        let route = page_route(
            &mut app,
            &settings,
            Rc::new(|_app, _context| SizedBox::shrink().into_widget()),
        );
        Navigator::of(&mut app, context, false).push(&mut app, route.as_route());
        settle(&mut app);

        assert!(
            binding.handle_pop_route(&mut app),
            "the pushed route is popped"
        );
        settle(&mut app);
        assert!(
            !binding.handle_pop_route(&mut app),
            "back at the first route it answers that it did not go back"
        );
    }

    #[test]
    fn the_locale_argument_overrides_the_resolver_and_the_widgets_delegate_is_supplied() {
        let cell = app_with_view();
        let mut app = cell.borrow_mut();
        install_fonts(&mut app);
        let captured: Rc<Cell<Option<BuildContext>>> = Rc::default();
        let sink = Rc::clone(&captured);
        drop(app);
        mount(
            &cell,
            WidgetsApp::new(COLOR)
                .locale(Locale::new("fr"))
                .supported_locales([Locale::new("en").country_code("US"), Locale::new("fr")])
                .builder(move |_app, context, _child| {
                    sink.set(Some(context));
                    Marker.into_widget()
                })
                .into_widget(),
        );
        let mut app = cell.borrow_mut();
        let context = captured.get().expect("the builder ran");
        assert_eq!(
            Localizations::locale_of(&mut app, context),
            Locale::new("fr"),
            "the locale argument wins over the platform's locales"
        );
        let localizations = Localizations::of::<dyn WidgetsLocalizations>(&mut app, context)
            .expect("DefaultWidgetsLocalizations");
        assert_eq!(localizations.text_direction(), TextDirection::Ltr);
    }

    #[test]
    fn the_default_shortcuts_bind_tab_to_focus_traversal_on_every_platform() {
        for platform in [
            TargetPlatform::Android,
            TargetPlatform::Fuchsia,
            TargetPlatform::Linux,
            TargetPlatform::Windows,
            TargetPlatform::IOS,
            TargetPlatform::MacOS,
        ] {
            let cell = app_of(platform);
            let app = cell.borrow();
            let shortcuts = WidgetsApp::default_shortcuts(&app);
            let tab = intents_for(&shortcuts, LogicalKeyboardKey::TAB);
            assert_eq!(tab.len(), 2, "{platform:?} binds tab and shift-tab");
            assert!(
                tab[0].as_any().is::<NextFocusIntent>(),
                "{platform:?} binds tab to NextFocusIntent"
            );
            assert!(
                tab[1].as_any().is::<PreviousFocusIntent>(),
                "{platform:?} binds shift-tab to PreviousFocusIntent"
            );
        }
    }

    #[test]
    fn the_apple_default_shortcuts_scroll_with_the_meta_key() {
        let cell = app_of(TargetPlatform::MacOS);
        let app = cell.borrow();
        let shortcuts = WidgetsApp::default_shortcuts(&app);
        let arrow_up = intents_for(&shortcuts, LogicalKeyboardKey::ARROW_UP);
        assert!(arrow_up[0].as_any().is::<DirectionalFocusIntent>());
        assert!(arrow_up[1].as_any().is::<ScrollIntent>());
        let (activator, _) = shortcuts
            .iter()
            .find(|(_, intent)| intent.as_any().is::<ScrollIntent>())
            .expect("a scroll shortcut");
        assert!(
            activator.debug_describe_keys().contains("Meta"),
            "the apple table scrolls with meta, not control"
        );
    }

    #[test]
    fn the_default_actions_request_focus_for_a_request_focus_intent() {
        let cell = app_with_view();
        let mut app = cell.borrow_mut();
        install_fonts(&mut app);
        drop(app);
        mount(
            &cell,
            Focus::new(SizedBox::shrink())
                .debug_label("target")
                .into_widget(),
        );
        let mut app = cell.borrow_mut();
        let root_scope = FocusManager::instance(&mut app).root_scope(&app);
        let node: AnyFocusNode = FocusNodeLeaf::as_node(root_scope)
            .descendants(&mut app)
            .into_iter()
            .find(|node| node.debug_label(&app).as_deref() == Some("target"))
            .expect("the focus node");
        let actions = WidgetsApp::default_actions(&mut app);
        let action = actions[&TypeId::of::<RequestFocusIntent>()];
        action.invoke(&mut app, &RequestFocusIntent::new(node), None);
        app.drain_microtasks();
        assert_eq!(primary_focus(&mut app), Some(node));
    }

    #[test]
    fn the_debug_banner_is_built_unless_it_is_turned_off() {
        let cell = app_with_view();
        let mut app = cell.borrow_mut();
        install_fonts(&mut app);
        drop(app);
        mount(&cell, widgets_app().home(Marker).into_widget());
        let mut app = cell.borrow_mut();
        settle(&mut app);
        let root = root_element(&mut app);
        assert_eq!(
            has_widget::<CheckedModeBanner>(&app, root),
            cfg!(debug_assertions),
            "the banner is a debug-mode widget"
        );

        let cell = app_with_view();
        let mut app = cell.borrow_mut();
        install_fonts(&mut app);
        drop(app);
        mount(
            &cell,
            widgets_app()
                .home(Marker)
                .debug_show_checked_mode_banner(false)
                .into_widget(),
        );
        let mut app = cell.borrow_mut();
        settle(&mut app);
        let root = root_element(&mut app);
        assert!(!has_widget::<CheckedModeBanner>(&app, root));
    }

    #[test]
    fn a_restoration_scope_id_names_the_root_restoration_scope() {
        let cell = app_with_view();
        let mut app = cell.borrow_mut();
        install_fonts(&mut app);
        drop(app);
        mount(
            &cell,
            widgets_app()
                .home(Marker)
                .restoration_scope_id("app")
                .into_widget(),
        );
        let mut app = cell.borrow_mut();
        settle(&mut app);
        let root = root_element(&mut app);
        let scope =
            find_widget::<RootRestorationScope>(&app, root).expect("the app always builds one");
        assert_eq!(scope.restoration_id.as_deref(), Some("app"));

        let cell = app_with_view();
        let mut app = cell.borrow_mut();
        install_fonts(&mut app);
        drop(app);
        mount(&cell, widgets_app().home(Marker).into_widget());
        let mut app = cell.borrow_mut();
        settle(&mut app);
        let root = root_element(&mut app);
        let scope =
            find_widget::<RootRestorationScope>(&app, root).expect("the app always builds one");
        assert_eq!(scope.restoration_id, None, "restoration stays off");
    }

    #[test]
    fn the_platforms_default_route_name_overrides_the_initial_route() {
        let cell = app_opening_at("/deep");
        let mut app = cell.borrow_mut();
        install_fonts(&mut app);
        let deep: WidgetBuilder = Rc::new(|_app, _context| Marker.into_widget());
        drop(app);
        mount(
            &cell,
            widgets_app()
                .initial_route("/ignored")
                .routes([
                    (
                        String::from("/"),
                        Rc::new(|_app: &mut App, _context: BuildContext| {
                            SizedBox::shrink().into_widget()
                        }) as WidgetBuilder,
                    ),
                    (String::from("/deep"), deep),
                ])
                .into_widget(),
        );
        let mut app = cell.borrow_mut();
        settle(&mut app);
        let root = root_element(&mut app);
        assert!(has_widget::<Marker>(&app, root));
    }
}
