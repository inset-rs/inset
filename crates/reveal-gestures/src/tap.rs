//! Flutter counterpart: `gestures/tap.dart`.
//!
//! Leaf Handle. Superclass bags and `super` namespaces are in [`recognizer`](crate::recognizer).

use std::fmt::{self, Debug};
use std::rc::Rc;

use reveal_embedder::{Matrix4, Offset, PointerDeviceKind};
use reveal_foundation::{App, Handle, HandleId, Listener, ValueChanged};

use crate::arena::{GestureArenaMember, GestureDisposition};
use crate::constants::K_PRESS_TIMEOUT;
use crate::events::{
    K_PRIMARY_BUTTON, K_SECONDARY_BUTTON, K_TERTIARY_BUTTON, PointerCancelEvent, PointerDownEvent,
    PointerEvent, PointerUpEvent,
};
use crate::gesture_details::PositionedGestureDetails;
use crate::gesture_settings::DeviceGestureSettings;
use crate::recognizer::{
    GestureRecognizer, GestureRecognizerData, GestureRecognizerState, OneSequenceData,
    OneSequenceGestureRecognizer, PrimaryPointerData, PrimaryPointerGestureRecognizer,
    RecognizerLeaf, RecognizerLeafData, UNSET_TOUCH_SLOP,
};
use crate::team::GestureArenaTeam;

/// Details for `GestureTapDownCallback`, such as position.
///
/// See also:
///
///  * `GestureDetector.onTapDown`, which receives this information.
///  * `TapGestureRecognizer`, which passes this information to one of its callbacks.
pub struct TapDownDetails {
    /// The global position at which the pointer contacted the screen.
    pub global_position: Offset,

    /// The local position at which the pointer contacted the screen.
    pub local_position: Offset,

    /// The kind of the device that initiated the event.
    pub kind: Option<PointerDeviceKind>,
}

impl TapDownDetails {
    /// Creates details for a tap-down callback.
    pub fn new(
        global_position: Offset,
        local_position: Option<Offset>,
        kind: Option<PointerDeviceKind>,
    ) -> TapDownDetails {
        TapDownDetails {
            global_position,
            local_position: local_position.unwrap_or(global_position),
            kind,
        }
    }
}

impl Default for TapDownDetails {
    fn default() -> TapDownDetails {
        TapDownDetails {
            global_position: Offset::ZERO,
            local_position: Offset::ZERO,
            kind: None,
        }
    }
}

impl PositionedGestureDetails for TapDownDetails {
    fn global_position(&self) -> Offset {
        self.global_position
    }

    fn local_position(&self) -> Offset {
        self.local_position
    }
}

/// Signature for when a pointer that might cause a tap has contacted the screen.
pub type GestureTapDownCallback = ValueChanged<TapDownDetails>;

/// Details for `GestureTapUpCallback`, such as position.
///
/// See also:
///
///  * `GestureDetector.onTapUp`, which receives this information.
///  * `TapGestureRecognizer`, which passes this information to one of its callbacks.
pub struct TapUpDetails {
    /// The global position at which the pointer contacted the screen.
    pub global_position: Offset,

    /// The local position at which the pointer contacted the screen.
    pub local_position: Offset,

    /// The kind of the device that initiated the event.
    pub kind: PointerDeviceKind,
}

impl TapUpDetails {
    /// Creates a [`TapUpDetails`] data object.
    pub fn new(
        global_position: Offset,
        local_position: Option<Offset>,
        kind: PointerDeviceKind,
    ) -> TapUpDetails {
        TapUpDetails {
            global_position,
            local_position: local_position.unwrap_or(global_position),
            kind,
        }
    }
}

impl PositionedGestureDetails for TapUpDetails {
    fn global_position(&self) -> Offset {
        self.global_position
    }

    fn local_position(&self) -> Offset {
        self.local_position
    }
}

/// Signature for when a pointer that will trigger a tap has stopped contacting the screen.
pub type GestureTapUpCallback = ValueChanged<TapUpDetails>;

/// Details object for callbacks that use `GestureTapMoveCallback`.
///
/// See also:
///
/// * `GestureDetector.onTapMove`, which receives this information.
/// * `TapGestureRecognizer`, which passes this information to one of its callbacks.
pub struct TapMoveDetails {
    /// The global position at which the pointer contacted the screen.
    pub global_position: Offset,

    /// The local position at which the pointer contacted the screen.
    pub local_position: Offset,

    /// The kind of the device that initiated the event.
    pub kind: PointerDeviceKind,

    /// The amount the pointer has moved in the coordinate space of the
    /// event receiver since the previous update.
    pub delta: Offset,
}

impl TapMoveDetails {
    /// Creates a [`TapMoveDetails`] data object.
    pub fn new(
        kind: PointerDeviceKind,
        global_position: Offset,
        delta: Offset,
        local_position: Option<Offset>,
    ) -> TapMoveDetails {
        TapMoveDetails {
            kind,
            global_position,
            delta,
            local_position: local_position.unwrap_or(global_position),
        }
    }
}

/// Signature for when a tap has occurred.
pub type GestureTapCallback = Listener;

/// Signature for when a pointer that triggered a tap has moved.
pub type GestureTapMoveCallback = ValueChanged<TapMoveDetails>;

/// Signature for when the pointer that previously triggered a tap-down will not
/// end up causing a tap.
pub type GestureTapCancelCallback = Listener;

/// Field bag for Dart's `BaseTapGestureRecognizer`.
pub(crate) struct BaseTapData {
    sent_tap_down: bool,
    won_arena_for_primary_pointer: bool,
    down: Option<PointerDownEvent>,
    up: Option<PointerUpEvent>,
}

impl BaseTapData {
    fn new() -> BaseTapData {
        BaseTapData {
            sent_tap_down: false,
            won_arena_for_primary_pointer: false,
            down: None,
            up: None,
        }
    }
}

pub(crate) struct TapGestureData {
    recognizer: GestureRecognizerData,
    one_sequence: OneSequenceData,
    primary: PrimaryPointerData,
    base_tap: BaseTapData,
    on_tap_down: Option<GestureTapDownCallback>,
    on_tap_up: Option<GestureTapUpCallback>,
    on_tap: Option<GestureTapCallback>,
    on_tap_move: Option<GestureTapMoveCallback>,
    on_tap_cancel: Option<GestureTapCancelCallback>,
    on_secondary_tap: Option<GestureTapCallback>,
    on_secondary_tap_down: Option<GestureTapDownCallback>,
    on_secondary_tap_up: Option<GestureTapUpCallback>,
    on_secondary_tap_cancel: Option<GestureTapCancelCallback>,
    on_tertiary_tap_down: Option<GestureTapDownCallback>,
    on_tertiary_tap_up: Option<GestureTapUpCallback>,
    on_tertiary_tap_cancel: Option<GestureTapCancelCallback>,
}

/// Recognizes taps.
///
/// [`TapGestureRecognizer`] considers all the pointers involved in the pointer
/// event sequence as contributing to one gesture. For this reason, extra
/// pointer interactions during a tap sequence are not recognized as additional
/// taps. For example, down-1, down-2, up-1, up-2 produces only one tap on up-1.
///
/// [`TapGestureRecognizer`] competes on pointer events of [`K_PRIMARY_BUTTON`] only
/// when it has at least one non-null `on_tap*` callback, on events of
/// [`K_SECONDARY_BUTTON`] only when it has at least one non-null `on_secondary_tap*`
/// callback, and on events of [`K_TERTIARY_BUTTON`] only when it has at least
/// one non-null `on_tertiary_tap*` callback. If it has no callbacks, it is a
/// no-op.
#[derive(Clone, Copy)]
pub struct TapGestureRecognizer(Handle<TapGestureData>);

impl TapGestureRecognizer {
    /// Creates a tap gesture recognizer.
    pub fn new(app: &mut App) -> TapGestureRecognizer {
        TapGestureRecognizer(app.create(TapGestureData {
            recognizer: GestureRecognizerData::new(),
            one_sequence: OneSequenceData::new(),
            primary: PrimaryPointerData::new(
                Some(K_PRESS_TIMEOUT),
                Some(UNSET_TOUCH_SLOP),
                Some(UNSET_TOUCH_SLOP),
            ),
            base_tap: BaseTapData::new(),
            on_tap_down: None,
            on_tap_up: None,
            on_tap: None,
            on_tap_move: None,
            on_tap_cancel: None,
            on_secondary_tap: None,
            on_secondary_tap_down: None,
            on_secondary_tap_up: None,
            on_secondary_tap_cancel: None,
            on_tertiary_tap_down: None,
            on_tertiary_tap_up: None,
            on_tertiary_tap_cancel: None,
        }))
    }

    /// The kind of devices that are allowed to be recognized.
    ///
    /// If none, events from all device kinds will be tracked and recognized.
    pub fn supported_devices(
        self,
        app: &mut App,
        devices: impl IntoIterator<Item = PointerDeviceKind>,
    ) -> TapGestureRecognizer {
        app.get_mut(self.0).recognizer.supported_devices = Some(devices.into_iter().collect());
        self
    }

    /// Called when interaction starts. Limits buttons this recognizer accepts.
    pub fn allowed_buttons_filter(
        self,
        app: &mut App,
        filter: impl Fn(i64) -> bool + 'static,
    ) -> TapGestureRecognizer {
        app.get_mut(self.0).recognizer.allowed_buttons_filter = Rc::new(filter);
        self
    }

    /// The maximum distance the gesture may drift before it is accepted.
    ///
    /// `None` means unlimited. Omitted at [`new`](Self::new) uses touch slop.
    pub fn pre_accept_slop_tolerance(
        self,
        app: &mut App,
        value: Option<f64>,
    ) -> TapGestureRecognizer {
        app.get_mut(self.0).primary.pre_accept_slop_tolerance = value;
        self
    }

    /// The maximum distance the gesture may drift after it is accepted.
    ///
    /// `None` means unlimited. Omitted at [`new`](Self::new) uses touch slop.
    pub fn post_accept_slop_tolerance(
        self,
        app: &mut App,
        value: Option<f64>,
    ) -> TapGestureRecognizer {
        app.get_mut(self.0).primary.post_accept_slop_tolerance = value;
        self
    }

    /// Optional device specific configuration that takes precedence over
    /// framework defaults.
    pub fn gesture_settings(self, app: &App) -> Option<DeviceGestureSettings> {
        app.get(self.0).recognizer.gesture_settings
    }

    /// Sets [`gesture_settings`](Self::gesture_settings).
    pub fn set_gesture_settings(self, app: &mut App, settings: Option<DeviceGestureSettings>) {
        app.get_mut(self.0).recognizer.gesture_settings = settings;
    }

    /// A pointer has contacted the screen at a particular location with a
    /// primary button, which might be the start of a tap.
    pub fn set_on_tap_down(
        self,
        app: &mut App,
        callback: impl Fn(&mut App, TapDownDetails) + 'static,
    ) {
        app.get_mut(self.0).on_tap_down = Some(Rc::new(callback));
    }

    /// A pointer has stopped contacting the screen at a particular location,
    /// which is recognized as a tap of a primary button.
    pub fn set_on_tap_up(self, app: &mut App, callback: impl Fn(&mut App, TapUpDetails) + 'static) {
        app.get_mut(self.0).on_tap_up = Some(Rc::new(callback));
    }

    /// A pointer has stopped contacting the screen, which is recognized as a
    /// tap of a primary button.
    pub fn set_on_tap(self, app: &mut App, callback: impl Fn(&mut App) + 'static) {
        app.get_mut(self.0).on_tap = Some(Listener::new(callback));
    }

    /// A pointer that triggered a tap has moved.
    pub fn set_on_tap_move(
        self,
        app: &mut App,
        callback: impl Fn(&mut App, TapMoveDetails) + 'static,
    ) {
        app.get_mut(self.0).on_tap_move = Some(Rc::new(callback));
    }

    /// A pointer that previously triggered [`set_on_tap_down`](Self::set_on_tap_down)
    /// will not end up causing a tap.
    pub fn set_on_tap_cancel(self, app: &mut App, callback: impl Fn(&mut App) + 'static) {
        app.get_mut(self.0).on_tap_cancel = Some(Listener::new(callback));
    }

    /// A pointer has stopped contacting the screen, which is recognized as a
    /// tap of a secondary button.
    pub fn set_on_secondary_tap(self, app: &mut App, callback: impl Fn(&mut App) + 'static) {
        app.get_mut(self.0).on_secondary_tap = Some(Listener::new(callback));
    }

    /// A pointer has contacted the screen with a secondary button, which might
    /// be the start of a secondary tap.
    pub fn set_on_secondary_tap_down(
        self,
        app: &mut App,
        callback: impl Fn(&mut App, TapDownDetails) + 'static,
    ) {
        app.get_mut(self.0).on_secondary_tap_down = Some(Rc::new(callback));
    }

    /// A pointer has stopped contacting the screen, which is recognized as a
    /// tap of a secondary button.
    pub fn set_on_secondary_tap_up(
        self,
        app: &mut App,
        callback: impl Fn(&mut App, TapUpDetails) + 'static,
    ) {
        app.get_mut(self.0).on_secondary_tap_up = Some(Rc::new(callback));
    }

    /// A pointer that previously triggered [`set_on_secondary_tap_down`](Self::set_on_secondary_tap_down)
    /// will not end up causing a tap.
    pub fn set_on_secondary_tap_cancel(self, app: &mut App, callback: impl Fn(&mut App) + 'static) {
        app.get_mut(self.0).on_secondary_tap_cancel = Some(Listener::new(callback));
    }

    /// A pointer has contacted the screen with a tertiary button, which might
    /// be the start of a tertiary tap.
    pub fn set_on_tertiary_tap_down(
        self,
        app: &mut App,
        callback: impl Fn(&mut App, TapDownDetails) + 'static,
    ) {
        app.get_mut(self.0).on_tertiary_tap_down = Some(Rc::new(callback));
    }

    /// A pointer has stopped contacting the screen, which is recognized as a
    /// tap of a tertiary button.
    pub fn set_on_tertiary_tap_up(
        self,
        app: &mut App,
        callback: impl Fn(&mut App, TapUpDetails) + 'static,
    ) {
        app.get_mut(self.0).on_tertiary_tap_up = Some(Rc::new(callback));
    }

    /// A pointer that previously triggered [`set_on_tertiary_tap_down`](Self::set_on_tertiary_tap_down)
    /// will not end up causing a tap.
    pub fn set_on_tertiary_tap_cancel(self, app: &mut App, callback: impl Fn(&mut App) + 'static) {
        app.get_mut(self.0).on_tertiary_tap_cancel = Some(Listener::new(callback));
    }

    /// The team that this recognizer belongs to, if any.
    pub fn team(self, app: &App) -> Option<GestureArenaTeam> {
        app.get(self.0).one_sequence.team
    }

    /// The [`team`](Self::team) can only be set once.
    pub fn set_team(self, app: &mut App, value: GestureArenaTeam) {
        debug_assert!(app.get(self.0).one_sequence.entries.is_empty());
        debug_assert!(app.get(self.0).one_sequence.tracked_pointers.is_empty());
        debug_assert!(app.get(self.0).one_sequence.team.is_none());
        app.get_mut(self.0).one_sequence.team = Some(value);
    }

    /// Registers a new pointer that might be relevant to this gesture detector.
    pub fn add_pointer(self, app: &mut App, event: PointerDownEvent) {
        GestureRecognizer::add_pointer(self, app, event);
    }

    /// Registers a new pointer pan/zoom that might be relevant to this gesture
    /// detector.
    pub fn add_pointer_pan_zoom(
        self,
        app: &mut App,
        event: crate::events::PointerPanZoomStartEvent,
    ) {
        GestureRecognizer::add_pointer_pan_zoom(self, app, event);
    }

    /// Releases any resources used by the object.
    pub fn dispose(self, app: &mut App) {
        <Self as RecognizerLeaf>::dispose(self, app);
    }

    /// Returns a very short pretty description of the gesture that the
    /// recognizer looks for.
    pub fn debug_description(self) -> &'static str {
        "tap"
    }

    fn handle_tap_down(self, app: &mut App, down: &PointerDownEvent) {
        let details = TapDownDetails::new(
            down.position,
            Some(down.local_position()),
            Some(GestureRecognizer::kind_for_pointer(self, app, down.pointer)),
        );
        match down.buttons {
            K_PRIMARY_BUTTON => {
                if let Some(callback) = app.get(self.0).on_tap_down.clone() {
                    self.invoke_callback(app, "onTapDown", |app| callback(app, details));
                }
            }
            K_SECONDARY_BUTTON => {
                if let Some(callback) = app.get(self.0).on_secondary_tap_down.clone() {
                    self.invoke_callback(app, "onSecondaryTapDown", |app| callback(app, details));
                }
            }
            K_TERTIARY_BUTTON => {
                if let Some(callback) = app.get(self.0).on_tertiary_tap_down.clone() {
                    self.invoke_callback(app, "onTertiaryTapDown", |app| callback(app, details));
                }
            }
            _ => {}
        }
    }

    fn handle_tap_up(self, app: &mut App, down: &PointerDownEvent, up: &PointerUpEvent) {
        let details = TapUpDetails::new(up.position, Some(up.local_position()), up.kind);
        match down.buttons {
            K_PRIMARY_BUTTON => {
                if let Some(callback) = app.get(self.0).on_tap_up.clone() {
                    self.invoke_callback(app, "onTapUp", |app| callback(app, details));
                }
                if let Some(callback) = app.get(self.0).on_tap.clone() {
                    self.invoke_callback(app, "onTap", |app| callback.call(app));
                }
            }
            K_SECONDARY_BUTTON => {
                if let Some(callback) = app.get(self.0).on_secondary_tap_up.clone() {
                    self.invoke_callback(app, "onSecondaryTapUp", |app| callback(app, details));
                }
                if let Some(callback) = app.get(self.0).on_secondary_tap.clone() {
                    self.invoke_callback(app, "onSecondaryTap", |app| callback.call(app));
                }
            }
            K_TERTIARY_BUTTON => {
                if let Some(callback) = app.get(self.0).on_tertiary_tap_up.clone() {
                    self.invoke_callback(app, "onTertiaryTapUp", |app| callback(app, details));
                }
            }
            _ => {}
        }
    }

    fn handle_tap_move(self, app: &mut App, move_event: &crate::events::PointerMoveEvent) {
        if app.get(self.0).on_tap_move.is_some() && move_event.buttons == K_PRIMARY_BUTTON {
            let details = TapMoveDetails::new(
                GestureRecognizer::kind_for_pointer(self, app, move_event.pointer),
                move_event.position,
                move_event.delta,
                Some(move_event.local_position()),
            );
            let callback = app.get(self.0).on_tap_move.clone().unwrap();
            self.invoke_callback(app, "onTapMove", |app| callback(app, details));
        }
    }

    fn handle_tap_cancel(
        self,
        app: &mut App,
        down: &PointerDownEvent,
        _cancel: Option<&PointerCancelEvent>,
        reason: &str,
    ) {
        let note = if reason.is_empty() {
            String::new()
        } else {
            format!("{reason} ")
        };
        match down.buttons {
            K_PRIMARY_BUTTON => {
                if let Some(callback) = app.get(self.0).on_tap_cancel.clone() {
                    self.invoke_callback(app, &format!("{note}onTapCancel"), |app| {
                        callback.call(app)
                    });
                }
            }
            K_SECONDARY_BUTTON => {
                if let Some(callback) = app.get(self.0).on_secondary_tap_cancel.clone() {
                    self.invoke_callback(app, &format!("{note}onSecondaryTapCancel"), |app| {
                        callback.call(app)
                    });
                }
            }
            K_TERTIARY_BUTTON => {
                if let Some(callback) = app.get(self.0).on_tertiary_tap_cancel.clone() {
                    self.invoke_callback(app, &format!("{note}onTertiaryTapCancel"), |app| {
                        callback.call(app)
                    });
                }
            }
            _ => {}
        }
    }

    fn invoke_callback<T>(
        self,
        app: &mut App,
        name: &str,
        callback: impl FnOnce(&mut App) -> T,
    ) -> Option<T> {
        GestureRecognizer::invoke_callback(self, app, name, callback)
    }
}

impl Debug for TapGestureRecognizer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("TapGestureRecognizer")
            .field(&self.0.id())
            .finish()
    }
}

impl GestureArenaMember for TapGestureRecognizer {
    fn accept_gesture(&self, app: &mut App, pointer: i64) {
        BaseTapGestureRecognizer::accept_gesture(*self, app, pointer);
    }

    fn reject_gesture(&self, app: &mut App, pointer: i64) {
        BaseTapGestureRecognizer::reject_gesture(*self, app, pointer);
    }

    fn member_id(&self) -> HandleId {
        self.0.id()
    }
}

impl RecognizerLeafData for TapGestureData {
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
    fn primary(&self) -> &PrimaryPointerData {
        &self.primary
    }
    fn primary_mut(&mut self) -> &mut PrimaryPointerData {
        &mut self.primary
    }
}

impl RecognizerLeaf for TapGestureRecognizer {
    type Data = TapGestureData;

    fn slot(self) -> Handle<TapGestureData> {
        self.0
    }

    fn from_slot(slot: Handle<TapGestureData>) -> TapGestureRecognizer {
        TapGestureRecognizer(slot)
    }

    fn add_allowed_pointer(self, app: &mut App, event: PointerDownEvent) {
        BaseTapGestureRecognizer::add_allowed_pointer(self, app, event);
    }

    fn start_tracking_pointer(self, app: &mut App, pointer: i64, transform: Option<Matrix4>) {
        BaseTapGestureRecognizer::start_tracking_pointer(self, app, pointer, transform);
    }

    fn handle_primary_pointer(self, app: &mut App, event: PointerEvent) {
        BaseTapGestureRecognizer::handle_primary_pointer(self, app, event);
    }

    fn did_exceed_deadline(self, app: &mut App) {
        BaseTapGestureRecognizer::did_exceed_deadline(self, app);
    }

    fn resolve(self, app: &mut App, disposition: GestureDisposition) {
        BaseTapGestureRecognizer::resolve(self, app, disposition);
    }

    fn is_pointer_allowed(self, app: &App, event: &PointerDownEvent) -> bool {
        let allowed = {
            let data = app.get(self.0);
            match event.buttons {
                K_PRIMARY_BUTTON => {
                    data.on_tap_down.is_some()
                        || data.on_tap.is_some()
                        || data.on_tap_up.is_some()
                        || data.on_tap_cancel.is_some()
                        || data.on_tap_move.is_some()
                }
                K_SECONDARY_BUTTON => {
                    data.on_secondary_tap.is_some()
                        || data.on_secondary_tap_down.is_some()
                        || data.on_secondary_tap_up.is_some()
                        || data.on_secondary_tap_cancel.is_some()
                }
                K_TERTIARY_BUTTON => {
                    data.on_tertiary_tap_down.is_some()
                        || data.on_tertiary_tap_up.is_some()
                        || data.on_tertiary_tap_cancel.is_some()
                }
                _ => false,
            }
        };
        allowed && GestureRecognizer::is_pointer_allowed(self, app, event)
    }
}

struct BaseTapGestureRecognizer;

impl BaseTapGestureRecognizer {
    fn add_allowed_pointer(this: TapGestureRecognizer, app: &mut App, event: PointerDownEvent) {
        if app.get(this.0).primary.state == GestureRecognizerState::Ready {
            if app.get(this.0).base_tap.down.is_some() && app.get(this.0).base_tap.up.is_some() {
                debug_assert_eq!(
                    app.get(this.0).base_tap.down.as_ref().unwrap().pointer,
                    app.get(this.0).base_tap.up.as_ref().unwrap().pointer
                );
                Self::reset(this, app);
            }
            debug_assert!(
                app.get(this.0).base_tap.down.is_none() && app.get(this.0).base_tap.up.is_none()
            );
            app.get_mut(this.0).base_tap.down = Some(event.clone());
        }
        if app.get(this.0).base_tap.down.is_some() {
            PrimaryPointerGestureRecognizer::add_allowed_pointer(this, app, event);
        }
    }

    fn start_tracking_pointer(
        this: TapGestureRecognizer,
        app: &mut App,
        pointer: i64,
        transform: Option<Matrix4>,
    ) {
        debug_assert!(app.get(this.0).base_tap.down.is_some());
        OneSequenceGestureRecognizer::start_tracking_pointer(this, app, pointer, transform);
    }

    fn handle_primary_pointer(this: TapGestureRecognizer, app: &mut App, event: PointerEvent) {
        if let PointerEvent::Up(up) = &event {
            app.get_mut(this.0).base_tap.up = Some(up.clone());
            Self::check_up(this, app);
        } else if let PointerEvent::Cancel(cancel) = &event {
            this.resolve(app, GestureDisposition::Rejected);
            if app.get(this.0).base_tap.sent_tap_down {
                Self::check_cancel(this, app, Some(cancel), "");
            }
            Self::reset(this, app);
        } else if event.buttons() != app.get(this.0).base_tap.down.as_ref().unwrap().buttons {
            this.resolve(app, GestureDisposition::Rejected);
            let primary = app.get(this.0).primary.primary_pointer.unwrap();
            OneSequenceGestureRecognizer::stop_tracking_pointer(this, app, primary);
        } else if let PointerEvent::Move(move_event) = &event {
            Self::check_move(this, app, move_event);
        }
    }

    fn resolve(this: TapGestureRecognizer, app: &mut App, disposition: GestureDisposition) {
        if app.get(this.0).base_tap.won_arena_for_primary_pointer
            && disposition == GestureDisposition::Rejected
        {
            debug_assert!(app.get(this.0).base_tap.sent_tap_down);
            Self::check_cancel(this, app, None, "spontaneous");
            Self::reset(this, app);
        }
        OneSequenceGestureRecognizer::resolve(this, app, disposition);
    }

    fn did_exceed_deadline(this: TapGestureRecognizer, app: &mut App) {
        Self::check_down(this, app);
    }

    fn accept_gesture(this: TapGestureRecognizer, app: &mut App, pointer: i64) {
        PrimaryPointerGestureRecognizer::accept_gesture(this, app, pointer);
        if Some(pointer) == app.get(this.0).primary.primary_pointer {
            Self::check_down(this, app);
            app.get_mut(this.0).base_tap.won_arena_for_primary_pointer = true;
            Self::check_up(this, app);
        }
    }

    fn reject_gesture(this: TapGestureRecognizer, app: &mut App, pointer: i64) {
        PrimaryPointerGestureRecognizer::reject_gesture(this, app, pointer);
        if Some(pointer) == app.get(this.0).primary.primary_pointer {
            debug_assert!(app.get(this.0).primary.state != GestureRecognizerState::Possible);
            if app.get(this.0).base_tap.sent_tap_down {
                Self::check_cancel(this, app, None, "forced");
            }
            Self::reset(this, app);
        }
    }

    fn check_down(this: TapGestureRecognizer, app: &mut App) {
        if app.get(this.0).base_tap.sent_tap_down {
            return;
        }
        let down = app.get(this.0).base_tap.down.clone().unwrap();
        this.handle_tap_down(app, &down);
        app.get_mut(this.0).base_tap.sent_tap_down = true;
    }

    fn check_up(this: TapGestureRecognizer, app: &mut App) {
        if !app.get(this.0).base_tap.won_arena_for_primary_pointer
            || app.get(this.0).base_tap.up.is_none()
        {
            return;
        }
        debug_assert_eq!(
            app.get(this.0).base_tap.up.as_ref().unwrap().pointer,
            app.get(this.0).base_tap.down.as_ref().unwrap().pointer
        );
        let down = app.get(this.0).base_tap.down.clone().unwrap();
        let up = app.get(this.0).base_tap.up.clone().unwrap();
        this.handle_tap_up(app, &down, &up);
        Self::reset(this, app);
    }

    fn check_cancel(
        this: TapGestureRecognizer,
        app: &mut App,
        event: Option<&PointerCancelEvent>,
        note: &str,
    ) {
        let down = app.get(this.0).base_tap.down.clone().unwrap();
        this.handle_tap_cancel(app, &down, event, note);
    }

    fn check_move(
        this: TapGestureRecognizer,
        app: &mut App,
        event: &crate::events::PointerMoveEvent,
    ) {
        debug_assert_eq!(
            event.pointer,
            app.get(this.0).base_tap.down.as_ref().unwrap().pointer
        );
        this.handle_tap_move(app, event);
    }

    fn reset(this: TapGestureRecognizer, app: &mut App) {
        let data = app.get_mut(this.0);
        data.base_tap.sent_tap_down = false;
        data.base_tap.won_arena_for_primary_pointer = false;
        data.base_tap.up = None;
        data.base_tap.down = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    use crate::binding::GestureBinding;
    use crate::events::{PointerMoveEvent, PointerUpEvent};

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

    fn sweep(app: &mut App, pointer: i64) {
        GestureBinding::instance(app)
            .gesture_arena(app)
            .sweep(app, pointer);
    }

    #[test]
    fn local_position_defaults_to_global() {
        let down_details = TapDownDetails::new(Offset::new(3.0, 4.0), None, None);
        assert_eq!(down_details.local_position, Offset::new(3.0, 4.0));
        let up_details = TapUpDetails::new(Offset::new(1.0, 2.0), None, PointerDeviceKind::Touch);
        assert_eq!(up_details.local_position, Offset::new(1.0, 2.0));
        let move_details = TapMoveDetails::new(
            PointerDeviceKind::Touch,
            Offset::new(5.0, 6.0),
            Offset::new(1.0, 0.0),
            None,
        );
        assert_eq!(move_details.local_position, Offset::new(5.0, 6.0));
    }

    #[test]
    fn should_recognize_tap() {
        let mut app = App::new();
        let tap = TapGestureRecognizer::new(&mut app);
        let recognized = Rc::new(Cell::new(false));
        let flag = Rc::clone(&recognized);
        tap.set_on_tap(&mut app, move |_app| flag.set(true));

        let down1 = down(1, Offset::new(10.0, 10.0));
        let up1 = up(1, Offset::new(11.0, 9.0));
        tap.add_pointer(&mut app, down1.clone());
        close_arena(&mut app, 1);
        assert!(!recognized.get());
        route(&mut app, PointerEvent::Down(down1));
        assert!(!recognized.get());
        route(&mut app, PointerEvent::Up(up1));
        assert!(recognized.get());
        sweep(&mut app, 1);
        assert!(recognized.get());
        tap.dispose(&mut app);
    }

    #[test]
    fn should_recognize_tap_for_supported_devices_only() {
        let mut app = App::new();
        let tap = TapGestureRecognizer::new(&mut app).supported_devices(
            &mut app,
            [PointerDeviceKind::Mouse, PointerDeviceKind::Stylus],
        );
        let recognized = Rc::new(Cell::new(false));
        let flag = Rc::clone(&recognized);
        tap.set_on_tap(&mut app, move |_app| flag.set(true));

        let touch_down = down(1, Offset::new(10.0, 10.0));
        let touch_up = up(1, Offset::new(11.0, 9.0));
        tap.add_pointer(&mut app, touch_down.clone());
        close_arena(&mut app, 1);
        route(&mut app, PointerEvent::Down(touch_down));
        route(&mut app, PointerEvent::Up(touch_up));
        sweep(&mut app, 1);
        assert!(!recognized.get());

        let mouse_down = PointerDownEvent {
            kind: PointerDeviceKind::Mouse,
            pointer: 1,
            position: Offset::new(10.0, 10.0),
            buttons: K_PRIMARY_BUTTON,
            ..PointerDownEvent::default()
        };
        let mouse_up = PointerUpEvent {
            kind: PointerDeviceKind::Mouse,
            pointer: 1,
            position: Offset::new(11.0, 9.0),
            buttons: K_PRIMARY_BUTTON,
            ..PointerUpEvent::default()
        };
        tap.add_pointer(&mut app, mouse_down.clone());
        close_arena(&mut app, 1);
        route(&mut app, PointerEvent::Down(mouse_down));
        route(&mut app, PointerEvent::Up(mouse_up));
        sweep(&mut app, 1);
        assert!(recognized.get());

        recognized.set(false);
        let stylus_down = PointerDownEvent {
            kind: PointerDeviceKind::Stylus,
            pointer: 1,
            position: Offset::new(10.0, 10.0),
            buttons: K_PRIMARY_BUTTON,
            ..PointerDownEvent::default()
        };
        let stylus_up = PointerUpEvent {
            kind: PointerDeviceKind::Stylus,
            pointer: 1,
            position: Offset::new(11.0, 9.0),
            buttons: K_PRIMARY_BUTTON,
            ..PointerUpEvent::default()
        };
        tap.add_pointer(&mut app, stylus_down.clone());
        close_arena(&mut app, 1);
        route(&mut app, PointerEvent::Down(stylus_down));
        route(&mut app, PointerEvent::Up(stylus_up));
        sweep(&mut app, 1);
        assert!(recognized.get());
        tap.dispose(&mut app);
    }

    #[test]
    fn details_contain_the_correct_device_kind() {
        let mut app = App::new();
        let tap = TapGestureRecognizer::new(&mut app);
        let last_down = Rc::new(RefCell::new(None));
        let last_up = Rc::new(RefCell::new(None));
        let down_slot = Rc::clone(&last_down);
        let up_slot = Rc::clone(&last_up);
        tap.set_on_tap_down(&mut app, move |_app, details| {
            *down_slot.borrow_mut() = Some(details.kind);
        });
        tap.set_on_tap_up(&mut app, move |_app, details| {
            *up_slot.borrow_mut() = Some(details.kind);
        });

        let mouse_down = PointerDownEvent {
            pointer: 1,
            kind: PointerDeviceKind::Mouse,
            buttons: K_PRIMARY_BUTTON,
            ..PointerDownEvent::default()
        };
        let mouse_up = PointerUpEvent {
            pointer: 1,
            kind: PointerDeviceKind::Mouse,
            buttons: K_PRIMARY_BUTTON,
            ..PointerUpEvent::default()
        };
        tap.add_pointer(&mut app, mouse_down.clone());
        close_arena(&mut app, 1);
        route(&mut app, PointerEvent::Down(mouse_down));
        assert_eq!(*last_down.borrow(), Some(Some(PointerDeviceKind::Mouse)));
        route(&mut app, PointerEvent::Up(mouse_up));
        assert_eq!(*last_up.borrow(), Some(PointerDeviceKind::Mouse));
        tap.dispose(&mut app);
    }

    #[test]
    fn no_duplicate_tap_events() {
        let mut app = App::new();
        let tap = TapGestureRecognizer::new(&mut app);
        let taps = Rc::new(Cell::new(0));
        let count = Rc::clone(&taps);
        tap.set_on_tap(&mut app, move |_app| count.set(count.get() + 1));

        let down1 = down(1, Offset::new(10.0, 10.0));
        let up1 = up(1, Offset::new(11.0, 9.0));
        tap.add_pointer(&mut app, down1.clone());
        close_arena(&mut app, 1);
        route(&mut app, PointerEvent::Down(down1.clone()));
        route(&mut app, PointerEvent::Up(up1.clone()));
        sweep(&mut app, 1);
        assert_eq!(taps.get(), 1);

        tap.add_pointer(&mut app, down1.clone());
        close_arena(&mut app, 1);
        route(&mut app, PointerEvent::Down(down1));
        route(&mut app, PointerEvent::Up(up1));
        sweep(&mut app, 1);
        assert_eq!(taps.get(), 2);
        tap.dispose(&mut app);
    }

    #[test]
    fn should_not_recognize_two_overlapping_taps_fifo() {
        let mut app = App::new();
        let tap = TapGestureRecognizer::new(&mut app);
        let taps = Rc::new(Cell::new(0));
        let count = Rc::clone(&taps);
        tap.set_on_tap(&mut app, move |_app| count.set(count.get() + 1));

        let down1 = down(1, Offset::new(10.0, 10.0));
        let up1 = up(1, Offset::new(11.0, 9.0));
        let down2 = down(2, Offset::new(30.0, 30.0));
        let up2 = up(2, Offset::new(31.0, 29.0));
        tap.add_pointer(&mut app, down1.clone());
        close_arena(&mut app, 1);
        route(&mut app, PointerEvent::Down(down1.clone()));
        tap.add_pointer(&mut app, down2);
        close_arena(&mut app, 2);
        route(&mut app, PointerEvent::Down(down1));
        route(&mut app, PointerEvent::Up(up1));
        sweep(&mut app, 1);
        assert_eq!(taps.get(), 1);
        route(&mut app, PointerEvent::Up(up2));
        sweep(&mut app, 2);
        assert_eq!(taps.get(), 1);
        tap.dispose(&mut app);
    }

    #[test]
    fn distance_cancels_tap() {
        let mut app = App::new();
        let tap = TapGestureRecognizer::new(&mut app);
        let recognized = Rc::new(Cell::new(false));
        let canceled = Rc::new(Cell::new(false));
        let rec = Rc::clone(&recognized);
        let can = Rc::clone(&canceled);
        tap.set_on_tap(&mut app, move |_app| rec.set(true));
        tap.set_on_tap_cancel(&mut app, move |_app| can.set(true));

        let down3 = down(3, Offset::new(10.0, 10.0));
        tap.add_pointer(&mut app, down3.clone());
        close_arena(&mut app, 3);
        route(&mut app, PointerEvent::Down(down3));
        route(
            &mut app,
            PointerEvent::Move(move_event(3, Offset::new(25.0, 25.0))),
        );
        assert!(!recognized.get());
        assert!(canceled.get());
        route(&mut app, PointerEvent::Up(up(3, Offset::new(25.0, 25.0))));
        sweep(&mut app, 3);
        assert!(!recognized.get());
        assert!(canceled.get());
        tap.dispose(&mut app);
    }

    #[test]
    fn short_distance_does_not_cancel_tap() {
        let mut app = App::new();
        let tap = TapGestureRecognizer::new(&mut app);
        let recognized = Rc::new(Cell::new(false));
        let canceled = Rc::new(Cell::new(false));
        let rec = Rc::clone(&recognized);
        let can = Rc::clone(&canceled);
        tap.set_on_tap(&mut app, move |_app| rec.set(true));
        tap.set_on_tap_cancel(&mut app, move |_app| can.set(true));

        let down4 = down(4, Offset::new(10.0, 10.0));
        tap.add_pointer(&mut app, down4.clone());
        close_arena(&mut app, 4);
        route(&mut app, PointerEvent::Down(down4));
        route(
            &mut app,
            PointerEvent::Move(move_event(4, Offset::new(22.0, 22.0))),
        );
        route(&mut app, PointerEvent::Up(up(4, Offset::new(22.0, 22.0))));
        sweep(&mut app, 4);
        assert!(recognized.get());
        assert!(!canceled.get());
        tap.dispose(&mut app);
    }

    #[test]
    fn timeout_does_not_cancel_tap() {
        let mut app = App::new();
        let tap = TapGestureRecognizer::new(&mut app);
        let recognized = Rc::new(Cell::new(false));
        let flag = Rc::clone(&recognized);
        tap.set_on_tap(&mut app, move |_app| flag.set(true));

        let down1 = down(1, Offset::new(10.0, 10.0));
        tap.add_pointer(&mut app, down1.clone());
        close_arena(&mut app, 1);
        route(&mut app, PointerEvent::Down(down1));
        app.elapse(std::time::Duration::from_millis(500));
        assert!(!recognized.get());
        route(&mut app, PointerEvent::Up(up(1, Offset::new(11.0, 9.0))));
        sweep(&mut app, 1);
        assert!(recognized.get());
        tap.dispose(&mut app);
    }

    #[test]
    fn press_timeout_fires_on_tap_down() {
        let mut app = App::new();
        let tap = TapGestureRecognizer::new(&mut app);
        let down_fired = Rc::new(Cell::new(false));
        let flag = Rc::clone(&down_fired);
        tap.set_on_tap_down(&mut app, move |_app, _details| flag.set(true));
        tap.set_on_tap(&mut app, |_app| {});

        let down1 = down(1, Offset::new(10.0, 10.0));
        tap.add_pointer(&mut app, down1.clone());
        let competitor = TestMember::new(&mut app);
        GestureBinding::instance(&mut app)
            .gesture_arena(&app)
            .add(&mut app, 1, competitor);
        close_arena(&mut app, 1);
        route(&mut app, PointerEvent::Down(down1));
        assert!(!down_fired.get());
        app.elapse(K_PRESS_TIMEOUT);
        assert!(down_fired.get());
        tap.dispose(&mut app);
    }

    #[test]
    fn pointer_cancel_after_deadline_cancels_tap() {
        let mut app = App::new();
        let tap = TapGestureRecognizer::new(&mut app);
        let log = Rc::new(RefCell::new(Vec::new()));
        let down_log = Rc::clone(&log);
        let cancel_log = Rc::clone(&log);
        tap.set_on_tap_down(&mut app, move |_app, _details| {
            down_log.borrow_mut().push("down");
        });
        tap.set_on_tap(&mut app, |_app| {});
        tap.set_on_tap_cancel(&mut app, move |_app| {
            cancel_log.borrow_mut().push("cancel");
        });

        let down5 = down(5, Offset::new(10.0, 10.0));
        tap.add_pointer(&mut app, down5.clone());
        close_arena(&mut app, 5);
        app.elapse(std::time::Duration::from_millis(5000));
        assert_eq!(*log.borrow(), ["down"]);
        route(
            &mut app,
            PointerEvent::Cancel(PointerCancelEvent {
                pointer: 5,
                position: Offset::new(10.0, 10.0),
                ..PointerCancelEvent::default()
            }),
        );
        assert_eq!(*log.borrow(), ["down", "cancel"]);
        tap.dispose(&mut app);
    }

    #[test]
    fn should_yield_to_other_arena_members() {
        let mut app = App::new();
        let tap = TapGestureRecognizer::new(&mut app);
        let recognized = Rc::new(Cell::new(false));
        let flag = Rc::clone(&recognized);
        tap.set_on_tap(&mut app, move |_app| flag.set(true));

        let down1 = down(1, Offset::new(10.0, 10.0));
        tap.add_pointer(&mut app, down1.clone());
        let member = TestMember::new(&mut app);
        let entry = GestureBinding::instance(&mut app)
            .gesture_arena(&app)
            .add(&mut app, 1, member);
        GestureBinding::instance(&mut app)
            .gesture_arena(&app)
            .hold(&mut app, 1);
        close_arena(&mut app, 1);
        route(&mut app, PointerEvent::Down(down1));
        route(&mut app, PointerEvent::Up(up(1, Offset::new(11.0, 9.0))));
        sweep(&mut app, 1);
        assert!(!recognized.get());
        entry.resolve(&mut app, GestureDisposition::Accepted);
        assert!(!recognized.get());
        tap.dispose(&mut app);
    }

    #[test]
    fn should_trigger_on_release_of_held_arena() {
        let mut app = App::new();
        let tap = TapGestureRecognizer::new(&mut app);
        let recognized = Rc::new(Cell::new(false));
        let flag = Rc::clone(&recognized);
        tap.set_on_tap(&mut app, move |_app| flag.set(true));

        let down1 = down(1, Offset::new(10.0, 10.0));
        tap.add_pointer(&mut app, down1.clone());
        let member = TestMember::new(&mut app);
        let entry = GestureBinding::instance(&mut app)
            .gesture_arena(&app)
            .add(&mut app, 1, member);
        GestureBinding::instance(&mut app)
            .gesture_arena(&app)
            .hold(&mut app, 1);
        close_arena(&mut app, 1);
        route(&mut app, PointerEvent::Down(down1));
        route(&mut app, PointerEvent::Up(up(1, Offset::new(11.0, 9.0))));
        sweep(&mut app, 1);
        assert!(!recognized.get());
        entry.resolve(&mut app, GestureDisposition::Rejected);
        app.drain_microtasks();
        assert!(recognized.get());
        tap.dispose(&mut app);
    }

    struct TestMemberData;

    #[derive(Clone, Copy)]
    struct TestMember(Handle<TestMemberData>);

    impl TestMember {
        fn new(app: &mut App) -> TestMember {
            TestMember(app.create(TestMemberData))
        }
    }

    impl GestureArenaMember for TestMember {
        fn accept_gesture(&self, _app: &mut App, _pointer: i64) {}

        fn reject_gesture(&self, _app: &mut App, _pointer: i64) {}

        fn member_id(&self) -> HandleId {
            self.0.id()
        }
    }
}
