//! Flutter counterpart: `scheduler/ticker.dart`.

use std::cell::{Cell, OnceCell};
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};
use std::time::Duration;

use reveal_foundation::{App, Completer, CompleterFuture, Handle, Listener};

use crate::binding::{FrameCallback, SchedulerBinding, SchedulerPhase};

/// Signature for the callback passed to the [`Ticker`] class's constructor.
///
/// The argument is the time elapsed from the frame timestamp when the ticker
/// was last started to the current frame timestamp.
pub type TickerCallback = FrameCallback;

/// An interface implemented by classes that can vend [`Ticker`] objects.
///
/// Tickers can be used by any object that wants to be notified whenever a
/// frame triggers, but are most commonly used indirectly via an
/// `AnimationController`. `AnimationController`s need a [`TickerProvider`] to
/// obtain their [`Ticker`].
///
/// An object in the App implements [`TickerProviderObject`]; its [`Handle`] is then a
/// provider — Dart's `TickerProviderStateMixin` is `vsync: this` — and a field that Dart types
/// as `TickerProvider` holds an `Rc<dyn TickerProvider>`.
pub trait TickerProvider {
    /// Creates a ticker with the given callback.
    ///
    /// The kind of ticker provided depends on the kind of ticker provider.
    fn create_ticker(&self, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker>;
}

/// [`TickerProvider`] for an object in the App: Dart's `implements TickerProvider` on a
/// `State`, whose `vsync: this` is the object's [`Handle`].
pub trait TickerProviderObject: Sized + 'static {
    /// See [`TickerProvider::create_ticker`].
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker>;
}

impl<T: TickerProviderObject> TickerProvider for Handle<T> {
    fn create_ticker(&self, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
        TickerProviderObject::create_ticker(*self, app, on_tick)
    }
}

impl<T: TickerProvider + ?Sized> TickerProvider for Rc<T> {
    fn create_ticker(&self, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
        T::create_ticker(self, app, on_tick)
    }
}

/// Calls its callback once per animation frame, when enabled.
///
/// When created, a ticker is initially disabled. Call
/// [`start`](Ticker::start) to enable the ticker.
///
/// A [`Ticker`] can be silenced by setting muted to true. While silenced, time
/// still elapses, and [`start`](Ticker::start) and
/// [`stop`](Ticker::stop) can still be called, but no callbacks are
/// called.
///
/// By convention, the [`start`](Ticker::start) and
/// [`stop`](Ticker::stop) methods are used by the ticker's consumer
/// (for example, an `AnimationController`), and the muted property is
/// controlled by the [`TickerProvider`] that created the ticker.
pub struct Ticker {
    future: Option<TickerFuture>,

    /// If true, this ticker will request frames using
    /// [`SchedulerBinding::schedule_forced_frame`] instead of
    /// [`SchedulerBinding::schedule_frame`].
    ///
    /// This allows granular control to advance frames even when frames are
    /// typically disabled (e.g. when the app is in the background). This
    /// should be used sparingly as it can increase battery usage.
    pub force_frames: bool,

    muted: bool,
    start_time: Option<Duration>,
    on_tick: TickerCallback,
    animation_id: Option<i64>,
}

impl Ticker {
    /// Creates a ticker that will call the provided callback once per frame
    /// while running.
    pub fn new(app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
        app.create(Ticker {
            future: None,
            force_frames: false,
            muted: false,
            start_time: None,
            on_tick,
            animation_id: None,
        })
    }

    /// Whether this ticker has been silenced.
    ///
    /// While silenced, a ticker's clock can still run, but the callback will
    /// not be called.
    pub fn muted(self: Handle<Self>, app: &App) -> bool {
        app.get(self).muted
    }

    /// Whether time is elapsing for this [`Ticker`]. Becomes true when
    /// [`start`](Ticker::start) is called and false when
    /// [`stop`](Ticker::stop) is called.
    ///
    /// A ticker can be active yet not be actually ticking (i.e. not be
    /// calling the callback). To determine if a ticker is actually ticking,
    /// use [`is_ticking`](Ticker::is_ticking).
    pub fn is_active(self: Handle<Self>, app: &App) -> bool {
        app.get(self).future.is_some()
    }

    /// Whether this [`Ticker`] has already scheduled a frame callback.
    fn scheduled(self: Handle<Self>, app: &App) -> bool {
        app.get(self).animation_id.is_some()
    }

    /// Whether a tick should be scheduled.
    ///
    /// If this is true, then calling [`schedule_tick`](Ticker::schedule_tick)
    /// should succeed.
    ///
    /// Reasons why a tick should not be scheduled include:
    ///
    /// * A tick has already been scheduled for the coming frame.
    /// * The ticker is not active ([`start`](Ticker::start) has not
    ///   been called).
    /// * The ticker is not ticking, e.g. because it is muted (see
    ///   [`is_ticking`](Ticker::is_ticking)).
    fn should_schedule_tick(self: Handle<Self>, app: &mut App) -> bool {
        !app.get(self).muted && self.is_active(app) && !self.scheduled(app)
    }

    /// Dart's `_tick`, the frame callback registered with the binding.
    fn tick(self: Handle<Self>, app: &mut App, time_stamp: Duration) {
        debug_assert!(self.is_ticking(app));
        debug_assert!(self.scheduled(app));
        app.get_mut(self).animation_id = None;

        let ticker = app.get_mut(self);
        let start_time = *ticker.start_time.get_or_insert(time_stamp);
        let on_tick = ticker.on_tick.clone();
        on_tick.call(app, time_stamp - start_time);

        // The onTick callback may have scheduled another tick already, for
        // example by calling stop then start again.
        if self.should_schedule_tick(app) {
            self.schedule_tick(app, true);
        }
    }

    /// Schedules a tick for the next frame.
    ///
    /// This should only be called if
    /// [`should_schedule_tick`](Ticker::should_schedule_tick) is true.
    fn schedule_tick(self: Handle<Self>, app: &mut App, rescheduling: bool) {
        debug_assert!(!self.scheduled(app));
        debug_assert!(self.should_schedule_tick(app));
        if app.get(self).force_frames {
            SchedulerBinding::schedule_forced_frame(app);
        } else {
            SchedulerBinding::schedule_frame(app);
        }
        let animation_id = SchedulerBinding::schedule_frame_callback(
            app,
            FrameCallback::handle_method(self, Ticker::tick),
            rescheduling,
            false,
        );
        app.get_mut(self).animation_id = Some(animation_id);
    }

    /// Cancels the frame callback that was requested by
    /// [`schedule_tick`](Ticker::schedule_tick), if any.
    ///
    /// Calling this method when no tick is scheduled is harmless.
    fn unschedule_tick(self: Handle<Self>, app: &mut App) {
        if let Some(animation_id) = app.get_mut(self).animation_id.take() {
            SchedulerBinding::cancel_frame_callback_with_id(app, animation_id);
        }
        debug_assert!(!self.should_schedule_tick(app));
    }

    /// When set to true, silences the ticker. If a tick is already scheduled,
    /// it will unschedule it. This will not unschedule the next frame.
    ///
    /// When set to false, unsilences the ticker, potentially scheduling a
    /// frame to handle the next tick.
    ///
    /// By convention, muted is controlled by the object that created the
    /// [`Ticker`] (typically a [`TickerProvider`]), not the object that
    /// listens to the ticker's ticks.
    pub fn set_muted(self: Handle<Self>, app: &mut App, value: bool) {
        if value == app.get(self).muted {
            return;
        }
        app.get_mut(self).muted = value;
        if value {
            self.unschedule_tick(app);
        } else if self.should_schedule_tick(app) {
            self.schedule_tick(app, false);
        }
    }

    /// Whether this [`Ticker`] has scheduled a call to call its callback on
    /// the next frame.
    ///
    /// A ticker that is muted can be active (see [`Ticker::is_active`]) yet
    /// not be ticking. In that case, the ticker will not call its callback,
    /// and `is_ticking` will be false, but time will still be progressing.
    ///
    /// This will return false if frames are not currently enabled. The
    /// lifecycle is not ported, so frames stay enabled.
    #[allow(clippy::wrong_self_convention)]
    pub fn is_ticking(self: Handle<Self>, app: &mut App) -> bool {
        if app.get(self).future.is_none() {
            return false;
        }
        if app.get(self).muted {
            return false;
        }
        if SchedulerBinding::frames_enabled(app) {
            return true;
        }
        if SchedulerBinding::scheduler_phase(app) != SchedulerPhase::Idle {
            return true;
        } // for example, we might be in a warm-up frame or forced frame
        false
    }

    /// Starts the clock for this [`Ticker`]. If the ticker is not muted, then
    /// this also starts calling the ticker's callback once per animation
    /// frame.
    ///
    /// The returned future resolves once the ticker stops ticking. If the
    /// ticker is disposed, the future does not resolve.
    ///
    /// Calling this sets [`Ticker::is_active`] to true.
    ///
    /// # Panics
    ///
    /// In debug builds, if the ticker is already active — Dart throws a
    /// `FlutterError` from an assert block here.
    pub fn start(self: Handle<Self>, app: &mut App) -> TickerFuture {
        debug_assert!(
            !self.is_active(app),
            "A ticker was started twice. A ticker that is already active cannot be started again without first stopping it."
        );
        debug_assert!(app.get(self).start_time.is_none());

        let future = TickerFuture::new();
        app.get_mut(self).future = Some(future.clone());
        if self.should_schedule_tick(app) {
            self.schedule_tick(app, false);
        }
        let phase = SchedulerBinding::scheduler_phase(app);
        if phase > SchedulerPhase::Idle && phase < SchedulerPhase::PostFrameCallbacks {
            let time_stamp = SchedulerBinding::current_frame_time_stamp(app);
            app.get_mut(self).start_time = Some(time_stamp);
        }
        future
    }

    /// Stops calling this [`Ticker`]'s callback.
    ///
    /// If called with `canceled` false (Dart's default), causes the future
    /// returned by [`start`](Ticker::start) to resolve. If called
    /// with `canceled` true, the future does not resolve, and callbacks
    /// registered with [`TickerFuture::when_complete_or_cancel`]
    /// resolve as canceled.
    ///
    /// Calling this sets [`Ticker::is_active`] to false.
    ///
    /// This method does nothing if called when the ticker is inactive.
    pub fn stop(self: Handle<Self>, app: &mut App, canceled: bool) {
        if !self.is_active(app) {
            return;
        }

        // We take the future into a local so that isTicking is false when we
        // actually complete the future (isTicking uses the future to
        // determine its state).
        let ticker = app.get_mut(self);
        let local_future = ticker.future.take().unwrap();
        ticker.start_time = None;
        debug_assert!(!self.is_active(app));

        self.unschedule_tick(app);
        if canceled {
            local_future.mark_canceled(app, Some(self));
        } else {
            local_future.mark_complete(app);
        }
    }

    /// Makes this [`Ticker`] take the state of another ticker, and disposes
    /// the other ticker.
    ///
    /// This is useful if an object with a [`Ticker`] is given a new
    /// [`TickerProvider`] but needs to maintain continuity. In particular,
    /// this maintains the identity of the [`TickerFuture`] returned by the
    /// [`start`](Ticker::start) function of the original [`Ticker`]
    /// if the original ticker is active.
    ///
    /// This ticker must not be active when this method is called.
    pub fn absorb_ticker(self: Handle<Self>, app: &mut App, original_ticker: Handle<Ticker>) {
        debug_assert!(!self.is_active(app));
        debug_assert!(app.get(self).future.is_none());
        debug_assert!(app.get(self).start_time.is_none());
        debug_assert!(app.get(self).animation_id.is_none());
        debug_assert!(
            app.get(original_ticker).future.is_some()
                || app.get(original_ticker).start_time.is_none(),
            "Cannot absorb Ticker after it has been disposed."
        );
        if app.get(original_ticker).future.is_some() {
            let original = app.get_mut(original_ticker);
            let future = original.future.take(); // so the future is not
            let start_time = original.start_time; // canceled when we dispose
            let ticker = app.get_mut(self); // the original ticker below
            ticker.future = future;
            ticker.start_time = start_time;
            if self.should_schedule_tick(app) {
                self.schedule_tick(app, false);
            }
            original_ticker.unschedule_tick(app);
        }
        original_ticker.dispose(app);
    }

    /// Release the resources used by this object. The object is no longer
    /// usable after this method is called.
    ///
    /// It is legal to call this method while [`Ticker::is_active`] is true, in
    /// which case:
    ///
    /// * The frame callback that was requested by `schedule_tick`, if any, is
    ///   canceled.
    /// * The future that was returned by [`start`](Ticker::start)
    ///   does not resolve.
    /// * Callbacks registered with
    ///   [`TickerFuture::when_complete_or_cancel`] resolve as
    ///   canceled.
    pub fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(local_future) = app.get_mut(self).future.take() {
            debug_assert!(!self.is_active(app));
            self.unschedule_tick(app);
            local_future.mark_canceled(app, Some(self));
        }
        #[cfg(debug_assertions)]
        {
            // We intentionally don't null out startTime. This means that if
            // start() was ever called, the object is now in a bogus state.
            // This weakly helps catch cases of use-after-dispose.
            app.get_mut(self).start_time = Some(Duration::ZERO);
        }
    }
}

/// An object representing an ongoing [`Ticker`] sequence.
///
/// The [`Ticker::start`] method returns a [`TickerFuture`]. The [`TickerFuture`] will
/// complete successfully if the [`Ticker`] is stopped using [`Ticker::stop`] with `canceled`
/// false. If the [`Ticker`] is disposed without being stopped, or if it is stopped with
/// `canceled` true, then this future will never complete.
///
/// A `Future<Output = ()>`, as Dart's implements `Future<void>`: `await` it in a task, or
/// register a callback with [`when_complete`](Self::when_complete). Clones share the one
/// sequence, as Dart's references to the one object do.
///
/// To be notified when the sequence is canceled, use [`or_cancel`](Self::or_cancel) or
/// [`when_complete_or_cancel`](Self::when_complete_or_cancel).
#[derive(Clone)]
pub struct TickerFuture {
    shared: Rc<TickerFutureState>,
}

/// Written once each: the outcome, and the secondary completer `or_cancel` creates on first
/// use. No runtime borrow can conflict here.
struct TickerFutureState {
    primary: Completer<()>,
    secondary: OnceCell<Completer<Result<(), TickerCanceled>>>,
    /// `None` means unresolved, `Some(true)` complete, `Some(false)` canceled.
    completed: Cell<Option<bool>>,
}

impl TickerFuture {
    fn new() -> TickerFuture {
        TickerFuture {
            shared: Rc::new(TickerFutureState {
                primary: Completer::new(),
                secondary: OnceCell::new(),
                completed: Cell::new(None),
            }),
        }
    }

    /// Creates a [`TickerFuture`] instance that represents an already-complete [`Ticker`]
    /// sequence.
    ///
    /// This is useful for implementing objects that normally defer to a [`Ticker`] but
    /// sometimes can skip the ticker because the animation is of zero duration, but which
    /// still need to represent the completed animation in the form of a [`TickerFuture`].
    pub fn complete() -> TickerFuture {
        TickerFuture {
            shared: Rc::new(TickerFutureState {
                primary: Completer::completed(()),
                secondary: OnceCell::new(),
                completed: Cell::new(Some(true)),
            }),
        }
    }

    /// Dart's private `_complete`, called by [`Ticker::stop`].
    fn mark_complete(&self, app: &mut App) {
        debug_assert!(self.shared.completed.get().is_none());
        self.shared.completed.set(Some(true));
        self.shared.primary.complete(app, ());
        if let Some(secondary) = self.shared.secondary.get() {
            secondary.complete(app, Ok(()));
        }
    }

    /// Dart's private `_cancel`, called by [`Ticker::stop`] with `canceled` and by
    /// [`Ticker::dispose`].
    fn mark_canceled(&self, app: &mut App, ticker: Option<Handle<Ticker>>) {
        debug_assert!(self.shared.completed.get().is_none());
        self.shared.completed.set(Some(false));
        if let Some(secondary) = self.shared.secondary.get() {
            secondary.complete(app, Err(TickerCanceled { ticker }));
        }
    }

    /// A future that resolves when this future resolves or with a [`TickerCanceled`] when
    /// the ticker is canceled.
    ///
    /// Until this is first called, canceling the ticker records no error anywhere; from
    /// then on a cancellation resolves the returned future with `Err`, which the awaiting
    /// task is expected to handle.
    pub fn or_cancel(&self) -> CompleterFuture<Result<(), TickerCanceled>> {
        self.shared
            .secondary
            .get_or_init(|| match self.shared.completed.get() {
                None => Completer::new(),
                Some(true) => Completer::completed(Ok(())),
                Some(false) => Completer::completed(Err(TickerCanceled { ticker: None })),
            })
            .future()
    }

    /// Dart's `then` / `whenComplete`: runs `callback` on the microtask queue after the
    /// ticker stops without being canceled, never inline — a `.then` on an already resolved
    /// future still runs asynchronously. On a canceled ticker it never runs.
    pub fn when_complete(&self, app: &mut App, callback: Listener) {
        self.shared
            .primary
            .future()
            .then(app, move |app, ()| callback.call(app));
    }

    /// Calls `callback` either when this future resolves or when the ticker is canceled.
    ///
    /// Calling this method registers a handler for the [`or_cancel`](Self::or_cancel) error,
    /// so canceling the ticker afterwards leaves no unobserved cancellation.
    pub fn when_complete_or_cancel(&self, app: &mut App, callback: Listener) {
        self.or_cancel().then(app, move |app, _| callback.call(app));
    }
}

impl Future for TickerFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let mut primary = self.shared.primary.future();
        Pin::new(&mut primary).poll(cx)
    }
}

impl fmt::Debug for TickerFuture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state = match self.shared.completed.get() {
            None => "active",
            Some(true) => "complete",
            Some(false) => "canceled",
        };
        write!(f, "TickerFuture({state})")
    }
}

/// Exception thrown by [`Ticker`] objects on the [`TickerFuture::or_cancel`] future when
/// the ticker is canceled.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TickerCanceled {
    /// Reference to the [`Ticker`] object that was canceled.
    ///
    /// This may be `None` in the case that the future created for
    /// [`TickerFuture::or_cancel`] was created after the ticker was canceled.
    pub ticker: Option<Handle<Ticker>>,
}

impl fmt::Display for TickerCanceled {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.ticker {
            Some(ticker) => write!(f, "This ticker was canceled: {ticker:?}"),
            None => write!(
                f,
                "The ticker was canceled before the \"orCancel\" property was first used."
            ),
        }
    }
}

impl std::error::Error for TickerCanceled {}

#[cfg(test)]
mod tests {
    use reveal_foundation::AppCell;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    use super::*;

    fn pump(app: &mut App, at: Duration) {
        SchedulerBinding::handle_begin_frame(app, Some(at));
        app.drain_microtasks();
        SchedulerBinding::handle_draw_frame(app);
        app.drain_microtasks();
    }

    fn elapsed_recorder(log: &Rc<RefCell<Vec<Duration>>>) -> TickerCallback {
        let log = Rc::clone(log);
        FrameCallback::new(move |_app, elapsed| log.borrow_mut().push(elapsed))
    }

    // ticker_test.dart 'ticker can be started and stopped' (adapted: frames
    // come from handle_begin_frame, as flutter_test's pump does).
    #[test]
    fn a_ticker_ticks_with_elapsed_time_until_stopped() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let log = Rc::new(RefCell::new(Vec::new()));
        let ticker = Ticker::new(&mut app, elapsed_recorder(&log));

        ticker.start(&mut app);
        assert!(ticker.is_active(&app));
        assert!(SchedulerBinding::has_scheduled_frame(&mut app));

        pump(&mut app, Duration::from_millis(10));
        pump(&mut app, Duration::from_millis(25));
        ticker.stop(&mut app, false);
        pump(&mut app, Duration::from_millis(40));

        // Elapsed is measured from the first tick's time stamp.
        assert_eq!(
            *log.borrow(),
            vec![Duration::ZERO, Duration::from_millis(15)]
        );
        assert!(!ticker.is_active(&app));
    }

    // ticker_test.dart 'Ticker mute control test' (without lifecycle).
    #[test]
    fn a_muted_ticker_keeps_time_but_does_not_call_back() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let log = Rc::new(RefCell::new(Vec::new()));
        let ticker = Ticker::new(&mut app, elapsed_recorder(&log));

        ticker.start(&mut app);
        assert!(ticker.is_ticking(&mut app));
        pump(&mut app, Duration::from_millis(10));

        ticker.set_muted(&mut app, true);
        assert!(!ticker.is_ticking(&mut app));
        assert!(ticker.is_active(&app));
        SchedulerBinding::schedule_frame(&mut app);
        pump(&mut app, Duration::from_millis(20));
        assert_eq!(log.borrow().len(), 1, "no tick while muted");

        ticker.set_muted(&mut app, false);
        assert!(ticker.is_ticking(&mut app));
        pump(&mut app, Duration::from_millis(30));

        // Time elapsed while muted: the clock ran from the 10ms tick.
        assert_eq!(
            *log.borrow(),
            vec![Duration::ZERO, Duration::from_millis(20)]
        );

        ticker.stop(&mut app, false);
        ticker.set_muted(&mut app, false);
        pump(&mut app, Duration::from_millis(40));
        assert_eq!(log.borrow().len(), 2, "stopped ticker does not tick");
        assert!(!ticker.is_ticking(&mut app));
        assert!(!ticker.is_active(&app));
    }

    // ticker_test.dart 'Ticker can be slowed down with time dilation'
    #[test]
    fn a_ticker_is_slowed_by_time_dilation() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        SchedulerBinding::set_time_dilation(&mut app, 2.0);
        let last = Rc::new(Cell::new(None::<Duration>));
        let ticker = Ticker::new(
            &mut app,
            FrameCallback::new({
                let last = Rc::clone(&last);
                move |_app, duration| last.set(Some(duration))
            }),
        );
        ticker.start(&mut app);
        pump(&mut app, Duration::from_millis(10));
        pump(&mut app, Duration::from_millis(20));
        assert_eq!(last.get(), Some(Duration::from_millis(5)));
        ticker.dispose(&mut app);
        SchedulerBinding::set_time_dilation(&mut app, 1.0);
    }

    #[test]
    fn the_ticker_future_completes_through_the_microtask_queue() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let ticker = Ticker::new(&mut app, FrameCallback::new(|_app, _t| {}));
        let future = ticker.start(&mut app);

        let completed = Rc::new(Cell::new(false));
        future.when_complete(
            &mut app,
            Listener::new({
                let completed = Rc::clone(&completed);
                move |_app| completed.set(true)
            }),
        );

        ticker.stop(&mut app, false);
        assert!(
            !completed.get(),
            "Dart resolves Future listeners on the microtask queue, not during stop()"
        );
        app.drain_microtasks();
        assert!(completed.get());
    }

    #[test]
    fn disposing_an_active_ticker_cancels_without_completing() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let ticker = Ticker::new(&mut app, FrameCallback::new(|_app, _t| {}));
        let future = ticker.start(&mut app);

        let completed = Rc::new(Cell::new(false));
        let either = Rc::new(Cell::new(false));
        future.when_complete(
            &mut app,
            Listener::new({
                let completed = Rc::clone(&completed);
                move |_app| completed.set(true)
            }),
        );
        future.when_complete_or_cancel(
            &mut app,
            Listener::new({
                let either = Rc::clone(&either);
                move |_app| either.set(true)
            }),
        );

        ticker.dispose(&mut app);
        app.drain_microtasks();

        assert!(!completed.get(), "the primary future never resolves");
        assert!(
            either.get(),
            "whenCompleteOrCancel observes the cancellation"
        );
    }

    // ticker.dart's absorbTicker contract: the original's active future
    // survives under the absorbing ticker's identity.
    #[test]
    fn absorbing_a_ticker_keeps_the_original_future_alive() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let log = Rc::new(RefCell::new(Vec::new()));
        let original = Ticker::new(&mut app, elapsed_recorder(&log));
        let future = original.start(&mut app);

        let absorber = Ticker::new(&mut app, FrameCallback::new(|_app, _t| {}));
        absorber.absorb_ticker(&mut app, original);

        assert!(absorber.is_active(&app));

        let completed = Rc::new(Cell::new(false));
        future.when_complete(
            &mut app,
            Listener::new({
                let completed = Rc::clone(&completed);
                move |_app| completed.set(true)
            }),
        );
        absorber.stop(&mut app, false);
        app.drain_microtasks();
        assert!(completed.get(), "not canceled by the original's dispose");
    }

    #[test]
    fn a_zero_duration_sequence_is_already_complete() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let future = TickerFuture::complete();

        let ran = Rc::new(Cell::new(false));
        future.when_complete(
            &mut app,
            Listener::new({
                let ran = Rc::clone(&ran);
                move |_app| ran.set(true)
            }),
        );
        assert!(!ran.get(), "still asynchronous on a resolved future");
        app.drain_microtasks();
        assert!(ran.get());
    }

    struct TestVSync;

    impl TickerProvider for TestVSync {
        fn create_ticker(&self, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
            Ticker::new(app, on_tick)
        }
    }

    #[test]
    fn a_ticker_provider_vends_a_ticker_that_ticks() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let log = Rc::new(RefCell::new(Vec::new()));
        let ticker = TestVSync.create_ticker(&mut app, elapsed_recorder(&log));
        ticker.start(&mut app);
        pump(&mut app, Duration::from_millis(10));
        assert_eq!(*log.borrow(), vec![Duration::ZERO]);
        ticker.stop(&mut app, false);
    }

    struct Host {
        ticker: Option<Handle<Ticker>>,
    }

    impl TickerProvider for Handle<Host> {
        fn create_ticker(&self, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
            let ticker = Ticker::new(app, on_tick);
            app.get_mut(*self).ticker = Some(ticker);
            ticker
        }
    }

    #[test]
    fn a_provider_in_the_app_can_vend_a_ticker() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let host = app.create(Host { ticker: None });
        let log = Rc::new(RefCell::new(Vec::new()));
        let ticker = host.create_ticker(&mut app, elapsed_recorder(&log));
        assert!(app.get(host).ticker.is_some());
        ticker.start(&mut app);
        pump(&mut app, Duration::from_millis(10));
        assert_eq!(*log.borrow(), vec![Duration::ZERO]);
        ticker.stop(&mut app, false);
    }
}
