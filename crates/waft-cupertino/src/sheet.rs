//! Flutter counterpart: `cupertino/sheet.dart`.

use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use waft_animation::{
    Animatable, AnimationBehavior, AnimationController, AnimationRef, CurvedAnimation, Curves,
    Drive, ValueTween,
};
use waft_embedder::{Brightness, Color, Offset, Radius, SystemUiOverlayStyle};
use waft_foundation::{
    App, ChangeNotifier, ChangeNotifierData, CompleterFuture, Handle, Listener, OptionRetained, Ref,
};
use waft_gestures::{
    Drag, DragEndDetails, DragGestureRecognizer, DragStartDetails, DragUpdateDetails,
    IOSScrollViewFlingVelocityTracker, PointerDownEvent, PointerEvent, RecognizerLeaf,
    VerticalDragGestureRecognizer,
};
use waft_painting::{
    Alignment, AlignmentGeometry, BorderRadius, BorderRadiusGeometry, BorderSide, EdgeInsets,
    EdgeInsetsGeometry, RoundedSuperellipseBorder, ShapeDecoration,
};
use waft_rendering::{HitTestBehavior, StackFit, ViewportOffset};
use waft_scheduler::{
    FrameCallback, SchedulerBinding, Ticker, TickerCallback, TickerProviderObject,
};
use waft_widgets::{
    Align, AlwaysScrollableScrollPhysics, AnimatedBuilder, AnnotatedRegion, AnyRoute,
    AnyScrollPosition, AnyTransitionRoute, BuildContext, ClipRSuperellipse, ColoredBox,
    DecoratedBox, DelegatedTransitionBuilder, FadeTransition, GlobalKey, InheritedWidget,
    IntoWidget, KeyRef, LocalHistoryRoute, LocalHistoryRouteData, MediaQuery, ModalRoute,
    ModalRouteData, Navigator, NavigatorPopHandler, NavigatorState, OverlayRoute, OverlayRouteData,
    Padding, PageRoute, PageRouteData, PopScope, PredictiveBackRoute, Route, RouteData,
    RouteResult, RouteSettingsRef, ScaleTransition, ScrollActivityDelegate, ScrollActivityRef,
    ScrollContext, ScrollControllerData, ScrollControllerLeaf, ScrollControllerRef,
    ScrollPhysicsRef, ScrollPosition, ScrollPositionData, ScrollPositionRef,
    ScrollPositionWithSingleContext, ScrollPositionWithSingleContextData,
    ScrollPositionWithSingleContextLeaf, ScrollableWidgetBuilder, SingleTickerProviderStateMixin,
    SingleTickerProviderStateMixinData, SizedBox, SlideTransition, Stack, State, StateData,
    StatefulWidget, TransitionRoute, TransitionRouteData, WidgetBuilder, WidgetRef,
};

use crate::colors::CupertinoColors;
use crate::interface_level::{CupertinoUserInterfaceLevel, CupertinoUserInterfaceLevelData};
use crate::route::{CupertinoPageRoute, stop_user_gesture_when_the_animation_ends};
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

/// Dart's `Tween<BorderRadiusGeometry>` as a value, a [`ValueTween`] whose type has no
/// `TweenLerp`: waft-painting does not depend on waft-animation, so neither crate can give it one.
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
const K_BOTTOM_UP_TWEEN: ValueTween<Offset> = ValueTween::new(Offset::new(0.0, 1.0), Offset::ZERO);

// Offset change for when a new sheet covers another sheet. '0.0' represents the
// top of the space available for the new sheet, but because the previous sheet
// was lowered slightly, the new sheet needs to go slightly higher than that.
// Values found from eyeballing a simulator running iOS 18.0.
const K_BOTTOM_UP_TWEEN_WHEN_COVERING_OTHER_SHEET: ValueTween<Offset> =
    ValueTween::new(Offset::new(0.0, 1.0), Offset::new(0.0, -0.02));

// Tween that animates a sheet slightly up when it is covered by a new sheet.
// Values found from eyeballing a simulator running iOS 18.0.
const K_MID_UP_TWEEN: ValueTween<Offset> = ValueTween::new(Offset::ZERO, Offset::new(0.0, -0.005));

// Offset from top of screen to slightly down when a fullscreen page is covered
// by a sheet. Values found from eyeballing a simulator running iOS 18.0.
const K_TOP_DOWN_TWEEN: ValueTween<Offset> = ValueTween::new(Offset::ZERO, Offset::new(0.0, 0.07));

// Opacity of the overlay color put over the sheet as it moves into the background.
// Used to distinguish the sheet from the background. Value derived from eyeballing
// a simulator running iOS 18.0.
const K_OPACITY_TWEEN: ValueTween<f64> = ValueTween::new(0.0, 0.10);

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

const K_SCALE_TWEEN: ValueTween<f64> = ValueTween::new(1.0, 1.0 - K_SHEET_SCALE_FACTOR);

/// Dart's `ValueGetter<bool>` on a route flag.
type RouteFlagGetter = Rc<dyn Fn(&App) -> bool>;

/// Dart's `ValueGetter<_CupertinoDragGestureController<T>>`.
type StartPopGestureCallback = Rc<dyn Fn(&mut App) -> Ref<CupertinoDragGestureController>>;

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
/// Returns a future that resolves to the value (if any) that was passed to [`Navigator::pop`]
/// when the sheet was closed.
///
/// See also:
///
///  * [`CupertinoSheetRoute`] the basic route version of the sheet view.
///  * [`ShowCupertinoDialog`] which displays an iOS-styled
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
) -> CompleterFuture<RouteResult> {
    debug_assert!(
        top_gap.is_none_or(|top_gap| (0.0..=0.9).contains(&top_gap)),
        "topGap must be between 0.0 and 0.9"
    );
    debug_assert!(builder.is_some() || scrollable_builder.is_some());
    debug_assert!(
        (builder.is_none() && scrollable_builder.is_some()) || scrollable_builder.is_none()
    );

    let effective_builder = builder;
    let nested_navigator_key = GlobalKey::new();
    if !use_nested_navigation {
        let mut route = CupertinoSheetRoute::new(app).enable_drag(app, enable_drag);
        if let Some(effective_builder) = effective_builder {
            route = route.builder(app, effective_builder);
        }
        if let Some(scrollable_builder) = scrollable_builder {
            route = route.scrollable_builder(app, scrollable_builder);
        }
        if let Some(settings) = settings {
            route = route.settings(app, settings);
        }
        if let Some(top_gap) = top_gap {
            route = route.top_gap(app, top_gap);
        }
        let route = route.show_drag_handle(app, show_drag_handle);

        let navigator = Navigator::of(app, context, true);
        navigator.push(app, route.into_erased())
    } else {
        let mut route = CupertinoSheetRoute::new(app)
            .enable_drag(app, enable_drag)
            .scrollable_builder(
                app,
                Rc::new(move |_app: &mut App, _context: BuildContext, controller| {
                    let builder: WidgetBuilder = match &scrollable_builder {
                        Some(scrollable_builder) => {
                            let scrollable_builder = Rc::clone(scrollable_builder);
                            Rc::new(move |app: &mut App, context: BuildContext| {
                                scrollable_builder(app, context, controller.clone())
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
            route = route.settings(app, settings);
        }
        if let Some(top_gap) = top_gap {
            route = route.top_gap(app, top_gap);
        }
        let route = route.show_drag_handle(app, show_drag_handle);
        let navigator = Navigator::of(app, context, true);
        navigator.push(app, route.into_erased())
    }
}

/// Dart's local `nestedNavigationContent` inside `showCupertinoSheet`.
fn nested_navigation_content(
    nested_navigator_key: &GlobalKey,
    builder: WidgetBuilder,
) -> WidgetRef {
    let key: KeyRef = nested_navigator_key.clone().into();
    let pop_key = nested_navigator_key.clone();
    NavigatorPopHandler::new(
        Navigator::new()
            .key(key)
            .initial_route("/")
            .on_generate_initial_routes(
                move |app: &mut App, _navigator: Handle<NavigatorState>, _initial_route_name| {
                    let builder = Rc::clone(&builder);
                    vec![
                        CupertinoPageRoute::new(
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
                        )
                        .into_erased(),
                    ]
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
    pub primary_route_animation: AnimationRef<f64>,

    /// A linear route animation from 0.0 to 1.0 when another screen is being pushed on top of
    /// this one.
    pub secondary_route_animation: AnimationRef<f64>,

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
        primary_route_animation: AnimationRef<f64>,
        secondary_route_animation: AnimationRef<f64>,
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
        animation: AnimationRef<f64>,
        secondary_animation: AnimationRef<f64>,
        allow_snapshotting: bool,
        child: Option<&WidgetRef>,
    ) -> Option<WidgetRef> {
        let _ = (animation, allow_snapshotting);
        if CupertinoSheetRoute::has_parent_sheet(app, context) {
            return CupertinoSheetTransition::delegated_cover_sheet_secondary_transition(
                app,
                secondary_animation.clone(),
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
            CurvedAnimation::new(app, secondary_animation.clone(), curve, Some(reverse_curve));

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

        let radius_animation = curved_animation.drive(app, decoration_tween);
        let opacity_animation = curved_animation.drive(app, K_OPACITY_TWEEN);
        let slide_animation = curved_animation.drive(app, K_TOP_DOWN_TWEEN);
        let scale_animation = curved_animation.drive(app, K_SCALE_TWEEN);
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
                                    Rc::new(radius_animation.clone()),
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
        secondary_animation: AnimationRef<f64>,
        child: Option<&WidgetRef>,
    ) -> Option<WidgetRef> {
        let curve = Curves::linear_to_ease_out();
        let reverse_curve = Curves::ease_in_to_linear();
        let curved_animation =
            CurvedAnimation::new(app, secondary_animation, curve, Some(reverse_curve));

        let slide_animation = curved_animation.drive(app, K_MID_UP_TWEEN);
        let scale_animation = curved_animation.drive(app, K_SCALE_TWEEN);
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
    stretch_drag_controller: Option<Ref<AnimationController>>,
    // Animates the top padding of the sheet based on the stretch drag controller's value.
    stretch_drag_animation: Option<AnimationRef<f64>>,
    // The offset animation when this page is being covered by another sheet.
    secondary_position_animation: Option<AnimationRef<Offset>>,
    // The scale animation when this page is being covered by another sheet.
    secondary_scale_animation: Option<AnimationRef<f64>>,
    // Curve of primary page which is coming in to cover another route.
    primary_position_curve: Option<Ref<CurvedAnimation>>,
    // Curve of secondary page which is becoming covered by another sheet.
    secondary_position_curve: Option<Ref<CurvedAnimation>>,
}

impl CupertinoSheetTransitionState {
    fn stretch_drag_controller(self: Handle<Self>, app: &App) -> Handle<AnimationController> {
        app.get(self)
            .stretch_drag_controller
            .handle()
            .expect("init_state ran first")
    }

    fn setup_animation(self: Handle<Self>, app: &mut App) {
        let widget = self.widget(app);
        let primary_route_animation = widget.primary_route_animation.clone();
        let secondary_route_animation = widget.secondary_route_animation.clone();
        let top_gap = widget.top_gap;
        let primary_position_curve = CurvedAnimation::new(
            app,
            primary_route_animation,
            Curves::fast_ease_in_to_slow_ease_out(),
            Some(Rc::new(Curves::fast_ease_in_to_slow_ease_out().flipped())),
        );
        let secondary_position_curve = CurvedAnimation::new(
            app,
            secondary_route_animation,
            Curves::linear_to_ease_out(),
            Some(Curves::ease_in_to_linear()),
        );
        let state = app.get_mut(self);
        state.primary_position_curve = Some(primary_position_curve);
        state.secondary_position_curve = Some(secondary_position_curve.clone());
        // Maintain the same stretch distance (0.008 of screen height) regardless of custom
        // top_gap.
        const STRETCH_DISTANCE: f64 = K_TOP_GAP_RATIO - K_STRETCHED_TOP_GAP_RATIO;
        let stretched_top_gap = top_gap - STRETCH_DISTANCE;
        let stretch_drag_animation = app
            .get(self)
            .stretch_drag_controller
            .clone()
            .expect("init_state ran first")
            .drive(app, ValueTween::new(top_gap, stretched_top_gap));
        let secondary_position_animation = secondary_position_curve.drive(app, K_MID_UP_TWEEN);
        let secondary_scale_animation = secondary_position_curve.drive(app, K_SCALE_TWEEN);
        let state = app.get_mut(self);
        state.stretch_drag_animation = Some(stretch_drag_animation);
        state.secondary_position_animation = Some(secondary_position_animation);
        state.secondary_scale_animation = Some(secondary_scale_animation);
    }

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

    fn cover_sheet_primary_transition(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        animation: AnimationRef<f64>,
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
        let curved_animation = CurvedAnimation::new(app, animation, curve, Some(reverse_curve));

        let position_animation = curved_animation.drive(app, offset_tween);

        curved_animation.dispose(app);

        SlideTransition::new(position_animation)
            .child(child)
            .into_widget()
    }

    fn cover_sheet_secondary_transition(
        self: Handle<Self>,
        app: &App,
        secondary_animation: AnimationRef<f64>,
        child: WidgetRef,
    ) -> WidgetRef {
        let _ = secondary_animation;
        let state = app.get(self);
        let secondary_position_animation = state
            .secondary_position_animation
            .clone()
            .expect("init_state ran first");
        let secondary_scale_animation = state
            .secondary_scale_animation
            .clone()
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
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Ref<Ticker> {
        SingleTickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl State for CupertinoSheetTransitionState {
    type Widget = CupertinoSheetTransition;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let stretch_drag_controller = AnimationController::new(
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

    fn activate(self: Handle<Self>, app: &mut App) {
        SingleTickerProviderStateMixin::activate(self, app);
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
            .clone()
            .expect("init_state ran first");
        let stretch_drag_controller = app
            .get(self)
            .stretch_drag_controller
            .clone()
            .expect("init_state ran first");
        StretchDragControllerProvider {
            controller: stretch_drag_controller,
            child: SizedBox::expand()
                .child(AnimatedBuilder::new(
                    Rc::new(stretch_drag_animation.clone()),
                    move |app: &mut App, context: BuildContext, _child: Option<&WidgetRef>| {
                        let top =
                            MediaQuery::height_of(app, context) * stretch_drag_animation.value(app);
                        let widget = self.widget(app);
                        let primary_route_animation = widget.primary_route_animation.clone();
                        let secondary_route_animation = widget.secondary_route_animation.clone();
                        let linear_transition = widget.linear_transition;
                        let child = widget.child.clone();
                        let primary = self.cover_sheet_primary_transition(
                            app,
                            context,
                            primary_route_animation,
                            linear_transition,
                            child,
                        );
                        self.cover_sheet_secondary_transition(
                            app,
                            secondary_route_animation,
                            Padding::new(EdgeInsetsGeometry::only(0.0, top, 0.0, 0.0))
                                .child(primary)
                                .into_widget(),
                        )
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
    controller: Ref<AnimationController>,
    child: WidgetRef,
}

impl StretchDragControllerProvider {
    fn maybe_of(app: &App, context: BuildContext) -> Option<Handle<AnimationController>> {
        context
            .get_inherited_widget_of_exact_type::<StretchDragControllerProvider>(app)
            .map(|provider| provider.controller.handle())
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
    pub fn new(app: &mut App) -> Ref<CupertinoSheetRoute> {
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
    pub fn builder(self: Ref<Self>, app: &mut App, builder: WidgetBuilder) -> Ref<Self> {
        app.get_mut(&self).builder = Some(builder);
        self
    }

    /// Dart `CupertinoSheetRoute(scrollableBuilder:)`.
    pub fn scrollable_builder(
        self: Ref<Self>,
        app: &mut App,
        scrollable_builder: ScrollableWidgetBuilder,
    ) -> Ref<Self> {
        app.get_mut(&self).scrollable_builder = Some(scrollable_builder);
        self
    }

    /// Dart `CupertinoSheetRoute(settings:)`.
    pub fn settings(self: Ref<Self>, app: &mut App, settings: RouteSettingsRef) -> Ref<Self> {
        self.route_data_mut(app).set_settings(settings);
        self
    }

    /// Dart `CupertinoSheetRoute(enableDrag:)`.
    pub fn enable_drag(self: Ref<Self>, app: &mut App, enable_drag: bool) -> Ref<Self> {
        app.get_mut(&self).enable_drag = enable_drag;
        self
    }

    /// Dart `CupertinoSheetRoute(showDragHandle:)`.
    pub fn show_drag_handle(self: Ref<Self>, app: &mut App, show_drag_handle: bool) -> Ref<Self> {
        app.get_mut(&self).show_drag_handle = show_drag_handle;
        self
    }

    /// Dart `CupertinoSheetRoute(topGap:)`.
    pub fn top_gap(self: Ref<Self>, app: &mut App, top_gap: f64) -> Ref<Self> {
        debug_assert!(
            (0.0..=0.9).contains(&top_gap),
            "topGap must be between 0.0 and 0.9"
        );
        app.get_mut(&self).top_gap = Some(top_gap);
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
        controller: ScrollControllerRef,
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
    waft_widgets::page_route_overrides!();
}

impl OverlayRoute for CupertinoSheetRoute {
    waft_widgets::modal_overlay_route_overrides!();
}

impl PredictiveBackRoute for CupertinoSheetRoute {
    waft_widgets::page_predictive_back_route_overrides!();
}

impl TransitionRoute for CupertinoSheetRoute {
    waft_widgets::modal_transition_route_overrides!();

    fn allow_snapshotting(self: Handle<Self>, app: &App) -> bool {
        PageRoute::allow_snapshotting(self, app)
    }

    fn transition_duration(self: Handle<Self>, app: &mut App) -> Duration {
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
    waft_widgets::local_history_route_accessors!();
}

impl ModalRoute for CupertinoSheetRoute {
    waft_widgets::modal_route_accessors!();

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
        animation: AnimationRef<f64>,
        secondary_animation: AnimationRef<f64>,
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
        animation: AnimationRef<f64>,
        secondary_animation: AnimationRef<f64>,
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
    waft_widgets::page_route_accessors!();
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
    fn transition_duration(self: Handle<Self>, app: &mut App) -> Duration {
        let _ = app;
        Duration::from_millis(500)
    }

    /// Dart's `_CupertinoSheetRouteTransitionMixin.delegatedTransition`.
    fn delegated_transition(self: Handle<Self>, app: &App) -> Option<DelegatedTransitionBuilder> {
        if self.has_custom_top_gap(app) {
            return None;
        }
        Some(DelegatedTransitionBuilder::function(
            CupertinoSheetTransition::delegate_transition,
        ))
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
        animation: AnimationRef<f64>,
        secondary_animation: AnimationRef<f64>,
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
        animation: AnimationRef<f64>,
        secondary_animation: AnimationRef<f64>,
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
) -> Ref<CupertinoDragGestureController> {
    let navigator = Route::navigator(this, app).expect("an installed route");
    let controller = TransitionRoute::controller(this, app)
        .expect("an installed route has a controller")
        .keep(app);
    // Dart's closures keep the route for as long as the gesture lasts.
    let kept_route = this.keep(app);
    CupertinoDragGestureController::new(
        app,
        top_gap,
        navigator,
        Rc::new({
            let kept_route = kept_route.clone();
            move |app| kept_route.as_route().is_current(app)
        }),
        Rc::new(move |app| kept_route.as_route().is_active(app)),
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
    animation: AnimationRef<f64>,
    secondary_animation: AnimationRef<f64>,
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

// Reads the height from the render box instead of context.size, which throws
// in debug mode when the box is dirty for layout. A drag can end while a
// relayout is pending, and the last laid out height is the one the drag was
// made against.
fn sheet_height_of(app: &App, context: BuildContext) -> f64 {
    context
        .find_render_object(app)
        .expect("the sheet has a render object")
        .as_box()
        .expect("the sheet is a box")
        .size(app)
        .height()
}

/// Dart's `_CupertinoDragGestureDetector`.
struct CupertinoDragGestureDetector {
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
    drag_gesture_controller: Option<Ref<CupertinoDragGestureController>>,
    recognizer: Option<Ref<VerticalDragGestureRecognizer>>,
    /// The enclosing transition's controller, an ancestor's.
    stretch_drag_controller: Option<Handle<AnimationController>>,
}

impl CupertinoDragGestureDetectorState {
    /// Dart's `_cupertinoVelocityBuilder`.
    fn cupertino_velocity_builder(
        event: &PointerEvent,
    ) -> Box<dyn waft_gestures::AnyVelocityTracker> {
        Box::new(IOSScrollViewFlingVelocityTracker::new(event.kind()))
    }

    fn recognizer(self: Handle<Self>, app: &App) -> Handle<VerticalDragGestureRecognizer> {
        app.get(self)
            .recognizer
            .handle()
            .expect("init_state ran first")
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
            .handle()
            .expect("a drag start ran first");
        let Some(stretch_drag_controller) = app.get(self).stretch_drag_controller else {
            return;
        };
        let sheet_height = sheet_height_of(app, self.context(app));
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
            .handle()
            .expect("a drag start ran first");
        let Some(stretch_drag_controller) = app.get(self).stretch_drag_controller else {
            app.get_mut(self).drag_gesture_controller = None;
            return;
        };
        let sheet_height = sheet_height_of(app, self.context(app));
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
        if let Some(controller) = app.get(self).drag_gesture_controller.handle() {
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
    waft_widgets::state_accessors!();

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
            // The callback keeps the disposed state, as Dart's closure keeps `this`.
            let this = self.keep(app);
            SchedulerBinding::add_post_frame_callback(
                app,
                FrameCallback::new(move |app, _timestamp| {
                    let state = this.handle();
                    if let Some(controller) = app.get(state).drag_gesture_controller.handle() {
                        let navigator = app.get(controller).navigator;
                        if navigator.mounted(app) {
                            navigator.did_stop_user_gesture(app);
                        }
                    }
                    app.get_mut(state).drag_gesture_controller = None;
                }),
                None,
            );
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let _ = context;
        let child = self.widget(app).child.clone();
        waft_widgets::Listener::new()
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
    pop_drag_controller: Ref<AnimationController>,
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
        pop_drag_controller: Ref<AnimationController>,
    ) -> Ref<CupertinoDragGestureController> {
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
        let pop_drag_controller = app.get(self).pop_drag_controller.handle();
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
        let pop_drag_controller = app.get(self).pop_drag_controller.handle();
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
            let navigator = app.get(self).navigator;
            stop_user_gesture_when_the_animation_ends(app, navigator, pop_drag_controller);
        } else {
            let navigator = app.get(self).navigator;
            navigator.did_stop_user_gesture(app);
        }
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
    ) -> Ref<CupertinoSheetScrollController> {
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
    waft_widgets::scroll_controller_accessors!();

    fn create_scroll_position(
        self: Handle<Self>,
        app: &mut App,
        physics: ScrollPhysicsRef,
        context: Rc<dyn ScrollContext>,
        old_position: Option<AnyScrollPosition>,
    ) -> ScrollPositionRef {
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
        .into_erased()
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
    ballistic_controllers: Vec<Ref<AnimationController>>,
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
    ) -> Ref<CupertinoSheetScrollPosition> {
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
        ScrollPositionWithSingleContext::init(this.handle(), app, Some(0.0), old_position);
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
    waft_widgets::scroll_position_with_single_context_viewport_offset_overrides!();
}

impl ScrollPosition for CupertinoSheetScrollPosition {
    waft_widgets::scroll_position_with_single_context_overrides!();
}

impl ScrollActivityDelegate for CupertinoSheetScrollPosition {
    waft_widgets::scroll_position_with_single_context_delegate_overrides!();
}

impl ScrollPositionWithSingleContextLeaf for CupertinoSheetScrollPosition {
    waft_widgets::scroll_position_with_single_context_accessors!();

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

    fn begin_activity(self: Handle<Self>, app: &mut App, new_activity: Option<ScrollActivityRef>) {
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
    scroll_controller: Option<Ref<CupertinoSheetScrollController>>,
    drag_gesture_controller: Option<Ref<CupertinoDragGestureController>>,
}

impl CupertinoDraggableScrollableSheetState {
    fn scroll_controller(self: Handle<Self>, app: &App) -> Handle<CupertinoSheetScrollController> {
        app.get(self)
            .scroll_controller
            .handle()
            .expect("init_state has run")
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
        if let Some(controller) = app.get(self).drag_gesture_controller.handle() {
            let sheet_height = sheet_height_of(app, self.context(app));
            controller.drag_update(
                app,
                delta / (sheet_height - (sheet_height * K_TOP_GAP_RATIO)),
                None,
            );
        }
    }

    fn handle_drag_end(self: Handle<Self>, app: &mut App, velocity: f64) {
        debug_assert!(self.mounted(app));
        if let Some(controller) = app.get(self).drag_gesture_controller.handle() {
            controller.drag_end(
                app,
                -velocity / sheet_height_of(app, self.context(app)),
                None,
            );
            app.get_mut(self).drag_gesture_controller = None;
        }
    }
}

impl State for CupertinoDraggableScrollableSheetState {
    type Widget = CupertinoDraggableScrollableSheet;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let scroll_controller = CupertinoSheetScrollController::new(
            app,
            Rc::new(move |app: &mut App| self.drag_start(app)),
            Rc::new(move |app: &mut App, delta| self.drag_update(app, delta)),
            Rc::new(move |app: &mut App, velocity| self.handle_drag_end(app, velocity)),
            Rc::new(move |app: &App| {
                app.get(self)
                    .drag_gesture_controller
                    .handle()
                    .is_some_and(|controller| controller.is_dragged(app))
            }),
        );
        app.get_mut(self).scroll_controller = Some(scroll_controller);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        // If this is disposed during a drag, call `navigator.did_stop_user_gesture`.
        if app.get(self).drag_gesture_controller.is_some() {
            // The callback keeps the disposed state, as Dart's closure keeps `this`.
            let this = self.keep(app);
            SchedulerBinding::add_post_frame_callback(
                app,
                FrameCallback::new(move |app, _timestamp| {
                    let state = this.handle();
                    if let Some(controller) = app.get(state).drag_gesture_controller.handle() {
                        let navigator = app.get(controller).navigator;
                        if navigator.mounted(app) {
                            navigator.did_stop_user_gesture(app);
                        }
                    }
                    app.get_mut(state).drag_gesture_controller = None;
                }),
                None,
            );
        }
        self.scroll_controller(app).dispose(app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let builder = Rc::clone(&self.widget(app).builder);
        let scroll_controller = app
            .get(self)
            .scroll_controller
            .as_ref()
            .expect("init_state has run")
            .erase();
        builder(app, context, scroll_controller)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use waft_embedder::{Locale, PointerChange, TextDirection};
    use waft_test::{assert_no_new_objects, object_snapshot};
    use waft_widgets::testing::{TestPointer, WidgetTester, test_widgets};
    use waft_widgets::{
        Builder, DefaultWidgetsLocalizations, Directionality, Localizations, SingleChildScrollView,
    };

    use super::*;
    use crate::localizations::DefaultCupertinoLocalizations;
    use crate::route::CupertinoPageRoute;

    /// The tester's view, in logical pixels.
    const VIEW_WIDTH: f64 = 800.0;
    const VIEW_HEIGHT: f64 = 600.0;

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
        let key: KeyRef = key.clone().into();
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

    /// Moves `finger` to `to`, the move stamped `time_stamp` as a host stamps it.
    fn move_finger(
        tester: &mut WidgetTester,
        finger: &mut TestPointer,
        to: Offset,
        time_stamp: Duration,
    ) {
        let delta = to - finger.position;
        finger.position = to;
        tester.send_at(finger, PointerChange::Move, delta, time_stamp);
    }

    /// Lifts `finger` where it is, stamped `time_stamp`.
    fn lift_finger(tester: &mut WidgetTester, finger: &TestPointer, time_stamp: Duration) {
        tester.send_at(finger, PointerChange::Up, Offset::ZERO, time_stamp);
    }

    /// A vertical drag of `steps` moves of `step` each from `start`, one every 60 milliseconds
    /// with a frame after each; the finger stays down.
    fn drag_down(tester: &mut WidgetTester, start: Offset, step: f64, steps: u32) -> TestPointer {
        let mut finger = TestPointer::touch(1);
        tester.down(&mut finger, start);
        for n in 1..=steps {
            let to = start + Offset::new(0.0, step * f64::from(n));
            move_finger(tester, &mut finger, to, Duration::from_millis(60) * n);
            tester.pump_by(Duration::from_millis(60));
        }
        finger
    }

    /// The top-left corner the keyed widget paints at, in the view's coordinate system.
    fn top_left(key: &GlobalKey, tester: &WidgetTester) -> Offset {
        let render = tester
            .render_object(key)
            .as_box()
            .expect("the widget is a box");
        render.local_to_global(&tester.cell.borrow(), Offset::ZERO, None)
    }

    /// An app showing one settled `CupertinoPageRoute` whose content reports to `probe`.
    fn app_with_page(tester: &mut WidgetTester, probe: &Rc<Probe>, key: &GlobalKey) {
        let navigator_key = GlobalKey::new();
        let builder = page(probe, key);
        tester.pump_widget(navigator(&navigator_key, move |navigator| {
            navigator.on_generate_route(move |app, _settings| {
                Some(CupertinoPageRoute::new(app, Rc::clone(&builder)).into_erased())
            })
        }));
        tester.pump_and_settle();
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

    /// The whole scenario: a page, a sheet shown over it, and both settled.
    struct Sheet {
        page: Rc<Probe>,
        page_key: GlobalKey,
        sheet: Rc<Probe>,
        sheet_key: GlobalKey,
        popped: CompleterFuture<RouteResult>,
    }

    /// Pushes a sheet over a settled page; the push has not run a frame yet.
    fn show_sheet(tester: &mut WidgetTester, show_drag_handle: bool) -> Sheet {
        let page_probe: Rc<Probe> = Rc::default();
        let page_key = GlobalKey::new();
        app_with_page(tester, &page_probe, &page_key);
        let mut app = tester.cell.borrow_mut();

        let sheet_probe: Rc<Probe> = Rc::default();
        let sheet_key = GlobalKey::new();
        let context = page_probe.context.get().expect("the page built");
        let builder = page(&sheet_probe, &sheet_key);
        let popped = show_cupertino_sheet(
            &mut app,
            context,
            Some(builder),
            None,
            false,
            true,
            None,
            None,
            show_drag_handle,
        );
        drop(app);
        Sheet {
            page: page_probe,
            page_key,
            sheet: sheet_probe,
            sheet_key,
            popped,
        }
    }

    /// The route of the sheet whose content reported to `sheet`: Dart's `ModalRoute.of`.
    fn route_of_sheet(app: &mut App, sheet: &Probe) -> AnyRoute {
        let context = sheet.context.get().expect("the sheet built");
        waft_widgets::AnyModalRoute::of(app, context)
            .expect("the sheet content is under its route")
            .as_route()
    }

    /// Not a Dart case: Dart's collector frees what each frame of a sheet's push builds and
    /// replaces, for the sheet and for the page it covers; here the transitions free what they
    /// make once they hand it on.
    #[test]
    fn a_frame_of_a_sheet_push_leaves_no_new_arena_objects() {
        test_widgets(|tester| {
            show_sheet(tester, false);
            for _ in 0..3 {
                tester.pump_by(Duration::from_millis(20));
            }
            let snapshot = object_snapshot(&tester.cell.borrow());
            tester.pump_by(Duration::from_millis(20));
            assert_no_new_objects(&tester.cell.borrow(), &snapshot);
        });
    }

    #[test]
    fn show_cupertino_sheet_slides_a_sheet_route_up_from_the_bottom() {
        test_widgets(|tester| {
            let Sheet {
                sheet, sheet_key, ..
            } = show_sheet(tester, false);

            for _ in 0..4 {
                tester.pump_by(Duration::from_millis(20));
            }
            let mid = top_left(&sheet_key, tester).dy();
            assert!(
                mid > SHEET_TOP && mid < VIEW_HEIGHT,
                "the sheet is part way up from the bottom, at {mid}"
            );

            tester.pump_and_settle();
            assert_eq!(
                top_left(&sheet_key, tester).dy(),
                SHEET_TOP,
                "the sheet stops below the top of the screen"
            );
            let mut app = tester.cell.borrow_mut();
            let sheet_route = route_of_sheet(&mut app, &sheet)
                .downcast::<CupertinoSheetRoute>(&app)
                .expect("show_cupertino_sheet pushes a CupertinoSheetRoute");
            let animation =
                TransitionRoute::animation(sheet_route, &app).expect("an installed route");
            assert_eq!(animation.value(&app), 1.0);
            assert!(!TransitionRoute::opaque(sheet_route, &app));
            assert_eq!(
                TransitionRoute::transition_duration(sheet_route, &mut app),
                Duration::from_millis(500)
            );
        });
    }

    #[test]
    fn the_route_below_slides_down_and_scales_behind_the_sheet() {
        test_widgets(|tester| {
            let Sheet { page_key, .. } = show_sheet(tester, false);
            tester.pump_and_settle();

            let offset = top_left(&page_key, tester);
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
        });
    }

    #[test]
    fn has_parent_sheet_is_true_inside_the_sheet_and_false_outside() {
        test_widgets(|tester| {
            let Sheet { page, sheet, .. } = show_sheet(tester, false);
            tester.pump_and_settle();

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
        });
    }

    #[test]
    fn a_drag_handle_pads_the_sheet_content() {
        test_widgets(|tester| {
            let Sheet { sheet, .. } = show_sheet(tester, true);
            tester.pump_and_settle();
            assert_eq!(
                sheet.top_padding.get(),
                Some(15.0),
                "the drag handle reserves the top padding of the sheet content"
            );
        });
    }

    /// How many drag handles are painted under `root`: decorated boxes whose shape is filled
    /// with the handle's tertiary label colour.
    fn drag_handle_count(app: &App, root: waft_rendering::AnyRenderObject) -> usize {
        let mut count = 0;
        if let Some(decorated) = root.downcast::<waft_rendering::RenderDecoratedBox>(app)
            && decorated
                .decoration(app)
                .as_any()
                .downcast_ref::<ShapeDecoration>()
                .is_some_and(|decoration| {
                    decoration.color.as_ref() == Some(&CupertinoColors::TERTIARY_LABEL)
                })
        {
            count += 1;
        }
        root.visit_children(app, &mut |child| count += drag_handle_count(app, child));
        count
    }

    #[test]
    fn show_cupertino_sheet_forwards_show_drag_handle_to_the_sheet_route() {
        // Regression test for https://github.com/flutter/flutter/issues/187693.
        for use_nested_navigation in [false, true] {
            test_widgets(|tester| {
                let page_probe: Rc<Probe> = Rc::default();
                let page_key = GlobalKey::new();
                app_with_page(tester, &page_probe, &page_key);
                let mut app = tester.cell.borrow_mut();
                let context = page_probe.context.get().expect("the page built");
                let sheet_probe: Rc<Probe> = Rc::default();
                let sheet_key = GlobalKey::new();
                let builder = page(&sheet_probe, &sheet_key);
                show_cupertino_sheet(
                    &mut app,
                    context,
                    None,
                    Some(Rc::new(move |app, context, _controller| {
                        builder(app, context)
                    })),
                    use_nested_navigation,
                    true,
                    None,
                    None,
                    true,
                );
                drop(app);
                tester.pump_and_settle();
                let app = tester.cell.borrow();

                assert!(
                    sheet_probe.context.get().is_some(),
                    "the sheet content built (nested navigation {use_nested_navigation})"
                );
                let root = context
                    .find_ancestor_render_object_of_type::<waft_rendering::RenderView>(&app)
                    .expect("the page is under the view")
                    .as_object();
                assert_eq!(
                    drag_handle_count(&app, root),
                    1,
                    "one drag handle (nested navigation {use_nested_navigation})"
                );
            });
        }
    }

    #[test]
    fn a_covered_sheet_does_not_reveal_the_root_route_through_its_top_gap() {
        test_widgets(|tester| {
            let page_probe: Rc<Probe> = Rc::default();
            let page_key = GlobalKey::new();
            app_with_page(tester, &page_probe, &page_key);
            let mut app = tester.cell.borrow_mut();

            let second_probe: Rc<Probe> = Rc::default();
            let second_key = GlobalKey::new();
            let context = page_probe.context.get().expect("the page built");
            let second = CupertinoSheetRoute::new(&mut app)
                .builder(&mut app, page(&second_probe, &second_key));
            Navigator::of(&mut app, context, false).push(&mut app, second.clone().into_erased());
            drop(app);
            tester.pump_and_settle();
            let mut app = tester.cell.borrow_mut();

            let third_probe: Rc<Probe> = Rc::default();
            let third_key = GlobalKey::new();
            let context = second_probe.context.get().expect("the second sheet built");
            let third = CupertinoSheetRoute::new(&mut app)
                .builder(&mut app, page(&third_probe, &third_key));
            Navigator::of(&mut app, context, false).push(&mut app, third.clone().into_erased());
            drop(app);
            tester.pump_and_settle();

            let page1_top = top_left(&page_key, tester).dy();
            let page2_top = top_left(&second_key, tester).dy();
            let page3_top = top_left(&third_key, tester).dy();
            // Sheet 2 moves up above Page 1 so that the root route (Page 1) is completely hidden
            // behind Sheet 2.
            assert!(page2_top <= page1_top, "{page2_top} <= {page1_top}");
            // Sheet 3 is stacked on top of Sheet 2, with Sheet 2 peeking out above Sheet 3.
            assert!(page2_top <= page3_top, "{page2_top} <= {page3_top}");
        });
    }

    #[test]
    fn pop_sheet_pops_the_sheet() {
        test_widgets(|tester| {
            let Sheet { sheet, popped, .. } = show_sheet(tester, false);
            tester.pump_and_settle();
            assert!(!popped.is_completed());

            let context = sheet.context.get().expect("the sheet built");
            tester.update(|app| CupertinoSheetRoute::pop_sheet(app, context));
            tester.pump_and_settle();
            assert!(popped.is_completed(), "pop_sheet popped the sheet");
        });
    }

    #[test]
    fn a_drag_past_the_dismiss_threshold_pops_the_sheet() {
        test_widgets(|tester| {
            let Sheet { popped, .. } = show_sheet(tester, false);
            tester.pump_and_settle();

            let finger = drag_down(tester, Offset::new(400.0, 100.0), 80.0, 5);
            lift_finger(tester, &finger, Duration::from_millis(300));
            tester.pump_and_settle();

            assert!(popped.is_completed(), "the drag dismissed the sheet");
        });
    }

    #[test]
    fn a_short_drag_snaps_the_sheet_back() {
        test_widgets(|tester| {
            let Sheet {
                sheet_key, popped, ..
            } = show_sheet(tester, false);
            tester.pump_and_settle();

            let finger = drag_down(tester, Offset::new(400.0, 100.0), 20.0, 4);
            let dragged = top_left(&sheet_key, tester).dy();
            assert!(
                dragged > SHEET_TOP,
                "the drag moved the sheet down, to {dragged}"
            );
            lift_finger(tester, &finger, Duration::from_millis(240));
            tester.pump_and_settle();

            assert!(
                !popped.is_completed(),
                "the short drag did not pop the sheet"
            );
            assert_eq!(
                top_left(&sheet_key, tester).dy(),
                SHEET_TOP,
                "the sheet snapped back open"
            );
        });
    }
    #[test]
    fn a_drag_can_end_while_a_relayout_of_the_sheet_is_pending() {
        test_widgets(|tester| {
            // Regression test for https://github.com/flutter/flutter/issues/194681.
            let Sheet {
                sheet_key, popped, ..
            } = show_sheet(tester, false);
            tester.pump_and_settle();
            let initial_position = top_left(&sheet_key, tester).dy();

            let mut finger = TestPointer::touch(1);
            tester.down(&mut finger, Offset::new(200.0, 400.0));
            move_finger(
                tester,
                &mut finger,
                Offset::new(200.0, 600.0),
                Duration::from_millis(20),
            );
            tester.pump_by(Duration::from_millis(20));
            let mut app = tester.cell.borrow_mut();

            // The sheet measures the drag against the height of the box the drag gesture
            // detector renders. Mark it dirty so the drag ends before the next layout.
            let detector = sheet_key
                .current_context(&app)
                .expect("the sheet content is mounted")
                .find_ancestor_state_of_type::<CupertinoDragGestureDetectorState>(&app)
                .expect("the sheet content is under the drag gesture detector");
            detector
                .context(&app)
                .find_render_object(&app)
                .expect("the detector has a render object")
                .mark_needs_layout(&mut app);
            drop(app);

            lift_finger(tester, &finger, Duration::from_millis(40));
            tester.pump_and_settle();

            assert!(!popped.is_completed(), "the sheet is still up");
            assert_eq!(top_left(&sheet_key, tester).dy(), initial_position);
        });
    }

    /// The whole scenario for a sheet whose content is a scroll view under the route's own
    /// controller: the view fills the sheet, the content is three views tall.
    struct ScrollableSheet {
        /// The scroll view, which is the sheet's own top.
        view_key: GlobalKey,
        /// The scrolled content, which moves as the list scrolls.
        content_key: GlobalKey,
        popped: CompleterFuture<RouteResult>,
    }

    fn show_scrollable_sheet(tester: &mut WidgetTester) -> ScrollableSheet {
        let page_probe: Rc<Probe> = Rc::default();
        let page_key = GlobalKey::new();
        app_with_page(tester, &page_probe, &page_key);
        let mut app = tester.cell.borrow_mut();

        let view_key = GlobalKey::new();
        let content_key = GlobalKey::new();
        let context = page_probe.context.get().expect("the page built");
        let scrollable_builder: ScrollableWidgetBuilder = {
            let view_key: KeyRef = view_key.clone().into();
            let content_key: KeyRef = content_key.clone().into();
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
        let popped = show_cupertino_sheet(
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
        drop(app);
        tester.pump_and_settle();
        ScrollableSheet {
            view_key,
            content_key,
            popped,
        }
    }

    #[test]
    fn a_scrollable_built_through_the_scrollable_builder_scrolls() {
        test_widgets(|tester| {
            let ScrollableSheet {
                view_key,
                content_key,
                popped,
            } = show_scrollable_sheet(tester);
            assert_eq!(top_left(&view_key, tester).dy(), SHEET_TOP);
            assert_eq!(top_left(&content_key, tester).dy(), SHEET_TOP);

            // Dragging up scrolls the list; the sheet itself stays where it is.
            let finger = drag_down(tester, Offset::new(400.0, 300.0), -40.0, 4);
            lift_finger(tester, &finger, Duration::from_millis(240));
            tester.pump_and_settle();

            assert!(
                !popped.is_completed(),
                "the scroll did not dismiss the sheet"
            );
            assert_eq!(
                top_left(&view_key, tester).dy(),
                SHEET_TOP,
                "the sheet stayed open"
            );
            let scrolled = top_left(&content_key, tester).dy();
            assert!(
                scrolled < SHEET_TOP,
                "the list scrolled its content up, to {scrolled}"
            );
        });
    }

    #[test]
    fn a_downward_drag_at_the_top_of_the_scrollable_dismisses_the_sheet() {
        test_widgets(|tester| {
            let ScrollableSheet {
                view_key, popped, ..
            } = show_scrollable_sheet(tester);

            let finger = drag_down(tester, Offset::new(400.0, 100.0), 80.0, 5);
            let dragged = top_left(&view_key, tester).dy();
            assert!(
                dragged > SHEET_TOP,
                "the drag at the top of the list moved the sheet down, to {dragged}"
            );
            lift_finger(tester, &finger, Duration::from_millis(300));
            tester.pump_and_settle();

            assert!(
                popped.is_completed(),
                "the drag handed over to the sheet's dismiss gesture"
            );
        });
    }

    #[test]
    fn a_drag_can_end_while_a_relayout_of_the_sheet_content_is_pending() {
        test_widgets(|tester| {
            // Regression test for https://github.com/flutter/flutter/issues/194681.
            let ScrollableSheet {
                view_key, popped, ..
            } = show_scrollable_sheet(tester);
            let initial_position = top_left(&view_key, tester).dy();

            // Drag the sheet down while its content is scrolled to the top, with a small drag
            // first to win the gesture arena.
            let mut finger = TestPointer::touch(1);
            tester.down(&mut finger, Offset::new(200.0, 300.0));
            move_finger(
                tester,
                &mut finger,
                Offset::new(200.0, 360.0),
                Duration::from_millis(100),
            );
            move_finger(
                tester,
                &mut finger,
                Offset::new(200.0, 460.0),
                Duration::from_millis(200),
            );
            tester.pump_by(Duration::from_millis(200));
            let dragged = top_left(&view_key, tester).dy();
            assert!(
                dragged > initial_position,
                "the drag moved the sheet down, to {dragged}"
            );

            // The sheet measures the drag against the height of its content. Mark the content
            // dirty so the drag ends before the next layout.
            let content = tester.render_object(&view_key);
            tester.update(|app| content.mark_needs_layout(app));

            lift_finger(tester, &finger, Duration::from_millis(300));
            tester.pump_and_settle();

            assert!(!popped.is_completed(), "the sheet is still up");
            assert_eq!(top_left(&view_key, tester).dy(), initial_position);
        });
    }

    #[test]
    fn a_nested_navigation_sheets_inner_route_pops_back_to_the_root() {
        test_widgets(|tester| {
            let page_probe: Rc<Probe> = Rc::default();
            let page_key = GlobalKey::new();
            app_with_page(tester, &page_probe, &page_key);
            let mut app = tester.cell.borrow_mut();

            let sheet_probe: Rc<Probe> = Rc::default();
            let sheet_key = GlobalKey::new();
            let inner_probe: Rc<Probe> = Rc::default();
            let inner_key = GlobalKey::new();
            let context = page_probe.context.get().expect("the page built");
            let sheet_popped = show_cupertino_sheet(
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
            drop(app);
            tester.pump_and_settle();
            let mut app = tester.cell.borrow_mut();
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
            nested.push(&mut app, inner_route.clone().into_erased());
            drop(app);
            tester.pump_and_settle();
            let mut app = tester.cell.borrow_mut();
            assert!(inner_route.is_active(&app), "the inner route was pushed");
            assert!(!sheet_popped.is_completed(), "the sheet is still up");

            let inner_context = inner_probe.context.get().expect("the inner route built");
            let nested = Navigator::of(&mut app, inner_context, false);
            nested.pop(&mut app, None);
            drop(app);
            tester.pump_and_settle();
            let app = tester.cell.borrow();

            assert!(!inner_route.is_active(&app), "the inner route popped");
            assert!(
                !sheet_popped.is_completed(),
                "popping the inner route left the sheet up"
            );
        });
    }
}
