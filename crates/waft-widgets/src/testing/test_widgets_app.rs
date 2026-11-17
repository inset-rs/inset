//! flutter_test's `TestWidgetsApp`: a minimal [`WidgetsApp`] a case wraps its tree in.
//!
//! It gives a test the app-level widgets [`WidgetsApp`] provides — a [`Directionality`], a
//! [`MediaQuery`], a [`Localizations`] — and a [`Navigator`] with an [`Overlay`], without a
//! Material or Cupertino app. The page routes it builds have no transition, so a case that
//! navigates needs no settle.
//!
//! [`Directionality`]: crate::Directionality
//! [`MediaQuery`]: crate::MediaQuery
//! [`Localizations`]: crate::Localizations
//! [`Navigator`]: crate::Navigator
//! [`Overlay`]: crate::Overlay

use std::any::TypeId;
use std::collections::HashMap;
use std::fmt::{self, Debug};
use std::rc::Rc;

use waft_embedder::{Color, Locale};
use waft_foundation::App;
use waft_painting::TextStyle;

use crate::{
    ActionRef, BuildContext, GlobalKey, InitialRouteListFactory, IntoWidget, KeyRef,
    LocaleListResolutionCallback, LocalizationsDelegateRef, NavigatorObserverRef, PageRouteBuilder,
    PageRouteFactory, PageRouteRef, RouteFactory, RoutePageBuilder, RouteRef, RouteSettings,
    RouteSettingsRef, ShortcutMap, StatelessWidget, TransitionBuilder, WidgetBuilder, WidgetRef,
    WidgetsApp,
};

/// flutter_test's `TestWidgetsApp`: a minimal [`WidgetsApp`] for use in widget tests.
///
/// The page routes it builds have no transition, so a case navigates without pumping a settle.
pub struct TestWidgetsApp {
    /// See [`Widget::key`](crate::Widget::key).
    pub key: Option<KeyRef>,

    /// A key to use when building the [`Navigator`](crate::Navigator).
    pub navigator_key: Option<GlobalKey>,

    /// The widget displayed when the app launches.
    pub home: Option<WidgetRef>,

    /// The name of the first route to show when the app launches.
    pub initial_route: Option<String>,

    /// The route generator callback used when the app is navigated to a named route.
    pub on_generate_route: Option<RouteFactory>,

    /// The routes generator callback used for generating initial routes.
    pub on_generate_initial_routes: Option<InitialRouteListFactory>,

    /// Called when [`on_generate_route`](Self::on_generate_route) fails to generate a route.
    pub on_unknown_route: Option<RouteFactory>,

    /// The observers for the app's [`Navigator`](crate::Navigator).
    pub navigator_observers: Vec<NavigatorObserverRef>,

    /// The application's top-level routing table.
    pub routes: HashMap<String, WidgetBuilder>,

    /// The primary color to use for the application in the operating system interface.
    pub color: Color,

    /// The default text style for [`Text`](crate::Text) widgets in the app.
    pub text_style: Option<TextStyle>,

    /// A function that creates page routes for named navigation.
    pub page_route_builder: PageRouteFactory,

    /// A builder for inserting widgets above the [`Navigator`](crate::Navigator).
    pub builder: Option<TransitionBuilder>,

    /// The application's keyboard shortcut map.
    pub shortcuts: Option<ShortcutMap>,

    /// The application's action map.
    pub actions: Option<HashMap<TypeId, ActionRef>>,

    /// The identifier to use for state restoration of the app's [`Navigator`](crate::Navigator).
    pub restoration_scope_id: Option<String>,

    /// The delegates for this app's [`Localizations`](crate::Localizations).
    pub localizations_delegates: Option<Vec<LocalizationsDelegateRef>>,

    /// The callback responsible for choosing the app's locale when it is started.
    pub locale_list_resolution_callback: Option<LocaleListResolutionCallback>,
}

impl TestWidgetsApp {
    /// Creates a minimal [`WidgetsApp`] for testing.
    pub fn new() -> TestWidgetsApp {
        TestWidgetsApp {
            key: None,
            navigator_key: None,
            home: None,
            initial_route: None,
            on_generate_route: None,
            on_generate_initial_routes: None,
            on_unknown_route: None,
            navigator_observers: Vec::new(),
            routes: HashMap::new(),
            color: Color::new(0xFFFFFFFF),
            text_style: None,
            page_route_builder: Rc::new(TestWidgetsApp::default_page_route_builder),
            builder: None,
            shortcuts: None,
            actions: None,
            restoration_scope_id: None,
            localizations_delegates: None,
            locale_list_resolution_callback: None,
        }
    }

    /// Dart `TestWidgetsApp(key:)`.
    pub fn key(mut self, key: KeyRef) -> TestWidgetsApp {
        self.key = Some(key);
        self
    }

    /// Dart `TestWidgetsApp(navigatorKey:)`.
    pub fn navigator_key(mut self, navigator_key: GlobalKey) -> TestWidgetsApp {
        self.navigator_key = Some(navigator_key);
        self
    }

    /// Dart `TestWidgetsApp(home:)`.
    pub fn home<K>(mut self, home: impl IntoWidget<K>) -> TestWidgetsApp {
        self.home = Some(home.into_widget());
        self
    }

    /// Dart `TestWidgetsApp(initialRoute:)`.
    pub fn initial_route(mut self, initial_route: impl Into<String>) -> TestWidgetsApp {
        self.initial_route = Some(initial_route.into());
        self
    }

    /// Dart `TestWidgetsApp(onGenerateRoute:)`.
    pub fn on_generate_route(
        mut self,
        on_generate_route: impl Fn(&mut App, &RouteSettings) -> Option<RouteRef> + 'static,
    ) -> TestWidgetsApp {
        self.on_generate_route = Some(Rc::new(on_generate_route));
        self
    }

    /// Dart `TestWidgetsApp(onGenerateInitialRoutes:)`.
    pub fn on_generate_initial_routes(
        mut self,
        on_generate_initial_routes: impl Fn(&mut App, &str) -> Vec<RouteRef> + 'static,
    ) -> TestWidgetsApp {
        self.on_generate_initial_routes = Some(Rc::new(on_generate_initial_routes));
        self
    }

    /// Dart `TestWidgetsApp(onUnknownRoute:)`.
    pub fn on_unknown_route(
        mut self,
        on_unknown_route: impl Fn(&mut App, &RouteSettings) -> Option<RouteRef> + 'static,
    ) -> TestWidgetsApp {
        self.on_unknown_route = Some(Rc::new(on_unknown_route));
        self
    }

    /// Dart `TestWidgetsApp(navigatorObservers:)`.
    pub fn navigator_observers(
        mut self,
        navigator_observers: impl IntoIterator<Item = NavigatorObserverRef>,
    ) -> TestWidgetsApp {
        self.navigator_observers = navigator_observers.into_iter().collect();
        self
    }

    /// Dart `TestWidgetsApp(routes:)`.
    pub fn routes(
        mut self,
        routes: impl IntoIterator<Item = (String, WidgetBuilder)>,
    ) -> TestWidgetsApp {
        self.routes = routes.into_iter().collect();
        self
    }

    /// Dart `TestWidgetsApp(color:)`.
    pub fn color(mut self, color: Color) -> TestWidgetsApp {
        self.color = color;
        self
    }

    /// Dart `TestWidgetsApp(textStyle:)`.
    pub fn text_style(mut self, text_style: TextStyle) -> TestWidgetsApp {
        self.text_style = Some(text_style);
        self
    }

    /// Dart `TestWidgetsApp(pageRouteBuilder:)`.
    pub fn page_route_builder(
        mut self,
        page_route_builder: impl Fn(&mut App, &RouteSettings, WidgetBuilder) -> PageRouteRef + 'static,
    ) -> TestWidgetsApp {
        self.page_route_builder = Rc::new(page_route_builder);
        self
    }

    /// Dart `TestWidgetsApp(builder:)`.
    pub fn builder(
        mut self,
        builder: impl Fn(&mut App, BuildContext, Option<&WidgetRef>) -> WidgetRef + 'static,
    ) -> TestWidgetsApp {
        self.builder = Some(Rc::new(builder));
        self
    }

    /// Dart `TestWidgetsApp(shortcuts:)`.
    pub fn shortcuts(mut self, shortcuts: ShortcutMap) -> TestWidgetsApp {
        self.shortcuts = Some(shortcuts);
        self
    }

    /// Dart `TestWidgetsApp(actions:)`.
    pub fn actions(mut self, actions: HashMap<TypeId, ActionRef>) -> TestWidgetsApp {
        self.actions = Some(actions);
        self
    }

    /// Dart `TestWidgetsApp(restorationScopeId:)`.
    pub fn restoration_scope_id(
        mut self,
        restoration_scope_id: impl Into<String>,
    ) -> TestWidgetsApp {
        self.restoration_scope_id = Some(restoration_scope_id.into());
        self
    }

    /// Dart `TestWidgetsApp(localizationsDelegates:)`.
    pub fn localizations_delegates(
        mut self,
        localizations_delegates: impl IntoIterator<Item = LocalizationsDelegateRef>,
    ) -> TestWidgetsApp {
        self.localizations_delegates = Some(localizations_delegates.into_iter().collect());
        self
    }

    /// Dart `TestWidgetsApp(localeListResolutionCallback:)`.
    pub fn locale_list_resolution_callback(
        mut self,
        locale_list_resolution_callback: impl Fn(Option<&[Locale]>, &[Locale]) -> Option<Locale>
        + 'static,
    ) -> TestWidgetsApp {
        self.locale_list_resolution_callback = Some(Rc::new(locale_list_resolution_callback));
        self
    }
}

impl TestWidgetsApp {
    /// flutter_test's `TestWidgetsApp._defaultPageRouteBuilder`: a page with no transition.
    ///
    /// A case that pushes a route of its own builds it here, as it would pass the route's
    /// builder to a [`TestWidgetsApp`](Self)'s `page_route_builder`.
    pub fn default_page_route_builder(
        app: &mut App,
        settings: &RouteSettings,
        builder: WidgetBuilder,
    ) -> PageRouteRef {
        let page_builder: RoutePageBuilder =
            Rc::new(move |app, context, _animation, _secondary| builder(app, context));
        PageRouteBuilder::new(app, page_builder)
            .settings(app, RouteSettingsRef::Settings(settings.clone()))
            .into_erased()
    }
}

impl Default for TestWidgetsApp {
    fn default() -> TestWidgetsApp {
        TestWidgetsApp::new()
    }
}

impl Debug for TestWidgetsApp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TestWidgetsApp")
            .field("color", &self.color)
            .finish_non_exhaustive()
    }
}

impl StatelessWidget for TestWidgetsApp {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        let mut widgets_app = WidgetsApp::new(self.color);
        widgets_app.navigator_key = self.navigator_key.clone();
        widgets_app.on_generate_route = self.on_generate_route.clone();
        widgets_app.on_generate_initial_routes = self.on_generate_initial_routes.clone();
        widgets_app.page_route_builder = Some(Rc::clone(&self.page_route_builder));
        widgets_app.home = self.home.clone();
        widgets_app.routes = self.routes.clone();
        widgets_app.on_unknown_route = self.on_unknown_route.clone();
        widgets_app.initial_route = self.initial_route.clone();
        widgets_app.navigator_observers = self.navigator_observers.clone();
        widgets_app.builder = self.builder.clone();
        widgets_app.text_style = self.text_style.clone();
        widgets_app.localizations_delegates = self.localizations_delegates.clone();
        widgets_app.locale_list_resolution_callback = self.locale_list_resolution_callback.clone();
        widgets_app.shortcuts = self.shortcuts.clone();
        widgets_app.actions = self.actions.clone();
        widgets_app.restoration_scope_id = self.restoration_scope_id.clone();
        widgets_app.into_widget()
    }
}
