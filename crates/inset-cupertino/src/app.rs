//! Flutter counterpart: `cupertino/app.dart`.
//!
//! [`CupertinoApp`], the widget an iOS-designed application is built from, and the
//! [`CupertinoScrollBehavior`] it installs. `CupertinoApp.router` waits with `router.dart`.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt::{self, Debug};
use std::rc::Rc;

use inset_embedder::{Brightness, Locale, SystemUiOverlayStyle, TargetPlatform};
use inset_foundation::{App, Handle};
use inset_gestures::MultitouchDragStrategy;
use inset_painting::AnyColor;
use inset_services::SystemChrome;
use inset_widgets::{
    AnyAction, AnyNavigatorObserver, AnyRoute, BouncingScrollPhysics, BuildContext, Builder,
    GenerateAppTitle, GlobalKey, HeroController, HeroControllerScope, InitialRouteListFactory,
    IntoWidget, KeyRef, LocaleListResolutionCallback, LocaleResolutionCallback,
    LocalizationsDelegateRef, MediaQuery, NavigationNotification, NotificationListenerCallback,
    PageRoute, RouteFactory, RouteSettingsRef, ScrollBehavior, ScrollBehaviorRef,
    ScrollConfiguration, ScrollDecelerationRate, ScrollPhysicsRef, ScrollableDetails, ShortcutMap,
    State, StateData, StatefulWidget, TransitionBuilder, WidgetBuilder, WidgetRef, WidgetsApp,
};

use crate::colors::CupertinoDynamicColor;
use crate::interface_level::{CupertinoUserInterfaceLevel, CupertinoUserInterfaceLevelData};
use crate::localizations::DefaultCupertinoLocalizations;
use crate::route::CupertinoPageRoute;
use crate::scrollbar::CupertinoScrollbar;
use crate::theme::{CupertinoTheme, CupertinoThemeData};

/// An application that uses Cupertino design.
///
/// A convenience widget that wraps a number of widgets that are commonly
/// required for an iOS-design targeting application. It builds upon a
/// [`WidgetsApp`] by iOS specific defaulting such as fonts and scrolling
/// physics.
///
/// The [`CupertinoApp`] configures the top-level `Navigator` to search for routes
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
/// If [`home`](Self::home), [`routes`](Self::routes),
/// [`on_generate_route`](Self::on_generate_route), and
/// [`on_unknown_route`](Self::on_unknown_route) are all null, and
/// [`builder`](Self::builder) is not null, then no `Navigator` is created.
///
/// This widget also configures the observer of the top-level `Navigator` (if
/// any) to perform `Hero` animations.
///
/// The [`CupertinoApp`] widget isn't a required ancestor for other Cupertino
/// widgets, but many Cupertino widgets could depend on the [`CupertinoTheme`]
/// widget, which the [`CupertinoApp`] composes.
///
/// Use this widget with caution on Android since it may produce behaviors
/// Android users are not expecting such as:
///
///  * Pages will be dismissible via a back swipe.
///  * Scrolling past extremities will trigger iOS-style spring overscrolls.
///  * The San Francisco font family is unavailable on Android and can result
///    in undefined font behavior.
///
/// ```text
/// CupertinoApp::new().home(
///     CupertinoPageScaffold::new(Center::new().child(Icon::new(CupertinoIcons::SHARE))),
/// )
/// ```
///
/// See also:
///
///  * `CupertinoPageScaffold`, which provides a standard page layout default
///    with nav bars.
///  * `Navigator`, which is used to manage the app's stack of pages.
///  * [`CupertinoPageRoute`], which defines an app page that transitions in an
///    iOS-specific way.
///  * [`WidgetsApp`], which defines the basic app elements but does not depend
///    on the Cupertino library.
pub struct CupertinoApp {
    /// See [`Widget::key`](inset_widgets::Widget::key).
    pub key: Option<KeyRef>,

    /// See [`WidgetsApp::navigator_key`].
    pub navigator_key: Option<GlobalKey>,

    /// See [`WidgetsApp::home`].
    pub home: Option<WidgetRef>,

    /// The top-level [`CupertinoTheme`] styling.
    ///
    /// A null [`theme`](Self::theme) or unspecified [`theme`](Self::theme) attributes will
    /// default to iOS system values.
    pub theme: Option<CupertinoThemeData>,

    /// The application's top-level routing table.
    ///
    /// When a named route is pushed with `Navigator::push_named`, the route name is
    /// looked up in this map. If the name is present, the associated
    /// [`WidgetBuilder`] is used to construct a [`CupertinoPageRoute`] that
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
    pub builder: Option<TransitionBuilder>,

    /// See [`WidgetsApp::title`].
    ///
    /// This value is passed unmodified to [`WidgetsApp::title`].
    pub title: Option<String>,

    /// See [`WidgetsApp::on_generate_title`].
    ///
    /// This value is passed unmodified to [`WidgetsApp::on_generate_title`].
    pub on_generate_title: Option<GenerateAppTitle>,

    /// See [`WidgetsApp::color`].
    pub color: Option<AnyColor>,

    /// See [`WidgetsApp::locale`].
    pub locale: Option<Locale>,

    /// See [`WidgetsApp::localizations_delegates`].
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
    /// When null, defaults to [`CupertinoScrollBehavior`].
    ///
    /// See also:
    ///
    ///  * [`ScrollConfiguration`], which controls how `Scrollable` widgets behave
    ///    in a subtree.
    pub scroll_behavior: Option<ScrollBehaviorRef>,

    /// Dart's deprecated `useInheritedMediaQuery`. This setting is ignored.
    ///
    /// The widget never introduces its own `MediaQuery`; the `View` widget takes
    /// care of that.
    pub use_inherited_media_query: bool,
}

impl Default for CupertinoApp {
    fn default() -> CupertinoApp {
        CupertinoApp {
            key: None,
            navigator_key: None,
            home: None,
            theme: None,
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
            checkerboard_raster_cache_images: false,
            checkerboard_offscreen_layers: false,
            show_semantics_debugger: false,
            debug_show_checked_mode_banner: true,
            shortcuts: None,
            actions: None,
            restoration_scope_id: None,
            scroll_behavior: None,
            use_inherited_media_query: false,
        }
    }
}

impl CupertinoApp {
    /// Creates a CupertinoApp; Dart's named arguments are the setters.
    ///
    /// At least one of [`home`](Self::home), [`routes`](Self::routes),
    /// [`on_generate_route`](Self::on_generate_route), or [`builder`](Self::builder) must be
    /// set. If only [`routes`](Self::routes) is given, it must include an entry for
    /// `Navigator::DEFAULT_ROUTE_NAME` (`/`), since that is the route used when the
    /// application is launched with an intent that specifies an otherwise unsupported route.
    ///
    /// This widget creates an instance of [`WidgetsApp`].
    pub fn new() -> CupertinoApp {
        CupertinoApp::default()
    }

    /// Dart `CupertinoApp(key:)`.
    pub fn key(mut self, key: KeyRef) -> CupertinoApp {
        self.key = Some(key);
        self
    }

    /// Dart `CupertinoApp(navigatorKey:)`.
    pub fn navigator_key(mut self, navigator_key: GlobalKey) -> CupertinoApp {
        self.navigator_key = Some(navigator_key);
        self
    }

    /// Dart `CupertinoApp(home:)`.
    pub fn home<K>(mut self, home: impl IntoWidget<K>) -> CupertinoApp {
        self.home = Some(home.into_widget());
        self
    }

    /// Dart `CupertinoApp(theme:)`.
    pub fn theme(mut self, theme: CupertinoThemeData) -> CupertinoApp {
        self.theme = Some(theme);
        self
    }

    /// Dart `CupertinoApp(routes:)`.
    pub fn routes(
        mut self,
        routes: impl IntoIterator<Item = (String, WidgetBuilder)>,
    ) -> CupertinoApp {
        self.routes = routes.into_iter().collect();
        self
    }

    /// Dart `CupertinoApp(initialRoute:)`.
    pub fn initial_route(mut self, initial_route: impl Into<String>) -> CupertinoApp {
        self.initial_route = Some(initial_route.into());
        self
    }

    /// Dart `CupertinoApp(onGenerateRoute:)`.
    pub fn on_generate_route(
        mut self,
        on_generate_route: impl Fn(&mut App, &inset_widgets::RouteSettings) -> Option<AnyRoute>
        + 'static,
    ) -> CupertinoApp {
        self.on_generate_route = Some(Rc::new(on_generate_route));
        self
    }

    /// Dart `CupertinoApp(onGenerateInitialRoutes:)`.
    pub fn on_generate_initial_routes(
        mut self,
        on_generate_initial_routes: impl Fn(&mut App, &str) -> Vec<AnyRoute> + 'static,
    ) -> CupertinoApp {
        self.on_generate_initial_routes = Some(Rc::new(on_generate_initial_routes));
        self
    }

    /// Dart `CupertinoApp(onUnknownRoute:)`.
    pub fn on_unknown_route(
        mut self,
        on_unknown_route: impl Fn(&mut App, &inset_widgets::RouteSettings) -> Option<AnyRoute> + 'static,
    ) -> CupertinoApp {
        self.on_unknown_route = Some(Rc::new(on_unknown_route));
        self
    }

    /// Dart `CupertinoApp(onNavigationNotification:)`.
    pub fn on_navigation_notification(
        mut self,
        on_navigation_notification: impl Fn(&mut App, &NavigationNotification) -> bool + 'static,
    ) -> CupertinoApp {
        self.on_navigation_notification = Some(Rc::new(on_navigation_notification));
        self
    }

    /// Dart `CupertinoApp(navigatorObservers:)`.
    pub fn navigator_observers(
        mut self,
        navigator_observers: impl IntoIterator<Item = AnyNavigatorObserver>,
    ) -> CupertinoApp {
        self.navigator_observers = navigator_observers.into_iter().collect();
        self
    }

    /// Dart `CupertinoApp(builder:)`.
    pub fn builder(
        mut self,
        builder: impl Fn(&mut App, BuildContext, Option<&WidgetRef>) -> WidgetRef + 'static,
    ) -> CupertinoApp {
        self.builder = Some(Rc::new(builder));
        self
    }

    /// Dart `CupertinoApp(title:)`.
    pub fn title(mut self, title: impl Into<String>) -> CupertinoApp {
        self.title = Some(title.into());
        self
    }

    /// Dart `CupertinoApp(onGenerateTitle:)`.
    pub fn on_generate_title(
        mut self,
        on_generate_title: impl Fn(&mut App, BuildContext) -> String + 'static,
    ) -> CupertinoApp {
        self.on_generate_title = Some(Rc::new(on_generate_title));
        self
    }

    /// Dart `CupertinoApp(color:)`.
    pub fn color(mut self, color: impl Into<AnyColor>) -> CupertinoApp {
        self.color = Some(color.into());
        self
    }

    /// Dart `CupertinoApp(locale:)`.
    pub fn locale(mut self, locale: Locale) -> CupertinoApp {
        self.locale = Some(locale);
        self
    }

    /// Dart `CupertinoApp(localizationsDelegates:)`.
    pub fn localizations_delegates(
        mut self,
        localizations_delegates: impl IntoIterator<Item = LocalizationsDelegateRef>,
    ) -> CupertinoApp {
        self.localizations_delegates = Some(localizations_delegates.into_iter().collect());
        self
    }

    /// Dart `CupertinoApp(localeListResolutionCallback:)`.
    pub fn locale_list_resolution_callback(
        mut self,
        locale_list_resolution_callback: impl Fn(Option<&[Locale]>, &[Locale]) -> Option<Locale>
        + 'static,
    ) -> CupertinoApp {
        self.locale_list_resolution_callback = Some(Rc::new(locale_list_resolution_callback));
        self
    }

    /// Dart `CupertinoApp(localeResolutionCallback:)`.
    pub fn locale_resolution_callback(
        mut self,
        locale_resolution_callback: impl Fn(Option<&Locale>, &[Locale]) -> Option<Locale> + 'static,
    ) -> CupertinoApp {
        self.locale_resolution_callback = Some(Rc::new(locale_resolution_callback));
        self
    }

    /// Dart `CupertinoApp(supportedLocales:)`.
    pub fn supported_locales(
        mut self,
        supported_locales: impl IntoIterator<Item = Locale>,
    ) -> CupertinoApp {
        self.supported_locales = supported_locales.into_iter().collect();
        self
    }

    /// Dart `CupertinoApp(showPerformanceOverlay:)`.
    pub fn show_performance_overlay(mut self, show_performance_overlay: bool) -> CupertinoApp {
        self.show_performance_overlay = show_performance_overlay;
        self
    }

    /// Dart `CupertinoApp(checkerboardRasterCacheImages:)`.
    pub fn checkerboard_raster_cache_images(
        mut self,
        checkerboard_raster_cache_images: bool,
    ) -> CupertinoApp {
        self.checkerboard_raster_cache_images = checkerboard_raster_cache_images;
        self
    }

    /// Dart `CupertinoApp(checkerboardOffscreenLayers:)`.
    pub fn checkerboard_offscreen_layers(
        mut self,
        checkerboard_offscreen_layers: bool,
    ) -> CupertinoApp {
        self.checkerboard_offscreen_layers = checkerboard_offscreen_layers;
        self
    }

    /// Dart `CupertinoApp(showSemanticsDebugger:)`.
    pub fn show_semantics_debugger(mut self, show_semantics_debugger: bool) -> CupertinoApp {
        self.show_semantics_debugger = show_semantics_debugger;
        self
    }

    /// Dart `CupertinoApp(debugShowCheckedModeBanner:)`.
    pub fn debug_show_checked_mode_banner(
        mut self,
        debug_show_checked_mode_banner: bool,
    ) -> CupertinoApp {
        self.debug_show_checked_mode_banner = debug_show_checked_mode_banner;
        self
    }

    /// Dart `CupertinoApp(shortcuts:)`.
    pub fn shortcuts(mut self, shortcuts: ShortcutMap) -> CupertinoApp {
        self.shortcuts = Some(shortcuts);
        self
    }

    /// Dart `CupertinoApp(actions:)`.
    pub fn actions(mut self, actions: HashMap<TypeId, AnyAction>) -> CupertinoApp {
        self.actions = Some(actions);
        self
    }

    /// Dart `CupertinoApp(restorationScopeId:)`.
    pub fn restoration_scope_id(mut self, restoration_scope_id: impl Into<String>) -> CupertinoApp {
        self.restoration_scope_id = Some(restoration_scope_id.into());
        self
    }

    /// Dart `CupertinoApp(scrollBehavior:)`.
    pub fn scroll_behavior(mut self, scroll_behavior: ScrollBehaviorRef) -> CupertinoApp {
        self.scroll_behavior = Some(scroll_behavior);
        self
    }

    /// Dart `CupertinoApp(useInheritedMediaQuery:)`, which is ignored.
    pub fn use_inherited_media_query(mut self, use_inherited_media_query: bool) -> CupertinoApp {
        self.use_inherited_media_query = use_inherited_media_query;
        self
    }

    /// The [`HeroController`] used for Cupertino page transitions.
    ///
    /// Used by `CupertinoTabView` and [`CupertinoApp`].
    pub fn create_cupertino_hero_controller(app: &mut App) -> Handle<HeroController> {
        // Linear tweening.
        HeroController::new(app)
    }
}

impl Debug for CupertinoApp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CupertinoApp")
            .field("title", &self.title)
            .field("theme", &self.theme)
            .field("locale", &self.locale)
            .field("restorationScopeId", &self.restoration_scope_id)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for CupertinoApp {
    type State = CupertinoAppState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> CupertinoAppState {
        CupertinoAppState {
            state: StateData::new(),
            hero_controller: None,
            widgets_app_key: None,
        }
    }
}

/// Dart's `_CupertinoAppState`.
pub struct CupertinoAppState {
    state: StateData<CupertinoApp>,
    hero_controller: Option<Handle<HeroController>>,
    widgets_app_key: Option<GlobalKey>,
}

impl CupertinoAppState {
    fn hero_controller(self: Handle<Self>, app: &App) -> Handle<HeroController> {
        app.get(self)
            .hero_controller
            .expect("init_state creates the controller")
    }

    /// The key that keeps the [`WidgetsApp`] element alive across rebuilds; Dart's
    /// `GlobalObjectKey(this)`.
    fn widgets_app_key(self: Handle<Self>, app: &App) -> GlobalKey {
        app.get(self)
            .widgets_app_key
            .clone()
            .expect("init_state creates the key")
    }

    /// Combine the default localization for Cupertino with the ones contributed
    /// by the [`CupertinoApp::localizations_delegates`] parameter, if any. Only the first
    /// delegate of a particular `LocalizationsDelegate` type is loaded so the
    /// [`CupertinoApp::localizations_delegates`] parameter can be used to override
    /// `CupertinoLocalizationsDelegate`.
    fn localizations_delegates(self: Handle<Self>, app: &App) -> Vec<LocalizationsDelegateRef> {
        let mut delegates = self
            .widget(app)
            .localizations_delegates
            .clone()
            .unwrap_or_default();
        delegates.push(DefaultCupertinoLocalizations::delegate());
        delegates
    }

    fn build_widget_app(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetsApp {
        let effective_theme_data = CupertinoTheme::of(app, context);
        let color = CupertinoDynamicColor::resolve(
            &self
                .widget(app)
                .color
                .clone()
                .unwrap_or_else(|| effective_theme_data.primary_color()),
            app,
            context,
        );

        // The `WidgetsApp.router` branch waits with `router.dart`; see PORTING.md.

        let localizations_delegates = self.localizations_delegates(app);
        let key = self.widgets_app_key(app);
        let widget = self.widget(app);
        let mut widgets_app = WidgetsApp::new(color.color())
            .key(Rc::new(key))
            .navigator_observers(widget.navigator_observers.clone())
            .page_route_builder(|app, settings, builder| {
                let route = CupertinoPageRoute::new(app, builder)
                    .settings(app, RouteSettingsRef::Settings(settings.clone()));
                PageRoute::as_page_route(route)
            })
            .routes(widget.routes.clone())
            .text_style(effective_theme_data.text_theme().text_style())
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

impl State for CupertinoAppState {
    type Widget = CupertinoApp;
    inset_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let hero_controller = CupertinoApp::create_cupertino_hero_controller(app);
        let state = app.get_mut(self);
        state.hero_controller = Some(hero_controller);
        state.widgets_app_key = Some(GlobalKey::new());
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.hero_controller(app).dispose(app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let effective_theme_data = self
            .widget(app)
            .theme
            .clone()
            .unwrap_or_default()
            .resolve_from(app, context);

        // Prefer theme brightness if set, otherwise check system brightness.
        let brightness = effective_theme_data
            .brightness()
            .unwrap_or_else(|| MediaQuery::platform_brightness_of(app, context));

        SystemChrome::set_system_ui_overlay_style(
            app,
            match brightness {
                Brightness::Dark => &SystemUiOverlayStyle::LIGHT,
                Brightness::Light => &SystemUiOverlayStyle::DARK,
            },
        );

        let behavior = self
            .widget(app)
            .scroll_behavior
            .clone()
            .unwrap_or_else(|| ScrollBehaviorRef::new(CupertinoScrollBehavior::new()));
        let hero_controller = self.hero_controller(app);
        ScrollConfiguration::new(
            behavior,
            CupertinoUserInterfaceLevel::new(
                CupertinoUserInterfaceLevelData::Base,
                CupertinoTheme::new(
                    effective_theme_data,
                    // `DefaultSelectionStyle` waits; see PORTING.md.
                    HeroControllerScope::new(
                        hero_controller,
                        Builder::new(move |app, context| {
                            self.build_widget_app(app, context).into_widget()
                        }),
                    ),
                ),
            ),
        )
        .into_widget()
    }
}

/// Describes how `Scrollable` widgets behave for [`CupertinoApp`]s.
///
/// Setting a [`CupertinoScrollBehavior`] will result in descendant `Scrollable` widgets
/// using [`BouncingScrollPhysics`] by default. No `GlowingOverscrollIndicator` is
/// applied when using a [`CupertinoScrollBehavior`] either, regardless of platform.
/// When executing on desktop platforms, a [`CupertinoScrollbar`] is applied to the child.
///
/// See also:
///
///  * [`ScrollBehavior`], the default scrolling behavior extended by this class.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CupertinoScrollBehavior;

impl CupertinoScrollBehavior {
    /// Creates a CupertinoScrollBehavior that uses [`BouncingScrollPhysics`] and
    /// adds [`CupertinoScrollbar`]s on desktop platforms.
    pub const fn new() -> CupertinoScrollBehavior {
        CupertinoScrollBehavior
    }
}

impl ScrollBehavior for CupertinoScrollBehavior {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn build_scrollbar(
        &self,
        app: &mut App,
        context: BuildContext,
        child: WidgetRef,
        details: &ScrollableDetails,
    ) -> WidgetRef {
        // When modifying this function, consider modifying the implementation in
        // the base class as well.
        match self.get_platform(app, context) {
            TargetPlatform::Linux | TargetPlatform::MacOS | TargetPlatform::Windows => {
                debug_assert!(details.controller.is_some());
                let mut scrollbar = CupertinoScrollbar::new(child);
                if let Some(controller) = details.controller {
                    scrollbar = scrollbar.controller(controller);
                }
                scrollbar.into_widget()
            }
            TargetPlatform::Android | TargetPlatform::Fuchsia | TargetPlatform::IOS => child,
        }
    }

    fn build_overscroll_indicator(
        &self,
        app: &mut App,
        context: BuildContext,
        child: WidgetRef,
        details: &ScrollableDetails,
    ) -> WidgetRef {
        // No overscroll indicator.
        // When modifying this function, consider modifying the implementation in
        // the base class as well.
        let _ = (app, context, details);
        child
    }

    fn get_scroll_physics(&self, app: &App, context: BuildContext) -> ScrollPhysicsRef {
        // When modifying this function, consider modifying the implementation in
        // the base class ScrollBehavior as well.
        if self.get_platform(app, context) == TargetPlatform::MacOS {
            return Rc::new(
                BouncingScrollPhysics::new().with_deceleration_rate(ScrollDecelerationRate::Fast),
            );
        }
        Rc::new(BouncingScrollPhysics::new())
    }

    fn get_multitouch_drag_strategy(
        &self,
        app: &App,
        context: BuildContext,
    ) -> MultitouchDragStrategy {
        let _ = (app, context);
        MultitouchDragStrategy::AverageBoundaryPointers
    }
}

#[cfg(test)]
mod tests {
    use inset_foundation::AppCell;
    use std::cell::Cell;
    use std::time::Duration;

    use inset_embedder::Color;
    use inset_painting::PaintingBinding;
    use inset_test::TestPlatform;
    use inset_widgets::{
        AnyElement, AnyModalRoute, Localizations, Navigator, ScrollController,
        ScrollControllerLeaf, SizedBox, StatelessWidget, WidgetsBinding, WidgetsLocalizations,
        downcast_widget,
    };

    use super::*;
    use crate::localizations::CupertinoLocalizations;
    use crate::test_support::{build, test_view};

    /// An app on `target_platform`, over the shared test view.
    fn app_of(target_platform: TargetPlatform) -> Rc<AppCell> {
        let platform = TestPlatform::new()
            .on(target_platform)
            .with_view(test_view());
        let cell = AppCell::with_platform(Rc::new(platform));
        let mut app = cell.borrow_mut();
        install_fonts(&mut app);
        drop(app);
        cell
    }

    /// What the shell does at start-up: the app-wide fonts, which the debug banner needs.
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
        for _ in 0..40 {
            at += Duration::from_millis(20);
            crate::test_support::pump(app, at);
        }
    }

    /// A leaf that is easy to find in the element tree.
    #[derive(Debug)]
    struct Marker;

    impl StatelessWidget for Marker {
        fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
            SizedBox::new().width(10.0).height(10.0).into_widget()
        }
    }

    /// A page that hands its build context to `sink` on every build.
    fn context_page(sink: &Rc<Cell<Option<BuildContext>>>) -> WidgetBuilder {
        let sink = Rc::clone(sink);
        Rc::new(move |_app, context| {
            sink.set(Some(context));
            Marker.into_widget()
        })
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

    fn has_widget<W: 'static>(app: &App, root: AnyElement) -> bool {
        descendants(app, root)
            .into_iter()
            .any(|element| downcast_widget::<W>(&**element.widget(app)).is_some())
    }

    /// Mounts `configure(CupertinoApp::new())` and settles its route transitions.
    fn mount(cell: &AppCell, configure: impl FnOnce(CupertinoApp) -> CupertinoApp) {
        build(cell, configure(CupertinoApp::new()).into_widget());
        settle(&mut cell.borrow_mut());
    }

    /// The route the home page of a mounted app sits in.
    fn home_route(app: &mut App, context: BuildContext) -> Handle<CupertinoPageRoute> {
        AnyModalRoute::of(app, context)
            .expect("the home page sits in a modal route")
            .as_route()
            .downcast::<CupertinoPageRoute>(app)
            .expect("a Cupertino page route")
    }

    #[test]
    fn an_app_with_a_home_wraps_it_in_the_cupertino_chrome_and_a_navigator() {
        let cell = app_of(TargetPlatform::IOS);
        mount(&cell, |cupertino_app| cupertino_app.home(Marker));
        let mut app = cell.borrow_mut();
        let root = root_element(&mut app);
        assert!(has_widget::<ScrollConfiguration>(&app, root));
        assert!(has_widget::<CupertinoUserInterfaceLevel>(&app, root));
        assert!(has_widget::<CupertinoTheme>(&app, root));
        assert!(has_widget::<HeroControllerScope>(&app, root));
        assert!(has_widget::<WidgetsApp>(&app, root));
        assert!(has_widget::<Navigator>(&app, root));
        assert!(
            has_widget::<Marker>(&app, root),
            "the home route shows home"
        );
    }

    #[test]
    fn the_home_route_reads_the_cupertino_and_widgets_localizations() {
        let cell = app_of(TargetPlatform::IOS);
        let home = Rc::new(Cell::new(None));
        let builder = context_page(&home);
        mount(&cell, |cupertino_app| {
            cupertino_app.home(Builder::new(move |app, context| builder(app, context)))
        });
        let mut app = cell.borrow_mut();
        let context = home.get().expect("the home page built");
        assert_eq!(
            <dyn CupertinoLocalizations>::of(&mut app, context).alert_dialog_label(),
            "Alert",
            "the Cupertino delegate is supplied"
        );
        assert!(
            Localizations::of::<dyn WidgetsLocalizations>(&mut app, context).is_some(),
            "the widgets delegate the WidgetsApp appends is still there"
        );
    }

    #[test]
    fn the_home_route_reads_the_theme_the_app_was_given() {
        const PRIMARY: Color = Color::new(0xFF00FF00);

        let cell = app_of(TargetPlatform::IOS);
        let home = Rc::new(Cell::new(None));
        let builder = context_page(&home);
        mount(&cell, |cupertino_app| {
            cupertino_app
                .theme(CupertinoThemeData::new().with_primary_color(PRIMARY))
                .home(Builder::new(move |app, context| builder(app, context)))
        });
        let mut app = cell.borrow_mut();
        let context = home.get().expect("the home page built");
        assert_eq!(
            CupertinoTheme::of(&mut app, context).primary_color(),
            AnyColor::new(PRIMARY)
        );
    }

    #[test]
    fn the_route_the_home_page_sits_in_is_a_cupertino_page_route() {
        let cell = app_of(TargetPlatform::IOS);
        let home = Rc::new(Cell::new(None));
        let builder = context_page(&home);
        mount(&cell, |cupertino_app| {
            cupertino_app.home(Builder::new(move |app, context| builder(app, context)))
        });
        let mut app = cell.borrow_mut();
        let context = home.get().expect("the home page built");
        home_route(&mut app, context);
    }

    #[test]
    fn a_routes_table_entry_becomes_a_cupertino_page_route() {
        let cell = app_of(TargetPlatform::IOS);
        let home = Rc::new(Cell::new(None));
        mount(&cell, |cupertino_app| {
            cupertino_app.routes([(String::from("/"), context_page(&home))])
        });
        let mut app = cell.borrow_mut();
        let context = home.get().expect("the default route built");
        home_route(&mut app, context);
    }

    #[test]
    fn on_generate_route_supplies_a_route_the_table_does_not_have() {
        let cell = app_of(TargetPlatform::IOS);
        let generated = Rc::new(Cell::new(None));
        let details = Rc::new(Cell::new(None));
        let asked_for = Rc::clone(&generated);
        let details_page = context_page(&details);
        mount(&cell, |cupertino_app| {
            cupertino_app
                .home(SizedBox::expand())
                .initial_route("/details")
                .on_generate_route(move |app, settings| {
                    asked_for.set(settings.name.clone());
                    let builder = Rc::clone(&details_page);
                    let route = CupertinoPageRoute::new(app, builder);
                    Some(inset_widgets::Route::as_route(route))
                })
        });
        assert_eq!(generated.take().as_deref(), Some("/details"));
        assert!(details.get().is_some(), "the generated route is shown");
    }

    #[test]
    fn the_scrollbar_is_built_on_desktop_platforms_only() {
        for platform in [
            TargetPlatform::Linux,
            TargetPlatform::MacOS,
            TargetPlatform::Windows,
        ] {
            let (cell, context) = app_with_context(platform);
            let mut app = cell.borrow_mut();
            assert!(
                is_cupertino_scrollbar(&mut app, context),
                "{platform:?} gets a CupertinoScrollbar"
            );
        }
        for platform in [
            TargetPlatform::Android,
            TargetPlatform::Fuchsia,
            TargetPlatform::IOS,
        ] {
            let (cell, context) = app_with_context(platform);
            let mut app = cell.borrow_mut();
            assert!(
                !is_cupertino_scrollbar(&mut app, context),
                "{platform:?} keeps the bare child"
            );
        }
    }

    #[test]
    fn the_physics_bounce_everywhere_and_decelerate_fast_on_macos() {
        for platform in [
            TargetPlatform::Android,
            TargetPlatform::Fuchsia,
            TargetPlatform::IOS,
            TargetPlatform::Linux,
            TargetPlatform::Windows,
        ] {
            let (cell, context) = app_with_context(platform);
            let app = cell.borrow();
            let physics = CupertinoScrollBehavior::new().get_scroll_physics(&app, context);
            let bouncing = physics
                .as_any()
                .downcast_ref::<BouncingScrollPhysics>()
                .unwrap_or_else(|| panic!("{platform:?} bounces"));
            assert_eq!(bouncing.deceleration_rate, ScrollDecelerationRate::Normal);
            assert!(
                bouncing.parent.is_none(),
                "no RangeMaintainingScrollPhysics"
            );
        }
        let (cell, context) = app_with_context(TargetPlatform::MacOS);
        let app = cell.borrow();
        let physics = CupertinoScrollBehavior::new().get_scroll_physics(&app, context);
        let bouncing = physics
            .as_any()
            .downcast_ref::<BouncingScrollPhysics>()
            .expect("macOS bounces");
        assert_eq!(bouncing.deceleration_rate, ScrollDecelerationRate::Fast);
    }

    #[test]
    fn the_multitouch_drag_strategy_averages_the_boundary_pointers() {
        let (cell, context) = app_with_context(TargetPlatform::Android);
        let app = cell.borrow();
        assert_eq!(
            CupertinoScrollBehavior::new().get_multitouch_drag_strategy(&app, context),
            MultitouchDragStrategy::AverageBoundaryPointers
        );
    }

    #[test]
    fn no_overscroll_indicator_is_ever_built() {
        let (cell, context) = app_with_context(TargetPlatform::Android);
        let mut app = cell.borrow_mut();
        let child: WidgetRef = SizedBox::new().into_widget();
        let details = ScrollableDetails::vertical(false);
        let decorated = CupertinoScrollBehavior::new().build_overscroll_indicator(
            &mut app,
            context,
            child.clone(),
            &details,
        );
        assert!(Rc::ptr_eq(&decorated, &child));
    }

    /// An app on `platform` with one mounted build context to resolve against.
    fn app_with_context(platform: TargetPlatform) -> (Rc<AppCell>, BuildContext) {
        let cell = app_of(platform);
        let seen = Rc::new(Cell::new(None));
        let sink = Rc::clone(&seen);
        build(
            &cell,
            Builder::new(move |_app, context| {
                sink.set(Some(context));
                SizedBox::new().into_widget()
            })
            .into_widget(),
        );
        let context = seen.get().expect("the builder ran");
        (cell, context)
    }

    fn is_cupertino_scrollbar(app: &mut App, context: BuildContext) -> bool {
        let controller = ScrollController::default(app).as_controller();
        let details = ScrollableDetails::vertical(false).controller(controller);
        let child: WidgetRef = SizedBox::new().into_widget();
        let decorated =
            CupertinoScrollBehavior::new().build_scrollbar(app, context, child, &details);
        downcast_widget::<CupertinoScrollbar>(&*decorated).is_some()
    }
}
