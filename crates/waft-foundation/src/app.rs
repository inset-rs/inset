//! The one [`App`] and the arena of [`Handle`]s inside it. Not a Dart file.

use std::any::type_name;
use std::collections::{HashMap, VecDeque};
use std::fmt::{self, Debug};
use std::marker::PhantomData;
use std::ops::Receiver;
use std::rc::{Rc, Weak};
use std::time::Duration;

use slotmap::new_key_type;
use waft_embedder::{
    ClipboardEvent, DragReport, EditCommand, MenuId, NotificationActivation, OpenRequest,
    PlatformRef, RequestId, TrayIconEvent, TrayIconId, ViewError, ViewFocusEvent, ViewId,
};

use crate::app_cell::{AppCell, AsyncApp};
use crate::change_notifier::Listener;
use crate::entity::EntityMap;
use crate::executor::{ExecutorHandle, ForegroundExecutor, Task};
use crate::handle_map::HandleMap;
pub use crate::handle_map::{AnyRef, OptionRetained, Ref, Retained};
use crate::timers::{Timer, Timers};

/// Drain budget for one [`App::drain_microtasks`]: two callbacks scheduling
/// each other forever must fail loud, not starve the event loop. Not a Dart
/// or Flutter constant — the isolate drains until empty; a cycle there hangs.
const MICROTASK_BUDGET: usize = 100_000;

new_key_type! {
    /// Untyped [`Handle`]. For a reference that may point at more than one type;
    /// [`App::handle`] narrows it.
    ///
    /// A free bumps the generation, so a leftover id does not silently name the next occupant.
    pub struct HandleId;
}

/// A Dart object, as far as Rust ownership is concerned, named without keeping it.
///
/// Flutter objects form cycles — an element names its widget and its render object, a notifier
/// names its listeners, a listener often names the notifier. Dart allows that because the GC
/// owns the objects. Here the object lives in [`App`] while a [`Ref`] keeps it (or the tree code,
/// for a tree node); this is the id that names it.
///
/// `Copy` and owns nothing: the method receiver, the form layout and paint walk with, and what a
/// field holds when it points back at what holds its object (a parent, an owner). Valid while
/// something keeps the object: one on the stack is good until the next checkpoint, and one kept
/// past it is a [`Ref`] or is checked with [`upgrade`](Handle::upgrade) or [`App::contains`].
/// Stale is detected by generation.
///
/// Point access, not a lease: [`App::get`] / [`App::get_mut`] for one field, then drop the borrow
/// before any call that can run framework code. Holding the object out of the arena for a whole
/// pass (shaft-rs-next) meant a layout callback could not re-enter the node it was laying out.
///
/// [`App::get`] panics on stale — a typed handle is a promise the entry is live. [`App::handle`]
/// returns [`None`] for an id that might be dead or the wrong type.
///
/// `PhantomData<fn() -> T>` so `Handle<T>: Copy` even when `T` is not.
pub struct Handle<T> {
    id: HandleId,
    state: PhantomData<fn() -> T>,
}

impl<T> Handle<T> {
    /// Unchecked. [`App::create`] mints these; [`App::handle`] is the checked path and
    /// [`from_id`](Self::from_id) the trusted one.
    pub(crate) fn new(id: HandleId) -> Handle<T> {
        Handle {
            id,
            state: PhantomData,
        }
    }

    /// Wraps an id that was minted for `T` without looking at the entry. The check is deferred to
    /// [`App::get`], which panics on a stale or wrong-typed id. For a type-erased handle that
    /// reconstructs the typed handle it was made from.
    pub fn from_id(id: HandleId) -> Handle<T> {
        Handle::new(id)
    }

    pub fn id(self) -> HandleId {
        self.id
    }
}

impl<T: 'static> Handle<T> {
    /// A [`Ref`] while something still keeps the object, `None` once it is freed or its last
    /// keeper has let go: GPUI's `WeakEntity::upgrade`.
    ///
    /// # Panics
    ///
    /// If the entry holds another type.
    pub fn upgrade(self, app: &App) -> Option<Ref<T>> {
        app.handles.upgrade(self)
    }

    /// A [`Ref`] to an object something is known to keep: an object keeping itself in a field
    /// of its own or in a closure it stores, while its owner holds it.
    ///
    /// # Panics
    ///
    /// If nothing keeps the object, naming its type and where it was created, or with the
    /// stale-read message if it is freed.
    #[track_caller]
    pub fn keep(self, app: &App) -> Ref<T> {
        Retained::keep(app, self)
    }
}

impl HandleId {
    /// An [`AnyRef`] while something still keeps the object; see [`Handle::upgrade`].
    pub fn upgrade(self, app: &App) -> Option<AnyRef> {
        app.handles.upgrade_id(self)
    }

    /// An [`AnyRef`] to an object something is known to keep; see [`Handle::keep`].
    #[track_caller]
    pub fn keep(self, app: &App) -> AnyRef {
        Retained::keep(app, self)
    }
}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Handle<T> {
        *self
    }
}

impl<T> Copy for Handle<T> {}

/// An arena object's methods take `self: Handle<Self>`: the handle is the receiver, the
/// object is `app.get(self)`.
impl<T> Receiver for Handle<T> {
    type Target = T;
}

impl<T> PartialEq for Handle<T> {
    fn eq(&self, other: &Handle<T>) -> bool {
        self.id == other.id
    }
}

impl<T> Eq for Handle<T> {}

impl<T> std::hash::Hash for Handle<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state)
    }
}

impl<T> Debug for Handle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Handle<{}>({:?})", type_name::<T>(), self.id)
    }
}

impl<T> From<Handle<T>> for HandleId {
    fn from(handle: Handle<T>) -> HandleId {
        handle.id
    }
}

/// Owns every Flutter object. Callbacks receive `&mut App` plus a [`Handle`] to themselves.
///
/// Lives in an [`AppCell`], which is what the shell and the tests hold; a task reaches it
/// through [`AsyncApp`]. Private and `#[non_exhaustive]`: scheduler, timers, and further
/// arenas are additive.
#[non_exhaustive]
pub struct App {
    /// The cell this App lives in — gpui's `App::this`.
    this: Weak<AppCell>,
    handles: HandleMap,
    pub(crate) entities: EntityMap,
    /// The App is what keeps a singleton.
    singletons: HashMap<std::any::TypeId, AnyRef>,
    /// The receivers of the skipped tear-offs already reported, so each is reported once.
    #[cfg(debug_assertions)]
    skipped_tear_off_receivers: std::collections::HashSet<HandleId>,
    #[cfg(debug_assertions)]
    skipped_tear_offs: Vec<String>,
    microtasks: VecDeque<Listener>,
    timers: Timers,
    /// The debug flags set on this app ([`set_debug_flag`](App::set_debug_flag)); one absent is
    /// off.
    debug_flags: HashMap<std::any::TypeId, bool>,
    executor: ForegroundExecutor,
    platform: PlatformRef,
    /// The host's threads, told when to wake this application for its next timer.
    dispatcher: std::sync::Arc<dyn waft_embedder::Dispatcher>,
    platform_callbacks: PlatformCallbacks,
    open_requests: UntilHandled<OpenRequest>,
    notification_activations: UntilHandled<NotificationActivation>,
}

/// A debug flag an [`App`] keeps: a marker type per Dart `debug…` bool, declared in the crate
/// that reads it (`waft_rendering::DebugPaintSizeEnabled`), so this crate needs to know none of
/// them. Each crate keeps Dart's name as a function over the app.
pub trait DebugFlag: 'static {
    /// The flag's value until it is set: Dart's initializer, `false` for most.
    const DEFAULT: bool = false;
}

/// What hears one kind of what the system asks of the app, and what it asked before there was a
/// handler, which waits for one, as a launch's requests do.
struct UntilHandled<T> {
    handler: Option<Handler<T>>,
    waiting: Vec<T>,
}

impl<T> Default for UntilHandled<T> {
    fn default() -> UntilHandled<T> {
        UntilHandled {
            handler: None,
            waiting: Vec::new(),
        }
    }
}

/// The app's handler for one kind of what the system asks of it.
type Handler<T> = Rc<dyn Fn(&mut App, T)>;

/// The app's handler for the system's requests to open files and links.
pub type OpenRequestedCallback = Rc<dyn Fn(&mut App, OpenRequest)>;

/// The app's handler for the clicks on its notifications.
pub type NotificationActivatedCallback = Rc<dyn Fn(&mut App, NotificationActivation)>;

/// The callbacks the framework assigns onto dart:ui's `PlatformDispatcher`
/// (`onPlatformBrightnessChanged`, `onLocaleChanged`). The host's half of the dispatcher
/// is the `Platform` the `App` holds; this is the half the framework sets and the host's
/// client invokes.
#[derive(Default)]
pub struct PlatformCallbacks {
    pub on_platform_brightness_changed: Option<Listener>,
    pub on_locale_changed: Option<Listener>,
    /// Flutter `PlatformDispatcher.onViewFocusChange`.
    pub on_view_focus_change: Option<ViewFocusChangeCallback>,
    /// What is dragged from outside the application over a view; no Flutter counterpart.
    pub on_drag: Option<DragCallback>,
    /// The user copied, cut or pasted in a view through the platform's own command; no Flutter
    /// counterpart.
    pub on_clipboard_event: Option<ClipboardEventCallback>,
    /// The user chose an edit command for a view from the platform's own Edit menu; no Flutter
    /// counterpart.
    pub on_edit_command: Option<EditCommandCallback>,
    /// The user chose an item of the app's menu bar. Flutter's `Menu.selectedCallback` on the
    /// `flutter/menu` channel, which `DefaultPlatformMenuDelegate` handles.
    pub on_menu_selected: Option<MenuSelectedCallback>,
    /// The user chose an item of the Dock icon's menu; no Flutter counterpart, and the app's to
    /// set.
    pub on_dock_menu_selected: Option<MenuSelectedCallback>,
    /// Something happened to one of the app's tray icons; no Flutter counterpart, and the app's to
    /// set.
    pub on_tray_icon_event: Option<TrayIconEventCallback>,
    /// The user asked to go back in a view. Answers whether the application went back there.
    /// Flutter's `popRoute` on `SystemChannels.navigation`, which names no view.
    pub on_pop_route: Option<PopRouteCallback>,
    /// The platform added a view: one the platform gave, or one the app asked for. Flutter's
    /// `_addView` tells it only as `onMetricsChanged`.
    pub on_view_added: Option<ViewChangeCallback>,
    /// A view went.
    pub on_view_removed: Option<ViewChangeCallback>,
    /// The user asked to close a view. Flutter's engine reports it to the window's controller.
    pub on_view_close_requested: Option<ViewCloseRequestedCallback>,
    /// A view's window was minimized or restored, became occluded or shown, or moved. Flutter's
    /// engine reports it to the window's controller.
    pub on_window_changed: Option<WindowChangedCallback>,
    /// A request for a view got no view.
    pub on_view_request_failed: Option<ViewRequestFailedCallback>,
}

/// Flutter `PlatformDispatcher.onViewFocusChange`.
pub type ViewFocusChangeCallback = Rc<dyn Fn(&mut App, ViewFocusEvent)>;

/// The framework's handler for `EmbedderClient::drag_report`.
pub type DragCallback = Rc<dyn Fn(&mut App, DragReport)>;

/// The framework's handler for `EmbedderClient::clipboard_event`.
pub type ClipboardEventCallback = Rc<dyn Fn(&mut App, ViewId, ClipboardEvent)>;

/// The framework's handler for `EmbedderClient::edit_command`.
pub type EditCommandCallback = Rc<dyn Fn(&mut App, ViewId, EditCommand)>;

/// The handler for `EmbedderClient::menu_selected` and `dock_menu_selected`.
pub type MenuSelectedCallback = Rc<dyn Fn(&mut App, MenuId)>;

/// The app's handler for `EmbedderClient::tray_icon_event`.
pub type TrayIconEventCallback = Rc<dyn Fn(&mut App, TrayIconId, TrayIconEvent)>;

/// The framework's handler for `EmbedderClient::pop_route`.
pub type PopRouteCallback = Rc<dyn Fn(&mut App, ViewId) -> bool>;

/// The framework's handler for `EmbedderClient::view_added` and `view_removed`.
pub type ViewChangeCallback = Rc<dyn Fn(&mut App, ViewId)>;

/// The framework's handler for `EmbedderClient::view_close_requested`.
pub type ViewCloseRequestedCallback = Rc<dyn Fn(&mut App, ViewId)>;

/// The framework's handler for `EmbedderClient::window_changed`.
pub type WindowChangedCallback = Rc<dyn Fn(&mut App, ViewId)>;

/// The framework's handler for `EmbedderClient::view_request_failed`.
pub type ViewRequestFailedCallback = Rc<dyn Fn(&mut App, RequestId, ViewError)>;

impl App {
    pub(crate) fn build(
        this: Weak<AppCell>,
        platform: PlatformRef,
        executor: ForegroundExecutor,
    ) -> App {
        App {
            this,
            handles: HandleMap::new(),
            entities: EntityMap::new(),
            singletons: HashMap::new(),
            #[cfg(debug_assertions)]
            skipped_tear_off_receivers: std::collections::HashSet::new(),
            #[cfg(debug_assertions)]
            skipped_tear_offs: Vec::new(),
            microtasks: VecDeque::new(),
            timers: Timers::default(),
            debug_flags: HashMap::new(),
            executor,
            dispatcher: platform.dispatcher(),
            platform,
            platform_callbacks: PlatformCallbacks::default(),
            open_requests: UntilHandled::default(),
            notification_activations: UntilHandled::default(),
        }
    }

    /// The executor's polling side, for the cell's checkpoint.
    pub(crate) fn executor_handle(&self) -> ExecutorHandle {
        self.executor.handle()
    }

    /// This App as a task sees it: the handle an `async` body captures across its `await`s.
    pub fn to_async(&self) -> AsyncApp {
        AsyncApp::new(self.this.clone(), self.executor.handle())
    }

    /// Queues `f` as a task on this App: the continuation of a Dart `async` body.
    ///
    /// Dart runs an `async` body synchronously up to its first `await`; port that prefix
    /// inline — including the call whose future is awaited — and spawn only what follows the
    /// `await`. The continuation first runs at the next [`AppCell::checkpoint`], the end of
    /// the current platform event, and thereafter at the checkpoint after whatever it awaited
    /// completed; it cannot run inline, since the caller holds the App it would borrow.
    ///
    /// The returned [`Task`] is the continuation's future; dropping it does not cancel it.
    pub fn spawn<R: 'static>(&self, f: impl AsyncFnOnce(&mut AsyncApp) -> R + 'static) -> Task<R> {
        let mut cx = self.to_async();
        self.executor.spawn(async move { f(&mut cx).await })
    }

    /// Runs `work` off the main thread, on the host's dispatcher, and answers its result as a
    /// task: dart:isolate's `Isolate.run`. The task resolves at a checkpoint after the work is
    /// done, since the executor wakes the host for it, and a panic in `work` resumes in
    /// whoever awaits the task, as `Isolate.run` rethrows the isolate's error.
    pub fn run_in_background<T: Send + 'static>(
        &self,
        work: impl FnOnce() -> T + Send + 'static,
    ) -> Task<T> {
        self.executor.run_in_background(work)
    }

    pub fn platform(&self) -> PlatformRef {
        Rc::clone(&self.platform)
    }

    /// The dispatcher callbacks the framework has assigned.
    pub fn platform_callbacks(&self) -> &PlatformCallbacks {
        &self.platform_callbacks
    }

    pub fn platform_callbacks_mut(&mut self) -> &mut PlatformCallbacks {
        &mut self.platform_callbacks
    }

    /// Sets what hears the system's requests to open files and links
    /// (`EmbedderClient::open_requested`), and hands it at once the requests that came before it
    /// was set, as a launch's do; with `None`, later requests wait for the next handler. No
    /// Flutter counterpart: Flutter's embedders hand these to their plugins.
    pub fn set_open_requested_handler(&mut self, handler: Option<OpenRequestedCallback>) {
        self.set_handler(|app| &mut app.open_requests, handler);
    }

    /// Asks the app to open `request`: the handler hears it, or it waits for one. The host calls it
    /// with what the system hands over (`EmbedderClient::open_requested`); the app calls it with
    /// the files it decides its own command line names, at launch or as a later launch passes it
    /// on (`waft-single-instance`), so every request reaches the one handler.
    pub fn open_requested(&mut self, request: OpenRequest) {
        self.hand_to_handler(|app| &mut app.open_requests, request);
    }

    /// Sets what hears the clicks on the app's notifications
    /// (`EmbedderClient::notification_activated`), and hands it at once the clicks that came
    /// before it was set, as the one that started the app does; with `None`, later clicks wait for
    /// the next handler. No Flutter counterpart: Flutter's apps hear them through plugins.
    pub fn set_notification_activated_handler(
        &mut self,
        handler: Option<NotificationActivatedCallback>,
    ) {
        self.set_handler(|app| &mut app.notification_activations, handler);
    }

    /// The user clicked a notification: the handler hears it, or it waits for one.
    pub fn notification_activated(&mut self, activation: NotificationActivation) {
        self.hand_to_handler(|app| &mut app.notification_activations, activation);
    }

    fn set_handler<T>(
        &mut self,
        queue: fn(&mut App) -> &mut UntilHandled<T>,
        handler: Option<Handler<T>>,
    ) {
        queue(self).handler = handler.clone();
        let Some(handler) = handler else {
            return;
        };
        for waiting in std::mem::take(&mut queue(self).waiting) {
            handler(self, waiting);
        }
    }

    fn hand_to_handler<T>(&mut self, queue: fn(&mut App) -> &mut UntilHandled<T>, asked: T) {
        match queue(self).handler.clone() {
            Some(handler) => handler(self, asked),
            None => queue(self).waiting.push(asked),
        }
    }

    /// One `T` per App (`SchedulerBinding.instance`, `kAlwaysCompleteAnimation`), kept by the
    /// App for its whole life.
    #[track_caller]
    pub fn singleton<T: 'static + Default>(&mut self) -> Handle<T> {
        if let Some(existing) = self.existing_singleton() {
            return existing;
        }
        let singleton = self.create(T::default());
        let handle = singleton.handle();
        self.singletons
            .insert(std::any::TypeId::of::<T>(), singleton.to_any());
        handle
    }

    /// The `T` an earlier [`singleton`](App::singleton) call made, or `None` before the
    /// first one: Dart's `instance` getter of a binding, which reads and never creates.
    pub fn existing_singleton<T: 'static>(&self) -> Option<Handle<T>> {
        self.singletons
            .get(&std::any::TypeId::of::<T>())
            .map(|singleton| Handle::new(singleton.handle()))
    }

    /// Dart's `scheduleMicrotask`: queues `callback` to run after the current
    /// turn, before the next platform event — never inline.
    ///
    /// The host empties the queue with
    /// [`drain_microtasks`](App::drain_microtasks) after every platform
    /// event, and between `_beginFrame` and `_drawFrame` — where the engine,
    /// which runs the two in one native task, puts an explicit flush.
    pub fn schedule_microtask(&mut self, callback: Listener) {
        self.microtasks.push_back(callback);
    }

    /// Runs queued microtasks until the queue is empty.
    ///
    /// A callback scheduled during the drain runs in the same drain, as
    /// Dart's queue does. Call with no borrow of the App held.
    ///
    /// # Panics
    ///
    /// After `MICROTASK_BUDGET` callbacks in one drain, on the assumption
    /// that two callbacks are scheduling each other in a cycle.
    pub fn drain_microtasks(&mut self) {
        let mut drained = 0usize;
        while let Some(callback) = self.microtasks.pop_front() {
            drained += 1;
            assert!(
                drained <= MICROTASK_BUDGET,
                "microtasks did not converge after {MICROTASK_BUDGET} callbacks; \
                 are two callbacks scheduling each other in a cycle?"
            );
            callback.call(self);
        }
    }

    /// Whether any microtask is queued.
    ///
    /// The queue is empty when a new platform event begins, and the engine
    /// explicitly flushes it between the two frame callbacks; the scheduler
    /// asserts the latter.
    pub fn has_pending_microtasks(&self) -> bool {
        !self.microtasks.is_empty()
    }

    /// Dart's `Timer(duration, callback)`. Fires when the clock reaches
    /// `now + duration` via [`AppCell::elapse`].
    pub fn schedule_timer(&mut self, duration: Duration, callback: Listener) -> Timer {
        let timer = self.timers.schedule(duration, callback);
        self.update_timer_wakeup();
        timer
    }

    /// Dart's `Timer.periodic(duration, callback)`. Fires whenever the clock
    /// reaches each multiple of `duration` after `now`, until it is cancelled.
    pub fn schedule_periodic_timer(
        &mut self,
        duration: Duration,
        callback: impl Fn(&mut App, Timer) + 'static,
    ) -> Timer {
        let timer = self.timers.schedule_periodic(duration, Rc::new(callback));
        self.update_timer_wakeup();
        timer
    }

    /// Dart's `Timer.cancel()`. Idempotent.
    pub fn cancel_timer(&mut self, timer: Timer) {
        self.timers.cancel(timer);
        // Cancelling the earliest timer moves the wakeup later, or leaves no timer at all.
        self.update_timer_wakeup();
    }

    /// Dart's `Timer.isActive`.
    pub fn timer_is_active(&self, timer: Timer) -> bool {
        self.timers.is_active(timer)
    }

    /// Dart's `Timer.tick`: the fires a periodic timer has had.
    pub fn timer_tick(&self, timer: Timer) -> u64 {
        self.timers.tick(timer)
    }

    /// The App clock: what [`AppCell::elapse`] has advanced it to. Flutter's test binding's
    /// `clock`; a frame a test pumps is stamped with it.
    pub fn clock(&self) -> Duration {
        self.timers.now()
    }

    /// Whether a [`Timer`] is waiting to fire: what flutter_test checks has not happened once a
    /// test's tree is disposed.
    pub fn has_pending_timers(&self) -> bool {
        self.timers.has_pending()
    }

    /// Whether the debug flag `F` is on for this app: Dart's library-level `debug…` bools, kept
    /// per app as its clock is, so two tests, or two apps, never share one. [`DebugFlag::DEFAULT`]
    /// until set.
    pub fn debug_flag<F: DebugFlag>(&self) -> bool {
        self.debug_flags
            .get(&std::any::TypeId::of::<F>())
            .copied()
            .unwrap_or(F::DEFAULT)
    }

    /// Turns the debug flag `F` on or off for this app; see [`debug_flag`](Self::debug_flag).
    pub fn set_debug_flag<F: DebugFlag>(&mut self, value: bool) {
        self.debug_flags.insert(std::any::TypeId::of::<F>(), value);
    }

    /// Fires the next timer due by `target`, if any.
    pub(crate) fn fire_next_due(&mut self, target: Duration) -> bool {
        let Some((timer, callback)) = self.timers.pop_next_due(target) else {
            return false;
        };
        callback(self, timer);
        true
    }

    pub(crate) fn advance_clock_to(&mut self, target: Duration) {
        self.timers.advance_to(target);
    }

    /// Tells the host when to next wake this application for a timer, unless that is
    /// already what it was told. Every path that can change the earliest waiting timer ends
    /// here, and the call does nothing when that timer has not changed.
    pub(crate) fn update_timer_wakeup(&mut self) {
        if let Some(delay) = self.timers.timer_wakeup_if_changed() {
            let wakeup = delay.map(|delay| self.platform.now() + delay);
            self.dispatcher.wake_at(wakeup);
        }
    }

    /// Puts `state` in the arena and answers the one [`Ref`] that keeps it: Dart's constructor
    /// call. The object is freed at the checkpoint after the last `Ref` to it is dropped.
    ///
    /// A [`Ref`] is for Flutter's objects and the relations between them, as Dart's reference
    /// is. The app's own data and the logic that changes it are an [`Entity`](crate::Entity),
    /// which [`new_entity`](Self::new_entity) makes: a widget reads it in `build` and is rebuilt
    /// by it when it notifies, where a `Ref` is what a controller, an animation or a scroll
    /// position is held as.
    ///
    /// ```
    /// # use waft_foundation::{App, AppCell};
    /// let cell = AppCell::new();
    /// let mut app = cell.borrow_mut();
    /// let counter = app.create(0u32);      // a field holds this `Ref`, not the bare handle
    /// *app.get_mut(&counter) += 1;
    /// assert_eq!(*app.get(&counter), 1);
    /// ```
    ///
    /// A tree node (an element, a state, a render object, a layer) is handed to the tree code at
    /// once with [`Retained::into_tree_node`].
    #[track_caller]
    pub fn create<T: 'static>(&mut self, state: T) -> Ref<T> {
        self.handles.create(state)
    }

    /// See [`Retained::into_tree_node`].
    pub(crate) fn adopt_tree_node(&mut self, id: HandleId, keep: Rc<crate::handle_map::Keep>) {
        self.handles.adopt_tree_node(id, keep);
    }

    /// The tree code is done with a node, at one of the four places Flutter's lifecycle ends one:
    /// an element unmounted, a state disposed with its element, a render object or a layer
    /// disposed. Marks it disposed and gives up the tree's part of its count, so it is freed at
    /// the next checkpoint unless a [`Ref`] still keeps it.
    ///
    /// # Panics
    ///
    /// If the object is not a tree node ([`Retained::into_tree_node`]), or was released already.
    pub fn release_tree_node(&mut self, node: impl Into<HandleId>) {
        self.handles.release_tree_node(node.into());
    }

    /// Why [`Retained::keep`] found no keeper for `id`.
    pub(crate) fn not_kept(&self, id: HandleId) -> String {
        self.handles.not_kept(id)
    }

    /// Whether the entry exists: kept, or let go and waiting for the next checkpoint.
    pub fn contains(&self, handle: impl Into<HandleId>) -> bool {
        self.handles.contains(handle.into())
    }

    /// Records that Dart's `dispose` has run on the object. The object stays readable for as
    /// long as something keeps it; this is a fact for tooling and for [`is_disposed`].
    ///
    /// [`is_disposed`]: Self::is_disposed
    pub fn mark_disposed(&mut self, handle: impl Into<HandleId>) {
        self.handles.mark_disposed(handle.into());
    }

    /// Whether a tear-off's receiver is still in the arena; a tear-off ([`Listener::handle_method`]
    /// and its twins) runs only when it is. One whose receiver was freed without removing it is
    /// skipped, as GPUI ends an observer when its receiver is gone; a debug build reports it once
    /// per receiver, with the receiver's type and creation site.
    pub fn tear_off_receiver_is_live(&mut self, receiver: impl Into<HandleId>) -> bool {
        let receiver = receiver.into();
        let live = self.handles.contains(receiver);
        #[cfg(debug_assertions)]
        if !live && self.skipped_tear_off_receivers.insert(receiver) {
            let message = format!(
                "waft: a listener whose receiver was freed without removing it was skipped: {}",
                self.handles.debug_describe(receiver)
            );
            eprintln!("{message}");
            self.skipped_tear_offs.push(message);
        }
        live
    }

    /// Records that the object has a `dispose` its holder must call, as Flutter's
    /// `FlutterMemoryAllocations.dispatchObjectCreated` does: a debug build reports it if it is
    /// freed without one ([`debug_take_not_disposed`]). A [`ChangeNotifier`] records itself when
    /// it gets its first listener, as Dart's does; other types call this in their constructor.
    /// Nothing in a release build.
    ///
    /// [`debug_take_not_disposed`]: Self::debug_take_not_disposed
    /// [`ChangeNotifier`]: crate::ChangeNotifier
    pub fn expect_dispose(&mut self, handle: impl Into<HandleId>) {
        self.handles.expect_dispose(handle.into());
    }

    /// Flutter's "not disposed" leaks reported since the last call: objects that expected a
    /// `dispose` ([`expect_dispose`](Self::expect_dispose)) and were freed without one, each with
    /// its type and creation site. For a test that turns them into a failure.
    #[cfg(debug_assertions)]
    pub fn debug_take_not_disposed(&mut self) -> Vec<String> {
        self.handles.take_not_disposed()
    }

    /// Flutter's "not collected" leaks reported since the last call: objects disposed
    /// ([`mark_disposed`](Self::mark_disposed)) and still kept two checkpoints later, each with
    /// its type and creation site; a ring of `Ref`s, or a holder that disposed an object and kept
    /// it. For a test that turns them into a failure.
    #[cfg(debug_assertions)]
    pub fn debug_take_not_collected(&mut self) -> Vec<String> {
        self.handles.take_not_collected()
    }

    /// Every object the arena holds, kept or waiting for the next checkpoint, with its type and
    /// creation site: what a test's snapshot records to say afterwards which objects are new.
    #[cfg(debug_assertions)]
    pub fn debug_live_objects(&self) -> Vec<crate::DebugObject> {
        self.handles.debug_live_objects()
    }

    /// The skipped tear-offs reported since the last call, for a test that turns them into a
    /// failure.
    #[cfg(debug_assertions)]
    pub fn debug_take_skipped_tear_offs(&mut self) -> Vec<String> {
        std::mem::take(&mut self.skipped_tear_offs)
    }

    /// A checkpoint begins: the frees from here on are dated by it in the stale-read message.
    pub(crate) fn begin_checkpoint(&mut self) {
        self.handles.begin_checkpoint();
    }

    /// Frees the objects whose last keeper went since the last sweep.
    pub(crate) fn free_dropped_objects(&mut self) {
        self.handles.free_dropped();
    }

    /// Whether Dart's `dispose` has run on the object ([`mark_disposed`], or the tree's
    /// [`release_tree_node`]). `false` for a freed one.
    ///
    /// [`mark_disposed`]: Self::mark_disposed
    /// [`release_tree_node`]: Self::release_tree_node
    pub fn is_disposed(&self, handle: impl Into<HandleId>) -> bool {
        self.handles.is_disposed(handle.into())
    }

    /// Narrow a [`HandleId`]. `None` if freed or the wrong type. [`get`](App::get) panics instead.
    pub fn handle<T: 'static>(&self, handle: impl Into<HandleId>) -> Option<Handle<T>> {
        self.handles.handle(handle.into())
    }

    /// # Panics
    ///
    /// If the object is freed; the message names its type, where it was created and the
    /// checkpoint that freed it, while a debug build remembers them.
    pub fn get<T: 'static>(&self, handle: impl ToHandle<T>) -> &T {
        self.handles.get(handle.to_handle())
    }

    /// See [`get`](App::get).
    pub fn get_mut<T: 'static>(&mut self, handle: impl ToHandle<T>) -> &mut T {
        self.handles.get_mut(handle.to_handle())
    }

    /// Two live entries at once, for an object that works on another (a render object shaping
    /// its text against the font collection).
    ///
    /// # Panics
    ///
    /// If either handle is stale, or both name the same entry.
    pub fn get_disjoint_mut<A: 'static, B: 'static>(
        &mut self,
        a: impl ToHandle<A>,
        b: impl ToHandle<B>,
    ) -> (&mut A, &mut B) {
        self.handles.get_disjoint_mut(a.to_handle(), b.to_handle())
    }
}

/// What [`App::get`] reads through: a bare [`Handle`], or a borrowed [`Ref`], which reads without
/// giving up its count. A `Ref` by value is not one, so a read never drops the last keeper.
pub trait ToHandle<T> {
    /// The handle to read through.
    fn to_handle(self) -> Handle<T>;
}

impl<T> ToHandle<T> for Handle<T> {
    fn to_handle(self) -> Handle<T> {
        self
    }
}

impl<T> ToHandle<T> for &Handle<T> {
    fn to_handle(self) -> Handle<T> {
        *self
    }
}

impl<T> ToHandle<T> for &Ref<T> {
    fn to_handle(self) -> Handle<T> {
        self.handle()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Timer;

    #[derive(Debug, Default, PartialEq)]
    struct Counter(i32);

    #[derive(Debug, PartialEq)]
    struct Label(&'static str);

    #[test]
    fn state_survives_a_round_trip_through_a_handle() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let counter_ref = app.create(Counter(1));
        let counter = counter_ref.handle();

        assert_eq!(app.get(counter), &Counter(1));
        app.get_mut(counter).0 = 7;
        assert_eq!(app.get(counter), &Counter(7));
    }

    #[test]
    fn handles_of_different_types_share_one_arena() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let counter_ref = app.create(Counter(1));
        let counter = counter_ref.handle();
        let label_ref = app.create(Label("hello"));
        let label = label_ref.handle();

        assert_eq!(app.get(counter), &Counter(1));
        assert_eq!(app.get(label), &Label("hello"));
    }

    #[test]
    fn a_freed_entry_is_reused_but_its_handle_is_not() {
        let cell = AppCell::new();
        let first_id = cell.borrow_mut().create(Counter(1)).id();
        cell.checkpoint();

        let mut app = cell.borrow_mut();
        let second_ref = app.create(Counter(2));
        let second = second_ref.handle();
        // slotmap: version in high 32 bits, index in low 32. Without reuse this test is vacuous.
        let entry_index = |id: HandleId| slotmap::Key::data(&id).as_ffi() & 0xffff_ffff;
        assert_eq!(
            entry_index(second.id()),
            entry_index(first_id),
            "the freed entry is reused, so the version is what tells them apart"
        );
        assert_ne!(second.id(), first_id);
        assert!(!app.contains(first_id));
        assert!(app.contains(second));
    }

    #[test]
    #[cfg(debug_assertions)]
    fn a_dropped_ref_keeps_the_object_readable_until_the_checkpoint() {
        let cell = AppCell::new();
        let counter_ref = cell.borrow_mut().create(Counter(3));
        let counter = counter_ref.handle();

        drop(counter_ref);
        {
            let app = cell.borrow();
            assert_eq!(app.get(counter), &Counter(3), "still readable");
            assert!(app.contains(counter), "still in the arena");
        }
        cell.checkpoint();
        assert!(!cell.borrow().contains(counter), "freed at the checkpoint");
    }

    #[test]
    #[should_panic(expected = "stale handle")]
    fn reading_through_a_stale_handle_panics() {
        let cell = AppCell::new();
        let counter = cell.borrow_mut().create(Counter(1)).handle();
        cell.checkpoint();
        cell.borrow().get(counter);
    }

    #[test]
    fn the_checked_path_from_an_untyped_id_answers_none_rather_than_panicking() {
        let cell = AppCell::new();
        let counter_ref = cell.borrow_mut().create(Counter(1));
        let counter = counter_ref.handle();
        let id = counter.id();

        assert_eq!(cell.borrow().handle::<Counter>(id), Some(counter));
        assert_eq!(cell.borrow().handle::<Label>(id), None, "wrong type");

        drop(counter_ref);
        cell.checkpoint();
        assert_eq!(cell.borrow().handle::<Counter>(id), None, "stale");
    }

    #[test]
    fn a_singleton_is_one_handle_per_type_per_app() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();

        let first: Handle<Counter> = app.singleton();
        let second: Handle<Counter> = app.singleton();
        assert_eq!(first, second);

        app.get_mut(first).0 = 9;
        assert_eq!(app.get(second).0, 9, "one entry behind both handles");

        let other_cell = AppCell::new();
        let mut other_app = other_cell.borrow_mut();
        let elsewhere: Handle<Counter> = other_app.singleton();
        assert_eq!(elsewhere.id(), first.id(), "ids may collide across Apps");
        assert_eq!(other_app.get(elsewhere).0, 0, "but the state is per App");
    }

    #[test]
    fn a_handle_is_copy_so_an_edge_can_be_a_plain_field() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let counter_ref = app.create(Counter(1));
        let counter = counter_ref.handle();

        struct Edge {
            target: Handle<Counter>,
        }
        let edge = Edge { target: counter };
        let copied = edge.target;

        assert_eq!(copied, counter);
        assert_eq!(app.get(edge.target).0, 1);
    }

    #[test]
    fn a_microtask_runs_on_drain_not_inline() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let counter_ref = app.create(Counter(0));
        let counter = counter_ref.handle();
        app.schedule_microtask(crate::Listener::new(move |app| {
            app.get_mut(counter).0 = 1;
        }));
        assert_eq!(app.get(counter).0, 0);
        assert!(app.has_pending_microtasks());
        app.drain_microtasks();
        assert_eq!(app.get(counter).0, 1);
        assert!(!app.has_pending_microtasks());
    }

    #[test]
    fn a_microtask_scheduled_during_drain_runs_in_the_same_drain() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let counter_ref = app.create(Counter(0));
        let counter = counter_ref.handle();
        app.schedule_microtask(crate::Listener::new(move |app| {
            app.get_mut(counter).0 = 1;
            app.schedule_microtask(crate::Listener::new(move |app| {
                app.get_mut(counter).0 = 2;
            }));
        }));
        app.drain_microtasks();
        assert_eq!(app.get(counter).0, 2);
    }

    #[test]
    fn timers_fire_in_due_order_after_microtasks() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let log_ref = app.create(Vec::<&'static str>::new());
        let log = log_ref.handle();
        app.schedule_microtask(crate::Listener::new(move |app| {
            app.get_mut(log).push("micro");
        }));
        Timer::new(
            &mut app,
            Duration::from_millis(200),
            crate::Listener::new(move |app| {
                app.get_mut(log).push("late");
            }),
        );
        Timer::new(
            &mut app,
            Duration::from_millis(100),
            crate::Listener::new(move |app| {
                app.get_mut(log).push("early");
            }),
        );
        Timer::new(
            &mut app,
            Duration::from_millis(100),
            crate::Listener::new(move |app| {
                app.get_mut(log).push("early-second");
            }),
        );
        assert!(app.get(log).is_empty());
        drop(app);
        cell.elapse(Duration::from_millis(200));
        let app = cell.borrow();
        assert_eq!(
            app.get(log).as_slice(),
            ["micro", "early", "early-second", "late"]
        );
    }

    #[test]
    fn cancel_prevents_a_timer_from_firing() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let counter_ref = app.create(Counter(0));
        let counter = counter_ref.handle();
        let timer = Timer::new(
            &mut app,
            Duration::from_millis(10),
            crate::Listener::new(move |app| {
                app.get_mut(counter).0 = 1;
            }),
        );
        assert!(timer.is_active(&app));
        timer.cancel(&mut app);
        assert!(!timer.is_active(&app));
        drop(app);
        cell.elapse(Duration::from_millis(10));
        let app = cell.borrow();
        assert_eq!(app.get(counter).0, 0);
    }
    #[test]
    fn get_disjoint_mut_borrows_two_entries_at_once() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let a_ref = app.create(1u32);
        let a = a_ref.handle();
        let b_ref = app.create(String::from("x"));
        let b = b_ref.handle();
        let (a_value, b_value) = app.get_disjoint_mut(a, b);
        *a_value += 1;
        b_value.push('y');
        assert_eq!(*app.get(a), 2);
        assert_eq!(app.get(b), "xy");
    }

    #[test]
    #[should_panic(expected = "not two live entries")]
    fn get_disjoint_mut_rejects_the_same_entry_twice() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let a_ref = app.create(1u32);
        let a = a_ref.handle();
        let _ = app.get_disjoint_mut(a, a);
    }

    #[test]
    fn from_id_reads_like_the_minted_handle() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let minted_ref = app.create(Counter(7));
        let minted = minted_ref.handle();
        let rebuilt = Handle::<Counter>::from_id(minted.id());
        assert_eq!(rebuilt, minted);
        assert_eq!(app.get(rebuilt).0, 7);
    }

    #[test]
    #[should_panic(expected = "stale handle")]
    fn from_id_defers_the_stale_check_to_get() {
        let cell = AppCell::new();
        let id = cell.borrow_mut().create(Counter(0)).id();
        cell.checkpoint();
        let rebuilt = Handle::<Counter>::from_id(id);
        cell.borrow().get(rebuilt);
    }
}
