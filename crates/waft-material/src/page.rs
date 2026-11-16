//! Port of material_ui's `page.dart`.
//!
//! `debugLabel` waits with diagnostics.

use std::any::{Any, TypeId};
use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use waft_animation::AnimationRef;
use waft_cupertino::CupertinoPageTransitionsBuilder;
use waft_embedder::{Color, TargetPlatform};
use waft_foundation::{App, Handle, Ref};
use waft_scheduler::TickerFuture;
use waft_widgets::{
    AnyRoute, AnyTransitionRoute, BuildContext, DelegatedTransitionBuilder, IntoWidget, KeyRef,
    LocalHistoryRoute, LocalHistoryRouteData, ModalRoute, ModalRouteData, OverlayRoute,
    OverlayRouteData, Page, PageRef, PageRoute, PageRouteData, PageTransitionsBuilderRef,
    PopInvokedWithResultCallback, PredictiveBackRoute, Route, RouteData, RouteRef, RouteResult,
    RouteSettingsRef, Semantics, State, TransitionRoute, TransitionRouteData,
    TraversalEdgeBehavior, WidgetBuilder, WidgetRef,
};

use crate::page_transitions_theme::ZoomPageTransitionsBuilder;
use crate::theme::Theme;

/// A modal route that replaces the entire screen with a platform-adaptive transition.
///
/// For Android, the entrance transition for the page zooms in and fades in while the exiting
/// page zooms out and fades out. The exit transition is similar, but in reverse.
///
/// For iOS, the page slides in from the right and exits in reverse. The page also shifts to the
/// left in parallax when another page enters to cover it. (These directions are flipped in
/// environments with a right-to-left reading direction.)
///
/// By default, when a modal route is replaced by another, the previous route remains in memory.
/// To free all the resources when this is not necessary, set
/// [`maintain_state`](Self::maintain_state) to false.
///
/// The `fullscreen_dialog` property specifies whether the incoming route is a fullscreen modal
/// dialog. On iOS, those routes animate from the bottom to the top rather than horizontally.
///
/// If `barrier_dismissible` is true, then pressing the escape key on the keyboard will cause the
/// current route to be popped with no value.
///
/// See also:
///
///  * [`MaterialRouteTransitionMixin`], which provides the material transition for this route.
///  * [`MaterialPage`], which is a [`Page`] of this class.
pub struct MaterialPageRoute {
    route: RouteData,
    overlay_route: OverlayRouteData,
    transition_route: TransitionRouteData,
    local_history: LocalHistoryRouteData,
    modal_route: ModalRouteData,
    page_route: PageRouteData,
    /// Builds the primary contents of the route.
    pub builder: WidgetBuilder,
    maintain_state: bool,
    include_route_semantics: bool,
}

impl MaterialPageRoute {
    /// Construct a MaterialPageRoute whose contents are defined by `builder`; Dart's optional
    /// arguments are the setters.
    ///
    /// Dart's defaults: `maintain_state` true, `fullscreen_dialog` false, `allow_snapshotting`
    /// true, `include_route_semantics` true, `barrier_dismissible` false.
    pub fn new(app: &mut App, builder: WidgetBuilder) -> Ref<MaterialPageRoute> {
        let route = RouteData::new(app, None, None);
        let transition_route = TransitionRouteData::new(app);
        let modal_route = ModalRouteData::new(app);
        let this = app.create(MaterialPageRoute {
            route,
            overlay_route: OverlayRouteData::new(),
            transition_route,
            local_history: LocalHistoryRouteData::new(),
            modal_route,
            page_route: PageRouteData::new(),
            builder,
            maintain_state: true,
            include_route_semantics: true,
        });
        debug_assert!(TransitionRoute::opaque(this.handle(), app));
        this
    }

    /// Dart `MaterialPageRoute(settings:)`.
    pub fn settings(self: Ref<Self>, app: &mut App, settings: RouteSettingsRef) -> Ref<Self> {
        self.route_data_mut(app).set_settings(settings);
        self
    }

    /// Dart `MaterialPageRoute(requestFocus:)`.
    pub fn request_focus(self: Ref<Self>, app: &mut App, request_focus: bool) -> Ref<Self> {
        self.route_data_mut(app)
            .set_request_focus(Some(request_focus));
        self
    }

    /// Dart `MaterialPageRoute(maintainState:)`.
    pub fn maintain_state(self: Ref<Self>, app: &mut App, maintain_state: bool) -> Ref<Self> {
        app.get_mut(&self).maintain_state = maintain_state;
        self
    }

    /// Dart `MaterialPageRoute(fullscreenDialog:)`.
    pub fn fullscreen_dialog(self: Ref<Self>, app: &mut App, fullscreen_dialog: bool) -> Ref<Self> {
        self.page_route_data_mut(app).fullscreen_dialog = fullscreen_dialog;
        self
    }

    /// Dart `MaterialPageRoute(allowSnapshotting:)`.
    pub fn allow_snapshotting(
        self: Ref<Self>,
        app: &mut App,
        allow_snapshotting: bool,
    ) -> Ref<Self> {
        self.page_route_data_mut(app).allow_snapshotting = allow_snapshotting;
        self
    }

    /// Dart `MaterialPageRoute(includeRouteSemantics:)`.
    pub fn include_route_semantics(
        self: Ref<Self>,
        app: &mut App,
        include_route_semantics: bool,
    ) -> Ref<Self> {
        app.get_mut(&self).include_route_semantics = include_route_semantics;
        self
    }

    /// Dart `MaterialPageRoute(barrierDismissible:)`.
    pub fn barrier_dismissible(
        self: Ref<Self>,
        app: &mut App,
        barrier_dismissible: bool,
    ) -> Ref<Self> {
        self.page_route_data_mut(app).barrier_dismissible = barrier_dismissible;
        self
    }

    /// Dart `MaterialPageRoute(traversalEdgeBehavior:)`.
    pub fn traversal_edge_behavior(
        self: Ref<Self>,
        app: &mut App,
        traversal_edge_behavior: TraversalEdgeBehavior,
    ) -> Ref<Self> {
        self.modal_route_data_mut(app).traversal_edge_behavior = Some(traversal_edge_behavior);
        self
    }

    /// Dart `MaterialPageRoute(directionalTraversalEdgeBehavior:)`.
    pub fn directional_traversal_edge_behavior(
        self: Ref<Self>,
        app: &mut App,
        directional_traversal_edge_behavior: TraversalEdgeBehavior,
    ) -> Ref<Self> {
        self.modal_route_data_mut(app)
            .directional_traversal_edge_behavior = Some(directional_traversal_edge_behavior);
        self
    }
}

impl Route for MaterialPageRoute {
    crate::material_route_transition_mixin_route_overrides!();
}

impl OverlayRoute for MaterialPageRoute {
    waft_widgets::modal_overlay_route_overrides!();
}

impl PredictiveBackRoute for MaterialPageRoute {
    waft_widgets::page_predictive_back_route_overrides!();
}

impl TransitionRoute for MaterialPageRoute {
    crate::material_route_transition_mixin_transition_route_overrides!();

    fn opaque(self: Handle<Self>, app: &App) -> bool {
        PageRoute::opaque(self, app)
    }
}

impl LocalHistoryRoute for MaterialPageRoute {
    waft_widgets::local_history_route_accessors!();
}

impl ModalRoute for MaterialPageRoute {
    crate::material_route_transition_mixin_modal_route_overrides!();

    fn maintain_state(self: Handle<Self>, app: &App) -> bool {
        app.get(self).maintain_state
    }

    fn fullscreen_dialog(self: Handle<Self>, app: &App) -> bool {
        PageRoute::fullscreen_dialog(self, app)
    }
}

impl PageRoute for MaterialPageRoute {
    waft_widgets::page_route_accessors!();
}

impl MaterialRouteTransitionMixin for MaterialPageRoute {
    fn build_content(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let builder = Rc::clone(&app.get(self).builder);
        builder(app, context)
    }

    fn include_route_semantics(self: Handle<Self>, app: &App) -> bool {
        app.get(self).include_route_semantics
    }
}

/// A mixin that provides platform-adaptive transitions for a [`PageRoute`].
///
/// For Android, the entrance transition for the page zooms in and fades in while the exiting
/// page zooms out and fades out. The exit transition is similar, but in reverse.
///
/// For iOS, the page slides in from the right and exits in reverse. The page also shifts to the
/// left in parallax when another page enters to cover it. (These directions are flipped in
/// environments with a right-to-left reading direction.)
///
/// A leaf writes [`material_route_transition_mixin_route_overrides!`](crate::material_route_transition_mixin_route_overrides),
/// [`material_route_transition_mixin_transition_route_overrides!`](crate::material_route_transition_mixin_transition_route_overrides)
/// and [`material_route_transition_mixin_modal_route_overrides!`](crate::material_route_transition_mixin_modal_route_overrides)
/// into its `Route`, `TransitionRoute` and `ModalRoute` impls, where Dart's mixin overrides
/// those classes' members.
///
/// See also:
///
///  * [`PageTransitionsTheme`](crate::PageTransitionsTheme), which defines the default page
///    transitions used by the [`MaterialRouteTransitionMixin::build_transitions`].
///  * [`ZoomPageTransitionsBuilder`], which is the default page transition used by the
///    [`PageTransitionsTheme`](crate::PageTransitionsTheme).
///  * [`CupertinoPageTransitionsBuilder`], which is the default page transition for iOS and
///    macOS.
pub trait MaterialRouteTransitionMixin: PageRoute {
    /// Builds the primary contents of the route.
    fn build_content(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef;

    /// Whether this route introduces a route scope in the semantics tree.
    ///
    /// Defaults to true. When true, screen readers can treat pushes and pops of
    /// this route as page-level navigation. Set to false for routes that represent
    /// tab or shell content in a nested navigator. This prevents screen readers
    /// from treating the route as a new screen.
    fn include_route_semantics(self: Handle<Self>, app: &App) -> bool {
        let _ = app;
        true
    }

    /// Dart's `MaterialRouteTransitionMixin.transitionDuration`.
    fn transition_duration(self: Handle<Self>, app: &mut App) -> Duration {
        let context = navigator_context(self, app);
        get_page_transition_builder(app, context).map_or(Duration::from_micros(300), |builder| {
            builder.transition_duration()
        })
    }

    /// Dart's `MaterialRouteTransitionMixin.reverseTransitionDuration`.
    fn reverse_transition_duration(self: Handle<Self>, app: &mut App) -> Duration {
        let context = navigator_context(self, app);
        get_page_transition_builder(app, context).map_or(Duration::from_micros(300), |builder| {
            builder.reverse_transition_duration()
        })
    }

    // The transitionDuration is used to create the AnimationController which is only built
    // once, so when page transition builder is updated and transitionDuration has a new value,
    // the AnimationController cannot be updated automatically. So we manually update its
    // duration here.
    // TODO(quncCccccc): Clean up this override method when controller can be updated as the transitionDuration is changed.
    /// Dart's `MaterialRouteTransitionMixin.didPush`.
    fn did_push(self: Handle<Self>, app: &mut App) -> TickerFuture {
        if let Some(controller) = TransitionRoute::controller(self, app) {
            let duration = TransitionRoute::transition_duration(self, app);
            app.get_mut(controller).duration = Some(duration);
        }
        ModalRoute::did_push(self, app)
    }

    // The reverseTransitionDuration is used to create the AnimationController which is only
    // built once, so when page transition builder is updated and reverseTransitionDuration has
    // a new value, the AnimationController cannot be updated automatically. So we manually
    // update its reverseDuration here.
    // TODO(quncCccccc): Clean up this override method when controller can beupdated as the reverseTransitionDuration is changed.
    /// Dart's `MaterialRouteTransitionMixin.didPop`.
    fn did_pop(self: Handle<Self>, app: &mut App, result: RouteResult) -> bool {
        if let Some(controller) = TransitionRoute::controller(self, app) {
            let reverse_duration = TransitionRoute::reverse_transition_duration(self, app);
            app.get_mut(controller).reverse_duration = Some(reverse_duration);
        }
        LocalHistoryRoute::did_pop(self, app, result)
    }

    /// Dart's `MaterialRouteTransitionMixin.barrierColor`.
    fn barrier_color(self: Handle<Self>, app: &App) -> Option<Color> {
        let _ = app;
        None
    }

    /// Dart's `MaterialRouteTransitionMixin.barrierLabel`.
    fn barrier_label(self: Handle<Self>, app: &App) -> Option<String> {
        let _ = app;
        None
    }

    /// Dart's `MaterialRouteTransitionMixin.delegatedTransition`: the tear-off of the static
    /// `_delegatedTransition`, so every route that mixes this in hands out an equal builder.
    fn delegated_transition(self: Handle<Self>, app: &App) -> Option<DelegatedTransitionBuilder> {
        let _ = app;
        Some(DelegatedTransitionBuilder::function(
            material_delegated_transition,
        ))
    }

    /// Dart's `MaterialRouteTransitionMixin.canTransitionTo`.
    fn can_transition_to(self: Handle<Self>, app: &App, next_route: AnyTransitionRoute) -> bool {
        let next_route = next_route.as_route();
        // Don't perform outgoing animation if the next route is a fullscreen dialog, or there is
        // no matching transition to use.
        // Don't perform outgoing animation if the next route is a fullscreen dialog.
        let next_route_is_not_fullscreen = match next_route.as_page_route(app) {
            None => true,
            Some(next_route) => !next_route.as_modal_route().fullscreen_dialog(app),
        };

        // If the next route has a delegated transition, then this route is able to use that
        // delegated transition to smoothly sync with the next route's transition.
        let next_route_has_delegated_transition = next_route
            .as_modal_route(app)
            .is_some_and(|next_route| next_route.delegated_transition(app).is_some());

        // Otherwise if the next route has the same route transition mixin as this one, then this
        // route will already be synced with its transition.
        next_route_is_not_fullscreen
            && (next_route
                .interface::<Rc<dyn MaterialRouteTransition>>()
                .is_some()
                || next_route_has_delegated_transition)
    }

    /// Dart's `MaterialRouteTransitionMixin.canTransitionFrom`.
    fn can_transition_from(
        self: Handle<Self>,
        app: &App,
        previous_route: AnyTransitionRoute,
    ) -> bool {
        // Suppress previous route from transitioning if this is a fullscreenDialog route.
        previous_route.as_route().as_page_route(app).is_some()
            && !PageRoute::fullscreen_dialog(self, app)
    }

    /// Dart's `MaterialRouteTransitionMixin.buildPage`.
    fn build_page(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        animation: AnimationRef<f64>,
        secondary_animation: AnimationRef<f64>,
    ) -> WidgetRef {
        let _ = (animation, secondary_animation);
        let result = self.build_content(app, context);
        if !self.include_route_semantics(app) {
            return result;
        }
        Semantics::new()
            .scopes_route(true)
            .explicit_child_nodes(true)
            .child(result)
            .into_widget()
    }

    /// Dart's `MaterialRouteTransitionMixin.buildTransitions`.
    fn build_transitions(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        animation: AnimationRef<f64>,
        secondary_animation: AnimationRef<f64>,
        child: WidgetRef,
    ) -> WidgetRef {
        let theme = Theme::of(app, context).page_transitions_theme.clone();
        theme.build_transitions(
            app,
            PageRoute::as_page_route(self),
            context,
            animation,
            secondary_animation,
            child,
        )
    }

    /// Dart's `route is MaterialRouteTransitionMixin`, answered through `Route::interface`:
    /// this route as an `Rc<dyn MaterialRouteTransition>` when `id` names that type.
    fn interface(self: Handle<Self>, id: TypeId) -> Option<Box<dyn Any>> {
        if id != TypeId::of::<Rc<dyn MaterialRouteTransition>>() {
            return None;
        }
        let transition: Rc<dyn MaterialRouteTransition> = Rc::new(self);
        Some(Box::new(transition))
    }
}

/// [`MaterialRouteTransitionMixin`] on a shared reference: what Dart's
/// `route is MaterialRouteTransitionMixin` produces, read as
/// `route.interface::<Rc<dyn MaterialRouteTransition>>()` on an [`AnyRoute`]. Every route that
/// implements the mixin is one through its handle.
pub trait MaterialRouteTransition {
    /// This route as the erased [`AnyRoute`].
    fn as_route(&self) -> AnyRoute;
}

impl<T: MaterialRouteTransitionMixin> MaterialRouteTransition for Handle<T> {
    fn as_route(&self) -> AnyRoute {
        Route::as_route(*self)
    }
}

/// Dart's `navigator!.context`.
fn navigator_context<R: Route>(route: Handle<R>, app: &App) -> BuildContext {
    Route::navigator(route, app)
        .expect("an installed route has a navigator")
        .context(app)
}

/// Dart's `_getPageTransitionBuilder`.
fn get_page_transition_builder(
    app: &mut App,
    context: BuildContext,
) -> Option<PageTransitionsBuilderRef> {
    let platform = Theme::of(app, context).platform;
    let page_transitions_theme = Theme::of(app, context).page_transitions_theme.clone();
    let builder = match page_transitions_theme.builders().get(&platform) {
        Some(builder) => builder.clone(),
        None => match platform {
            TargetPlatform::IOS | TargetPlatform::MacOS => {
                PageTransitionsBuilderRef::new(CupertinoPageTransitionsBuilder::new())
            }
            TargetPlatform::Android
            | TargetPlatform::Fuchsia
            | TargetPlatform::Windows
            | TargetPlatform::Linux => {
                PageTransitionsBuilderRef::new(ZoomPageTransitionsBuilder::new())
            }
        },
    };
    Some(builder)
}

/// Dart's static `MaterialRouteTransitionMixin._delegatedTransition`.
fn material_delegated_transition(
    app: &mut App,
    context: BuildContext,
    animation: AnimationRef<f64>,
    secondary_animation: AnimationRef<f64>,
    allow_snapshotting: bool,
    child: Option<&WidgetRef>,
) -> Option<WidgetRef> {
    let theme_data = Theme::of(app, context);
    let theme = theme_data.page_transitions_theme.clone();
    let platform = theme_data.platform;
    let theme_delegated_transition = theme.delegated_transition(platform);
    match theme_delegated_transition {
        Some(theme_delegated_transition) => theme_delegated_transition.call(
            app,
            context,
            animation,
            secondary_animation,
            allow_snapshotting,
            child,
        ),
        None => None,
    }
}

/// The `Route` overrides a [`MaterialRouteTransitionMixin`] leaf inherits: `waft-widgets`'
/// `page_route_overrides!` with `did_push`, `did_pop` and `interface` pointed at the mixin.
/// Write it inside `impl Route for MyRoute { .. }`.
#[macro_export]
macro_rules! material_route_transition_mixin_route_overrides {
    () => {
        ::waft_widgets::route_accessors!();

        fn as_transition_route(
            self: ::waft_foundation::Handle<Self>,
        ) -> ::std::option::Option<::waft_widgets::AnyTransitionRoute> {
            ::std::option::Option::Some(::waft_widgets::TransitionRoute::as_transition_route(self))
        }

        fn as_modal_route(
            self: ::waft_foundation::Handle<Self>,
        ) -> ::std::option::Option<::waft_widgets::AnyModalRoute> {
            ::std::option::Option::Some(::waft_widgets::ModalRoute::as_modal_route(self))
        }

        fn as_page_route(
            self: ::waft_foundation::Handle<Self>,
        ) -> ::std::option::Option<::waft_widgets::AnyPageRoute> {
            ::std::option::Option::Some(::waft_widgets::PageRoute::as_page_route(self))
        }

        fn interface(
            self: ::waft_foundation::Handle<Self>,
            id: ::std::any::TypeId,
        ) -> ::std::option::Option<::std::boxed::Box<dyn ::std::any::Any>> {
            $crate::MaterialRouteTransitionMixin::interface(self, id)
        }

        fn overlay_entries(
            self: ::waft_foundation::Handle<Self>,
            app: &::waft_foundation::App,
        ) -> ::std::vec::Vec<::waft_foundation::Handle<::waft_widgets::OverlayEntry>> {
            ::waft_widgets::OverlayRoute::overlay_entries(self, app)
        }

        fn install(self: ::waft_foundation::Handle<Self>, app: &mut ::waft_foundation::App) {
            ::waft_widgets::ModalRoute::install(self, app)
        }

        fn did_push(
            self: ::waft_foundation::Handle<Self>,
            app: &mut ::waft_foundation::App,
        ) -> ::waft_scheduler::TickerFuture {
            $crate::MaterialRouteTransitionMixin::did_push(self, app)
        }

        fn did_add(self: ::waft_foundation::Handle<Self>, app: &mut ::waft_foundation::App) {
            ::waft_widgets::ModalRoute::did_add(self, app)
        }

        fn did_replace(
            self: ::waft_foundation::Handle<Self>,
            app: &mut ::waft_foundation::App,
            old_route: ::std::option::Option<::waft_widgets::AnyRoute>,
        ) {
            ::waft_widgets::TransitionRoute::did_replace(self, app, old_route)
        }

        fn will_pop(
            self: ::waft_foundation::Handle<Self>,
            app: &mut ::waft_foundation::App,
        ) -> ::waft_widgets::RoutePopDisposition {
            ::waft_widgets::LocalHistoryRoute::will_pop(self, app)
        }

        fn pop_disposition(
            self: ::waft_foundation::Handle<Self>,
            app: &mut ::waft_foundation::App,
        ) -> ::waft_widgets::RoutePopDisposition {
            ::waft_widgets::ModalRoute::pop_disposition(self, app)
        }

        fn on_pop_invoked_with_result(
            self: ::waft_foundation::Handle<Self>,
            app: &mut ::waft_foundation::App,
            did_pop: bool,
            result: ::waft_widgets::RouteResult,
        ) {
            ::waft_widgets::ModalRoute::on_pop_invoked_with_result(self, app, did_pop, result)
        }

        fn will_handle_pop_internally(
            self: ::waft_foundation::Handle<Self>,
            app: &::waft_foundation::App,
        ) -> bool {
            ::waft_widgets::LocalHistoryRoute::will_handle_pop_internally(self, app)
        }

        fn did_pop(
            self: ::waft_foundation::Handle<Self>,
            app: &mut ::waft_foundation::App,
            result: ::waft_widgets::RouteResult,
        ) -> bool {
            $crate::MaterialRouteTransitionMixin::did_pop(self, app, result)
        }

        fn did_pop_next(
            self: ::waft_foundation::Handle<Self>,
            app: &mut ::waft_foundation::App,
            next_route: ::waft_widgets::AnyRoute,
        ) {
            ::waft_widgets::ModalRoute::did_pop_next(self, app, next_route)
        }

        fn did_change_next(
            self: ::waft_foundation::Handle<Self>,
            app: &mut ::waft_foundation::App,
            next_route: ::std::option::Option<::waft_widgets::AnyRoute>,
        ) {
            ::waft_widgets::ModalRoute::did_change_next(self, app, next_route)
        }

        fn did_change_previous(
            self: ::waft_foundation::Handle<Self>,
            app: &mut ::waft_foundation::App,
            previous_route: ::std::option::Option<::waft_widgets::AnyRoute>,
        ) {
            ::waft_widgets::ModalRoute::did_change_previous(self, app, previous_route)
        }

        fn changed_internal_state(
            self: ::waft_foundation::Handle<Self>,
            app: &mut ::waft_foundation::App,
        ) {
            ::waft_widgets::ModalRoute::changed_internal_state(self, app)
        }

        fn changed_external_state(
            self: ::waft_foundation::Handle<Self>,
            app: &mut ::waft_foundation::App,
        ) {
            ::waft_widgets::ModalRoute::changed_external_state(self, app)
        }

        fn dispose(self: ::waft_foundation::Handle<Self>, app: &mut ::waft_foundation::App) {
            ::waft_widgets::TransitionRoute::dispose(self, app)
        }
    };
}

/// The `TransitionRoute` overrides a [`MaterialRouteTransitionMixin`] leaf inherits, plus
/// `waft-widgets`' `modal_transition_route_overrides!`. Write it inside
/// `impl TransitionRoute for MyRoute { .. }`, next to the leaf's own `opaque`.
#[macro_export]
macro_rules! material_route_transition_mixin_transition_route_overrides {
    () => {
        ::waft_widgets::modal_transition_route_overrides!();

        fn allow_snapshotting(
            self: ::waft_foundation::Handle<Self>,
            app: &::waft_foundation::App,
        ) -> bool {
            ::waft_widgets::PageRoute::allow_snapshotting(self, app)
        }

        fn transition_duration(
            self: ::waft_foundation::Handle<Self>,
            app: &mut ::waft_foundation::App,
        ) -> ::std::time::Duration {
            $crate::MaterialRouteTransitionMixin::transition_duration(self, app)
        }

        fn reverse_transition_duration(
            self: ::waft_foundation::Handle<Self>,
            app: &mut ::waft_foundation::App,
        ) -> ::std::time::Duration {
            $crate::MaterialRouteTransitionMixin::reverse_transition_duration(self, app)
        }

        fn can_transition_to(
            self: ::waft_foundation::Handle<Self>,
            app: &::waft_foundation::App,
            next_route: ::waft_widgets::AnyTransitionRoute,
        ) -> bool {
            $crate::MaterialRouteTransitionMixin::can_transition_to(self, app, next_route)
        }

        fn can_transition_from(
            self: ::waft_foundation::Handle<Self>,
            app: &::waft_foundation::App,
            previous_route: ::waft_widgets::AnyTransitionRoute,
        ) -> bool {
            $crate::MaterialRouteTransitionMixin::can_transition_from(self, app, previous_route)
        }
    };
}

/// The `ModalRoute` overrides a [`MaterialRouteTransitionMixin`] leaf inherits, plus
/// `waft-widgets`' `modal_route_accessors!`. Write it inside
/// `impl ModalRoute for MyRoute { .. }`, next to the leaf's own `maintain_state`.
#[macro_export]
macro_rules! material_route_transition_mixin_modal_route_overrides {
    () => {
        ::waft_widgets::modal_route_accessors!();

        fn barrier_dismissible(
            self: ::waft_foundation::Handle<Self>,
            app: &::waft_foundation::App,
        ) -> bool {
            ::waft_widgets::PageRoute::barrier_dismissible(self, app)
        }

        fn barrier_color(
            self: ::waft_foundation::Handle<Self>,
            app: &::waft_foundation::App,
        ) -> ::std::option::Option<::waft_painting::Color> {
            $crate::MaterialRouteTransitionMixin::barrier_color(self, app)
        }

        fn barrier_label(
            self: ::waft_foundation::Handle<Self>,
            app: &::waft_foundation::App,
        ) -> ::std::option::Option<::std::string::String> {
            $crate::MaterialRouteTransitionMixin::barrier_label(self, app)
        }

        fn delegated_transition(
            self: ::waft_foundation::Handle<Self>,
            app: &::waft_foundation::App,
        ) -> ::std::option::Option<::waft_widgets::DelegatedTransitionBuilder> {
            $crate::MaterialRouteTransitionMixin::delegated_transition(self, app)
        }

        fn build_page(
            self: ::waft_foundation::Handle<Self>,
            app: &mut ::waft_foundation::App,
            context: ::waft_widgets::BuildContext,
            animation: ::waft_animation::AnimationRef<f64>,
            secondary_animation: ::waft_animation::AnimationRef<f64>,
        ) -> ::waft_widgets::WidgetRef {
            $crate::MaterialRouteTransitionMixin::build_page(
                self,
                app,
                context,
                animation,
                secondary_animation,
            )
        }

        fn build_transitions(
            self: ::waft_foundation::Handle<Self>,
            app: &mut ::waft_foundation::App,
            context: ::waft_widgets::BuildContext,
            animation: ::waft_animation::AnimationRef<f64>,
            secondary_animation: ::waft_animation::AnimationRef<f64>,
            child: ::waft_widgets::WidgetRef,
        ) -> ::waft_widgets::WidgetRef {
            $crate::MaterialRouteTransitionMixin::build_transitions(
                self,
                app,
                context,
                animation,
                secondary_animation,
                child,
            )
        }
    };
}

/// A page that creates a material style [`PageRoute`].
///
/// For Android, the entrance transition for the page zooms in and fades in while the exiting
/// page zooms out and fades out. The exit transition is similar, but in reverse.
///
/// For iOS, the page slides in from the right and exits in reverse. The page also shifts to the
/// left in parallax when another page enters to cover it. (These directions are flipped in
/// environments with a right-to-left reading direction.)
///
/// By default, when the created route is replaced by another, the previous route remains in
/// memory. To free all the resources when this is not necessary, set
/// [`maintain_state`](Self::maintain_state) to false.
///
/// The `fullscreen_dialog` property specifies whether the created route is a fullscreen modal
/// dialog. On iOS, those routes animate from the bottom to the top rather than horizontally.
///
/// See also:
///
///  * [`MaterialPageRoute`], which is the [`PageRoute`] version of this class
#[derive(Clone)]
pub struct MaterialPage {
    /// See [`Page::key`].
    pub key: Option<KeyRef>,

    /// See [`Page::can_pop`].
    pub can_pop: bool,

    /// See [`Page::on_pop_invoked`].
    pub on_pop_invoked: Option<PopInvokedWithResultCallback>,

    /// See [`Page::name`].
    pub name: Option<String>,

    /// See [`Page::arguments`].
    pub arguments: Option<Rc<dyn Any>>,

    /// See [`Page::restoration_id`].
    pub restoration_id: Option<String>,

    /// The content to be shown in the [`Route`] created by this page.
    pub child: WidgetRef,

    /// Whether the route should remain in memory when it is inactive.
    pub maintain_state: bool,

    /// Whether this page route is a full-screen dialog.
    pub fullscreen_dialog: bool,

    /// Whether the route transition will prefer to animate a snapshot of the entering and
    /// exiting routes.
    pub allow_snapshotting: bool,

    /// Whether the route introduces a route scope in the semantics tree; see
    /// [`MaterialRouteTransitionMixin::include_route_semantics`].
    pub include_route_semantics: bool,
}

impl MaterialPage {
    /// Creates a material page; Dart's optional arguments are the setters.
    ///
    /// Dart's defaults: `maintain_state` true, `fullscreen_dialog` false, `allow_snapshotting`
    /// true, `include_route_semantics` true, `can_pop` true.
    pub fn new<K>(child: impl IntoWidget<K>) -> MaterialPage {
        MaterialPage {
            key: None,
            can_pop: true,
            on_pop_invoked: None,
            name: None,
            arguments: None,
            restoration_id: None,
            child: child.into_widget(),
            maintain_state: true,
            fullscreen_dialog: false,
            allow_snapshotting: true,
            include_route_semantics: true,
        }
    }

    /// Dart `MaterialPage(maintainState:)`.
    pub fn maintain_state(mut self, maintain_state: bool) -> MaterialPage {
        self.maintain_state = maintain_state;
        self
    }

    /// Dart `MaterialPage(fullscreenDialog:)`.
    pub fn fullscreen_dialog(mut self, fullscreen_dialog: bool) -> MaterialPage {
        self.fullscreen_dialog = fullscreen_dialog;
        self
    }

    /// Dart `MaterialPage(allowSnapshotting:)`.
    pub fn allow_snapshotting(mut self, allow_snapshotting: bool) -> MaterialPage {
        self.allow_snapshotting = allow_snapshotting;
        self
    }

    /// Dart `MaterialPage(includeRouteSemantics:)`.
    pub fn include_route_semantics(mut self, include_route_semantics: bool) -> MaterialPage {
        self.include_route_semantics = include_route_semantics;
        self
    }

    /// Dart `MaterialPage(key:)`.
    pub fn key(mut self, key: KeyRef) -> MaterialPage {
        self.key = Some(key);
        self
    }

    /// Dart `MaterialPage(canPop:)`.
    pub fn can_pop(mut self, can_pop: bool) -> MaterialPage {
        self.can_pop = can_pop;
        self
    }

    /// Dart `MaterialPage(onPopInvoked:)`.
    pub fn on_pop_invoked(mut self, on_pop_invoked: PopInvokedWithResultCallback) -> MaterialPage {
        self.on_pop_invoked = Some(on_pop_invoked);
        self
    }

    /// Dart `MaterialPage(name:)`.
    pub fn name(mut self, name: String) -> MaterialPage {
        self.name = Some(name);
        self
    }

    /// Dart `MaterialPage(arguments:)`.
    pub fn arguments(mut self, arguments: Rc<dyn Any>) -> MaterialPage {
        self.arguments = Some(arguments);
        self
    }

    /// Dart `MaterialPage(restorationId:)`.
    pub fn restoration_id(mut self, restoration_id: String) -> MaterialPage {
        self.restoration_id = Some(restoration_id);
        self
    }
}

impl Debug for MaterialPage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MaterialPage")
            .field("name", &self.name)
            .field("child", &self.child)
            .finish_non_exhaustive()
    }
}

impl Page for MaterialPage {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    fn arguments(&self) -> Option<&Rc<dyn Any>> {
        self.arguments.as_ref()
    }

    fn restoration_id(&self) -> Option<&str> {
        self.restoration_id.as_deref()
    }

    fn can_pop(&self) -> bool {
        self.can_pop
    }

    fn on_pop_invoked(&self, app: &mut App, did_pop: bool, result: RouteResult) {
        if let Some(on_pop_invoked) = self.on_pop_invoked.clone() {
            on_pop_invoked(app, did_pop, result);
        }
    }

    fn create_route(&self, app: &mut App, context: BuildContext, this: &PageRef) -> RouteRef {
        let _ = context;
        PageBasedMaterialPageRoute::new(app, Rc::clone(this), self.allow_snapshotting).into_erased()
    }
}

// A page-based version of MaterialPageRoute.
//
// This route uses the builder from the page to build its content. This ensures the content is up
// to date after page updates.
struct PageBasedMaterialPageRoute {
    route: RouteData,
    overlay_route: OverlayRouteData,
    transition_route: TransitionRouteData,
    local_history: LocalHistoryRouteData,
    modal_route: ModalRouteData,
    page_route: PageRouteData,
}

impl PageBasedMaterialPageRoute {
    fn new(
        app: &mut App,
        page: PageRef,
        allow_snapshotting: bool,
    ) -> Ref<PageBasedMaterialPageRoute> {
        let route = RouteData::new(app, Some(RouteSettingsRef::Page(page)), None);
        let transition_route = TransitionRouteData::new(app);
        let modal_route = ModalRouteData::new(app);
        let mut page_route = PageRouteData::new();
        page_route.allow_snapshotting = allow_snapshotting;
        let this = app.create(PageBasedMaterialPageRoute {
            route,
            overlay_route: OverlayRouteData::new(),
            transition_route,
            local_history: LocalHistoryRouteData::new(),
            modal_route,
            page_route,
        });
        debug_assert!(TransitionRoute::opaque(this.handle(), app));
        this
    }

    /// Dart's `_page` getter, read in place.
    fn with_page<T>(self: Handle<Self>, app: &App, read: impl FnOnce(&MaterialPage) -> T) -> T {
        match Route::settings(self, app) {
            RouteSettingsRef::Page(page) => read(
                page.as_any()
                    .downcast_ref::<MaterialPage>()
                    .expect("a page-based material page route's settings are its MaterialPage"),
            ),
            RouteSettingsRef::Settings(_) => {
                panic!("a page-based material page route's settings are its MaterialPage")
            }
        }
    }
}

impl Route for PageBasedMaterialPageRoute {
    crate::material_route_transition_mixin_route_overrides!();
}

impl OverlayRoute for PageBasedMaterialPageRoute {
    waft_widgets::modal_overlay_route_overrides!();
}

impl PredictiveBackRoute for PageBasedMaterialPageRoute {
    waft_widgets::page_predictive_back_route_overrides!();
}

impl TransitionRoute for PageBasedMaterialPageRoute {
    crate::material_route_transition_mixin_transition_route_overrides!();

    fn opaque(self: Handle<Self>, app: &App) -> bool {
        PageRoute::opaque(self, app)
    }
}

impl LocalHistoryRoute for PageBasedMaterialPageRoute {
    waft_widgets::local_history_route_accessors!();
}

impl ModalRoute for PageBasedMaterialPageRoute {
    crate::material_route_transition_mixin_modal_route_overrides!();

    fn maintain_state(self: Handle<Self>, app: &App) -> bool {
        self.with_page(app, |page| page.maintain_state)
    }

    fn fullscreen_dialog(self: Handle<Self>, app: &App) -> bool {
        PageRoute::fullscreen_dialog(self, app)
    }
}

impl PageRoute for PageBasedMaterialPageRoute {
    waft_widgets::page_route_accessors!();

    fn fullscreen_dialog(self: Handle<Self>, app: &App) -> bool {
        self.with_page(app, |page| page.fullscreen_dialog)
    }
}

impl MaterialRouteTransitionMixin for PageBasedMaterialPageRoute {
    fn build_content(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let _ = context;
        self.with_page(app, |page| page.child.clone())
    }

    fn include_route_semantics(self: Handle<Self>, app: &App) -> bool {
        self.with_page(app, |page| page.include_route_semantics)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use waft_cupertino::CupertinoPageRoute;
    use waft_embedder::Offset;
    use waft_foundation::Listener;
    use waft_painting::EdgeInsetsGeometry;
    use waft_rendering::{AnyLayer, OpacityLayer, TransformLayer};
    use waft_widgets::{
        AnyElement, AnyModalRoute, Center, Container, FocusNodeLeaf, FocusScope, GestureDetector,
        GlobalKey, Navigator, NavigatorState, Padding, PageRouteBuilder, PreferredSizeWidgetRef,
        SizedBox, SnapshotWidget, StatefulBuilder, Text,
    };

    use waft_widgets::testing::{
        TargetPlatformVariant, TestPointer, WidgetTester, test_widgets, test_widgets_variant,
    };

    use super::*;
    use crate::app::MaterialApp;
    use crate::app_bar::AppBar;
    use crate::elevated_button::ElevatedButton;
    use crate::icon_button::IconButton;
    use crate::material::Material;
    use crate::page_transitions_theme::PageTransitionsTheme;
    use crate::scaffold::Scaffold;
    use crate::test_support::material_page_app;
    use crate::text_button::TextButton;
    use crate::theme_data::ThemeData;

    /// `tester.pumpWidget(MaterialApp(theme: theme(app), ..))` with the pages `configure` gives.
    fn pump_app(
        tester: &mut WidgetTester,
        theme: impl FnOnce(&App) -> ThemeData,
        configure: impl FnOnce(MaterialApp) -> MaterialApp,
    ) {
        let root = {
            let theme = theme(&tester.cell.borrow());
            material_page_app(theme, configure)
        };
        tester.pump_widget(root);
    }

    fn material_text(text: &'static str) -> WidgetBuilder {
        Rc::new(move |_app, _context| Material::new().child(Text::new(text)).into_widget())
    }

    fn scaffold_text(text: &'static str) -> WidgetBuilder {
        Rc::new(move |_app, _context| Scaffold::new().body(Text::new(text)).into_widget())
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

    fn the_text(tester: &WidgetTester, text: &str) -> AnyElement {
        *tester.find_text(text).first().expect("the text is mounted")
    }

    /// Dart's `tester.getTopLeft(find.text(text))`.
    fn top_left(tester: &WidgetTester, text: &str) -> Offset {
        tester
            .global_rect(tester.render_box_of(the_text(tester, text)))
            .top_left()
    }

    fn onstage(tester: &WidgetTester, text: &str) -> bool {
        !tester.find_onstage_text(text).is_empty()
    }

    fn push(tester: &WidgetTester, route: impl FnOnce(&mut App) -> RouteRef) {
        let navigator = navigator(tester);
        let mut app = tester.cell.borrow_mut();
        let route = route(&mut app);
        navigator.push(&mut app, route);
    }

    fn pop(tester: &WidgetTester) {
        navigator(tester).pop(&mut tester.cell.borrow_mut(), None);
    }

    // 'test page transition (CupertinoPageTransition)', without the edge shadow's paints, which
    // waft-cupertino's cases cover.
    #[test]
    fn a_material_page_route_slides_in_from_the_right_on_ios_and_macos() {
        for platform in [TargetPlatform::IOS, TargetPlatform::MacOS] {
            test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                pump_app(tester, ThemeData::new, |app| {
                    app.home(Material::new().child(Text::new("Page 1")))
                        .routes([("/next".to_string(), material_text("Page 2"))])
                });

                let widget_1_initial_top_left = top_left(tester, "Page 1");

                navigator(tester).push_named(&mut tester.cell.borrow_mut(), "/next", None);

                tester.pump();
                tester.pump_by(Duration::from_millis(150));

                let widget_1_transient_top_left = top_left(tester, "Page 1");
                let widget_2_top_left = top_left(tester, "Page 2");

                // Page 1 is moving to the left.
                assert!(widget_1_transient_top_left.dx() < widget_1_initial_top_left.dx());
                // Page 1 isn't moving vertically.
                assert!(widget_1_transient_top_left.dy() == widget_1_initial_top_left.dy());
                // iOS transition is horizontal only.
                assert!(widget_1_initial_top_left.dy() == widget_2_top_left.dy());
                // Page 2 is coming in from the right.
                assert!(widget_2_top_left.dx() > widget_1_initial_top_left.dx());

                tester.pump_and_settle();

                // Page 2 covers page 1.
                assert!(!onstage(tester, "Page 1"));
                assert!(onstage(tester, "Page 2"));

                pop(tester);
                tester.pump();
                tester.pump_by(Duration::from_millis(100));

                let widget_1_transient_top_left = top_left(tester, "Page 1");
                let widget_2_top_left = top_left(tester, "Page 2");

                // Page 1 is coming back from the left.
                assert!(widget_1_transient_top_left.dx() < widget_1_initial_top_left.dx());
                // Page 1 isn't moving vertically.
                assert!(widget_1_transient_top_left.dy() == widget_1_initial_top_left.dy());
                // iOS transition is horizontal only.
                assert!(widget_1_initial_top_left.dy() == widget_2_top_left.dy());
                // Page 2 is leaving towards the right.
                assert!(widget_2_top_left.dx() > widget_1_initial_top_left.dx());

                tester.pump_and_settle();

                assert!(onstage(tester, "Page 1"));
                assert!(tester.find_text("Page 2").is_empty());

                // Page 1 is back where it started.
                assert_eq!(top_left(tester, "Page 1"), widget_1_initial_top_left);
            });
        }
    }

    #[test]
    fn a_fullscreen_dialog_comes_in_from_the_bottom_on_ios_and_macos() {
        for platform in [TargetPlatform::IOS, TargetPlatform::MacOS] {
            test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                pump_app(tester, ThemeData::new, |app| {
                    app.home(Material::new().child(Text::new("Page 1")))
                });

                let widget_1_initial_top_left = top_left(tester, "Page 1");

                push(tester, |app| {
                    let route = MaterialPageRoute::new(app, material_text("Page 2"))
                        .fullscreen_dialog(app, true);
                    route.into_erased()
                });

                tester.pump();
                tester.pump_by(Duration::from_millis(100));

                let widget_1_transient_top_left = top_left(tester, "Page 1");
                let widget_2_top_left = top_left(tester, "Page 2");

                // Page 1 doesn't move.
                assert_eq!(widget_1_transient_top_left, widget_1_initial_top_left);
                // Fullscreen dialogs transitions vertically only.
                assert!(widget_1_initial_top_left.dx() == widget_2_top_left.dx());
                // Page 2 is coming in from the bottom.
                assert!(widget_2_top_left.dy() > widget_1_initial_top_left.dy());

                tester.pump_and_settle();

                // Page 2 covers page 1.
                assert!(!onstage(tester, "Page 1"));
                assert!(onstage(tester, "Page 2"));

                pop(tester);
                tester.pump();
                tester.pump_by(Duration::from_millis(100));

                let widget_1_transient_top_left = top_left(tester, "Page 1");
                let widget_2_top_left = top_left(tester, "Page 2");

                // Page 1 doesn't move.
                assert_eq!(widget_1_transient_top_left, widget_1_initial_top_left);
                // Fullscreen dialogs transitions vertically only.
                assert!(widget_1_initial_top_left.dx() == widget_2_top_left.dx());
                // Page 2 is leaving towards the bottom.
                assert!(widget_2_top_left.dy() > widget_1_initial_top_left.dy());

                tester.pump_and_settle();

                assert!(onstage(tester, "Page 1"));
                assert!(tester.find_text("Page 2").is_empty());

                // Page 1 is back where it started.
                assert_eq!(top_left(tester, "Page 1"), widget_1_initial_top_left);
            });
        }
    }

    /// The back-gesture cases' app: `Page 1` at home and `Page 2` at `/next`, each the body of a
    /// scaffold, with `/next` pushed and settled.
    fn pushed_scaffold_pages(platform: TargetPlatform) -> WidgetTester {
        let mut tester = WidgetTester::on(platform);
        pump_app(&mut tester, ThemeData::new, |app| {
            app.home(Scaffold::new().body(Text::new("Page 1")))
                .routes([("/next".to_string(), scaffold_text("Page 2"))])
        });

        navigator(&tester).push_named(&mut tester.cell.borrow_mut(), "/next", None);
        tester.pump_and_settle();

        assert!(tester.find_onstage_text("Page 1").is_empty());
        assert!(onstage(&tester, "Page 2"));
        tester
    }

    #[test]
    fn there_is_no_back_gesture_on_android() {
        test_widgets(|_tester| {
            let mut tester = pushed_scaffold_pages(TargetPlatform::Android);

            // Drag from left edge to invoke the gesture.
            let mut gesture = TestPointer::touch(1);
            tester.down(&mut gesture, Offset::new(5.0, 100.0));
            tester.move_to(&mut gesture, Offset::new(405.0, 100.0), true);
            tester.pump();

            assert!(tester.find_onstage_text("Page 1").is_empty());
            assert!(onstage(&tester, "Page 2"));

            // Page 2 didn't move.
            assert_eq!(top_left(&tester, "Page 2"), Offset::ZERO);
        });
    }

    #[test]
    fn the_back_gesture_drags_the_page_with_the_finger_on_ios_and_macos() {
        for platform in [TargetPlatform::IOS, TargetPlatform::MacOS] {
            test_widgets(|_tester| {
                let mut tester = pushed_scaffold_pages(platform);

                // Drag from left edge to invoke the gesture.
                let mut gesture = TestPointer::touch(1);
                tester.down(&mut gesture, Offset::new(5.0, 100.0));
                tester.move_to(&mut gesture, Offset::new(405.0, 100.0), true);
                tester.pump();

                // Page 1 is now visible.
                assert!(onstage(&tester, "Page 1"));
                assert!(onstage(&tester, "Page 2"));

                // The route widget position needs to track the finger position very exactly.
                assert_eq!(top_left(&tester, "Page 2"), Offset::new(400.0, 0.0));

                tester.move_to(&mut gesture, Offset::new(205.0, 100.0), true);
                tester.pump();

                assert_eq!(top_left(&tester, "Page 2"), Offset::new(200.0, 0.0));

                tester.move_to(&mut gesture, Offset::new(105.0, 300.0), true);
                tester.pump();

                assert_eq!(top_left(&tester, "Page 2"), Offset::new(100.0, 0.0));
            });
        }
    }

    #[test]
    fn there_is_no_back_gesture_on_a_fullscreen_dialog_on_ios_and_macos() {
        for platform in [TargetPlatform::IOS, TargetPlatform::MacOS] {
            test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                pump_app(tester, ThemeData::new, |app| {
                    app.home(Scaffold::new().body(Text::new("Page 1")))
                });

                push(tester, |app| {
                    let route = MaterialPageRoute::new(app, scaffold_text("Page 2"))
                        .fullscreen_dialog(app, true);
                    route.into_erased()
                });
                tester.pump_and_settle();

                assert!(tester.find_onstage_text("Page 1").is_empty());
                assert!(onstage(tester, "Page 2"));

                // Drag from left edge to invoke the gesture.
                let mut gesture = TestPointer::touch(1);
                tester.down(&mut gesture, Offset::new(5.0, 100.0));
                tester.move_to(&mut gesture, Offset::new(405.0, 100.0), true);
                tester.pump();

                assert!(tester.find_onstage_text("Page 1").is_empty());
                assert!(onstage(tester, "Page 2"));

                // Page 2 didn't move.
                assert_eq!(top_left(tester, "Page 2"), Offset::ZERO);
            });
        }
    }

    fn app_bar_page(title: &'static str) -> WidgetRef {
        Scaffold::new()
            .app_bar(PreferredSizeWidgetRef::new(
                AppBar::new().title(Text::new(title)),
            ))
            .body(SizedBox::new())
            .into_widget()
    }

    #[test]
    fn a_fullscreen_dialog_does_not_move_the_route_below_it_on_ios_and_macos() {
        for platform in [TargetPlatform::IOS, TargetPlatform::MacOS] {
            test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                pump_app(tester, ThemeData::new, |app| {
                    app.initial_route("/").on_generate_route(|app, settings| {
                        if settings.name.as_deref() == Some("/") {
                            let route = PageRouteBuilder::new(
                                app,
                                Rc::new(|_app, _context, _animation, _secondary| {
                                    app_bar_page("Page 1")
                                }),
                            );
                            return Some(route.erase());
                        }
                        let route = MaterialPageRoute::new(
                            app,
                            Rc::new(|_app, _context| app_bar_page("Page 2")),
                        )
                        .fullscreen_dialog(app, true);
                        Some(route.erase())
                    })
                });

                assert_eq!(tester.find_onstage_text("Page 1").len(), 1);
                assert!(tester.find_onstage_text("Page 2").is_empty());

                let page_title_dx = top_left(tester, "Page 1").dx();

                navigator(tester).push_named(&mut tester.cell.borrow_mut(), "/next", None);
                tester.pump();
                tester.pump_by(Duration::from_millis(100));

                // Second page transition has started.
                assert_eq!(tester.find_onstage_text("Page 2").len(), 1);

                // First page has not moved.
                assert_eq!(top_left(tester, "Page 1").dx(), page_title_dx);
            });
        }
    }

    #[test]
    fn a_material_page_route_slides_out_under_a_cupertino_page_route_on_ios_and_macos() {
        for platform in [TargetPlatform::IOS, TargetPlatform::MacOS] {
            test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                pump_app(
                    tester,
                    |app| {
                        ThemeData::builder()
                            .platform(TargetPlatform::IOS)
                            .build(app)
                    },
                    |app| app.home(app_bar_page("Title")),
                );

                let title_initial_top_left = top_left(tester, "Title");

                push(tester, |app| {
                    (CupertinoPageRoute::new(
                        app,
                        Rc::new(|_app, _context| SizedBox::expand().into_widget()),
                    ))
                    .into_erased()
                });

                tester.pump();
                tester.pump_by(Duration::from_millis(150));

                let title_transient_top_left = top_left(tester, "Title");

                // Title of the first route slides to the left.
                assert_eq!(title_initial_top_left.dy(), title_transient_top_left.dy());
                assert!(title_initial_top_left.dx() > title_transient_top_left.dx());
            });
        }
    }

    // 'test adaptable transitions switch during execution'.
    #[test]
    fn the_transition_follows_the_theme_platform_while_it_runs() {
        for platform in [TargetPlatform::IOS, TargetPlatform::MacOS] {
            test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                let routes = || [("/next".to_string(), material_text("Page 2"))];
                pump_app(
                    tester,
                    |app| {
                        ThemeData::builder()
                            .platform(TargetPlatform::Android)
                            .page_transitions_theme(PageTransitionsTheme::new().with_builders(
                                indexmap::IndexMap::from([(
                                    TargetPlatform::Android,
                                    PageTransitionsBuilderRef::new(
                                        waft_widgets::FadeUpwardsPageTransitionsBuilder::new(),
                                    ),
                                )]),
                            ))
                            .build(app)
                    },
                    move |app| {
                        app.home(Material::new().child(Text::new("Page 1")))
                            .routes(routes())
                    },
                );

                let widget_1_initial_top_left = top_left(tester, "Page 1");

                navigator(tester).push_named(&mut tester.cell.borrow_mut(), "/next", None);
                tester.pump();
                tester.pump_by(Duration::from_millis(100));

                let widget_2_rect =
                    tester.global_rect(tester.render_box_of(the_text(tester, "Page 2")));
                let widget_2_top_left = widget_2_rect.top_left();

                // Android transition is vertical only.
                assert!(widget_1_initial_top_left.dx() == widget_2_top_left.dx());
                // Page 1 is above page 2 mid-transition.
                assert!(widget_1_initial_top_left.dy() < widget_2_top_left.dy());
                // Animation begins from the top of the page.
                assert!(widget_2_top_left.dy() < widget_2_rect.height());

                tester.pump_by(Duration::from_millis(300));

                // Page 2 covers page 1.
                assert!(tester.find_onstage_text("Page 1").is_empty());
                assert!(onstage(tester, "Page 2"));

                // Re-pump the same app but with iOS instead of Android.
                pump_app(tester, ThemeData::new, move |app| {
                    app.home(Material::new().child(Text::new("Page 1")))
                        .routes(routes())
                });

                pop(tester);
                tester.pump();
                tester.pump_by(Duration::from_millis(100));

                let widget_1_transient_top_left = top_left(tester, "Page 1");
                let widget_2_top_left = top_left(tester, "Page 2");

                // Page 1 is coming back from the left.
                assert!(widget_1_transient_top_left.dx() < widget_1_initial_top_left.dx());
                // Page 1 isn't moving vertically.
                assert!(widget_1_transient_top_left.dy() == widget_1_initial_top_left.dy());
                // iOS transition is horizontal only.
                assert!(widget_1_initial_top_left.dy() == widget_2_top_left.dy());
                // Page 2 is leaving towards the right.
                assert!(widget_2_top_left.dx() > widget_1_initial_top_left.dx());

                tester.pump_by(Duration::from_millis(300));

                assert!(onstage(tester, "Page 1"));
                assert!(tester.find_text("Page 2").is_empty());

                // Page 1 is back where it started.
                assert_eq!(top_left(tester, "Page 1"), widget_1_initial_top_left);
            });
        }
    }

    #[test]
    fn a_barrier_dismissible_material_page_route_pops_on_escape() {
        test_widgets(|tester| {
            pump_app(tester, ThemeData::new, |app| {
                app.home(Scaffold::new().body(Text::new("home")))
            });
            push(tester, |app| {
                let route = MaterialPageRoute::new(app, scaffold_text("route"))
                    .barrier_dismissible(app, true);
                route.into_erased()
            });
            tester.pump_and_settle();
            assert_eq!(tester.find_onstage_text("route").len(), 1);
            assert!(tester.find_onstage_text("home").is_empty());

            // Try to dismiss the route with the escape key.
            tester.send_key_event(
                waft_services::PhysicalKeyboardKey::ESCAPE,
                waft_services::LogicalKeyboardKey::ESCAPE,
            );
            tester.pump_and_settle();

            assert!(tester.find_onstage_text("route").is_empty());
        });
    }

    #[test]
    fn a_route_that_does_not_maintain_state_is_not_built_under_another() {
        test_widgets(|tester| {
            pump_app(tester, ThemeData::new, |app| {
                app.on_generate_route(|app, settings| {
                    let first = settings.name.as_deref() == Some("/");
                    let route = MaterialPageRoute::new(
                        app,
                        Rc::new(move |_app, _context| {
                            Text::new(if first { "first" } else { "second" }).into_widget()
                        }),
                    )
                    .maintain_state(app, !first);
                    Some(route.erase())
                })
            });
            navigator(tester).push_named(&mut tester.cell.borrow_mut(), "/second", None);
            tester.pump_and_settle();

            // The first page does not maintain state.
            assert!(tester.find_text("first").is_empty());
            assert_eq!(tester.find_onstage_text("second").len(), 1);
        });
    }

    #[test]
    fn material_page_routes_hand_each_other_one_delegated_transition() {
        test_widgets(|tester| {
            let first_page_context: Rc<std::cell::Cell<Option<BuildContext>>> = Rc::default();
            pump_app(tester, ThemeData::new, {
                let first_page_context = Rc::clone(&first_page_context);
                move |app| {
                    app.home(waft_widgets::Builder::new(move |_app, context| {
                        first_page_context.set(Some(context));
                        Text::new("first").into_widget()
                    }))
                }
            });
            let pushed: Rc<std::cell::Cell<Option<Handle<MaterialPageRoute>>>> = Rc::default();
            push(tester, |app| {
                let route = MaterialPageRoute::new(app, material_text("second"));
                pushed.set(Some(route.handle()));
                route.into_erased()
            });
            tester.pump_and_settle();

            let mut app = tester.cell.borrow_mut();
            let first = AnyModalRoute::of(&mut app, first_page_context.get().expect("built"))
                .expect("the first page's route")
                .as_route()
                .downcast::<MaterialPageRoute>(&app)
                .expect("a material page route");
            let second = pushed.get().expect("pushed");
            // Both are the tear-off of the mixin's static `_delegatedTransition`.
            assert_eq!(
                ModalRoute::delegated_transition(first, &app),
                ModalRoute::delegated_transition(second, &app)
            );
            assert!(ModalRoute::delegated_transition(first, &app).is_some());
            // So the route below receives no transition: the two already run in sync.
            assert!(first.modal_route_data(&app).received_transition.is_none());
            assert!(TransitionRoute::can_transition_to(
                first,
                &app,
                TransitionRoute::as_transition_route(second)
            ));
        });
    }

    #[test]
    fn a_material_page_updates_its_route_content_without_a_new_route() {
        test_widgets(|tester| {
            let key: KeyRef = KeyRef::Local(Rc::new(waft_foundation::UniqueKey::new()));
            let context: Rc<std::cell::Cell<Option<BuildContext>>> = Rc::default();
            let pages = |text: &'static str| -> Vec<PageRef> {
                let context = Rc::clone(&context);
                vec![Rc::new(
                    MaterialPage::new(waft_widgets::Builder::new(move |_app, page_context| {
                        context.set(Some(page_context));
                        Text::new(text).into_widget()
                    }))
                    .key(key.clone()),
                )]
            };
            let navigator_for = |pages: Vec<PageRef>| {
                waft_widgets::Directionality::new(
                    waft_embedder::TextDirection::Ltr,
                    Navigator::new()
                        .pages(pages)
                        .on_did_remove_page(|_app, _page| panic!("no page is removed")),
                )
                .into_widget()
            };
            tester.pump_widget(navigator_for(pages("first")));
            assert_eq!(tester.find_onstage_text("first").len(), 1);
            let first_route = {
                let mut app = tester.cell.borrow_mut();
                AnyModalRoute::of(&mut app, context.get().expect("built")).expect("a route")
            };

            tester.pump_widget(navigator_for(pages("second")));
            let second_route = {
                let mut app = tester.cell.borrow_mut();
                AnyModalRoute::of(&mut app, context.get().expect("built")).expect("a route")
            };
            // There should be no transition because the page has the same key.
            assert_eq!(first_route, second_route);
            // The content does update.
            assert!(tester.find_text("first").is_empty());
            assert_eq!(tester.find_onstage_text("second").len(), 1);
        });
    }

    #[test]
    fn a_material_page_can_toggle_maintain_state() {
        test_widgets(|tester| {
            let key_one: KeyRef = KeyRef::Local(Rc::new(waft_foundation::UniqueKey::new()));
            let key_two: KeyRef = KeyRef::Local(Rc::new(waft_foundation::UniqueKey::new()));
            let pages = |maintain_state: bool| -> Vec<PageRef> {
                vec![
                    Rc::new(
                        MaterialPage::new(Text::new("first"))
                            .key(key_one.clone())
                            .maintain_state(maintain_state),
                    ),
                    Rc::new(MaterialPage::new(Text::new("second")).key(key_two.clone())),
                ]
            };
            let navigator_for = |pages: Vec<PageRef>| {
                waft_widgets::Directionality::new(
                    waft_embedder::TextDirection::Ltr,
                    Navigator::new()
                        .pages(pages)
                        .on_did_remove_page(|_app, _page| panic!("no page is removed")),
                )
                .into_widget()
            };
            tester.pump_widget(navigator_for(pages(false)));

            // Page one does not maintain state.
            assert!(tester.find_text("first").is_empty());
            assert_eq!(tester.find_onstage_text("second").len(), 1);

            tester.pump_widget(navigator_for(pages(true)));

            // Page one sets the maintain state to be true, its widget tree should be built.
            assert_eq!(tester.find_text("first").len(), 1);
            assert_eq!(tester.find_onstage_text("second").len(), 1);
        });
    }

    #[test]
    fn the_route_transition_durations_are_the_platform_builders() {
        for (platform, duration) in [
            (TargetPlatform::Android, Duration::from_millis(450)),
            (TargetPlatform::IOS, Duration::from_millis(500)),
            (TargetPlatform::MacOS, Duration::from_millis(500)),
            (TargetPlatform::Windows, Duration::from_millis(300)),
            (TargetPlatform::Linux, Duration::from_millis(300)),
            (TargetPlatform::Fuchsia, Duration::from_millis(300)),
        ] {
            test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                pump_app(tester, ThemeData::new, |app| {
                    app.home(Material::new().child(Text::new("home")))
                });
                let pushed: Rc<std::cell::Cell<Option<Handle<MaterialPageRoute>>>> = Rc::default();
                push(tester, |app| {
                    let route = MaterialPageRoute::new(app, material_text("next"));
                    pushed.set(Some(route.handle()));
                    route.into_erased()
                });
                let route = pushed.get().expect("pushed");
                let mut app = tester.cell.borrow_mut();
                assert_eq!(
                    TransitionRoute::transition_duration(route, &mut app),
                    duration,
                    "{platform:?}"
                );
                let controller = TransitionRoute::controller(route, &app).expect("installed");
                assert_eq!(app.get(controller).duration, Some(duration), "{platform:?}");
            });
        }
    }

    /// A page with a `TextButton` showing `label` that runs `on_pressed` with the page's context.
    fn button_page(
        label: &'static str,
        on_pressed: impl Fn(&mut App, BuildContext) + 'static,
    ) -> WidgetBuilder {
        let on_pressed = Rc::new(on_pressed);
        Rc::new(move |_app, context| {
            let on_pressed = Rc::clone(&on_pressed);
            Material::new()
                .child(TextButton::new(
                    Some(Listener::new(move |app| on_pressed(app, context))),
                    Some(Text::new(label)),
                ))
                .into_widget()
        })
    }

    /// Dart's `tester.getCenter(find.text(text))`.
    fn center(tester: &WidgetTester, text: &str) -> Offset {
        tester
            .global_rect(tester.render_box_of(the_text(tester, text)))
            .center()
    }

    /// Dart's `tester.tap(find.text(text))`.
    fn tap_text(tester: &mut WidgetTester, text: &str) {
        let at = center(tester, text);
        tester.tap(at, 1);
    }

    fn count_onstage(tester: &WidgetTester, text: &str) -> usize {
        tester.find_onstage_text(text).len()
    }

    fn platform_at(tester: &WidgetTester, text: &str) -> TargetPlatform {
        let element = the_text(tester, text);
        Theme::of(&mut tester.cell.borrow_mut(), element).platform
    }

    fn platform_theme(platform: TargetPlatform) -> impl FnOnce(&App) -> ThemeData {
        move |app| ThemeData::builder().platform(platform).build(app)
    }

    #[test]
    fn back_gesture_while_os_changes() {
        test_widgets(|tester| {
            let routes = || {
                [
                    (
                        "/".to_string(),
                        button_page("PUSH", |app, context| {
                            Navigator::push_named(app, context, "/b", None);
                        }),
                    ),
                    (
                        "/b".to_string(),
                        Rc::new(|_app: &mut App, _context: BuildContext| {
                            Text::new("HELLO").into_widget()
                        }) as WidgetBuilder,
                    ),
                ]
            };
            pump_app(tester, platform_theme(TargetPlatform::IOS), |app| {
                app.routes(routes())
            });
            tap_text(tester, "PUSH");
            assert_eq!(tester.pump_and_settle_by(Duration::from_secs(60)), 2);
            assert_eq!(count_onstage(tester, "PUSH"), 0);
            assert_eq!(count_onstage(tester, "HELLO"), 1);
            let hello_position_1 = center(tester, "HELLO");
            let mut gesture = TestPointer::touch(1);
            tester.down(&mut gesture, Offset::new(2.5, 300.0));
            tester.pump_by(Duration::from_millis(20));
            let to = gesture.position + Offset::new(100.0, 0.0);
            tester.move_to(&mut gesture, to, true);
            assert_eq!(count_onstage(tester, "PUSH"), 0);
            assert_eq!(count_onstage(tester, "HELLO"), 1);
            tester.pump_by(Duration::from_millis(20));
            assert_eq!(count_onstage(tester, "PUSH"), 1);
            assert_eq!(count_onstage(tester, "HELLO"), 1);
            let hello_position_2 = center(tester, "HELLO");
            assert!(hello_position_1.dx() < hello_position_2.dx());
            assert_eq!(hello_position_1.dy(), hello_position_2.dy());
            assert_eq!(platform_at(tester, "HELLO"), TargetPlatform::IOS);
            pump_app(tester, platform_theme(TargetPlatform::Android), |app| {
                app.routes(routes())
            });
            // Now we have to let the theme animation run through.
            // This takes three frames (including the first one above):
            //  1. Start the Theme animation. It's at t=0 so everything else is identical.
            //  2. Start any animations that are informed by the Theme, for example, the
            //     DefaultTextStyle, on the first frame that the theme is not at t=0. In
            //     this case, it's at t=1.0 of the theme animation, so this is also the
            //     frame in which the theme animation ends.
            //  3. End all the other animations.
            assert_eq!(tester.pump_and_settle_by(Duration::from_secs(60)), 2);
            assert_eq!(platform_at(tester, "HELLO"), TargetPlatform::Android);
            let hello_position_3 = center(tester, "HELLO");
            assert_eq!(hello_position_3, hello_position_2);
            assert_eq!(count_onstage(tester, "PUSH"), 1);
            assert_eq!(count_onstage(tester, "HELLO"), 1);
            let to = gesture.position + Offset::new(100.0, 0.0);
            tester.move_to(&mut gesture, to, true);
            tester.pump_by(Duration::from_millis(20));
            assert_eq!(count_onstage(tester, "PUSH"), 1);
            assert_eq!(count_onstage(tester, "HELLO"), 1);
            let hello_position_4 = center(tester, "HELLO");
            assert!(hello_position_3.dx() < hello_position_4.dx());
            assert_eq!(hello_position_3.dy(), hello_position_4.dy());
            let to = gesture.position + Offset::new(500.0, 0.0);
            tester.move_to(&mut gesture, to, true);
            tester.up(&gesture);
            assert_eq!(tester.pump_and_settle_by(Duration::from_secs(60)), 3);
            assert_eq!(count_onstage(tester, "PUSH"), 1);
            assert!(tester.find_text("HELLO").is_empty());

            pump_app(tester, platform_theme(TargetPlatform::MacOS), |app| {
                app.routes(routes())
            });
            tap_text(tester, "PUSH");
            assert_eq!(tester.pump_and_settle_by(Duration::from_secs(60)), 2);
            assert_eq!(count_onstage(tester, "PUSH"), 0);
            assert_eq!(count_onstage(tester, "HELLO"), 1);
            let hello_position_5 = center(tester, "HELLO");
            tester.down(&mut gesture, Offset::new(2.5, 300.0));
            tester.pump_by(Duration::from_millis(20));
            let to = gesture.position + Offset::new(100.0, 0.0);
            tester.move_to(&mut gesture, to, true);
            assert_eq!(count_onstage(tester, "PUSH"), 0);
            assert_eq!(count_onstage(tester, "HELLO"), 1);
            tester.pump_by(Duration::from_millis(20));
            assert_eq!(count_onstage(tester, "PUSH"), 1);
            assert_eq!(count_onstage(tester, "HELLO"), 1);
            let hello_position_6 = center(tester, "HELLO");
            assert!(hello_position_5.dx() < hello_position_6.dx());
            assert_eq!(hello_position_5.dy(), hello_position_6.dy());
            assert_eq!(platform_at(tester, "HELLO"), TargetPlatform::MacOS);
        });
    }

    /// `MaterialApp(onGenerateRoute:)` of `MaterialPageRoute`s: `/` is 'Page 1', the rest
    /// 'Page 2', each in a `Center`.
    fn numbered_pages(tester: &mut WidgetTester) {
        pump_app(tester, ThemeData::new, |app| {
            app.on_generate_route(|app, settings| {
                let page_number = if settings.name.as_deref() == Some("/") {
                    "1"
                } else {
                    "2"
                };
                let route = MaterialPageRoute::new(
                    app,
                    Rc::new(move |_app, _context| {
                        Center::new()
                            .child(Text::new(format!("Page {page_number}")))
                            .into_widget()
                    }),
                )
                .settings(app, RouteSettingsRef::Settings(settings.clone()));
                Some(route.erase())
            })
        });
    }

    #[test]
    fn test_edge_swipe_then_drop_back_at_starting_point_works() {
        for platform in [TargetPlatform::IOS, TargetPlatform::MacOS] {
            test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                numbered_pages(tester);

                navigator(tester).push_named(&mut tester.cell.borrow_mut(), "/next", None);

                tester.pump();
                tester.pump_by(Duration::from_secs(1));

                assert_eq!(count_onstage(tester, "Page 1"), 0);
                assert!(onstage(tester, "Page 2"));

                let mut gesture = TestPointer::touch(1);
                tester.down(&mut gesture, Offset::new(5.0, 200.0));
                let to = gesture.position + Offset::new(300.0, 0.0);
                tester.move_to(&mut gesture, to, true);
                tester.pump();
                // Bring it exactly back such that there's nothing to animate when releasing.
                let to = gesture.position + Offset::new(-300.0, 0.0);
                tester.move_to(&mut gesture, to, true);
                tester.up(&gesture);
                tester.pump();

                assert_eq!(count_onstage(tester, "Page 1"), 0);
                assert!(onstage(tester, "Page 2"));
            });
        }
    }

    #[test]
    fn test_edge_swipe_then_drop_back_at_ending_point_works() {
        for platform in [TargetPlatform::IOS, TargetPlatform::MacOS] {
            test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                numbered_pages(tester);

                navigator(tester).push_named(&mut tester.cell.borrow_mut(), "/next", None);

                tester.pump();
                tester.pump_by(Duration::from_secs(1));

                assert_eq!(count_onstage(tester, "Page 1"), 0);
                assert!(onstage(tester, "Page 2"));

                let mut gesture = TestPointer::touch(1);
                tester.down(&mut gesture, Offset::new(5.0, 200.0));
                // The width of the page.
                let to = gesture.position + Offset::new(800.0, 0.0);
                tester.move_to(&mut gesture, to, true);
                tester.up(&gesture);
                tester.pump();

                assert!(onstage(tester, "Page 1"));
                assert_eq!(count_onstage(tester, "Page 2"), 0);
            });
        }
    }

    /// Dart's `tester.getTopLeft(find.ancestor(of: find.text(text), matching: find.byType(Scaffold)))`.
    fn scaffold_top_left(tester: &WidgetTester, text: &str) -> Offset {
        let scaffold = tester.ancestor_of_type::<Scaffold>(the_text(tester, text));
        tester
            .global_rect(tester.render_box_of(scaffold))
            .top_left()
    }

    /// Dart's `tester.getTopLeft(find.byKey(key))`.
    fn key_top_left(tester: &WidgetTester, key: &GlobalKey) -> Offset {
        let render = tester.render_object(key).as_box().expect("a render box");
        render.local_to_global(&tester.cell.borrow(), Offset::ZERO, None)
    }

    fn route_scaffold(app: &mut App) -> RouteRef {
        let route = MaterialPageRoute::new(
            app,
            Rc::new(|_app, _context| {
                Scaffold::new()
                    .body(Center::new().child(Text::new("route")))
                    .into_widget()
            }),
        );
        route.into_erased()
    }

    #[test]
    fn back_swipe_dismiss_interrupted_by_route_push() {
        for platform in [TargetPlatform::IOS, TargetPlatform::MacOS] {
            test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                // Regression test for https://github.com/flutter/flutter/issues/28728
                let scaffold_key = GlobalKey::new();
                let home_key = scaffold_key.clone();
                pump_app(tester, ThemeData::new, move |app| {
                    let push_key = home_key.clone();
                    app.home(
                        Scaffold::new()
                            .key(home_key.into())
                            .body(Center::new().child(ElevatedButton::new(
                                Some(Listener::new(move |app| {
                                    let context = push_key.current_context(app).expect("mounted");
                                    let route = route_scaffold(app);
                                    Navigator::push(app, context, route.clone());
                                })),
                                Some(Text::new("push")),
                            ))),
                    )
                });

                // Check the basic iOS back-swipe dismiss transition. Dragging the pushed
                // route halfway across the screen will trigger the iOS dismiss animation.

                tap_text(tester, "push");
                tester.pump_and_settle();
                assert_eq!(count_onstage(tester, "route"), 1);
                assert_eq!(count_onstage(tester, "push"), 0);

                let mut gesture = TestPointer::touch(1);
                tester.down(&mut gesture, Offset::new(5.0, 300.0));
                let to = gesture.position + Offset::new(400.0, 0.0);
                tester.move_to(&mut gesture, to, true);
                tester.up(&gesture);
                tester.pump();
                // The 'route' route has been dragged to the right, halfway across the screen.
                assert_eq!(scaffold_top_left(tester, "route"), Offset::new(400.0, 0.0));
                // The 'push' route is sliding in from the left.
                assert!(scaffold_top_left(tester, "push").dx() < 0.0);
                tester.pump_and_settle();
                assert_eq!(count_onstage(tester, "push"), 1);
                assert_eq!(scaffold_top_left(tester, "push"), Offset::ZERO);
                assert_eq!(count_onstage(tester, "route"), 0);

                // Run the dismiss animation 60%, which exposes the route "push" button,
                // and then press the button. A drag dropped animation is 400ms when dropped
                // exactly halfway. It follows a curve that is very steep initially.

                tap_text(tester, "push");
                tester.pump_and_settle();
                assert_eq!(count_onstage(tester, "route"), 1);
                assert_eq!(count_onstage(tester, "push"), 0);

                let mut gesture = TestPointer::touch(2);
                tester.down(&mut gesture, Offset::new(5.0, 300.0));
                // Drag halfway.
                let to = gesture.position + Offset::new(400.0, 0.0);
                tester.move_to(&mut gesture, to, true);
                tester.up(&gesture);
                // Trigger the dropped snapping animation.
                tester.pump();
                assert_eq!(scaffold_top_left(tester, "route"), Offset::new(400.0, 0.0));
                // Let the dismissing snapping animation go 60%.
                tester.pump_by(Duration::from_millis(240));
                let dx = scaffold_top_left(tester, "route").dx();
                assert!((dx - 794.0).abs() <= 1.0, "{dx}");

                // Use the navigator to push a route instead of tapping the 'push' button.
                // The topmost route (the one that's animating away), ignores input while
                // the pop is underway because route.navigator.userGestureInProgress.
                {
                    let mut app = tester.cell.borrow_mut();
                    let context = scaffold_key.current_context(&app).expect("mounted");
                    let route = route_scaffold(&mut app);
                    Navigator::push(&mut app, context, route.clone());
                }

                tester.pump_and_settle();
                assert_eq!(count_onstage(tester, "route"), 1);
                assert_eq!(count_onstage(tester, "push"), 0);
            });
        }
    }

    /// The home scaffold and the page scaffold of the back-swipe input cases, each over a
    /// `GestureDetector` that counts its taps; answers the two counts.
    fn tap_counting_pages(
        tester: &mut WidgetTester,
        home_scaffold_key: &GlobalKey,
    ) -> (Rc<Cell<u32>>, Rc<Cell<u32>>) {
        let home_tap_count: Rc<Cell<u32>> = Rc::default();
        let page_tap_count: Rc<Cell<u32>> = Rc::default();
        let home_key = home_scaffold_key.clone();
        let home_taps = Rc::clone(&home_tap_count);
        pump_app(tester, ThemeData::new, move |app| {
            app.home(
                Scaffold::new()
                    .key(home_key.into())
                    .body(GestureDetector::new().on_tap(Listener::new(move |_app| {
                        home_taps.set(home_taps.get() + 1);
                    }))),
            )
        });
        (home_tap_count, page_tap_count)
    }

    fn push_tap_counting_page(
        tester: &WidgetTester,
        home_scaffold_key: &GlobalKey,
        page_scaffold_key: &GlobalKey,
        page_tap_count: &Rc<Cell<u32>>,
    ) {
        let mut app = tester.cell.borrow_mut();
        let context = home_scaffold_key.current_context(&app).expect("mounted");
        let page_key = page_scaffold_key.clone();
        let page_taps = Rc::clone(page_tap_count);
        let route = MaterialPageRoute::new(
            &mut app,
            Rc::new(move |_app, _context| {
                let page_taps = Rc::clone(&page_taps);
                Scaffold::new()
                    .key(page_key.clone().into())
                    .app_bar(PreferredSizeWidgetRef::new(
                        AppBar::new().title(Text::new("Page")),
                    ))
                    .body(Padding::new(EdgeInsetsGeometry::all(16.0)).child(
                        GestureDetector::new().on_tap(Listener::new(move |_app| {
                            page_taps.set(page_taps.get() + 1);
                        })),
                    ))
                    .into_widget()
            }),
        );
        Navigator::push(&mut app, context, route.erase());
    }

    fn tap_key(tester: &mut WidgetTester, key: &GlobalKey) {
        let at = tester.center_of(key);
        tester.tap(at, 1);
    }

    #[test]
    fn during_back_swipe_the_route_ignores_input() {
        for platform in [TargetPlatform::IOS, TargetPlatform::MacOS] {
            test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                // Regression test for https://github.com/flutter/flutter/issues/39989

                let home_scaffold_key = GlobalKey::new();
                let page_scaffold_key = GlobalKey::new();
                let (home_tap_count, page_tap_count) =
                    tap_counting_pages(tester, &home_scaffold_key);

                tap_key(tester, &home_scaffold_key);
                assert_eq!(home_tap_count.get(), 1);
                assert_eq!(page_tap_count.get(), 0);

                push_tap_counting_page(
                    tester,
                    &home_scaffold_key,
                    &page_scaffold_key,
                    &page_tap_count,
                );

                tester.pump_and_settle();
                tap_key(tester, &page_scaffold_key);
                assert_eq!(home_tap_count.get(), 1);
                assert_eq!(page_tap_count.get(), 1);

                // Start the basic iOS back-swipe dismiss transition. Drag the pushed
                // "page" route halfway across the screen. The underlying "home" will
                // start sliding in from the left.

                let mut gesture = TestPointer::touch(2);
                tester.down(&mut gesture, Offset::new(5.0, 300.0));
                let to = gesture.position + Offset::new(400.0, 0.0);
                tester.move_to(&mut gesture, to, true);
                tester.pump();
                assert_eq!(
                    key_top_left(tester, &page_scaffold_key),
                    Offset::new(400.0, 0.0)
                );
                assert!(key_top_left(tester, &home_scaffold_key).dx() < 0.0);

                // Tapping on the "page" route doesn't trigger the GestureDetector because
                // it's being dragged.
                tap_key(tester, &page_scaffold_key);
                assert_eq!(home_tap_count.get(), 1);
                assert_eq!(page_tap_count.get(), 1);

                // Tapping the "page" route's back button doesn't do anything either.
                let back_button = tester
                    .find_by_type::<IconButton>()
                    .into_iter()
                    .find(|button| {
                        tester.widget(*button, |button: &IconButton| {
                            button.tooltip.as_deref() == Some("Back")
                        })
                    })
                    .expect("the back button");
                let at = tester
                    .global_rect(tester.render_box_of(back_button))
                    .center();
                tester.tap(at, 1);
                tester.pump_and_settle();
                assert_eq!(
                    key_top_left(tester, &page_scaffold_key),
                    Offset::new(400.0, 0.0)
                );
                assert!(key_top_left(tester, &home_scaffold_key).dx() < 0.0);
            });
        }
    }

    #[test]
    fn after_a_pop_caused_by_a_back_swipe_input_reaches_the_exposed_route() {
        for platform in [TargetPlatform::IOS, TargetPlatform::MacOS] {
            test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                // Regression test for https://github.com/flutter/flutter/issues/41024

                let home_scaffold_key = GlobalKey::new();
                let page_scaffold_key = GlobalKey::new();
                let (home_tap_count, page_tap_count) =
                    tap_counting_pages(tester, &home_scaffold_key);

                tap_key(tester, &home_scaffold_key);
                assert_eq!(home_tap_count.get(), 1);
                assert_eq!(page_tap_count.get(), 0);

                let notifier = {
                    let mut app = tester.cell.borrow_mut();
                    let context = home_scaffold_key.current_context(&app).expect("mounted");
                    Navigator::of(&mut app, context, false).user_gesture_in_progress_notifier(&app)
                };
                assert!(!*tester.cell.borrow().get(notifier).value());

                push_tap_counting_page(
                    tester,
                    &home_scaffold_key,
                    &page_scaffold_key,
                    &page_tap_count,
                );

                tester.pump_and_settle();
                tap_key(tester, &page_scaffold_key);
                assert_eq!(home_tap_count.get(), 1);
                assert_eq!(page_tap_count.get(), 1);

                // Trigger the basic iOS back-swipe dismiss transition. Drag the pushed
                // "page" route more than halfway across the screen and then release it.

                let mut gesture = TestPointer::touch(2);
                tester.down(&mut gesture, Offset::new(5.0, 300.0));
                let to = gesture.position + Offset::new(500.0, 0.0);
                tester.move_to(&mut gesture, to, true);
                tester.pump();
                assert_eq!(
                    key_top_left(tester, &page_scaffold_key),
                    Offset::new(500.0, 0.0)
                );
                assert!(key_top_left(tester, &home_scaffold_key).dx() < 0.0);
                assert!(*tester.cell.borrow().get(notifier).value());
                tester.up(&gesture);
                tester.pump_and_settle();
                assert!(!*tester.cell.borrow().get(notifier).value());
                assert!(
                    page_scaffold_key
                        .current_context(&tester.cell.borrow_mut())
                        .is_none()
                );

                // The back-swipe dismiss pop transition has finished and input on the
                // home page still works.
                tap_key(tester, &home_scaffold_key);
                assert_eq!(home_tap_count.get(), 2);
                assert_eq!(page_tap_count.get(), 1);
            });
        }
    }

    #[test]
    fn material_page_does_not_lose_its_state_when_transitioning_out() {
        test_widgets(|tester| {
            let navigator_key = GlobalKey::new();
            // Dart's `KeepsStateTestWidget`.
            let subpage: Rc<RefCell<Option<String>>> =
                Rc::new(RefCell::new(Some("subpage".to_string())));
            let keeps_state = StatefulBuilder::new({
                let navigator_key = navigator_key.clone();
                move |_app, _context, set_state| {
                    let mut pages: Vec<PageRef> =
                        vec![Rc::new(MaterialPage::new(Text::new("home")))];
                    if let Some(subpage) = subpage.borrow().clone() {
                        pages.push(Rc::new(MaterialPage::new(Text::new(subpage))));
                    }
                    let subpage = Rc::clone(&subpage);
                    MaterialApp::new()
                        .home(
                            Navigator::new()
                                .key(navigator_key.clone().into())
                                .pages(pages)
                                .on_pop_page(move |app, route, result| {
                                    if !route.did_pop(app, result) {
                                        return false;
                                    }
                                    let subpage = Rc::clone(&subpage);
                                    set_state(app, &mut move |_app| {
                                        *subpage.borrow_mut() = None;
                                    });
                                    true
                                }),
                        )
                        .into_widget()
                }
            });
            tester.pump_widget(keeps_state.into_widget());
            assert_eq!(count_onstage(tester, "subpage"), 1);
            assert_eq!(count_onstage(tester, "home"), 0);

            {
                let mut app = tester.cell.borrow_mut();
                navigator_key
                    .current_state::<NavigatorState>(&app)
                    .expect("a navigator")
                    .pop(&mut app, None);
            }
            tester.pump();

            assert_eq!(count_onstage(tester, "subpage"), 1);
            assert_eq!(count_onstage(tester, "home"), 1);
        });
    }

    #[test]
    fn setting_material_page_route_request_focus_to_false_does_not_request_focus_on_the_page() {
        test_widgets(|tester| {
            let saved_context: Rc<Cell<Option<BuildContext>>> = Rc::default();
            const PAGE_TWO_TEXT: &str = "Page Two";
            let saved = Rc::clone(&saved_context);
            pump_app(tester, ThemeData::new, move |app| {
                app.home(waft_widgets::Builder::new(move |_app, context| {
                    saved.set(Some(context));
                    Container::new().into_widget()
                }))
            });
            tester.pump();

            // Check page two is not on the screen.
            assert_eq!(count_onstage(tester, PAGE_TWO_TEXT), 0);

            // Navigate to page two with text.
            let navigator = Navigator::of(
                &mut tester.cell.borrow_mut(),
                saved_context.get().expect("built"),
                false,
            );
            push(tester, |app| {
                (MaterialPageRoute::new(
                    app,
                    Rc::new(|_app, _context| Text::new(PAGE_TWO_TEXT).into_widget()),
                ))
                .into_erased()
            });
            tester.pump();
            tester.pump_by(Duration::from_millis(100));

            // The page two is showing and the text widget has focus.
            let text_on_page_two = the_text(tester, PAGE_TWO_TEXT);
            {
                let mut app = tester.cell.borrow_mut();
                let focus_scope_node = FocusScope::of(&mut app, text_on_page_two, false);
                assert!(focus_scope_node.has_focus(&mut app));
            }

            // Navigate back to page one.
            navigator.pop(&mut tester.cell.borrow_mut(), None);
            tester.pump();
            tester.pump_by(Duration::from_millis(100));

            // Navigate to page two again with requestFocus set to false.
            push(tester, |app| {
                (MaterialPageRoute::new(
                    app,
                    Rc::new(|_app, _context| Text::new(PAGE_TWO_TEXT).into_widget()),
                )
                .request_focus(app, false))
                .into_erased()
            });
            tester.pump();
            tester.pump_by(Duration::from_millis(100));

            // The page two is showing and the text widget is not focused.
            let text_on_page_two = the_text(tester, PAGE_TWO_TEXT);
            let mut app = tester.cell.borrow_mut();
            let focus_scope_node = FocusScope::of(&mut app, text_on_page_two, false);
            assert!(!focus_scope_node.has_focus(&mut app));
        });
    }

    /// The `SnapshotWidget` nearest above `text`: Dart's
    /// `find.ancestor(of: of, matching: find.byType(SnapshotWidget)).first`.
    fn snapshot_above(tester: &WidgetTester, text: &str) -> AnyElement {
        tester.ancestor_of_type::<SnapshotWidget>(the_text(tester, text))
    }

    /// Whether a `SnapshotWidget` above `text` is set to take a snapshot. Dart's
    /// `isSnapshotted` reads that from the two layers a snapshot leaves in the layer tree around
    /// the page; Waft's `SnapshotWidget` paints the child live (`OffsetLayer.toImageSync` is
    /// deferred), so the case reads the controllers, as page_transitions_theme's cases do.
    fn is_snapshotted(tester: &WidgetTester, text: &str) -> bool {
        let ancestors = tester.ancestors(the_text(tester, text));
        let app = tester.cell.borrow();
        ancestors.into_iter().any(|ancestor| {
            waft_widgets::downcast_widget::<SnapshotWidget>(&**ancestor.widget(&app))
                .is_some_and(|snapshot| snapshot.controller.allow_snapshotting(&app))
        })
    }

    fn count_layers<T: 'static>(tester: &WidgetTester, layers: &[AnyLayer]) -> usize {
        let app = tester.cell.borrow();
        layers
            .iter()
            .filter(|layer| app.handle::<T>(layer.id()).is_some())
            .count()
    }

    fn is_transitioning_without_snapshotting(tester: &WidgetTester, text: &str) -> bool {
        // When snapshotting is off, the OpacityLayer and TransformLayer will be
        // applied directly.
        let layers = tester.layer_list_of(snapshot_above(tester, text));
        !is_snapshotted(tester, text)
            && count_layers::<OpacityLayer>(tester, &layers) == 1
            && count_layers::<TransformLayer>(tester, &layers) == 1
    }

    #[test]
    fn test_page_transition_zoom_page_transition_with_rasterization_disables_snapshotting_for_enter_route()
     {
        test_widgets(|tester| {
            pump_app(tester, ThemeData::new, |app| {
                app.routes([
                    ("/1".to_string(), material_text("Page 1")),
                    ("/2".to_string(), material_text("Page 2")),
                ])
                .initial_route("/1")
                .builder(|app, context, child| {
                    let theme_data = Theme::of(app, context);
                    let mut builders = theme_data.page_transitions_theme.builders().clone();
                    builders.insert(
                        TargetPlatform::Android,
                        PageTransitionsBuilderRef::new(
                            ZoomPageTransitionsBuilder::new().allow_enter_route_snapshotting(false),
                        ),
                    );
                    let child = child.cloned().expect("the navigator");
                    Theme::new(
                        theme_data
                            .copy_with()
                            .page_transitions_theme(
                                PageTransitionsTheme::new().with_builders(builders),
                            )
                            .build(),
                        waft_widgets::Builder::new(move |_app, _context| child.clone()),
                    )
                    .into_widget()
                })
            });

            // Page 1 on top.
            assert!(!is_snapshotted(tester, "Page 1"));

            // Transitioning from page 1 to page 2.
            navigator(tester).push_named(&mut tester.cell.borrow_mut(), "/2", None);
            tester.pump();
            tester.pump_by(Duration::from_millis(50));

            assert!(is_snapshotted(tester, "Page 1"));
            assert!(is_transitioning_without_snapshotting(tester, "Page 2"));

            // Page 2 on top.
            tester.pump_and_settle();
            assert!(!is_snapshotted(tester, "Page 2"));

            // Transitioning back from page 2 to page 1.
            pop(tester);
            tester.pump();
            tester.pump_by(Duration::from_millis(50));

            assert!(is_transitioning_without_snapshotting(tester, "Page 1"));
            assert!(is_snapshotted(tester, "Page 2"));

            // Page 1 on top.
            tester.pump_and_settle();
            assert!(!is_snapshotted(tester, "Page 1"));
        });
    }
}
