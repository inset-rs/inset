//! Flutter counterpart: `scheduler/binding.dart`.

use std::cell::Cell;
use std::collections::HashSet;
use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use indexmap::IndexMap;
use inset_embedder::{AppLifecycleState, ViewId};
use inset_foundation::{App, Handle, Listener, Timer};

/// Signature for frame-related callbacks from the scheduler.
///
/// The `time_stamp` is the number of milliseconds since the beginning of the
/// scheduler's epoch. Use time_stamp to determine how far to advance animation
/// timelines so that all the animations in the system are synchronized to a
/// common time base.
///
/// Receives [`App`] because a Rust callback cannot own what it mutates.
#[derive(Clone)]
pub struct FrameCallback(Rc<TimedCallback>);

type TimedCallback = dyn Fn(&mut App, Duration);

impl FrameCallback {
    /// Wraps a closure as a frame callback.
    pub fn new(callback: impl Fn(&mut App, Duration) + 'static) -> FrameCallback {
        FrameCallback(Rc::new(callback))
    }

    /// A handle's associated function — Dart passing a method tear-off like
    /// `Ticker._tick`.
    pub fn handle_method<T: 'static, F>(this: Handle<T>, f: F) -> FrameCallback
    where
        F: Fn(Handle<T>, &mut App, Duration) + 'static,
    {
        FrameCallback(Rc::new(move |app: &mut App, time_stamp| {
            f(this, app, time_stamp)
        }))
    }

    /// Calls the callback.
    pub fn call(&self, app: &mut App, time_stamp: Duration) {
        (self.0)(app, time_stamp);
    }
}

impl Debug for FrameCallback {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("FrameCallback")
    }
}

/// The object side of Flutter's `WidgetsBinding` override of
/// `SchedulerBinding.handleAppLifecycleStateChanged`, which tells the widget layer's observers.
/// Implement it on the binding above and register `Rc::new(handle)` with
/// [`SchedulerBinding::set_overrides`]; `Handle<T>` is then [`SchedulerBindingOverrides`].
pub trait SchedulerBindingOverridesObject: Sized + 'static {
    /// The override's part of [`SchedulerBinding::handle_app_lifecycle_state_changed`], which
    /// runs after the scheduler's own, as Dart's calls `super` first.
    fn handle_app_lifecycle_state_changed(
        self: Handle<Self>,
        app: &mut App,
        state: AppLifecycleState,
    );
}

/// What the binding above this one overrides; kept erased.
pub trait SchedulerBindingOverrides {
    /// See [`SchedulerBindingOverridesObject::handle_app_lifecycle_state_changed`].
    fn handle_app_lifecycle_state_changed(&self, app: &mut App, state: AppLifecycleState);
}

impl<T: SchedulerBindingOverridesObject> SchedulerBindingOverrides for Handle<T> {
    fn handle_app_lifecycle_state_changed(&self, app: &mut App, state: AppLifecycleState) {
        T::handle_app_lifecycle_state_changed(*self, app, state);
    }
}

/// The various phases that a [`SchedulerBinding`] goes through during
/// [`SchedulerBinding::handle_begin_frame`].
///
/// This is exposed by [`SchedulerBinding::scheduler_phase`].
///
/// The values of this enum are ordered in the same order as the phases occur,
/// so their relative order can be compared to each other.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SchedulerPhase {
    /// No frame is being processed. Tasks, microtasks, event handlers, and
    /// other callbacks may be executing.
    Idle,

    /// The transient callbacks (scheduled by
    /// [`SchedulerBinding::schedule_frame_callback`]) are currently executing.
    ///
    /// Typically, these callbacks handle updating objects to new animation
    /// states.
    TransientCallbacks,

    /// Microtasks scheduled during the processing of transient callbacks are
    /// currently executing.
    MidFrameMicrotasks,

    /// The persistent callbacks (scheduled by
    /// [`SchedulerBinding::add_persistent_frame_callback`]) are currently
    /// executing.
    ///
    /// Typically, this is the build/layout/paint pipeline.
    PersistentCallbacks,

    /// The post-frame callbacks (scheduled by
    /// [`SchedulerBinding::add_post_frame_callback`]) are currently executing.
    ///
    /// Typically, these callbacks handle cleanup and scheduling of work for
    /// the next frame.
    PostFrameCallbacks,
}

/// Signature for the callbacks [`SchedulerBinding::add_persistent_frame_callback`] registers: they
/// draw a refresh's frame of the views given, at the time stamp given.
///
/// Receives [`App`] because a Rust callback cannot own what it mutates.
#[derive(Clone)]
pub struct PersistentFrameCallback(Rc<DrawingCallback>);

type DrawingCallback = dyn Fn(&mut App, &[ViewId], Duration);

impl PersistentFrameCallback {
    /// Wraps a closure as a persistent frame callback.
    pub fn new(callback: impl Fn(&mut App, &[ViewId], Duration) + 'static) -> Self {
        PersistentFrameCallback(Rc::new(callback))
    }

    /// Calls the callback.
    pub fn call(&self, app: &mut App, views: &[ViewId], time_stamp: Duration) {
        (self.0)(app, views, time_stamp);
    }
}

impl Debug for PersistentFrameCallback {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PersistentFrameCallback")
    }
}

/// The views a request for a frame is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameTarget {
    /// One view; a view the platform no longer has counts as none, so as every view.
    View(ViewId),
    /// Every view the platform has, each at its own screen's next refresh.
    EveryView,
}

/// Dart's `_FrameCallbackEntry`, with the view whose frame runs the callback; `None` for no view.
struct FrameCallbackEntry {
    callback: FrameCallback,
    view: Option<ViewId>,
}

/// A post-frame callback and the view whose frame runs it; `None` for no view.
struct PostFrameCallbackEntry {
    callback: FrameCallback,
    view: Option<ViewId>,
}

/// Scheduler for running the following:
///
/// * _Transient callbacks_, triggered by the system's begin-frame callback,
///   for synchronizing the application's behavior to the system's display. For
///   example, [`crate::Ticker`]s and `AnimationController`s trigger from these.
///
/// * _Persistent callbacks_, triggered by the system's draw-frame callback,
///   for updating the system's display after transient callbacks have executed.
///   For example, the rendering layer uses this to drive its rendering
///   pipeline.
///
/// * _Post-frame callbacks_, which are run after persistent callbacks, just
///   before returning from the draw-frame callback.
///
/// A frame is a screen's refresh, of the views on that screen that asked for one: a transient or
/// post-frame callback runs at the first frame of its view, and one of no view at the next frame
/// of any. Frames never overlap.
///
/// Dart's `SchedulerBinding.instance` is [`SchedulerBinding::instance`], the
/// App's singleton; members take `&mut App`.
pub struct SchedulerBinding {
    next_frame_callback_id: i64,
    // Insertion-ordered, as Dart's `Map` is: the relative order of two
    // tickers' callbacks is dispatch order.
    transient_callbacks: IndexMap<i64, FrameCallbackEntry>,
    removed_ids: HashSet<i64>,
    persistent_callbacks: Vec<PersistentFrameCallback>,
    post_frame_callbacks: Vec<PostFrameCallbackEntry>,
    /// Dart's `_hasScheduledFrame`, for each view: the views a frame was asked of whose refresh
    /// has not begun.
    views_with_scheduled_frame: Vec<ViewId>,
    /// A warm-up frame is scheduled or running: the host's frames wait for it.
    warm_up_frame: bool,
    /// The views of the refresh the host gave while the warm-up frame ran, whose frames are asked
    /// for again after it.
    reschedule_after_warm_up_frame: Option<Vec<ViewId>>,
    // Detached from the arena so the frame handlers' `finally` guards can
    // reach them without the App.
    scheduler_phase: Rc<Cell<SchedulerPhase>>,
    /// The views of the frame under way; read only while one is.
    current_frame_views: Vec<ViewId>,
    lifecycle_state: Option<AppLifecycleState>,
    frames_enabled: bool,
    time_dilation: f64,
    first_raw_time_stamp_in_epoch: Option<Duration>,
    epoch_start: Duration,
    last_raw_time_stamp: Duration,
    current_frame_time_stamp: Rc<Cell<Option<Duration>>>,
    overrides: Option<Rc<dyn SchedulerBindingOverrides>>,
}

/// Dart's `finally` blocks in `handleBeginFrame` and `handleDrawFrame`: the
/// phase advance (and, for the draw handler, the time-stamp clear) runs even
/// when a frame callback panics.
struct FrameFinally {
    phase: Rc<Cell<SchedulerPhase>>,
    on_drop: SchedulerPhase,
    time_stamp: Option<Rc<Cell<Option<Duration>>>>,
}

impl Drop for FrameFinally {
    fn drop(&mut self) {
        self.phase.set(self.on_drop);
        if let Some(time_stamp) = &self.time_stamp {
            time_stamp.set(None);
        }
    }
}

impl Default for SchedulerBinding {
    fn default() -> SchedulerBinding {
        SchedulerBinding {
            next_frame_callback_id: 0,
            transient_callbacks: IndexMap::new(),
            removed_ids: HashSet::new(),
            persistent_callbacks: Vec::new(),
            post_frame_callbacks: Vec::new(),
            views_with_scheduled_frame: Vec::new(),
            warm_up_frame: false,
            reschedule_after_warm_up_frame: None,
            scheduler_phase: Rc::new(Cell::new(SchedulerPhase::Idle)),
            current_frame_views: Vec::new(),
            lifecycle_state: None,
            frames_enabled: true,
            time_dilation: 1.0,
            first_raw_time_stamp_in_epoch: None,
            epoch_start: Duration::ZERO,
            last_raw_time_stamp: Duration::ZERO,
            current_frame_time_stamp: Rc::new(Cell::new(None)),
            overrides: None,
        }
    }
}

impl SchedulerBinding {
    /// The current [`SchedulerBinding`], if one has been created.
    ///
    /// Dart's `SchedulerBinding.instance`.
    pub fn instance(app: &mut App) -> Handle<SchedulerBinding> {
        app.singleton::<SchedulerBinding>()
    }

    /// Registers the binding layered above this one, as Flutter's `WidgetsBinding` does by
    /// overriding [`handle_app_lifecycle_state_changed`](Self::handle_app_lifecycle_state_changed).
    pub fn set_overrides(app: &mut App, overrides: Rc<dyn SchedulerBindingOverrides>) {
        let this = SchedulerBinding::instance(app);
        app.get_mut(this).overrides = Some(overrides);
    }

    /// Whether the application is visible, and if so, whether it is currently
    /// interactive.
    ///
    /// This is set by [`handle_app_lifecycle_state_changed`](SchedulerBinding::handle_app_lifecycle_state_changed)
    /// when the host reports a lifecycle change through `EmbedderClient`.
    ///
    /// The preferred ways to watch for changes to this value are using
    /// `WidgetsBindingObserver::did_change_app_lifecycle_state`, or through an
    /// `AppLifecycleListener` object.
    pub fn lifecycle_state(app: &mut App) -> Option<AppLifecycleState> {
        let this = SchedulerBinding::instance(app);
        app.get(this).lifecycle_state
    }

    /// Called when the application lifecycle state changes.
    ///
    /// Notifies all the observers using
    /// `WidgetsBindingObserver::did_change_app_lifecycle_state`.
    ///
    /// This method exposes notifications from `SystemChannels.lifecycle`, which the host
    /// sends through `EmbedderClient::app_lifecycle_state_changed`.
    ///
    /// It records the state and turns frames on or off, then runs the override the widgets
    /// binding registered, which tells the observers, as Dart's does after its `super` call.
    pub fn handle_app_lifecycle_state_changed(app: &mut App, state: AppLifecycleState) {
        let this = SchedulerBinding::instance(app);
        SchedulerBinding::record_app_lifecycle_state(app, state);
        let overrides = app.get(this).overrides.clone();
        if let Some(overrides) = overrides {
            overrides.handle_app_lifecycle_state_changed(app, state);
        }
    }

    /// The scheduler's own body of `handleAppLifecycleStateChanged`, which the override's
    /// `super` call reaches.
    fn record_app_lifecycle_state(app: &mut App, state: AppLifecycleState) {
        let this = SchedulerBinding::instance(app);
        if app.get(this).lifecycle_state == Some(state) {
            return;
        }
        app.get_mut(this).lifecycle_state = Some(state);
        match state {
            AppLifecycleState::Resumed | AppLifecycleState::Inactive => {
                SchedulerBinding::set_frames_enabled_state(app, true);
            }
            AppLifecycleState::Hidden | AppLifecycleState::Paused | AppLifecycleState::Detached => {
                SchedulerBinding::set_frames_enabled_state(app, false);
            }
        }
    }

    /// The current number of transient frame callbacks scheduled.
    ///
    /// This is reset to zero just before all the currently scheduled
    /// transient callbacks are called, at the start of a frame.
    pub fn transient_callback_count(app: &mut App) -> usize {
        let this = SchedulerBinding::instance(app);
        app.get(this).transient_callbacks.len()
    }

    /// Schedules the given transient frame callback.
    ///
    /// Adds the given callback to the list of frame callbacks, and ensures
    /// that a frame is scheduled if `schedule_new_frame` is true.
    ///
    /// If this is called during the frame's animation phase (when transient
    /// frame callbacks are still being invoked), `callback` will be called in
    /// the next frame, not in the current frame.
    ///
    /// The callback runs at the first frame of `view`, or of any view for `None`.
    ///
    /// Callbacks registered with this method can be canceled using
    /// [`cancel_frame_callback_with_id`](SchedulerBinding::cancel_frame_callback_with_id).
    pub fn schedule_frame_callback(
        app: &mut App,
        callback: FrameCallback,
        view: Option<ViewId>,
        _rescheduling: bool,
        schedule_new_frame: bool,
    ) -> i64 {
        if schedule_new_frame {
            let target = match view {
                Some(view) => FrameTarget::View(view),
                None => FrameTarget::EveryView,
            };
            SchedulerBinding::schedule_frame(app, target);
        }
        let this = SchedulerBinding::instance(app);
        let binding = app.get_mut(this);
        binding.next_frame_callback_id += 1;
        let id = binding.next_frame_callback_id;
        binding
            .transient_callbacks
            .insert(id, FrameCallbackEntry { callback, view });
        id
    }

    /// Cancels the transient frame callback with the given `id`.
    ///
    /// Removes the given callback from the list of frame callbacks. If a
    /// frame has been requested, this does not also cancel that request.
    pub fn cancel_frame_callback_with_id(app: &mut App, id: i64) {
        debug_assert!(id > 0);
        let this = SchedulerBinding::instance(app);
        let binding = app.get_mut(this);
        binding.transient_callbacks.shift_remove(&id);
        binding.removed_ids.insert(id);
    }

    /// Asserts that there are no registered transient callbacks.
    ///
    /// Does nothing if asserts are disabled. Always returns true.
    pub fn debug_assert_no_transient_callbacks(app: &mut App, reason: &str) -> bool {
        debug_assert!(
            SchedulerBinding::transient_callback_count(app) == 0,
            "{reason}"
        );
        true
    }

    /// Asserts that there is no artificial time dilation in debug mode.
    ///
    /// Always returns true.
    pub fn debug_assert_no_time_dilation(app: &mut App, reason: &str) -> bool {
        debug_assert!(SchedulerBinding::time_dilation(app) == 1.0, "{reason}");
        true
    }

    /// Adds a persistent frame callback.
    ///
    /// Persistent callbacks are called after transient (non-persistent) frame
    /// callbacks.
    ///
    /// Does *not* request a new frame. Persistent frame callbacks cannot be
    /// unregistered. Once registered, they are called for every frame for the
    /// lifetime of the application.
    pub fn add_persistent_frame_callback(app: &mut App, callback: PersistentFrameCallback) {
        let this = SchedulerBinding::instance(app);
        app.get_mut(this).persistent_callbacks.push(callback);
    }

    /// Schedule a callback for the end of a frame of `view`, or of any view for `None`.
    ///
    /// The provided callback is run immediately after a frame, just after the
    /// persistent frame callbacks (which is when the main rendering pipeline
    /// has been flushed).
    ///
    /// This method does *not* request a new frame. If a frame of `view` is already in
    /// progress and the execution of post-frame callbacks has not yet begun,
    /// then the registered callback is executed at the end of the current
    /// frame. Otherwise, the registered callback is executed after the next
    /// frame (whenever that may be, if ever).
    ///
    /// A frame runs the callbacks of its views, then those of no view, each in the order in
    /// which they have been added. Post-frame callbacks cannot be unregistered. They are called
    /// exactly once.
    pub fn add_post_frame_callback(app: &mut App, callback: FrameCallback, view: Option<ViewId>) {
        let this = SchedulerBinding::instance(app);
        app.get_mut(this)
            .post_frame_callbacks
            .push(PostFrameCallbackEntry { callback, view });
    }

    /// Whether this scheduler has requested that
    /// [`handle_begin_frame`](SchedulerBinding::handle_begin_frame) be called
    /// soon, for a view the platform has.
    pub fn has_scheduled_frame(app: &mut App) -> bool {
        let this = SchedulerBinding::instance(app);
        let app = &*app;
        app.get(this)
            .views_with_scheduled_frame
            .iter()
            .any(|&view| SchedulerBinding::existing_view(app, Some(view)).is_some())
    }

    /// The phase that the scheduler is currently operating under.
    pub fn scheduler_phase(app: &mut App) -> SchedulerPhase {
        let this = SchedulerBinding::instance(app);
        app.get(this).scheduler_phase.get()
    }

    /// Whether frames are currently being scheduled when
    /// [`schedule_frame`](SchedulerBinding::schedule_frame) is called.
    ///
    /// This value depends on the value of the
    /// [`lifecycle_state`](SchedulerBinding::lifecycle_state).
    pub fn frames_enabled(app: &mut App) -> bool {
        let this = SchedulerBinding::instance(app);
        app.get(this).frames_enabled
    }

    fn set_frames_enabled_state(app: &mut App, enabled: bool) {
        let this = SchedulerBinding::instance(app);
        if app.get(this).frames_enabled == enabled {
            return;
        }
        app.get_mut(this).frames_enabled = enabled;
        if enabled {
            SchedulerBinding::schedule_frame(app, FrameTarget::EveryView);
        }
    }

    /// Schedules a new frame of `target` using
    /// [`schedule_frame`](SchedulerBinding::schedule_frame) for each of its views whose frame is
    /// not currently being produced.
    ///
    /// A view's frame is being produced while
    /// [`scheduler_phase`](SchedulerBinding::scheduler_phase) is
    /// [`SchedulerPhase::TransientCallbacks`] or [`SchedulerPhase::MidFrameMicrotasks`]
    /// (because the frame is already being prepared in that case), or
    /// [`SchedulerPhase::PersistentCallbacks`] (because the frame is actively being rendered in
    /// that case), and the frame is of that view. A frame of other views does not stand for one
    /// of it.
    pub fn ensure_visual_update(app: &mut App, target: FrameTarget) {
        let this = SchedulerBinding::instance(app);
        let in_frame = match app.get(this).scheduler_phase.get() {
            SchedulerPhase::Idle | SchedulerPhase::PostFrameCallbacks => Vec::new(),
            SchedulerPhase::TransientCallbacks
            | SchedulerPhase::MidFrameMicrotasks
            | SchedulerPhase::PersistentCallbacks => app.get(this).current_frame_views.clone(),
        };
        let views: Vec<ViewId> = SchedulerBinding::views_of(app, target)
            .into_iter()
            .filter(|view| !in_frame.contains(view))
            .collect();
        if SchedulerBinding::frames_enabled(app) {
            SchedulerBinding::request_frames(app, &views);
        }
    }

    /// If necessary, schedules a new frame of each view `target` names.
    pub fn schedule_frame(app: &mut App, target: FrameTarget) {
        if !SchedulerBinding::frames_enabled(app) {
            return;
        }
        let views = SchedulerBinding::views_of(app, target);
        SchedulerBinding::request_frames(app, &views);
    }

    /// Asks the host for a frame of each of `views` that has none asked for.
    fn request_frames(app: &mut App, views: &[ViewId]) {
        let this = SchedulerBinding::instance(app);
        for &view in views {
            if app.get(this).views_with_scheduled_frame.contains(&view) {
                continue;
            }
            app.platform().request_frame(view);
            app.get_mut(this).views_with_scheduled_frame.push(view);
        }
    }

    /// The views of the platform's that `target` names.
    fn views_of(app: &App, target: FrameTarget) -> Vec<ViewId> {
        let view = match target {
            FrameTarget::View(view) => SchedulerBinding::existing_view(app, Some(view)),
            FrameTarget::EveryView => None,
        };
        match view {
            Some(view) => vec![view],
            None => app
                .platform()
                .views()
                .iter()
                .map(|view| view.id())
                .collect(),
        }
    }

    /// `view`, where the platform has it: a view it no longer has gets no frames, so its work
    /// counts as that of no view.
    fn existing_view(app: &App, view: Option<ViewId>) -> Option<ViewId> {
        view.filter(|&view| app.platform().view(view).is_some())
    }

    /// Whether the scheduler is currently producing a warm-up frame.
    ///
    /// A warm-up frame is scheduled by
    /// [`schedule_warm_up_frame`](SchedulerBinding::schedule_warm_up_frame) and might not
    /// produce visible output on screen (e.g., if the rendering surface is not yet available).
    pub fn is_warm_up_frame(app: &mut App) -> bool {
        let this = SchedulerBinding::instance(app);
        app.get(this).warm_up_frame
    }

    /// Schedule a frame to run as soon as possible, rather than waiting for the
    /// host to request a frame in response to a system "Vsync" signal.
    ///
    /// This is used during application startup so that the first frame (which
    /// is likely to be quite expensive, being the first time the app has been
    /// rendered) can start a few milliseconds earlier.
    ///
    /// If a frame has already been scheduled with
    /// [`schedule_frame`](SchedulerBinding::schedule_frame) or
    /// [`schedule_forced_frame`](SchedulerBinding::schedule_forced_frame), this
    /// call may delay that frame.
    ///
    /// If any scheduled frame has already begun or if another
    /// [`schedule_warm_up_frame`](SchedulerBinding::schedule_warm_up_frame) was
    /// already called, this call will be ignored.
    ///
    /// Prefer [`schedule_frame`](SchedulerBinding::schedule_frame) to update the
    /// display in normal operation.
    ///
    /// The warm-up frame is a frame of every view the platform has. It runs on two zero-duration
    /// timers, "begin" and "draw", with the microtasks drained between them; the host runs the
    /// timers it owes before it takes new events, so nothing reaches the framework in between.
    /// Dart locks events for the same reason.
    pub fn schedule_warm_up_frame(app: &mut App) {
        let this = SchedulerBinding::instance(app);
        if app.get(this).warm_up_frame
            || app.get(this).scheduler_phase.get() != SchedulerPhase::Idle
        {
            return;
        }
        app.get_mut(this).warm_up_frame = true;
        let had_scheduled_frames = app.get(this).views_with_scheduled_frame.clone();
        let views = SchedulerBinding::views_of(app, FrameTarget::EveryView);
        Timer::new(
            app,
            Duration::ZERO,
            Listener::new({
                let views = views.clone();
                move |app| SchedulerBinding::handle_begin_frame(app, &views, None)
            }),
        );
        Timer::new(
            app,
            Duration::ZERO,
            Listener::new(move |app| {
                SchedulerBinding::handle_draw_frame(app, &views);
                // `reset_epoch` after this frame so that, in the hot reload case,
                // the very next frame pretends to have occurred immediately after
                // this warm-up frame. The warm-up frame's timestamp will typically
                // be far in the past (the time of the last real frame), so without
                // the reset there would be a sudden jump from the old time in the
                // warm-up frame to the new time in the "real" frame: implicit
                // animations would be triggered at the old time and then skip
                // every frame and finish in the new time.
                SchedulerBinding::reset_epoch(app);
                let this = SchedulerBinding::instance(app);
                app.get_mut(this).warm_up_frame = false;
                for &view in &had_scheduled_frames {
                    SchedulerBinding::schedule_frame(app, FrameTarget::View(view));
                }
            }),
        );
    }

    /// Dart's `_handleBeginFrame`, what `PlatformDispatcher.onBeginFrame` is
    /// set to: the host's signal that a frame of `views` begins. A warm-up frame in
    /// progress takes the frame's place, and the frame is scheduled again
    /// once it is done.
    pub fn on_begin_frame(app: &mut App, views: &[ViewId], raw_time_stamp: Option<Duration>) {
        let this = SchedulerBinding::instance(app);
        if app.get(this).warm_up_frame {
            // "begin frame" and "draw frame" must strictly alternate. Therefore
            // reschedule_after_warm_up_frame cannot possibly be set here as it
            // is reset by on_draw_frame.
            debug_assert!(app.get(this).reschedule_after_warm_up_frame.is_none());
            app.get_mut(this).reschedule_after_warm_up_frame = Some(views.to_vec());
            return;
        }
        SchedulerBinding::handle_begin_frame(app, views, raw_time_stamp);
    }

    /// Dart's `_handleDrawFrame`, what `PlatformDispatcher.onDrawFrame` is set
    /// to: the host's signal to draw the frame of `views`, after the microtasks that
    /// followed [`on_begin_frame`](SchedulerBinding::on_begin_frame).
    pub fn on_draw_frame(app: &mut App, views: &[ViewId]) {
        let this = SchedulerBinding::instance(app);
        if let Some(rescheduled) = app.get_mut(this).reschedule_after_warm_up_frame.take() {
            // Reschedule in a post-frame callback to allow the draw-frame phase
            // of the warm-up frame to finish.
            SchedulerBinding::add_post_frame_callback(
                app,
                FrameCallback::new(move |app, _time_stamp| {
                    // Force a host frame. The views' scheduled frames are cleared here
                    // because the original host frame was cancelled, and therefore
                    // `handle_begin_frame`, which is responsible for clearing them, did
                    // not run. So if a frame callback asked for one in the "so far"
                    // part of the frame, the record would be wrong and any later
                    // frame request would be dropped.
                    let this = SchedulerBinding::instance(app);
                    app.get_mut(this)
                        .views_with_scheduled_frame
                        .retain(|view| !rescheduled.contains(view));
                    for &view in &rescheduled {
                        SchedulerBinding::schedule_frame(app, FrameTarget::View(view));
                    }
                }),
                // The rescheduled views get no frame until this asks for one.
                None,
            );
            return;
        }
        SchedulerBinding::handle_draw_frame(app, views);
    }

    /// Schedules a new frame of each view `target` names even when frames would normally not be
    /// scheduled by [`schedule_frame`](SchedulerBinding::schedule_frame) (e.g. when the
    /// device's screen is turned off).
    pub fn schedule_forced_frame(app: &mut App, target: FrameTarget) {
        let views = SchedulerBinding::views_of(app, target);
        SchedulerBinding::request_frames(app, &views);
    }

    /// Slows down animations by this factor to help in development.
    ///
    /// Dart's top-level `timeDilation`.
    pub fn time_dilation(app: &mut App) -> f64 {
        let this = SchedulerBinding::instance(app);
        app.get(this).time_dilation
    }

    /// If the [`SchedulerBinding`] has been initialized, setting the time
    /// dilation automatically calls [`reset_epoch`](SchedulerBinding::reset_epoch)
    /// to ensure that time stamps seen by consumers of the scheduler binding
    /// are always increasing.
    pub fn set_time_dilation(app: &mut App, value: f64) {
        debug_assert!(value > 0.0);
        if SchedulerBinding::time_dilation(app) == value {
            return;
        }
        // If the binding has been created, we need to resetEpoch first so that
        // we capture start of the epoch with the current time dilation.
        SchedulerBinding::reset_epoch(app);
        let this = SchedulerBinding::instance(app);
        app.get_mut(this).time_dilation = value;
    }

    /// Prepares the scheduler for a non-monotonic change to how time stamps are
    /// calculated.
    ///
    /// Callbacks received from the scheduler assume that their time stamps are
    /// monotonically increasing. The raw time stamp passed to
    /// [`handle_begin_frame`](SchedulerBinding::handle_begin_frame) is
    /// monotonic, but the scheduler might adjust those time stamps to provide
    /// [`time_dilation`](SchedulerBinding::time_dilation). Without careful
    /// handling, these adjusts could cause time to appear to run backwards.
    ///
    /// Setting time dilation calls [`reset_epoch`](SchedulerBinding::reset_epoch)
    /// automatically. You don't need to call it yourself.
    pub fn reset_epoch(app: &mut App) {
        let this = SchedulerBinding::instance(app);
        let binding = app.get_mut(this);
        binding.epoch_start = binding.adjust_for_epoch(binding.last_raw_time_stamp);
        binding.first_raw_time_stamp_in_epoch = None;
    }

    /// Adjusts the given time stamp into the current epoch.
    ///
    /// This both offsets the time stamp to account for when the epoch started
    /// (both in raw time and in the epoch's own time line) and scales the time
    /// stamp to reflect the time dilation in the current epoch.
    fn adjust_for_epoch(&self, raw_time_stamp: Duration) -> Duration {
        let raw_duration_since_epoch = match self.first_raw_time_stamp_in_epoch {
            None => Duration::ZERO,
            Some(first) => raw_time_stamp - first,
        };
        let micros =
            (raw_duration_since_epoch.as_micros() as f64 / self.time_dilation).round() as u64;
        Duration::from_micros(micros) + self.epoch_start
    }

    /// The time stamp for the frame currently being processed.
    ///
    /// This is only valid while between the start of
    /// [`handle_begin_frame`](SchedulerBinding::handle_begin_frame) and the
    /// end of the corresponding
    /// [`handle_draw_frame`](SchedulerBinding::handle_draw_frame), i.e. while
    /// a frame is being produced.
    ///
    /// # Panics
    ///
    /// Outside a frame.
    pub fn current_frame_time_stamp(app: &mut App) -> Duration {
        let this = SchedulerBinding::instance(app);
        app.get(this)
            .current_frame_time_stamp
            .get()
            .expect("currentFrameTimeStamp is only valid while a frame is being produced")
    }

    /// The raw time stamp as provided by the engine to begin-frame for the
    /// frame currently being processed.
    ///
    /// Unlike [`current_frame_time_stamp`](SchedulerBinding::current_frame_time_stamp),
    /// this time stamp is neither adjusted to offset when the epoch started
    /// nor scaled to reflect the time dilation in the current epoch.
    pub fn current_system_frame_time_stamp(app: &mut App) -> Duration {
        let this = SchedulerBinding::instance(app);
        app.get(this).last_raw_time_stamp
    }

    /// Called by the embedder to prepare the framework to produce a new frame, of `views`.
    ///
    /// This function calls the transient frame callbacks registered by
    /// [`schedule_frame_callback`](SchedulerBinding::schedule_frame_callback) whose view is one
    /// of `views`, then those of no view.
    /// It then returns; any scheduled microtasks are run (the embedder must
    /// [`App::drain_microtasks`]), and
    /// [`handle_draw_frame`](SchedulerBinding::handle_draw_frame) is called to
    /// continue the frame.
    ///
    /// If the given time stamp is `None`, the time stamp from the last frame
    /// is reused. A time stamp before the last frame's, which another screen's refresh can bring,
    /// is taken as the last frame's: frame time never goes back.
    pub fn handle_begin_frame(app: &mut App, views: &[ViewId], raw_time_stamp: Option<Duration>) {
        let this = SchedulerBinding::instance(app);
        let time_stamp;
        let phase;
        {
            let binding = app.get_mut(this);
            let raw_time_stamp = raw_time_stamp.map(|raw| raw.max(binding.last_raw_time_stamp));
            if binding.first_raw_time_stamp_in_epoch.is_none() {
                binding.first_raw_time_stamp_in_epoch = raw_time_stamp;
            }
            binding.current_frame_time_stamp.set(Some(
                binding.adjust_for_epoch(raw_time_stamp.unwrap_or(binding.last_raw_time_stamp)),
            ));
            if let Some(raw) = raw_time_stamp {
                binding.last_raw_time_stamp = raw;
            }

            debug_assert!(binding.scheduler_phase.get() == SchedulerPhase::Idle);
            binding
                .views_with_scheduled_frame
                .retain(|view| !views.contains(view));
            binding.current_frame_views = views.to_vec();

            // TRANSIENT FRAME CALLBACKS
            binding
                .scheduler_phase
                .set(SchedulerPhase::TransientCallbacks);
            time_stamp = binding.current_frame_time_stamp.get().unwrap();
            phase = Rc::clone(&binding.scheduler_phase);
        }
        let callbacks = SchedulerBinding::take_transient_callbacks_due(app, views);
        let _finally = FrameFinally {
            phase,
            on_drop: SchedulerPhase::MidFrameMicrotasks,
            time_stamp: None,
        };
        for (id, callback) in callbacks {
            if !app.get(this).removed_ids.contains(&id) {
                callback.call(app, time_stamp);
            }
        }
        app.get_mut(this).removed_ids.clear();
    }

    /// Called by the embedder to produce a new frame, of `views`.
    ///
    /// This method is called immediately after
    /// [`handle_begin_frame`](SchedulerBinding::handle_begin_frame) and the
    /// microtask drain. It calls all the callbacks registered by
    /// [`add_persistent_frame_callback`](SchedulerBinding::add_persistent_frame_callback),
    /// which typically drive the rendering pipeline, and then calls the
    /// callbacks registered by
    /// [`add_post_frame_callback`](SchedulerBinding::add_post_frame_callback) for `views`, then
    /// those of no view.
    pub fn handle_draw_frame(app: &mut App, views: &[ViewId]) {
        let this = SchedulerBinding::instance(app);
        debug_assert!(app.get(this).scheduler_phase.get() == SchedulerPhase::MidFrameMicrotasks);
        debug_assert!(app.get(this).current_frame_views == views);
        debug_assert!(
            !app.has_pending_microtasks(),
            "drain microtasks between handle_begin_frame and handle_draw_frame"
        );

        // PERSISTENT FRAME CALLBACKS
        let binding = app.get_mut(this);
        binding
            .scheduler_phase
            .set(SchedulerPhase::PersistentCallbacks);
        let time_stamp = binding.current_frame_time_stamp.get().unwrap();
        let persistent = binding.persistent_callbacks.clone();
        let _finally = FrameFinally {
            phase: Rc::clone(&binding.scheduler_phase),
            on_drop: SchedulerPhase::Idle,
            time_stamp: Some(Rc::clone(&binding.current_frame_time_stamp)),
        };
        for callback in persistent {
            callback.call(app, views, time_stamp);
        }

        // POST-FRAME CALLBACKS
        app.get_mut(this)
            .scheduler_phase
            .set(SchedulerPhase::PostFrameCallbacks);
        for callback in SchedulerBinding::take_post_frame_callbacks_due(app, views) {
            callback.call(app, time_stamp);
        }
    }

    /// Takes the transient callbacks a frame of `views` runs: those whose view is one of them,
    /// then those of no view, each in the order they were scheduled. The others stay.
    fn take_transient_callbacks_due(app: &mut App, views: &[ViewId]) -> Vec<(i64, FrameCallback)> {
        let this = SchedulerBinding::instance(app);
        let scheduled = std::mem::take(&mut app.get_mut(this).transient_callbacks);
        let mut of_views = Vec::new();
        let mut of_no_view = Vec::new();
        let mut staying = IndexMap::new();
        for (id, entry) in scheduled {
            match SchedulerBinding::existing_view(app, entry.view) {
                Some(view) if views.contains(&view) => of_views.push((id, entry.callback)),
                Some(_) => {
                    staying.insert(id, entry);
                }
                None => of_no_view.push((id, entry.callback)),
            }
        }
        app.get_mut(this).transient_callbacks = staying;
        of_views.extend(of_no_view);
        of_views
    }

    /// Takes the post-frame callbacks a frame of `views` runs: those of one of them, then those
    /// of no view, each in the order they were added. The others stay, ahead of any added while
    /// these run.
    fn take_post_frame_callbacks_due(app: &mut App, views: &[ViewId]) -> Vec<FrameCallback> {
        let this = SchedulerBinding::instance(app);
        let added = std::mem::take(&mut app.get_mut(this).post_frame_callbacks);
        let mut of_views = Vec::new();
        let mut of_no_view = Vec::new();
        let mut staying = Vec::new();
        for entry in added {
            match SchedulerBinding::existing_view(app, entry.view) {
                Some(view) if views.contains(&view) => of_views.push(entry.callback),
                Some(_) => staying.push(entry),
                None => of_no_view.push(entry.callback),
            }
        }
        app.get_mut(this).post_frame_callbacks = staying;
        of_views.extend(of_no_view);
        of_views
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::rc::Rc;

    use inset_test::IMPLICIT_VIEW_ID;

    use super::*;
    use crate::test_support::{app_with_views, pump};

    type Log = Rc<RefCell<Vec<String>>>;

    const SECOND_VIEW: ViewId = ViewId(1);

    fn logging(log: &Log, name: &'static str) -> FrameCallback {
        let log = Rc::clone(log);
        FrameCallback::new(move |_app, time_stamp| {
            log.borrow_mut()
                .push(format!("{name}@{}ms", time_stamp.as_millis()));
        })
    }

    fn millis(ms: u64) -> Duration {
        Duration::from_millis(ms)
    }

    #[test]
    fn a_transient_callback_runs_once_per_registration() {
        let (cell, _platform) = app_with_views(&[IMPLICIT_VIEW_ID]);
        let mut app = cell.borrow_mut();
        let log = Log::default();

        SchedulerBinding::schedule_frame_callback(
            &mut app,
            logging(&log, "a"),
            Some(IMPLICIT_VIEW_ID),
            false,
            true,
        );
        assert!(SchedulerBinding::has_scheduled_frame(&mut app));

        // The epoch starts at the first frame, so its adjusted stamp is 0.
        pump(&mut app, &[IMPLICIT_VIEW_ID], millis(10));
        pump(&mut app, &[IMPLICIT_VIEW_ID], millis(20));
        assert_eq!(*log.borrow(), vec!["a@0ms"], "not re-registered");
    }

    #[test]
    fn canceling_by_id_prevents_the_call_even_mid_frame() {
        let (cell, _platform) = app_with_views(&[IMPLICIT_VIEW_ID]);
        let mut app = cell.borrow_mut();
        let log = Log::default();

        // The first callback cancels the second during the same frame — the
        // removed-ids check Dart does at invoke time.
        let second = Rc::new(RefCell::new(None::<i64>));
        let canceler = {
            let second = Rc::clone(&second);
            FrameCallback::new(move |app, _t| {
                SchedulerBinding::cancel_frame_callback_with_id(app, second.borrow().unwrap());
            })
        };
        SchedulerBinding::schedule_frame_callback(
            &mut app,
            canceler,
            Some(IMPLICIT_VIEW_ID),
            false,
            true,
        );
        let id = SchedulerBinding::schedule_frame_callback(
            &mut app,
            logging(&log, "b"),
            Some(IMPLICIT_VIEW_ID),
            false,
            true,
        );
        *second.borrow_mut() = Some(id);

        pump(&mut app, &[IMPLICIT_VIEW_ID], millis(10));
        assert!(log.borrow().is_empty(), "canceled before its turn came");
    }

    #[test]
    fn the_frame_runs_phases_in_order_and_post_frame_runs_once() {
        let (cell, _platform) = app_with_views(&[IMPLICIT_VIEW_ID]);
        let mut app = cell.borrow_mut();
        let log = Log::default();

        SchedulerBinding::add_persistent_frame_callback(
            &mut app,
            PersistentFrameCallback::new({
                let log = Rc::clone(&log);
                move |_app, _views, time_stamp| {
                    log.borrow_mut()
                        .push(format!("persistent@{}ms", time_stamp.as_millis()));
                }
            }),
        );
        SchedulerBinding::add_post_frame_callback(
            &mut app,
            logging(&log, "post"),
            Some(IMPLICIT_VIEW_ID),
        );
        SchedulerBinding::schedule_frame_callback(
            &mut app,
            logging(&log, "transient"),
            Some(IMPLICIT_VIEW_ID),
            false,
            true,
        );

        pump(&mut app, &[IMPLICIT_VIEW_ID], millis(5));
        assert_eq!(
            *log.borrow(),
            vec!["transient@0ms", "persistent@0ms", "post@0ms"]
        );

        log.borrow_mut().clear();
        pump(&mut app, &[IMPLICIT_VIEW_ID], millis(6));
        assert_eq!(
            *log.borrow(),
            vec!["persistent@1ms"],
            "post ran exactly once"
        );
    }

    // binding_test.dart 'Adding a persistent frame callback during a persistent frame callback'
    #[test]
    fn a_persistent_callback_added_during_a_frame_runs_on_the_next() {
        let (cell, _platform) = app_with_views(&[IMPLICIT_VIEW_ID]);
        let mut app = cell.borrow_mut();
        let called_back = Rc::new(Cell::new(false));
        SchedulerBinding::add_persistent_frame_callback(
            &mut app,
            PersistentFrameCallback::new({
                let called_back = Rc::clone(&called_back);
                move |app, _views, _t| {
                    if !called_back.get() {
                        SchedulerBinding::add_persistent_frame_callback(
                            app,
                            PersistentFrameCallback::new({
                                let called_back = Rc::clone(&called_back);
                                move |_app, _views, _t| called_back.set(true)
                            }),
                        );
                    }
                }
            }),
        );

        pump(&mut app, &[IMPLICIT_VIEW_ID], millis(0));
        assert!(!called_back.get());
        pump(&mut app, &[IMPLICIT_VIEW_ID], millis(1));
        assert!(called_back.get());
    }

    // Dart's `finally` in handleBeginFrame: the phase advances even when a
    // callback throws and the error is caught above the framework.
    #[test]
    fn a_panicking_transient_callback_still_advances_the_phase() {
        let (cell, _platform) = app_with_views(&[IMPLICIT_VIEW_ID]);
        let mut app = cell.borrow_mut();
        SchedulerBinding::schedule_frame_callback(
            &mut app,
            FrameCallback::new(|_app, _t| panic!("a failing tick")),
            Some(IMPLICIT_VIEW_ID),
            false,
            true,
        );

        let caught = catch_unwind(AssertUnwindSafe(|| {
            SchedulerBinding::handle_begin_frame(&mut app, &[IMPLICIT_VIEW_ID], Some(millis(1)));
        }));
        assert!(caught.is_err());
        assert_eq!(
            SchedulerBinding::scheduler_phase(&mut app),
            SchedulerPhase::MidFrameMicrotasks,
            "the finally ran on unwind"
        );
    }

    #[test]
    fn time_dilation_slows_the_adjusted_clock() {
        let (cell, _platform) = app_with_views(&[IMPLICIT_VIEW_ID]);
        let mut app = cell.borrow_mut();
        let seen = Rc::new(RefCell::new(Vec::new()));

        pump(&mut app, &[IMPLICIT_VIEW_ID], millis(0));
        SchedulerBinding::set_time_dilation(&mut app, 2.0);

        let recorder = {
            let seen = Rc::clone(&seen);
            FrameCallback::new(move |_app, t| seen.borrow_mut().push(t))
        };
        SchedulerBinding::schedule_frame_callback(
            &mut app,
            recorder.clone(),
            Some(IMPLICIT_VIEW_ID),
            false,
            true,
        );
        pump(&mut app, &[IMPLICIT_VIEW_ID], millis(100));
        SchedulerBinding::schedule_frame_callback(
            &mut app,
            recorder,
            Some(IMPLICIT_VIEW_ID),
            false,
            true,
        );
        pump(&mut app, &[IMPLICIT_VIEW_ID], millis(200));

        // resetEpoch re-bases at the next frame (100ms -> 0); the following
        // 100 raw ms are halved by dilation 2.
        assert_eq!(*seen.borrow(), vec![Duration::ZERO, millis(50)]);
    }

    #[test]
    fn schedule_frame_pokes_the_platform_once() {
        let (cell, platform) = app_with_views(&[IMPLICIT_VIEW_ID]);
        let mut app = cell.borrow_mut();

        let target = FrameTarget::View(IMPLICIT_VIEW_ID);
        SchedulerBinding::schedule_frame(&mut app, target);
        SchedulerBinding::schedule_frame(&mut app, target);
        assert_eq!(platform.frame_requests(), [IMPLICIT_VIEW_ID]);
        assert!(SchedulerBinding::has_scheduled_frame(&mut app));
    }

    // binding_test.dart 'Application lifecycle affects frame scheduling', without the forced
    // and warm-up frames.
    #[test]
    fn the_application_lifecycle_affects_frame_scheduling() {
        let (cell, _platform) = app_with_views(&[IMPLICIT_VIEW_ID]);
        let mut app = cell.borrow_mut();
        assert!(!SchedulerBinding::has_scheduled_frame(&mut app));

        SchedulerBinding::handle_app_lifecycle_state_changed(&mut app, AppLifecycleState::Paused);
        assert!(!SchedulerBinding::has_scheduled_frame(&mut app));

        SchedulerBinding::handle_app_lifecycle_state_changed(&mut app, AppLifecycleState::Resumed);
        assert!(SchedulerBinding::has_scheduled_frame(&mut app));
        pump(&mut app, &[IMPLICIT_VIEW_ID], millis(10));
        assert!(!SchedulerBinding::has_scheduled_frame(&mut app));

        SchedulerBinding::handle_app_lifecycle_state_changed(&mut app, AppLifecycleState::Inactive);
        assert!(!SchedulerBinding::has_scheduled_frame(&mut app));

        SchedulerBinding::handle_app_lifecycle_state_changed(&mut app, AppLifecycleState::Hidden);
        SchedulerBinding::schedule_frame(&mut app, FrameTarget::EveryView);
        assert!(
            !SchedulerBinding::has_scheduled_frame(&mut app),
            "frames are disabled while hidden"
        );
        assert_eq!(
            SchedulerBinding::lifecycle_state(&mut app),
            Some(AppLifecycleState::Hidden)
        );

        SchedulerBinding::handle_app_lifecycle_state_changed(&mut app, AppLifecycleState::Inactive);
        assert!(SchedulerBinding::has_scheduled_frame(&mut app));
    }

    #[test]
    fn the_host_can_pump_scheduler_handlers() {
        let (cell, _platform) = app_with_views(&[IMPLICIT_VIEW_ID]);
        let mut app = cell.borrow_mut();
        let log = Log::default();
        SchedulerBinding::schedule_frame_callback(
            &mut app,
            logging(&log, "a"),
            Some(IMPLICIT_VIEW_ID),
            false,
            true,
        );

        pump(&mut app, &[IMPLICIT_VIEW_ID], millis(10));
        assert_eq!(*log.borrow(), vec!["a@0ms"]);
    }

    #[test]
    fn a_frame_runs_the_callbacks_of_its_views_then_those_of_no_view() {
        let (cell, _platform) = app_with_views(&[IMPLICIT_VIEW_ID, SECOND_VIEW]);
        let mut app = cell.borrow_mut();
        let log = Log::default();
        SchedulerBinding::schedule_frame_callback(
            &mut app,
            logging(&log, "no view"),
            None,
            false,
            true,
        );
        SchedulerBinding::schedule_frame_callback(
            &mut app,
            logging(&log, "first"),
            Some(IMPLICIT_VIEW_ID),
            false,
            true,
        );
        SchedulerBinding::schedule_frame_callback(
            &mut app,
            logging(&log, "second"),
            Some(SECOND_VIEW),
            false,
            true,
        );
        SchedulerBinding::add_post_frame_callback(&mut app, logging(&log, "no view post"), None);
        SchedulerBinding::add_post_frame_callback(
            &mut app,
            logging(&log, "first post"),
            Some(IMPLICIT_VIEW_ID),
        );
        SchedulerBinding::add_post_frame_callback(
            &mut app,
            logging(&log, "second post"),
            Some(SECOND_VIEW),
        );

        pump(&mut app, &[IMPLICIT_VIEW_ID], millis(10));
        assert_eq!(
            *log.borrow(),
            [
                "first@0ms",
                "no view@0ms",
                "first post@0ms",
                "no view post@0ms"
            ],
        );

        log.borrow_mut().clear();
        pump(&mut app, &[SECOND_VIEW], millis(12));
        assert_eq!(*log.borrow(), ["second@2ms", "second post@2ms"]);
    }

    #[test]
    fn a_request_for_another_view_made_during_a_frame_is_not_lost() {
        let (cell, platform) = app_with_views(&[IMPLICIT_VIEW_ID, SECOND_VIEW]);
        let mut app = cell.borrow_mut();
        let asking = FrameCallback::new(|app, _time_stamp| {
            SchedulerBinding::ensure_visual_update(app, FrameTarget::View(SECOND_VIEW));
            SchedulerBinding::ensure_visual_update(app, FrameTarget::View(IMPLICIT_VIEW_ID));
            SchedulerBinding::ensure_visual_update(app, FrameTarget::EveryView);
        });
        SchedulerBinding::schedule_frame_callback(
            &mut app,
            asking,
            Some(IMPLICIT_VIEW_ID),
            false,
            true,
        );
        assert_eq!(platform.take_frame_requests(), [IMPLICIT_VIEW_ID]);

        pump(&mut app, &[IMPLICIT_VIEW_ID], millis(10));
        assert_eq!(
            platform.take_frame_requests(),
            [SECOND_VIEW],
            "the frame under way stands for its own views, not another"
        );
    }

    #[test]
    fn a_frame_time_never_goes_back() {
        let (cell, _platform) = app_with_views(&[IMPLICIT_VIEW_ID, SECOND_VIEW]);
        let mut app = cell.borrow_mut();
        let seen = Rc::new(RefCell::new(Vec::new()));
        SchedulerBinding::add_persistent_frame_callback(
            &mut app,
            PersistentFrameCallback::new({
                let seen = Rc::clone(&seen);
                move |_app, _views, time_stamp| seen.borrow_mut().push(time_stamp)
            }),
        );

        pump(&mut app, &[IMPLICIT_VIEW_ID], millis(10));
        pump(&mut app, &[IMPLICIT_VIEW_ID], millis(26));
        // Another screen, whose refresh came before the last one.
        pump(&mut app, &[SECOND_VIEW], millis(20));
        assert_eq!(*seen.borrow(), [Duration::ZERO, millis(16), millis(16)]);
    }

    #[test]
    fn a_callback_for_a_view_the_platform_does_not_have_runs_at_the_next_frame() {
        let (cell, _platform) = app_with_views(&[IMPLICIT_VIEW_ID]);
        let mut app = cell.borrow_mut();
        let log = Log::default();
        let gone = ViewId(7);
        SchedulerBinding::schedule_frame_callback(
            &mut app,
            logging(&log, "tick"),
            Some(gone),
            false,
            true,
        );
        SchedulerBinding::add_post_frame_callback(&mut app, logging(&log, "post"), Some(gone));
        assert!(
            SchedulerBinding::has_scheduled_frame(&mut app),
            "asked of the views there are"
        );

        pump(&mut app, &[IMPLICIT_VIEW_ID], millis(10));
        assert_eq!(*log.borrow(), ["tick@0ms", "post@0ms"]);
    }

    #[test]
    fn the_warm_up_frame_is_a_frame_of_every_view() {
        let (cell, _platform) = app_with_views(&[IMPLICIT_VIEW_ID, SECOND_VIEW]);
        let frames = Rc::new(RefCell::new(Vec::new()));
        SchedulerBinding::add_persistent_frame_callback(
            &mut cell.borrow_mut(),
            PersistentFrameCallback::new({
                let frames = Rc::clone(&frames);
                move |_app, views, _time_stamp| frames.borrow_mut().push(views.to_vec())
            }),
        );
        SchedulerBinding::schedule_warm_up_frame(&mut cell.borrow_mut());
        cell.elapse(Duration::ZERO);
        assert_eq!(*frames.borrow(), [vec![IMPLICIT_VIEW_ID, SECOND_VIEW]]);
    }

    #[test]
    fn a_callback_scheduled_without_a_new_frame_asks_for_none_after_a_frame() {
        let (cell, platform) = app_with_views(&[IMPLICIT_VIEW_ID]);
        let mut app = cell.borrow_mut();
        let log = Log::default();
        let scheduled = Rc::new(Cell::new(false));
        // As a render object's layout schedules work for the next frame without asking for one.
        SchedulerBinding::add_persistent_frame_callback(&mut app, {
            let log = Rc::clone(&log);
            PersistentFrameCallback::new(move |app, _views, _time_stamp| {
                if !scheduled.replace(true) {
                    SchedulerBinding::schedule_frame_callback(
                        app,
                        logging(&log, "next frame"),
                        None,
                        false,
                        false,
                    );
                }
            })
        });

        pump(&mut app, &[IMPLICIT_VIEW_ID], millis(10));
        assert!(
            platform.take_frame_requests().is_empty(),
            "no frame is asked for it"
        );
        pump(&mut app, &[IMPLICIT_VIEW_ID], millis(20));
        assert_eq!(*log.borrow(), ["next frame@10ms"]);
    }
}
