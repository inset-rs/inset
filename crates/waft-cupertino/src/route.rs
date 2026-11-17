//! Flutter counterpart: `cupertino/route.dart`.
//!
//! `debugFillProperties` waits with diagnostics, and `DisplayFeatureSubScreen` with
//! `MediaQuery.displayFeatures`.

use std::any::{Any, TypeId};
use std::cell::OnceCell;
use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use waft_animation::{
    Animatable, Animation, AnimationController, AnimationRef, AnimationStatus,
    AnimationStatusListener, CurvedAnimation, Curves, Drive, ValueTween,
    WeakAnimationStatusListener,
};
use waft_embedder::{Canvas, Color, ImageFilter, Offset, Paint, Rect, TextDirection};
use waft_foundation::{
    App, CompleterFuture, Handle, Listener, OptionRetained, Ref, ValueNotifier, VoidCallback,
};
use waft_gestures::{
    DragEndDetails, DragGestureRecognizer, DragStartDetails, DragUpdateDetails,
    HorizontalDragGestureRecognizer, PointerDownEvent, RecognizerLeaf,
};
use waft_painting::{
    AlignmentGeometry, AnyColor, BoxPainter, Decoration, DecorationRef, ImageConfiguration,
};
use waft_physics::{Simulation, SpringDescription, SpringSimulation, Tolerance};
use waft_rendering::{HitTestBehavior, ImageFilterConfig, StackFit};
use waft_scheduler::{FrameCallback, SchedulerBinding};
use waft_widgets::{
    Align, AnyPageRoute, AnyRoute, AnyTransitionRoute, BuildContext, Builder,
    DecoratedBoxTransition, DecorationTween, DelegatedTransitionBuilder, Directionality,
    FadeTransition, FractionalTranslation, IntoWidget, KeyRef, LocalHistoryRoute,
    LocalHistoryRouteData, MediaQuery, ModalRoute, ModalRouteData, Navigator, NavigatorState,
    OverlayRoute, OverlayRouteData, Page, PageRef, PageRoute, PageRouteData,
    PageTransitionsBuilder, PopInvokedWithResultCallback, PopupRoute, Positioned,
    PredictiveBackRoute, RawDialogRoute, RawDialogRouteData, RawDialogRouteLeaf, Route, RouteData,
    RoutePageBuilder, RouteRef, RouteResult, RouteSettingsRef, RouteTransitionsBuilder,
    ScaleTransition, Semantics, SlideTransition, Stack, State, StateData, StatefulWidget,
    TransitionRoute, TransitionRouteData, WidgetBuilder, WidgetRef,
};

use crate::colors::{CupertinoColors, CupertinoDynamicColor};
use crate::interface_level::{CupertinoUserInterfaceLevel, CupertinoUserInterfaceLevelData};
use crate::localizations::CupertinoLocalizations;

const K_BACK_GESTURE_WIDTH: f64 = 20.0;
/// Screen widths per second.
const K_MIN_FLING_VELOCITY: f64 = 1.0;

/// The duration for a page to animate when the user releases it mid-swipe.
const K_DROPPED_SWIPE_PAGE_ANIMATION_DURATION: Duration = Duration::from_millis(350);

/// Barrier color used for a barrier visible during transitions for Cupertino page routes.
///
/// This barrier color is only used for full-screen page routes with `fullscreen_dialog: false`.
///
/// By default, `fullscreen_dialog` Cupertino route transitions have no `barrier_color`, and
/// [`CupertinoDialogRoute`]s and [`CupertinoModalPopupRoute`]s have a `barrier_color` defined
/// by [`K_CUPERTINO_MODAL_BARRIER_COLOR`].
///
/// A relatively rigorous eyeball estimation.
const K_CUPERTINO_PAGE_TRANSITION_BARRIER_COLOR: Color = Color::new(0x18000000);

const K_CUPERTINO_MODAL_BARRIER_COLOR_DYNAMIC: CupertinoDynamicColor =
    CupertinoDynamicColor::with_brightness(Color::new(0x33000000), Color::new(0x7A000000));

/// Barrier color for a Cupertino modal barrier.
///
/// Extracted from <https://developer.apple.com/design/resources/>.
pub const K_CUPERTINO_MODAL_BARRIER_COLOR: AnyColor =
    K_CUPERTINO_MODAL_BARRIER_COLOR_DYNAMIC.to_any();

/// The duration of the transition used when a modal popup is shown.
const K_MODAL_POPUP_TRANSITION_DURATION: Duration = Duration::from_millis(335);

/// Offset from offscreen to the right to fully on screen.
const K_RIGHT_MIDDLE_TWEEN: ValueTween<Offset> =
    ValueTween::new(Offset::new(1.0, 0.0), Offset::ZERO);

/// Offset from fully on screen to 1/3 offscreen to the left.
const K_MIDDLE_LEFT_TWEEN: ValueTween<Offset> =
    ValueTween::new(Offset::ZERO, Offset::new(-1.0 / 3.0, 0.0));

/// Offset from offscreen below to fully on screen.
const K_BOTTOM_UP_TWEEN: ValueTween<Offset> = ValueTween::new(Offset::new(0.0, 1.0), Offset::ZERO);

// ---------------------------------------------------------------------------------------------
// CupertinoRouteTransitionMixin

/// The fields Dart's `CupertinoRouteTransitionMixin` declares; every route that mixes it in
/// carries this bag under the field `cupertino_route_transition`.
pub struct CupertinoRouteTransitionMixinData {
    previous_title: Option<Ref<ValueNotifier<Option<String>>>>,
}

impl CupertinoRouteTransitionMixinData {
    /// The bag of a freshly created route.
    pub fn new() -> CupertinoRouteTransitionMixinData {
        CupertinoRouteTransitionMixinData {
            previous_title: None,
        }
    }
}

impl Default for CupertinoRouteTransitionMixinData {
    fn default() -> CupertinoRouteTransitionMixinData {
        CupertinoRouteTransitionMixinData::new()
    }
}

/// The accessors [`CupertinoRouteTransitionMixin`] asks for, for a struct whose bag is the
/// field `cupertino_route_transition`.
#[macro_export]
macro_rules! cupertino_route_transition_mixin_accessors {
    () => {
        fn cupertino_route_transition_data(
            self: ::waft_foundation::Handle<Self>,
            app: &::waft_foundation::App,
        ) -> &$crate::CupertinoRouteTransitionMixinData {
            &app.get(self).cupertino_route_transition
        }

        fn cupertino_route_transition_data_mut(
            self: ::waft_foundation::Handle<Self>,
            app: &mut ::waft_foundation::App,
        ) -> &mut $crate::CupertinoRouteTransitionMixinData {
            &mut app.get_mut(self).cupertino_route_transition
        }
    };
}

/// A mixin that replaces the entire screen with an iOS transition for a [`PageRoute`].
///
/// The page slides in from the right and exits in reverse. The page also shifts to the left in
/// parallax when another page enters to cover it.
///
/// The page slides in from the bottom and exits in reverse with no parallax effect for
/// fullscreen dialogs.
///
/// See also:
///
///  * [`CupertinoPageRoute`], which is a [`PageRoute`] that leverages this mixin.
pub trait CupertinoRouteTransitionMixin: PageRoute {
    /// Dart's `CupertinoRouteTransitionMixin` fields, held under the field
    /// `cupertino_route_transition`
    /// ([`cupertino_route_transition_mixin_accessors!`](crate::cupertino_route_transition_mixin_accessors)).
    fn cupertino_route_transition_data(
        self: Handle<Self>,
        app: &App,
    ) -> &CupertinoRouteTransitionMixinData;

    /// See [`cupertino_route_transition_data`](Self::cupertino_route_transition_data).
    fn cupertino_route_transition_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut CupertinoRouteTransitionMixinData;

    /// The duration of the page transition.
    ///
    /// A relatively rigorous eyeball estimation. Dart's
    /// `CupertinoRouteTransitionMixin.kTransitionDuration`.
    const K_TRANSITION_DURATION: Duration = Duration::from_millis(500);

    /// Builds the primary contents of the route.
    fn build_content(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef;

    /// Whether this route introduces a route scope in the semantics tree.
    ///
    /// Defaults to true. When true, screen readers can treat pushes and pops of
    /// this route as navigation to a new screen and announce the change to users.
    ///
    /// Set this to false for routes that update only part of the screen, such as
    /// tab or shell content in a nested navigator. This prevents screen readers
    /// from treating the route as a new screen.
    fn include_route_semantics(self: Handle<Self>, app: &App) -> bool {
        let _ = app;
        true
    }

    /// A title string for this route.
    ///
    /// Used to auto-populate `CupertinoNavigationBar` and `CupertinoSliverNavigationBar`'s
    /// `middle` / `large_title` widgets when one is not manually supplied.
    fn title(self: Handle<Self>, app: &App) -> Option<String>;

    /// The title string of the previous [`CupertinoPageRoute`].
    ///
    /// The `ValueListenable`'s value is readable after the route is installed onto a
    /// [`Navigator`]. The `ValueListenable` will also notify its listeners if the value
    /// changes (such as by replacing the previous route).
    ///
    /// Its content value will be `None` if the previous route has no title or is not a
    /// [`CupertinoRouteTransitionMixin`].
    ///
    /// Panics before the route is installed.
    fn previous_title(self: Handle<Self>, app: &App) -> Handle<ValueNotifier<Option<String>>> {
        self.cupertino_route_transition_data(app)
            .previous_title
            .handle()
            .expect("Cannot read the previousTitle for a route that has not yet been installed")
    }

    /// Dart's `CupertinoRouteTransitionMixin.dispose`.
    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(previous_title) = self
            .cupertino_route_transition_data(app)
            .previous_title
            .handle()
        {
            previous_title.dispose(app);
        }
        TransitionRoute::dispose(self, app);
    }

    /// Dart's `CupertinoRouteTransitionMixin.didChangePrevious`.
    fn did_change_previous(self: Handle<Self>, app: &mut App, previous_route: Option<AnyRoute>) {
        let previous_title_string = previous_route
            .and_then(|previous_route| {
                previous_route.interface::<Rc<dyn CupertinoRouteTransition>>()
            })
            .and_then(|previous_route| previous_route.title(app));
        match self
            .cupertino_route_transition_data(app)
            .previous_title
            .handle()
        {
            None => {
                let previous_title = app.create(ValueNotifier::new(previous_title_string));
                self.cupertino_route_transition_data_mut(app).previous_title = Some(previous_title);
            }
            Some(previous_title) => previous_title.set_value(app, previous_title_string),
        }
        ModalRoute::did_change_previous(self, app, previous_route);
    }

    /// Dart's `CupertinoRouteTransitionMixin.transitionDuration`.
    fn transition_duration(self: Handle<Self>, app: &mut App) -> Duration {
        let _ = app;
        Self::K_TRANSITION_DURATION
    }

    /// Dart's `CupertinoRouteTransitionMixin.barrierColor`.
    fn barrier_color(self: Handle<Self>, app: &App) -> Option<Color> {
        (!PageRoute::fullscreen_dialog(self, app))
            .then_some(K_CUPERTINO_PAGE_TRANSITION_BARRIER_COLOR)
    }

    /// Dart's `CupertinoRouteTransitionMixin.barrierLabel`.
    fn barrier_label(self: Handle<Self>, app: &App) -> Option<String> {
        let _ = app;
        None
    }

    /// Dart's `CupertinoRouteTransitionMixin.canTransitionTo`.
    fn can_transition_to(self: Handle<Self>, app: &App, next_route: AnyTransitionRoute) -> bool {
        let next_route = next_route.as_route();
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

        // Otherwise if the next route has the same route transition mixin as this one, then
        // this route will already be synced with its transition.
        next_route_is_not_fullscreen
            && (next_route
                .interface::<Rc<dyn CupertinoRouteTransition>>()
                .is_some()
                || next_route_has_delegated_transition)
    }

    /// Dart's `CupertinoRouteTransitionMixin.canTransitionFrom`.
    fn can_transition_from(
        self: Handle<Self>,
        app: &App,
        previous_route: AnyTransitionRoute,
    ) -> bool {
        // Suppress previous route from transitioning if this is a fullscreenDialog route.
        previous_route.as_route().as_page_route(app).is_some()
            && !PageRoute::fullscreen_dialog(self, app)
    }

    /// Dart's `CupertinoRouteTransitionMixin.buildPage`.
    fn build_page(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        animation: AnimationRef<f64>,
        secondary_animation: AnimationRef<f64>,
    ) -> WidgetRef {
        let _ = (animation, secondary_animation);
        let child = self.build_content(app, context);
        if !self.include_route_semantics(app) {
            return child;
        }
        Semantics::new()
            .scopes_route(true)
            .explicit_child_nodes(true)
            .child(child)
            .into_widget()
    }

    /// Dart's `CupertinoRouteTransitionMixin.buildTransitions`.
    fn build_transitions(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        animation: AnimationRef<f64>,
        secondary_animation: AnimationRef<f64>,
        child: WidgetRef,
    ) -> WidgetRef {
        build_page_transitions(
            PageRoute::as_page_route(self),
            app,
            context,
            animation,
            secondary_animation,
            child,
        )
    }

    /// Dart's `route is CupertinoRouteTransitionMixin`, answered through `Route::interface`:
    /// this route as an `Rc<dyn CupertinoRouteTransition>` when `id` names that type.
    fn interface(self: Handle<Self>, id: TypeId) -> Option<Box<dyn Any>> {
        if id != TypeId::of::<Rc<dyn CupertinoRouteTransition>>() {
            return None;
        }
        let transition: Rc<dyn CupertinoRouteTransition> = Rc::new(self);
        Some(Box::new(transition))
    }
}

/// [`CupertinoRouteTransitionMixin`] on a shared reference: what Dart's
/// `route is CupertinoRouteTransitionMixin` produces, read as
/// `route.interface::<Rc<dyn CupertinoRouteTransition>>()` on an [`AnyRoute`]. Every route that
/// implements the mixin is one through its handle.
pub trait CupertinoRouteTransition {
    /// This route as the erased [`AnyRoute`].
    fn as_route(&self) -> AnyRoute;

    /// See [`CupertinoRouteTransitionMixin::title`].
    fn title(&self, app: &App) -> Option<String>;

    /// See [`CupertinoRouteTransitionMixin::previous_title`].
    fn previous_title(&self, app: &App) -> Handle<ValueNotifier<Option<String>>>;
}

impl<T: CupertinoRouteTransitionMixin> CupertinoRouteTransition for Handle<T> {
    fn as_route(&self) -> AnyRoute {
        Route::as_route(*self)
    }

    fn title(&self, app: &App) -> Option<String> {
        CupertinoRouteTransitionMixin::title(*self, app)
    }

    fn previous_title(&self, app: &App) -> Handle<ValueNotifier<Option<String>>> {
        CupertinoRouteTransitionMixin::previous_title(*self, app)
    }
}

/// Called by [`CupertinoBackGestureDetector`] when a pop ("back") drag start gesture is
/// detected. The returned controller handles all of the subsequent drag events.
fn start_pop_gesture(route: AnyPageRoute, app: &mut App) -> Ref<CupertinoBackGestureController> {
    let transition_route = route.as_modal_route().as_transition_route();
    debug_assert!(transition_route.pop_gesture_enabled(app));

    let route = route.as_route();
    let navigator = route.navigator(app).expect("an installed route");
    let controller = transition_route
        .controller(app)
        .expect("an installed route has a controller")
        .keep(app);
    // Dart's closures keep the route for as long as the gesture lasts.
    let kept_route = transition_route.keep(app);
    CupertinoBackGestureController::new(
        app,
        navigator,
        controller,
        Rc::new({
            let kept_route = kept_route.clone();
            move |app| kept_route.as_route().is_current(app)
        }),
        Rc::new(move |app| kept_route.as_route().is_active(app)),
    )
}

/// Returns a [`CupertinoFullscreenDialogTransition`] if `route` is a full screen dialog,
/// otherwise a [`CupertinoPageTransition`] is returned.
///
/// Used by [`CupertinoPageRoute`]'s `build_transitions`.
///
/// This function can be applied to any [`PageRoute`], not just [`CupertinoPageRoute`]. It's
/// typically used to provide a Cupertino style horizontal transition for material widgets when
/// the target platform is `TargetPlatform::IOS`.
///
/// See also:
///
///  * [`CupertinoPageTransitionsBuilder`], which uses this function to define a
///    `PageTransitionsBuilder` for the `PageTransitionsTheme`.
pub fn build_page_transitions(
    route: AnyPageRoute,
    app: &mut App,
    context: BuildContext,
    animation: AnimationRef<f64>,
    secondary_animation: AnimationRef<f64>,
    child: WidgetRef,
) -> WidgetRef {
    // Check if the route has an animation that's currently participating in a back swipe
    // gesture.
    //
    // In the middle of a back gesture drag, let the transition be linear to match finger
    // motions.
    let linear_transition = route.as_modal_route().pop_gesture_in_progress(app);
    let _ = context;
    if route.as_modal_route().fullscreen_dialog(app) {
        CupertinoFullscreenDialogTransition::new(
            animation,
            secondary_animation,
            child,
            linear_transition,
        )
        .into_widget()
    } else {
        let detector = CupertinoBackGestureDetector {
            enabled_callback: Rc::new(move |app| {
                route
                    .as_modal_route()
                    .as_transition_route()
                    .pop_gesture_enabled(app)
            }),
            on_start_pop_gesture: Rc::new(move |app| start_pop_gesture(route, app)),
            child,
        };
        CupertinoPageTransition::new(animation, secondary_animation, detector, linear_transition)
            .into_widget()
    }
}

/// The `Route` overrides a [`CupertinoRouteTransitionMixin`] leaf inherits: `waft-widgets`'
/// `page_route_overrides!` with `dispose` and `did_change_previous` pointed at the mixin.
/// Write it inside `impl Route for MyRoute { .. }`.
#[macro_export]
macro_rules! cupertino_route_transition_mixin_route_overrides {
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
            $crate::CupertinoRouteTransitionMixin::interface(self, id)
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
            ::waft_widgets::ModalRoute::did_push(self, app)
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
            ::waft_widgets::LocalHistoryRoute::did_pop(self, app, result)
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
            $crate::CupertinoRouteTransitionMixin::did_change_previous(self, app, previous_route)
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
            $crate::CupertinoRouteTransitionMixin::dispose(self, app)
        }
    };
}

/// The `TransitionRoute` overrides a [`CupertinoRouteTransitionMixin`] leaf inherits, plus
/// `waft-widgets`' `modal_transition_route_overrides!`. Write it inside
/// `impl TransitionRoute for MyRoute { .. }`, next to the leaf's own `opaque`.
#[macro_export]
macro_rules! cupertino_route_transition_mixin_transition_route_overrides {
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
            $crate::CupertinoRouteTransitionMixin::transition_duration(self, app)
        }

        fn can_transition_to(
            self: ::waft_foundation::Handle<Self>,
            app: &::waft_foundation::App,
            next_route: ::waft_widgets::AnyTransitionRoute,
        ) -> bool {
            $crate::CupertinoRouteTransitionMixin::can_transition_to(self, app, next_route)
        }

        fn can_transition_from(
            self: ::waft_foundation::Handle<Self>,
            app: &::waft_foundation::App,
            previous_route: ::waft_widgets::AnyTransitionRoute,
        ) -> bool {
            $crate::CupertinoRouteTransitionMixin::can_transition_from(self, app, previous_route)
        }
    };
}

/// The `ModalRoute` overrides a [`CupertinoRouteTransitionMixin`] leaf inherits, plus
/// `waft-widgets`' `modal_route_accessors!`. Write it inside
/// `impl ModalRoute for MyRoute { .. }`, next to the leaf's own `maintain_state`.
#[macro_export]
macro_rules! cupertino_route_transition_mixin_modal_route_overrides {
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
            $crate::CupertinoRouteTransitionMixin::barrier_color(self, app)
        }

        fn barrier_label(
            self: ::waft_foundation::Handle<Self>,
            app: &::waft_foundation::App,
        ) -> ::std::option::Option<::std::string::String> {
            $crate::CupertinoRouteTransitionMixin::barrier_label(self, app)
        }

        fn build_page(
            self: ::waft_foundation::Handle<Self>,
            app: &mut ::waft_foundation::App,
            context: ::waft_widgets::BuildContext,
            animation: ::waft_animation::AnimationRef<f64>,
            secondary_animation: ::waft_animation::AnimationRef<f64>,
        ) -> ::waft_widgets::WidgetRef {
            $crate::CupertinoRouteTransitionMixin::build_page(
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
            $crate::CupertinoRouteTransitionMixin::build_transitions(
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

// ---------------------------------------------------------------------------------------------
// CupertinoPageRoute

/// A modal route that replaces the entire screen with an iOS transition.
///
/// The page slides in from the right and exits in reverse. The page also shifts to the left in
/// parallax when another page enters to cover it.
///
/// The page slides in from the bottom and exits in reverse with no parallax effect for
/// fullscreen dialogs.
///
/// By default, when a modal route is replaced by another, the previous route remains in memory.
/// To free all the resources when this is not necessary, set
/// [`maintain_state`](Self::maintain_state) to false.
///
/// If `barrier_dismissible` is true, then pressing the escape key on the keyboard will cause the
/// current route to be popped with no value.
///
/// See also:
///
///  * [`CupertinoRouteTransitionMixin`], for a mixin that provides iOS transition for this
///    modal route.
///  * `CupertinoPageScaffold`, for applications that have one page with a fixed navigation bar
///    on top.
///  * [`CupertinoPage`], for a [`Page`] version of this class.
pub struct CupertinoPageRoute {
    route: RouteData,
    overlay_route: OverlayRouteData,
    transition_route: TransitionRouteData,
    local_history: LocalHistoryRouteData,
    modal_route: ModalRouteData,
    page_route: PageRouteData,
    cupertino_route_transition: CupertinoRouteTransitionMixinData,
    /// Builds the primary contents of the route.
    pub builder: WidgetBuilder,
    title: Option<String>,
    maintain_state: bool,
    include_route_semantics: bool,
}

impl CupertinoPageRoute {
    /// Creates a page route for use in an iOS designed app; Dart's optional arguments are the
    /// setters.
    ///
    /// Dart's defaults: `maintain_state` true, `fullscreen_dialog` false, `allow_snapshotting`
    /// true, `include_route_semantics` true, `barrier_dismissible` false.
    pub fn new(app: &mut App, builder: WidgetBuilder) -> Ref<CupertinoPageRoute> {
        let route = RouteData::new(app, None, None);
        let transition_route = TransitionRouteData::new(app);
        let modal_route = ModalRouteData::new(app);
        let this = app.create(CupertinoPageRoute {
            route,
            overlay_route: OverlayRouteData::new(),
            transition_route,
            local_history: LocalHistoryRouteData::new(),
            modal_route,
            page_route: PageRouteData::new(),
            cupertino_route_transition: CupertinoRouteTransitionMixinData::new(),
            builder,
            title: None,
            maintain_state: true,
            include_route_semantics: true,
        });
        debug_assert!(TransitionRoute::opaque(this.handle(), app));
        this
    }

    /// Dart `CupertinoPageRoute(title:)`.
    pub fn title(self: Ref<Self>, app: &mut App, title: String) -> Ref<Self> {
        app.get_mut(&self).title = Some(title);
        self
    }

    /// Dart `CupertinoPageRoute(settings:)`.
    pub fn settings(self: Ref<Self>, app: &mut App, settings: RouteSettingsRef) -> Ref<Self> {
        self.route_data_mut(app).set_settings(settings);
        self
    }

    /// Dart `CupertinoPageRoute(requestFocus:)`.
    pub fn request_focus(self: Ref<Self>, app: &mut App, request_focus: bool) -> Ref<Self> {
        self.route_data_mut(app)
            .set_request_focus(Some(request_focus));
        self
    }

    /// Dart `CupertinoPageRoute(maintainState:)`.
    pub fn maintain_state(self: Ref<Self>, app: &mut App, maintain_state: bool) -> Ref<Self> {
        app.get_mut(&self).maintain_state = maintain_state;
        self
    }

    /// Dart `CupertinoPageRoute(fullscreenDialog:)`.
    pub fn fullscreen_dialog(self: Ref<Self>, app: &mut App, fullscreen_dialog: bool) -> Ref<Self> {
        self.page_route_data_mut(app).fullscreen_dialog = fullscreen_dialog;
        self
    }

    /// Dart `CupertinoPageRoute(allowSnapshotting:)`.
    pub fn allow_snapshotting(
        self: Ref<Self>,
        app: &mut App,
        allow_snapshotting: bool,
    ) -> Ref<Self> {
        self.page_route_data_mut(app).allow_snapshotting = allow_snapshotting;
        self
    }

    /// Dart `CupertinoPageRoute(includeRouteSemantics:)`.
    pub fn include_route_semantics(
        self: Ref<Self>,
        app: &mut App,
        include_route_semantics: bool,
    ) -> Ref<Self> {
        app.get_mut(&self).include_route_semantics = include_route_semantics;
        self
    }

    /// Dart `CupertinoPageRoute(barrierDismissible:)`.
    pub fn barrier_dismissible(
        self: Ref<Self>,
        app: &mut App,
        barrier_dismissible: bool,
    ) -> Ref<Self> {
        self.page_route_data_mut(app).barrier_dismissible = barrier_dismissible;
        self
    }
}

impl Route for CupertinoPageRoute {
    crate::cupertino_route_transition_mixin_route_overrides!();
}

impl OverlayRoute for CupertinoPageRoute {
    waft_widgets::modal_overlay_route_overrides!();
}

impl PredictiveBackRoute for CupertinoPageRoute {
    waft_widgets::page_predictive_back_route_overrides!();
}

impl TransitionRoute for CupertinoPageRoute {
    crate::cupertino_route_transition_mixin_transition_route_overrides!();

    fn opaque(self: Handle<Self>, app: &App) -> bool {
        PageRoute::opaque(self, app)
    }
}

impl LocalHistoryRoute for CupertinoPageRoute {
    waft_widgets::local_history_route_accessors!();
}

impl ModalRoute for CupertinoPageRoute {
    crate::cupertino_route_transition_mixin_modal_route_overrides!();

    fn maintain_state(self: Handle<Self>, app: &App) -> bool {
        app.get(self).maintain_state
    }

    fn fullscreen_dialog(self: Handle<Self>, app: &App) -> bool {
        PageRoute::fullscreen_dialog(self, app)
    }

    fn delegated_transition(self: Handle<Self>, app: &App) -> Option<DelegatedTransitionBuilder> {
        let _ = app;
        Some(DelegatedTransitionBuilder::function(
            CupertinoPageTransition::delegated_transition,
        ))
    }
}

impl PageRoute for CupertinoPageRoute {
    waft_widgets::page_route_accessors!();
}

impl CupertinoRouteTransitionMixin for CupertinoPageRoute {
    crate::cupertino_route_transition_mixin_accessors!();

    fn build_content(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let builder = Rc::clone(&app.get(self).builder);
        builder(app, context)
    }

    fn include_route_semantics(self: Handle<Self>, app: &App) -> bool {
        app.get(self).include_route_semantics
    }

    fn title(self: Handle<Self>, app: &App) -> Option<String> {
        app.get(self).title.clone()
    }
}

// A page-based version of CupertinoPageRoute.
//
// This route uses the builder from the page to build its content. This ensures the content is
// up to date after page updates.
struct PageBasedCupertinoPageRoute {
    route: RouteData,
    overlay_route: OverlayRouteData,
    transition_route: TransitionRouteData,
    local_history: LocalHistoryRouteData,
    modal_route: ModalRouteData,
    page_route: PageRouteData,
    cupertino_route_transition: CupertinoRouteTransitionMixinData,
}

impl PageBasedCupertinoPageRoute {
    fn new(
        app: &mut App,
        page: PageRef,
        allow_snapshotting: bool,
    ) -> Ref<PageBasedCupertinoPageRoute> {
        let route = RouteData::new(app, Some(RouteSettingsRef::Page(page)), None);
        let transition_route = TransitionRouteData::new(app);
        let modal_route = ModalRouteData::new(app);
        let mut page_route = PageRouteData::new();
        page_route.allow_snapshotting = allow_snapshotting;
        let this = app.create(PageBasedCupertinoPageRoute {
            route,
            overlay_route: OverlayRouteData::new(),
            transition_route,
            local_history: LocalHistoryRouteData::new(),
            modal_route,
            page_route,
            cupertino_route_transition: CupertinoRouteTransitionMixinData::new(),
        });
        debug_assert!(TransitionRoute::opaque(this.handle(), app));
        this
    }

    /// Dart's `_page` getter, read in place.
    fn with_page<T>(self: Handle<Self>, app: &App, read: impl FnOnce(&CupertinoPage) -> T) -> T {
        match Route::settings(self, app) {
            RouteSettingsRef::Page(page) => read(
                page.as_any()
                    .downcast_ref::<CupertinoPage>()
                    .expect("a page-based Cupertino page route's settings are its CupertinoPage"),
            ),
            RouteSettingsRef::Settings(_) => {
                panic!("a page-based Cupertino page route's settings are its CupertinoPage")
            }
        }
    }
}

impl Route for PageBasedCupertinoPageRoute {
    crate::cupertino_route_transition_mixin_route_overrides!();
}

impl OverlayRoute for PageBasedCupertinoPageRoute {
    waft_widgets::modal_overlay_route_overrides!();
}

impl PredictiveBackRoute for PageBasedCupertinoPageRoute {
    waft_widgets::page_predictive_back_route_overrides!();
}

impl TransitionRoute for PageBasedCupertinoPageRoute {
    crate::cupertino_route_transition_mixin_transition_route_overrides!();

    fn opaque(self: Handle<Self>, app: &App) -> bool {
        PageRoute::opaque(self, app)
    }
}

impl LocalHistoryRoute for PageBasedCupertinoPageRoute {
    waft_widgets::local_history_route_accessors!();
}

impl ModalRoute for PageBasedCupertinoPageRoute {
    crate::cupertino_route_transition_mixin_modal_route_overrides!();

    fn maintain_state(self: Handle<Self>, app: &App) -> bool {
        self.with_page(app, |page| page.maintain_state)
    }

    fn fullscreen_dialog(self: Handle<Self>, app: &App) -> bool {
        PageRoute::fullscreen_dialog(self, app)
    }

    fn delegated_transition(self: Handle<Self>, app: &App) -> Option<DelegatedTransitionBuilder> {
        (!PageRoute::fullscreen_dialog(self, app)).then(|| {
            DelegatedTransitionBuilder::function(CupertinoPageTransition::delegated_transition)
        })
    }
}

impl PageRoute for PageBasedCupertinoPageRoute {
    waft_widgets::page_route_accessors!();

    fn fullscreen_dialog(self: Handle<Self>, app: &App) -> bool {
        self.with_page(app, |page| page.fullscreen_dialog)
    }
}

impl CupertinoRouteTransitionMixin for PageBasedCupertinoPageRoute {
    crate::cupertino_route_transition_mixin_accessors!();

    fn build_content(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let _ = context;
        self.with_page(app, |page| page.child.clone())
    }

    fn include_route_semantics(self: Handle<Self>, app: &App) -> bool {
        self.with_page(app, |page| page.include_route_semantics)
    }

    fn title(self: Handle<Self>, app: &App) -> Option<String> {
        self.with_page(app, |page| page.title.clone())
    }
}

// ---------------------------------------------------------------------------------------------
// CupertinoPage

/// A page that creates a cupertino style [`PageRoute`].
///
/// The page slides in from the right and exits in reverse. The page also shifts to the left in
/// parallax when another page enters to cover it.
///
/// The page slides in from the bottom and exits in reverse with no parallax effect for
/// fullscreen dialogs.
///
/// By default, when a created modal route is replaced by another, the previous route remains in
/// memory. To free all the resources when this is not necessary, set
/// [`maintain_state`](Self::maintain_state) to false.
///
/// See also:
///
///  * [`CupertinoPageRoute`], for a [`PageRoute`] version of this class.
#[derive(Clone)]
pub struct CupertinoPage {
    /// See [`Page::key`].
    pub key: Option<KeyRef>,

    /// See [`Page::name`].
    pub name: Option<String>,

    /// See [`Page::arguments`].
    pub arguments: Option<Rc<dyn Any>>,

    /// See [`Page::restoration_id`].
    pub restoration_id: Option<String>,

    /// See [`Page::can_pop`].
    pub can_pop: bool,

    /// See [`Page::on_pop_invoked`].
    pub on_pop_invoked: Option<PopInvokedWithResultCallback>,

    /// The content to be shown in the [`Route`] created by this page.
    pub child: WidgetRef,

    /// A title string for this route.
    pub title: Option<String>,

    /// Whether the route should remain in memory when it is inactive.
    pub maintain_state: bool,

    /// Whether this page route is a full-screen dialog.
    pub fullscreen_dialog: bool,

    /// Whether the route transition will prefer to animate a snapshot of the entering and
    /// exiting routes.
    pub allow_snapshotting: bool,

    /// Whether this route introduces a route scope in the semantics tree.
    ///
    /// See [`CupertinoRouteTransitionMixin::include_route_semantics`].
    pub include_route_semantics: bool,
}

impl CupertinoPage {
    /// Creates a cupertino page; Dart's optional arguments are the setters.
    ///
    /// Dart's defaults: `maintain_state` true, `fullscreen_dialog` false, `allow_snapshotting`
    /// true, `include_route_semantics` true, `can_pop` true.
    pub fn new<K>(child: impl IntoWidget<K>) -> CupertinoPage {
        CupertinoPage {
            key: None,
            name: None,
            arguments: None,
            restoration_id: None,
            can_pop: true,
            on_pop_invoked: None,
            child: child.into_widget(),
            title: None,
            maintain_state: true,
            fullscreen_dialog: false,
            allow_snapshotting: true,
            include_route_semantics: true,
        }
    }

    /// Dart `CupertinoPage(key:)`.
    pub fn key(mut self, key: KeyRef) -> CupertinoPage {
        self.key = Some(key);
        self
    }

    /// Dart `CupertinoPage(name:)`.
    pub fn name(mut self, name: String) -> CupertinoPage {
        self.name = Some(name);
        self
    }

    /// Dart `CupertinoPage(arguments:)`.
    pub fn arguments(mut self, arguments: Rc<dyn Any>) -> CupertinoPage {
        self.arguments = Some(arguments);
        self
    }

    /// Dart `CupertinoPage(restorationId:)`.
    pub fn restoration_id(mut self, restoration_id: String) -> CupertinoPage {
        self.restoration_id = Some(restoration_id);
        self
    }

    /// Dart `CupertinoPage(canPop:)`.
    pub fn can_pop(mut self, can_pop: bool) -> CupertinoPage {
        self.can_pop = can_pop;
        self
    }

    /// Dart `CupertinoPage(onPopInvoked:)`.
    pub fn on_pop_invoked(mut self, on_pop_invoked: PopInvokedWithResultCallback) -> CupertinoPage {
        self.on_pop_invoked = Some(on_pop_invoked);
        self
    }

    /// Dart `CupertinoPage(title:)`.
    pub fn title(mut self, title: String) -> CupertinoPage {
        self.title = Some(title);
        self
    }

    /// Dart `CupertinoPage(maintainState:)`.
    pub fn maintain_state(mut self, maintain_state: bool) -> CupertinoPage {
        self.maintain_state = maintain_state;
        self
    }

    /// Dart `CupertinoPage(fullscreenDialog:)`.
    pub fn fullscreen_dialog(mut self, fullscreen_dialog: bool) -> CupertinoPage {
        self.fullscreen_dialog = fullscreen_dialog;
        self
    }

    /// Dart `CupertinoPage(allowSnapshotting:)`.
    pub fn allow_snapshotting(mut self, allow_snapshotting: bool) -> CupertinoPage {
        self.allow_snapshotting = allow_snapshotting;
        self
    }

    /// Dart `CupertinoPage(includeRouteSemantics:)`.
    pub fn include_route_semantics(mut self, include_route_semantics: bool) -> CupertinoPage {
        self.include_route_semantics = include_route_semantics;
        self
    }
}

impl Debug for CupertinoPage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CupertinoPage")
            .field("name", &self.name)
            .field("title", &self.title)
            .field("child", &self.child)
            .finish_non_exhaustive()
    }
}

impl Page for CupertinoPage {
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
        PageBasedCupertinoPageRoute::new(app, Rc::clone(this), self.allow_snapshotting)
            .into_erased()
    }
}

// ---------------------------------------------------------------------------------------------
// CupertinoPageTransition

/// Provides an iOS-style page transition animation.
///
/// The page slides in from the right and exits in reverse. It also shifts to the left in a
/// parallax motion when another page enters to cover it.
#[derive(Debug)]
pub struct CupertinoPageTransition {
    /// See `Widget::key`.
    pub key: Option<KeyRef>,

    /// A linear route animation from 0.0 to 1.0 when this screen is being pushed.
    pub primary_route_animation: AnimationRef<f64>,

    /// A linear route animation from 0.0 to 1.0 when another screen is being pushed on top of
    /// this one.
    pub secondary_route_animation: AnimationRef<f64>,

    /// The widget below this widget in the tree.
    pub child: WidgetRef,

    /// Whether to perform the transitions linearly.
    ///
    /// Used to precisely track back gesture drags.
    pub linear_transition: bool,
}

impl CupertinoPageTransition {
    /// Creates an iOS-style page transition.
    pub fn new<K>(
        primary_route_animation: AnimationRef<f64>,
        secondary_route_animation: AnimationRef<f64>,
        child: impl IntoWidget<K>,
        linear_transition: bool,
    ) -> CupertinoPageTransition {
        CupertinoPageTransition {
            key: None,
            primary_route_animation,
            secondary_route_animation,
            child: child.into_widget(),
            linear_transition,
        }
    }

    /// Dart `CupertinoPageTransition(key:)`.
    pub fn key(mut self, key: KeyRef) -> CupertinoPageTransition {
        self.key = Some(key);
        self
    }

    /// The Cupertino styled `DelegatedTransitionBuilder` provided to the previous route.
    pub fn delegated_transition(
        app: &mut App,
        context: BuildContext,
        animation: AnimationRef<f64>,
        secondary_animation: AnimationRef<f64>,
        allow_snapshotting: bool,
        child: Option<&WidgetRef>,
    ) -> Option<WidgetRef> {
        let _ = (animation, allow_snapshotting);
        let animation = CurvedAnimation::new(
            app,
            secondary_animation,
            Curves::linear_to_ease_out(),
            Some(Curves::ease_in_to_linear()),
        );
        let delegated_position_animation = animation.drive(app, K_MIDDLE_LEFT_TWEEN);
        animation.dispose(app);

        let text_direction = Directionality::of(app, context);
        let mut transition = SlideTransition::new(delegated_position_animation)
            .text_direction(text_direction)
            .transform_hit_tests(false);
        if let Some(child) = child {
            transition = transition.child(child.clone());
        }
        Some(transition.into_widget())
    }
}

impl StatefulWidget for CupertinoPageTransition {
    type State = CupertinoPageTransitionState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> CupertinoPageTransitionState {
        CupertinoPageTransitionState {
            state: StateData::new(),
            primary_position_animation: None,
            secondary_position_animation: None,
            primary_shadow_animation: None,
            primary_position_curve: None,
            secondary_position_curve: None,
            primary_shadow_curve: None,
        }
    }
}

/// Dart's `_CupertinoPageTransitionState`.
pub struct CupertinoPageTransitionState {
    state: StateData<CupertinoPageTransition>,
    // When this page is coming in to cover another page.
    primary_position_animation: Option<AnimationRef<Offset>>,
    // When this page is becoming covered by another page.
    secondary_position_animation: Option<AnimationRef<Offset>>,
    // Shadow of page which is coming in to cover another page.
    primary_shadow_animation: Option<AnimationRef<DecorationRef>>,
    // Curve of primary page which is coming in to cover another page.
    primary_position_curve: Option<Ref<CurvedAnimation>>,
    // Curve of secondary page which is becoming covered by another page.
    secondary_position_curve: Option<Ref<CurvedAnimation>>,
    // Curve of primary page's shadow.
    primary_shadow_curve: Option<Ref<CurvedAnimation>>,
}

impl CupertinoPageTransitionState {
    fn dispose_curve(self: Handle<Self>, app: &mut App) {
        let state = app.get_mut(self);
        let curves = [
            state.primary_position_curve.take(),
            state.secondary_position_curve.take(),
            state.primary_shadow_curve.take(),
        ];
        for curve in curves.into_iter().flatten() {
            curve.dispose(app);
        }
    }

    fn setup_animation(self: Handle<Self>, app: &mut App) {
        let widget = self.widget(app);
        let primary_route_animation = widget.primary_route_animation.clone();
        let secondary_route_animation = widget.secondary_route_animation.clone();
        if !widget.linear_transition {
            let primary_position_curve = CurvedAnimation::new(
                app,
                primary_route_animation.clone(),
                Curves::fast_ease_in_to_slow_ease_out(),
                Some(Rc::new(Curves::fast_ease_in_to_slow_ease_out().flipped())),
            );
            let secondary_position_curve = CurvedAnimation::new(
                app,
                secondary_route_animation.clone(),
                Curves::linear_to_ease_out(),
                Some(Curves::ease_in_to_linear()),
            );
            let primary_shadow_curve = CurvedAnimation::new(
                app,
                primary_route_animation.clone(),
                Curves::linear_to_ease_out(),
                None,
            );
            let state = app.get_mut(self);
            state.primary_position_curve = Some(primary_position_curve);
            state.secondary_position_curve = Some(secondary_position_curve);
            state.primary_shadow_curve = Some(primary_shadow_curve);
        }
        let primary_position_parent = match &app.get(self).primary_position_curve {
            Some(curve) => curve.erase(),
            None => primary_route_animation.clone(),
        };
        let primary_position_animation = primary_position_parent.drive(app, K_RIGHT_MIDDLE_TWEEN);
        let secondary_position_parent = match &app.get(self).secondary_position_curve {
            Some(curve) => curve.erase(),
            None => secondary_route_animation.clone(),
        };
        let secondary_position_animation =
            secondary_position_parent.drive(app, K_MIDDLE_LEFT_TWEEN);
        let primary_shadow_parent = match &app.get(self).primary_shadow_curve {
            Some(curve) => curve.erase(),
            None => primary_route_animation.clone(),
        };
        let primary_shadow_animation =
            primary_shadow_parent.drive(app, CupertinoEdgeShadowDecoration::k_tween());
        let state = app.get_mut(self);
        state.primary_position_animation = Some(primary_position_animation);
        state.secondary_position_animation = Some(secondary_position_animation);
        state.primary_shadow_animation = Some(primary_shadow_animation);
    }
}

impl State for CupertinoPageTransitionState {
    type Widget = CupertinoPageTransition;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        self.setup_animation(app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &CupertinoPageTransition) {
        let widget = self.widget(app);
        let changed = old_widget.primary_route_animation != widget.primary_route_animation
            || old_widget.secondary_route_animation != widget.secondary_route_animation
            || old_widget.linear_transition != widget.linear_transition;
        if changed {
            self.dispose_curve(app);
            self.setup_animation(app);
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.dispose_curve(app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let text_direction = Directionality::of(app, context);
        let state = app.get(self);
        let secondary_position_animation = state
            .secondary_position_animation
            .clone()
            .expect("init_state ran first");
        let primary_position_animation = state
            .primary_position_animation
            .clone()
            .expect("init_state ran first");
        let primary_shadow_animation = state
            .primary_shadow_animation
            .clone()
            .expect("init_state ran first");
        let child = self.widget(app).child.clone();
        SlideTransition::new(secondary_position_animation)
            .text_direction(text_direction)
            .transform_hit_tests(false)
            .child(
                SlideTransition::new(primary_position_animation)
                    .text_direction(text_direction)
                    .child(DecoratedBoxTransition::new(primary_shadow_animation, child)),
            )
            .into_widget()
    }
}

// ---------------------------------------------------------------------------------------------
// CupertinoFullscreenDialogTransition

/// An iOS-style transition used for summoning fullscreen dialogs.
///
/// For example, used when creating a new calendar event by bringing in the next screen from the
/// bottom.
#[derive(Debug)]
pub struct CupertinoFullscreenDialogTransition {
    /// See `Widget::key`.
    pub key: Option<KeyRef>,

    /// A linear route animation from 0.0 to 1.0 when this screen is being pushed.
    pub primary_route_animation: AnimationRef<f64>,

    /// A linear route animation from 0.0 to 1.0 when another screen is being pushed on top of
    /// this one.
    pub secondary_route_animation: AnimationRef<f64>,

    /// The widget below this widget in the tree.
    pub child: WidgetRef,

    /// Whether to perform the transitions linearly.
    ///
    /// Used to precisely track back gesture drags.
    pub linear_transition: bool,
}

impl CupertinoFullscreenDialogTransition {
    /// Creates an iOS-style transition used for summoning fullscreen dialogs.
    pub fn new<K>(
        primary_route_animation: AnimationRef<f64>,
        secondary_route_animation: AnimationRef<f64>,
        child: impl IntoWidget<K>,
        linear_transition: bool,
    ) -> CupertinoFullscreenDialogTransition {
        CupertinoFullscreenDialogTransition {
            key: None,
            primary_route_animation,
            secondary_route_animation,
            child: child.into_widget(),
            linear_transition,
        }
    }

    /// Dart `CupertinoFullscreenDialogTransition(key:)`.
    pub fn key(mut self, key: KeyRef) -> CupertinoFullscreenDialogTransition {
        self.key = Some(key);
        self
    }
}

impl StatefulWidget for CupertinoFullscreenDialogTransition {
    type State = CupertinoFullscreenDialogTransitionState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> CupertinoFullscreenDialogTransitionState {
        CupertinoFullscreenDialogTransitionState {
            state: StateData::new(),
            primary_position_animation: None,
            secondary_position_animation: None,
            primary_position_curve: None,
            secondary_position_curve: None,
        }
    }
}

/// Dart's `_CupertinoFullscreenDialogTransitionState`.
pub struct CupertinoFullscreenDialogTransitionState {
    state: StateData<CupertinoFullscreenDialogTransition>,
    /// When this page is coming in to cover another page.
    primary_position_animation: Option<AnimationRef<Offset>>,
    /// When this page is becoming covered by another page.
    secondary_position_animation: Option<AnimationRef<Offset>>,
    /// Curve of primary page which is coming in to cover another page.
    primary_position_curve: Option<Ref<CurvedAnimation>>,
    /// Curve of secondary page which is becoming covered by another page.
    secondary_position_curve: Option<Ref<CurvedAnimation>>,
}

impl CupertinoFullscreenDialogTransitionState {
    fn dispose_curve(self: Handle<Self>, app: &mut App) {
        let state = app.get_mut(self);
        let curves = [
            state.primary_position_curve.take(),
            state.secondary_position_curve.take(),
        ];
        for curve in curves.into_iter().flatten() {
            curve.dispose(app);
        }
    }

    fn setup_animation(self: Handle<Self>, app: &mut App) {
        let widget = self.widget(app);
        let primary_route_animation = widget.primary_route_animation.clone();
        let secondary_route_animation = widget.secondary_route_animation.clone();
        let linear_transition = widget.linear_transition;
        let primary_position_curve = CurvedAnimation::new(
            app,
            primary_route_animation,
            Curves::linear_to_ease_out(),
            // The curve must be flipped so that the reverse animation doesn't play an ease-in
            // curve, which iOS does not use.
            Some(Rc::new(Curves::linear_to_ease_out().flipped())),
        );
        app.get_mut(self).primary_position_curve = Some(primary_position_curve.clone());
        let primary_position_animation = primary_position_curve.drive(app, K_BOTTOM_UP_TWEEN);
        let secondary_parent = if linear_transition {
            secondary_route_animation
        } else {
            let secondary_position_curve = CurvedAnimation::new(
                app,
                secondary_route_animation,
                Curves::linear_to_ease_out(),
                Some(Curves::ease_in_to_linear()),
            );
            app.get_mut(self).secondary_position_curve = Some(secondary_position_curve.clone());
            secondary_position_curve.into_erased()
        };
        let secondary_position_animation = secondary_parent.drive(app, K_MIDDLE_LEFT_TWEEN);
        let state = app.get_mut(self);
        state.primary_position_animation = Some(primary_position_animation);
        state.secondary_position_animation = Some(secondary_position_animation);
    }
}

impl State for CupertinoFullscreenDialogTransitionState {
    type Widget = CupertinoFullscreenDialogTransition;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        self.setup_animation(app);
    }

    fn did_update_widget(
        self: Handle<Self>,
        app: &mut App,
        old_widget: &CupertinoFullscreenDialogTransition,
    ) {
        let widget = self.widget(app);
        let changed = old_widget.primary_route_animation != widget.primary_route_animation
            || old_widget.secondary_route_animation != widget.secondary_route_animation
            || old_widget.linear_transition != widget.linear_transition;
        if changed {
            self.dispose_curve(app);
            self.setup_animation(app);
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.dispose_curve(app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let text_direction = Directionality::of(app, context);
        let state = app.get(self);
        let secondary_position_animation = state
            .secondary_position_animation
            .clone()
            .expect("init_state ran first");
        let primary_position_animation = state
            .primary_position_animation
            .clone()
            .expect("init_state ran first");
        let child = self.widget(app).child.clone();
        SlideTransition::new(secondary_position_animation)
            .text_direction(text_direction)
            .transform_hit_tests(false)
            .child(SlideTransition::new(primary_position_animation).child(child))
            .into_widget()
    }
}

// ---------------------------------------------------------------------------------------------
// The back gesture

/// Dart's `ValueGetter<bool>` on a route flag.
type RouteFlagGetter = Rc<dyn Fn(&App) -> bool>;

/// Dart's `ValueGetter<_CupertinoBackGestureController<T>>`.
type StartPopGestureCallback = Rc<dyn Fn(&mut App) -> Ref<CupertinoBackGestureController>>;

/// Dart's `ValueGetter<bool>` reading a route through the arena.
type BackGestureEnabledCallback = Rc<dyn Fn(&mut App) -> bool>;

/// This widget is the widget side of [`CupertinoBackGestureController`].
///
/// It provides a gesture recognizer which, when it determines the route can be closed with a
/// back gesture, creates the controller and feeds it the input from the gesture recognizer.
///
/// The gesture data is converted from absolute coordinates to logical coordinates by this
/// widget.
struct CupertinoBackGestureDetector {
    enabled_callback: BackGestureEnabledCallback,
    on_start_pop_gesture: StartPopGestureCallback,
    child: WidgetRef,
}

impl Debug for CupertinoBackGestureDetector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CupertinoBackGestureDetector")
            .field("child", &self.child)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for CupertinoBackGestureDetector {
    type State = CupertinoBackGestureDetectorState;

    fn create_state(&self) -> CupertinoBackGestureDetectorState {
        CupertinoBackGestureDetectorState {
            state: StateData::new(),
            back_gesture_controller: None,
            recognizer: None,
        }
    }
}

/// Dart's `_CupertinoBackGestureDetectorState`.
struct CupertinoBackGestureDetectorState {
    state: StateData<CupertinoBackGestureDetector>,
    back_gesture_controller: Option<Ref<CupertinoBackGestureController>>,
    recognizer: Option<Ref<HorizontalDragGestureRecognizer>>,
}

impl CupertinoBackGestureDetectorState {
    fn recognizer(self: Handle<Self>, app: &App) -> Handle<HorizontalDragGestureRecognizer> {
        app.get(self)
            .recognizer
            .handle()
            .expect("init_state ran first")
    }

    fn handle_drag_start(self: Handle<Self>, app: &mut App, _details: DragStartDetails) {
        debug_assert!(self.mounted(app));
        debug_assert!(app.get(self).back_gesture_controller.is_none());
        let on_start_pop_gesture = Rc::clone(&self.widget(app).on_start_pop_gesture);
        let controller = on_start_pop_gesture(app);
        app.get_mut(self).back_gesture_controller = Some(controller);
    }

    fn handle_drag_update(self: Handle<Self>, app: &mut App, details: DragUpdateDetails) {
        debug_assert!(self.mounted(app));
        let controller = app
            .get(self)
            .back_gesture_controller
            .handle()
            .expect("a drag start ran first");
        let width = self
            .context(app)
            .size(app)
            .expect("the detector is laid out")
            .width();
        let delta = self.convert_to_logical(
            app,
            details.primary_delta.expect("a horizontal drag") / width,
        );
        controller.drag_update(app, delta);
    }

    fn handle_drag_end(self: Handle<Self>, app: &mut App, details: DragEndDetails) {
        debug_assert!(self.mounted(app));
        let controller = app
            .get(self)
            .back_gesture_controller
            .handle()
            .expect("a drag start ran first");
        let width = self
            .context(app)
            .size(app)
            .expect("the detector is laid out")
            .width();
        let velocity =
            self.convert_to_logical(app, details.velocity.pixels_per_second.dx() / width);
        controller.drag_end(app, velocity);
        app.get_mut(self).back_gesture_controller = None;
    }

    fn handle_drag_cancel(self: Handle<Self>, app: &mut App) {
        debug_assert!(self.mounted(app));
        // This can be called even if start is not called, paired with the "down" event that we
        // don't consider here.
        if let Some(controller) = app.get(self).back_gesture_controller.handle() {
            controller.drag_end(app, 0.0);
        }
        app.get_mut(self).back_gesture_controller = None;
    }

    fn handle_pointer_down(self: Handle<Self>, app: &mut App, event: PointerDownEvent) {
        let enabled_callback = Rc::clone(&self.widget(app).enabled_callback);
        if enabled_callback(app) {
            self.recognizer(app).add_pointer(app, event);
        }
    }

    fn convert_to_logical(self: Handle<Self>, app: &mut App, value: f64) -> f64 {
        match Directionality::of(app, self.context(app)) {
            TextDirection::Rtl => -value,
            TextDirection::Ltr => value,
        }
    }
}

impl State for CupertinoBackGestureDetectorState {
    type Widget = CupertinoBackGestureDetector;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let recognizer = HorizontalDragGestureRecognizer::new(app);
        recognizer.set_on_start(
            app,
            Some(Rc::new(move |app: &mut App, details: DragStartDetails| {
                self.handle_drag_start(app, details);
            })),
        );
        recognizer.set_on_update(
            app,
            Some(Rc::new(move |app: &mut App, details: DragUpdateDetails| {
                self.handle_drag_update(app, details);
            })),
        );
        recognizer.set_on_end(
            app,
            Some(Rc::new(move |app: &mut App, details: DragEndDetails| {
                self.handle_drag_end(app, details);
            })),
        );
        recognizer.set_on_cancel(
            app,
            Some(Listener::new(move |app: &mut App| {
                self.handle_drag_cancel(app);
            })),
        );
        app.get_mut(self).recognizer = Some(recognizer);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let recognizer = self.recognizer(app);
        RecognizerLeaf::dispose(recognizer, app);

        // If this is disposed during a drag, call navigator.didStopUserGesture.
        if app.get(self).back_gesture_controller.is_some() {
            // The callback keeps the disposed state, as Dart's closure keeps `this`.
            let this = self.keep(app);
            SchedulerBinding::add_post_frame_callback(
                app,
                FrameCallback::new(move |app, _timestamp| {
                    let state = this.handle();
                    if let Some(controller) = app.get(state).back_gesture_controller.handle() {
                        let navigator = app.get(controller).navigator;
                        if navigator.mounted(app) {
                            navigator.did_stop_user_gesture(app);
                        }
                    }
                    app.get_mut(state).back_gesture_controller = None;
                }),
                None,
            );
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        // For devices with notches, the drag area needs to be larger on the side that has the
        // notch.
        let text_direction = Directionality::of(app, context);
        let padding = MediaQuery::padding_of(app, context);
        let drag_area_width = match text_direction {
            TextDirection::Rtl => padding.right,
            TextDirection::Ltr => padding.left,
        };
        let child = self.widget(app).child.clone();
        let listener = waft_widgets::Listener::new()
            .on_pointer_down(Rc::new(move |app: &mut App, event: PointerDownEvent| {
                self.handle_pointer_down(app, event);
            }))
            .behavior(HitTestBehavior::Translucent);
        Stack::new()
            .fit(StackFit::Passthrough)
            .children(vec![
                child,
                Positioned::directional(text_direction, Some(0.0), None, listener)
                    .width(drag_area_width.max(K_BACK_GESTURE_WIDTH))
                    .top(0.0)
                    .bottom(0.0)
                    .into_widget(),
            ])
            .into_widget()
    }
}

/// A controller for an iOS-style back gesture.
///
/// This is created by a [`CupertinoPageRoute`] in response from a gesture caught by a
/// [`CupertinoBackGestureDetector`] widget, which then also feeds it input from the gesture. It
/// controls the animation controller owned by the route, based on the input provided by the
/// gesture detector.
///
/// This object works entirely in logical coordinates (0.0 is new page dismissed, 1.0 is new
/// page on top).
struct CupertinoBackGestureController {
    controller: Ref<AnimationController>,
    navigator: Handle<NavigatorState>,
    get_is_active: RouteFlagGetter,
    get_is_current: RouteFlagGetter,
}

impl CupertinoBackGestureController {
    /// Creates a controller for an iOS-style back gesture.
    fn new(
        app: &mut App,
        navigator: Handle<NavigatorState>,
        controller: Ref<AnimationController>,
        get_is_current: RouteFlagGetter,
        get_is_active: RouteFlagGetter,
    ) -> Ref<CupertinoBackGestureController> {
        let this = app.create(CupertinoBackGestureController {
            controller,
            navigator,
            get_is_active,
            get_is_current,
        });
        navigator.did_start_user_gesture(app);
        this
    }

    /// The drag gesture has changed by `delta`. The total range of the drag should be 0.0 to
    /// 1.0.
    fn drag_update(self: Handle<Self>, app: &mut App, delta: f64) {
        let controller = app.get(self).controller.handle();
        let value = controller.value(app);
        controller.set_value(app, value - delta);
    }

    /// The drag gesture has ended with a horizontal motion of `velocity` as a fraction of
    /// screen width per second.
    fn drag_end(self: Handle<Self>, app: &mut App, velocity: f64) {
        // Fling in the appropriate direction.
        //
        // This curve has been determined through rigorously eyeballing native iOS animations.
        let animation_curve = Curves::fast_ease_in_to_slow_ease_out();
        let controller = app.get(self).controller.handle();
        let is_current = (Rc::clone(&app.get(self).get_is_current))(app);
        let animate_forward = if !is_current {
            // If the page has already been navigated away from, then the animation direction
            // depends on whether or not it's still in the navigation stack, regardless of
            // velocity or drag position. For example, if a route is being slowly dragged back
            // by just a few pixels, but then a programmatic pop occurs, the route should still
            // be animated off the screen.
            // See https://github.com/flutter/flutter/issues/141268.
            (Rc::clone(&app.get(self).get_is_active))(app)
        } else if velocity.abs() >= K_MIN_FLING_VELOCITY {
            // If the user releases the page before mid screen with sufficient velocity, or
            // after mid screen, we should animate the page out. Otherwise, the page should be
            // animated back in.
            velocity <= 0.0
        } else {
            controller.value(app) > 0.5
        };

        if animate_forward {
            controller.animate_to(
                app,
                1.0,
                Some(K_DROPPED_SWIPE_PAGE_ANIMATION_DURATION),
                Rc::clone(&animation_curve),
            );
        } else {
            if is_current {
                // This route is destined to pop at this point. Reuse navigator's pop.
                let navigator = app.get(self).navigator;
                navigator.pop(app, None);
            }

            // The popping may have finished inline if already at the target destination.
            if controller.is_animating(app) {
                controller.animate_back(
                    app,
                    0.0,
                    Some(K_DROPPED_SWIPE_PAGE_ANIMATION_DURATION),
                    Rc::clone(&animation_curve),
                );
            }
        }

        if controller.is_animating(app) {
            // Keep the userGestureInProgress in true state so we don't change the curve of the
            // page transition mid-flight since CupertinoPageTransition depends on
            // userGestureInProgress.
            let navigator = app.get(self).navigator;
            stop_user_gesture_when_the_animation_ends(app, navigator, controller);
        } else {
            let navigator = app.get(self).navigator;
            navigator.did_stop_user_gesture(app);
        }
    }
}

/// Dart's `animationStatusCallback` in `_CupertinoBackGestureController.dragEnd` and
/// `_CupertinoDragGestureController.dragEnd`: the first status change of `controller` ends the
/// user gesture on `navigator`, and the listener removes itself.
///
/// The listener names itself through a cell of its own weak handle, so nothing points back at it
/// strongly: it lives in the controller's list and goes with it. The weak handle is returned for
/// the tests.
pub(crate) fn stop_user_gesture_when_the_animation_ends(
    app: &mut App,
    navigator: Handle<NavigatorState>,
    controller: Handle<AnimationController>,
) -> WeakAnimationStatusListener {
    let me: Rc<OnceCell<WeakAnimationStatusListener>> = Rc::default();
    let listener = AnimationStatusListener::new({
        let me = Rc::clone(&me);
        move |_status, app| {
            navigator.did_stop_user_gesture(app);
            if let Some(listener) = me.get().and_then(WeakAnimationStatusListener::upgrade) {
                Animation::remove_status_listener(controller, app, &listener);
            }
        }
    });
    let weak = listener.downgrade();
    let _ = me.set(weak.clone());
    Animation::add_status_listener(controller, app, listener);
    weak
}

// ---------------------------------------------------------------------------------------------
// The edge shadow

/// A custom `Decoration` used to paint an extra shadow on the start edge of the box it's
/// decorating. It's like a `BoxDecoration` with only a gradient except it paints on the start
/// side of the box instead of behind the box.
#[derive(Clone, Debug, PartialEq)]
struct CupertinoEdgeShadowDecoration {
    // Colors used to paint a gradient at the start edge of the box it is decorating.
    //
    // The first color in the list is used at the start of the gradient, which is located at the
    // start edge of the decorated box.
    //
    // If this is `None`, no shadow is drawn.
    //
    // The list must have at least two colors in it (otherwise it would not be a gradient).
    colors: Option<Vec<Color>>,
}

impl CupertinoEdgeShadowDecoration {
    const fn new(colors: Option<Vec<Color>>) -> CupertinoEdgeShadowDecoration {
        CupertinoEdgeShadowDecoration { colors }
    }

    /// Dart's `static DecorationTween kTween`.
    fn k_tween() -> DecorationTween {
        DecorationTween::new(
            // No decoration initially.
            Some(DecorationRef::new(CupertinoEdgeShadowDecoration::new(None))),
            // Eyeballed gradient used to mimic a drop shadow on the start side only.
            Some(DecorationRef::new(CupertinoEdgeShadowDecoration::new(
                Some(vec![
                    Color::new(0x04000000),
                    CupertinoColors::TRANSPARENT.color(),
                ]),
            ))),
        )
    }

    /// Linearly interpolate between two edge shadow decorations.
    ///
    /// The `t` argument represents position on the timeline, with 0.0 meaning that the
    /// interpolation has not started, returning `a` (or something equivalent to `a`), 1.0
    /// meaning that the interpolation has finished, returning `b` (or something equivalent to
    /// `b`), and values in between meaning that the interpolation is at the relevant point on
    /// the timeline between `a` and `b`. The interpolation can be extrapolated beyond 0.0 and
    /// 1.0, so negative values and values greater than 1.0 are valid (and can easily be
    /// generated by curves such as `Curves::elastic_in_out`).
    fn lerp(
        a: Option<&CupertinoEdgeShadowDecoration>,
        b: Option<&CupertinoEdgeShadowDecoration>,
        t: f64,
    ) -> Option<CupertinoEdgeShadowDecoration> {
        if let (Some(a), Some(b)) = (a, b)
            && std::ptr::eq(a, b)
        {
            return Some(a.clone());
        }
        match (a, b) {
            (None, None) => None,
            (None, Some(b)) => Some(match &b.colors {
                None => b.clone(),
                Some(colors) => CupertinoEdgeShadowDecoration::new(Some(
                    colors
                        .iter()
                        .map(|color| Color::lerp(None, Some(*color), t).expect("one end is set"))
                        .collect(),
                )),
            }),
            (Some(a), None) => Some(match &a.colors {
                None => a.clone(),
                Some(colors) => CupertinoEdgeShadowDecoration::new(Some(
                    colors
                        .iter()
                        .map(|color| {
                            Color::lerp(None, Some(*color), 1.0 - t).expect("one end is set")
                        })
                        .collect(),
                )),
            }),
            (Some(a), Some(b)) => {
                debug_assert!(b.colors.is_some() || a.colors.is_some());
                // If it ever becomes necessary, we could allow decorations with different
                // lengths here, similarly to how it is handled in `LinearGradient::lerp`.
                debug_assert!(match (&a.colors, &b.colors) {
                    (Some(a), Some(b)) => a.len() == b.len(),
                    _ => true,
                });
                let b_colors = b.colors.as_ref().expect("the assert above");
                Some(CupertinoEdgeShadowDecoration::new(Some(
                    b_colors
                        .iter()
                        .enumerate()
                        .map(|(index, color)| {
                            let a_color = a
                                .colors
                                .as_ref()
                                .and_then(|colors| colors.get(index))
                                .copied();
                            Color::lerp(a_color, Some(*color), t).expect("one end is set")
                        })
                        .collect(),
                )))
            }
        }
    }
}

impl Decoration for CupertinoEdgeShadowDecoration {
    fn lerp_from(&self, a: Option<&dyn Decoration>, t: f64) -> Option<DecorationRef> {
        let a = a.and_then(|a| a.as_any().downcast_ref::<CupertinoEdgeShadowDecoration>());
        CupertinoEdgeShadowDecoration::lerp(a, Some(self), t).map(DecorationRef::new)
    }

    fn lerp_to(&self, b: Option<&dyn Decoration>, t: f64) -> Option<DecorationRef> {
        let b = b.and_then(|b| b.as_any().downcast_ref::<CupertinoEdgeShadowDecoration>());
        CupertinoEdgeShadowDecoration::lerp(Some(self), b, t).map(DecorationRef::new)
    }

    fn create_box_painter(&self, on_changed: Option<VoidCallback>) -> Box<dyn BoxPainter> {
        Box::new(CupertinoEdgeShadowPainter::new(self.clone(), on_changed))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn eq_decoration(&self, other: &dyn Decoration) -> bool {
        other
            .as_any()
            .downcast_ref::<CupertinoEdgeShadowDecoration>()
            .is_some_and(|other| other == self)
    }
}

/// A `BoxPainter` used to draw the page transition shadow using gradients.
struct CupertinoEdgeShadowPainter {
    decoration: CupertinoEdgeShadowDecoration,
    on_changed: Option<VoidCallback>,
}

impl CupertinoEdgeShadowPainter {
    fn new(
        decoration: CupertinoEdgeShadowDecoration,
        on_changed: Option<VoidCallback>,
    ) -> CupertinoEdgeShadowPainter {
        debug_assert!(
            decoration
                .colors
                .as_ref()
                .is_none_or(|colors| colors.len() > 1)
        );
        CupertinoEdgeShadowPainter {
            decoration,
            on_changed,
        }
    }
}

impl BoxPainter for CupertinoEdgeShadowPainter {
    fn paint(&mut self, canvas: &mut Canvas, offset: Offset, configuration: &ImageConfiguration) {
        let Some(colors) = self.decoration.colors.as_ref() else {
            return;
        };

        // The following code simulates drawing a `LinearGradient` configured as follows:
        //
        // LinearGradient(
        //   begin: AlignmentDirectional(0.90, 0.0), // Spans 5% of the page.
        //   colors: decoration.colors,
        // )
        //
        // A performance evaluation on Feb 8, 2021 showed that drawing the gradient manually as
        // implemented below is more performant than relying on `LinearGradient.createShader`
        // because compiling that shader takes a long time. On an iPhone XR, the implementation
        // below reduced the worst frame time for a cupertino page transition of a newly
        // installed app from ~95ms down to ~30ms, mainly because there's no longer a need to
        // compile a shader for the LinearGradient.
        //
        // The implementation below divides the width of the shadow into multiple bands of equal
        // width, one for each color interval defined by `decoration.colors`. Band x is filled
        // with a gradient going from `decoration.colors[x]` to `decoration.colors[x + 1]` by
        // drawing a bunch of 1px wide rects. The rects change their color by lerping between
        // the two colors that define the interval of the band.

        // Shadow spans 5% of the page.
        let size = configuration
            .size
            .expect("the ImageConfiguration must have a size");
        let shadow_width = 0.05 * size.width();
        let shadow_height = size.height();
        let band_width = shadow_width / (colors.len() - 1) as f64;

        let text_direction = configuration
            .text_direction
            .expect("the ImageConfiguration must have a text direction");
        let (shadow_direction, start) = match text_direction {
            TextDirection::Rtl => (1.0, offset.dx() + size.width()),
            TextDirection::Ltr => (-1.0, offset.dx()),
        };

        let mut band_color_index = 0usize;
        let mut dx = 0i64;
        while (dx as f64) < shadow_width {
            if (dx as f64 / band_width).trunc() as usize != band_color_index {
                band_color_index += 1;
            }
            let paint = Paint {
                color: Color::lerp(
                    Some(colors[band_color_index]),
                    Some(colors[band_color_index + 1]),
                    (dx as f64 % band_width) / band_width,
                )
                .expect("both ends are set")
                .into(),
                ..Paint::default()
            };
            let x = start + shadow_direction * dx as f64;
            canvas.draw_rect(
                Rect::from_ltwh(x - 1.0, offset.dy(), 1.0, shadow_height),
                &paint,
            );
            dx += 1;
        }
    }

    fn on_changed(&self) -> Option<&dyn Fn(&mut App)> {
        self.on_changed.as_deref()
    }
}

// ---------------------------------------------------------------------------------------------
// CupertinoModalPopupRoute

// The stiffness used by dialogs and action sheets.
//
// The stiffness value is obtained by examining the properties of `CASpringAnimation` in Xcode.
// The damping value is derived similarly, with additional precision calculated based on
// `K_STANDARD_STIFFNESS` to ensure a damping ratio of 1 (critically damped):
// damping = 2 * sqrt(stiffness)
const K_STANDARD_STIFFNESS: f64 = 522.35;
const K_STANDARD_DAMPING: f64 = 45.7099552;
const K_STANDARD_SPRING: SpringDescription =
    SpringDescription::new(1.0, K_STANDARD_STIFFNESS, K_STANDARD_DAMPING);
// The iOS spring animation duration is 0.404 seconds, based on the properties of
// `CASpringAnimation` in Xcode. At this point, the spring's position `x(0.404)` is
// approximately 0.9990000, suggesting that iOS uses a position tolerance of 1e-3 (matching the
// default tolerance).
//
// However, the spring's velocity `dx(0.404)` is about 0.02, indicating that iOS may not
// consider velocity when determining the animation's end condition. To account for this, a
// larger velocity tolerance is applied here for added safety.
const K_STANDARD_TOLERANCE: Tolerance = Tolerance::new(
    Tolerance::DEFAULT_TOLERANCE.distance,
    Tolerance::DEFAULT_TOLERANCE.time,
    0.03,
);

/// The spring simulation both a [`CupertinoModalPopupRoute`] and a [`CupertinoDialogRoute`]
/// drive their transition with.
fn create_standard_simulation<R: TransitionRoute>(
    this: Handle<R>,
    app: &mut App,
    forward: bool,
) -> Option<Box<dyn Simulation>> {
    debug_assert!(
        !TransitionRoute::debug_transition_completed(this, app),
        "Cannot reuse a route after disposing it."
    );
    let end = if forward { 1.0 } else { 0.0 };
    let controller = TransitionRoute::controller(this, app).expect("an installed route");
    let start = controller.value(app);
    Some(Box::new(SpringSimulation::with_options(
        K_STANDARD_SPRING,
        start,
        end,
        0.0,
        true,
        K_STANDARD_TOLERANCE,
    )))
}

/// A route that shows a modal iOS-style popup that slides up from the bottom of the screen.
///
/// Such a popup is an alternative to a menu or a dialog and prevents the user from interacting
/// with the rest of the app.
///
/// It is used internally by [`ShowCupertinoModalPopup`] or can be directly pushed onto the
/// [`Navigator`] stack to enable state restoration.
///
/// The `barrier_color` argument determines the color of the barrier underneath the popup. When
/// unspecified, the barrier color defaults to a light opacity black scrim based on iOS's dialog
/// screens. To correctly have iOS resolve to the appropriate modal colors, pass in
/// `CupertinoDynamicColor::resolve(&K_CUPERTINO_MODAL_BARRIER_COLOR, app, context)`.
///
/// The `barrier_dismissible` argument determines whether clicking outside the popup results in
/// dismissal. It is `true` by default.
///
/// The `semantics_dismissible` argument is used to determine whether the semantics of the modal
/// barrier are included in the semantics tree.
///
/// The `settings` argument is used to provide `RouteSettings` to the created route.
///
/// See also:
///
///  * `CupertinoActionSheet`, which is the widget usually returned by the
///    [`builder`](Self::builder) argument.
///  * <https://developer.apple.com/design/human-interface-guidelines/ios/views/action-sheets/>
pub struct CupertinoModalPopupRoute {
    route: RouteData,
    overlay_route: OverlayRouteData,
    transition_route: TransitionRouteData,
    local_history: LocalHistoryRouteData,
    modal_route: ModalRouteData,
    /// A builder that builds the widget tree for the [`CupertinoModalPopupRoute`].
    ///
    /// The builder typically builds a `CupertinoActionSheet` widget.
    ///
    /// Content below the widget is dimmed with a `ModalBarrier`. The widget built by the
    /// builder does not share a context with the route it was originally built from. Use a
    /// custom `StatefulWidget` if the widget needs to update dynamically.
    pub builder: WidgetBuilder,
    barrier_dismissible: bool,
    semantics_dismissible: bool,
    barrier_label: String,
    barrier_color: Option<AnyColor>,
}

impl CupertinoModalPopupRoute {
    /// Offset from offscreen below to fully on screen; Dart's `_offsetTween`.
    const OFFSET_TWEEN: ValueTween<Offset> = ValueTween::new(Offset::new(0.0, 1.0), Offset::ZERO);

    /// A route that shows a modal iOS-style popup that slides up from the bottom of the screen;
    /// Dart's optional arguments are the setters.
    ///
    /// Dart's defaults: `barrier_label` "Dismiss", `barrier_color`
    /// [`K_CUPERTINO_MODAL_BARRIER_COLOR`], `barrier_dismissible` true, `semantics_dismissible`
    /// false.
    pub fn new(app: &mut App, builder: WidgetBuilder) -> Ref<CupertinoModalPopupRoute> {
        let route = RouteData::new(app, None, None);
        let transition_route = TransitionRouteData::new(app);
        let modal_route = ModalRouteData::new(app);
        app.create(CupertinoModalPopupRoute {
            route,
            overlay_route: OverlayRouteData::new(),
            transition_route,
            local_history: LocalHistoryRouteData::new(),
            modal_route,
            builder,
            barrier_dismissible: true,
            semantics_dismissible: false,
            barrier_label: "Dismiss".to_string(),
            barrier_color: Some(K_CUPERTINO_MODAL_BARRIER_COLOR),
        })
    }

    /// Dart `CupertinoModalPopupRoute(barrierLabel:)`.
    pub fn barrier_label(self: Ref<Self>, app: &mut App, barrier_label: String) -> Ref<Self> {
        app.get_mut(&self).barrier_label = barrier_label;
        self
    }

    /// Dart `CupertinoModalPopupRoute(barrierColor:)`.
    pub fn barrier_color(
        self: Ref<Self>,
        app: &mut App,
        barrier_color: Option<AnyColor>,
    ) -> Ref<Self> {
        app.get_mut(&self).barrier_color = barrier_color;
        self
    }

    /// Dart `CupertinoModalPopupRoute(barrierDismissible:)`.
    pub fn barrier_dismissible(
        self: Ref<Self>,
        app: &mut App,
        barrier_dismissible: bool,
    ) -> Ref<Self> {
        app.get_mut(&self).barrier_dismissible = barrier_dismissible;
        self
    }

    /// Dart `CupertinoModalPopupRoute(semanticsDismissible:)`.
    pub fn semantics_dismissible(
        self: Ref<Self>,
        app: &mut App,
        semantics_dismissible: bool,
    ) -> Ref<Self> {
        app.get_mut(&self).semantics_dismissible = semantics_dismissible;
        self
    }

    /// Dart `CupertinoModalPopupRoute(filter:)`.
    pub fn filter(self: Ref<Self>, app: &mut App, filter: ImageFilter) -> Ref<Self> {
        self.modal_route_data_mut(app).filter = Some(ImageFilterConfig::new(filter));
        self
    }

    /// Dart `CupertinoModalPopupRoute(settings:)`.
    pub fn settings(self: Ref<Self>, app: &mut App, settings: RouteSettingsRef) -> Ref<Self> {
        self.route_data_mut(app).set_settings(settings);
        self
    }

    /// Dart `CupertinoModalPopupRoute(requestFocus:)`.
    pub fn request_focus(self: Ref<Self>, app: &mut App, request_focus: bool) -> Ref<Self> {
        self.route_data_mut(app)
            .set_request_focus(Some(request_focus));
        self
    }
}

impl Route for CupertinoModalPopupRoute {
    waft_widgets::modal_route_overrides!();
}

impl OverlayRoute for CupertinoModalPopupRoute {
    waft_widgets::modal_overlay_route_overrides!();
}

impl PredictiveBackRoute for CupertinoModalPopupRoute {
    waft_widgets::modal_predictive_back_route_overrides!(ModalRoute::pop_gesture_enabled);
}

impl TransitionRoute for CupertinoModalPopupRoute {
    waft_widgets::modal_transition_route_overrides!();

    fn transition_duration(self: Handle<Self>, app: &mut App) -> Duration {
        let _ = app;
        K_MODAL_POPUP_TRANSITION_DURATION
    }

    fn opaque(self: Handle<Self>, app: &App) -> bool {
        PopupRoute::opaque(self, app)
    }

    fn allow_snapshotting(self: Handle<Self>, app: &App) -> bool {
        PopupRoute::allow_snapshotting(self, app)
    }

    fn create_simulation(
        self: Handle<Self>,
        app: &mut App,
        forward: bool,
    ) -> Option<Box<dyn Simulation>> {
        create_standard_simulation(self, app, forward)
    }
}

impl LocalHistoryRoute for CupertinoModalPopupRoute {
    waft_widgets::local_history_route_accessors!();
}

impl ModalRoute for CupertinoModalPopupRoute {
    waft_widgets::modal_route_accessors!();

    fn barrier_dismissible(self: Handle<Self>, app: &App) -> bool {
        app.get(self).barrier_dismissible
    }

    fn semantics_dismissible(self: Handle<Self>, app: &App) -> bool {
        app.get(self).semantics_dismissible
    }

    fn barrier_label(self: Handle<Self>, app: &App) -> Option<String> {
        Some(app.get(self).barrier_label.clone())
    }

    fn barrier_color(self: Handle<Self>, app: &App) -> Option<Color> {
        app.get(self)
            .barrier_color
            .as_ref()
            .map(|barrier_color| barrier_color.color())
    }

    fn maintain_state(self: Handle<Self>, app: &App) -> bool {
        PopupRoute::maintain_state(self, app)
    }

    fn build_page(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        animation: AnimationRef<f64>,
        secondary_animation: AnimationRef<f64>,
    ) -> WidgetRef {
        let _ = (context, animation, secondary_animation);
        let builder = Rc::clone(&app.get(self).builder);
        CupertinoUserInterfaceLevel::new(
            CupertinoUserInterfaceLevelData::Elevated,
            Builder::new(move |app, context| builder(app, context)),
        )
        .into_widget()
    }

    fn build_transitions(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        animation: AnimationRef<f64>,
        secondary_animation: AnimationRef<f64>,
        child: WidgetRef,
    ) -> WidgetRef {
        let _ = (context, secondary_animation);
        let translation = CupertinoModalPopupRoute::OFFSET_TWEEN.evaluate(app, animation.handle());
        Align::new()
            .alignment(AlignmentGeometry::BOTTOM_CENTER)
            .child(FractionalTranslation::new(translation).child(child))
            .into_widget()
    }
}

impl PopupRoute for CupertinoModalPopupRoute {}

/// Shows a modal iOS-style popup that slides up from the bottom of the screen.
///
/// Such a popup is an alternative to a menu or a dialog and prevents the user from interacting
/// with the rest of the app.
///
/// The `context` argument is used to look up the [`Navigator`] for the popup. It is only used
/// when the function is called. Its corresponding widget can be safely removed from the tree
/// before the popup is closed.
///
/// The `barrier_color` argument determines the color of the barrier underneath the popup. When
/// unspecified, the barrier color defaults to a light opacity black scrim based on iOS's dialog
/// screens.
///
/// The `barrier_dismissible` argument determines whether clicking outside the popup results in
/// dismissal. It is `true` by default.
///
/// The `use_root_navigator` argument is used to determine whether to push the popup to the
/// [`Navigator`] furthest from or nearest to the given `context`. It is `true` by default.
///
/// The `semantics_dismissible` argument is used to determine whether the semantics of the modal
/// barrier are included in the semantics tree.
///
/// The `route_settings` argument is used to provide `RouteSettings` to the created route.
///
/// The `builder` argument typically builds a `CupertinoActionSheet` widget. Content below the
/// widget is dimmed with a `ModalBarrier`.
///
/// Returns a future that resolves to the value that was passed to [`Navigator::pop`] when the
/// popup was closed.
///
/// See also:
///
///  * `CupertinoActionSheet`, which is the widget usually returned by the `builder` argument.
///  * <https://developer.apple.com/design/human-interface-guidelines/ios/views/action-sheets/>
pub struct ShowCupertinoModalPopup {
    builder: WidgetBuilder,
    filter: Option<ImageFilter>,
    barrier_color: AnyColor,
    barrier_dismissible: bool,
    use_root_navigator: bool,
    semantics_dismissible: bool,
    route_settings: Option<RouteSettingsRef>,
    request_focus: Option<bool>,
}

impl ShowCupertinoModalPopup {
    /// Creates a builder for a Cupertino modal popup; Dart's `showCupertinoModalPopup` required
    /// argument is the `builder`, and every other argument is a setter.
    pub fn new(builder: WidgetBuilder) -> ShowCupertinoModalPopup {
        ShowCupertinoModalPopup {
            builder,
            filter: None,
            barrier_color: K_CUPERTINO_MODAL_BARRIER_COLOR,
            barrier_dismissible: true,
            use_root_navigator: true,
            semantics_dismissible: false,
            route_settings: None,
            request_focus: None,
        }
    }

    /// An image filter applied to the popup's barrier.
    pub fn filter(mut self, filter: ImageFilter) -> ShowCupertinoModalPopup {
        self.filter = Some(filter);
        self
    }

    /// The color of the barrier underneath the popup.
    pub fn barrier_color(mut self, barrier_color: impl Into<AnyColor>) -> ShowCupertinoModalPopup {
        self.barrier_color = barrier_color.into();
        self
    }

    /// Whether clicking outside the popup dismisses it. Defaults to `true`.
    pub fn barrier_dismissible(mut self, barrier_dismissible: bool) -> ShowCupertinoModalPopup {
        self.barrier_dismissible = barrier_dismissible;
        self
    }

    /// Whether to push the popup to the root navigator. Defaults to `true`.
    pub fn use_root_navigator(mut self, use_root_navigator: bool) -> ShowCupertinoModalPopup {
        self.use_root_navigator = use_root_navigator;
        self
    }

    /// Whether the modal barrier's semantics are included in the semantics tree. Defaults to
    /// `false`.
    pub fn semantics_dismissible(mut self, semantics_dismissible: bool) -> ShowCupertinoModalPopup {
        self.semantics_dismissible = semantics_dismissible;
        self
    }

    /// The route settings passed to the popup's route.
    pub fn route_settings(mut self, route_settings: RouteSettingsRef) -> ShowCupertinoModalPopup {
        self.route_settings = Some(route_settings);
        self
    }

    /// Whether the popup's route requests focus when it is pushed.
    pub fn request_focus(mut self, request_focus: bool) -> ShowCupertinoModalPopup {
        self.request_focus = Some(request_focus);
        self
    }

    /// Shows the popup and returns a future that resolves to the value passed to
    /// [`Navigator::pop`] when it closes.
    pub fn show(self, app: &mut App, context: BuildContext) -> CompleterFuture<RouteResult> {
        let ShowCupertinoModalPopup {
            builder,
            filter,
            barrier_color,
            barrier_dismissible,
            use_root_navigator,
            semantics_dismissible,
            route_settings,
            request_focus,
        } = self;

        let resolved_barrier_color = CupertinoDynamicColor::resolve(&barrier_color, app, context);
        let mut route = CupertinoModalPopupRoute::new(app, builder)
            .barrier_color(app, Some(resolved_barrier_color))
            .barrier_dismissible(app, barrier_dismissible)
            .semantics_dismissible(app, semantics_dismissible);
        if let Some(filter) = filter {
            route = route.filter(app, filter);
        }
        if let Some(route_settings) = route_settings {
            route = route.settings(app, route_settings);
        }
        if let Some(request_focus) = request_focus {
            route = route.request_focus(app, request_focus);
        }
        let navigator = Navigator::of(app, context, use_root_navigator);
        navigator.push(app, route.into_erased())
    }
}

// ---------------------------------------------------------------------------------------------
// CupertinoDialogRoute

/// Dart's `_buildCupertinoDialogTransitions`: a jump cut.
///
/// Dart hands it to `RawDialogRoute`'s constructor as the `transitionBuilder` to fall back on.
/// It sits in the bag until [`transition_builder`](CupertinoDialogRoute::transition_builder)
/// replaces it, and [`CupertinoDialogRoute::build_transitions`] reaches the base body only once
/// it has been replaced, so nothing ever runs it — as in Dart.
fn build_cupertino_dialog_transitions(
    _app: &mut App,
    _context: BuildContext,
    _animation: AnimationRef<f64>,
    _secondary_animation: AnimationRef<f64>,
    child: WidgetRef,
) -> WidgetRef {
    child
}

/// Displays an iOS-style dialog above the current contents of the app, with iOS-style entrance
/// and exit animations, modal barrier color, and modal barrier behavior (by default, the dialog
/// is not dismissible with a tap on the barrier).
///
/// This function takes a `builder` which typically builds a `CupertinoAlertDialog` widget.
/// Content below the dialog is dimmed with a `ModalBarrier`. The widget returned by the
/// `builder` does not share a context with the location that [`ShowCupertinoDialog`] is
/// originally called from.
///
/// The `context` argument is used to look up the [`Navigator`] for the dialog. It is only used
/// when the function is called. Its corresponding widget can be safely removed from the tree
/// before the dialog is closed.
///
/// The `use_root_navigator` argument is used to determine whether to push the dialog to the
/// [`Navigator`] furthest from or nearest to the given `context`.
///
/// Returns a future that resolves to the value (if any) that was passed to [`Navigator::pop`]
/// when the dialog was closed.
///
/// See also:
///
///  * `CupertinoAlertDialog`, an iOS-style alert dialog.
///  * `show_general_dialog`, which allows for customization of the dialog popup.
///  * <https://developer.apple.com/design/human-interface-guidelines/alerts/>
pub struct ShowCupertinoDialog {
    builder: WidgetBuilder,
    barrier_label: Option<String>,
    barrier_color: Option<Color>,
    use_root_navigator: bool,
    barrier_dismissible: bool,
    route_settings: Option<RouteSettingsRef>,
    request_focus: Option<bool>,
}

impl ShowCupertinoDialog {
    /// Creates a builder for an iOS-style dialog; Dart's `showCupertinoDialog` required
    /// argument is the `builder`, and every other argument is a setter.
    pub fn new(builder: WidgetBuilder) -> ShowCupertinoDialog {
        ShowCupertinoDialog {
            builder,
            barrier_label: None,
            barrier_color: None,
            use_root_navigator: true,
            barrier_dismissible: false,
            route_settings: None,
            request_focus: None,
        }
    }

    /// The semantic label of the dialog's modal barrier.
    pub fn barrier_label(mut self, barrier_label: impl Into<String>) -> ShowCupertinoDialog {
        self.barrier_label = Some(barrier_label.into());
        self
    }

    /// The color of the dialog's modal barrier.
    pub fn barrier_color(mut self, barrier_color: Color) -> ShowCupertinoDialog {
        self.barrier_color = Some(barrier_color);
        self
    }

    /// Whether to push the dialog to the root navigator. Defaults to `true`.
    pub fn use_root_navigator(mut self, use_root_navigator: bool) -> ShowCupertinoDialog {
        self.use_root_navigator = use_root_navigator;
        self
    }

    /// Whether tapping the barrier dismisses the dialog. Defaults to `false`.
    pub fn barrier_dismissible(mut self, barrier_dismissible: bool) -> ShowCupertinoDialog {
        self.barrier_dismissible = barrier_dismissible;
        self
    }

    /// The route settings passed to the dialog's route.
    pub fn route_settings(mut self, route_settings: RouteSettingsRef) -> ShowCupertinoDialog {
        self.route_settings = Some(route_settings);
        self
    }

    /// Whether the dialog's route requests focus when it is pushed.
    pub fn request_focus(mut self, request_focus: bool) -> ShowCupertinoDialog {
        self.request_focus = Some(request_focus);
        self
    }

    /// Shows the dialog and returns a future that resolves to the value passed to
    /// [`Navigator::pop`] when it closes.
    pub fn show(self, app: &mut App, context: BuildContext) -> CompleterFuture<RouteResult> {
        let ShowCupertinoDialog {
            builder,
            barrier_label,
            barrier_color,
            use_root_navigator,
            barrier_dismissible,
            route_settings,
            request_focus,
        } = self;

        let mut route = CupertinoDialogRoute::new(app, builder, context)
            .barrier_dismissible(app, barrier_dismissible);
        if let Some(barrier_label) = barrier_label {
            route = route.barrier_label(app, barrier_label);
        }
        if let Some(barrier_color) = barrier_color {
            route = route.barrier_color(app, Some(barrier_color));
        }
        if let Some(route_settings) = route_settings {
            route = route.settings(app, route_settings);
        }
        if let Some(request_focus) = request_focus {
            route = route.request_focus(app, request_focus);
        }
        let navigator = Navigator::of(app, context, use_root_navigator);
        navigator.push(app, route.into_erased())
    }
}

/// A dialog route that shows an iOS-style dialog.
///
/// It is used internally by [`ShowCupertinoDialog`] or can be directly pushed onto the
/// [`Navigator`] stack to enable state restoration.
///
/// This route takes a `builder` which typically builds a dialog widget. Content below the
/// dialog is dimmed with a `ModalBarrier`. The widget returned by the `builder` does not share
/// a context with the location the dialog is shown from.
///
/// The `context` argument is used to look up
/// [`CupertinoLocalizations::modal_barrier_dismiss_label`], which provides the modal with a
/// localized accessibility label that will be used for the modal's barrier. However, a custom
/// `barrier_label` can be passed in as well.
///
/// The `barrier_dismissible` argument is used to indicate whether tapping on the barrier will
/// dismiss the dialog.
///
/// The `barrier_color` argument is used to specify the color of the modal barrier that darkens
/// everything below the dialog. If unset, [`K_CUPERTINO_MODAL_BARRIER_COLOR`] resolved against
/// the context is used.
///
/// See also:
///
///  * [`ShowCupertinoDialog`], which is a way to display an iOS-style dialog.
///  * `show_general_dialog`, which allows for customization of the dialog popup.
pub struct CupertinoDialogRoute {
    route: RouteData,
    overlay_route: OverlayRouteData,
    transition_route: TransitionRouteData,
    local_history: LocalHistoryRouteData,
    modal_route: ModalRouteData,
    raw_dialog_route: RawDialogRouteData,
    /// Custom transition builder.
    pub transition_builder: Option<RouteTransitionsBuilder>,
}

impl CupertinoDialogRoute {
    // The curve and initial scale values were mostly eyeballed from iOS, however they reuse the
    // same animation curve that was modeled after native page transitions.
    const DIALOG_SCALE_TWEEN: ValueTween<f64> = ValueTween::new(1.3, 1.0);

    /// A dialog route that shows an iOS-style dialog; Dart's optional arguments are the
    /// setters.
    ///
    /// Dart's defaults: `barrier_dismissible` true, `transition_duration` 250 ms (eyeballed
    /// comparing with iOS), `barrier_label` the context's
    /// [`CupertinoLocalizations::modal_barrier_dismiss_label`], and `barrier_color`
    /// [`K_CUPERTINO_MODAL_BARRIER_COLOR`] resolved against the context.
    pub fn new(
        app: &mut App,
        builder: WidgetBuilder,
        context: BuildContext,
    ) -> Ref<CupertinoDialogRoute> {
        let barrier_label =
            <dyn CupertinoLocalizations>::of(app, context).modal_barrier_dismiss_label();
        let barrier_color =
            CupertinoDynamicColor::resolve(&K_CUPERTINO_MODAL_BARRIER_COLOR, app, context).color();
        let route = RouteData::new(app, None, None);
        let transition_route = TransitionRouteData::new(app);
        let modal_route = ModalRouteData::new(app);
        let page_builder: RoutePageBuilder =
            Rc::new(move |app, context, _animation, _secondary_animation| builder(app, context));
        let mut raw_dialog_route = RawDialogRouteData::new(page_builder);
        raw_dialog_route.barrier_label = Some(barrier_label);
        raw_dialog_route.barrier_color = Some(barrier_color);
        raw_dialog_route.transition_duration = Duration::from_millis(250);
        raw_dialog_route.transition_builder = Some(Rc::new(build_cupertino_dialog_transitions));
        app.create(CupertinoDialogRoute {
            route,
            overlay_route: OverlayRouteData::new(),
            transition_route,
            local_history: LocalHistoryRouteData::new(),
            modal_route,
            raw_dialog_route,
            transition_builder: None,
        })
    }

    /// Dart `CupertinoDialogRoute(barrierDismissible:)`.
    pub fn barrier_dismissible(
        self: Ref<Self>,
        app: &mut App,
        barrier_dismissible: bool,
    ) -> Ref<Self> {
        self.raw_dialog_route_data_mut(app).barrier_dismissible = barrier_dismissible;
        self
    }

    /// Dart `CupertinoDialogRoute(barrierColor:)`.
    pub fn barrier_color(
        self: Ref<Self>,
        app: &mut App,
        barrier_color: Option<Color>,
    ) -> Ref<Self> {
        self.raw_dialog_route_data_mut(app).barrier_color = barrier_color;
        self
    }

    /// Dart `CupertinoDialogRoute(barrierLabel:)`.
    pub fn barrier_label(self: Ref<Self>, app: &mut App, barrier_label: String) -> Ref<Self> {
        self.raw_dialog_route_data_mut(app).barrier_label = Some(barrier_label);
        self
    }

    /// Dart `CupertinoDialogRoute(transitionDuration:)`.
    pub fn transition_duration(
        self: Ref<Self>,
        app: &mut App,
        transition_duration: Duration,
    ) -> Ref<Self> {
        self.raw_dialog_route_data_mut(app).transition_duration = transition_duration;
        self
    }

    /// Dart `CupertinoDialogRoute(transitionBuilder:)`, which Dart also hands to
    /// `RawDialogRoute` in place of the jump-cut fallback.
    pub fn transition_builder(
        self: Ref<Self>,
        app: &mut App,
        transition_builder: RouteTransitionsBuilder,
    ) -> Ref<Self> {
        self.raw_dialog_route_data_mut(app).transition_builder =
            Some(Rc::clone(&transition_builder));
        app.get_mut(&self).transition_builder = Some(transition_builder);
        self
    }

    /// Dart `CupertinoDialogRoute(settings:)`.
    pub fn settings(self: Ref<Self>, app: &mut App, settings: RouteSettingsRef) -> Ref<Self> {
        self.route_data_mut(app).set_settings(settings);
        self
    }

    /// Dart `CupertinoDialogRoute(requestFocus:)`.
    pub fn request_focus(self: Ref<Self>, app: &mut App, request_focus: bool) -> Ref<Self> {
        self.route_data_mut(app)
            .set_request_focus(Some(request_focus));
        self
    }
}

impl Route for CupertinoDialogRoute {
    waft_widgets::modal_route_overrides!();
}

impl OverlayRoute for CupertinoDialogRoute {
    waft_widgets::modal_overlay_route_overrides!();
}

impl PredictiveBackRoute for CupertinoDialogRoute {
    waft_widgets::modal_predictive_back_route_overrides!(ModalRoute::pop_gesture_enabled);
}

impl TransitionRoute for CupertinoDialogRoute {
    waft_widgets::raw_dialog_route_transition_route_overrides!();

    fn create_simulation(
        self: Handle<Self>,
        app: &mut App,
        forward: bool,
    ) -> Option<Box<dyn Simulation>> {
        create_standard_simulation(self, app, forward)
    }
}

impl LocalHistoryRoute for CupertinoDialogRoute {
    waft_widgets::local_history_route_accessors!();
}

impl ModalRoute for CupertinoDialogRoute {
    waft_widgets::raw_dialog_route_modal_route_overrides!();
}

impl PopupRoute for CupertinoDialogRoute {}

impl RawDialogRouteLeaf for CupertinoDialogRoute {
    waft_widgets::raw_dialog_route_accessors!();

    fn build_transitions(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        animation: AnimationRef<f64>,
        secondary_animation: AnimationRef<f64>,
        child: WidgetRef,
    ) -> WidgetRef {
        if app.get(self).transition_builder.is_some() {
            return RawDialogRoute::build_transitions(
                self,
                app,
                context,
                animation,
                secondary_animation,
                child,
            );
        }

        if animation.status(app) == AnimationStatus::Reverse {
            return FadeTransition::new(animation).child(child).into_widget();
        }
        let scale = animation.drive(app, CupertinoDialogRoute::DIALOG_SCALE_TWEEN);
        FadeTransition::new(animation)
            .child(ScaleTransition::new(scale).child(child))
            .into_widget()
    }
}

// ---------------------------------------------------------------------------------------------
// CupertinoPageTransitionsBuilder

/// A `PageTransitionsBuilder` that provides an iOS-style page transition animation.
///
/// The page slides in from the right and exits in reverse. It also shifts to the left in a
/// parallax motion when another page enters to cover it. This transition is commonly seen in
/// native iOS applications.
///
/// In a `CupertinoApp`, this transition is used automatically when navigating with
/// [`CupertinoPageRoute`].
///
/// See also:
///
///  * [`CupertinoPageRoute`], which uses this transition style by default for Cupertino apps.
///  * [`CupertinoPageTransition`], the widget that implements the iOS page transition
///    animation.
///  * `MaterialPageRoute`, an adaptive [`PageRoute`] that can use this builder through
///    `PageTransitionsTheme`.
///  * `PageTransitionsTheme`, which defines the page transitions used by `MaterialPageRoute`
///    for different target platforms.
#[derive(Clone, Copy, Debug, Default)]
pub struct CupertinoPageTransitionsBuilder;

impl CupertinoPageTransitionsBuilder {
    /// Constructs a page transition animation that matches the iOS transition.
    pub const fn new() -> CupertinoPageTransitionsBuilder {
        CupertinoPageTransitionsBuilder
    }
}

impl PageTransitionsBuilder for CupertinoPageTransitionsBuilder {
    fn transition_duration(&self) -> Duration {
        // Dart's static `CupertinoRouteTransitionMixin.kTransitionDuration`; the trait's
        // associated const is read through one of its routes.
        <CupertinoPageRoute as CupertinoRouteTransitionMixin>::K_TRANSITION_DURATION
    }

    fn delegated_transition(&self) -> Option<DelegatedTransitionBuilder> {
        Some(DelegatedTransitionBuilder::function(
            CupertinoPageTransition::delegated_transition,
        ))
    }

    fn build_transitions(
        &self,
        app: &mut App,
        route: AnyPageRoute,
        context: BuildContext,
        animation: AnimationRef<f64>,
        secondary_animation: AnimationRef<f64>,
        child: WidgetRef,
    ) -> WidgetRef {
        build_page_transitions(route, app, context, animation, secondary_animation, child)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use waft_embedder::{Locale, PointerChange};
    use waft_test::{assert_no_new_objects, object_snapshot};
    use waft_widgets::testing::{TestPointer, TestVSync, WidgetTester, test_widgets};
    use waft_widgets::{
        AnyModalRoute, DefaultWidgetsLocalizations, GlobalKey, Localizations, Navigator,
        NavigatorState, PageRef, SizedBox,
    };

    use super::*;
    use crate::localizations::DefaultCupertinoLocalizations;

    /// The tester's view width, in logical pixels.
    const VIEW_WIDTH: f64 = 800.0;

    /// A back-gesture drag from the left edge at `(5, 150)`: `steps` moves of `step` each,
    /// stamped `interval` apart, with a frame `interval` long after each; the finger stays
    /// down.
    fn drag_from_the_edge(
        tester: &mut WidgetTester,
        step: f64,
        steps: u32,
        interval: Duration,
    ) -> TestPointer {
        let mut finger = TestPointer::touch(1);
        tester.down(&mut finger, Offset::new(5.0, 150.0));
        for n in 1..=steps {
            finger.position = finger.position + Offset::new(step, 0.0);
            tester.send_at(
                &finger,
                PointerChange::Move,
                Offset::new(step, 0.0),
                interval * n,
            );
            tester.pump_by(interval);
        }
        finger
    }

    /// A navigator whose every route is a plain `CupertinoPageRoute`, settled.
    fn mount_plain_navigator(tester: &mut WidgetTester, navigator_key: &GlobalKey) {
        tester.pump_widget(navigator(navigator_key, |navigator| {
            navigator.on_generate_route(|app, _settings| {
                Some(CupertinoPageRoute::new(app, plain_page()).into_erased())
            })
        }));
        tester.pump_and_settle();
    }

    /// A navigator whose initial page hands its build context to the cell this returns,
    /// settled.
    fn mount_context_navigator(tester: &mut WidgetTester) -> Rc<Cell<Option<BuildContext>>> {
        let navigator_key = GlobalKey::new();
        let page_context: Rc<Cell<Option<BuildContext>>> = Rc::default();
        tester.pump_widget({
            let page_context = Rc::clone(&page_context);
            navigator(&navigator_key, move |navigator| {
                navigator.on_generate_route(move |app, _settings| {
                    Some(
                        CupertinoPageRoute::new(app, context_page(Rc::clone(&page_context)))
                            .into_erased(),
                    )
                })
            })
        });
        tester.pump_and_settle();
        page_context
    }

    /// Pushes a `CupertinoPageRoute` of a page keyed `page_key` and settles the push.
    fn push_keyed_page(
        tester: &mut WidgetTester,
        navigator_key: &GlobalKey,
        page_key: &GlobalKey,
    ) -> Ref<CupertinoPageRoute> {
        let state = navigator_state(navigator_key, &mut tester.cell.borrow_mut());
        let pushed = tester.update(|app| {
            let pushed = CupertinoPageRoute::new(app, keyed_page(page_key));
            state.push(app, pushed.clone().into_erased());
            pushed
        });
        tester.pump_and_settle();
        pushed
    }

    /// A page filling the view, keyed so a test can find its render box.
    fn keyed_page(key: &GlobalKey) -> WidgetBuilder {
        let key: KeyRef = key.clone().into();
        Rc::new(move |_app, _context| SizedBox::expand().key(key.clone()).into_widget())
    }

    fn plain_page() -> WidgetBuilder {
        Rc::new(|_app, _context| SizedBox::expand().into_widget())
    }

    /// A page that hands its build context to `sink` on every build.
    fn context_page(sink: Rc<Cell<Option<BuildContext>>>) -> WidgetBuilder {
        Rc::new(move |_app, context| {
            sink.set(Some(context));
            SizedBox::expand().into_widget()
        })
    }

    fn navigator(key: &GlobalKey, configure: impl FnOnce(Navigator) -> Navigator) -> WidgetRef {
        let navigator = configure(Navigator::new().key(key.clone().into()));
        Localizations::new(
            Locale::new("en"),
            vec![
                DefaultWidgetsLocalizations::delegate(),
                DefaultCupertinoLocalizations::delegate(),
            ],
        )
        .child(Directionality::new(TextDirection::Ltr, navigator))
        .into_widget()
    }

    fn navigator_state(key: &GlobalKey, app: &mut App) -> Handle<NavigatorState> {
        key.current_state::<NavigatorState>(app)
            .expect("a mounted navigator")
    }

    /// The x coordinate the keyed page paints at, in the view's coordinate system.
    fn page_left(key: &GlobalKey, tester: &WidgetTester) -> f64 {
        top_left(key, tester).dx()
    }

    /// The y coordinate the keyed page paints at, in the view's coordinate system.
    fn page_top(key: &GlobalKey, tester: &WidgetTester) -> f64 {
        top_left(key, tester).dy()
    }

    fn top_left(key: &GlobalKey, tester: &WidgetTester) -> Offset {
        let render = tester
            .render_object(key)
            .as_box()
            .expect("the page is a box");
        render.local_to_global(&tester.cell.borrow(), Offset::ZERO, None)
    }

    /// Checks that a frame part way into `push`, over a settled page, leaves no new arena
    /// objects: Dart's collector frees what each frame's build replaces.
    fn assert_a_mid_push_frame_leaves_nothing(
        push: impl FnOnce(&mut App, BuildContext, Handle<NavigatorState>) + Send,
    ) {
        test_widgets(|tester| {
            let navigator_key = GlobalKey::new();
            let page_context: Rc<Cell<Option<BuildContext>>> = Rc::default();
            tester.pump_widget({
                let page_context = Rc::clone(&page_context);
                navigator(&navigator_key, move |navigator| {
                    navigator.on_generate_route(move |app, _settings| {
                        Some(
                            CupertinoPageRoute::new(app, context_page(Rc::clone(&page_context)))
                                .into_erased(),
                        )
                    })
                })
            });
            tester.pump_and_settle();
            tester.update(|app| {
                let state = navigator_state(&navigator_key, app);
                push(app, page_context.get().expect("the page built"), state);
            });
            for _ in 0..3 {
                tester.pump_by(Duration::from_millis(20));
            }
            let snapshot = object_snapshot(&tester.cell.borrow());
            tester.pump_by(Duration::from_millis(20));
            assert_no_new_objects(&tester.cell.borrow(), &snapshot);
        });
    }

    /// Not a Dart case: a controller disposed before its status changes takes the back
    /// gesture's listener with it, as the closure Dart makes goes with its controller.
    #[test]
    fn the_back_gestures_status_listener_goes_with_a_controller_disposed_first() {
        test_widgets(|tester| {
            use waft_animation::AnimationBehavior;
            let navigator_key = GlobalKey::new();
            tester.pump_widget(navigator(&navigator_key, |navigator| {
                navigator.on_generate_route(|app, _settings| {
                    Some(CupertinoPageRoute::new(app, plain_page()).into_erased())
                })
            }));
            let mut app = tester.cell.borrow_mut();
            let state = navigator_state(&navigator_key, &mut app);
            let controller = AnimationController::new(
                &mut app,
                None,
                Some(Duration::from_millis(100)),
                None,
                0.0,
                1.0,
                AnimationBehavior::Normal,
                TestVSync,
            );
            let listener =
                stop_user_gesture_when_the_animation_ends(&mut app, state, controller.handle());
            assert!(
                listener.upgrade().is_some(),
                "the controller's list holds it"
            );
            controller.dispose(&mut app);
            drop(controller);
            drop(app);
            tester.cell.checkpoint();
            assert!(listener.upgrade().is_none(), "nothing else holds it");
        });
    }

    /// Not a Dart case: see [`assert_a_mid_push_frame_leaves_nothing`].
    #[test]
    fn a_frame_of_a_cupertino_page_push_leaves_no_new_arena_objects() {
        assert_a_mid_push_frame_leaves_nothing(|app, _context, state| {
            let pushed = CupertinoPageRoute::new(app, plain_page());
            state.push(app, pushed.clone().into_erased());
        });
    }

    /// Not a Dart case: see [`assert_a_mid_push_frame_leaves_nothing`].
    #[test]
    fn a_frame_of_a_fullscreen_dialog_push_leaves_no_new_arena_objects() {
        assert_a_mid_push_frame_leaves_nothing(|app, _context, state| {
            let pushed = CupertinoPageRoute::new(app, plain_page()).fullscreen_dialog(app, true);
            state.push(app, pushed.clone().into_erased());
        });
    }

    /// Not a Dart case: see [`assert_a_mid_push_frame_leaves_nothing`].
    #[test]
    fn a_frame_of_a_cupertino_dialog_leaves_no_new_arena_objects() {
        assert_a_mid_push_frame_leaves_nothing(|app, context, _state| {
            let _popped = ShowCupertinoDialog::new(plain_page()).show(app, context);
        });
    }

    #[test]
    fn a_pushed_cupertino_page_route_slides_in_from_the_right() {
        test_widgets(|tester| {
            let navigator_key = GlobalKey::new();
            mount_plain_navigator(tester, &navigator_key);
            let state = navigator_state(&navigator_key, &mut tester.cell.borrow_mut());

            let page_key = GlobalKey::new();
            let pushed = tester.update(|app| {
                let pushed = CupertinoPageRoute::new(app, keyed_page(&page_key));
                state.push(app, pushed.clone().into_erased());
                pushed
            });
            for _ in 0..4 {
                tester.pump_by(Duration::from_millis(20));
            }
            let mid = page_left(&page_key, tester);
            assert!(
                mid > 0.0 && mid < VIEW_WIDTH,
                "the page is part way in from the right, at {mid}"
            );

            tester.pump_and_settle();
            assert_eq!(
                page_left(&page_key, tester),
                0.0,
                "the slide ends with the page fully on screen"
            );
            {
                let app = tester.cell.borrow();
                let animation =
                    TransitionRoute::animation(pushed.handle(), &app).expect("an installed route");
                assert_eq!(animation.value(&app), 1.0);
            }

            tester.update(|app| state.pop(app, None));
            tester.pump_and_settle();
            assert!(
                !Route::is_active(pushed.handle(), &tester.cell.borrow()),
                "the route popped"
            );
        });
    }

    #[test]
    fn cupertino_pages_build_page_based_routes_that_carry_the_previous_title() {
        test_widgets(|tester| {
            let navigator_key = GlobalKey::new();
            let top_context: Rc<Cell<Option<BuildContext>>> = Rc::default();
            let pages: Vec<PageRef> = vec![
                Rc::new(CupertinoPage::new(SizedBox::expand()).title("Home".to_string())),
                Rc::new(
                    CupertinoPage::new(
                        Builder::new({
                            let top_context = Rc::clone(&top_context);
                            move |_app, context| {
                                top_context.set(Some(context));
                                SizedBox::expand().into_widget()
                            }
                        })
                        .into_widget(),
                    )
                    .title("Details".to_string()),
                ),
            ];
            tester.pump_widget(navigator(&navigator_key, move |navigator| {
                navigator.pages(pages).on_did_remove_page(|_app, _page| {})
            }));
            tester.pump_and_settle();
            let mut app = tester.cell.borrow_mut();

            let context = top_context.get().expect("the top page built");
            let route = AnyModalRoute::of(&mut app, context)
                .expect("a modal route")
                .as_route()
                .downcast::<PageBasedCupertinoPageRoute>(&app)
                .expect("a page-based Cupertino page route");
            assert_eq!(
                CupertinoRouteTransitionMixin::title(route, &app),
                Some("Details".to_string())
            );
            let previous_title = CupertinoRouteTransitionMixin::previous_title(route, &app);
            assert_eq!(
                app.get(previous_title).value().clone(),
                Some("Home".to_string()),
                "the previous route's title is readable once installed"
            );
            assert!(!ModalRoute::fullscreen_dialog(route, &app));
            assert!(ModalRoute::maintain_state(route, &app));
        });
    }

    #[test]
    fn a_flung_back_gesture_drag_moves_the_page_and_pops_the_route() {
        test_widgets(|tester| {
            let navigator_key = GlobalKey::new();
            mount_plain_navigator(tester, &navigator_key);
            let page_key = GlobalKey::new();
            let pushed = push_keyed_page(tester, &navigator_key, &page_key);
            assert_eq!(page_left(&page_key, tester), 0.0);

            let finger = drag_from_the_edge(tester, 25.0, 4, Duration::from_millis(12));
            let dragged = page_left(&page_key, tester);
            assert!(dragged > 0.0, "the drag moved the page right, to {dragged}");

            tester.send_at(
                &finger,
                PointerChange::Up,
                Offset::ZERO,
                Duration::from_millis(48),
            );
            tester.pump_and_settle();
            assert!(
                !Route::is_active(pushed.handle(), &tester.cell.borrow()),
                "the fling popped the route"
            );
        });
    }

    #[test]
    fn a_dropped_short_back_gesture_drag_snaps_the_page_back() {
        test_widgets(|tester| {
            let navigator_key = GlobalKey::new();
            mount_plain_navigator(tester, &navigator_key);
            let page_key = GlobalKey::new();
            let pushed = push_keyed_page(tester, &navigator_key, &page_key);

            let finger = drag_from_the_edge(tester, 5.0, 4, Duration::from_millis(60));
            let dragged = page_left(&page_key, tester);
            assert!(dragged > 0.0, "the drag moved the page right, to {dragged}");
            tester.send_at(
                &finger,
                PointerChange::Up,
                Offset::ZERO,
                Duration::from_millis(240),
            );
            tester.pump_and_settle();

            assert!(
                Route::is_active(pushed.handle(), &tester.cell.borrow()),
                "the short drag did not pop"
            );
            assert_eq!(
                page_left(&page_key, tester),
                0.0,
                "the page snapped back on screen"
            );
        });
    }

    #[test]
    fn a_modal_popup_slides_up_from_the_bottom_and_dismisses_on_a_barrier_tap() {
        test_widgets(|tester| {
            let page_context = mount_context_navigator(tester);

            let popup_key = GlobalKey::new();
            let key: KeyRef = popup_key.clone().into();
            let context = page_context.get().expect("the page built");
            let popped = tester.update(|app| {
                ShowCupertinoModalPopup::new(Rc::new(move |_app, _context| {
                    SizedBox::new()
                        .key(key.clone())
                        .width(200.0)
                        .height(100.0)
                        .into_widget()
                }))
                .show(app, context)
            });
            for _ in 0..3 {
                tester.pump_by(Duration::from_millis(20));
            }
            let mid = page_top(&popup_key, tester);
            assert!(mid > 500.0, "the popup starts below the view, at {mid}");

            tester.pump_and_settle();
            assert_eq!(
                page_top(&popup_key, tester),
                500.0,
                "the popup ends flush with the bottom of the 600pt view"
            );

            tester.tap_at(Offset::new(200.0, 20.0), 1);
            tester.pump_and_settle();
            assert!(
                matches!(popped.peek(), Some(None)),
                "the barrier tap dismissed the popup with no result"
            );
        });
    }

    #[test]
    fn a_cupertino_dialog_fades_in() {
        test_widgets(|tester| {
            let page_context = mount_context_navigator(tester);

            let dialog_context: Rc<Cell<Option<BuildContext>>> = Rc::default();
            let context = page_context.get().expect("the page built");
            let popped = tester.update(|app| {
                ShowCupertinoDialog::new(context_page(Rc::clone(&dialog_context)))
                    .show(app, context)
            });
            for _ in 0..2 {
                tester.pump_by(Duration::from_millis(20));
            }
            let dialog_context = dialog_context.get().expect("the dialog built");
            let opacity = dialog_context
                .find_ancestor_widget_of_exact_type::<FadeTransition>(&tester.cell.borrow())
                .expect("the dialog is wrapped in a FadeTransition")
                .opacity
                .clone();
            let value = opacity.value(&tester.cell.borrow());
            assert!(
                value > 0.0 && value < 1.0,
                "the dialog is fading in at {value}"
            );

            tester.pump_and_settle();
            assert_eq!(
                opacity.value(&tester.cell.borrow()),
                1.0,
                "the fade completes"
            );
            assert!(!popped.is_completed(), "the dialog is still up");
        });
    }

    #[test]
    fn the_pop_gesture_is_disabled_for_the_first_route_and_for_a_fullscreen_dialog() {
        test_widgets(|tester| {
            let navigator_key = GlobalKey::new();
            let first = Rc::new(Cell::new(None));
            tester.pump_widget({
                let first = Rc::clone(&first);
                navigator(&navigator_key, move |navigator| {
                    navigator.on_generate_route(move |app, _settings| {
                        let route = CupertinoPageRoute::new(app, plain_page());
                        first.set(Some(route.handle()));
                        Some(route.into_erased())
                    })
                })
            });
            tester.pump_and_settle();
            let state = navigator_state(&navigator_key, &mut tester.cell.borrow_mut());
            let first = first.get().expect("the initial route was generated");
            assert!(
                !tester.update(|app| PredictiveBackRoute::pop_gesture_enabled(first, app)),
                "the first route has nothing to go back to"
            );

            let dialog = tester.update(|app| {
                let dialog =
                    CupertinoPageRoute::new(app, plain_page()).fullscreen_dialog(app, true);
                state.push(app, dialog.clone().into_erased());
                dialog
            });
            tester.pump_and_settle();
            assert!(
                !tester
                    .update(|app| PredictiveBackRoute::pop_gesture_enabled(dialog.handle(), app)),
                "a fullscreen dialog is not dismissible by back swipe"
            );

            tester.update(|app| state.pop(app, None));
            tester.pump_and_settle();
            let second = tester.update(|app| {
                let second = CupertinoPageRoute::new(app, plain_page());
                state.push(app, second.clone().into_erased());
                second
            });
            tester.pump_and_settle();
            assert!(
                tester.update(|app| PredictiveBackRoute::pop_gesture_enabled(second.handle(), app)),
                "a plain second page route can be swiped back"
            );
        });
    }

    // ---- CupertinoPageTransitionsBuilder ----

    /// A page route whose transitions are the builder's, handed the route itself, as a
    /// `MaterialPageRoute` hands them through its theme.
    fn builder_route(
        app: &mut App,
        page: WidgetBuilder,
        fullscreen_dialog: bool,
    ) -> Ref<waft_widgets::PageRouteBuilder> {
        let route = waft_widgets::PageRouteBuilder::new(
            app,
            Rc::new(move |app, context, _animation, _secondary_animation| page(app, context)),
        )
        .fullscreen_dialog(app, fullscreen_dialog);
        let page_route = PageRoute::as_page_route(route.handle());
        route.transitions_builder(
            app,
            Rc::new(move |app, context, animation, secondary_animation, child| {
                CupertinoPageTransitionsBuilder::new().build_transitions(
                    app,
                    page_route,
                    context,
                    animation,
                    secondary_animation,
                    child,
                )
            }),
        )
    }

    /// route_test.dart's "builds a CupertinoPageTransition", with the builder in place of
    /// `CupertinoPageRoute`'s own transitions.
    #[test]
    fn the_builder_builds_a_cupertino_page_transition() {
        test_widgets(|tester| {
            let key = GlobalKey::new();
            let first = GlobalKey::new();
            let second = GlobalKey::new();
            tester.pump_widget(navigator(&key, {
                let first = first.clone();
                move |navigator| {
                    navigator.on_generate_route(move |app, _settings| {
                        Some(builder_route(app, keyed_page(&first), false).into_erased())
                    })
                }
            }));
            let mut app = tester.cell.borrow_mut();
            let first_context = first.current_context(&app).expect("the page is mounted");
            assert!(
                first_context
                    .find_ancestor_widget_of_exact_type::<CupertinoPageTransition>(&app)
                    .is_some()
            );

            let pushed = builder_route(&mut app, keyed_page(&second), false).into_erased();
            navigator_state(&key, &mut app).push(&mut app, pushed);
            drop(app);
            tester.pump_and_settle();
            let app = tester.cell.borrow();
            let second_context = second.current_context(&app).expect("the page is mounted");
            assert!(
                second_context
                    .find_ancestor_widget_of_exact_type::<CupertinoPageTransition>(&app)
                    .is_some()
            );
            drop(app);
            assert_eq!(page_left(&second, tester), 0.0, "the page slid in");

            let mut app = tester.cell.borrow_mut();
            let dialog_key = GlobalKey::new();
            let dialog = builder_route(&mut app, keyed_page(&dialog_key), true).into_erased();
            navigator_state(&key, &mut app).push(&mut app, dialog);
            drop(app);
            tester.pump_and_settle();
            let app = tester.cell.borrow();
            let dialog_context = dialog_key
                .current_context(&app)
                .expect("the dialog is mounted");
            assert!(
                dialog_context
                    .find_ancestor_widget_of_exact_type::<CupertinoFullscreenDialogTransition>(&app)
                    .is_some(),
                "a fullscreen dialog gets the dialog transition"
            );
        });
    }

    /// route_test.dart's "has correct transitionDuration of 500ms".
    #[test]
    fn the_builder_has_a_transition_duration_of_500_ms() {
        let builder = CupertinoPageTransitionsBuilder::new();

        // CupertinoRouteTransitionMixin.kTransitionDuration is 500ms
        assert_eq!(builder.transition_duration(), Duration::from_millis(500));
        assert_eq!(
            builder.reverse_transition_duration(),
            Duration::from_millis(500)
        );
    }

    /// route_test.dart's "has delegatedTransition".
    #[test]
    fn the_builder_has_cupertino_page_transition_s_delegated_transition() {
        let builder = CupertinoPageTransitionsBuilder::new();

        let delegated_transition = builder.delegated_transition();
        assert!(delegated_transition.is_some());
        assert!(
            delegated_transition
                == Some(DelegatedTransitionBuilder::function(
                    CupertinoPageTransition::delegated_transition
                ))
        );
        assert!(
            delegated_transition == builder.delegated_transition(),
            "the same function on every call, as Dart's tear-off of a static method"
        );
    }

    /// route_test.dart's "Update pages during a drag gesture will not stuck": as in Dart's
    /// `_TestPageUpdate`, each build makes a new navigator key, so updating the pages replaces
    /// the navigator and disposes the back-gesture detector in the middle of its drag.
    #[test]
    fn updating_the_pages_during_a_drag_gesture_does_not_get_stuck() {
        test_widgets(|tester| {
            let update_pages = tester.cell.borrow_mut().new_entity(|_cx| false);
            let navigator_key = Rc::new(std::cell::RefCell::new(GlobalKey::new()));
            let new_page_key = GlobalKey::new();
            let root = {
                let update_pages = update_pages.clone();
                let navigator_key = Rc::clone(&navigator_key);
                let new_page_key = new_page_key.clone();
                Builder::new(move |app, _context| {
                    let pages: Vec<PageRef> = if *update_pages.read(app) {
                        vec![
                            Rc::new(
                                CupertinoPage::new(SizedBox::expand()).name("/home".to_string()),
                            ),
                            Rc::new(
                                CupertinoPage::new(
                                    SizedBox::expand().key(new_page_key.clone().into()),
                                )
                                .name("/home/new".to_string()),
                            ),
                        ]
                    } else {
                        vec![
                            Rc::new(
                                CupertinoPage::new(SizedBox::expand()).name("/home".to_string()),
                            ),
                            Rc::new(
                                CupertinoPage::new(SizedBox::expand())
                                    .name("/home/old".to_string()),
                            ),
                        ]
                    };
                    let key = GlobalKey::new();
                    *navigator_key.borrow_mut() = key.clone();
                    navigator(&key, move |navigator| {
                        navigator
                            .pages(pages)
                            .on_pop_page(|_app, _route, _result| false)
                    })
                })
                .into_widget()
            };
            tester.pump_widget(root);
            tester.pump_and_settle();

            // Start swiping.
            let mut swipe_gesture = TestPointer::touch(1);
            tester.down(&mut swipe_gesture, Offset::new(5.0, 100.0));
            tester.move_to(&mut swipe_gesture, Offset::new(105.0, 100.0), true);
            tester.pump();
            let mut app = tester.cell.borrow_mut();

            let navigator = navigator_state(&navigator_key.borrow(), &mut app);
            assert!(navigator.user_gesture_in_progress(&app));

            update_pages.update(&mut app, |update_pages, cx| {
                *update_pages = true;
                cx.notify();
            });
            // Wait for pages to update.
            drop(app);
            tester.pump_by(Duration::from_secs(3));
            let mut app = tester.cell.borrow_mut();

            // Verify pages are updated.
            assert!(new_page_key.current_context(&app).is_some());
            // Verify `userGestureInProgress` is set to false.
            let navigator = navigator_state(&navigator_key.borrow(), &mut app);
            assert!(!navigator.user_gesture_in_progress(&app));
        });
    }

    /// Not a Dart case: the pages change during a back gesture's drag and the navigator stays,
    /// so the detector disposed with the dragged route stops the navigator's user gesture.
    #[test]
    fn a_navigator_that_outlives_a_detector_disposed_mid_drag_stops_its_user_gesture() {
        test_widgets(|tester| {
            let update_pages = tester.cell.borrow_mut().new_entity(|_cx| false);
            let navigator_key = GlobalKey::new();
            let new_page_key = GlobalKey::new();
            let root = {
                let update_pages = update_pages.clone();
                let navigator_key = navigator_key.clone();
                let new_page_key = new_page_key.clone();
                Builder::new(move |app, _context| {
                    let home: PageRef = Rc::new(
                        CupertinoPage::new(SizedBox::expand())
                            .key(waft_foundation::ValueKey::new("/home").into()),
                    );
                    let top: PageRef = if *update_pages.read(app) {
                        Rc::new(
                            CupertinoPage::new(SizedBox::expand().key(new_page_key.clone().into()))
                                .key(waft_foundation::ValueKey::new("/home/new").into()),
                        )
                    } else {
                        Rc::new(
                            CupertinoPage::new(SizedBox::expand())
                                .key(waft_foundation::ValueKey::new("/home/old").into()),
                        )
                    };
                    navigator(&navigator_key, move |navigator| {
                        navigator
                            .pages(vec![home, top])
                            .on_did_remove_page(|_app, _page| {})
                    })
                })
                .into_widget()
            };
            tester.pump_widget(root);
            tester.pump_and_settle();

            let mut finger = TestPointer::touch(1);
            tester.down(&mut finger, Offset::new(5.0, 100.0));
            tester.move_to(&mut finger, Offset::new(105.0, 100.0), true);
            tester.pump();
            let mut app = tester.cell.borrow_mut();
            let navigator = navigator_state(&navigator_key, &mut app);
            assert!(navigator.user_gesture_in_progress(&app));

            update_pages.update(&mut app, |update_pages, cx| {
                *update_pages = true;
                cx.notify();
            });
            drop(app);
            tester.pump_and_settle();
            let mut app = tester.cell.borrow_mut();

            assert!(new_page_key.current_context(&app).is_some());
            assert_eq!(navigator_state(&navigator_key, &mut app), navigator);
            assert!(!navigator.user_gesture_in_progress(&app));
        });
    }
}
