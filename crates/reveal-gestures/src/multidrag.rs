//! Flutter counterpart: `gestures/multidrag.dart`.

use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::time::Duration;

use reveal_embedder::{Offset, PointerDeviceKind};
use reveal_foundation::{App, Handle, HandleId, Listener, Timer};

use crate::arena::{GestureArenaEntry, GestureDisposition};
use crate::binding::GestureBinding;
use crate::constants::K_LONG_PRESS_TIMEOUT;
use crate::drag::Drag;
use crate::drag_details::{DragEndDetails, DragUpdateDetails};
use crate::events::{
    K_PRIMARY_BUTTON, PointerDownEvent, PointerEvent, PointerMoveEvent, compute_hit_slop,
};
use crate::gesture_settings::DeviceGestureSettings;
use crate::pointer_router::PointerRoute;
use crate::recognizer::{
    AllowedButtonsFilter, GestureRecognizer, GestureRecognizerData, RecognizerLeaf,
    RecognizerLeafData,
};
use crate::velocity_tracker::VelocityTracker;

/// Signature for when `MultiDragGestureRecognizer` recognizes the start of a drag gesture.
pub type GestureMultiDragStartCallback = Rc<dyn Fn(&mut App, Offset) -> Option<Rc<dyn Drag>>>;

/// Per-pointer state for a `MultiDragGestureRecognizer`.
///
/// A `MultiDragGestureRecognizer` tracks each pointer separately. The state for
/// each pointer is a subclass of `MultiDragPointerState`.
pub struct MultiDragPointerStateData {
    gesture_settings: Option<DeviceGestureSettings>,
    initial_position: Offset,
    velocity_tracker: VelocityTracker,
    kind: PointerDeviceKind,
    client: Option<Rc<dyn Drag>>,
    pending_delta: Option<Offset>,
    last_pending_event_timestamp: Option<Duration>,
    arena_entry: Option<GestureArenaEntry>,
}

impl MultiDragPointerStateData {
    pub fn new(
        initial_position: Offset,
        kind: PointerDeviceKind,
        gesture_settings: Option<DeviceGestureSettings>,
    ) -> Self {
        Self {
            gesture_settings,
            initial_position,
            velocity_tracker: VelocityTracker::with_kind(kind),
            kind,
            client: None,
            pending_delta: Some(Offset::ZERO),
            last_pending_event_timestamp: None,
            arena_entry: None,
        }
    }
}

pub trait MultiDragPointerState: Sized + 'static {
    fn pointer_state(&self) -> &MultiDragPointerStateData;

    fn pointer_state_mut(&mut self) -> &mut MultiDragPointerStateData;

    fn initial_position(self: Handle<Self>, app: &App) -> Offset {
        app.get(self).pointer_state().initial_position
    }

    fn pending_delta(self: Handle<Self>, app: &App) -> Option<Offset> {
        app.get(self).pointer_state().pending_delta
    }

    /// Resolve this pointer's entry in the gesture arena with the given disposition.
    fn resolve(self: Handle<Self>, app: &mut App, disposition: GestureDisposition) {
        let entry = app.get(self).pointer_state().arena_entry.clone().unwrap();
        entry.resolve(app, disposition);
    }

    /// Override this to call resolve() if the drag should be accepted or rejected.
    fn check_for_resolution_after_move(self: Handle<Self>, _app: &mut App) {}

    /// Called when the gesture was accepted. Call `starter` to start the drag.
    fn accepted(self: Handle<Self>, app: &mut App, starter: GestureMultiDragStartCallback);

    fn rejected(self: Handle<Self>, app: &mut App) {
        let state = app.get_mut(self).pointer_state_mut();
        debug_assert!(state.arena_entry.is_some());
        debug_assert!(state.client.is_none());
        debug_assert!(state.pending_delta.is_some());
        state.pending_delta = None;
        state.last_pending_event_timestamp = None;
        state.arena_entry = None;
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        dispose_pointer_state(self, app);
    }

    fn as_pointer_state(self: Handle<Self>) -> AnyMultiDragPointerState {
        AnyMultiDragPointerState {
            id: self.id(),
            table: const { &PointerStateVTable::of::<Self>() },
        }
    }
}

fn dispose_pointer_state<P: MultiDragPointerState>(this: Handle<P>, app: &mut App) {
    let entry = app.get(this).pointer_state().arena_entry.clone();
    if let Some(entry) = entry {
        entry.resolve(app, GestureDisposition::Rejected);
    }
    let state = app.get_mut(this).pointer_state_mut();
    state.arena_entry = None;
    if cfg!(debug_assertions) {
        state.pending_delta = None;
    }
    app.destroy(this);
}

fn move_pointer<P: MultiDragPointerState>(this: Handle<P>, app: &mut App, event: PointerMoveEvent) {
    let state = app.get_mut(this).pointer_state_mut();
    debug_assert!(state.arena_entry.is_some());
    if !event.synthesized {
        state
            .velocity_tracker
            .add_position(event.time_stamp, event.position);
    }
    if let Some(client) = state.client.clone() {
        debug_assert!(state.pending_delta.is_none());
        client.update(
            app,
            DragUpdateDetails::new(
                event.position,
                None,
                Some(event.time_stamp),
                event.delta,
                None,
                None,
            ),
        );
    } else {
        state.pending_delta = Some(state.pending_delta.unwrap() + event.delta);
        state.last_pending_event_timestamp = Some(event.time_stamp);
        this.check_for_resolution_after_move(app);
    }
}

fn start_pointer<P: MultiDragPointerState>(this: Handle<P>, app: &mut App, client: Rc<dyn Drag>) {
    let state = app.get_mut(this).pointer_state_mut();
    debug_assert!(state.arena_entry.is_some());
    debug_assert!(state.client.is_none());
    state.client = Some(client.clone());
    let details = DragUpdateDetails::new(
        state.initial_position,
        None,
        state.last_pending_event_timestamp.take(),
        state.pending_delta.take().unwrap(),
        None,
        None,
    );
    client.update(app, details);
}

fn end_pointer<P: MultiDragPointerState>(this: Handle<P>, app: &mut App, cancel: bool) {
    let state = app.get_mut(this).pointer_state_mut();
    debug_assert!(state.arena_entry.is_some());
    if let Some(client) = state.client.take() {
        debug_assert!(state.pending_delta.is_none());
        let velocity = state.velocity_tracker.get_velocity();
        if cancel {
            client.cancel(app);
        } else {
            client.end(app, DragEndDetails::new(Offset::ZERO, None, velocity, None));
        }
    } else {
        debug_assert!(state.pending_delta.is_some());
        state.pending_delta = None;
        state.last_pending_event_timestamp = None;
    }
}

#[derive(Clone, Copy)]
pub struct AnyMultiDragPointerState {
    id: HandleId,
    table: &'static PointerStateVTable,
}

struct PointerStateVTable {
    set_entry: fn(&mut App, HandleId, GestureArenaEntry),
    moved: fn(&mut App, HandleId, PointerMoveEvent),
    accepted: fn(&mut App, HandleId, GestureMultiDragStartCallback),
    start: fn(&mut App, HandleId, Rc<dyn Drag>),
    rejected: fn(&mut App, HandleId),
    end: fn(&mut App, HandleId, bool),
    dispose: fn(&mut App, HandleId),
}

impl PointerStateVTable {
    const fn of<P: MultiDragPointerState>() -> Self {
        Self {
            set_entry: |app, id, entry| {
                let state = app.get_mut(Handle::<P>::from_id(id)).pointer_state_mut();
                debug_assert!(state.arena_entry.is_none());
                state.arena_entry = Some(entry);
            },
            moved: |app, id, event| move_pointer(Handle::<P>::from_id(id), app, event),
            accepted: |app, id, start| P::accepted(Handle::from_id(id), app, start),
            start: |app, id, client| start_pointer(Handle::<P>::from_id(id), app, client),
            rejected: |app, id| P::rejected(Handle::from_id(id), app),
            end: |app, id, cancel| end_pointer(Handle::<P>::from_id(id), app, cancel),
            dispose: |app, id| P::dispose(Handle::from_id(id), app),
        }
    }
}

/// Recognizes movement on a per-pointer basis.
///
/// Unlike one-sequence recognizers, multiple drags can be recognized concurrently.
pub struct MultiDragGestureRecognizerData {
    on_start: Option<GestureMultiDragStartCallback>,
    pointers: Option<HashMap<i64, AnyMultiDragPointerState>>,
}

impl Default for MultiDragGestureRecognizerData {
    fn default() -> Self {
        Self {
            on_start: None,
            pointers: Some(HashMap::new()),
        }
    }
}

pub trait MultiDragGestureRecognizer: RecognizerLeaf {
    fn multi_drag(&self) -> &MultiDragGestureRecognizerData;

    fn multi_drag_mut(&mut self) -> &mut MultiDragGestureRecognizerData;

    /// Create per-pointer state to track the pointer associated with the given event.
    fn create_new_pointer_state(
        self: Handle<Self>,
        app: &mut App,
        event: PointerDownEvent,
    ) -> AnyMultiDragPointerState;

    fn on_start(self: Handle<Self>, app: &App) -> Option<GestureMultiDragStartCallback> {
        app.get(self).multi_drag().on_start.clone()
    }

    fn set_on_start(
        self: Handle<Self>,
        app: &mut App,
        value: Option<GestureMultiDragStartCallback>,
    ) {
        app.get_mut(self).multi_drag_mut().on_start = value;
    }

    fn set_gesture_settings(
        self: Handle<Self>,
        app: &mut App,
        value: Option<DeviceGestureSettings>,
    ) {
        app.get_mut(self)
            .recognizer_mut()
            .set_gesture_settings(value);
    }

    fn set_supported_devices(
        self: Handle<Self>,
        app: &mut App,
        value: Option<HashSet<PointerDeviceKind>>,
    ) {
        app.get_mut(self).recognizer_mut().supported_devices = value;
    }

    fn set_allowed_buttons_filter(self: Handle<Self>, app: &mut App, value: AllowedButtonsFilter) {
        app.get_mut(self)
            .recognizer_mut()
            .set_allowed_buttons_filter(value);
    }
}

fn add_allowed_pointer<R: MultiDragGestureRecognizer>(
    this: Handle<R>,
    app: &mut App,
    event: PointerDownEvent,
) {
    let pointer = event.pointer;
    debug_assert!(
        !app.get(this)
            .multi_drag()
            .pointers
            .as_ref()
            .unwrap()
            .contains_key(&pointer)
    );
    let state = this.create_new_pointer_state(app, event);
    app.get_mut(this)
        .multi_drag_mut()
        .pointers
        .as_mut()
        .unwrap()
        .insert(pointer, state);
    let binding = GestureBinding::instance(app);
    binding.pointer_router(app).add_route(
        app,
        pointer,
        PointerRoute::handle_method(this, R::handle_event_route),
        None,
    );
    let entry = binding.gesture_arena(app).add(app, pointer, this);
    (state.table.set_entry)(app, state.id, entry);
}

fn handle_event<R: MultiDragGestureRecognizer>(
    this: Handle<R>,
    app: &mut App,
    event: PointerEvent,
) {
    let pointer = event.pointer();
    let state = app.get(this).multi_drag().pointers.as_ref().unwrap()[&pointer];
    // Dart keeps the receiver alive if a client's callback disposes the recognizer.
    let _retained = app.retain(this);
    let _state_retained = app.retain(state.id);
    match event {
        PointerEvent::Move(event) => (state.table.moved)(app, state.id, event),
        PointerEvent::Up(_) | PointerEvent::Cancel(_) => {
            (state.table.end)(app, state.id, matches!(event, PointerEvent::Cancel(_)));
            remove_state(this, app, pointer);
        }
        PointerEvent::Down(_) => {}
        _ => debug_assert!(false),
    }
}

fn accept_gesture<R: MultiDragGestureRecognizer>(this: Handle<R>, app: &mut App, pointer: i64) {
    let state = app
        .get(this)
        .multi_drag()
        .pointers
        .as_ref()
        .unwrap()
        .get(&pointer)
        .copied();
    if let Some(state) = state {
        (state.table.accepted)(
            app,
            state.id,
            Rc::new(move |app, position| start_drag(this, app, position, pointer)),
        );
    }
}

fn start_drag<R: MultiDragGestureRecognizer>(
    this: Handle<R>,
    app: &mut App,
    position: Offset,
    pointer: i64,
) -> Option<Rc<dyn Drag>> {
    let state = app.get(this).multi_drag().pointers.as_ref().unwrap()[&pointer];
    let _retained = app.retain(this);
    let _state_retained = app.retain(state.id);
    let drag = this.on_start(app).and_then(|callback| {
        GestureRecognizer::invoke_callback(this, app, "onStart", |app| callback(app, position))
            .flatten()
    });
    if let Some(drag) = drag.clone() {
        (state.table.start)(app, state.id, drag);
    } else {
        remove_state(this, app, pointer);
    }
    drag
}

fn reject_gesture<R: MultiDragGestureRecognizer>(this: Handle<R>, app: &mut App, pointer: i64) {
    let state = app
        .get(this)
        .multi_drag()
        .pointers
        .as_ref()
        .unwrap()
        .get(&pointer)
        .copied();
    if let Some(state) = state {
        (state.table.rejected)(app, state.id);
        remove_state(this, app, pointer);
    }
}

fn remove_state<R: MultiDragGestureRecognizer>(this: Handle<R>, app: &mut App, pointer: i64) {
    let Some(pointers) = app.get_mut(this).multi_drag_mut().pointers.as_mut() else {
        return;
    };
    let state = pointers.remove(&pointer).expect("tracked pointer");
    let router = GestureBinding::instance(app).pointer_router(app);
    router.remove_route(
        app,
        pointer,
        &PointerRoute::handle_method(this, R::handle_event_route),
    );
    (state.table.dispose)(app, state.id);
}

fn dispose_recognizer<R: MultiDragGestureRecognizer>(this: Handle<R>, app: &mut App) {
    let pointers: Vec<_> = app
        .get(this)
        .multi_drag()
        .pointers
        .as_ref()
        .unwrap()
        .keys()
        .copied()
        .collect();
    for pointer in pointers {
        remove_state(this, app, pointer);
    }
    app.get_mut(this).multi_drag_mut().pointers = None;
    GestureRecognizer::dispose(this, app);
}

macro_rules! pointer_state {
    ($name:ident, $distance:expr) => {
        struct $name {
            state: MultiDragPointerStateData,
        }

        impl MultiDragPointerState for $name {
            fn pointer_state(&self) -> &MultiDragPointerStateData {
                &self.state
            }

            fn pointer_state_mut(&mut self) -> &mut MultiDragPointerStateData {
                &mut self.state
            }

            fn check_for_resolution_after_move(self: Handle<Self>, app: &mut App) {
                let state = &app.get(self).state;
                if ($distance)(state.pending_delta.unwrap())
                    > compute_hit_slop(state.kind, state.gesture_settings)
                {
                    self.resolve(app, GestureDisposition::Accepted);
                }
            }

            fn accepted(self: Handle<Self>, app: &mut App, starter: GestureMultiDragStartCallback) {
                starter(app, self.initial_position(app));
            }
        }
    };
}

pointer_state!(ImmediatePointerState, |offset: Offset| offset.distance());
pointer_state!(HorizontalPointerState, |offset: Offset| offset.dx().abs());
pointer_state!(VerticalPointerState, |offset: Offset| offset.dy().abs());

struct DelayedPointerState {
    state: MultiDragPointerStateData,
    timer: Option<Timer>,
    starter: Option<GestureMultiDragStartCallback>,
}

impl DelayedPointerState {
    fn new(
        app: &mut App,
        initial_position: Offset,
        delay: Duration,
        kind: PointerDeviceKind,
        settings: Option<DeviceGestureSettings>,
    ) -> Handle<Self> {
        let this = app.create(Self {
            state: MultiDragPointerStateData::new(initial_position, kind, settings),
            timer: None,
            starter: None,
        });
        let timer = Timer::new(
            app,
            delay,
            Listener::handle_method(this, Self::delay_passed),
        );
        app.get_mut(this).timer = Some(timer);
        this
    }

    fn delay_passed(self: Handle<Self>, app: &mut App) {
        let _retained = app.retain(self);
        debug_assert!(app.get(self).timer.is_some());
        app.get_mut(self).timer = None;
        if let Some(starter) = app.get(self).starter.clone() {
            starter(app, self.initial_position(app));
            app.get_mut(self).starter = None;
        } else {
            self.resolve(app, GestureDisposition::Accepted);
        }
    }

    fn ensure_timer_stopped(self: Handle<Self>, app: &mut App) {
        if let Some(timer) = app.get_mut(self).timer.take() {
            timer.cancel(app);
        }
    }
}

impl MultiDragPointerState for DelayedPointerState {
    fn pointer_state(&self) -> &MultiDragPointerStateData {
        &self.state
    }

    fn pointer_state_mut(&mut self) -> &mut MultiDragPointerStateData {
        &mut self.state
    }

    fn accepted(self: Handle<Self>, app: &mut App, starter: GestureMultiDragStartCallback) {
        debug_assert!(app.get(self).starter.is_none());
        if app.get(self).timer.is_none() {
            starter(app, self.initial_position(app));
        } else {
            app.get_mut(self).starter = Some(starter);
        }
    }

    fn check_for_resolution_after_move(self: Handle<Self>, app: &mut App) {
        if app.get(self).timer.is_none() {
            debug_assert!(app.get(self).starter.is_some());
            return;
        }
        let state = &app.get(self).state;
        if state.pending_delta.unwrap().distance()
            > compute_hit_slop(state.kind, state.gesture_settings)
        {
            let _retained = app.retain(self);
            self.resolve(app, GestureDisposition::Rejected);
            self.ensure_timer_stopped(app);
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.ensure_timer_stopped(app);
        dispose_pointer_state(self, app);
    }
}

macro_rules! recognizer {
    ($name:ident, $description:literal, $create:expr $(, $field:ident: $ty:ty = $default:expr)*) => {
        pub struct $name {
            recognizer: GestureRecognizerData,
            multi_drag: MultiDragGestureRecognizerData,
            $($field: $ty,)*
        }

        impl $name {
            pub fn new(app: &mut App) -> Handle<Self> {
                let mut recognizer = GestureRecognizerData::new();
                recognizer.set_allowed_buttons_filter(Rc::new(|buttons| buttons == K_PRIMARY_BUTTON));
                app.create(Self {
                    recognizer,
                    multi_drag: MultiDragGestureRecognizerData::default(),
                    $($field: $default,)*
                })
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

        impl MultiDragGestureRecognizer for $name {
            fn multi_drag(&self) -> &MultiDragGestureRecognizerData {
                &self.multi_drag
            }

            fn multi_drag_mut(&mut self) -> &mut MultiDragGestureRecognizerData {
                &mut self.multi_drag
            }

            fn create_new_pointer_state(
                self: Handle<Self>,
                app: &mut App,
                event: PointerDownEvent,
            ) -> AnyMultiDragPointerState {
                ($create)(self, app, event)
            }
        }

        impl RecognizerLeaf for $name {
            fn add_allowed_pointer(self: Handle<Self>, app: &mut App, event: PointerDownEvent) {
                add_allowed_pointer(self, app, event);
            }

            fn handle_event(self: Handle<Self>, app: &mut App, event: PointerEvent) {
                handle_event(self, app, event);
            }

            fn accept_gesture(self: Handle<Self>, app: &mut App, pointer: i64) {
                accept_gesture(self, app, pointer);
            }

            fn reject_gesture(self: Handle<Self>, app: &mut App, pointer: i64) {
                reject_gesture(self, app, pointer);
            }

            fn dispose(self: Handle<Self>, app: &mut App) {
                dispose_recognizer(self, app);
            }

            fn debug_description(self: Handle<Self>) -> &'static str {
                $description
            }
        }
    };
}

recognizer!(
    ImmediateMultiDragGestureRecognizer,
    "multidrag",
    |this: Handle<ImmediateMultiDragGestureRecognizer>, app: &mut App, event: PointerDownEvent| {
        let state = MultiDragPointerStateData::new(
            event.position,
            event.kind,
            app.get(this).recognizer.gesture_settings,
        );
        app.create(ImmediatePointerState { state })
            .as_pointer_state()
    }
);

recognizer!(
    HorizontalMultiDragGestureRecognizer,
    "horizontal multidrag",
    |this: Handle<HorizontalMultiDragGestureRecognizer>, app: &mut App, event: PointerDownEvent| {
        let state = MultiDragPointerStateData::new(
            event.position,
            event.kind,
            app.get(this).recognizer.gesture_settings,
        );
        app.create(HorizontalPointerState { state })
            .as_pointer_state()
    }
);

recognizer!(
    VerticalMultiDragGestureRecognizer,
    "vertical multidrag",
    |this: Handle<VerticalMultiDragGestureRecognizer>, app: &mut App, event: PointerDownEvent| {
        let state = MultiDragPointerStateData::new(
            event.position,
            event.kind,
            app.get(this).recognizer.gesture_settings,
        );
        app.create(VerticalPointerState { state })
            .as_pointer_state()
    }
);

recognizer!(
    DelayedMultiDragGestureRecognizer,
    "long multidrag",
    |this: Handle<DelayedMultiDragGestureRecognizer>, app: &mut App, event: PointerDownEvent| {
        DelayedPointerState::new(
            app,
            event.position,
            app.get(this).delay,
            event.kind,
            app.get(this).recognizer.gesture_settings,
        )
        .as_pointer_state()
    },
    delay: Duration = K_LONG_PRESS_TIMEOUT
);

impl DelayedMultiDragGestureRecognizer {
    pub fn delay(self: Handle<Self>, app: &App) -> Duration {
        app.get(self).delay
    }

    pub fn set_delay(self: Handle<Self>, app: &mut App, value: Duration) {
        app.get_mut(self).delay = value;
    }
}
