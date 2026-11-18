//! A Fluent application: WinUI's theme resources and accent, its icon font, and its page
//! entrance, composed the way Flutter's `MaterialApp` and `CupertinoApp` compose
//! [`WidgetsApp`].
//!
//! [`FluentApp`] is the widget an app designed with WinUI's controls is built from. It builds a
//! [`WidgetsApp`] inside a [`ThemeScope`] for the theme chosen from the app's light and dark
//! themes and the system's brightness, installs the Fluent icon font, and navigates with
//! [`FluentPageRoute`].

use std::any::TypeId;
use std::collections::HashMap;
use std::fmt::{self, Debug};

use waft_embedder::{Brightness, Color, Locale, SystemUiOverlayStyle};
use waft_foundation::{App, Handle};
use waft_services::SystemChrome;
use waft_widgets::{
    ActionRef, BuildContext, GenerateAppTitle, GlobalKey, InitialRouteListFactory, IntoWidget,
    KeyRef, LocaleListResolutionCallback, LocaleResolutionCallback, LocalizationsDelegateRef,
    MediaQuery, NavigationNotification, NavigatorObserverRef, NotificationListenerCallback,
    RouteFactory, RouteSettingsRef, ShortcutMap, State, StateData, StatefulWidget,
    TransitionBuilder, View, WidgetBuilder, WidgetRef, WidgetsApp,
};

use crate::primitives::install_icon_font;
use crate::route::FluentPageRoute;
use crate::theme::{AccentPalette, Theme, ThemeScope};

/// A Fluent app's colours: which WinUI theme dictionary its controls resolve against, and the
/// accent palette they draw with.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FluentTheme {
    /// Whether controls resolve in WinUI's light or dark theme dictionary.
    pub theme: Theme,
    /// The `SystemAccentColor` shades.
    pub accent: AccentPalette,
}

impl FluentTheme {
    /// WinUI's light theme in the default accent.
    pub const fn light() -> FluentTheme {
        FluentTheme {
            theme: Theme::Light,
            accent: AccentPalette::WINDOWS_BLUE,
        }
    }

    /// WinUI's dark theme in the default accent.
    pub const fn dark() -> FluentTheme {
        FluentTheme {
            theme: Theme::Dark,
            accent: AccentPalette::WINDOWS_BLUE,
        }
    }

    /// The theme `theme` in `accent`.
    pub const fn new(theme: Theme, accent: AccentPalette) -> FluentTheme {
        FluentTheme { theme, accent }
    }

    /// The same theme in `accent`.
    pub fn accent(mut self, accent: AccentPalette) -> FluentTheme {
        self.accent = accent;
        self
    }
}

impl Default for FluentTheme {
    fn default() -> FluentTheme {
        FluentTheme::light()
    }
}

/// Which of a [`FluentApp`]'s themes it shows.
///
/// Flutter's `ThemeMode`, in Fluent's name: `System` follows the platform's brightness,
/// `Light` and `Dark` choose one whatever the platform says.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FluentThemeMode {
    /// The dark theme for a platform in dark mode, else the light theme.
    #[default]
    System,
    /// Always the light theme.
    Light,
    /// Always the dark theme.
    Dark,
}

/// An application using Fluent design, WinUI's controls and their theme resources.
///
/// The [`FluentApp`] configures the top-level `Navigator` to search for routes as
/// [`WidgetsApp`] does, and builds each page with [`FluentPageRoute`], so a page enters with
/// WinUI's `Frame` entrance. It installs the Fluent icon font itself, so a widget of this
/// library finds its glyphs without an installation step.
///
/// ```text
/// FluentApp::new()
///     .title("Notes")
///     .home(NotesPage::new())
/// ```
///
/// See also:
///
///  * [`FluentPageRoute`], which plays WinUI's page entrance.
///  * [`ThemeScope`], which the app puts its pages in.
///  * [`WidgetsApp`], which defines the app's routing, shortcuts and title.
pub struct FluentApp {
    /// See [`waft_widgets::Widget::key`].
    pub key: Option<KeyRef>,

    /// See [`WidgetsApp::navigator_key`].
    pub navigator_key: Option<GlobalKey>,

    /// See [`WidgetsApp::home`].
    pub home: Option<WidgetRef>,

    /// The light theme, and the one the app shows unless the mode and the platform ask for the
    /// dark one. [`FluentTheme::light`] when neither theme is set.
    pub theme: Option<FluentTheme>,

    /// The theme shown when the mode is [`FluentThemeMode::Dark`], or the platform's bright and
    /// the mode is [`FluentThemeMode::System`]. The light theme when not set.
    pub dark_theme: Option<FluentTheme>,

    /// Which theme is shown. [`FluentThemeMode::System`] when not set.
    pub theme_mode: Option<FluentThemeMode>,

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
    pub navigator_observers: Vec<NavigatorObserverRef>,

    /// See [`WidgetsApp::builder`].
    pub builder: Option<TransitionBuilder>,

    /// See [`WidgetsApp::title`].
    pub title: Option<String>,

    /// See [`WidgetsApp::on_generate_title`].
    pub on_generate_title: Option<GenerateAppTitle>,

    /// The name old Android versions show in the app switcher; the accent's `base` when not
    /// set, as a Material app takes its theme's primary colour.
    pub color: Option<Color>,

    /// See [`WidgetsApp::locale`].
    pub locale: Option<Locale>,

    /// See [`WidgetsApp::localizations_delegates`].
    pub localizations_delegates: Option<Vec<LocalizationsDelegateRef>>,

    /// See [`WidgetsApp::locale_list_resolution_callback`].
    pub locale_list_resolution_callback: Option<LocaleListResolutionCallback>,

    /// See [`WidgetsApp::locale_resolution_callback`].
    pub locale_resolution_callback: Option<LocaleResolutionCallback>,

    /// See [`WidgetsApp::supported_locales`].
    pub supported_locales: Vec<Locale>,

    /// See [`WidgetsApp::show_performance_overlay`].
    pub show_performance_overlay: bool,

    /// See [`WidgetsApp::show_semantics_debugger`].
    pub show_semantics_debugger: bool,

    /// See [`WidgetsApp::debug_show_checked_mode_banner`].
    pub debug_show_checked_mode_banner: bool,

    /// See [`WidgetsApp::shortcuts`].
    pub shortcuts: Option<ShortcutMap>,

    /// See [`WidgetsApp::actions`].
    pub actions: Option<HashMap<TypeId, ActionRef>>,

    /// See [`WidgetsApp::restoration_scope_id`].
    pub restoration_scope_id: Option<String>,

    /// Dart's deprecated `useInheritedMediaQuery`. This setting is ignored.
    pub use_inherited_media_query: bool,
}

impl Default for FluentApp {
    fn default() -> FluentApp {
        FluentApp {
            key: None,
            navigator_key: None,
            home: None,
            theme: None,
            dark_theme: None,
            theme_mode: None,
            routes: HashMap::new(),
            initial_route: None,
            on_generate_route: None,
            on_generate_initial_routes: None,
            on_unknown_route: None,
            on_navigation_notification: None,
            navigator_observers: Vec::new(),
            builder: None,
            title: None,
            on_generate_title: None,
            color: None,
            locale: None,
            localizations_delegates: None,
            locale_list_resolution_callback: None,
            locale_resolution_callback: None,
            supported_locales: vec![Locale::new("en").country_code("US")],
            show_performance_overlay: false,
            show_semantics_debugger: false,
            debug_show_checked_mode_banner: true,
            shortcuts: None,
            actions: None,
            restoration_scope_id: None,
            use_inherited_media_query: false,
        }
    }
}

impl FluentApp {
    /// Creates a Fluent app with WinUI's light theme.
    pub fn new() -> FluentApp {
        FluentApp::default()
    }

    /// Dart `MaterialApp(key:)`.
    pub fn key(mut self, key: KeyRef) -> FluentApp {
        self.key = Some(key);
        self
    }

    /// Dart `WidgetsApp(navigatorKey:)`.
    pub fn navigator_key(mut self, navigator_key: GlobalKey) -> FluentApp {
        self.navigator_key = Some(navigator_key);
        self
    }

    /// Dart `WidgetsApp(home:)`.
    pub fn home<K>(mut self, home: impl waft_widgets::IntoWidget<K>) -> FluentApp {
        self.home = Some(home.into_widget());
        self
    }

    /// The theme the app shows, and the one an unset [`theme_mode`](Self::theme_mode) falls
    /// back to.
    pub fn theme(mut self, theme: FluentTheme) -> FluentApp {
        self.theme = Some(theme);
        self
    }

    /// The theme shown in dark mode.
    pub fn dark_theme(mut self, theme: FluentTheme) -> FluentApp {
        self.dark_theme = Some(theme);
        self
    }

    /// Which theme the app shows.
    pub fn theme_mode(mut self, mode: FluentThemeMode) -> FluentApp {
        self.theme_mode = Some(mode);
        self
    }

    /// Dart `WidgetsApp(routes:)`.
    pub fn routes(
        mut self,
        routes: impl IntoIterator<Item = (String, WidgetBuilder)>,
    ) -> FluentApp {
        self.routes = routes.into_iter().collect();
        self
    }

    /// Dart `WidgetsApp(initialRoute:)`.
    pub fn initial_route(mut self, initial_route: impl Into<String>) -> FluentApp {
        self.initial_route = Some(initial_route.into());
        self
    }

    /// Dart `WidgetsApp(onGenerateRoute:)`.
    pub fn on_generate_route(
        mut self,
        on_generate_route: impl Fn(
            &mut App,
            &waft_widgets::RouteSettings,
        ) -> Option<waft_widgets::RouteRef>
        + 'static,
    ) -> FluentApp {
        self.on_generate_route = Some(std::rc::Rc::new(on_generate_route));
        self
    }

    /// Dart `WidgetsApp(onGenerateInitialRoutes:)`.
    pub fn on_generate_initial_routes(
        mut self,
        on_generate_initial_routes: impl Fn(&mut App, &str) -> Vec<waft_widgets::RouteRef> + 'static,
    ) -> FluentApp {
        self.on_generate_initial_routes = Some(std::rc::Rc::new(on_generate_initial_routes));
        self
    }

    /// Dart `WidgetsApp(onUnknownRoute:)`.
    pub fn on_unknown_route(
        mut self,
        on_unknown_route: impl Fn(
            &mut App,
            &waft_widgets::RouteSettings,
        ) -> Option<waft_widgets::RouteRef>
        + 'static,
    ) -> FluentApp {
        self.on_unknown_route = Some(std::rc::Rc::new(on_unknown_route));
        self
    }

    /// Dart `WidgetsApp(onNavigationNotification:)`.
    pub fn on_navigation_notification(
        mut self,
        on_navigation_notification: impl Fn(&mut App, &NavigationNotification) -> bool + 'static,
    ) -> FluentApp {
        self.on_navigation_notification = Some(std::rc::Rc::new(on_navigation_notification));
        self
    }

    /// Dart `WidgetsApp(navigatorObservers:)`.
    pub fn navigator_observers(
        mut self,
        navigator_observers: impl IntoIterator<Item = NavigatorObserverRef>,
    ) -> FluentApp {
        self.navigator_observers = navigator_observers.into_iter().collect();
        self
    }

    /// Dart `WidgetsApp(builder:)`.
    pub fn builder(
        mut self,
        builder: impl Fn(&mut App, BuildContext, Option<&WidgetRef>) -> WidgetRef + 'static,
    ) -> FluentApp {
        self.builder = Some(std::rc::Rc::new(builder));
        self
    }

    /// Dart `WidgetsApp(title:)`.
    pub fn title(mut self, title: impl Into<String>) -> FluentApp {
        self.title = Some(title.into());
        self
    }

    /// Dart `WidgetsApp(onGenerateTitle:)`.
    pub fn on_generate_title(
        mut self,
        on_generate_title: impl Fn(&mut App, BuildContext) -> String + 'static,
    ) -> FluentApp {
        self.on_generate_title = Some(std::rc::Rc::new(on_generate_title));
        self
    }

    /// The colour the old Android app switcher shows.
    pub fn color(mut self, color: Color) -> FluentApp {
        self.color = Some(color);
        self
    }

    /// Dart `WidgetsApp(locale:)`.
    pub fn locale(mut self, locale: Locale) -> FluentApp {
        self.locale = Some(locale);
        self
    }

    /// Dart `WidgetsApp(localizationsDelegates:)`.
    pub fn localizations_delegates(
        mut self,
        delegates: impl IntoIterator<Item = LocalizationsDelegateRef>,
    ) -> FluentApp {
        self.localizations_delegates = Some(delegates.into_iter().collect());
        self
    }

    /// Dart `WidgetsApp(localeListResolutionCallback:)`.
    pub fn locale_list_resolution_callback(
        mut self,
        callback: impl Fn(Option<&[Locale]>, &[Locale]) -> Option<Locale> + 'static,
    ) -> FluentApp {
        self.locale_list_resolution_callback = Some(std::rc::Rc::new(callback));
        self
    }

    /// Dart `WidgetsApp(localeResolutionCallback:)`.
    pub fn locale_resolution_callback(
        mut self,
        callback: impl Fn(Option<&Locale>, &[Locale]) -> Option<Locale> + 'static,
    ) -> FluentApp {
        self.locale_resolution_callback = Some(std::rc::Rc::new(callback));
        self
    }

    /// Dart `WidgetsApp(supportedLocales:)`.
    pub fn supported_locales(mut self, locales: impl IntoIterator<Item = Locale>) -> FluentApp {
        self.supported_locales = locales.into_iter().collect();
        self
    }

    /// Dart `WidgetsApp(showPerformanceOverlay:)`.
    pub fn show_performance_overlay(mut self, show: bool) -> FluentApp {
        self.show_performance_overlay = show;
        self
    }

    /// Dart `WidgetsApp(showSemanticsDebugger:)`.
    pub fn show_semantics_debugger(mut self, show: bool) -> FluentApp {
        self.show_semantics_debugger = show;
        self
    }

    /// Dart `WidgetsApp(debugShowCheckedModeBanner:)`.
    pub fn debug_show_checked_mode_banner(mut self, show: bool) -> FluentApp {
        self.debug_show_checked_mode_banner = show;
        self
    }

    /// Dart `WidgetsApp(shortcuts:)`.
    pub fn shortcuts(mut self, shortcuts: ShortcutMap) -> FluentApp {
        self.shortcuts = Some(shortcuts);
        self
    }

    /// Dart `WidgetsApp(actions:)`.
    pub fn actions(mut self, actions: HashMap<TypeId, ActionRef>) -> FluentApp {
        self.actions = Some(actions);
        self
    }

    /// Dart `WidgetsApp(restorationScopeId:)`.
    pub fn restoration_scope_id(mut self, id: impl Into<String>) -> FluentApp {
        self.restoration_scope_id = Some(id.into());
        self
    }
}

impl FluentAppState {
    /// The theme the app shows now, from its mode and the platform's brightness.
    fn effective_theme(
        self: Handle<FluentAppState>,
        app: &mut App,
        context: BuildContext,
    ) -> FluentTheme {
        let (theme, dark_theme, mode) = {
            let widget = self.widget(app);
            (
                widget.theme,
                widget.dark_theme,
                widget.theme_mode.unwrap_or_default(),
            )
        };
        let dark = mode == FluentThemeMode::Dark
            || (mode == FluentThemeMode::System
                && MediaQuery::platform_brightness_of(app, context) == Brightness::Dark);
        if dark {
            dark_theme.or(theme).unwrap_or_else(FluentTheme::dark)
        } else {
            theme.unwrap_or_else(FluentTheme::light)
        }
    }

    /// Builds the [`WidgetsApp`] this app is: its routes, its Fluent page routes and its title.
    fn build_widget_app(
        self: Handle<FluentAppState>,
        app: &mut App,
        context: BuildContext,
    ) -> WidgetsApp {
        let theme = self.effective_theme(app, context);
        let widget = self.widget(app);
        let color = widget.color.unwrap_or(theme.accent.base);

        // The `WidgetsApp.router` branch waits with `router.dart`; see PORTING.md.
        let mut widgets_app = WidgetsApp::new(color)
            .key(GlobalKey::object(self.id()).into())
            .navigator_observers(widget.navigator_observers.clone())
            .page_route_builder(|app, settings, builder| {
                let route = FluentPageRoute::new(app, builder)
                    .settings(app, RouteSettingsRef::Settings(settings.clone()));
                route.into_erased()
            })
            .routes(widget.routes.clone())
            .supported_locales(widget.supported_locales.clone())
            .show_performance_overlay(widget.show_performance_overlay)
            .show_semantics_debugger(widget.show_semantics_debugger)
            .debug_show_checked_mode_banner(widget.debug_show_checked_mode_banner);

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
        if let Some(builder) = widget.builder.clone() {
            widgets_app =
                widgets_app.builder(move |app, context, child| builder(app, context, child));
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
        if let Some(delegates) = widget.localizations_delegates.clone() {
            widgets_app = widgets_app.localizations_delegates(delegates);
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

impl StatefulWidget for FluentApp {
    type State = FluentAppState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> FluentAppState {
        FluentAppState {
            state: StateData::new(),
        }
    }
}

/// The state of a [`FluentApp`]: its widget's fields through [`State`].
pub struct FluentAppState {
    state: StateData<FluentApp>,
}

impl State for FluentAppState {
    type Widget = FluentApp;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        install_icon_font(app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let theme = self.effective_theme(app, context);
        if let Some(view) = View::maybe_of(app, context) {
            SystemChrome::set_system_ui_overlay_style(
                app,
                view.id(),
                match theme.theme {
                    Theme::Dark => &SystemUiOverlayStyle::LIGHT,
                    Theme::Light => &SystemUiOverlayStyle::DARK,
                },
            );
        }
        ThemeScope::new(
            theme.theme,
            waft_widgets::Builder::new(move |app, context| {
                self.build_widget_app(app, context).into_widget()
            }),
        )
        .accent(theme.accent)
        .into_widget()
    }
}

impl Debug for FluentApp {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FluentApp")
            .field("key", &self.key)
            .field("home", &self.home)
            .field("theme", &self.theme)
            .field("darkTheme", &self.dark_theme)
            .field("themeMode", &self.theme_mode)
            .field("routes", &self.routes.keys().collect::<Vec<_>>())
            .field("locale", &self.locale)
            .field("restorationScopeId", &self.restoration_scope_id)
            .finish_non_exhaustive()
    }
}
