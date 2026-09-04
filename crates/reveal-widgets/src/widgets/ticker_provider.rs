//! Flutter counterpart: `widgets/ticker_provider.dart`.

use std::any::type_name;
use std::collections::HashSet;
use std::rc::Rc;

use reveal_foundation::{App, Handle, Listenable, Listener, ValueListenable, ValueNotifier};
pub use reveal_scheduler::TickerProvider;
use reveal_scheduler::{Ticker, TickerCallback};

use crate::framework::{
    BuildContext, InheritedWidget, IntoWidget, KeyRef, State, StateData, StatefulWidget, WidgetRef,
};

/// Enables or disables tickers (and thus animation controllers) in the widget
/// subtree.
///
/// This only works if `AnimationController` objects are created using
/// widget-aware ticker providers. For example, using a
/// [`TickerProviderStateMixin`] or a [`SingleTickerProviderStateMixin`].
#[derive(Debug)]
pub struct TickerMode {
    /// See [`Widget::key`](crate::Widget::key).
    pub key: Option<KeyRef>,

    /// The requested ticker mode for this subtree.
    ///
    /// The effective ticker mode of this subtree may differ from this value
    /// if there is an ancestor [`TickerMode`] with this field set to false.
    ///
    /// If true and all ancestor [`TickerMode`]s are also enabled, then tickers in
    /// this subtree will tick.
    ///
    /// If false, then tickers in this subtree will not tick regardless of any
    /// ancestor [`TickerMode`]s. Animations driven by such tickers are not paused,
    /// they just don't call their callbacks. Time still elapses.
    pub enabled: bool,

    /// If true, tickers in this subtree will force frames even if
    /// frames would normally not be scheduled (e.g. even if the
    /// device's screen is turned off).
    ///
    /// Use sparingly as this will cause significantly higher battery
    /// usage when the device should be idle.
    pub force_frames: bool,

    /// The widget below this widget in the tree.
    pub child: WidgetRef,
}

impl TickerMode {
    /// Whether tickers in the given subtree should be enabled or disabled.
    ///
    /// This is used automatically by [`TickerProviderStateMixin`] and
    /// [`SingleTickerProviderStateMixin`] to decide if their tickers should be
    /// enabled or disabled.
    ///
    /// In the absence of a [`TickerMode`] widget, this function defaults to true.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// let ticking_enabled = TickerMode::of(app, context);
    /// ```
    #[deprecated(
        note = "Use TickerMode::values_of to get both enabled and force_frames. This feature was \
                deprecated after v3.35.0-0.0.pre."
    )]
    pub fn of(app: &mut App, context: BuildContext) -> bool {
        let widget = context.depend_on_inherited_widget_of_exact_type::<EffectiveTickerMode>(app);
        widget.is_none_or(|widget| widget.enabled)
    }

    /// Obtains a [`ValueListenable`] from the [`TickerMode`] surrounding the `context`,
    /// which indicates whether tickers are enabled in the given subtree.
    ///
    /// When that [`TickerMode`] enables or disables tickers, the listenable notifies
    /// its listeners.
    ///
    /// While the [`ValueListenable`] is stable for the lifetime of the surrounding
    /// [`TickerMode`], calling this method does not establish a dependency between
    /// the `context` and the [`TickerMode`] and the widget owning the `context`
    /// does not rebuild when the ticker mode changes from true to false or vice
    /// versa. This is preferable when the ticker mode does not impact what is
    /// currently rendered on screen, e.g. because it is only used to mute/unmute a
    /// [`Ticker`]. Since no dependency is established, the widget owning the
    /// `context` is also not informed when it is moved to a new location in the
    /// tree where it may have a different [`TickerMode`] ancestor. When this
    /// happens, the widget must manually unsubscribe from the old listenable,
    /// obtain a new one from the new ancestor [`TickerMode`] by calling this method
    /// again, and re-subscribe to it. [`StatefulWidget`]s can, for example, do this
    /// in [`State::activate`], which is called after the widget has been moved to
    /// a new location.
    ///
    /// Alternatively, [`of`](Self::of) can be used instead of this method to create a
    /// dependency between the provided `context` and the ancestor [`TickerMode`].
    /// In this case, the widget automatically rebuilds when the ticker mode
    /// changes or when it is moved to a new [`TickerMode`] ancestor, which
    /// simplifies the management cost in the widget at the expense of some
    /// potential unnecessary rebuilds.
    ///
    /// In the absence of a [`TickerMode`] widget, this function returns a
    /// [`ValueListenable`], whose [`ValueListenable::value`] is always true.
    #[deprecated(
        note = "Use TickerMode::get_values_notifier to get both enabled and force_frames. This \
                feature was deprecated after v3.35.0-0.0.pre."
    )]
    pub fn get_notifier(app: &App, context: BuildContext) -> Rc<dyn ValueListenable<bool>> {
        let widget = context.get_inherited_widget_of_exact_type::<EffectiveTickerMode>(app);
        match widget {
            Some(widget) => Rc::new(widget.notifier),
            None => Rc::new(ConstantValueListenable::new(true)),
        }
    }

    /// Returns the requested ticker mode values for this subtree and establishes
    /// a dependency on the ancestor [`TickerMode`], if any.
    ///
    /// This is used automatically by [`TickerProviderStateMixin`] and
    /// [`SingleTickerProviderStateMixin`] to decide if their tickers should be
    /// enabled or disabled.
    ///
    /// In the absence of a [`TickerMode`] widget, this defaults to enabled
    /// tickers that don't force frames.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// let ticking_enabled = TickerMode::values_of(app, context).enabled;
    /// ```
    pub fn values_of(app: &mut App, context: BuildContext) -> TickerModeData {
        let values_notifier = context
            .depend_on_inherited_widget_of_exact_type::<EffectiveTickerMode>(app)
            .map(|widget| widget.values_notifier);
        match values_notifier {
            Some(values_notifier) => *values_notifier.value(app),
            None => TickerModeData::FALLBACK,
        }
    }

    /// Obtains a [`ValueListenable`] from the [`TickerMode`] surrounding the `context`,
    /// which indicates whether tickers are enabled in the given subtree.
    ///
    /// When that [`TickerMode`] enabled or disabled tickers, the listenable notifies
    /// its listeners.
    ///
    /// While the [`ValueListenable`] is stable for the lifetime of the surrounding
    /// [`TickerMode`], calling this method does not establish a dependency between
    /// the `context` and the [`TickerMode`] and the widget owning the `context`
    /// does not rebuild when the ticker mode data changes. This is preferable
    /// when the ticker mode does not impact what is currently rendered on screen,
    /// e.g. because it is only used to mute/unmute a [`Ticker`]. Since no dependency
    /// is established, the widget owning the `context` is also not informed when
    /// it is moved to a new location in the tree where it may have a different
    /// [`TickerMode`] ancestor. When this happens, the widget must manually
    /// unsubscribe from the old listenable, obtain a new one from the new ancestor
    /// [`TickerMode`] by calling this method again, and re-subscribe to it.
    /// [`StatefulWidget`]s can, for example, do this in [`State::activate`],
    /// which is called after the widget has been moved to a new location.
    ///
    /// Alternatively, [`of`](Self::of) can be used instead of this method to create a
    /// dependency between the provided `context` and the ancestor [`TickerMode`].
    /// In this case, the widget automatically rebuilds when the ticker mode
    /// changes or when it is moved to a new [`TickerMode`] ancestor, which
    /// simplifies the management cost in the widget at the expensive of some
    /// potential unnecessary rebuilds.
    ///
    /// In the absence of a [`TickerMode`] widget, this function returns a
    /// [`ValueListenable`], whose [`ValueListenable::value`] is
    /// [`TickerModeData::FALLBACK`].
    ///
    /// Every call wraps the ancestor's notifier anew: two results for the same
    /// [`TickerMode`] are not `Rc::ptr_eq`, though they notify from the same object.
    pub fn get_values_notifier(
        app: &App,
        context: BuildContext,
    ) -> Rc<dyn ValueListenable<TickerModeData>> {
        let fallback = ConstantTickerModeDataListenable::new(TickerModeData::FALLBACK);

        // The getInheritedWidgetOfExactType() method throws an assertion error if called during
        // State.dispose(), which becomes a problem when an animation controller is set as a
        // late final class member and isn't referenced until then.
        if !context.mounted(app) {
            return Rc::new(fallback);
        }

        let widget = context.get_inherited_widget_of_exact_type::<EffectiveTickerMode>(app);
        match widget {
            Some(widget) => Rc::new(widget.values_notifier),
            None => Rc::new(fallback),
        }
    }
}

impl StatefulWidget for TickerMode {
    type State = TickerModeState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> TickerModeState {
        TickerModeState {
            state: StateData::new(),
            ancestor_ticker_mode: TickerModeData::FALLBACK.enabled,
            ancestor_force_frames: TickerModeData::FALLBACK.force_frames,
            effective_mode: None,
            effective_values: None,
        }
    }
}

/// The state of a [`TickerMode`] (Dart's `_TickerModeState`).
pub struct TickerModeState {
    state: StateData<TickerMode>,
    ancestor_ticker_mode: bool,
    ancestor_force_frames: bool,
    // Dart's field initializers: the notifiers need a slot in the App, which the state
    // does not have until `init_state`; nothing reads them before that.
    effective_mode: Option<Handle<ValueNotifier<bool>>>,
    effective_values: Option<Handle<ValueNotifier<TickerModeData>>>,
}

impl TickerModeState {
    fn effective_mode(self: Handle<Self>, app: &App) -> Handle<ValueNotifier<bool>> {
        app.get(self)
            .effective_mode
            .expect("the notifier is created in init_state")
    }

    fn effective_values(self: Handle<Self>, app: &App) -> Handle<ValueNotifier<TickerModeData>> {
        app.get(self)
            .effective_values
            .expect("the notifier is created in init_state")
    }

    fn update_effective_mode(self: Handle<Self>, app: &mut App) {
        let widget = self.widget(app);
        let enabled = app.get(self).ancestor_ticker_mode && widget.enabled;
        let force = app.get(self).ancestor_force_frames || widget.force_frames;
        self.effective_mode(app).set_value(app, enabled);
        self.effective_values(app).set_value(
            app,
            TickerModeData {
                enabled,
                force_frames: force,
            },
        );
    }
}

impl State for TickerModeState {
    type Widget = TickerMode;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let effective_mode = app.create(ValueNotifier::new(TickerModeData::FALLBACK.enabled));
        let effective_values = app.create(ValueNotifier::new(TickerModeData::FALLBACK));
        let state = app.get_mut(self);
        state.effective_mode = Some(effective_mode);
        state.effective_values = Some(effective_values);
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        let context = self.context(app);
        let parent = context.depend_on_inherited_widget_of_exact_type::<EffectiveTickerMode>(app);
        let ancestor_ticker_mode =
            parent.map_or(TickerModeData::FALLBACK.enabled, |parent| parent.enabled);
        let ancestor_force_frames = parent
            .map_or(TickerModeData::FALLBACK.force_frames, |parent| {
                parent.force_frames
            });
        let state = app.get_mut(self);
        state.ancestor_ticker_mode = ancestor_ticker_mode;
        state.ancestor_force_frames = ancestor_force_frames;
        self.update_effective_mode(app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, _old_widget: &TickerMode) {
        self.update_effective_mode(app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let effective_mode = self.effective_mode(app);
        let effective_values = self.effective_values(app);
        app.get_mut(effective_mode).dispose();
        app.get_mut(effective_values).dispose();
        app.destroy(effective_mode);
        app.destroy(effective_values);
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let effective_mode = self.effective_mode(app);
        let effective_values = self.effective_values(app);
        EffectiveTickerMode {
            enabled: *effective_mode.value(app),
            force_frames: effective_values.value(app).force_frames,
            notifier: effective_mode,
            values_notifier: effective_values,
            child: self.widget(app).child.clone(),
        }
        .into_widget()
    }
}

/// Dart's `_EffectiveTickerMode`: the inherited widget a [`TickerMode`] builds.
#[derive(Debug)]
struct EffectiveTickerMode {
    enabled: bool,
    force_frames: bool,
    notifier: Handle<ValueNotifier<bool>>,
    values_notifier: Handle<ValueNotifier<TickerModeData>>,
    child: WidgetRef,
}

impl InheritedWidget for EffectiveTickerMode {
    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &EffectiveTickerMode) -> bool {
        self.enabled != old_widget.enabled || self.force_frames != old_widget.force_frames
    }
}

/// The fields of Dart's `SingleTickerProviderStateMixin`.
#[derive(Default)]
pub struct SingleTickerProviderStateMixinData {
    ticker: Option<Handle<Ticker>>,
    ticker_mode_notifier: Option<Rc<dyn ValueListenable<TickerModeData>>>,
}

impl SingleTickerProviderStateMixinData {
    /// No ticker vended yet.
    pub fn new() -> SingleTickerProviderStateMixinData {
        SingleTickerProviderStateMixinData::default()
    }
}

/// Provides a single [`Ticker`] that is configured to only tick while the current
/// tree is enabled, as defined by [`TickerMode`].
///
/// To create the `AnimationController` in a [`State`] that only uses a single
/// `AnimationController`, mix in this class, then pass `vsync: this`
/// to the animation controller constructor.
///
/// This mixin only supports vending a single ticker. If you might have multiple
/// `AnimationController` objects over the lifetime of the [`State`], use a full
/// [`TickerProviderStateMixin`] instead.
///
/// The state holds a [`SingleTickerProviderStateMixinData`] under the field
/// `single_ticker_provider` and implements the accessor pair. Rust cannot interpose a
/// mixin on [`State`]'s hooks, so the state's own [`State::activate`] and
/// [`State::dispose`] call [`activate`](Self::activate) and [`dispose`](Self::dispose)
/// here, where Dart's mixin body would run; `vsync: this` is
/// [`create_ticker`](Self::create_ticker) on the state handle.
pub trait SingleTickerProviderStateMixin: State {
    /// Mixin field access.
    fn single_ticker_provider_data(
        self: Handle<Self>,
        app: &App,
    ) -> &SingleTickerProviderStateMixinData;

    /// See [`single_ticker_provider_data`](Self::single_ticker_provider_data).
    fn single_ticker_provider_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut SingleTickerProviderStateMixinData;

    /// [`TickerProvider::create_ticker`]: creates a ticker with the given callback.
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
        debug_assert!(
            self.single_ticker_provider_data(app).ticker.is_none(),
            "{} is a SingleTickerProviderStateMixin but multiple tickers were created. A \
             SingleTickerProviderStateMixin can only be used as a TickerProvider once. If a State \
             is used for multiple AnimationController objects, or if it is passed to other \
             objects and those objects might use it more than one time in total, then instead of \
             mixing in a SingleTickerProviderStateMixin, use a regular TickerProviderStateMixin.",
            type_name::<Self>()
        );
        let ticker = Ticker::new(app, on_tick);
        self.single_ticker_provider_data_mut(app).ticker = Some(ticker);
        self.update_ticker_mode_notifier(app);
        self.update_ticker(app); // Sets _ticker.mute correctly.
        ticker
    }

    /// The mixin's part of [`State::dispose`]; the state calls it where Dart's
    /// `super.dispose()` would reach the mixin. The vended ticker must already be
    /// disposed (by disposing its `AnimationController`).
    fn dispose(self: Handle<Self>, app: &mut App) {
        let ticker = self.single_ticker_provider_data(app).ticker;
        debug_assert!(
            ticker.is_none_or(|ticker| !ticker.is_active(app)),
            "{name} was disposed with an active Ticker. {name} created a Ticker via its \
             SingleTickerProviderStateMixin, but at the time dispose() was called on the mixin, \
             that Ticker was still active. The Ticker must be disposed before calling \
             super.dispose(). Tickers used by AnimationControllers should be disposed by calling \
             dispose() on the AnimationController itself. Otherwise, the ticker will leak.",
            name = type_name::<Self>()
        );
        if let Some(notifier) = self
            .single_ticker_provider_data_mut(app)
            .ticker_mode_notifier
            .take()
        {
            notifier.remove_listener(app, &Listener::handle_method(self, Self::update_ticker));
        }
    }

    /// The mixin's part of [`State::activate`]; the state calls it where Dart's
    /// `super.activate()` would reach the mixin.
    fn activate(self: Handle<Self>, app: &mut App) {
        // We may have a new TickerMode ancestor.
        self.update_ticker_mode_notifier(app);
        self.update_ticker(app);
    }
}

/// Dart's private members of `SingleTickerProviderStateMixin`.
trait SingleTickerProviderStateMixinPrivate: SingleTickerProviderStateMixin {
    fn update_ticker(self: Handle<Self>, app: &mut App) {
        let values = *self
            .single_ticker_provider_data(app)
            .ticker_mode_notifier
            .as_ref()
            .expect("set before the ticker is vended")
            .value(app);
        if let Some(ticker) = self.single_ticker_provider_data(app).ticker {
            ticker.set_muted(app, !values.enabled);
            app.get_mut(ticker).force_frames = values.force_frames;
        }
    }

    fn update_ticker_mode_notifier(self: Handle<Self>, app: &mut App) {
        let new_notifier = TickerMode::get_values_notifier(app, self.context(app));
        if let Some(current) = &self.single_ticker_provider_data(app).ticker_mode_notifier
            && Rc::ptr_eq(current, &new_notifier)
        {
            return;
        }
        if let Some(current) = self
            .single_ticker_provider_data_mut(app)
            .ticker_mode_notifier
            .take()
        {
            current.remove_listener(app, &Listener::handle_method(self, Self::update_ticker));
        }
        new_notifier.add_listener(app, Listener::handle_method(self, Self::update_ticker));
        self.single_ticker_provider_data_mut(app)
            .ticker_mode_notifier = Some(new_notifier);
    }
}

impl<S: SingleTickerProviderStateMixin> SingleTickerProviderStateMixinPrivate for S {}

/// The fields of Dart's `TickerProviderStateMixin`.
#[derive(Default)]
pub struct TickerProviderStateMixinData {
    tickers: Option<HashSet<Handle<Ticker>>>,
    ticker_mode_notifier: Option<Rc<dyn ValueListenable<TickerModeData>>>,
}

impl TickerProviderStateMixinData {
    /// No tickers vended yet.
    pub fn new() -> TickerProviderStateMixinData {
        TickerProviderStateMixinData::default()
    }
}

/// Provides [`Ticker`] objects that are configured to only tick while the current
/// tree is enabled, as defined by [`TickerMode`].
///
/// To create an `AnimationController` in a class that uses this mixin, pass
/// `vsync: this` to the animation controller constructor whenever you
/// create a new animation controller.
///
/// If you only have a single [`Ticker`] (for example only a single
/// `AnimationController`) for the lifetime of your [`State`], then using a
/// [`SingleTickerProviderStateMixin`] is more efficient. This is the common case.
///
/// When creating multiple `AnimationController`s, using a single state with
/// [`TickerProviderStateMixin`] as vsync for all `AnimationController`s is more
/// efficient than creating multiple states with
/// [`SingleTickerProviderStateMixin`].
///
/// The state holds a [`TickerProviderStateMixinData`] under the field `ticker_provider`
/// and wires [`activate`](Self::activate) / [`dispose`](Self::dispose) as
/// [`SingleTickerProviderStateMixin`] describes.
///
/// A vended ticker stays in the state's set after it is disposed: Dart's `_WidgetTicker`
/// overrides `Ticker.dispose` to unregister itself, and [`Ticker::dispose`] is not a
/// hook here. Muting a disposed ticker is a no-op.
pub trait TickerProviderStateMixin: State {
    /// Mixin field access.
    fn ticker_provider_data(self: Handle<Self>, app: &App) -> &TickerProviderStateMixinData;

    /// See [`ticker_provider_data`](Self::ticker_provider_data).
    fn ticker_provider_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut TickerProviderStateMixinData;

    /// [`TickerProvider::create_ticker`]: creates a ticker with the given callback.
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
        if self
            .ticker_provider_data(app)
            .ticker_mode_notifier
            .is_none()
        {
            // Setup TickerMode notifier before we vend the first ticker.
            self.update_ticker_mode_notifier(app);
        }
        debug_assert!(
            self.ticker_provider_data(app)
                .ticker_mode_notifier
                .is_some()
        );
        let data = self.ticker_provider_data_mut(app);
        if data.tickers.is_none() {
            data.tickers = Some(HashSet::new());
        }
        let values = *self
            .ticker_provider_data(app)
            .ticker_mode_notifier
            .as_ref()
            .expect("set above")
            .value(app);
        let result = Ticker::new(app, on_tick);
        result.set_muted(app, !values.enabled);
        app.get_mut(result).force_frames = values.force_frames;
        self.ticker_provider_data_mut(app)
            .tickers
            .as_mut()
            .expect("created above")
            .insert(result);
        result
    }

    /// The mixin's part of [`State::activate`]; the state calls it where Dart's
    /// `super.activate()` would reach the mixin.
    fn activate(self: Handle<Self>, app: &mut App) {
        // We may have a new TickerMode ancestor, get its Notifier.
        self.update_ticker_mode_notifier(app);
        self.update_tickers(app);
    }

    /// The mixin's part of [`State::dispose`]; the state calls it where Dart's
    /// `super.dispose()` would reach the mixin. Every vended ticker must already be
    /// disposed (by disposing its `AnimationController`).
    fn dispose(self: Handle<Self>, app: &mut App) {
        if cfg!(debug_assertions)
            && let Some(tickers) = &self.ticker_provider_data(app).tickers
        {
            for ticker in tickers.iter().copied() {
                assert!(
                    !ticker.is_active(app),
                    "{name} was disposed with an active Ticker. {name} created a Ticker via its \
                     TickerProviderStateMixin, but at the time dispose() was called on the mixin, \
                     that Ticker was still active. All Tickers must be disposed before calling \
                     super.dispose(). Tickers used by AnimationControllers should be disposed by \
                     calling dispose() on the AnimationController itself. Otherwise, the ticker \
                     will leak.",
                    name = type_name::<Self>()
                );
            }
        }
        if let Some(notifier) = self
            .ticker_provider_data_mut(app)
            .ticker_mode_notifier
            .take()
        {
            notifier.remove_listener(app, &Listener::handle_method(self, Self::update_tickers));
        }
    }
}

/// Dart's private members of `TickerProviderStateMixin`.
trait TickerProviderStateMixinPrivate: TickerProviderStateMixin {
    fn update_tickers(self: Handle<Self>, app: &mut App) {
        let tickers: Option<Vec<Handle<Ticker>>> = self
            .ticker_provider_data(app)
            .tickers
            .as_ref()
            .map(|tickers| tickers.iter().copied().collect());
        if let Some(tickers) = tickers {
            let values = *self
                .ticker_provider_data(app)
                .ticker_mode_notifier
                .as_ref()
                .expect("set before the first ticker is vended")
                .value(app);
            let muted = !values.enabled;
            for ticker in tickers {
                ticker.set_muted(app, muted);
                app.get_mut(ticker).force_frames = values.force_frames;
            }
        }
    }

    fn update_ticker_mode_notifier(self: Handle<Self>, app: &mut App) {
        let new_notifier = TickerMode::get_values_notifier(app, self.context(app));
        if let Some(current) = &self.ticker_provider_data(app).ticker_mode_notifier
            && Rc::ptr_eq(current, &new_notifier)
        {
            return;
        }
        if let Some(current) = self
            .ticker_provider_data_mut(app)
            .ticker_mode_notifier
            .take()
        {
            current.remove_listener(app, &Listener::handle_method(self, Self::update_tickers));
        }
        new_notifier.add_listener(app, Listener::handle_method(self, Self::update_tickers));
        self.ticker_provider_data_mut(app).ticker_mode_notifier = Some(new_notifier);
    }
}

impl<S: TickerProviderStateMixin> TickerProviderStateMixinPrivate for S {}

// Dart's `_WidgetTicker` (a `Ticker` whose `dispose` removes it from its creator's set)
// waits for a dispose hook on `Ticker`; see the crate's PORTING.md.

/// Dart's `_ConstantValueListenable<T>`: a value that never changes.
struct ConstantValueListenable<T> {
    value: T,
}

impl<T> ConstantValueListenable<T> {
    const fn new(value: T) -> ConstantValueListenable<T> {
        ConstantValueListenable { value }
    }
}

impl<T> Listenable for ConstantValueListenable<T> {
    fn add_listener(&self, _app: &mut App, _listener: Listener) {
        // Intentionally left empty: Value cannot change, so we never have to
        // notify registered listeners.
    }

    fn remove_listener(&self, _app: &mut App, _listener: &Listener) {
        // Intentionally left empty: Value cannot change, so we never have to
        // notify registered listeners.
    }
}

impl<T> ValueListenable<T> for ConstantValueListenable<T> {
    fn value<'a>(&'a self, _app: &'a App) -> &'a T {
        &self.value
    }
}

/// Immutable compound values that describe the effective ticker behavior
/// for a subtree.
///
/// Instances of this class are produced by [`TickerMode::values_of`] and
/// [`TickerMode::get_values_notifier`] and reflect the values that apply at a given
/// location in the widget tree after taking ancestor [`TickerMode`] widgets into
/// account.
///
/// Semantics of the fields:
/// - [`enabled`](Self::enabled): A ticker is considered enabled only if all ancestor
///   [`TickerMode::enabled`] values and the local [`TickerMode::enabled`] are true
///   (logical AND). When false, tickers are muted (time still elapses but
///   callbacks are not invoked).
/// - [`force_frames`](Self::force_frames): When true, tickers in the subtree request
///   frames using `SchedulerBinding::schedule_forced_frame` while active. This value is
///   combined across ancestors using logical OR, so any ancestor requesting
///   forced frames enables it for the subtree.
///
/// For most widgets, reading these values is unnecessary; mixins such as
/// [`SingleTickerProviderStateMixin`] and [`TickerProviderStateMixin`] apply them
/// automatically to the [`Ticker`]s they vend. Use this class when you need to
/// observe or react to ticker policy explicitly.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TickerModeData {
    /// Whether tickers are enabled (not muted) for the subtree.
    ///
    /// Effective value is the logical AND of all ancestor and local
    /// [`TickerMode::enabled`] values.
    pub enabled: bool,

    /// Whether tickers should request forced frames while active.
    ///
    /// Effective value is the logical OR of all ancestor and local
    /// [`TickerMode::force_frames`] values. Forcing frames may increase battery
    /// usage, so use sparingly.
    pub force_frames: bool,
}

impl TickerModeData {
    /// Fallback values used when there is no ancestor [`TickerMode`].
    ///
    /// This corresponds to tickers being enabled and not forcing frames.
    pub const FALLBACK: TickerModeData = TickerModeData {
        enabled: true,
        force_frames: false,
    };
}

/// Dart's `_ConstantTickerModeDataListenable`: [`TickerModeData`] that never changes.
struct ConstantTickerModeDataListenable {
    value: TickerModeData,
}

impl ConstantTickerModeDataListenable {
    const fn new(value: TickerModeData) -> ConstantTickerModeDataListenable {
        ConstantTickerModeDataListenable { value }
    }
}

impl Listenable for ConstantTickerModeDataListenable {
    fn add_listener(&self, _app: &mut App, _listener: Listener) {}

    fn remove_listener(&self, _app: &mut App, _listener: &Listener) {}
}

impl ValueListenable<TickerModeData> for ConstantTickerModeDataListenable {
    fn value<'a>(&'a self, _app: &'a App) -> &'a TickerModeData {
        &self.value
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::time::Duration;

    use reveal_animation::{AnimationBehavior, AnimationController};
    use reveal_embedder::Size;
    use reveal_foundation::ChangeNotifier;
    use reveal_rendering::{
        AnyRenderObject, BoxConstraints, RenderBox, RenderConstrainedBox, RenderHandle,
    };
    use reveal_scheduler::{SchedulerBinding, TickerFuture, TickerProviderObject};

    use super::*;
    use crate::framework::{
        AnyElement, Element, LeafRenderObjectWidget, RenderObjectWidget, StatelessWidget,
    };
    use crate::test_harness::Harness;

    /// `SizedBox` in miniature.
    #[derive(Debug)]
    struct Sized;

    impl RenderObjectWidget for Sized {
        type RenderObject = RenderConstrainedBox;

        fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
            RenderConstrainedBox::new(app, BoxConstraints::tight(Size::new(10.0, 10.0)), None)
                .as_object()
        }

        fn update_render_object(
            &self,
            _app: &mut App,
            _context: BuildContext,
            _render_object: RenderHandle<RenderConstrainedBox>,
        ) {
        }
    }

    impl LeafRenderObjectWidget for Sized {}

    fn ticker_mode(enabled: bool, child: WidgetRef) -> WidgetRef {
        TickerMode {
            key: None,
            enabled,
            force_frames: false,
            child,
        }
        .into_widget()
    }

    fn new_controller(app: &mut App, vsync: impl TickerProvider) -> Handle<AnimationController> {
        AnimationController::create(
            app,
            None,
            Some(Duration::from_millis(100)),
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            vsync,
        )
    }

    /// The elements under the test root, root-most first: TickerMode, its effective mode,
    /// then the host.
    fn descendant(harness: &Harness, app: &App, depth: usize) -> AnyElement {
        let mut element = harness.root.as_element();
        for _ in 0..=depth {
            element = element.children(app)[0];
        }
        element
    }

    // ---- a state with SingleTickerProviderStateMixin (ticker_provider_test.dart's _TickerWidget) ----

    #[derive(Debug)]
    struct SingleHost;

    impl StatefulWidget for SingleHost {
        type State = SingleHostState;

        fn create_state(&self) -> SingleHostState {
            SingleHostState {
                state: StateData::new(),
                single_ticker_provider: SingleTickerProviderStateMixinData::new(),
                controller: None,
                future: None,
            }
        }
    }

    struct SingleHostState {
        state: StateData<SingleHost>,
        single_ticker_provider: SingleTickerProviderStateMixinData,
        controller: Option<Handle<AnimationController>>,
        future: Option<Handle<TickerFuture>>,
    }

    /// Dart's `vsync: this`.
    impl TickerProviderObject for SingleHostState {
        fn create_ticker(
            self: Handle<Self>,
            app: &mut App,
            on_tick: TickerCallback,
        ) -> Handle<Ticker> {
            SingleTickerProviderStateMixin::create_ticker(self, app, on_tick)
        }
    }

    impl SingleTickerProviderStateMixin for SingleHostState {
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

    impl State for SingleHostState {
        type Widget = SingleHost;
        crate::state_accessors!();

        fn init_state(self: Handle<Self>, app: &mut App) {
            let controller = new_controller(app, self);
            let future = controller.forward(app, None);
            let state = app.get_mut(self);
            state.controller = Some(controller);
            state.future = Some(future);
        }

        fn activate(self: Handle<Self>, app: &mut App) {
            SingleTickerProviderStateMixin::activate(self, app);
        }

        fn dispose(self: Handle<Self>, app: &mut App) {
            let controller = app
                .get_mut(self)
                .controller
                .take()
                .expect("created in init_state");
            controller.dispose(app);
            SingleTickerProviderStateMixin::dispose(self, app);
        }

        fn build(self: Handle<Self>, _app: &mut App, _context: BuildContext) -> WidgetRef {
            Sized.into_widget()
        }
    }

    fn single_host_state(harness: &Harness, app: &App) -> Handle<SingleHostState> {
        descendant(harness, app, 2)
            .state_handle::<SingleHostState>(app)
            .expect("the host's state")
    }

    // ticker_provider_test.dart 'TickerMode'
    #[test]
    fn ticker_mode_mutes_the_ticker_a_single_provider_vends() {
        let mut app = App::new();
        let harness = Harness::mount(&mut app, ticker_mode(false, SingleHost.into_widget()));
        harness.pump(&mut app);
        assert_eq!(SchedulerBinding::transient_callback_count(&mut app), 0);
        let state = single_host_state(&harness, &app);
        let ticker = app
            .get(state)
            .single_ticker_provider
            .ticker
            .expect("vended");
        assert!(ticker.muted(&app));
        assert!(ticker.is_active(&app), "time still elapses while muted");

        harness.set_child(&mut app, ticker_mode(true, SingleHost.into_widget()));
        harness.pump(&mut app);
        assert!(!ticker.muted(&app));
        assert_eq!(SchedulerBinding::transient_callback_count(&mut app), 1);

        harness.set_child(&mut app, ticker_mode(false, SingleHost.into_widget()));
        harness.pump(&mut app);
        assert!(ticker.muted(&app));
        assert_eq!(SchedulerBinding::transient_callback_count(&mut app), 0);
    }

    #[test]
    fn a_single_provider_disposes_its_ticker_with_the_state() {
        let mut app = App::new();
        let harness = Harness::mount(&mut app, ticker_mode(true, SingleHost.into_widget()));
        harness.pump(&mut app);
        let mode_state = descendant(&harness, &app, 0)
            .state_handle::<TickerModeState>(&app)
            .expect("the TickerMode's state");
        let values_notifier = mode_state.effective_values(&app);
        assert!(
            app.get(values_notifier)
                .change_notifier_data()
                .has_listeners()
        );
        let state = single_host_state(&harness, &app);
        let ticker = app
            .get(state)
            .single_ticker_provider
            .ticker
            .expect("vended");
        let future = app.get(state).future.expect("started");
        let canceled = Rc::new(Cell::new(false));
        future.when_complete_or_cancel(
            &mut app,
            Listener::new({
                let canceled = Rc::clone(&canceled);
                move |_app| canceled.set(true)
            }),
        );

        harness.set_child(&mut app, ticker_mode(true, Sized.into_widget()));
        harness.pump(&mut app);
        app.drain_microtasks();
        assert!(!app.contains(state), "the state is gone");
        assert!(
            !ticker.is_active(&app),
            "the controller's dispose stopped the ticker"
        );
        assert!(
            canceled.get(),
            "the ticker was disposed, not merely stopped"
        );
        assert!(
            !app.get(values_notifier)
                .change_notifier_data()
                .has_listeners(),
            "the mixin's dispose unsubscribed from the TickerMode"
        );
        assert_eq!(SchedulerBinding::transient_callback_count(&mut app), 0);
    }

    // ---- a state with TickerProviderStateMixin ----

    #[derive(Debug)]
    struct MultiHost;

    impl StatefulWidget for MultiHost {
        type State = MultiHostState;

        fn create_state(&self) -> MultiHostState {
            MultiHostState {
                state: StateData::new(),
                ticker_provider: TickerProviderStateMixinData::new(),
                controllers: Vec::new(),
            }
        }
    }

    struct MultiHostState {
        state: StateData<MultiHost>,
        ticker_provider: TickerProviderStateMixinData,
        controllers: Vec<Handle<AnimationController>>,
    }

    impl TickerProviderObject for MultiHostState {
        fn create_ticker(
            self: Handle<Self>,
            app: &mut App,
            on_tick: TickerCallback,
        ) -> Handle<Ticker> {
            TickerProviderStateMixin::create_ticker(self, app, on_tick)
        }
    }

    impl TickerProviderStateMixin for MultiHostState {
        fn ticker_provider_data(self: Handle<Self>, app: &App) -> &TickerProviderStateMixinData {
            &app.get(self).ticker_provider
        }

        fn ticker_provider_data_mut(
            self: Handle<Self>,
            app: &mut App,
        ) -> &mut TickerProviderStateMixinData {
            &mut app.get_mut(self).ticker_provider
        }
    }

    impl State for MultiHostState {
        type Widget = MultiHost;
        crate::state_accessors!();

        fn init_state(self: Handle<Self>, app: &mut App) {
            for _ in 0..2 {
                let controller = new_controller(app, self);
                controller.forward(app, None);
                app.get_mut(self).controllers.push(controller);
            }
        }

        fn activate(self: Handle<Self>, app: &mut App) {
            TickerProviderStateMixin::activate(self, app);
        }

        fn dispose(self: Handle<Self>, app: &mut App) {
            for controller in std::mem::take(&mut app.get_mut(self).controllers) {
                controller.dispose(app);
            }
            TickerProviderStateMixin::dispose(self, app);
        }

        fn build(self: Handle<Self>, _app: &mut App, _context: BuildContext) -> WidgetRef {
            Sized.into_widget()
        }
    }

    #[test]
    fn ticker_mode_mutes_every_ticker_a_provider_vends() {
        let mut app = App::new();
        let harness = Harness::mount(&mut app, ticker_mode(false, MultiHost.into_widget()));
        harness.pump(&mut app);
        let state = descendant(&harness, &app, 2)
            .state_handle::<MultiHostState>(&app)
            .expect("the host's state");
        let tickers: Vec<Handle<Ticker>> = app
            .get(state)
            .ticker_provider
            .tickers
            .as_ref()
            .expect("vended")
            .iter()
            .copied()
            .collect();
        assert_eq!(tickers.len(), 2);
        assert!(tickers.iter().all(|ticker| ticker.muted(&app)));
        assert_eq!(SchedulerBinding::transient_callback_count(&mut app), 0);

        harness.set_child(&mut app, ticker_mode(true, MultiHost.into_widget()));
        harness.pump(&mut app);
        assert!(tickers.iter().all(|ticker| !ticker.muted(&app)));
        assert_eq!(SchedulerBinding::transient_callback_count(&mut app), 2);

        harness.set_child(&mut app, ticker_mode(true, Sized.into_widget()));
        harness.pump(&mut app);
        assert!(!app.contains(state));
        assert!(tickers.iter().all(|ticker| !ticker.is_active(&app)));
        assert_eq!(SchedulerBinding::transient_callback_count(&mut app), 0);
    }

    // ---- TickerMode.valuesOf ----

    /// Reads the ticker mode where it builds.
    #[derive(Debug)]
    struct ModeProbe {
        seen: Rc<Cell<Option<TickerModeData>>>,
    }

    impl StatelessWidget for ModeProbe {
        fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
            self.seen.set(Some(TickerMode::values_of(app, context)));
            Sized.into_widget()
        }
    }

    #[test]
    fn a_disabled_ancestor_wins_and_forced_frames_accumulate() {
        let mut app = App::new();
        let seen = Rc::new(Cell::new(None));
        let probe = ModeProbe {
            seen: Rc::clone(&seen),
        }
        .into_widget();
        let harness = Harness::mount(&mut app, probe.clone());
        harness.pump(&mut app);
        assert_eq!(seen.get(), Some(TickerModeData::FALLBACK));

        let inner = TickerMode {
            key: None,
            enabled: true,
            force_frames: true,
            child: probe,
        }
        .into_widget();
        harness.set_child(&mut app, ticker_mode(false, inner));
        harness.pump(&mut app);
        assert_eq!(
            seen.get(),
            Some(TickerModeData {
                enabled: false,
                force_frames: true,
            })
        );
    }
}
