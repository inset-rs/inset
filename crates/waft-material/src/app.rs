//! Port of material_ui's `app.dart`.
//!
//! [`MaterialApp`], the widget a Material Design application is built from, the
//! [`ThemeMode`] it chooses its theme by, and the [`MaterialScrollBehavior`] it installs.
//! `MaterialApp.router` waits with `router.dart`.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use waft_animation::{AnimationStyle, Curve, Curves};
use waft_embedder::{
    Brightness, Clip, Color, FontWeight, Locale, SystemUiOverlayStyle, TargetPlatform,
    TextDecoration, TextDecorationStyle,
};
use waft_foundation::{App, Handle};
use waft_painting::{Axis, TextStyle, axis_direction_to_axis};
use waft_services::SystemChrome;
use waft_widgets::{
    AndroidOverscrollIndicator, AnyAction, AnyNavigatorObserver, AnyRoute, BuildContext, Builder,
    DefaultSelectionStyle, GenerateAppTitle, GlobalKey, GlowingOverscrollIndicator, HeroController,
    HeroControllerScope, InitialRouteListFactory, IntoWidget, KeyRef, LocaleListResolutionCallback,
    LocaleResolutionCallback, LocalizationsDelegateRef, MediaQuery, NavigationNotification,
    NotificationListenerCallback, PageRoute, RouteFactory, RouteSettings, RouteSettingsRef,
    ScrollBehavior, ScrollBehaviorRef, ScrollConfiguration, ScrollableDetails, ShortcutMap,
    SizedBox, State, StateData, StatefulWidget, StretchingOverscrollIndicator, TransitionBuilder,
    View, WidgetBuilder, WidgetRef, WidgetsApp,
};

use waft_cupertino::DefaultCupertinoLocalizations;

use crate::arc::MaterialRectArcTween;
use crate::colors::Colors;
use crate::material_localizations::DefaultMaterialLocalizations;
use crate::page::MaterialPageRoute;
use crate::scaffold::ScaffoldMessenger;
use crate::scrollbar::Scrollbar;
use crate::theme::{AnimatedTheme, K_THEME_ANIMATION_DURATION, Theme};
use crate::theme_data::ThemeData;

/// [`MaterialApp`] uses this [`TextStyle`] as its `DefaultTextStyle` to encourage
/// developers to be intentional about their `DefaultTextStyle`.
///
/// In Material Design, most `Text` widgets are contained in `Material` widgets,
/// which sets a specific `DefaultTextStyle`. If you're seeing text that uses
/// this text style, consider putting your text in a `Material` widget (or
/// another widget that sets a `DefaultTextStyle`).
///
/// Dart's `const _errorTextStyle`, a function because a `TextStyle` holds a `String`.
fn error_text_style() -> TextStyle {
    TextStyle::new()
        .color(Color::new(0xD0FF0000))
        .font_family("monospace")
        .font_size(48.0)
        .font_weight(FontWeight::W900)
        .decoration(TextDecoration::UNDERLINE)
        .decoration_color(Color::new(0xFFFFFF00))
        .decoration_style(TextDecorationStyle::Double)
        .debug_label("fallback style; consider putting your text in a Material")
}

/// Describes which theme will be used by [`MaterialApp`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ThemeMode {
    /// Use either the light or dark theme based on what the user has selected in
    /// the system settings.
    System,

    /// Always use the light mode regardless of system preference.
    Light,

    /// Always use the dark mode (if available) regardless of system preference.
    Dark,
}

impl ThemeMode {
    /// Whether this theme mode follows the system setting.
    pub fn is_system(self) -> bool {
        self == ThemeMode::System
    }

    /// Whether this theme mode forces light mode.
    pub fn is_light(self) -> bool {
        self == ThemeMode::Light
    }

    /// Whether this theme mode forces dark mode.
    pub fn is_dark(self) -> bool {
        self == ThemeMode::Dark
    }
}

/// An application that uses Material Design.
///
/// A convenience widget that wraps a number of widgets that are commonly
/// required for Material Design applications. It builds upon a [`WidgetsApp`] by
/// adding material-design specific functionality, such as [`AnimatedTheme`] and
/// `GridPaper`.
///
/// [`MaterialApp`] configures its [`WidgetsApp::text_style`] with an ugly red/yellow
/// text style that's intended to warn the developer that their app hasn't defined
/// a default text style. Typically the app's `Scaffold` builds a `Material` widget
/// whose default `Material::text_style` defines the text style for the entire scaffold.
///
/// The [`MaterialApp`] configures the top-level `Navigator` to search for routes
/// in the following order:
///
///  1. For the `/` route, the [`home`](Self::home) property, if non-null, is used.
///
///  2. Otherwise, the [`routes`](Self::routes) table is used, if it has an entry for the route.
///
///  3. Otherwise, [`on_generate_route`](Self::on_generate_route) is called, if provided. It
///     should return a non-null value for any _valid_ route not handled by
///     [`home`](Self::home) and [`routes`](Self::routes).
///
///  4. Finally if all else fails [`on_unknown_route`](Self::on_unknown_route) is called.
///
/// If a `Navigator` is created, at least one of these options must handle the
/// `/` route, since it is used when an invalid [`initial_route`](Self::initial_route) is
/// specified on startup (e.g. by another application launching this one with an intent on
/// Android; see `Platform::default_route_name`).
///
/// This widget also configures the observer of the top-level `Navigator` (if
/// any) to perform `Hero` animations.
///
/// The [`MaterialApp`] automatically creates a [`DefaultSelectionStyle`]. It uses
/// the colors in the [`ThemeData::text_selection_theme`] if they are not null;
/// otherwise, the [`MaterialApp`] sets [`DefaultSelectionStyle::selection_color`] to
/// `ColorScheme::primary` with 0.4 opacity and
/// [`DefaultSelectionStyle::cursor_color`] to `ColorScheme::primary`.
///
/// If [`home`](Self::home), [`routes`](Self::routes),
/// [`on_generate_route`](Self::on_generate_route), and
/// [`on_unknown_route`](Self::on_unknown_route) are all null, and
/// [`builder`](Self::builder) is not null, then no `Navigator` is created.
///
/// ```text
/// MaterialApp::new()
///     .debug_show_checked_mode_banner(false)
///     .home(Scaffold::new().body(Center::new().child(Text::new("Hello World"))))
/// ```
///
/// ## Troubleshooting
///
/// ### Why is my app's text red with yellow underlines?
///
/// `Text` widgets that lack a `Material` ancestor will be rendered with an ugly
/// red/yellow text style.
///
/// The typical fix is to give the widget a `Scaffold` ancestor. The `Scaffold` creates
/// a `Material` widget that defines its default text style.
///
/// See also:
///
///  * `Scaffold`, which provides standard app elements like an `AppBar` and a `Drawer`.
///  * `Navigator`, which is used to manage the app's stack of pages.
///  * [`MaterialPageRoute`], which defines an app page that transitions in a material-specific way.
///  * [`WidgetsApp`], which defines the basic app elements but does not depend on the material library.
///
/// The four themes are shared, as Dart shares the one [`ThemeData`] object: the theme a build
/// selects is the one given, so `ThemeData::localize`'s identity cache hits across rebuilds.
pub struct MaterialApp {
    /// See [`Widget::key`](waft_widgets::Widget::key).
    pub key: Option<KeyRef>,

    /// See [`WidgetsApp::navigator_key`].
    pub navigator_key: Option<GlobalKey>,

    /// A key to use when building the [`ScaffoldMessenger`].
    ///
    /// If a [`scaffold_messenger_key`](Self::scaffold_messenger_key) is specified, the
    /// [`ScaffoldMessenger`] can be directly manipulated without first obtaining it from a
    /// [`BuildContext`] via [`ScaffoldMessenger::of`]: from the
    /// [`scaffold_messenger_key`](Self::scaffold_messenger_key), use
    /// `GlobalKey::current_state`.
    pub scaffold_messenger_key: Option<GlobalKey>,

    /// See [`WidgetsApp::home`].
    pub home: Option<WidgetRef>,

    /// The application's top-level routing table.
    ///
    /// When a named route is pushed with `Navigator::push_named`, the route name is
    /// looked up in this map. If the name is present, the associated
    /// [`WidgetBuilder`] is used to construct a [`MaterialPageRoute`] that
    /// performs an appropriate transition, including `Hero` animations, to the
    /// new route.
    ///
    /// See [`WidgetsApp::routes`].
    pub routes: HashMap<String, WidgetBuilder>,

    /// See [`WidgetsApp::initial_route`].
    pub initial_route: Option<String>,

    /// See [`WidgetsApp::on_generate_route`].
    pub on_generate_route: Option<RouteFactory>,

    /// See [`WidgetsApp::on_generate_initial_routes`].
    pub on_generate_initial_routes: Option<InitialRouteListFactory>,

    /// See [`WidgetsApp::on_unknown_route`].
    pub on_unknown_route: Option<RouteFactory>,

    /// See [`WidgetsApp::on_navigation_notification`].
    pub on_navigation_notification: Option<NotificationListenerCallback<NavigationNotification>>,

    /// See [`WidgetsApp::navigator_observers`].
    pub navigator_observers: Vec<AnyNavigatorObserver>,

    /// See [`WidgetsApp::builder`].
    ///
    /// Material specific features such as `showDialog` and `showMenu`, and widgets
    /// such as `Tooltip`, `PopupMenuButton`, also require a `Navigator` to properly
    /// function.
    pub builder: Option<TransitionBuilder>,

    /// See [`WidgetsApp::title`].
    ///
    /// This value is passed unmodified to [`WidgetsApp::title`].
    pub title: Option<String>,

    /// See [`WidgetsApp::on_generate_title`].
    ///
    /// This value is passed unmodified to [`WidgetsApp::on_generate_title`].
    pub on_generate_title: Option<GenerateAppTitle>,

    /// Default visual properties, like colors fonts and shapes, for this app's
    /// material widgets.
    ///
    /// A second [`dark_theme`](Self::dark_theme) [`ThemeData`] value, which is used to provide
    /// a dark version of the user interface can also be specified.
    /// [`theme_mode`](Self::theme_mode) will control which theme will be used if a
    /// [`dark_theme`](Self::dark_theme) is provided.
    ///
    /// The default value of this property is the value of [`ThemeData::light`].
    ///
    /// See also:
    ///
    ///  * [`theme_mode`](Self::theme_mode), which controls which theme to use.
    ///  * `MediaQueryData::platform_brightness`, which indicates the platform's
    ///    desired brightness and is used to automatically toggle between
    ///    [`theme`](Self::theme) and [`dark_theme`](Self::dark_theme) in [`MaterialApp`].
    ///  * [`ThemeData::brightness`], which indicates the [`Brightness`] of a theme's
    ///    colors.
    pub theme: Option<Rc<ThemeData>>,

    /// The [`ThemeData`] to use when a 'dark mode' is requested by the system.
    ///
    /// Some host platforms allow the users to select a system-wide 'dark mode',
    /// or the application may want to offer the user the ability to choose a
    /// dark theme just for this application. This is theme that will be used for
    /// such cases. [`theme_mode`](Self::theme_mode) will control which theme will be used.
    ///
    /// This theme should have a [`ThemeData::brightness`] set to [`Brightness::Dark`].
    ///
    /// Uses [`theme`](Self::theme) instead when null. Defaults to the value of
    /// [`ThemeData::light`] when both [`dark_theme`](Self::dark_theme) and
    /// [`theme`](Self::theme) are null.
    ///
    /// See also:
    ///
    ///  * [`theme_mode`](Self::theme_mode), which controls which theme to use.
    ///  * `MediaQueryData::platform_brightness`, which indicates the platform's
    ///    desired brightness and is used to automatically toggle between
    ///    [`theme`](Self::theme) and [`dark_theme`](Self::dark_theme) in [`MaterialApp`].
    ///  * [`ThemeData::brightness`], which is typically set to the value of
    ///    `MediaQueryData::platform_brightness`.
    pub dark_theme: Option<Rc<ThemeData>>,

    /// The [`ThemeData`] to use when 'high contrast' is requested by the system.
    ///
    /// Some host platforms (for example, iOS) allow the users to increase
    /// contrast through an accessibility setting.
    ///
    /// Uses [`theme`](Self::theme) instead when null.
    ///
    /// See also:
    ///
    ///  * `MediaQueryData::high_contrast`, which indicates the platform's
    ///    desire to increase contrast.
    pub high_contrast_theme: Option<Rc<ThemeData>>,

    /// The [`ThemeData`] to use when a 'dark mode' and 'high contrast' is requested
    /// by the system.
    ///
    /// Some host platforms (for example, iOS) allow the users to increase
    /// contrast through an accessibility setting.
    ///
    /// This theme should have a [`ThemeData::brightness`] set to [`Brightness::Dark`].
    ///
    /// Uses [`dark_theme`](Self::dark_theme) instead when null.
    ///
    /// See also:
    ///
    ///  * `MediaQueryData::high_contrast`, which indicates the platform's
    ///    desire to increase contrast.
    pub high_contrast_dark_theme: Option<Rc<ThemeData>>,

    /// Determines which theme will be used by the application if both [`theme`](Self::theme)
    /// and [`dark_theme`](Self::dark_theme) are provided.
    ///
    /// If set to [`ThemeMode::System`], the choice of which theme to use will
    /// be based on the user's system preferences. If the [`MediaQuery::platform_brightness_of`]
    /// is [`Brightness::Light`], [`theme`](Self::theme) will be used. If it is
    /// [`Brightness::Dark`], [`dark_theme`](Self::dark_theme) will be used (unless it is null,
    /// in which case [`theme`](Self::theme) will be used.
    ///
    /// If set to [`ThemeMode::Light`] the [`theme`](Self::theme) will always be used,
    /// regardless of the user's system preference.
    ///
    /// If set to [`ThemeMode::Dark`] the [`dark_theme`](Self::dark_theme) will be used
    /// regardless of the user's system preference. If [`dark_theme`](Self::dark_theme) is null
    /// then it will fallback to using [`theme`](Self::theme).
    ///
    /// The default value is [`ThemeMode::System`].
    ///
    /// See also:
    ///
    ///  * [`theme`](Self::theme), which is used when a light mode is selected.
    ///  * [`dark_theme`](Self::dark_theme), which is used when a dark mode is selected.
    ///  * [`ThemeData::brightness`], which indicates to various parts of the
    ///    system what kind of theme is being used.
    pub theme_mode: Option<ThemeMode>,

    /// The duration of animated theme changes.
    ///
    /// When the theme changes (either by the [`theme`](Self::theme),
    /// [`dark_theme`](Self::dark_theme) or [`theme_mode`](Self::theme_mode) parameters
    /// changing) it is animated to the new theme over time.
    /// The [`theme_animation_duration`](Self::theme_animation_duration) determines how long
    /// this animation takes.
    ///
    /// To have the theme change immediately, you can set this to [`Duration::ZERO`].
    ///
    /// The default is [`K_THEME_ANIMATION_DURATION`].
    ///
    /// See also:
    ///   [`theme_animation_curve`](Self::theme_animation_curve), which defines the curve used
    ///   for the animation.
    pub theme_animation_duration: Duration,

    /// The curve to apply when animating theme changes.
    ///
    /// The default is [`Curves::linear`].
    ///
    /// This is ignored if [`theme_animation_duration`](Self::theme_animation_duration) is
    /// [`Duration::ZERO`].
    ///
    /// See also:
    ///   [`theme_animation_duration`](Self::theme_animation_duration), which defines how long
    ///   the animation is.
    pub theme_animation_curve: Rc<dyn Curve>,

    /// See [`WidgetsApp::color`].
    pub color: Option<Color>,

    /// See [`WidgetsApp::locale`].
    pub locale: Option<Locale>,

    /// See [`WidgetsApp::localizations_delegates`].
    ///
    /// Delegates that produce `WidgetsLocalizations` and `MaterialLocalizations`
    /// are included automatically. Apps can provide their own versions of these
    /// localizations by creating implementations of `LocalizationsDelegate` whose load
    /// methods return custom versions of `WidgetsLocalizations` or `MaterialLocalizations`.
    ///
    /// Only the first delegate of each `LocalizationsDelegate` type is used and the
    /// automatically included delegates are added to the end of the app's
    /// [`localizations_delegates`](Self::localizations_delegates) list, so a delegate given
    /// here overrides the included one of its type.
    ///
    /// See also:
    ///
    ///  * [`supported_locales`](Self::supported_locales), which must be specified along with
    ///    [`localizations_delegates`](Self::localizations_delegates).
    pub localizations_delegates: Option<Vec<LocalizationsDelegateRef>>,

    /// See [`WidgetsApp::locale_list_resolution_callback`].
    ///
    /// This callback is passed along to the [`WidgetsApp`] built by this widget.
    pub locale_list_resolution_callback: Option<LocaleListResolutionCallback>,

    /// See [`WidgetsApp::locale_resolution_callback`].
    ///
    /// This callback is passed along to the [`WidgetsApp`] built by this widget.
    pub locale_resolution_callback: Option<LocaleResolutionCallback>,

    /// See [`WidgetsApp::supported_locales`].
    ///
    /// It is passed along unmodified to the [`WidgetsApp`] built by this widget.
    ///
    /// See also:
    ///
    ///  * [`localizations_delegates`](Self::localizations_delegates), which must be specified
    ///    for localized applications.
    pub supported_locales: Vec<Locale>,

    /// Turns on a performance overlay.
    ///
    /// See also:
    ///
    ///  * <https://flutter.dev/to/performance-overlay>
    pub show_performance_overlay: bool,

    /// Turns on checkerboarding of raster cache images.
    pub checkerboard_raster_cache_images: bool,

    /// Turns on checkerboarding of layers rendered to offscreen bitmaps.
    pub checkerboard_offscreen_layers: bool,

    /// Turns on an overlay that shows the accessibility information
    /// reported by the framework.
    pub show_semantics_debugger: bool,

    /// See [`WidgetsApp::debug_show_checked_mode_banner`].
    pub debug_show_checked_mode_banner: bool,

    /// See [`WidgetsApp::shortcuts`].
    pub shortcuts: Option<ShortcutMap>,

    /// See [`WidgetsApp::actions`].
    pub actions: Option<HashMap<TypeId, AnyAction>>,

    /// See [`WidgetsApp::restoration_scope_id`].
    pub restoration_scope_id: Option<String>,

    /// The default [`ScrollBehavior`] for the application.
    ///
    /// When null, defaults to [`MaterialScrollBehavior`].
    ///
    /// See also:
    ///
    ///  * [`ScrollConfiguration`], which controls how `Scrollable` widgets behave
    ///    in a subtree.
    pub scroll_behavior: Option<ScrollBehaviorRef>,

    /// Turns on a `GridPaper` overlay that paints a baseline grid
    /// Material apps.
    ///
    /// Only available in debug mode.
    ///
    /// See also:
    ///
    ///  * <https://material.io/design/layout/spacing-methods.html>
    pub debug_show_material_grid: bool,

    /// Dart's deprecated `useInheritedMediaQuery`. This setting is ignored.
    ///
    /// The widget never introduces its own `MediaQuery`; the `View` widget takes
    /// care of that.
    pub use_inherited_media_query: bool,

    /// Used to override the theme animation curve and duration.
    ///
    /// If [`AnimationStyle::duration`] is provided, it will be used to override
    /// the theme animation duration in the underlying [`AnimatedTheme`] widget.
    /// If it is null, then [`theme_animation_duration`](Self::theme_animation_duration) will
    /// be used. Otherwise, defaults to 200ms.
    ///
    /// If [`AnimationStyle::curve`] is provided, it will be used to override
    /// the theme animation curve in the underlying [`AnimatedTheme`] widget.
    /// If it is null, then [`theme_animation_curve`](Self::theme_animation_curve) will be
    /// used. Otherwise, defaults to [`Curves::linear`].
    ///
    /// To disable the theme animation, use [`AnimationStyle::NO_ANIMATION`].
    pub theme_animation_style: Option<AnimationStyle>,
}

impl Default for MaterialApp {
    fn default() -> MaterialApp {
        MaterialApp {
            key: None,
            navigator_key: None,
            scaffold_messenger_key: None,
            home: None,
            routes: HashMap::new(),
            initial_route: None,
            on_generate_route: None,
            on_generate_initial_routes: None,
            on_unknown_route: None,
            on_navigation_notification: None,
            navigator_observers: Vec::new(),
            builder: None,
            title: Some(String::new()),
            on_generate_title: None,
            theme: None,
            dark_theme: None,
            high_contrast_theme: None,
            high_contrast_dark_theme: None,
            theme_mode: Some(ThemeMode::System),
            theme_animation_duration: K_THEME_ANIMATION_DURATION,
            theme_animation_curve: Curves::linear(),
            color: None,
            locale: None,
            localizations_delegates: None,
            locale_list_resolution_callback: None,
            locale_resolution_callback: None,
            supported_locales: vec![Locale::new("en").country_code("US")],
            show_performance_overlay: false,
            checkerboard_raster_cache_images: false,
            checkerboard_offscreen_layers: false,
            show_semantics_debugger: false,
            debug_show_checked_mode_banner: true,
            shortcuts: None,
            actions: None,
            restoration_scope_id: None,
            scroll_behavior: None,
            debug_show_material_grid: false,
            use_inherited_media_query: false,
            theme_animation_style: None,
        }
    }
}

impl MaterialApp {
    /// Creates a MaterialApp; Dart's named arguments are the setters.
    ///
    /// At least one of [`home`](Self::home), [`routes`](Self::routes),
    /// [`on_generate_route`](Self::on_generate_route), or [`builder`](Self::builder) must be
    /// set. If only [`routes`](Self::routes) is given, it must include an entry for
    /// `Navigator::DEFAULT_ROUTE_NAME` (`/`), since that is the route used when the
    /// application is launched with an intent that specifies an otherwise unsupported route.
    ///
    /// This class creates an instance of [`WidgetsApp`].
    pub fn new() -> MaterialApp {
        MaterialApp::default()
    }

    /// Dart `MaterialApp(key:)`.
    pub fn key(mut self, key: KeyRef) -> MaterialApp {
        self.key = Some(key);
        self
    }

    /// Dart `MaterialApp(navigatorKey:)`.
    pub fn navigator_key(mut self, navigator_key: GlobalKey) -> MaterialApp {
        self.navigator_key = Some(navigator_key);
        self
    }

    /// Dart `MaterialApp(scaffoldMessengerKey:)`.
    pub fn scaffold_messenger_key(mut self, scaffold_messenger_key: GlobalKey) -> MaterialApp {
        self.scaffold_messenger_key = Some(scaffold_messenger_key);
        self
    }

    /// Dart `MaterialApp(home:)`.
    pub fn home<K>(mut self, home: impl IntoWidget<K>) -> MaterialApp {
        self.home = Some(home.into_widget());
        self
    }

    /// Dart `MaterialApp(routes:)`.
    pub fn routes(
        mut self,
        routes: impl IntoIterator<Item = (String, WidgetBuilder)>,
    ) -> MaterialApp {
        self.routes = routes.into_iter().collect();
        self
    }

    /// Dart `MaterialApp(initialRoute:)`.
    pub fn initial_route(mut self, initial_route: impl Into<String>) -> MaterialApp {
        self.initial_route = Some(initial_route.into());
        self
    }

    /// Dart `MaterialApp(onGenerateRoute:)`.
    pub fn on_generate_route(
        mut self,
        on_generate_route: impl Fn(&mut App, &RouteSettings) -> Option<AnyRoute> + 'static,
    ) -> MaterialApp {
        self.on_generate_route = Some(Rc::new(on_generate_route));
        self
    }

    /// Dart `MaterialApp(onGenerateInitialRoutes:)`.
    pub fn on_generate_initial_routes(
        mut self,
        on_generate_initial_routes: impl Fn(&mut App, &str) -> Vec<AnyRoute> + 'static,
    ) -> MaterialApp {
        self.on_generate_initial_routes = Some(Rc::new(on_generate_initial_routes));
        self
    }

    /// Dart `MaterialApp(onUnknownRoute:)`.
    pub fn on_unknown_route(
        mut self,
        on_unknown_route: impl Fn(&mut App, &RouteSettings) -> Option<AnyRoute> + 'static,
    ) -> MaterialApp {
        self.on_unknown_route = Some(Rc::new(on_unknown_route));
        self
    }

    /// Dart `MaterialApp(onNavigationNotification:)`.
    pub fn on_navigation_notification(
        mut self,
        on_navigation_notification: impl Fn(&mut App, &NavigationNotification) -> bool + 'static,
    ) -> MaterialApp {
        self.on_navigation_notification = Some(Rc::new(on_navigation_notification));
        self
    }

    /// Dart `MaterialApp(navigatorObservers:)`.
    pub fn navigator_observers(
        mut self,
        navigator_observers: impl IntoIterator<Item = AnyNavigatorObserver>,
    ) -> MaterialApp {
        self.navigator_observers = navigator_observers.into_iter().collect();
        self
    }

    /// Dart `MaterialApp(builder:)`.
    pub fn builder(
        mut self,
        builder: impl Fn(&mut App, BuildContext, Option<&WidgetRef>) -> WidgetRef + 'static,
    ) -> MaterialApp {
        self.builder = Some(Rc::new(builder));
        self
    }

    /// Dart `MaterialApp(title:)`.
    pub fn title(mut self, title: impl Into<String>) -> MaterialApp {
        self.title = Some(title.into());
        self
    }

    /// Dart `MaterialApp(onGenerateTitle:)`.
    pub fn on_generate_title(
        mut self,
        on_generate_title: impl Fn(&mut App, BuildContext) -> String + 'static,
    ) -> MaterialApp {
        self.on_generate_title = Some(Rc::new(on_generate_title));
        self
    }

    /// Dart `MaterialApp(theme:)`.
    pub fn theme(mut self, theme: impl Into<Rc<ThemeData>>) -> MaterialApp {
        self.theme = Some(theme.into());
        self
    }

    /// Dart `MaterialApp(darkTheme:)`.
    pub fn dark_theme(mut self, dark_theme: impl Into<Rc<ThemeData>>) -> MaterialApp {
        self.dark_theme = Some(dark_theme.into());
        self
    }

    /// Dart `MaterialApp(highContrastTheme:)`.
    pub fn high_contrast_theme(
        mut self,
        high_contrast_theme: impl Into<Rc<ThemeData>>,
    ) -> MaterialApp {
        self.high_contrast_theme = Some(high_contrast_theme.into());
        self
    }

    /// Dart `MaterialApp(highContrastDarkTheme:)`.
    pub fn high_contrast_dark_theme(
        mut self,
        high_contrast_dark_theme: impl Into<Rc<ThemeData>>,
    ) -> MaterialApp {
        self.high_contrast_dark_theme = Some(high_contrast_dark_theme.into());
        self
    }

    /// Dart `MaterialApp(themeMode:)`.
    pub fn theme_mode(mut self, theme_mode: ThemeMode) -> MaterialApp {
        self.theme_mode = Some(theme_mode);
        self
    }

    /// Dart `MaterialApp(themeAnimationDuration:)`.
    pub fn theme_animation_duration(mut self, theme_animation_duration: Duration) -> MaterialApp {
        self.theme_animation_duration = theme_animation_duration;
        self
    }

    /// Dart `MaterialApp(themeAnimationCurve:)`.
    pub fn theme_animation_curve(mut self, theme_animation_curve: Rc<dyn Curve>) -> MaterialApp {
        self.theme_animation_curve = theme_animation_curve;
        self
    }

    /// Dart `MaterialApp(color:)`.
    pub fn color(mut self, color: Color) -> MaterialApp {
        self.color = Some(color);
        self
    }

    /// Dart `MaterialApp(locale:)`.
    pub fn locale(mut self, locale: Locale) -> MaterialApp {
        self.locale = Some(locale);
        self
    }

    /// Dart `MaterialApp(localizationsDelegates:)`.
    pub fn localizations_delegates(
        mut self,
        localizations_delegates: impl IntoIterator<Item = LocalizationsDelegateRef>,
    ) -> MaterialApp {
        self.localizations_delegates = Some(localizations_delegates.into_iter().collect());
        self
    }

    /// Dart `MaterialApp(localeListResolutionCallback:)`.
    pub fn locale_list_resolution_callback(
        mut self,
        locale_list_resolution_callback: impl Fn(Option<&[Locale]>, &[Locale]) -> Option<Locale>
        + 'static,
    ) -> MaterialApp {
        self.locale_list_resolution_callback = Some(Rc::new(locale_list_resolution_callback));
        self
    }

    /// Dart `MaterialApp(localeResolutionCallback:)`.
    pub fn locale_resolution_callback(
        mut self,
        locale_resolution_callback: impl Fn(Option<&Locale>, &[Locale]) -> Option<Locale> + 'static,
    ) -> MaterialApp {
        self.locale_resolution_callback = Some(Rc::new(locale_resolution_callback));
        self
    }

    /// Dart `MaterialApp(supportedLocales:)`.
    pub fn supported_locales(
        mut self,
        supported_locales: impl IntoIterator<Item = Locale>,
    ) -> MaterialApp {
        self.supported_locales = supported_locales.into_iter().collect();
        self
    }

    /// Dart `MaterialApp(debugShowMaterialGrid:)`.
    pub fn debug_show_material_grid(mut self, debug_show_material_grid: bool) -> MaterialApp {
        self.debug_show_material_grid = debug_show_material_grid;
        self
    }

    /// Dart `MaterialApp(showPerformanceOverlay:)`.
    pub fn show_performance_overlay(mut self, show_performance_overlay: bool) -> MaterialApp {
        self.show_performance_overlay = show_performance_overlay;
        self
    }

    /// Dart `MaterialApp(checkerboardRasterCacheImages:)`.
    pub fn checkerboard_raster_cache_images(
        mut self,
        checkerboard_raster_cache_images: bool,
    ) -> MaterialApp {
        self.checkerboard_raster_cache_images = checkerboard_raster_cache_images;
        self
    }

    /// Dart `MaterialApp(checkerboardOffscreenLayers:)`.
    pub fn checkerboard_offscreen_layers(
        mut self,
        checkerboard_offscreen_layers: bool,
    ) -> MaterialApp {
        self.checkerboard_offscreen_layers = checkerboard_offscreen_layers;
        self
    }

    /// Dart `MaterialApp(showSemanticsDebugger:)`.
    pub fn show_semantics_debugger(mut self, show_semantics_debugger: bool) -> MaterialApp {
        self.show_semantics_debugger = show_semantics_debugger;
        self
    }

    /// Dart `MaterialApp(debugShowCheckedModeBanner:)`.
    pub fn debug_show_checked_mode_banner(
        mut self,
        debug_show_checked_mode_banner: bool,
    ) -> MaterialApp {
        self.debug_show_checked_mode_banner = debug_show_checked_mode_banner;
        self
    }

    /// Dart `MaterialApp(shortcuts:)`.
    pub fn shortcuts(mut self, shortcuts: ShortcutMap) -> MaterialApp {
        self.shortcuts = Some(shortcuts);
        self
    }

    /// Dart `MaterialApp(actions:)`.
    pub fn actions(mut self, actions: HashMap<TypeId, AnyAction>) -> MaterialApp {
        self.actions = Some(actions);
        self
    }

    /// Dart `MaterialApp(restorationScopeId:)`.
    pub fn restoration_scope_id(mut self, restoration_scope_id: impl Into<String>) -> MaterialApp {
        self.restoration_scope_id = Some(restoration_scope_id.into());
        self
    }

    /// Dart `MaterialApp(scrollBehavior:)`.
    pub fn scroll_behavior(mut self, scroll_behavior: ScrollBehaviorRef) -> MaterialApp {
        self.scroll_behavior = Some(scroll_behavior);
        self
    }

    /// Dart `MaterialApp(useInheritedMediaQuery:)`, which is ignored.
    pub fn use_inherited_media_query(mut self, use_inherited_media_query: bool) -> MaterialApp {
        self.use_inherited_media_query = use_inherited_media_query;
        self
    }

    /// Dart `MaterialApp(themeAnimationStyle:)`.
    pub fn theme_animation_style(mut self, theme_animation_style: AnimationStyle) -> MaterialApp {
        self.theme_animation_style = Some(theme_animation_style);
        self
    }

    /// The [`HeroController`] used for Material page transitions.
    ///
    /// Used by the [`MaterialApp`].
    pub fn create_material_hero_controller(app: &mut App) -> Handle<HeroController> {
        HeroController::new(app).create_rect_tween(
            app,
            Rc::new(|_app, begin, end| Rc::new(MaterialRectArcTween::new(begin, end))),
        )
    }
}

impl Debug for MaterialApp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MaterialApp")
            .field("title", &self.title)
            .field("themeMode", &self.theme_mode)
            .field("locale", &self.locale)
            .field("restorationScopeId", &self.restoration_scope_id)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for MaterialApp {
    type State = MaterialAppState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> MaterialAppState {
        MaterialAppState {
            state: StateData::new(),
            hero_controller: None,
        }
    }
}

/// Describes how `Scrollable` widgets behave for [`MaterialApp`]s.
///
/// Setting a [`MaterialScrollBehavior`] will apply a
/// [`GlowingOverscrollIndicator`] to `Scrollable` descendants when executing on
/// [`TargetPlatform::Android`] and [`TargetPlatform::Fuchsia`].
///
/// When using the desktop platform, if the `Scrollable` widget scrolls in the
/// [`Axis::Vertical`], a [`Scrollbar`] is applied.
///
/// If the scroll direction is [`Axis::Horizontal`] scroll views are less
/// discoverable, so consider adding a Scrollbar in these cases, either directly
/// or through the [`build_scrollbar`](ScrollBehavior::build_scrollbar) method.
///
/// [`ThemeData::use_material3`] specifies the
/// overscroll indicator that is used on [`TargetPlatform::Android`], which
/// defaults to true, resulting in a [`StretchingOverscrollIndicator`]. Setting
/// [`ThemeData::use_material3`] to false will instead use a
/// [`GlowingOverscrollIndicator`].
///
/// See also:
///
///  * [`ScrollBehavior`], the default scrolling behavior extended by this class.
#[derive(Clone, Copy, Debug, Default)]
pub struct MaterialScrollBehavior;

impl MaterialScrollBehavior {
    /// Creates a MaterialScrollBehavior that decorates `Scrollable`s with
    /// [`StretchingOverscrollIndicator`]s and [`Scrollbar`]s based on the current
    /// platform and provided [`ScrollableDetails`].
    ///
    /// [`ThemeData::use_material3`] specifies the
    /// overscroll indicator that is used on [`TargetPlatform::Android`], which
    /// defaults to true, resulting in a [`StretchingOverscrollIndicator`]. Setting
    /// [`ThemeData::use_material3`] to false will instead use a
    /// [`GlowingOverscrollIndicator`].
    pub const fn new() -> MaterialScrollBehavior {
        MaterialScrollBehavior
    }
}

impl ScrollBehavior for MaterialScrollBehavior {
    fn as_any(&self) -> &dyn Any {
        self
    }

    /// Dart reads `Theme.of(context).platform`; this reads the nearest [`Theme`] above
    /// `context` without registering a dependency, because the getter is given `&App` only.
    /// Without one it answers what `Theme.of`'s fallbacks answer: the default platform.
    fn get_platform(&self, app: &App, context: BuildContext) -> TargetPlatform {
        context
            .find_ancestor_widget_of_exact_type::<Theme>(app)
            .map_or_else(
                || app.platform().target_platform(),
                |theme| theme.data.platform,
            )
    }

    fn build_scrollbar(
        &self,
        app: &mut App,
        context: BuildContext,
        child: WidgetRef,
        details: &ScrollableDetails,
    ) -> WidgetRef {
        // When modifying this function, consider modifying the implementation in
        // the base class ScrollBehavior as well.
        match axis_direction_to_axis(details.direction) {
            Axis::Horizontal => child,
            Axis::Vertical => match self.get_platform(app, context) {
                TargetPlatform::Linux | TargetPlatform::MacOS | TargetPlatform::Windows => {
                    debug_assert!(details.controller.is_some());
                    let mut scrollbar = Scrollbar::new(child);
                    if let Some(controller) = details.controller {
                        scrollbar = scrollbar.controller(controller);
                    }
                    scrollbar.into_widget()
                }
                TargetPlatform::Android | TargetPlatform::Fuchsia | TargetPlatform::IOS => child,
            },
        }
    }

    fn build_overscroll_indicator(
        &self,
        app: &mut App,
        context: BuildContext,
        child: WidgetRef,
        details: &ScrollableDetails,
    ) -> WidgetRef {
        // When modifying this function, consider modifying the implementation in
        // the base class ScrollBehavior as well.
        let indicator = if Theme::of(app, context).use_material3 {
            AndroidOverscrollIndicator::Stretch
        } else {
            AndroidOverscrollIndicator::Glow
        };
        match self.get_platform(app, context) {
            TargetPlatform::IOS
            | TargetPlatform::Linux
            | TargetPlatform::MacOS
            | TargetPlatform::Windows => return child,
            TargetPlatform::Android => match indicator {
                AndroidOverscrollIndicator::Stretch => {
                    return StretchingOverscrollIndicator::new(details.direction)
                        .clip_behavior(details.decoration_clip_behavior.unwrap_or(Clip::HardEdge))
                        .child(child)
                        .into_widget();
                }
                AndroidOverscrollIndicator::Glow => {}
            },
            TargetPlatform::Fuchsia => {}
        }
        GlowingOverscrollIndicator::new(
            details.direction,
            Theme::of(app, context).color_scheme.secondary,
        )
        .child(child)
        .into_widget()
    }
}

/// Dart's `_MaterialAppState`.
pub struct MaterialAppState {
    state: StateData<MaterialApp>,
    hero_controller: Option<Handle<HeroController>>,
}

impl MaterialAppState {
    fn hero_controller(self: Handle<Self>, app: &App) -> Handle<HeroController> {
        app.get(self)
            .hero_controller
            .expect("init_state creates the controller")
    }

    /// Combine the Localizations for Material with the ones contributed
    /// by the [`MaterialApp::localizations_delegates`] parameter, if any. Only the first
    /// delegate of a particular `LocalizationsDelegate` type is loaded so the
    /// [`MaterialApp::localizations_delegates`] parameter can be used to override
    /// `_MaterialLocalizationsDelegate`.
    fn localizations_delegates(self: Handle<Self>, app: &App) -> Vec<LocalizationsDelegateRef> {
        let mut delegates = self
            .widget(app)
            .localizations_delegates
            .clone()
            .unwrap_or_default();
        delegates.push(DefaultMaterialLocalizations::delegate());
        delegates.push(DefaultCupertinoLocalizations::delegate());
        delegates
    }

    fn theme_builder(self: Handle<Self>, app: &mut App, context: BuildContext) -> Rc<ThemeData> {
        // Resolve which theme to use based on brightness and high contrast.
        let mode = self.widget(app).theme_mode.unwrap_or(ThemeMode::System);
        let platform_brightness = MediaQuery::platform_brightness_of(app, context);
        let use_dark_theme = mode == ThemeMode::Dark
            || (mode == ThemeMode::System && platform_brightness == Brightness::Dark);
        let high_contrast = MediaQuery::high_contrast_of(app, context);
        let widget = self.widget(app);
        let mut theme: Option<&Rc<ThemeData>> = None;
        if use_dark_theme && high_contrast && widget.high_contrast_dark_theme.is_some() {
            theme = widget.high_contrast_dark_theme.as_ref();
        } else if use_dark_theme && widget.dark_theme.is_some() {
            theme = widget.dark_theme.as_ref();
        } else if high_contrast && widget.high_contrast_theme.is_some() {
            theme = widget.high_contrast_theme.as_ref();
        }
        let theme = match theme.or(widget.theme.as_ref()) {
            Some(theme) => Rc::clone(theme),
            None => Rc::new(ThemeData::new(app)),
        };
        if let Some(view) = View::maybe_of(app, context) {
            SystemChrome::set_system_ui_overlay_style(
                app,
                view.id(),
                match theme.brightness() {
                    Brightness::Dark => &SystemUiOverlayStyle::LIGHT,
                    Brightness::Light => &SystemUiOverlayStyle::DARK,
                },
            );
        }

        theme
    }

    fn material_builder(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        child: Option<&WidgetRef>,
    ) -> WidgetRef {
        let theme = self.theme_builder(app, context);
        // Dart's deprecated `withOpacity`: its rounding to an 8-bit alpha is part of the
        // behaviour being ported.
        #[allow(deprecated)]
        let effective_selection_color = theme
            .text_selection_theme
            .selection_color
            .unwrap_or_else(|| theme.color_scheme.primary.with_opacity(0.40));
        let effective_cursor_color = theme
            .text_selection_theme
            .cursor_color
            .unwrap_or(theme.color_scheme.primary);

        let mut child_widget = child
            .cloned()
            .unwrap_or_else(|| SizedBox::shrink().into_widget());

        if let Some(builder) = self.widget(app).builder.clone() {
            let child = child.cloned();
            child_widget = Builder::new(move |app, context| {
                // Why are we surrounding a builder with a builder?
                //
                // The widget.builder may contain code that invokes
                // Theme.of(), which should return the theme we selected
                // above in AnimatedTheme. However, if we invoke
                // widget.builder() directly as the child of AnimatedTheme
                // then there is no BuildContext separating them, the
                // widget.builder() will not find the theme. Therefore, we
                // surround widget.builder with yet another builder so that
                // a context separates them and Theme.of() correctly
                // resolves to the theme we passed to AnimatedTheme.
                builder(app, context, child.as_ref())
            })
            .into_widget();
        }

        let mut scaffold_messenger = ScaffoldMessenger::new(
            DefaultSelectionStyle::new(child_widget)
                .selection_color(effective_selection_color)
                .cursor_color(effective_cursor_color),
        );
        if let Some(scaffold_messenger_key) = self.widget(app).scaffold_messenger_key.clone() {
            scaffold_messenger = scaffold_messenger.key(scaffold_messenger_key.into());
        }
        child_widget = scaffold_messenger.into_widget();

        let widget = self.widget(app);
        if widget.theme_animation_style.as_ref() != Some(&AnimationStyle::NO_ANIMATION) {
            let duration = widget
                .theme_animation_style
                .as_ref()
                .and_then(|style| style.duration)
                .unwrap_or(widget.theme_animation_duration);
            let curve = widget
                .theme_animation_style
                .as_ref()
                .and_then(|style| style.curve.clone())
                .unwrap_or_else(|| Rc::clone(&widget.theme_animation_curve));
            child_widget = AnimatedTheme::new(theme, child_widget)
                .duration(duration)
                .curve(curve)
                .into_widget();
        } else {
            child_widget = Theme::new(theme, child_widget).into_widget();
        }

        child_widget
    }

    fn build_widget_app(self: Handle<Self>, app: &App) -> WidgetsApp {
        // The color property is always pulled from the light theme, even if dark
        // mode is activated. This was done to simplify the technical details
        // of switching themes and it was deemed acceptable because this color
        // property is only used on old Android OSes to color the app bar in
        // Android's switcher UI.
        //
        // blue is the primary color of the default theme.
        let widget = self.widget(app);
        let material_color = widget
            .color
            .or_else(|| widget.theme.as_ref().map(|theme| theme.primary_color))
            .unwrap_or_else(|| Colors::BLUE.primary());

        // The `WidgetsApp.router` branch waits with `router.dart`; see PORTING.md.

        let localizations_delegates = self.localizations_delegates(app);
        let mut widgets_app = WidgetsApp::new(material_color)
            .key(GlobalKey::object(self.id()).into())
            .navigator_observers(widget.navigator_observers.clone())
            .page_route_builder(|app, settings, builder| {
                let route = MaterialPageRoute::new(app, builder)
                    .settings(app, RouteSettingsRef::Settings(settings.clone()));
                PageRoute::as_page_route(route)
            })
            .routes(widget.routes.clone())
            .builder(move |app, context, child| self.material_builder(app, context, child))
            .text_style(error_text_style())
            .localizations_delegates(localizations_delegates)
            .supported_locales(widget.supported_locales.clone())
            .show_performance_overlay(widget.show_performance_overlay)
            .show_semantics_debugger(widget.show_semantics_debugger)
            .debug_show_checked_mode_banner(widget.debug_show_checked_mode_banner);
        // The inspector's three button builders wait with `widget_inspector.dart`; see
        // PORTING.md.

        if let Some(navigator_key) = widget.navigator_key.clone() {
            widgets_app = widgets_app.navigator_key(navigator_key);
        }
        if let Some(home) = widget.home.clone() {
            widgets_app = widgets_app.home(home);
        }
        if let Some(initial_route) = widget.initial_route.clone() {
            widgets_app = widgets_app.initial_route(initial_route);
        }
        if let Some(on_generate_route) = widget.on_generate_route.clone() {
            widgets_app = widgets_app
                .on_generate_route(move |app, settings| on_generate_route(app, settings));
        }
        if let Some(on_generate_initial_routes) = widget.on_generate_initial_routes.clone() {
            widgets_app = widgets_app
                .on_generate_initial_routes(move |app, name| on_generate_initial_routes(app, name));
        }
        if let Some(on_unknown_route) = widget.on_unknown_route.clone() {
            widgets_app =
                widgets_app.on_unknown_route(move |app, settings| on_unknown_route(app, settings));
        }
        if let Some(on_navigation_notification) = widget.on_navigation_notification.clone() {
            widgets_app = widgets_app.on_navigation_notification(move |app, notification| {
                on_navigation_notification(app, notification)
            });
        }
        if let Some(title) = widget.title.clone() {
            widgets_app = widgets_app.title(title);
        }
        if let Some(on_generate_title) = widget.on_generate_title.clone() {
            widgets_app =
                widgets_app.on_generate_title(move |app, context| on_generate_title(app, context));
        }
        if let Some(locale) = widget.locale.clone() {
            widgets_app = widgets_app.locale(locale);
        }
        if let Some(callback) = widget.locale_resolution_callback.clone() {
            widgets_app = widgets_app
                .locale_resolution_callback(move |locale, supported| callback(locale, supported));
        }
        if let Some(callback) = widget.locale_list_resolution_callback.clone() {
            widgets_app = widgets_app.locale_list_resolution_callback(move |locales, supported| {
                callback(locales, supported)
            });
        }
        if let Some(shortcuts) = widget.shortcuts.clone() {
            widgets_app = widgets_app.shortcuts(shortcuts);
        }
        if let Some(actions) = widget.actions.clone() {
            widgets_app = widgets_app.actions(actions);
        }
        if let Some(restoration_scope_id) = widget.restoration_scope_id.clone() {
            widgets_app = widgets_app.restoration_scope_id(restoration_scope_id);
        }
        widgets_app
    }
}

impl State for MaterialAppState {
    type Widget = MaterialApp;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let hero_controller = MaterialApp::create_material_hero_controller(app);
        app.get_mut(self).hero_controller = Some(hero_controller);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.hero_controller(app).dispose(app);
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let result = self.build_widget_app(app).into_widget();
        // The `GridPaper` that `debug_show_material_grid` puts over `result` waits with
        // `grid_paper.dart`; see PORTING.md.

        let behavior = self
            .widget(app)
            .scroll_behavior
            .clone()
            .unwrap_or_else(|| ScrollBehaviorRef::new(MaterialScrollBehavior::new()));
        let hero_controller = self.hero_controller(app);
        ScrollConfiguration::new(behavior, HeroControllerScope::new(hero_controller, result))
            .into_widget()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::panic::{AssertUnwindSafe, catch_unwind};

    use waft_embedder::{Offset, Rect, Size};
    use waft_foundation::{CompleterFuture, Listener};
    use waft_test::TestPlatform;
    use waft_widgets::{
        AnyElement, Center, CheckedModeBanner, FocusNodeLeaf, FocusScope, FocusScopeNode, ListView,
        Localizations, MediaQuery, MediaQueryData, Navigator, NavigatorState,
        NeverScrollableScrollPhysics, PageRouteBuilder, RouteResult, ScrollController,
        ScrollControllerLeaf, ScrollPhysicsRef, ScrollViewKeyboardDismissBehavior, Text,
        UnconstrainedBox, WidgetsBinding, WidgetsLocalizations, downcast_widget,
    };

    use super::*;
    use crate::elevated_button::ElevatedButton;
    use crate::material::Material;
    use crate::material_localizations::MaterialLocalizations;
    use crate::scaffold::{Scaffold, ScaffoldMessengerState};
    use crate::test_support::{WidgetTester, is_same_color_as, with_main_thread_stack};
    use waft_cupertino::CupertinoLocalizations;

    const ALL_PLATFORMS: [TargetPlatform; 6] = [
        TargetPlatform::Android,
        TargetPlatform::Fuchsia,
        TargetPlatform::IOS,
        TargetPlatform::Linux,
        TargetPlatform::MacOS,
        TargetPlatform::Windows,
    ];

    fn tester_on(platform: TargetPlatform) -> WidgetTester {
        let tester = WidgetTester::on(platform);
        tester.install_fonts();
        tester
    }

    /// An Android tester whose host reports `brightness`.
    fn tester_with_brightness(brightness: Brightness) -> WidgetTester {
        let tester = WidgetTester::with_platform(
            TestPlatform::new()
                .on(TargetPlatform::Android)
                .with_brightness(brightness),
        );
        tester.install_fonts();
        tester
    }

    /// Dart's `tester.pumpWidget(app)`, with the theme data `app` needs made on the tester's app.
    fn pump(tester: &mut WidgetTester, app: impl FnOnce(&App) -> MaterialApp) {
        let root = app(&tester.cell.borrow()).into_widget();
        tester.pump_widget(root);
    }

    /// A `Builder` that keeps the theme it sees in `seen`, each time it builds.
    fn theme_probe(seen: &Rc<RefCell<Option<Rc<ThemeData>>>>) -> Builder {
        let seen = Rc::clone(seen);
        Builder::new(move |app, context| {
            *seen.borrow_mut() = Some(Theme::of(app, context));
            SizedBox::new().into_widget()
        })
    }

    /// A `Builder` that keeps its build context in `seen`.
    fn context_probe(seen: &Rc<Cell<Option<BuildContext>>>) -> Builder {
        let seen = Rc::clone(seen);
        Builder::new(move |_app, context| {
            seen.set(Some(context));
            SizedBox::new().into_widget()
        })
    }

    fn applied(seen: &Rc<RefCell<Option<Rc<ThemeData>>>>) -> Rc<ThemeData> {
        seen.borrow().clone().expect("the probe built")
    }

    fn brightness_theme(app: &App, brightness: Brightness) -> ThemeData {
        ThemeData::builder().brightness(brightness).build(app)
    }

    fn primary_theme(app: &App, primary_color: Color) -> ThemeData {
        ThemeData::builder().primary_color(primary_color).build(app)
    }

    fn text_page(text: &'static str) -> WidgetBuilder {
        Rc::new(move |_app, _context| Text::new(text).into_widget())
    }

    fn count_page(text: &'static str, count: &Rc<Cell<u32>>) -> WidgetBuilder {
        let count = Rc::clone(count);
        Rc::new(move |_app, _context| {
            count.set(count.get() + 1);
            Text::new(text).into_widget()
        })
    }

    fn route_table(names: &[&'static str]) -> Vec<(String, WidgetBuilder)> {
        names
            .iter()
            .map(|name| {
                let text: &'static str = match *name {
                    "/" => "route \"/\"",
                    "/a" => "route \"/a\"",
                    "/a/b" => "route \"/a/b\"",
                    _ => "route \"/b\"",
                };
                (name.to_string(), text_page(text))
            })
            .collect()
    }

    fn found(tester: &WidgetTester, text: &str) -> usize {
        tester.find_text(text).len()
    }

    fn onstage(tester: &WidgetTester, text: &str) -> usize {
        tester.find_onstage_text(text).len()
    }

    fn navigator(tester: &WidgetTester) -> Handle<NavigatorState> {
        let element = *tester
            .find_by_type::<Navigator>()
            .first()
            .expect("a navigator");
        element
            .state_handle::<NavigatorState>(&tester.cell.borrow())
            .expect("a navigator's state")
    }

    fn center_of_text(tester: &WidgetTester, text: &str) -> Offset {
        let element = *tester.find_text(text).first().expect("the text is mounted");
        tester.global_rect(tester.render_box_of(element)).center()
    }

    /// The colour of the first `Material`: Dart's `tester.widget<Material>(find.byType(Material)).color`.
    fn material_color(tester: &WidgetTester) -> Option<Color> {
        let element = *tester
            .find_by_type::<Material>()
            .first()
            .expect("a Material");
        tester.widget(element, |material: &Material| material.color)
    }

    fn count_of<W: 'static>(tester: &WidgetTester) -> usize {
        tester.find_by_type::<W>().len()
    }

    #[test]
    fn the_theme_mode_getters_say_which_mode_it_is() {
        assert!(ThemeMode::System.is_system());
        assert!(!ThemeMode::System.is_light());
        assert!(!ThemeMode::System.is_dark());

        assert!(!ThemeMode::Light.is_system());
        assert!(ThemeMode::Light.is_light());
        assert!(!ThemeMode::Light.is_dark());

        assert!(!ThemeMode::Dark.is_system());
        assert!(!ThemeMode::Dark.is_light());
        assert!(ThemeMode::Dark.is_dark());
    }

    // 'Can nest apps'.
    #[test]
    fn apps_can_nest() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            pump(&mut tester, |_| {
                MaterialApp::new().home(MaterialApp::new().home(Text::new("Home sweet home")))
            });
            assert_eq!(found(&tester, "Home sweet home"), 1);
        });
    }

    // 'Do not rebuild page during a route transition'.
    #[test]
    fn a_pushed_page_builds_once_through_its_route_transition() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let build_counter = Rc::new(Cell::new(0));
            let next = count_page("Y", &build_counter);
            pump(&mut tester, |_| {
                MaterialApp::new()
                    .home(Builder::new(|_app, context| {
                        Material::new()
                            .child(ElevatedButton::new(
                                Some(Listener::new(move |app| {
                                    Navigator::push_named(app, context, "/next", None);
                                })),
                                Some(Text::new("X")),
                            ))
                            .into_widget()
                    }))
                    .routes([("/next".to_string(), next)])
            });

            assert_eq!(build_counter.get(), 0);
            let x = center_of_text(&tester, "X");
            tester.tap(x, 1);
            assert_eq!(build_counter.get(), 0);
            tester.pump(Duration::ZERO);
            assert_eq!(build_counter.get(), 1);
            for _ in 0..4 {
                tester.pump(Duration::from_millis(10));
                assert_eq!(build_counter.get(), 1);
            }
            tester.pump(Duration::from_secs(1));
            assert_eq!(build_counter.get(), 1);
            assert_eq!(onstage(&tester, "Y"), 1);
        });
    }

    // 'Do rebuild the home page if it changes'.
    #[test]
    fn the_home_page_rebuilds_when_it_changes() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let build_counter = Rc::new(Cell::new(0));
            let home = |text: &'static str| {
                let build_counter = Rc::clone(&build_counter);
                Builder::new(move |_app, _context| {
                    build_counter.set(build_counter.get() + 1);
                    Text::new(text).into_widget()
                })
            };
            let a = home("A");
            pump(&mut tester, |_| MaterialApp::new().home(a));
            assert_eq!(build_counter.get(), 1);
            assert_eq!(found(&tester, "A"), 1);
            let b = home("B");
            pump(&mut tester, |_| MaterialApp::new().home(b));
            assert_eq!(build_counter.get(), 2);
            assert_eq!(found(&tester, "B"), 1);
        });
    }

    // 'Do not rebuild the home page if it does not actually change'.
    #[test]
    fn the_home_page_does_not_rebuild_when_it_is_the_same_widget() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let build_counter = Rc::new(Cell::new(0));
            let home: WidgetRef = {
                let build_counter = Rc::clone(&build_counter);
                Builder::new(move |_app, _context| {
                    build_counter.set(build_counter.get() + 1);
                    SizedBox::new().into_widget()
                })
                .into_widget()
            };
            pump(&mut tester, |_| MaterialApp::new().home(home.clone()));
            assert_eq!(build_counter.get(), 1);
            pump(&mut tester, |_| MaterialApp::new().home(home.clone()));
            assert_eq!(build_counter.get(), 1);
        });
    }

    // 'Do rebuild pages that come from the routes table if the MaterialApp changes'.
    #[test]
    fn pages_from_the_routes_table_rebuild_when_the_app_changes() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let build_counter = Rc::new(Cell::new(0));
            let routes = vec![("/".to_string(), count_page("page", &build_counter))];
            pump(&mut tester, |_| MaterialApp::new().routes(routes.clone()));
            assert_eq!(build_counter.get(), 1);
            pump(&mut tester, |_| MaterialApp::new().routes(routes.clone()));
            assert_eq!(build_counter.get(), 2);
        });
    }

    // 'Cannot pop the initial route'.
    #[test]
    fn the_initial_route_cannot_be_popped() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            pump(&mut tester, |_| MaterialApp::new().home(Text::new("Home")));
            assert_eq!(found(&tester, "Home"), 1);

            let navigator = navigator(&tester);
            let result = navigator.maybe_pop(&mut tester.cell.borrow_mut(), None);

            assert!(!result);
            assert_eq!(found(&tester, "Home"), 1);
        });
    }

    // 'Default initialRoute'.
    #[test]
    fn the_default_initial_route_is_the_slash_route() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            pump(&mut tester, |_| {
                MaterialApp::new().routes(route_table(&["/"]))
            });
            assert_eq!(onstage(&tester, "route \"/\""), 1);
        });
    }

    // 'One-step initial route'.
    #[test]
    fn a_one_step_initial_route_is_pushed_over_the_slash_route() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            pump(&mut tester, |_| {
                MaterialApp::new()
                    .initial_route("/a")
                    .routes(route_table(&["/", "/a", "/a/b", "/b"]))
            });
            assert_eq!(found(&tester, "route \"/\""), 1);
            assert_eq!(onstage(&tester, "route \"/a\""), 1);
            assert_eq!(found(&tester, "route \"/a/b\""), 0);
            assert_eq!(found(&tester, "route \"/b\""), 0);
        });
    }

    // 'Two-step initial route'.
    #[test]
    fn a_two_step_initial_route_is_pushed_over_each_of_its_parents() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            pump(&mut tester, |_| {
                MaterialApp::new()
                    .initial_route("/a/b")
                    .routes(route_table(&["/", "/a", "/a/b", "/b"]))
            });
            assert_eq!(found(&tester, "route \"/\""), 1);
            assert_eq!(found(&tester, "route \"/a\""), 1);
            assert_eq!(onstage(&tester, "route \"/a/b\""), 1);
            assert_eq!(found(&tester, "route \"/b\""), 0);
        });
    }

    // 'Make sure initialRoute is only used the first time'.
    #[test]
    fn the_initial_route_is_only_used_the_first_time() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let routes = route_table(&["/", "/a", "/b"]);
            let expect_a = |tester: &WidgetTester| {
                assert_eq!(found(tester, "route \"/\""), 1);
                assert_eq!(onstage(tester, "route \"/a\""), 1);
                assert_eq!(found(tester, "route \"/b\""), 0);
            };

            pump(&mut tester, |_| {
                MaterialApp::new()
                    .initial_route("/a")
                    .routes(routes.clone())
            });
            expect_a(&tester);

            // changing initialRoute has no effect
            pump(&mut tester, |_| {
                MaterialApp::new()
                    .initial_route("/b")
                    .routes(routes.clone())
            });
            expect_a(&tester);

            // removing it has no effect
            pump(&mut tester, |_| MaterialApp::new().routes(routes.clone()));
            expect_a(&tester);
        });
    }

    // 'MaterialApp with builder and no route information works.' and 'MaterialApp.builder can
    // build app without a Navigator'.
    #[test]
    fn a_builder_without_routes_builds_the_app_without_a_navigator() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let builder_child = Rc::new(Cell::new(true));
            let seen = Rc::clone(&builder_child);
            pump(&mut tester, |_| {
                MaterialApp::new().builder(move |_app, _context, child| {
                    seen.set(child.is_some());
                    SizedBox::new().into_widget()
                })
            });
            assert!(!builder_child.get());
            assert_eq!(count_of::<Navigator>(&tester), 0);
        });
    }

    // 'MaterialApp with builder works when themeAnimationStyle is AnimationStyle.noAnimation'.
    #[test]
    fn a_builder_works_without_the_theme_animation() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            pump(&mut tester, |_| {
                MaterialApp::new()
                    .theme_animation_style(AnimationStyle::NO_ANIMATION)
                    .builder(|_app, _context, _child| Text::new("Works").into_widget())
            });
            assert_eq!(found(&tester, "Works"), 1);
        });
    }

    // 'MaterialApp.navigatorKey' and 'MaterialApp.navigatorKey can be updated'.
    #[test]
    fn the_navigator_key_names_the_navigator_while_the_app_is_given_it() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let key = GlobalKey::new();
            let color = Color::new(0xFF112233);
            let current = |tester: &WidgetTester, key: &GlobalKey| {
                key.current_state::<NavigatorState>(&tester.cell.borrow_mut())
            };

            pump(&mut tester, |_| {
                MaterialApp::new()
                    .navigator_key(key.clone())
                    .color(color)
                    .home(SizedBox::new())
            });
            assert!(current(&tester, &key).is_some());
            pump(&mut tester, |_| {
                MaterialApp::new().color(color).home(SizedBox::new())
            });
            assert!(current(&tester, &key).is_none());
            pump(&mut tester, |_| {
                MaterialApp::new()
                    .navigator_key(key.clone())
                    .color(color)
                    .home(SizedBox::new())
            });
            assert!(current(&tester, &key).is_some());

            let key2 = GlobalKey::new();
            pump(&mut tester, |_| {
                MaterialApp::new()
                    .navigator_key(key2.clone())
                    .home(SizedBox::new())
            });
            assert!(current(&tester, &key2).is_some());
            assert!(current(&tester, &key).is_none());
        });
    }

    #[test]
    fn the_scaffold_messenger_key_names_the_apps_scaffold_messenger() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let key = GlobalKey::new();
            pump(&mut tester, |_| {
                MaterialApp::new()
                    .scaffold_messenger_key(key.clone())
                    .home(SizedBox::new())
            });
            assert!(
                key.current_state::<ScaffoldMessengerState>(&tester.cell.borrow_mut())
                    .is_some()
            );
        });
    }

    // 'Has default material and cupertino localizations'.
    #[test]
    fn the_app_has_default_material_and_cupertino_localizations() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let seen = Rc::new(Cell::new(None));
            let probe = context_probe(&seen);
            pump(&mut tester, |_| MaterialApp::new().home(probe));
            let context = seen.get().expect("the probe built");
            let mut app = tester.cell.borrow_mut();
            // Default US "select all" text.
            assert_eq!(
                <dyn MaterialLocalizations>::of(&mut app, context).select_all_button_label(),
                "Select all"
            );
            // Default Cupertino US "select all" text.
            assert_eq!(
                <dyn CupertinoLocalizations>::of(&mut app, context).select_all_button_label(),
                "Select All"
            );
            assert!(Localizations::of::<dyn WidgetsLocalizations>(&mut app, context).is_some());
        });
    }

    /// The brightness of the theme a page sees under `app`, on a host reporting
    /// `platform_brightness`.
    fn applied_brightness(
        platform_brightness: Brightness,
        app: impl FnOnce(&App, Builder) -> MaterialApp,
    ) -> Brightness {
        let mut tester = tester_with_brightness(platform_brightness);
        let seen = Rc::new(RefCell::new(None));
        let probe = theme_probe(&seen);
        pump(&mut tester, |cell_app| app(cell_app, probe));
        applied(&seen).brightness()
    }

    // 'MaterialApp uses regular theme when themeMode is light'.
    #[test]
    fn a_light_theme_mode_uses_the_regular_theme() {
        for platform_brightness in [Brightness::Light, Brightness::Dark] {
            with_main_thread_stack(|| {
                let brightness = applied_brightness(platform_brightness, |app, probe| {
                    MaterialApp::new()
                        .theme(brightness_theme(app, Brightness::Light))
                        .dark_theme(brightness_theme(app, Brightness::Dark))
                        .theme_mode(ThemeMode::Light)
                        .home(probe)
                });
                assert_eq!(brightness, Brightness::Light);
            });
        }
    }

    // 'MaterialApp uses darkTheme when themeMode is dark'.
    #[test]
    fn a_dark_theme_mode_uses_the_dark_theme() {
        for platform_brightness in [Brightness::Light, Brightness::Dark] {
            with_main_thread_stack(|| {
                let brightness = applied_brightness(platform_brightness, |app, probe| {
                    MaterialApp::new()
                        .theme(brightness_theme(app, Brightness::Light))
                        .dark_theme(brightness_theme(app, Brightness::Dark))
                        .theme_mode(ThemeMode::Dark)
                        .home(probe)
                });
                assert_eq!(brightness, Brightness::Dark);
            });
        }
    }

    // 'MaterialApp uses regular theme when themeMode is system and platformBrightness is light',
    // 'MaterialApp uses darkTheme when themeMode is system and platformBrightness is dark' and
    // 'MaterialApp uses dark theme when platformBrightness is dark'.
    #[test]
    fn the_system_theme_mode_follows_the_platform_brightness() {
        for platform_brightness in [Brightness::Light, Brightness::Dark] {
            with_main_thread_stack(|| {
                let brightness = applied_brightness(platform_brightness, |app, probe| {
                    MaterialApp::new()
                        .theme(brightness_theme(app, Brightness::Light))
                        .dark_theme(brightness_theme(app, Brightness::Dark))
                        .home(probe)
                });
                assert_eq!(brightness, platform_brightness);
            });
        }
    }

    // 'MaterialApp uses light theme when platformBrightness is dark but no dark theme is
    // provided'.
    #[test]
    fn a_dark_platform_without_a_dark_theme_uses_the_regular_theme() {
        with_main_thread_stack(|| {
            let brightness = applied_brightness(Brightness::Dark, |app, probe| {
                MaterialApp::new()
                    .theme(brightness_theme(app, Brightness::Light))
                    .home(probe)
            });
            assert_eq!(brightness, Brightness::Light);
        });
    }

    // 'MaterialApp uses fallback light theme when platformBrightness is dark but no theme is
    // provided at all'.
    #[test]
    fn a_dark_platform_without_any_theme_uses_the_fallback_light_theme() {
        with_main_thread_stack(|| {
            let brightness =
                applied_brightness(Brightness::Dark, |_, probe| MaterialApp::new().home(probe));
            assert_eq!(brightness, Brightness::Light);
        });
    }

    // 'MaterialApp uses fallback light theme when platformBrightness is light and a dark theme
    // is provided'.
    #[test]
    fn a_light_platform_with_only_a_dark_theme_uses_the_fallback_light_theme() {
        with_main_thread_stack(|| {
            let brightness = applied_brightness(Brightness::Light, |app, probe| {
                MaterialApp::new()
                    .dark_theme(brightness_theme(app, Brightness::Dark))
                    .home(probe)
            });
            assert_eq!(brightness, Brightness::Light);
        });
    }

    /// The primary colour of the theme a page sees under `app`, on a host reporting
    /// `platform_brightness`, under a `MediaQuery` that asks for high contrast.
    fn applied_high_contrast_primary(
        platform_brightness: Brightness,
        app: impl FnOnce(&App, Builder) -> MaterialApp,
    ) -> Color {
        let mut tester = tester_with_brightness(platform_brightness);
        let seen = Rc::new(RefCell::new(None));
        let probe = theme_probe(&seen);
        let material_app: WidgetRef = app(&tester.cell.borrow(), probe).into_widget();
        let root = Builder::new(move |app, context| {
            MediaQuery::new(
                MediaQuery::of(app, context).high_contrast(true),
                material_app.clone(),
            )
            .into_widget()
        });
        tester.pump_widget(root.into_widget());
        applied(&seen).primary_color
    }

    // 'MaterialApp uses high contrast theme when appropriate'.
    #[test]
    fn high_contrast_uses_the_high_contrast_theme() {
        with_main_thread_stack(|| {
            let primary = applied_high_contrast_primary(Brightness::Light, |app, probe| {
                MaterialApp::new()
                    .theme(primary_theme(app, Colors::LIGHT_BLUE.primary()))
                    .high_contrast_theme(primary_theme(app, Colors::BLUE.primary()))
                    .home(probe)
            });
            assert_eq!(primary, Colors::BLUE.primary());
        });
    }

    // 'MaterialApp uses high contrast dark theme when appropriate'.
    #[test]
    fn high_contrast_on_a_dark_platform_uses_the_high_contrast_dark_theme() {
        with_main_thread_stack(|| {
            let primary = applied_high_contrast_primary(Brightness::Dark, |app, probe| {
                MaterialApp::new()
                    .theme(primary_theme(app, Colors::LIGHT_BLUE.primary()))
                    .dark_theme(primary_theme(app, Colors::LIGHT_GREEN.primary()))
                    .high_contrast_theme(primary_theme(app, Colors::BLUE.primary()))
                    .high_contrast_dark_theme(primary_theme(app, Colors::GREEN.primary()))
                    .home(probe)
            });
            assert_eq!(primary, Colors::GREEN.primary());
        });
    }

    // 'MaterialApp uses dark theme when no high contrast dark theme is provided'.
    #[test]
    fn high_contrast_on_a_dark_platform_without_a_high_contrast_dark_theme_uses_the_dark_theme() {
        with_main_thread_stack(|| {
            let primary = applied_high_contrast_primary(Brightness::Dark, |app, probe| {
                MaterialApp::new()
                    .theme(primary_theme(app, Colors::LIGHT_BLUE.primary()))
                    .dark_theme(primary_theme(app, Colors::LIGHT_GREEN.primary()))
                    .home(probe)
            });
            assert_eq!(primary, Colors::LIGHT_GREEN.primary());
        });
    }

    // 'MaterialApp animates theme changes'.
    #[test]
    fn the_app_animates_theme_changes() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let (light_theme, dark_theme) = {
                let app = tester.cell.borrow();
                (ThemeData::new(&app), ThemeData::dark(&app, None))
            };
            let scaffold = || Builder::new(|_app, _context| Scaffold::new().into_widget());
            let (light, dark) = (light_theme.clone(), dark_theme.clone());
            pump(&mut tester, |_| {
                MaterialApp::new()
                    .theme(light)
                    .dark_theme(dark)
                    .theme_mode(ThemeMode::Light)
                    .home(scaffold())
            });
            assert_eq!(
                material_color(&tester),
                Some(light_theme.scaffold_background_color)
            );

            // Change to dark theme
            pump(&mut tester, |app| {
                MaterialApp::new()
                    .dark_theme(ThemeData::dark(app, None))
                    .theme_mode(ThemeMode::Dark)
                    .home(scaffold())
            });

            // Wait half kThemeAnimationDuration = 200ms.
            tester.pump(Duration::from_millis(100));

            // Default curve is linear so background should be half way between
            // the two colors.
            let half_bg_color = Color::lerp(
                Some(light_theme.scaffold_background_color),
                Some(dark_theme.scaffold_background_color),
                0.5,
            );
            assert_eq!(material_color(&tester), half_bg_color);
        });
    }

    // 'MaterialApp theme animation can be turned off'.
    #[test]
    fn the_theme_animation_can_be_turned_off() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let (light_theme, dark_theme) = {
                let app = tester.cell.borrow();
                (ThemeData::new(&app), ThemeData::dark(&app, None))
            };
            let scaffold_rebuilds = Rc::new(Cell::new(0));
            let scaffold: WidgetRef = {
                let scaffold_rebuilds = Rc::clone(&scaffold_rebuilds);
                Builder::new(move |app, context| {
                    scaffold_rebuilds.set(scaffold_rebuilds.get() + 1);
                    // Use Theme.of() to ensure we are building when the theme changes.
                    Scaffold::new()
                        .background_color(Theme::of(app, context).scaffold_background_color)
                        .into_widget()
                })
                .into_widget()
            };

            let (light, dark) = (light_theme.clone(), dark_theme.clone());
            pump(&mut tester, |_| {
                MaterialApp::new()
                    .theme(light)
                    .dark_theme(dark)
                    .theme_mode(ThemeMode::Light)
                    .theme_animation_duration(Duration::ZERO)
                    .home(scaffold.clone())
            });
            assert_eq!(
                material_color(&tester),
                Some(light_theme.scaffold_background_color)
            );
            assert_eq!(scaffold_rebuilds.get(), 1);

            // Change to dark theme
            pump(&mut tester, |app| {
                MaterialApp::new()
                    .dark_theme(ThemeData::dark(app, None))
                    .theme_mode(ThemeMode::Dark)
                    .theme_animation_duration(Duration::ZERO)
                    .home(scaffold.clone())
            });

            // Wait for any animation to finish.
            tester.pump_and_settle();
            assert_eq!(
                material_color(&tester),
                Some(dark_theme.scaffold_background_color)
            );
            assert_eq!(scaffold_rebuilds.get(), 2);
        });
    }

    // 'MaterialApp switches themes when the platformBrightness changes.'
    #[test]
    fn the_app_switches_themes_when_the_platform_brightness_changes() {
        with_main_thread_stack(|| {
            let (mut tester, platform) = WidgetTester::sharing(
                TestPlatform::new()
                    .on(TargetPlatform::Android)
                    .with_brightness(Brightness::Light),
            );
            tester.install_fonts();
            let before: Rc<RefCell<Option<Brightness>>> = Rc::new(RefCell::new(None));
            let after: Rc<RefCell<Option<Brightness>>> = Rc::new(RefCell::new(None));
            let probe = {
                let (before, after) = (Rc::clone(&before), Rc::clone(&after));
                Builder::new(move |app, context| {
                    let brightness = Some(Theme::of(app, context).brightness());
                    if before.borrow().is_none() {
                        *before.borrow_mut() = brightness;
                    } else {
                        *after.borrow_mut() = brightness;
                    }
                    SizedBox::new().into_widget()
                })
            };
            pump(&mut tester, |app| {
                MaterialApp::new()
                    .theme(brightness_theme(app, Brightness::Light))
                    .dark_theme(brightness_theme(app, Brightness::Dark))
                    .home(probe)
            });

            // Switch the platformBrightness from light to dark and pump the widget tree
            // to process changes.
            platform.set_brightness(Brightness::Dark);
            {
                let mut app = tester.cell.borrow_mut();
                WidgetsBinding::instance(&mut app).handle_platform_brightness_changed(&mut app);
            }
            tester.pump_and_settle();

            assert_eq!(*before.borrow(), Some(Brightness::Light));
            assert_eq!(*after.borrow(), Some(Brightness::Dark));
        });
    }

    // 'MaterialApp can customize initial routes'.
    #[test]
    fn on_generate_initial_routes_replaces_the_initial_routes() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let navigator_key = GlobalKey::new();
            let page = |text: &'static str| {
                move |app: &mut App| {
                    let page_builder: waft_widgets::RoutePageBuilder =
                        Rc::new(move |_app, _context, _animation, _secondary| {
                            Text::new(text).into_widget()
                        });
                    waft_widgets::Route::as_route(PageRouteBuilder::new(app, page_builder))
                }
            };
            pump(&mut tester, |_| {
                MaterialApp::new()
                    .navigator_key(navigator_key.clone())
                    .on_generate_initial_routes(move |app, initial_route| {
                        assert_eq!(initial_route, "/abc");
                        vec![
                            page("non-regular page one")(app),
                            page("non-regular page two")(app),
                        ]
                    })
                    .initial_route("/abc")
                    .routes([
                        ("/".to_string(), text_page("regular page one")),
                        ("/abc".to_string(), text_page("regular page two")),
                    ])
            });
            assert_eq!(onstage(&tester, "non-regular page two"), 1);
            assert_eq!(onstage(&tester, "non-regular page one"), 0);
            assert_eq!(onstage(&tester, "regular page one"), 0);
            assert_eq!(onstage(&tester, "regular page two"), 0);
            {
                let mut app = tester.cell.borrow_mut();
                let navigator = navigator_key
                    .current_state::<NavigatorState>(&app)
                    .expect("the navigator");
                navigator.pop(&mut app, None);
            }
            tester.pump_and_settle();
            assert_eq!(onstage(&tester, "non-regular page two"), 0);
            assert_eq!(onstage(&tester, "non-regular page one"), 1);
            assert_eq!(onstage(&tester, "regular page one"), 0);
            assert_eq!(onstage(&tester, "regular page two"), 0);
        });
    }

    // 'MaterialApp does create HeroController with the MaterialRectArcTween', read by what the
    // tween answers, since a `RectTweenObject` has no type to test.
    #[test]
    fn the_hero_controller_tweens_rects_along_a_material_arc() {
        let tester = tester_on(TargetPlatform::Android);
        let mut app = tester.cell.borrow_mut();
        let controller = MaterialApp::create_material_hero_controller(&mut app);
        let begin = Some(Rect::from_ltrb(0.0, 0.0, 10.0, 10.0));
        let end = Some(Rect::from_ltrb(0.0, 0.0, 20.0, 20.0));
        let create_rect_tween = app
            .get(controller)
            .create_rect_tween
            .clone()
            .expect("the controller has a rect tween factory");
        let tween = create_rect_tween(&mut app, begin, end);
        let arc = MaterialRectArcTween::new(begin, end);
        for t in [0.25, 0.5, 0.75] {
            assert_eq!(tween.lerp(&app, t), Some(arc.lerp(t)));
        }
        assert_ne!(
            tween.lerp(&app, 0.5),
            Some(Rect::from_ltrb(0.0, 0.0, 15.0, 15.0)),
            "not a linear RectTween"
        );
        controller.dispose(&mut app);
    }

    /// The scroll configuration a page sees under `app`, and the page's context.
    fn page_scroll_behavior(
        tester: &mut WidgetTester,
        app: impl FnOnce(Builder) -> MaterialApp,
    ) -> (ScrollBehaviorRef, BuildContext) {
        let seen = Rc::new(Cell::new(None));
        let probe = context_probe(&seen);
        pump(tester, |_| app(probe));
        let context = seen.get().expect("the probe built");
        let behavior = ScrollConfiguration::of(&mut tester.cell.borrow_mut(), context);
        (behavior, context)
    }

    // 'MaterialApp has correct default ScrollBehavior' and 'MaterialApp has correct default
    // KeyboardDismissBehavior'.
    #[test]
    fn the_default_scroll_behavior_is_a_material_scroll_behavior() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let (behavior, context) =
                page_scroll_behavior(&mut tester, |probe| MaterialApp::new().home(probe));
            assert_eq!(behavior.type_id(), TypeId::of::<MaterialScrollBehavior>());
            assert_eq!(
                behavior.get_keyboard_dismiss_behavior(&tester.cell.borrow(), context),
                ScrollViewKeyboardDismissBehavior::Manual
            );
        });
    }

    // 'MaterialApp can override default KeyboardDismissBehavior'.
    #[test]
    fn the_keyboard_dismiss_behavior_can_be_overridden() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let (behavior, context) = page_scroll_behavior(&mut tester, |probe| {
                MaterialApp::new()
                    .scroll_behavior(ScrollBehaviorRef::new(
                        ScrollBehaviorRef::new(MaterialScrollBehavior::new())
                            .copy_with()
                            .keyboard_dismiss_behavior(ScrollViewKeyboardDismissBehavior::OnDrag),
                    ))
                    .home(probe)
            });
            assert_eq!(
                behavior.get_keyboard_dismiss_behavior(&tester.cell.borrow(), context),
                ScrollViewKeyboardDismissBehavior::OnDrag
            );
        });
    }

    /// Dart's `MockScrollBehavior`.
    #[derive(Debug)]
    struct MockScrollBehavior;

    impl ScrollBehavior for MockScrollBehavior {
        fn as_any(&self) -> &dyn Any {
            self
        }

        fn get_scroll_physics(&self, _app: &App, _context: BuildContext) -> ScrollPhysicsRef {
            Rc::new(NeverScrollableScrollPhysics::new())
        }
    }

    // 'A ScrollBehavior can be set for MaterialApp'.
    #[test]
    fn a_scroll_behavior_can_be_set_for_the_app() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let (behavior, context) = page_scroll_behavior(&mut tester, |probe| {
                MaterialApp::new()
                    .scroll_behavior(ScrollBehaviorRef::new(MockScrollBehavior))
                    .home(probe)
            });
            assert_eq!(behavior.type_id(), TypeId::of::<MockScrollBehavior>());
            let physics = behavior.get_scroll_physics(&tester.cell.borrow(), context);
            assert!(
                physics
                    .as_any()
                    .downcast_ref::<NeverScrollableScrollPhysics>()
                    .is_some()
            );
        });
    }

    fn list_view() -> ListView {
        ListView::new().children([SizedBox::new()
            .height(1000.0)
            .width(1000.0)
            .child(Text::new("Test"))
            .into_widget()])
    }

    /// How many stretching and glowing indicators an Android `ListView` gets under `app`.
    fn overscroll_indicators(
        app: impl FnOnce(&App, ListView) -> MaterialApp + Send,
    ) -> (usize, usize) {
        let mut result = (0, 0);
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            pump(&mut tester, |cell_app| app(cell_app, list_view()));
            result = (
                count_of::<StretchingOverscrollIndicator>(&tester),
                count_of::<GlowingOverscrollIndicator>(&tester),
            );
        });
        result
    }

    // 'Material2 - MaterialApp provides default overscroll color'. The painter is the glow's
    // own `CustomPaint`: Dart's first `CustomPaint` is that one because flutter_test turns the
    // debug banner, whose `CustomPaint` is above it, off.
    #[test]
    fn material_2_glows_in_the_color_scheme_secondary() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            // The overscroll color should be a transparent version of the colorScheme's
            // secondary color.
            let secondary_color = Color::new(0xff008800);
            #[allow(deprecated)]
            let glow_secondary_color = secondary_color.with_opacity(0.05);
            pump(&mut tester, |app| {
                let theme =
                    ThemeData::from(crate::ColorScheme::light().with_secondary(secondary_color))
                        .use_material3(false)
                        .build(app);
                MaterialApp::new().theme(theme).home(
                    waft_widgets::SingleChildScrollView::new()
                        .child(SizedBox::new().height(2000.0)),
                )
            });

            let glow = *tester
                .find_by_type::<GlowingOverscrollIndicator>()
                .first()
                .expect("a glowing indicator");
            let custom_paint = tester
                .find_by_type::<waft_widgets::CustomPaint>()
                .into_iter()
                .find(|custom_paint| tester.ancestors(*custom_paint).contains(&glow))
                .expect("the glow paints through a CustomPaint");
            let painter = tester.render_box_of(custom_paint).as_object();
            let mut gesture = WidgetTester::touch(1);
            tester.down(&mut gesture, Offset::new(200.0, 200.0));
            for _ in 0..10 {
                let at = gesture.position + Offset::new(0.0, 5.0);
                tester.move_to(&mut gesture, at, true);
                tester.pump(Duration::from_millis(20));
            }
            tester.up(&gesture);
            assert!(
                tester.painted(painter).iter().any(|drawn| matches!(
                    drawn,
                    crate::test_support::Drawn::Path(color, _)
                        if *color == glow_secondary_color.to_argb32()
                )),
                "the glow paints a circle in the secondary colour"
            );
        });
    }

    // 'Material3 - ListView clip behavior updates overscroll indicator clip behavior'.
    #[test]
    fn the_stretch_clips_as_the_list_view_does() {
        use waft_rendering::{RenderClipRect, RenderCustomClip};

        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let build_frame = |clip_behavior: Clip| {
                MaterialApp::new()
                    .home(
                        waft_widgets::Column::new().children([
                            SizedBox::new()
                                .height(300.0)
                                .child(
                                    ListView::builder(|_app, _context, index| {
                                        Some(
                                            waft_widgets::Padding::new(
                                                waft_painting::EdgeInsetsGeometry::all(10.0),
                                            )
                                            .child(Text::new(format!("Index {index}")))
                                            .into_widget(),
                                        )
                                    })
                                    .item_count(20)
                                    .build()
                                    .clip_behavior(clip_behavior),
                                )
                                .into_widget(),
                            waft_widgets::Opacity::new(0.5)
                                .child(
                                    waft_widgets::Container::new()
                                        .color(Color::new(0xD0FF0000))
                                        .height(100.0),
                                )
                                .into_widget(),
                        ]),
                    )
                    .into_widget()
            };
            let clip_of = |tester: &WidgetTester| {
                let render_clip = tester.first::<RenderClipRect>();
                render_clip.clip_behavior(&tester.cell.borrow())
            };
            let overscroll_the_start = |tester: &mut WidgetTester, expected: Clip| {
                let mut gesture = WidgetTester::touch(1);
                let center = center_of_text(tester, "Index 1");
                tester.down(&mut gesture, center);
                // Overscroll the start.
                let at = gesture.position + Offset::new(0.0, 200.0);
                tester.move_to(&mut gesture, at, true);
                tester.pump_and_settle();
                assert_eq!(found(tester, "Index 1"), 1);
                assert!(center_of_text(tester, "Index 1").dy() > 0.0);
                // Now clipping
                assert_eq!(clip_of(tester), expected);

                tester.up(&gesture);
                tester.pump_and_settle();
            };

            // Test default clip behavior.
            tester.pump_widget(build_frame(Clip::HardEdge));

            assert_eq!(count_of::<StretchingOverscrollIndicator>(&tester), 1);
            assert_eq!(count_of::<GlowingOverscrollIndicator>(&tester), 0);
            assert_eq!(found(&tester, "Index 1"), 1);

            // Currently not clipping
            assert_eq!(clip_of(&tester), Clip::None);
            overscroll_the_start(&mut tester, Clip::HardEdge);

            // Test custom clip behavior.
            tester.pump_widget(build_frame(Clip::None));

            // Currently not clipping
            assert_eq!(clip_of(&tester), Clip::None);
            overscroll_the_start(&mut tester, Clip::None);
        });
    }

    // 'Material2 - ScrollBehavior default android overscroll indicator'.
    #[test]
    fn material_2_glows_on_android() {
        let (stretching, glowing) = overscroll_indicators(|app, list| {
            MaterialApp::new()
                .theme(ThemeData::builder().use_material3(false).build(app))
                .scroll_behavior(ScrollBehaviorRef::new(MaterialScrollBehavior::new()))
                .home(list)
        });
        assert_eq!((stretching, glowing), (0, 1));
    }

    // 'Material3 - ScrollBehavior default android overscroll indicator'.
    #[test]
    fn material_3_stretches_on_android() {
        let (stretching, glowing) = overscroll_indicators(|_, list| {
            MaterialApp::new()
                .scroll_behavior(ScrollBehaviorRef::new(MaterialScrollBehavior::new()))
                .home(list)
        });
        assert_eq!((stretching, glowing), (1, 0));
    }

    // 'MaterialScrollBehavior default stretch android overscroll indicator'.
    #[test]
    fn the_default_behavior_stretches_on_android() {
        let (stretching, glowing) = overscroll_indicators(|_, list| MaterialApp::new().home(list));
        assert_eq!((stretching, glowing), (1, 0));
    }

    // 'Overscroll indicator can be set by theme'.
    #[test]
    fn the_theme_sets_the_overscroll_indicator() {
        let (stretching, glowing) = overscroll_indicators(|app, list| {
            // The current default is M3 and stretch overscroll, setting via the theme should override.
            #[allow(deprecated)]
            let theme = ThemeData::new(app).copy_with().use_material3(false).build();
            MaterialApp::new().theme(theme).home(list)
        });
        assert_eq!((stretching, glowing), (0, 1));
    }

    /// A page context under a `MaterialApp` on `platform`, inside a scroll view scrolling
    /// `axis` whose behavior has no scrollbars.
    fn scroll_view_context(tester: &mut WidgetTester, axis: Axis) -> BuildContext {
        let seen = Rc::new(Cell::new(None));
        let probe = {
            let seen = Rc::clone(&seen);
            Builder::new(move |_app, context| {
                seen.set(Some(context));
                SizedBox::new().height(1000.0).into_widget()
            })
        };
        pump(tester, |_| {
            MaterialApp::new().home(ScrollConfiguration::new(
                // Avoid the default ones here.
                ScrollBehaviorRef::new(
                    ScrollBehaviorRef::new(MaterialScrollBehavior::new())
                        .copy_with()
                        .scrollbars(false),
                ),
                waft_widgets::SingleChildScrollView::new()
                    .scroll_direction(axis)
                    .child(probe),
            ))
        });
        seen.get().expect("the probe built")
    }

    /// Whether `build_scrollbar` over `details` panics with the controller assert.
    fn build_scrollbar_asserts(
        tester: &WidgetTester,
        context: BuildContext,
        details: &ScrollableDetails,
    ) -> bool {
        let mut app = tester.cell.borrow_mut();
        let child: WidgetRef = SizedBox::new().into_widget();
        let built = catch_unwind(AssertUnwindSafe(|| {
            MaterialScrollBehavior::new().build_scrollbar(&mut app, context, child, details)
        }));
        match built {
            Ok(_) => false,
            Err(panic) => {
                let message = panic
                    .downcast_ref::<&str>()
                    .map(|message| message.to_string())
                    .or_else(|| panic.downcast_ref::<String>().cloned())
                    .unwrap_or_default();
                assert!(
                    message.contains("details.controller.is_some()"),
                    "{message}"
                );
                true
            }
        }
    }

    // 'Assert in buildScrollbar that controller != null when using it (vertical)'.
    #[test]
    fn a_vertical_scrollbar_asserts_its_controller_on_desktop() {
        for platform in ALL_PLATFORMS {
            with_main_thread_stack(|| {
                let mut tester = tester_on(platform);
                let context = scroll_view_context(&mut tester, Axis::Vertical);
                let details = ScrollableDetails::vertical(false);
                let asserts = build_scrollbar_asserts(&tester, context, &details);
                match platform {
                    TargetPlatform::Android | TargetPlatform::Fuchsia | TargetPlatform::IOS => {
                        // Does not throw if we aren't using it.
                        assert!(!asserts, "{platform:?}");
                    }
                    TargetPlatform::Linux | TargetPlatform::MacOS | TargetPlatform::Windows => {
                        assert!(asserts, "{platform:?}");
                    }
                }
            });
        }
    }

    // 'Assert in buildScrollbar that controller != null when using it (horizontal)'.
    #[test]
    fn a_horizontal_scroll_view_gets_no_scrollbar_on_any_platform() {
        for platform in ALL_PLATFORMS {
            with_main_thread_stack(|| {
                let mut tester = tester_on(platform);
                let context = scroll_view_context(&mut tester, Axis::Horizontal);
                let details = ScrollableDetails::horizontal(true);
                // Does not throw if we aren't using it.
                // Horizontal axis gets no scrollbars for all platforms.
                assert!(
                    !build_scrollbar_asserts(&tester, context, &details),
                    "{platform:?}"
                );
            });
        }
    }

    #[test]
    fn a_vertical_scroll_view_gets_a_material_scrollbar_on_desktop_only() {
        for platform in ALL_PLATFORMS {
            with_main_thread_stack(|| {
                let mut tester = tester_on(platform);
                let context = scroll_view_context(&mut tester, Axis::Vertical);
                let mut app = tester.cell.borrow_mut();
                let controller = ScrollController::default(&mut app).as_controller();
                let details = ScrollableDetails::vertical(false).controller(controller);
                let child: WidgetRef = SizedBox::new().into_widget();
                let built = MaterialScrollBehavior::new().build_scrollbar(
                    &mut app,
                    context,
                    child.clone(),
                    &details,
                );
                let desktop = matches!(
                    platform,
                    TargetPlatform::Linux | TargetPlatform::MacOS | TargetPlatform::Windows
                );
                assert_eq!(
                    downcast_widget::<Scrollbar>(&*built).is_some(),
                    desktop,
                    "{platform:?}"
                );
                assert_eq!(!Rc::ptr_eq(&built, &child), desktop, "{platform:?}");
            });
        }
    }

    // 'Override theme animation using AnimationStyle'.
    #[test]
    fn the_theme_animation_style_overrides_the_theme_animation() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let (light_theme, dark_theme) = {
                let app = tester.cell.borrow();
                (ThemeData::new(&app), ThemeData::dark(&app, None))
            };
            let build_widget = |theme_mode: ThemeMode, animation_style: Option<AnimationStyle>| {
                let mut material_app = MaterialApp::new()
                    .theme(light_theme.clone())
                    .dark_theme(dark_theme.clone())
                    .theme_mode(theme_mode)
                    .home(Scaffold::new().body(Text::new("body")));
                material_app.theme_animation_style = animation_style;
                material_app.into_widget()
            };
            let surface_of = |tester: &WidgetTester| material_color(tester).expect("a colour");

            // Test the initial Scaffold background color.
            tester.pump_widget(build_widget(ThemeMode::Light, None));

            assert!(is_same_color_as(
                surface_of(&tester),
                light_theme.color_scheme.surface
            ));

            // Test the Scaffold background color animation from light to dark theme.
            tester.pump_widget(build_widget(ThemeMode::Dark, None));
            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_millis(50)); // Advance animation by 50 milliseconds.

            // Scaffold background color is slightly updated.
            assert!(is_same_color_as(
                surface_of(&tester),
                Color::new(0xffc3bdc5)
            ));

            // Let the animation finish.
            tester.pump_and_settle();

            // Scaffold background color is fully updated to dark theme.
            assert!(is_same_color_as(
                surface_of(&tester),
                dark_theme.color_scheme.surface
            ));

            // Reset to light theme to compare the Scaffold background color animation
            // with the default animation curve.
            tester.pump_widget(build_widget(ThemeMode::Light, None));
            tester.pump_and_settle();

            // Switch to dark theme with overridden animation curve.
            tester.pump_widget(build_widget(
                ThemeMode::Dark,
                Some(AnimationStyle::new(
                    Some(Curves::ease_in()),
                    None,
                    None,
                    None,
                )),
            ));
            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_millis(50));

            // Scaffold background color is slightly updated but with a different
            // color than the default animation curve.
            assert!(is_same_color_as(
                surface_of(&tester),
                Color::new(0xffe7e1e9)
            ));

            // Let the animation finish.
            tester.pump_and_settle();

            // Scaffold background color is fully updated to dark theme.
            assert!(is_same_color_as(
                surface_of(&tester),
                dark_theme.color_scheme.surface
            ));

            // Switch from dark to light theme with overridden animation duration.
            tester.pump_widget(build_widget(
                ThemeMode::Light,
                Some(AnimationStyle::NO_ANIMATION),
            ));
            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_millis(1));

            assert_ne!(surface_of(&tester), dark_theme.color_scheme.surface);
            assert!(is_same_color_as(
                surface_of(&tester),
                light_theme.color_scheme.surface
            ));
        });
    }

    // 'AnimationStyle.noAnimation removes AnimatedTheme from the tree'.
    #[test]
    fn no_animation_removes_the_animated_theme_from_the_tree() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            pump(&mut tester, |_| {
                MaterialApp::new()
                    .theme_animation_style(AnimationStyle::default())
                    .home(SizedBox::new())
            });

            assert_eq!(count_of::<AnimatedTheme>(&tester), 1);
            assert_eq!(count_of::<Theme>(&tester), 1);

            pump(&mut tester, |_| {
                MaterialApp::new()
                    .theme_animation_style(AnimationStyle::NO_ANIMATION)
                    .home(SizedBox::new())
            });

            assert_eq!(count_of::<AnimatedTheme>(&tester), 0);
            assert_eq!(count_of::<Theme>(&tester), 1);
        });
    }

    fn material_app_size(tester: &WidgetTester) -> Size {
        let element: AnyElement = *tester
            .find_by_type::<MaterialApp>()
            .first()
            .expect("a MaterialApp");
        tester.render_box_of(element).size(&tester.cell.borrow())
    }

    // 'MaterialApp works in an unconstrained environment'.
    #[test]
    fn the_app_takes_its_homes_size_when_unconstrained() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let app = MaterialApp::new().home(SizedBox::new().width(123.0).height(456.0));
            tester.pump_widget(UnconstrainedBox::new().child(app).into_widget());

            assert_eq!(material_app_size(&tester), Size::new(123.0, 456.0));
        });
    }

    // 'MaterialApp does not crash at zero area'.
    #[test]
    fn the_app_does_not_crash_at_zero_area() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let app = MaterialApp::new().home(Text::new("X"));
            tester.pump_widget(
                Center::new()
                    .child(SizedBox::shrink().child(app))
                    .into_widget(),
            );
            assert_eq!(material_app_size(&tester), Size::ZERO);
        });
    }

    #[test]
    fn the_checked_mode_banner_shows_unless_turned_off() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            pump(&mut tester, |_| MaterialApp::new().home(SizedBox::new()));
            assert_eq!(count_of::<CheckedModeBanner>(&tester), 1);

            pump(&mut tester, |_| {
                MaterialApp::new()
                    .debug_show_checked_mode_banner(false)
                    .home(SizedBox::new())
            });
            assert_eq!(count_of::<CheckedModeBanner>(&tester), 0);
        });
    }

    #[test]
    fn the_default_selection_style_comes_from_the_color_scheme_primary() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let seen = Rc::new(Cell::new(None));
            let probe = context_probe(&seen);
            pump(&mut tester, |_| MaterialApp::new().home(probe));
            let context = seen.get().expect("the probe built");
            let mut app = tester.cell.borrow_mut();
            let primary = Theme::of(&mut app, context).color_scheme.primary;
            let style = DefaultSelectionStyle::of(&mut app, context);
            #[allow(deprecated)]
            let selection = primary.with_opacity(0.40);
            assert_eq!(
                style.selection_color.map(|color| color.color()),
                Some(selection)
            );
            assert_eq!(style.cursor_color.map(|color| color.color()), Some(primary));
        });
    }

    #[test]
    fn can_place_app_inside_focus_scope() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let focus_scope_node = FocusScopeNode::new(&mut tester.cell.borrow_mut());

            tester.pump_widget(
                FocusScope::new(MaterialApp::new().home(Text::new("Home")))
                    .autofocus(true)
                    .node(focus_scope_node)
                    .into_widget(),
            );

            assert_eq!(tester.find_onstage_text("Home").len(), 1);
            FocusNodeLeaf::dispose(focus_scope_node, &mut tester.cell.borrow_mut());
        });
    }

    /// Dart's `tester.tap(find.text(text))`.
    fn tap_text(tester: &mut WidgetTester, text: &str) {
        let element = *tester
            .find_onstage_text(text)
            .first()
            .expect("the text is onstage");
        let center = tester.global_rect(tester.render_box_of(element)).center();
        tester.tap(center, 1);
    }

    #[test]
    fn return_value_from_pop_is_correct() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let result: Rc<RefCell<Option<CompleterFuture<RouteResult>>>> = Rc::default();
            let pushed = Rc::clone(&result);
            pump(&mut tester, move |_| {
                MaterialApp::new()
                    .home(Builder::new(move |_app, context| {
                        let pushed = Rc::clone(&pushed);
                        Material::new()
                            .child(ElevatedButton::new(
                                Some(Listener::new(move |app| {
                                    *pushed.borrow_mut() =
                                        Some(Navigator::push_named(app, context, "/a", None));
                                })),
                                Some(Text::new("X")),
                            ))
                            .into_widget()
                    }))
                    .routes([(
                        "/a".to_string(),
                        Rc::new(|_app: &mut App, context: BuildContext| {
                            Material::new()
                                .child(ElevatedButton::new(
                                    Some(Listener::new(move |app| {
                                        Navigator::pop(
                                            app,
                                            context,
                                            Some(Rc::new("all done") as Rc<dyn Any>),
                                        );
                                    })),
                                    Some(Text::new("Y")),
                                ))
                                .into_widget()
                        }) as WidgetBuilder,
                    )])
            });
            tap_text(&mut tester, "X");
            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_secs(1));
            assert_eq!(tester.find_onstage_text("Y").len(), 1);
            tap_text(&mut tester, "Y");
            tester.pump(Duration::ZERO);

            let value = result
                .borrow()
                .as_ref()
                .expect("X pushed /a")
                .peek()
                .expect("the route completed");
            let value = value.expect("a result");
            assert_eq!(value.downcast_ref::<&str>(), Some(&"all done"));
        });
    }

    #[test]
    fn when_use_inherited_media_query_is_true_an_existing_media_query_is_used_if_one_is_available()
    {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::Android);
            let captured_context: Rc<Cell<Option<BuildContext>>> = Rc::default();
            let unique_key: KeyRef = KeyRef::Local(Rc::new(waft_foundation::UniqueKey::new()));
            let captured = Rc::clone(&captured_context);
            #[allow(deprecated)]
            let app = MaterialApp::new()
                .use_inherited_media_query(true)
                .builder(move |_app, context, _child| {
                    captured.set(Some(context));
                    // Dart's `Placeholder`, which Waft does not have; the case reads only the context.
                    SizedBox::new().into_widget()
                })
                .color(Color::new(0xFF123456));
            tester.pump_widget(
                MediaQuery::new(MediaQueryData::new(), app)
                    .key(unique_key.clone())
                    .into_widget(),
            );
            let context = captured_context.get().expect("the builder ran");
            let mut app = tester.cell.borrow_mut();
            let key = context
                .depend_on_inherited_widget_of_exact_type::<MediaQuery>(&mut app)
                .and_then(|media_query| media_query.key.clone());
            assert_eq!(key, Some(unique_key));
        });
    }
}
