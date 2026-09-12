//! Flutter counterpart: `gestures/force_press.dart`.
//!
//! Leaf object. Superclass bags and `super` namespaces are in `recognizer.rs`.

use std::collections::HashSet;
use std::fmt::{self, Debug};
use std::rc::Rc;

use inset_embedder::{Offset, PointerDeviceKind, clamp_double};
use inset_foundation::{App, Handle, ValueChanged};

use crate::arena::GestureDisposition;
use crate::events::{PointerDownEvent, PointerEvent, compute_hit_slop};
use crate::gesture_details::PositionedGestureDetails;
use crate::gesture_settings::DeviceGestureSettings;
use crate::recognizer::{
    GestureRecognizer, GestureRecognizerData, OffsetPair, OneSequenceData,
    OneSequenceGestureRecognizer, OneSequenceLeafData, RecognizerLeaf, RecognizerLeafData,
};
use crate::team::GestureArenaTeam;

enum ForceState {
    /// No pointer has touched down and the detector is ready for a pointer down to occur.
    Ready,
    /// A pointer has touched down, but a force press gesture has not yet been detected.
    Possible,
    /// A pointer is down and a force press gesture has been detected. However, if
    /// the [`ForcePressGestureRecognizer`] is the only recognizer in the arena, thus
    /// accepted as soon as the gesture state is possible, the gesture will not
    /// yet have started.
    Accepted,
    /// A pointer is down and the gesture has started, ie. the pressure of the pointer
    /// has just become greater than the [`ForcePressGestureRecognizer::start_pressure`].
    Started,
    /// A pointer is down and the pressure of the pointer has just become greater
    /// than the [`ForcePressGestureRecognizer::peak_pressure`]. Even after a pointer
    /// crosses this threshold, onUpdate callbacks will still be sent.
    Peaked,
}

/// Details object for callbacks that use [`GestureForcePressStartCallback`],
/// [`GestureForcePressPeakCallback`], [`GestureForcePressEndCallback`] or
/// [`GestureForcePressUpdateCallback`].
///
/// See also:
///
///  * [`ForcePressGestureRecognizer::set_on_start`], [`ForcePressGestureRecognizer::set_on_peak`],
///    [`ForcePressGestureRecognizer::set_on_end`], and [`ForcePressGestureRecognizer::set_on_update`]
///    which use [`ForcePressDetails`].
#[derive(Clone)]
pub struct ForcePressDetails {
    /// The global position at which the pointer contacted the screen.
    pub global_position: Offset,

    /// The local position at which the pointer contacted the screen.
    pub local_position: Offset,

    /// The pressure of the pointer on the screen.
    pub pressure: f64,
}

impl ForcePressDetails {
    /// Creates details for a [`GestureForcePressStartCallback`],
    /// [`GestureForcePressPeakCallback`] or [`GestureForcePressEndCallback`].
    pub fn new(
        global_position: Offset,
        local_position: Option<Offset>,
        pressure: f64,
    ) -> ForcePressDetails {
        ForcePressDetails {
            global_position,
            local_position: local_position.unwrap_or(global_position),
            pressure,
        }
    }
}

impl PositionedGestureDetails for ForcePressDetails {
    fn global_position(&self) -> Offset {
        self.global_position
    }

    fn local_position(&self) -> Offset {
        self.local_position
    }
}

impl Debug for ForcePressDetails {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ForcePressDetails")
            .field("globalPosition", &self.global_position)
            .field("localPosition", &self.local_position)
            .field("pressure", &self.pressure)
            .finish()
    }
}

/// Signature used by a [`ForcePressGestureRecognizer`] for when a pointer has
/// pressed with at least [`ForcePressGestureRecognizer::start_pressure`].
pub type GestureForcePressStartCallback = ValueChanged<ForcePressDetails>;

/// Signature used by [`ForcePressGestureRecognizer`] for when a pointer that has
/// pressed with at least [`ForcePressGestureRecognizer::peak_pressure`].
pub type GestureForcePressPeakCallback = ValueChanged<ForcePressDetails>;

/// Signature used by [`ForcePressGestureRecognizer`] during the frames
/// after the triggering of a [`ForcePressGestureRecognizer::set_on_start`] callback.
pub type GestureForcePressUpdateCallback = ValueChanged<ForcePressDetails>;

/// Signature for when the pointer that previously triggered a
/// [`ForcePressGestureRecognizer::set_on_start`] callback is no longer in contact
/// with the screen.
pub type GestureForcePressEndCallback = ValueChanged<ForcePressDetails>;

/// Signature used by [`ForcePressGestureRecognizer`] for interpolating the raw
/// device pressure to a value in the range `[0, 1]` given the device's pressure
/// min and pressure max.
pub type GestureForceInterpolation = Rc<dyn Fn(f64, f64, f64) -> f64>;

/// Recognizes a force press on devices that have force sensors.
///
/// Only the force from a single pointer is used to invoke events. A tap
/// recognizer will win against this recognizer on pointer up as long as the
/// pointer has not pressed with a force greater than
/// [`ForcePressGestureRecognizer::start_pressure`]. A long press recognizer will
/// win when the press down time exceeds the threshold time as long as the
/// pointer's pressure was never greater than
/// [`ForcePressGestureRecognizer::start_pressure`] in that duration.
///
/// As of November, 2018 iPhone devices of generation 6S and higher have
/// force touch functionality, with the exception of the iPhone XR. In addition,
/// a small handful of Android devices have this functionality as well.
///
/// Devices with faux screen pressure sensors like the Pixel 2 and 3 will not
/// send any force press related callbacks.
///
/// Reported pressure will always be in the range 0.0 to 1.0, where 1.0 is
/// maximum pressure and 0.0 is minimum pressure. If using a custom
/// [`interpolation`](Self::interpolation) callback, the pressure reported will correspond to that
/// custom curve.
pub struct ForcePressGestureRecognizer {
    recognizer: GestureRecognizerData,
    one_sequence: OneSequenceData,
    on_start: Option<GestureForcePressStartCallback>,
    on_update: Option<GestureForcePressUpdateCallback>,
    on_peak: Option<GestureForcePressPeakCallback>,
    on_end: Option<GestureForcePressEndCallback>,
    start_pressure: f64,
    peak_pressure: f64,
    interpolation: GestureForceInterpolation,
    last_position: OffsetPair,
    last_pressure: f64,
    state: ForceState,
}

impl ForcePressGestureRecognizer {
    /// Creates a force press gesture recognizer.
    ///
    /// The [`start_pressure`](Self::start_pressure) defaults to 0.4, and [`peak_pressure`](Self::peak_pressure) defaults to 0.85
    /// where a value of 0.0 is no pressure and a value of 1.0 is maximum pressure.
    ///
    /// The [`peak_pressure`](Self::peak_pressure) argument must be greater than [`start_pressure`](Self::start_pressure).
    /// The [`interpolation`](Self::interpolation) callback must always return a value in the range 0.0
    /// to 1.0 for values of `pressure` that are between `pressure_min` and
    /// `pressure_max`.
    pub fn new(app: &mut App) -> Handle<ForcePressGestureRecognizer> {
        // Dart: `assert(peakPressure > startPressure)` on the constructor defaults.
        #[allow(clippy::assertions_on_constants)]
        {
            debug_assert!(0.85 > 0.4);
        }
        app.create(ForcePressGestureRecognizer {
            recognizer: GestureRecognizerData::new(),
            one_sequence: OneSequenceData::new(),
            on_start: None,
            on_update: None,
            on_peak: None,
            on_end: None,
            start_pressure: 0.4,
            peak_pressure: 0.85,
            interpolation: Rc::new(inverse_lerp),
            last_position: OffsetPair::ZERO,
            last_pressure: 0.0,
            state: ForceState::Ready,
        })
    }

    /// The kind of devices that are allowed to be recognized.
    ///
    /// If none, events from all device kinds will be tracked and recognized.
    pub fn supported_devices(
        self: Handle<Self>,
        app: &mut App,
        devices: impl IntoIterator<Item = PointerDeviceKind>,
    ) -> Handle<ForcePressGestureRecognizer> {
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
    ) -> Handle<ForcePressGestureRecognizer> {
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

    /// Releases any resources used by the object.
    pub fn dispose(self: Handle<Self>, app: &mut App) {
        <Self as RecognizerLeaf>::dispose(self, app);
    }

    /// A pointer is in contact with the screen and has just pressed with a force
    /// exceeding the [`start_pressure`](Self::start_pressure). Consequently, if there were other gesture
    /// detectors, only the force press gesture will be detected and all others
    /// will be rejected.
    ///
    /// The position of the pointer is provided in the callback's `details`
    /// argument, which is a [`ForcePressDetails`] object.
    pub fn set_on_start(
        self: Handle<Self>,
        app: &mut App,
        callback: Option<GestureForcePressStartCallback>,
    ) {
        app.get_mut(self).on_start = callback;
    }

    /// A pointer is in contact with the screen and is either moving on the plane
    /// of the screen, pressing the screen with varying forces or both
    /// simultaneously.
    ///
    /// This callback will be invoked for every pointer event after the invocation
    /// of [`set_on_start`](Self::set_on_start) and/or [`set_on_peak`](Self::set_on_peak) and before the invocation of [`set_on_end`](Self::set_on_end), no
    /// matter what the pressure is during this time period. The position and
    /// pressure of the pointer is provided in the callback's `details` argument,
    /// which is a [`ForcePressDetails`] object.
    pub fn set_on_update(
        self: Handle<Self>,
        app: &mut App,
        callback: Option<GestureForcePressUpdateCallback>,
    ) {
        app.get_mut(self).on_update = callback;
    }

    /// A pointer is in contact with the screen and has just pressed with a force
    /// exceeding the [`peak_pressure`](Self::peak_pressure). This is an arbitrary second level action
    /// threshold and isn't necessarily the maximum possible device pressure
    /// (which is 1.0).
    ///
    /// The position of the pointer is provided in the callback's `details`
    /// argument, which is a [`ForcePressDetails`] object.
    pub fn set_on_peak(
        self: Handle<Self>,
        app: &mut App,
        callback: Option<GestureForcePressPeakCallback>,
    ) {
        app.get_mut(self).on_peak = callback;
    }

    /// A pointer is no longer in contact with the screen.
    ///
    /// The position of the pointer is provided in the callback's `details`
    /// argument, which is a [`ForcePressDetails`] object.
    pub fn set_on_end(
        self: Handle<Self>,
        app: &mut App,
        callback: Option<GestureForcePressEndCallback>,
    ) {
        app.get_mut(self).on_end = callback;
    }

    /// The pressure of the press required to initiate a force press.
    ///
    /// A value of 0.0 is no pressure, and 1.0 is maximum pressure.
    pub fn start_pressure(self: Handle<Self>, app: &App) -> f64 {
        app.get(self).start_pressure
    }

    /// Sets [`start_pressure`](Self::start_pressure).
    pub fn set_start_pressure(self: Handle<Self>, app: &mut App, value: f64) {
        debug_assert!(app.get(self).peak_pressure > value);
        app.get_mut(self).start_pressure = value;
    }

    /// The pressure of the press required to peak a force press.
    ///
    /// A value of 0.0 is no pressure, and 1.0 is maximum pressure. This value
    /// must be greater than [`start_pressure`](Self::start_pressure).
    pub fn peak_pressure(self: Handle<Self>, app: &App) -> f64 {
        app.get(self).peak_pressure
    }

    /// Sets [`peak_pressure`](Self::peak_pressure).
    pub fn set_peak_pressure(self: Handle<Self>, app: &mut App, value: f64) {
        debug_assert!(value > app.get(self).start_pressure);
        app.get_mut(self).peak_pressure = value;
    }

    /// The function used to convert the raw device pressure values into a value
    /// in the range 0.0 to 1.0.
    ///
    /// The function takes in the device's minimum, maximum and raw touch pressure
    /// and returns a value in the range 0.0 to 1.0 denoting the interpolated
    /// touch pressure.
    ///
    /// This function must always return values in the range 0.0 to 1.0 given a
    /// pressure that is between the minimum and maximum pressures. It may return
    /// `f64::NAN` for values that it does not want to support.
    ///
    /// By default, the function is a linear interpolation; however, changing the
    /// function could be useful to accommodate variations in the way different
    /// devices respond to pressure, or to change how animations from pressure
    /// feedback are rendered.
    pub fn interpolation(self: Handle<Self>, app: &App) -> GestureForceInterpolation {
        Rc::clone(&app.get(self).interpolation)
    }

    /// Sets [`interpolation`](Self::interpolation).
    pub fn set_interpolation(self: Handle<Self>, app: &mut App, value: GestureForceInterpolation) {
        app.get_mut(self).interpolation = value;
    }
}

impl RecognizerLeafData for ForcePressGestureRecognizer {
    fn recognizer(&self) -> &GestureRecognizerData {
        &self.recognizer
    }
    fn recognizer_mut(&mut self) -> &mut GestureRecognizerData {
        &mut self.recognizer
    }
}

impl OneSequenceLeafData for ForcePressGestureRecognizer {
    fn one_sequence(&self) -> &OneSequenceData {
        &self.one_sequence
    }
    fn one_sequence_mut(&mut self) -> &mut OneSequenceData {
        &mut self.one_sequence
    }
}

impl RecognizerLeaf for ForcePressGestureRecognizer {
    fn add_allowed_pointer(self: Handle<Self>, app: &mut App, event: PointerDownEvent) {
        // If the device has a maximum pressure of less than or equal to 1, it
        // doesn't have touch pressure sensing capabilities. Do not participate
        // in the gesture arena.
        if event.pressure_max <= 1.0 {
            self.resolve(app, GestureDisposition::Rejected);
        } else {
            OneSequenceGestureRecognizer::add_allowed_pointer(self, app, &event);
            if matches!(app.get(self).state, ForceState::Ready) {
                app.get_mut(self).state = ForceState::Possible;
                app.get_mut(self).last_position =
                    OffsetPair::from_event_position(&PointerEvent::Down(event));
            }
        }
    }

    fn handle_non_allowed_pointer(self: Handle<Self>, app: &mut App, _event: &PointerDownEvent) {
        OneSequenceGestureRecognizer::handle_non_allowed_pointer(self, app);
    }

    fn handle_event(self: Handle<Self>, app: &mut App, event: PointerEvent) {
        debug_assert!(!matches!(app.get(self).state, ForceState::Ready));
        // A static pointer with changes in pressure creates PointerMoveEvent events.
        if matches!(event, PointerEvent::Move(_) | PointerEvent::Down(_)) {
            let interpolation = Rc::clone(&app.get(self).interpolation);
            let pressure =
                interpolation(event.pressure_min(), event.pressure_max(), event.pressure());
            debug_assert!((0.0..=1.0).contains(&pressure) || pressure.is_nan());

            app.get_mut(self).last_position = OffsetPair::from_event_position(&event);
            app.get_mut(self).last_pressure = pressure;

            if matches!(app.get(self).state, ForceState::Possible) {
                if pressure > app.get(self).start_pressure {
                    app.get_mut(self).state = ForceState::Started;
                    self.resolve(app, GestureDisposition::Accepted);
                } else if event.delta().distance_squared()
                    > compute_hit_slop(event.kind(), app.get(self).recognizer.gesture_settings)
                {
                    self.resolve(app, GestureDisposition::Rejected);
                }
            }
            // In case this is the only gesture detector we still don't want to start
            // the gesture until the pressure is greater than the startPressure.
            if pressure > app.get(self).start_pressure
                && matches!(app.get(self).state, ForceState::Accepted)
            {
                app.get_mut(self).state = ForceState::Started;
                if let Some(callback) = app.get(self).on_start.clone() {
                    let last_position = app.get(self).last_position;
                    GestureRecognizer::invoke_callback(self, app, "onStart", |app| {
                        callback(
                            app,
                            ForcePressDetails::new(
                                last_position.global,
                                Some(last_position.local),
                                pressure,
                            ),
                        );
                    });
                }
            }
            if pressure > app.get(self).peak_pressure
                && matches!(app.get(self).state, ForceState::Started)
                && let Some(callback) = app.get(self).on_peak.clone()
            {
                app.get_mut(self).state = ForceState::Peaked;
                GestureRecognizer::invoke_callback(self, app, "onPeak", |app| {
                    callback(
                        app,
                        ForcePressDetails::new(
                            event.position(),
                            Some(event.local_position()),
                            pressure,
                        ),
                    );
                });
            }
            if !pressure.is_nan()
                && matches!(
                    app.get(self).state,
                    ForceState::Started | ForceState::Peaked
                )
                && let Some(callback) = app.get(self).on_update.clone()
            {
                GestureRecognizer::invoke_callback(self, app, "onUpdate", |app| {
                    callback(
                        app,
                        ForcePressDetails::new(
                            event.position(),
                            Some(event.local_position()),
                            pressure,
                        ),
                    );
                });
            }
        }
        OneSequenceGestureRecognizer::stop_tracking_if_pointer_no_longer_down(self, app, &event);
    }

    fn accept_gesture(self: Handle<Self>, app: &mut App, _pointer: i64) {
        if matches!(app.get(self).state, ForceState::Possible) {
            app.get_mut(self).state = ForceState::Accepted;
        }

        if app.get(self).on_start.is_some() && matches!(app.get(self).state, ForceState::Started) {
            let callback = app.get(self).on_start.clone().unwrap();
            let last_pressure = app.get(self).last_pressure;
            let last_position = app.get(self).last_position;
            GestureRecognizer::invoke_callback(self, app, "onStart", |app| {
                callback(
                    app,
                    ForcePressDetails::new(
                        last_position.global,
                        Some(last_position.local),
                        last_pressure,
                    ),
                );
            });
        }
    }

    fn did_stop_tracking_last_pointer(self: Handle<Self>, app: &mut App, _pointer: i64) {
        let was_accepted = matches!(
            app.get(self).state,
            ForceState::Started | ForceState::Peaked
        );
        if matches!(app.get(self).state, ForceState::Possible) {
            self.resolve(app, GestureDisposition::Rejected);
            return;
        }
        if was_accepted && let Some(callback) = app.get(self).on_end.clone() {
            let last_position = app.get(self).last_position;
            GestureRecognizer::invoke_callback(self, app, "onEnd", |app| {
                callback(
                    app,
                    ForcePressDetails::new(last_position.global, Some(last_position.local), 0.0),
                );
            });
        }
        app.get_mut(self).state = ForceState::Ready;
    }

    fn reject_gesture(self: Handle<Self>, app: &mut App, pointer: i64) {
        OneSequenceGestureRecognizer::stop_tracking_pointer(self, app, pointer);
        self.did_stop_tracking_last_pointer(app, pointer);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        OneSequenceGestureRecognizer::dispose(self, app);
    }

    fn debug_description(self: Handle<Self>) -> &'static str {
        "force press"
    }
}

fn inverse_lerp(min: f64, max: f64, t: f64) -> f64 {
    debug_assert!(min <= max);
    let mut value = (t - min) / (max - min);

    // If the device incorrectly reports a pressure outside of pressureMin
    // and pressureMax, we still want this recognizer to respond normally.
    if !value.is_nan() {
        value = clamp_double(value, 0.0, 1.0);
    }
    value
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use inset_foundation::AppCell;

    use super::*;
    use crate::binding::GestureBinding;
    use crate::events::{PointerMoveEvent, PointerUpEvent};

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

    fn move_at(
        pointer: i64,
        position: Offset,
        pressure: f64,
        pressure_min: f64,
        pressure_max: f64,
    ) -> PointerMoveEvent {
        PointerMoveEvent {
            pointer,
            position,
            pressure,
            pressure_min,
            pressure_max,
            down: true,
            ..PointerMoveEvent::default()
        }
    }

    #[test]
    fn a_force_press_can_be_recognized() {
        const PRESSURE_MIN: f64 = 0.0;
        const PRESSURE_MAX: f64 = 6.66;

        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let force = ForcePressGestureRecognizer::new(&mut app);

        let started = Rc::new(Cell::new(0));
        let peaked = Rc::new(Cell::new(0));
        let updated = Rc::new(Cell::new(0));
        let ended = Rc::new(Cell::new(0));
        let start_global_position = Rc::new(Cell::new(None));

        let started_c = Rc::clone(&started);
        let pos_c = Rc::clone(&start_global_position);
        force.set_on_start(
            &mut app,
            Some(Rc::new(move |_app, details: ForcePressDetails| {
                pos_c.set(Some(details.global_position));
                started_c.set(started_c.get() + 1);
            })),
        );
        let peaked_c = Rc::clone(&peaked);
        force.set_on_peak(
            &mut app,
            Some(Rc::new(move |_app, _details| {
                peaked_c.set(peaked_c.get() + 1);
            })),
        );
        let updated_c = Rc::clone(&updated);
        force.set_on_update(
            &mut app,
            Some(Rc::new(move |_app, _details| {
                updated_c.set(updated_c.get() + 1);
            })),
        );
        let ended_c = Rc::clone(&ended);
        force.set_on_end(
            &mut app,
            Some(Rc::new(move |_app, _details| {
                ended_c.set(ended_c.get() + 1);
            })),
        );

        let pointer_value = 1;
        let down = PointerDownEvent {
            pointer: pointer_value,
            position: Offset::new(10.0, 10.0),
            pressure: 0.0,
            pressure_min: PRESSURE_MIN,
            pressure_max: PRESSURE_MAX,
            ..PointerDownEvent::default()
        };
        force.add_pointer(&mut app, down.clone());
        close_arena(&mut app, pointer_value);

        assert_eq!(started.get(), 0);
        assert_eq!(peaked.get(), 0);
        assert_eq!(updated.get(), 0);
        assert_eq!(ended.get(), 0);

        route(
            &mut app,
            PointerEvent::Move(move_at(
                pointer_value,
                Offset::new(10.0, 10.0),
                2.5,
                PRESSURE_MIN,
                PRESSURE_MAX,
            )),
        );
        assert_eq!(started.get(), 0);
        assert_eq!(peaked.get(), 0);
        assert_eq!(updated.get(), 0);
        assert_eq!(ended.get(), 0);

        route(
            &mut app,
            PointerEvent::Move(move_at(
                pointer_value,
                Offset::new(10.0, 10.0),
                2.8,
                PRESSURE_MIN,
                PRESSURE_MAX,
            )),
        );
        assert_eq!(started.get(), 1);
        assert_eq!(peaked.get(), 0);
        assert_eq!(updated.get(), 1);
        assert_eq!(ended.get(), 0);

        route(
            &mut app,
            PointerEvent::Move(move_at(
                pointer_value,
                Offset::new(10.0, 10.0),
                3.3,
                PRESSURE_MIN,
                PRESSURE_MAX,
            )),
        );
        route(
            &mut app,
            PointerEvent::Move(move_at(
                pointer_value,
                Offset::new(10.0, 10.0),
                4.0,
                PRESSURE_MIN,
                PRESSURE_MAX,
            )),
        );
        route(
            &mut app,
            PointerEvent::Move(move_at(
                pointer_value,
                Offset::new(10.0, 10.0),
                5.0,
                PRESSURE_MIN,
                PRESSURE_MAX,
            )),
        );
        route(
            &mut app,
            PointerEvent::Move(move_at(
                pointer_value,
                Offset::new(10.0, 10.0),
                1.0,
                PRESSURE_MIN,
                PRESSURE_MAX,
            )),
        );
        assert_eq!(started.get(), 1);
        assert_eq!(updated.get(), 5);
        assert_eq!(peaked.get(), 0);
        assert_eq!(ended.get(), 0);
        assert_eq!(start_global_position.get(), Some(Offset::new(10.0, 10.0)));

        route(
            &mut app,
            PointerEvent::Move(move_at(
                pointer_value,
                Offset::new(10.0, 10.0),
                6.0,
                PRESSURE_MIN,
                PRESSURE_MAX,
            )),
        );
        assert_eq!(started.get(), 1);
        assert_eq!(updated.get(), 6);
        assert_eq!(peaked.get(), 1);
        assert_eq!(ended.get(), 0);

        route(
            &mut app,
            PointerEvent::Up(PointerUpEvent {
                pointer: pointer_value,
                position: Offset::new(10.0, 10.0),
                pressure_min: PRESSURE_MIN,
                pressure_max: PRESSURE_MAX,
                ..PointerUpEvent::default()
            }),
        );
        assert_eq!(started.get(), 1);
        assert_eq!(updated.get(), 6);
        assert_eq!(peaked.get(), 1);
        assert_eq!(ended.get(), 1);

        force.dispose(&mut app);
    }
}
