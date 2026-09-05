//! Flutter counterpart: `cupertino/sheet.dart`.

use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use reveal_animation::{
    Animatable, Animation, AnimationBehavior, AnimationController, AnimationStatus,
    AnimationStatusListener, AnyAnimation, CurvedAnimation, Curves,
};
use reveal_embedder::{Brightness, Color, Offset, Radius, SystemUiOverlayStyle};
use reveal_foundation::{App, ChangeNotifier, ChangeNotifierData, Handle, Listener};
use reveal_gestures::{
    Drag, DragEndDetails, DragGestureRecognizer, DragStartDetails, DragUpdateDetails,
    IOSScrollViewFlingVelocityTracker, PointerDownEvent, PointerEvent, RecognizerLeaf,
    VerticalDragGestureRecognizer,
};
use reveal_painting::{
    Alignment, AlignmentGeometry, BorderRadius, BorderRadiusGeometry, BorderSide, EdgeInsets,
    EdgeInsetsGeometry, RoundedSuperellipseBorder, ShapeDecoration,
};
use reveal_rendering::{HitTestBehavior, StackFit, ViewportOffset};
use reveal_scheduler::{
    FrameCallback, SchedulerBinding, Ticker, TickerCallback, TickerProviderObject,
};
use reveal_widgets::{
    Align, AlwaysScrollableScrollPhysics, AnimatedBuilder, AnnotatedRegion, AnyRoute,
    AnyScrollActivity, AnyScrollController, AnyScrollPosition, AnyTransitionRoute, BuildContext,
    ClipRSuperellipse, ColoredBox, DecoratedBox, DelegatedTransitionBuilder, FadeTransition,
    GlobalKey, InheritedWidget, IntoWidget, KeyRef, LocalHistoryRoute, LocalHistoryRouteData,
    MediaQuery, ModalRoute, ModalRouteData, Navigator, NavigatorPopHandler, NavigatorState,
    OverlayRoute, OverlayRouteData, Padding, PageRoute, PageRouteData, PopScope,
    PredictiveBackRoute, Route, RouteData, RouteResult, RouteSettingsRef, ScaleTransition,
    ScrollActivityDelegate, ScrollContext, ScrollControllerData, ScrollControllerLeaf,
    ScrollPhysicsRef, ScrollPosition, ScrollPositionData, ScrollPositionWithSingleContext,
    ScrollPositionWithSingleContextData, ScrollPositionWithSingleContextLeaf,
    ScrollableWidgetBuilder, SingleTickerProviderStateMixin, SingleTickerProviderStateMixinData,
    SizedBox, SlideTransition, Stack, State, StateData, StatefulWidget, TransitionRoute,
    TransitionRouteData, WidgetBuilder, WidgetRef,
};

use crate::colors::CupertinoColors;
use crate::interface_level::{CupertinoUserInterfaceLevel, CupertinoUserInterfaceLevelData};
use crate::route::CupertinoPageRoute;
use crate::theme::CupertinoTheme;

// Smoothing factor applied to the device's top padding (which approximates the corner radius)
// to achieve a smoother end to the corner radius animation.  A value of 1.0 would use
// the full top padding. Values less than 1.0 reduce the effective corner radius, improving
// the animation's appearance.  Determined through empirical testing.
const K_DEVICE_CORNER_RADIUS_SMOOTHING_FACTOR: f64 = 0.9;

// Threshold in logical pixels. If the calculated device corner radius (after applying
// the smoothing factor) is below this value, the corner radius transition animation will
// start from zero. This prevents abrupt transitions for devices with small or negligible
// corner radii.  This value, combined with the smoothing factor, corresponds roughly
// to double the targeted radius of 12.  Determined through testing and visual inspection.
const K_ROUNDED_DEVICE_CORNERS_THRESHOLD: f64 = 20.0;

// The distance from the top of the open sheet to the top of the screen, as a ratio
// of the total height of the screen. Found from eyeballing a simulator running
// iOS 18.0.
const K_TOP_GAP_RATIO: f64 = 0.08;

// The minimum distance (i.e., maximum upward stretch) from the top of the sheet
// to the top of the screen, as a ratio of total screen height. This value represents
// how far the sheet can be temporarily pulled upward before snapping back.
// Determined through visual tuning to feel natural on <iPhone16, iPhone 16 Pro>
// running iOS 18.0 simulators.
const K_STRETCHED_TOP_GAP_RATIO: f64 = 0.072;

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

/// Dart's `Tween<double>`, as a value; see [`OffsetTween`].
#[derive(Clone, Copy, Debug)]
struct DoubleTween {
    begin: f64,
    end: f64,
}

impl DoubleTween {
    const fn new(begin: f64, end: f64) -> DoubleTween {
        DoubleTween { begin, end }
    }
}

impl Animatable<f64> for DoubleTween {
    fn transform(&self, _app: &App, t: f64) -> f64 {
        self.begin + (self.end - self.begin) * t
    }
}

/// Dart's `Tween<BorderRadiusGeometry>`, as a value; see [`OffsetTween`].
#[derive(Clone, Copy, Debug)]
struct BorderRadiusGeometryTween {
    begin: BorderRadiusGeometry,
    end: BorderRadiusGeometry,
}

impl BorderRadiusGeometryTween {
    const fn new(
        begin: BorderRadiusGeometry,
        end: BorderRadiusGeometry,
    ) -> BorderRadiusGeometryTween {
        BorderRadiusGeometryTween { begin, end }
    }
}

impl Animatable<BorderRadiusGeometry> for BorderRadiusGeometryTween {
    fn transform(&self, _app: &App, t: f64) -> BorderRadiusGeometry {
        if t == 0.0 {
            return self.begin;
        }
        if t == 1.0 {
            return self.end;
        }
        BorderRadiusGeometry::lerp(Some(self.begin), Some(self.end), t).expect("both ends are set")
    }
}

// Tween for animating a Cupertino sheet onto the screen.
//
// Begins fully offscreen below the screen and ends onscreen with a small gap at
// the top of the screen. Values found from eyeballing a simulator running iOS 18.0.
const K_BOTTOM_UP_TWEEN: OffsetTween = OffsetTween::new(Offset::new(0.0, 1.0), Offset::ZERO);

// Offset change for when a new sheet covers another sheet. '0.0' represents the
// top of the space available for the new sheet, but because the previous sheet
// was lowered slightly, the new sheet needs to go slightly higher than that.
// Values found from eyeballing a simulator running iOS 18.0.
const K_BOTTOM_UP_TWEEN_WHEN_COVERING_OTHER_SHEET: OffsetTween =
    OffsetTween::new(Offset::new(0.0, 1.0), Offset::new(0.0, -0.02));

// Tween that animates a sheet slightly up when it is covered by a new sheet.
// Values found from eyeballing a simulator running iOS 18.0.
const K_MID_UP_TWEEN: OffsetTween = OffsetTween::new(Offset::ZERO, Offset::new(0.0, -0.005));

// Offset from top of screen to slightly down when a fullscreen page is covered
// by a sheet. Values found from eyeballing a simulator running iOS 18.0.
const K_TOP_DOWN_TWEEN: OffsetTween = OffsetTween::new(Offset::ZERO, Offset::new(0.0, 0.07));

// Opacity of the overlay color put over the sheet as it moves into the background.
// Used to distinguish the sheet from the background. Value derived from eyeballing
// a simulator running iOS 18.0.
const K_OPACITY_TWEEN: DoubleTween = DoubleTween::new(0.0, 0.10);

// The minimum velocity needed for a drag downwards to dismiss the sheet. Eyeballed
// from a comparison against a simulator running iOS 18.0.
/// Screen heights per second.
const K_MIN_FLING_VELOCITY: f64 = 2.0;

// The duration for a page to animate when the user releases it mid-swipe. Eyeballed
// from a comparison against a simulator running iOS 18.0.
const K_DROPPED_SHEET_DRAG_ANIMATION_DURATION: Duration = Duration::from_millis(300);

// Amount the sheet in the background scales down. Found by measuring the width
// of the sheet in the background and comparing against the screen width on the
// iOS simulator showing an iPhone 16 pro running iOS 18.0. The scale transition
// will go from a default of 1.0 to 1.0 - _kSheetScaleFactor.
const K_SHEET_SCALE_FACTOR: f64 = 0.0835;

const K_SCALE_TWEEN: DoubleTween = DoubleTween::new(1.0, 1.0 - K_SHEET_SCALE_FACTOR);

/// Dart's `ValueGetter<bool>` on a route flag.
type RouteFlagGetter = Rc<dyn Fn(&App) -> bool>;

/// Dart's `ValueGetter<_CupertinoDragGestureController<T>>`.
type StartPopGestureCallback = Rc<dyn Fn(&mut App) -> Handle<CupertinoDragGestureController>>;

/// Dart's `ValueGetter<bool>` on the route's `enableDrag`.
type DragEnabledCallback = Rc<dyn Fn() -> bool>;

/// The signature for a method called on the start of a drag.
type DragStartCallback = Rc<dyn Fn(&mut App)>;

/// The signature for a method called to trigger a change based on a moving drag gesture.
type DragUpdateCallback = Rc<dyn Fn(&mut App, f64)>;

/// The signature for a method called on the end of a drag, passing the velocity at
/// the end of the drag along.
type DragEndCallback = Rc<dyn Fn(&mut App, f64)>;

/// The signature for a method that checks if the sheet is currently dragged downwards.
type GetSheetDragged = Rc<dyn Fn(&App) -> bool>;

/// Shows a Cupertino-style sheet widget that slides up from the bottom of the
/// screen and stacks the previous route behind the new sheet.
///
/// This is a convenience method for displaying [`CupertinoSheetRoute`] for most
/// use cases. The widget returned from `scrollable_builder` will be used to display
/// the content on the [`CupertinoSheetRoute`]. If the content of the sheet has a
/// scrollable view, the scroll controller provided by `scrollable_builder` can be
/// used to enable the drag-to-dismiss gesture to work with the scrolling of the
/// content. See [`CupertinoSheetRoute::scrollable_builder`] for an example.
///
/// `use_nested_navigation` allows new routes to be pushed inside of a
/// [`CupertinoSheetRoute`] by adding a new [`Navigator`] inside of the
/// [`CupertinoSheetRoute`].
///
/// When `use_nested_navigation` is `true`, any route pushed to the stack
/// from within the context of the [`CupertinoSheetRoute`] will display within that
/// sheet. System back gestures and programmatic pops on the initial route in a
/// sheet will also be intercepted to pop the whole [`CupertinoSheetRoute`]. If
/// a custom [`Navigator`] setup is needed, like for example to enable named routes
/// or the pages API, then it is recommended to directly push a [`CupertinoSheetRoute`]
/// to the stack with whatever configuration needed.
///
/// The whole sheet can be popped at once by either dragging down on the sheet,
/// or calling [`CupertinoSheetRoute::pop_sheet`].
///
/// When `enable_drag` is `true` (Dart's default), users can dismiss the sheet
/// by dragging it down or by calling [`CupertinoSheetRoute::pop_sheet`]. When
/// `enable_drag` is `false`, users cannot dismiss the sheet by dragging, and it
/// can only be closed by calling [`CupertinoSheetRoute::pop_sheet`].
///
/// The `top_gap` parameter can be used to customize the gap between the top of
/// the screen and the top of the sheet as a ratio of the screen height.
/// It should be a value between 0.0 and 0.9, where 0.0 means no gap and 0.9
/// means the sheet takes up only the bottom 10% of the screen. If not provided, defaults
/// to 0.08 (8% of screen height).
///
/// When `show_drag_handle` is `true`, then a drag handle will be placed at
/// the top of the sheet. This flag defaults to false.
///
/// iOS sheet widgets are generally designed to be tightly coupled to the context
/// of the widget that opened the sheet. As such, it is not recommended to push
/// a non-sheet route that covers the sheet without first popping the sheet. If
/// necessary however, it can be done by pushing to the root [`Navigator`].
///
/// If `use_nested_navigation` is `false` (Dart's default), then a [`CupertinoSheetRoute`]
/// will be shown with no [`Navigator`] widget. Multiple calls to [`show_cupertino_sheet`]
/// can still be made to show multiple stacked sheets, if desired.
///
/// [`show_cupertino_sheet`] always pushes the [`CupertinoSheetRoute`] to the root
/// [`Navigator`]. This is to ensure the previous route animates correctly.
///
/// The returned route is the pushed [`CupertinoSheetRoute`]; register a callback on
/// `AnyRoute::when_popped` for the value (if any) that was passed to `NavigatorState::pop` when
/// the sheet was closed.
///
/// See also:
///
///  * [`CupertinoSheetRoute`] the basic route version of the sheet view.
///  * [`show_cupertino_dialog`](crate::show_cupertino_dialog) which displays an iOS-styled
///    dialog.
///  * <https://developer.apple.com/design/human-interface-guidelines/sheets>
#[expect(
    clippy::too_many_arguments,
    reason = "Dart's named arguments, in Dart's order"
)]
pub fn show_cupertino_sheet(
    app: &mut App,
    context: BuildContext,
    builder: Option<WidgetBuilder>,
    scrollable_builder: Option<ScrollableWidgetBuilder>,
    use_nested_navigation: bool,
    enable_drag: bool,
    settings: Option<RouteSettingsRef>,
    top_gap: Option<f64>,
    show_drag_handle: bool,
) -> AnyRoute {
    debug_assert!(
        top_gap.is_none_or(|top_gap| (0.0..=0.9).contains(&top_gap)),
        "topGap must be between 0.0 and 0.9"
    );
    debug_assert!(builder.is_some() || scrollable_builder.is_some());
    debug_assert!(
        (builder.is_none() && scrollable_builder.is_some()) || scrollable_builder.is_none()
    );
    // Dart accepts `showDragHandle` but never forwards it to the route it builds.
    let _ = show_drag_handle;

    let effective_builder = builder;
    let nested_navigator_key = GlobalKey::new();
    if !use_nested_navigation {
        let route = CupertinoSheetRoute::new(app).enable_drag(app, enable_drag);
        if let Some(effective_builder) = effective_builder {
            route.builder(app, effective_builder);
        }
        if let Some(scrollable_builder) = scrollable_builder {
            route.scrollable_builder(app, scrollable_builder);
        }
        if let Some(settings) = settings {
            route.settings(app, settings);
        }
        if let Some(top_gap) = top_gap {
            route.top_gap(app, top_gap);
        }

        let navigator = Navigator::of(app, context, true);
        navigator.push(app, Route::as_route(route))
    } else {
        let route = CupertinoSheetRoute::new(app)
            .enable_drag(app, enable_drag)
            .scrollable_builder(
                app,
                Rc::new(move |_app: &mut App, _context: BuildContext, controller| {
                    let builder: WidgetBuilder = match &scrollable_builder {
                        Some(scrollable_builder) => {
                            let scrollable_builder = Rc::clone(scrollable_builder);
                            Rc::new(move |app: &mut App, context: BuildContext| {
                                scrollable_builder(app, context, controller)
                            })
                        }
                        None => Rc::clone(
                            effective_builder
                                .as_ref()
                                .expect("either builder or scrollableBuilder is given"),
                        ),
                    };
                    nested_navigation_content(&nested_navigator_key, builder)
                }),
            );
        if let Some(settings) = settings {
            route.settings(app, settings);
        }
        if let Some(top_gap) = top_gap {
            route.top_gap(app, top_gap);
        }
        let navigator = Navigator::of(app, context, true);
        navigator.push(app, Route::as_route(route))
    }
}

/// Dart's local `nestedNavigationContent` inside `showCupertinoSheet`.
fn nested_navigation_content(
    nested_navigator_key: &GlobalKey,
    builder: WidgetBuilder,
) -> WidgetRef {
    let key: KeyRef = Rc::new(nested_navigator_key.clone());
    let pop_key = nested_navigator_key.clone();
    NavigatorPopHandler::new(
        Navigator::new()
            .key(key)
            .initial_route("/")
            .on_generate_initial_routes(
                move |app: &mut App, _navigator: Handle<NavigatorState>, _initial_route_name| {
                    let builder = Rc::clone(&builder);
                    vec![Route::as_route(CupertinoPageRoute::new(
                        app,
                        Rc::new(move |app: &mut App, context: BuildContext| {
                            let child = builder(app, context);
                            PopScope::new(child)
                                .can_pop(false)
                                .on_pop_invoked_with_result(Rc::new(
                                    move |app: &mut App, did_pop: bool, result: RouteResult| {
                                        if did_pop {
                                            return;
                                        }
                                        let navigator = Navigator::of(app, context, true);
                                        navigator.pop(app, result);
                                    },
                                ))
                                .into_widget()
                        }),
                    ))]
                },
            ),
    )
    .on_pop_with_result(move |app: &mut App, _result| {
        pop_key
            .current_state::<NavigatorState>(app)
            .expect("the nested navigator is mounted")
            .maybe_pop(app, None);
    })
    .into_widget()
}

// ---------------------------------------------------------------------------------------------
// CupertinoSheetTransition

/// Provides an iOS-style sheet transition.
///
/// The page slides up and stops below the top of the screen. When covered by
/// another sheet view, it will slide slightly up and scale down to appear
/// stacked behind the new sheet.
#[derive(Debug)]
pub struct CupertinoSheetTransition {
    /// See `Widget::key`.
    pub key: Option<KeyRef>,

    /// A linear route animation from 0.0 to 1.0 when this screen is being pushed.
    pub primary_route_animation: AnyAnimation<f64>,

    /// A linear route animation from 0.0 to 1.0 when another screen is being pushed on top of
    /// this one.
    pub secondary_route_animation: AnyAnimation<f64>,

    /// The widget below this widget in the tree.
    pub child: WidgetRef,

    /// Whether to perform the transition linearly.
    ///
    /// Used to respond to a drag gesture.
    pub linear_transition: bool,

    /// The gap between the top of the screen and the top of the sheet as a ratio
    /// of the screen height.
    ///
    /// This value should be between 0.0 and 0.9, where 0.0 means no gap (sheet
    /// extends to the top of the screen) and 0.9 means the sheet covers only the
    /// bottom 10% of the screen. A value of 0.08 represents 8% of the screen height.
    ///
    /// Defaults to 0.08.
    pub top_gap: f64,
}

impl CupertinoSheetTransition {
    /// Creates an iOS style sheet transition; Dart's optional arguments are the setters.
    pub fn new<K>(
        primary_route_animation: AnyAnimation<f64>,
        secondary_route_animation: AnyAnimation<f64>,
        child: impl IntoWidget<K>,
        linear_transition: bool,
    ) -> CupertinoSheetTransition {
        CupertinoSheetTransition {
            key: None,
            primary_route_animation,
            secondary_route_animation,
            child: child.into_widget(),
            linear_transition,
            top_gap: K_TOP_GAP_RATIO,
        }
    }

    /// Dart `CupertinoSheetTransition(key:)`.
    pub fn key(mut self, key: KeyRef) -> CupertinoSheetTransition {
        self.key = Some(key);
        self
    }

    /// Dart `CupertinoSheetTransition(topGap:)`.
    pub fn top_gap(mut self, top_gap: f64) -> CupertinoSheetTransition {
        self.top_gap = top_gap;
        self
    }

    /// The primary delegated transition. Will slide a non [`CupertinoSheetRoute`] page down.
    ///
    /// Provided to the previous route to coordinate transitions between routes.
    ///
    /// If a [`CupertinoSheetRoute`] already exists in the stack, then it will
    /// slide the previous sheet upwards instead.
    pub fn delegate_transition(
        app: &mut App,
        context: BuildContext,
        animation: AnyAnimation<f64>,
        secondary_animation: AnyAnimation<f64>,
        allow_snapshotting: bool,
        child: Option<&WidgetRef>,
    ) -> Option<WidgetRef> {
        let _ = (animation, allow_snapshotting);
        if CupertinoSheetRoute::has_parent_sheet(app, context) {
            return CupertinoSheetTransition::delegated_cover_sheet_secondary_transition(
                app,
                secondary_animation,
                child,
            );
        }
        let linear = Navigator::of(app, context, false).user_gesture_in_progress(app);

        let curve = if linear {
            Curves::linear()
        } else {
            Curves::linear_to_ease_out()
        };
        let reverse_curve = if linear {
            Curves::linear()
        } else {
            Curves::ease_in_to_linear()
        };
        let curved_animation =
            CurvedAnimation::create(app, secondary_animation, curve, Some(reverse_curve));

        let device_corner_radius = MediaQuery::maybe_view_padding_of(app, context)
            .map_or(0.0, |view_padding| view_padding.top)
            * K_DEVICE_CORNER_RADIUS_SMOOTHING_FACTOR;
        let rounded_device_corners = device_corner_radius > K_ROUNDED_DEVICE_CORNERS_THRESHOLD;

        let decoration_tween = BorderRadiusGeometryTween::new(
            BorderRadiusGeometry::BorderRadius(BorderRadius::vertical(
                Radius::circular(if rounded_device_corners {
                    device_corner_radius
                } else {
                    0.0
                }),
                Radius::ZERO,
            )),
            BorderRadiusGeometry::BorderRadius(BorderRadius::all(Radius::circular(12.0))),
        );

        let radius_animation = curved_animation.as_animation().drive(app, decoration_tween);
        let opacity_animation = curved_animation.as_animation().drive(app, K_OPACITY_TWEEN);
        let slide_animation = curved_animation.as_animation().drive(app, K_TOP_DOWN_TWEEN);
        let scale_animation = curved_animation.as_animation().drive(app, K_SCALE_TWEEN);
        curved_animation.dispose(app);

        let is_dark_mode = CupertinoTheme::brightness_of(app, context) == Brightness::Dark;
        let overlay_color = if is_dark_mode {
            Color::new(0xFFC8C8C8)
        } else {
            Color::new(0xFF000000)
        };

        let contrasted_child = match child {
            Some(child) if !secondary_animation.is_dismissed(app) => Some(
                Stack::new()
                    .children(vec![
                        child.clone(),
                        FadeTransition::new(opacity_animation)
                            .child(ColoredBox::new(overlay_color).child(SizedBox::expand()))
                            .into_widget(),
                    ])
                    .into_widget(),
            ),
            _ => child.cloned(),
        };

        let top_gap_height = MediaQuery::size_of(app, context).height() * K_TOP_GAP_RATIO;

        Some(
            Stack::new()
                .children(vec![
                    AnnotatedRegion::new(
                        SizedBox::new().height(top_gap_height).width(f64::INFINITY),
                        SystemUiOverlayStyle::default()
                            .status_bar_brightness(Brightness::Dark)
                            .status_bar_icon_brightness(Brightness::Light),
                    )
                    .into_widget(),
                    SlideTransition::new(slide_animation)
                        .child(
                            ScaleTransition::new(scale_animation)
                                .alignment(Alignment::TOP_CENTER)
                                .child(AnimatedBuilder::new(
                                    Rc::new(radius_animation),
                                    move |app, _context, _child| {
                                        let mut clip = ClipRSuperellipse::new().border_radius(
                                            if !secondary_animation.is_dismissed(app) {
                                                radius_animation.value(app)
                                            } else {
                                                BorderRadiusGeometry::ZERO
                                            },
                                        );
                                        if let Some(child) = contrasted_child.as_ref() {
                                            clip = clip.child(child.clone());
                                        }
                                        clip.into_widget()
                                    },
                                )),
                        )
                        .into_widget(),
                ])
                .into_widget(),
        )
    }

    /// Dart's `CupertinoSheetTransition._delegatedCoverSheetSecondaryTransition`.
    fn delegated_cover_sheet_secondary_transition(
        app: &mut App,
        secondary_animation: AnyAnimation<f64>,
        child: Option<&WidgetRef>,
    ) -> Option<WidgetRef> {
        let curve = Curves::linear_to_ease_out();
        let reverse_curve = Curves::ease_in_to_linear();
        let curved_animation =
            CurvedAnimation::create(app, secondary_animation, curve, Some(reverse_curve));

        let slide_animation = curved_animation.as_animation().drive(app, K_MID_UP_TWEEN);
        let scale_animation = curved_animation.as_animation().drive(app, K_SCALE_TWEEN);
        curved_animation.dispose(app);

        let mut clip = ClipRSuperellipse::new().border_radius(BorderRadiusGeometry::BorderRadius(
            BorderRadius::vertical(Radius::circular(12.0), Radius::ZERO),
        ));
        if let Some(child) = child {
            clip = clip.child(child.clone());
        }
        Some(
            SlideTransition::new(slide_animation)
                .transform_hit_tests(false)
                .child(
                    ScaleTransition::new(scale_animation)
                        .alignment(Alignment::TOP_CENTER)
                        .child(clip),
                )
                .into_widget(),
        )
    }
}

impl StatefulWidget for CupertinoSheetTransition {
    type State = CupertinoSheetTransitionState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> CupertinoSheetTransitionState {
        CupertinoSheetTransitionState {
            state: StateData::new(),
            single_ticker_provider: SingleTickerProviderStateMixinData::default(),
            stretch_drag_controller: None,
            stretch_drag_animation: None,
            secondary_position_animation: None,
            secondary_scale_animation: None,
            primary_position_curve: None,
            secondary_position_curve: None,
        }
    }
}

/// Dart's `_CupertinoSheetTransitionState`.
pub struct CupertinoSheetTransitionState {
    state: StateData<CupertinoSheetTransition>,
    single_ticker_provider: SingleTickerProviderStateMixinData,
    // Controls the top padding animation when the sheet is being slightly stretched upward.
    stretch_drag_controller: Option<Handle<AnimationController>>,
    // Animates the top padding of the sheet based on the stretch drag controller's value.
    stretch_drag_animation: Option<AnyAnimation<f64>>,
    // The offset animation when this page is being covered by another sheet.
    secondary_position_animation: Option<AnyAnimation<Offset>>,
    // The scale animation when this page is being covered by another sheet.
    secondary_scale_animation: Option<AnyAnimation<f64>>,
    // Curve of primary page which is coming in to cover another route.
    primary_position_curve: Option<Handle<CurvedAnimation>>,
    // Curve of secondary page which is becoming covered by another sheet.
    secondary_position_curve: Option<Handle<CurvedAnimation>>,
}

impl CupertinoSheetTransitionState {
    fn stretch_drag_controller(self: Handle<Self>, app: &App) -> Handle<AnimationController> {
        app.get(self)
            .stretch_drag_controller
            .expect("init_state ran first")
    }

    fn setup_animation(self: Handle<Self>, app: &mut App) {
        let widget = self.widget(app);
        let primary_route_animation = widget.primary_route_animation;
        let secondary_route_animation = widget.secondary_route_animation;
        let top_gap = widget.top_gap;
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
        let state = app.get_mut(self);
        state.primary_position_curve = Some(primary_position_curve);
        state.secondary_position_curve = Some(secondary_position_curve);
        // Maintain the same stretch distance (0.008 of screen height) regardless of custom
        // top_gap.
        const STRETCH_DISTANCE: f64 = K_TOP_GAP_RATIO - K_STRETCHED_TOP_GAP_RATIO;
        let stretched_top_gap = top_gap - STRETCH_DISTANCE;
        let stretch_drag_animation = self
            .stretch_drag_controller(app)
            .drive(app, DoubleTween::new(top_gap, stretched_top_gap));
        let secondary_position_animation = secondary_position_curve
            .as_animation()
            .drive(app, K_MID_UP_TWEEN);
        let secondary_scale_animation = secondary_position_curve
            .as_animation()
            .drive(app, K_SCALE_TWEEN);
        let state = app.get_mut(self);
        state.stretch_drag_animation = Some(stretch_drag_animation);
        state.secondary_position_animation = Some(secondary_position_animation);
        state.secondary_scale_animation = Some(secondary_scale_animation);
    }

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

    fn cover_sheet_primary_transition(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        animation: AnyAnimation<f64>,
        linear_transition: bool,
        child: WidgetRef,
    ) -> WidgetRef {
        let offset_tween = if CupertinoSheetRoute::has_parent_sheet(app, context) {
            K_BOTTOM_UP_TWEEN_WHEN_COVERING_OTHER_SHEET
        } else {
            K_BOTTOM_UP_TWEEN
        };

        let curve = if linear_transition {
            Curves::linear()
        } else {
            Curves::fast_ease_in_to_slow_ease_out()
        };
        let reverse_curve = if linear_transition {
            Curves::linear()
        } else {
            Rc::new(Curves::fast_ease_in_to_slow_ease_out().flipped())
        };
        let curved_animation = CurvedAnimation::create(app, animation, curve, Some(reverse_curve));

        let position_animation = curved_animation.as_animation().drive(app, offset_tween);

        curved_animation.dispose(app);

        SlideTransition::new(position_animation)
            .child(child)
            .into_widget()
    }

    fn cover_sheet_secondary_transition(
        self: Handle<Self>,
        app: &App,
        secondary_animation: AnyAnimation<f64>,
        child: WidgetRef,
    ) -> WidgetRef {
        let _ = secondary_animation;
        let state = app.get(self);
        let secondary_position_animation = state
            .secondary_position_animation
            .expect("init_state ran first");
        let secondary_scale_animation = state
            .secondary_scale_animation
            .expect("init_state ran first");
        SlideTransition::new(secondary_position_animation)
            .transform_hit_tests(false)
            .child(
                ScaleTransition::new(secondary_scale_animation)
                    .alignment(Alignment::TOP_CENTER)
                    .child(child),
            )
            .into_widget()
    }
}

impl SingleTickerProviderStateMixin for CupertinoSheetTransitionState {
    fn single_ticker_provider_data(
        self: Handle<Self>,
        app: &App,
    ) -> &SingleTickerProviderStateMixinData {
        &app.get(self).single_ticker_provider
    }

    fn single_ticker_provider_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut SingleTickerProviderStateMixinData {
        &mut app.get_mut(self).single_ticker_provider
    }
}

/// Dart's `vsync: this`.
impl TickerProviderObject for CupertinoSheetTransitionState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
        SingleTickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl State for CupertinoSheetTransitionState {
    type Widget = CupertinoSheetTransition;
    reveal_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let stretch_drag_controller = AnimationController::create(
            app,
            None,
            Some(Duration::from_micros(1)),
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            self,
        );
        app.get_mut(self).stretch_drag_controller = Some(stretch_drag_controller);
        self.setup_animation(app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &CupertinoSheetTransition) {
        let widget = self.widget(app);
        let changed = old_widget.primary_route_animation != widget.primary_route_animation
            || old_widget.secondary_route_animation != widget.secondary_route_animation;
        if changed {
            self.dispose_curve(app);
            self.setup_animation(app);
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.dispose_curve(app);
        self.stretch_drag_controller(app).dispose(app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let _ = context;
        let stretch_drag_animation = app
            .get(self)
            .stretch_drag_animation
            .expect("init_state ran first");
        StretchDragControllerProvider {
            controller: self.stretch_drag_controller(app),
            child: SizedBox::expand()
                .child(AnimatedBuilder::new(
                    Rc::new(stretch_drag_animation),
                    move |app: &mut App, context: BuildContext, _child: Option<&WidgetRef>| {
                        let top =
                            MediaQuery::height_of(app, context) * stretch_drag_animation.value(app);
                        let widget = self.widget(app);
                        let primary_route_animation = widget.primary_route_animation;
                        let secondary_route_animation = widget.secondary_route_animation;
                        let linear_transition = widget.linear_transition;
                        let child = widget.child.clone();
                        let primary = self.cover_sheet_primary_transition(
                            app,
                            context,
                            primary_route_animation,
                            linear_transition,
                            child,
                        );
                        let secondary = self.cover_sheet_secondary_transition(
                            app,
                            secondary_route_animation,
                            primary,
                        );
                        Padding::new(EdgeInsetsGeometry::only(0.0, top, 0.0, 0.0))
                            .child(secondary)
                            .into_widget()
                    },
                ))
                .into_widget(),
        }
        .into_widget()
    }
}

/// Internally used to provide the controller for upward stretch animation.
#[derive(Debug)]
struct StretchDragControllerProvider {
    controller: Handle<AnimationController>,
    child: WidgetRef,
}

impl StretchDragControllerProvider {
    fn maybe_of(app: &App, context: BuildContext) -> Option<Handle<AnimationController>> {
        context
            .get_inherited_widget_of_exact_type::<StretchDragControllerProvider>(app)
            .map(|provider| provider.controller)
    }
}

impl InheritedWidget for StretchDragControllerProvider {
    fn key(&self) -> Option<&KeyRef> {
        None
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, _old_widget: &StretchDragControllerProvider) -> bool {
        false
    }
}

// ---------------------------------------------------------------------------------------------
// CupertinoSheetRoute

/// Route for displaying an iOS sheet styled page.
///
/// The [`CupertinoSheetRoute`] will slide up from the bottom of the screen and stop
/// below the top of the screen. If the previous route is a non-sheet route, then
/// it will animate downwards to stack behind the new sheet. If the previous route
/// is a sheet route, then it will animate slightly upwards to look like it is laying
/// on top of the previous stack of sheets.
///
/// Typically called by [`show_cupertino_sheet`], which provides some boilerplate for
/// pushing the [`CupertinoSheetRoute`] to the root navigator.
///
/// The sheet will be dismissed by dragging downwards on the screen, or a call to
/// [`CupertinoSheetRoute::pop_sheet`].
///
/// Any time a [`CupertinoSheetRoute`] contains a large scrollable that might conflict
/// with the dismiss drag gesture, pass the scroll controller provided by
/// [`scrollable_builder`](Self::scrollable_builder) to the scrollable. A scrollable widget
/// used within the sheet that does not use this scroll controller will still scroll in
/// response to user gestures, but the drag to dismiss behavior of the sheet will not
/// trigger. If there is no scrollable area within the sheet, this parameter can be ignored.
///
/// See also:
///   * [`show_cupertino_sheet`], which is a convenience method for pushing a
///     [`CupertinoSheetRoute`], with optional nested navigation built in.
pub struct CupertinoSheetRoute {
    route: RouteData,
    overlay_route: OverlayRouteData,
    transition_route: TransitionRouteData,
    local_history: LocalHistoryRouteData,
    modal_route: ModalRouteData,
    page_route: PageRouteData,
    /// Builds the primary contents of the sheet route.
    pub builder: Option<WidgetBuilder>,
    /// Builds the primary contents of the sheet route with a provided scroll controller.
    ///
    /// If the scrollable content built by this builder uses the provided scroll controller,
    /// then when a downward drag is applied to the scrollable area while the content
    /// is scrolled to the top, the drag to dismiss behavior of the sheet will be triggered.
    pub scrollable_builder: Option<ScrollableWidgetBuilder>,
    enable_drag: bool,
    // The gap between the top of the screen and the top of the sheet.
    top_gap: Option<f64>,
    show_drag_handle: bool,
}

impl CupertinoSheetRoute {
    /// Creates a page route that displays an iOS styled sheet; Dart's optional arguments are
    /// the setters.
    ///
    /// Dart's defaults: `enable_drag` true, `show_drag_handle` false, `top_gap` 0.08.
    ///
    /// Dart's assert that one of `builder` and `scrollable_builder` is given runs on the
    /// first build, since neither is a required argument.
    pub fn new(app: &mut App) -> Handle<CupertinoSheetRoute> {
        let route = RouteData::new(app, None, None);
        let transition_route = TransitionRouteData::new(app);
        let modal_route = ModalRouteData::new(app);
        app.create(CupertinoSheetRoute {
            route,
            overlay_route: OverlayRouteData::new(),
            transition_route,
            local_history: LocalHistoryRouteData::new(),
            modal_route,
            page_route: PageRouteData::new(),
            builder: None,
            scrollable_builder: None,
            enable_drag: true,
            top_gap: None,
            show_drag_handle: false,
        })
    }

    /// Dart `CupertinoSheetRoute(builder:)`.
    pub fn builder(self: Handle<Self>, app: &mut App, builder: WidgetBuilder) -> Handle<Self> {
        app.get_mut(self).builder = Some(builder);
        self
    }

    /// Dart `CupertinoSheetRoute(scrollableBuilder:)`.
    pub fn scrollable_builder(
        self: Handle<Self>,
        app: &mut App,
        scrollable_builder: ScrollableWidgetBuilder,
    ) -> Handle<Self> {
        app.get_mut(self).scrollable_builder = Some(scrollable_builder);
        self
    }

    /// Dart `CupertinoSheetRoute(settings:)`.
    pub fn settings(self: Handle<Self>, app: &mut App, settings: RouteSettingsRef) -> Handle<Self> {
        self.route_data_mut(app).set_settings(settings);
        self
    }

    /// Dart `CupertinoSheetRoute(enableDrag:)`.
    pub fn enable_drag(self: Handle<Self>, app: &mut App, enable_drag: bool) -> Handle<Self> {
        app.get_mut(self).enable_drag = enable_drag;
        self
    }

    /// Dart `CupertinoSheetRoute(showDragHandle:)`.
    pub fn show_drag_handle(
        self: Handle<Self>,
        app: &mut App,
        show_drag_handle: bool,
    ) -> Handle<Self> {
        app.get_mut(self).show_drag_handle = show_drag_handle;
        self
    }

    /// Dart `CupertinoSheetRoute(topGap:)`.
    pub fn top_gap(self: Handle<Self>, app: &mut App, top_gap: f64) -> Handle<Self> {
        debug_assert!(
            (0.0..=0.9).contains(&top_gap),
            "topGap must be between 0.0 and 0.9"
        );
        app.get_mut(self).top_gap = Some(top_gap);
        self
    }

    /// Dart's `_effectiveBuilder`.
    fn effective_builder(self: Handle<Self>, app: &App) -> ScrollableWidgetBuilder {
        if let Some(scrollable_builder) = app.get(self).scrollable_builder.clone() {
            return scrollable_builder;
        }
        let builder = app
            .get(self)
            .builder
            .clone()
            .expect("Either scrollableBuilder or builder must not be null");
        Rc::new(move |app: &mut App, context: BuildContext, _controller| builder(app, context))
    }

    /// Dart's `_sheetWithDragHandle`.
    fn sheet_with_drag_handle(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        controller: AnyScrollController,
    ) -> WidgetRef {
        let builder = self.effective_builder(app);
        if !app.get(self).show_drag_handle {
            return builder(app, context, controller);
        }

        // Values derived from Apple's Figma files and a simulator running iOS 18.2.
        const DRAG_HANDLE_TOP_PADDING: f64 = 5.0;
        const DRAG_HANDLE_HEIGHT: f64 = 5.0;
        const DRAG_HANDLE_WIDTH: f64 = 36.0;
        const DRAG_HANDLE_PADDING: f64 = 15.0;

        let data = MediaQuery::of(app, context)
            .copy_with()
            .padding(EdgeInsets::only(0.0, DRAG_HANDLE_PADDING, 0.0, 0.0));
        let content = builder(app, context, controller);
        Stack::new()
            .fit(StackFit::Expand)
            .children(vec![
                MediaQuery::new(data, content).into_widget(),
                Align::new()
                    .alignment(AlignmentGeometry::TOP_CENTER)
                    .child(
                        Padding::new(EdgeInsetsGeometry::only(
                            0.0,
                            DRAG_HANDLE_TOP_PADDING,
                            0.0,
                            0.0,
                        ))
                        .child(
                            DecoratedBox::new(
                                ShapeDecoration::new(RoundedSuperellipseBorder::new(
                                    BorderSide::NONE,
                                    Some(BorderRadiusGeometry::all(Radius::circular(
                                        DRAG_HANDLE_WIDTH / 2.0,
                                    ))),
                                ))
                                .color(CupertinoColors::TERTIARY_LABEL),
                            )
                            .child(
                                SizedBox::new()
                                    .height(DRAG_HANDLE_HEIGHT)
                                    .width(DRAG_HANDLE_WIDTH),
                            ),
                        ),
                    )
                    .into_widget(),
            ])
            .into_widget()
    }

    /// Checks if a Cupertino sheet view exists in the widget tree above the current
    /// context.
    pub fn has_parent_sheet(app: &App, context: BuildContext) -> bool {
        CupertinoSheetScope::maybe_of(app, context).is_some()
    }

    /// Pops the entire [`CupertinoSheetRoute`], if a sheet route exists in the stack.
    ///
    /// Used to pop an entire sheet at once.
    pub fn pop_sheet(app: &mut App, context: BuildContext) {
        if CupertinoSheetRoute::has_parent_sheet(app, context) {
            let navigator = Navigator::of(app, context, true);
            navigator.pop(app, None);
        }
    }
}

impl Route for CupertinoSheetRoute {
    reveal_widgets::page_route_overrides!();
}

impl OverlayRoute for CupertinoSheetRoute {
    reveal_widgets::modal_overlay_route_overrides!();
}

impl PredictiveBackRoute for CupertinoSheetRoute {
    reveal_widgets::page_predictive_back_route_overrides!();
}

impl TransitionRoute for CupertinoSheetRoute {
    reveal_widgets::modal_transition_route_overrides!();

    fn allow_snapshotting(self: Handle<Self>, app: &App) -> bool {
        PageRoute::allow_snapshotting(self, app)
    }

    fn transition_duration(self: Handle<Self>, app: &App) -> Duration {
        CupertinoSheetRouteTransitionMixin::transition_duration(self, app)
    }

    fn can_transition_to(self: Handle<Self>, app: &App, next_route: AnyTransitionRoute) -> bool {
        CupertinoSheetRouteTransitionMixin::can_transition_to(self, app, next_route)
    }

    fn can_transition_from(
        self: Handle<Self>,
        app: &App,
        previous_route: AnyTransitionRoute,
    ) -> bool {
        CupertinoSheetRouteTransitionMixin::can_transition_from(self, app, previous_route)
    }

    fn opaque(self: Handle<Self>, app: &App) -> bool {
        let _ = app;
        false
    }
}

impl LocalHistoryRoute for CupertinoSheetRoute {
    reveal_widgets::local_history_route_accessors!();
}

impl ModalRoute for CupertinoSheetRoute {
    reveal_widgets::modal_route_accessors!();

    fn barrier_dismissible(self: Handle<Self>, app: &App) -> bool {
        let _ = app;
        false
    }

    fn barrier_color(self: Handle<Self>, app: &App) -> Option<Color> {
        let _ = app;
        Some(CupertinoColors::TRANSPARENT.color())
    }

    fn barrier_label(self: Handle<Self>, app: &App) -> Option<String> {
        let _ = app;
        None
    }

    fn maintain_state(self: Handle<Self>, app: &App) -> bool {
        let _ = app;
        true
    }

    fn fullscreen_dialog(self: Handle<Self>, app: &App) -> bool {
        PageRoute::fullscreen_dialog(self, app)
    }

    fn delegated_transition(self: Handle<Self>, app: &App) -> Option<DelegatedTransitionBuilder> {
        CupertinoSheetRouteTransitionMixin::delegated_transition(self, app)
    }

    fn build_page(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        animation: AnyAnimation<f64>,
        secondary_animation: AnyAnimation<f64>,
    ) -> WidgetRef {
        CupertinoSheetRouteTransitionMixin::build_page(
            self,
            app,
            context,
            animation,
            secondary_animation,
        )
    }

    fn build_transitions(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        animation: AnyAnimation<f64>,
        secondary_animation: AnyAnimation<f64>,
        child: WidgetRef,
    ) -> WidgetRef {
        CupertinoSheetRouteTransitionMixin::build_transitions(
            self,
            app,
            context,
            animation,
            secondary_animation,
            child,
        )
    }
}

impl PageRoute for CupertinoSheetRoute {
    reveal_widgets::page_route_accessors!();
}

impl CupertinoSheetRouteTransitionMixin for CupertinoSheetRoute {
    fn build_content(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let enable_drag = app.get(self).enable_drag;
        let top_gap = CupertinoSheetRouteTransitionMixin::top_gap(self, app);
        MediaQuery::remove_padding(
            app,
            context,
            false,
            true,
            false,
            false,
            ClipRSuperellipse::new()
                .border_radius(BorderRadiusGeometry::BorderRadius(BorderRadius::vertical(
                    Radius::circular(12.0),
                    Radius::ZERO,
                )))
                .child(CupertinoUserInterfaceLevel::new(
                    CupertinoUserInterfaceLevelData::Elevated,
                    CupertinoSheetScope {
                        child: CupertinoDraggableScrollableSheet {
                            key: None,
                            enabled_callback: Rc::new(move || enable_drag),
                            on_start_pop_gesture: Rc::new(move |app| {
                                start_pop_gesture(self, app, top_gap)
                            }),
                            builder: Rc::new(move |app, context, controller| {
                                self.sheet_with_drag_handle(app, context, controller)
                            }),
                        }
                        .into_widget(),
                    },
                )),
        )
        .into_widget()
    }

    fn enable_drag(self: Handle<Self>, app: &App) -> bool {
        app.get(self).enable_drag
    }

    fn top_gap(self: Handle<Self>, app: &App) -> f64 {
        app.get(self).top_gap.unwrap_or(K_TOP_GAP_RATIO)
    }

    fn has_custom_top_gap(self: Handle<Self>, app: &App) -> bool {
        app.get(self).top_gap.is_some()
    }
}

/// Internally used to see if another sheet is in the tree already.
#[derive(Debug)]
struct CupertinoSheetScope {
    child: WidgetRef,
}

impl CupertinoSheetScope {
    fn maybe_of(app: &App, context: BuildContext) -> Option<&CupertinoSheetScope> {
        context.get_inherited_widget_of_exact_type::<CupertinoSheetScope>(app)
    }
}

impl InheritedWidget for CupertinoSheetScope {
    fn key(&self) -> Option<&KeyRef> {
        None
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, _old_widget: &CupertinoSheetScope) -> bool {
        false
    }
}

// ---------------------------------------------------------------------------------------------
// _CupertinoSheetRouteTransitionMixin

/// A mixin that replaces the entire screen with an iOS sheet transition for a [`PageRoute`].
///
/// See also:
///
///  * [`CupertinoSheetRoute`], which is a [`PageRoute`] that leverages this mixin.
trait CupertinoSheetRouteTransitionMixin: PageRoute {
    /// Builds the primary contents of the route.
    fn build_content(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef;

    /// Dart's `_CupertinoSheetRouteTransitionMixin.transitionDuration`.
    fn transition_duration(self: Handle<Self>, app: &App) -> Duration {
        let _ = app;
        Duration::from_millis(500)
    }

    /// Dart's `_CupertinoSheetRouteTransitionMixin.delegatedTransition`.
    fn delegated_transition(self: Handle<Self>, app: &App) -> Option<DelegatedTransitionBuilder> {
        if self.has_custom_top_gap(app) {
            return None;
        }
        Some(Rc::new(CupertinoSheetTransition::delegate_transition))
    }

    /// Determines whether the content can be dragged.
    ///
    /// If `true`, dragging is enabled; otherwise, it remains fixed.
    fn enable_drag(self: Handle<Self>, app: &App) -> bool;

    /// The gap between the top of the screen and the top of the sheet as a ratio
    /// of the screen height.
    fn top_gap(self: Handle<Self>, app: &App) -> f64;

    /// Whether a custom top gap has been set.
    fn has_custom_top_gap(self: Handle<Self>, app: &App) -> bool;

    /// Dart's `_CupertinoSheetRouteTransitionMixin.buildPage`.
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

    /// Dart's `_CupertinoSheetRouteTransitionMixin.canTransitionFrom`.
    fn can_transition_from(
        self: Handle<Self>,
        app: &App,
        previous_route: AnyTransitionRoute,
    ) -> bool {
        let _ = previous_route;
        !self.has_custom_top_gap(app)
    }

    /// Dart's `_CupertinoSheetRouteTransitionMixin.canTransitionTo`.
    fn can_transition_to(self: Handle<Self>, app: &App, next_route: AnyTransitionRoute) -> bool {
        if Route::as_route(self)
            .downcast::<CupertinoSheetRoute>(app)
            .is_some()
            && self.has_custom_top_gap(app)
        {
            return false;
        }
        is_cupertino_sheet_route_transition(app, next_route.as_route())
    }

    /// Dart's `_CupertinoSheetRouteTransitionMixin.buildTransitions`.
    fn build_transitions(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        animation: AnyAnimation<f64>,
        secondary_animation: AnyAnimation<f64>,
        child: WidgetRef,
    ) -> WidgetRef {
        let enable_drag = self.enable_drag(app);
        let top_gap = self.top_gap(app);
        build_page_transitions(
            self,
            app,
            context,
            animation,
            secondary_animation,
            child,
            enable_drag,
            top_gap,
        )
    }
}

/// Dart's `route is _CupertinoSheetRouteTransitionMixin`.
///
/// The mixin is private to this file with one implementor, as Dart's is to its library, so a
/// downcast to that route type is the whole answer.
fn is_cupertino_sheet_route_transition(app: &App, route: AnyRoute) -> bool {
    route.downcast::<CupertinoSheetRoute>(app).is_some()
}

/// Dart's `_CupertinoSheetRouteTransitionMixin._startPopGesture`.
fn start_pop_gesture<R: ModalRoute>(
    this: Handle<R>,
    app: &mut App,
    top_gap: f64,
) -> Handle<CupertinoDragGestureController> {
    let navigator = Route::navigator(this, app).expect("an installed route");
    let controller =
        TransitionRoute::controller(this, app).expect("an installed route has a controller");
    let route = Route::as_route(this);
    CupertinoDragGestureController::new(
        app,
        top_gap,
        navigator,
        Rc::new(move |app| route.is_current(app)),
        Rc::new(move |app| route.is_active(app)),
        controller,
    )
}

/// Returns a [`CupertinoSheetTransition`]; Dart's
/// `_CupertinoSheetRouteTransitionMixin.buildPageTransitions`.
#[expect(
    clippy::too_many_arguments,
    reason = "Dart's static takes the route plus the five transition arguments"
)]
fn build_page_transitions<R: ModalRoute>(
    this: Handle<R>,
    app: &mut App,
    context: BuildContext,
    animation: AnyAnimation<f64>,
    secondary_animation: AnyAnimation<f64>,
    child: WidgetRef,
    enable_drag: bool,
    top_gap: f64,
) -> WidgetRef {
    let _ = context;
    let linear_transition = ModalRoute::pop_gesture_in_progress(this, app);
    CupertinoSheetTransition::new(
        animation,
        secondary_animation,
        CupertinoDragGestureDetector {
            key: None,
            enabled_callback: Rc::new(move || enable_drag),
            on_start_pop_gesture: Rc::new(move |app| start_pop_gesture(this, app, top_gap)),
            child,
        },
        linear_transition,
    )
    .top_gap(top_gap)
    .into_widget()
}

// ---------------------------------------------------------------------------------------------
// The drag-to-dismiss gesture

/// Dart's `_CupertinoDragGestureDetector`.
struct CupertinoDragGestureDetector {
    key: Option<KeyRef>,
    enabled_callback: DragEnabledCallback,
    on_start_pop_gesture: StartPopGestureCallback,
    child: WidgetRef,
}

impl Debug for CupertinoDragGestureDetector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CupertinoDragGestureDetector")
            .field("child", &self.child)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for CupertinoDragGestureDetector {
    type State = CupertinoDragGestureDetectorState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> CupertinoDragGestureDetectorState {
        CupertinoDragGestureDetectorState {
            state: StateData::new(),
            drag_gesture_controller: None,
            recognizer: None,
            stretch_drag_controller: None,
        }
    }
}

/// Dart's `_CupertinoDragGestureDetectorState`.
struct CupertinoDragGestureDetectorState {
    state: StateData<CupertinoDragGestureDetector>,
    drag_gesture_controller: Option<Handle<CupertinoDragGestureController>>,
    recognizer: Option<Handle<VerticalDragGestureRecognizer>>,
    stretch_drag_controller: Option<Handle<AnimationController>>,
}

impl CupertinoDragGestureDetectorState {
    /// Dart's `_cupertinoVelocityBuilder`.
    fn cupertino_velocity_builder(
        event: &PointerEvent,
    ) -> Box<dyn reveal_gestures::AnyVelocityTracker> {
        Box::new(IOSScrollViewFlingVelocityTracker::new(event.kind()))
    }

    fn recognizer(self: Handle<Self>, app: &App) -> Handle<VerticalDragGestureRecognizer> {
        app.get(self).recognizer.expect("init_state ran first")
    }

    fn sheet_height(self: Handle<Self>, app: &App) -> f64 {
        self.context(app)
            .size(app)
            .expect("the detector is laid out")
            .height()
    }

    fn handle_drag_start(self: Handle<Self>, app: &mut App, _details: DragStartDetails) {
        debug_assert!(self.mounted(app));
        debug_assert!(app.get(self).drag_gesture_controller.is_none());
        let on_start_pop_gesture = Rc::clone(&self.widget(app).on_start_pop_gesture);
        let controller = on_start_pop_gesture(app);
        app.get_mut(self).drag_gesture_controller = Some(controller);
    }

    fn handle_drag_update(self: Handle<Self>, app: &mut App, details: DragUpdateDetails) {
        debug_assert!(self.mounted(app));
        let controller = app
            .get(self)
            .drag_gesture_controller
            .expect("a drag start ran first");
        let Some(stretch_drag_controller) = app.get(self).stretch_drag_controller else {
            return;
        };
        let sheet_height = self.sheet_height(app);
        let delta = if sheet_height > 0.0 {
            details.primary_delta.expect("a vertical drag") / sheet_height
        } else {
            0.0
        };
        // Divide by size of the sheet.
        controller.drag_update(app, delta, Some(stretch_drag_controller));
    }

    fn handle_drag_end(self: Handle<Self>, app: &mut App, details: DragEndDetails) {
        debug_assert!(self.mounted(app));
        let controller = app
            .get(self)
            .drag_gesture_controller
            .expect("a drag start ran first");
        let Some(stretch_drag_controller) = app.get(self).stretch_drag_controller else {
            app.get_mut(self).drag_gesture_controller = None;
            return;
        };
        let sheet_height = self.sheet_height(app);
        let velocity = if sheet_height > 0.0 {
            details.velocity.pixels_per_second.dy() / sheet_height
        } else {
            0.0
        };
        controller.drag_end(app, velocity, Some(stretch_drag_controller));
        app.get_mut(self).drag_gesture_controller = None;
    }

    fn handle_drag_cancel(self: Handle<Self>, app: &mut App) {
        debug_assert!(self.mounted(app));
        // This can be called even if start is not called, paired with the "down" event
        // that we don't consider here.
        let Some(stretch_drag_controller) = app.get(self).stretch_drag_controller else {
            app.get_mut(self).drag_gesture_controller = None;
            return;
        };
        if let Some(controller) = app.get(self).drag_gesture_controller {
            controller.drag_end(app, 0.0, Some(stretch_drag_controller));
        }
        app.get_mut(self).drag_gesture_controller = None;
    }

    fn handle_pointer_down(self: Handle<Self>, app: &mut App, event: PointerDownEvent) {
        let enabled_callback = Rc::clone(&self.widget(app).enabled_callback);
        if enabled_callback() {
            self.recognizer(app).add_pointer(app, event);
        }
    }
}

impl State for CupertinoDragGestureDetectorState {
    type Widget = CupertinoDragGestureDetector;
    reveal_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        debug_assert!(app.get(self).stretch_drag_controller.is_none());
        let context = self.context(app);
        app.get_mut(self).stretch_drag_controller =
            StretchDragControllerProvider::maybe_of(app, context);
        let recognizer = VerticalDragGestureRecognizer::new(app);
        recognizer.set_velocity_tracker_builder(
            app,
            Rc::new(CupertinoDragGestureDetectorState::cupertino_velocity_builder),
        );
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

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        let context = self.context(app);
        app.get_mut(self).stretch_drag_controller =
            StretchDragControllerProvider::maybe_of(app, context);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let recognizer = self.recognizer(app);
        RecognizerLeaf::dispose(recognizer, app);

        // If this is disposed during a drag, call navigator.didStopUserGesture.
        if app.get(self).drag_gesture_controller.is_some() {
            SchedulerBinding::add_post_frame_callback(
                app,
                FrameCallback::new(move |app, _timestamp| {
                    if let Some(controller) = app.get(self).drag_gesture_controller {
                        let navigator = app.get(controller).navigator;
                        if navigator.mounted(app) {
                            navigator.did_stop_user_gesture(app);
                        }
                    }
                    app.get_mut(self).drag_gesture_controller = None;
                }),
            );
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let _ = context;
        let child = self.widget(app).child.clone();
        reveal_widgets::Listener::new()
            .on_pointer_down(Rc::new(move |app: &mut App, event: PointerDownEvent| {
                self.handle_pointer_down(app, event);
            }))
            .behavior(HitTestBehavior::Translucent)
            .child(child)
            .into_widget()
    }
}

/// A controller for the iOS-style sheet drag-to-dismiss gesture.
///
/// This is created by a [`CupertinoSheetRoute`] in response to a gesture caught by a
/// [`CupertinoDragGestureDetector`] widget, which then also feeds it input from the gesture. It
/// controls the animation controller owned by the route, based on the input provided by the
/// gesture detector.
struct CupertinoDragGestureController {
    pop_drag_controller: Handle<AnimationController>,
    navigator: Handle<NavigatorState>,
    get_is_active: RouteFlagGetter,
    get_is_current: RouteFlagGetter,
    #[expect(
        dead_code,
        reason = "Dart's `_CupertinoDragGestureController.topGap` is never read"
    )]
    top_gap: f64,
}

impl CupertinoDragGestureController {
    /// Creates a controller for an iOS-style back gesture.
    fn new(
        app: &mut App,
        top_gap: f64,
        navigator: Handle<NavigatorState>,
        get_is_current: RouteFlagGetter,
        get_is_active: RouteFlagGetter,
        pop_drag_controller: Handle<AnimationController>,
    ) -> Handle<CupertinoDragGestureController> {
        let this = app.create(CupertinoDragGestureController {
            pop_drag_controller,
            navigator,
            get_is_active,
            get_is_current,
            top_gap,
        });
        navigator.did_start_user_gesture(app);
        this
    }

    /// The drag gesture has changed by `delta`. The total range of the drag
    /// should be 0.0 to 1.0.
    fn drag_update(
        self: Handle<Self>,
        app: &mut App,
        delta: f64,
        up_controller: Option<Handle<AnimationController>>,
    ) {
        let pop_drag_controller = app.get(self).pop_drag_controller;
        if let Some(up_controller) = up_controller
            && pop_drag_controller.value(app) == 1.0
            && (up_controller.value(app) > 0.0 || delta < 0.0)
        {
            // Divide by stretchable range (when dragging upward at max extent).
            // Maintain the same stretch distance regardless of custom top_gap.
            const STRETCH_DISTANCE: f64 = K_TOP_GAP_RATIO - K_STRETCHED_TOP_GAP_RATIO;
            let value = up_controller.value(app);
            up_controller.set_value(app, value - delta / STRETCH_DISTANCE);
        } else {
            let value = pop_drag_controller.value(app);
            pop_drag_controller.set_value(app, value - delta);
        }
    }

    /// Whether the sheet is currently dragged downwards.
    fn is_dragged(self: Handle<Self>, app: &App) -> bool {
        app.get(self).pop_drag_controller.value(app) != 1.0
    }

    /// The drag gesture has ended with a vertical motion of `velocity` as a
    /// fraction of screen height per second.
    fn drag_end(
        self: Handle<Self>,
        app: &mut App,
        velocity: f64,
        up_controller: Option<Handle<AnimationController>>,
    ) {
        // If the sheet is in a stretched state (dragged upward beyond max size),
        // reverse the stretch to return to the normal max height.
        if let Some(up_controller) = up_controller
            && up_controller.value(app) > 0.0
        {
            up_controller.animate_back(
                app,
                0.0,
                Some(Duration::from_millis(180)),
                Curves::ease_out(),
            );
            let navigator = app.get(self).navigator;
            navigator.did_stop_user_gesture(app);
            return;
        }

        // Fling in the appropriate direction.
        //
        // This curve has been determined through rigorously eyeballing native iOS
        // animations on a simulator running iOS 18.0.
        let animation_curve = Curves::ease_out();
        let pop_drag_controller = app.get(self).pop_drag_controller;
        let is_current = (Rc::clone(&app.get(self).get_is_current))(app);
        let animate_forward = if !is_current {
            // If the page has already been navigated away from, then the animation
            // direction depends on whether or not it's still in the navigation stack,
            // regardless of velocity or drag position. For example, if a route is
            // being slowly dragged back by just a few pixels, but then a programmatic
            // pop occurs, the route should still be animated off the screen.
            // See https://github.com/flutter/flutter/issues/141268.
            (Rc::clone(&app.get(self).get_is_active))(app)
        } else if velocity.abs() >= K_MIN_FLING_VELOCITY {
            // If the user releases the page before mid screen with sufficient velocity,
            // or after mid screen, we should animate the page out. Otherwise, the page
            // should be animated back in.
            velocity <= 0.0
        } else {
            // If the drag is dropped with low velocity, the sheet will pop if the
            // the drag goes a little past the halfway point on the screen. This is
            // eyeballed on a simulator running iOS 18.0.
            pop_drag_controller.value(app) > 0.52
        };

        if animate_forward {
            pop_drag_controller.animate_to(
                app,
                1.0,
                Some(K_DROPPED_SHEET_DRAG_ANIMATION_DURATION),
                Rc::clone(&animation_curve),
            );
        } else {
            if is_current {
                // This route is destined to pop at this point. Reuse navigator's pop.
                let navigator = app.get(self).navigator;
                navigator.pop(app, None);
            }

            if pop_drag_controller.is_animating(app) {
                pop_drag_controller.animate_back(
                    app,
                    0.0,
                    Some(K_DROPPED_SHEET_DRAG_ANIMATION_DURATION),
                    Rc::clone(&animation_curve),
                );
            }
        }

        if pop_drag_controller.is_animating(app) {
            // Keep the userGestureInProgress in true state so we don't change the
            // curve of the page transition mid-flight since CupertinoPageTransition
            // depends on userGestureInProgress.
            Animation::add_status_listener(
                pop_drag_controller,
                app,
                AnimationStatusListener::handle_method(
                    self,
                    CupertinoDragGestureController::handle_animation_status,
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
        let pop_drag_controller = app.get(self).pop_drag_controller;
        Animation::remove_status_listener(
            pop_drag_controller,
            app,
            &AnimationStatusListener::handle_method(
                self,
                CupertinoDragGestureController::handle_animation_status,
            ),
        );
    }
}

// ---------------------------------------------------------------------------------------------
// _CupertinoSheetScrollController

/// Dart's `_CupertinoSheetScrollController`: the scroll controller a sheet hands its
/// `scrollableBuilder`, which reports the drag back to the sheet's dismiss gesture.
struct CupertinoSheetScrollController {
    change_notifier: ChangeNotifierData,
    scroll_controller: ScrollControllerData,
    on_drag_start: DragStartCallback,
    on_drag_update: DragUpdateCallback,
    on_drag_end: DragEndCallback,
    sheet_is_dragged_down: GetSheetDragged,
}

impl CupertinoSheetScrollController {
    fn new(
        app: &mut App,
        on_drag_start: DragStartCallback,
        on_drag_update: DragUpdateCallback,
        on_drag_end: DragEndCallback,
        sheet_is_dragged_down: GetSheetDragged,
    ) -> Handle<CupertinoSheetScrollController> {
        app.create(CupertinoSheetScrollController {
            change_notifier: ChangeNotifierData::new(),
            scroll_controller: ScrollControllerData::new(0.0, true, None, None, None),
            on_drag_start,
            on_drag_update,
            on_drag_end,
            sheet_is_dragged_down,
        })
    }
}

impl ChangeNotifier for CupertinoSheetScrollController {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl ScrollControllerLeaf for CupertinoSheetScrollController {
    reveal_widgets::scroll_controller_accessors!();

    fn create_scroll_position(
        self: Handle<Self>,
        app: &mut App,
        physics: ScrollPhysicsRef,
        context: Rc<dyn ScrollContext>,
        old_position: Option<AnyScrollPosition>,
    ) -> AnyScrollPosition {
        let physics = physics.apply_to(Some(Rc::new(AlwaysScrollableScrollPhysics::new())));
        let controller = app.get(self);
        let on_drag_start = Rc::clone(&controller.on_drag_start);
        let on_drag_update = Rc::clone(&controller.on_drag_update);
        let on_drag_end = Rc::clone(&controller.on_drag_end);
        let sheet_is_dragged_down = Rc::clone(&controller.sheet_is_dragged_down);
        CupertinoSheetScrollPosition::new(
            app,
            physics,
            context,
            old_position,
            on_drag_start,
            on_drag_update,
            on_drag_end,
            sheet_is_dragged_down,
        )
        .as_scroll_position()
    }
}

// ---------------------------------------------------------------------------------------------
// _CupertinoSheetScrollPosition

/// A scroll position that manages scroll activities for [`CupertinoSheetScrollController`];
/// Dart's `_CupertinoSheetScrollPosition`.
///
/// An instance of this type manages scroll activities which in response to user gestures
/// either change the visible content offset in the `Scrollable`'s viewport, or report
/// the delta change or velocity of the user's gestures back to
/// [`CupertinoDragGestureController`] through [`CupertinoSheetScrollController`].
struct CupertinoSheetScrollPosition {
    // This type is a modified version of `_DraggableScrollableSheetScrollPosition`
    // from `DraggableScrollableSheet`. If a change needs to be made to this, check and
    // see if the original type needs the change as well.
    change_notifier: ChangeNotifierData,
    scroll_position: ScrollPositionData,
    single_context: ScrollPositionWithSingleContextData,
    drag_cancel_callback: Option<Listener>,
    ballistic_controllers: Vec<Handle<AnimationController>>,
    on_drag_start: DragStartCallback,
    on_drag_update: DragUpdateCallback,
    on_drag_end: DragEndCallback,
    sheet_is_dragged_down: GetSheetDragged,
}

impl CupertinoSheetScrollPosition {
    #[expect(
        clippy::too_many_arguments,
        reason = "Dart's named arguments, in Dart's order"
    )]
    fn new(
        app: &mut App,
        physics: ScrollPhysicsRef,
        context: Rc<dyn ScrollContext>,
        old_position: Option<AnyScrollPosition>,
        on_drag_start: DragStartCallback,
        on_drag_update: DragUpdateCallback,
        on_drag_end: DragEndCallback,
        sheet_is_dragged_down: GetSheetDragged,
    ) -> Handle<CupertinoSheetScrollPosition> {
        let scroll_position = ScrollPositionData::new(app, physics, context, true, None);
        let this = app.create(CupertinoSheetScrollPosition {
            change_notifier: ChangeNotifierData::new(),
            scroll_position,
            single_context: ScrollPositionWithSingleContextData::new(),
            drag_cancel_callback: None,
            ballistic_controllers: Vec::new(),
            on_drag_start,
            on_drag_update,
            on_drag_end,
            sheet_is_dragged_down,
        });
        ScrollPositionWithSingleContext::init(this, app, Some(0.0), old_position);
        this
    }

    /// Whether the list scrolls rather than the sheet being dragged.
    fn list_should_scroll(self: Handle<Self>, app: &App) -> bool {
        ViewportOffset::pixels(self, app) > 0.0
    }
}

impl ChangeNotifier for CupertinoSheetScrollPosition {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl ViewportOffset for CupertinoSheetScrollPosition {
    reveal_widgets::scroll_position_with_single_context_viewport_offset_overrides!();
}

impl ScrollPosition for CupertinoSheetScrollPosition {
    reveal_widgets::scroll_position_with_single_context_overrides!();
}

impl ScrollActivityDelegate for CupertinoSheetScrollPosition {
    reveal_widgets::scroll_position_with_single_context_delegate_overrides!();
}

impl ScrollPositionWithSingleContextLeaf for CupertinoSheetScrollPosition {
    reveal_widgets::scroll_position_with_single_context_accessors!();

    fn absorb(self: Handle<Self>, app: &mut App, other: AnyScrollPosition) {
        ScrollPositionWithSingleContext::absorb(self, app, other);
        debug_assert!(app.get(self).drag_cancel_callback.is_none());

        let Some(other) = other.downcast::<CupertinoSheetScrollPosition>(app) else {
            return;
        };

        if let Some(drag_cancel_callback) = app.get_mut(other).drag_cancel_callback.take() {
            app.get_mut(self).drag_cancel_callback = Some(drag_cancel_callback);
        }
    }

    fn begin_activity(self: Handle<Self>, app: &mut App, new_activity: Option<AnyScrollActivity>) {
        // Cancel the running ballistic simulations
        for ballistic_controller in app.get(self).ballistic_controllers.clone() {
            ballistic_controller.stop(app, true);
        }
        ScrollPositionWithSingleContext::begin_activity(self, app, new_activity);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        for ballistic_controller in std::mem::take(&mut app.get_mut(self).ballistic_controllers) {
            ballistic_controller.dispose(app);
        }
        ScrollPositionWithSingleContext::dispose(self, app);
    }

    fn apply_user_offset(self: Handle<Self>, app: &mut App, delta: f64) {
        (Rc::clone(&app.get(self).on_drag_start))(app);
        let sheet_is_dragged_down = Rc::clone(&app.get(self).sheet_is_dragged_down);
        if !self.list_should_scroll(app) && (delta > 0.0 || sheet_is_dragged_down(app)) {
            (Rc::clone(&app.get(self).on_drag_update))(app, delta);
        } else {
            ScrollPositionWithSingleContext::apply_user_offset(self, app, delta);
        }
    }

    fn go_ballistic(self: Handle<Self>, app: &mut App, velocity: f64) {
        // End drag gesture.
        if (velocity == 0.0)
            || (velocity < 0.0 && self.list_should_scroll(app))
            || (velocity > 0.0 && ViewportOffset::pixels(self, app) != self.max_scroll_extent(app))
        {
            (Rc::clone(&app.get(self).on_drag_end))(app, 0.0);
            ScrollPositionWithSingleContext::go_ballistic(self, app, velocity);
            return;
        }
        if let Some(drag_cancel_callback) = app.get_mut(self).drag_cancel_callback.take() {
            drag_cancel_callback.call(app);
        }
        if velocity < 0.0 && !self.list_should_scroll(app) {
            (Rc::clone(&app.get(self).on_drag_end))(app, velocity);
            ScrollPositionWithSingleContext::go_ballistic(self, app, 0.0);
            return;
        }
        (Rc::clone(&app.get(self).on_drag_end))(app, 0.0);
        ScrollPositionWithSingleContext::go_ballistic(self, app, velocity);
    }

    fn drag(
        self: Handle<Self>,
        app: &mut App,
        details: DragStartDetails,
        drag_cancel_callback: Listener,
    ) -> Rc<dyn Drag> {
        // Save this so we can call it later if we have to `go_ballistic` on our own.
        app.get_mut(self).drag_cancel_callback = Some(drag_cancel_callback.clone());
        ScrollPositionWithSingleContext::drag(self, app, details, drag_cancel_callback)
    }
}

// ---------------------------------------------------------------------------------------------
// _CupertinoDraggableScrollableSheet

/// Dart's `_CupertinoDraggableScrollableSheet`: the sheet's content, built with the scroll
/// controller that feeds the dismiss gesture.
struct CupertinoDraggableScrollableSheet {
    key: Option<KeyRef>,
    builder: ScrollableWidgetBuilder,
    #[expect(
        dead_code,
        reason = "Dart's `_CupertinoDraggableScrollableSheet.enabledCallback` is never read"
    )]
    enabled_callback: DragEnabledCallback,
    on_start_pop_gesture: StartPopGestureCallback,
}

impl Debug for CupertinoDraggableScrollableSheet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CupertinoDraggableScrollableSheet")
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for CupertinoDraggableScrollableSheet {
    type State = CupertinoDraggableScrollableSheetState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> CupertinoDraggableScrollableSheetState {
        CupertinoDraggableScrollableSheetState {
            state: StateData::new(),
            scroll_controller: None,
            drag_gesture_controller: None,
        }
    }
}

/// Dart's `_CupertinoDraggableScrollableSheetState`.
struct CupertinoDraggableScrollableSheetState {
    state: StateData<CupertinoDraggableScrollableSheet>,
    scroll_controller: Option<Handle<CupertinoSheetScrollController>>,
    drag_gesture_controller: Option<Handle<CupertinoDragGestureController>>,
}

impl CupertinoDraggableScrollableSheetState {
    fn scroll_controller(self: Handle<Self>, app: &App) -> Handle<CupertinoSheetScrollController> {
        app.get(self).scroll_controller.expect("init_state has run")
    }

    fn sheet_height(self: Handle<Self>, app: &App) -> f64 {
        self.context(app)
            .size(app)
            .expect("the sheet is laid out")
            .height()
    }

    fn drag_start(self: Handle<Self>, app: &mut App) {
        debug_assert!(self.mounted(app));
        if app.get(self).drag_gesture_controller.is_none() {
            let on_start_pop_gesture = Rc::clone(&self.widget(app).on_start_pop_gesture);
            let controller = on_start_pop_gesture(app);
            app.get_mut(self).drag_gesture_controller = Some(controller);
        }
    }

    fn drag_update(self: Handle<Self>, app: &mut App, delta: f64) {
        debug_assert!(self.mounted(app));
        if let Some(controller) = app.get(self).drag_gesture_controller {
            let sheet_height = self.sheet_height(app);
            controller.drag_update(
                app,
                delta / (sheet_height - (sheet_height * K_TOP_GAP_RATIO)),
                None,
            );
        }
    }

    fn handle_drag_end(self: Handle<Self>, app: &mut App, velocity: f64) {
        debug_assert!(self.mounted(app));
        if let Some(controller) = app.get(self).drag_gesture_controller {
            let sheet_height = self.sheet_height(app);
            controller.drag_end(app, -velocity / sheet_height, None);
            app.get_mut(self).drag_gesture_controller = None;
        }
    }
}

impl State for CupertinoDraggableScrollableSheetState {
    type Widget = CupertinoDraggableScrollableSheet;
    reveal_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let scroll_controller = CupertinoSheetScrollController::new(
            app,
            Rc::new(move |app: &mut App| self.drag_start(app)),
            Rc::new(move |app: &mut App, delta| self.drag_update(app, delta)),
            Rc::new(move |app: &mut App, velocity| self.handle_drag_end(app, velocity)),
            Rc::new(move |app: &App| {
                app.get(self)
                    .drag_gesture_controller
                    .is_some_and(|controller| controller.is_dragged(app))
            }),
        );
        app.get_mut(self).scroll_controller = Some(scroll_controller);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        // If this is disposed during a drag, call `navigator.did_stop_user_gesture`.
        if app.get(self).drag_gesture_controller.is_some() {
            SchedulerBinding::add_post_frame_callback(
                app,
                FrameCallback::new(move |app, _timestamp| {
                    if let Some(controller) = app.get(self).drag_gesture_controller {
                        let navigator = app.get(controller).navigator;
                        if navigator.mounted(app) {
                            navigator.did_stop_user_gesture(app);
                        }
                    }
                    app.get_mut(self).drag_gesture_controller = None;
                }),
            );
        }
        self.scroll_controller(app).dispose(app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let builder = Rc::clone(&self.widget(app).builder);
        let scroll_controller = self.scroll_controller(app);
        builder(app, context, scroll_controller.as_controller())
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use reveal_embedder::{
        Locale, PointerChange, PointerData, PointerDataPacket, PointerDeviceKind, TextDirection,
    };
    use reveal_gestures::GestureBinding;
    use reveal_widgets::{
        Builder, DefaultWidgetsLocalizations, Directionality, Localizations, SingleChildScrollView,
    };

    use super::*;
    use crate::localizations::DefaultCupertinoLocalizations;
    use crate::route::CupertinoPageRoute;
    use crate::test_support::{app, build, pump};

    /// The test view is 800x600 physical at 2x.
    const VIEW_WIDTH: f64 = 400.0;
    const VIEW_HEIGHT: f64 = 300.0;

    /// The top of a fully open sheet: [`K_TOP_GAP_RATIO`] of the view height.
    const SHEET_TOP: f64 = VIEW_HEIGHT * K_TOP_GAP_RATIO;

    /// What a page builder reports back to the test on every build.
    #[derive(Default)]
    struct Probe {
        context: Cell<Option<BuildContext>>,
        has_parent_sheet: Cell<Option<bool>>,
        top_padding: Cell<Option<f64>>,
    }

    /// A page filling the view, keyed so a test can find its render box, reporting to `probe`.
    ///
    /// The report comes from a `Builder` so the context is the one the page's own widgets see,
    /// not the one the route handed the builder.
    fn page(probe: &Rc<Probe>, key: &GlobalKey) -> WidgetBuilder {
        let probe = Rc::clone(probe);
        let key: KeyRef = Rc::new(key.clone());
        Rc::new(move |_app: &mut App, _context: BuildContext| {
            let probe = Rc::clone(&probe);
            let key = key.clone();
            Builder::new(move |app: &mut App, context: BuildContext| {
                probe.context.set(Some(context));
                probe
                    .has_parent_sheet
                    .set(Some(CupertinoSheetRoute::has_parent_sheet(app, context)));
                probe
                    .top_padding
                    .set(Some(MediaQuery::padding_of(app, context).top));
                SizedBox::expand().key(key.clone()).into_widget()
            })
            .into_widget()
        })
    }

    /// Sends one pointer packet; the test view is 2x, so logical coordinates double.
    fn send(app: &mut App, change: PointerChange, x: f64, y: f64, at: Duration) {
        send_from(app, change, x, y, y, at);
    }

    /// Sends one pointer packet carrying the vertical movement from `previous_y`.
    fn send_from(
        app: &mut App,
        change: PointerChange,
        x: f64,
        y: f64,
        previous_y: f64,
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
                physical_delta_y: (y - previous_y) * 2.0,
                ..PointerData::default()
            }]),
        );
        app.drain_microtasks();
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

    /// The top-left corner the keyed widget paints at, in the view's coordinate system.
    fn top_left(key: &GlobalKey, app: &mut App) -> Offset {
        let context = key.current_context(app).expect("the widget is mounted");
        context
            .find_render_object(app)
            .expect("the widget has a render object")
            .as_box()
            .expect("the widget is a box")
            .local_to_global(app, Offset::ZERO, None)
    }

    /// An app showing one settled `CupertinoPageRoute` whose content reports to `probe`.
    fn app_with_page(probe: &Rc<Probe>, key: &GlobalKey) -> (App, Duration) {
        let mut app = app();
        let navigator_key = GlobalKey::new();
        let builder = page(probe, key);
        build(
            &mut app,
            navigator(&navigator_key, move |navigator| {
                navigator.on_generate_route(move |app, _settings| {
                    Some(Route::as_route(CupertinoPageRoute::new(
                        app,
                        Rc::clone(&builder),
                    )))
                })
            }),
        );
        let at = settle(&mut app, Duration::ZERO);
        (app, at)
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

    /// The whole scenario: a page, a sheet shown over it, and both settled.
    struct Sheet {
        app: App,
        page: Rc<Probe>,
        page_key: GlobalKey,
        sheet: Rc<Probe>,
        sheet_key: GlobalKey,
        route: AnyRoute,
        at: Duration,
    }

    /// Pushes a sheet over a page and runs `steps` frames of the push transition.
    fn show_sheet(show_drag_handle: bool) -> Sheet {
        let page_probe: Rc<Probe> = Rc::default();
        let page_key = GlobalKey::new();
        let (mut app, at) = app_with_page(&page_probe, &page_key);

        let sheet_probe: Rc<Probe> = Rc::default();
        let sheet_key = GlobalKey::new();
        let context = page_probe.context.get().expect("the page built");
        let builder = page(&sheet_probe, &sheet_key);
        let route = if show_drag_handle {
            // `show_cupertino_sheet` never forwards its `show_drag_handle`, as Dart's
            // `showCupertinoSheet` does not either, so a sheet with a handle is pushed as
            // its own route.
            let route = CupertinoSheetRoute::new(&mut app)
                .builder(&mut app, builder)
                .show_drag_handle(&mut app, true);
            let navigator = Navigator::of(&mut app, context, true);
            navigator.push(&mut app, Route::as_route(route))
        } else {
            show_cupertino_sheet(
                &mut app,
                context,
                Some(builder),
                None,
                false,
                true,
                None,
                None,
                false,
            )
        };
        Sheet {
            app,
            page: page_probe,
            page_key,
            sheet: sheet_probe,
            sheet_key,
            route,
            at,
        }
    }

    #[test]
    fn show_cupertino_sheet_slides_a_sheet_route_up_from_the_bottom() {
        let Sheet {
            mut app,
            sheet_key,
            route,
            at,
            ..
        } = show_sheet(false);
        let sheet_route = route
            .downcast::<CupertinoSheetRoute>(&app)
            .expect("show_cupertino_sheet pushes a CupertinoSheetRoute");

        let mut at = at;
        for _ in 0..4 {
            at += Duration::from_millis(20);
            pump(&mut app, at);
        }
        let mid = top_left(&sheet_key, &mut app).dy();
        assert!(
            mid > SHEET_TOP && mid < VIEW_HEIGHT,
            "the sheet is part way up from the bottom, at {mid}"
        );

        settle(&mut app, at);
        assert_eq!(
            top_left(&sheet_key, &mut app).dy(),
            SHEET_TOP,
            "the sheet stops below the top of the screen"
        );
        let animation = TransitionRoute::animation(sheet_route, &app).expect("an installed route");
        assert_eq!(animation.value(&app), 1.0);
        assert!(!TransitionRoute::opaque(sheet_route, &app));
        assert_eq!(
            TransitionRoute::transition_duration(sheet_route, &app),
            Duration::from_millis(500)
        );
    }

    #[test]
    fn the_route_below_slides_down_and_scales_behind_the_sheet() {
        let Sheet {
            mut app,
            page_key,
            at,
            ..
        } = show_sheet(false);
        settle(&mut app, at);

        let offset = top_left(&page_key, &mut app);
        let expected_top = VIEW_HEIGHT * 0.07;
        assert!(
            (offset.dy() - expected_top).abs() < 0.5,
            "the page below slid down by the top-down tween, to {}",
            offset.dy()
        );
        let expected_left = VIEW_WIDTH * K_SHEET_SCALE_FACTOR / 2.0;
        assert!(
            (offset.dx() - expected_left).abs() < 0.5,
            "the page below scaled down about its top centre, to {}",
            offset.dx()
        );
    }

    #[test]
    fn has_parent_sheet_is_true_inside_the_sheet_and_false_outside() {
        let Sheet {
            mut app,
            page,
            sheet,
            at,
            ..
        } = show_sheet(false);
        settle(&mut app, at);

        assert_eq!(
            page.has_parent_sheet.get(),
            Some(false),
            "the page below is not in a sheet"
        );
        assert_eq!(
            sheet.has_parent_sheet.get(),
            Some(true),
            "the sheet content is in a sheet"
        );
    }

    #[test]
    fn a_drag_handle_pads_the_sheet_content() {
        let Sheet {
            mut app, sheet, at, ..
        } = show_sheet(true);
        settle(&mut app, at);
        assert_eq!(
            sheet.top_padding.get(),
            Some(15.0),
            "the drag handle reserves the top padding of the sheet content"
        );
    }

    #[test]
    fn pop_sheet_pops_the_sheet() {
        let Sheet {
            mut app,
            sheet,
            route,
            at,
            ..
        } = show_sheet(false);
        let at = settle(&mut app, at);
        assert!(route.is_active(&app));

        let context = sheet.context.get().expect("the sheet built");
        CupertinoSheetRoute::pop_sheet(&mut app, context);
        settle(&mut app, at);
        assert!(!route.is_active(&app), "pop_sheet popped the sheet");
    }

    #[test]
    fn a_drag_past_the_dismiss_threshold_pops_the_sheet() {
        let Sheet {
            mut app, route, at, ..
        } = show_sheet(false);
        let at = settle(&mut app, at);

        send(&mut app, PointerChange::Down, 200.0, 50.0, Duration::ZERO);
        for step in 1_u64..=5 {
            send_from(
                &mut app,
                PointerChange::Move,
                200.0,
                50.0 + 40.0 * step as f64,
                50.0 + 40.0 * (step - 1) as f64,
                Duration::from_millis(step * 60),
            );
            pump(&mut app, at + Duration::from_millis(step * 60));
        }
        send(
            &mut app,
            PointerChange::Up,
            200.0,
            250.0,
            Duration::from_millis(300),
        );
        settle(&mut app, at + Duration::from_millis(300));

        assert!(!route.is_active(&app), "the drag dismissed the sheet");
    }

    #[test]
    fn a_short_drag_snaps_the_sheet_back() {
        let Sheet {
            mut app,
            sheet_key,
            route,
            at,
            ..
        } = show_sheet(false);
        let at = settle(&mut app, at);

        send(&mut app, PointerChange::Down, 200.0, 50.0, Duration::ZERO);
        for step in 1_u64..=4 {
            send_from(
                &mut app,
                PointerChange::Move,
                200.0,
                50.0 + 10.0 * step as f64,
                50.0 + 10.0 * (step - 1) as f64,
                Duration::from_millis(step * 60),
            );
            pump(&mut app, at + Duration::from_millis(step * 60));
        }
        let dragged = top_left(&sheet_key, &mut app).dy();
        assert!(
            dragged > SHEET_TOP,
            "the drag moved the sheet down, to {dragged}"
        );
        send(
            &mut app,
            PointerChange::Up,
            200.0,
            90.0,
            Duration::from_millis(240),
        );
        settle(&mut app, at + Duration::from_millis(240));

        assert!(
            route.is_active(&app),
            "the short drag did not pop the sheet"
        );
        assert_eq!(
            top_left(&sheet_key, &mut app).dy(),
            SHEET_TOP,
            "the sheet snapped back open"
        );
    }
    /// The whole scenario for a sheet whose content is a scroll view under the route's own
    /// controller: the view fills the sheet, the content is three views tall.
    struct ScrollableSheet {
        app: App,
        /// The scroll view, which is the sheet's own top.
        view_key: GlobalKey,
        /// The scrolled content, which moves as the list scrolls.
        content_key: GlobalKey,
        route: AnyRoute,
        at: Duration,
    }

    fn show_scrollable_sheet() -> ScrollableSheet {
        let page_probe: Rc<Probe> = Rc::default();
        let page_key = GlobalKey::new();
        let (mut app, at) = app_with_page(&page_probe, &page_key);

        let view_key = GlobalKey::new();
        let content_key = GlobalKey::new();
        let context = page_probe.context.get().expect("the page built");
        let scrollable_builder: ScrollableWidgetBuilder = {
            let view_key: KeyRef = Rc::new(view_key.clone());
            let content_key: KeyRef = Rc::new(content_key.clone());
            Rc::new(move |_app, _context, controller| {
                SingleChildScrollView::new()
                    .key(view_key.clone())
                    .controller(controller)
                    .child(
                        SizedBox::new()
                            .key(content_key.clone())
                            .width(VIEW_WIDTH)
                            .height(VIEW_HEIGHT * 3.0),
                    )
                    .into_widget()
            })
        };
        let route = show_cupertino_sheet(
            &mut app,
            context,
            None,
            Some(scrollable_builder),
            false,
            true,
            None,
            None,
            false,
        );
        let at = settle(&mut app, at);
        ScrollableSheet {
            app,
            view_key,
            content_key,
            route,
            at,
        }
    }

    #[test]
    fn a_scrollable_built_through_the_scrollable_builder_scrolls() {
        let ScrollableSheet {
            mut app,
            view_key,
            content_key,
            route,
            at,
        } = show_scrollable_sheet();
        assert_eq!(top_left(&view_key, &mut app).dy(), SHEET_TOP);
        assert_eq!(top_left(&content_key, &mut app).dy(), SHEET_TOP);

        // Dragging up scrolls the list; the sheet itself stays where it is.
        send(&mut app, PointerChange::Down, 200.0, 150.0, Duration::ZERO);
        for step in 1_u64..=4 {
            send_from(
                &mut app,
                PointerChange::Move,
                200.0,
                150.0 - 20.0 * step as f64,
                150.0 - 20.0 * (step - 1) as f64,
                Duration::from_millis(step * 60),
            );
            pump(&mut app, at + Duration::from_millis(step * 60));
        }
        send(
            &mut app,
            PointerChange::Up,
            200.0,
            70.0,
            Duration::from_millis(240),
        );
        settle(&mut app, at + Duration::from_millis(240));

        assert!(
            route.is_active(&app),
            "the scroll did not dismiss the sheet"
        );
        assert_eq!(
            top_left(&view_key, &mut app).dy(),
            SHEET_TOP,
            "the sheet stayed open"
        );
        let scrolled = top_left(&content_key, &mut app).dy();
        assert!(
            scrolled < SHEET_TOP,
            "the list scrolled its content up, to {scrolled}"
        );
    }

    #[test]
    fn a_downward_drag_at_the_top_of_the_scrollable_dismisses_the_sheet() {
        let ScrollableSheet {
            mut app,
            view_key,
            route,
            at,
            ..
        } = show_scrollable_sheet();

        send(&mut app, PointerChange::Down, 200.0, 50.0, Duration::ZERO);
        for step in 1_u64..=5 {
            send_from(
                &mut app,
                PointerChange::Move,
                200.0,
                50.0 + 40.0 * step as f64,
                50.0 + 40.0 * (step - 1) as f64,
                Duration::from_millis(step * 60),
            );
            pump(&mut app, at + Duration::from_millis(step * 60));
        }
        let dragged = top_left(&view_key, &mut app).dy();
        assert!(
            dragged > SHEET_TOP,
            "the drag at the top of the list moved the sheet down, to {dragged}"
        );
        send(
            &mut app,
            PointerChange::Up,
            200.0,
            250.0,
            Duration::from_millis(300),
        );
        settle(&mut app, at + Duration::from_millis(300));

        assert!(
            !route.is_active(&app),
            "the drag handed over to the sheet's dismiss gesture"
        );
    }

    #[test]
    fn a_nested_navigation_sheets_inner_route_pops_back_to_the_root() {
        let page_probe: Rc<Probe> = Rc::default();
        let page_key = GlobalKey::new();
        let (mut app, at) = app_with_page(&page_probe, &page_key);

        let sheet_probe: Rc<Probe> = Rc::default();
        let sheet_key = GlobalKey::new();
        let inner_probe: Rc<Probe> = Rc::default();
        let inner_key = GlobalKey::new();
        let context = page_probe.context.get().expect("the page built");
        let sheet_route = show_cupertino_sheet(
            &mut app,
            context,
            Some(page(&sheet_probe, &sheet_key)),
            None,
            true,
            true,
            None,
            None,
            false,
        );
        let at = settle(&mut app, at);
        assert_eq!(
            sheet_probe.has_parent_sheet.get(),
            Some(true),
            "the nested navigator's initial route is inside the sheet"
        );

        // The sheet's content sits under the nested navigator, so a push from it stays
        // inside the sheet.
        let sheet_context = sheet_probe.context.get().expect("the sheet built");
        let inner_builder = page(&inner_probe, &inner_key);
        let inner_route = CupertinoPageRoute::new(&mut app, inner_builder);
        let nested = Navigator::of(&mut app, sheet_context, false);
        let inner_route = nested.push(&mut app, Route::as_route(inner_route));
        let at = settle(&mut app, at);
        assert!(inner_route.is_active(&app), "the inner route was pushed");
        assert!(sheet_route.is_active(&app), "the sheet is still up");

        let inner_context = inner_probe.context.get().expect("the inner route built");
        let nested = Navigator::of(&mut app, inner_context, false);
        nested.pop(&mut app, None);
        settle(&mut app, at);

        assert!(!inner_route.is_active(&app), "the inner route popped");
        assert!(
            sheet_route.is_active(&app),
            "popping the inner route left the sheet up"
        );
    }
}
