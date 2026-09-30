//! Flutter counterpart: `gestures/scale.dart`.
//!
//! Leaf object. Superclass bags and `super` namespaces are in `recognizer.rs`.

use std::collections::{HashMap, HashSet};
use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use indexmap::IndexMap;
use inset_embedder::{Matrix4, Offset, PointerDeviceKind};
use inset_foundation::{App, Handle, ValueChanged};

use crate::arena::GestureDisposition;
use crate::constants::{K_MAX_FLING_VELOCITY, K_MIN_FLING_VELOCITY};
use crate::events::{
    PointerDownEvent, PointerEvent, PointerPanZoomStartEvent, PointerPanZoomUpdateEvent,
    compute_pan_slop, compute_scale_slop, transform_position,
};
use crate::gesture_settings::DeviceGestureSettings;
use crate::recognizer::{
    DragStartBehavior, GestureRecognizer, GestureRecognizerData, OneSequenceData,
    OneSequenceGestureRecognizer, OneSequenceLeafData, RecognizerLeaf, RecognizerLeafData,
};
use crate::team::GestureArenaTeam;
use crate::velocity::Velocity;
use crate::velocity_tracker::VelocityTracker;

/// The default conversion factor when treating mouse scrolling as scaling.
///
/// The value was arbitrarily chosen to feel natural for most mousewheels on
/// all supported platforms.
pub const K_DEFAULT_MOUSE_SCROLL_TO_SCALE_FACTOR: f64 = 200.0;

/// The default conversion factor when treating trackpad scrolling as scaling.
///
/// This factor matches the default [`K_DEFAULT_MOUSE_SCROLL_TO_SCALE_FACTOR`] of 200 to
/// feel natural for most trackpads, and the convention that scrolling up means
/// zooming in.
pub const K_DEFAULT_TRACKPAD_SCROLL_TO_SCALE_FACTOR: Offset =
    Offset::new(0.0, -1.0 / K_DEFAULT_MOUSE_SCROLL_TO_SCALE_FACTOR);

/// The possible states of a [`ScaleGestureRecognizer`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum ScaleState {
    /// The recognizer is ready to start recognizing a gesture.
    Ready,

    /// The sequence of pointer events seen thus far is consistent with a scale
    /// gesture but the gesture has not been accepted definitively.
    Possible,

    /// The sequence of pointer events seen thus far has been accepted
    /// definitively as a scale gesture.
    Accepted,

    /// The sequence of pointer events seen thus far has been accepted
    /// definitively as a scale gesture and the pointers established a focal point
    /// and initial scale.
    Started,
}

/// Dart's `_PointerPanZoomData`. Its getters read the recognizer that owns it, which Dart
/// holds as `parent`.
#[derive(Clone, Copy, Debug)]
struct PointerPanZoomData {
    position: Offset,
    pan: Offset,
    scale: f64,
    rotation: f64,
}

impl PointerPanZoomData {
    fn from_start_event(event: &PointerPanZoomStartEvent) -> PointerPanZoomData {
        PointerPanZoomData {
            position: event.position,
            pan: Offset::ZERO,
            scale: 1.0,
            rotation: 0.0,
        }
    }

    fn from_update_event(event: &PointerPanZoomUpdateEvent) -> PointerPanZoomData {
        PointerPanZoomData {
            position: event.position,
            pan: event.pan,
            scale: event.scale,
            rotation: event.rotation,
        }
    }

    fn focal_point(&self, parent: &ScaleGestureRecognizer) -> Offset {
        if parent.trackpad_scroll_causes_scale {
            return self.position;
        }
        self.position + self.pan
    }

    fn scale(&self, parent: &ScaleGestureRecognizer) -> f64 {
        if parent.trackpad_scroll_causes_scale {
            return self.scale
                * ((self.pan.dx() * parent.trackpad_scroll_to_scale_factor.dx())
                    + (self.pan.dy() * parent.trackpad_scroll_to_scale_factor.dy()))
                .exp();
        }
        self.scale
    }

    fn rotation(&self) -> f64 {
        self.rotation
    }
}

/// Details for [`GestureScaleStartCallback`].
#[derive(Clone)]
pub struct ScaleStartDetails {
    /// The initial focal point of the pointers in contact with the screen.
    ///
    /// Reported in global coordinates.
    ///
    /// See also:
    ///
    ///  * [`local_focal_point`](Self::local_focal_point), which is the same value reported in local
    ///    coordinates.
    pub focal_point: Offset,

    /// The initial focal point of the pointers in contact with the screen.
    ///
    /// Reported in local coordinates. Defaults to [`focal_point`](Self::focal_point) if not set in the
    /// constructor.
    ///
    /// See also:
    ///
    ///  * [`focal_point`](Self::focal_point), which is the same value reported in global
    ///    coordinates.
    pub local_focal_point: Offset,

    /// The number of pointers being tracked by the gesture recognizer.
    ///
    /// Typically this is the number of fingers being used to pan the widget using the gesture
    /// recognizer.
    pub pointer_count: usize,

    /// Recorded timestamp of the source pointer event that triggered the scale
    /// event.
    ///
    /// Could be `None` if triggered from proxied events such as accessibility.
    pub source_time_stamp: Option<Duration>,

    /// The kind of the device that initiated the event.
    ///
    /// If multiple pointers are touching the screen, the kind of the pointer
    /// device that first initiated the event is used.
    pub kind: Option<PointerDeviceKind>,
}

impl ScaleStartDetails {
    /// Creates details for [`GestureScaleStartCallback`].
    pub fn new(
        focal_point: Offset,
        local_focal_point: Option<Offset>,
        pointer_count: usize,
        source_time_stamp: Option<Duration>,
        kind: Option<PointerDeviceKind>,
    ) -> ScaleStartDetails {
        ScaleStartDetails {
            focal_point,
            local_focal_point: local_focal_point.unwrap_or(focal_point),
            pointer_count,
            source_time_stamp,
            kind,
        }
    }
}

impl Default for ScaleStartDetails {
    fn default() -> ScaleStartDetails {
        ScaleStartDetails::new(Offset::ZERO, None, 0, None, None)
    }
}

impl Debug for ScaleStartDetails {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ScaleStartDetails")
            .field("focalPoint", &self.focal_point)
            .field("localFocalPoint", &self.local_focal_point)
            .field("pointerCount", &self.pointer_count)
            .field("sourceTimeStamp", &self.source_time_stamp)
            .finish()
    }
}

/// Details for [`GestureScaleUpdateCallback`].
#[derive(Clone)]
pub struct ScaleUpdateDetails {
    /// The amount the gesture's focal point has moved in the coordinate space of
    /// the event receiver since the previous update.
    ///
    /// Defaults to zero if not specified in the constructor.
    pub focal_point_delta: Offset,

    /// The focal point of the pointers in contact with the screen.
    ///
    /// Reported in global coordinates.
    ///
    /// See also:
    ///
    ///  * [`local_focal_point`](Self::local_focal_point), which is the same value reported in local
    ///    coordinates.
    pub focal_point: Offset,

    /// The focal point of the pointers in contact with the screen.
    ///
    /// Reported in local coordinates. Defaults to [`focal_point`](Self::focal_point) if not set in the
    /// constructor.
    ///
    /// See also:
    ///
    ///  * [`focal_point`](Self::focal_point), which is the same value reported in global
    ///    coordinates.
    pub local_focal_point: Offset,

    /// The scale implied by the average distance between the pointers in contact
    /// with the screen.
    ///
    /// This value must be greater than or equal to zero.
    ///
    /// See also:
    ///
    ///  * [`horizontal_scale`](Self::horizontal_scale), which is the scale along the horizontal axis.
    ///  * [`vertical_scale`](Self::vertical_scale), which is the scale along the vertical axis.
    pub scale: f64,

    /// The scale implied by the average distance along the horizontal axis
    /// between the pointers in contact with the screen.
    ///
    /// This value must be greater than or equal to zero.
    ///
    /// See also:
    ///
    ///  * [`scale`](Self::scale), which is the general scale implied by the pointers.
    ///  * [`vertical_scale`](Self::vertical_scale), which is the scale along the vertical axis.
    pub horizontal_scale: f64,

    /// The scale implied by the average distance along the vertical axis
    /// between the pointers in contact with the screen.
    ///
    /// This value must be greater than or equal to zero.
    ///
    /// See also:
    ///
    ///  * [`scale`](Self::scale), which is the general scale implied by the pointers.
    ///  * [`horizontal_scale`](Self::horizontal_scale), which is the scale along the horizontal axis.
    pub vertical_scale: f64,

    /// The angle implied by the first two pointers to enter in contact with
    /// the screen.
    ///
    /// Expressed in radians.
    pub rotation: f64,

    /// The number of pointers being tracked by the gesture recognizer.
    ///
    /// Typically this is the number of fingers being used to pan the widget using the gesture
    /// recognizer. Due to platform limitations, trackpad gestures count as two fingers
    /// even if more than two fingers are used.
    pub pointer_count: usize,

    /// Recorded timestamp of the source pointer event that triggered the scale
    /// event.
    ///
    /// Could be `None` if triggered from proxied events such as accessibility.
    pub source_time_stamp: Option<Duration>,
}

impl ScaleUpdateDetails {
    /// Creates details for [`GestureScaleUpdateCallback`].
    ///
    /// The `scale`, `horizontal_scale`, and `vertical_scale` arguments must be
    /// greater than or equal to zero.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        focal_point: Offset,
        local_focal_point: Option<Offset>,
        scale: f64,
        horizontal_scale: f64,
        vertical_scale: f64,
        rotation: f64,
        pointer_count: usize,
        focal_point_delta: Offset,
        source_time_stamp: Option<Duration>,
    ) -> ScaleUpdateDetails {
        debug_assert!(scale >= 0.0);
        debug_assert!(horizontal_scale >= 0.0);
        debug_assert!(vertical_scale >= 0.0);
        ScaleUpdateDetails {
            focal_point_delta,
            focal_point,
            local_focal_point: local_focal_point.unwrap_or(focal_point),
            scale,
            horizontal_scale,
            vertical_scale,
            rotation,
            pointer_count,
            source_time_stamp,
        }
    }
}

impl Default for ScaleUpdateDetails {
    fn default() -> ScaleUpdateDetails {
        ScaleUpdateDetails::new(
            Offset::ZERO,
            None,
            1.0,
            1.0,
            1.0,
            0.0,
            0,
            Offset::ZERO,
            None,
        )
    }
}

impl Debug for ScaleUpdateDetails {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ScaleUpdateDetails")
            .field("focalPointDelta", &self.focal_point_delta)
            .field("focalPoint", &self.focal_point)
            .field("localFocalPoint", &self.local_focal_point)
            .field("scale", &self.scale)
            .field("horizontalScale", &self.horizontal_scale)
            .field("verticalScale", &self.vertical_scale)
            .field("rotation", &self.rotation)
            .field("pointerCount", &self.pointer_count)
            .field("sourceTimeStamp", &self.source_time_stamp)
            .finish()
    }
}

/// Details for [`GestureScaleEndCallback`].
#[derive(Clone)]
pub struct ScaleEndDetails {
    /// The velocity of the last pointer to be lifted off of the screen.
    pub velocity: Velocity,

    /// The final velocity of the scale factor reported by the gesture.
    pub scale_velocity: f64,

    /// The number of pointers being tracked by the gesture recognizer.
    ///
    /// Typically this is the number of fingers being used to pan the widget using the gesture
    /// recognizer.
    pub pointer_count: usize,
}

impl ScaleEndDetails {
    /// Creates details for [`GestureScaleEndCallback`].
    pub fn new(velocity: Velocity, scale_velocity: f64, pointer_count: usize) -> ScaleEndDetails {
        ScaleEndDetails {
            velocity,
            scale_velocity,
            pointer_count,
        }
    }
}

impl Default for ScaleEndDetails {
    fn default() -> ScaleEndDetails {
        ScaleEndDetails::new(Velocity::ZERO, 0.0, 0)
    }
}

impl Debug for ScaleEndDetails {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ScaleEndDetails")
            .field("velocity", &self.velocity)
            .field("scaleVelocity", &self.scale_velocity)
            .field("pointerCount", &self.pointer_count)
            .finish()
    }
}

/// Signature for when the pointers in contact with the screen have established
/// a focal point and initial scale of 1.0.
pub type GestureScaleStartCallback = ValueChanged<ScaleStartDetails>;

/// Signature for when the pointers in contact with the screen have indicated a
/// new focal point and/or scale.
pub type GestureScaleUpdateCallback = ValueChanged<ScaleUpdateDetails>;

/// Signature for when the pointers are no longer in contact with the screen.
pub type GestureScaleEndCallback = ValueChanged<ScaleEndDetails>;

fn is_fling_gesture(velocity: Velocity) -> bool {
    let speed_squared = velocity.pixels_per_second.distance_squared();
    speed_squared > K_MIN_FLING_VELOCITY * K_MIN_FLING_VELOCITY
}

/// Defines a line between two pointers on screen.
///
/// [`LineBetweenPointers`] is an abstraction of a line between two pointers in
/// contact with the screen. Used to track the rotation of a scale gesture.
#[derive(Clone, Copy, Debug)]
struct LineBetweenPointers {
    // The location and the id of the pointer that marks the start of the line.
    pointer_start_location: Offset,
    pointer_start_id: i64,

    // The location and the id of the pointer that marks the end of the line.
    pointer_end_location: Offset,
    pointer_end_id: i64,
}

impl LineBetweenPointers {
    /// Creates a [`LineBetweenPointers`]. `pointer_start_id` and `pointer_end_id`
    /// should be different.
    fn new(
        pointer_start_location: Offset,
        pointer_start_id: i64,
        pointer_end_location: Offset,
        pointer_end_id: i64,
    ) -> LineBetweenPointers {
        debug_assert!(pointer_start_id != pointer_end_id);
        LineBetweenPointers {
            pointer_start_location,
            pointer_start_id,
            pointer_end_location,
            pointer_end_id,
        }
    }
}

/// Recognizes a scale gesture.
///
/// [`ScaleGestureRecognizer`] tracks the pointers in contact with the screen and
/// calculates their focal point, indicated scale, and rotation. When a focal
/// point is established, the recognizer calls [`set_on_start`](Self::set_on_start). As the focal point,
/// scale, and rotation change, the recognizer calls [`set_on_update`](Self::set_on_update). When the
/// pointers are no longer in contact with the screen, the recognizer calls
/// [`set_on_end`](Self::set_on_end).
pub struct ScaleGestureRecognizer {
    recognizer: GestureRecognizerData,
    one_sequence: OneSequenceData,
    drag_start_behavior: DragStartBehavior,
    on_start: Option<GestureScaleStartCallback>,
    on_update: Option<GestureScaleUpdateCallback>,
    on_end: Option<GestureScaleEndCallback>,
    state: ScaleState,
    last_transform: Option<Matrix4>,
    trackpad_scroll_causes_scale: bool,
    trackpad_scroll_to_scale_factor: Offset,
    initial_focal_point: Offset,
    current_focal_point: Option<Offset>,
    initial_span: f64,
    current_span: f64,
    initial_horizontal_span: f64,
    current_horizontal_span: f64,
    initial_vertical_span: f64,
    current_vertical_span: f64,
    local_focal_point: Offset,
    initial_line: Option<LineBetweenPointers>,
    current_line: Option<LineBetweenPointers>,
    pointer_locations: IndexMap<i64, Offset>,
    /// A queue to sort pointers in order of entrance.
    pointer_queue: Vec<i64>,
    velocity_trackers: HashMap<i64, VelocityTracker>,
    scale_velocity_tracker: Option<VelocityTracker>,
    delta: Offset,
    pointer_pan_zooms: IndexMap<i64, PointerPanZoomData>,
    initial_pan_zoom_scale_factor: f64,
    initial_pan_zoom_rotation_factor: f64,
    initial_event_timestamp: Option<Duration>,
}

impl ScaleGestureRecognizer {
    /// Create a gesture recognizer for interactions intended for scaling content.
    ///
    /// Dart's defaults: `dragStartBehavior` [`DragStartBehavior::Down`],
    /// `trackpadScrollCausesScale` false, `trackpadScrollToScaleFactor`
    /// [`K_DEFAULT_TRACKPAD_SCROLL_TO_SCALE_FACTOR`].
    pub fn new(app: &mut App) -> Handle<ScaleGestureRecognizer> {
        app.create(ScaleGestureRecognizer {
            recognizer: GestureRecognizerData::new(),
            one_sequence: OneSequenceData::new(),
            drag_start_behavior: DragStartBehavior::Down,
            on_start: None,
            on_update: None,
            on_end: None,
            state: ScaleState::Ready,
            last_transform: None,
            trackpad_scroll_causes_scale: false,
            trackpad_scroll_to_scale_factor: K_DEFAULT_TRACKPAD_SCROLL_TO_SCALE_FACTOR,
            initial_focal_point: Offset::ZERO,
            current_focal_point: None,
            initial_span: 0.0,
            current_span: 0.0,
            initial_horizontal_span: 0.0,
            current_horizontal_span: 0.0,
            initial_vertical_span: 0.0,
            current_vertical_span: 0.0,
            local_focal_point: Offset::ZERO,
            initial_line: None,
            current_line: None,
            pointer_locations: IndexMap::new(),
            pointer_queue: Vec::new(),
            velocity_trackers: HashMap::new(),
            scale_velocity_tracker: None,
            delta: Offset::ZERO,
            pointer_pan_zooms: IndexMap::new(),
            initial_pan_zoom_scale_factor: 1.0,
            initial_pan_zoom_rotation_factor: 0.0,
            initial_event_timestamp: None,
        })
    }

    /// The kind of devices that are allowed to be recognized.
    ///
    /// If none, events from all device kinds will be tracked and recognized.
    pub fn supported_devices(
        self: Handle<Self>,
        app: &mut App,
        devices: impl IntoIterator<Item = PointerDeviceKind>,
    ) -> Handle<ScaleGestureRecognizer> {
        app.get_mut(self).recognizer.supported_devices = Some(devices.into_iter().collect());
        self
    }

    /// Sets the kind of devices that are allowed to be recognized; `None` tracks
    /// and recognizes events from all device kinds.
    pub fn set_supported_devices(
        self: Handle<Self>,
        app: &mut App,
        devices: Option<HashSet<PointerDeviceKind>>,
    ) {
        app.get_mut(self).recognizer.supported_devices = devices;
    }

    /// Called when interaction starts. Limits buttons this recognizer accepts.
    pub fn allowed_buttons_filter(
        self: Handle<Self>,
        app: &mut App,
        filter: impl Fn(i64) -> bool + 'static,
    ) -> Handle<ScaleGestureRecognizer> {
        app.get_mut(self).recognizer.allowed_buttons_filter = Rc::new(filter);
        self
    }

    /// Optional device specific configuration that takes precedence over
    /// framework defaults.
    pub fn gesture_settings(self: Handle<Self>, app: &App) -> Option<DeviceGestureSettings> {
        app.get(self).recognizer.gesture_settings
    }

    /// Sets [`gesture_settings`](Self::gesture_settings).
    pub fn set_gesture_settings(
        self: Handle<Self>,
        app: &mut App,
        settings: Option<DeviceGestureSettings>,
    ) {
        app.get_mut(self).recognizer.gesture_settings = settings;
    }

    /// The team that this recognizer belongs to, if any.
    pub fn team(self: Handle<Self>, app: &App) -> Option<Handle<GestureArenaTeam>> {
        app.get(self).one_sequence.team
    }

    /// The [`team`](Self::team) can only be set once.
    pub fn set_team(self: Handle<Self>, app: &mut App, value: Handle<GestureArenaTeam>) {
        debug_assert!(app.get(self).one_sequence.entries.is_empty());
        debug_assert!(app.get(self).one_sequence.tracked_pointers.is_empty());
        debug_assert!(app.get(self).one_sequence.team.is_none());
        app.get_mut(self).one_sequence.team = Some(value);
    }

    /// Registers a new pointer that might be relevant to this gesture detector.
    pub fn add_pointer(self: Handle<Self>, app: &mut App, event: PointerDownEvent) {
        GestureRecognizer::add_pointer(self, app, event);
    }

    /// Registers a new pointer pan/zoom that might be relevant to this gesture
    /// detector.
    pub fn add_pointer_pan_zoom(
        self: Handle<Self>,
        app: &mut App,
        event: PointerPanZoomStartEvent,
    ) {
        GestureRecognizer::add_pointer_pan_zoom(self, app, event);
    }

    /// Releases any resources used by the object.
    pub fn dispose(self: Handle<Self>, app: &mut App) {
        <Self as RecognizerLeaf>::dispose(self, app);
    }

    /// Determines what point is used as the starting point in all calculations
    /// involving this gesture.
    ///
    /// When set to [`DragStartBehavior::Down`], the scale is calculated starting
    /// from the position where the pointer first contacted the screen.
    ///
    /// When set to [`DragStartBehavior::Start`], the scale is calculated starting
    /// from the position where the scale gesture began. The scale gesture may
    /// begin after the time that the pointer first contacted the screen if there
    /// are multiple listeners competing for the gesture. In that case, the
    /// gesture arena waits to determine whether or not the gesture is a scale
    /// gesture before giving the gesture to this GestureRecognizer. This happens
    /// in the case of nested GestureDetectors, for example.
    ///
    /// Defaults to [`DragStartBehavior::Down`].
    pub fn drag_start_behavior(self: Handle<Self>, app: &App) -> DragStartBehavior {
        app.get(self).drag_start_behavior
    }

    /// Sets [`drag_start_behavior`](Self::drag_start_behavior).
    pub fn set_drag_start_behavior(self: Handle<Self>, app: &mut App, value: DragStartBehavior) {
        app.get_mut(self).drag_start_behavior = value;
    }

    /// The pointers in contact with the screen have established a focal point and
    /// initial scale of 1.0.
    ///
    /// This won't be called until the gesture arena has determined that this
    /// GestureRecognizer has won the gesture.
    pub fn set_on_start(
        self: Handle<Self>,
        app: &mut App,
        callback: Option<GestureScaleStartCallback>,
    ) {
        app.get_mut(self).on_start = callback;
    }

    /// The pointers in contact with the screen have indicated a new focal point
    /// and/or scale.
    pub fn set_on_update(
        self: Handle<Self>,
        app: &mut App,
        callback: Option<GestureScaleUpdateCallback>,
    ) {
        app.get_mut(self).on_update = callback;
    }

    /// The pointers are no longer in contact with the screen.
    pub fn set_on_end(
        self: Handle<Self>,
        app: &mut App,
        callback: Option<GestureScaleEndCallback>,
    ) {
        app.get_mut(self).on_end = callback;
    }

    /// Whether scrolling up/down on a trackpad should cause scaling instead of
    /// panning.
    ///
    /// Defaults to false.
    pub fn trackpad_scroll_causes_scale(self: Handle<Self>, app: &App) -> bool {
        app.get(self).trackpad_scroll_causes_scale
    }

    /// Sets [`trackpad_scroll_causes_scale`](Self::trackpad_scroll_causes_scale).
    pub fn set_trackpad_scroll_causes_scale(self: Handle<Self>, app: &mut App, value: bool) {
        app.get_mut(self).trackpad_scroll_causes_scale = value;
    }

    /// A factor to control the direction and magnitude of scale when converting
    /// trackpad scrolling.
    ///
    /// Incoming trackpad pan offsets will be divided by this factor to get scale
    /// values. Increasing this offset will reduce the amount of scaling caused by
    /// a fixed amount of trackpad scrolling.
    ///
    /// Defaults to [`K_DEFAULT_TRACKPAD_SCROLL_TO_SCALE_FACTOR`].
    pub fn trackpad_scroll_to_scale_factor(self: Handle<Self>, app: &App) -> Offset {
        app.get(self).trackpad_scroll_to_scale_factor
    }

    /// Sets [`trackpad_scroll_to_scale_factor`](Self::trackpad_scroll_to_scale_factor).
    pub fn set_trackpad_scroll_to_scale_factor(self: Handle<Self>, app: &mut App, value: Offset) {
        app.get_mut(self).trackpad_scroll_to_scale_factor = value;
    }

    /// The number of pointers being tracked by the gesture recognizer.
    ///
    /// Typically this is the number of fingers being used to pan the widget using the gesture
    /// recognizer.
    pub fn pointer_count(self: Handle<Self>, app: &App) -> usize {
        let this = app.get(self);
        // PointerPanZoom protocol doesn't contain the exact number of pointers
        // used on the trackpad, as it isn't exposed by all platforms. However, it
        // will always be at least two.
        (2 * this.pointer_pan_zooms.len()) + this.pointer_queue.len()
    }
}

impl ScaleGestureRecognizer {
    fn pointer_scale_factor(&self) -> f64 {
        if self.initial_span > 0.0 {
            self.current_span / self.initial_span
        } else {
            1.0
        }
    }

    fn pointer_horizontal_scale_factor(&self) -> f64 {
        if self.initial_horizontal_span > 0.0 {
            self.current_horizontal_span / self.initial_horizontal_span
        } else {
            1.0
        }
    }

    fn pointer_vertical_scale_factor(&self) -> f64 {
        if self.initial_vertical_span > 0.0 {
            self.current_vertical_span / self.initial_vertical_span
        } else {
            1.0
        }
    }

    fn scale_factor(&self) -> f64 {
        let mut scale = self.pointer_scale_factor();
        for p in self.pointer_pan_zooms.values() {
            scale *= p.scale(self) / self.initial_pan_zoom_scale_factor;
        }
        scale
    }

    fn horizontal_scale_factor(&self) -> f64 {
        let mut scale = self.pointer_horizontal_scale_factor();
        for p in self.pointer_pan_zooms.values() {
            scale *= p.scale(self) / self.initial_pan_zoom_scale_factor;
        }
        scale
    }

    fn vertical_scale_factor(&self) -> f64 {
        let mut scale = self.pointer_vertical_scale_factor();
        for p in self.pointer_pan_zooms.values() {
            scale *= p.scale(self) / self.initial_pan_zoom_scale_factor;
        }
        scale
    }

    fn compute_rotation_factor(&self) -> f64 {
        let mut factor = 0.0;
        if let (Some(initial_line), Some(current_line)) = (self.initial_line, self.current_line) {
            let fx = initial_line.pointer_start_location.dx();
            let fy = initial_line.pointer_start_location.dy();
            let sx = initial_line.pointer_end_location.dx();
            let sy = initial_line.pointer_end_location.dy();

            let nfx = current_line.pointer_start_location.dx();
            let nfy = current_line.pointer_start_location.dy();
            let nsx = current_line.pointer_end_location.dx();
            let nsy = current_line.pointer_end_location.dy();

            let angle1 = (fy - sy).atan2(fx - sx);
            let angle2 = (nfy - nsy).atan2(nfx - nsx);

            factor = angle2 - angle1;
        }
        for p in self.pointer_pan_zooms.values() {
            factor += p.rotation();
        }
        factor -= self.initial_pan_zoom_rotation_factor;
        factor
    }

    fn update(&mut self) {
        let previous_focal_point = self.current_focal_point;

        // Compute the focal point
        let mut focal_point = Offset::ZERO;
        for location in self.pointer_locations.values() {
            focal_point = focal_point + *location;
        }
        for p in self.pointer_pan_zooms.values() {
            focal_point = focal_point + p.focal_point(self);
        }
        let current_focal_point = focal_point
            / (self.pointer_locations.len() + self.pointer_pan_zooms.len()).max(1) as f64;
        self.current_focal_point = Some(current_focal_point);

        if previous_focal_point.is_none() {
            self.local_focal_point = transform_position(self.last_transform, current_focal_point);
            self.delta = Offset::ZERO;
        } else {
            let local_previous_focal_point = self.local_focal_point;
            self.local_focal_point = transform_position(self.last_transform, current_focal_point);
            self.delta = self.local_focal_point - local_previous_focal_point;
        }

        let count = self.pointer_locations.len();

        let mut pointer_focal_point = Offset::ZERO;
        for location in self.pointer_locations.values() {
            pointer_focal_point = pointer_focal_point + *location;
        }
        if count > 0 {
            pointer_focal_point = pointer_focal_point / count as f64;
        }

        // Span is the average deviation from focal point. Horizontal and vertical
        // spans are the average deviations from the focal point's horizontal and
        // vertical coordinates, respectively.
        let mut total_deviation = 0.0;
        let mut total_horizontal_deviation = 0.0;
        let mut total_vertical_deviation = 0.0;
        for location in self.pointer_locations.values() {
            total_deviation += (pointer_focal_point - *location).distance();
            total_horizontal_deviation += (pointer_focal_point.dx() - location.dx()).abs();
            total_vertical_deviation += (pointer_focal_point.dy() - location.dy()).abs();
        }
        self.current_span = if count > 0 {
            total_deviation / count as f64
        } else {
            0.0
        };
        self.current_horizontal_span = if count > 0 {
            total_horizontal_deviation / count as f64
        } else {
            0.0
        };
        self.current_vertical_span = if count > 0 {
            total_vertical_deviation / count as f64
        } else {
            0.0
        };
    }

    /// Updates [`initial_line`](Self::initial_line) and
    /// [`current_line`](Self::current_line) accordingly to the situation of the
    /// registered pointers.
    fn update_lines(&mut self) {
        let count = self.pointer_locations.len();
        debug_assert!(self.pointer_queue.len() >= count);

        // In case of just one pointer registered, reconfigure `initial_line`.
        if count < 2 {
            self.initial_line = self.current_line;
        } else if let Some(initial_line) = self.initial_line
            && initial_line.pointer_start_id == self.pointer_queue[0]
            && initial_line.pointer_end_id == self.pointer_queue[1]
        {
            // Rotation updated, set the `current_line`.
            self.current_line = Some(LineBetweenPointers::new(
                self.pointer_locations[&self.pointer_queue[0]],
                self.pointer_queue[0],
                self.pointer_locations[&self.pointer_queue[1]],
                self.pointer_queue[1],
            ));
        } else {
            // A new rotation process is on the way, set the `initial_line`.
            self.initial_line = Some(LineBetweenPointers::new(
                self.pointer_locations[&self.pointer_queue[0]],
                self.pointer_queue[0],
                self.pointer_locations[&self.pointer_queue[1]],
                self.pointer_queue[1],
            ));
            self.current_line = self.initial_line;
        }
    }
}

/// The private members of Dart's `ScaleGestureRecognizer` that call back into the app.
impl ScaleGestureRecognizer {
    fn reconfigure(self: Handle<Self>, app: &mut App, pointer: i64) -> bool {
        let this = app.get_mut(self);
        this.initial_focal_point = this
            .current_focal_point
            .expect("update ran before the pointers are reconfigured");
        this.initial_span = this.current_span;
        this.initial_line = this.current_line;
        this.initial_horizontal_span = this.current_horizontal_span;
        this.initial_vertical_span = this.current_vertical_span;
        if this.pointer_pan_zooms.is_empty() {
            this.initial_pan_zoom_scale_factor = 1.0;
            this.initial_pan_zoom_rotation_factor = 0.0;
        } else {
            this.initial_pan_zoom_scale_factor = this.scale_factor() / this.pointer_scale_factor();
            this.initial_pan_zoom_rotation_factor = this
                .pointer_pan_zooms
                .values()
                .map(PointerPanZoomData::rotation)
                .reduce(|a, b| a + b)
                .expect("not empty");
        }
        if this.state == ScaleState::Started {
            if let Some(on_end) = app.get(self).on_end.clone() {
                let this = app.get(self);
                let mut velocity = this.velocity_trackers[&pointer].get_velocity();
                let scale_velocity = this
                    .scale_velocity_tracker
                    .as_ref()
                    .map_or(-1.0, |tracker| {
                        tracker.get_velocity().pixels_per_second.dx()
                    });
                let pointer_count = self.pointer_count(app);
                if is_fling_gesture(velocity) {
                    let pixels_per_second = velocity.pixels_per_second;
                    if pixels_per_second.distance_squared()
                        > K_MAX_FLING_VELOCITY * K_MAX_FLING_VELOCITY
                    {
                        velocity = Velocity {
                            pixels_per_second: (pixels_per_second / pixels_per_second.distance())
                                * K_MAX_FLING_VELOCITY,
                        };
                    }
                    GestureRecognizer::invoke_callback(self, app, "onEnd", |app| {
                        on_end(
                            app,
                            ScaleEndDetails::new(velocity, scale_velocity, pointer_count),
                        );
                    });
                } else {
                    GestureRecognizer::invoke_callback(self, app, "onEnd", |app| {
                        on_end(
                            app,
                            ScaleEndDetails::new(Velocity::ZERO, scale_velocity, pointer_count),
                        );
                    });
                }
            }
            let this = app.get_mut(self);
            this.state = ScaleState::Accepted;
            // arbitrary PointerDeviceKind
            this.scale_velocity_tracker =
                Some(VelocityTracker::with_kind(PointerDeviceKind::Touch));
            return false;
        }
        // arbitrary PointerDeviceKind
        app.get_mut(self).scale_velocity_tracker =
            Some(VelocityTracker::with_kind(PointerDeviceKind::Touch));
        true
    }

    fn advance_state_machine(
        self: Handle<Self>,
        app: &mut App,
        should_start_if_accepted: bool,
        event: &PointerEvent,
    ) {
        if app.get(self).state == ScaleState::Ready {
            app.get_mut(self).state = ScaleState::Possible;
        }

        let state = app.get(self).state;
        if state == ScaleState::Possible {
            let this = app.get(self);
            let span_delta = (this.current_span - this.initial_span).abs();
            let focal_point_delta = (this
                .current_focal_point
                .expect("update ran before the state machine")
                - this.initial_focal_point)
                .distance();
            let scale_factor = this.scale_factor();
            let pointer_scale_factor = this.pointer_scale_factor();
            if span_delta > compute_scale_slop(event.kind())
                || focal_point_delta
                    > compute_pan_slop(event.kind(), this.recognizer.gesture_settings)
                || (scale_factor / pointer_scale_factor).max(pointer_scale_factor / scale_factor)
                    > 1.05
            {
                self.resolve(app, GestureDisposition::Accepted);
            }
        } else if state >= ScaleState::Accepted {
            self.resolve(app, GestureDisposition::Accepted);
        }

        if app.get(self).state == ScaleState::Accepted && should_start_if_accepted {
            let this = app.get_mut(self);
            this.initial_event_timestamp = Some(event.time_stamp());
            this.state = ScaleState::Started;
            self.dispatch_on_start_callback_if_needed(app);
        }

        if app.get(self).state == ScaleState::Started {
            let scale_factor = app.get(self).scale_factor();
            if let Some(tracker) = app.get_mut(self).scale_velocity_tracker.as_mut() {
                tracker.add_position(event.time_stamp(), Offset::new(scale_factor, 0.0));
            }
            if let Some(on_update) = app.get(self).on_update.clone() {
                let this = app.get(self);
                let details = ScaleUpdateDetails::new(
                    this.current_focal_point
                        .expect("update ran before the state machine"),
                    Some(this.local_focal_point),
                    this.scale_factor(),
                    this.horizontal_scale_factor(),
                    this.vertical_scale_factor(),
                    this.compute_rotation_factor(),
                    self.pointer_count(app),
                    this.delta,
                    Some(event.time_stamp()),
                );
                GestureRecognizer::invoke_callback(self, app, "onUpdate", |app| {
                    on_update(app, details);
                });
            }
        }
    }

    fn dispatch_on_start_callback_if_needed(self: Handle<Self>, app: &mut App) {
        debug_assert!(app.get(self).state == ScaleState::Started);
        if let Some(on_start) = app.get(self).on_start.clone() {
            let this = app.get(self);
            let first_pointer = this
                .pointer_queue
                .first()
                .or_else(|| this.pointer_pan_zooms.keys().next())
                .copied();
            let kind = first_pointer
                .map(|pointer| GestureRecognizer::kind_for_pointer(self, app, pointer));
            let this = app.get(self);
            let details = ScaleStartDetails::new(
                this.current_focal_point
                    .expect("update ran before the gesture started"),
                Some(this.local_focal_point),
                self.pointer_count(app),
                this.initial_event_timestamp,
                kind,
            );
            GestureRecognizer::invoke_callback(self, app, "onStart", |app| {
                on_start(app, details);
            });
        }
        app.get_mut(self).initial_event_timestamp = None;
    }
}

impl RecognizerLeafData for ScaleGestureRecognizer {
    fn recognizer(&self) -> &GestureRecognizerData {
        &self.recognizer
    }
    fn recognizer_mut(&mut self) -> &mut GestureRecognizerData {
        &mut self.recognizer
    }
}

impl OneSequenceLeafData for ScaleGestureRecognizer {
    fn one_sequence(&self) -> &OneSequenceData {
        &self.one_sequence
    }
    fn one_sequence_mut(&mut self) -> &mut OneSequenceData {
        &mut self.one_sequence
    }
}

impl RecognizerLeaf for ScaleGestureRecognizer {
    fn add_allowed_pointer(self: Handle<Self>, app: &mut App, event: PointerDownEvent) {
        OneSequenceGestureRecognizer::add_allowed_pointer(self, app, &event);
        let this = app.get_mut(self);
        this.velocity_trackers
            .insert(event.pointer, VelocityTracker::with_kind(event.kind));
        this.initial_event_timestamp = Some(event.time_stamp);
        if this.state == ScaleState::Ready {
            this.state = ScaleState::Possible;
            this.initial_span = 0.0;
            this.current_span = 0.0;
            this.initial_horizontal_span = 0.0;
            this.current_horizontal_span = 0.0;
            this.initial_vertical_span = 0.0;
            this.current_vertical_span = 0.0;
        }
    }

    fn handle_non_allowed_pointer(self: Handle<Self>, app: &mut App, _event: &PointerDownEvent) {
        OneSequenceGestureRecognizer::handle_non_allowed_pointer(self, app);
    }

    fn is_pointer_pan_zoom_allowed(
        self: Handle<Self>,
        _app: &App,
        _event: &PointerPanZoomStartEvent,
    ) -> bool {
        true
    }

    fn add_allowed_pointer_pan_zoom(
        self: Handle<Self>,
        app: &mut App,
        event: PointerPanZoomStartEvent,
    ) {
        self.start_tracking_pointer(app, event.pointer, event.transform);
        let this = app.get_mut(self);
        this.velocity_trackers
            .insert(event.pointer, VelocityTracker::with_kind(event.kind));
        this.initial_event_timestamp = Some(event.time_stamp);
        if this.state == ScaleState::Ready {
            this.state = ScaleState::Possible;
            this.initial_pan_zoom_scale_factor = 1.0;
            this.initial_pan_zoom_rotation_factor = 0.0;
        }
    }

    fn handle_event(self: Handle<Self>, app: &mut App, event: PointerEvent) {
        debug_assert!(app.get(self).state != ScaleState::Ready);
        let mut did_change_configuration = false;
        let mut should_start_if_accepted = false;
        let this = app.get_mut(self);
        match &event {
            PointerEvent::Move(move_event) => {
                let tracker = this
                    .velocity_trackers
                    .get_mut(&move_event.pointer)
                    .expect("a tracker for every tracked pointer");
                if !move_event.synthesized {
                    tracker.add_position(move_event.time_stamp, move_event.position);
                }
                this.pointer_locations
                    .insert(move_event.pointer, move_event.position);
                should_start_if_accepted = true;
                this.last_transform = move_event.transform;
            }
            PointerEvent::Down(down) => {
                this.pointer_locations.insert(down.pointer, down.position);
                this.pointer_queue.push(down.pointer);
                did_change_configuration = true;
                should_start_if_accepted = true;
                this.last_transform = down.transform;
            }
            PointerEvent::Up(_) | PointerEvent::Cancel(_) => {
                let pointer = event.pointer();
                this.pointer_locations.shift_remove(&pointer);
                this.pointer_queue.retain(|&queued| queued != pointer);
                did_change_configuration = true;
                this.last_transform = event.transform();
            }
            PointerEvent::PanZoomStart(start) => {
                debug_assert!(!this.pointer_pan_zooms.contains_key(&start.pointer));
                this.pointer_pan_zooms
                    .insert(start.pointer, PointerPanZoomData::from_start_event(start));
                did_change_configuration = true;
                should_start_if_accepted = true;
                this.last_transform = start.transform;
            }
            PointerEvent::PanZoomUpdate(update) => {
                debug_assert!(this.pointer_pan_zooms.contains_key(&update.pointer));
                if !update.synthesized && !this.trackpad_scroll_causes_scale {
                    this.velocity_trackers
                        .get_mut(&update.pointer)
                        .expect("a tracker for every tracked pan/zoom")
                        .add_position(update.time_stamp, update.pan);
                }
                this.pointer_pan_zooms.insert(
                    update.pointer,
                    PointerPanZoomData::from_update_event(update),
                );
                this.last_transform = update.transform;
                should_start_if_accepted = true;
            }
            PointerEvent::PanZoomEnd(end) => {
                debug_assert!(this.pointer_pan_zooms.contains_key(&end.pointer));
                this.pointer_pan_zooms.shift_remove(&end.pointer);
                did_change_configuration = true;
            }
            _ => {}
        }

        this.update_lines();
        this.update();

        if !did_change_configuration || self.reconfigure(app, event.pointer()) {
            self.advance_state_machine(app, should_start_if_accepted, &event);
        }
        OneSequenceGestureRecognizer::stop_tracking_if_pointer_no_longer_down(self, app, &event);
    }

    fn accept_gesture(self: Handle<Self>, app: &mut App, _pointer: i64) {
        if app.get(self).state == ScaleState::Possible {
            app.get_mut(self).state = ScaleState::Started;
            self.dispatch_on_start_callback_if_needed(app);
            let this = app.get_mut(self);
            if this.drag_start_behavior == DragStartBehavior::Start {
                this.initial_focal_point = this
                    .current_focal_point
                    .expect("update ran before the gesture is accepted");
                this.initial_span = this.current_span;
                this.initial_line = this.current_line;
                this.initial_horizontal_span = this.current_horizontal_span;
                this.initial_vertical_span = this.current_vertical_span;
                if this.pointer_pan_zooms.is_empty() {
                    this.initial_pan_zoom_scale_factor = 1.0;
                    this.initial_pan_zoom_rotation_factor = 0.0;
                } else {
                    this.initial_pan_zoom_scale_factor =
                        this.scale_factor() / this.pointer_scale_factor();
                    this.initial_pan_zoom_rotation_factor = this
                        .pointer_pan_zooms
                        .values()
                        .map(PointerPanZoomData::rotation)
                        .reduce(|a, b| a + b)
                        .expect("not empty");
                }
            }
        }
    }

    fn reject_gesture(self: Handle<Self>, app: &mut App, pointer: i64) {
        let this = app.get_mut(self);
        this.pointer_pan_zooms.shift_remove(&pointer);
        this.pointer_locations.shift_remove(&pointer);
        this.pointer_queue.retain(|&queued| queued != pointer);
        OneSequenceGestureRecognizer::stop_tracking_pointer(self, app, pointer);
    }

    fn did_stop_tracking_last_pointer(self: Handle<Self>, app: &mut App, _pointer: i64) {
        match app.get(self).state {
            ScaleState::Possible => self.resolve(app, GestureDisposition::Rejected),
            // We should have not seen a pointer yet
            ScaleState::Ready => debug_assert!(false),
            ScaleState::Accepted => {}
            // We should be in the accepted state when user is done
            ScaleState::Started => debug_assert!(false),
        }
        app.get_mut(self).state = ScaleState::Ready;
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).velocity_trackers.clear();
        OneSequenceGestureRecognizer::dispose(self, app);
    }

    fn debug_description(self: Handle<Self>) -> &'static str {
        "scale"
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use inset_foundation::AppCell;

    use super::*;
    use crate::binding::GestureBinding;
    use crate::drag_details::{DragEndDetails, DragStartDetails};
    use crate::events::K_PRIMARY_BUTTON;
    use crate::events::{
        PointerMoveEvent, PointerPanZoomEndEvent, PointerPanZoomUpdateEvent, PointerUpEvent,
    };
    use crate::monodrag::{DragGestureRecognizer, HorizontalDragGestureRecognizer};
    use crate::tap::TapGestureRecognizer;

    /// Flutter's `TestPointer`: keeps the last location so a move carries the
    /// delta the recognizer reads, and the last pan so a pan/zoom update carries its
    /// pan delta.
    struct TestPointer {
        pointer: i64,
        kind: PointerDeviceKind,
        location: Offset,
        pan: Offset,
    }

    impl TestPointer {
        fn new(pointer: i64) -> TestPointer {
            TestPointer::with_kind(pointer, PointerDeviceKind::Touch)
        }

        fn with_kind(pointer: i64, kind: PointerDeviceKind) -> TestPointer {
            TestPointer {
                pointer,
                kind,
                location: Offset::ZERO,
                pan: Offset::ZERO,
            }
        }

        fn down(&mut self, position: Offset) -> PointerDownEvent {
            self.location = position;
            PointerDownEvent {
                pointer: self.pointer,
                kind: self.kind,
                position,
                buttons: K_PRIMARY_BUTTON,
                ..PointerDownEvent::default()
            }
        }

        fn move_to(&mut self, position: Offset) -> PointerMoveEvent {
            let delta = position - self.location;
            self.location = position;
            PointerMoveEvent {
                pointer: self.pointer,
                kind: self.kind,
                position,
                delta,
                down: true,
                buttons: K_PRIMARY_BUTTON,
                ..PointerMoveEvent::default()
            }
        }

        fn up(&mut self) -> PointerUpEvent {
            PointerUpEvent {
                pointer: self.pointer,
                kind: self.kind,
                position: self.location,
                ..PointerUpEvent::default()
            }
        }

        fn pan_zoom_start(&mut self, position: Offset) -> PointerPanZoomStartEvent {
            self.location = position;
            self.pan = Offset::ZERO;
            PointerPanZoomStartEvent {
                pointer: self.pointer,
                kind: self.kind,
                position,
                ..PointerPanZoomStartEvent::default()
            }
        }

        fn pan_zoom_update(
            &mut self,
            position: Offset,
            pan: Offset,
            scale: f64,
            time_stamp: Duration,
        ) -> PointerPanZoomUpdateEvent {
            let pan_delta = pan - self.pan;
            self.location = position;
            self.pan = pan;
            PointerPanZoomUpdateEvent {
                pointer: self.pointer,
                kind: self.kind,
                position,
                pan,
                pan_delta,
                scale,
                time_stamp,
                ..PointerPanZoomUpdateEvent::default()
            }
        }

        fn pan_zoom_end(&mut self, time_stamp: Duration) -> PointerPanZoomEndEvent {
            PointerPanZoomEndEvent {
                pointer: self.pointer,
                kind: self.kind,
                position: self.location,
                time_stamp,
                ..PointerPanZoomEndEvent::default()
            }
        }
    }

    fn close_arena(app: &mut App, pointer: i64) {
        GestureBinding::instance(app)
            .gesture_arena(app)
            .close(app, pointer);
    }

    fn route(app: &mut App, event: PointerEvent) {
        GestureBinding::instance(app)
            .pointer_router(app)
            .route(app, event);
        app.drain_microtasks();
    }

    /// What the callbacks of Flutter's scale tests record, each cleared by the test once
    /// checked.
    #[derive(Default)]
    struct Recorded {
        did_start_scale: Cell<bool>,
        updated_focal_point: Cell<Option<Offset>>,
        updated_pointer_count: Cell<Option<usize>>,
        updated_scale: Cell<Option<f64>>,
        updated_horizontal_scale: Cell<Option<f64>>,
        updated_vertical_scale: Cell<Option<f64>>,
        updated_delta: Cell<Option<Offset>>,
        did_end_scale: Cell<bool>,
        scale_end_velocity: Cell<Option<f64>>,
    }

    fn record(app: &mut App, scale: Handle<ScaleGestureRecognizer>) -> Rc<Recorded> {
        let recorded = Rc::new(Recorded::default());
        let sink = Rc::clone(&recorded);
        scale.set_on_start(
            app,
            Some(Rc::new(move |_app, details: ScaleStartDetails| {
                sink.did_start_scale.set(true);
                sink.updated_focal_point.set(Some(details.focal_point));
                sink.updated_pointer_count.set(Some(details.pointer_count));
            })),
        );
        let sink = Rc::clone(&recorded);
        scale.set_on_update(
            app,
            Some(Rc::new(move |_app, details: ScaleUpdateDetails| {
                sink.updated_scale.set(Some(details.scale));
                sink.updated_horizontal_scale
                    .set(Some(details.horizontal_scale));
                sink.updated_vertical_scale
                    .set(Some(details.vertical_scale));
                sink.updated_focal_point.set(Some(details.focal_point));
                sink.updated_delta.set(Some(details.focal_point_delta));
                sink.updated_pointer_count.set(Some(details.pointer_count));
            })),
        );
        let sink = Rc::clone(&recorded);
        scale.set_on_end(
            app,
            Some(Rc::new(move |_app, details: ScaleEndDetails| {
                sink.did_end_scale.set(true);
                sink.scale_end_velocity.set(Some(details.scale_velocity));
            })),
        );
        recorded
    }

    fn assert_offset_near(actual: Option<Offset>, expected: Offset) {
        let actual = actual.expect("an offset was reported");
        assert!(
            (actual.dx() - expected.dx()).abs() < 1e-9
                && (actual.dy() - expected.dy()).abs() < 1e-9,
            "{actual:?} != {expected:?}"
        );
    }

    #[test]
    fn should_recognize_scale_gestures() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let scale = ScaleGestureRecognizer::new(&mut app);
        let tap = TapGestureRecognizer::new(&mut app);
        let r = record(&mut app, scale);
        let did_tap = Rc::new(Cell::new(false));
        let tapped = Rc::clone(&did_tap);
        tap.set_on_tap(
            &mut app,
            Some(inset_foundation::Listener::new(move |_| tapped.set(true))),
        );

        let mut pointer1 = TestPointer::new(1);
        let down = pointer1.down(Offset::ZERO);
        scale.add_pointer(&mut app, down.clone());
        tap.add_pointer(&mut app, down.clone());

        close_arena(&mut app, 1);
        assert!(!r.did_start_scale.get());
        assert_eq!(r.updated_scale.get(), None);
        assert_eq!(r.updated_focal_point.get(), None);
        assert_eq!(r.updated_delta.get(), None);
        assert_eq!(r.updated_pointer_count.get(), None);
        assert!(!r.did_end_scale.get());
        assert!(!did_tap.get());

        // One-finger panning
        route(&mut app, PointerEvent::Down(down));
        assert!(!r.did_start_scale.get());
        assert_eq!(r.updated_scale.get(), None);
        assert!(!did_tap.get());

        route(
            &mut app,
            PointerEvent::Move(pointer1.move_to(Offset::new(20.0, 30.0))),
        );
        assert!(r.did_start_scale.replace(false));
        assert_offset_near(r.updated_focal_point.take(), Offset::new(20.0, 30.0));
        assert_eq!(r.updated_scale.take(), Some(1.0));
        assert_offset_near(r.updated_delta.take(), Offset::new(20.0, 30.0));
        assert_eq!(r.updated_pointer_count.take(), Some(1));
        assert!(!r.did_end_scale.get());
        assert!(!did_tap.get());

        // Two-finger scaling
        let mut pointer2 = TestPointer::new(2);
        let down2 = pointer2.down(Offset::new(10.0, 20.0));
        scale.add_pointer(&mut app, down2.clone());
        tap.add_pointer(&mut app, down2.clone());
        close_arena(&mut app, 2);
        route(&mut app, PointerEvent::Down(down2));

        assert!(r.did_end_scale.replace(false));
        assert_eq!(r.updated_scale.get(), None);
        assert_eq!(r.updated_focal_point.get(), None);
        assert_eq!(r.updated_delta.get(), None);
        assert_eq!(r.updated_pointer_count.get(), None);
        assert!(!r.did_start_scale.get());

        // Zoom in
        route(
            &mut app,
            PointerEvent::Move(pointer2.move_to(Offset::new(0.0, 10.0))),
        );
        assert!(r.did_start_scale.replace(false));
        assert_offset_near(r.updated_focal_point.take(), Offset::new(10.0, 20.0));
        assert_eq!(r.updated_scale.take(), Some(2.0));
        assert_eq!(r.updated_horizontal_scale.take(), Some(2.0));
        assert_eq!(r.updated_vertical_scale.take(), Some(2.0));
        assert_offset_near(r.updated_delta.take(), Offset::new(-5.0, -5.0));
        assert_eq!(r.updated_pointer_count.take(), Some(2));
        assert!(!r.did_end_scale.get());
        assert!(!did_tap.get());

        // Zoom out
        route(
            &mut app,
            PointerEvent::Move(pointer2.move_to(Offset::new(15.0, 25.0))),
        );
        assert_offset_near(r.updated_focal_point.get(), Offset::new(17.5, 27.5));
        assert_eq!(r.updated_scale.get(), Some(0.5));
        assert_eq!(r.updated_horizontal_scale.get(), Some(0.5));
        assert_eq!(r.updated_vertical_scale.get(), Some(0.5));
        assert_offset_near(r.updated_delta.get(), Offset::new(7.5, 7.5));
        assert_eq!(r.updated_pointer_count.get(), Some(2));
        assert!(!did_tap.get());

        // Horizontal scaling
        route(
            &mut app,
            PointerEvent::Move(pointer2.move_to(Offset::new(0.0, 20.0))),
        );
        assert_eq!(r.updated_horizontal_scale.get(), Some(2.0));
        assert_eq!(r.updated_vertical_scale.get(), Some(1.0));
        assert_eq!(r.updated_pointer_count.get(), Some(2));

        // Vertical scaling
        route(
            &mut app,
            PointerEvent::Move(pointer2.move_to(Offset::new(10.0, 10.0))),
        );
        assert_eq!(r.updated_horizontal_scale.get(), Some(1.0));
        assert_eq!(r.updated_vertical_scale.get(), Some(2.0));
        assert_offset_near(r.updated_delta.get(), Offset::new(5.0, -5.0));
        assert_eq!(r.updated_pointer_count.get(), Some(2));
        route(
            &mut app,
            PointerEvent::Move(pointer2.move_to(Offset::new(15.0, 25.0))),
        );
        r.updated_focal_point.set(None);
        r.updated_scale.set(None);
        r.updated_delta.set(None);
        r.updated_pointer_count.set(None);

        // Three-finger scaling
        let mut pointer3 = TestPointer::new(3);
        let down3 = pointer3.down(Offset::new(25.0, 35.0));
        scale.add_pointer(&mut app, down3.clone());
        tap.add_pointer(&mut app, down3.clone());
        close_arena(&mut app, 3);
        route(&mut app, PointerEvent::Down(down3));

        assert!(r.did_end_scale.replace(false));
        assert_eq!(r.updated_scale.get(), None);
        assert_eq!(r.updated_focal_point.get(), None);
        assert_eq!(r.updated_delta.get(), None);
        assert_eq!(r.updated_pointer_count.get(), None);
        assert!(!r.did_start_scale.get());

        // Zoom in
        route(
            &mut app,
            PointerEvent::Move(pointer3.move_to(Offset::new(55.0, 65.0))),
        );
        assert!(r.did_start_scale.replace(false));
        assert_offset_near(r.updated_focal_point.take(), Offset::new(30.0, 40.0));
        assert_eq!(r.updated_scale.take(), Some(5.0));
        assert_offset_near(r.updated_delta.take(), Offset::new(10.0, 10.0));
        assert_eq!(r.updated_pointer_count.take(), Some(3));
        assert!(!r.did_end_scale.get());
        assert!(!did_tap.get());

        // Return to original positions but with different fingers
        route(
            &mut app,
            PointerEvent::Move(pointer1.move_to(Offset::new(25.0, 35.0))),
        );
        route(
            &mut app,
            PointerEvent::Move(pointer2.move_to(Offset::new(20.0, 30.0))),
        );
        route(
            &mut app,
            PointerEvent::Move(pointer3.move_to(Offset::new(15.0, 25.0))),
        );
        assert!(!r.did_start_scale.get());
        assert_offset_near(r.updated_focal_point.take(), Offset::new(20.0, 30.0));
        let scale_factor = r.updated_scale.take().expect("a scale");
        assert!((scale_factor - 1.0).abs() < 1e-9);
        let delta = r.updated_delta.take().expect("a delta");
        assert!((delta.dx() + 13.3).abs() < 0.1);
        assert!((delta.dy() + 13.3).abs() < 0.1);
        assert_eq!(r.updated_pointer_count.take(), Some(3));
        assert!(!r.did_end_scale.get());
        assert!(!did_tap.get());

        route(&mut app, PointerEvent::Up(pointer1.up()));
        assert!(!r.did_start_scale.get());
        assert_eq!(r.updated_focal_point.get(), None);
        assert_eq!(r.updated_scale.get(), None);
        assert_eq!(r.updated_delta.get(), None);
        assert!(r.did_end_scale.replace(false));
        assert!(!did_tap.get());

        // Continue scaling with two fingers
        route(
            &mut app,
            PointerEvent::Move(pointer3.move_to(Offset::new(10.0, 20.0))),
        );
        assert!(r.did_start_scale.replace(false));
        assert_offset_near(r.updated_focal_point.take(), Offset::new(15.0, 25.0));
        assert_eq!(r.updated_scale.take(), Some(2.0));
        assert_offset_near(r.updated_delta.take(), Offset::new(-2.5, -2.5));
        assert_eq!(r.updated_pointer_count.take(), Some(2));

        // Continue rotating with two fingers
        route(
            &mut app,
            PointerEvent::Move(pointer3.move_to(Offset::new(30.0, 40.0))),
        );
        assert_offset_near(r.updated_focal_point.take(), Offset::new(25.0, 35.0));
        assert_eq!(r.updated_scale.take(), Some(2.0));
        assert_offset_near(r.updated_delta.take(), Offset::new(10.0, 10.0));
        route(
            &mut app,
            PointerEvent::Move(pointer3.move_to(Offset::new(10.0, 20.0))),
        );
        assert_offset_near(r.updated_focal_point.take(), Offset::new(15.0, 25.0));
        assert_eq!(r.updated_scale.take(), Some(2.0));
        assert_offset_near(r.updated_delta.take(), Offset::new(-10.0, -10.0));
        assert_eq!(r.updated_pointer_count.take(), Some(2));

        route(&mut app, PointerEvent::Up(pointer2.up()));
        assert!(!r.did_start_scale.get());
        assert_eq!(r.updated_focal_point.get(), None);
        assert_eq!(r.updated_scale.get(), None);
        assert_eq!(r.updated_delta.get(), None);
        assert!(r.did_end_scale.replace(false));
        assert!(!did_tap.get());

        // Continue panning with one finger
        route(&mut app, PointerEvent::Move(pointer3.move_to(Offset::ZERO)));
        assert!(r.did_start_scale.replace(false));
        assert_offset_near(r.updated_focal_point.take(), Offset::ZERO);
        assert_eq!(r.updated_scale.take(), Some(1.0));
        assert_offset_near(r.updated_delta.take(), Offset::new(-10.0, -20.0));
        assert_eq!(r.updated_pointer_count.take(), Some(1));

        // We are done
        route(&mut app, PointerEvent::Up(pointer3.up()));
        assert!(!r.did_start_scale.get());
        assert_eq!(r.updated_focal_point.get(), None);
        assert_eq!(r.updated_scale.get(), None);
        assert_eq!(r.updated_delta.get(), None);
        assert_eq!(r.updated_pointer_count.get(), None);
        assert!(r.did_end_scale.replace(false));
        assert!(!did_tap.get());

        scale.dispose(&mut app);
        tap.dispose(&mut app);
    }

    #[test]
    fn rejects_scale_gestures_from_unallowed_device_kinds() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let scale = ScaleGestureRecognizer::new(&mut app)
            .supported_devices(&mut app, [PointerDeviceKind::Touch]);
        let r = record(&mut app, scale);

        let mut mouse_pointer = TestPointer::with_kind(1, PointerDeviceKind::Mouse);
        let down = mouse_pointer.down(Offset::ZERO);
        scale.add_pointer(&mut app, down.clone());
        close_arena(&mut app, 1);

        // One-finger panning
        route(&mut app, PointerEvent::Down(down));
        assert!(!r.did_start_scale.get());
        assert_eq!(r.updated_scale.get(), None);

        // Using a mouse, the scale gesture shouldn't even start.
        route(
            &mut app,
            PointerEvent::Move(mouse_pointer.move_to(Offset::new(20.0, 30.0))),
        );
        assert!(!r.did_start_scale.get());
        assert_eq!(r.updated_scale.get(), None);

        scale.dispose(&mut app);
    }

    #[test]
    fn scale_gesture_competes_with_drag() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let scale = ScaleGestureRecognizer::new(&mut app);
        let drag = HorizontalDragGestureRecognizer::new(&mut app);

        let log = Rc::new(RefCell::new(Vec::<&'static str>::new()));
        let sink = Rc::clone(&log);
        scale.set_on_start(
            &mut app,
            Some(Rc::new(move |_app, _details| {
                sink.borrow_mut().push("scale-start")
            })),
        );
        let sink = Rc::clone(&log);
        scale.set_on_update(
            &mut app,
            Some(Rc::new(move |_app, _details| {
                sink.borrow_mut().push("scale-update")
            })),
        );
        let sink = Rc::clone(&log);
        scale.set_on_end(
            &mut app,
            Some(Rc::new(move |_app, _details| {
                sink.borrow_mut().push("scale-end")
            })),
        );
        let sink = Rc::clone(&log);
        drag.set_on_start(
            &mut app,
            Some(Rc::new(move |_app, _details: DragStartDetails| {
                sink.borrow_mut().push("drag-start")
            })),
        );
        let sink = Rc::clone(&log);
        drag.set_on_end(
            &mut app,
            Some(Rc::new(move |_app, _details: DragEndDetails| {
                sink.borrow_mut().push("drag-end")
            })),
        );

        let mut pointer1 = TestPointer::new(1);
        let down = pointer1.down(Offset::new(10.0, 10.0));
        scale.add_pointer(&mut app, down.clone());
        drag.add_pointer(&mut app, down.clone());

        close_arena(&mut app, 1);
        assert!(log.borrow().is_empty());

        // Vertical moves are scales.
        route(&mut app, PointerEvent::Down(down));
        assert!(log.borrow().is_empty());

        // Scale will win if focal point delta exceeds 18.0*2.
        route(
            &mut app,
            PointerEvent::Move(pointer1.move_to(Offset::new(10.0, 50.0))),
        );
        assert_eq!(log.take(), ["scale-start", "scale-update"]);

        let mut pointer2 = TestPointer::new(2);
        let down2 = pointer2.down(Offset::new(10.0, 20.0));
        scale.add_pointer(&mut app, down2.clone());
        drag.add_pointer(&mut app, down2.clone());

        close_arena(&mut app, 2);
        assert!(log.borrow().is_empty());

        // Second pointer joins scale even though it moves horizontally.
        route(&mut app, PointerEvent::Down(down2));
        assert_eq!(log.take(), ["scale-end"]);

        route(
            &mut app,
            PointerEvent::Move(pointer2.move_to(Offset::new(30.0, 20.0))),
        );
        assert_eq!(log.take(), ["scale-start", "scale-update"]);

        route(&mut app, PointerEvent::Up(pointer1.up()));
        assert_eq!(log.take(), ["scale-end"]);

        route(&mut app, PointerEvent::Up(pointer2.up()));
        assert!(log.borrow().is_empty());

        // Horizontal moves are either drags or scales, depending on which wins first.
        // In this case, we move fast, so that the scale wins. If we moved slowly,
        // the horizontal drag would win, since it was added first.
        let mut pointer3 = TestPointer::new(3);
        let down3 = pointer3.down(Offset::new(30.0, 30.0));
        scale.add_pointer(&mut app, down3.clone());
        drag.add_pointer(&mut app, down3.clone());
        close_arena(&mut app, 3);
        route(&mut app, PointerEvent::Down(down3));

        assert!(log.borrow().is_empty());

        route(
            &mut app,
            PointerEvent::Move(pointer3.move_to(Offset::new(100.0, 30.0))),
        );
        assert_eq!(log.take(), ["scale-start", "scale-update"]);

        route(&mut app, PointerEvent::Up(pointer3.up()));
        assert_eq!(log.take(), ["scale-end"]);

        scale.dispose(&mut app);
        DragGestureRecognizer::dispose(drag, &mut app);
    }

    #[test]
    fn should_recognize_scale_gestures_from_pointer_pan_zoom_events() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let scale = ScaleGestureRecognizer::new(&mut app);
        let drag = HorizontalDragGestureRecognizer::new(&mut app);
        let r = record(&mut app, scale);

        let mut pointer1 = TestPointer::with_kind(2, PointerDeviceKind::Trackpad);
        let start = pointer1.pan_zoom_start(Offset::ZERO);
        scale.add_pointer_pan_zoom(&mut app, start.clone());
        drag.add_pointer_pan_zoom(&mut app, start.clone());

        close_arena(&mut app, 2);
        assert!(!r.did_start_scale.get());
        assert_eq!(r.updated_scale.get(), None);
        assert_eq!(r.updated_focal_point.get(), None);
        assert_eq!(r.updated_delta.get(), None);
        assert_eq!(r.updated_pointer_count.get(), None);
        assert!(!r.did_end_scale.get());

        // Panning.
        route(&mut app, PointerEvent::PanZoomStart(start));
        assert!(!r.did_start_scale.get());
        assert_eq!(r.updated_scale.get(), None);
        assert_eq!(r.updated_focal_point.get(), None);
        assert!(!r.did_end_scale.get());

        route(
            &mut app,
            PointerEvent::PanZoomUpdate(pointer1.pan_zoom_update(
                Offset::ZERO,
                Offset::new(20.0, 30.0),
                1.0,
                Duration::ZERO,
            )),
        );
        assert!(r.did_start_scale.replace(false));
        assert_offset_near(r.updated_focal_point.take(), Offset::new(20.0, 30.0));
        assert_eq!(r.updated_scale.take(), Some(1.0));
        assert_offset_near(r.updated_delta.take(), Offset::new(20.0, 30.0));
        assert_eq!(r.updated_pointer_count.take(), Some(2));
        assert!(!r.did_end_scale.get());

        // Zoom in.
        route(
            &mut app,
            PointerEvent::PanZoomUpdate(pointer1.pan_zoom_update(
                Offset::ZERO,
                Offset::new(20.0, 30.0),
                2.0,
                Duration::ZERO,
            )),
        );
        assert_offset_near(r.updated_focal_point.take(), Offset::new(20.0, 30.0));
        assert_eq!(r.updated_scale.take(), Some(2.0));
        assert_eq!(r.updated_horizontal_scale.take(), Some(2.0));
        assert_eq!(r.updated_vertical_scale.take(), Some(2.0));
        assert_offset_near(r.updated_delta.take(), Offset::ZERO);
        assert_eq!(r.updated_pointer_count.take(), Some(2));
        assert!(!r.did_end_scale.get());

        // Zoom out.
        route(
            &mut app,
            PointerEvent::PanZoomUpdate(pointer1.pan_zoom_update(
                Offset::ZERO,
                Offset::new(20.0, 30.0),
                1.0,
                Duration::ZERO,
            )),
        );
        assert_offset_near(r.updated_focal_point.take(), Offset::new(20.0, 30.0));
        assert_eq!(r.updated_scale.take(), Some(1.0));
        assert_eq!(r.updated_horizontal_scale.take(), Some(1.0));
        assert_eq!(r.updated_vertical_scale.take(), Some(1.0));
        assert_offset_near(r.updated_delta.take(), Offset::ZERO);
        assert_eq!(r.updated_pointer_count.take(), Some(2));
        assert!(!r.did_end_scale.get());

        // We are done.
        route(
            &mut app,
            PointerEvent::PanZoomEnd(pointer1.pan_zoom_end(Duration::ZERO)),
        );
        assert!(!r.did_start_scale.get());
        assert_eq!(r.updated_focal_point.get(), None);
        assert_eq!(r.updated_scale.get(), None);
        assert_eq!(r.updated_delta.get(), None);
        assert_eq!(r.updated_pointer_count.get(), None);
        assert!(r.did_end_scale.replace(false));

        scale.dispose(&mut app);
        DragGestureRecognizer::dispose(drag, &mut app);
    }

    #[test]
    fn scale_ending_velocity() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let scale = ScaleGestureRecognizer::new(&mut app);
        scale.set_drag_start_behavior(&mut app, DragStartBehavior::Start);
        scale.set_trackpad_scroll_causes_scale(&mut app, true);
        let r = record(&mut app, scale);
        // Flutter's test listens for the start and the end only.
        scale.set_on_update(&mut app, None);

        let mut pointer1 = TestPointer::with_kind(2, PointerDeviceKind::Trackpad);
        let start = pointer1.pan_zoom_start(Offset::ZERO);
        scale.add_pointer_pan_zoom(&mut app, start.clone());

        close_arena(&mut app, 2);
        assert!(!r.did_start_scale.get());
        assert_eq!(r.updated_focal_point.get(), None);
        assert!(!r.did_end_scale.get());

        route(&mut app, PointerEvent::PanZoomStart(start));
        assert!(r.did_start_scale.replace(false));
        assert_offset_near(r.updated_focal_point.take(), Offset::ZERO);
        assert!(!r.did_end_scale.get());

        // Zoom in by scrolling up.
        for i in 0..100_u64 {
            route(
                &mut app,
                PointerEvent::PanZoomUpdate(pointer1.pan_zoom_update(
                    Offset::ZERO,
                    Offset::new(0.0, i as f64 * -10.0),
                    1.0,
                    Duration::from_millis(i * 25),
                )),
            );
        }

        // End.
        route(
            &mut app,
            PointerEvent::PanZoomEnd(pointer1.pan_zoom_end(Duration::from_millis(2500))),
        );
        assert!(!r.did_start_scale.get());
        assert_eq!(r.updated_focal_point.get(), None);
        assert!(r.did_end_scale.replace(false));
        let velocity = r.scale_end_velocity.get().expect("an end velocity");
        assert!(
            (velocity - 281.414_540_980_277_65).abs() < 1e-6,
            "{velocity}"
        );

        scale.dispose(&mut app);
    }
}
