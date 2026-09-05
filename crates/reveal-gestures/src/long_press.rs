//! Flutter counterpart: `gestures/long_press.dart`.

use std::collections::HashSet;
use std::rc::Rc;
use std::time::Duration;

use reveal_embedder::{Offset, PointerDeviceKind};
use reveal_foundation::{App, Handle, Listener, ValueChanged};

use crate::arena::GestureDisposition;
use crate::constants::K_LONG_PRESS_TIMEOUT;
use crate::events::{
    K_PRIMARY_BUTTON, K_SECONDARY_BUTTON, K_TERTIARY_BUTTON, PointerDownEvent, PointerEvent,
};
use crate::gesture_details::PositionedGestureDetails;
use crate::gesture_settings::DeviceGestureSettings;
use crate::recognizer::{
    GestureRecognizer, GestureRecognizerData, GestureRecognizerState, OffsetPair, OneSequenceData,
    OneSequenceGestureRecognizer, PrimaryPointerData, PrimaryPointerGestureRecognizer,
    PrimaryPointerLeaf, PrimaryPointerLeafData, RecognizerLeaf, RecognizerLeafData,
    UNSET_TOUCH_SLOP,
};
use crate::team::GestureArenaTeam;
use crate::velocity::Velocity;
use crate::velocity_tracker::VelocityTracker;

/// Details for callbacks that use `GestureLongPressDownCallback`.
pub struct LongPressDownDetails {
    /// The global position at which the pointer contacted the screen.
    pub global_position: Offset,

    /// The local position at which the pointer contacted the screen.
    pub local_position: Offset,

    /// The kind of the device that initiated the event.
    pub kind: Option<PointerDeviceKind>,
}

impl LongPressDownDetails {
    /// Creates the details for a long-press-down callback.
    ///
    /// If `local_position` is none, it defaults to the global position.
    pub fn new(
        global_position: Offset,
        local_position: Option<Offset>,
        kind: Option<PointerDeviceKind>,
    ) -> LongPressDownDetails {
        LongPressDownDetails {
            global_position,
            local_position: local_position.unwrap_or(global_position),
            kind,
        }
    }
}

impl Default for LongPressDownDetails {
    fn default() -> LongPressDownDetails {
        LongPressDownDetails::new(Offset::ZERO, None, None)
    }
}

impl PositionedGestureDetails for LongPressDownDetails {
    fn global_position(&self) -> Offset {
        self.global_position
    }

    fn local_position(&self) -> Offset {
        self.local_position
    }
}

/// Details for callbacks that use `GestureLongPressStartCallback`.
pub struct LongPressStartDetails {
    /// The global position at which the pointer contacted the screen.
    pub global_position: Offset,

    /// The local position at which the pointer contacted the screen.
    pub local_position: Offset,
}

impl LongPressStartDetails {
    /// Creates the details for a long-press-start callback.
    pub fn new(global_position: Offset, local_position: Option<Offset>) -> LongPressStartDetails {
        LongPressStartDetails {
            global_position,
            local_position: local_position.unwrap_or(global_position),
        }
    }
}

impl Default for LongPressStartDetails {
    fn default() -> LongPressStartDetails {
        LongPressStartDetails::new(Offset::ZERO, None)
    }
}

impl PositionedGestureDetails for LongPressStartDetails {
    fn global_position(&self) -> Offset {
        self.global_position
    }

    fn local_position(&self) -> Offset {
        self.local_position
    }
}

/// Details for callbacks that use `GestureLongPressMoveUpdateCallback`.
pub struct LongPressMoveUpdateDetails {
    /// The global position at which the pointer contacted the screen.
    pub global_position: Offset,

    /// The local position at which the pointer contacted the screen.
    pub local_position: Offset,

    /// A delta offset from the point where the long press drag initially contacted
    /// the screen to the point where the pointer is currently located.
    pub offset_from_origin: Offset,

    /// A local delta offset from the initial contact point to the current
    /// [`local_position`](Self::local_position).
    pub local_offset_from_origin: Offset,
}

impl LongPressMoveUpdateDetails {
    /// Creates the details for a long-press-move callback.
    pub fn new(
        global_position: Offset,
        local_position: Option<Offset>,
        offset_from_origin: Offset,
        local_offset_from_origin: Option<Offset>,
    ) -> LongPressMoveUpdateDetails {
        LongPressMoveUpdateDetails {
            global_position,
            local_position: local_position.unwrap_or(global_position),
            offset_from_origin,
            local_offset_from_origin: local_offset_from_origin.unwrap_or(offset_from_origin),
        }
    }
}

impl Default for LongPressMoveUpdateDetails {
    fn default() -> LongPressMoveUpdateDetails {
        LongPressMoveUpdateDetails::new(Offset::ZERO, None, Offset::ZERO, None)
    }
}

impl PositionedGestureDetails for LongPressMoveUpdateDetails {
    fn global_position(&self) -> Offset {
        self.global_position
    }

    fn local_position(&self) -> Offset {
        self.local_position
    }
}

/// Details for callbacks that use `GestureLongPressEndCallback`.
pub struct LongPressEndDetails {
    /// The global position at which the pointer contacted the screen.
    pub global_position: Offset,

    /// The local position at which the pointer contacted the screen.
    pub local_position: Offset,

    /// The pointer's velocity when it stopped contacting the screen.
    pub velocity: Velocity,
}

impl LongPressEndDetails {
    /// Creates the details for a long-press-end callback.
    pub fn new(
        global_position: Offset,
        local_position: Option<Offset>,
        velocity: Velocity,
    ) -> LongPressEndDetails {
        LongPressEndDetails {
            global_position,
            local_position: local_position.unwrap_or(global_position),
            velocity,
        }
    }
}

impl Default for LongPressEndDetails {
    fn default() -> LongPressEndDetails {
        LongPressEndDetails::new(Offset::ZERO, None, Velocity::ZERO)
    }
}

impl PositionedGestureDetails for LongPressEndDetails {
    fn global_position(&self) -> Offset {
        self.global_position
    }

    fn local_position(&self) -> Offset {
        self.local_position
    }
}

/// Signature for [`LongPressGestureRecognizer::set_on_long_press_down`].
pub type GestureLongPressDownCallback = ValueChanged<LongPressDownDetails>;

/// Signature for [`LongPressGestureRecognizer::set_on_long_press_cancel`].
pub type GestureLongPressCancelCallback = Listener;

/// Signature for [`LongPressGestureRecognizer::set_on_long_press`].
pub type GestureLongPressCallback = Listener;

/// Signature for [`LongPressGestureRecognizer::set_on_long_press_up`].
pub type GestureLongPressUpCallback = Listener;

/// Signature for [`LongPressGestureRecognizer::set_on_long_press_start`].
pub type GestureLongPressStartCallback = ValueChanged<LongPressStartDetails>;

/// Signature for [`LongPressGestureRecognizer::set_on_long_press_move_update`].
pub type GestureLongPressMoveUpdateCallback = ValueChanged<LongPressMoveUpdateDetails>;

/// Signature for [`LongPressGestureRecognizer::set_on_long_press_end`].
pub type GestureLongPressEndCallback = ValueChanged<LongPressEndDetails>;

fn default_button_accept_behavior(buttons: i64) -> bool {
    buttons == K_PRIMARY_BUTTON || buttons == K_SECONDARY_BUTTON || buttons == K_TERTIARY_BUTTON
}

/// Recognizes when the user has pressed down at the same location for a long
/// period of time.
///
/// The gesture must not deviate in position from its touch down point for 500ms
/// until it's recognized. Once the gesture is accepted, the finger can be
/// moved, triggering `on_long_press_move_update` callbacks, unless the
/// [`post_accept_slop_tolerance`](Self::post_accept_slop_tolerance) constructor
/// argument is specified.
///
/// [`LongPressGestureRecognizer`] may compete on pointer events of
/// [`K_PRIMARY_BUTTON`], [`K_SECONDARY_BUTTON`], and/or [`K_TERTIARY_BUTTON`] if
/// at least one corresponding callback is non-null. If it has no callbacks, it
/// is a no-op.
pub struct LongPressGestureRecognizer {
    recognizer: GestureRecognizerData,
    one_sequence: OneSequenceData,
    primary: PrimaryPointerData,
    long_press_accepted: bool,
    long_press_origin: Option<OffsetPair>,
    initial_buttons: Option<i64>,
    on_long_press_down: Option<GestureLongPressDownCallback>,
    on_long_press_cancel: Option<GestureLongPressCancelCallback>,
    on_long_press: Option<GestureLongPressCallback>,
    on_long_press_start: Option<GestureLongPressStartCallback>,
    on_long_press_move_update: Option<GestureLongPressMoveUpdateCallback>,
    on_long_press_up: Option<GestureLongPressUpCallback>,
    on_long_press_end: Option<GestureLongPressEndCallback>,
    on_secondary_long_press_down: Option<GestureLongPressDownCallback>,
    on_secondary_long_press_cancel: Option<GestureLongPressCancelCallback>,
    on_secondary_long_press: Option<GestureLongPressCallback>,
    on_secondary_long_press_start: Option<GestureLongPressStartCallback>,
    on_secondary_long_press_move_update: Option<GestureLongPressMoveUpdateCallback>,
    on_secondary_long_press_up: Option<GestureLongPressUpCallback>,
    on_secondary_long_press_end: Option<GestureLongPressEndCallback>,
    on_tertiary_long_press_down: Option<GestureLongPressDownCallback>,
    on_tertiary_long_press_cancel: Option<GestureLongPressCancelCallback>,
    on_tertiary_long_press: Option<GestureLongPressCallback>,
    on_tertiary_long_press_start: Option<GestureLongPressStartCallback>,
    on_tertiary_long_press_move_update: Option<GestureLongPressMoveUpdateCallback>,
    on_tertiary_long_press_up: Option<GestureLongPressUpCallback>,
    on_tertiary_long_press_end: Option<GestureLongPressEndCallback>,
    velocity_tracker: Option<VelocityTracker>,
}

impl LongPressGestureRecognizer {
    /// Creates a long-press gesture recognizer.
    ///
    /// Consider assigning the [`set_on_long_press_start`](Self::set_on_long_press_start)
    /// callback after creating this object.
    ///
    /// [`post_accept_slop_tolerance`](Self::post_accept_slop_tolerance) defaults
    /// to none: the gesture can be moved without limit once the long press is
    /// accepted.
    pub fn new(app: &mut App) -> Handle<LongPressGestureRecognizer> {
        let mut recognizer = GestureRecognizerData::new();
        recognizer.allowed_buttons_filter = Rc::new(default_button_accept_behavior);
        app.create(LongPressGestureRecognizer {
            recognizer,
            one_sequence: OneSequenceData::new(),
            primary: PrimaryPointerData::new(
                Some(K_LONG_PRESS_TIMEOUT),
                Some(UNSET_TOUCH_SLOP),
                None,
            ),
            long_press_accepted: false,
            long_press_origin: None,
            initial_buttons: None,
            on_long_press_down: None,
            on_long_press_cancel: None,
            on_long_press: None,
            on_long_press_start: None,
            on_long_press_move_update: None,
            on_long_press_up: None,
            on_long_press_end: None,
            on_secondary_long_press_down: None,
            on_secondary_long_press_cancel: None,
            on_secondary_long_press: None,
            on_secondary_long_press_start: None,
            on_secondary_long_press_move_update: None,
            on_secondary_long_press_up: None,
            on_secondary_long_press_end: None,
            on_tertiary_long_press_down: None,
            on_tertiary_long_press_cancel: None,
            on_tertiary_long_press: None,
            on_tertiary_long_press_start: None,
            on_tertiary_long_press_move_update: None,
            on_tertiary_long_press_up: None,
            on_tertiary_long_press_end: None,
            velocity_tracker: None,
        })
    }

    /// Overwrites the default duration after which the long press will be
    /// recognized.
    pub fn duration(
        self: Handle<Self>,
        app: &mut App,
        duration: Duration,
    ) -> Handle<LongPressGestureRecognizer> {
        app.get_mut(self).primary.deadline = Some(duration);
        self
    }

    /// The maximum distance the gesture may drift after it is accepted.
    ///
    /// `None` means unlimited.
    pub fn post_accept_slop_tolerance(
        self: Handle<Self>,
        app: &mut App,
        value: Option<f64>,
    ) -> Handle<LongPressGestureRecognizer> {
        app.get_mut(self).primary.post_accept_slop_tolerance = value;
        self
    }

    /// The kind of devices that are allowed to be recognized.
    pub fn supported_devices(
        self: Handle<Self>,
        app: &mut App,
        devices: impl IntoIterator<Item = PointerDeviceKind>,
    ) -> Handle<LongPressGestureRecognizer> {
        app.get_mut(self).recognizer.supported_devices = Some(devices.into_iter().collect());
        self
    }

    /// Sets the kind of devices that are allowed to be recognized; `None` tracks and
    /// recognizes events from all device kinds.
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
    ) -> Handle<LongPressGestureRecognizer> {
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

    /// A pointer has contacted the screen at a particular location with a
    /// primary button, which might be the start of a long-press.
    pub fn set_on_long_press_down(
        self: Handle<Self>,
        app: &mut App,
        callback: Option<GestureLongPressDownCallback>,
    ) {
        app.get_mut(self).on_long_press_down = callback;
    }

    /// A pointer that previously triggered [`set_on_long_press_down`](Self::set_on_long_press_down)
    /// will not end up causing a long-press.
    pub fn set_on_long_press_cancel(
        self: Handle<Self>,
        app: &mut App,
        callback: Option<GestureLongPressCancelCallback>,
    ) {
        app.get_mut(self).on_long_press_cancel = callback;
    }

    /// A long press gesture by a primary button has been recognized.
    pub fn set_on_long_press(
        self: Handle<Self>,
        app: &mut App,
        callback: Option<GestureLongPressCallback>,
    ) {
        app.get_mut(self).on_long_press = callback;
    }

    /// A long press gesture by a primary button has been recognized.
    pub fn set_on_long_press_start(
        self: Handle<Self>,
        app: &mut App,
        callback: Option<GestureLongPressStartCallback>,
    ) {
        app.get_mut(self).on_long_press_start = callback;
    }

    /// Moving after the long press by a primary button is recognized.
    pub fn set_on_long_press_move_update(
        self: Handle<Self>,
        app: &mut App,
        callback: Option<GestureLongPressMoveUpdateCallback>,
    ) {
        app.get_mut(self).on_long_press_move_update = callback;
    }

    /// The pointer stopped contacting the screen after a long-press by a
    /// primary button.
    pub fn set_on_long_press_up(
        self: Handle<Self>,
        app: &mut App,
        callback: Option<GestureLongPressUpCallback>,
    ) {
        app.get_mut(self).on_long_press_up = callback;
    }

    /// The pointer stopped contacting the screen after a long-press by a
    /// primary button.
    pub fn set_on_long_press_end(
        self: Handle<Self>,
        app: &mut App,
        callback: Option<GestureLongPressEndCallback>,
    ) {
        app.get_mut(self).on_long_press_end = callback;
    }

    /// A pointer has contacted the screen with a secondary button, which might
    /// be the start of a long-press.
    pub fn set_on_secondary_long_press_down(
        self: Handle<Self>,
        app: &mut App,
        callback: Option<GestureLongPressDownCallback>,
    ) {
        app.get_mut(self).on_secondary_long_press_down = callback;
    }

    /// A pointer that previously triggered
    /// [`set_on_secondary_long_press_down`](Self::set_on_secondary_long_press_down)
    /// will not end up causing a long-press.
    pub fn set_on_secondary_long_press_cancel(
        self: Handle<Self>,
        app: &mut App,
        callback: Option<GestureLongPressCancelCallback>,
    ) {
        app.get_mut(self).on_secondary_long_press_cancel = callback;
    }

    /// A long press gesture by a secondary button has been recognized.
    pub fn set_on_secondary_long_press(
        self: Handle<Self>,
        app: &mut App,
        callback: Option<GestureLongPressCallback>,
    ) {
        app.get_mut(self).on_secondary_long_press = callback;
    }

    /// A long press gesture by a secondary button has been recognized.
    pub fn set_on_secondary_long_press_start(
        self: Handle<Self>,
        app: &mut App,
        callback: Option<GestureLongPressStartCallback>,
    ) {
        app.get_mut(self).on_secondary_long_press_start = callback;
    }

    /// Moving after the long press by a secondary button is recognized.
    pub fn set_on_secondary_long_press_move_update(
        self: Handle<Self>,
        app: &mut App,
        callback: Option<GestureLongPressMoveUpdateCallback>,
    ) {
        app.get_mut(self).on_secondary_long_press_move_update = callback;
    }

    /// The pointer stopped contacting the screen after a long-press by a
    /// secondary button.
    pub fn set_on_secondary_long_press_up(
        self: Handle<Self>,
        app: &mut App,
        callback: Option<GestureLongPressUpCallback>,
    ) {
        app.get_mut(self).on_secondary_long_press_up = callback;
    }

    /// The pointer stopped contacting the screen after a long-press by a
    /// secondary button.
    pub fn set_on_secondary_long_press_end(
        self: Handle<Self>,
        app: &mut App,
        callback: Option<GestureLongPressEndCallback>,
    ) {
        app.get_mut(self).on_secondary_long_press_end = callback;
    }

    /// A pointer has contacted the screen with a tertiary button, which might
    /// be the start of a long-press.
    pub fn set_on_tertiary_long_press_down(
        self: Handle<Self>,
        app: &mut App,
        callback: Option<GestureLongPressDownCallback>,
    ) {
        app.get_mut(self).on_tertiary_long_press_down = callback;
    }

    /// A pointer that previously triggered
    /// [`set_on_tertiary_long_press_down`](Self::set_on_tertiary_long_press_down)
    /// will not end up causing a long-press.
    pub fn set_on_tertiary_long_press_cancel(
        self: Handle<Self>,
        app: &mut App,
        callback: Option<GestureLongPressCancelCallback>,
    ) {
        app.get_mut(self).on_tertiary_long_press_cancel = callback;
    }

    /// A long press gesture by a tertiary button has been recognized.
    pub fn set_on_tertiary_long_press(
        self: Handle<Self>,
        app: &mut App,
        callback: Option<GestureLongPressCallback>,
    ) {
        app.get_mut(self).on_tertiary_long_press = callback;
    }

    /// A long press gesture by a tertiary button has been recognized.
    pub fn set_on_tertiary_long_press_start(
        self: Handle<Self>,
        app: &mut App,
        callback: Option<GestureLongPressStartCallback>,
    ) {
        app.get_mut(self).on_tertiary_long_press_start = callback;
    }

    /// Moving after the long press by a tertiary button is recognized.
    pub fn set_on_tertiary_long_press_move_update(
        self: Handle<Self>,
        app: &mut App,
        callback: Option<GestureLongPressMoveUpdateCallback>,
    ) {
        app.get_mut(self).on_tertiary_long_press_move_update = callback;
    }

    /// The pointer stopped contacting the screen after a long-press by a
    /// tertiary button.
    pub fn set_on_tertiary_long_press_up(
        self: Handle<Self>,
        app: &mut App,
        callback: Option<GestureLongPressUpCallback>,
    ) {
        app.get_mut(self).on_tertiary_long_press_up = callback;
    }

    /// The pointer stopped contacting the screen after a long-press by a
    /// tertiary button.
    pub fn set_on_tertiary_long_press_end(
        self: Handle<Self>,
        app: &mut App,
        callback: Option<GestureLongPressEndCallback>,
    ) {
        app.get_mut(self).on_tertiary_long_press_end = callback;
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
        event: crate::events::PointerPanZoomStartEvent,
    ) {
        GestureRecognizer::add_pointer_pan_zoom(self, app, event);
    }

    /// Releases any resources used by the object.
    pub fn dispose(self: Handle<Self>, app: &mut App) {
        <Self as RecognizerLeaf>::dispose(self, app);
    }

    /// Returns a very short pretty description of the gesture that the
    /// recognizer looks for.
    pub fn debug_description(self: Handle<Self>) -> &'static str {
        <Self as RecognizerLeaf>::debug_description(self)
    }

    fn check_long_press_down(self: Handle<Self>, app: &mut App, event: &PointerDownEvent) {
        debug_assert!(app.get(self).long_press_origin.is_some());
        let origin = app.get(self).long_press_origin.unwrap();
        let details = LongPressDownDetails::new(
            origin.global,
            Some(origin.local),
            Some(GestureRecognizer::kind_for_pointer(
                self,
                app,
                event.pointer,
            )),
        );
        let initial_buttons = app.get(self).initial_buttons;
        match initial_buttons {
            Some(K_PRIMARY_BUTTON) => {
                if let Some(callback) = app.get(self).on_long_press_down.clone() {
                    GestureRecognizer::invoke_callback(self, app, "onLongPressDown", |app| {
                        callback(app, details)
                    });
                }
            }
            Some(K_SECONDARY_BUTTON) => {
                if let Some(callback) = app.get(self).on_secondary_long_press_down.clone() {
                    GestureRecognizer::invoke_callback(
                        self,
                        app,
                        "onSecondaryLongPressDown",
                        |app| callback(app, details),
                    );
                }
            }
            Some(K_TERTIARY_BUTTON) => {
                if let Some(callback) = app.get(self).on_tertiary_long_press_down.clone() {
                    GestureRecognizer::invoke_callback(
                        self,
                        app,
                        "onTertiaryLongPressDown",
                        |app| callback(app, details),
                    );
                }
            }
            _ => debug_assert!(false, "Unhandled button {initial_buttons:?}"),
        }
    }

    fn check_long_press_cancel(self: Handle<Self>, app: &mut App) {
        if app.get(self).primary.state == GestureRecognizerState::Possible {
            let initial_buttons = app.get(self).initial_buttons;
            match initial_buttons {
                Some(K_PRIMARY_BUTTON) => {
                    if let Some(callback) = app.get(self).on_long_press_cancel.clone() {
                        GestureRecognizer::invoke_callback(self, app, "onLongPressCancel", |app| {
                            callback.call(app)
                        });
                    }
                }
                Some(K_SECONDARY_BUTTON) => {
                    if let Some(callback) = app.get(self).on_secondary_long_press_cancel.clone() {
                        GestureRecognizer::invoke_callback(
                            self,
                            app,
                            "onSecondaryLongPressCancel",
                            |app| callback.call(app),
                        );
                    }
                }
                Some(K_TERTIARY_BUTTON) => {
                    if let Some(callback) = app.get(self).on_tertiary_long_press_cancel.clone() {
                        GestureRecognizer::invoke_callback(
                            self,
                            app,
                            "onTertiaryLongPressCancel",
                            |app| callback.call(app),
                        );
                    }
                }
                _ => debug_assert!(false, "Unhandled button {initial_buttons:?}"),
            }
        }
    }

    fn check_long_press_start(self: Handle<Self>, app: &mut App) {
        let origin = app.get(self).long_press_origin.unwrap();
        let initial_buttons = app.get(self).initial_buttons;
        match initial_buttons {
            Some(K_PRIMARY_BUTTON) => {
                if app.get(self).on_long_press_start.is_some() {
                    let details = LongPressStartDetails::new(origin.global, Some(origin.local));
                    let callback = app.get(self).on_long_press_start.clone().unwrap();
                    GestureRecognizer::invoke_callback(self, app, "onLongPressStart", |app| {
                        callback(app, details)
                    });
                }
                if let Some(callback) = app.get(self).on_long_press.clone() {
                    GestureRecognizer::invoke_callback(self, app, "onLongPress", |app| {
                        callback.call(app)
                    });
                }
            }
            Some(K_SECONDARY_BUTTON) => {
                if app.get(self).on_secondary_long_press_start.is_some() {
                    let details = LongPressStartDetails::new(origin.global, Some(origin.local));
                    let callback = app.get(self).on_secondary_long_press_start.clone().unwrap();
                    GestureRecognizer::invoke_callback(
                        self,
                        app,
                        "onSecondaryLongPressStart",
                        |app| callback(app, details),
                    );
                }
                if let Some(callback) = app.get(self).on_secondary_long_press.clone() {
                    GestureRecognizer::invoke_callback(self, app, "onSecondaryLongPress", |app| {
                        callback.call(app)
                    });
                }
            }
            Some(K_TERTIARY_BUTTON) => {
                if app.get(self).on_tertiary_long_press_start.is_some() {
                    let details = LongPressStartDetails::new(origin.global, Some(origin.local));
                    let callback = app.get(self).on_tertiary_long_press_start.clone().unwrap();
                    GestureRecognizer::invoke_callback(
                        self,
                        app,
                        "onTertiaryLongPressStart",
                        |app| callback(app, details),
                    );
                }
                if let Some(callback) = app.get(self).on_tertiary_long_press.clone() {
                    GestureRecognizer::invoke_callback(self, app, "onTertiaryLongPress", |app| {
                        callback.call(app)
                    });
                }
            }
            _ => debug_assert!(false, "Unhandled button {initial_buttons:?}"),
        }
    }

    fn check_long_press_move_update(self: Handle<Self>, app: &mut App, event: &PointerEvent) {
        let origin = app.get(self).long_press_origin.unwrap();
        let details = LongPressMoveUpdateDetails::new(
            event.position(),
            Some(event.local_position()),
            event.position() - origin.global,
            Some(event.local_position() - origin.local),
        );
        let initial_buttons = app.get(self).initial_buttons;
        match initial_buttons {
            Some(K_PRIMARY_BUTTON) => {
                if let Some(callback) = app.get(self).on_long_press_move_update.clone() {
                    GestureRecognizer::invoke_callback(self, app, "onLongPressMoveUpdate", |app| {
                        callback(app, details)
                    });
                }
            }
            Some(K_SECONDARY_BUTTON) => {
                if let Some(callback) = app.get(self).on_secondary_long_press_move_update.clone() {
                    GestureRecognizer::invoke_callback(
                        self,
                        app,
                        "onSecondaryLongPressMoveUpdate",
                        |app| callback(app, details),
                    );
                }
            }
            Some(K_TERTIARY_BUTTON) => {
                if let Some(callback) = app.get(self).on_tertiary_long_press_move_update.clone() {
                    GestureRecognizer::invoke_callback(
                        self,
                        app,
                        "onTertiaryLongPressMoveUpdate",
                        |app| callback(app, details),
                    );
                }
            }
            _ => debug_assert!(false, "Unhandled button {initial_buttons:?}"),
        }
    }

    fn check_long_press_end(self: Handle<Self>, app: &mut App, event: &PointerEvent) {
        let estimate = app
            .get(self)
            .velocity_tracker
            .as_ref()
            .and_then(VelocityTracker::get_velocity_estimate);
        let velocity = match estimate {
            None => Velocity::ZERO,
            Some(estimate) => Velocity::new(estimate.pixels_per_second),
        };
        let details =
            LongPressEndDetails::new(event.position(), Some(event.local_position()), velocity);
        app.get_mut(self).velocity_tracker = None;
        let initial_buttons = app.get(self).initial_buttons;
        match initial_buttons {
            Some(K_PRIMARY_BUTTON) => {
                if let Some(callback) = app.get(self).on_long_press_end.clone() {
                    GestureRecognizer::invoke_callback(self, app, "onLongPressEnd", |app| {
                        callback(app, details)
                    });
                }
                if let Some(callback) = app.get(self).on_long_press_up.clone() {
                    GestureRecognizer::invoke_callback(self, app, "onLongPressUp", |app| {
                        callback.call(app)
                    });
                }
            }
            Some(K_SECONDARY_BUTTON) => {
                if let Some(callback) = app.get(self).on_secondary_long_press_end.clone() {
                    GestureRecognizer::invoke_callback(
                        self,
                        app,
                        "onSecondaryLongPressEnd",
                        |app| callback(app, details),
                    );
                }
                if let Some(callback) = app.get(self).on_secondary_long_press_up.clone() {
                    GestureRecognizer::invoke_callback(
                        self,
                        app,
                        "onSecondaryLongPressUp",
                        |app| callback.call(app),
                    );
                }
            }
            Some(K_TERTIARY_BUTTON) => {
                if let Some(callback) = app.get(self).on_tertiary_long_press_end.clone() {
                    GestureRecognizer::invoke_callback(
                        self,
                        app,
                        "onTertiaryLongPressEnd",
                        |app| callback(app, details),
                    );
                }
                if let Some(callback) = app.get(self).on_tertiary_long_press_up.clone() {
                    GestureRecognizer::invoke_callback(self, app, "onTertiaryLongPressUp", |app| {
                        callback.call(app)
                    });
                }
            }
            _ => debug_assert!(false, "Unhandled button {initial_buttons:?}"),
        }
    }

    fn reset(self: Handle<Self>, app: &mut App) {
        let recognizer = app.get_mut(self);
        recognizer.long_press_accepted = false;
        recognizer.long_press_origin = None;
        recognizer.initial_buttons = None;
        recognizer.velocity_tracker = None;
    }
}

impl RecognizerLeafData for LongPressGestureRecognizer {
    fn recognizer(&self) -> &GestureRecognizerData {
        &self.recognizer
    }
    fn recognizer_mut(&mut self) -> &mut GestureRecognizerData {
        &mut self.recognizer
    }
    fn one_sequence(&self) -> &OneSequenceData {
        &self.one_sequence
    }
    fn one_sequence_mut(&mut self) -> &mut OneSequenceData {
        &mut self.one_sequence
    }
}

impl PrimaryPointerLeafData for LongPressGestureRecognizer {
    fn primary(&self) -> &PrimaryPointerData {
        &self.primary
    }
    fn primary_mut(&mut self) -> &mut PrimaryPointerData {
        &mut self.primary
    }
}

impl RecognizerLeaf for LongPressGestureRecognizer {
    fn add_allowed_pointer(self: Handle<Self>, app: &mut App, event: PointerDownEvent) {
        PrimaryPointerGestureRecognizer::add_allowed_pointer(self, app, event);
    }

    fn handle_non_allowed_pointer(self: Handle<Self>, app: &mut App, event: &PointerDownEvent) {
        PrimaryPointerGestureRecognizer::handle_non_allowed_pointer(self, app, event);
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

    fn reject_gesture(self: Handle<Self>, app: &mut App, pointer: i64) {
        PrimaryPointerGestureRecognizer::reject_gesture(self, app, pointer);
    }

    fn is_pointer_allowed(self: Handle<Self>, app: &App, event: &PointerDownEvent) -> bool {
        let recognizer = app.get(self);
        let allowed = match event.buttons {
            K_PRIMARY_BUTTON => {
                recognizer.on_long_press_down.is_some()
                    || recognizer.on_long_press_cancel.is_some()
                    || recognizer.on_long_press_start.is_some()
                    || recognizer.on_long_press.is_some()
                    || recognizer.on_long_press_move_update.is_some()
                    || recognizer.on_long_press_end.is_some()
                    || recognizer.on_long_press_up.is_some()
            }
            K_SECONDARY_BUTTON => {
                recognizer.on_secondary_long_press_down.is_some()
                    || recognizer.on_secondary_long_press_cancel.is_some()
                    || recognizer.on_secondary_long_press_start.is_some()
                    || recognizer.on_secondary_long_press.is_some()
                    || recognizer.on_secondary_long_press_move_update.is_some()
                    || recognizer.on_secondary_long_press_end.is_some()
                    || recognizer.on_secondary_long_press_up.is_some()
            }
            K_TERTIARY_BUTTON => {
                recognizer.on_tertiary_long_press_down.is_some()
                    || recognizer.on_tertiary_long_press_cancel.is_some()
                    || recognizer.on_tertiary_long_press_start.is_some()
                    || recognizer.on_tertiary_long_press.is_some()
                    || recognizer.on_tertiary_long_press_move_update.is_some()
                    || recognizer.on_tertiary_long_press_end.is_some()
                    || recognizer.on_tertiary_long_press_up.is_some()
            }
            _ => false,
        };
        allowed && GestureRecognizer::is_pointer_allowed(self, app, event)
    }

    fn resolve(self: Handle<Self>, app: &mut App, disposition: GestureDisposition) {
        if disposition == GestureDisposition::Rejected {
            if app.get(self).long_press_accepted {
                self.reset(app);
            } else {
                self.check_long_press_cancel(app);
            }
        }
        OneSequenceGestureRecognizer::resolve(self, app, disposition);
    }

    fn accept_gesture(self: Handle<Self>, _app: &mut App, _pointer: i64) {
        // Winning the arena isn't important here since it may happen from a sweep.
        // Explicitly exceeding the deadline puts the gesture in accepted state.
    }

    fn debug_description(self: Handle<Self>) -> &'static str {
        "long press"
    }
}

impl PrimaryPointerLeaf for LongPressGestureRecognizer {
    fn did_exceed_deadline(self: Handle<Self>, app: &mut App) {
        self.resolve(app, GestureDisposition::Accepted);
        app.get_mut(self).long_press_accepted = true;
        let pointer = app.get(self).primary.primary_pointer.unwrap();
        PrimaryPointerGestureRecognizer::accept_gesture(self, app, pointer);
        self.check_long_press_start(app);
    }

    fn handle_primary_pointer(self: Handle<Self>, app: &mut App, event: PointerEvent) {
        if !event.synthesized() {
            if let PointerEvent::Down(down) = &event {
                app.get_mut(self).velocity_tracker = Some(VelocityTracker::with_kind(down.kind));
                app.get_mut(self)
                    .velocity_tracker
                    .as_mut()
                    .unwrap()
                    .add_position(down.time_stamp, down.local_position());
            }
            if let PointerEvent::Move(moved) = &event {
                debug_assert!(app.get(self).velocity_tracker.is_some());
                app.get_mut(self)
                    .velocity_tracker
                    .as_mut()
                    .unwrap()
                    .add_position(moved.time_stamp, moved.local_position());
            }
        }

        if matches!(event, PointerEvent::Up(_)) {
            if app.get(self).long_press_accepted {
                self.check_long_press_end(app, &event);
            } else {
                self.resolve(app, GestureDisposition::Rejected);
            }
            self.reset(app);
        } else if matches!(event, PointerEvent::Cancel(_)) {
            self.check_long_press_cancel(app);
            self.reset(app);
        } else if let PointerEvent::Down(down) = &event {
            app.get_mut(self).long_press_origin = Some(OffsetPair::from_event_position(&event));
            app.get_mut(self).initial_buttons = Some(down.buttons);
            self.check_long_press_down(app, down);
        } else if let PointerEvent::Move(moved) = &event {
            let initial = app.get(self).initial_buttons;
            let accepted = app.get(self).long_press_accepted;
            if Some(moved.buttons) != initial && !accepted {
                self.resolve(app, GestureDisposition::Rejected);
                let primary = app.get(self).primary.primary_pointer.unwrap();
                OneSequenceGestureRecognizer::stop_tracking_pointer(self, app, primary);
            } else if accepted {
                self.check_long_press_move_update(app, &event);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reveal_foundation::AppCell;
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::time::Duration;

    use crate::binding::GestureBinding;
    use crate::events::{PointerMoveEvent, PointerUpEvent};
    use crate::tap::TapGestureRecognizer;

    fn down(pointer: i64, position: Offset) -> PointerDownEvent {
        PointerDownEvent {
            pointer,
            position,
            buttons: K_PRIMARY_BUTTON,
            ..PointerDownEvent::default()
        }
    }

    fn up(pointer: i64, position: Offset) -> PointerUpEvent {
        PointerUpEvent {
            pointer,
            position,
            buttons: K_PRIMARY_BUTTON,
            ..PointerUpEvent::default()
        }
    }

    fn move_event(pointer: i64, position: Offset) -> PointerMoveEvent {
        PointerMoveEvent {
            pointer,
            position,
            down: true,
            buttons: K_PRIMARY_BUTTON,
            ..PointerMoveEvent::default()
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

    fn set_handlers(
        gesture: Handle<LongPressGestureRecognizer>,
        app: &mut App,
        log: &Rc<RefCell<Vec<&'static str>>>,
    ) {
        let down_log = Rc::clone(log);
        gesture.set_on_long_press_down(
            app,
            Some(Rc::new(move |_app, _details| {
                down_log.borrow_mut().push("down");
            })),
        );
        let cancel_log = Rc::clone(log);
        gesture.set_on_long_press_cancel(
            app,
            Some(Listener::new(move |_app| {
                cancel_log.borrow_mut().push("cancel");
            })),
        );
        let start_log = Rc::clone(log);
        gesture.set_on_long_press(
            app,
            Some(Listener::new(move |_app| {
                start_log.borrow_mut().push("start");
            })),
        );
        let move_log = Rc::clone(log);
        gesture.set_on_long_press_move_update(
            app,
            Some(Rc::new(move |_app, _details| {
                move_log.borrow_mut().push("move");
            })),
        );
        let end_log = Rc::clone(log);
        gesture.set_on_long_press_up(
            app,
            Some(Listener::new(move |_app| {
                end_log.borrow_mut().push("end");
            })),
        );
    }

    #[test]
    fn local_position_defaults_to_global() {
        let down = LongPressDownDetails::new(Offset::new(3.0, 4.0), None, None);
        assert_eq!(down.local_position, Offset::new(3.0, 4.0));
        let start = LongPressStartDetails::new(Offset::new(1.0, 2.0), None);
        assert_eq!(start.local_position, Offset::new(1.0, 2.0));
        let moved = LongPressMoveUpdateDetails::new(
            Offset::new(5.0, 6.0),
            None,
            Offset::new(1.0, 0.0),
            None,
        );
        assert_eq!(moved.local_position, Offset::new(5.0, 6.0));
        assert_eq!(moved.local_offset_from_origin, Offset::new(1.0, 0.0));
        let end = LongPressEndDetails::new(Offset::new(7.0, 8.0), None, Velocity::ZERO);
        assert_eq!(end.local_position, Offset::new(7.0, 8.0));
    }

    #[test]
    fn should_recognize_long_press() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let gesture = LongPressGestureRecognizer::new(&mut app);
        let log = Rc::new(RefCell::new(Vec::new()));
        set_handlers(gesture, &mut app, &log);

        let down1 = down(5, Offset::new(10.0, 10.0));
        gesture.add_pointer(&mut app, down1.clone());
        close_arena(&mut app, 5);
        assert!(log.borrow().is_empty());
        route(&mut app, PointerEvent::Down(down1));
        assert_eq!(*log.borrow(), ["down"]);
        drop(app);
        cell.elapse(Duration::from_millis(300));
        assert_eq!(*log.borrow(), ["down"]);
        cell.elapse(Duration::from_millis(700));
        let mut app = cell.borrow_mut();
        assert_eq!(*log.borrow(), ["down", "start"]);
        gesture.dispose(&mut app);
        assert_eq!(*log.borrow(), ["down", "start"]);
    }

    #[test]
    fn should_recognize_long_press_with_altered_duration() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let gesture = LongPressGestureRecognizer::new(&mut app)
            .duration(&mut app, Duration::from_millis(100));
        let log = Rc::new(RefCell::new(Vec::new()));
        set_handlers(gesture, &mut app, &log);

        let down1 = down(5, Offset::new(10.0, 10.0));
        gesture.add_pointer(&mut app, down1.clone());
        close_arena(&mut app, 5);
        route(&mut app, PointerEvent::Down(down1));
        assert_eq!(*log.borrow(), ["down"]);
        drop(app);
        cell.elapse(Duration::from_millis(50));
        assert_eq!(*log.borrow(), ["down"]);
        cell.elapse(Duration::from_millis(50));
        let mut app = cell.borrow_mut();
        assert_eq!(*log.borrow(), ["down", "start"]);
        gesture.dispose(&mut app);
        assert_eq!(*log.borrow(), ["down", "start"]);
    }

    #[test]
    fn up_cancels_long_press() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let gesture = LongPressGestureRecognizer::new(&mut app);
        let log = Rc::new(RefCell::new(Vec::new()));
        set_handlers(gesture, &mut app, &log);

        let down1 = down(5, Offset::new(10.0, 10.0));
        gesture.add_pointer(&mut app, down1.clone());
        close_arena(&mut app, 5);
        route(&mut app, PointerEvent::Down(down1));
        assert_eq!(*log.borrow(), ["down"]);
        drop(app);
        cell.elapse(Duration::from_millis(300));
        let mut app = cell.borrow_mut();
        assert_eq!(*log.borrow(), ["down"]);
        route(&mut app, PointerEvent::Up(up(5, Offset::new(11.0, 9.0))));
        assert_eq!(*log.borrow(), ["down", "cancel"]);
        drop(app);
        cell.elapse(Duration::from_secs(1));
        let mut app = cell.borrow_mut();
        gesture.dispose(&mut app);
        assert_eq!(*log.borrow(), ["down", "cancel"]);
    }

    #[test]
    fn moving_before_accept_cancels() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let gesture = LongPressGestureRecognizer::new(&mut app);
        let log = Rc::new(RefCell::new(Vec::new()));
        set_handlers(gesture, &mut app, &log);

        let down1 = down(5, Offset::new(10.0, 10.0));
        gesture.add_pointer(&mut app, down1.clone());
        close_arena(&mut app, 5);
        route(&mut app, PointerEvent::Down(down1));
        assert_eq!(*log.borrow(), ["down"]);
        drop(app);
        cell.elapse(Duration::from_millis(300));
        let mut app = cell.borrow_mut();
        assert_eq!(*log.borrow(), ["down"]);
        route(
            &mut app,
            PointerEvent::Move(move_event(5, Offset::new(100.0, 200.0))),
        );
        assert_eq!(*log.borrow(), ["down", "cancel"]);
        drop(app);
        cell.elapse(Duration::from_secs(1));
        let mut app = cell.borrow_mut();
        route(&mut app, PointerEvent::Up(up(5, Offset::new(100.0, 200.0))));
        drop(app);
        cell.elapse(Duration::from_millis(300));
        let mut app = cell.borrow_mut();
        assert_eq!(*log.borrow(), ["down", "cancel"]);
        gesture.dispose(&mut app);
        assert_eq!(*log.borrow(), ["down", "cancel"]);
    }

    #[test]
    fn moving_after_accept_is_ok() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let gesture = LongPressGestureRecognizer::new(&mut app);
        let log = Rc::new(RefCell::new(Vec::new()));
        set_handlers(gesture, &mut app, &log);

        let down1 = down(5, Offset::new(10.0, 10.0));
        gesture.add_pointer(&mut app, down1.clone());
        close_arena(&mut app, 5);
        route(&mut app, PointerEvent::Down(down1));
        assert_eq!(*log.borrow(), ["down"]);
        drop(app);
        cell.elapse(Duration::from_secs(1));
        let mut app = cell.borrow_mut();
        assert_eq!(*log.borrow(), ["down", "start"]);
        route(
            &mut app,
            PointerEvent::Move(move_event(5, Offset::new(100.0, 200.0))),
        );
        assert_eq!(*log.borrow(), ["down", "start", "move"]);
        route(&mut app, PointerEvent::Up(up(5, Offset::new(11.0, 9.0))));
        assert_eq!(*log.borrow(), ["down", "start", "move", "end"]);
        drop(app);
        cell.elapse(Duration::from_millis(300));
        let mut app = cell.borrow_mut();
        assert_eq!(*log.borrow(), ["down", "start", "move", "end"]);
        gesture.dispose(&mut app);
        assert_eq!(*log.borrow(), ["down", "start", "move", "end"]);
    }

    #[test]
    fn should_recognize_both_tap_down_and_long_press() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let gesture = LongPressGestureRecognizer::new(&mut app);
        let tap = TapGestureRecognizer::new(&mut app);
        let log = Rc::new(RefCell::new(Vec::new()));
        set_handlers(gesture, &mut app, &log);
        let tap_log = Rc::clone(&log);
        tap.set_on_tap_down(
            &mut app,
            Some(Rc::new(move |_app, _details| {
                tap_log.borrow_mut().push("tap_down");
            })),
        );

        let down1 = down(5, Offset::new(10.0, 10.0));
        tap.add_pointer(&mut app, down1.clone());
        gesture.add_pointer(&mut app, down1.clone());
        close_arena(&mut app, 5);
        assert!(log.borrow().is_empty());
        route(&mut app, PointerEvent::Down(down1));
        assert_eq!(*log.borrow(), ["down"]);
        drop(app);
        cell.elapse(Duration::from_millis(300));
        assert_eq!(*log.borrow(), ["down", "tap_down"]);
        cell.elapse(Duration::from_millis(700));
        let mut app = cell.borrow_mut();
        assert_eq!(*log.borrow(), ["down", "tap_down", "start"]);
        tap.dispose(&mut app);
        gesture.dispose(&mut app);
        assert_eq!(*log.borrow(), ["down", "tap_down", "start"]);
    }

    #[test]
    fn should_recognize_long_press_up() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let gesture = LongPressGestureRecognizer::new(&mut app);
        let log = Rc::new(RefCell::new(Vec::new()));
        set_handlers(gesture, &mut app, &log);

        let down1 = down(5, Offset::new(10.0, 10.0));
        gesture.add_pointer(&mut app, down1.clone());
        close_arena(&mut app, 5);
        route(&mut app, PointerEvent::Down(down1));
        assert_eq!(*log.borrow(), ["down"]);
        drop(app);
        cell.elapse(Duration::from_millis(300));
        assert_eq!(*log.borrow(), ["down"]);
        cell.elapse(Duration::from_millis(700));
        let mut app = cell.borrow_mut();
        assert_eq!(*log.borrow(), ["down", "start"]);
        route(&mut app, PointerEvent::Up(up(5, Offset::new(11.0, 9.0))));
        assert_eq!(*log.borrow(), ["down", "start", "end"]);
        gesture.dispose(&mut app);
        assert_eq!(*log.borrow(), ["down", "start", "end"]);
    }
}
