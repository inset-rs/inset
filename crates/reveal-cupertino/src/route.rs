//! Flutter counterpart: `cupertino/route.dart`.
//!
//! The `Semantics` wrappers wait with accessibility, `debugFillProperties` with diagnostics,
//! and `DisplayFeatureSubScreen` with `MediaQuery.displayFeatures`.

use std::any::{Any, TypeId};
use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use reveal_animation::{
    Animatable, Animation, AnimationController, AnimationStatus, AnimationStatusListener,
    AnyAnimation, CurvedAnimation, Curves, Tween,
};
use reveal_embedder::{Canvas, Color, ImageFilter, Offset, Paint, Rect, TextDirection};
use reveal_foundation::{App, CompleterFuture, Handle, Listener, ValueNotifier};
use reveal_gestures::{
    DragEndDetails, DragGestureRecognizer, DragStartDetails, DragUpdateDetails,
    HorizontalDragGestureRecognizer, PointerDownEvent, RecognizerLeaf,
};
use reveal_painting::{AlignmentGeometry, AnyColor, BoxPainter, Decoration, ImageConfiguration};
use reveal_physics::{Simulation, SpringDescription, SpringSimulation, Tolerance};
use reveal_rendering::{HitTestBehavior, ImageFilterConfig, StackFit};
use reveal_scheduler::{FrameCallback, SchedulerBinding};
use reveal_widgets::{
    Align, AnyRoute, AnyTransitionRoute, BuildContext, Builder, DecoratedBoxTransition,
    DecorationTween, DelegatedTransitionBuilder, Directionality, FadeTransition,
    FractionalTranslation, IntoWidget, KeyRef, LocalHistoryRoute, LocalHistoryRouteData,
    MediaQuery, ModalRoute, ModalRouteData, Navigator, NavigatorState, OverlayRoute,
    OverlayRouteData, Page, PageRef, PageRoute, PageRouteData, PopInvokedWithResultCallback,
    PopupRoute, Positioned, PredictiveBackRoute, RawDialogRoute, RawDialogRouteData,
    RawDialogRouteLeaf, Route, RouteData, RoutePageBuilder, RouteResult, RouteSettingsRef,
    RouteTransitionsBuilder, ScaleTransition, SlideTransition, Stack, State, StateData,
    StatefulWidget, TransitionRoute, TransitionRouteData, WidgetBuilder, WidgetRef,
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

/// Dart's `Tween<Offset>`: a value rather than an arena object, so a driven animation may
/// outlive the call that built it without a slot to free.
#[derive(Clone, Copy, Debug)]
struct OffsetTween {
    begin: Offset,
    end: Offset,
}

impl OffsetTween {
    const fn new(begin: Offset, end: Offset) -> OffsetTween {
        OffsetTween { begin, end }
    }
}

impl Animatable<Offset> for OffsetTween {
    fn transform(&self, _app: &App, t: f64) -> Offset {
        if t == 0.0 {
            return self.begin;
        }
        if t == 1.0 {
            return self.end;
        }
        Offset::lerp(Some(self.begin), Some(self.end), t).expect("both ends are set")
    }
}

/// Offset from offscreen to the right to fully on screen.
const K_RIGHT_MIDDLE_TWEEN: OffsetTween = OffsetTween::new(Offset::new(1.0, 0.0), Offset::ZERO);

/// Offset from fully on screen to 1/3 offscreen to the left.
const K_MIDDLE_LEFT_TWEEN: OffsetTween =
    OffsetTween::new(Offset::ZERO, Offset::new(-1.0 / 3.0, 0.0));

/// Offset from offscreen below to fully on screen.
const K_BOTTOM_UP_TWEEN: OffsetTween = OffsetTween::new(Offset::new(0.0, 1.0), Offset::ZERO);

// ---------------------------------------------------------------------------------------------
// CupertinoRouteTransitionMixin

/// The fields Dart's `CupertinoRouteTransitionMixin` declares; every route that mixes it in
/// carries this bag under the field `cupertino_route_transition`.
pub struct CupertinoRouteTransitionMixinData {
    previous_title: Option<Handle<ValueNotifier<Option<String>>>>,
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
            self: ::reveal_foundation::Handle<Self>,
            app: &::reveal_foundation::App,
        ) -> &$crate::CupertinoRouteTransitionMixinData {
            &app.get(self).cupertino_route_transition
        }

        fn cupertino_route_transition_data_mut(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
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
            .expect("Cannot read the previousTitle for a route that has not yet been installed")
    }

    /// Dart's `CupertinoRouteTransitionMixin.dispose`.
    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(previous_title) = self.cupertino_route_transition_data(app).previous_title {
            app.get_mut(previous_title).dispose();
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
        match self.cupertino_route_transition_data(app).previous_title {
            None => {
                let previous_title = app.create(ValueNotifier::new(previous_title_string));
                self.cupertino_route_transition_data_mut(app).previous_title = Some(previous_title);
            }
            Some(previous_title) => previous_title.set_value(app, previous_title_string),
        }
        ModalRoute::did_change_previous(self, app, previous_route);
    }

    /// Dart's `CupertinoRouteTransitionMixin.transitionDuration`.
    fn transition_duration(self: Handle<Self>, app: &App) -> Duration {
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
        animation: AnyAnimation<f64>,
        secondary_animation: AnyAnimation<f64>,
    ) -> WidgetRef {
        let _ = (animation, secondary_animation);
        self.build_content(app, context)
    }

    /// Dart's `CupertinoRouteTransitionMixin.buildTransitions`.
    fn build_transitions(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        animation: AnyAnimation<f64>,
        secondary_animation: AnyAnimation<f64>,
        child: WidgetRef,
    ) -> WidgetRef {
        build_page_transitions(self, app, context, animation, secondary_animation, child)
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
fn start_pop_gesture<R: PageRoute>(
    this: Handle<R>,
    app: &mut App,
) -> Handle<CupertinoBackGestureController> {
    debug_assert!(PredictiveBackRoute::pop_gesture_enabled(this, app));

    let navigator = Route::navigator(this, app).expect("an installed route");
    let controller =
        TransitionRoute::controller(this, app).expect("an installed route has a controller");
    let route = Route::as_route(this);
    CupertinoBackGestureController::new(
        app,
        navigator,
        controller,
        Rc::new(move |app| route.is_current(app)),
        Rc::new(move |app| route.is_active(app)),
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
pub fn build_page_transitions<R: PageRoute>(
    this: Handle<R>,
    app: &mut App,
    context: BuildContext,
    animation: AnyAnimation<f64>,
    secondary_animation: AnyAnimation<f64>,
    child: WidgetRef,
) -> WidgetRef {
    // Check if the route has an animation that's currently participating in a back swipe
    // gesture.
    //
    // In the middle of a back gesture drag, let the transition be linear to match finger
    // motions.
    let linear_transition = ModalRoute::pop_gesture_in_progress(this, app);
    let _ = context;
    if PageRoute::fullscreen_dialog(this, app) {
        CupertinoFullscreenDialogTransition::new(
            animation,
            secondary_animation,
            child,
            linear_transition,
        )
        .into_widget()
    } else {
        let detector = CupertinoBackGestureDetector {
            key: None,
            enabled_callback: Rc::new(move |app| {
                PredictiveBackRoute::pop_gesture_enabled(this, app)
            }),
            on_start_pop_gesture: Rc::new(move |app| start_pop_gesture(this, app)),
            child,
        };
        CupertinoPageTransition::new(animation, secondary_animation, detector, linear_transition)
            .into_widget()
    }
}

/// The `Route` overrides a [`CupertinoRouteTransitionMixin`] leaf inherits: `reveal-widgets`'
/// `page_route_overrides!` with `dispose` and `did_change_previous` pointed at the mixin.
/// Write it inside `impl Route for MyRoute { .. }`.
#[macro_export]
macro_rules! cupertino_route_transition_mixin_route_overrides {
    () => {
        ::reveal_widgets::route_accessors!();

        fn as_transition_route(
            self: ::reveal_foundation::Handle<Self>,
        ) -> ::std::option::Option<::reveal_widgets::AnyTransitionRoute> {
            ::std::option::Option::Some(::reveal_widgets::TransitionRoute::as_transition_route(
                self,
            ))
        }

        fn as_modal_route(
            self: ::reveal_foundation::Handle<Self>,
        ) -> ::std::option::Option<::reveal_widgets::AnyModalRoute> {
            ::std::option::Option::Some(::reveal_widgets::ModalRoute::as_modal_route(self))
        }

        fn as_page_route(
            self: ::reveal_foundation::Handle<Self>,
        ) -> ::std::option::Option<::reveal_widgets::AnyPageRoute> {
            ::std::option::Option::Some(::reveal_widgets::PageRoute::as_page_route(self))
        }

        fn interface(
            self: ::reveal_foundation::Handle<Self>,
            id: ::std::any::TypeId,
        ) -> ::std::option::Option<::std::boxed::Box<dyn ::std::any::Any>> {
            $crate::CupertinoRouteTransitionMixin::interface(self, id)
        }

        fn overlay_entries(
            self: ::reveal_foundation::Handle<Self>,
            app: &::reveal_foundation::App,
        ) -> ::std::vec::Vec<::reveal_foundation::Handle<::reveal_widgets::OverlayEntry>> {
            ::reveal_widgets::OverlayRoute::overlay_entries(self, app)
        }

        fn install(self: ::reveal_foundation::Handle<Self>, app: &mut ::reveal_foundation::App) {
            ::reveal_widgets::ModalRoute::install(self, app)
        }

        fn did_push(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
        ) -> ::reveal_scheduler::TickerFuture {
            ::reveal_widgets::ModalRoute::did_push(self, app)
        }

        fn did_add(self: ::reveal_foundation::Handle<Self>, app: &mut ::reveal_foundation::App) {
            ::reveal_widgets::ModalRoute::did_add(self, app)
        }

        fn did_replace(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
            old_route: ::std::option::Option<::reveal_widgets::AnyRoute>,
        ) {
            ::reveal_widgets::TransitionRoute::did_replace(self, app, old_route)
        }

        fn will_pop(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
        ) -> ::reveal_widgets::RoutePopDisposition {
            ::reveal_widgets::LocalHistoryRoute::will_pop(self, app)
        }

        fn pop_disposition(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
        ) -> ::reveal_widgets::RoutePopDisposition {
            ::reveal_widgets::ModalRoute::pop_disposition(self, app)
        }

        fn on_pop_invoked_with_result(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
            did_pop: bool,
            result: ::reveal_widgets::RouteResult,
        ) {
            ::reveal_widgets::ModalRoute::on_pop_invoked_with_result(self, app, did_pop, result)
        }

        fn will_handle_pop_internally(
            self: ::reveal_foundation::Handle<Self>,
            app: &::reveal_foundation::App,
        ) -> bool {
            ::reveal_widgets::LocalHistoryRoute::will_handle_pop_internally(self, app)
        }

        fn did_pop(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
            result: ::reveal_widgets::RouteResult,
        ) -> bool {
            ::reveal_widgets::LocalHistoryRoute::did_pop(self, app, result)
        }

        fn did_pop_next(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
            next_route: ::reveal_widgets::AnyRoute,
        ) {
            ::reveal_widgets::ModalRoute::did_pop_next(self, app, next_route)
        }

        fn did_change_next(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
            next_route: ::std::option::Option<::reveal_widgets::AnyRoute>,
        ) {
            ::reveal_widgets::ModalRoute::did_change_next(self, app, next_route)
        }

        fn did_change_previous(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
            previous_route: ::std::option::Option<::reveal_widgets::AnyRoute>,
        ) {
            $crate::CupertinoRouteTransitionMixin::did_change_previous(self, app, previous_route)
        }

        fn changed_internal_state(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
        ) {
            ::reveal_widgets::ModalRoute::changed_internal_state(self, app)
        }

        fn changed_external_state(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
        ) {
            ::reveal_widgets::ModalRoute::changed_external_state(self, app)
        }

        fn dispose(self: ::reveal_foundation::Handle<Self>, app: &mut ::reveal_foundation::App) {
            $crate::CupertinoRouteTransitionMixin::dispose(self, app)
        }
    };
}

/// The `TransitionRoute` overrides a [`CupertinoRouteTransitionMixin`] leaf inherits, plus
/// `reveal-widgets`' `modal_transition_route_overrides!`. Write it inside
/// `impl TransitionRoute for MyRoute { .. }`, next to the leaf's own `opaque`.
#[macro_export]
macro_rules! cupertino_route_transition_mixin_transition_route_overrides {
    () => {
        ::reveal_widgets::modal_transition_route_overrides!();

        fn allow_snapshotting(
            self: ::reveal_foundation::Handle<Self>,
            app: &::reveal_foundation::App,
        ) -> bool {
            ::reveal_widgets::PageRoute::allow_snapshotting(self, app)
        }

        fn transition_duration(
            self: ::reveal_foundation::Handle<Self>,
            app: &::reveal_foundation::App,
        ) -> ::std::time::Duration {
            $crate::CupertinoRouteTransitionMixin::transition_duration(self, app)
        }

        fn can_transition_to(
            self: ::reveal_foundation::Handle<Self>,
            app: &::reveal_foundation::App,
            next_route: ::reveal_widgets::AnyTransitionRoute,
        ) -> bool {
            $crate::CupertinoRouteTransitionMixin::can_transition_to(self, app, next_route)
        }

        fn can_transition_from(
            self: ::reveal_foundation::Handle<Self>,
            app: &::reveal_foundation::App,
            previous_route: ::reveal_widgets::AnyTransitionRoute,
        ) -> bool {
            $crate::CupertinoRouteTransitionMixin::can_transition_from(self, app, previous_route)
        }
    };
}

/// The `ModalRoute` overrides a [`CupertinoRouteTransitionMixin`] leaf inherits, plus
/// `reveal-widgets`' `modal_route_accessors!`. Write it inside
/// `impl ModalRoute for MyRoute { .. }`, next to the leaf's own `maintain_state`.
#[macro_export]
macro_rules! cupertino_route_transition_mixin_modal_route_overrides {
    () => {
        ::reveal_widgets::modal_route_accessors!();

        fn barrier_dismissible(
            self: ::reveal_foundation::Handle<Self>,
            app: &::reveal_foundation::App,
        ) -> bool {
            ::reveal_widgets::PageRoute::barrier_dismissible(self, app)
        }

        fn barrier_color(
            self: ::reveal_foundation::Handle<Self>,
            app: &::reveal_foundation::App,
        ) -> ::std::option::Option<::reveal_painting::Color> {
            $crate::CupertinoRouteTransitionMixin::barrier_color(self, app)
        }

        fn barrier_label(
            self: ::reveal_foundation::Handle<Self>,
            app: &::reveal_foundation::App,
        ) -> ::std::option::Option<::std::string::String> {
            $crate::CupertinoRouteTransitionMixin::barrier_label(self, app)
        }

        fn build_page(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
            context: ::reveal_widgets::BuildContext,
            animation: ::reveal_animation::AnyAnimation<f64>,
            secondary_animation: ::reveal_animation::AnyAnimation<f64>,
        ) -> ::reveal_widgets::WidgetRef {
            $crate::CupertinoRouteTransitionMixin::build_page(
                self,
                app,
                context,
                animation,
                secondary_animation,
            )
        }

        fn build_transitions(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
            context: ::reveal_widgets::BuildContext,
            animation: ::reveal_animation::AnyAnimation<f64>,
            secondary_animation: ::reveal_animation::AnyAnimation<f64>,
            child: ::reveal_widgets::WidgetRef,
        ) -> ::reveal_widgets::WidgetRef {
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
}

impl CupertinoPageRoute {
    /// Creates a page route for use in an iOS designed app; Dart's optional arguments are the
    /// setters.
    ///
    /// Dart's defaults: `maintain_state` true, `fullscreen_dialog` false, `allow_snapshotting`
    /// true, `barrier_dismissible` false.
    pub fn new(app: &mut App, builder: WidgetBuilder) -> Handle<CupertinoPageRoute> {
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
        });
        debug_assert!(TransitionRoute::opaque(this, app));
        this
    }

    /// Dart `CupertinoPageRoute(title:)`.
    pub fn title(self: Handle<Self>, app: &mut App, title: String) -> Handle<Self> {
        app.get_mut(self).title = Some(title);
        self
    }

    /// Dart `CupertinoPageRoute(settings:)`.
    pub fn settings(self: Handle<Self>, app: &mut App, settings: RouteSettingsRef) -> Handle<Self> {
        self.route_data_mut(app).set_settings(settings);
        self
    }

    /// Dart `CupertinoPageRoute(requestFocus:)`.
    pub fn request_focus(self: Handle<Self>, app: &mut App, request_focus: bool) -> Handle<Self> {
        self.route_data_mut(app)
            .set_request_focus(Some(request_focus));
        self
    }

    /// Dart `CupertinoPageRoute(maintainState:)`.
    pub fn maintain_state(self: Handle<Self>, app: &mut App, maintain_state: bool) -> Handle<Self> {
        app.get_mut(self).maintain_state = maintain_state;
        self
    }

    /// Dart `CupertinoPageRoute(fullscreenDialog:)`.
    pub fn fullscreen_dialog(
        self: Handle<Self>,
        app: &mut App,
        fullscreen_dialog: bool,
    ) -> Handle<Self> {
        self.page_route_data_mut(app).fullscreen_dialog = fullscreen_dialog;
        self
    }

    /// Dart `CupertinoPageRoute(allowSnapshotting:)`.
    pub fn allow_snapshotting(
        self: Handle<Self>,
        app: &mut App,
        allow_snapshotting: bool,
    ) -> Handle<Self> {
        self.page_route_data_mut(app).allow_snapshotting = allow_snapshotting;
        self
    }

    /// Dart `CupertinoPageRoute(barrierDismissible:)`.
    pub fn barrier_dismissible(
        self: Handle<Self>,
        app: &mut App,
        barrier_dismissible: bool,
    ) -> Handle<Self> {
        self.page_route_data_mut(app).barrier_dismissible = barrier_dismissible;
        self
    }
}

impl Route for CupertinoPageRoute {
    crate::cupertino_route_transition_mixin_route_overrides!();
}

impl OverlayRoute for CupertinoPageRoute {
    reveal_widgets::modal_overlay_route_overrides!();
}

impl PredictiveBackRoute for CupertinoPageRoute {
    reveal_widgets::page_predictive_back_route_overrides!();
}

impl TransitionRoute for CupertinoPageRoute {
    crate::cupertino_route_transition_mixin_transition_route_overrides!();

    fn opaque(self: Handle<Self>, app: &App) -> bool {
        PageRoute::opaque(self, app)
    }
}

impl LocalHistoryRoute for CupertinoPageRoute {
    reveal_widgets::local_history_route_accessors!();
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
        Some(Rc::new(CupertinoPageTransition::delegated_transition))
    }
}

impl PageRoute for CupertinoPageRoute {
    reveal_widgets::page_route_accessors!();
}

impl CupertinoRouteTransitionMixin for CupertinoPageRoute {
    crate::cupertino_route_transition_mixin_accessors!();

    fn build_content(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let builder = Rc::clone(&app.get(self).builder);
        builder(app, context)
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
    ) -> Handle<PageBasedCupertinoPageRoute> {
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
        debug_assert!(TransitionRoute::opaque(this, app));
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
    reveal_widgets::modal_overlay_route_overrides!();
}

impl PredictiveBackRoute for PageBasedCupertinoPageRoute {
    reveal_widgets::page_predictive_back_route_overrides!();
}

impl TransitionRoute for PageBasedCupertinoPageRoute {
    crate::cupertino_route_transition_mixin_transition_route_overrides!();

    fn opaque(self: Handle<Self>, app: &App) -> bool {
        PageRoute::opaque(self, app)
    }
}

impl LocalHistoryRoute for PageBasedCupertinoPageRoute {
    reveal_widgets::local_history_route_accessors!();
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
            Rc::new(CupertinoPageTransition::delegated_transition) as DelegatedTransitionBuilder
        })
    }
}

impl PageRoute for PageBasedCupertinoPageRoute {
    reveal_widgets::page_route_accessors!();

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
}

impl CupertinoPage {
    /// Creates a cupertino page; Dart's optional arguments are the setters.
    ///
    /// Dart's defaults: `maintain_state` true, `fullscreen_dialog` false, `allow_snapshotting`
    /// true, `can_pop` true.
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

    fn create_route(&self, app: &mut App, context: BuildContext, this: &PageRef) -> AnyRoute {
        let _ = context;
        Route::as_route(PageBasedCupertinoPageRoute::new(
            app,
            Rc::clone(this),
            self.allow_snapshotting,
        ))
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
    pub primary_route_animation: AnyAnimation<f64>,

    /// A linear route animation from 0.0 to 1.0 when another screen is being pushed on top of
    /// this one.
    pub secondary_route_animation: AnyAnimation<f64>,

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
        primary_route_animation: AnyAnimation<f64>,
        secondary_route_animation: AnyAnimation<f64>,
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
        animation: AnyAnimation<f64>,
        secondary_animation: AnyAnimation<f64>,
        allow_snapshotting: bool,
        child: Option<&WidgetRef>,
    ) -> Option<WidgetRef> {
        let _ = (animation, allow_snapshotting);
        let animation = CurvedAnimation::create(
            app,
            secondary_animation,
            Curves::linear_to_ease_out(),
            Some(Curves::ease_in_to_linear()),
        );
        let delegated_position_animation = animation.as_animation().drive(app, K_MIDDLE_LEFT_TWEEN);
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
    primary_position_animation: Option<AnyAnimation<Offset>>,
    // When this page is becoming covered by another page.
    secondary_position_animation: Option<AnyAnimation<Offset>>,
    // Shadow of page which is coming in to cover another page.
    primary_shadow_animation: Option<AnyAnimation<Box<dyn Decoration>>>,
    // Curve of primary page which is coming in to cover another page.
    primary_position_curve: Option<Handle<CurvedAnimation>>,
    // Curve of secondary page which is becoming covered by another page.
    secondary_position_curve: Option<Handle<CurvedAnimation>>,
    // Curve of primary page's shadow.
    primary_shadow_curve: Option<Handle<CurvedAnimation>>,
}

impl CupertinoPageTransitionState {
    fn dispose_curve(self: Handle<Self>, app: &mut App) {
        let state = app.get(self);
        let curves = [
            state.primary_position_curve,
            state.secondary_position_curve,
            state.primary_shadow_curve,
        ];
        for curve in curves.into_iter().flatten() {
            curve.dispose(app);
        }
        let state = app.get_mut(self);
        state.primary_position_curve = None;
        state.secondary_position_curve = None;
        state.primary_shadow_curve = None;
    }

    fn setup_animation(self: Handle<Self>, app: &mut App) {
        let widget = self.widget(app);
        let primary_route_animation = widget.primary_route_animation;
        let secondary_route_animation = widget.secondary_route_animation;
        if !widget.linear_transition {
            let primary_position_curve = CurvedAnimation::create(
                app,
                primary_route_animation,
                Curves::fast_ease_in_to_slow_ease_out(),
                Some(Rc::new(Curves::fast_ease_in_to_slow_ease_out().flipped())),
            );
            let secondary_position_curve = CurvedAnimation::create(
                app,
                secondary_route_animation,
                Curves::linear_to_ease_out(),
                Some(Curves::ease_in_to_linear()),
            );
            let primary_shadow_curve = CurvedAnimation::create(
                app,
                primary_route_animation,
                Curves::linear_to_ease_out(),
                None,
            );
            let state = app.get_mut(self);
            state.primary_position_curve = Some(primary_position_curve);
            state.secondary_position_curve = Some(secondary_position_curve);
            state.primary_shadow_curve = Some(primary_shadow_curve);
        }
        let primary_position_parent = match app.get(self).primary_position_curve {
            Some(curve) => curve.as_animation(),
            None => primary_route_animation,
        };
        let primary_position_animation = primary_position_parent.drive(app, K_RIGHT_MIDDLE_TWEEN);
        let secondary_position_parent = match app.get(self).secondary_position_curve {
            Some(curve) => curve.as_animation(),
            None => secondary_route_animation,
        };
        let secondary_position_animation =
            secondary_position_parent.drive(app, K_MIDDLE_LEFT_TWEEN);
        let primary_shadow_parent = match app.get(self).primary_shadow_curve {
            Some(curve) => curve.as_animation(),
            None => primary_route_animation,
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
    reveal_widgets::state_accessors!();

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
            .expect("init_state ran first");
        let primary_position_animation = state
            .primary_position_animation
            .expect("init_state ran first");
        let primary_shadow_animation = state
            .primary_shadow_animation
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
    pub primary_route_animation: AnyAnimation<f64>,

    /// A linear route animation from 0.0 to 1.0 when another screen is being pushed on top of
    /// this one.
    pub secondary_route_animation: AnyAnimation<f64>,

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
        primary_route_animation: AnyAnimation<f64>,
        secondary_route_animation: AnyAnimation<f64>,
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
    primary_position_animation: Option<AnyAnimation<Offset>>,
    /// When this page is becoming covered by another page.
    secondary_position_animation: Option<AnyAnimation<Offset>>,
    /// Curve of primary page which is coming in to cover another page.
    primary_position_curve: Option<Handle<CurvedAnimation>>,
    /// Curve of secondary page which is becoming covered by another page.
    secondary_position_curve: Option<Handle<CurvedAnimation>>,
}

impl CupertinoFullscreenDialogTransitionState {
    fn dispose_curve(self: Handle<Self>, app: &mut App) {
        let state = app.get(self);
        let curves = [state.primary_position_curve, state.secondary_position_curve];
        for curve in curves.into_iter().flatten() {
            curve.dispose(app);
        }
        let state = app.get_mut(self);
        state.primary_position_curve = None;
        state.secondary_position_curve = None;
    }

    fn setup_animation(self: Handle<Self>, app: &mut App) {
        let widget = self.widget(app);
        let primary_route_animation = widget.primary_route_animation;
        let secondary_route_animation = widget.secondary_route_animation;
        let linear_transition = widget.linear_transition;
        let primary_position_curve = CurvedAnimation::create(
            app,
            primary_route_animation,
            Curves::linear_to_ease_out(),
            // The curve must be flipped so that the reverse animation doesn't play an ease-in
            // curve, which iOS does not use.
            Some(Rc::new(Curves::linear_to_ease_out().flipped())),
        );
        app.get_mut(self).primary_position_curve = Some(primary_position_curve);
        let primary_position_animation = primary_position_curve
            .as_animation()
            .drive(app, K_BOTTOM_UP_TWEEN);
        let secondary_parent = if linear_transition {
            secondary_route_animation
        } else {
            let secondary_position_curve = CurvedAnimation::create(
                app,
                secondary_route_animation,
                Curves::linear_to_ease_out(),
                Some(Curves::ease_in_to_linear()),
            );
            app.get_mut(self).secondary_position_curve = Some(secondary_position_curve);
            secondary_position_curve.as_animation()
        };
        let secondary_position_animation = secondary_parent.drive(app, K_MIDDLE_LEFT_TWEEN);
        let state = app.get_mut(self);
        state.primary_position_animation = Some(primary_position_animation);
        state.secondary_position_animation = Some(secondary_position_animation);
    }
}

impl State for CupertinoFullscreenDialogTransitionState {
    type Widget = CupertinoFullscreenDialogTransition;
    reveal_widgets::state_accessors!();

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
            .expect("init_state ran first");
        let primary_position_animation = state
            .primary_position_animation
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
type StartPopGestureCallback = Rc<dyn Fn(&mut App) -> Handle<CupertinoBackGestureController>>;

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
    key: Option<KeyRef>,
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

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

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
    back_gesture_controller: Option<Handle<CupertinoBackGestureController>>,
    recognizer: Option<Handle<HorizontalDragGestureRecognizer>>,
}

impl CupertinoBackGestureDetectorState {
    fn recognizer(self: Handle<Self>, app: &App) -> Handle<HorizontalDragGestureRecognizer> {
        app.get(self).recognizer.expect("init_state ran first")
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
        if let Some(controller) = app.get(self).back_gesture_controller {
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
    reveal_widgets::state_accessors!();

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
            SchedulerBinding::add_post_frame_callback(
                app,
                FrameCallback::new(move |app, _timestamp| {
                    if let Some(controller) = app.get(self).back_gesture_controller {
                        let navigator = app.get(controller).navigator;
                        if navigator.mounted(app) {
                            navigator.did_stop_user_gesture(app);
                        }
                    }
                    app.get_mut(self).back_gesture_controller = None;
                }),
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
        let listener = reveal_widgets::Listener::new()
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
    controller: Handle<AnimationController>,
    navigator: Handle<NavigatorState>,
    get_is_active: RouteFlagGetter,
    get_is_current: RouteFlagGetter,
}

impl CupertinoBackGestureController {
    /// Creates a controller for an iOS-style back gesture.
    fn new(
        app: &mut App,
        navigator: Handle<NavigatorState>,
        controller: Handle<AnimationController>,
        get_is_current: RouteFlagGetter,
        get_is_active: RouteFlagGetter,
    ) -> Handle<CupertinoBackGestureController> {
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
        let controller = app.get(self).controller;
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
        let controller = app.get(self).controller;
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
            Animation::add_status_listener(
                controller,
                app,
                AnimationStatusListener::handle_method(
                    self,
                    CupertinoBackGestureController::handle_animation_status,
                ),
            );
        } else {
            let navigator = app.get(self).navigator;
            navigator.did_stop_user_gesture(app);
        }
    }

    fn handle_animation_status(self: Handle<Self>, app: &mut App, _status: AnimationStatus) {
        let navigator = app.get(self).navigator;
        navigator.did_stop_user_gesture(app);
        let controller = app.get(self).controller;
        Animation::remove_status_listener(
            controller,
            app,
            &AnimationStatusListener::handle_method(
                self,
                CupertinoBackGestureController::handle_animation_status,
            ),
        );
    }
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
            Some(Box::new(CupertinoEdgeShadowDecoration::new(None))),
            // Eyeballed gradient used to mimic a drop shadow on the start side only.
            Some(Box::new(CupertinoEdgeShadowDecoration::new(Some(vec![
                Color::new(0x04000000),
                CupertinoColors::TRANSPARENT.color(),
            ])))),
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
    fn lerp_from(&self, a: Option<&dyn Decoration>, t: f64) -> Option<Box<dyn Decoration>> {
        let a = a.and_then(|a| a.as_any().downcast_ref::<CupertinoEdgeShadowDecoration>());
        CupertinoEdgeShadowDecoration::lerp(a, Some(self), t)
            .map(|decoration| Box::new(decoration) as Box<dyn Decoration>)
    }

    fn lerp_to(&self, b: Option<&dyn Decoration>, t: f64) -> Option<Box<dyn Decoration>> {
        let b = b.and_then(|b| b.as_any().downcast_ref::<CupertinoEdgeShadowDecoration>());
        CupertinoEdgeShadowDecoration::lerp(Some(self), b, t)
            .map(|decoration| Box::new(decoration) as Box<dyn Decoration>)
    }

    fn create_box_painter(&self, on_changed: Option<Box<dyn Fn()>>) -> Box<dyn BoxPainter> {
        Box::new(CupertinoEdgeShadowPainter::new(self.clone(), on_changed))
    }

    fn clone_box(&self) -> Box<dyn Decoration> {
        Box::new(self.clone())
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
    on_changed: Option<Box<dyn Fn()>>,
}

impl CupertinoEdgeShadowPainter {
    fn new(
        decoration: CupertinoEdgeShadowDecoration,
        on_changed: Option<Box<dyn Fn()>>,
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

    fn on_changed(&self) -> Option<&dyn Fn()> {
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
/// It is used internally by [`show_cupertino_modal_popup`] or can be directly pushed onto the
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
    const OFFSET_TWEEN: OffsetTween = OffsetTween::new(Offset::new(0.0, 1.0), Offset::ZERO);

    /// A route that shows a modal iOS-style popup that slides up from the bottom of the screen;
    /// Dart's optional arguments are the setters.
    ///
    /// Dart's defaults: `barrier_label` "Dismiss", `barrier_color`
    /// [`K_CUPERTINO_MODAL_BARRIER_COLOR`], `barrier_dismissible` true, `semantics_dismissible`
    /// false.
    pub fn new(app: &mut App, builder: WidgetBuilder) -> Handle<CupertinoModalPopupRoute> {
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
    pub fn barrier_label(self: Handle<Self>, app: &mut App, barrier_label: String) -> Handle<Self> {
        app.get_mut(self).barrier_label = barrier_label;
        self
    }

    /// Dart `CupertinoModalPopupRoute(barrierColor:)`.
    pub fn barrier_color(
        self: Handle<Self>,
        app: &mut App,
        barrier_color: Option<AnyColor>,
    ) -> Handle<Self> {
        app.get_mut(self).barrier_color = barrier_color;
        self
    }

    /// Dart `CupertinoModalPopupRoute(barrierDismissible:)`.
    pub fn barrier_dismissible(
        self: Handle<Self>,
        app: &mut App,
        barrier_dismissible: bool,
    ) -> Handle<Self> {
        app.get_mut(self).barrier_dismissible = barrier_dismissible;
        self
    }

    /// Dart `CupertinoModalPopupRoute(semanticsDismissible:)`.
    pub fn semantics_dismissible(
        self: Handle<Self>,
        app: &mut App,
        semantics_dismissible: bool,
    ) -> Handle<Self> {
        app.get_mut(self).semantics_dismissible = semantics_dismissible;
        self
    }

    /// Dart `CupertinoModalPopupRoute(filter:)`.
    pub fn filter(self: Handle<Self>, app: &mut App, filter: ImageFilter) -> Handle<Self> {
        self.modal_route_data_mut(app).filter = Some(ImageFilterConfig::new(filter));
        self
    }

    /// Dart `CupertinoModalPopupRoute(settings:)`.
    pub fn settings(self: Handle<Self>, app: &mut App, settings: RouteSettingsRef) -> Handle<Self> {
        self.route_data_mut(app).set_settings(settings);
        self
    }

    /// Dart `CupertinoModalPopupRoute(requestFocus:)`.
    pub fn request_focus(self: Handle<Self>, app: &mut App, request_focus: bool) -> Handle<Self> {
        self.route_data_mut(app)
            .set_request_focus(Some(request_focus));
        self
    }
}

impl Route for CupertinoModalPopupRoute {
    reveal_widgets::modal_route_overrides!();
}

impl OverlayRoute for CupertinoModalPopupRoute {
    reveal_widgets::modal_overlay_route_overrides!();
}

impl PredictiveBackRoute for CupertinoModalPopupRoute {
    reveal_widgets::modal_predictive_back_route_overrides!(ModalRoute::pop_gesture_enabled);
}

impl TransitionRoute for CupertinoModalPopupRoute {
    reveal_widgets::modal_transition_route_overrides!();

    fn transition_duration(self: Handle<Self>, app: &App) -> Duration {
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
    reveal_widgets::local_history_route_accessors!();
}

impl ModalRoute for CupertinoModalPopupRoute {
    reveal_widgets::modal_route_accessors!();

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
        animation: AnyAnimation<f64>,
        secondary_animation: AnyAnimation<f64>,
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
        animation: AnyAnimation<f64>,
        secondary_animation: AnyAnimation<f64>,
        child: WidgetRef,
    ) -> WidgetRef {
        let _ = (context, secondary_animation);
        let translation = CupertinoModalPopupRoute::OFFSET_TWEEN.evaluate(app, animation);
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
#[expect(
    clippy::too_many_arguments,
    reason = "Dart's named arguments; a free function has no receiver for the fluent setters"
)]
pub fn show_cupertino_modal_popup(
    app: &mut App,
    context: BuildContext,
    builder: WidgetBuilder,
    filter: Option<ImageFilter>,
    barrier_color: AnyColor,
    barrier_dismissible: bool,
    use_root_navigator: bool,
    semantics_dismissible: bool,
    route_settings: Option<RouteSettingsRef>,
    request_focus: Option<bool>,
) -> CompleterFuture<RouteResult> {
    let resolved_barrier_color = CupertinoDynamicColor::resolve(&barrier_color, app, context);
    let route = CupertinoModalPopupRoute::new(app, builder)
        .barrier_color(app, Some(resolved_barrier_color))
        .barrier_dismissible(app, barrier_dismissible)
        .semantics_dismissible(app, semantics_dismissible);
    if let Some(filter) = filter {
        route.filter(app, filter);
    }
    if let Some(route_settings) = route_settings {
        route.settings(app, route_settings);
    }
    if let Some(request_focus) = request_focus {
        route.request_focus(app, request_focus);
    }
    let navigator = Navigator::of(app, context, use_root_navigator);
    navigator.push(app, Route::as_route(route))
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
    _animation: AnyAnimation<f64>,
    _secondary_animation: AnyAnimation<f64>,
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
/// `builder` does not share a context with the location that [`show_cupertino_dialog`] is
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
#[expect(
    clippy::too_many_arguments,
    reason = "Dart's named arguments; a free function has no receiver for the fluent setters"
)]
pub fn show_cupertino_dialog(
    app: &mut App,
    context: BuildContext,
    builder: WidgetBuilder,
    barrier_label: Option<String>,
    barrier_color: Option<Color>,
    use_root_navigator: bool,
    barrier_dismissible: bool,
    route_settings: Option<RouteSettingsRef>,
    request_focus: Option<bool>,
) -> CompleterFuture<RouteResult> {
    let route = CupertinoDialogRoute::new(app, builder, context)
        .barrier_dismissible(app, barrier_dismissible);
    if let Some(barrier_label) = barrier_label {
        route.barrier_label(app, barrier_label);
    }
    if let Some(barrier_color) = barrier_color {
        route.barrier_color(app, Some(barrier_color));
    }
    if let Some(route_settings) = route_settings {
        route.settings(app, route_settings);
    }
    if let Some(request_focus) = request_focus {
        route.request_focus(app, request_focus);
    }
    let navigator = Navigator::of(app, context, use_root_navigator);
    navigator.push(app, Route::as_route(route))
}

/// A dialog route that shows an iOS-style dialog.
///
/// It is used internally by [`show_cupertino_dialog`] or can be directly pushed onto the
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
///  * [`show_cupertino_dialog`], which is a way to display an iOS-style dialog.
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
    // The curve and initial scale values were mostly eyeballed from iOS, however they reuse the
    // same animation curve that was modeled after native page transitions.
    dialog_scale_tween: Handle<Tween<f64>>,
}

impl CupertinoDialogRoute {
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
    ) -> Handle<CupertinoDialogRoute> {
        let barrier_label =
            <dyn CupertinoLocalizations>::of(app, context).modal_barrier_dismiss_label();
        let barrier_color =
            CupertinoDynamicColor::resolve(&K_CUPERTINO_MODAL_BARRIER_COLOR, app, context).color();
        let route = RouteData::new(app, None, None);
        let transition_route = TransitionRouteData::new(app);
        let modal_route = ModalRouteData::new(app);
        let dialog_scale_tween = Tween::new(app, Some(1.3), Some(1.0));
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
            dialog_scale_tween,
        })
    }

    /// Dart `CupertinoDialogRoute(barrierDismissible:)`.
    pub fn barrier_dismissible(
        self: Handle<Self>,
        app: &mut App,
        barrier_dismissible: bool,
    ) -> Handle<Self> {
        self.raw_dialog_route_data_mut(app).barrier_dismissible = barrier_dismissible;
        self
    }

    /// Dart `CupertinoDialogRoute(barrierColor:)`.
    pub fn barrier_color(
        self: Handle<Self>,
        app: &mut App,
        barrier_color: Option<Color>,
    ) -> Handle<Self> {
        self.raw_dialog_route_data_mut(app).barrier_color = barrier_color;
        self
    }

    /// Dart `CupertinoDialogRoute(barrierLabel:)`.
    pub fn barrier_label(self: Handle<Self>, app: &mut App, barrier_label: String) -> Handle<Self> {
        self.raw_dialog_route_data_mut(app).barrier_label = Some(barrier_label);
        self
    }

    /// Dart `CupertinoDialogRoute(transitionDuration:)`.
    pub fn transition_duration(
        self: Handle<Self>,
        app: &mut App,
        transition_duration: Duration,
    ) -> Handle<Self> {
        self.raw_dialog_route_data_mut(app).transition_duration = transition_duration;
        self
    }

    /// Dart `CupertinoDialogRoute(transitionBuilder:)`, which Dart also hands to
    /// `RawDialogRoute` in place of the jump-cut fallback.
    pub fn transition_builder(
        self: Handle<Self>,
        app: &mut App,
        transition_builder: RouteTransitionsBuilder,
    ) -> Handle<Self> {
        self.raw_dialog_route_data_mut(app).transition_builder =
            Some(Rc::clone(&transition_builder));
        app.get_mut(self).transition_builder = Some(transition_builder);
        self
    }

    /// Dart `CupertinoDialogRoute(settings:)`.
    pub fn settings(self: Handle<Self>, app: &mut App, settings: RouteSettingsRef) -> Handle<Self> {
        self.route_data_mut(app).set_settings(settings);
        self
    }

    /// Dart `CupertinoDialogRoute(requestFocus:)`.
    pub fn request_focus(self: Handle<Self>, app: &mut App, request_focus: bool) -> Handle<Self> {
        self.route_data_mut(app)
            .set_request_focus(Some(request_focus));
        self
    }
}

impl Route for CupertinoDialogRoute {
    reveal_widgets::modal_route_overrides!();
}

impl OverlayRoute for CupertinoDialogRoute {
    reveal_widgets::modal_overlay_route_overrides!();
}

impl PredictiveBackRoute for CupertinoDialogRoute {
    reveal_widgets::modal_predictive_back_route_overrides!(ModalRoute::pop_gesture_enabled);
}

impl TransitionRoute for CupertinoDialogRoute {
    reveal_widgets::raw_dialog_route_transition_route_overrides!();

    fn create_simulation(
        self: Handle<Self>,
        app: &mut App,
        forward: bool,
    ) -> Option<Box<dyn Simulation>> {
        create_standard_simulation(self, app, forward)
    }
}

impl LocalHistoryRoute for CupertinoDialogRoute {
    reveal_widgets::local_history_route_accessors!();
}

impl ModalRoute for CupertinoDialogRoute {
    reveal_widgets::raw_dialog_route_modal_route_overrides!();
}

impl PopupRoute for CupertinoDialogRoute {}

impl RawDialogRouteLeaf for CupertinoDialogRoute {
    reveal_widgets::raw_dialog_route_accessors!();

    fn build_transitions(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        animation: AnyAnimation<f64>,
        secondary_animation: AnyAnimation<f64>,
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
        let scale = animation.drive(app, app.get(self).dialog_scale_tween);
        FadeTransition::new(animation)
            .child(ScaleTransition::new(scale).child(child))
            .into_widget()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use reveal_embedder::{
        Locale, PointerChange, PointerData, PointerDataPacket, PointerDeviceKind,
    };
    use reveal_gestures::GestureBinding;
    use reveal_widgets::{
        AnyModalRoute, DefaultWidgetsLocalizations, GlobalKey, Localizations, Navigator,
        NavigatorState, PageRef, SizedBox,
    };

    use super::*;
    use crate::localizations::DefaultCupertinoLocalizations;
    use crate::test_support::{build, pump, test_cell};

    /// The test view is 800x600 physical at 2x.
    const VIEW_WIDTH: f64 = 400.0;

    /// Sends one pointer packet; the test view is 2x, so logical coordinates double.
    fn send(app: &mut App, change: PointerChange, x: f64, y: f64, at: Duration) {
        send_from(app, change, x, y, x, at);
    }

    /// Sends one pointer packet carrying the movement from `previous_x`.
    fn send_from(
        app: &mut App,
        change: PointerChange,
        x: f64,
        y: f64,
        previous_x: f64,
        at: Duration,
    ) {
        GestureBinding::instance(app).handle_pointer_data_packet(
            app,
            PointerDataPacket::new(vec![PointerData {
                change,
                kind: PointerDeviceKind::Touch,
                time_stamp: at,
                pointer_identifier: 1,
                physical_x: x * 2.0,
                physical_y: y * 2.0,
                physical_delta_x: (x - previous_x) * 2.0,
                ..PointerData::default()
            }]),
        );
        app.drain_microtasks();
    }

    fn tap_at(app: &mut App, x: f64, y: f64) {
        send(app, PointerChange::Down, x, y, Duration::ZERO);
        send(app, PointerChange::Up, x, y, Duration::from_millis(16));
    }

    /// Runs frames until every route transition has settled.
    fn settle(app: &mut App, from: Duration) -> Duration {
        let mut at = from;
        for _ in 0..60 {
            at += Duration::from_millis(20);
            pump(app, at);
        }
        at
    }

    /// A page filling the view, keyed so a test can find its render box.
    fn keyed_page(key: &GlobalKey) -> WidgetBuilder {
        let key: KeyRef = Rc::new(key.clone());
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
        let navigator = configure(Navigator::new().key(Rc::new(key.clone())));
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
    fn page_left(key: &GlobalKey, app: &mut App) -> f64 {
        let context = key.current_context(app).expect("the page is mounted");
        context
            .find_render_object(app)
            .expect("the page has a render object")
            .as_box()
            .expect("the page is a box")
            .local_to_global(app, Offset::ZERO, None)
            .dx()
    }

    /// The y coordinate the keyed page paints at, in the view's coordinate system.
    fn page_top(key: &GlobalKey, app: &mut App) -> f64 {
        let context = key.current_context(app).expect("the page is mounted");
        context
            .find_render_object(app)
            .expect("the page has a render object")
            .as_box()
            .expect("the page is a box")
            .local_to_global(app, Offset::ZERO, None)
            .dy()
    }

    #[test]
    fn a_pushed_cupertino_page_route_slides_in_from_the_right() {
        let cell = test_cell();
        let app = cell.borrow();
        let navigator_key = GlobalKey::new();
        drop(app);
        build(
            &cell,
            navigator(&navigator_key, |navigator| {
                navigator.on_generate_route(|app, _settings| {
                    Some(Route::as_route(CupertinoPageRoute::new(app, plain_page())))
                })
            }),
        );
        let mut app = cell.borrow_mut();
        let state = navigator_state(&navigator_key, &mut app);
        let at = settle(&mut app, Duration::ZERO);

        let page_key = GlobalKey::new();
        let pushed = CupertinoPageRoute::new(&mut app, keyed_page(&page_key));
        state.push(&mut app, Route::as_route(pushed));
        let mut at = at;
        for _ in 0..4 {
            at += Duration::from_millis(20);
            pump(&mut app, at);
        }
        let mid = page_left(&page_key, &mut app);
        assert!(
            mid > 0.0 && mid < VIEW_WIDTH,
            "the page is part way in from the right, at {mid}"
        );

        let at = settle(&mut app, at);
        assert_eq!(
            page_left(&page_key, &mut app),
            0.0,
            "the slide ends with the page fully on screen"
        );
        let animation = TransitionRoute::animation(pushed, &app).expect("an installed route");
        assert_eq!(animation.value(&app), 1.0);

        state.pop(&mut app, None);
        settle(&mut app, at);
        assert!(!Route::is_active(pushed, &app), "the route popped");
    }

    #[test]
    fn cupertino_pages_build_page_based_routes_that_carry_the_previous_title() {
        let cell = test_cell();
        let app = cell.borrow();
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
        drop(app);
        build(
            &cell,
            navigator(&navigator_key, move |navigator| {
                navigator.pages(pages).on_did_remove_page(|_app, _page| {})
            }),
        );
        let mut app = cell.borrow_mut();
        settle(&mut app, Duration::ZERO);

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
    }

    #[test]
    fn a_flung_back_gesture_drag_moves_the_page_and_pops_the_route() {
        let cell = test_cell();
        let app = cell.borrow();
        let navigator_key = GlobalKey::new();
        drop(app);
        build(
            &cell,
            navigator(&navigator_key, |navigator| {
                navigator.on_generate_route(|app, _settings| {
                    Some(Route::as_route(CupertinoPageRoute::new(app, plain_page())))
                })
            }),
        );
        let mut app = cell.borrow_mut();
        let state = navigator_state(&navigator_key, &mut app);
        let at = settle(&mut app, Duration::ZERO);

        let page_key = GlobalKey::new();
        let pushed = CupertinoPageRoute::new(&mut app, keyed_page(&page_key));
        state.push(&mut app, Route::as_route(pushed));
        let at = settle(&mut app, at);
        assert_eq!(page_left(&page_key, &mut app), 0.0);

        send(&mut app, PointerChange::Down, 5.0, 150.0, Duration::ZERO);
        for step in 1_u64..=4 {
            send_from(
                &mut app,
                PointerChange::Move,
                5.0 + 25.0 * step as f64,
                150.0,
                5.0 + 25.0 * (step - 1) as f64,
                Duration::from_millis(step * 12),
            );
            pump(&mut app, at + Duration::from_millis(step * 12));
        }
        let dragged = page_left(&page_key, &mut app);
        assert!(dragged > 0.0, "the drag moved the page right, to {dragged}");

        send(
            &mut app,
            PointerChange::Up,
            105.0,
            150.0,
            Duration::from_millis(48),
        );
        settle(&mut app, at + Duration::from_millis(48));
        assert!(
            !Route::is_active(pushed, &app),
            "the fling popped the route"
        );
    }

    #[test]
    fn a_dropped_short_back_gesture_drag_snaps_the_page_back() {
        let cell = test_cell();
        let app = cell.borrow();
        let navigator_key = GlobalKey::new();
        drop(app);
        build(
            &cell,
            navigator(&navigator_key, |navigator| {
                navigator.on_generate_route(|app, _settings| {
                    Some(Route::as_route(CupertinoPageRoute::new(app, plain_page())))
                })
            }),
        );
        let mut app = cell.borrow_mut();
        let state = navigator_state(&navigator_key, &mut app);
        let at = settle(&mut app, Duration::ZERO);

        let page_key = GlobalKey::new();
        let pushed = CupertinoPageRoute::new(&mut app, keyed_page(&page_key));
        state.push(&mut app, Route::as_route(pushed));
        let at = settle(&mut app, at);

        send(&mut app, PointerChange::Down, 5.0, 150.0, Duration::ZERO);
        for step in 1_u64..=4 {
            send_from(
                &mut app,
                PointerChange::Move,
                5.0 + 5.0 * step as f64,
                150.0,
                5.0 + 5.0 * (step - 1) as f64,
                Duration::from_millis(step * 60),
            );
            pump(&mut app, at + Duration::from_millis(step * 60));
        }
        let dragged = page_left(&page_key, &mut app);
        assert!(dragged > 0.0, "the drag moved the page right, to {dragged}");
        send(
            &mut app,
            PointerChange::Up,
            25.0,
            150.0,
            Duration::from_millis(240),
        );
        settle(&mut app, at + Duration::from_millis(240));

        assert!(Route::is_active(pushed, &app), "the short drag did not pop");
        assert_eq!(
            page_left(&page_key, &mut app),
            0.0,
            "the page snapped back on screen"
        );
    }

    #[test]
    fn a_modal_popup_slides_up_from_the_bottom_and_dismisses_on_a_barrier_tap() {
        let cell = test_cell();
        let app = cell.borrow();
        let navigator_key = GlobalKey::new();
        let page_context: Rc<Cell<Option<BuildContext>>> = Rc::default();
        drop(app);
        build(&cell, {
            let page_context = Rc::clone(&page_context);
            navigator(&navigator_key, move |navigator| {
                navigator.on_generate_route(move |app, _settings| {
                    Some(Route::as_route(CupertinoPageRoute::new(
                        app,
                        context_page(Rc::clone(&page_context)),
                    )))
                })
            })
        });
        let mut app = cell.borrow_mut();
        let at = settle(&mut app, Duration::ZERO);

        let popup_key = GlobalKey::new();
        let key: KeyRef = Rc::new(popup_key.clone());
        let context = page_context.get().expect("the page built");
        let popped = show_cupertino_modal_popup(
            &mut app,
            context,
            Rc::new(move |_app, _context| {
                SizedBox::new()
                    .key(key.clone())
                    .width(200.0)
                    .height(100.0)
                    .into_widget()
            }),
            None,
            K_CUPERTINO_MODAL_BARRIER_COLOR,
            true,
            true,
            false,
            None,
            None,
        );
        let mut at = at;
        for _ in 0..3 {
            at += Duration::from_millis(20);
            pump(&mut app, at);
        }
        let mid = page_top(&popup_key, &mut app);
        assert!(mid > 200.0, "the popup starts below the view, at {mid}");

        let at = settle(&mut app, at);
        assert_eq!(
            page_top(&popup_key, &mut app),
            200.0,
            "the popup ends flush with the bottom of the 300pt view"
        );

        tap_at(&mut app, 200.0, 20.0);
        settle(&mut app, at);
        assert!(
            matches!(popped.peek(), Some(None)),
            "the barrier tap dismissed the popup with no result"
        );
    }

    #[test]
    fn a_cupertino_dialog_fades_in() {
        let cell = test_cell();
        let app = cell.borrow();
        let navigator_key = GlobalKey::new();
        let page_context: Rc<Cell<Option<BuildContext>>> = Rc::default();
        drop(app);
        build(&cell, {
            let page_context = Rc::clone(&page_context);
            navigator(&navigator_key, move |navigator| {
                navigator.on_generate_route(move |app, _settings| {
                    Some(Route::as_route(CupertinoPageRoute::new(
                        app,
                        context_page(Rc::clone(&page_context)),
                    )))
                })
            })
        });
        let mut app = cell.borrow_mut();
        let at = settle(&mut app, Duration::ZERO);

        let dialog_context: Rc<Cell<Option<BuildContext>>> = Rc::default();
        let context = page_context.get().expect("the page built");
        let popped = show_cupertino_dialog(
            &mut app,
            context,
            context_page(Rc::clone(&dialog_context)),
            None,
            None,
            true,
            false,
            None,
            None,
        );
        let mut at = at;
        for _ in 0..2 {
            at += Duration::from_millis(20);
            pump(&mut app, at);
        }
        let dialog_context = dialog_context.get().expect("the dialog built");
        let opacity = dialog_context
            .find_ancestor_widget_of_exact_type::<FadeTransition>(&app)
            .expect("the dialog is wrapped in a FadeTransition")
            .opacity;
        let value = opacity.value(&app);
        assert!(
            value > 0.0 && value < 1.0,
            "the dialog is fading in at {value}"
        );

        settle(&mut app, at);
        assert_eq!(opacity.value(&app), 1.0, "the fade completes");
        assert!(!popped.is_completed(), "the dialog is still up");
    }

    #[test]
    fn the_pop_gesture_is_disabled_for_the_first_route_and_for_a_fullscreen_dialog() {
        let cell = test_cell();
        let app = cell.borrow();
        let navigator_key = GlobalKey::new();
        let first = Rc::new(Cell::new(None));
        drop(app);
        build(&cell, {
            let first = Rc::clone(&first);
            navigator(&navigator_key, move |navigator| {
                navigator.on_generate_route(move |app, _settings| {
                    let route = CupertinoPageRoute::new(app, plain_page());
                    first.set(Some(route));
                    Some(Route::as_route(route))
                })
            })
        });
        let mut app = cell.borrow_mut();
        let state = navigator_state(&navigator_key, &mut app);
        let at = settle(&mut app, Duration::ZERO);
        let first = first.get().expect("the initial route was generated");
        assert!(
            !PredictiveBackRoute::pop_gesture_enabled(first, &mut app),
            "the first route has nothing to go back to"
        );

        let dialog =
            CupertinoPageRoute::new(&mut app, plain_page()).fullscreen_dialog(&mut app, true);
        state.push(&mut app, Route::as_route(dialog));
        let at = settle(&mut app, at);
        assert!(
            !PredictiveBackRoute::pop_gesture_enabled(dialog, &mut app),
            "a fullscreen dialog is not dismissible by back swipe"
        );

        state.pop(&mut app, None);
        let at = settle(&mut app, at);
        let second = CupertinoPageRoute::new(&mut app, plain_page());
        state.push(&mut app, Route::as_route(second));
        settle(&mut app, at);
        assert!(
            PredictiveBackRoute::pop_gesture_enabled(second, &mut app),
            "a plain second page route can be swiped back"
        );
    }
}
