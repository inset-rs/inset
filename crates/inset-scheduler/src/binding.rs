//! Flutter counterpart: `scheduler/binding.dart`.

use std::cell::Cell;
use std::collections::HashSet;
use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use indexmap::IndexMap;
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
/// Dart's `SchedulerBinding.instance` is [`SchedulerBinding::instance`], the
/// App's singleton; members take `&mut App`.
pub struct SchedulerBinding {
    next_frame_callback_id: i64,
    // Insertion-ordered, as Dart's `Map` is: the relative order of two
    // tickers' callbacks is dispatch order.
    transient_callbacks: IndexMap<i64, FrameCallback>,
    removed_ids: HashSet<i64>,
    persistent_callbacks: Vec<FrameCallback>,
    post_frame_callbacks: Vec<FrameCallback>,
    has_scheduled_frame: bool,
    /// A warm-up frame is scheduled or running: the host's frames wait for it.
    warm_up_frame: bool,
    /// The host asked for a frame while the warm-up frame ran; it is rescheduled after.
    reschedule_after_warm_up_frame: bool,
    // Detached from the arena so the frame handlers' `finally` guards can
    // reach them without the App.
    scheduler_phase: Rc<Cell<SchedulerPhase>>,
    frames_enabled: bool,
    time_dilation: f64,
    first_raw_time_stamp_in_epoch: Option<Duration>,
    epoch_start: Duration,
    last_raw_time_stamp: Duration,
    current_frame_time_stamp: Rc<Cell<Option<Duration>>>,
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
            has_scheduled_frame: false,
            warm_up_frame: false,
            reschedule_after_warm_up_frame: false,
            scheduler_phase: Rc::new(Cell::new(SchedulerPhase::Idle)),
            frames_enabled: true,
            time_dilation: 1.0,
            first_raw_time_stamp_in_epoch: None,
            epoch_start: Duration::ZERO,
            last_raw_time_stamp: Duration::ZERO,
            current_frame_time_stamp: Rc::new(Cell::new(None)),
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
    /// Callbacks registered with this method can be canceled using
    /// [`cancel_frame_callback_with_id`](SchedulerBinding::cancel_frame_callback_with_id).
    pub fn schedule_frame_callback(
        app: &mut App,
        callback: FrameCallback,
        _rescheduling: bool,
        schedule_new_frame: bool,
    ) -> i64 {
        if schedule_new_frame {
            SchedulerBinding::schedule_frame(app);
        }
        let this = SchedulerBinding::instance(app);
        let binding = app.get_mut(this);
        binding.next_frame_callback_id += 1;
        let id = binding.next_frame_callback_id;
        binding.transient_callbacks.insert(id, callback);
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
    pub fn add_persistent_frame_callback(app: &mut App, callback: FrameCallback) {
        let this = SchedulerBinding::instance(app);
        app.get_mut(this).persistent_callbacks.push(callback);
    }

    /// Schedule a callback for the end of this frame.
    ///
    /// The provided callback is run immediately after a frame, just after the
    /// persistent frame callbacks (which is when the main rendering pipeline
    /// has been flushed).
    ///
    /// This method does *not* request a new frame. If a frame is already in
    /// progress and the execution of post-frame callbacks has not yet begun,
    /// then the registered callback is executed at the end of the current
    /// frame. Otherwise, the registered callback is executed after the next
    /// frame (whenever that may be, if ever).
    ///
    /// The callbacks are executed in the order in which they have been added.
    /// Post-frame callbacks cannot be unregistered. They are called exactly
    /// once.
    pub fn add_post_frame_callback(app: &mut App, callback: FrameCallback) {
        let this = SchedulerBinding::instance(app);
        app.get_mut(this).post_frame_callbacks.push(callback);
    }

    /// Whether this scheduler has requested that
    /// [`handle_begin_frame`](SchedulerBinding::handle_begin_frame) be called
    /// soon.
    pub fn has_scheduled_frame(app: &mut App) -> bool {
        let this = SchedulerBinding::instance(app);
        app.get(this).has_scheduled_frame
    }

    /// The phase that the scheduler is currently operating under.
    pub fn scheduler_phase(app: &mut App) -> SchedulerPhase {
        let this = SchedulerBinding::instance(app);
        app.get(this).scheduler_phase.get()
    }

    /// Whether frames are currently being scheduled when
    /// [`schedule_frame`](SchedulerBinding::schedule_frame) is called.
    ///
    /// This value depends on the value of the lifecycle state. The lifecycle
    /// is not ported, so this stays true.
    pub fn frames_enabled(app: &mut App) -> bool {
        let this = SchedulerBinding::instance(app);
        app.get(this).frames_enabled
    }

    /// Schedules a new frame using [`schedule_frame`](SchedulerBinding::schedule_frame)
    /// if this object is not currently producing a frame.
    ///
    /// This has no effect if [`scheduler_phase`](SchedulerBinding::scheduler_phase)
    /// is [`SchedulerPhase::TransientCallbacks`] or
    /// [`SchedulerPhase::MidFrameMicrotasks`] (because a frame is already being
    /// prepared in that case), or [`SchedulerPhase::PersistentCallbacks`]
    /// (because a frame is actively being rendered in that case). It will
    /// schedule a frame if the phase is [`SchedulerPhase::Idle`] (in between
    /// frames) or [`SchedulerPhase::PostFrameCallbacks`] (after a frame).
    pub fn ensure_visual_update(app: &mut App) {
        match SchedulerBinding::scheduler_phase(app) {
            SchedulerPhase::Idle | SchedulerPhase::PostFrameCallbacks => {
                SchedulerBinding::schedule_frame(app);
            }
            SchedulerPhase::TransientCallbacks
            | SchedulerPhase::MidFrameMicrotasks
            | SchedulerPhase::PersistentCallbacks => {}
        }
    }

    /// If necessary, schedules a new frame.
    pub fn schedule_frame(app: &mut App) {
        let this = SchedulerBinding::instance(app);
        if app.get(this).has_scheduled_frame || !app.get(this).frames_enabled {
            return;
        }
        app.platform().request_frame();
        app.get_mut(this).has_scheduled_frame = true;
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
    /// The frame runs on two zero-duration timers, "begin" and "draw", with the
    /// microtasks drained between them; the host runs the timers it owes before
    /// it takes new events, so nothing reaches the framework in between. Dart
    /// locks events for the same reason.
    pub fn schedule_warm_up_frame(app: &mut App) {
        let this = SchedulerBinding::instance(app);
        if app.get(this).warm_up_frame
            || app.get(this).scheduler_phase.get() != SchedulerPhase::Idle
        {
            return;
        }
        app.get_mut(this).warm_up_frame = true;
        let had_scheduled_frame = app.get(this).has_scheduled_frame;
        Timer::new(
            app,
            Duration::ZERO,
            Listener::new(|app| SchedulerBinding::handle_begin_frame(app, None)),
        );
        Timer::new(
            app,
            Duration::ZERO,
            Listener::new(move |app| {
                SchedulerBinding::handle_draw_frame(app);
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
                if had_scheduled_frame {
                    SchedulerBinding::schedule_frame(app);
                }
            }),
        );
    }

    /// Dart's `_handleBeginFrame`, what `PlatformDispatcher.onBeginFrame` is
    /// set to: the host's signal that a frame begins. A warm-up frame in
    /// progress takes the frame's place, and the frame is scheduled again
    /// once it is done.
    pub fn on_begin_frame(app: &mut App, raw_time_stamp: Option<Duration>) {
        let this = SchedulerBinding::instance(app);
        if app.get(this).warm_up_frame {
            // "begin frame" and "draw frame" must strictly alternate. Therefore
            // reschedule_after_warm_up_frame cannot possibly be true here as it
            // is reset by on_draw_frame.
            debug_assert!(!app.get(this).reschedule_after_warm_up_frame);
            app.get_mut(this).reschedule_after_warm_up_frame = true;
            return;
        }
        SchedulerBinding::handle_begin_frame(app, raw_time_stamp);
    }

    /// Dart's `_handleDrawFrame`, what `PlatformDispatcher.onDrawFrame` is set
    /// to: the host's signal to draw the frame, after the microtasks that
    /// followed [`on_begin_frame`](SchedulerBinding::on_begin_frame).
    pub fn on_draw_frame(app: &mut App) {
        let this = SchedulerBinding::instance(app);
        if app.get(this).reschedule_after_warm_up_frame {
            app.get_mut(this).reschedule_after_warm_up_frame = false;
            // Reschedule in a post-frame callback to allow the draw-frame phase
            // of the warm-up frame to finish.
            SchedulerBinding::add_post_frame_callback(
                app,
                FrameCallback::new(|app, _time_stamp| {
                    // Force a host frame. `has_scheduled_frame` is reset here
                    // because the original host frame was cancelled, and
                    // therefore `handle_begin_frame`, which is responsible for
                    // resetting it, did not run. So if a frame callback set it
                    // in the "so far" part of the frame, the flag would be wrong
                    // and any later frame request would be dropped.
                    let this = SchedulerBinding::instance(app);
                    app.get_mut(this).has_scheduled_frame = false;
                    SchedulerBinding::schedule_frame(app);
                }),
            );
            return;
        }
        SchedulerBinding::handle_draw_frame(app);
    }

    /// Schedules a new frame even when frames would normally not be scheduled
    /// by [`schedule_frame`](SchedulerBinding::schedule_frame) (e.g. when the
    /// device's screen is turned off).
    pub fn schedule_forced_frame(app: &mut App) {
        let this = SchedulerBinding::instance(app);
        if app.get(this).has_scheduled_frame {
            return;
        }
        app.platform().request_frame();
        app.get_mut(this).has_scheduled_frame = true;
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

    /// Called by the embedder to prepare the framework to produce a new frame.
    ///
    /// This function calls all the transient frame callbacks registered by
    /// [`schedule_frame_callback`](SchedulerBinding::schedule_frame_callback).
    /// It then returns; any scheduled microtasks are run (the embedder must
    /// [`App::drain_microtasks`]), and
    /// [`handle_draw_frame`](SchedulerBinding::handle_draw_frame) is called to
    /// continue the frame.
    ///
    /// If the given time stamp is `None`, the time stamp from the last frame
    /// is reused.
    pub fn handle_begin_frame(app: &mut App, raw_time_stamp: Option<Duration>) {
        let this = SchedulerBinding::instance(app);
        let time_stamp;
        let callbacks;
        let phase;
        {
            let binding = app.get_mut(this);
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
            binding.has_scheduled_frame = false;

            // TRANSIENT FRAME CALLBACKS
            binding
                .scheduler_phase
                .set(SchedulerPhase::TransientCallbacks);
            callbacks = std::mem::take(&mut binding.transient_callbacks);
            time_stamp = binding.current_frame_time_stamp.get().unwrap();
            phase = Rc::clone(&binding.scheduler_phase);
        }
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

    /// Called by the embedder to produce a new frame.
    ///
    /// This method is called immediately after
    /// [`handle_begin_frame`](SchedulerBinding::handle_begin_frame) and the
    /// microtask drain. It calls all the callbacks registered by
    /// [`add_persistent_frame_callback`](SchedulerBinding::add_persistent_frame_callback),
    /// which typically drive the rendering pipeline, and then calls the
    /// callbacks registered by
    /// [`add_post_frame_callback`](SchedulerBinding::add_post_frame_callback).
    pub fn handle_draw_frame(app: &mut App) {
        let this = SchedulerBinding::instance(app);
        debug_assert!(app.get(this).scheduler_phase.get() == SchedulerPhase::MidFrameMicrotasks);
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
            callback.call(app, time_stamp);
        }

        // POST-FRAME CALLBACKS
        let binding = app.get_mut(this);
        binding
            .scheduler_phase
            .set(SchedulerPhase::PostFrameCallbacks);
        let post = std::mem::take(&mut binding.post_frame_callbacks);
        for callback in post {
            callback.call(app, time_stamp);
        }
    }
}

#[cfg(test)]
mod tests {
    use inset_foundation::AppCell;
    use std::cell::{Cell, RefCell};
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::rc::Rc;

    use super::*;

    type Log = Rc<RefCell<Vec<String>>>;

    fn logging(log: &Log, name: &'static str) -> FrameCallback {
        let log = Rc::clone(log);
        FrameCallback::new(move |_app, time_stamp| {
            log.borrow_mut()
                .push(format!("{name}@{}ms", time_stamp.as_millis()));
        })
    }

    fn pump(app: &mut App, at: Duration) {
        SchedulerBinding::handle_begin_frame(app, Some(at));
        app.drain_microtasks();
        SchedulerBinding::handle_draw_frame(app);
        app.drain_microtasks();
    }

    #[test]
    fn a_transient_callback_runs_once_per_registration() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let log = Log::default();

        SchedulerBinding::schedule_frame_callback(&mut app, logging(&log, "a"), false, true);
        assert!(SchedulerBinding::has_scheduled_frame(&mut app));

        // The epoch starts at the first frame, so its adjusted stamp is 0.
        pump(&mut app, Duration::from_millis(10));
        pump(&mut app, Duration::from_millis(20));
        assert_eq!(*log.borrow(), vec!["a@0ms"], "not re-registered");
    }

    #[test]
    fn canceling_by_id_prevents_the_call_even_mid_frame() {
        let cell = AppCell::new();
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
        SchedulerBinding::schedule_frame_callback(&mut app, canceler, false, true);
        let id =
            SchedulerBinding::schedule_frame_callback(&mut app, logging(&log, "b"), false, true);
        *second.borrow_mut() = Some(id);

        pump(&mut app, Duration::from_millis(10));
        assert!(log.borrow().is_empty(), "canceled before its turn came");
    }

    #[test]
    fn the_frame_runs_phases_in_order_and_post_frame_runs_once() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let log = Log::default();

        SchedulerBinding::add_persistent_frame_callback(&mut app, logging(&log, "persistent"));
        SchedulerBinding::add_post_frame_callback(&mut app, logging(&log, "post"));
        SchedulerBinding::schedule_frame_callback(
            &mut app,
            logging(&log, "transient"),
            false,
            true,
        );

        pump(&mut app, Duration::from_millis(5));
        assert_eq!(
            *log.borrow(),
            vec!["transient@0ms", "persistent@0ms", "post@0ms"]
        );

        log.borrow_mut().clear();
        pump(&mut app, Duration::from_millis(6));
        assert_eq!(
            *log.borrow(),
            vec!["persistent@1ms"],
            "post ran exactly once"
        );
    }

    // binding_test.dart 'Adding a persistent frame callback during a persistent frame callback'
    #[test]
    fn a_persistent_callback_added_during_a_frame_runs_on_the_next() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let called_back = Rc::new(Cell::new(false));
        SchedulerBinding::add_persistent_frame_callback(
            &mut app,
            FrameCallback::new({
                let called_back = Rc::clone(&called_back);
                move |app, _t| {
                    if !called_back.get() {
                        SchedulerBinding::add_persistent_frame_callback(
                            app,
                            FrameCallback::new({
                                let called_back = Rc::clone(&called_back);
                                move |_app, _t| called_back.set(true)
                            }),
                        );
                    }
                }
            }),
        );

        pump(&mut app, Duration::from_millis(0));
        assert!(!called_back.get());
        pump(&mut app, Duration::from_millis(1));
        assert!(called_back.get());
    }

    // Dart's `finally` in handleBeginFrame: the phase advances even when a
    // callback throws and the error is caught above the framework.
    #[test]
    fn a_panicking_transient_callback_still_advances_the_phase() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        SchedulerBinding::schedule_frame_callback(
            &mut app,
            FrameCallback::new(|_app, _t| panic!("a failing tick")),
            false,
            true,
        );

        let caught = catch_unwind(AssertUnwindSafe(|| {
            SchedulerBinding::handle_begin_frame(&mut app, Some(Duration::from_millis(1)));
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
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let seen = Rc::new(RefCell::new(Vec::new()));

        pump(&mut app, Duration::from_millis(0));
        SchedulerBinding::set_time_dilation(&mut app, 2.0);

        let recorder = {
            let seen = Rc::clone(&seen);
            FrameCallback::new(move |_app, t| seen.borrow_mut().push(t))
        };
        SchedulerBinding::schedule_frame_callback(&mut app, recorder.clone(), false, true);
        pump(&mut app, Duration::from_millis(100));
        SchedulerBinding::schedule_frame_callback(&mut app, recorder, false, true);
        pump(&mut app, Duration::from_millis(200));

        // resetEpoch re-bases at the next frame (100ms -> 0); the following
        // 100 raw ms are halved by dilation 2.
        assert_eq!(
            *seen.borrow(),
            vec![Duration::ZERO, Duration::from_millis(50)]
        );
    }

    struct RecordingPlatform {
        frames: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    }

    impl inset_embedder::Platform for RecordingPlatform {
        fn target_platform(&self) -> inset_embedder::TargetPlatform {
            inset_embedder::TargetPlatform::Android
        }

        fn request_frame(&self) {
            self.frames
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }

        fn now(&self) -> std::time::Instant {
            std::time::Instant::now()
        }

        fn wake_at(&self, _deadline: std::time::Instant) {}

        fn views(&self) -> Vec<inset_embedder::ViewRef> {
            Vec::new()
        }

        fn view(&self, _id: inset_embedder::ViewId) -> Option<inset_embedder::ViewRef> {
            None
        }

        fn implicit_view(&self) -> Option<inset_embedder::ViewRef> {
            None
        }
    }

    #[test]
    fn schedule_frame_pokes_the_platform_once() {
        let frames = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let platform = std::rc::Rc::new(RecordingPlatform {
            frames: std::sync::Arc::clone(&frames),
        });
        let cell = AppCell::with_platform(platform);
        let mut app = cell.borrow_mut();

        SchedulerBinding::schedule_frame(&mut app);
        SchedulerBinding::schedule_frame(&mut app);
        assert_eq!(frames.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert!(SchedulerBinding::has_scheduled_frame(&mut app));
    }

    #[test]
    fn the_host_can_pump_scheduler_handlers() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let log = Log::default();
        SchedulerBinding::schedule_frame_callback(&mut app, logging(&log, "a"), false, true);

        pump(&mut app, Duration::from_millis(10));
        assert_eq!(*log.borrow(), vec!["a@0ms"]);
    }
}
