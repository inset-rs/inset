//! Flutter counterpart: material_ui's `tab_controller.dart`.

use std::fmt;
use std::rc::Rc;
use std::time::Duration;

use waft_animation::{AnimationBehavior, AnimationController, AnyAnimation, Curve, Curves};
use waft_foundation::{App, ChangeNotifier, ChangeNotifierData, Handle, Listener, Ref};
use waft_scheduler::{Ticker, TickerCallback, TickerProvider, TickerProviderObject};
use waft_widgets::{
    BuildContext, InheritedWidget, IntoWidget, KeyRef, SingleTickerProviderStateMixin,
    SingleTickerProviderStateMixinData, State, StateData, StatefulWidget, TickerMode, WidgetRef,
};

use crate::constants::K_TAB_SCROLL_DURATION;

/// Coordinates tab selection between a `TabBar` and a `TabBarView`.
///
/// The [`index`](Self::index) property is the index of the selected tab and the
/// [`animation`](Self::animation) represents the current scroll positions of the tab bar and
/// the tab bar view. The selected tab's index can be changed with
/// [`animate_to`](Self::animate_to).
///
/// A stateful widget that builds a `TabBar` or a `TabBarView` can create
/// a [`TabController`] and share it directly.
///
/// When the `TabBar` and `TabBarView` don't have a convenient stateful
/// ancestor, a [`TabController`] can be shared by providing a
/// [`DefaultTabController`] inherited widget.
///
/// A disposed controller stays in the arena, as a Dart object outlives its `dispose` while
/// something still holds it: a `TabBar` rebuilt with a new controller reads the old one's
/// [`animation`](Self::animation), `None`, to learn it was disposed. It is freed once nothing
/// holds it.
pub struct TabController {
    change_notifier: ChangeNotifierData,
    animation_controller: Option<Ref<AnimationController>>,
    animation_duration: Duration,

    /// The total number of tabs.
    ///
    /// Typically greater than one. Must match `TabBar::tabs`'s and
    /// `TabBarView::children`'s length.
    pub length: usize,

    index: usize,
    previous_index: usize,
    index_is_changing_count: i32,
}

impl TabController {
    /// Creates an object that manages the state required by `TabBar` and a
    /// `TabBarView`.
    ///
    /// The `length` is typically a value greater than one, i.e. typically there are two or more
    /// tabs. The `length` must match `TabBar::tabs`'s and `TabBarView::children`'s length.
    ///
    /// The `initial_index` must be valid given `length`. If `length` is zero, then
    /// `initial_index` must be 0 (Dart's default).
    pub fn new(
        app: &mut App,
        initial_index: usize,
        animation_duration: Option<Duration>,
        length: usize,
        vsync: impl TickerProvider,
    ) -> Ref<TabController> {
        debug_assert!(length == 0 || initial_index < length);
        let animation_controller = AnimationController::unbounded(
            app,
            initial_index as f64,
            None,
            None,
            AnimationBehavior::Preserve,
            vsync,
        );
        app.create(TabController {
            change_notifier: ChangeNotifierData::new(),
            animation_controller: Some(animation_controller),
            animation_duration: animation_duration.unwrap_or(K_TAB_SCROLL_DURATION),
            length,
            index: initial_index,
            previous_index: initial_index,
            index_is_changing_count: 0,
        })
    }

    // Private constructor used by `copy_with_and_dispose`. This allows a new TabController to
    // be created without having to create a new animationController.
    fn new_private(
        app: &mut App,
        index: usize,
        previous_index: usize,
        animation_controller: Option<Ref<AnimationController>>,
        animation_duration: Duration,
        length: usize,
    ) -> Ref<TabController> {
        app.create(TabController {
            change_notifier: ChangeNotifierData::new(),
            animation_controller,
            animation_duration,
            length,
            index,
            previous_index,
            index_is_changing_count: 0,
        })
    }

    /// Creates a new [`TabController`] with `index`, `previous_index`, `length`, and
    /// `animation_duration` if they are non-null, and disposes current instance.
    ///
    /// This method is used by [`DefaultTabController`].
    ///
    /// When [`DefaultTabController::length`] is updated, this method is called to
    /// create a new [`TabController`] without creating a new [`AnimationController`].
    /// Instead the animation controller is taken from the current instance and
    /// passed to the new instance.
    fn copy_with_and_dispose(
        self: Handle<Self>,
        app: &mut App,
        index: Option<usize>,
        length: Option<usize>,
        previous_index: Option<usize>,
        animation_duration: Option<Duration>,
    ) -> Ref<TabController> {
        if let Some(index) = index {
            self.animation_controller_handle(app)
                .set_value(app, index as f64);
        }
        let this = app.get(self);
        let (current_index, current_length, current_previous_index, current_duration) = (
            this.index,
            this.length,
            this.previous_index,
            this.animation_duration,
        );
        let animation_controller = this.animation_controller.clone();
        let result = TabController::new_private(
            app,
            index.unwrap_or(current_index),
            previous_index.unwrap_or(current_previous_index),
            animation_controller,
            animation_duration.unwrap_or(current_duration),
            length.unwrap_or(current_length),
        );
        app.get_mut(self).animation_controller = None;
        self.dispose(app);
        result
    }

    fn animation_controller_handle(self: Handle<Self>, app: &App) -> Handle<AnimationController> {
        app.get(self)
            .animation_controller
            .as_ref()
            .expect("a TabController is used after dispose")
            .handle()
    }

    /// An animation whose value represents the current position of the `TabBar`'s
    /// selected tab indicator as well as the scrollOffsets of the `TabBar`
    /// and `TabBarView`.
    ///
    /// The animation's value ranges from 0.0 to [`length`](Self::length) - 1.0. After the
    /// selected tab is changed, the animation's value equals [`index`](Self::index). The
    /// animation's value can be [`offset`](Self::offset) by +/- 1.0 to reflect `TabBarView`
    /// drag scrolling.
    ///
    /// If this [`TabController`] was disposed, then return `None`.
    pub fn animation(self: Handle<Self>, app: &App) -> Option<AnyAnimation<f64>> {
        app.get(self)
            .animation_controller
            .as_ref()
            .map(|controller| controller.view())
    }

    /// Controls the duration of TabController and TabBarView animations.
    ///
    /// Defaults to `K_TAB_SCROLL_DURATION`.
    pub fn animation_duration(self: Handle<Self>, app: &App) -> Duration {
        app.get(self).animation_duration
    }

    /// The total number of tabs; see [`length`](Self::length).
    pub fn length(self: Handle<Self>, app: &App) -> usize {
        app.get(self).length
    }

    fn change_index(
        self: Handle<Self>,
        app: &mut App,
        value: usize,
        duration: Option<Duration>,
        curve: Option<Rc<dyn Curve>>,
    ) {
        let this = app.get(self);
        debug_assert!(value < this.length || this.length == 0);
        debug_assert!(duration.is_some() || curve.is_none());
        debug_assert!(this.index_is_changing_count >= 0);
        if value == this.index || this.length < 2 {
            return;
        }
        let index = self.index(app);
        let this = app.get_mut(self);
        this.previous_index = index;
        this.index = value;
        let animation_controller = self.animation_controller_handle(app);
        if let Some(duration) = duration
            && duration > Duration::ZERO
        {
            app.get_mut(self).index_is_changing_count += 1;
            self.notify_listeners(app); // Because the value of indexIsChanging may have changed.
            let future = animation_controller.animate_to(
                app,
                value as f64,
                Some(duration),
                curve.expect("a duration comes with a curve"),
            );
            // Dart's closure captures `this`, so the controller stays alive until the
            // animation ends, even if it was disposed and every other reference went.
            let this = self.upgrade(app).expect("a live TabController");
            future.when_complete_or_cancel(
                app,
                Listener::new(move |app| {
                    let this = this.handle();
                    // don't notify if we've been disposed
                    if app.get(this).animation_controller.is_some() {
                        app.get_mut(this).index_is_changing_count -= 1;
                        this.notify_listeners(app);
                    }
                }),
            );
        } else {
            app.get_mut(self).index_is_changing_count += 1;
            animation_controller.set_value(app, value as f64);
            app.get_mut(self).index_is_changing_count -= 1;
            self.notify_listeners(app);
        }
    }

    /// The index of the currently selected tab.
    ///
    /// Changing the index also updates [`previous_index`](Self::previous_index), sets the
    /// [`animation`](Self::animation)'s value to index, resets
    /// [`index_is_changing`](Self::index_is_changing) to false, and notifies listeners.
    ///
    /// To change the currently selected tab and play the [`animation`](Self::animation) use
    /// [`animate_to`](Self::animate_to).
    ///
    /// The value of [`index`](Self::index) must be valid given [`length`](Self::length). If
    /// [`length`](Self::length) is zero, then [`index`](Self::index) will also be zero.
    pub fn index(self: Handle<Self>, app: &App) -> usize {
        app.get(self).index
    }

    /// Sets [`index`](Self::index).
    pub fn set_index(self: Handle<Self>, app: &mut App, value: usize) {
        self.change_index(app, value, None, None);
    }

    /// The index of the previously selected tab.
    ///
    /// Initially the same as [`index`](Self::index).
    pub fn previous_index(self: Handle<Self>, app: &App) -> usize {
        app.get(self).previous_index
    }

    /// True while we're animating from [`previous_index`](Self::previous_index) to
    /// [`index`](Self::index) as a consequence of calling [`animate_to`](Self::animate_to).
    ///
    /// This value is true during the [`animate_to`](Self::animate_to) animation that's
    /// triggered when the user taps a `TabBar` tab. It is false when [`offset`](Self::offset)
    /// is changing as a consequence of the user dragging (and "flinging") the `TabBarView`.
    pub fn index_is_changing(self: Handle<Self>, app: &App) -> bool {
        app.get(self).index_is_changing_count != 0
    }

    /// Immediately sets [`index`](Self::index) and [`previous_index`](Self::previous_index)
    /// and then plays the [`animation`](Self::animation) from its current value to
    /// [`index`](Self::index).
    ///
    /// While the animation is running [`index_is_changing`](Self::index_is_changing) is true.
    /// When the animation completes [`offset`](Self::offset) will be 0.0.
    ///
    /// Dart's defaults: `duration` the controller's
    /// [`animation_duration`](Self::animation_duration), `curve` `Curves::ease()`.
    pub fn animate_to(
        self: Handle<Self>,
        app: &mut App,
        value: usize,
        duration: Option<Duration>,
        curve: Option<Rc<dyn Curve>>,
    ) {
        let duration = duration.unwrap_or(app.get(self).animation_duration);
        self.change_index(
            app,
            value,
            Some(duration),
            Some(curve.unwrap_or_else(Curves::ease)),
        );
    }

    /// The difference between the [`animation`](Self::animation)'s value and
    /// [`index`](Self::index).
    ///
    /// The offset value must be between -1.0 and 1.0.
    ///
    /// This property is typically set by the `TabBarView` when the user
    /// drags left or right. A value between -1.0 and 0.0 implies that the
    /// TabBarView has been dragged to the left. Similarly a value between
    /// 0.0 and 1.0 implies that the TabBarView has been dragged to the right.
    pub fn offset(self: Handle<Self>, app: &App) -> f64 {
        self.animation_controller_handle(app).value(app) - app.get(self).index as f64
    }

    /// Sets [`offset`](Self::offset).
    pub fn set_offset(self: Handle<Self>, app: &mut App, value: f64) {
        debug_assert!((-1.0..=1.0).contains(&value));
        debug_assert!(!self.index_is_changing(app));
        if value == self.offset(app) {
            return;
        }
        let index = app.get(self).index as f64;
        self.animation_controller_handle(app)
            .set_value(app, value + index);
    }

    /// Discards the controller's animation; Dart's `dispose`.
    ///
    /// The disposed animation controller lives on, as Dart's outlives its `dispose`: a tab
    /// bar's indicator painter and its tabs' animations still hold it, and take their
    /// listeners off it only when they are replaced.
    pub fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(animation_controller) = app.get_mut(self).animation_controller.take() {
            animation_controller.dispose(app);
        }
        ChangeNotifierData::dispose(app, self);
    }
}

impl ChangeNotifier for TabController {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

/// Dart's `_TabControllerScope`.
#[derive(Debug)]
struct TabControllerScope {
    controller: Ref<TabController>,
    enabled: bool,
    child: WidgetRef,
}

impl InheritedWidget for TabControllerScope {
    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old: &TabControllerScope) -> bool {
        self.enabled != old.enabled || self.controller != old.controller
    }
}

/// The [`TabController`] for descendant widgets that don't specify one
/// explicitly.
///
/// [`DefaultTabController`] is an inherited widget that is used to share a
/// [`TabController`] with a `TabBar` or a `TabBarView`. It's used when sharing an
/// explicitly created [`TabController`] isn't convenient because the tab bar
/// widgets are created by a stateless parent widget or by different parent
/// widgets.
pub struct DefaultTabController {
    pub key: Option<KeyRef>,

    /// The total number of tabs.
    ///
    /// Typically greater than one. Must match `TabBar::tabs`'s and
    /// `TabBarView::children`'s length.
    pub length: usize,

    /// The initial index of the selected tab.
    ///
    /// Defaults to zero.
    pub initial_index: usize,

    /// Controls the duration of DefaultTabController and TabBarView animations.
    ///
    /// Defaults to `K_TAB_SCROLL_DURATION`.
    pub animation_duration: Option<Duration>,

    /// The widget below this widget in the tree.
    ///
    /// Typically a `Scaffold` whose `AppBar` includes a `TabBar`.
    pub child: WidgetRef,
}

impl DefaultTabController {
    /// Creates a default tab controller for the given [`child`](Self::child) widget.
    ///
    /// The [`length`](Self::length) argument is typically greater than one. The
    /// [`length`](Self::length) must match `TabBar::tabs`'s and `TabBarView::children`'s length.
    pub fn new<K>(length: usize, child: impl IntoWidget<K>) -> DefaultTabController {
        DefaultTabController {
            key: None,
            length,
            initial_index: 0,
            animation_duration: None,
            child: child.into_widget(),
        }
    }

    /// Dart `DefaultTabController(key:)`.
    pub fn key(mut self, key: KeyRef) -> DefaultTabController {
        self.key = Some(key);
        self
    }

    /// Dart `DefaultTabController(initialIndex:)`.
    pub fn initial_index(mut self, initial_index: usize) -> DefaultTabController {
        self.initial_index = initial_index;
        self
    }

    /// Dart `DefaultTabController(animationDuration:)`.
    pub fn animation_duration(mut self, animation_duration: Duration) -> DefaultTabController {
        self.animation_duration = Some(animation_duration);
        self
    }

    /// Dart's constructor assert, across `length` and `initial_index`.
    fn debug_check_constructor(&self) {
        debug_assert!(self.length == 0 || self.initial_index < self.length);
    }

    /// The closest instance of [`DefaultTabController`] that encloses the given
    /// context, or `None` if none is found.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// let controller = DefaultTabController::maybe_of(app, context);
    /// ```
    ///
    /// Calling this method will create a dependency on the closest
    /// [`DefaultTabController`] in the `context`, if there is one.
    ///
    /// See also:
    ///
    /// * [`DefaultTabController::of`], which is similar to this method, but asserts
    ///   if no [`DefaultTabController`] ancestor is found.
    pub fn maybe_of(app: &mut App, context: BuildContext) -> Option<Ref<TabController>> {
        context
            .depend_on_inherited_widget_of_exact_type::<TabControllerScope>(app)
            .map(|scope| scope.controller.clone())
    }

    /// The closest instance of [`DefaultTabController`] that encloses the given
    /// context.
    ///
    /// If no instance is found, this method will assert in debug mode and throw
    /// an exception in release mode.
    ///
    /// Calling this method will create a dependency on the closest
    /// [`DefaultTabController`] in the `context`.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// let controller = DefaultTabController::of(app, context);
    /// ```
    ///
    /// See also:
    ///
    /// * [`DefaultTabController::maybe_of`], which is similar to this method, but
    ///   returns `None` if no [`DefaultTabController`] ancestor is found.
    pub fn of(app: &mut App, context: BuildContext) -> Ref<TabController> {
        let controller = DefaultTabController::maybe_of(app, context);
        if cfg!(debug_assertions) && controller.is_none() {
            panic!(
                "DefaultTabController.of() was called with a context that does not contain a \
                 DefaultTabController widget.\n\
                 No DefaultTabController widget ancestor could be found starting from the \
                 context that was passed to DefaultTabController.of(). This can happen because \
                 you are using a widget that looks for a DefaultTabController ancestor, but no \
                 such ancestor exists.\n\
                 The context used was:\n  {context:?}"
            );
        }
        controller.expect("no DefaultTabController ancestor")
    }
}

impl fmt::Debug for DefaultTabController {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DefaultTabController")
            .field("length", &self.length)
            .field("initial_index", &self.initial_index)
            .field("animation_duration", &self.animation_duration)
            .finish()
    }
}

impl StatefulWidget for DefaultTabController {
    type State = DefaultTabControllerState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn debug_check_constructor(&self) {
        DefaultTabController::debug_check_constructor(self);
    }

    fn create_state(&self) -> DefaultTabControllerState {
        DefaultTabControllerState {
            state: StateData::new(),
            single_ticker_provider: SingleTickerProviderStateMixinData::default(),
            controller: None,
        }
    }
}

/// Dart's `_DefaultTabControllerState`.
pub struct DefaultTabControllerState {
    state: StateData<DefaultTabController>,
    single_ticker_provider: SingleTickerProviderStateMixinData,
    /// Dart's `late TabController _controller`, created in `init_state`.
    controller: Option<Ref<TabController>>,
}

impl DefaultTabControllerState {
    fn controller(self: Handle<Self>, app: &App) -> Ref<TabController> {
        app.get(self)
            .controller
            .clone()
            .expect("created in init_state")
    }
}

impl SingleTickerProviderStateMixin for DefaultTabControllerState {
    fn single_ticker_provider_data(
        self: Handle<Self>,
        app: &App,
    ) -> &SingleTickerProviderStateMixinData {
        &app.get(self).single_ticker_provider
    }

    fn single_ticker_provider_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut SingleTickerProviderStateMixinData {
        &mut app.get_mut(self).single_ticker_provider
    }
}

/// Dart's `vsync: this`.
impl TickerProviderObject for DefaultTabControllerState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Ref<Ticker> {
        SingleTickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl State for DefaultTabControllerState {
    type Widget = DefaultTabController;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let widget = self.widget(app);
        let (length, initial_index, animation_duration) = (
            widget.length,
            widget.initial_index,
            widget.animation_duration,
        );
        let controller = TabController::new(app, initial_index, animation_duration, length, self);
        app.get_mut(self).controller = Some(controller);
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        SingleTickerProviderStateMixin::activate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let controller = self.controller(app);
        controller.dispose(app);
        SingleTickerProviderStateMixin::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        #[allow(deprecated)]
        let enabled = TickerMode::of(app, context);
        TabControllerScope {
            controller: self.controller(app),
            enabled,
            child: self.widget(app).child.clone(),
        }
        .into_widget()
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &DefaultTabController) {
        let widget = self.widget(app);
        let (length, animation_duration) = (widget.length, widget.animation_duration);
        if old_widget.length != length {
            // If the length is shortened while the last tab is selected, we should
            // automatically update the index of the controller to be the new last tab.
            let controller = self.controller(app);
            let mut new_index = None;
            let mut previous_index = controller.previous_index(app);
            if controller.index(app) >= length {
                new_index = Some(length.saturating_sub(1));
                previous_index = controller.index(app);
            }
            let controller = controller.copy_with_and_dispose(
                app,
                new_index,
                Some(length),
                Some(previous_index),
                animation_duration,
            );
            app.get_mut(self).controller = Some(controller);
        }

        if old_widget.animation_duration != animation_duration {
            let controller = self.controller(app);
            let (index, previous_index) = (controller.index(app), controller.previous_index(app));
            let controller = controller.copy_with_and_dispose(
                app,
                Some(index),
                Some(length),
                Some(previous_index),
                animation_duration,
            );
            app.get_mut(self).controller = Some(controller);
        }
    }
}
