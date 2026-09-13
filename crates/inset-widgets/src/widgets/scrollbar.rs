//! Flutter counterpart: `widgets/scrollbar.dart`.
//!
//! [`ScrollbarPainter`] is an arena `ChangeNotifier` reached through
//! [`ScrollbarPainterRef`], the value a `CustomPaint` holds. [`RawScrollbar`] and
//! [`RawScrollbarState`] are base classes with a bag apiece
//! ([`RawScrollbarData`] / [`RawScrollbarStateData`]) and a leaf trait
//! ([`RawScrollbarLeaf`] / [`RawScrollbarStateLeaf`]), so `CupertinoScrollbar` can extend
//! them the way Dart does.

use std::any::Any;
use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use inset_animation::{
    Animation, AnimationBehavior, AnimationController, AnimationLocalStatusListenersMixin,
    AnimationStatus, AnimationStatusListener, AnyAnimation, CurvedAnimation, Curves,
};
use inset_embedder::{
    Canvas, Color, FillRule, Matrix4, Offset, Paint, PaintStyle, PathBuilder, PointerDeviceKind,
    RRect, Radius, Rect, Size, Stroke, TargetPlatform, TextDirection, clamp_double,
};
use inset_foundation::PRECISION_ERROR_TOLERANCE;
use inset_foundation::{
    App, ChangeNotifier, ChangeNotifierData, Handle, Listenable, Listener, Timer,
};
use inset_gestures::DragDirection;
use inset_gestures::{
    BaseTapData, BaseTapGestureRecognizer, BaseTapLeaf, BaseTapLeafData, DeviceGestureSettings,
    Drag, DragData, DragDownDetails, DragEndDetails, DragGestureRecognizer, DragLeaf, DragLeafData,
    DragStartBehavior, DragStartDetails, DragUpdateDetails, GestureBinding, GestureDisposition,
    GestureRecognizer, GestureRecognizerData, GestureTapDownCallback,
    HorizontalDragGestureRecognizerBase, K_PRESS_TIMEOUT, K_PRIMARY_BUTTON, OneSequenceData,
    OneSequenceLeafData, PointerCancelEvent, PointerDownEvent, PointerEvent, PointerHoverEvent,
    PointerPanZoomStartEvent, PointerScrollEvent, PointerUpEvent, PrimaryPointerData,
    PrimaryPointerGestureRecognizer, PrimaryPointerLeaf, PrimaryPointerLeafData, RecognizerLeaf,
    RecognizerLeafData, TapDownDetails, UNSET_TOUCH_SLOP, Velocity, VelocityEstimate,
    VerticalDragGestureRecognizerBase,
};
use inset_painting::{
    Axis, AxisDirection, EdgeInsets, EdgeInsetsGeometry, OutlinedBorder,
    axis_direction_is_reversed, axis_direction_to_axis, draw_rrect,
};
use inset_rendering::{AnyRenderBox, CustomPainter};

use crate::framework::{
    BuildContext, BuildOwner, GlobalKey, IntoWidget, KeyRef, State, StateData, StatefulWidget,
    WidgetRef, downcast_widget,
};
use crate::widgets::basic::{
    CustomPaint, Listener as PointerListener, MouseRegion, RepaintBoundary,
};
use crate::widgets::gesture_detector::{
    GestureRecognizerFactories, GestureRecognizerFactory, GestureRecognizerFactoryWithHandlers,
    RawGestureDetector,
};
use crate::widgets::media_query::MediaQuery;
use crate::widgets::notification_listener::NotificationListener;
use crate::widgets::primary_scroll_controller::PrimaryScrollController;
use crate::widgets::scroll_activity::ScrollHoldController;
use crate::widgets::scroll_configuration::ScrollConfiguration;
use crate::widgets::scroll_controller::AnyScrollController;
use crate::widgets::scroll_metrics::ScrollMetrics;
use crate::widgets::scroll_notification::{
    OverscrollNotification, ScrollEndNotification, ScrollNotification, ScrollNotificationPredicate,
    ScrollUpdateNotification, default_scroll_notification_predicate,
};
use crate::widgets::scroll_position::ScrollMetricsNotification;
use crate::widgets::scrollable::Scrollable;
use crate::widgets::scrollable_helpers::{ScrollAction, ScrollIncrementType, ScrollIntent};
use crate::widgets::ticker_provider::{TickerProviderStateMixin, TickerProviderStateMixinData};
use inset_scheduler::{
    FrameCallback, SchedulerBinding, Ticker, TickerCallback, TickerProviderObject,
};

const K_MIN_THUMB_EXTENT: f64 = 18.0;
const K_MIN_INTERACTIVE_SIZE: f64 = 48.0;
const K_SCROLLBAR_THICKNESS: f64 = 6.0;
const K_SCROLLBAR_FADE_DURATION: Duration = Duration::from_millis(300);
const K_SCROLLBAR_TIME_TO_FADE: Duration = Duration::from_millis(600);

/// The default thumb colour of a [`RawScrollbar`].
const K_DEFAULT_THUMB_COLOR: Color = Color::from_argb(0x66, 0xBC, 0xBC, 0xBC);
/// The default track colour of a [`RawScrollbar`].
const K_DEFAULT_TRACK_COLOR: Color = Color::from_argb(0x08, 0x00, 0x00, 0x00);
/// The default track border colour of a [`RawScrollbar`].
const K_DEFAULT_TRACK_BORDER_COLOR: Color = Color::from_argb(0x1a, 0x00, 0x00, 0x00);
/// The colour a hidden track and track border paint with.
const K_TRANSPARENT: Color = Color::from_argb(0, 0, 0, 0);

/// An orientation along either the horizontal or vertical [`Axis`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScrollbarOrientation {
    /// Place towards the left of the screen.
    Left,

    /// Place towards the right of the screen.
    Right,

    /// Place on top of the screen.
    Top,

    /// Place on the bottom of the screen.
    Bottom,
}

impl ScrollbarOrientation {
    fn is_vertical(self) -> bool {
        matches!(
            self,
            ScrollbarOrientation::Left | ScrollbarOrientation::Right
        )
    }
}

/// Paints a scrollbar's track and thumb.
///
/// The size of the scrollbar along its scroll direction is typically
/// proportional to the percentage of content completely visible on screen,
/// as long as its size isn't less than [`min_length`](Self::min_length) and it isn't
/// overscrolling.
///
/// Unlike painters that only repaint when `should_repaint` returns true (which requires the
/// painter to be rebuilt), this painter has the added optimization of repainting and not
/// rebuilding when:
///
///  * the scroll position changes; and
///  * when the scrollbar fades away.
///
/// Calling [`update`](Self::update) with the new [`ScrollMetrics`] will repaint the new
/// scrollbar position.
///
/// Updating the value on the provided [`fadeout_opacity_animation`](Self::fadeout_opacity_animation)
/// will repaint with the new opacity.
///
/// You must call [`dispose`](Self::dispose) on this painter when it is no longer used.
///
/// The painter is an arena object because it is a `ChangeNotifier` with identity; the value
/// a `CustomPaint` holds is a [`ScrollbarPainterRef`] over its handle.
pub struct ScrollbarPainter {
    change_notifier: ChangeNotifierData,
    color: Color,
    track_color: Color,
    track_border_color: Color,
    track_radius: Option<Radius>,
    text_direction: Option<TextDirection>,
    thickness: f64,
    fadeout_opacity_animation: AnyAnimation<f64>,
    main_axis_margin: f64,
    cross_axis_margin: f64,
    radius: Option<Radius>,
    shape: Option<Box<dyn OutlinedBorder>>,
    padding: EdgeInsetsGeometry,
    min_length: f64,
    min_overscroll_length: f64,
    scrollbar_orientation: Option<ScrollbarOrientation>,
    ignore_pointer: bool,

    // - Scrollbar Details
    track_rect: Option<Rect>,
    resolved_padding: Option<EdgeInsets>,
    thumb_rect: Option<Rect>,
    // The current scroll position + the leading thumb main axis offset.
    thumb_offset: f64,
    // The fraction visible in relation to the traversable length of the track.
    thumb_extent: f64,

    // - Scrollable Details
    last_metrics: Option<Rc<dyn ScrollMetrics>>,
    last_axis_direction: Option<AxisDirection>,
}

impl ScrollbarPainter {
    /// Creates a scrollbar painter with Dart's constructor defaults; the remaining
    /// constructor arguments are the setters below, which a fresh painter has no listener to
    /// notify.
    pub fn new(
        app: &mut App,
        color: Color,
        fadeout_opacity_animation: AnyAnimation<f64>,
    ) -> Handle<ScrollbarPainter> {
        let this = app.create(ScrollbarPainter {
            change_notifier: ChangeNotifierData::new(),
            color,
            track_color: K_TRANSPARENT,
            track_border_color: K_TRANSPARENT,
            track_radius: None,
            text_direction: None,
            thickness: K_SCROLLBAR_THICKNESS,
            fadeout_opacity_animation,
            main_axis_margin: 0.0,
            cross_axis_margin: 0.0,
            radius: None,
            shape: None,
            padding: EdgeInsetsGeometry::ZERO,
            min_length: K_MIN_THUMB_EXTENT,
            min_overscroll_length: K_MIN_THUMB_EXTENT,
            scrollbar_orientation: None,
            ignore_pointer: false,
            track_rect: None,
            resolved_padding: Some(EdgeInsets::ZERO),
            thumb_rect: None,
            thumb_offset: 0.0,
            thumb_extent: 0.0,
            last_metrics: None,
            last_axis_direction: None,
        });
        fadeout_opacity_animation.add_listener(
            app,
            Listener::handle_method(this, Handle::<ScrollbarPainter>::notify_listeners),
        );
        this
    }

    /// [`Color`] of the thumb.
    pub fn color(self: Handle<Self>, app: &App) -> Color {
        app.get(self).color
    }

    /// Sets [`color`](Self::color).
    pub fn set_color(self: Handle<Self>, app: &mut App, value: Color) {
        if self.color(app) == value {
            return;
        }
        app.get_mut(self).color = value;
        self.notify_listeners(app);
    }

    /// [`Color`] of the track.
    pub fn track_color(self: Handle<Self>, app: &App) -> Color {
        app.get(self).track_color
    }

    /// Sets [`track_color`](Self::track_color).
    pub fn set_track_color(self: Handle<Self>, app: &mut App, value: Color) {
        if self.track_color(app) == value {
            return;
        }
        app.get_mut(self).track_color = value;
        self.notify_listeners(app);
    }

    /// [`Color`] of the track border.
    pub fn track_border_color(self: Handle<Self>, app: &App) -> Color {
        app.get(self).track_border_color
    }

    /// Sets [`track_border_color`](Self::track_border_color).
    pub fn set_track_border_color(self: Handle<Self>, app: &mut App, value: Color) {
        if self.track_border_color(app) == value {
            return;
        }
        app.get_mut(self).track_border_color = value;
        self.notify_listeners(app);
    }

    /// [`Radius`] of corners of the scrollbar's track.
    ///
    /// The track will be rectangular if this is `None`.
    pub fn track_radius(self: Handle<Self>, app: &App) -> Option<Radius> {
        app.get(self).track_radius
    }

    /// Sets [`track_radius`](Self::track_radius).
    pub fn set_track_radius(self: Handle<Self>, app: &mut App, value: Option<Radius>) {
        if self.track_radius(app) == value {
            return;
        }
        app.get_mut(self).track_radius = value;
        self.notify_listeners(app);
    }

    /// [`TextDirection`] of the `BuildContext` which dictates the side of the
    /// screen the scrollbar appears in (the trailing side). Must be set prior to
    /// calling [`paint`](Self::paint).
    pub fn text_direction(self: Handle<Self>, app: &App) -> Option<TextDirection> {
        app.get(self).text_direction
    }

    /// Sets [`text_direction`](Self::text_direction).
    pub fn set_text_direction(self: Handle<Self>, app: &mut App, value: Option<TextDirection>) {
        debug_assert!(value.is_some());
        if self.text_direction(app) == value {
            return;
        }
        let painter = app.get_mut(self);
        painter.text_direction = value;
        painter.resolved_padding = Some(painter.padding.resolve(value));
        self.notify_listeners(app);
    }

    /// Thickness of the scrollbar in its cross-axis in logical pixels.
    pub fn thickness(self: Handle<Self>, app: &App) -> f64 {
        app.get(self).thickness
    }

    /// Sets [`thickness`](Self::thickness).
    pub fn set_thickness(self: Handle<Self>, app: &mut App, value: f64) {
        if self.thickness(app) == value {
            return;
        }
        app.get_mut(self).thickness = value;
        self.notify_listeners(app);
    }

    /// An opacity `Animation` that dictates the opacity of the thumb.
    ///
    /// Changes in the value of this `Listenable` automatically trigger repaints.
    pub fn fadeout_opacity_animation(self: Handle<Self>, app: &App) -> AnyAnimation<f64> {
        app.get(self).fadeout_opacity_animation
    }

    /// Distance from the scrollbar thumb's start and end to the edge of the
    /// viewport in logical pixels. It affects the amount of available paint area.
    ///
    /// The scrollbar track consumes this space. Defaults to 0.
    pub fn main_axis_margin(self: Handle<Self>, app: &App) -> f64 {
        app.get(self).main_axis_margin
    }

    /// Sets [`main_axis_margin`](Self::main_axis_margin).
    pub fn set_main_axis_margin(self: Handle<Self>, app: &mut App, value: f64) {
        if self.main_axis_margin(app) == value {
            return;
        }
        app.get_mut(self).main_axis_margin = value;
        self.notify_listeners(app);
    }

    /// Distance from the scrollbar thumb to the nearest cross axis edge in logical
    /// pixels.
    ///
    /// The scrollbar track consumes this space. Defaults to zero.
    pub fn cross_axis_margin(self: Handle<Self>, app: &App) -> f64 {
        app.get(self).cross_axis_margin
    }

    /// Sets [`cross_axis_margin`](Self::cross_axis_margin).
    pub fn set_cross_axis_margin(self: Handle<Self>, app: &mut App, value: f64) {
        if self.cross_axis_margin(app) == value {
            return;
        }
        app.get_mut(self).cross_axis_margin = value;
        self.notify_listeners(app);
    }

    /// [`Radius`] of corners if the scrollbar should have rounded corners.
    ///
    /// The scrollbar will be rectangular if this is `None`.
    pub fn radius(self: Handle<Self>, app: &App) -> Option<Radius> {
        app.get(self).radius
    }

    /// Sets [`radius`](Self::radius).
    pub fn set_radius(self: Handle<Self>, app: &mut App, value: Option<Radius>) {
        debug_assert!(app.get(self).shape.is_none() || value.is_none());
        if self.radius(app) == value {
            return;
        }
        app.get_mut(self).radius = value;
        self.notify_listeners(app);
    }

    /// The [`OutlinedBorder`] of the scrollbar's thumb.
    ///
    /// Only one of [`radius`](Self::radius) and this may be specified. For a rounded
    /// rectangle, it is simplest to just specify a radius. By default, the scrollbar thumb's
    /// shape is a simple rectangle.
    ///
    /// If a shape is specified, the thumb will take the shape of the passed
    /// [`OutlinedBorder`] and fill itself with [`color`](Self::color).
    pub fn shape(self: Handle<Self>, app: &App) -> Option<Box<dyn OutlinedBorder>> {
        app.get(self)
            .shape
            .as_ref()
            .map(|shape| shape.clone_outlined())
    }

    /// Sets [`shape`](Self::shape).
    pub fn set_shape(self: Handle<Self>, app: &mut App, value: Option<Box<dyn OutlinedBorder>>) {
        debug_assert!(app.get(self).radius.is_none() || value.is_none());
        if shapes_equal(app.get(self).shape.as_deref(), value.as_deref()) {
            return;
        }
        app.get_mut(self).shape = value;
        self.notify_listeners(app);
    }

    /// The amount of space by which to inset the scrollbar's start and end, as
    /// well as its side to the nearest edge, in logical pixels.
    ///
    /// This is typically set to the current `MediaQueryData.padding` to avoid partial
    /// obstructions such as display notches. Defaults to `EdgeInsets::ZERO`.
    pub fn padding(self: Handle<Self>, app: &App) -> EdgeInsetsGeometry {
        app.get(self).padding
    }

    /// Sets [`padding`](Self::padding).
    pub fn set_padding(self: Handle<Self>, app: &mut App, value: EdgeInsetsGeometry) {
        if self.padding(app) == value {
            return;
        }
        let painter = app.get_mut(self);
        painter.padding = value;
        let text_direction = painter.text_direction;
        painter.resolved_padding = Some(painter.padding.resolve(text_direction));
        self.notify_listeners(app);
    }

    /// The preferred smallest size the scrollbar thumb can shrink to when the
    /// total scrollable extent is large, the current visible viewport is small,
    /// and the viewport is not overscrolled.
    ///
    /// The size of the scrollbar may shrink to a smaller size than this to fit in
    /// the available paint area. Defaults to 18.0.
    pub fn min_length(self: Handle<Self>, app: &App) -> f64 {
        app.get(self).min_length
    }

    /// Sets [`min_length`](Self::min_length).
    pub fn set_min_length(self: Handle<Self>, app: &mut App, value: f64) {
        if self.min_length(app) == value {
            return;
        }
        app.get_mut(self).min_length = value;
        self.notify_listeners(app);
    }

    /// The preferred smallest size the scrollbar thumb can shrink to when the
    /// viewport is overscrolled.
    ///
    /// The value is less than or equal to [`min_length`](Self::min_length) and greater than
    /// or equal to 0.
    pub fn min_overscroll_length(self: Handle<Self>, app: &App) -> f64 {
        app.get(self).min_overscroll_length
    }

    /// Sets [`min_overscroll_length`](Self::min_overscroll_length).
    pub fn set_min_overscroll_length(self: Handle<Self>, app: &mut App, value: f64) {
        if self.min_overscroll_length(app) == value {
            return;
        }
        app.get_mut(self).min_overscroll_length = value;
        self.notify_listeners(app);
    }

    /// Dictates the orientation of the scrollbar.
    ///
    /// [`ScrollbarOrientation::Top`] and [`ScrollbarOrientation::Bottom`] can only be used
    /// with a vertical scroll. [`ScrollbarOrientation::Left`] and
    /// [`ScrollbarOrientation::Right`] can only be used with a horizontal scroll.
    ///
    /// For a vertical scroll the orientation defaults to [`ScrollbarOrientation::Right`] for
    /// [`TextDirection::Ltr`] and [`ScrollbarOrientation::Left`] for [`TextDirection::Rtl`].
    /// For a horizontal scroll it defaults to [`ScrollbarOrientation::Bottom`].
    pub fn scrollbar_orientation(self: Handle<Self>, app: &App) -> Option<ScrollbarOrientation> {
        app.get(self).scrollbar_orientation
    }

    /// Sets [`scrollbar_orientation`](Self::scrollbar_orientation).
    pub fn set_scrollbar_orientation(
        self: Handle<Self>,
        app: &mut App,
        value: Option<ScrollbarOrientation>,
    ) {
        if self.scrollbar_orientation(app) == value {
            return;
        }
        app.get_mut(self).scrollbar_orientation = value;
        self.notify_listeners(app);
    }

    /// Whether the painter will be ignored during hit testing.
    pub fn ignore_pointer(self: Handle<Self>, app: &App) -> bool {
        app.get(self).ignore_pointer
    }

    /// Sets [`ignore_pointer`](Self::ignore_pointer).
    pub fn set_ignore_pointer(self: Handle<Self>, app: &mut App, value: bool) {
        if self.ignore_pointer(app) == value {
            return;
        }
        app.get_mut(self).ignore_pointer = value;
        self.notify_listeners(app);
    }

    // - Scrollbar Details

    /// The full painted length of the track.
    fn track_extent(self: Handle<Self>, app: &App) -> f64 {
        self.metrics(app).viewport_dimension() - self.total_track_main_axis_offsets(app)
    }

    /// The full length of the track that the thumb can travel.
    fn traversable_track_extent(self: Handle<Self>, app: &App) -> f64 {
        self.track_extent(app) - (2.0 * self.main_axis_margin(app))
    }

    /// The track is offset by only padding.
    fn total_track_main_axis_offsets(self: Handle<Self>, app: &App) -> f64 {
        let padding = self.resolved_padding(app);
        if self.is_vertical(app) {
            padding.vertical()
        } else {
            padding.horizontal()
        }
    }

    fn leading_track_main_axis_offset(self: Handle<Self>, app: &App) -> f64 {
        let padding = self.resolved_padding(app);
        match self.resolved_orientation(app) {
            ScrollbarOrientation::Left | ScrollbarOrientation::Right => padding.top,
            ScrollbarOrientation::Top | ScrollbarOrientation::Bottom => padding.left,
        }
    }

    /// The thumb is offset by padding and margins.
    fn leading_thumb_main_axis_offset(self: Handle<Self>, app: &App) -> f64 {
        self.leading_track_main_axis_offset(app) + self.main_axis_margin(app)
    }

    fn resolved_padding(self: Handle<Self>, app: &App) -> EdgeInsets {
        app.get(self)
            .resolved_padding
            .expect("the padding resolves when it or the text direction is set")
    }

    fn metrics(self: Handle<Self>, app: &App) -> Rc<dyn ScrollMetrics> {
        app.get(self)
            .last_metrics
            .clone()
            .expect("update() supplies the metrics before the painter reads them")
    }

    fn set_thumb_extent(self: Handle<Self>, app: &mut App) {
        // Thumb extent reflects fraction of content visible, as long as this
        // isn't less than the absolute minimum size.
        let metrics = self.metrics(app);
        let total_track_main_axis_offsets = self.total_track_main_axis_offsets(app);
        let fraction_visible = clamp_double(
            (metrics.extent_inside() - total_track_main_axis_offsets)
                / (self.total_content_extent(app) - total_track_main_axis_offsets),
            0.0,
            1.0,
        );

        let traversable_track_extent = self.traversable_track_extent(app);
        let thumb_extent = traversable_track_extent
            .min(self.min_overscroll_length(app))
            .max(traversable_track_extent * fraction_visible);

        let fraction_overscrolled = 1.0 - metrics.extent_inside() / metrics.viewport_dimension();
        let safe_min_length = self.min_length(app).min(traversable_track_extent);
        let new_min_length = if self.before_extent(app) > 0.0 && self.after_extent(app) > 0.0 {
            // Thumb extent is no smaller than `min_length` if scrolling normally.
            safe_min_length
        } else {
            // The user is overscrolling. The thumb extent can be less than `min_length` but
            // no smaller than `min_overscroll_length`. The percentage of the content that is
            // still in the viewport determines the size of the thumb: iOS behavior appears to
            // have the thumb reach its minimum size with ~20% of overscroll, so the
            // percentage of `min_length` maps from [0.8, 1.0] to [0.0, 1.0].
            safe_min_length * (1.0 - clamp_double(fraction_overscrolled, 0.0, 0.2) / 0.2)
        };

        // The thumb extent should be no greater than the track size, otherwise the scrollbar
        // may scroll towards the wrong direction.
        app.get_mut(self).thumb_extent =
            clamp_double(thumb_extent, new_min_length, traversable_track_extent);
    }

    // - Scrollable Details

    fn last_metrics_are_scrollable(self: Handle<Self>, app: &App) -> bool {
        let metrics = self.metrics(app);
        metrics.min_scroll_extent() != metrics.max_scroll_extent()
    }

    fn is_vertical(self: Handle<Self>, app: &App) -> bool {
        matches!(
            app.get(self).last_axis_direction,
            Some(AxisDirection::Down) | Some(AxisDirection::Up)
        )
    }

    fn is_reversed(self: Handle<Self>, app: &App) -> bool {
        matches!(
            app.get(self).last_axis_direction,
            Some(AxisDirection::Up) | Some(AxisDirection::Left)
        )
    }

    /// The amount of scroll distance before the current position.
    fn before_extent(self: Handle<Self>, app: &App) -> f64 {
        let metrics = self.metrics(app);
        if self.is_reversed(app) {
            metrics.extent_after()
        } else {
            metrics.extent_before()
        }
    }

    /// The amount of scroll distance after the current position.
    fn after_extent(self: Handle<Self>, app: &App) -> f64 {
        let metrics = self.metrics(app);
        if self.is_reversed(app) {
            metrics.extent_before()
        } else {
            metrics.extent_after()
        }
    }

    /// The total size of the scrollable content.
    fn total_content_extent(self: Handle<Self>, app: &App) -> f64 {
        let metrics = self.metrics(app);
        metrics.max_scroll_extent() - metrics.min_scroll_extent() + metrics.viewport_dimension()
    }

    fn resolved_orientation(self: Handle<Self>, app: &App) -> ScrollbarOrientation {
        match self.scrollbar_orientation(app) {
            Some(orientation) => orientation,
            None => {
                if self.is_vertical(app) {
                    if self.text_direction(app) == Some(TextDirection::Ltr) {
                        ScrollbarOrientation::Right
                    } else {
                        ScrollbarOrientation::Left
                    }
                } else {
                    ScrollbarOrientation::Bottom
                }
            }
        }
    }

    fn debug_assert_is_valid_orientation(
        self: Handle<Self>,
        app: &App,
        orientation: ScrollbarOrientation,
    ) {
        debug_assert!(
            self.is_vertical(app) == orientation.is_vertical(),
            "The given ScrollbarOrientation: {orientation:?} is incompatible with the current \
             AxisDirection: {:?}.",
            app.get(self).last_axis_direction
        );
    }

    // - Updating

    /// Update with new [`ScrollMetrics`]. If the metrics change, the scrollbar will
    /// show and redraw itself based on these new metrics.
    ///
    /// The scrollbar will remain on screen.
    pub fn update(
        self: Handle<Self>,
        app: &mut App,
        metrics: Rc<dyn ScrollMetrics>,
        axis_direction: AxisDirection,
    ) {
        if let Some(last) = app.get(self).last_metrics.clone()
            && last.extent_before() == metrics.extent_before()
            && last.extent_inside() == metrics.extent_inside()
            && last.extent_after() == metrics.extent_after()
            && app.get(self).last_axis_direction == Some(axis_direction)
        {
            return;
        }

        let old_metrics = app.get(self).last_metrics.clone();
        let painter = app.get_mut(self);
        painter.last_metrics = Some(metrics.clone());
        painter.last_axis_direction = Some(axis_direction);

        if !need_paint(old_metrics.as_deref()) && !need_paint(Some(&*metrics)) {
            return;
        }
        self.notify_listeners(app);
    }

    /// Update and redraw with new scrollbar thickness and radius.
    pub fn update_thickness(
        self: Handle<Self>,
        app: &mut App,
        next_thickness: f64,
        next_radius: Radius,
    ) {
        self.set_thickness(app, next_thickness);
        self.set_radius(app, Some(next_radius));
    }

    // - Painting

    fn paint_thumb(self: Handle<Self>, app: &App) -> Paint {
        let color = self.color(app);
        let opacity = self.fadeout_opacity_animation(app).value(app);
        Paint {
            color: color
                .with_values(Some(color.a * opacity), None, None, None, None)
                .into(),
            ..Paint::default()
        }
    }

    fn paint_track(self: Handle<Self>, app: &App, is_border: bool) -> Paint {
        let opacity = self.fadeout_opacity_animation(app).value(app);
        if is_border {
            let color = self.track_border_color(app);
            return Paint {
                color: color
                    .with_values(Some(color.a * opacity), None, None, None, None)
                    .into(),
                style: PaintStyle::Stroke(Stroke::new(1.0)),
                ..Paint::default()
            };
        }
        let color = self.track_color(app);
        Paint {
            color: color
                .with_values(Some(color.a * opacity), None, None, None, None)
                .into(),
            ..Paint::default()
        }
    }

    fn paint_scrollbar(self: Handle<Self>, app: &mut App, canvas: &mut Canvas, size: Size) {
        debug_assert!(
            self.text_direction(app).is_some(),
            "A TextDirection must be provided before a Scrollbar can be painted."
        );
        let orientation = self.resolved_orientation(app);
        self.debug_assert_is_valid_orientation(app, orientation);

        let thickness = self.thickness(app);
        let cross_axis_margin = self.cross_axis_margin(app);
        let padding = self.resolved_padding(app);
        let track_extent = self.track_extent(app);
        let thumb_extent = app.get(self).thumb_extent;
        let thumb_offset = app.get(self).thumb_offset;
        let leading_track_main_axis_offset = self.leading_track_main_axis_offset(app);

        let (x, y, thumb_size, track_size, track_offset, border_start, border_end) =
            match orientation {
                ScrollbarOrientation::Left => {
                    let thumb_size = Size::new(thickness, thumb_extent);
                    let track_size = Size::new(thickness + 2.0 * cross_axis_margin, track_extent);
                    let x = cross_axis_margin + padding.left;
                    let y = thumb_offset;
                    let track_offset =
                        Offset::new(x - cross_axis_margin, leading_track_main_axis_offset);
                    let border_start = track_offset + Offset::new(track_size.width(), 0.0);
                    let border_end = Offset::new(
                        track_offset.dx() + track_size.width(),
                        track_offset.dy() + track_extent,
                    );
                    (
                        x,
                        y,
                        thumb_size,
                        track_size,
                        track_offset,
                        border_start,
                        border_end,
                    )
                }
                ScrollbarOrientation::Right => {
                    let thumb_size = Size::new(thickness, thumb_extent);
                    let track_size = Size::new(thickness + 2.0 * cross_axis_margin, track_extent);
                    let x = size.width() - thickness - cross_axis_margin - padding.right;
                    let y = thumb_offset;
                    let track_offset =
                        Offset::new(x - cross_axis_margin, leading_track_main_axis_offset);
                    let border_start = track_offset;
                    let border_end =
                        Offset::new(track_offset.dx(), track_offset.dy() + track_extent);
                    (
                        x,
                        y,
                        thumb_size,
                        track_size,
                        track_offset,
                        border_start,
                        border_end,
                    )
                }
                ScrollbarOrientation::Top => {
                    let thumb_size = Size::new(thumb_extent, thickness);
                    let track_size = Size::new(track_extent, thickness + 2.0 * cross_axis_margin);
                    let x = thumb_offset;
                    let y = cross_axis_margin + padding.top;
                    let track_offset =
                        Offset::new(leading_track_main_axis_offset, y - cross_axis_margin);
                    let border_start = track_offset + Offset::new(0.0, track_size.height());
                    let border_end = Offset::new(
                        track_offset.dx() + track_extent,
                        track_offset.dy() + track_size.height(),
                    );
                    (
                        x,
                        y,
                        thumb_size,
                        track_size,
                        track_offset,
                        border_start,
                        border_end,
                    )
                }
                ScrollbarOrientation::Bottom => {
                    let thumb_size = Size::new(thumb_extent, thickness);
                    let track_size = Size::new(track_extent, thickness + 2.0 * cross_axis_margin);
                    let x = thumb_offset;
                    let y = size.height() - thickness - cross_axis_margin - padding.bottom;
                    let track_offset =
                        Offset::new(leading_track_main_axis_offset, y - cross_axis_margin);
                    let border_start = track_offset;
                    let border_end =
                        Offset::new(track_offset.dx() + track_extent, track_offset.dy());
                    (
                        x,
                        y,
                        thumb_size,
                        track_size,
                        track_offset,
                        border_start,
                        border_end,
                    )
                }
            };

        // Whether we paint or not, calculating these rects allows us to hit test
        // when the scrollbar is transparent.
        let track_rect = track_offset & track_size;
        let thumb_rect = Offset::new(x, y) & thumb_size;
        let painter = app.get_mut(self);
        painter.track_rect = Some(track_rect);
        painter.thumb_rect = Some(thumb_rect);

        // Paint if the opacity dictates visibility.
        if self.fadeout_opacity_animation(app).value(app) == 0.0 {
            return;
        }
        // Track
        match self.track_radius(app) {
            None => canvas.draw_rect(track_rect, &self.paint_track(app, false)),
            Some(track_radius) => draw_rrect(
                canvas,
                RRect::from_rect_and_radius(track_rect, track_radius),
                &self.paint_track(app, false),
            ),
        }
        // Track border
        let mut border = PathBuilder::new();
        border.move_to(border_start);
        border.line_to(border_end);
        canvas.draw_path(
            &border.build(),
            FillRule::NonZero,
            &self.paint_track(app, true),
        );

        if let Some(radius) = self.radius(app) {
            // Rounded rect thumb
            draw_rrect(
                canvas,
                RRect::from_rect_and_radius(thumb_rect, radius),
                &self.paint_thumb(app),
            );
            return;
        }
        let Some(shape) = self.shape(app) else {
            // Square thumb
            canvas.draw_rect(thumb_rect, &self.paint_thumb(app));
            return;
        };
        // Custom-shaped thumb
        let text_direction = self.text_direction(app);
        if shape.prefer_paint_interior() {
            shape.paint_interior(canvas, thumb_rect, &self.paint_thumb(app), text_direction);
        } else {
            let outer_path = shape.get_outer_path(thumb_rect, text_direction);
            canvas.draw_path(&outer_path, FillRule::NonZero, &self.paint_thumb(app));
        }
        shape.paint(canvas, thumb_rect, text_direction);
    }

    /// Paints the scrollbar; see [`CustomPainter::paint`].
    pub fn paint(self: Handle<Self>, app: &mut App, canvas: &mut Canvas, size: Size) {
        if app.get(self).last_axis_direction.is_none()
            || !need_paint(app.get(self).last_metrics.as_deref())
        {
            return;
        }

        // Skip painting if there's not enough space.
        if self.traversable_track_extent(app) <= 0.0 {
            return;
        }
        // Do not paint a scrollbar if the scroll view is infinitely long.
        if self.metrics(app).max_scroll_extent().is_infinite() {
            return;
        }

        self.set_thumb_extent(app);
        let thumb_extent = app.get(self).thumb_extent;
        let metrics = self.metrics(app);
        let thumb_position_offset = self.get_scroll_to_track(app, &*metrics, thumb_extent);
        app.get_mut(self).thumb_offset =
            thumb_position_offset + self.leading_thumb_main_axis_offset(app);

        self.paint_scrollbar(app, canvas, size);
    }

    // - Scroll Position Conversion

    /// Convert between a thumb track position and the corresponding scroll
    /// position.
    ///
    /// The `thumb_offset_local` argument is a position in the thumb track.
    pub fn get_track_to_scroll(self: Handle<Self>, app: &App, thumb_offset_local: f64) -> f64 {
        let metrics = self.metrics(app);
        let scrollable_extent = metrics.max_scroll_extent() - metrics.min_scroll_extent();
        let thumb_movable_extent = self.traversable_track_extent(app) - app.get(self).thumb_extent;

        scrollable_extent * thumb_offset_local / thumb_movable_extent
    }

    /// The thumb's corresponding scroll offset in the track.
    pub fn get_thumb_scroll_offset(self: Handle<Self>, app: &App) -> f64 {
        let metrics = self.metrics(app);
        debug_assert!(
            metrics.max_scroll_extent().is_finite() && metrics.min_scroll_extent().is_finite()
        );
        let scrollable_extent = metrics.max_scroll_extent() - metrics.min_scroll_extent();
        let max_fraction = metrics.max_scroll_extent() / scrollable_extent;
        let min_fraction = metrics.min_scroll_extent() / scrollable_extent;

        let fraction_past = if scrollable_extent > 0.0 {
            clamp_double(
                metrics.pixels() / scrollable_extent,
                min_fraction,
                max_fraction,
            )
        } else {
            0.0
        };

        fraction_past * (self.traversable_track_extent(app) - app.get(self).thumb_extent)
    }

    /// Converts between a scroll position and the corresponding position in the
    /// thumb track.
    fn get_scroll_to_track(
        self: Handle<Self>,
        app: &App,
        metrics: &dyn ScrollMetrics,
        thumb_extent: f64,
    ) -> f64 {
        let scrollable_extent = metrics.max_scroll_extent() - metrics.min_scroll_extent();

        let fraction_past = if scrollable_extent > 0.0 {
            clamp_double(
                (metrics.pixels() - metrics.min_scroll_extent()) / scrollable_extent,
                0.0,
                1.0,
            )
        } else {
            0.0
        };

        let fraction_past = if self.is_reversed(app) {
            1.0 - fraction_past
        } else {
            fraction_past
        };
        fraction_past * (self.traversable_track_extent(app) - thumb_extent)
    }

    /// Dart's `ScrollbarPainter.shouldRepaint`: whether any property differs from
    /// `old_delegate`'s.
    pub fn should_repaint(self: Handle<Self>, app: &App, old_delegate: Handle<Self>) -> bool {
        let (new, old) = (app.get(self), app.get(old_delegate));
        let same_shape = match (&new.shape, &old.shape) {
            (None, None) => true,
            (Some(new), Some(old)) => new.eq_shape(&**old),
            _ => false,
        };
        new.color != old.color
            || new.track_color != old.track_color
            || new.track_border_color != old.track_border_color
            || new.text_direction != old.text_direction
            || new.thickness != old.thickness
            || new.fadeout_opacity_animation != old.fadeout_opacity_animation
            || new.main_axis_margin != old.main_axis_margin
            || new.cross_axis_margin != old.cross_axis_margin
            || new.radius != old.radius
            || new.track_radius != old.track_radius
            || !same_shape
            || new.padding != old.padding
            || new.min_length != old.min_length
            || new.min_overscroll_length != old.min_overscroll_length
            || new.scrollbar_orientation != old.scrollbar_orientation
            || new.ignore_pointer != old.ignore_pointer
    }

    // - Hit Testing

    /// Dart's `ScrollbarPainter.hitTest`, which the [`CustomPainter`] impl of
    /// [`ScrollbarPainterRef`] forwards to.
    pub fn hit_test(self: Handle<Self>, app: &App, position: Option<Offset>) -> Option<bool> {
        // There is nothing painted to hit.
        app.get(self).thumb_rect?;
        let track_rect = app.get(self).track_rect.expect("a thumb implies a track");

        // Interaction disabled, the thumb is not able to be hit when transparent, or the
        // metrics are not scrollable.
        if self.ignore_pointer(app)
            || self.fadeout_opacity_animation(app).value(app) == 0.0
            || !self.last_metrics_are_scrollable(app)
        {
            return Some(false);
        }

        Some(track_rect.contains(position.expect("a hit test has a position")))
    }

    /// Same as [`hit_test`](Self::hit_test), but includes some padding when the pointer event
    /// is caused by [`PointerDeviceKind::Touch`] to make sure that the region isn't too small
    /// to be interacted with by the user.
    ///
    /// The hit test area for hovering with [`PointerDeviceKind::Mouse`] over the scrollbar
    /// also uses this extra padding. When `for_hover` is true, the larger hit test area will
    /// be used.
    pub fn hit_test_interactive(
        self: Handle<Self>,
        app: &App,
        position: Offset,
        kind: PointerDeviceKind,
        for_hover: bool,
    ) -> bool {
        // We have not computed the scrollbar position yet.
        let Some(interactive_rect) = app.get(self).track_rect else {
            return false;
        };
        if self.ignore_pointer(app) {
            return false;
        }
        if !self.last_metrics_are_scrollable(app) {
            return false;
        }

        let thumb_rect = app.get(self).thumb_rect.expect("a track implies a thumb");
        let padded_rect = interactive_rect.expand_to_include(Rect::from_circle(
            thumb_rect.center(),
            K_MIN_INTERACTIVE_SIZE / 2.0,
        ));

        // The scrollbar is not able to be hit when transparent — except when hovering with a
        // mouse. This should bring the scrollbar into view so the mouse can interact with it.
        if self.fadeout_opacity_animation(app).value(app) == 0.0 {
            if for_hover && kind == PointerDeviceKind::Mouse {
                return padded_rect.contains(position);
            }
            return false;
        }

        match kind {
            PointerDeviceKind::Touch | PointerDeviceKind::Trackpad => {
                padded_rect.contains(position)
            }
            PointerDeviceKind::Mouse
            | PointerDeviceKind::Stylus
            | PointerDeviceKind::InvertedStylus
            | PointerDeviceKind::Unknown => interactive_rect.contains(position),
        }
    }

    /// Same as [`hit_test_interactive`](Self::hit_test_interactive), but excludes the track
    /// portion of the scrollbar. Used to evaluate interactions with only the scrollbar thumb.
    pub fn hit_test_only_thumb_interactive(
        self: Handle<Self>,
        app: &App,
        position: Offset,
        kind: PointerDeviceKind,
    ) -> bool {
        let Some(thumb_rect) = app.get(self).thumb_rect else {
            return false;
        };
        if self.ignore_pointer(app) {
            return false;
        }
        // The thumb is not able to be hit when transparent.
        if self.fadeout_opacity_animation(app).value(app) == 0.0 {
            return false;
        }
        if !self.last_metrics_are_scrollable(app) {
            return false;
        }

        match kind {
            PointerDeviceKind::Touch | PointerDeviceKind::Trackpad => {
                let touch_thumb_rect = thumb_rect.expand_to_include(Rect::from_circle(
                    thumb_rect.center(),
                    K_MIN_INTERACTIVE_SIZE / 2.0,
                ));
                touch_thumb_rect.contains(position)
            }
            PointerDeviceKind::Mouse
            | PointerDeviceKind::Stylus
            | PointerDeviceKind::InvertedStylus
            | PointerDeviceKind::Unknown => thumb_rect.contains(position),
        }
    }

    /// Removes the listener this painter added to its fade animation.
    pub fn dispose(self: Handle<Self>, app: &mut App) {
        let animation = self.fadeout_opacity_animation(app);
        animation.remove_listener(
            app,
            &Listener::handle_method(self, Handle::<ScrollbarPainter>::notify_listeners),
        );
        app.get_mut(self).change_notifier.dispose();
    }
}

impl ChangeNotifier for ScrollbarPainter {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl Debug for ScrollbarPainter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ScrollbarPainter")
    }
}

fn need_paint(metrics: Option<&dyn ScrollMetrics>) -> bool {
    metrics.is_some_and(|metrics| {
        metrics.max_scroll_extent() - metrics.min_scroll_extent() > PRECISION_ERROR_TOLERANCE
    })
}

fn shapes_equal(a: Option<&dyn OutlinedBorder>, b: Option<&dyn OutlinedBorder>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => {
            a as &dyn inset_painting::ShapeBorder == b as &dyn inset_painting::ShapeBorder
        }
        _ => false,
    }
}

/// A [`ScrollbarPainter`] as the `CustomPainter` a `CustomPaint` holds: the painter is an
/// arena object, so the value the widget carries is its handle plus the `Listenable` the
/// render object subscribes to.
#[derive(Clone)]
pub struct ScrollbarPainterRef {
    painter: Handle<ScrollbarPainter>,
    repaint: Rc<dyn Listenable>,
}

impl ScrollbarPainterRef {
    /// Wraps a [`ScrollbarPainter`] as a `CustomPainter`.
    pub fn new(painter: Handle<ScrollbarPainter>) -> ScrollbarPainterRef {
        ScrollbarPainterRef {
            painter,
            repaint: Rc::new(painter),
        }
    }

    /// The painter itself.
    pub fn painter(&self) -> Handle<ScrollbarPainter> {
        self.painter
    }
}

impl CustomPainter for ScrollbarPainterRef {
    fn repaint(&self) -> Option<&Rc<dyn Listenable>> {
        Some(&self.repaint)
    }

    fn paint(&self, app: &mut App, canvas: &mut Canvas, size: Size) {
        self.painter.paint(app, canvas, size);
    }

    fn should_repaint(&self, app: &App, old_delegate: &dyn CustomPainter) -> bool {
        let old_delegate = old_delegate
            .as_any()
            .downcast_ref::<ScrollbarPainterRef>()
            .expect("Dart's covariant parameter");
        self.painter.should_repaint(app, old_delegate.painter)
    }

    fn hit_test(&self, app: &App, position: Offset) -> Option<bool> {
        self.painter.hit_test(app, Some(position))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Debug for ScrollbarPainterRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ScrollbarPainter")
    }
}

/// The fields of Dart's `RawScrollbar`, which its subclasses carry.
pub struct RawScrollbarData {
    pub key: Option<KeyRef>,

    /// The widget below this widget in the tree.
    ///
    /// The scrollbar will be stacked on top of this child. This child (and its
    /// subtree) should include a source of `ScrollNotification` notifications.
    pub child: WidgetRef,

    /// The scroll controller used to implement scrollbar dragging.
    ///
    /// If nothing is passed, the default behavior is to automatically enable
    /// scrollbar dragging on the nearest scroll controller using
    /// [`PrimaryScrollController::of`].
    pub controller: Option<AnyScrollController>,

    /// Indicates that the scrollbar thumb should be visible, even when a scroll
    /// is not underway.
    ///
    /// When false, the scrollbar will be shown during scrolling and will fade out
    /// otherwise. When true, the scrollbar will always be visible and never fade
    /// out. This requires that the scrollbar can access the scroll controller of
    /// the associated scrollable widget.
    ///
    /// Defaults to false when `None`.
    pub thumb_visibility: Option<bool>,

    /// The [`OutlinedBorder`] of the scrollbar's thumb.
    ///
    /// Only one of [`radius`](Self::radius) and this may be specified.
    pub shape: Option<Box<dyn OutlinedBorder>>,

    /// The [`Radius`] of the scrollbar thumb's rounded rectangle corners.
    ///
    /// The scrollbar will be rectangular if this is `None`, which is the default
    /// behavior.
    pub radius: Option<Radius>,

    /// The thickness of the scrollbar in the cross axis of the scrollable.
    ///
    /// If `None`, defaults to 6.0 pixels.
    pub thickness: Option<f64>,

    /// The color of the scrollbar thumb.
    ///
    /// If `None`, defaults to `Color(0x66BCBCBC)`.
    pub thumb_color: Option<Color>,

    /// The preferred smallest size the scrollbar thumb can shrink to when the
    /// total scrollable extent is large, the current visible viewport is small,
    /// and the viewport is not overscrolled.
    ///
    /// Defaults to 18.0.
    pub min_thumb_length: f64,

    /// The preferred smallest size the scrollbar thumb can shrink to when the
    /// viewport is overscrolled.
    ///
    /// When `None`, it defaults to the value of
    /// [`min_thumb_length`](Self::min_thumb_length).
    pub min_overscroll_length: Option<f64>,

    /// Indicates that the scrollbar track should be visible.
    ///
    /// When true, the scrollbar track will always be visible so long as the thumb
    /// is visible. If the scrollbar thumb is not visible, the track will not be
    /// visible either.
    ///
    /// Defaults to false when `None`.
    pub track_visibility: Option<bool>,

    /// The [`Radius`] of the scrollbar track's rounded rectangle corners.
    pub track_radius: Option<Radius>,

    /// The color of the scrollbar track.
    ///
    /// If `None`, defaults to `Color(0x08000000)`.
    pub track_color: Option<Color>,

    /// The color of the scrollbar track's border.
    ///
    /// If `None`, defaults to `Color(0x1a000000)`.
    pub track_border_color: Option<Color>,

    /// The [`Duration`] of the fade animation.
    ///
    /// Defaults to 300 milliseconds.
    pub fade_duration: Duration,

    /// The [`Duration`] of time until the fade animation begins.
    ///
    /// Defaults to 600 milliseconds.
    pub time_to_fade: Duration,

    /// The [`Duration`] of time that a long press will trigger the drag gesture of
    /// the scrollbar thumb.
    ///
    /// Defaults to [`Duration::ZERO`].
    pub press_duration: Duration,

    /// A check that specifies whether a `ScrollNotification` should be handled by
    /// this widget.
    ///
    /// By default, checks whether `notification.depth() == 0`. That means if the
    /// scrollbar is wrapped around multiple scroll views, it only responds to the
    /// nearest one and shows the corresponding scrollbar thumb.
    pub notification_predicate: ScrollNotificationPredicate,

    /// Whether the scrollbar should be interactive and respond to dragging on the
    /// thumb, or tapping in the track area.
    ///
    /// When false, the scrollbar will not respond to gesture or hover events, and
    /// will allow to click through it.
    ///
    /// Defaults to true when `None`.
    pub interactive: Option<bool>,

    /// Dictates the orientation of the scrollbar.
    pub scrollbar_orientation: Option<ScrollbarOrientation>,

    /// Distance from the scrollbar thumb's start or end to the nearest edge of the
    /// viewport in logical pixels. It affects the amount of available paint area.
    ///
    /// Defaults to 0.
    pub main_axis_margin: f64,

    /// Distance from the scrollbar thumb's side to the nearest cross axis edge in
    /// logical pixels.
    ///
    /// Defaults to zero.
    pub cross_axis_margin: f64,

    /// The insets by which the scrollbar thumb and track should be padded.
    ///
    /// When `None`, the inherited `MediaQueryData.padding` is used.
    pub padding: Option<EdgeInsetsGeometry>,
}

impl RawScrollbarData {
    /// Dart's `RawScrollbar` constructor defaults.
    pub fn new(child: WidgetRef) -> RawScrollbarData {
        RawScrollbarData {
            key: None,
            child,
            controller: None,
            thumb_visibility: None,
            shape: None,
            radius: None,
            thickness: None,
            thumb_color: None,
            min_thumb_length: K_MIN_THUMB_EXTENT,
            min_overscroll_length: None,
            track_visibility: None,
            track_radius: None,
            track_color: None,
            track_border_color: None,
            fade_duration: K_SCROLLBAR_FADE_DURATION,
            time_to_fade: K_SCROLLBAR_TIME_TO_FADE,
            press_duration: Duration::ZERO,
            notification_predicate: Rc::new(default_scroll_notification_predicate),
            interactive: None,
            scrollbar_orientation: None,
            main_axis_margin: 0.0,
            cross_axis_margin: 0.0,
            padding: None,
        }
    }
}

impl Debug for RawScrollbarData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RawScrollbar")
            .field("child", &self.child)
            .finish_non_exhaustive()
    }
}

/// The [`RawScrollbarData`] accessors a [`RawScrollbarLeaf`] implementor writes; the bag
/// lives under the field `raw_scrollbar`.
#[macro_export]
macro_rules! raw_scrollbar_accessors {
    () => {
        fn raw_scrollbar_data(&self) -> &$crate::RawScrollbarData {
            &self.raw_scrollbar
        }

        fn raw_scrollbar_data_mut(&mut self) -> &mut $crate::RawScrollbarData {
            &mut self.raw_scrollbar
        }
    };
}

/// The fluent setters of Dart's `RawScrollbar` named arguments, defined on a subclass.
#[macro_export]
macro_rules! raw_scrollbar_setters {
    ($name:ident) => {
        /// Dart `key:`.
        pub fn key(mut self, key: $crate::KeyRef) -> $name {
            self.raw_scrollbar.key = Some(key);
            self
        }

        /// Dart `controller:`.
        pub fn controller(mut self, controller: $crate::AnyScrollController) -> $name {
            self.raw_scrollbar.controller = Some(controller);
            self
        }

        /// Dart `thumbVisibility:`.
        pub fn thumb_visibility(mut self, thumb_visibility: bool) -> $name {
            debug_assert!(
                thumb_visibility || !self.raw_scrollbar.track_visibility.unwrap_or(false),
                "A scrollbar track cannot be drawn without a scrollbar thumb."
            );
            self.raw_scrollbar.thumb_visibility = Some(thumb_visibility);
            self
        }

        /// Dart `trackVisibility:`.
        pub fn track_visibility(mut self, track_visibility: bool) -> $name {
            debug_assert!(
                !(self.raw_scrollbar.thumb_visibility == Some(false) && track_visibility),
                "A scrollbar track cannot be drawn without a scrollbar thumb."
            );
            self.raw_scrollbar.track_visibility = Some(track_visibility);
            self
        }

        /// Dart `shape:`.
        pub fn shape(mut self, shape: Box<dyn inset_painting::OutlinedBorder>) -> $name {
            debug_assert!(self.raw_scrollbar.radius.is_none());
            self.raw_scrollbar.shape = Some(shape);
            self
        }

        /// Dart `radius:`.
        pub fn radius(mut self, radius: inset_embedder::Radius) -> $name {
            debug_assert!(self.raw_scrollbar.shape.is_none());
            self.raw_scrollbar.radius = Some(radius);
            self
        }

        /// Dart `thickness:`.
        pub fn thickness(mut self, thickness: f64) -> $name {
            self.raw_scrollbar.thickness = Some(thickness);
            self
        }

        /// Dart `thumbColor:`.
        pub fn thumb_color(mut self, thumb_color: inset_embedder::Color) -> $name {
            self.raw_scrollbar.thumb_color = Some(thumb_color);
            self
        }

        /// Dart `minThumbLength:`.
        pub fn min_thumb_length(mut self, min_thumb_length: f64) -> $name {
            debug_assert!(min_thumb_length >= 0.0);
            debug_assert!(
                self.raw_scrollbar
                    .min_overscroll_length
                    .is_none_or(|length| length <= min_thumb_length)
            );
            self.raw_scrollbar.min_thumb_length = min_thumb_length;
            self
        }

        /// Dart `minOverscrollLength:`.
        pub fn min_overscroll_length(mut self, min_overscroll_length: f64) -> $name {
            debug_assert!(min_overscroll_length >= 0.0);
            debug_assert!(min_overscroll_length <= self.raw_scrollbar.min_thumb_length);
            self.raw_scrollbar.min_overscroll_length = Some(min_overscroll_length);
            self
        }

        /// Dart `trackRadius:`.
        pub fn track_radius(mut self, track_radius: inset_embedder::Radius) -> $name {
            self.raw_scrollbar.track_radius = Some(track_radius);
            self
        }

        /// Dart `trackColor:`.
        pub fn track_color(mut self, track_color: inset_embedder::Color) -> $name {
            self.raw_scrollbar.track_color = Some(track_color);
            self
        }

        /// Dart `trackBorderColor:`.
        pub fn track_border_color(mut self, track_border_color: inset_embedder::Color) -> $name {
            self.raw_scrollbar.track_border_color = Some(track_border_color);
            self
        }

        /// Dart `fadeDuration:`.
        pub fn fade_duration(mut self, fade_duration: std::time::Duration) -> $name {
            self.raw_scrollbar.fade_duration = fade_duration;
            self
        }

        /// Dart `timeToFade:`.
        pub fn time_to_fade(mut self, time_to_fade: std::time::Duration) -> $name {
            self.raw_scrollbar.time_to_fade = time_to_fade;
            self
        }

        /// Dart `pressDuration:`.
        pub fn press_duration(mut self, press_duration: std::time::Duration) -> $name {
            self.raw_scrollbar.press_duration = press_duration;
            self
        }

        /// Dart `notificationPredicate:`.
        pub fn notification_predicate(
            mut self,
            notification_predicate: $crate::ScrollNotificationPredicate,
        ) -> $name {
            self.raw_scrollbar.notification_predicate = notification_predicate;
            self
        }

        /// Dart `interactive:`.
        pub fn interactive(mut self, interactive: bool) -> $name {
            self.raw_scrollbar.interactive = Some(interactive);
            self
        }

        /// Dart `scrollbarOrientation:`.
        pub fn scrollbar_orientation(
            mut self,
            scrollbar_orientation: $crate::ScrollbarOrientation,
        ) -> $name {
            self.raw_scrollbar.scrollbar_orientation = Some(scrollbar_orientation);
            self
        }

        /// Dart `mainAxisMargin:`.
        pub fn main_axis_margin(mut self, main_axis_margin: f64) -> $name {
            self.raw_scrollbar.main_axis_margin = main_axis_margin;
            self
        }

        /// Dart `crossAxisMargin:`.
        pub fn cross_axis_margin(mut self, cross_axis_margin: f64) -> $name {
            self.raw_scrollbar.cross_axis_margin = cross_axis_margin;
            self
        }

        /// Dart `padding:`.
        pub fn padding(mut self, padding: inset_painting::EdgeInsetsGeometry) -> $name {
            self.raw_scrollbar.padding = Some(padding);
            self
        }
    };
}

/// A widget with the fields of Dart's `RawScrollbar`, whose state is a
/// [`RawScrollbarStateLeaf`].
pub trait RawScrollbarLeaf: StatefulWidget {
    /// The [`RawScrollbarData`] bag
    /// ([`raw_scrollbar_accessors!`](crate::raw_scrollbar_accessors)).
    fn raw_scrollbar_data(&self) -> &RawScrollbarData;

    /// The [`RawScrollbarData`] bag, mutably.
    fn raw_scrollbar_data_mut(&mut self) -> &mut RawScrollbarData;
}

/// An extendable base class for building scrollbars that fade in and out.
///
/// To add a scrollbar to a `ScrollView`, like a `ListView` or a `CustomScrollView`, wrap the
/// scroll view widget in a [`RawScrollbar`] widget.
///
/// A scrollbar thumb indicates which portion of a `ScrollView` is actually
/// visible.
///
/// By default, the thumb will fade in and out as the child scroll view scrolls.
/// When [`thumb_visibility`](RawScrollbarData::thumb_visibility) is true, the scrollbar thumb
/// will remain visible without the fade animation. This requires that the scroll controller
/// associated with the scrollable widget is provided to
/// [`controller`](RawScrollbarData::controller), or that the [`PrimaryScrollController`] is
/// being used by that scrollable widget.
///
/// If the scrollbar is wrapped around multiple scroll views, it only responds to the nearest
/// one and shows the corresponding scrollbar thumb by default. The
/// [`notification_predicate`](RawScrollbarData::notification_predicate) allows the ability to
/// customize which scroll notifications the scrollbar should listen to.
///
/// If the child scroll view is infinitely long, the [`RawScrollbar`] will not be painted. In
/// this case, the scrollbar cannot accurately represent the relative location of the visible
/// area, or calculate the accurate delta to apply when dragging on the thumb or tapping on
/// the track.
///
/// ### Interaction
///
/// Scrollbars are interactive and can use the [`PrimaryScrollController`] if a controller is
/// not set. Interactive scrollbar thumbs can be dragged along the main axis of the scroll
/// view to change the scroll position. Tapping along the track exclusive of the thumb will
/// trigger a [`ScrollIncrementType::Page`] based on the relative position to the thumb.
///
/// ### Automatic scrollbars on desktop platforms
///
/// Scrollbars are added to most scrollable widgets by default on the desktop platforms. This
/// is done through
/// [`ScrollBehavior::build_scrollbar`](crate::ScrollBehavior::build_scrollbar) as part of an
/// app's [`ScrollConfiguration`].
pub struct RawScrollbar {
    raw_scrollbar: RawScrollbarData,
}

impl RawScrollbar {
    /// Creates a basic raw scrollbar that wraps the given child.
    ///
    /// The child, or a descendant of the child, should be a source of
    /// `ScrollNotification` notifications, typically a `Scrollable` widget.
    pub fn new<K>(child: impl IntoWidget<K>) -> RawScrollbar {
        RawScrollbar {
            raw_scrollbar: RawScrollbarData::new(child.into_widget()),
        }
    }

    crate::raw_scrollbar_setters!(RawScrollbar);
}

impl RawScrollbarLeaf for RawScrollbar {
    crate::raw_scrollbar_accessors!();
}

impl Debug for RawScrollbar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.raw_scrollbar, f)
    }
}

impl StatefulWidget for RawScrollbar {
    type State = RawScrollbarState;

    fn key(&self) -> Option<&KeyRef> {
        self.raw_scrollbar.key.as_ref()
    }

    fn create_state(&self) -> RawScrollbarState {
        RawScrollbarState {
            state: StateData::new(),
            ticker_provider: TickerProviderStateMixinData::new(),
            raw_scrollbar_state: RawScrollbarStateData::new(),
        }
    }
}

/// The fields of Dart's `RawScrollbarState`, which its subclasses carry.
pub struct RawScrollbarStateData {
    start_drag_scrollbar_axis_offset: Option<Offset>,
    last_drag_update_offset: Option<Offset>,
    start_drag_thumb_offset: Option<f64>,
    cached_controller: Option<AnyScrollController>,
    fadeout_timer: Option<Timer>,
    fadeout_animation_controller: Option<Handle<AnimationController>>,
    fadeout_opacity_animation: Option<Handle<CurvedAnimation>>,
    scrollbar_painter: Option<Handle<ScrollbarPainter>>,
    scrollbar_painter_key: Rc<GlobalKey>,
    hover_is_active: bool,
    thumb_drag: Option<Rc<dyn Drag>>,
    max_scroll_extent_permits_scrolling: bool,
    thumb_hold: Option<Rc<dyn ScrollHoldController>>,
    axis: Option<Axis>,
    gesture_detector_key: Rc<GlobalKey>,
}

impl RawScrollbarStateData {
    /// The state a fresh [`RawScrollbarState`] starts from.
    pub fn new() -> RawScrollbarStateData {
        RawScrollbarStateData {
            start_drag_scrollbar_axis_offset: None,
            last_drag_update_offset: None,
            start_drag_thumb_offset: None,
            cached_controller: None,
            fadeout_timer: None,
            fadeout_animation_controller: None,
            fadeout_opacity_animation: None,
            scrollbar_painter: None,
            scrollbar_painter_key: Rc::new(GlobalKey::new()),
            hover_is_active: false,
            thumb_drag: None,
            max_scroll_extent_permits_scrolling: false,
            thumb_hold: None,
            axis: None,
            gesture_detector_key: Rc::new(GlobalKey::new()),
        }
    }
}

impl Default for RawScrollbarStateData {
    fn default() -> RawScrollbarStateData {
        RawScrollbarStateData::new()
    }
}

/// The [`RawScrollbarStateData`] accessors a [`RawScrollbarStateLeaf`] implementor writes;
/// the bag lives under the field `raw_scrollbar_state`.
#[macro_export]
macro_rules! raw_scrollbar_state_accessors {
    () => {
        fn raw_scrollbar_state_data(
            self: ::inset_foundation::Handle<Self>,
            app: &::inset_foundation::App,
        ) -> &$crate::RawScrollbarStateData {
            &app.get(self).raw_scrollbar_state
        }

        fn raw_scrollbar_state_data_mut(
            self: ::inset_foundation::Handle<Self>,
            app: &mut ::inset_foundation::App,
        ) -> &mut $crate::RawScrollbarStateData {
            &mut app.get_mut(self).raw_scrollbar_state
        }

        fn raw_scrollbar_state_bag(&mut self) -> &mut $crate::RawScrollbarStateData {
            &mut self.raw_scrollbar_state
        }
    };
}

/// The state for a [`RawScrollbar`] widget, also shared by the `CupertinoScrollbar` widget.
///
/// Controls the animation that fades a scrollbar's thumb in and out of view, and provides
/// default gestures for dragging the scrollbar thumb and tapping on the scrollbar track.
///
/// The shared bodies are associated functions on [`RawScrollbarState`]; an override calls
/// one where Dart writes `super.…`.
pub trait RawScrollbarStateLeaf: State + TickerProviderStateMixin + TickerProviderObject
where
    Self::Widget: RawScrollbarLeaf,
{
    /// The [`RawScrollbarStateData`] bag
    /// ([`raw_scrollbar_state_accessors!`](crate::raw_scrollbar_state_accessors)).
    fn raw_scrollbar_state_data(self: Handle<Self>, app: &App) -> &RawScrollbarStateData;

    /// The [`RawScrollbarStateData`] bag, mutably.
    fn raw_scrollbar_state_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut RawScrollbarStateData;

    /// The [`RawScrollbarStateData`] bag of a state the framework already holds mutably,
    /// which is what a `set_state` closure gets.
    fn raw_scrollbar_state_bag(&mut self) -> &mut RawScrollbarStateData;

    /// Overridable getter to indicate that the scrollbar should be visible, even
    /// when a scroll is not underway.
    ///
    /// Subclasses can override this getter to make its value depend on an inherited theme.
    ///
    /// Defaults to false when [`RawScrollbarData::thumb_visibility`] is `None`.
    fn show_scrollbar(self: Handle<Self>, app: &mut App) -> bool {
        self.widget(app)
            .raw_scrollbar_data()
            .thumb_visibility
            .unwrap_or(false)
    }

    /// Overridable getter to indicate whether gestures should be enabled on the
    /// scrollbar.
    ///
    /// When false, the scrollbar will not respond to gesture or hover events, and
    /// will allow to click through it.
    ///
    /// Defaults to true when [`RawScrollbarData::interactive`] is `None`.
    fn enable_gestures(self: Handle<Self>, app: &mut App) -> bool {
        self.widget(app)
            .raw_scrollbar_data()
            .interactive
            .unwrap_or(true)
    }

    /// This method is responsible for configuring the
    /// [`scrollbar_painter`](RawScrollbarState::scrollbar_painter) according to the widget's
    /// properties and any inherited widgets the painter depends on, like `Directionality`
    /// and `MediaQuery`.
    ///
    /// Subclasses can override to configure the painter.
    fn update_scrollbar_painter(self: Handle<Self>, app: &mut App) {
        RawScrollbarState::update_scrollbar_painter(self, app);
    }

    /// Returns the [`Axis`] of the child scroll view, or `None` if we haven't seen
    /// a scroll metrics notification yet.
    fn get_scrollbar_direction(self: Handle<Self>, app: &App) -> Option<Axis> {
        self.raw_scrollbar_state_data(app).axis
    }

    /// Handler called when a press on the scrollbar thumb has been recognized.
    ///
    /// Cancels the timer associated with the fade animation of the scrollbar.
    fn handle_thumb_press(self: Handle<Self>, app: &mut App) {
        RawScrollbarState::handle_thumb_press(self, app);
    }

    /// Handler called when a drag gesture on the thumb has started.
    ///
    /// Begins the fade out animation and creates the thumb's drag controller.
    fn handle_thumb_press_start(self: Handle<Self>, app: &mut App, local_position: Offset) {
        RawScrollbarState::handle_thumb_press_start(self, app, local_position);
    }

    /// Handler called when a currently active drag gesture on the thumb moves.
    ///
    /// Updates the position of the child scrollable via the thumb's drag controller.
    fn handle_thumb_press_update(self: Handle<Self>, app: &mut App, local_position: Offset) {
        RawScrollbarState::handle_thumb_press_update(self, app, local_position);
    }

    /// Handler called when a drag gesture on the thumb has ended.
    fn handle_thumb_press_end(
        self: Handle<Self>,
        app: &mut App,
        local_position: Offset,
        velocity: Velocity,
    ) {
        RawScrollbarState::handle_thumb_press_end(self, app, local_position, velocity);
    }

    /// Handler called when the track is tapped in order to page in the tapped
    /// direction.
    fn handle_track_tap_down(self: Handle<Self>, app: &mut App, details: TapDownDetails) {
        RawScrollbarState::handle_track_tap_down(self, app, details);
    }

    /// Cancels the fade out animation so the scrollbar will remain visible for
    /// interaction.
    ///
    /// Can be overridden by subclasses to respond to a hover event. The helper methods
    /// [`RawScrollbarState::is_pointer_over_scrollbar`],
    /// [`RawScrollbarState::is_pointer_over_thumb`] and
    /// [`RawScrollbarState::is_pointer_over_track`] can be used to determine the location of
    /// the pointer relative to the painted scrollbar elements.
    fn handle_hover(self: Handle<Self>, app: &mut App, event: &PointerHoverEvent) {
        RawScrollbarState::handle_hover(self, app, event);
    }

    /// Initiates the fade out animation.
    ///
    /// Can be overridden by subclasses to respond to a pointer exit event.
    fn handle_hover_exit(self: Handle<Self>, app: &mut App) {
        RawScrollbarState::handle_hover_exit(self, app);
    }
}

/// The state for a [`RawScrollbar`] widget.
pub struct RawScrollbarState {
    state: StateData<RawScrollbar>,
    ticker_provider: TickerProviderStateMixinData,
    raw_scrollbar_state: RawScrollbarStateData,
}

impl RawScrollbarState {
    /// Used to paint the scrollbar.
    ///
    /// Can be customized by subclasses to change scrollbar behavior by overriding
    /// [`RawScrollbarStateLeaf::update_scrollbar_painter`].
    pub fn scrollbar_painter<S: RawScrollbarStateLeaf>(
        this: Handle<S>,
        app: &App,
    ) -> Handle<ScrollbarPainter>
    where
        S::Widget: RawScrollbarLeaf,
    {
        this.raw_scrollbar_state_data(app)
            .scrollbar_painter
            .expect("initState creates the painter")
    }

    /// The fade animation controller; Dart's `_fadeoutAnimationController`.
    pub fn fadeout_animation_controller<S: RawScrollbarStateLeaf>(
        this: Handle<S>,
        app: &App,
    ) -> Handle<AnimationController>
    where
        S::Widget: RawScrollbarLeaf,
    {
        this.raw_scrollbar_state_data(app)
            .fadeout_animation_controller
            .expect("initState creates the controller")
    }

    fn effective_scroll_controller<S: RawScrollbarStateLeaf>(
        this: Handle<S>,
        app: &mut App,
    ) -> Option<AnyScrollController>
    where
        S::Widget: RawScrollbarLeaf,
    {
        if let Some(controller) = this.widget(app).raw_scrollbar_data().controller {
            return Some(controller);
        }
        let context = this.context(app);
        PrimaryScrollController::maybe_of(app, context)
    }

    fn show_track<S: RawScrollbarStateLeaf>(this: Handle<S>, app: &mut App) -> bool
    where
        S::Widget: RawScrollbarLeaf,
    {
        this.show_scrollbar(app)
            && this
                .widget(app)
                .raw_scrollbar_data()
                .track_visibility
                .unwrap_or(false)
    }

    /// Dart's `initState`, which a leaf's [`State::init_state`] calls.
    pub fn init_state<S: RawScrollbarStateLeaf>(this: Handle<S>, app: &mut App)
    where
        S::Widget: RawScrollbarLeaf,
    {
        let fade_duration = this.widget(app).raw_scrollbar_data().fade_duration;
        let controller = AnimationController::create(
            app,
            None,
            Some(fade_duration),
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            this,
        );
        AnimationLocalStatusListenersMixin::add_status_listener(
            controller,
            app,
            AnimationStatusListener::handle_method(this, validate_interactions::<S>),
        );
        let opacity =
            CurvedAnimation::create(app, controller.view(), Curves::fast_out_slow_in(), None);
        this.raw_scrollbar_state_data_mut(app)
            .fadeout_animation_controller = Some(controller);
        this.raw_scrollbar_state_data_mut(app)
            .fadeout_opacity_animation = Some(opacity);

        let widget = this.widget(app).raw_scrollbar_data();
        let color = widget.thumb_color.unwrap_or(K_DEFAULT_THUMB_COLOR);
        let thickness = widget.thickness.unwrap_or(K_SCROLLBAR_THICKNESS);
        let radius = widget.radius;
        let track_radius = widget.track_radius;
        let scrollbar_orientation = widget.scrollbar_orientation;
        let main_axis_margin = widget.main_axis_margin;
        let shape = widget.shape.as_ref().map(|shape| shape.clone_outlined());
        let cross_axis_margin = widget.cross_axis_margin;
        let min_length = widget.min_thumb_length;
        let min_overscroll_length = widget.min_overscroll_length.unwrap_or(min_length);

        let painter = ScrollbarPainter::new(app, color, opacity.as_animation());
        painter.set_thickness(app, thickness);
        painter.set_radius(app, radius);
        painter.set_track_radius(app, track_radius);
        painter.set_scrollbar_orientation(app, scrollbar_orientation);
        painter.set_main_axis_margin(app, main_axis_margin);
        painter.set_shape(app, shape);
        painter.set_cross_axis_margin(app, cross_axis_margin);
        painter.set_min_length(app, min_length);
        painter.set_min_overscroll_length(app, min_overscroll_length);
        this.raw_scrollbar_state_data_mut(app).scrollbar_painter = Some(painter);
    }

    /// Dart's `didChangeDependencies`, which a leaf's [`State::did_change_dependencies`]
    /// calls.
    pub fn did_change_dependencies<S: RawScrollbarStateLeaf>(this: Handle<S>, app: &mut App)
    where
        S::Widget: RawScrollbarLeaf,
    {
        RawScrollbarState::debug_schedule_check_has_valid_scroll_position(this, app);
    }

    fn debug_schedule_check_has_valid_scroll_position<S: RawScrollbarStateLeaf>(
        this: Handle<S>,
        app: &mut App,
    ) where
        S::Widget: RawScrollbarLeaf,
    {
        if !cfg!(debug_assertions) || !this.show_scrollbar(app) {
            return;
        }
        SchedulerBinding::add_post_frame_callback(
            app,
            FrameCallback::new(move |app: &mut App, _duration| {
                RawScrollbarState::debug_check_has_valid_scroll_position(this, app);
            }),
        );
    }

    fn debug_check_has_valid_scroll_position<S: RawScrollbarStateLeaf>(
        this: Handle<S>,
        app: &mut App,
    ) where
        S::Widget: RawScrollbarLeaf,
    {
        if !cfg!(debug_assertions) || !this.mounted(app) {
            return;
        }
        let scroll_controller = RawScrollbarState::effective_scroll_controller(this, app);
        let try_primary = this.widget(app).raw_scrollbar_data().controller.is_none();
        let controller_for_error = if try_primary {
            "PrimaryScrollController"
        } else {
            "provided ScrollController"
        };

        let when = if this
            .widget(app)
            .raw_scrollbar_data()
            .thumb_visibility
            .unwrap_or(false)
        {
            "Scrollbar.thumbVisibility is true"
        } else if this.enable_gestures(app) {
            "the scrollbar is interactive"
        } else {
            "using the Scrollbar"
        };

        assert!(
            scroll_controller.is_some(),
            "A ScrollController is required when {when}. {}",
            if try_primary {
                "The Scrollbar was not provided a ScrollController, and attempted to use the \
                 PrimaryScrollController, but none was found."
            } else {
                ""
            }
        );
        let scroll_controller = scroll_controller.expect("asserted above");
        assert!(
            scroll_controller.has_clients(app),
            "The Scrollbar's ScrollController has no ScrollPosition attached. A Scrollbar \
             cannot be painted without a ScrollPosition. The Scrollbar attempted to use the \
             {controller_for_error}. This ScrollController should be associated with the \
             ScrollView that the Scrollbar is being applied to."
        );
        assert!(
            scroll_controller.positions(app).len() == 1,
            "The {controller_for_error} is attached to more than one ScrollPosition. The \
             Scrollbar requires a single ScrollPosition in order to be painted. When {when}, \
             the associated ScrollController must only have one ScrollPosition attached."
        );
    }

    /// Dart's `updateScrollbarPainter`, which a leaf's
    /// [`RawScrollbarStateLeaf::update_scrollbar_painter`] calls.
    pub fn update_scrollbar_painter<S: RawScrollbarStateLeaf>(this: Handle<S>, app: &mut App)
    where
        S::Widget: RawScrollbarLeaf,
    {
        let context = this.context(app);
        let text_direction = crate::widgets::basic::Directionality::of(app, context);
        let show_track = RawScrollbarState::show_track(this, app);
        let enable_gestures = this.enable_gestures(app);
        let painter = RawScrollbarState::scrollbar_painter(this, app);

        let widget = this.widget(app).raw_scrollbar_data();
        let color = widget.thumb_color.unwrap_or(K_DEFAULT_THUMB_COLOR);
        let track_radius = widget.track_radius;
        let track_color = if show_track {
            widget.track_color.unwrap_or(K_DEFAULT_TRACK_COLOR)
        } else {
            K_TRANSPARENT
        };
        let track_border_color = if show_track {
            widget
                .track_border_color
                .unwrap_or(K_DEFAULT_TRACK_BORDER_COLOR)
        } else {
            K_TRANSPARENT
        };
        let thickness = widget.thickness.unwrap_or(K_SCROLLBAR_THICKNESS);
        let radius = widget.radius;
        let padding = widget.padding;
        let scrollbar_orientation = widget.scrollbar_orientation;
        let main_axis_margin = widget.main_axis_margin;
        let shape = widget.shape.as_ref().map(|shape| shape.clone_outlined());
        let cross_axis_margin = widget.cross_axis_margin;
        let min_length = widget.min_thumb_length;
        let min_overscroll_length = widget.min_overscroll_length.unwrap_or(min_length);

        let padding = padding
            .unwrap_or_else(|| EdgeInsetsGeometry::from(MediaQuery::padding_of(app, context)))
            .resolve(Some(text_direction));

        painter.set_color(app, color);
        painter.set_track_radius(app, track_radius);
        painter.set_track_color(app, track_color);
        painter.set_track_border_color(app, track_border_color);
        painter.set_text_direction(app, Some(text_direction));
        painter.set_thickness(app, thickness);
        painter.set_radius(app, radius);
        painter.set_padding(app, EdgeInsetsGeometry::from(padding));
        painter.set_scrollbar_orientation(app, scrollbar_orientation);
        painter.set_main_axis_margin(app, main_axis_margin);
        painter.set_shape(app, shape);
        painter.set_cross_axis_margin(app, cross_axis_margin);
        painter.set_min_length(app, min_length);
        painter.set_min_overscroll_length(app, min_overscroll_length);
        painter.set_ignore_pointer(app, !enable_gestures);
    }

    /// Dart's `didUpdateWidget`, which a leaf's [`State::did_update_widget`] calls.
    pub fn did_update_widget<S: RawScrollbarStateLeaf>(
        this: Handle<S>,
        app: &mut App,
        old_widget: &S::Widget,
    ) where
        S::Widget: RawScrollbarLeaf,
    {
        let thumb_visibility = this.widget(app).raw_scrollbar_data().thumb_visibility;
        if thumb_visibility == old_widget.raw_scrollbar_data().thumb_visibility {
            return;
        }
        let controller = RawScrollbarState::fadeout_animation_controller(this, app);
        if thumb_visibility.unwrap_or(false) {
            RawScrollbarState::debug_schedule_check_has_valid_scroll_position(this, app);
            RawScrollbarState::cancel_fadeout_timer(this, app);
            controller.animate_to(app, 1.0, None, Curves::linear());
        } else {
            controller.reverse(app, None);
        }
    }

    fn cancel_fadeout_timer<S: RawScrollbarStateLeaf>(this: Handle<S>, app: &mut App)
    where
        S::Widget: RawScrollbarLeaf,
    {
        if let Some(timer) = this.raw_scrollbar_state_data(app).fadeout_timer {
            timer.cancel(app);
            this.raw_scrollbar_state_data_mut(app).fadeout_timer = None;
        }
    }

    fn maybe_start_fadeout_timer<S: RawScrollbarStateLeaf>(this: Handle<S>, app: &mut App)
    where
        S::Widget: RawScrollbarLeaf,
    {
        if this.show_scrollbar(app) {
            return;
        }
        RawScrollbarState::cancel_fadeout_timer(this, app);
        let time_to_fade = this.widget(app).raw_scrollbar_data().time_to_fade;
        let timer = Timer::new(
            app,
            time_to_fade,
            Listener::new(move |app: &mut App| {
                RawScrollbarState::fadeout_timer_fired(this, app);
            }),
        );
        this.raw_scrollbar_state_data_mut(app).fadeout_timer = Some(timer);
    }

    fn fadeout_timer_fired<S: RawScrollbarStateLeaf>(this: Handle<S>, app: &mut App)
    where
        S::Widget: RawScrollbarLeaf,
    {
        RawScrollbarState::fadeout_animation_controller(this, app).reverse(app, None);
        this.raw_scrollbar_state_data_mut(app).fadeout_timer = None;
    }

    fn dispose_thumb_drag<S: RawScrollbarStateLeaf>(this: Handle<S>, app: &mut App)
    where
        S::Widget: RawScrollbarLeaf,
    {
        this.raw_scrollbar_state_data_mut(app).thumb_drag = None;
    }

    fn dispose_thumb_hold<S: RawScrollbarStateLeaf>(this: Handle<S>, app: &mut App)
    where
        S::Widget: RawScrollbarLeaf,
    {
        this.raw_scrollbar_state_data_mut(app).thumb_hold = None;
    }

    // Given the drag's local position (see `handle_thumb_press_update`), compute the scroll
    // position delta in the scroll axis direction. Deals with the complications arising from
    // scroll metrics changes that have occurred since the last drag update and the need to
    // prevent overscrolling on some platforms.
    fn get_primary_delta<S: RawScrollbarStateLeaf>(
        this: Handle<S>,
        app: &mut App,
        local_position: Offset,
    ) -> Option<f64>
    where
        S::Widget: RawScrollbarLeaf,
    {
        let data = this.raw_scrollbar_state_data(app);
        let cached_controller = data
            .cached_controller
            .expect("a press caches the controller");
        let start_drag_scrollbar_axis_offset = data
            .start_drag_scrollbar_axis_offset
            .expect("a drag records its start");
        let last_drag_update_offset = data
            .last_drag_update_offset
            .expect("a drag records its last update");
        let start_drag_thumb_offset = data
            .start_drag_thumb_offset
            .expect("a drag records the thumb offset");

        let position = cached_controller.position(app);
        let (primary_delta_from_drag_start, primary_delta_from_last_drag_update) =
            match position.axis_direction(app) {
                AxisDirection::Up => (
                    start_drag_scrollbar_axis_offset.dy() - local_position.dy(),
                    last_drag_update_offset.dy() - local_position.dy(),
                ),
                AxisDirection::Right => (
                    local_position.dx() - start_drag_scrollbar_axis_offset.dx(),
                    local_position.dx() - last_drag_update_offset.dx(),
                ),
                AxisDirection::Down => (
                    local_position.dy() - start_drag_scrollbar_axis_offset.dy(),
                    local_position.dy() - last_drag_update_offset.dy(),
                ),
                AxisDirection::Left => (
                    start_drag_scrollbar_axis_offset.dx() - local_position.dx(),
                    last_drag_update_offset.dx() - local_position.dx(),
                ),
            };

        // Convert the amount that the scrollbar moved since the drag started or was last
        // updated into the coordinate space of the scroll position.
        let painter = RawScrollbarState::scrollbar_painter(this, app);
        let mut scroll_offset_global = painter
            .get_track_to_scroll(app, start_drag_thumb_offset + primary_delta_from_drag_start);

        let pixels = position.pixels(app);
        if (primary_delta_from_drag_start > 0.0 && scroll_offset_global < pixels)
            || (primary_delta_from_drag_start < 0.0 && scroll_offset_global > pixels)
        {
            // Adjust the position value if the scrolling direction conflicts with the
            // dragging direction due to scroll metrics shrink.
            scroll_offset_global =
                pixels + painter.get_track_to_scroll(app, primary_delta_from_last_drag_update);
        }
        if scroll_offset_global == pixels {
            return None;
        }

        // Ensure we don't drag into overscroll if the physics do not allow it.
        let metrics = position.copy_with(app);
        let physics_adjustment = position
            .physics(app)
            .apply_boundary_conditions(&metrics, scroll_offset_global);
        let mut new_position = scroll_offset_global - physics_adjustment;

        // The physics may allow overscroll when actually *scrolling*, but dragging on the
        // scrollbar does not always allow us to enter overscroll.
        let context = this.context(app);
        match ScrollConfiguration::of(app, context).get_platform(app, context) {
            TargetPlatform::Fuchsia
            | TargetPlatform::Linux
            | TargetPlatform::MacOS
            | TargetPlatform::Windows => {
                new_position = clamp_double(
                    new_position,
                    position.min_scroll_extent(app),
                    position.max_scroll_extent(app),
                );
            }
            // We can only drag the scrollbar into overscroll on mobile platforms, and only
            // then if the physics allow it.
            TargetPlatform::IOS | TargetPlatform::Android => {}
        }
        let is_reversed = axis_direction_is_reversed(position.axis_direction(app));
        Some(if is_reversed {
            new_position - pixels
        } else {
            pixels - new_position
        })
    }

    /// Dart's `handleThumbPress`, which a leaf's
    /// [`RawScrollbarStateLeaf::handle_thumb_press`] calls.
    pub fn handle_thumb_press<S: RawScrollbarStateLeaf>(this: Handle<S>, app: &mut App)
    where
        S::Widget: RawScrollbarLeaf,
    {
        RawScrollbarState::debug_check_has_valid_scroll_position(this, app);
        let controller = RawScrollbarState::effective_scroll_controller(this, app);
        this.raw_scrollbar_state_data_mut(app).cached_controller = controller;
        if this.get_scrollbar_direction(app).is_none() {
            return;
        }
        RawScrollbarState::cancel_fadeout_timer(this, app);
        let position = controller.expect("a valid scroll position").position(app);
        let hold = position.hold(
            app,
            Listener::new(move |app: &mut App| {
                RawScrollbarState::dispose_thumb_hold(this, app);
            }),
        );
        this.raw_scrollbar_state_data_mut(app).thumb_hold = Some(hold);
    }

    /// Dart's `handleThumbPressStart`, which a leaf's
    /// [`RawScrollbarStateLeaf::handle_thumb_press_start`] calls.
    pub fn handle_thumb_press_start<S: RawScrollbarStateLeaf>(
        this: Handle<S>,
        app: &mut App,
        local_position: Offset,
    ) where
        S::Widget: RawScrollbarLeaf,
    {
        RawScrollbarState::debug_check_has_valid_scroll_position(this, app);
        if this.get_scrollbar_direction(app).is_none() {
            return;
        }
        RawScrollbarState::cancel_fadeout_timer(this, app);
        RawScrollbarState::fadeout_animation_controller(this, app).forward(app, None);

        debug_assert!(this.raw_scrollbar_state_data(app).thumb_drag.is_none());
        let position = this
            .raw_scrollbar_state_data(app)
            .cached_controller
            .expect("a press caches the controller")
            .position(app);
        let render_box = RawScrollbarState::painter_render_box(this, app);
        let details = DragStartDetails::new(
            render_box.local_to_global(app, local_position, None),
            Some(local_position),
            None,
            None,
        );
        let drag = position.drag(
            app,
            details,
            Listener::new(move |app: &mut App| {
                RawScrollbarState::dispose_thumb_drag(this, app);
            }),
        );
        this.raw_scrollbar_state_data_mut(app).thumb_drag = Some(drag);
        debug_assert!(this.raw_scrollbar_state_data(app).thumb_hold.is_none());

        let thumb_offset =
            RawScrollbarState::scrollbar_painter(this, app).get_thumb_scroll_offset(app);
        let data = this.raw_scrollbar_state_data_mut(app);
        data.start_drag_scrollbar_axis_offset = Some(local_position);
        data.last_drag_update_offset = Some(local_position);
        data.start_drag_thumb_offset = Some(thumb_offset);
    }

    /// Dart's `handleThumbPressUpdate`, which a leaf's
    /// [`RawScrollbarStateLeaf::handle_thumb_press_update`] calls.
    pub fn handle_thumb_press_update<S: RawScrollbarStateLeaf>(
        this: Handle<S>,
        app: &mut App,
        local_position: Offset,
    ) where
        S::Widget: RawScrollbarLeaf,
    {
        RawScrollbarState::debug_check_has_valid_scroll_position(this, app);
        if this.raw_scrollbar_state_data(app).last_drag_update_offset == Some(local_position) {
            return;
        }
        let position = this
            .raw_scrollbar_state_data(app)
            .cached_controller
            .expect("a press caches the controller")
            .position(app);
        let metrics = position.copy_with(app);
        if !position.physics(app).should_accept_user_offset(&metrics) {
            return;
        }
        let Some(direction) = this.get_scrollbar_direction(app) else {
            return;
        };
        // The thumb drag might be `None` if the drag activity ended.
        debug_assert!(
            this.raw_scrollbar_state_data(app).thumb_hold.is_none()
                || this.raw_scrollbar_state_data(app).thumb_drag.is_none()
        );
        let Some(thumb_drag) = this.raw_scrollbar_state_data(app).thumb_drag.clone() else {
            return;
        };

        let Some(primary_delta) = RawScrollbarState::get_primary_delta(this, app, local_position)
        else {
            return;
        };

        let delta = match direction {
            Axis::Horizontal => Offset::new(primary_delta, 0.0),
            Axis::Vertical => Offset::new(0.0, primary_delta),
        };
        let render_box = RawScrollbarState::painter_render_box(this, app);
        let scroll_details = DragUpdateDetails::new(
            render_box.local_to_global(app, local_position, None),
            Some(local_position),
            None,
            delta,
            Some(primary_delta),
            None,
        );
        // Triggers updates to the scroll position and the scrollbar painter.
        thumb_drag.update(app, scroll_details);

        this.raw_scrollbar_state_data_mut(app)
            .last_drag_update_offset = Some(local_position);
    }

    /// Dart's `handleThumbPressEnd`, which a leaf's
    /// [`RawScrollbarStateLeaf::handle_thumb_press_end`] calls.
    pub fn handle_thumb_press_end<S: RawScrollbarStateLeaf>(
        this: Handle<S>,
        app: &mut App,
        local_position: Offset,
        velocity: Velocity,
    ) where
        S::Widget: RawScrollbarLeaf,
    {
        RawScrollbarState::debug_check_has_valid_scroll_position(this, app);
        let Some(direction) = this.get_scrollbar_direction(app) else {
            return;
        };
        RawScrollbarState::maybe_start_fadeout_timer(this, app);
        let data = this.raw_scrollbar_state_data_mut(app);
        data.cached_controller = None;
        data.last_drag_update_offset = None;

        // The thumb drag might be `None` if the drag activity ended.
        debug_assert!(
            this.raw_scrollbar_state_data(app).thumb_hold.is_none()
                || this.raw_scrollbar_state_data(app).thumb_drag.is_none()
        );
        let Some(thumb_drag) = this.raw_scrollbar_state_data(app).thumb_drag.clone() else {
            return;
        };

        // On mobile platforms flinging the scrollbar thumb causes a ballistic scroll, just
        // like it does via a touch drag. Likewise for desktops when dragging on the trackpad
        // or with a stylus.
        let context = this.context(app);
        let platform = ScrollConfiguration::of(app, context).get_platform(app, context);
        let adjusted_velocity = match platform {
            TargetPlatform::IOS | TargetPlatform::Android => -velocity,
            _ => Velocity::ZERO,
        };
        let render_box = RawScrollbarState::painter_render_box(this, app);
        let details = DragEndDetails::new(
            render_box.local_to_global(app, local_position, None),
            Some(local_position),
            adjusted_velocity,
            Some(match direction {
                Axis::Horizontal => adjusted_velocity.pixels_per_second.dx(),
                Axis::Vertical => adjusted_velocity.pixels_per_second.dy(),
            }),
        );

        thumb_drag.end(app, details);
        debug_assert!(this.raw_scrollbar_state_data(app).thumb_drag.is_none());

        let data = this.raw_scrollbar_state_data_mut(app);
        data.start_drag_scrollbar_axis_offset = None;
        data.last_drag_update_offset = None;
        data.start_drag_thumb_offset = None;
        data.cached_controller = None;
    }

    /// Dart's `handleTrackTapDown`, which a leaf's
    /// [`RawScrollbarStateLeaf::handle_track_tap_down`] calls.
    pub fn handle_track_tap_down<S: RawScrollbarStateLeaf>(
        this: Handle<S>,
        app: &mut App,
        details: TapDownDetails,
    ) where
        S::Widget: RawScrollbarLeaf,
    {
        // The scrollbar should page towards the position of the tap on the track.
        RawScrollbarState::debug_check_has_valid_scroll_position(this, app);
        let controller = RawScrollbarState::effective_scroll_controller(this, app);
        this.raw_scrollbar_state_data_mut(app).cached_controller = controller;

        let position = controller.expect("a valid scroll position").position(app);
        let metrics = position.copy_with(app);
        if !position.physics(app).should_accept_user_offset(&metrics) {
            return;
        }

        // Determines the scroll direction.
        let thumb_offset = app
            .get(RawScrollbarState::scrollbar_painter(this, app))
            .thumb_offset;
        let local_position = details.local_position;
        let scroll_direction = match axis_direction_to_axis(position.axis_direction(app)) {
            Axis::Vertical => {
                if local_position.dy() > thumb_offset {
                    AxisDirection::Down
                } else {
                    AxisDirection::Up
                }
            }
            Axis::Horizontal => {
                if local_position.dx() > thumb_offset {
                    AxisDirection::Right
                } else {
                    AxisDirection::Left
                }
            }
        };

        let notification_context = position
            .context(app)
            .notification_context(app)
            .expect("a laid-out scrollable has a notification context");
        let state = Scrollable::maybe_of(app, notification_context, None);
        let intent = ScrollIntent {
            direction: scroll_direction,
            r#type: ScrollIncrementType::Page,
        };
        debug_assert!(state.is_some());
        let scroll_increment =
            ScrollAction::get_directional_increment(app, state.expect("a scrollable"), &intent);

        position.move_to(
            app,
            position.pixels(app) + scroll_increment,
            Some(Duration::from_millis(100)),
            Some(Curves::ease_in_out()),
            None,
        );
    }

    // The scroll controller takes precedence over the scroll notification.
    fn should_update_painter<S: RawScrollbarStateLeaf>(
        this: Handle<S>,
        app: &mut App,
        notification_axis: Axis,
    ) -> bool
    where
        S::Widget: RawScrollbarLeaf,
    {
        // Only update the painter of this scrollbar if the notification metrics do not
        // conflict with the information we have from the scroll controller.
        let Some(scroll_controller) = RawScrollbarState::effective_scroll_controller(this, app)
        else {
            // We do not have a scroll controller dictating the axis.
            return true;
        };
        // Has more than one attached position.
        if scroll_controller.positions(app).len() > 1 {
            return false;
        }

        // The scroll controller is not attached to a position, or the notification matches
        // the scroll controller's axis.
        !scroll_controller.has_clients(app)
            || scroll_controller.position(app).axis(app) == notification_axis
    }

    fn handle_scroll_metrics_notification<S: RawScrollbarStateLeaf>(
        this: Handle<S>,
        app: &mut App,
        notification: &ScrollMetricsNotification,
    ) -> bool
    where
        S::Widget: RawScrollbarLeaf,
    {
        let predicate = this
            .widget(app)
            .raw_scrollbar_data()
            .notification_predicate
            .clone();
        if !predicate(&notification.as_scroll_update()) {
            return false;
        }

        let controller = RawScrollbarState::fadeout_animation_controller(this, app);
        if this.show_scrollbar(app) && !controller.status(app).is_forward_or_completed() {
            controller.forward(app, None);
        }

        let metrics = notification.metrics.clone();
        if RawScrollbarState::should_update_painter(this, app, metrics.axis()) {
            let axis_direction = metrics.axis_direction();
            RawScrollbarState::scrollbar_painter(this, app).update(
                app,
                metrics.clone(),
                axis_direction,
            );
        }
        if Some(metrics.axis()) != this.raw_scrollbar_state_data(app).axis {
            let axis = metrics.axis();
            this.set_state(app, |state| {
                state.raw_scrollbar_state_bag().axis = Some(axis);
            });
        }
        let permits = metrics.max_scroll_extent() > 0.0;
        if this
            .raw_scrollbar_state_data(app)
            .max_scroll_extent_permits_scrolling
            != permits
        {
            this.set_state(app, |state| {
                let data = state.raw_scrollbar_state_bag();
                data.max_scroll_extent_permits_scrolling =
                    !data.max_scroll_extent_permits_scrolling;
            });
        }

        false
    }

    fn handle_scroll_notification<S: RawScrollbarStateLeaf>(
        this: Handle<S>,
        app: &mut App,
        notification: &dyn ScrollNotification,
    ) -> bool
    where
        S::Widget: RawScrollbarLeaf,
    {
        let predicate = this
            .widget(app)
            .raw_scrollbar_data()
            .notification_predicate
            .clone();
        if !predicate(notification) {
            return false;
        }

        let metrics = notification.metrics().clone();
        let controller = RawScrollbarState::fadeout_animation_controller(this, app);
        if metrics.max_scroll_extent() <= metrics.min_scroll_extent() {
            // Hide the bar when the scrollable widget has no space to scroll.
            if controller.status(app).is_forward_or_completed() {
                controller.reverse(app, None);
            }

            if RawScrollbarState::should_update_painter(this, app, metrics.axis()) {
                let axis_direction = metrics.axis_direction();
                RawScrollbarState::scrollbar_painter(this, app).update(
                    app,
                    metrics,
                    axis_direction,
                );
            }
            return false;
        }

        let any = notification.as_any();
        if any.is::<ScrollUpdateNotification>() || any.is::<OverscrollNotification>() {
            // Any movement always makes the scrollbar start showing up.
            if !controller.status(app).is_forward_or_completed() {
                controller.forward(app, None);
            }

            RawScrollbarState::cancel_fadeout_timer(this, app);

            if RawScrollbarState::should_update_painter(this, app, metrics.axis()) {
                let axis_direction = metrics.axis_direction();
                RawScrollbarState::scrollbar_painter(this, app).update(
                    app,
                    metrics,
                    axis_direction,
                );
            }
        } else if any.is::<ScrollEndNotification>()
            && this.raw_scrollbar_state_data(app).thumb_drag.is_none()
        {
            RawScrollbarState::maybe_start_fadeout_timer(this, app);
        }
        false
    }

    // The protected API methods — `handle_thumb_press_start`, `handle_thumb_press_update`,
    // `handle_thumb_press_end` — all depend on a local position parameter that defines the
    // event's location relative to the scrollbar. Ensure that the local position is reported
    // consistently, even if the source of the event is a trackpad or a stylus.
    fn global_to_scrollbar<S: RawScrollbarStateLeaf>(
        this: Handle<S>,
        app: &mut App,
        offset: Offset,
    ) -> Offset
    where
        S::Widget: RawScrollbarLeaf,
    {
        RawScrollbarState::painter_render_box(this, app).global_to_local(app, offset, None)
    }

    fn painter_render_box<S: RawScrollbarStateLeaf>(this: Handle<S>, app: &mut App) -> AnyRenderBox
    where
        S::Widget: RawScrollbarLeaf,
    {
        let key = this
            .raw_scrollbar_state_data(app)
            .scrollbar_painter_key
            .clone();
        let context = key
            .current_context(app)
            .expect("the CustomPaint is in the tree");
        context
            .find_render_object(app)
            .expect("the CustomPaint has a render object")
            .as_box()
            .expect("the key is on the scrollbar's CustomPaint")
    }

    fn handle_thumb_drag_cancel<S: RawScrollbarStateLeaf>(this: Handle<S>, app: &mut App)
    where
        S::Widget: RawScrollbarLeaf,
    {
        let key = this
            .raw_scrollbar_state_data(app)
            .gesture_detector_key
            .clone();
        if key.current_context(app).is_none() {
            // The cancel was caused by the gesture detector getting disposed, which means we
            // will get disposed momentarily as well and shouldn't do any work.
            return;
        }
        // The hold might be `None` if the drag started; the drag might be `None` if the drag
        // activity ended.
        debug_assert!(
            this.raw_scrollbar_state_data(app).thumb_hold.is_none()
                || this.raw_scrollbar_state_data(app).thumb_drag.is_none()
        );
        if let Some(hold) = this.raw_scrollbar_state_data(app).thumb_hold.clone() {
            hold.cancel(app);
        }
        if let Some(drag) = this.raw_scrollbar_state_data(app).thumb_drag.clone() {
            drag.cancel(app);
        }
        debug_assert!(this.raw_scrollbar_state_data(app).thumb_hold.is_none());
        debug_assert!(this.raw_scrollbar_state_data(app).thumb_drag.is_none());
    }

    fn can_handle_scroll_gestures<S: RawScrollbarStateLeaf>(this: Handle<S>, app: &mut App) -> bool
    where
        S::Widget: RawScrollbarLeaf,
    {
        if !this.enable_gestures(app) {
            return false;
        }
        let Some(controller) = RawScrollbarState::effective_scroll_controller(this, app) else {
            return false;
        };
        if controller.positions(app).len() != 1 {
            return false;
        }
        let position = controller.position(app);
        position.has_content_dimensions(app)
            && position.max_scroll_extent(app) - position.min_scroll_extent(app)
                > PRECISION_ERROR_TOLERANCE
    }

    fn gestures<S: RawScrollbarStateLeaf>(
        this: Handle<S>,
        app: &mut App,
    ) -> GestureRecognizerFactories
    where
        S::Widget: RawScrollbarLeaf,
    {
        let mut gestures: GestureRecognizerFactories = Vec::new();
        if !RawScrollbarState::can_handle_scroll_gestures(this, app) {
            return gestures;
        }

        let painter_key = this
            .raw_scrollbar_state_data(app)
            .scrollbar_painter_key
            .clone();
        let owner = this
            .context(app)
            .owner(app)
            .expect("a mounted element has a build owner");
        let controller =
            RawScrollbarState::effective_scroll_controller(this, app).expect("checked above");
        match controller.position(app).axis(app) {
            Axis::Horizontal => {
                let key = painter_key.clone();
                gestures.push((
                    std::any::TypeId::of::<HorizontalThumbDragGestureRecognizer>(),
                    GestureRecognizerFactoryWithHandlers::new(
                        move |app: &mut App| {
                            HorizontalThumbDragGestureRecognizer::new(app, owner, key.clone())
                        },
                        move |app: &mut App, instance| {
                            init_thumb_drag_gesture_recognizer(this, app, instance);
                        },
                    )
                    .into_factory(),
                ));
            }
            Axis::Vertical => {
                let key = painter_key.clone();
                gestures.push((
                    std::any::TypeId::of::<VerticalThumbDragGestureRecognizer>(),
                    GestureRecognizerFactoryWithHandlers::new(
                        move |app: &mut App| {
                            VerticalThumbDragGestureRecognizer::new(app, owner, key.clone())
                        },
                        move |app: &mut App, instance| {
                            init_thumb_drag_gesture_recognizer(this, app, instance);
                        },
                    )
                    .into_factory(),
                ));
            }
        }

        let key = painter_key;
        gestures.push((
            std::any::TypeId::of::<TrackTapGestureRecognizer>(),
            GestureRecognizerFactoryWithHandlers::new(
                move |app: &mut App| TrackTapGestureRecognizer::new(app, owner, key.clone()),
                move |app: &mut App, instance: Handle<TrackTapGestureRecognizer>| {
                    instance.set_on_tap_down(
                        app,
                        Some(Rc::new(move |app: &mut App, details| {
                            this.handle_track_tap_down(app, details);
                        })),
                    );
                },
            )
            .into_factory(),
        ));

        gestures
    }

    /// Returns true if the provided [`Offset`] is located over the track of the scrollbar.
    ///
    /// Excludes the scrollbar thumb.
    pub fn is_pointer_over_track<S: RawScrollbarStateLeaf>(
        this: Handle<S>,
        app: &mut App,
        position: Offset,
        kind: PointerDeviceKind,
    ) -> bool
    where
        S::Widget: RawScrollbarLeaf,
    {
        if !RawScrollbarState::has_painter_context(this, app) {
            return false;
        }
        let local_offset = RawScrollbarState::global_to_scrollbar(this, app, position);
        let painter = RawScrollbarState::scrollbar_painter(this, app);
        painter.hit_test_interactive(app, local_offset, kind, false)
            && !painter.hit_test_only_thumb_interactive(app, local_offset, kind)
    }

    /// Returns true if the provided [`Offset`] is located over the thumb of the scrollbar.
    pub fn is_pointer_over_thumb<S: RawScrollbarStateLeaf>(
        this: Handle<S>,
        app: &mut App,
        position: Offset,
        kind: PointerDeviceKind,
    ) -> bool
    where
        S::Widget: RawScrollbarLeaf,
    {
        if !RawScrollbarState::has_painter_context(this, app) {
            return false;
        }
        let local_offset = RawScrollbarState::global_to_scrollbar(this, app, position);
        RawScrollbarState::scrollbar_painter(this, app).hit_test_only_thumb_interactive(
            app,
            local_offset,
            kind,
        )
    }

    /// Returns true if the provided [`Offset`] is located over the track or thumb of the
    /// scrollbar.
    ///
    /// The hit test area for mouse hovering over the scrollbar is larger than
    /// regular hit testing. This is to make it easier to interact with the
    /// scrollbar and present it to the mouse for interaction based on proximity.
    pub fn is_pointer_over_scrollbar<S: RawScrollbarStateLeaf>(
        this: Handle<S>,
        app: &mut App,
        position: Offset,
        kind: PointerDeviceKind,
    ) -> bool
    where
        S::Widget: RawScrollbarLeaf,
    {
        if !RawScrollbarState::has_painter_context(this, app) {
            return false;
        }
        let local_offset = RawScrollbarState::global_to_scrollbar(this, app, position);
        RawScrollbarState::scrollbar_painter(this, app).hit_test_interactive(
            app,
            local_offset,
            kind,
            true,
        )
    }

    fn has_painter_context<S: RawScrollbarStateLeaf>(this: Handle<S>, app: &mut App) -> bool
    where
        S::Widget: RawScrollbarLeaf,
    {
        let key = this
            .raw_scrollbar_state_data(app)
            .scrollbar_painter_key
            .clone();
        key.current_context(app).is_some()
    }

    /// Dart's `handleHover`, which a leaf's [`RawScrollbarStateLeaf::handle_hover`] calls.
    pub fn handle_hover<S: RawScrollbarStateLeaf>(
        this: Handle<S>,
        app: &mut App,
        event: &PointerHoverEvent,
    ) where
        S::Widget: RawScrollbarLeaf,
    {
        // Check if the position of the pointer falls over the painted scrollbar.
        if RawScrollbarState::is_pointer_over_scrollbar(this, app, event.position, event.kind) {
            this.raw_scrollbar_state_data_mut(app).hover_is_active = true;
            // Bring the scrollbar back into view if it has faded or started to fade away.
            RawScrollbarState::fadeout_animation_controller(this, app).forward(app, None);
            RawScrollbarState::cancel_fadeout_timer(this, app);
        } else if this.raw_scrollbar_state_data(app).hover_is_active {
            // The pointer is not over the painted scrollbar.
            this.raw_scrollbar_state_data_mut(app).hover_is_active = false;
            RawScrollbarState::maybe_start_fadeout_timer(this, app);
        }
    }

    /// Dart's `handleHoverExit`, which a leaf's
    /// [`RawScrollbarStateLeaf::handle_hover_exit`] calls.
    pub fn handle_hover_exit<S: RawScrollbarStateLeaf>(this: Handle<S>, app: &mut App)
    where
        S::Widget: RawScrollbarLeaf,
    {
        this.raw_scrollbar_state_data_mut(app).hover_is_active = false;
        RawScrollbarState::maybe_start_fadeout_timer(this, app);
    }

    // Returns the delta that should result from applying the event with axis and direction
    // taken into account.
    fn pointer_signal_event_delta<S: RawScrollbarStateLeaf>(
        this: Handle<S>,
        app: &mut App,
        event: &PointerScrollEvent,
    ) -> f64
    where
        S::Widget: RawScrollbarLeaf,
    {
        let position = this
            .raw_scrollbar_state_data(app)
            .cached_controller
            .expect("a pointer signal caches the controller")
            .position(app);
        let mut delta = if position.axis(app) == Axis::Horizontal {
            event.scroll_delta.dx()
        } else {
            event.scroll_delta.dy()
        };

        if axis_direction_is_reversed(position.axis_direction(app)) {
            delta *= -1.0;
        }
        delta
    }

    // Returns the offset that should result from applying the event to the current position,
    // taking min/max scroll extent into account.
    fn target_scroll_offset_for_pointer_scroll<S: RawScrollbarStateLeaf>(
        this: Handle<S>,
        app: &mut App,
        delta: f64,
    ) -> f64
    where
        S::Widget: RawScrollbarLeaf,
    {
        let position = this
            .raw_scrollbar_state_data(app)
            .cached_controller
            .expect("a pointer signal caches the controller")
            .position(app);
        (position.pixels(app) + delta)
            .max(position.min_scroll_extent(app))
            .min(position.max_scroll_extent(app))
    }

    fn handle_pointer_scroll<S: RawScrollbarStateLeaf>(
        this: Handle<S>,
        app: &mut App,
        event: &PointerEvent,
    ) where
        S::Widget: RawScrollbarLeaf,
    {
        let PointerEvent::Scroll(event) = event else {
            debug_assert!(false, "the pointer signal resolver replays a scroll event");
            return;
        };
        let controller = RawScrollbarState::effective_scroll_controller(this, app);
        this.raw_scrollbar_state_data_mut(app).cached_controller = controller;
        let delta = RawScrollbarState::pointer_signal_event_delta(this, app, event);
        let target_scroll_offset =
            RawScrollbarState::target_scroll_offset_for_pointer_scroll(this, app, delta);
        let position = controller.expect("a cached controller").position(app);
        if delta != 0.0 && target_scroll_offset != position.pixels(app) {
            position.pointer_scroll(app, delta);
        }
    }

    fn received_pointer_signal<S: RawScrollbarStateLeaf>(
        this: Handle<S>,
        app: &mut App,
        event: &PointerEvent,
    ) where
        S::Widget: RawScrollbarLeaf,
    {
        let controller = RawScrollbarState::effective_scroll_controller(this, app);
        this.raw_scrollbar_state_data_mut(app).cached_controller = controller;
        // Only try to scroll if the bar absorbs the hit test.
        let painter = RawScrollbarState::scrollbar_painter(this, app);
        let hit = painter
            .hit_test(app, Some(event.local_position()))
            .unwrap_or(false);
        let Some(controller) = controller else {
            return;
        };
        if !hit
            || !controller.has_clients(app)
            || this.raw_scrollbar_state_data(app).thumb_drag.is_some()
        {
            return;
        }
        let position = controller.position(app);
        match event {
            PointerEvent::Scroll(scroll) => {
                let metrics = position.copy_with(app);
                if !position.physics(app).should_accept_user_offset(&metrics) {
                    return;
                }
                let delta = RawScrollbarState::pointer_signal_event_delta(this, app, scroll);
                let target_scroll_offset =
                    RawScrollbarState::target_scroll_offset_for_pointer_scroll(this, app, delta);
                if delta != 0.0 && target_scroll_offset != position.pixels(app) {
                    let resolver = GestureBinding::instance(app).pointer_signal_resolver(app);
                    resolver.register(
                        app,
                        event,
                        Rc::new(move |app: &mut App, event: &PointerEvent| {
                            RawScrollbarState::handle_pointer_scroll(this, app, event);
                        }),
                    );
                }
            }
            PointerEvent::ScrollInertiaCancel(_) => {
                position.jump_to(app, position.pixels(app));
                // Don't use the pointer signal resolver, all hit-tested scrollables should
                // stop.
            }
            _ => {}
        }
    }

    /// Dart's `dispose`, which a leaf's [`State::dispose`] calls.
    pub fn dispose<S: RawScrollbarStateLeaf>(this: Handle<S>, app: &mut App)
    where
        S::Widget: RawScrollbarLeaf,
    {
        RawScrollbarState::fadeout_animation_controller(this, app).dispose(app);
        RawScrollbarState::cancel_fadeout_timer(this, app);
        RawScrollbarState::scrollbar_painter(this, app).dispose(app);
        if let Some(opacity) = this.raw_scrollbar_state_data(app).fadeout_opacity_animation {
            opacity.dispose(app);
        }
    }

    /// Dart's `build`, which a leaf's [`State::build`] calls.
    pub fn build<S: RawScrollbarStateLeaf>(
        this: Handle<S>,
        app: &mut App,
        _context: BuildContext,
    ) -> WidgetRef
    where
        S::Widget: RawScrollbarLeaf,
    {
        this.update_scrollbar_painter(app);

        let data = this.raw_scrollbar_state_data(app);
        let painter_key = data.scrollbar_painter_key.clone();
        let gesture_detector_key = data.gesture_detector_key.clone();
        let painter = RawScrollbarState::scrollbar_painter(this, app);
        let child = this.widget(app).raw_scrollbar_data().child.clone();
        let gestures = RawScrollbarState::gestures(this, app);

        let custom_paint = CustomPaint::new()
            .key(painter_key)
            .foreground_painter(ScrollbarPainterRef::new(painter))
            .child(RepaintBoundary::new().child(child));

        let mouse_region = MouseRegion::new()
            .on_exit(Rc::new(move |app: &mut App, event| match event.kind {
                PointerDeviceKind::Mouse | PointerDeviceKind::Trackpad => {
                    if this.enable_gestures(app) {
                        this.handle_hover_exit(app);
                    }
                }
                PointerDeviceKind::Stylus
                | PointerDeviceKind::InvertedStylus
                | PointerDeviceKind::Unknown
                | PointerDeviceKind::Touch => {}
            }))
            .on_hover(Rc::new(move |app: &mut App, event| match event.kind {
                PointerDeviceKind::Mouse | PointerDeviceKind::Trackpad => {
                    if this.enable_gestures(app) {
                        this.handle_hover(app, &event);
                    }
                }
                PointerDeviceKind::Stylus
                | PointerDeviceKind::InvertedStylus
                | PointerDeviceKind::Unknown
                | PointerDeviceKind::Touch => {}
            }))
            .child(custom_paint);

        let gesture_detector = RawGestureDetector::new()
            .key(gesture_detector_key)
            .gestures(gestures)
            .child(mouse_region);

        let listener = PointerListener::new()
            .on_pointer_signal(Rc::new(move |app: &mut App, event| {
                RawScrollbarState::received_pointer_signal(this, app, &event);
            }))
            .child(gesture_detector);

        NotificationListener::<ScrollMetricsNotification>::new(
            NotificationListener::<dyn ScrollNotification>::new(
                RepaintBoundary::new().child(listener),
            )
            .on_notification(
                move |app: &mut App, notification: &dyn ScrollNotification| {
                    RawScrollbarState::handle_scroll_notification(this, app, notification)
                },
            )
            .into_widget(),
        )
        .on_notification(
            move |app: &mut App, notification: &ScrollMetricsNotification| {
                RawScrollbarState::handle_scroll_metrics_notification(this, app, notification)
            },
        )
        .into_widget()
    }
}

fn validate_interactions<S: RawScrollbarStateLeaf>(
    this: Handle<S>,
    app: &mut App,
    status: AnimationStatus,
) where
    S::Widget: RawScrollbarLeaf,
{
    if status.is_dismissed() {
        // We do not check for a valid scroll position if the scrollbar is not visible,
        // because it cannot be interacted with.
        return;
    }
    if RawScrollbarState::effective_scroll_controller(this, app).is_none()
        || !this.enable_gestures(app)
    {
        return;
    }
    // Interactive scrollbars need to be properly configured. If it is visible for
    // interaction, ensure we are set up properly. Don't assert immediately if we're in the
    // middle of updating the widget as the controller may not be attached yet in that frame.
    let controller = RawScrollbarState::fadeout_animation_controller(this, app);
    if controller.status(app) == AnimationStatus::Forward
        && this
            .widget(app)
            .raw_scrollbar_data()
            .thumb_visibility
            .unwrap_or(false)
    {
        // When `thumbVisibility` is true and we're animating forward, the check is already
        // scheduled by `debug_schedule_check_has_valid_scroll_position`.
        return;
    }
    RawScrollbarState::debug_check_has_valid_scroll_position(this, app);
}

fn init_thumb_drag_gesture_recognizer<S, R>(this: Handle<S>, app: &mut App, instance: Handle<R>)
where
    S: RawScrollbarStateLeaf,
    S::Widget: RawScrollbarLeaf,
    R: DragGestureRecognizer + ThumbDragGestureRecognizer,
{
    instance.set_on_down(
        app,
        Some(Rc::new(move |app: &mut App, _details: DragDownDetails| {
            this.handle_thumb_press(app);
        })),
    );
    instance.set_on_start(
        app,
        Some(Rc::new(move |app: &mut App, details: DragStartDetails| {
            let local = RawScrollbarState::global_to_scrollbar(this, app, details.global_position);
            this.handle_thumb_press_start(app, local);
        })),
    );
    instance.set_on_update(
        app,
        Some(Rc::new(move |app: &mut App, details: DragUpdateDetails| {
            let local = RawScrollbarState::global_to_scrollbar(this, app, details.global_position);
            this.handle_thumb_press_update(app, local);
        })),
    );
    instance.set_on_end(
        app,
        Some(Rc::new(move |app: &mut App, details: DragEndDetails| {
            let local = RawScrollbarState::global_to_scrollbar(this, app, details.global_position);
            this.handle_thumb_press_end(app, local, details.velocity);
        })),
    );
    instance.set_on_cancel(
        app,
        Some(Listener::new(move |app: &mut App| {
            RawScrollbarState::handle_thumb_drag_cancel(this, app);
        })),
    );
    instance.set_thumb_gesture_settings(app, DeviceGestureSettings::new(Some(0.0)));
    instance.set_drag_start_behavior(app, DragStartBehavior::Down);
}

impl State for RawScrollbarState {
    type Widget = RawScrollbar;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        RawScrollbarState::init_state(self, app);
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        RawScrollbarState::did_change_dependencies(self, app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &RawScrollbar) {
        RawScrollbarState::did_update_widget(self, app, old_widget);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        RawScrollbarState::dispose(self, app);
        TickerProviderStateMixin::dispose(self, app);
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        TickerProviderStateMixin::activate(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        RawScrollbarState::build(self, app, context)
    }
}

impl RawScrollbarStateLeaf for RawScrollbarState {
    crate::raw_scrollbar_state_accessors!();
}

impl TickerProviderObject for RawScrollbarState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
        TickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl TickerProviderStateMixin for RawScrollbarState {
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

impl Debug for RawScrollbarState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RawScrollbarState")
    }
}

/// Dart's `_getLocalOffset`, reading the element registry directly because a recognizer's
/// `isPointerAllowed` gets `&App`.
fn get_local_offset(
    app: &App,
    owner: Handle<BuildOwner>,
    scrollbar_painter_key: &GlobalKey,
    position: Offset,
) -> Option<Offset> {
    let element = owner.global_key_element(app, scrollbar_painter_key.identity())?;
    let render_box = element
        .find_render_object(app)?
        .as_box()
        .expect("the key is on the scrollbar's CustomPaint");
    Some(render_box.global_to_local(app, position, None))
}

fn painter_of(
    app: &App,
    owner: Handle<BuildOwner>,
    custom_paint_key: &GlobalKey,
) -> Option<Handle<ScrollbarPainter>> {
    let element = owner.global_key_element(app, custom_paint_key.identity())?;
    let custom_paint = downcast_widget::<CustomPaint>(&**element.widget(app))?;
    custom_paint
        .foreground_painter
        .as_ref()?
        .as_any()
        .downcast_ref::<ScrollbarPainterRef>()
        .map(ScrollbarPainterRef::painter)
}

fn is_thumb_event(
    app: &App,
    owner: Handle<BuildOwner>,
    custom_paint_key: &GlobalKey,
    event: &PointerDownEvent,
) -> bool {
    let Some(painter) = painter_of(app, owner, custom_paint_key) else {
        return false;
    };
    let Some(local_offset) = get_local_offset(app, owner, custom_paint_key, event.position) else {
        return false;
    };
    painter.hit_test_only_thumb_interactive(app, local_offset, event.kind)
}

fn is_track_event(
    app: &App,
    owner: Handle<BuildOwner>,
    custom_paint_key: &GlobalKey,
    event: &PointerDownEvent,
) -> bool {
    let Some(painter) = painter_of(app, owner, custom_paint_key) else {
        return false;
    };
    let Some(local_offset) = get_local_offset(app, owner, custom_paint_key, event.position) else {
        return false;
    };
    let kind = event.kind;
    painter.hit_test_interactive(app, local_offset, kind, false)
        && !painter.hit_test_only_thumb_interactive(app, local_offset, kind)
}

/// The configuration [`RawScrollbarState`] applies to whichever axis' thumb drag recognizer
/// it built; Dart sets `gestureSettings` on the `DragGestureRecognizer` directly.
pub trait ThumbDragGestureRecognizer: Sized + 'static {
    /// Optional device specific configuration that takes precedence over framework
    /// defaults.
    fn set_thumb_gesture_settings(
        self: Handle<Self>,
        app: &mut App,
        settings: DeviceGestureSettings,
    );
}

/// The forwarders every thumb drag leaf writes: Dart's
/// `_VerticalThumbDragGestureRecognizer` and `_HorizontalThumbDragGestureRecognizer` inherit
/// them from their drag recognizer superclass.
macro_rules! thumb_drag_gesture_recognizer_leaf {
    ($name:ident, $description:literal) => {
        impl $name {
            /// Creates a recognizer that only accepts pointers over the scrollbar thumb the
            /// `custom_paint_key` names.
            pub fn new(
                app: &mut App,
                owner: Handle<BuildOwner>,
                custom_paint_key: Rc<GlobalKey>,
            ) -> Handle<$name> {
                app.create($name {
                    recognizer: GestureRecognizerData::new(),
                    one_sequence: OneSequenceData::new(),
                    drag: DragData::new(),
                    owner,
                    custom_paint_key,
                })
            }
        }

        impl ThumbDragGestureRecognizer for $name {
            fn set_thumb_gesture_settings(
                self: Handle<Self>,
                app: &mut App,
                settings: DeviceGestureSettings,
            ) {
                app.get_mut(self)
                    .recognizer
                    .set_gesture_settings(Some(settings));
            }
        }

        impl RecognizerLeafData for $name {
            fn recognizer(&self) -> &GestureRecognizerData {
                &self.recognizer
            }

            fn recognizer_mut(&mut self) -> &mut GestureRecognizerData {
                &mut self.recognizer
            }
        }

        impl OneSequenceLeafData for $name {
            fn one_sequence(&self) -> &OneSequenceData {
                &self.one_sequence
            }

            fn one_sequence_mut(&mut self) -> &mut OneSequenceData {
                &mut self.one_sequence
            }
        }

        impl DragLeafData for $name {
            fn drag(&self) -> &DragData {
                &self.drag
            }

            fn drag_mut(&mut self) -> &mut DragData {
                &mut self.drag
            }
        }

        impl RecognizerLeaf for $name {
            fn handle_non_allowed_pointer(
                self: Handle<Self>,
                app: &mut App,
                _event: &PointerDownEvent,
            ) {
                inset_gestures::OneSequenceGestureRecognizer::handle_non_allowed_pointer(self, app);
            }

            fn is_pointer_allowed(self: Handle<Self>, app: &App, event: &PointerDownEvent) -> bool {
                let recognizer = app.get(self);
                is_thumb_event(app, recognizer.owner, &recognizer.custom_paint_key, event)
                    && DragGestureRecognizer::is_pointer_allowed(self, app, event)
            }

            fn is_pointer_pan_zoom_allowed(
                self: Handle<Self>,
                _app: &App,
                _event: &PointerPanZoomStartEvent,
            ) -> bool {
                false
            }

            fn add_allowed_pointer(self: Handle<Self>, app: &mut App, event: PointerDownEvent) {
                DragGestureRecognizer::add_allowed_pointer(self, app, event);
            }

            fn add_allowed_pointer_pan_zoom(
                self: Handle<Self>,
                app: &mut App,
                event: PointerPanZoomStartEvent,
            ) {
                DragGestureRecognizer::add_allowed_pointer_pan_zoom(self, app, event);
            }

            fn handle_event(self: Handle<Self>, app: &mut App, event: PointerEvent) {
                DragGestureRecognizer::handle_event(self, app, event);
            }

            fn accept_gesture(self: Handle<Self>, app: &mut App, pointer: i64) {
                DragGestureRecognizer::accept_gesture(self, app, pointer);
            }

            fn reject_gesture(self: Handle<Self>, app: &mut App, pointer: i64) {
                DragGestureRecognizer::reject_gesture(self, app, pointer);
            }

            fn did_stop_tracking_last_pointer(self: Handle<Self>, app: &mut App, pointer: i64) {
                DragGestureRecognizer::did_stop_tracking_last_pointer(self, app, pointer);
            }

            fn dispose(self: Handle<Self>, app: &mut App) {
                DragGestureRecognizer::dispose(self, app);
            }

            fn debug_description(self: Handle<Self>) -> &'static str {
                $description
            }
        }
    };
}

/// Dart's `_VerticalThumbDragGestureRecognizer`: a vertical drag that only tracks pointers
/// over the scrollbar thumb.
pub struct VerticalThumbDragGestureRecognizer {
    recognizer: GestureRecognizerData,
    one_sequence: OneSequenceData,
    drag: DragData,
    owner: Handle<BuildOwner>,
    custom_paint_key: Rc<GlobalKey>,
}

thumb_drag_gesture_recognizer_leaf!(VerticalThumbDragGestureRecognizer, "vertical thumb drag");

impl DragLeaf for VerticalThumbDragGestureRecognizer {
    fn is_fling_gesture(
        self: Handle<Self>,
        app: &App,
        estimate: &VelocityEstimate,
        kind: PointerDeviceKind,
    ) -> bool {
        VerticalDragGestureRecognizerBase::is_fling_gesture(self, app, estimate, kind)
    }

    fn consider_fling(
        self: Handle<Self>,
        app: &App,
        estimate: &VelocityEstimate,
        kind: PointerDeviceKind,
    ) -> Option<DragEndDetails> {
        VerticalDragGestureRecognizerBase::consider_fling(self, app, estimate, kind)
    }

    fn has_sufficient_global_distance_to_accept(
        self: Handle<Self>,
        app: &App,
        pointer_device_kind: PointerDeviceKind,
        device_touch_slop: Option<f64>,
    ) -> bool {
        VerticalDragGestureRecognizerBase::has_sufficient_global_distance_to_accept(
            self,
            app,
            pointer_device_kind,
            device_touch_slop,
        )
    }

    fn get_delta_for_details(self: Handle<Self>, app: &App, delta: Offset) -> Offset {
        VerticalDragGestureRecognizerBase::get_delta_for_details(self, app, delta)
    }

    fn get_primary_value_from_offset(self: Handle<Self>, app: &App, value: Offset) -> Option<f64> {
        VerticalDragGestureRecognizerBase::get_primary_value_from_offset(self, app, value)
    }

    fn get_primary_drag_axis(self: Handle<Self>, app: &App) -> Option<DragDirection> {
        VerticalDragGestureRecognizerBase::get_primary_drag_axis(self, app)
    }
}

/// Dart's `_HorizontalThumbDragGestureRecognizer`: a horizontal drag that only tracks
/// pointers over the scrollbar thumb.
pub struct HorizontalThumbDragGestureRecognizer {
    recognizer: GestureRecognizerData,
    one_sequence: OneSequenceData,
    drag: DragData,
    owner: Handle<BuildOwner>,
    custom_paint_key: Rc<GlobalKey>,
}

thumb_drag_gesture_recognizer_leaf!(
    HorizontalThumbDragGestureRecognizer,
    "horizontal thumb drag"
);

impl DragLeaf for HorizontalThumbDragGestureRecognizer {
    fn is_fling_gesture(
        self: Handle<Self>,
        app: &App,
        estimate: &VelocityEstimate,
        kind: PointerDeviceKind,
    ) -> bool {
        HorizontalDragGestureRecognizerBase::is_fling_gesture(self, app, estimate, kind)
    }

    fn consider_fling(
        self: Handle<Self>,
        app: &App,
        estimate: &VelocityEstimate,
        kind: PointerDeviceKind,
    ) -> Option<DragEndDetails> {
        HorizontalDragGestureRecognizerBase::consider_fling(self, app, estimate, kind)
    }

    fn has_sufficient_global_distance_to_accept(
        self: Handle<Self>,
        app: &App,
        pointer_device_kind: PointerDeviceKind,
        device_touch_slop: Option<f64>,
    ) -> bool {
        HorizontalDragGestureRecognizerBase::has_sufficient_global_distance_to_accept(
            self,
            app,
            pointer_device_kind,
            device_touch_slop,
        )
    }

    fn get_delta_for_details(self: Handle<Self>, app: &App, delta: Offset) -> Offset {
        HorizontalDragGestureRecognizerBase::get_delta_for_details(self, app, delta)
    }

    fn get_primary_value_from_offset(self: Handle<Self>, app: &App, value: Offset) -> Option<f64> {
        HorizontalDragGestureRecognizerBase::get_primary_value_from_offset(self, app, value)
    }

    fn get_primary_drag_axis(self: Handle<Self>, app: &App) -> Option<DragDirection> {
        HorizontalDragGestureRecognizerBase::get_primary_drag_axis(self, app)
    }
}

/// Dart's `_TrackTapGestureRecognizer`: a tap that only tracks pointers over the scrollbar
/// track, outside its thumb.
pub struct TrackTapGestureRecognizer {
    recognizer: GestureRecognizerData,
    one_sequence: OneSequenceData,
    primary: PrimaryPointerData,
    base_tap: BaseTapData,
    owner: Handle<BuildOwner>,
    custom_paint_key: Rc<GlobalKey>,
    on_tap_down: Option<GestureTapDownCallback>,
}

impl TrackTapGestureRecognizer {
    /// Creates a recognizer that only accepts pointers over the scrollbar track the
    /// `custom_paint_key` names.
    pub fn new(
        app: &mut App,
        owner: Handle<BuildOwner>,
        custom_paint_key: Rc<GlobalKey>,
    ) -> Handle<TrackTapGestureRecognizer> {
        app.create(TrackTapGestureRecognizer {
            recognizer: GestureRecognizerData::new(),
            one_sequence: OneSequenceData::new(),
            primary: PrimaryPointerData::new(
                Some(K_PRESS_TIMEOUT),
                Some(UNSET_TOUCH_SLOP),
                Some(UNSET_TOUCH_SLOP),
            ),
            base_tap: BaseTapData::new(),
            owner,
            custom_paint_key,
            on_tap_down: None,
        })
    }

    /// A pointer has contacted the screen at a particular location with a primary
    /// button, which might be the start of a tap.
    pub fn on_tap_down(self: Handle<Self>, app: &App) -> Option<GestureTapDownCallback> {
        app.get(self).on_tap_down.clone()
    }

    /// Sets [`on_tap_down`](Self::on_tap_down).
    pub fn set_on_tap_down(
        self: Handle<Self>,
        app: &mut App,
        callback: Option<GestureTapDownCallback>,
    ) {
        app.get_mut(self).on_tap_down = callback;
    }
}

impl RecognizerLeafData for TrackTapGestureRecognizer {
    fn recognizer(&self) -> &GestureRecognizerData {
        &self.recognizer
    }

    fn recognizer_mut(&mut self) -> &mut GestureRecognizerData {
        &mut self.recognizer
    }
}

impl OneSequenceLeafData for TrackTapGestureRecognizer {
    fn one_sequence(&self) -> &OneSequenceData {
        &self.one_sequence
    }

    fn one_sequence_mut(&mut self) -> &mut OneSequenceData {
        &mut self.one_sequence
    }
}

impl PrimaryPointerLeafData for TrackTapGestureRecognizer {
    fn primary(&self) -> &PrimaryPointerData {
        &self.primary
    }

    fn primary_mut(&mut self) -> &mut PrimaryPointerData {
        &mut self.primary
    }
}

impl BaseTapLeafData for TrackTapGestureRecognizer {
    fn base_tap(&self) -> &BaseTapData {
        &self.base_tap
    }

    fn base_tap_mut(&mut self) -> &mut BaseTapData {
        &mut self.base_tap
    }
}

impl RecognizerLeaf for TrackTapGestureRecognizer {
    fn add_allowed_pointer(self: Handle<Self>, app: &mut App, event: PointerDownEvent) {
        BaseTapGestureRecognizer::add_allowed_pointer(self, app, event);
    }

    fn handle_non_allowed_pointer(self: Handle<Self>, app: &mut App, event: &PointerDownEvent) {
        PrimaryPointerGestureRecognizer::handle_non_allowed_pointer(self, app, event);
    }

    fn start_tracking_pointer(
        self: Handle<Self>,
        app: &mut App,
        pointer: i64,
        transform: Option<Matrix4>,
    ) {
        BaseTapGestureRecognizer::start_tracking_pointer(self, app, pointer, transform);
    }

    fn handle_event(self: Handle<Self>, app: &mut App, event: PointerEvent) {
        PrimaryPointerGestureRecognizer::handle_event(self, app, event);
    }

    fn did_stop_tracking_last_pointer(self: Handle<Self>, app: &mut App, pointer: i64) {
        PrimaryPointerGestureRecognizer::did_stop_tracking_last_pointer(self, app, pointer);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        PrimaryPointerGestureRecognizer::dispose(self, app);
    }

    fn resolve(self: Handle<Self>, app: &mut App, disposition: GestureDisposition) {
        BaseTapGestureRecognizer::resolve(self, app, disposition);
    }

    fn accept_gesture(self: Handle<Self>, app: &mut App, pointer: i64) {
        BaseTapGestureRecognizer::accept_gesture(self, app, pointer);
    }

    fn reject_gesture(self: Handle<Self>, app: &mut App, pointer: i64) {
        BaseTapGestureRecognizer::reject_gesture(self, app, pointer);
    }

    fn is_pointer_allowed(self: Handle<Self>, app: &App, event: &PointerDownEvent) -> bool {
        let recognizer = app.get(self);
        is_track_event(app, recognizer.owner, &recognizer.custom_paint_key, event)
            && recognizer.on_tap_down.is_some()
            && event.buttons == K_PRIMARY_BUTTON
            && GestureRecognizer::is_pointer_allowed(self, app, event)
    }

    fn debug_description(self: Handle<Self>) -> &'static str {
        "track tap"
    }
}

impl PrimaryPointerLeaf for TrackTapGestureRecognizer {
    fn handle_primary_pointer(self: Handle<Self>, app: &mut App, event: PointerEvent) {
        BaseTapGestureRecognizer::handle_primary_pointer(self, app, event);
    }

    fn did_exceed_deadline(self: Handle<Self>, app: &mut App) {
        BaseTapGestureRecognizer::did_exceed_deadline(self, app);
    }
}

impl BaseTapLeaf for TrackTapGestureRecognizer {
    fn handle_tap_down(self: Handle<Self>, app: &mut App, down: &PointerDownEvent) {
        if down.buttons != K_PRIMARY_BUTTON {
            return;
        }
        let details = TapDownDetails::new(
            down.position,
            Some(down.local_position()),
            Some(GestureRecognizer::kind_for_pointer(self, app, down.pointer)),
        );
        if let Some(callback) = app.get(self).on_tap_down.clone() {
            GestureRecognizer::invoke_callback(self, app, "onTapDown", |app| {
                callback(app, details)
            });
        }
    }

    fn handle_tap_up(
        self: Handle<Self>,
        _app: &mut App,
        _down: &PointerDownEvent,
        _up: &PointerUpEvent,
    ) {
    }

    fn handle_tap_cancel(
        self: Handle<Self>,
        _app: &mut App,
        _down: &PointerDownEvent,
        _cancel: Option<&PointerCancelEvent>,
        _reason: &str,
    ) {
    }
}

#[cfg(test)]
mod tests {
    use inset_embedder::{Size, TextDirection};
    use inset_foundation::AppCell;
    use inset_gestures::HitTestResult;
    use inset_painting::EdgeInsets;
    use inset_rendering::{BoxHitTestEntry, RenderCustomPaint, RendererBinding};

    use super::*;
    use crate::test_harness::{VIEW_HEIGHT, VIEW_WIDTH, binding_cell, binding_mount, binding_pump};
    use crate::widgets::basic::{Directionality, SizedBox};
    use crate::widgets::media_query::MediaQueryData;
    use crate::widgets::scroll_controller::{ScrollController, ScrollControllerLeaf};
    use crate::widgets::scroll_view::ListView;

    const ITEM_EXTENT: f64 = 50.0;
    const ITEM_COUNT: usize = 40;

    /// A scrollbar with a global key around a `ListView` of 40 fixed-height boxes, both on the
    /// same controller: the shape every scrollbar test mounts.
    fn mount(cell: &AppCell) -> (Rc<GlobalKey>, Handle<ScrollController>) {
        let key = Rc::new(GlobalKey::new());
        let controller = ScrollController::default(&mut cell.borrow_mut());
        let list = ListView::new()
            .controller(controller.as_controller())
            .children((0..ITEM_COUNT).map(|_| SizedBox::new().height(ITEM_EXTENT).into_widget()));
        let scrollbar = RawScrollbar::new(list)
            .key(key.clone())
            .controller(controller.as_controller());
        let tree: WidgetRef = MediaQuery::new(
            MediaQueryData::new()
                .size(Size::new(VIEW_WIDTH, VIEW_HEIGHT))
                .padding(EdgeInsets::ZERO),
            scrollbar,
        )
        .into_widget();
        binding_mount(
            cell,
            Directionality::new(TextDirection::Ltr, tree).into_widget(),
        );
        // The metrics notification is dispatched from a microtask after layout, so the
        // painter only knows the metrics from the second frame on.
        binding_pump(&mut cell.borrow_mut(), Duration::from_millis(16));
        (key, controller)
    }

    fn painter(app: &mut App, key: &GlobalKey) -> Handle<ScrollbarPainter> {
        let state = key
            .current_state::<RawScrollbarState>(app)
            .expect("the scrollbar is mounted");
        RawScrollbarState::scrollbar_painter(state, app)
    }

    #[test]
    fn a_raw_scrollbar_paints_a_thumb_whose_extent_follows_the_metrics_and_fades_after_the_timer() {
        let cell = binding_cell();
        let (key, controller) = mount(&cell);
        let mut app = cell.borrow_mut();

        controller.jump_to(&mut app, 10.0);
        for step in 1..4 {
            binding_pump(&mut app, Duration::from_millis(16 + step * 100));
        }

        let painter = painter(&mut app, &key);
        let thumb = app.get(painter).thumb_rect.expect("the thumb is painted");
        let content = ITEM_EXTENT * ITEM_COUNT as f64;
        assert!(
            (thumb.height() - VIEW_HEIGHT * (VIEW_HEIGHT / content)).abs() < 0.5,
            "the thumb extent is the visible fraction of the track: {thumb:?}"
        );
        assert!(
            painter.fadeout_opacity_animation(&app).value(&app) > 0.0,
            "a scroll fades the thumb in"
        );

        // The fade-out timer starts when the scroll ends, and the reverse animation runs for
        // the fade duration after it fires.
        drop(app);
        cell.elapse(K_SCROLLBAR_TIME_TO_FADE + Duration::from_millis(1));
        let mut app = cell.borrow_mut();
        for step in 0..8 {
            binding_pump(&mut app, Duration::from_millis(700 + step * 60));
        }
        assert_eq!(
            painter.fadeout_opacity_animation(&app).value(&app),
            0.0,
            "the thumb has faded out"
        );
    }

    #[test]
    fn a_hit_on_the_painted_scrollbar_stops_at_the_custom_paint() {
        let cell = binding_cell();
        let (key, controller) = mount(&cell);
        let mut app = cell.borrow_mut();
        controller.jump_to(&mut app, 10.0);
        for step in 1..4 {
            binding_pump(&mut app, Duration::from_millis(16 + step * 100));
        }
        let painter = painter(&mut app, &key);
        let thumb = app.get(painter).thumb_rect.expect("the thumb is painted");

        // The boxes a hit test through the view walks, innermost first.
        let boxes_at = |app: &mut App, position: Offset| -> Vec<AnyRenderBox> {
            let mut result = HitTestResult::new();
            let views = RendererBinding::instance(app).render_views(app);
            let [view] = views.as_slice() else {
                panic!("the View widget registered one render view: {views:?}");
            };
            view.hit_test(app, &mut result, position);
            result
                .path()
                .iter()
                .filter_map(|entry| {
                    let target: &dyn Any = entry.target();
                    target
                        .downcast_ref::<BoxHitTestEntry>()
                        .map(BoxHitTestEntry::target)
                })
                .collect()
        };
        let is_custom_paint = |app: &App, hit: &AnyRenderBox| {
            hit.as_object().downcast::<RenderCustomPaint>(app).is_some()
        };

        let on_thumb = boxes_at(&mut app, thumb.center());
        assert!(
            is_custom_paint(&app, &on_thumb[0]),
            "the painter absorbs the hit, so nothing below its CustomPaint is hit: {on_thumb:?}"
        );
        let beside_thumb = boxes_at(&mut app, Offset::new(20.0, thumb.center().dy()));
        assert!(
            !is_custom_paint(&app, &beside_thumb[0]) && beside_thumb.len() > on_thumb.len(),
            "a hit off the track goes on to the scrollable underneath: {beside_thumb:?}"
        );
    }

    #[test]
    fn dragging_the_raw_scrollbar_thumb_moves_the_scroll_position() {
        let cell = binding_cell();
        let (key, controller) = mount(&cell);
        let mut app = cell.borrow_mut();
        let state = key
            .current_state::<RawScrollbarState>(&mut app)
            .expect("the scrollbar is mounted");
        assert_eq!(
            state.get_scrollbar_direction(&app),
            Some(Axis::Vertical),
            "the metrics notification tells the scrollbar its axis"
        );

        state.handle_thumb_press(&mut app);
        state.handle_thumb_press_start(&mut app, Offset::new(VIEW_WIDTH - 3.0, 5.0));
        state.handle_thumb_press_update(&mut app, Offset::new(VIEW_WIDTH - 3.0, 25.0));

        let content = ITEM_EXTENT * ITEM_COUNT as f64;
        let scrollable = content - VIEW_HEIGHT;
        let thumb_extent = VIEW_HEIGHT * (VIEW_HEIGHT / content);
        let expected = 20.0 * scrollable / (VIEW_HEIGHT - thumb_extent);
        assert!(
            (controller.offset(&app) - expected).abs() < 0.5,
            "a 20 pixel thumb drag scrolls the track fraction: {}",
            controller.offset(&app)
        );

        state.handle_thumb_press_end(
            &mut app,
            Offset::new(VIEW_WIDTH - 3.0, 25.0),
            Velocity::ZERO,
        );
        assert!(
            app.get(state).raw_scrollbar_state.thumb_drag.is_none(),
            "the drag is released"
        );
    }

    #[test]
    fn a_raw_scrollbar_hides_its_thumb_when_the_child_cannot_scroll() {
        let cell = binding_cell();
        let mut app = cell.borrow_mut();
        let key = Rc::new(GlobalKey::new());
        let controller = ScrollController::default(&mut app);
        let list = ListView::new()
            .controller(controller.as_controller())
            .children([SizedBox::new().height(ITEM_EXTENT).into_widget()]);
        let scrollbar = RawScrollbar::new(list)
            .key(key.clone())
            .controller(controller.as_controller());
        let tree: WidgetRef = MediaQuery::new(
            MediaQueryData::new().size(Size::new(VIEW_WIDTH, VIEW_HEIGHT)),
            scrollbar,
        )
        .into_widget();
        drop(app);
        binding_mount(
            &cell,
            Directionality::new(TextDirection::Ltr, tree).into_widget(),
        );
        let mut app = cell.borrow_mut();

        let painter = painter(&mut app, &key);
        assert_eq!(
            painter.fadeout_opacity_animation(&app).value(&app),
            0.0,
            "a scroll view with nothing to scroll never fades the thumb in"
        );
    }
}
