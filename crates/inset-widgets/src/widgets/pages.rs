//! Flutter counterpart: `widgets/pages.dart`.

use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use inset_animation::AnyAnimation;
use inset_foundation::{App, Handle, HandleId};
use inset_painting::Color;

use crate::framework::{BuildContext, WidgetRef};
use crate::widgets::navigator::{AnyRoute, Route, RouteData, RouteSettingsRef};
use crate::widgets::routes::{
    AnyModalRoute, AnyTransitionRoute, LocalHistoryRoute, LocalHistoryRouteData, ModalRoute,
    ModalRouteData, OverlayRoute, OverlayRouteData, PredictiveBackRoute, RoutePageBuilder,
    RouteTransitionsBuilder, TransitionRoute, TransitionRouteData,
};

/// The fields Dart's `PageRoute` declares; every page route carries this bag under the field
/// `page_route`.
pub struct PageRouteData {
    /// Whether this page route is a full-screen dialog.
    ///
    /// In Material and Cupertino, being fullscreen has the effects of making the app bars have
    /// a close button instead of a back button. On iOS, dialogs transitions animate differently
    /// and are also not closeable with the back swipe gesture.
    pub fullscreen_dialog: bool,
    /// See [`TransitionRoute::allow_snapshotting`].
    pub allow_snapshotting: bool,
    /// See [`ModalRoute::barrier_dismissible`].
    pub barrier_dismissible: bool,
}

impl PageRouteData {
    /// The bag of a freshly created page route; Dart's constructor defaults.
    pub fn new() -> PageRouteData {
        PageRouteData {
            fullscreen_dialog: false,
            allow_snapshotting: true,
            barrier_dismissible: false,
        }
    }
}

impl Default for PageRouteData {
    fn default() -> PageRouteData {
        PageRouteData::new()
    }
}

/// The accessors [`PageRoute`] asks for, for a struct whose bag is the field `page_route`.
#[macro_export]
macro_rules! page_route_accessors {
    () => {
        fn page_route_data(
            self: ::inset_foundation::Handle<Self>,
            app: &::inset_foundation::App,
        ) -> &$crate::PageRouteData {
            &app.get(self).page_route
        }

        fn page_route_data_mut(
            self: ::inset_foundation::Handle<Self>,
            app: &mut ::inset_foundation::App,
        ) -> &mut $crate::PageRouteData {
            &mut app.get_mut(self).page_route
        }
    };
}

/// A modal route that replaces the entire screen.
///
/// [`PageRouteBuilder`] provides a way to create a [`PageRoute`] using callbacks rather than by
/// defining a new type.
///
/// If [`barrier_dismissible`](ModalRoute::barrier_dismissible) is true, then pressing the
/// escape key on the keyboard will cause the current route to be popped with no value.
///
/// See also:
///
///  * [`Route`], which documents the meaning of Dart's `T` generic type argument.
pub trait PageRoute: ModalRoute {
    /// Dart's `PageRoute` fields, held under the field `page_route`
    /// ([`page_route_accessors!`](crate::page_route_accessors)).
    fn page_route_data(self: Handle<Self>, app: &App) -> &PageRouteData;

    /// See [`page_route_data`](Self::page_route_data).
    fn page_route_data_mut(self: Handle<Self>, app: &mut App) -> &mut PageRouteData;

    /// This route as the erased [`AnyPageRoute`].
    fn as_page_route(self: Handle<Self>) -> AnyPageRoute {
        AnyPageRoute {
            id: self.id(),
            vtable: const { &PageRouteVTable::of::<Self>() },
        }
    }

    /// Dart's `PageRoute.fullscreenDialog`.
    fn fullscreen_dialog(self: Handle<Self>, app: &App) -> bool {
        self.page_route_data(app).fullscreen_dialog
    }

    /// Dart's `PageRoute.allowSnapshotting`.
    fn allow_snapshotting(self: Handle<Self>, app: &App) -> bool {
        self.page_route_data(app).allow_snapshotting
    }

    /// Dart's `PageRoute.opaque`.
    fn opaque(self: Handle<Self>, app: &App) -> bool {
        let _ = app;
        true
    }

    /// Dart's `PageRoute.barrierDismissible`.
    fn barrier_dismissible(self: Handle<Self>, app: &App) -> bool {
        self.page_route_data(app).barrier_dismissible
    }

    /// Dart's `PageRoute.canTransitionTo`.
    fn can_transition_to(self: Handle<Self>, app: &App, next_route: AnyTransitionRoute) -> bool {
        next_route.as_route().as_page_route(app).is_some()
    }

    /// Dart's `PageRoute.canTransitionFrom`.
    fn can_transition_from(
        self: Handle<Self>,
        app: &App,
        previous_route: AnyTransitionRoute,
    ) -> bool {
        previous_route.as_route().as_page_route(app).is_some()
    }

    /// Dart's `PageRoute.popGestureEnabled`.
    fn pop_gesture_enabled(self: Handle<Self>, app: &mut App) -> bool {
        // Fullscreen dialogs aren't dismissible by back swipe.
        !PageRoute::fullscreen_dialog(self, app) && ModalRoute::pop_gesture_enabled(self, app)
    }
}

/// The vtable of an erased [`AnyPageRoute`].
struct PageRouteVTable {
    type_name: fn() -> &'static str,
    route: fn(HandleId) -> AnyRoute,
    modal_route: fn(HandleId) -> AnyModalRoute,
}

impl PageRouteVTable {
    /// The table for one page route type.
    const fn of<R: PageRoute>() -> PageRouteVTable {
        PageRouteVTable {
            type_name: std::any::type_name::<R>,
            route: |id| Route::as_route(resolve::<R>(id)),
            modal_route: |id| ModalRoute::as_modal_route(resolve::<R>(id)),
        }
    }
}

/// Erased [`PageRoute`]: what Dart's `route is PageRoute` produces.
#[derive(Clone, Copy)]
pub struct AnyPageRoute {
    id: HandleId,
    vtable: &'static PageRouteVTable,
}

impl PartialEq for AnyPageRoute {
    fn eq(&self, other: &AnyPageRoute) -> bool {
        self.id == other.id
    }
}

impl Eq for AnyPageRoute {}

impl Debug for AnyPageRoute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({:?})", (self.vtable.type_name)(), self.id)
    }
}

impl AnyPageRoute {
    /// This page route as the erased [`AnyRoute`].
    pub fn as_route(self) -> AnyRoute {
        (self.vtable.route)(self.id)
    }

    /// This page route as the erased [`AnyModalRoute`].
    pub fn as_modal_route(self) -> AnyModalRoute {
        (self.vtable.modal_route)(self.id)
    }
}

/// The typed handle for an erased id. Free: nothing is looked up; `get` checks the slot.
fn resolve<L: 'static>(id: HandleId) -> Handle<L> {
    Handle::from_id(id)
}

/// The `PredictiveBackRoute` implementation a [`PageRoute`] leaf inherits. Write it inside
/// `impl PredictiveBackRoute for MyRoute { .. }`.
#[macro_export]
macro_rules! page_predictive_back_route_overrides {
    () => {
        $crate::modal_predictive_back_route_overrides!($crate::PageRoute::pop_gesture_enabled);
    };
}

/// The `TransitionRoute` overrides a [`PageRoute`] leaf inherits, plus
/// [`modal_transition_route_overrides!`](crate::modal_transition_route_overrides). Write it
/// inside `impl TransitionRoute for MyRoute { .. }`, next to the leaf's own
/// `transition_duration` and `opaque` (which a leaf that does not override it writes as
/// `fn opaque(self: Handle<Self>, app: &App) -> bool { PageRoute::opaque(self, app) }`).
#[macro_export]
macro_rules! page_transition_route_overrides {
    () => {
        $crate::modal_transition_route_overrides!();

        fn allow_snapshotting(
            self: ::inset_foundation::Handle<Self>,
            app: &::inset_foundation::App,
        ) -> bool {
            $crate::PageRoute::allow_snapshotting(self, app)
        }

        fn can_transition_to(
            self: ::inset_foundation::Handle<Self>,
            app: &::inset_foundation::App,
            next_route: $crate::AnyTransitionRoute,
        ) -> bool {
            $crate::PageRoute::can_transition_to(self, app, next_route)
        }

        fn can_transition_from(
            self: ::inset_foundation::Handle<Self>,
            app: &::inset_foundation::App,
            previous_route: $crate::AnyTransitionRoute,
        ) -> bool {
            $crate::PageRoute::can_transition_from(self, app, previous_route)
        }
    };
}

/// The `Route` overrides a [`PageRoute`] leaf inherits, plus
/// [`modal_route_overrides!`](crate::modal_route_overrides). Write it inside
/// `impl Route for MyRoute { .. }`.
#[macro_export]
macro_rules! page_route_overrides {
    () => {
        $crate::modal_route_overrides!();

        fn as_page_route(
            self: ::inset_foundation::Handle<Self>,
        ) -> ::std::option::Option<$crate::AnyPageRoute> {
            ::std::option::Option::Some($crate::PageRoute::as_page_route(self))
        }
    };
}

/// Dart's `_defaultTransitionsBuilder`: a jump cut.
fn default_transitions_builder(
    _app: &mut App,
    _context: BuildContext,
    _animation: AnyAnimation<f64>,
    _secondary_animation: AnyAnimation<f64>,
    child: WidgetRef,
) -> WidgetRef {
    child
}

/// A utility type for defining one-off page routes in terms of callbacks.
///
/// Callers must define the [`page_builder`](Self::page_builder) function which creates the
/// route's primary contents. To add transitions define the
/// [`transitions_builder`](Self::transitions_builder) function.
///
/// See also:
///
///  * [`Route`], which documents the meaning of Dart's `T` generic type argument.
pub struct PageRouteBuilder {
    route: RouteData,
    overlay_route: OverlayRouteData,
    transition_route: TransitionRouteData,
    local_history: LocalHistoryRouteData,
    modal_route: ModalRouteData,
    page_route: PageRouteData,
    /// Used to build the route's primary contents.
    ///
    /// See [`ModalRoute::build_page`] for the complete definition of the parameters.
    pub page_builder: RoutePageBuilder,
    /// Used to build the route's transitions.
    ///
    /// The `animation` argument drives this route's own entrance and exit transition. The
    /// `secondary_animation` argument drives transitions for this route when another route is
    /// pushed on top of it or popped from above it, if both routes allow transition
    /// coordination. See [`TransitionRoute::can_transition_to`] and
    /// [`TransitionRoute::can_transition_from`].
    ///
    /// The default transition is a jump cut (i.e. no animation).
    pub transitions_builder: RouteTransitionsBuilder,
    transition_duration: Duration,
    reverse_transition_duration: Duration,
    opaque: bool,
    barrier_dismissible: bool,
    barrier_color: Option<Color>,
    barrier_label: Option<String>,
    maintain_state: bool,
}

impl PageRouteBuilder {
    /// Creates a route that delegates to builder callbacks; Dart's optional arguments are the
    /// setters.
    ///
    /// Dart's defaults: `transitions_builder` a jump cut, `transition_duration` and
    /// `reverse_transition_duration` 300 ms, `opaque` true, `barrier_dismissible` false,
    /// `maintain_state` true, `fullscreen_dialog` false, `allow_snapshotting` true.
    pub fn new(app: &mut App, page_builder: RoutePageBuilder) -> Handle<PageRouteBuilder> {
        let route = RouteData::new(app, None, None);
        let transition_route = TransitionRouteData::new(app);
        let modal_route = ModalRouteData::new(app);
        app.create(PageRouteBuilder {
            route,
            overlay_route: OverlayRouteData::new(),
            transition_route,
            local_history: LocalHistoryRouteData::new(),
            modal_route,
            page_route: PageRouteData::new(),
            page_builder,
            transitions_builder: Rc::new(default_transitions_builder),
            transition_duration: Duration::from_millis(300),
            reverse_transition_duration: Duration::from_millis(300),
            opaque: true,
            barrier_dismissible: false,
            barrier_color: None,
            barrier_label: None,
            maintain_state: true,
        })
    }

    /// Dart `PageRouteBuilder(settings:)`.
    pub fn settings(self: Handle<Self>, app: &mut App, settings: RouteSettingsRef) -> Handle<Self> {
        self.route_data_mut(app).set_settings(settings);
        self
    }

    /// Dart `PageRouteBuilder(requestFocus:)`.
    pub fn request_focus(self: Handle<Self>, app: &mut App, request_focus: bool) -> Handle<Self> {
        self.route_data_mut(app)
            .set_request_focus(Some(request_focus));
        self
    }

    /// Dart `PageRouteBuilder(transitionsBuilder:)`.
    pub fn transitions_builder(
        self: Handle<Self>,
        app: &mut App,
        transitions_builder: RouteTransitionsBuilder,
    ) -> Handle<Self> {
        app.get_mut(self).transitions_builder = transitions_builder;
        self
    }

    /// Dart `PageRouteBuilder(transitionDuration:)`.
    pub fn transition_duration(
        self: Handle<Self>,
        app: &mut App,
        transition_duration: Duration,
    ) -> Handle<Self> {
        app.get_mut(self).transition_duration = transition_duration;
        self
    }

    /// Dart `PageRouteBuilder(reverseTransitionDuration:)`.
    pub fn reverse_transition_duration(
        self: Handle<Self>,
        app: &mut App,
        reverse_transition_duration: Duration,
    ) -> Handle<Self> {
        app.get_mut(self).reverse_transition_duration = reverse_transition_duration;
        self
    }

    /// Dart `PageRouteBuilder(opaque:)`.
    pub fn opaque(self: Handle<Self>, app: &mut App, opaque: bool) -> Handle<Self> {
        app.get_mut(self).opaque = opaque;
        self
    }

    /// Dart `PageRouteBuilder(barrierDismissible:)`.
    pub fn barrier_dismissible(
        self: Handle<Self>,
        app: &mut App,
        barrier_dismissible: bool,
    ) -> Handle<Self> {
        app.get_mut(self).barrier_dismissible = barrier_dismissible;
        self
    }

    /// Dart `PageRouteBuilder(barrierColor:)`.
    pub fn barrier_color(self: Handle<Self>, app: &mut App, barrier_color: Color) -> Handle<Self> {
        app.get_mut(self).barrier_color = Some(barrier_color);
        self
    }

    /// Dart `PageRouteBuilder(barrierLabel:)`.
    pub fn barrier_label(self: Handle<Self>, app: &mut App, barrier_label: String) -> Handle<Self> {
        app.get_mut(self).barrier_label = Some(barrier_label);
        self
    }

    /// Dart `PageRouteBuilder(maintainState:)`.
    pub fn maintain_state(self: Handle<Self>, app: &mut App, maintain_state: bool) -> Handle<Self> {
        app.get_mut(self).maintain_state = maintain_state;
        self
    }

    /// Dart `PageRouteBuilder(fullscreenDialog:)`.
    pub fn fullscreen_dialog(
        self: Handle<Self>,
        app: &mut App,
        fullscreen_dialog: bool,
    ) -> Handle<Self> {
        self.page_route_data_mut(app).fullscreen_dialog = fullscreen_dialog;
        self
    }

    /// Dart `PageRouteBuilder(allowSnapshotting:)`.
    pub fn allow_snapshotting(
        self: Handle<Self>,
        app: &mut App,
        allow_snapshotting: bool,
    ) -> Handle<Self> {
        self.page_route_data_mut(app).allow_snapshotting = allow_snapshotting;
        self
    }
}

impl Route for PageRouteBuilder {
    crate::page_route_overrides!();
}

impl OverlayRoute for PageRouteBuilder {
    crate::modal_overlay_route_overrides!();
}

impl PredictiveBackRoute for PageRouteBuilder {
    crate::page_predictive_back_route_overrides!();
}

impl TransitionRoute for PageRouteBuilder {
    crate::page_transition_route_overrides!();

    fn transition_duration(self: Handle<Self>, app: &App) -> Duration {
        app.get(self).transition_duration
    }

    fn reverse_transition_duration(self: Handle<Self>, app: &App) -> Duration {
        app.get(self).reverse_transition_duration
    }

    fn opaque(self: Handle<Self>, app: &App) -> bool {
        app.get(self).opaque
    }
}

impl LocalHistoryRoute for PageRouteBuilder {
    crate::local_history_route_accessors!();
}

impl ModalRoute for PageRouteBuilder {
    crate::modal_route_accessors!();

    fn barrier_dismissible(self: Handle<Self>, app: &App) -> bool {
        app.get(self).barrier_dismissible
    }

    fn barrier_color(self: Handle<Self>, app: &App) -> Option<Color> {
        app.get(self).barrier_color
    }

    fn barrier_label(self: Handle<Self>, app: &App) -> Option<String> {
        app.get(self).barrier_label.clone()
    }

    fn maintain_state(self: Handle<Self>, app: &App) -> bool {
        app.get(self).maintain_state
    }

    fn fullscreen_dialog(self: Handle<Self>, app: &App) -> bool {
        PageRoute::fullscreen_dialog(self, app)
    }

    fn build_page(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        animation: AnyAnimation<f64>,
        secondary_animation: AnyAnimation<f64>,
    ) -> WidgetRef {
        let page_builder = Rc::clone(&app.get(self).page_builder);
        page_builder(app, context, animation, secondary_animation)
    }

    fn build_transitions(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        animation: AnyAnimation<f64>,
        secondary_animation: AnyAnimation<f64>,
        child: WidgetRef,
    ) -> WidgetRef {
        let transitions_builder = Rc::clone(&app.get(self).transitions_builder);
        transitions_builder(app, context, animation, secondary_animation, child)
    }
}

impl PageRoute for PageRouteBuilder {
    crate::page_route_accessors!();
}
