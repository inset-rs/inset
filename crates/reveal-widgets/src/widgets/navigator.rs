//! Flutter counterpart: `widgets/navigator.dart`.
//!
//! `Route` and its type-erased handle [`AnyRoute`], the [`Navigator`] widget and [`NavigatorState`],
//! the pages API with its [`TransitionDelegate`], the observers, and the restoration of the
//! history.

use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use reveal_embedder::{Clip, RestorationData, RestorationMap};
use reveal_foundation::{App, Handle, HandleId, ListenableObject, Listener, ValueNotifier};
use reveal_gestures::GestureBinding;
use reveal_rendering::RenderAbsorbPointer;
use reveal_scheduler::{
    FrameCallback, SchedulerBinding, SchedulerPhase, Ticker, TickerCallback, TickerFuture,
    TickerProviderObject,
};
use reveal_services::RestorationBucket;

use crate::framework::{
    BuildContext, GlobalKey, InheritedWidget, IntoWidget, KeyRef, Notification, State, StateData,
    StatefulWidget, WidgetRef, keys_equal,
};
use crate::widgets::basic::AbsorbPointer;
use crate::widgets::focus_manager::{FocusNode, FocusNodeLeaf, TraversalEdgeBehavior};
use crate::widgets::focus_scope::Focus;
use crate::widgets::focus_traversal::FocusTraversalGroup;
use crate::widgets::heroes::HeroController;
use crate::widgets::notification_listener::NotificationListener;
use crate::widgets::overlay::{Overlay, OverlayEntry, OverlayState};
use crate::widgets::restoration::{
    RestorableProperty, RestorablePropertyData, RestorationMixin, RestorationMixinData,
    UnmanagedRestorationScope,
};
use crate::widgets::restoration_properties::{RestorableInt, RestorableValue};
use crate::widgets::ticker_provider::{TickerProviderStateMixin, TickerProviderStateMixinData};

/// Duration for delay before refocusing in android so that the focus won't be interrupted.
#[expect(dead_code, reason = "read by the accessibility refocus, which waits")]
const K_ANDROID_REFOCUSING_DELAY_DURATION: Duration = Duration::from_millis(300);

/// A route's result: what `Navigator::pop` hands back to the code that pushed the route.
///
/// Dart's `T?` on `Route<T>`; erased here because a `Route` is reached through [`AnyRoute`].
pub type RouteResult = Option<Rc<dyn Any>>;

/// A callback that is run when a route's [`popped`](AnyRoute::when_popped) or
/// [`disposed`](AnyRoute::when_disposed) completion is reached.
///
/// Dart's `route.popped.then(..)`; there is no event loop, so the completion is a callback the
/// caller registers, resolved through a microtask exactly as `TickerFuture` resolves.
pub type RouteResultCallback = Rc<dyn Fn(&mut App, RouteResult)>;

/// Creates a route for the given route settings.
///
/// Used by [`Navigator::on_generate_route`].
pub type RouteFactory = Rc<dyn Fn(&mut App, &RouteSettings) -> Option<AnyRoute>>;

/// Creates a series of one or more routes.
///
/// Used by [`Navigator::on_generate_initial_routes`].
pub type RouteListFactory = Rc<dyn Fn(&mut App, Handle<NavigatorState>, &str) -> Vec<AnyRoute>>;

/// Creates a [`Route`] that is to be added to a [`Navigator`].
///
/// The route can be configured with the provided `arguments`. The provided `context` is the
/// [`BuildContext`] of the [`Navigator`] to which the route is added.
///
/// Used by the restorable methods of the [`Navigator`] that add anonymous routes (e.g.
/// [`NavigatorState::restorable_push`]).
pub type RestorableRouteBuilder =
    Rc<dyn Fn(&mut App, BuildContext, Option<Rc<dyn Any>>) -> AnyRoute>;

/// Signature for the [`NavigatorState::pop_until`] predicate argument.
pub type RoutePredicate = Rc<dyn Fn(&mut App, AnyRoute) -> bool>;

/// Signature for the [`Navigator::on_pop_page`] callback.
///
/// This callback must call [`AnyRoute::did_pop`] on the specified route and must properly
/// update the pages list the next time it is passed into [`Navigator::pages`] so that it no
/// longer includes the corresponding [`Page`].
pub type PopPageCallback = Rc<dyn Fn(&mut App, AnyRoute, RouteResult) -> bool>;

/// Signature for the [`Navigator::on_did_remove_page`] callback.
///
/// This must properly update the pages list the next time it is passed into
/// [`Navigator::pages`] so that it no longer includes the input `page`.
pub type DidRemovePageCallback = Rc<dyn Fn(&mut App, PageRef)>;

/// Indicates whether the current route should be popped.
///
/// Used as the return value for [`AnyRoute::will_pop`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoutePopDisposition {
    /// Pop the route.
    ///
    /// If [`AnyRoute::will_pop`] or [`AnyRoute::pop_disposition`] return
    /// [`Pop`](Self::Pop) then the back button will actually pop the current route.
    Pop,

    /// Do not pop the route.
    ///
    /// If [`AnyRoute::will_pop`] or [`AnyRoute::pop_disposition`] return
    /// [`DoNotPop`](Self::DoNotPop) then the back button will be ignored.
    DoNotPop,

    /// Delegate this to the next level of navigation.
    ///
    /// If [`AnyRoute::will_pop`] or [`AnyRoute::pop_disposition`] return
    /// [`Bubble`](Self::Bubble) then the back button will be handled by the platform's
    /// navigator, which will usually close the application.
    Bubble,
}

// ---------------------------------------------------------------------------------------------
// RouteSettings and Page

/// Data that might be useful in constructing a [`Route`].
#[derive(Clone, Default)]
pub struct RouteSettings {
    /// The name of the route (e.g., "/settings").
    ///
    /// If `None`, the route is anonymous.
    pub name: Option<String>,
    /// The arguments passed to this route.
    ///
    /// May be used when building the route, e.g. in [`Navigator::on_generate_route`].
    pub arguments: Option<Rc<dyn Any>>,
}

impl RouteSettings {
    /// Creates data used to construct routes; Dart's named arguments are the setters.
    pub fn new() -> RouteSettings {
        RouteSettings::default()
    }

    /// Dart `RouteSettings(name:)`.
    pub fn name(mut self, name: impl Into<String>) -> RouteSettings {
        self.name = Some(name.into());
        self
    }

    /// Dart `RouteSettings(arguments:)`.
    pub fn arguments(mut self, arguments: Rc<dyn Any>) -> RouteSettings {
        self.arguments = Some(arguments);
        self
    }
}

impl Debug for RouteSettings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.name {
            None => write!(f, "RouteSettings(none)"),
            Some(name) => write!(f, "RouteSettings(\"{name}\")"),
        }
    }
}

/// A shared [`Page`] (Dart's `Page<T>` reference).
pub type PageRef = Rc<dyn Page>;

/// Describes the configuration of a [`Route`].
///
/// [`can_pop`](Self::can_pop) and [`on_pop_invoked`](Self::on_pop_invoked) are used for
/// intercepting pops.
///
/// A page is also the [`RouteSettings`] of the route it creates: [`name`](Self::name) and
/// [`arguments`](Self::arguments) are Dart's inherited `RouteSettings` fields.
///
/// See also:
///
///  * [`Navigator::pages`], which accepts a list of [`Page`]s and updates its routes history.
pub trait Page: Debug + 'static {
    /// The concrete page, for Dart's `runtimeType` in [`can_update`](Self::can_update).
    fn as_any(&self) -> &dyn Any;

    /// The key associated with this page.
    ///
    /// This key will be used for comparing pages in [`can_update`](Self::can_update).
    fn key(&self) -> Option<&KeyRef> {
        None
    }

    /// See [`RouteSettings::name`].
    fn name(&self) -> Option<&str> {
        None
    }

    /// See [`RouteSettings::arguments`].
    fn arguments(&self) -> Option<&Rc<dyn Any>> {
        None
    }

    /// Restoration ID to save and restore the state of the [`Route`] configured by this page.
    ///
    /// If no restoration ID is provided, the [`Route`] will not restore its state.
    fn restoration_id(&self) -> Option<&str> {
        None
    }

    /// When false, blocks the associated route from being popped.
    ///
    /// If this is set to false for the first page in the [`Navigator`] it prevents the
    /// application from exiting.
    ///
    /// If there are any `PopScope` widgets in a route's widget subtree, each of their `can_pop`
    /// must be `true`, in addition to this `can_pop`, in order for the route to be able to pop.
    fn can_pop(&self) -> bool {
        true
    }

    /// Called after a pop on the associated route was handled.
    ///
    /// It's not possible to prevent the pop from happening at the time that this method is
    /// called; the pop has already happened. Use [`can_pop`](Self::can_pop) to disable pops in
    /// advance.
    ///
    /// This will still be called even when the pop is canceled.
    fn on_pop_invoked(&self, app: &mut App, did_pop: bool, result: RouteResult) {
        let _ = (app, did_pop, result);
    }

    /// Whether this page can be updated with the `other` page.
    ///
    /// Two pages are considered updatable if they have the same `runtimeType` and
    /// [`key`](Self::key).
    fn can_update(&self, other: &dyn Page) -> bool {
        other.as_any().type_id() == self.as_any().type_id() && keys_equal(other.key(), self.key())
    }

    /// Creates the [`Route`] that corresponds to this page.
    ///
    /// The created route must have its settings set to this page: `this` is the shared
    /// reference the navigator holds, which Dart writes as `this`.
    fn create_route(&self, app: &mut App, context: BuildContext, this: &PageRef) -> AnyRoute;
}

/// What a [`Route`]'s settings hold: plain [`RouteSettings`], or the [`Page`] that created the
/// route.
///
/// Dart's `Page<T> extends RouteSettings`, so `Route.settings` is either; `settings is Page` is
/// [`as_page`](Self::as_page).
#[derive(Clone)]
pub enum RouteSettingsRef {
    /// A pageless route's settings.
    Settings(RouteSettings),
    /// A page-based route's settings: the page itself.
    Page(PageRef),
}

impl RouteSettingsRef {
    /// See [`RouteSettings::name`].
    pub fn name(&self) -> Option<&str> {
        match self {
            RouteSettingsRef::Settings(settings) => settings.name.as_deref(),
            RouteSettingsRef::Page(page) => page.name(),
        }
    }

    /// See [`RouteSettings::arguments`].
    pub fn arguments(&self) -> Option<&Rc<dyn Any>> {
        match self {
            RouteSettingsRef::Settings(settings) => settings.arguments.as_ref(),
            RouteSettingsRef::Page(page) => page.arguments(),
        }
    }

    /// Dart's `settings is Page<Object?>`.
    pub fn as_page(&self) -> Option<&PageRef> {
        match self {
            RouteSettingsRef::Settings(_) => None,
            RouteSettingsRef::Page(page) => Some(page),
        }
    }
}

impl PartialEq for RouteSettingsRef {
    fn eq(&self, other: &RouteSettingsRef) -> bool {
        match (self, other) {
            (RouteSettingsRef::Page(a), RouteSettingsRef::Page(b)) => Rc::ptr_eq(a, b),
            (RouteSettingsRef::Settings(a), RouteSettingsRef::Settings(b)) => {
                a.name == b.name
                    && match (&a.arguments, &b.arguments) {
                        (None, None) => true,
                        (Some(a), Some(b)) => Rc::ptr_eq(a, b),
                        _ => false,
                    }
            }
            _ => false,
        }
    }
}

impl Debug for RouteSettingsRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RouteSettingsRef::Settings(settings) => Debug::fmt(settings, f),
            RouteSettingsRef::Page(page) => Debug::fmt(page, f),
        }
    }
}

impl From<RouteSettings> for RouteSettingsRef {
    fn from(settings: RouteSettings) -> RouteSettingsRef {
        RouteSettingsRef::Settings(settings)
    }
}

impl From<PageRef> for RouteSettingsRef {
    fn from(page: PageRef) -> RouteSettingsRef {
        RouteSettingsRef::Page(page)
    }
}

// ---------------------------------------------------------------------------------------------
// Route

/// Dart's `Completer<T?>`: the completion a caller registers a callback on.
///
/// Resolved callbacks run through a microtask, as `TickerFuture`'s do.
#[derive(Default)]
pub(crate) struct RouteCompleter {
    result: Option<RouteResult>,
    callbacks: Vec<RouteResultCallback>,
}

impl RouteCompleter {
    /// A completion nothing has resolved yet.
    pub(crate) fn new() -> RouteCompleter {
        RouteCompleter::default()
    }

    pub(crate) fn is_completed(&self) -> bool {
        self.result.is_some()
    }

    /// The result the completion resolved with, if it has.
    pub(crate) fn result(&self) -> Option<RouteResult> {
        self.result.clone()
    }

    /// Registers a callback on a completion that has not resolved yet.
    pub(crate) fn push(&mut self, callback: RouteResultCallback) {
        debug_assert!(!self.is_completed());
        self.callbacks.push(callback);
    }

    /// Resolves the completion, returning the callbacks the caller schedules as microtasks.
    pub(crate) fn complete(&mut self, result: RouteResult) -> Vec<RouteResultCallback> {
        debug_assert!(!self.is_completed());
        self.result = Some(result);
        std::mem::take(&mut self.callbacks)
    }
}

/// The fields Dart's `Route` declares; every route carries this bag under the field `route`.
pub struct RouteData {
    request_focus: Option<bool>,
    navigator: Option<Handle<NavigatorState>>,
    settings: RouteSettingsRef,
    restoration_scope_id: Handle<ValueNotifier<Option<String>>>,
    pop_completer: RouteCompleter,
    dispose_completer: RouteCompleter,
}

impl RouteData {
    /// The bag of a freshly created route: Dart's constructor defaults.
    ///
    /// `settings` is Dart's `settings ?? const RouteSettings()`, `request_focus` its
    /// `requestFocus`.
    pub fn new(
        app: &mut App,
        settings: Option<RouteSettingsRef>,
        request_focus: Option<bool>,
    ) -> RouteData {
        RouteData {
            request_focus,
            navigator: None,
            settings: settings
                .unwrap_or_else(|| RouteSettingsRef::Settings(RouteSettings::default())),
            restoration_scope_id: app.create(ValueNotifier::new(None)),
            pop_completer: RouteCompleter::default(),
            dispose_completer: RouteCompleter::default(),
        }
    }
}

impl RouteData {
    /// Dart's `Route(settings:)`, for a leaf whose constructor takes it as a fluent setter.
    pub fn set_settings(&mut self, settings: RouteSettingsRef) {
        self.settings = settings;
    }

    /// Dart's `Route(requestFocus:)`, for a leaf whose constructor takes it as a fluent setter.
    pub fn set_request_focus(&mut self, request_focus: Option<bool>) {
        self.request_focus = request_focus;
    }
}

/// The accessors [`Route`] asks for, for a struct whose bag is the field `route`.
#[macro_export]
macro_rules! route_accessors {
    () => {
        fn route_data(
            self: ::reveal_foundation::Handle<Self>,
            app: &::reveal_foundation::App,
        ) -> &$crate::RouteData {
            &app.get(self).route
        }

        fn route_data_mut(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
        ) -> &mut $crate::RouteData {
            &mut app.get_mut(self).route
        }
    };
}

/// An abstraction for an entry managed by a [`Navigator`].
///
/// This trait defines an abstract interface between the navigator and the "routes" that are
/// pushed on and popped off the navigator. Most routes have visual affordances, which they
/// place in the navigator's [`Overlay`] using one or more [`OverlayEntry`] objects.
///
/// A route can belong to a page if its settings are a [`Page`]. A page-based route, as opposed
/// to a pageless route, is created from [`Page::create_route`] during [`Navigator::pages`]
/// updates. The page associated with this route may change during the lifetime of the route. If
/// the [`Navigator`] updates the page of this route, it calls
/// [`changed_internal_state`](Self::changed_internal_state) to notify the route that the page
/// has been updated.
///
/// Dart's type argument `T` — the route's return type — is erased as [`RouteResult`]; a
/// concrete route keeps a typed accessor where one is possible.
///
/// The trait holds the virtuals and one-line forwarders for the shared members, which live on
/// [`AnyRoute`] because a route reaches its navigator and its neighbours through that handle.
pub trait Route: Sized + 'static {
    /// Dart's `Route` fields, held under the field `route` ([`route_accessors!`](crate::route_accessors)).
    fn route_data(self: Handle<Self>, app: &App) -> &RouteData;

    /// See [`route_data`](Self::route_data).
    fn route_data_mut(self: Handle<Self>, app: &mut App) -> &mut RouteData;

    /// This route as the erased [`AnyRoute`] — what to pass where a Dart API takes a `Route`.
    ///
    /// The object is untouched; this mints an erased second handle to it, so concrete members
    /// stay reachable through the typed one.
    fn as_route(self: Handle<Self>) -> AnyRoute {
        AnyRoute {
            id: self.id(),
            vtable: const { &RouteVTable::of::<Self>() },
        }
    }

    /// This route as the erased [`AnyTransitionRoute`](crate::AnyTransitionRoute), when it is
    /// a `TransitionRoute`.
    ///
    /// Dart's `route is TransitionRoute<dynamic>`; a `TransitionRoute` implementor overrides
    /// this to `Some(self.as_transition_route())`.
    fn as_transition_route(self: Handle<Self>) -> Option<crate::AnyTransitionRoute> {
        let _ = self;
        None
    }

    /// This route as the erased [`AnyModalRoute`](crate::AnyModalRoute), when it is a
    /// `ModalRoute`.
    ///
    /// Dart's `route is ModalRoute<dynamic>`; a `ModalRoute` implementor overrides this to
    /// `Some(self.as_modal_route())`.
    fn as_modal_route(self: Handle<Self>) -> Option<crate::AnyModalRoute> {
        let _ = self;
        None
    }

    /// This route as the erased [`AnyPageRoute`](crate::AnyPageRoute), when it is a
    /// `PageRoute`.
    ///
    /// Dart's `route is PageRoute<dynamic>`; a `PageRoute` implementor overrides this to
    /// `Some(self.as_page_route())`.
    fn as_page_route(self: Handle<Self>) -> Option<crate::AnyPageRoute> {
        let _ = self;
        None
    }

    /// Dart's `route is SomeMixin` for a mixin defined outside this crate: the erased handle of
    /// the interface `id` names, if this type implements it.
    ///
    /// An implementor answers each interface it implements with that interface's erased
    /// handle, boxed; [`AnyRoute::interface`] unboxes it. The default implements none.
    fn interface(self: Handle<Self>, id: TypeId) -> Option<Box<dyn Any>> {
        let _ = (self, id);
        None
    }

    /// The overlay entries of this route.
    ///
    /// These are typically populated by [`install`](Self::install). The [`Navigator`] is in
    /// charge of adding them to and removing them from the [`Overlay`].
    ///
    /// There must be at least one entry in this list after `install` has been invoked.
    fn overlay_entries(self: Handle<Self>, app: &App) -> Vec<Handle<OverlayEntry>> {
        RouteBase::overlay_entries(self, app)
    }

    /// Called when the route is inserted into the navigator.
    ///
    /// Use this to populate [`overlay_entries`](Self::overlay_entries).
    fn install(self: Handle<Self>, app: &mut App) {
        RouteBase::install(self, app)
    }

    /// Called after [`install`](Self::install) when the route is pushed onto the navigator.
    ///
    /// The returned value resolves when the push transition is complete.
    ///
    /// [`did_add`](Self::did_add) will be called instead of this when the route immediately
    /// appears on screen without any push transition.
    fn did_push(self: Handle<Self>, app: &mut App) -> Handle<TickerFuture> {
        RouteBase::did_push(self, app)
    }

    /// Called after [`install`](Self::install) when the route is added to the navigator.
    ///
    /// This method is called instead of [`did_push`](Self::did_push) when the route immediately
    /// appears on screen without any push transition.
    fn did_add(self: Handle<Self>, app: &mut App) {
        RouteBase::did_add(self, app)
    }

    /// Called after [`install`](Self::install) when the route replaced another in the navigator.
    fn did_replace(self: Handle<Self>, app: &mut App, old_route: Option<AnyRoute>) {
        RouteBase::did_replace(self, app, old_route)
    }

    /// Returns whether calling [`NavigatorState::maybe_pop`] when this route is current should
    /// do anything.
    ///
    /// Dart's deprecated `willPop`; [`pop_disposition`](Self::pop_disposition) is its
    /// replacement.
    fn will_pop(self: Handle<Self>, app: &mut App) -> RoutePopDisposition {
        RouteBase::will_pop(self, app)
    }

    /// Returns whether calling [`NavigatorState::maybe_pop`] when this route is current
    /// ([`is_current`](AnyRoute::is_current)) should do anything.
    ///
    /// By default, if a route is the first route in the history (i.e., if
    /// [`is_first`](AnyRoute::is_first)), it reports that pops should be bubbled
    /// ([`RoutePopDisposition::Bubble`]). This behavior prevents the user from popping the
    /// first route off the history and being stranded at a blank screen; instead, the larger
    /// scope is popped.
    ///
    /// In other cases, the default behavior is to accept the pop ([`RoutePopDisposition::Pop`]).
    fn pop_disposition(self: Handle<Self>, app: &mut App) -> RoutePopDisposition {
        RouteBase::pop_disposition(self, app)
    }

    /// Called after a route pop was handled.
    ///
    /// Even when the pop is canceled, for example by a `PopScope` widget, this will still be
    /// called. The `did_pop` parameter indicates whether or not the back navigation actually
    /// happened successfully.
    fn on_pop_invoked_with_result(
        self: Handle<Self>,
        app: &mut App,
        did_pop: bool,
        result: RouteResult,
    ) {
        RouteBase::on_pop_invoked_with_result(self, app, did_pop, result)
    }

    /// Whether calling [`did_pop`](Self::did_pop) would return false.
    fn will_handle_pop_internally(self: Handle<Self>, app: &App) -> bool {
        RouteBase::will_handle_pop_internally(self, app)
    }

    /// When this route is popped (see [`NavigatorState::pop`]) if the result isn't specified or
    /// if it is `None`, this value will be used instead.
    ///
    /// This fallback is implemented by [`did_complete`](Self::did_complete).
    fn current_result(self: Handle<Self>, app: &App) -> RouteResult {
        RouteBase::current_result(self, app)
    }

    /// A request was made to pop this route. If the route can handle it internally (e.g.
    /// because it has its own stack of internal state) then return false, otherwise return true
    /// (by returning the value of calling `Route::did_pop`). Returning false will prevent the
    /// default behavior of [`NavigatorState::pop`].
    ///
    /// When this function returns true, the navigator removes this route from the history but
    /// does not yet call [`dispose`](Self::dispose). Instead, it is the route's responsibility
    /// to call [`NavigatorState::finalize_route`], which will in turn call `dispose` on the
    /// route.
    fn did_pop(self: Handle<Self>, app: &mut App, result: RouteResult) -> bool {
        RouteBase::did_pop(self, app, result)
    }

    /// The route was popped or is otherwise being removed somewhat gracefully.
    ///
    /// This is called by [`did_pop`](Self::did_pop) and in response to
    /// [`NavigatorState::push_replacement`]. If `did_pop` was not called, then
    /// [`NavigatorState::finalize_route`] must be called immediately, and no exit animation
    /// will run.
    ///
    /// The `popped` completion is resolved by this method.
    fn did_complete(self: Handle<Self>, app: &mut App, result: RouteResult) {
        RouteBase::did_complete(self, app, result)
    }

    /// The given route, which was above this one, has been popped off the navigator.
    fn did_pop_next(self: Handle<Self>, app: &mut App, next_route: AnyRoute) {
        RouteBase::did_pop_next(self, app, next_route)
    }

    /// This route's next route has changed to the given new route.
    fn did_change_next(self: Handle<Self>, app: &mut App, next_route: Option<AnyRoute>) {
        RouteBase::did_change_next(self, app, next_route)
    }

    /// This route's previous route has changed to the given new route.
    fn did_change_previous(self: Handle<Self>, app: &mut App, previous_route: Option<AnyRoute>) {
        RouteBase::did_change_previous(self, app, previous_route)
    }

    /// Called whenever the internal state of the route has changed.
    ///
    /// This should be called whenever [`will_handle_pop_internally`](Self::will_handle_pop_internally),
    /// [`did_pop`](Self::did_pop), `ModalRoute::offstage`, or other internal state of the route
    /// changes value.
    fn changed_internal_state(self: Handle<Self>, app: &mut App) {
        RouteBase::changed_internal_state(self, app)
    }

    /// Called whenever the [`Navigator`] has updated in some manner that might affect routes,
    /// to indicate that the route may wish to rebuild as well.
    fn changed_external_state(self: Handle<Self>, app: &mut App) {
        RouteBase::changed_external_state(self, app)
    }

    /// Discards any resources used by the object.
    ///
    /// This method should not remove its [`overlay_entries`](Self::overlay_entries) from the
    /// [`Overlay`]. The object's owner is in charge of doing that.
    fn dispose(self: Handle<Self>, app: &mut App) {
        RouteBase::dispose(self, app)
    }

    // ---- one-line forwarders to the type-erased handle ----

    /// See [`AnyRoute::navigator`].
    fn navigator(self: Handle<Self>, app: &App) -> Option<Handle<NavigatorState>> {
        self.as_route().navigator(app)
    }

    /// See [`AnyRoute::settings`].
    fn settings(self: Handle<Self>, app: &App) -> RouteSettingsRef {
        self.as_route().settings(app)
    }

    /// See [`AnyRoute::is_page_based`].
    fn is_page_based(self: Handle<Self>, app: &App) -> bool {
        self.as_route().is_page_based(app)
    }

    /// See [`AnyRoute::request_focus`].
    fn request_focus(self: Handle<Self>, app: &App) -> bool {
        self.as_route().request_focus(app)
    }

    /// See [`AnyRoute::restoration_scope_id`].
    fn restoration_scope_id(
        self: Handle<Self>,
        app: &App,
    ) -> Handle<ValueNotifier<Option<String>>> {
        self.as_route().restoration_scope_id(app)
    }

    /// See [`AnyRoute::is_current`].
    fn is_current(self: Handle<Self>, app: &App) -> bool {
        self.as_route().is_current(app)
    }

    /// See [`AnyRoute::is_first`].
    fn is_first(self: Handle<Self>, app: &App) -> bool {
        self.as_route().is_first(app)
    }

    /// See [`AnyRoute::is_active`].
    fn is_active(self: Handle<Self>, app: &App) -> bool {
        self.as_route().is_active(app)
    }

    /// See [`AnyRoute::has_active_route_below`].
    fn has_active_route_below(self: Handle<Self>, app: &App) -> bool {
        self.as_route().has_active_route_below(app)
    }

    /// See [`AnyRoute::when_popped`].
    fn when_popped(self: Handle<Self>, app: &mut App, callback: RouteResultCallback) {
        self.as_route().when_popped(app, callback);
    }

    /// See [`AnyRoute::when_disposed`].
    fn when_disposed(self: Handle<Self>, app: &mut App, callback: RouteResultCallback) {
        self.as_route().when_disposed(app, callback);
    }
}

/// The bodies of Dart's `Route` class.
///
/// A subclass whose override runs Dart's `super.install()` calls [`RouteBase::install`]; a
/// [`Route`] default is one line that does the same. Every [`Route`] implementor gets these
/// bodies, so an override on the leaf never reaches them by accident.
pub trait RouteBase: Route {
    /// Dart's `Route.overlayEntries`.
    fn overlay_entries(self: Handle<Self>, app: &App) -> Vec<Handle<OverlayEntry>> {
        let _ = app;
        Vec::new()
    }

    /// Dart's `Route.install`.
    fn install(self: Handle<Self>, app: &mut App) {
        let _ = app;
    }

    /// Dart's `Route.didPush`.
    fn did_push(self: Handle<Self>, app: &mut App) -> Handle<TickerFuture> {
        let future = TickerFuture::complete(app);
        let route = self.as_route();
        future.when_complete(
            app,
            Listener::new(move |app| {
                if route.request_focus(app)
                    && let Some(navigator) = route.navigator(app)
                {
                    let focus_node = navigator.focus_node(app);
                    if let Some(scope) = focus_node.as_node().enclosing_scope(app) {
                        scope.request_focus(app, None);
                    }
                }
            }),
        );
        future
    }

    /// Dart's `Route.didAdd`.
    fn did_add(self: Handle<Self>, app: &mut App) {
        if !self.as_route().request_focus(app) {
            return;
        }
        // This completion serves two purposes. First, we want to make sure that animations
        // triggered by other operations will finish before focusing the navigator. Second,
        // `navigator.focus_node` might acquire more focused children in `install`
        // asynchronously.
        let route = self.as_route();
        let future = TickerFuture::complete(app);
        future.when_complete(
            app,
            Listener::new(move |app| {
                // The route can be disposed before the completion runs.
                if let Some(navigator) = route.navigator(app) {
                    let focus_node = navigator.focus_node(app);
                    if let Some(scope) = focus_node.as_node().enclosing_scope(app) {
                        scope.request_focus(app, None);
                    }
                }
            }),
        );
    }

    /// Dart's `Route.didReplace`.
    fn did_replace(self: Handle<Self>, app: &mut App, old_route: Option<AnyRoute>) {
        let _ = (app, old_route);
    }

    /// Dart's `Route.willPop`.
    fn will_pop(self: Handle<Self>, app: &mut App) -> RoutePopDisposition {
        if self.as_route().is_first(app) {
            RoutePopDisposition::Bubble
        } else {
            RoutePopDisposition::Pop
        }
    }

    /// Dart's `Route.popDisposition`.
    fn pop_disposition(self: Handle<Self>, app: &mut App) -> RoutePopDisposition {
        if let Some(page) = self.route_data(app).settings.as_page()
            && !page.can_pop()
        {
            return RoutePopDisposition::DoNotPop;
        }
        if self.as_route().is_first(app) {
            RoutePopDisposition::Bubble
        } else {
            RoutePopDisposition::Pop
        }
    }

    /// Dart's `Route.onPopInvokedWithResult`.
    fn on_pop_invoked_with_result(
        self: Handle<Self>,
        app: &mut App,
        did_pop: bool,
        result: RouteResult,
    ) {
        if let Some(page) = self.route_data(app).settings.as_page().cloned() {
            page.on_pop_invoked(app, did_pop, result);
        }
    }

    /// Dart's `Route.willHandlePopInternally`.
    fn will_handle_pop_internally(self: Handle<Self>, app: &App) -> bool {
        let _ = app;
        false
    }

    /// Dart's `Route.currentResult`.
    fn current_result(self: Handle<Self>, app: &App) -> RouteResult {
        let _ = app;
        None
    }

    /// Dart's `Route.didPop`.
    fn did_pop(self: Handle<Self>, app: &mut App, result: RouteResult) -> bool {
        Route::did_complete(self, app, result);
        true
    }

    /// Dart's `Route.didComplete`.
    fn did_complete(self: Handle<Self>, app: &mut App, result: RouteResult) {
        let result = result.or_else(|| Route::current_result(self, app));
        self.as_route().complete_popped(app, result);
    }

    /// Dart's `Route.didPopNext`.
    fn did_pop_next(self: Handle<Self>, app: &mut App, next_route: AnyRoute) {
        let _ = (app, next_route);
    }

    /// Dart's `Route.didChangeNext`.
    fn did_change_next(self: Handle<Self>, app: &mut App, next_route: Option<AnyRoute>) {
        let _ = (app, next_route);
    }

    /// Dart's `Route.didChangePrevious`.
    fn did_change_previous(self: Handle<Self>, app: &mut App, previous_route: Option<AnyRoute>) {
        let _ = (app, previous_route);
    }

    /// Dart's `Route.changedInternalState`.
    fn changed_internal_state(self: Handle<Self>, app: &mut App) {
        let _ = app;
    }

    /// Dart's `Route.changedExternalState`.
    fn changed_external_state(self: Handle<Self>, app: &mut App) {
        let _ = app;
    }

    /// Dart's `Route.dispose`.
    fn dispose(self: Handle<Self>, app: &mut App) {
        self.route_data_mut(app).navigator = None;
        let notifier = self.route_data(app).restoration_scope_id;
        app.get_mut(notifier).dispose();
        self.as_route().complete_disposed(app);
    }
}

impl<R: Route> RouteBase for R {}

/// The typed handle for an erased id. Free: nothing is looked up; `get` checks the slot.
fn resolve<L: 'static>(id: HandleId) -> Handle<L> {
    Handle::from_id(id)
}

/// The vtable of an erased [`AnyRoute`]: one `&'static` table per route type, built by
/// [`RouteVTable::of`].
struct RouteVTable {
    type_name: fn() -> &'static str,
    data: fn(&App, HandleId) -> &RouteData,
    data_mut: fn(&mut App, HandleId) -> &mut RouteData,
    overlay_entries: fn(&App, HandleId) -> Vec<Handle<OverlayEntry>>,
    install: fn(&mut App, HandleId),
    did_push: fn(&mut App, HandleId) -> Handle<TickerFuture>,
    did_add: fn(&mut App, HandleId),
    did_replace: fn(&mut App, HandleId, Option<AnyRoute>),
    will_pop: fn(&mut App, HandleId) -> RoutePopDisposition,
    pop_disposition: fn(&mut App, HandleId) -> RoutePopDisposition,
    on_pop_invoked_with_result: fn(&mut App, HandleId, bool, RouteResult),
    will_handle_pop_internally: fn(&App, HandleId) -> bool,
    current_result: fn(&App, HandleId) -> RouteResult,
    did_pop: fn(&mut App, HandleId, RouteResult) -> bool,
    did_complete: fn(&mut App, HandleId, RouteResult),
    did_pop_next: fn(&mut App, HandleId, AnyRoute),
    did_change_next: fn(&mut App, HandleId, Option<AnyRoute>),
    did_change_previous: fn(&mut App, HandleId, Option<AnyRoute>),
    changed_internal_state: fn(&mut App, HandleId),
    changed_external_state: fn(&mut App, HandleId),
    dispose: fn(&mut App, HandleId),
    as_transition_route: fn(HandleId) -> Option<crate::AnyTransitionRoute>,
    as_modal_route: fn(HandleId) -> Option<crate::AnyModalRoute>,
    as_page_route: fn(HandleId) -> Option<crate::AnyPageRoute>,
    interface: fn(HandleId, TypeId) -> Option<Box<dyn Any>>,
}

impl RouteVTable {
    /// The table for one route type.
    const fn of<R: Route>() -> RouteVTable {
        RouteVTable {
            type_name: std::any::type_name::<R>,
            data: |app, id| R::route_data(resolve(id), app),
            data_mut: |app, id| R::route_data_mut(resolve(id), app),
            overlay_entries: |app, id| R::overlay_entries(resolve(id), app),
            install: |app, id| R::install(resolve(id), app),
            did_push: |app, id| R::did_push(resolve(id), app),
            did_add: |app, id| R::did_add(resolve(id), app),
            did_replace: |app, id, old| R::did_replace(resolve(id), app, old),
            will_pop: |app, id| R::will_pop(resolve(id), app),
            pop_disposition: |app, id| R::pop_disposition(resolve(id), app),
            on_pop_invoked_with_result: |app, id, did_pop, result| {
                R::on_pop_invoked_with_result(resolve(id), app, did_pop, result)
            },
            will_handle_pop_internally: |app, id| R::will_handle_pop_internally(resolve(id), app),
            current_result: |app, id| R::current_result(resolve(id), app),
            did_pop: |app, id, result| R::did_pop(resolve(id), app, result),
            did_complete: |app, id, result| R::did_complete(resolve(id), app, result),
            did_pop_next: |app, id, next| R::did_pop_next(resolve(id), app, next),
            did_change_next: |app, id, next| R::did_change_next(resolve(id), app, next),
            did_change_previous: |app, id, prev| R::did_change_previous(resolve(id), app, prev),
            changed_internal_state: |app, id| R::changed_internal_state(resolve(id), app),
            changed_external_state: |app, id| R::changed_external_state(resolve(id), app),
            dispose: |app, id| R::dispose(resolve(id), app),
            as_transition_route: |id| R::as_transition_route(resolve::<R>(id)),
            as_modal_route: |id| R::as_modal_route(resolve::<R>(id)),
            as_page_route: |id| R::as_page_route(resolve::<R>(id)),
            interface: |id, interface| R::interface(resolve::<R>(id), interface),
        }
    }
}

/// Erased [`Route`]: one identity and a static vtable, the fat pointer rustc cannot build for
/// an arena id. No lease.
///
/// This is what a field or parameter Dart types as `Route<dynamic>` becomes; the route stays in
/// the [`App`] under its own type, and [`downcast`](Self::downcast) gets the typed handle back.
///
/// Equality is Dart's `==` on an object reference: two handles are equal exactly when they address
/// the same route.
#[derive(Clone, Copy)]
pub struct AnyRoute {
    id: HandleId,
    vtable: &'static RouteVTable,
}

impl PartialEq for AnyRoute {
    fn eq(&self, other: &AnyRoute) -> bool {
        self.id == other.id
    }
}

impl Eq for AnyRoute {}

impl std::hash::Hash for AnyRoute {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Debug for AnyRoute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({:?})", (self.vtable.type_name)(), self.id)
    }
}

impl AnyRoute {
    /// The arena id behind this handle.
    pub fn id(self) -> HandleId {
        self.id
    }

    /// Dart's `route as T`: the typed handle when this route is a `T`, else `None`.
    pub fn downcast<T: 'static>(self, app: &App) -> Option<Handle<T>> {
        app.handle::<T>(self.id)
    }

    /// Dart's `route is TransitionRoute<dynamic>`.
    pub fn as_transition_route(self, app: &App) -> Option<crate::AnyTransitionRoute> {
        let _ = app;
        (self.vtable.as_transition_route)(self.id)
    }

    /// Dart's `route is ModalRoute<dynamic>`.
    pub fn as_modal_route(self, app: &App) -> Option<crate::AnyModalRoute> {
        let _ = app;
        (self.vtable.as_modal_route)(self.id)
    }

    /// Dart's `route is PageRoute<dynamic>`.
    pub fn as_page_route(self, app: &App) -> Option<crate::AnyPageRoute> {
        let _ = app;
        (self.vtable.as_page_route)(self.id)
    }

    /// See [`Route::interface`]: Dart's `route is SomeMixin`, as
    /// `interface::<Rc<dyn SomeMixin>>()`.
    pub fn interface<I: 'static>(self) -> Option<I> {
        (self.vtable.interface)(self.id, TypeId::of::<I>())?
            .downcast()
            .ok()
            .map(|interface| *interface)
    }

    fn data(self, app: &App) -> &RouteData {
        (self.vtable.data)(app, self.id)
    }

    fn data_mut(self, app: &mut App) -> &mut RouteData {
        (self.vtable.data_mut)(app, self.id)
    }

    /// When the route state is updated, request focus if the current route is at the top.
    ///
    /// If not provided in the constructor, [`Navigator::request_focus`] is used instead.
    pub fn request_focus(self, app: &App) -> bool {
        if let Some(value) = self.data(app).request_focus {
            return value;
        }
        match self.navigator(app) {
            None => false,
            Some(navigator) => navigator.widget(app).request_focus,
        }
    }

    /// The navigator that the route is in, if any.
    pub fn navigator(self, app: &App) -> Option<Handle<NavigatorState>> {
        self.data(app).navigator
    }

    pub(crate) fn set_navigator(self, app: &mut App, navigator: Option<Handle<NavigatorState>>) {
        self.data_mut(app).navigator = navigator;
    }

    /// Dart's `Route._installed`.
    pub(crate) fn installed(self, app: &App) -> bool {
        self.data(app).navigator.is_some()
    }

    pub(crate) fn is_installed_in(self, app: &App, state: Handle<NavigatorState>) -> bool {
        self.data(app).navigator == Some(state)
    }

    /// The settings for this route.
    ///
    /// The settings can change during the route's lifetime. If the settings change, the route's
    /// overlays will be marked dirty (see [`changed_internal_state`](Self::changed_internal_state)).
    ///
    /// If the route is created from a [`Page`] in the [`Navigator::pages`] list, then this will
    /// be a page, and it will be updated each time its corresponding page has changed.
    pub fn settings(self, app: &App) -> RouteSettingsRef {
        self.data(app).settings.clone()
    }

    /// Dart's `Route._isPageBased`.
    pub fn is_page_based(self, app: &App) -> bool {
        matches!(self.data(app).settings, RouteSettingsRef::Page(_))
    }

    /// The restoration scope ID to be used for the `RestorationScope` surrounding this route.
    ///
    /// The restoration scope ID is `None` if restoration is currently disabled for this route.
    ///
    /// If the restoration scope ID changes (e.g. because restoration is enabled or disabled)
    /// during the life of the route, the notifier notifies its listeners.
    pub fn restoration_scope_id(self, app: &App) -> Handle<ValueNotifier<Option<String>>> {
        self.data(app).restoration_scope_id
    }

    fn update_settings(self, app: &mut App, new_settings: RouteSettingsRef) {
        if self.data(app).settings != new_settings {
            self.data_mut(app).settings = new_settings;
            if self.installed(app) {
                self.changed_internal_state(app);
            }
        }
    }

    fn update_restoration_id(self, app: &mut App, restoration_id: Option<String>) {
        let notifier = self.data(app).restoration_scope_id;
        notifier.set_value(app, restoration_id);
    }

    // ---- the completions Dart writes as `popped` and `_disposeCompleter` ----

    /// Registers a callback that runs when this route is popped off the navigator.
    ///
    /// The callback receives the value given to [`NavigatorState::pop`], if any, or else the
    /// value of [`current_result`](Self::current_result). It runs through a microtask, never
    /// inline, as a Dart `.then` on a resolved future does.
    pub fn when_popped(self, app: &mut App, callback: RouteResultCallback) {
        match self.data(app).pop_completer.result.clone() {
            None => self.data_mut(app).pop_completer.callbacks.push(callback),
            Some(result) => {
                app.schedule_microtask(Listener::new(move |app| {
                    callback(app, result.clone());
                }));
            }
        }
    }

    /// Whether the `popped` completion has been resolved (Dart's `_popCompleter.isCompleted`).
    pub fn popped_is_completed(self, app: &App) -> bool {
        self.data(app).pop_completer.is_completed()
    }

    /// Registers a callback that runs when this route is disposed.
    pub fn when_disposed(self, app: &mut App, callback: RouteResultCallback) {
        match self.data(app).dispose_completer.result.clone() {
            None => self
                .data_mut(app)
                .dispose_completer
                .callbacks
                .push(callback),
            Some(result) => {
                app.schedule_microtask(Listener::new(move |app| {
                    callback(app, result.clone());
                }));
            }
        }
    }

    fn complete_popped(self, app: &mut App, result: RouteResult) {
        debug_assert!(!self.data(app).pop_completer.is_completed());
        let completer = &mut self.data_mut(app).pop_completer;
        completer.result = Some(result.clone());
        let callbacks = std::mem::take(&mut completer.callbacks);
        for callback in callbacks {
            let result = result.clone();
            app.schedule_microtask(Listener::new(move |app| callback(app, result.clone())));
        }
    }

    fn complete_disposed(self, app: &mut App) {
        if self.data(app).dispose_completer.is_completed() {
            return;
        }
        let completer = &mut self.data_mut(app).dispose_completer;
        completer.result = Some(None);
        let callbacks = std::mem::take(&mut completer.callbacks);
        for callback in callbacks {
            app.schedule_microtask(Listener::new(move |app| callback(app, None)));
        }
    }

    // ---- the virtuals ----

    /// See [`Route::overlay_entries`].
    pub fn overlay_entries(self, app: &App) -> Vec<Handle<OverlayEntry>> {
        (self.vtable.overlay_entries)(app, self.id)
    }

    /// See [`Route::install`].
    pub fn install(self, app: &mut App) {
        (self.vtable.install)(app, self.id)
    }

    /// See [`Route::did_push`].
    pub fn did_push(self, app: &mut App) -> Handle<TickerFuture> {
        (self.vtable.did_push)(app, self.id)
    }

    /// See [`Route::did_add`].
    pub fn did_add(self, app: &mut App) {
        (self.vtable.did_add)(app, self.id)
    }

    /// See [`Route::did_replace`].
    pub fn did_replace(self, app: &mut App, old_route: Option<AnyRoute>) {
        (self.vtable.did_replace)(app, self.id, old_route)
    }

    /// See [`Route::will_pop`].
    pub fn will_pop(self, app: &mut App) -> RoutePopDisposition {
        (self.vtable.will_pop)(app, self.id)
    }

    /// See [`Route::pop_disposition`].
    pub fn pop_disposition(self, app: &mut App) -> RoutePopDisposition {
        (self.vtable.pop_disposition)(app, self.id)
    }

    /// See [`Route::on_pop_invoked_with_result`].
    pub fn on_pop_invoked_with_result(self, app: &mut App, did_pop: bool, result: RouteResult) {
        (self.vtable.on_pop_invoked_with_result)(app, self.id, did_pop, result)
    }

    /// See [`Route::will_handle_pop_internally`].
    pub fn will_handle_pop_internally(self, app: &App) -> bool {
        (self.vtable.will_handle_pop_internally)(app, self.id)
    }

    /// See [`Route::current_result`].
    pub fn current_result(self, app: &App) -> RouteResult {
        (self.vtable.current_result)(app, self.id)
    }

    /// See [`Route::did_pop`].
    pub fn did_pop(self, app: &mut App, result: RouteResult) -> bool {
        (self.vtable.did_pop)(app, self.id, result)
    }

    /// See [`Route::did_complete`].
    pub fn did_complete(self, app: &mut App, result: RouteResult) {
        (self.vtable.did_complete)(app, self.id, result)
    }

    /// See [`Route::did_pop_next`].
    pub fn did_pop_next(self, app: &mut App, next_route: AnyRoute) {
        (self.vtable.did_pop_next)(app, self.id, next_route)
    }

    /// See [`Route::did_change_next`].
    pub fn did_change_next(self, app: &mut App, next_route: Option<AnyRoute>) {
        (self.vtable.did_change_next)(app, self.id, next_route)
    }

    /// See [`Route::did_change_previous`].
    pub fn did_change_previous(self, app: &mut App, previous_route: Option<AnyRoute>) {
        (self.vtable.did_change_previous)(app, self.id, previous_route)
    }

    /// See [`Route::changed_internal_state`].
    pub fn changed_internal_state(self, app: &mut App) {
        (self.vtable.changed_internal_state)(app, self.id)
    }

    /// See [`Route::changed_external_state`].
    pub fn changed_external_state(self, app: &mut App) {
        (self.vtable.changed_external_state)(app, self.id)
    }

    /// See [`Route::dispose`].
    pub fn dispose(self, app: &mut App) {
        (self.vtable.dispose)(app, self.id)
    }

    // ---- the position of this route in its navigator's history ----

    /// Whether this route is the top-most route on the navigator.
    ///
    /// If this is true, then [`is_active`](Self::is_active) is also true.
    pub fn is_current(self, app: &App) -> bool {
        let Some(navigator) = self.navigator(app) else {
            return false;
        };
        match navigator.last_route_entry_where_or_null(app, RouteEntry::is_present_predicate) {
            None => false,
            Some(entry) => app.get(entry).route == self,
        }
    }

    /// Whether this route is the bottom-most active route on the navigator.
    ///
    /// If [`is_first`](Self::is_first) and [`is_current`](Self::is_current) are both true then
    /// this is the only route on the navigator.
    pub fn is_first(self, app: &App) -> bool {
        let Some(navigator) = self.navigator(app) else {
            return false;
        };
        match navigator.first_route_entry_where_or_null(app, RouteEntry::is_present_predicate) {
            None => false,
            Some(entry) => app.get(entry).route == self,
        }
    }

    /// Whether there is at least one active route underneath this route.
    pub fn has_active_route_below(self, app: &App) -> bool {
        let Some(navigator) = self.navigator(app) else {
            return false;
        };
        for entry in navigator.history_entries(app) {
            if app.get(entry).route == self {
                return false;
            }
            if RouteEntry::is_present_predicate(app, entry) {
                return true;
            }
        }
        false
    }

    /// Whether this route is on the navigator.
    ///
    /// If the route is not only active, but also the current route (the top-most route), then
    /// [`is_current`](Self::is_current) will also be true. If it is the first route (the
    /// bottom-most route), then [`is_first`](Self::is_first) will also be true.
    pub fn is_active(self, app: &App) -> bool {
        let Some(navigator) = self.navigator(app) else {
            return false;
        };
        navigator
            .first_route_entry_where_or_null(app, move |app, entry| app.get(entry).route == self)
            .is_some_and(|entry| RouteEntry::is_present_predicate(app, entry))
    }
}

// ---------------------------------------------------------------------------------------------
// NavigatorObserver

/// The fields Dart's `NavigatorObserver` keeps in its `Expando`; every observer carries this
/// bag under the field `navigator_observer`.
#[derive(Default)]
pub struct NavigatorObserverData {
    navigator: Option<Handle<NavigatorState>>,
}

impl NavigatorObserverData {
    /// An observer that is not observing a navigator yet.
    pub fn new() -> NavigatorObserverData {
        NavigatorObserverData::default()
    }
}

/// The accessors [`NavigatorObserver`] asks for, for a struct whose bag is the field
/// `navigator_observer`.
#[macro_export]
macro_rules! navigator_observer_accessors {
    () => {
        fn navigator_observer_data(
            self: ::reveal_foundation::Handle<Self>,
            app: &::reveal_foundation::App,
        ) -> &$crate::NavigatorObserverData {
            &app.get(self).navigator_observer
        }

        fn navigator_observer_data_mut(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
        ) -> &mut $crate::NavigatorObserverData {
            &mut app.get_mut(self).navigator_observer
        }
    };
}

/// An interface for observing the behavior of a [`Navigator`].
pub trait NavigatorObserver: Sized + 'static {
    /// Dart's `NavigatorObserver._navigators` entry, held under the field `navigator_observer`
    /// ([`navigator_observer_accessors!`](crate::navigator_observer_accessors)).
    fn navigator_observer_data(self: Handle<Self>, app: &App) -> &NavigatorObserverData;

    /// See [`navigator_observer_data`](Self::navigator_observer_data).
    fn navigator_observer_data_mut(self: Handle<Self>, app: &mut App)
    -> &mut NavigatorObserverData;

    /// This observer as the erased [`AnyNavigatorObserver`].
    fn as_observer(self: Handle<Self>) -> AnyNavigatorObserver {
        AnyNavigatorObserver {
            id: self.id(),
            vtable: const { &NavigatorObserverVTable::of::<Self>() },
        }
    }

    /// The navigator that the observer is observing, if any.
    fn navigator(self: Handle<Self>, app: &App) -> Option<Handle<NavigatorState>> {
        self.navigator_observer_data(app).navigator
    }

    /// The [`Navigator`] pushed `route`.
    ///
    /// The route immediately below that one, and thus the previously active route, is
    /// `previous_route`.
    fn did_push(
        self: Handle<Self>,
        app: &mut App,
        route: AnyRoute,
        previous_route: Option<AnyRoute>,
    ) {
        let _ = (app, route, previous_route);
    }

    /// The [`Navigator`] popped `route`.
    ///
    /// The route immediately below that one, and thus the newly active route, is
    /// `previous_route`.
    fn did_pop(
        self: Handle<Self>,
        app: &mut App,
        route: AnyRoute,
        previous_route: Option<AnyRoute>,
    ) {
        let _ = (app, route, previous_route);
    }

    /// The [`Navigator`] removed `route`.
    ///
    /// If only one route is being removed, then the route immediately below that one, if any,
    /// is `previous_route`.
    ///
    /// If multiple routes are being removed, then the route below the bottommost route being
    /// removed, if any, is `previous_route`, and this method will be called once for each
    /// removed route, from the topmost route to the bottommost route.
    fn did_remove(
        self: Handle<Self>,
        app: &mut App,
        route: AnyRoute,
        previous_route: Option<AnyRoute>,
    ) {
        let _ = (app, route, previous_route);
    }

    /// The [`Navigator`] replaced `old_route` with `new_route`.
    fn did_replace(
        self: Handle<Self>,
        app: &mut App,
        new_route: Option<AnyRoute>,
        old_route: Option<AnyRoute>,
    ) {
        let _ = (app, new_route, old_route);
    }

    /// The top most route has changed.
    ///
    /// The `top_route` is the new top most route. This can be a new route pushed on top of the
    /// screen, or an existing route that becomes the new top-most route because the previous
    /// top-most route has been popped.
    ///
    /// The `previous_top_route` was the top most route before the change.
    fn did_change_top(
        self: Handle<Self>,
        app: &mut App,
        top_route: AnyRoute,
        previous_top_route: Option<AnyRoute>,
    ) {
        let _ = (app, top_route, previous_top_route);
    }

    /// The [`Navigator`]'s routes are being moved by a user gesture.
    ///
    /// For example, this is called when an iOS back gesture starts, and is used to disable hero
    /// animations during such interactions.
    fn did_start_user_gesture(
        self: Handle<Self>,
        app: &mut App,
        route: AnyRoute,
        previous_route: Option<AnyRoute>,
    ) {
        let _ = (app, route, previous_route);
    }

    /// User gesture is no longer controlling the [`Navigator`].
    ///
    /// Paired with an earlier call to [`did_start_user_gesture`](Self::did_start_user_gesture).
    fn did_stop_user_gesture(self: Handle<Self>, app: &mut App) {
        let _ = app;
    }
}

/// The vtable of an erased [`AnyNavigatorObserver`].
struct NavigatorObserverVTable {
    type_name: fn() -> &'static str,
    data: fn(&App, HandleId) -> &NavigatorObserverData,
    data_mut: fn(&mut App, HandleId) -> &mut NavigatorObserverData,
    did_push: fn(&mut App, HandleId, AnyRoute, Option<AnyRoute>),
    did_pop: fn(&mut App, HandleId, AnyRoute, Option<AnyRoute>),
    did_remove: fn(&mut App, HandleId, AnyRoute, Option<AnyRoute>),
    did_replace: fn(&mut App, HandleId, Option<AnyRoute>, Option<AnyRoute>),
    did_change_top: fn(&mut App, HandleId, AnyRoute, Option<AnyRoute>),
    did_start_user_gesture: fn(&mut App, HandleId, AnyRoute, Option<AnyRoute>),
    did_stop_user_gesture: fn(&mut App, HandleId),
}

impl NavigatorObserverVTable {
    const fn of<O: NavigatorObserver>() -> NavigatorObserverVTable {
        NavigatorObserverVTable {
            type_name: std::any::type_name::<O>,
            data: |app, id| O::navigator_observer_data(resolve(id), app),
            data_mut: |app, id| O::navigator_observer_data_mut(resolve(id), app),
            did_push: |app, id, route, previous| O::did_push(resolve(id), app, route, previous),
            did_pop: |app, id, route, previous| O::did_pop(resolve(id), app, route, previous),
            did_remove: |app, id, route, previous| O::did_remove(resolve(id), app, route, previous),
            did_replace: |app, id, new_route, old_route| {
                O::did_replace(resolve(id), app, new_route, old_route)
            },
            did_change_top: |app, id, top, previous_top| {
                O::did_change_top(resolve(id), app, top, previous_top)
            },
            did_start_user_gesture: |app, id, route, previous| {
                O::did_start_user_gesture(resolve(id), app, route, previous)
            },
            did_stop_user_gesture: |app, id| O::did_stop_user_gesture(resolve(id), app),
        }
    }
}

/// Erased [`NavigatorObserver`]: what [`Navigator::observers`] holds.
#[derive(Clone, Copy)]
pub struct AnyNavigatorObserver {
    id: HandleId,
    vtable: &'static NavigatorObserverVTable,
}

impl PartialEq for AnyNavigatorObserver {
    fn eq(&self, other: &AnyNavigatorObserver) -> bool {
        self.id == other.id
    }
}

impl Eq for AnyNavigatorObserver {}

impl std::hash::Hash for AnyNavigatorObserver {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Debug for AnyNavigatorObserver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({:?})", (self.vtable.type_name)(), self.id)
    }
}

impl AnyNavigatorObserver {
    /// The arena id behind this handle.
    pub fn id(self) -> HandleId {
        self.id
    }

    /// Dart's `observer as T`: the typed handle when this observer is a `T`, else `None`.
    pub fn downcast<T: 'static>(self, app: &App) -> Option<Handle<T>> {
        app.handle::<T>(self.id)
    }

    /// See [`NavigatorObserver::navigator`].
    pub fn navigator(self, app: &App) -> Option<Handle<NavigatorState>> {
        (self.vtable.data)(app, self.id).navigator
    }

    fn set_navigator(self, app: &mut App, navigator: Option<Handle<NavigatorState>>) {
        (self.vtable.data_mut)(app, self.id).navigator = navigator;
    }

    /// See [`NavigatorObserver::did_push`].
    pub fn did_push(self, app: &mut App, route: AnyRoute, previous_route: Option<AnyRoute>) {
        (self.vtable.did_push)(app, self.id, route, previous_route)
    }

    /// See [`NavigatorObserver::did_pop`].
    pub fn did_pop(self, app: &mut App, route: AnyRoute, previous_route: Option<AnyRoute>) {
        (self.vtable.did_pop)(app, self.id, route, previous_route)
    }

    /// See [`NavigatorObserver::did_remove`].
    pub fn did_remove(self, app: &mut App, route: AnyRoute, previous_route: Option<AnyRoute>) {
        (self.vtable.did_remove)(app, self.id, route, previous_route)
    }

    /// See [`NavigatorObserver::did_replace`].
    pub fn did_replace(
        self,
        app: &mut App,
        new_route: Option<AnyRoute>,
        old_route: Option<AnyRoute>,
    ) {
        (self.vtable.did_replace)(app, self.id, new_route, old_route)
    }

    /// See [`NavigatorObserver::did_change_top`].
    pub fn did_change_top(
        self,
        app: &mut App,
        top_route: AnyRoute,
        previous_top_route: Option<AnyRoute>,
    ) {
        (self.vtable.did_change_top)(app, self.id, top_route, previous_top_route)
    }

    /// See [`NavigatorObserver::did_start_user_gesture`].
    pub fn did_start_user_gesture(
        self,
        app: &mut App,
        route: AnyRoute,
        previous_route: Option<AnyRoute>,
    ) {
        (self.vtable.did_start_user_gesture)(app, self.id, route, previous_route)
    }

    /// See [`NavigatorObserver::did_stop_user_gesture`].
    pub fn did_stop_user_gesture(self, app: &mut App) {
        (self.vtable.did_stop_user_gesture)(app, self.id)
    }
}

// ---------------------------------------------------------------------------------------------
// HeroControllerScope

/// An inherited widget to host a hero controller.
///
/// The hosted hero controller will be picked up by the navigator in the [`child`](Self::child)
/// subtree. Once a navigator picks up this controller, the navigator will bar any navigator
/// below its subtree from receiving this controller.
///
/// The hero controller inside the [`HeroControllerScope`] can only subscribe to one navigator at
/// a time.
#[derive(Debug)]
pub struct HeroControllerScope {
    /// See [`Widget::key`](crate::Widget::key).
    pub key: Option<KeyRef>,
    /// The hero controller that is hosted inside this widget.
    pub controller: Option<Handle<HeroController>>,
    /// The widget below this widget in the tree.
    pub child: WidgetRef,
}

impl HeroControllerScope {
    /// Creates a widget to host the input `controller`.
    pub fn new<K>(
        controller: Handle<HeroController>,
        child: impl IntoWidget<K>,
    ) -> HeroControllerScope {
        HeroControllerScope {
            key: None,
            controller: Some(controller),
            child: child.into_widget(),
        }
    }

    /// Creates a widget to prevent the subtree from receiving the hero controller above.
    pub fn none<K>(child: impl IntoWidget<K>) -> HeroControllerScope {
        HeroControllerScope {
            key: None,
            controller: None,
            child: child.into_widget(),
        }
    }

    /// Dart `HeroControllerScope(key:)`.
    pub fn key(mut self, key: KeyRef) -> HeroControllerScope {
        self.key = Some(key);
        self
    }

    /// Retrieves the [`HeroController`] from the closest [`HeroControllerScope`] ancestor, or
    /// `None` if none exists.
    ///
    /// Calling this method will create a dependency on the closest [`HeroControllerScope`] in
    /// the `context`, if there is one.
    pub fn maybe_of(app: &mut App, context: BuildContext) -> Option<Handle<HeroController>> {
        context
            .depend_on_inherited_widget_of_exact_type::<HeroControllerScope>(app)
            .and_then(|host| host.controller)
    }

    /// Retrieves the [`HeroController`] from the closest [`HeroControllerScope`] ancestor.
    ///
    /// Panics if no ancestor is found.
    pub fn of(app: &mut App, context: BuildContext) -> Handle<HeroController> {
        HeroControllerScope::maybe_of(app, context).expect(
            "HeroControllerScope.of() was called with a context that does not contain a \
             HeroControllerScope widget.",
        )
    }
}

impl InheritedWidget for HeroControllerScope {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &HeroControllerScope) -> bool {
        old_widget.controller != self.controller
    }
}

// ---------------------------------------------------------------------------------------------
// RouteTransitionRecord and TransitionDelegate

/// A [`Route`] wrapper interface that can be staged for [`TransitionDelegate`] to decide how its
/// underlying route should transition on or off screen.
///
/// Dart's `abstract class RouteTransitionRecord`; the record is the [`RouteEntry`] the navigator
/// keeps, so this is the trait that entry implements.
pub trait RouteTransitionRecord {
    /// Retrieves the wrapped [`Route`].
    fn route(self: Handle<Self>, app: &App) -> AnyRoute
    where
        Self: Sized;

    /// Whether this route is waiting for the decision on how to enter the screen.
    ///
    /// If this property is true, this route requires an explicit decision on how to transition
    /// into the screen. Such a decision should be made in [`TransitionDelegate::resolve`].
    fn is_waiting_for_entering_decision(self: Handle<Self>, app: &App) -> bool
    where
        Self: Sized;

    /// Whether this route is waiting for the decision on how to exit the screen.
    ///
    /// If this property is true, this route requires an explicit decision on how to transition
    /// off the screen. Such a decision should be made in [`TransitionDelegate::resolve`].
    fn is_waiting_for_exiting_decision(self: Handle<Self>, app: &App) -> bool
    where
        Self: Sized;

    /// Marks the route to be pushed with transition.
    fn mark_for_push(self: Handle<Self>, app: &mut App)
    where
        Self: Sized;

    /// Marks the route to be added without transition.
    fn mark_for_add(self: Handle<Self>, app: &mut App)
    where
        Self: Sized;

    /// Marks the route to be popped with transition.
    fn mark_for_pop(self: Handle<Self>, app: &mut App, result: RouteResult)
    where
        Self: Sized;

    /// Marks the route to be completed without transition.
    fn mark_for_complete(self: Handle<Self>, app: &mut App, result: RouteResult)
    where
        Self: Sized;
}

/// A shared [`TransitionDelegate`].
pub type TransitionDelegateRef = Rc<dyn TransitionDelegate>;

/// The delegate that decides how pages added and removed from [`Navigator::pages`] transition in
/// or out of the screen.
///
/// This trait implements the API to be called by [`Navigator`] when it requires explicit
/// decisions on how the routes transition on or off the screen.
///
/// To make route transition decisions, an implementor implements [`resolve`](Self::resolve).
///
/// See also:
///
///  * [`Navigator::transition_delegate`], which uses this trait to make route transition
///    decisions.
///  * [`DefaultTransitionDelegate`], which implements the default way to decide how routes
///    transition in or out of the screen.
pub trait TransitionDelegate: 'static {
    /// A method that will be called by the [`Navigator`] to decide how routes transition in or
    /// out of the screen when [`Navigator::pages`] is updated.
    ///
    /// The `new_page_route_history` list contains all page-based routes in the order that will
    /// be on the navigator's history stack after this update completes. If a route in it has
    /// [`RouteTransitionRecord::is_waiting_for_entering_decision`] set to true, this route
    /// requires an explicit decision on how it should transition onto the navigator. To make a
    /// decision, call [`RouteTransitionRecord::mark_for_push`] or
    /// [`RouteTransitionRecord::mark_for_add`].
    ///
    /// `location_to_exiting_page_route` contains the page-based routes that are removed from the
    /// routes history after the page update. This map records page-based routes to be removed
    /// with the location of the route in the original route history before the update. The keys
    /// are the locations represented by the page-based routes that are directly below the
    /// removed routes, and the values are the page-based routes to be removed. The location is
    /// `None` if the route to be removed is the bottom-most route.
    ///
    /// `page_route_to_pageless_routes` records the page-based routes and their associated
    /// pageless routes.
    ///
    /// Once all the decisions have been made, this method must merge the removed routes (whether
    /// or not they require decisions) and `new_page_route_history` and return the merged result.
    /// The return list must preserve the same order of routes in `new_page_route_history`; the
    /// removed routes can be inserted into the return list freely as long as all of them are
    /// included.
    fn resolve(
        &self,
        app: &mut App,
        new_page_route_history: &[Handle<RouteEntry>],
        location_to_exiting_page_route: &HashMap<Option<Handle<RouteEntry>>, Handle<RouteEntry>>,
        page_route_to_pageless_routes: &HashMap<
            Option<Handle<RouteEntry>>,
            Vec<Handle<RouteEntry>>,
        >,
    ) -> Vec<Handle<RouteEntry>>;
}

/// Dart's `TransitionDelegate._transition`: [`resolve`](TransitionDelegate::resolve) plus the
/// integrity check of the decisions that were made.
fn transition(
    delegate: &dyn TransitionDelegate,
    app: &mut App,
    new_page_route_history: &[Handle<RouteEntry>],
    location_to_exiting_page_route: &HashMap<Option<Handle<RouteEntry>>, Handle<RouteEntry>>,
    page_route_to_pageless_routes: &HashMap<Option<Handle<RouteEntry>>, Vec<Handle<RouteEntry>>>,
) -> Vec<Handle<RouteEntry>> {
    let results = delegate.resolve(
        app,
        new_page_route_history,
        location_to_exiting_page_route,
        page_route_to_pageless_routes,
    );
    // Verifies the integrity after the decisions have been made.
    //
    // Here are the rules:
    // - All the entering routes in new_page_route_history must either be pushed or added.
    // - All the exiting routes in location_to_exiting_page_route must either be popped,
    //   completed or removed.
    // - All the pageless routes that belong to exiting routes must either be popped, completed
    //   or removed.
    // - All the entering routes in the result must preserve the same order as the entering
    //   routes in new_page_route_history, and the result must contain all exiting routes.
    if cfg!(debug_assertions) {
        let mut exiting_page_routes: HashSet<Handle<RouteEntry>> =
            location_to_exiting_page_route.values().copied().collect();
        // Firstly, verifies all exiting routes have been marked.
        for exiting_page_route in &exiting_page_routes {
            assert!(!app.get(*exiting_page_route).is_waiting_for_exiting_decision);
            if let Some(pageless) = page_route_to_pageless_routes.get(&Some(*exiting_page_route)) {
                for pageless_route in pageless {
                    assert!(!app.get(*pageless_route).is_waiting_for_exiting_decision);
                }
            }
        }
        // Secondly, verifies the order of results matches new_page_route_history and contains
        // all the exiting routes.
        let mut index_of_next_route_in_new_history = 0;
        for route_entry in &results {
            let entry = app.get(*route_entry);
            assert!(
                entry.current_state != RouteLifecycle::Staging
                    && !entry.is_waiting_for_exiting_decision
            );
            if index_of_next_route_in_new_history >= new_page_route_history.len()
                || *route_entry != new_page_route_history[index_of_next_route_in_new_history]
            {
                assert!(exiting_page_routes.contains(route_entry));
                exiting_page_routes.remove(route_entry);
            } else {
                index_of_next_route_in_new_history += 1;
            }
        }
        assert!(
            index_of_next_route_in_new_history == new_page_route_history.len()
                && exiting_page_routes.is_empty(),
            "The merged result from the transition delegate's resolve does not include all \
             required routes. Do you remember to merge all exiting routes?"
        );
    }
    results
}

/// The default implementation of [`TransitionDelegate`] that the [`Navigator`] will use if its
/// [`Navigator::transition_delegate`] is not specified.
///
/// This transition delegate follows two rules. Firstly, all the entering routes are placed on
/// top of the exiting routes if they are at the same location. Secondly, the top most route will
/// always transition with an animated transition. All the other routes below will either be
/// completed with [`AnyRoute::current_result`] or added without an animated transition.
#[derive(Debug, Default)]
pub struct DefaultTransitionDelegate;

impl DefaultTransitionDelegate {
    /// Creates a default transition delegate.
    pub fn new() -> DefaultTransitionDelegate {
        DefaultTransitionDelegate
    }
}

impl TransitionDelegate for DefaultTransitionDelegate {
    fn resolve(
        &self,
        app: &mut App,
        new_page_route_history: &[Handle<RouteEntry>],
        location_to_exiting_page_route: &HashMap<Option<Handle<RouteEntry>>, Handle<RouteEntry>>,
        page_route_to_pageless_routes: &HashMap<
            Option<Handle<RouteEntry>>,
            Vec<Handle<RouteEntry>>,
        >,
    ) -> Vec<Handle<RouteEntry>> {
        let mut results: Vec<Handle<RouteEntry>> = Vec::new();

        // Handles the exiting route and its corresponding pageless routes at this location, then
        // recursively checks whether there is any other exiting route above it.
        fn handle_exiting_route(
            app: &mut App,
            results: &mut Vec<Handle<RouteEntry>>,
            location_to_exiting_page_route: &HashMap<
                Option<Handle<RouteEntry>>,
                Handle<RouteEntry>,
            >,
            page_route_to_pageless_routes: &HashMap<
                Option<Handle<RouteEntry>>,
                Vec<Handle<RouteEntry>>,
            >,
            location: Option<Handle<RouteEntry>>,
            is_last: bool,
        ) {
            let Some(exiting_page_route) = location_to_exiting_page_route.get(&location).copied()
            else {
                return;
            };
            if app.get(exiting_page_route).is_waiting_for_exiting_decision {
                let has_pageless_route =
                    page_route_to_pageless_routes.contains_key(&Some(exiting_page_route));
                let is_last_exiting_page_route = is_last
                    && !location_to_exiting_page_route.contains_key(&Some(exiting_page_route));
                let result = app.get(exiting_page_route).route.current_result(app);
                if is_last_exiting_page_route && !has_pageless_route {
                    exiting_page_route.mark_for_pop(app, result);
                } else {
                    exiting_page_route.mark_for_complete(app, result);
                }
                if has_pageless_route {
                    let pageless_routes =
                        page_route_to_pageless_routes[&Some(exiting_page_route)].clone();
                    let last = pageless_routes.last().copied();
                    for pageless_route in pageless_routes {
                        // It is possible that a pageless route that belongs to an exiting
                        // page-based route does not require an exiting decision. This can happen
                        // if the page list is updated right after a pop.
                        if app.get(pageless_route).is_waiting_for_exiting_decision {
                            let result = app.get(pageless_route).route.current_result(app);
                            if is_last_exiting_page_route && Some(pageless_route) == last {
                                pageless_route.mark_for_pop(app, result);
                            } else {
                                pageless_route.mark_for_complete(app, result);
                            }
                        }
                    }
                }
            }
            results.push(exiting_page_route);

            // It is possible there is another exiting route above this one.
            handle_exiting_route(
                app,
                results,
                location_to_exiting_page_route,
                page_route_to_pageless_routes,
                Some(exiting_page_route),
                is_last,
            );
        }

        // Handles exiting route in the beginning of list.
        handle_exiting_route(
            app,
            &mut results,
            location_to_exiting_page_route,
            page_route_to_pageless_routes,
            None,
            new_page_route_history.is_empty(),
        );

        for page_route in new_page_route_history.iter().copied() {
            let is_last_iteration = new_page_route_history.last() == Some(&page_route);
            if app.get(page_route).current_state == RouteLifecycle::Staging {
                if !location_to_exiting_page_route.contains_key(&Some(page_route))
                    && is_last_iteration
                {
                    page_route.mark_for_push(app);
                } else {
                    page_route.mark_for_add(app);
                }
            }
            results.push(page_route);
            handle_exiting_route(
                app,
                &mut results,
                location_to_exiting_page_route,
                page_route_to_pageless_routes,
                Some(page_route),
                is_last_iteration,
            );
        }
        results
    }
}

/// The default value of [`Navigator::route_traversal_edge_behavior`].
pub const K_DEFAULT_ROUTE_TRAVERSAL_EDGE_BEHAVIOR: TraversalEdgeBehavior =
    TraversalEdgeBehavior::ParentScope;

/// The default value of [`Navigator::route_directional_traversal_edge_behavior`].
pub const K_DEFAULT_ROUTE_DIRECTIONAL_TRAVERSAL_EDGE_BEHAVIOR: TraversalEdgeBehavior =
    TraversalEdgeBehavior::Stop;

// ---------------------------------------------------------------------------------------------
// Navigator

/// A widget that manages a set of child widgets with a stack discipline.
///
/// Many apps have a navigator near the top of their widget hierarchy in order to display their
/// logical history using an [`Overlay`] with the most recently visited pages visually on top of
/// the older pages. Using this pattern lets the navigator visually transition from one page to
/// another by moving the widgets around in the overlay. Similarly, the navigator can be used to
/// show a dialog by positioning the dialog widget above the current page.
///
/// ## Using the Pages API
///
/// The [`Navigator`] will convert its [`pages`](Self::pages) into a stack of [`Route`]s if it is
/// provided. A change in `pages` will trigger an update to the stack of routes. To use this API,
/// implement [`Page`] and define a list of pages for `pages`. An
/// [`on_did_remove_page`](Self::on_did_remove_page) callback is also required to properly clean
/// up the input pages in case of a pop.
///
/// By default, the [`Navigator`] will use [`DefaultTransitionDelegate`] to decide how routes
/// transition in or out of the screen.
///
/// ## Using the Navigator API
///
/// The navigator manages a stack of [`Route`] objects and provides two ways for managing the
/// stack, the declarative API [`pages`](Self::pages) or the imperative API
/// [`push`](Navigator::push) and [`pop`](Navigator::pop).
pub struct Navigator {
    /// See [`Widget::key`](crate::Widget::key).
    pub key: Option<KeyRef>,
    /// The list of pages with which to populate the history.
    ///
    /// Pages are turned into routes using [`Page::create_route`] in a manner analogous to how
    /// [`on_generate_route`](Self::on_generate_route) turns [`RouteSettings`] into [`Route`]s.
    ///
    /// When this list is updated, the new list is compared to the previous list and the set of
    /// routes is updated accordingly.
    pub pages: Vec<PageRef>,
    /// Called when [`pop`](Navigator::pop) is invoked but the current [`Route`] corresponds to a
    /// [`Page`] found in the [`pages`](Self::pages) list.
    ///
    /// Dart's deprecated `onPopPage`; [`on_did_remove_page`](Self::on_did_remove_page) is its
    /// replacement.
    pub on_pop_page: Option<PopPageCallback>,
    /// Called when a [`Route`] created from a [`Page`] in [`pages`](Self::pages) is removed.
    ///
    /// This must update the pages list the next time it is passed into [`pages`](Self::pages) so
    /// that it no longer includes the input page.
    pub on_did_remove_page: Option<DidRemovePageCallback>,
    /// The delegate used for deciding how routes transition in or off the screen during the
    /// [`pages`](Self::pages) updates.
    pub transition_delegate: TransitionDelegateRef,
    /// The name of the first route to show.
    ///
    /// Defaults to [`Navigator::DEFAULT_ROUTE_NAME`].
    ///
    /// If this string contains any `/` characters, then the string is split on those characters
    /// and substrings from the start of the string up to each such character are, in turn, used
    /// as routes to push.
    pub initial_route: Option<String>,
    /// Called to generate a route for a given [`RouteSettings`].
    pub on_generate_route: Option<RouteFactory>,
    /// Called when [`on_generate_route`](Self::on_generate_route) fails to generate a route.
    ///
    /// This callback is typically used for error handling. It receives the [`RouteSettings`] and
    /// must return a non-`None` [`Route`].
    pub on_unknown_route: Option<RouteFactory>,
    /// A list of observers for this navigator.
    pub observers: Vec<AnyNavigatorObserver>,
    /// Restoration ID to save and restore the state of the navigator, including its history.
    ///
    /// If a restoration ID is provided, the navigator will persist its internal state (including
    /// the route history) to the surrounding `RestorationScope` and restore it during state
    /// restoration.
    pub restoration_scope_id: Option<String>,
    /// Controls the transfer of focus beyond the first and the last items of a focus scope that
    /// defines focus traversal of widgets within a route.
    pub route_traversal_edge_behavior: TraversalEdgeBehavior,
    /// Controls the transfer of focus beyond the first and the last items of a focus scope that
    /// defines focus traversal of widgets within a route, in a directional traversal.
    pub route_directional_traversal_edge_behavior: TraversalEdgeBehavior,
    /// Called when the widget is created to generate the initial list of [`Route`] objects if
    /// [`initial_route`](Self::initial_route) is not `None`.
    ///
    /// Defaults to [`Navigator::default_generate_initial_routes`].
    pub on_generate_initial_routes: RouteListFactory,
    /// Whether this navigator should report route update message back to the engine when the
    /// top-most route changes.
    pub reports_route_update_to_engine: bool,
    /// The content will be clipped (or not) according to this option.
    ///
    /// Defaults to [`Clip::HardEdge`].
    pub clip_behavior: Clip,
    /// Whether or not the navigator and it's new topmost route should request focus when the new
    /// route is pushed onto the navigator.
    ///
    /// Defaults to true.
    pub request_focus: bool,
}

impl Navigator {
    /// The default name for the [`initial_route`](Self::initial_route).
    ///
    /// See also:
    ///
    ///  * `dart:ui.PlatformDispatcher.defaultRouteName`, which reflects the route that the
    ///    application was started with.
    pub const DEFAULT_ROUTE_NAME: &'static str = "/";

    /// Creates a widget that maintains a stack-based history of child widgets; Dart's named
    /// arguments are the setters.
    pub fn new() -> Navigator {
        Navigator::default()
    }

    /// Dart `Navigator(key:)`.
    pub fn key(mut self, key: KeyRef) -> Navigator {
        self.key = Some(key);
        self
    }

    /// Dart `Navigator(pages:)`.
    pub fn pages(mut self, pages: impl IntoIterator<Item = PageRef>) -> Navigator {
        self.pages = pages.into_iter().collect();
        self
    }

    /// Dart `Navigator(onPopPage:)`.
    pub fn on_pop_page(
        mut self,
        on_pop_page: impl Fn(&mut App, AnyRoute, RouteResult) -> bool + 'static,
    ) -> Navigator {
        self.on_pop_page = Some(Rc::new(on_pop_page));
        self
    }

    /// Dart `Navigator(onDidRemovePage:)`.
    pub fn on_did_remove_page(
        mut self,
        on_did_remove_page: impl Fn(&mut App, PageRef) + 'static,
    ) -> Navigator {
        self.on_did_remove_page = Some(Rc::new(on_did_remove_page));
        self
    }

    /// Dart `Navigator(transitionDelegate:)`.
    pub fn transition_delegate(mut self, delegate: TransitionDelegateRef) -> Navigator {
        self.transition_delegate = delegate;
        self
    }

    /// Dart `Navigator(initialRoute:)`.
    pub fn initial_route(mut self, initial_route: impl Into<String>) -> Navigator {
        self.initial_route = Some(initial_route.into());
        self
    }

    /// Dart `Navigator(onGenerateRoute:)`.
    pub fn on_generate_route(
        mut self,
        on_generate_route: impl Fn(&mut App, &RouteSettings) -> Option<AnyRoute> + 'static,
    ) -> Navigator {
        self.on_generate_route = Some(Rc::new(on_generate_route));
        self
    }

    /// Dart `Navigator(onUnknownRoute:)`.
    pub fn on_unknown_route(
        mut self,
        on_unknown_route: impl Fn(&mut App, &RouteSettings) -> Option<AnyRoute> + 'static,
    ) -> Navigator {
        self.on_unknown_route = Some(Rc::new(on_unknown_route));
        self
    }

    /// Dart `Navigator(observers:)`.
    pub fn observers(
        mut self,
        observers: impl IntoIterator<Item = AnyNavigatorObserver>,
    ) -> Navigator {
        self.observers = observers.into_iter().collect();
        self
    }

    /// Dart `Navigator(restorationScopeId:)`.
    pub fn restoration_scope_id(mut self, restoration_scope_id: impl Into<String>) -> Navigator {
        self.restoration_scope_id = Some(restoration_scope_id.into());
        self
    }

    /// Dart `Navigator(routeTraversalEdgeBehavior:)`.
    pub fn route_traversal_edge_behavior(mut self, behavior: TraversalEdgeBehavior) -> Navigator {
        self.route_traversal_edge_behavior = behavior;
        self
    }

    /// Dart `Navigator(routeDirectionalTraversalEdgeBehavior:)`.
    pub fn route_directional_traversal_edge_behavior(
        mut self,
        behavior: TraversalEdgeBehavior,
    ) -> Navigator {
        self.route_directional_traversal_edge_behavior = behavior;
        self
    }

    /// Dart `Navigator(onGenerateInitialRoutes:)`.
    pub fn on_generate_initial_routes(
        mut self,
        on_generate_initial_routes: impl Fn(&mut App, Handle<NavigatorState>, &str) -> Vec<AnyRoute>
        + 'static,
    ) -> Navigator {
        self.on_generate_initial_routes = Rc::new(on_generate_initial_routes);
        self
    }

    /// Dart `Navigator(reportsRouteUpdateToEngine:)`.
    pub fn reports_route_update_to_engine(mut self, value: bool) -> Navigator {
        self.reports_route_update_to_engine = value;
        self
    }

    /// Dart `Navigator(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> Navigator {
        self.clip_behavior = clip_behavior;
        self
    }

    /// Dart `Navigator(requestFocus:)`.
    pub fn request_focus(mut self, request_focus: bool) -> Navigator {
        self.request_focus = request_focus;
        self
    }

    /// Dart's `Navigator._usingPagesAPI`.
    fn using_pages_api(&self) -> bool {
        !self.pages.is_empty()
    }

    // ---- the static helpers ----

    /// The state from the closest instance of this class that encloses the given context.
    ///
    /// If `root_navigator` is set to true, the state from the furthest instance of this class is
    /// given instead. Useful for pushing contents above all subsequent instances of
    /// [`Navigator`].
    ///
    /// Panics if there is no [`Navigator`] in the given context.
    pub fn of(
        app: &mut App,
        context: BuildContext,
        root_navigator: bool,
    ) -> Handle<NavigatorState> {
        Navigator::maybe_of(app, context, root_navigator).expect(
            "Navigator operation requested with a context that does not include a Navigator. The \
             context used to push or pop routes from the Navigator must be that of a widget that \
             is a descendant of a Navigator widget.",
        )
    }

    /// The state from the closest instance of this class that encloses the given context, if any.
    ///
    /// If `root_navigator` is set to true, the state from the furthest instance of this class is
    /// given instead.
    pub fn maybe_of(
        app: &mut App,
        context: BuildContext,
        root_navigator: bool,
    ) -> Option<Handle<NavigatorState>> {
        let navigator = context.state_handle::<NavigatorState>(app);
        if root_navigator {
            context
                .find_root_ancestor_state_of_type::<NavigatorState>(app)
                .or(navigator)
        } else {
            navigator.or_else(|| context.find_ancestor_state_of_type::<NavigatorState>(app))
        }
    }

    /// Push a named route onto the navigator that most tightly encloses the given context.
    ///
    /// Returns the pushed [`Route`]; register [`AnyRoute::when_popped`] on it for what Dart's
    /// returned future resolves with.
    pub fn push_named(
        app: &mut App,
        context: BuildContext,
        route_name: &str,
        arguments: Option<Rc<dyn Any>>,
    ) -> AnyRoute {
        Navigator::of(app, context, false).push_named(app, route_name, arguments)
    }

    /// Push a named route onto the navigator that most tightly encloses the given context, and
    /// return its restoration ID.
    pub fn restorable_push_named(
        app: &mut App,
        context: BuildContext,
        route_name: &str,
        arguments: Option<RestorationData>,
    ) -> String {
        Navigator::of(app, context, false).restorable_push_named(app, route_name, arguments)
    }

    /// Replace the current route of the navigator that most tightly encloses the given context by
    /// pushing the route named `route_name` and then disposing the previous route once the
    /// animation has completed.
    pub fn push_replacement_named(
        app: &mut App,
        context: BuildContext,
        route_name: &str,
        result: RouteResult,
        arguments: Option<Rc<dyn Any>>,
    ) -> AnyRoute {
        Navigator::of(app, context, false)
            .push_replacement_named(app, route_name, result, arguments)
    }

    /// The restorable version of [`push_replacement_named`](Self::push_replacement_named).
    pub fn restorable_push_replacement_named(
        app: &mut App,
        context: BuildContext,
        route_name: &str,
        result: RouteResult,
        arguments: Option<RestorationData>,
    ) -> String {
        Navigator::of(app, context, false)
            .restorable_push_replacement_named(app, route_name, result, arguments)
    }

    /// Pop the current route off the navigator that most tightly encloses the given context and
    /// push a named route in its place.
    pub fn pop_and_push_named(
        app: &mut App,
        context: BuildContext,
        route_name: &str,
        result: RouteResult,
        arguments: Option<Rc<dyn Any>>,
    ) -> AnyRoute {
        Navigator::of(app, context, false).pop_and_push_named(app, route_name, result, arguments)
    }

    /// The restorable version of [`pop_and_push_named`](Self::pop_and_push_named).
    pub fn restorable_pop_and_push_named(
        app: &mut App,
        context: BuildContext,
        route_name: &str,
        result: RouteResult,
        arguments: Option<RestorationData>,
    ) -> String {
        Navigator::of(app, context, false)
            .restorable_pop_and_push_named(app, route_name, result, arguments)
    }

    /// Push the route with the given name onto the navigator that most tightly encloses the
    /// given context, and then remove all the previous routes until the `predicate` returns true.
    pub fn push_named_and_remove_until(
        app: &mut App,
        context: BuildContext,
        new_route_name: &str,
        predicate: RoutePredicate,
        arguments: Option<Rc<dyn Any>>,
    ) -> AnyRoute {
        Navigator::of(app, context, false).push_named_and_remove_until(
            app,
            new_route_name,
            predicate,
            arguments,
        )
    }

    /// The restorable version of
    /// [`push_named_and_remove_until`](Self::push_named_and_remove_until).
    pub fn restorable_push_named_and_remove_until(
        app: &mut App,
        context: BuildContext,
        new_route_name: &str,
        predicate: RoutePredicate,
        arguments: Option<RestorationData>,
    ) -> String {
        Navigator::of(app, context, false).restorable_push_named_and_remove_until(
            app,
            new_route_name,
            predicate,
            arguments,
        )
    }

    /// Push the given route onto the navigator that most tightly encloses the given context.
    pub fn push(app: &mut App, context: BuildContext, route: AnyRoute) -> AnyRoute {
        Navigator::of(app, context, false).push(app, route)
    }

    /// Push a new route onto the navigator that most tightly encloses the given context, and
    /// return its restoration ID.
    pub fn restorable_push(
        app: &mut App,
        context: BuildContext,
        route_builder: RestorableRouteBuilder,
        arguments: Option<RestorationData>,
    ) -> String {
        Navigator::of(app, context, false).restorable_push(app, route_builder, arguments)
    }

    /// Replace the current route of the navigator that most tightly encloses the given context by
    /// pushing the given route and then disposing the previous route.
    pub fn push_replacement(
        app: &mut App,
        context: BuildContext,
        new_route: AnyRoute,
        result: RouteResult,
    ) -> AnyRoute {
        Navigator::of(app, context, false).push_replacement(app, new_route, result)
    }

    /// The restorable version of [`push_replacement`](Self::push_replacement).
    pub fn restorable_push_replacement(
        app: &mut App,
        context: BuildContext,
        route_builder: RestorableRouteBuilder,
        result: RouteResult,
        arguments: Option<RestorationData>,
    ) -> String {
        Navigator::of(app, context, false).restorable_push_replacement(
            app,
            route_builder,
            result,
            arguments,
        )
    }

    /// Push the given route onto the navigator that most tightly encloses the given context, and
    /// then remove all the previous routes until the `predicate` returns true.
    pub fn push_and_remove_until(
        app: &mut App,
        context: BuildContext,
        new_route: AnyRoute,
        predicate: RoutePredicate,
    ) -> AnyRoute {
        Navigator::of(app, context, false).push_and_remove_until(app, new_route, predicate)
    }

    /// The restorable version of [`push_and_remove_until`](Self::push_and_remove_until).
    pub fn restorable_push_and_remove_until(
        app: &mut App,
        context: BuildContext,
        new_route_builder: RestorableRouteBuilder,
        predicate: RoutePredicate,
        arguments: Option<RestorationData>,
    ) -> String {
        Navigator::of(app, context, false).restorable_push_and_remove_until(
            app,
            new_route_builder,
            predicate,
            arguments,
        )
    }

    /// Replaces a route on the navigator that most tightly encloses the given context with a new
    /// route.
    pub fn replace(app: &mut App, context: BuildContext, old_route: AnyRoute, new_route: AnyRoute) {
        Navigator::of(app, context, false).replace(app, old_route, new_route);
    }

    /// The restorable version of [`replace`](Self::replace).
    pub fn restorable_replace(
        app: &mut App,
        context: BuildContext,
        old_route: AnyRoute,
        new_route_builder: RestorableRouteBuilder,
        arguments: Option<RestorationData>,
    ) -> String {
        Navigator::of(app, context, false).restorable_replace(
            app,
            old_route,
            new_route_builder,
            arguments,
        )
    }

    /// Replaces a route on the navigator that most tightly encloses the given context with a new
    /// route. The route to be replaced is the one below the given `anchor_route`.
    pub fn replace_route_below(
        app: &mut App,
        context: BuildContext,
        anchor_route: AnyRoute,
        new_route: AnyRoute,
    ) {
        Navigator::of(app, context, false).replace_route_below(app, anchor_route, new_route);
    }

    /// The restorable version of [`replace_route_below`](Self::replace_route_below).
    pub fn restorable_replace_route_below(
        app: &mut App,
        context: BuildContext,
        anchor_route: AnyRoute,
        new_route_builder: RestorableRouteBuilder,
        arguments: Option<RestorationData>,
    ) -> String {
        Navigator::of(app, context, false).restorable_replace_route_below(
            app,
            anchor_route,
            new_route_builder,
            arguments,
        )
    }

    /// Whether the navigator that most tightly encloses the given context can be popped.
    ///
    /// The initial route cannot be popped off the navigator, which implies that this function
    /// returns true only if popping the navigator would not remove the initial route.
    pub fn can_pop(app: &mut App, context: BuildContext) -> bool {
        match Navigator::maybe_of(app, context, false) {
            None => false,
            Some(navigator) => navigator.can_pop(app),
        }
    }

    /// Consults the current route's [`AnyRoute::pop_disposition`] method, and acts accordingly,
    /// potentially popping the route as a result; returns whether the pop request should be
    /// considered handled.
    pub fn maybe_pop(app: &mut App, context: BuildContext, result: RouteResult) -> bool {
        Navigator::of(app, context, false).maybe_pop(app, result)
    }

    /// Pop the top-most route off the navigator that most tightly encloses the given context.
    pub fn pop(app: &mut App, context: BuildContext, result: RouteResult) {
        Navigator::of(app, context, false).pop(app, result);
    }

    /// Calls [`pop`](Self::pop) repeatedly on the navigator that most tightly encloses the given
    /// context until the predicate returns true.
    pub fn pop_until(app: &mut App, context: BuildContext, predicate: RoutePredicate) {
        Navigator::of(app, context, false).pop_until(app, predicate);
    }

    /// Calls [`pop`](Self::pop) repeatedly until the predicate returns true, passing `result` to
    /// the last popped route.
    pub fn pop_until_with_result(
        app: &mut App,
        context: BuildContext,
        predicate: RoutePredicate,
        result: RouteResult,
    ) {
        Navigator::of(app, context, false).pop_until_with_result(app, predicate, result);
    }

    /// Immediately remove `route` from the navigator that most tightly encloses the given
    /// context, and dispose of it.
    pub fn remove_route(
        app: &mut App,
        context: BuildContext,
        route: AnyRoute,
        result: RouteResult,
    ) {
        Navigator::of(app, context, false).remove_route(app, route, result);
    }

    /// Immediately remove a route from the navigator that most tightly encloses the given
    /// context, and dispose of it. The route to be replaced is the one below the given
    /// `anchor_route`.
    pub fn remove_route_below(
        app: &mut App,
        context: BuildContext,
        anchor_route: AnyRoute,
        result: RouteResult,
    ) {
        Navigator::of(app, context, false).remove_route_below(app, anchor_route, result);
    }

    /// Turn a route name into a set of [`Route`] objects.
    ///
    /// This is the default value of [`on_generate_initial_routes`](Self::on_generate_initial_routes),
    /// which is used if that value is not supplied.
    ///
    /// If the route name starts with `/` and is longer than one character, the name is split on
    /// `/` characters and each prefix is pushed in turn.
    pub fn default_generate_initial_routes(
        app: &mut App,
        navigator: Handle<NavigatorState>,
        initial_route_name: &str,
    ) -> Vec<AnyRoute> {
        let mut result: Vec<Option<AnyRoute>> = Vec::new();
        if initial_route_name.starts_with('/') && initial_route_name.len() > 1 {
            let initial_route_name = &initial_route_name[1..]; // strip leading '/'
            debug_assert_eq!(Navigator::DEFAULT_ROUTE_NAME, "/");
            result.push(navigator.route_named(app, Navigator::DEFAULT_ROUTE_NAME, None, true));
            if !initial_route_name.is_empty() {
                let mut route_name = String::new();
                for part in initial_route_name.split('/') {
                    route_name.push('/');
                    route_name.push_str(part);
                    result.push(navigator.route_named(app, &route_name, None, true));
                }
            }
            if result.last() == Some(&None) {
                for route in result.iter().flatten() {
                    route.dispose(app);
                }
                result.clear();
            }
        } else if initial_route_name != Navigator::DEFAULT_ROUTE_NAME {
            // If initialRouteName wasn't '/', then we try to get it with allowNull:true, so that
            // if that fails, we fall back to '/' (without allowNull:true, see below).
            result.push(navigator.route_named(app, initial_route_name, None, true));
        }
        // Null route might be a result of gap in initialRouteName.
        result.retain(Option::is_some);
        if result.is_empty() {
            result.push(navigator.route_named(app, Navigator::DEFAULT_ROUTE_NAME, None, false));
        }
        result.into_iter().flatten().collect()
    }
}

impl Debug for Navigator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Navigator")
            .field("pages", &self.pages)
            .field("initial_route", &self.initial_route)
            .field("restoration_scope_id", &self.restoration_scope_id)
            .finish_non_exhaustive()
    }
}

impl Default for Navigator {
    fn default() -> Navigator {
        Navigator {
            key: None,
            pages: Vec::new(),
            on_pop_page: None,
            on_did_remove_page: None,
            transition_delegate: Rc::new(DefaultTransitionDelegate),
            initial_route: None,
            on_generate_route: None,
            on_unknown_route: None,
            observers: Vec::new(),
            restoration_scope_id: None,
            route_traversal_edge_behavior: K_DEFAULT_ROUTE_TRAVERSAL_EDGE_BEHAVIOR,
            route_directional_traversal_edge_behavior:
                K_DEFAULT_ROUTE_DIRECTIONAL_TRAVERSAL_EDGE_BEHAVIOR,
            on_generate_initial_routes: Rc::new(Navigator::default_generate_initial_routes),
            reports_route_update_to_engine: false,
            clip_behavior: Clip::HardEdge,
            request_focus: true,
        }
    }
}

impl StatefulWidget for Navigator {
    type State = NavigatorState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> NavigatorState {
        NavigatorState::new()
    }
}

// ---------------------------------------------------------------------------------------------
// _RouteEntry

/// The lifecycle of a [`RouteEntry`]. The variants are declared in Dart's order: the
/// comparisons the navigator makes on `currentState.index` are `PartialOrd` here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RouteLifecycle {
    /// We will wait for the transition delegate to decide what to do with this route.
    Staging,
    /// We'll want to run install, did_add, etc; a route created by
    /// [`Navigator::on_generate_initial_routes`] or by the initial [`Navigator::pages`].
    Add,
    /// We're waiting for the completion of the top-most route's `did_push`.
    Adding,
    /// We'll want to run install, did_push, etc; a route added via `push` and friends.
    Push,
    /// We'll want to run install, did_push, etc; a route added via `push_replacement` and
    /// friends.
    PushReplace,
    /// We're waiting for the completion of `did_push`.
    Pushing,
    /// We'll want to run install, did_replace, etc; a route added via `replace` and friends.
    Replace,
    /// The route is being harmless.
    Idle,
    /// We'll want to call `did_pop`.
    Pop,
    /// We'll want to call `did_complete`.
    Complete,
    /// We'll want to run did_replace / did_remove etc.
    Remove,
    /// We're waiting for the route to call `finalize_route` to switch to dispose.
    Popping,
    /// We are waiting for subsequent routes to be done animating, then will switch to dispose.
    Removing,
    /// We will dispose the route momentarily.
    Dispose,
    /// The entry is waiting for its widget subtree to be disposed.
    Disposing,
    /// We have disposed the route.
    Disposed,
}

/// Dart's `_RouteEntry`: one [`Route`] and its place in a [`NavigatorState`]'s history.
///
/// This is also the [`RouteTransitionRecord`] a [`TransitionDelegate`] receives.
pub struct RouteEntry {
    route: AnyRoute,
    restoration_information: Option<Rc<RestorationInformation>>,
    page_based: bool,
    current_state: RouteLifecycle,
    /// Last argument to `Route::did_change_previous`; `None` is Dart's `notAnnounced`.
    last_announced_previous_route: Option<Option<AnyRoute>>,
    /// Last argument to `Route::did_pop_next`; `None` is Dart's `notAnnounced`.
    last_announced_popped_next_route: Option<Option<AnyRoute>>,
    /// Last argument to `Route::did_change_next`; `None` is Dart's `notAnnounced`.
    last_announced_next_route: Option<Option<AnyRoute>>,
    /// Whether the route is being removed by an imperative API call.
    imperative_removal: bool,
    pending_result: RouteResult,
    report_removal_to_observer: bool,
    is_waiting_for_exiting_decision: bool,
}

impl RouteEntry {
    /// The maximum number of `did_pop` attempts a `mark_for_pop` makes on a route that handles
    /// pops internally.
    pub const K_DEBUG_POP_ATTEMPT_LIMIT: usize = 100;

    /// Creates an entry for `route`; `initial_state` must be one of [`RouteLifecycle::Staging`],
    /// [`Add`](RouteLifecycle::Add), [`Push`](RouteLifecycle::Push),
    /// [`PushReplace`](RouteLifecycle::PushReplace) or [`Replace`](RouteLifecycle::Replace).
    fn new(
        app: &mut App,
        route: AnyRoute,
        initial_state: RouteLifecycle,
        page_based: bool,
        restoration_information: Option<Rc<RestorationInformation>>,
    ) -> Handle<RouteEntry> {
        debug_assert!(!page_based || route.is_page_based(app));
        debug_assert!(matches!(
            initial_state,
            RouteLifecycle::Staging
                | RouteLifecycle::Add
                | RouteLifecycle::Push
                | RouteLifecycle::PushReplace
                | RouteLifecycle::Replace
        ));
        app.create(RouteEntry {
            route,
            restoration_information,
            page_based,
            current_state: initial_state,
            last_announced_previous_route: None,
            last_announced_popped_next_route: None,
            last_announced_next_route: None,
            imperative_removal: false,
            pending_result: None,
            report_removal_to_observer: true,
            is_waiting_for_exiting_decision: false,
        })
    }

    /// Whether this entry was created from a [`Page`] in [`Navigator::pages`].
    pub fn page_based(self: Handle<Self>, app: &App) -> bool {
        app.get(self).page_based
    }

    /// The lifecycle state this entry is in.
    pub fn current_state(self: Handle<Self>, app: &App) -> RouteLifecycle {
        app.get(self).current_state
    }

    /// The restoration ID under which this entry's route is serialized, if any.
    pub fn restoration_id(self: Handle<Self>, app: &App) -> Option<String> {
        // User-provided restoration ids of Pages are prefixed with 'p+'. Generated ids for
        // pageless routes are prefixed with 'r+' to avoid clashes.
        if app.get(self).page_based {
            let settings = app.get(self).route.settings(app);
            let page = settings.as_page().expect("a page-based route has a page");
            return page.restoration_id().map(|id| format!("p+{id}"));
        }
        app.get(self)
            .restoration_information
            .as_ref()
            .map(|information| format!("r+{}", information.restoration_scope_id()))
    }

    fn can_update_from(self: Handle<Self>, app: &App, page: &PageRef) -> bool {
        if !self.will_be_present(app) {
            return false;
        }
        if !app.get(self).page_based {
            return false;
        }
        let settings = app.get(self).route.settings(app);
        let route_page = settings.as_page().expect("a page-based route has a page");
        page.can_update(&**route_page)
    }

    fn handle_add(
        self: Handle<Self>,
        app: &mut App,
        navigator: Handle<NavigatorState>,
        previous_present: Option<AnyRoute>,
    ) {
        debug_assert!(app.get(self).current_state == RouteLifecycle::Add);
        app.get_mut(self).current_state = RouteLifecycle::Adding;
        let route = app.get(self).route;
        app.get_mut(navigator)
            .observed_route_additions
            .push_back(NavigatorObservation::Push {
                primary_route: route,
                secondary_route: previous_present,
            });
    }

    fn handle_push(
        self: Handle<Self>,
        app: &mut App,
        navigator: Handle<NavigatorState>,
        is_new_first: bool,
        previous: Option<AnyRoute>,
        previous_present: Option<AnyRoute>,
    ) {
        let previous_state = app.get(self).current_state;
        debug_assert!(matches!(
            previous_state,
            RouteLifecycle::Push | RouteLifecycle::PushReplace | RouteLifecycle::Replace
        ));
        let route = app.get(self).route;
        debug_assert!(
            !route.installed(app),
            "The pushed route has already been used. When pushing a route, a new Route object \
             must be provided."
        );
        route.set_navigator(app, Some(navigator));
        route.install(app);
        debug_assert!(!route.overlay_entries(app).is_empty());
        if matches!(
            previous_state,
            RouteLifecycle::Push | RouteLifecycle::PushReplace
        ) {
            let route_future = route.did_push(app);
            app.get_mut(self).current_state = RouteLifecycle::Pushing;
            route_future.when_complete_or_cancel(
                app,
                Listener::new(move |app| {
                    if app.contains(self) && app.get(self).current_state == RouteLifecycle::Pushing
                    {
                        app.get_mut(self).current_state = RouteLifecycle::Idle;
                        navigator.flush_history_updates(app, true);
                    }
                }),
            );
        } else {
            debug_assert!(previous_state == RouteLifecycle::Replace);
            route.did_replace(app, previous);
            app.get_mut(self).current_state = RouteLifecycle::Idle;
        }
        if is_new_first {
            route.did_change_next(app, None);
        }

        if matches!(
            previous_state,
            RouteLifecycle::Replace | RouteLifecycle::PushReplace
        ) {
            app.get_mut(navigator).observed_route_additions.push_back(
                NavigatorObservation::Replace {
                    primary_route: route,
                    secondary_route: previous_present,
                },
            );
            if let Some(previous_present) = previous_present
                && let Some(page) = previous_present.settings(app).as_page().cloned()
                && let Some(callback) = navigator.widget(app).on_did_remove_page.clone()
            {
                callback(app, page);
            }
        } else {
            debug_assert!(previous_state == RouteLifecycle::Push);
            app.get_mut(navigator)
                .observed_route_additions
                .push_back(NavigatorObservation::Push {
                    primary_route: route,
                    secondary_route: previous_present,
                });
        }
    }

    fn handle_did_pop_next(self: Handle<Self>, app: &mut App, popped_route: AnyRoute) {
        let route = app.get(self).route;
        route.did_pop_next(app, popped_route);
        app.get_mut(self).last_announced_popped_next_route = Some(Some(popped_route));
    }

    /// Returns whether the entry has been marked for pop.
    fn handle_pop(
        self: Handle<Self>,
        app: &mut App,
        navigator: Handle<NavigatorState>,
        _previous_present: Option<AnyRoute>,
    ) -> bool {
        let route = app.get(self).route;
        debug_assert!(route.is_installed_in(app, navigator));
        app.get_mut(self).current_state = RouteLifecycle::Popping;
        if route.popped_is_completed(app) {
            // This is a page-based route popped through the Navigator.pop. The didPop should
            // have been called. No further action is needed.
            debug_assert!(app.get(self).page_based);
            debug_assert!(app.get(self).pending_result.is_none());
            return true;
        }
        let pending_result = app.get(self).pending_result.clone();
        if !route.did_pop(app, pending_result.clone()) {
            app.get_mut(self).current_state = RouteLifecycle::Idle;
            return false;
        }
        route.on_pop_invoked_with_result(app, true, pending_result);
        if app.get(self).page_based
            && app.get(self).imperative_removal
            && let Some(page) = route.settings(app).as_page().cloned()
            && let Some(callback) = navigator.widget(app).on_did_remove_page.clone()
        {
            callback(app, page);
        }
        app.get_mut(self).pending_result = None;
        true
    }

    fn handle_complete(self: Handle<Self>, app: &mut App) {
        let route = app.get(self).route;
        let pending_result = app.get(self).pending_result.clone();
        route.did_complete(app, pending_result);
        app.get_mut(self).pending_result = None;
        // did_complete implies the popped completion resolved.
        debug_assert!(route.popped_is_completed(app));
        app.get_mut(self).current_state = RouteLifecycle::Remove;
    }

    fn handle_removal(
        self: Handle<Self>,
        app: &mut App,
        navigator: Handle<NavigatorState>,
        previous_present: Option<AnyRoute>,
    ) {
        let route = app.get(self).route;
        if route.is_installed_in(app, navigator) {
            app.get_mut(self).current_state = RouteLifecycle::Removing;
        } else {
            // The route was never installed (e.g. it was created for a page that was removed
            // before the route was ever pushed), so there is nothing to animate away.
            app.get_mut(self).current_state = RouteLifecycle::Dispose;
        }
        if app.get(self).report_removal_to_observer {
            app.get_mut(navigator).observed_route_deletions.push_back(
                NavigatorObservation::Remove {
                    primary_route: route,
                    secondary_route: previous_present,
                },
            );
        }
    }

    fn did_add(
        self: Handle<Self>,
        app: &mut App,
        navigator: Handle<NavigatorState>,
        is_new_first: bool,
    ) {
        let route = app.get(self).route;
        debug_assert!(!route.installed(app));
        route.set_navigator(app, Some(navigator));
        route.install(app);
        debug_assert!(!route.overlay_entries(app).is_empty());
        route.did_add(app);
        app.get_mut(self).current_state = RouteLifecycle::Idle;
        if is_new_first {
            route.did_change_next(app, None);
        }
    }

    fn pop(self: Handle<Self>, app: &mut App, result: RouteResult, imperative_removal: bool) {
        debug_assert!(self.is_present(app));
        let entry = app.get_mut(self);
        entry.pending_result = result;
        entry.current_state = RouteLifecycle::Pop;
        entry.imperative_removal = imperative_removal;
    }

    fn complete(
        self: Handle<Self>,
        app: &mut App,
        result: RouteResult,
        is_replaced: bool,
        imperative_removal: bool,
    ) {
        if app.get(self).current_state >= RouteLifecycle::Remove {
            return;
        }
        debug_assert!(self.is_present(app));
        let entry = app.get_mut(self);
        entry.report_removal_to_observer = !is_replaced;
        entry.pending_result = result;
        entry.current_state = RouteLifecycle::Complete;
        entry.imperative_removal = imperative_removal;
    }

    fn finalize(self: Handle<Self>, app: &mut App) {
        debug_assert!(app.get(self).current_state < RouteLifecycle::Dispose);
        app.get_mut(self).current_state = RouteLifecycle::Dispose;
    }

    /// Disposes the route immediately, without waiting for its widget subtree to be unmounted.
    fn forced_dispose(self: Handle<Self>, app: &mut App) {
        debug_assert!(app.get(self).current_state < RouteLifecycle::Disposed);
        app.get_mut(self).current_state = RouteLifecycle::Disposed;
        let route = app.get(self).route;
        route.dispose(app);
    }

    /// Disposes the route after the widget subtree the route's overlay entries built has been
    /// unmounted.
    fn dispose(self: Handle<Self>, app: &mut App) {
        debug_assert!(app.get(self).current_state < RouteLifecycle::Disposing);
        app.get_mut(self).current_state = RouteLifecycle::Disposing;

        let route = app.get(self).route;
        let mounted_entries: Vec<Handle<OverlayEntry>> = route
            .overlay_entries(app)
            .into_iter()
            .filter(|entry| entry.mounted(app))
            .collect();

        if mounted_entries.is_empty() {
            self.forced_dispose(app);
            return;
        }

        let navigator = route.navigator(app).expect("an installed route");
        app.get_mut(navigator)
            .entry_waiting_for_subtree_disposal
            .insert(self);
        let remaining = Rc::new(std::cell::Cell::new(mounted_entries.len()));
        for overlay_entry in mounted_entries {
            let slot: Rc<std::cell::RefCell<Option<Listener>>> =
                Rc::new(std::cell::RefCell::new(None));
            let listener = {
                let slot = Rc::clone(&slot);
                let remaining = Rc::clone(&remaining);
                Listener::new(move |app| {
                    debug_assert!(remaining.get() > 0);
                    debug_assert!(!overlay_entry.mounted(app));
                    remaining.set(remaining.get() - 1);
                    let registered = slot.borrow().clone();
                    if let Some(registered) = registered {
                        overlay_entry.remove_listener(app, &registered);
                    }
                    if remaining.get() == 0 {
                        app.schedule_microtask(Listener::new(move |app| {
                            if !app
                                .get_mut(navigator)
                                .entry_waiting_for_subtree_disposal
                                .remove(&self)
                            {
                                // This route entry was disposed by the navigator's dispose.
                                return;
                            }
                            debug_assert!(app.get(self).current_state == RouteLifecycle::Disposing);
                            self.forced_dispose(app);
                        }));
                    }
                })
            };
            *slot.borrow_mut() = Some(listener.clone());
            overlay_entry.add_listener(app, listener);
        }
    }

    fn will_be_present(self: Handle<Self>, app: &App) -> bool {
        let state = app.get(self).current_state;
        state <= RouteLifecycle::Idle && state >= RouteLifecycle::Add
    }

    fn is_present(self: Handle<Self>, app: &App) -> bool {
        let state = app.get(self).current_state;
        state <= RouteLifecycle::Remove && state >= RouteLifecycle::Add
    }

    fn is_present_for_restoration(self: Handle<Self>, app: &App) -> bool {
        app.get(self).current_state <= RouteLifecycle::Idle
    }

    fn suitable_for_announcement(self: Handle<Self>, app: &App) -> bool {
        let state = app.get(self).current_state;
        state <= RouteLifecycle::Removing && state >= RouteLifecycle::Push
    }

    fn suitable_for_transition_animation(self: Handle<Self>, app: &App) -> bool {
        let state = app.get(self).current_state;
        state <= RouteLifecycle::Remove && state >= RouteLifecycle::Push
    }

    fn should_announce_change_to_next(
        self: Handle<Self>,
        app: &App,
        next_route: Option<AnyRoute>,
    ) -> bool {
        let entry = app.get(self);
        debug_assert!(entry.last_announced_next_route != Some(next_route));
        // We do not announce when `next_route` is null and the last announced popped next route
        // is the same as the last announced next route: the route was already told that it is
        // the topmost by `did_pop_next`.
        !(next_route.is_none()
            && entry.last_announced_popped_next_route == entry.last_announced_next_route)
    }

    fn is_present_predicate(app: &App, entry: Handle<RouteEntry>) -> bool {
        entry.is_present(app)
    }

    fn suitable_for_transition_animation_predicate(app: &App, entry: Handle<RouteEntry>) -> bool {
        entry.suitable_for_transition_animation(app)
    }

    fn will_be_present_predicate(app: &App, entry: Handle<RouteEntry>) -> bool {
        entry.will_be_present(app)
    }

    #[expect(
        dead_code,
        reason = "Dart's `_RouteEntry.restorationEnabled` getter; only its setter is read"
    )]
    fn restoration_enabled(self: Handle<Self>, app: &App) -> bool {
        let notifier = app.get(self).route.restoration_scope_id(app);
        app.get(notifier).value().is_some()
    }

    fn set_restoration_enabled(self: Handle<Self>, app: &mut App, value: bool) {
        debug_assert!(!value || self.restoration_id(app).is_some());
        let id = value.then(|| self.restoration_id(app)).flatten();
        let route = app.get(self).route;
        route.update_restoration_id(app, id);
    }

    fn mark_needs_exiting_decision(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).is_waiting_for_exiting_decision = true;
    }
}

impl RouteTransitionRecord for RouteEntry {
    fn route(self: Handle<Self>, app: &App) -> AnyRoute {
        app.get(self).route
    }

    fn is_waiting_for_entering_decision(self: Handle<Self>, app: &App) -> bool {
        app.get(self).current_state == RouteLifecycle::Staging
    }

    fn is_waiting_for_exiting_decision(self: Handle<Self>, app: &App) -> bool {
        app.get(self).is_waiting_for_exiting_decision
    }

    fn mark_for_push(self: Handle<Self>, app: &mut App) {
        debug_assert!(
            self.is_waiting_for_entering_decision(app)
                && !self.is_waiting_for_exiting_decision(app),
            "This route cannot be marked for push. Either a decision has already been made or it \
             does not require an explicit decision on how to transition in."
        );
        app.get_mut(self).current_state = RouteLifecycle::Push;
    }

    fn mark_for_add(self: Handle<Self>, app: &mut App) {
        debug_assert!(
            self.is_waiting_for_entering_decision(app)
                && !self.is_waiting_for_exiting_decision(app),
            "This route cannot be marked for add. Either a decision has already been made or it \
             does not require an explicit decision on how to transition in."
        );
        app.get_mut(self).current_state = RouteLifecycle::Add;
    }

    fn mark_for_pop(self: Handle<Self>, app: &mut App, result: RouteResult) {
        debug_assert!(
            !self.is_waiting_for_entering_decision(app)
                && self.is_waiting_for_exiting_decision(app)
                && self.is_present(app),
            "This route cannot be marked for pop. Either a decision has already been made or it \
             does not require an explicit decision on how to transition out."
        );
        // Remove state that prevents a pop, e.g. LocalHistoryEntry.
        let route = app.get(self).route;
        let mut attempt = 0;
        while route.will_handle_pop_internally(app) {
            if cfg!(debug_assertions) {
                attempt += 1;
                assert!(
                    attempt < RouteEntry::K_DEBUG_POP_ATTEMPT_LIMIT,
                    "Attempted to pop {route:?} {} times, but still failed",
                    RouteEntry::K_DEBUG_POP_ATTEMPT_LIMIT
                );
            }
            let pop_result = route.did_pop(app, result.clone());
            debug_assert!(!pop_result);
        }
        self.pop(app, result, false);
        app.get_mut(self).is_waiting_for_exiting_decision = false;
    }

    fn mark_for_complete(self: Handle<Self>, app: &mut App, result: RouteResult) {
        debug_assert!(
            !self.is_waiting_for_entering_decision(app)
                && self.is_waiting_for_exiting_decision(app)
                && self.is_present(app),
            "This route cannot be marked for complete. Either a decision has already been made or \
             it does not require an explicit decision on how to transition out."
        );
        self.complete(app, result, false, false);
        app.get_mut(self).is_waiting_for_exiting_decision = false;
    }
}

/// Dart's `_NavigatorObservation` family: what an observer is told once the history has settled.
#[derive(Clone, Copy)]
enum NavigatorObservation {
    Push {
        primary_route: AnyRoute,
        secondary_route: Option<AnyRoute>,
    },
    Pop {
        primary_route: AnyRoute,
        secondary_route: Option<AnyRoute>,
    },
    Remove {
        primary_route: AnyRoute,
        secondary_route: Option<AnyRoute>,
    },
    Replace {
        primary_route: AnyRoute,
        secondary_route: Option<AnyRoute>,
    },
}

impl NavigatorObservation {
    fn notify(self, app: &mut App, observer: AnyNavigatorObserver) {
        match self {
            NavigatorObservation::Push {
                primary_route,
                secondary_route,
            } => observer.did_push(app, primary_route, secondary_route),
            NavigatorObservation::Pop {
                primary_route,
                secondary_route,
            } => observer.did_pop(app, primary_route, secondary_route),
            NavigatorObservation::Remove {
                primary_route,
                secondary_route,
            } => observer.did_remove(app, primary_route, secondary_route),
            NavigatorObservation::Replace {
                primary_route,
                secondary_route,
            } => observer.did_replace(app, Some(primary_route), secondary_route),
        }
    }
}

/// Dart's `_History`: the navigator's route entries, as a `ChangeNotifier`.
pub struct History {
    change_notifier: reveal_foundation::ChangeNotifierData,
    value: Vec<Handle<RouteEntry>>,
}

impl History {
    fn new(app: &mut App) -> Handle<History> {
        app.create(History {
            change_notifier: reveal_foundation::ChangeNotifierData::new(),
            value: Vec::new(),
        })
    }

    fn entries(self: Handle<Self>, app: &App) -> Vec<Handle<RouteEntry>> {
        app.get(self).value.clone()
    }

    fn len(self: Handle<Self>, app: &App) -> usize {
        app.get(self).value.len()
    }

    fn is_empty(self: Handle<Self>, app: &App) -> bool {
        app.get(self).value.is_empty()
    }

    fn at(self: Handle<Self>, app: &App, index: usize) -> Handle<RouteEntry> {
        app.get(self).value[index]
    }

    fn index_where(
        self: Handle<Self>,
        app: &App,
        test: impl Fn(&App, Handle<RouteEntry>) -> bool,
    ) -> Option<usize> {
        app.get(self)
            .value
            .iter()
            .position(|entry| test(app, *entry))
    }

    fn add(self: Handle<Self>, app: &mut App, element: Handle<RouteEntry>) {
        app.get_mut(self).value.push(element);
        self.notify_listeners(app);
    }

    fn add_all(self: Handle<Self>, app: &mut App, elements: Vec<Handle<RouteEntry>>) {
        if elements.is_empty() {
            return;
        }
        app.get_mut(self).value.extend(elements);
        self.notify_listeners(app);
    }

    fn clear(self: Handle<Self>, app: &mut App) {
        let value_was_empty = app.get(self).value.is_empty();
        app.get_mut(self).value.clear();
        if !value_was_empty {
            self.notify_listeners(app);
        }
    }

    fn insert(self: Handle<Self>, app: &mut App, index: usize, element: Handle<RouteEntry>) {
        app.get_mut(self).value.insert(index, element);
        self.notify_listeners(app);
    }

    fn remove_at(self: Handle<Self>, app: &mut App, index: usize) -> Handle<RouteEntry> {
        let entry = app.get_mut(self).value.remove(index);
        self.notify_listeners(app);
        entry
    }

    fn remove_last(self: Handle<Self>, app: &mut App) -> Handle<RouteEntry> {
        let entry = app.get_mut(self).value.pop().expect("a non-empty history");
        self.notify_listeners(app);
        entry
    }
}

impl reveal_foundation::ChangeNotifier for History {
    fn change_notifier_data(&self) -> &reveal_foundation::ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut reveal_foundation::ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl Debug for History {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.value, f)
    }
}

// ---------------------------------------------------------------------------------------------
// NavigatorState

/// The state for a [`Navigator`] widget.
///
/// A reference to this class can be obtained by calling [`Navigator::of`].
pub struct NavigatorState {
    state: StateData<Navigator>,
    ticker_provider: TickerProviderStateMixinData,
    restoration: RestorationMixinData,
    overlay_key: Option<GlobalKey>,
    history: Option<Handle<History>>,
    /// The [`RouteEntry`]s that are waiting for their subtrees to be disposed.
    entry_waiting_for_subtree_disposal: HashSet<Handle<RouteEntry>>,
    serializable_history: Option<Handle<HistoryProperty>>,
    observed_route_additions: VecDeque<NavigatorObservation>,
    observed_route_deletions: VecDeque<NavigatorObservation>,
    focus_node: Option<Handle<FocusNode>>,
    /// Used to prevent re-entrant calls to push, pop, and friends.
    debug_locked: bool,
    hero_controller_from_scope: Option<Handle<HeroController>>,
    effective_observers: Vec<AnyNavigatorObserver>,
    raw_next_pageless_restoration_scope_id: Option<Handle<RestorableInt>>,
    last_topmost_route: Option<Handle<RouteEntry>>,
    last_announced_route_name: Option<String>,
    debug_updating_page: bool,
    flushing_history: bool,
    user_gestures_in_progress_count: i64,
    user_gesture_in_progress_notifier: Option<Handle<ValueNotifier<bool>>>,
    active_pointers: HashSet<i64>,
}

impl NavigatorState {
    fn new() -> NavigatorState {
        NavigatorState {
            state: StateData::new(),
            ticker_provider: TickerProviderStateMixinData::new(),
            restoration: RestorationMixinData::new(),
            overlay_key: None,
            history: None,
            entry_waiting_for_subtree_disposal: HashSet::new(),
            serializable_history: None,
            observed_route_additions: VecDeque::new(),
            observed_route_deletions: VecDeque::new(),
            focus_node: None,
            debug_locked: false,
            hero_controller_from_scope: None,
            effective_observers: Vec::new(),
            raw_next_pageless_restoration_scope_id: None,
            last_topmost_route: None,
            last_announced_route_name: None,
            debug_updating_page: false,
            flushing_history: false,
            user_gestures_in_progress_count: 0,
            user_gesture_in_progress_notifier: None,
            active_pointers: HashSet::new(),
        }
    }

    /// The focus node for this navigator.
    ///
    /// The navigator will attempt to focus this node when the top-most route changes.
    pub fn focus_node(self: Handle<Self>, app: &App) -> Handle<FocusNode> {
        app.get(self).focus_node.expect("init_state has run")
    }

    /// True if the state's [`user_gesture_in_progress_notifier`](Self::user_gesture_in_progress_notifier)
    /// value is true, meaning a route is being moved by a user gesture.
    pub fn user_gesture_in_progress(self: Handle<Self>, app: &App) -> bool {
        *app.get(self.user_gesture_in_progress_notifier(app)).value()
    }

    /// Notifies its listeners if the value of [`user_gesture_in_progress`](Self::user_gesture_in_progress)
    /// changes.
    pub fn user_gesture_in_progress_notifier(
        self: Handle<Self>,
        app: &App,
    ) -> Handle<ValueNotifier<bool>> {
        app.get(self)
            .user_gesture_in_progress_notifier
            .expect("init_state has run")
    }

    fn history(self: Handle<Self>, app: &App) -> Handle<History> {
        app.get(self).history.expect("init_state has run")
    }

    fn serializable_history(self: Handle<Self>, app: &App) -> Handle<HistoryProperty> {
        app.get(self)
            .serializable_history
            .expect("init_state has run")
    }

    pub(crate) fn history_entries(self: Handle<Self>, app: &App) -> Vec<Handle<RouteEntry>> {
        self.history(app).entries(app)
    }

    /// Dart's `_handleHistoryChanged`.
    fn handle_history_changed(self: Handle<Self>, app: &mut App) {
        let can_handle_pop = self.get_navigator_can_handle_pop(app);
        match SchedulerBinding::scheduler_phase(app) {
            SchedulerPhase::PostFrameCallbacks => {
                let context = self.context(app);
                NavigationNotification { can_handle_pop }.dispatch(app, Some(context));
            }
            _ => {
                SchedulerBinding::add_post_frame_callback(
                    app,
                    FrameCallback::handle_method(self, NavigatorState::dispatch_navigation_state),
                );
            }
        }
    }

    fn dispatch_navigation_state(self: Handle<Self>, app: &mut App, _time_stamp: Duration) {
        if !self.mounted(app) {
            return;
        }
        let can_handle_pop = self.get_navigator_can_handle_pop(app);
        let context = self.context(app);
        NavigationNotification { can_handle_pop }.dispatch(app, Some(context));
    }

    fn get_navigator_can_handle_pop(self: Handle<Self>, app: &mut App) -> bool {
        if self.can_pop(app) {
            return true;
        }
        let last_entry = self.last_route_entry_where_or_null(app, RouteEntry::is_present_predicate);
        match last_entry {
            None => false,
            Some(entry) => {
                let route = app.get(entry).route;
                route.pop_disposition(app) == RoutePopDisposition::DoNotPop
            }
        }
    }

    fn debug_check_page_api_parameters(self: Handle<Self>, app: &App) -> bool {
        let widget = self.widget(app);
        if !widget.using_pages_api() {
            return true;
        }
        assert!(
            (widget.on_did_remove_page.is_none()) != (widget.on_pop_page.is_none()),
            "Either on_did_remove_page or on_pop_page must be provided to use the \
             Navigator.pages API but not both."
        );
        true
    }

    fn next_pageless_restoration_scope_id(self: Handle<Self>, app: &mut App) -> i64 {
        let property = app
            .get(self)
            .raw_next_pageless_restoration_scope_id
            .expect("init_state has run");
        let value = *property.value(app);
        property.set_value(app, value + 1);
        value
    }

    fn forced_dispose_all_route_entries(self: Handle<Self>, app: &mut App) {
        let waiting: Vec<Handle<RouteEntry>> = app
            .get_mut(self)
            .entry_waiting_for_subtree_disposal
            .drain()
            .collect();
        for entry in waiting {
            entry.forced_dispose(app);
        }
        let history = self.history(app);
        while !history.is_empty(app) {
            let entry = history.remove_last(app);
            NavigatorState::dispose_route_entry(app, entry, false);
        }
    }

    fn dispose_route_entry(app: &mut App, entry: Handle<RouteEntry>, graceful: bool) {
        let route = app.get(entry).route;
        for overlay_entry in route.overlay_entries(app) {
            overlay_entry.remove(app);
        }
        if graceful {
            entry.dispose(app);
        } else {
            entry.forced_dispose(app);
        }
    }

    fn update_hero_controller(
        self: Handle<Self>,
        app: &mut App,
        new_hero_controller: Option<Handle<HeroController>>,
    ) {
        if app.get(self).hero_controller_from_scope == new_hero_controller {
            return;
        }
        if let Some(controller) = new_hero_controller {
            controller.as_observer().set_navigator(app, Some(self));
        }
        if let Some(previous) = app.get(self).hero_controller_from_scope
            && previous.as_observer().navigator(app) == Some(self)
        {
            previous.as_observer().set_navigator(app, None);
        }
        app.get_mut(self).hero_controller_from_scope = new_hero_controller;
        self.update_effective_observers(app);
    }

    fn update_effective_observers(self: Handle<Self>, app: &mut App) {
        let mut observers = self.widget(app).observers.clone();
        if let Some(controller) = app.get(self).hero_controller_from_scope {
            observers.push(controller.as_observer());
        }
        app.get_mut(self).effective_observers = observers;
    }

    fn debug_check_duplicated_page_keys(self: Handle<Self>, app: &App) {
        if !cfg!(debug_assertions) {
            return;
        }
        let mut key_reservation: Vec<KeyRef> = Vec::new();
        for page in &self.widget(app).pages {
            if let Some(key) = page.key() {
                assert!(
                    !key_reservation.iter().any(|other| **other == **key),
                    "The Navigator.pages must not contain two pages with the same key."
                );
                key_reservation.push(Rc::clone(key));
            }
        }
    }

    /// The [`OverlayState`] of the [`Overlay`] this navigator builds, once it is mounted.
    pub fn overlay(self: Handle<Self>, app: &mut App) -> Option<Handle<OverlayState>> {
        let key = app.get(self).overlay_key.clone()?;
        key.current_state::<OverlayState>(app)
    }

    fn all_route_overlay_entries(self: Handle<Self>, app: &App) -> Vec<Handle<OverlayEntry>> {
        let mut entries = Vec::new();
        for entry in self.history_entries(app) {
            entries.extend(app.get(entry).route.overlay_entries(app));
        }
        entries
    }

    // ---- the history walkers ----

    fn get_route_before(
        self: Handle<Self>,
        app: &App,
        index: i64,
        predicate: fn(&App, Handle<RouteEntry>) -> bool,
    ) -> Option<Handle<RouteEntry>> {
        let index = self.get_index_before(app, index, predicate);
        (index >= 0).then(|| self.history(app).at(app, index as usize))
    }

    fn get_index_before(
        self: Handle<Self>,
        app: &App,
        mut index: i64,
        predicate: fn(&App, Handle<RouteEntry>) -> bool,
    ) -> i64 {
        let history = self.history(app);
        while index >= 0 && !predicate(app, history.at(app, index as usize)) {
            index -= 1;
        }
        index
    }

    fn get_route_after(
        self: Handle<Self>,
        app: &App,
        mut index: i64,
        predicate: fn(&App, Handle<RouteEntry>) -> bool,
    ) -> Option<Handle<RouteEntry>> {
        let history = self.history(app);
        let length = history.len(app) as i64;
        while index < length && !predicate(app, history.at(app, index as usize)) {
            index += 1;
        }
        (index < length).then(|| history.at(app, index as usize))
    }

    pub(crate) fn first_route_entry_where_or_null(
        self: Handle<Self>,
        app: &App,
        test: impl Fn(&App, Handle<RouteEntry>) -> bool,
    ) -> Option<Handle<RouteEntry>> {
        self.history_entries(app)
            .into_iter()
            .find(|entry| test(app, *entry))
    }

    pub(crate) fn last_route_entry_where_or_null(
        self: Handle<Self>,
        app: &App,
        test: impl Fn(&App, Handle<RouteEntry>) -> bool,
    ) -> Option<Handle<RouteEntry>> {
        let mut result = None;
        for entry in self.history_entries(app) {
            if test(app, entry) {
                result = Some(entry);
            }
        }
        result
    }
}

impl NavigatorState {
    /// Dart's `_updatePages`: the diff between the old history and the new [`Navigator::pages`].
    fn update_pages(self: Handle<Self>, app: &mut App) {
        debug_assert!(!app.get(self).debug_updating_page);
        self.debug_check_duplicated_page_keys(app);
        app.get_mut(self).debug_updating_page = true;

        // This attempts to diff the new pages list (widget.pages) with the old
        // _RouteEntry history (_history).
        //
        // In the case of a newly added page or an unmatched page, the new page is
        // added to the newHistory list with a staging state, and the transition
        // delegate decides how it enters the screen.
        let history = self.history(app);
        let mut needs_explicit_decision = false;
        let mut new_pages_bottom: i64 = 0;
        let mut old_entries_bottom: i64 = 0;
        let mut new_pages_top: i64 = self.widget(app).pages.len() as i64 - 1;
        let mut old_entries_top: i64 = history.len(app) as i64 - 1;

        let mut new_history: Vec<Handle<RouteEntry>> = Vec::new();
        let mut page_route_to_pageless_routes: HashMap<
            Option<Handle<RouteEntry>>,
            Vec<Handle<RouteEntry>>,
        > = HashMap::new();

        // Scans the top of the list until we found a page that does not exist in the new list.
        let mut previous_old_page_route_entry: Option<Handle<RouteEntry>> = None;
        while old_entries_bottom <= old_entries_top {
            let old_entry = history.at(app, old_entries_bottom as usize);
            debug_assert!(app.get(old_entry).current_state != RouteLifecycle::Disposed);
            // Records pageless route. The bottom most pageless routes will be added to the
            // most recently scanned page-based route.
            if !app.get(old_entry).page_based {
                page_route_to_pageless_routes
                    .entry(previous_old_page_route_entry)
                    .or_default()
                    .push(old_entry);
                old_entries_bottom += 1;
                continue;
            }
            if new_pages_bottom > new_pages_top {
                break;
            }
            let new_page = self.widget(app).pages[new_pages_bottom as usize].clone();
            if !old_entry.can_update_from(app, &new_page) {
                break;
            }
            previous_old_page_route_entry = Some(old_entry);
            let route = app.get(old_entry).route;
            route.update_settings(app, RouteSettingsRef::Page(new_page));
            new_history.push(old_entry);
            new_pages_bottom += 1;
            old_entries_bottom += 1;
        }

        let mut unattached_pageless_routes: Vec<Handle<RouteEntry>> = Vec::new();
        // Scans the bottom of the list until we found a page that does not exist in the new
        // list.
        while (old_entries_bottom <= old_entries_top) && (new_pages_bottom <= new_pages_top) {
            let old_entry = history.at(app, old_entries_top as usize);
            debug_assert!(app.get(old_entry).current_state != RouteLifecycle::Disposed);
            if !app.get(old_entry).page_based {
                unattached_pageless_routes.push(old_entry);
                old_entries_top -= 1;
                continue;
            }
            let new_page = self.widget(app).pages[new_pages_top as usize].clone();
            if !old_entry.can_update_from(app, &new_page) {
                break;
            }

            // We found the page for all the consecutive pageless routes below. Attach these
            // pageless routes to the page.
            if !unattached_pageless_routes.is_empty() {
                page_route_to_pageless_routes
                    .entry(Some(old_entry))
                    .or_insert_with(|| unattached_pageless_routes.clone());
                unattached_pageless_routes.clear();
            }

            old_entries_top -= 1;
            new_pages_top -= 1;
        }
        // Reverts the pageless routes that cannot be updated.
        old_entries_top += unattached_pageless_routes.len() as i64;

        // Scans the remaining routes and records the page keys.
        let mut old_entries_bottom_to_scan = old_entries_bottom;
        let mut page_key_to_old_entry: Vec<(KeyRef, Handle<RouteEntry>)> = Vec::new();
        // This set contains entries that are transitioning out but are still in the widget's
        // pages list; they should not be reused during the diff.
        let mut phantom_entries: HashSet<Handle<RouteEntry>> = HashSet::new();
        while old_entries_bottom_to_scan <= old_entries_top {
            let old_entry = history.at(app, old_entries_bottom_to_scan as usize);
            old_entries_bottom_to_scan += 1;
            debug_assert!(app.get(old_entry).current_state != RouteLifecycle::Disposed);
            // Pageless routes will be recorded when we update the middle of the old list.
            if !app.get(old_entry).page_based {
                continue;
            }

            let settings = app.get(old_entry).route.settings(app);
            let page = settings.as_page().expect("a page-based route has a page");
            let Some(key) = page.key().cloned() else {
                continue;
            };

            if !old_entry.will_be_present(app) {
                phantom_entries.insert(old_entry);
                continue;
            }
            debug_assert!(!page_key_to_old_entry.iter().any(|(k, _)| **k == *key));
            page_key_to_old_entry.push((key, old_entry));
        }

        // Updates the middle of the list.
        while new_pages_bottom <= new_pages_top {
            let next_page = self.widget(app).pages[new_pages_bottom as usize].clone();
            new_pages_bottom += 1;
            let matching = next_page.key().and_then(|key| {
                page_key_to_old_entry
                    .iter()
                    .position(|(other, _)| **other == **key)
            });
            let matching = matching.filter(|index| {
                let entry = page_key_to_old_entry[*index].1;
                entry.can_update_from(app, &next_page)
            });
            match matching {
                None => {
                    // There is no matching key in the old history, we need to create a new
                    // route and wait for the transition delegate to decide how to add it into
                    // the history.
                    let context = self.context(app);
                    let route = next_page.create_route(app, context, &next_page);
                    debug_assert!(
                        route.settings(app) == RouteSettingsRef::Page(Rc::clone(&next_page)),
                        "The settings of a page-based Route must be the Page object. Please set \
                         the settings to the Page in the Page::create_route method."
                    );
                    let new_entry =
                        RouteEntry::new(app, route, RouteLifecycle::Staging, true, None);
                    needs_explicit_decision = true;
                    new_history.push(new_entry);
                }
                Some(index) => {
                    // Removes the key from the keyToOldEntry map so it will not be reused.
                    let (_, matching_entry) = page_key_to_old_entry.remove(index);
                    debug_assert!(matching_entry.can_update_from(app, &next_page));
                    let route = app.get(matching_entry).route;
                    route.update_settings(app, RouteSettingsRef::Page(next_page));
                    new_history.push(matching_entry);
                }
            }
        }

        // Any remaining old routes that do not have a match will need to be removed.
        let mut location_to_exiting_page_route: HashMap<
            Option<Handle<RouteEntry>>,
            Handle<RouteEntry>,
        > = HashMap::new();
        while old_entries_bottom <= old_entries_top {
            let potential_entry_to_remove = history.at(app, old_entries_bottom as usize);
            old_entries_bottom += 1;

            if !app.get(potential_entry_to_remove).page_based {
                debug_assert!(previous_old_page_route_entry.is_some());
                page_route_to_pageless_routes
                    .entry(previous_old_page_route_entry)
                    .or_default()
                    .push(potential_entry_to_remove);
                let previous = previous_old_page_route_entry.expect("asserted above");
                if previous.is_waiting_for_exiting_decision(app)
                    && potential_entry_to_remove.will_be_present(app)
                {
                    potential_entry_to_remove.mark_needs_exiting_decision(app);
                }
                continue;
            }

            let settings = app.get(potential_entry_to_remove).route.settings(app);
            let potential_page_to_remove = settings
                .as_page()
                .expect("a page-based route has a page")
                .clone();

            // Marks for transition delegate to remove if this old page does not have a key,
            // was not taken during updating the middle of new page, or is a phantom entry.
            let key_is_free = match potential_page_to_remove.key() {
                None => true,
                Some(key) => page_key_to_old_entry
                    .iter()
                    .any(|(other, _)| **other == **key),
            };
            if key_is_free || phantom_entries.contains(&potential_entry_to_remove) {
                location_to_exiting_page_route
                    .insert(previous_old_page_route_entry, potential_entry_to_remove);
                // We only need a decision if it has not already been popped.
                if potential_entry_to_remove.will_be_present(app) {
                    potential_entry_to_remove.mark_needs_exiting_decision(app);
                }
            }
            previous_old_page_route_entry = Some(potential_entry_to_remove);
        }

        // We've scanned the whole list.
        debug_assert!(old_entries_bottom == old_entries_top + 1);
        debug_assert!(new_pages_bottom == new_pages_top + 1);
        new_pages_top = self.widget(app).pages.len() as i64 - 1;
        old_entries_top = history.len(app) as i64 - 1;
        // Verifies we either reach the bottom or the oldEntriesBottom must be updatable by
        // newPagesBottom.
        debug_assert!(if old_entries_bottom <= old_entries_top {
            let entry = history.at(app, old_entries_bottom as usize);
            let page = self.widget(app).pages[new_pages_bottom as usize].clone();
            new_pages_bottom <= new_pages_top
                && app.get(entry).page_based
                && entry.can_update_from(app, &page)
        } else {
            new_pages_bottom > new_pages_top
        });

        // Updates the top of the list.
        while (old_entries_bottom <= old_entries_top) && (new_pages_bottom <= new_pages_top) {
            let old_entry = history.at(app, old_entries_bottom as usize);
            debug_assert!(app.get(old_entry).current_state != RouteLifecycle::Disposed);
            if !app.get(old_entry).page_based {
                debug_assert!(previous_old_page_route_entry.is_some());
                page_route_to_pageless_routes
                    .entry(previous_old_page_route_entry)
                    .or_default()
                    .push(old_entry);
                old_entries_bottom += 1;
                continue;
            }
            previous_old_page_route_entry = Some(old_entry);
            let new_page = self.widget(app).pages[new_pages_bottom as usize].clone();
            debug_assert!(old_entry.can_update_from(app, &new_page));
            let route = app.get(old_entry).route;
            route.update_settings(app, RouteSettingsRef::Page(new_page));
            new_history.push(old_entry);
            old_entries_bottom += 1;
            new_pages_bottom += 1;
        }

        // Finally, uses transition delegate to make explicit decision if needed.
        needs_explicit_decision =
            needs_explicit_decision || !location_to_exiting_page_route.is_empty();
        let results = if needs_explicit_decision {
            let delegate = Rc::clone(&self.widget(app).transition_delegate);
            transition(
                &*delegate,
                app,
                &new_history,
                &location_to_exiting_page_route,
                &page_route_to_pageless_routes,
            )
        } else {
            new_history
        };
        history.clear(app);
        // Adds the leading pageless routes if there is any.
        if let Some(pageless) = page_route_to_pageless_routes.get(&None).cloned() {
            history.add_all(app, pageless);
        }
        for result in results {
            history.add(app, result);
            if let Some(pageless) = page_route_to_pageless_routes.get(&Some(result)).cloned() {
                history.add_all(app, pageless);
            }
        }
        app.get_mut(self).debug_updating_page = false;
        app.get_mut(self).debug_locked = true;
        self.flush_history_updates(app, true);
        app.get_mut(self).debug_locked = false;
    }

    /// Dart's `_flushHistoryUpdates`: runs the pending lifecycle transitions of every entry.
    pub(crate) fn flush_history_updates(
        self: Handle<Self>,
        app: &mut App,
        rearrange_overlay: bool,
    ) {
        debug_assert!(!app.get(self).debug_updating_page);
        app.get_mut(self).flushing_history = true;
        let history = self.history(app);

        // Clean up the list, sending updates to the routes that changed. Notably, we don't send
        // the didChangePrevious/didChangeNext updates to those that did not change at this
        // point, because we're not yet sure exactly what the routes will be at the end of the
        // day (some might get disposed).
        let mut index: i64 = history.len(app) as i64 - 1;
        let mut next: Option<Handle<RouteEntry>> = None;
        let mut entry: Option<Handle<RouteEntry>> = Some(history.at(app, index as usize));
        let mut previous: Option<Handle<RouteEntry>> =
            (index > 0).then(|| history.at(app, (index - 1) as usize));
        // Whether there is a fully opaque route on top to silently remove or add a route
        // underneath.
        let mut can_remove_or_add = false;
        // The route that should trigger did_pop_next on the top active route.
        let mut popped_route: Option<AnyRoute> = None;
        // Whether we've seen the route that would get did_pop_next.
        let mut seen_top_active_route = false;
        let mut to_be_disposed: Vec<Handle<RouteEntry>> = Vec::new();
        while index >= 0 {
            loop {
                let current = entry.expect("the history has an entry at this index");
                match app.get(current).current_state {
                    RouteLifecycle::Add => {
                        debug_assert!(rearrange_overlay);
                        let previous_present = self
                            .get_route_before(app, index - 1, RouteEntry::is_present_predicate)
                            .map(|entry| app.get(entry).route);
                        current.handle_add(app, self, previous_present);
                        debug_assert!(app.get(current).current_state == RouteLifecycle::Adding);
                        continue;
                    }
                    RouteLifecycle::Adding => {
                        if can_remove_or_add || next.is_none() {
                            current.did_add(app, self, next.is_none());
                            debug_assert!(app.get(current).current_state == RouteLifecycle::Idle);
                            continue;
                        }
                    }
                    RouteLifecycle::Push
                    | RouteLifecycle::PushReplace
                    | RouteLifecycle::Replace => {
                        debug_assert!(rearrange_overlay);
                        let previous_route = previous.map(|entry| app.get(entry).route);
                        let previous_present = self
                            .get_route_before(app, index - 1, RouteEntry::is_present_predicate)
                            .map(|entry| app.get(entry).route);
                        current.handle_push(
                            app,
                            self,
                            next.is_none(),
                            previous_route,
                            previous_present,
                        );
                        debug_assert!(!matches!(
                            app.get(current).current_state,
                            RouteLifecycle::Push
                                | RouteLifecycle::PushReplace
                                | RouteLifecycle::Replace
                        ));
                        if app.get(current).current_state == RouteLifecycle::Idle {
                            continue;
                        }
                    }
                    RouteLifecycle::Pushing => {
                        // Will exit this state when the animation completes.
                        if !seen_top_active_route && let Some(popped) = popped_route {
                            current.handle_did_pop_next(app, popped);
                        }
                        seen_top_active_route = true;
                    }
                    RouteLifecycle::Idle => {
                        if !seen_top_active_route && let Some(popped) = popped_route {
                            current.handle_did_pop_next(app, popped);
                        }
                        seen_top_active_route = true;
                        // This route is idle, so we can remove or add routes underneath it
                        // without an animation.
                        can_remove_or_add = true;
                    }
                    RouteLifecycle::Pop => {
                        let previous_present = self
                            .get_route_before(app, index, RouteEntry::will_be_present_predicate)
                            .map(|entry| app.get(entry).route);
                        if !current.handle_pop(app, self, previous_present) {
                            debug_assert!(app.get(current).current_state == RouteLifecycle::Idle);
                            continue;
                        }
                        if !seen_top_active_route {
                            if let Some(popped) = popped_route {
                                current.handle_did_pop_next(app, popped);
                            }
                            popped_route = Some(app.get(current).route);
                        }
                        let route = app.get(current).route;
                        let previous_present = self
                            .get_route_before(app, index, RouteEntry::will_be_present_predicate)
                            .map(|entry| app.get(entry).route);
                        app.get_mut(self).observed_route_deletions.push_back(
                            NavigatorObservation::Pop {
                                primary_route: route,
                                secondary_route: previous_present,
                            },
                        );
                        if app.get(current).current_state == RouteLifecycle::Dispose {
                            // A route that doesn't animate through its exit will be disposed
                            // right away.
                            continue;
                        }
                        debug_assert!(app.get(current).current_state == RouteLifecycle::Popping);
                        can_remove_or_add = true;
                    }
                    RouteLifecycle::Popping => {
                        // Will exit this state when the route calls finalize_route.
                    }
                    RouteLifecycle::Complete => {
                        current.handle_complete(app);
                        debug_assert!(app.get(current).current_state == RouteLifecycle::Remove);
                        continue;
                    }
                    RouteLifecycle::Remove => {
                        if !seen_top_active_route && app.get(current).route.installed(app) {
                            if let Some(popped) = popped_route {
                                current.handle_did_pop_next(app, popped);
                            }
                            popped_route = None;
                        }
                        let previous_present = self
                            .get_route_before(app, index, RouteEntry::will_be_present_predicate)
                            .map(|entry| app.get(entry).route);
                        current.handle_removal(app, self, previous_present);
                        debug_assert!(app.get(current).current_state >= RouteLifecycle::Removing);
                        continue;
                    }
                    RouteLifecycle::Removing => {
                        if !can_remove_or_add && next.is_some() {
                            // We aren't allowed to remove this route yet.
                            break;
                        }
                        app.get_mut(current).current_state = RouteLifecycle::Dispose;
                        continue;
                    }
                    RouteLifecycle::Dispose => {
                        // Delay disposal until didChangeNext/didChangePrevious have been sent.
                        to_be_disposed.push(history.remove_at(app, index as usize));
                        entry = next;
                    }
                    RouteLifecycle::Disposing
                    | RouteLifecycle::Disposed
                    | RouteLifecycle::Staging => {
                        unreachable!("unexpected route lifecycle while flushing the history")
                    }
                }
                break;
            }
            index -= 1;
            next = entry;
            entry = previous;
            previous = (index > 0).then(|| history.at(app, (index - 1) as usize));
        }

        // Informs the observers about route changes.
        self.flush_observer_notifications(app);

        // Now that the list is clean, send the didChangeNext/didChangePrevious notifications.
        self.flush_route_announcement(app);

        // Announces route name changes.
        let last_entry = self.last_route_entry_where_or_null(app, RouteEntry::is_present_predicate);
        if let Some(last_entry) = last_entry
            && app.get(self).last_topmost_route != Some(last_entry)
        {
            let top_route = app.get(last_entry).route;
            let previous_top_route = app
                .get(self)
                .last_topmost_route
                .filter(|entry| app.contains(*entry))
                .map(|entry| app.get(entry).route);
            for observer in app.get(self).effective_observers.clone() {
                observer.did_change_top(app, top_route, previous_top_route);
            }
        }
        app.get_mut(self).last_topmost_route = last_entry;

        if self.widget(app).reports_route_update_to_engine {
            let route_name = last_entry.and_then(|entry| {
                app.get(entry)
                    .route
                    .settings(app)
                    .name()
                    .map(str::to_string)
            });
            if let Some(route_name) = route_name
                && Some(&route_name) != app.get(self).last_announced_route_name.as_ref()
            {
                // `SystemNavigator.routeInformationUpdated` waits; see PORTING.md.
                app.get_mut(self).last_announced_route_name = Some(route_name);
            }
        }

        // Lastly, removes the overlay entries of all marked entries and disposes them.
        for entry in to_be_disposed {
            NavigatorState::dispose_route_entry(app, entry, true);
        }
        if rearrange_overlay && let Some(overlay) = self.overlay(app) {
            let entries = self.all_route_overlay_entries(app);
            overlay.rearrange(app, entries, None, None);
        }
        if self.bucket(app).is_some() {
            let serializable = self.serializable_history(app);
            let history = self.history(app);
            serializable.update(app, history);
        }
        app.get_mut(self).flushing_history = false;
    }

    fn flush_observer_notifications(self: Handle<Self>, app: &mut App) {
        if app.get(self).effective_observers.is_empty() {
            app.get_mut(self).observed_route_deletions.clear();
            app.get_mut(self).observed_route_additions.clear();
            return;
        }
        while let Some(observation) = app.get_mut(self).observed_route_additions.pop_back() {
            for observer in app.get(self).effective_observers.clone() {
                observation.notify(app, observer);
            }
        }
        while let Some(observation) = app.get_mut(self).observed_route_deletions.pop_front() {
            for observer in app.get(self).effective_observers.clone() {
                observation.notify(app, observer);
            }
        }
    }

    fn flush_route_announcement(self: Handle<Self>, app: &mut App) {
        let history = self.history(app);
        let mut index: i64 = history.len(app) as i64 - 1;
        while index >= 0 {
            let entry = history.at(app, index as usize);
            if !entry.suitable_for_announcement(app) {
                index -= 1;
                continue;
            }
            let next = self.get_route_after(
                app,
                index + 1,
                RouteEntry::suitable_for_transition_animation_predicate,
            );
            let next_route = next.map(|entry| app.get(entry).route);

            if Some(next_route) != app.get(entry).last_announced_next_route {
                if entry.should_announce_change_to_next(app, next_route) {
                    let route = app.get(entry).route;
                    route.did_change_next(app, next_route);
                }
                app.get_mut(entry).last_announced_next_route = Some(next_route);
            }
            let previous = self.get_route_before(
                app,
                index - 1,
                RouteEntry::suitable_for_transition_animation_predicate,
            );
            let previous_route = previous.map(|entry| app.get(entry).route);
            if Some(previous_route) != app.get(entry).last_announced_previous_route {
                let route = app.get(entry).route;
                route.did_change_previous(app, previous_route);
                app.get_mut(entry).last_announced_previous_route = Some(previous_route);
            }
            index -= 1;
        }
    }

    fn route_named(
        self: Handle<Self>,
        app: &mut App,
        name: &str,
        arguments: Option<Rc<dyn Any>>,
        allow_null: bool,
    ) -> Option<AnyRoute> {
        debug_assert!(!app.get(self).debug_locked);
        let on_generate_route = self.widget(app).on_generate_route.clone();
        let Some(on_generate_route) = on_generate_route else {
            assert!(
                allow_null,
                "Navigator.on_generate_route was None, but the route named \"{name}\" was \
                 referenced. To use the Navigator API with named routes (push_named, \
                 push_replacement_named, or push_named_and_remove_until), the Navigator must be \
                 provided with an on_generate_route handler."
            );
            return None;
        };
        let settings = RouteSettings {
            name: Some(name.to_string()),
            arguments,
        };
        let mut route = on_generate_route(app, &settings);
        if route.is_none() && !allow_null {
            let on_unknown_route = self.widget(app).on_unknown_route.clone();
            let on_unknown_route = on_unknown_route.unwrap_or_else(|| {
                panic!(
                    "Navigator.on_generate_route returned None when requested to build route \
                     \"{name}\". The on_generate_route callback must never return None, unless \
                     an on_unknown_route callback is provided as well."
                )
            });
            route = on_unknown_route(app, &settings);
            assert!(
                route.is_some(),
                "Navigator.on_unknown_route returned None when requested to build route \
                 \"{name}\". The on_unknown_route callback must never return None."
            );
        }
        debug_assert!(route.is_some() || allow_null);
        route
    }
}

impl NavigatorState {
    /// Push a named route onto the navigator.
    ///
    /// The route name will be passed to [`Navigator::on_generate_route`]. The returned route is
    /// the one that was pushed; register [`AnyRoute::when_popped`] on it for what Dart's
    /// returned future resolves with.
    pub fn push_named(
        self: Handle<Self>,
        app: &mut App,
        route_name: &str,
        arguments: Option<Rc<dyn Any>>,
    ) -> AnyRoute {
        let route = self
            .route_named(app, route_name, arguments, false)
            .expect("a route for the given name");
        self.push(app, route)
    }

    /// Push a named route onto the navigator and return its restoration ID.
    ///
    /// The route name will be passed to [`Navigator::on_generate_route`].
    pub fn restorable_push_named(
        self: Handle<Self>,
        app: &mut App,
        route_name: &str,
        arguments: Option<RestorationData>,
    ) -> String {
        let restoration_scope_id = self.next_pageless_restoration_scope_id(app);
        let information = Rc::new(RestorationInformation::Named {
            name: route_name.to_string(),
            arguments,
            restoration_scope_id,
        });
        let entry = information.to_route_entry(app, self, RouteLifecycle::Push);
        self.push_entry(app, entry);
        entry.restoration_id(app).expect("a restorable entry")
    }

    /// Replace the current route of the navigator by pushing the route named `route_name` and
    /// then disposing the previous route once the animation has completed.
    pub fn push_replacement_named(
        self: Handle<Self>,
        app: &mut App,
        route_name: &str,
        result: RouteResult,
        arguments: Option<Rc<dyn Any>>,
    ) -> AnyRoute {
        let route = self
            .route_named(app, route_name, arguments, false)
            .expect("a route for the given name");
        self.push_replacement(app, route, result)
    }

    /// The restorable version of [`push_replacement_named`](Self::push_replacement_named).
    pub fn restorable_push_replacement_named(
        self: Handle<Self>,
        app: &mut App,
        route_name: &str,
        result: RouteResult,
        arguments: Option<RestorationData>,
    ) -> String {
        let restoration_scope_id = self.next_pageless_restoration_scope_id(app);
        let information = Rc::new(RestorationInformation::Named {
            name: route_name.to_string(),
            arguments,
            restoration_scope_id,
        });
        let entry = information.to_route_entry(app, self, RouteLifecycle::PushReplace);
        self.push_replacement_entry(app, entry, result);
        entry.restoration_id(app).expect("a restorable entry")
    }

    /// Pop the current route off the navigator and push a named route in its place.
    pub fn pop_and_push_named(
        self: Handle<Self>,
        app: &mut App,
        route_name: &str,
        result: RouteResult,
        arguments: Option<Rc<dyn Any>>,
    ) -> AnyRoute {
        self.pop(app, result);
        self.push_named(app, route_name, arguments)
    }

    /// The restorable version of [`pop_and_push_named`](Self::pop_and_push_named).
    pub fn restorable_pop_and_push_named(
        self: Handle<Self>,
        app: &mut App,
        route_name: &str,
        result: RouteResult,
        arguments: Option<RestorationData>,
    ) -> String {
        self.pop(app, result);
        self.restorable_push_named(app, route_name, arguments)
    }

    /// Push the route with the given name onto the navigator, and then remove all the previous
    /// routes until the `predicate` returns true.
    pub fn push_named_and_remove_until(
        self: Handle<Self>,
        app: &mut App,
        new_route_name: &str,
        predicate: RoutePredicate,
        arguments: Option<Rc<dyn Any>>,
    ) -> AnyRoute {
        let route = self
            .route_named(app, new_route_name, arguments, false)
            .expect("a route for the given name");
        self.push_and_remove_until(app, route, predicate)
    }

    /// The restorable version of
    /// [`push_named_and_remove_until`](Self::push_named_and_remove_until).
    pub fn restorable_push_named_and_remove_until(
        self: Handle<Self>,
        app: &mut App,
        new_route_name: &str,
        predicate: RoutePredicate,
        arguments: Option<RestorationData>,
    ) -> String {
        let restoration_scope_id = self.next_pageless_restoration_scope_id(app);
        let information = Rc::new(RestorationInformation::Named {
            name: new_route_name.to_string(),
            arguments,
            restoration_scope_id,
        });
        let entry = information.to_route_entry(app, self, RouteLifecycle::Push);
        self.push_entry_and_remove_until(app, entry, predicate);
        entry.restoration_id(app).expect("a restorable entry")
    }

    /// Push the given route onto the navigator.
    ///
    /// Returns the route that was pushed; register [`AnyRoute::when_popped`] on it for what
    /// Dart's returned future resolves with.
    pub fn push(self: Handle<Self>, app: &mut App, route: AnyRoute) -> AnyRoute {
        let entry = RouteEntry::new(app, route, RouteLifecycle::Push, false, None);
        self.push_entry(app, entry);
        route
    }

    /// Push a new route onto the navigator and return its restoration ID.
    ///
    /// The route is created by `route_builder`, which is called again during state restoration.
    pub fn restorable_push(
        self: Handle<Self>,
        app: &mut App,
        route_builder: RestorableRouteBuilder,
        arguments: Option<RestorationData>,
    ) -> String {
        let restoration_scope_id = self.next_pageless_restoration_scope_id(app);
        let information = Rc::new(RestorationInformation::Anonymous {
            route_builder,
            arguments,
            restoration_scope_id,
        });
        let entry = information.to_route_entry(app, self, RouteLifecycle::Push);
        self.push_entry(app, entry);
        entry.restoration_id(app).expect("a restorable entry")
    }

    fn push_entry(self: Handle<Self>, app: &mut App, entry: Handle<RouteEntry>) {
        debug_assert!(!app.get(self).debug_locked);
        app.get_mut(self).debug_locked = true;
        debug_assert!(!app.get(entry).route.installed(app));
        debug_assert!(app.get(entry).current_state == RouteLifecycle::Push);
        self.history(app).add(app, entry);
        self.flush_history_updates(app, true);
        app.get_mut(self).debug_locked = false;
        let route = app.get(entry).route;
        self.after_navigation(app, Some(route));
    }

    fn after_navigation(self: Handle<Self>, app: &mut App, route: Option<AnyRoute>) {
        // Dart posts a `Flutter.Navigation` developer event here; diagnostics wait.
        let _ = route;
        self.cancel_active_pointers(app);
    }

    /// Replace the current route of the navigator by pushing the given route and then disposing
    /// the previous route once the new route has finished animating in.
    pub fn push_replacement(
        self: Handle<Self>,
        app: &mut App,
        new_route: AnyRoute,
        result: RouteResult,
    ) -> AnyRoute {
        debug_assert!(!new_route.installed(app));
        let entry = RouteEntry::new(app, new_route, RouteLifecycle::PushReplace, false, None);
        self.push_replacement_entry(app, entry, result);
        new_route
    }

    /// The restorable version of [`push_replacement`](Self::push_replacement).
    pub fn restorable_push_replacement(
        self: Handle<Self>,
        app: &mut App,
        route_builder: RestorableRouteBuilder,
        result: RouteResult,
        arguments: Option<RestorationData>,
    ) -> String {
        let restoration_scope_id = self.next_pageless_restoration_scope_id(app);
        let information = Rc::new(RestorationInformation::Anonymous {
            route_builder,
            arguments,
            restoration_scope_id,
        });
        let entry = information.to_route_entry(app, self, RouteLifecycle::PushReplace);
        self.push_replacement_entry(app, entry, result);
        entry.restoration_id(app).expect("a restorable entry")
    }

    fn push_replacement_entry(
        self: Handle<Self>,
        app: &mut App,
        entry: Handle<RouteEntry>,
        result: RouteResult,
    ) {
        debug_assert!(!app.get(self).debug_locked);
        app.get_mut(self).debug_locked = true;
        debug_assert!(!app.get(entry).route.installed(app));
        debug_assert!(!self.history(app).is_empty(app));
        debug_assert!(
            self.last_route_entry_where_or_null(app, RouteEntry::is_present_predicate)
                .is_some(),
            "Navigator has no active routes to replace."
        );
        debug_assert!(app.get(entry).current_state == RouteLifecycle::PushReplace);
        let last_present = self
            .last_route_entry_where_or_null(app, RouteEntry::is_present_predicate)
            .expect("asserted above");
        last_present.complete(app, result, true, true);
        self.history(app).add(app, entry);
        self.flush_history_updates(app, true);
        app.get_mut(self).debug_locked = false;
        let route = app.get(entry).route;
        self.after_navigation(app, Some(route));
    }

    /// Push the given route onto the navigator, and then remove all the previous routes until the
    /// `predicate` returns true.
    pub fn push_and_remove_until(
        self: Handle<Self>,
        app: &mut App,
        new_route: AnyRoute,
        predicate: RoutePredicate,
    ) -> AnyRoute {
        debug_assert!(!new_route.installed(app));
        debug_assert!(new_route.overlay_entries(app).is_empty());
        let entry = RouteEntry::new(app, new_route, RouteLifecycle::Push, false, None);
        self.push_entry_and_remove_until(app, entry, predicate);
        new_route
    }

    /// The restorable version of [`push_and_remove_until`](Self::push_and_remove_until).
    pub fn restorable_push_and_remove_until(
        self: Handle<Self>,
        app: &mut App,
        new_route_builder: RestorableRouteBuilder,
        predicate: RoutePredicate,
        arguments: Option<RestorationData>,
    ) -> String {
        let restoration_scope_id = self.next_pageless_restoration_scope_id(app);
        let information = Rc::new(RestorationInformation::Anonymous {
            route_builder: new_route_builder,
            arguments,
            restoration_scope_id,
        });
        let entry = information.to_route_entry(app, self, RouteLifecycle::Push);
        self.push_entry_and_remove_until(app, entry, predicate);
        entry.restoration_id(app).expect("a restorable entry")
    }

    fn push_entry_and_remove_until(
        self: Handle<Self>,
        app: &mut App,
        entry: Handle<RouteEntry>,
        predicate: RoutePredicate,
    ) {
        debug_assert!(!app.get(self).debug_locked);
        app.get_mut(self).debug_locked = true;
        debug_assert!(!app.get(entry).route.installed(app));
        debug_assert!(app.get(entry).route.overlay_entries(app).is_empty());
        debug_assert!(app.get(entry).current_state == RouteLifecycle::Push);
        let history = self.history(app);
        let mut index: i64 = history.len(app) as i64 - 1;
        history.add(app, entry);
        while index >= 0 {
            let candidate = history.at(app, index as usize);
            let route = app.get(candidate).route;
            if predicate(app, route) {
                break;
            }
            if candidate.is_present(app) {
                candidate.complete(app, None, false, true);
            }
            index -= 1;
        }
        self.flush_history_updates(app, true);
        app.get_mut(self).debug_locked = false;
        let route = app.get(entry).route;
        self.after_navigation(app, Some(route));
    }

    /// Replaces a route on the navigator with a new route.
    pub fn replace(self: Handle<Self>, app: &mut App, old_route: AnyRoute, new_route: AnyRoute) {
        debug_assert!(!app.get(self).debug_locked);
        debug_assert!(old_route.is_installed_in(app, self));
        let entry = RouteEntry::new(app, new_route, RouteLifecycle::Replace, false, None);
        self.replace_entry(app, entry, old_route);
    }

    /// The restorable version of [`replace`](Self::replace).
    pub fn restorable_replace(
        self: Handle<Self>,
        app: &mut App,
        old_route: AnyRoute,
        new_route_builder: RestorableRouteBuilder,
        arguments: Option<RestorationData>,
    ) -> String {
        debug_assert!(old_route.is_installed_in(app, self));
        let restoration_scope_id = self.next_pageless_restoration_scope_id(app);
        let information = Rc::new(RestorationInformation::Anonymous {
            route_builder: new_route_builder,
            arguments,
            restoration_scope_id,
        });
        let entry = information.to_route_entry(app, self, RouteLifecycle::Replace);
        self.replace_entry(app, entry, old_route);
        entry.restoration_id(app).expect("a restorable entry")
    }

    fn replace_entry(
        self: Handle<Self>,
        app: &mut App,
        entry: Handle<RouteEntry>,
        old_route: AnyRoute,
    ) {
        debug_assert!(!app.get(self).debug_locked);
        if old_route == app.get(entry).route {
            return;
        }
        app.get_mut(self).debug_locked = true;
        debug_assert!(app.get(entry).current_state == RouteLifecycle::Replace);
        debug_assert!(!app.get(entry).route.installed(app));
        let history = self.history(app);
        let index = history
            .index_where(app, |app, other| app.get(other).route == old_route)
            .expect("This Navigator does not contain the specified old_route.");
        debug_assert!(
            history.at(app, index).is_present(app),
            "The specified old_route has already been removed from the Navigator."
        );
        let was_current = old_route.is_current(app);
        history.insert(app, index + 1, entry);
        history.at(app, index).complete(app, None, true, true);
        self.flush_history_updates(app, true);
        app.get_mut(self).debug_locked = false;
        if was_current {
            let route = app.get(entry).route;
            self.after_navigation(app, Some(route));
        }
    }

    /// Replaces a route on the navigator with a new route. The route to be replaced is the one
    /// below the given `anchor_route`.
    pub fn replace_route_below(
        self: Handle<Self>,
        app: &mut App,
        anchor_route: AnyRoute,
        new_route: AnyRoute,
    ) {
        debug_assert!(!new_route.installed(app));
        debug_assert!(anchor_route.is_installed_in(app, self));
        let entry = RouteEntry::new(app, new_route, RouteLifecycle::Replace, false, None);
        self.replace_entry_below(app, entry, anchor_route);
    }

    /// The restorable version of [`replace_route_below`](Self::replace_route_below).
    pub fn restorable_replace_route_below(
        self: Handle<Self>,
        app: &mut App,
        anchor_route: AnyRoute,
        new_route_builder: RestorableRouteBuilder,
        arguments: Option<RestorationData>,
    ) -> String {
        debug_assert!(anchor_route.is_installed_in(app, self));
        let restoration_scope_id = self.next_pageless_restoration_scope_id(app);
        let information = Rc::new(RestorationInformation::Anonymous {
            route_builder: new_route_builder,
            arguments,
            restoration_scope_id,
        });
        let entry = information.to_route_entry(app, self, RouteLifecycle::Replace);
        self.replace_entry_below(app, entry, anchor_route);
        entry.restoration_id(app).expect("a restorable entry")
    }

    fn replace_entry_below(
        self: Handle<Self>,
        app: &mut App,
        entry: Handle<RouteEntry>,
        anchor_route: AnyRoute,
    ) {
        debug_assert!(!app.get(self).debug_locked);
        app.get_mut(self).debug_locked = true;
        let history = self.history(app);
        let anchor_index = history
            .index_where(app, |app, other| app.get(other).route == anchor_route)
            .expect("This Navigator does not contain the specified anchor_route.");
        debug_assert!(
            history.at(app, anchor_index).is_present(app),
            "The specified anchor_route has already been removed from the Navigator."
        );
        let mut index: i64 = anchor_index as i64 - 1;
        while index >= 0 {
            if history.at(app, index as usize).is_present(app) {
                break;
            }
            index -= 1;
        }
        assert!(
            index >= 0,
            "There are no routes below the specified anchor_route."
        );
        history.insert(app, (index + 1) as usize, entry);
        history
            .at(app, index as usize)
            .complete(app, None, true, true);
        self.flush_history_updates(app, true);
        app.get_mut(self).debug_locked = false;
    }

    /// Whether the navigator can be popped.
    pub fn can_pop(self: Handle<Self>, app: &App) -> bool {
        let mut present = self
            .history_entries(app)
            .into_iter()
            .filter(|entry| entry.is_present(app));
        let Some(first) = present.next() else {
            // Non-existent route.
            return false;
        };
        if app.get(first).route.will_handle_pop_internally(app) {
            // The first route can handle pops itself.
            return true;
        }
        // There's at least two routes, so we can pop.
        present.next().is_some()
    }

    /// Consults the current route's [`AnyRoute::pop_disposition`] method, and acts accordingly,
    /// potentially popping the route as a result; returns whether the pop request should be
    /// considered handled.
    pub fn maybe_pop(self: Handle<Self>, app: &mut App, result: RouteResult) -> bool {
        let last_entry = self.last_route_entry_where_or_null(app, RouteEntry::is_present_predicate);
        let Some(last_entry) = last_entry else {
            return false;
        };
        debug_assert!(app.get(last_entry).route.is_installed_in(app, self));

        // Dart awaits the deprecated `willPop`; there is no event loop here, so it answers
        // within the call.
        let route = app.get(last_entry).route;
        if route.will_pop(app) == RoutePopDisposition::DoNotPop {
            return true;
        }
        if !self.mounted(app) {
            // Forget about this pop, we were disposed in the meantime.
            return true;
        }

        let new_last_entry =
            self.last_route_entry_where_or_null(app, RouteEntry::is_present_predicate);
        if Some(last_entry) != new_last_entry {
            // Forget about this pop, something happened to our history in the meantime.
            return true;
        }

        match route.pop_disposition(app) {
            RoutePopDisposition::Bubble => false,
            RoutePopDisposition::Pop => {
                self.pop(app, result);
                true
            }
            RoutePopDisposition::DoNotPop => {
                route.on_pop_invoked_with_result(app, false, result);
                true
            }
        }
    }

    /// Pop the top-most route off the navigator.
    pub fn pop(self: Handle<Self>, app: &mut App, result: RouteResult) {
        debug_assert!(!app.get(self).debug_locked);
        app.get_mut(self).debug_locked = true;
        let entry = self
            .last_route_entry_where_or_null(app, RouteEntry::is_present_predicate)
            .expect("a present route to pop");
        let on_pop_page = self.widget(app).on_pop_page.clone();
        if app.get(entry).page_based
            && let Some(on_pop_page) = on_pop_page
        {
            let route = app.get(entry).route;
            if on_pop_page(app, route, result.clone()) {
                if app.get(entry).current_state <= RouteLifecycle::Idle {
                    debug_assert!(route.popped_is_completed(app));
                    app.get_mut(entry).current_state = RouteLifecycle::Pop;
                }
                route.on_pop_invoked_with_result(app, true, result);
            }
        } else {
            entry.pop(app, result, true);
            debug_assert!(app.get(entry).current_state == RouteLifecycle::Pop);
        }
        if app.get(entry).current_state == RouteLifecycle::Pop {
            self.flush_history_updates(app, false);
        }
        app.get_mut(self).debug_locked = false;
        let route = app.get(entry).route;
        self.after_navigation(app, Some(route));
    }

    /// Calls [`pop`](Self::pop) repeatedly until the `predicate` returns true.
    pub fn pop_until(self: Handle<Self>, app: &mut App, predicate: RoutePredicate) {
        let mut candidate =
            self.last_route_entry_where_or_null(app, RouteEntry::is_present_predicate);
        while let Some(entry) = candidate {
            let route = app.get(entry).route;
            if predicate(app, route) {
                return;
            }
            self.pop(app, None);
            candidate = self.last_route_entry_where_or_null(app, RouteEntry::is_present_predicate);
        }
    }

    /// Calls [`pop`](Self::pop) repeatedly until the `predicate` returns true, passing `result`
    /// to the last route that is popped.
    pub fn pop_until_with_result(
        self: Handle<Self>,
        app: &mut App,
        predicate: RoutePredicate,
        result: RouteResult,
    ) {
        let mut candidate =
            self.last_route_entry_where_or_null(app, RouteEntry::is_present_predicate);
        while let Some(entry) = candidate {
            let route = app.get(entry).route;
            if predicate(app, route) {
                return;
            }

            let next = self.last_route_entry_where_or_null(app, move |app, other| {
                RouteEntry::is_present_predicate(app, other) && other != entry
            });

            let pops_with_result = match next {
                Some(next) => {
                    let next_route = app.get(next).route;
                    !route.will_handle_pop_internally(app) && predicate(app, next_route)
                }
                None => false,
            };
            if pops_with_result {
                self.pop(app, result.clone());
            } else {
                self.pop(app, None);
            }

            candidate = self.last_route_entry_where_or_null(app, RouteEntry::is_present_predicate);
        }
    }

    /// Immediately remove `route` from the navigator, and dispose of it.
    pub fn remove_route(self: Handle<Self>, app: &mut App, route: AnyRoute, result: RouteResult) {
        debug_assert!(!app.get(self).debug_locked);
        app.get_mut(self).debug_locked = true;
        debug_assert!(route.is_installed_in(app, self));
        let was_current = route.is_current(app);
        let entry = self
            .first_route_entry_where_or_null(app, move |app, entry| app.get(entry).route == route)
            .expect("This Navigator does not contain the specified route.");
        entry.complete(app, result, false, true);
        self.flush_history_updates(app, false);
        app.get_mut(self).debug_locked = false;
        if was_current {
            let route = self
                .last_route_entry_where_or_null(app, RouteEntry::is_present_predicate)
                .map(|entry| app.get(entry).route);
            self.after_navigation(app, route);
        }
    }

    /// Immediately remove a route from the navigator, and dispose of it. The route to be removed
    /// is the one below the given `anchor_route`.
    pub fn remove_route_below(
        self: Handle<Self>,
        app: &mut App,
        anchor_route: AnyRoute,
        result: RouteResult,
    ) {
        debug_assert!(!app.get(self).debug_locked);
        app.get_mut(self).debug_locked = true;
        debug_assert!(anchor_route.is_installed_in(app, self));
        let history = self.history(app);
        let anchor_index = history
            .index_where(app, move |app, entry| app.get(entry).route == anchor_route)
            .expect("This Navigator does not contain the specified anchor_route.");
        debug_assert!(
            history.at(app, anchor_index).is_present(app),
            "The specified anchor_route has already been removed from the Navigator."
        );
        let mut index: i64 = anchor_index as i64 - 1;
        while index >= 0 {
            if history.at(app, index as usize).is_present(app) {
                break;
            }
            index -= 1;
        }
        assert!(
            index >= 0,
            "There are no routes below the specified anchor_route."
        );
        history
            .at(app, index as usize)
            .complete(app, result, false, true);
        self.flush_history_updates(app, false);
        app.get_mut(self).debug_locked = false;
    }

    /// Complete the lifecycle for a route that has been popped off the navigator.
    ///
    /// When the navigator pops a route, the navigator retains a reference to the route in order
    /// to call [`Route::dispose`] if the navigator itself is removed from the tree. When the
    /// route is finished with any exit animation, the route should call this function to complete
    /// its lifecycle (e.g., to receive a call to `dispose`).
    pub fn finalize_route(self: Handle<Self>, app: &mut App, route: AnyRoute) {
        // FinalizeRoute may have been called while we were already locked as a
        // responsibility of another route entry. For instance, popping a route may lead to
        // another route with the same animation being pushed and immediately finalized.
        let was_debug_locked = app.get(self).debug_locked;
        app.get_mut(self).debug_locked = true;
        let history = self.history(app);
        debug_assert!(
            history
                .entries(app)
                .iter()
                .filter(|entry| app.get(**entry).route == route)
                .count()
                == 1
        );
        let index = history
            .index_where(app, move |app, entry| app.get(entry).route == route)
            .expect("This Navigator does not contain the specified route.");
        let entry = history.at(app, index);

        // For page-based route with zero transition, the finalizeRoute can be called on
        // the entry directly without popping the entry from the history.
        if app.get(entry).page_based && app.get(entry).current_state < RouteLifecycle::Pop {
            let previous_present = self
                .get_route_before(app, index as i64 - 1, RouteEntry::will_be_present_predicate)
                .map(|entry| app.get(entry).route);
            app.get_mut(self)
                .observed_route_deletions
                .push_back(NavigatorObservation::Pop {
                    primary_route: route,
                    secondary_route: previous_present,
                });
        } else {
            debug_assert!(app.get(entry).current_state == RouteLifecycle::Popping);
        }
        entry.finalize(app);

        // finalizeRoute can be called during _flushHistoryUpdates if a pop is called from a
        // route's dispose.
        if !app.get(self).flushing_history {
            self.flush_history_updates(app, false);
        }
        app.get_mut(self).debug_locked = was_debug_locked;
    }

    fn get_route_by_id(self: Handle<Self>, app: &App, id: &str) -> Option<AnyRoute> {
        let id = id.to_string();
        self.first_route_entry_where_or_null(app, move |app, entry| {
            entry.restoration_id(app).as_deref() == Some(id.as_str())
        })
        .map(|entry| app.get(entry).route)
    }

    fn set_user_gestures_in_progress(self: Handle<Self>, app: &mut App, value: i64) {
        app.get_mut(self).user_gestures_in_progress_count = value;
        let notifier = self.user_gesture_in_progress_notifier(app);
        notifier.set_value(app, value > 0);
    }

    /// Called when a route is about to be moved by a user gesture.
    ///
    /// This is called by the route that is being moved, and is used to inform the observers.
    pub fn did_start_user_gesture(self: Handle<Self>, app: &mut App) {
        let count = app.get(self).user_gestures_in_progress_count + 1;
        self.set_user_gestures_in_progress(app, count);
        if count == 1 {
            let history = self.history(app);
            let route_index = self.get_index_before(
                app,
                history.len(app) as i64 - 1,
                RouteEntry::will_be_present_predicate,
            );
            let route = app.get(history.at(app, route_index as usize)).route;
            let mut previous_route = None;
            if !route.will_handle_pop_internally(app) && route_index > 0 {
                previous_route = Some(
                    app.get(
                        self.get_route_before(
                            app,
                            route_index - 1,
                            RouteEntry::will_be_present_predicate,
                        )
                        .expect("a route below the one being moved"),
                    )
                    .route,
                );
            }
            for observer in app.get(self).effective_observers.clone() {
                observer.did_start_user_gesture(app, route, previous_route);
            }
        }
    }

    /// A user gesture completed.
    ///
    /// Notifies the navigator that a gesture regarding the navigation has stopped.
    pub fn did_stop_user_gesture(self: Handle<Self>, app: &mut App) {
        debug_assert!(app.get(self).user_gestures_in_progress_count > 0);
        let count = app.get(self).user_gestures_in_progress_count - 1;
        self.set_user_gestures_in_progress(app, count);
        if count == 0 {
            for observer in app.get(self).effective_observers.clone() {
                observer.did_stop_user_gesture(app);
            }
        }
    }

    fn handle_pointer_down(self: Handle<Self>, app: &mut App, pointer: i64) {
        app.get_mut(self).active_pointers.insert(pointer);
    }

    fn handle_pointer_up_or_cancel(self: Handle<Self>, app: &mut App, pointer: i64) {
        app.get_mut(self).active_pointers.remove(&pointer);
    }

    fn cancel_active_pointers(self: Handle<Self>, app: &mut App) {
        // If we're between frames (SchedulerPhase.idle) then absorb any subsequent pointers
        // from this frame. The absorbing flag will be reset in the next build.
        if SchedulerBinding::scheduler_phase(app) == SchedulerPhase::Idle {
            let absorber = app
                .get(self)
                .overlay_key
                .clone()
                .and_then(|key| key.current_context(app))
                .and_then(|context| {
                    context.find_ancestor_render_object_of_type::<RenderAbsorbPointer>(app)
                });
            if let Some(absorber) = absorber {
                absorber.set_absorbing(app, true);
            }
            // We do this in setState so that we will reset _absorbing in build.
            self.set_state(app, |_state| {});
        }
        let pointers: Vec<i64> = app.get(self).active_pointers.iter().copied().collect();
        let binding = GestureBinding::instance(app);
        for pointer in pointers {
            binding.cancel_pointer(app, pointer);
        }
    }
}

impl TickerProviderStateMixin for NavigatorState {
    fn ticker_provider_data(self: Handle<Self>, app: &App) -> &TickerProviderStateMixinData {
        &app.get(self).ticker_provider
    }

    fn ticker_provider_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut TickerProviderStateMixinData {
        &mut app.get_mut(self).ticker_provider
    }
}

impl TickerProviderObject for NavigatorState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
        TickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl RestorationMixin for NavigatorState {
    crate::restoration_mixin_accessors!();

    fn restoration_id(self: Handle<Self>, app: &App) -> Option<&str> {
        self.widget(app).restoration_scope_id.as_deref()
    }

    fn restore_state(
        self: Handle<Self>,
        app: &mut App,
        _old_bucket: Option<Handle<RestorationBucket>>,
        _initial_restore: bool,
    ) {
        let raw_id = app
            .get(self)
            .raw_next_pageless_restoration_scope_id
            .expect("init_state has run");
        self.register_for_restoration(app, raw_id.as_property(), "id");
        let serializable = self.serializable_history(app);
        self.register_for_restoration(app, serializable.as_property(), "history");

        // Delete everything in the old history and clear the overlay.
        self.forced_dispose_all_route_entries(app);
        debug_assert!(self.history(app).is_empty(app));
        app.get_mut(self).overlay_key = Some(GlobalKey::new());

        // Populate the new history from restoration data.
        let history = self.history(app);
        let restored = serializable.restore_entries_for_page(app, None, self);
        history.add_all(app, restored);
        for page in self.widget(app).pages.clone() {
            let context = self.context(app);
            let route = page.create_route(app, context, &page);
            debug_assert!(
                route.settings(app) == RouteSettingsRef::Page(Rc::clone(&page)),
                "The settings of a page-based Route must be the Page object. Please set the \
                 settings to the Page in the Page::create_route method."
            );
            let entry = RouteEntry::new(app, route, RouteLifecycle::Add, true, None);
            history.add(app, entry);
            let restored = serializable.restore_entries_for_page(app, Some(entry), self);
            history.add_all(app, restored);
        }

        // If there was nothing to restore, we need to process the initial route.
        if !serializable.has_data(app) {
            let mut initial_route = self.widget(app).initial_route.clone();
            if self.widget(app).pages.is_empty() {
                initial_route =
                    initial_route.or_else(|| Some(Navigator::DEFAULT_ROUTE_NAME.to_string()));
            }
            if initial_route.is_some() {
                let on_generate_initial_routes =
                    Rc::clone(&self.widget(app).on_generate_initial_routes);
                let name = self
                    .widget(app)
                    .initial_route
                    .clone()
                    .unwrap_or_else(|| Navigator::DEFAULT_ROUTE_NAME.to_string());
                let routes = on_generate_initial_routes(app, self, &name);
                let mut entries = Vec::with_capacity(routes.len());
                for route in routes {
                    let information = route.settings(app).name().map(|name| {
                        let restoration_scope_id = self.next_pageless_restoration_scope_id(app);
                        Rc::new(RestorationInformation::Named {
                            name: name.to_string(),
                            arguments: None,
                            restoration_scope_id,
                        })
                    });
                    entries.push(RouteEntry::new(
                        app,
                        route,
                        RouteLifecycle::Add,
                        false,
                        information,
                    ));
                }
                history.add_all(app, entries);
            }
        }

        assert!(
            !history.is_empty(app),
            "All routes returned by on_generate_initial_routes are not restorable. Please make \
             sure that all routes returned by on_generate_initial_routes have their RouteSettings \
             defined with names that are defined in the app's routes table."
        );
        debug_assert!(!app.get(self).debug_locked);
        app.get_mut(self).debug_locked = true;
        self.flush_history_updates(app, true);
        app.get_mut(self).debug_locked = false;
    }

    fn did_toggle_bucket(
        self: Handle<Self>,
        app: &mut App,
        old_bucket: Option<Handle<RestorationBucket>>,
    ) {
        let _ = old_bucket;
        let serializable = self.serializable_history(app);
        if self.bucket(app).is_some() {
            let history = self.history(app);
            serializable.update(app, history);
        } else {
            serializable.clear(app);
        }
    }
}

impl State for NavigatorState {
    type Widget = Navigator;

    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let history = History::new(app);
        let focus_node = FocusNode::new(app);
        focus_node
            .as_node()
            .set_debug_label(app, Some("Navigator".to_string()));
        let serializable_history = HistoryProperty::new(app);
        let raw_next_pageless_restoration_scope_id = RestorableInt::new(app, 0);
        let user_gesture_in_progress_notifier = app.create(ValueNotifier::new(false));
        {
            let state = app.get_mut(self);
            state.history = Some(history);
            state.focus_node = Some(focus_node);
            state.serializable_history = Some(serializable_history);
            state.raw_next_pageless_restoration_scope_id =
                Some(raw_next_pageless_restoration_scope_id);
            state.user_gesture_in_progress_notifier = Some(user_gesture_in_progress_notifier);
        }
        debug_assert!(self.debug_check_page_api_parameters(app));
        for observer in self.widget(app).observers.clone() {
            debug_assert!(observer.navigator(app).is_none());
            observer.set_navigator(app, Some(self));
        }
        app.get_mut(self).effective_observers = self.widget(app).observers.clone();

        // We have to manually extract the inherited widget in initState because the
        // `didChangeDependencies` is called after the state is fully created.
        let context = self.context(app);
        let hero_controller = context
            .get_element_for_inherited_widget_of_exact_type::<HeroControllerScope>(app)
            .and_then(|element| {
                crate::framework::downcast_widget::<HeroControllerScope>(&**element.widget(app))
                    .and_then(|scope| scope.controller)
            });
        self.update_hero_controller(app, hero_controller);

        // `SystemNavigator.selectSingleEntryHistory` and the accessibility focus listener wait;
        // see PORTING.md.
        history.add_listener(
            app,
            Listener::handle_method(self, NavigatorState::handle_history_changed),
        );
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        self.did_change_dependencies_restoration(app);
        let context = self.context(app);
        let controller = HeroControllerScope::maybe_of(app, context);
        self.update_hero_controller(app, controller);
        for entry in self.history_entries(app) {
            let route = app.get(entry).route;
            if route.navigator(app) == Some(self) {
                route.changed_external_state(app);
            }
        }
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &Navigator) {
        self.did_update_restoration_id(app);
        debug_assert!(self.debug_check_page_api_parameters(app));
        let observers_changed = old_widget.observers.len() != self.widget(app).observers.len()
            || old_widget
                .observers
                .iter()
                .zip(self.widget(app).observers.iter())
                .any(|(a, b)| a != b);
        let old_observers = old_widget.observers.clone();
        let old_pages = old_widget.pages.clone();
        if observers_changed {
            for observer in old_observers {
                observer.set_navigator(app, None);
            }
            for observer in self.widget(app).observers.clone() {
                debug_assert!(observer.navigator(app).is_none());
                observer.set_navigator(app, Some(self));
            }
            self.update_effective_observers(app);
        }
        let pages_changed = old_pages.len() != self.widget(app).pages.len()
            || old_pages
                .iter()
                .zip(self.widget(app).pages.iter())
                .any(|(a, b)| !Rc::ptr_eq(a, b));
        if pages_changed && !self.restore_pending(app) {
            self.update_pages(app);
        }

        for entry in self.history_entries(app) {
            let route = app.get(entry).route;
            if route.navigator(app) == Some(self) {
                route.changed_external_state(app);
            }
        }
    }

    fn deactivate(self: Handle<Self>, app: &mut App) {
        for observer in app.get(self).effective_observers.clone() {
            observer.set_navigator(app, None);
        }
        app.get_mut(self).effective_observers.clear();
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        TickerProviderStateMixin::activate(self, app);
        self.update_effective_observers(app);
        for observer in app.get(self).effective_observers.clone() {
            debug_assert!(observer.navigator(app).is_none());
            observer.set_navigator(app, Some(self));
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        debug_assert!(!app.get(self).debug_locked);
        app.get_mut(self).debug_locked = true;
        debug_assert!(app.get(self).effective_observers.is_empty());
        self.update_hero_controller(app, None);
        self.focus_node(app).as_node().dispose(app);
        self.forced_dispose_all_route_entries(app);
        let raw_id = app
            .get(self)
            .raw_next_pageless_restoration_scope_id
            .expect("init_state has run");
        RestorableProperty::dispose(raw_id, app);
        let serializable = self.serializable_history(app);
        RestorableProperty::dispose(serializable, app);
        let notifier = self.user_gesture_in_progress_notifier(app);
        app.get_mut(notifier).dispose();
        let history = self.history(app);
        history.remove_listener(
            app,
            &Listener::handle_method(self, NavigatorState::handle_history_changed),
        );
        app.get_mut(history).change_notifier.dispose();
        TickerProviderStateMixin::dispose(self, app);
        self.dispose_restoration(app);
        // A route that is being disposed can call `finalize_route`, which reads `debug_locked`.
        debug_assert!(app.get(self).debug_locked);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(!app.get(self).debug_locked);
        debug_assert!(!self.history(app).is_empty(app));

        let overlay_key: KeyRef = Rc::new(
            app.get(self)
                .overlay_key
                .clone()
                .expect("restore_state has run"),
        );
        let initial_entries = match self.overlay(app) {
            None => self.all_route_overlay_entries(app),
            Some(_) => Vec::new(),
        };
        let bucket = self.bucket(app);
        let clip_behavior = self.widget(app).clip_behavior;
        let focus_node = self.focus_node(app).as_node();
        let policy = FocusTraversalGroup::maybe_of(app, context);

        let overlay = Overlay::new()
            .key(overlay_key)
            .clip_behavior(clip_behavior)
            .initial_entries(initial_entries);
        let scope = UnmanagedRestorationScope::new(overlay).bucket(bucket);
        let focus = Focus::new(scope)
            .focus_node(focus_node)
            .autofocus(true)
            .skip_traversal(true)
            .include_semantics(false);
        let mut group = FocusTraversalGroup::new(focus);
        if let Some(policy) = policy {
            group = group.policy(policy);
        }
        // `absorbing` is mutated directly by `cancel_active_pointers` above.
        let absorb = AbsorbPointer::new().absorbing(false).child(group);
        let listener = crate::widgets::basic::Listener::new()
            .on_pointer_down(Rc::new(
                move |app, event: reveal_gestures::PointerDownEvent| {
                    self.handle_pointer_down(app, event.pointer);
                },
            ))
            .on_pointer_up(Rc::new(
                move |app, event: reveal_gestures::PointerUpEvent| {
                    self.handle_pointer_up_or_cancel(app, event.pointer);
                },
            ))
            .on_pointer_cancel(Rc::new(
                move |app, event: reveal_gestures::PointerCancelEvent| {
                    self.handle_pointer_up_or_cancel(app, event.pointer);
                },
            ))
            .child(absorb);
        let notification_listener = NotificationListener::<NavigationNotification>::new(listener)
            .on_notification(move |app, notification: &NavigationNotification| {
                // If the state of this Navigator does not change whether or not the whole
                // Navigator can handle pops, propagate the notification to the next ancestor
                // Navigator.
                if notification.can_handle_pop || !self.get_navigator_can_handle_pop(app) {
                    // Propagate to the next ancestor.
                    return false;
                }
                let context = self.context(app);
                NavigationNotification {
                    can_handle_pop: true,
                }
                .dispatch(app, Some(context));
                true
            })
            .into_widget();
        HeroControllerScope::none(notification_listener).into_widget()
    }
}

// ---------------------------------------------------------------------------------------------
// Restoration of the history

/// The kind of a serialized [`RestorationInformation`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RouteRestorationType {
    Named,
    Anonymous,
}

impl RouteRestorationType {
    fn index(self) -> i64 {
        match self {
            RouteRestorationType::Named => 0,
            RouteRestorationType::Anonymous => 1,
        }
    }

    fn from_index(index: i64) -> RouteRestorationType {
        match index {
            0 => RouteRestorationType::Named,
            1 => RouteRestorationType::Anonymous,
            other => panic!("unknown route restoration type {other}"),
        }
    }
}

/// What a pageless route needs to be re-created during state restoration.
///
/// Dart's `_RestorationInformation` and its two subclasses.
pub enum RestorationInformation {
    /// Dart's `_NamedRestorationInformation`: a route created from a route name.
    Named {
        /// The name passed to [`Navigator::on_generate_route`].
        name: String,
        /// The arguments passed to [`Navigator::on_generate_route`].
        arguments: Option<RestorationData>,
        /// The generated ID of the restoration scope this route lives in.
        restoration_scope_id: i64,
    },
    /// Dart's `_AnonymousRestorationInformation`: a route created from a builder.
    Anonymous {
        /// The builder that creates the route.
        route_builder: RestorableRouteBuilder,
        /// The arguments handed to the builder.
        arguments: Option<RestorationData>,
        /// The generated ID of the restoration scope this route lives in.
        restoration_scope_id: i64,
    },
}

impl RestorationInformation {
    fn kind(&self) -> RouteRestorationType {
        match self {
            RestorationInformation::Named { .. } => RouteRestorationType::Named,
            RestorationInformation::Anonymous { .. } => RouteRestorationType::Anonymous,
        }
    }

    /// The generated ID of the restoration scope the route lives in.
    pub fn restoration_scope_id(&self) -> i64 {
        match self {
            RestorationInformation::Named {
                restoration_scope_id,
                ..
            }
            | RestorationInformation::Anonymous {
                restoration_scope_id,
                ..
            } => *restoration_scope_id,
        }
    }

    /// Whether this information can be written into the restoration data.
    ///
    /// An anonymous route cannot: there is no handle for a Rust function that survives a
    /// restart, so an anonymous route behaves as it does on the web.
    pub fn is_restorable(&self) -> bool {
        match self {
            RestorationInformation::Named { .. } => true,
            RestorationInformation::Anonymous { .. } => false,
        }
    }

    fn serializable_data(&self) -> RestorationData {
        debug_assert!(self.is_restorable());
        let mut values = vec![RestorationData::Int(self.kind().index())];
        match self {
            RestorationInformation::Named {
                name,
                arguments,
                restoration_scope_id,
            } => {
                values.push(RestorationData::Int(*restoration_scope_id));
                values.push(RestorationData::String(name.clone()));
                if let Some(arguments) = arguments {
                    values.push(arguments.clone());
                }
            }
            RestorationInformation::Anonymous { .. } => {
                unreachable!("an anonymous route is not restorable")
            }
        }
        RestorationData::List(values)
    }

    fn from_serializable_data(data: &RestorationData) -> Rc<RestorationInformation> {
        let values = data.as_list().expect("a serialized route is a list");
        debug_assert!(!values.is_empty());
        let kind =
            RouteRestorationType::from_index(values[0].as_int().expect("the type index is an int"));
        match kind {
            RouteRestorationType::Named => {
                debug_assert!(values.len() > 2);
                Rc::new(RestorationInformation::Named {
                    restoration_scope_id: values[1].as_int().expect("a scope id"),
                    name: values[2].as_str().expect("a route name").to_string(),
                    arguments: values.get(3).cloned(),
                })
            }
            RouteRestorationType::Anonymous => {
                unreachable!("an anonymous route is never written into the restoration data")
            }
        }
    }

    fn create_route(&self, app: &mut App, navigator: Handle<NavigatorState>) -> AnyRoute {
        match self {
            RestorationInformation::Named {
                name, arguments, ..
            } => {
                let arguments = arguments.clone().map(|data| Rc::new(data) as Rc<dyn Any>);
                navigator
                    .route_named(app, name, arguments, false)
                    .expect("a route for the restored name")
            }
            RestorationInformation::Anonymous {
                route_builder,
                arguments,
                ..
            } => {
                let context = navigator.context(app);
                let arguments = arguments.clone().map(|data| Rc::new(data) as Rc<dyn Any>);
                route_builder(app, context, arguments)
            }
        }
    }

    fn to_route_entry(
        self: &Rc<Self>,
        app: &mut App,
        navigator: Handle<NavigatorState>,
        initial_state: RouteLifecycle,
    ) -> Handle<RouteEntry> {
        let route = self.create_route(app, navigator);
        RouteEntry::new(app, route, initial_state, false, Some(Rc::clone(self)))
    }
}

impl Debug for RestorationInformation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RestorationInformation::Named { name, .. } => f
                .debug_struct("RestorationInformation::Named")
                .field("name", name)
                .finish_non_exhaustive(),
            RestorationInformation::Anonymous { .. } => {
                f.write_str("RestorationInformation::Anonymous")
            }
        }
    }
}

/// Dart's `_HistoryProperty`: the serialized pageless routes of a [`NavigatorState`], keyed by
/// the restoration ID of the page they belong to.
pub struct HistoryProperty {
    change_notifier: reveal_foundation::ChangeNotifierData,
    property: RestorablePropertyData,
    page_to_pageless_routes: Option<HashMap<Option<String>, Vec<RestorationData>>>,
}

impl HistoryProperty {
    fn new(app: &mut App) -> Handle<HistoryProperty> {
        app.create(HistoryProperty {
            change_notifier: reveal_foundation::ChangeNotifierData::new(),
            property: RestorablePropertyData::new(),
            page_to_pageless_routes: None,
        })
    }

    /// Updates the serialized pageless routes from the navigator's live history.
    fn update(self: Handle<Self>, app: &mut App, history: Handle<History>) {
        debug_assert!(RestorableProperty::is_registered(self, app));
        let was_uninitialized = app.get(self).page_to_pageless_routes.is_none();
        let mut needs_serialization = was_uninitialized;
        if was_uninitialized {
            app.get_mut(self).page_to_pageless_routes = Some(HashMap::new());
        }
        let old_map = app
            .get(self)
            .page_to_pageless_routes
            .clone()
            .expect("filled above");
        let mut current_page: Option<Handle<RouteEntry>> = None;
        let mut new_routes_for_current_page: Vec<RestorationData> = Vec::new();
        let mut old_routes_for_current_page: Vec<RestorationData> =
            old_map.get(&None).cloned().unwrap_or_default();
        let mut restoration_enabled = true;

        let mut new_map: HashMap<Option<String>, Vec<RestorationData>> = HashMap::new();
        let mut removed_pages: HashSet<Option<String>> = old_map.keys().cloned().collect();

        for entry in history.entries(app) {
            if !entry.is_present_for_restoration(app) {
                entry.set_restoration_enabled(app, false);
                continue;
            }

            if app.get(entry).page_based {
                needs_serialization = needs_serialization
                    || new_routes_for_current_page.len() != old_routes_for_current_page.len();
                HistoryProperty::finalize_entry(
                    app,
                    std::mem::take(&mut new_routes_for_current_page),
                    current_page,
                    &mut new_map,
                    &mut removed_pages,
                );
                current_page = Some(entry);
                restoration_enabled = entry.restoration_id(app).is_some();
                entry.set_restoration_enabled(app, restoration_enabled);
                if restoration_enabled {
                    let id = entry.restoration_id(app);
                    debug_assert!(id.is_some());
                    new_routes_for_current_page = Vec::new();
                    old_routes_for_current_page = old_map.get(&id).cloned().unwrap_or_default();
                } else {
                    new_routes_for_current_page = Vec::new();
                    old_routes_for_current_page = Vec::new();
                }
                continue;
            }

            restoration_enabled = restoration_enabled
                && app
                    .get(entry)
                    .restoration_information
                    .as_ref()
                    .is_some_and(|information| information.is_restorable());
            entry.set_restoration_enabled(app, restoration_enabled);
            if restoration_enabled {
                debug_assert!(entry.restoration_id(app).is_some());
                debug_assert!(
                    current_page.is_none()
                        || current_page.expect("checked").restoration_id(app).is_some()
                );
                let information = app
                    .get(entry)
                    .restoration_information
                    .clone()
                    .expect("a restorable pageless entry has restoration information");
                let serialized_data = information.serializable_data();
                needs_serialization = needs_serialization
                    || old_routes_for_current_page.len() <= new_routes_for_current_page.len()
                    || old_routes_for_current_page[new_routes_for_current_page.len()]
                        != serialized_data;
                new_routes_for_current_page.push(serialized_data);
            }
        }
        needs_serialization = needs_serialization
            || new_routes_for_current_page.len() != old_routes_for_current_page.len();
        HistoryProperty::finalize_entry(
            app,
            new_routes_for_current_page,
            current_page,
            &mut new_map,
            &mut removed_pages,
        );

        needs_serialization = needs_serialization || !removed_pages.is_empty();

        if needs_serialization {
            app.get_mut(self).page_to_pageless_routes = Some(new_map);
            self.notify_listeners(app);
        }
    }

    fn finalize_entry(
        app: &App,
        routes: Vec<RestorationData>,
        page: Option<Handle<RouteEntry>>,
        page_to_routes: &mut HashMap<Option<String>, Vec<RestorationData>>,
        pages_to_remove: &mut HashSet<Option<String>>,
    ) {
        debug_assert!(page.is_none_or(|page| app.get(page).page_based));
        let restoration_id = page.and_then(|page| page.restoration_id(app));
        debug_assert!(!page_to_routes.contains_key(&restoration_id));
        if !routes.is_empty() {
            debug_assert!(page.is_none() || restoration_id.is_some());
            page_to_routes.insert(restoration_id.clone(), routes);
            pages_to_remove.remove(&restoration_id);
        }
    }

    /// Forgets the serialized routes.
    fn clear(self: Handle<Self>, app: &mut App) {
        debug_assert!(RestorableProperty::is_registered(self, app));
        if app.get(self).page_to_pageless_routes.is_none() {
            return;
        }
        app.get_mut(self).page_to_pageless_routes = None;
        self.notify_listeners(app);
    }

    /// Whether there is restoration data to restore from.
    fn has_data(self: Handle<Self>, app: &App) -> bool {
        app.get(self).page_to_pageless_routes.is_some()
    }

    /// The pageless [`RouteEntry`]s that belong to the given page.
    fn restore_entries_for_page(
        self: Handle<Self>,
        app: &mut App,
        page: Option<Handle<RouteEntry>>,
        navigator: Handle<NavigatorState>,
    ) -> Vec<Handle<RouteEntry>> {
        debug_assert!(RestorableProperty::is_registered(self, app));
        debug_assert!(page.is_none_or(|page| app.get(page).page_based));
        let mut result = Vec::new();
        if app.get(self).page_to_pageless_routes.is_none()
            || page.is_some_and(|page| page.restoration_id(app).is_none())
        {
            return result;
        }
        let key = page.and_then(|page| page.restoration_id(app));
        let Some(serialized_data) = app
            .get(self)
            .page_to_pageless_routes
            .as_ref()
            .and_then(|map| map.get(&key))
            .cloned()
        else {
            return result;
        };
        for data in serialized_data {
            let information = RestorationInformation::from_serializable_data(&data);
            result.push(information.to_route_entry(app, navigator, RouteLifecycle::Add));
        }
        result
    }
}

impl reveal_foundation::ChangeNotifier for HistoryProperty {
    fn change_notifier_data(&self) -> &reveal_foundation::ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut reveal_foundation::ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl RestorableProperty for HistoryProperty {
    type Value = Option<HashMap<Option<String>, Vec<RestorationData>>>;

    crate::restorable_property_accessors!();

    fn create_default_value(self: Handle<Self>, _app: &mut App) -> Self::Value {
        None
    }

    fn from_primitives(self: Handle<Self>, _app: &mut App, data: &RestorationData) -> Self::Value {
        let map = data.as_map().expect("the serialized history is a map");
        let mut result = HashMap::new();
        for (key, value) in map {
            let key = match key {
                RestorationData::Null => None,
                RestorationData::String(name) => Some(name.clone()),
                other => panic!("unexpected history key {other:?}"),
            };
            let value = value
                .as_list()
                .expect("the serialized routes of a page are a list")
                .to_vec();
            result.insert(key, value);
        }
        Some(result)
    }

    fn init_with_value(self: Handle<Self>, app: &mut App, value: Self::Value) {
        app.get_mut(self).page_to_pageless_routes = value;
    }

    fn to_primitives(self: Handle<Self>, app: &App) -> RestorationData {
        match &app.get(self).page_to_pageless_routes {
            None => RestorationData::Null,
            Some(map) => {
                let mut result = RestorationMap::new();
                for (key, value) in map {
                    let key = match key {
                        None => RestorationData::Null,
                        Some(name) => RestorationData::String(name.clone()),
                    };
                    result.insert(key, RestorationData::List(value.clone()));
                }
                RestorationData::Map(result)
            }
        }
    }

    fn enabled(self: Handle<Self>, app: &App) -> bool {
        self.has_data(app)
    }
}

/// Signature for a callback that finds the [`NavigatorState`] a [`RestorableRouteFuture`]
/// operates on.
pub type NavigatorFinderCallback = Rc<dyn Fn(&mut App, BuildContext) -> Handle<NavigatorState>>;

/// Signature for the callback that presents the route a [`RestorableRouteFuture`] tracks.
///
/// The callback must use one of the `restorable*` methods and return the restoration ID it
/// produced.
pub type RoutePresentationCallback =
    Rc<dyn Fn(&mut App, Handle<NavigatorState>, Option<RestorationData>) -> String>;

/// Signature for the callback that is called when the route a [`RestorableRouteFuture`] tracks
/// completes.
pub type RouteCompletionCallback = Rc<dyn Fn(&mut App, RouteResult)>;

/// Gives access to a [`Route`] object and its return value that was added to a navigator via one
/// of its `restorable*` methods.
///
/// When a router is pushed with one of those methods, they return a restoration ID. That ID can
/// be used to obtain a [`RestorableRouteFuture`], which stays valid across state restoration.
pub struct RestorableRouteFuture {
    change_notifier: reveal_foundation::ChangeNotifierData,
    property: RestorablePropertyData,
    /// A callback that given the [`BuildContext`] of the [`State`] object to which this property
    /// is registered returns the [`NavigatorState`] of the navigator the route should be added
    /// to.
    pub navigator_finder: NavigatorFinderCallback,
    /// A callback that adds a route to the navigator and returns its restoration ID.
    pub on_present: RoutePresentationCallback,
    /// A callback that is invoked when the route added by
    /// [`on_present`](Self::on_present) completes.
    pub on_complete: Option<RouteCompletionCallback>,
    route: Option<AnyRoute>,
    disposed: bool,
}

impl RestorableRouteFuture {
    /// Creates a [`RestorableRouteFuture`]; Dart's optional arguments are the setters.
    pub fn new(
        app: &mut App,
        on_present: RoutePresentationCallback,
    ) -> Handle<RestorableRouteFuture> {
        app.create(RestorableRouteFuture {
            change_notifier: reveal_foundation::ChangeNotifierData::new(),
            property: RestorablePropertyData::new(),
            navigator_finder: Rc::new(RestorableRouteFuture::default_navigator_finder),
            on_present,
            on_complete: None,
            route: None,
            disposed: false,
        })
    }

    /// Dart `RestorableRouteFuture(navigatorFinder:)`.
    pub fn set_navigator_finder(
        self: Handle<Self>,
        app: &mut App,
        navigator_finder: NavigatorFinderCallback,
    ) {
        app.get_mut(self).navigator_finder = navigator_finder;
    }

    /// Dart `RestorableRouteFuture(onComplete:)`.
    pub fn set_on_complete(
        self: Handle<Self>,
        app: &mut App,
        on_complete: RouteCompletionCallback,
    ) {
        app.get_mut(self).on_complete = Some(on_complete);
    }

    fn default_navigator_finder(app: &mut App, context: BuildContext) -> Handle<NavigatorState> {
        Navigator::of(app, context, false)
    }

    /// Shows the route created by [`on_present`](Self::on_present).
    pub fn present(self: Handle<Self>, app: &mut App, arguments: Option<RestorationData>) {
        debug_assert!(!self.is_present(app));
        debug_assert!(RestorableProperty::is_registered(self, app));
        let navigator = self.navigator(app);
        let on_present = Rc::clone(&app.get(self).on_present);
        let route_id = on_present(app, navigator, arguments);
        self.hook_onto_route_future(app, &route_id);
        self.notify_listeners(app);
    }

    /// Whether the [`route`](Self::route) is currently shown.
    pub fn is_present(self: Handle<Self>, app: &App) -> bool {
        app.get(self).route.is_some()
    }

    /// The route the [`on_present`](Self::on_present) callback added to the navigator.
    pub fn route(self: Handle<Self>, app: &App) -> Option<AnyRoute> {
        app.get(self).route
    }

    fn navigator(self: Handle<Self>, app: &mut App) -> Handle<NavigatorState> {
        let finder = Rc::clone(&app.get(self).navigator_finder);
        let state = RestorableProperty::state(self, app);
        let context = state.context(app);
        finder(app, context)
    }

    fn hook_onto_route_future(self: Handle<Self>, app: &mut App, id: &str) {
        let navigator = self.navigator(app);
        let route = navigator
            .get_route_by_id(app, id)
            .expect("the restorable method's route is in the navigator");
        app.get_mut(self).route = Some(route);
        let notifier = route.restoration_scope_id(app);
        notifier.add_listener(
            app,
            Listener::handle_method(self, RestorableRouteFuture::notify),
        );
        route.when_popped(
            app,
            Rc::new(move |app, result| {
                if app.get(self).disposed {
                    return;
                }
                if let Some(route) = app.get(self).route {
                    let notifier = route.restoration_scope_id(app);
                    notifier.remove_listener(
                        app,
                        &Listener::handle_method(self, RestorableRouteFuture::notify),
                    );
                }
                app.get_mut(self).route = None;
                self.notify_listeners(app);
                if let Some(on_complete) = app.get(self).on_complete.clone() {
                    on_complete(app, result);
                }
            }),
        );
    }

    fn notify(self: Handle<Self>, app: &mut App) {
        self.notify_listeners(app);
    }
}

impl reveal_foundation::ChangeNotifier for RestorableRouteFuture {
    fn change_notifier_data(&self) -> &reveal_foundation::ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut reveal_foundation::ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl RestorableProperty for RestorableRouteFuture {
    type Value = Option<String>;

    crate::restorable_property_accessors!();

    fn create_default_value(self: Handle<Self>, _app: &mut App) -> Option<String> {
        None
    }

    fn from_primitives(
        self: Handle<Self>,
        _app: &mut App,
        data: &RestorationData,
    ) -> Option<String> {
        Some(
            data.as_str()
                .expect("a restoration ID is a string")
                .to_string(),
        )
    }

    fn init_with_value(self: Handle<Self>, app: &mut App, value: Option<String>) {
        if let Some(value) = value {
            self.hook_onto_route_future(app, &value);
        }
    }

    fn to_primitives(self: Handle<Self>, app: &App) -> RestorationData {
        let route = app.get(self).route.expect("a presented route");
        debug_assert!(RestorableProperty::enabled(self, app));
        let notifier = route.restoration_scope_id(app);
        match app.get(notifier).value() {
            None => RestorationData::Null,
            Some(id) => RestorationData::String(id.clone()),
        }
    }

    fn enabled(self: Handle<Self>, app: &App) -> bool {
        match app.get(self).route {
            None => false,
            Some(route) => {
                let notifier = route.restoration_scope_id(app);
                app.get(notifier).value().is_some()
            }
        }
    }

    fn did_dispose(self: Handle<Self>, app: &mut App) {
        if let Some(route) = app.get(self).route {
            let notifier = route.restoration_scope_id(app);
            notifier.remove_listener(
                app,
                &Listener::handle_method(self, RestorableRouteFuture::notify),
            );
        }
        app.get_mut(self).disposed = true;
    }
}

// ---------------------------------------------------------------------------------------------
// NavigationNotification

/// A `Notification` that gets dispatched when the [`Navigator`] can or cannot handle a pop.
///
/// This is used by the framework to determine whether the app should handle back navigation
/// itself or hand it to the platform.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NavigationNotification {
    /// Whether the [`Navigator`] in the subtree can handle a pop.
    pub can_handle_pop: bool,
}

impl NavigationNotification {
    /// Creates a notification that reports whether a pop can be handled.
    pub fn new(can_handle_pop: bool) -> NavigationNotification {
        NavigationNotification { can_handle_pop }
    }
}

impl Notification for NavigationNotification {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::time::Duration;

    use reveal_embedder::{
        Picture, Platform, PlatformRef, TargetPlatform, TextDirection, View as EmbedderView,
        ViewConstraints, ViewId, ViewMetrics, ViewRef,
    };
    use reveal_scheduler::SchedulerBinding;
    use reveal_services::{RestorationMap, RestorationUpdate};

    use super::*;
    use crate::binding::run_app;
    use reveal_foundation::ValueKey;

    use crate::framework::IntoWidget;
    use crate::widgets::basic::{Align, Directionality, SizedBox};
    use crate::widgets::pages::PageRouteBuilder;
    use crate::widgets::restoration::RootRestorationScope;
    use crate::widgets::routes::RoutePageBuilder;

    const VIEW_WIDTH: f64 = 300.0;
    const VIEW_HEIGHT: f64 = 200.0;

    struct TestView;

    impl EmbedderView for TestView {
        fn id(&self) -> ViewId {
            ViewId(0)
        }

        fn metrics(&self) -> ViewMetrics {
            ViewMetrics {
                physical_size: [VIEW_WIDTH, VIEW_HEIGHT],
                physical_constraints: ViewConstraints::tight(VIEW_WIDTH, VIEW_HEIGHT),
                device_pixel_ratio: 1.0,
                ..ViewMetrics::default()
            }
        }

        fn present(&self, _picture: &Picture) {}
    }

    struct TestPlatform {
        view: ViewRef,
        stored: RefCell<Option<RestorationUpdate>>,
        puts: RefCell<Vec<RestorationMap>>,
    }

    impl Platform for TestPlatform {
        fn target_platform(&self) -> TargetPlatform {
            TargetPlatform::MacOS
        }

        fn request_frame(&self) {}

        fn now(&self) -> std::time::Instant {
            std::time::Instant::now()
        }

        fn wake_at(&self, _deadline: std::time::Instant) {}

        fn views(&self) -> Vec<ViewRef> {
            vec![Rc::clone(&self.view)]
        }

        fn view(&self, id: ViewId) -> Option<ViewRef> {
            (self.view.id() == id).then(|| Rc::clone(&self.view))
        }

        fn implicit_view(&self) -> Option<ViewRef> {
            Some(Rc::clone(&self.view))
        }

        fn restoration_get(&self) -> Option<RestorationUpdate> {
            self.stored.borrow().clone()
        }

        fn restoration_put(&self, data: RestorationMap) {
            self.puts.borrow_mut().push(data);
        }
    }

    fn app_with_view(restoration_data: Option<RestorationMap>) -> (App, Rc<TestPlatform>) {
        let platform = Rc::new(TestPlatform {
            view: Rc::new(TestView),
            stored: RefCell::new(Some(RestorationUpdate {
                enabled: true,
                data: restoration_data,
            })),
            puts: RefCell::new(Vec::new()),
        });
        let erased: PlatformRef = Rc::clone(&platform) as PlatformRef;
        (App::with_platform(erased), platform)
    }

    fn pump_frame(app: &mut App, at: Duration) {
        SchedulerBinding::handle_begin_frame(app, Some(at));
        app.drain_microtasks();
        SchedulerBinding::handle_draw_frame(app);
        app.drain_microtasks();
    }

    fn mount(app: &mut App, child: WidgetRef) {
        run_app(app, child);
        app.elapse(Duration::ZERO);
        pump_frame(app, Duration::ZERO);
    }

    /// Rebuilds the root with a new widget, the way `run_app` first attached it.
    fn attach(binding: Handle<crate::binding::WidgetsBinding>, app: &mut App, child: WidgetRef) {
        let wrapped = binding.wrap_with_default_view(app, child);
        binding.attach_root_widget(app, wrapped);
    }

    /// Runs frames until every route transition has settled.
    fn settle(app: &mut App, from: Duration) -> Duration {
        let mut at = from;
        for _ in 0..40 {
            at += Duration::from_millis(20);
            pump_frame(app, at);
        }
        at
    }

    fn corner_page() -> RoutePageBuilder {
        Rc::new(|_app, _context, _animation, _secondary| {
            Align::new()
                .alignment(reveal_painting::AlignmentGeometry::TOP_LEFT)
                .child(SizedBox::new().width(10.0).height(10.0))
                .into_widget()
        })
    }

    /// A page whose route is a plain [`PageRouteBuilder`]; keyed by its name.
    #[derive(Debug)]
    struct TestPage {
        name: String,
        key: KeyRef,
        restoration_id: Option<String>,
    }

    impl TestPage {
        fn page(name: &str) -> PageRef {
            Rc::new(TestPage {
                name: name.to_string(),
                key: Rc::new(ValueKey::new(name.to_string())),
                restoration_id: None,
            })
        }

        fn restorable_page(name: &str) -> PageRef {
            Rc::new(TestPage {
                name: name.to_string(),
                key: Rc::new(ValueKey::new(name.to_string())),
                restoration_id: Some(name.to_string()),
            })
        }
    }

    impl Page for TestPage {
        fn as_any(&self) -> &dyn Any {
            self
        }

        fn key(&self) -> Option<&KeyRef> {
            Some(&self.key)
        }

        fn name(&self) -> Option<&str> {
            Some(&self.name)
        }

        fn restoration_id(&self) -> Option<&str> {
            self.restoration_id.as_deref()
        }

        fn create_route(&self, app: &mut App, _context: BuildContext, this: &PageRef) -> AnyRoute {
            PageRouteBuilder::new(app, corner_page())
                .settings(app, RouteSettingsRef::Page(Rc::clone(this)))
                .as_route()
        }
    }

    /// The names of the pages the navigator's history currently holds, bottom-most first.
    fn history_names(state: Handle<NavigatorState>, app: &App) -> Vec<String> {
        state
            .history_entries(app)
            .into_iter()
            .filter(|entry| entry.is_present(app))
            .filter_map(|entry| {
                app.get(entry)
                    .route
                    .settings(app)
                    .name()
                    .map(str::to_string)
            })
            .collect()
    }

    fn pages_navigator(key: &GlobalKey, pages: Vec<PageRef>) -> WidgetRef {
        Directionality::new(
            TextDirection::Ltr,
            Navigator::new()
                .key(Rc::new(key.clone()))
                .pages(pages)
                .on_did_remove_page(|_app, _page| {}),
        )
        .into_widget()
    }

    fn state(key: &GlobalKey, app: &mut App) -> Handle<NavigatorState> {
        key.current_state::<NavigatorState>(app)
            .expect("a mounted navigator")
    }

    // ---- the tests ----

    #[test]
    fn the_pages_api_inserts_removes_and_reorders_routes_through_the_transition_delegate() {
        let (mut app, _platform) = app_with_view(None);
        let key = GlobalKey::new();
        let binding = crate::binding::WidgetsBinding::instance(&mut app);
        mount(
            &mut app,
            pages_navigator(&key, vec![TestPage::page("a"), TestPage::page("b")]),
        );
        let navigator_state = state(&key, &mut app);
        let at = settle(&mut app, Duration::ZERO);
        assert_eq!(history_names(navigator_state, &app), vec!["a", "b"]);

        // Insert in the middle.
        attach(
            binding,
            &mut app,
            pages_navigator(
                &key,
                vec![
                    TestPage::page("a"),
                    TestPage::page("c"),
                    TestPage::page("b"),
                ],
            ),
        );
        let at = settle(&mut app, at);
        assert_eq!(history_names(navigator_state, &app), vec!["a", "c", "b"]);

        // Reorder.
        attach(
            binding,
            &mut app,
            pages_navigator(
                &key,
                vec![
                    TestPage::page("a"),
                    TestPage::page("b"),
                    TestPage::page("c"),
                ],
            ),
        );
        let at = settle(&mut app, at);
        assert_eq!(history_names(navigator_state, &app), vec!["a", "b", "c"]);

        // Remove.
        attach(
            binding,
            &mut app,
            pages_navigator(&key, vec![TestPage::page("a")]),
        );
        settle(&mut app, at);
        assert_eq!(history_names(navigator_state, &app), vec!["a"]);
    }

    #[test]
    fn a_navigator_observer_sees_every_push_pop_replace_and_remove() {
        #[derive(Default)]
        struct Recorder {
            navigator_observer: NavigatorObserverData,
            log: RefCell<Vec<String>>,
        }

        fn name(app: &App, route: Option<AnyRoute>) -> String {
            match route {
                None => "none".to_string(),
                Some(route) => route.settings(app).name().unwrap_or("?").to_string(),
            }
        }

        impl NavigatorObserver for Recorder {
            crate::navigator_observer_accessors!();

            fn did_push(
                self: Handle<Self>,
                app: &mut App,
                route: AnyRoute,
                previous_route: Option<AnyRoute>,
            ) {
                let entry = format!(
                    "push {} over {}",
                    name(app, Some(route)),
                    name(app, previous_route)
                );
                app.get(self).log.borrow_mut().push(entry);
            }

            fn did_pop(
                self: Handle<Self>,
                app: &mut App,
                route: AnyRoute,
                previous_route: Option<AnyRoute>,
            ) {
                let entry = format!(
                    "pop {} to {}",
                    name(app, Some(route)),
                    name(app, previous_route)
                );
                app.get(self).log.borrow_mut().push(entry);
            }

            fn did_remove(
                self: Handle<Self>,
                app: &mut App,
                route: AnyRoute,
                previous_route: Option<AnyRoute>,
            ) {
                let entry = format!(
                    "remove {} above {}",
                    name(app, Some(route)),
                    name(app, previous_route)
                );
                app.get(self).log.borrow_mut().push(entry);
            }

            fn did_replace(
                self: Handle<Self>,
                app: &mut App,
                new_route: Option<AnyRoute>,
                old_route: Option<AnyRoute>,
            ) {
                let entry = format!(
                    "replace {} with {}",
                    name(app, old_route),
                    name(app, new_route)
                );
                app.get(self).log.borrow_mut().push(entry);
            }
        }

        let (mut app, _platform) = app_with_view(None);
        let key = GlobalKey::new();
        let observer = app.create(Recorder::default());
        mount(
            &mut app,
            Directionality::new(
                TextDirection::Ltr,
                Navigator::new()
                    .key(Rc::new(key.clone()))
                    .observers(vec![observer.as_observer()])
                    .on_generate_route(|app, settings| {
                        let name = settings.name.clone().unwrap_or_default();
                        Some(
                            PageRouteBuilder::new(app, corner_page())
                                .settings(app, RouteSettings::new().name(name).into())
                                .as_route(),
                        )
                    }),
            )
            .into_widget(),
        );
        let navigator_state = state(&key, &mut app);
        let at = settle(&mut app, Duration::ZERO);
        assert_eq!(
            app.get(observer).log.borrow().as_slice(),
            ["push / over none"]
        );

        navigator_state.push_named(&mut app, "second", None);
        let at = settle(&mut app, at);
        navigator_state.push_replacement_named(&mut app, "third", None, None);
        let at = settle(&mut app, at);
        navigator_state.pop(&mut app, None);
        settle(&mut app, at);

        assert_eq!(
            app.get(observer).log.borrow().as_slice(),
            [
                "push / over none",
                "push second over /",
                "replace second with third",
                "pop third to /",
            ]
        );
    }

    #[test]
    fn the_history_is_restored_from_the_restoration_data_of_a_previous_run() {
        // Run once, pushing a restorable named route on top of a restorable page.
        let (mut app, platform) = app_with_view(None);
        let key = GlobalKey::new();
        mount(
            &mut app,
            Directionality::new(
                TextDirection::Ltr,
                RootRestorationScope::new(
                    Some("root".to_string()),
                    Navigator::new()
                        .key(Rc::new(key.clone()))
                        .restoration_scope_id("nav")
                        .pages(vec![TestPage::restorable_page("a")])
                        .on_did_remove_page(|_app, _page| {})
                        .on_generate_route(|app, settings| {
                            let name = settings.name.clone().unwrap_or_default();
                            Some(
                                PageRouteBuilder::new(app, corner_page())
                                    .settings(app, RouteSettings::new().name(name).into())
                                    .as_route(),
                            )
                        }),
                ),
            )
            .into_widget(),
        );
        let navigator_state = state(&key, &mut app);
        let at = settle(&mut app, Duration::ZERO);
        navigator_state.restorable_push_named(&mut app, "second", None);
        settle(&mut app, at);
        assert_eq!(history_names(navigator_state, &app), vec!["a", "second"]);
        app.drain_microtasks();

        let saved = platform
            .puts
            .borrow()
            .last()
            .cloned()
            .expect("the manager wrote the restoration data");

        // Run again from the saved data: the pageless route comes back.
        let (mut app, _platform) = app_with_view(Some(saved));
        let key = GlobalKey::new();
        mount(
            &mut app,
            Directionality::new(
                TextDirection::Ltr,
                RootRestorationScope::new(
                    Some("root".to_string()),
                    Navigator::new()
                        .key(Rc::new(key.clone()))
                        .restoration_scope_id("nav")
                        .pages(vec![TestPage::restorable_page("a")])
                        .on_did_remove_page(|_app, _page| {})
                        .on_generate_route(|app, settings| {
                            let name = settings.name.clone().unwrap_or_default();
                            Some(
                                PageRouteBuilder::new(app, corner_page())
                                    .settings(app, RouteSettings::new().name(name).into())
                                    .as_route(),
                            )
                        }),
                ),
            )
            .into_widget(),
        );
        let restored = state(&key, &mut app);
        settle(&mut app, Duration::ZERO);
        assert_eq!(history_names(restored, &app), vec!["a", "second"]);
    }

    #[test]
    fn a_route_reports_its_position_in_the_history() {
        let (mut app, _platform) = app_with_view(None);
        let key = GlobalKey::new();
        let pushed: Rc<Cell<Option<AnyRoute>>> = Rc::new(Cell::new(None));
        mount(
            &mut app,
            Directionality::new(
                TextDirection::Ltr,
                Navigator::new()
                    .key(Rc::new(key.clone()))
                    .on_generate_route({
                        let pushed = Rc::clone(&pushed);
                        move |app, _settings| {
                            let route = PageRouteBuilder::new(app, corner_page()).as_route();
                            pushed.set(Some(route));
                            Some(route)
                        }
                    }),
            )
            .into_widget(),
        );
        let navigator_state = state(&key, &mut app);
        let at = settle(&mut app, Duration::ZERO);
        let first = pushed.get().expect("the initial route");
        assert!(first.is_first(&app));
        assert!(first.is_current(&app));
        assert!(first.is_active(&app));
        assert!(!first.has_active_route_below(&app));

        let second = PageRouteBuilder::new(&mut app, corner_page()).as_route();
        navigator_state.push(&mut app, second);
        settle(&mut app, at);
        assert!(first.is_first(&app));
        assert!(!first.is_current(&app));
        assert!(second.is_current(&app));
        assert!(!second.is_first(&app));
        assert!(second.has_active_route_below(&app));
    }
}
