//! Flutter counterpart: `widgets/undo_history.dart`.

use std::any::TypeId;
use std::cell::Cell;
use std::collections::HashMap;
use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use inset_embedder::TargetPlatform;
use inset_foundation::{
    App, ChangeNotifier, ChangeNotifierData, Handle, Listenable, Listener, Timer, ValueNotifier,
};
use inset_services::{UndoDirection, UndoManager, UndoManagerClient};

use crate::framework::{
    BuildContext, IntoWidget, KeyRef, State, StateData, StatefulWidget, WidgetRef,
};
use crate::widgets::actions::{Action, Actions, AnyAction, CallbackAction};
use crate::widgets::focus_manager::{FocusNode, FocusNodeLeaf};
use crate::widgets::text_editing_intents::{RedoTextIntent, UndoTextIntent};

/// Called when an undo or redo causes a state change.
pub type UndoHistoryOnTriggered<T> = Rc<dyn Fn(&mut App, T)>;

/// Called when checking whether a value change should be pushed onto the undo stack.
pub type ShouldChangeUndoStack<T> = Rc<dyn Fn(Option<&T>, &T) -> bool>;

/// Called right before a new entry is pushed to the undo stack.
pub type UndoStackModifier<T> = Rc<dyn Fn(T) -> T>;

/// A leaf [`UndoHistory`] can listen to and read, Dart's `ValueNotifier<T>`.
///
/// [`TextEditingController`](crate::TextEditingController) is a sibling leaf of
/// [`ValueNotifier`], not a subclass, so the widget is generic over this trait instead
/// of `Handle<ValueNotifier<T>>`.
pub trait UndoHistorySource<T>: ChangeNotifier + 'static {
    /// The current value. Dart's `ValueNotifier.value`.
    fn history_value(self: Handle<Self>, app: &App) -> &T;
}

impl<T: 'static> UndoHistorySource<T> for ValueNotifier<T> {
    fn history_value(self: Handle<Self>, app: &App) -> &T {
        app.get(self).value()
    }
}

/// Provides undo/redo capabilities for a [`ValueNotifier`].
///
/// Listens to [`value`](Self::value) and saves relevant values for undoing/redoing. The
/// cadence at which values are saved is a best approximation of the native
/// behaviors of a number of hardware keyboard on Flutter's desktop
/// platforms, as there are subtle differences between each of the platforms.
///
/// Listens to keyboard undo/redo shortcuts and calls [`on_triggered`](Self::on_triggered) when a
/// shortcut is triggered that would affect the state of the [`value`](Self::value).
///
/// The [`child`](Self::child) must manage focus on the [`focus_node`](Self::focus_node). For
/// example, using a `TextField` or `Focus` widget.
pub struct UndoHistory<T: Clone + PartialEq + 'static, V: UndoHistorySource<T> = ValueNotifier<T>> {
    /// See [`Widget::key`](crate::Widget::key).
    pub key: Option<KeyRef>,
    /// Called when checking whether a value change should be pushed onto
    /// the undo stack.
    pub should_change_undo_stack: Option<ShouldChangeUndoStack<T>>,
    /// The value to track over time.
    pub value: Handle<V>,
    /// Called when an undo or redo causes a state change.
    ///
    /// If the state would still be the same before and after the undo/redo, this
    /// will not be called. For example, receiving a redo when there is nothing
    /// to redo will not call this method.
    ///
    /// Changes to the [`value`](Self::value) while this method is running will not be recorded
    /// on the undo stack. For example, a `TextInputFormatter` may change the value
    /// from what was on the undo stack, but this new value will not be recorded,
    /// as that would wipe out the redo history.
    pub on_triggered: UndoHistoryOnTriggered<T>,
    /// The [`FocusNode`] that will be used to listen for focus to set the initial
    /// undo state for the element.
    pub focus_node: Handle<FocusNode>,
    /// Called right before a new entry is pushed to the undo stack.
    ///
    /// The value returned from this method will be pushed to the stack instead
    /// of the original value.
    ///
    /// If null then the original value will always be pushed to the stack.
    pub undo_stack_modifier: Option<UndoStackModifier<T>>,
    /// Controls the undo state.
    ///
    /// If null, this widget will create its own [`UndoHistoryController`].
    pub controller: Option<Handle<UndoHistoryController>>,
    /// The child widget of [`UndoHistory`].
    pub child: WidgetRef,
}

impl<T: Clone + PartialEq + 'static, V: UndoHistorySource<T>> UndoHistory<T, V> {
    /// Creates an instance of [`UndoHistory`].
    pub fn new<K>(
        value: Handle<V>,
        on_triggered: impl Fn(&mut App, T) + 'static,
        focus_node: Handle<FocusNode>,
        child: impl IntoWidget<K>,
    ) -> UndoHistory<T, V> {
        UndoHistory {
            key: None,
            should_change_undo_stack: None,
            value,
            on_triggered: Rc::new(on_triggered),
            focus_node,
            undo_stack_modifier: None,
            controller: None,
            child: child.into_widget(),
        }
    }

    /// Dart `UndoHistory(key:)`.
    pub fn key(mut self, key: KeyRef) -> UndoHistory<T, V> {
        self.key = Some(key);
        self
    }

    /// Dart `UndoHistory(shouldChangeUndoStack:)`.
    pub fn should_change_undo_stack(
        mut self,
        should_change_undo_stack: impl Fn(Option<&T>, &T) -> bool + 'static,
    ) -> UndoHistory<T, V> {
        self.should_change_undo_stack = Some(Rc::new(should_change_undo_stack));
        self
    }

    /// Dart `UndoHistory(undoStackModifier:)`.
    pub fn undo_stack_modifier(
        mut self,
        undo_stack_modifier: impl Fn(T) -> T + 'static,
    ) -> UndoHistory<T, V> {
        self.undo_stack_modifier = Some(Rc::new(undo_stack_modifier));
        self
    }

    /// Dart `UndoHistory(controller:)`.
    pub fn controller(mut self, controller: Handle<UndoHistoryController>) -> UndoHistory<T, V> {
        self.controller = Some(controller);
        self
    }
}

impl<T: Clone + PartialEq + 'static, V: UndoHistorySource<T>> Debug for UndoHistory<T, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UndoHistory")
            .field("child", &self.child)
            .finish_non_exhaustive()
    }
}

impl<T: Clone + PartialEq + 'static, V: UndoHistorySource<T>> StatefulWidget for UndoHistory<T, V> {
    type State = UndoHistoryState<T, V>;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> UndoHistoryState<T, V> {
        UndoHistoryState {
            state: StateData::new(),
            stack: UndoStack::new(),
            throttle_timer: None,
            throttle_arg: None,
            during_trigger: Rc::new(Cell::new(false)),
            last_value: None,
            controller: None,
        }
    }
}

/// State for a [`UndoHistory`].
///
/// Provides [`undo`](Self::undo), [`redo`](Self::redo), [`can_undo`](Self::can_undo), and
/// [`can_redo`](Self::can_redo) for programmatic access to the undo state for custom undo
/// and redo UI implementations.
pub struct UndoHistoryState<
    T: Clone + PartialEq + 'static,
    V: UndoHistorySource<T> = ValueNotifier<T>,
> {
    state: StateData<UndoHistory<T, V>>,
    stack: UndoStack<T>,
    throttle_timer: Option<Timer>,
    throttle_arg: Option<T>,
    during_trigger: Rc<Cell<bool>>,
    // Record the last value to prevent pushing multiple
    // of the same value in a row onto the undo stack. For example, push gets
    // called both in initState and when the EditableText receives focus.
    last_value: Option<T>,
    controller: Option<Handle<UndoHistoryController>>,
}

/// Dart's `try`/`finally` around `_duringTrigger`. Holds the flag by `Rc`, never
/// a borrow of the state, so a panic still clears it.
struct DuringTriggerGuard {
    flag: Rc<Cell<bool>>,
}

impl DuringTriggerGuard {
    fn enter(flag: &Rc<Cell<bool>>) -> DuringTriggerGuard {
        flag.set(true);
        DuringTriggerGuard {
            flag: Rc::clone(flag),
        }
    }
}

impl Drop for DuringTriggerGuard {
    fn drop(&mut self) {
        self.flag.set(false);
    }
}

impl<T: Clone + PartialEq + 'static, V: UndoHistorySource<T>> UndoHistoryState<T, V> {
    fn effective_controller(self: Handle<Self>, app: &mut App) -> Handle<UndoHistoryController> {
        if let Some(controller) = self.widget(app).controller {
            return controller;
        }
        if let Some(controller) = app.get(self).controller {
            return controller;
        }
        let controller = UndoHistoryController::new(app);
        app.get_mut(self).controller = Some(controller);
        controller
    }

    /// Reverts the value on the stack to the previous value.
    pub fn undo(self: Handle<Self>, app: &mut App) {
        if app.get(self).stack.current_value().is_none() {
            // Returns early if there is not a first value registered in the history.
            // This is important because, if an undo is received while the initial
            // value is being pushed (a.k.a when the field gets the focus but the
            // throttling delay is pending), the initial push should not be canceled.
            return;
        }
        if app
            .get(self)
            .throttle_timer
            .is_some_and(|timer| timer.is_active(app))
        {
            if let Some(timer) = app.get(self).throttle_timer {
                timer.cancel(app); // Cancel ongoing push, if any.
            }
            let next = app.get(self).stack.current_value().cloned();
            self.update(app, next);
        } else {
            let next = app.get_mut(self).stack.undo().cloned();
            self.update(app, next);
        }
        self.update_state(app);
    }

    /// Updates the value on the stack to the next value.
    pub fn redo(self: Handle<Self>, app: &mut App) {
        let next = app.get_mut(self).stack.redo().cloned();
        self.update(app, next);
        self.update_state(app);
    }

    /// Will be true if there are past values on the stack.
    pub fn can_undo(self: Handle<Self>, app: &App) -> bool {
        app.get(self).stack.can_undo()
    }

    /// Will be true if there are future values on the stack.
    pub fn can_redo(self: Handle<Self>, app: &App) -> bool {
        app.get(self).stack.can_redo()
    }

    fn update_state(self: Handle<Self>, app: &mut App) {
        let can_undo = self.can_undo(app);
        let can_redo = self.can_redo(app);
        let controller = self.effective_controller(app);
        controller.set_value(
            app,
            UndoHistoryValue::new()
                .can_undo(can_undo)
                .can_redo(can_redo),
        );

        if app.platform().target_platform() != TargetPlatform::IOS {
            return;
        }

        if UndoManager::client(app) == Some(self.as_undo_manager_client()) {
            UndoManager::set_undo_state(app, can_undo, can_redo);
        }
    }

    fn undo_from_intent(self: Handle<Self>, app: &mut App) {
        self.undo(app);
    }

    fn redo_from_intent(self: Handle<Self>, app: &mut App) {
        self.redo(app);
    }

    fn update(self: Handle<Self>, app: &mut App, next_value: Option<T>) {
        let Some(next_value) = next_value else {
            return;
        };
        if app.get(self).last_value.as_ref() == Some(&next_value) {
            return;
        }
        app.get_mut(self).last_value = Some(next_value.clone());
        let flag = Rc::clone(&app.get(self).during_trigger);
        let _guard = DuringTriggerGuard::enter(&flag);
        let on_triggered = Rc::clone(&self.widget(app).on_triggered);
        on_triggered(app, next_value.clone());
        debug_assert!(self.widget(app).value.history_value(app) == &next_value);
    }

    fn push(self: Handle<Self>, app: &mut App) {
        let value = self.widget(app).value;
        let current = value.history_value(app).clone();
        if app.get(self).last_value.as_ref() == Some(&current) {
            return;
        }

        if app.get(self).during_trigger.get() {
            return;
        }

        if let Some(should_change) = self.widget(app).should_change_undo_stack.clone()
            && !should_change(app.get(self).last_value.as_ref(), &current)
        {
            return;
        }

        let next_value = if let Some(modifier) = self.widget(app).undo_stack_modifier.clone() {
            modifier(current)
        } else {
            current
        };
        if app.get(self).last_value.as_ref() == Some(&next_value) {
            return;
        }

        app.get_mut(self).last_value = Some(next_value.clone());

        let timer = self.throttled_push(app, next_value);
        app.get_mut(self).throttle_timer = Some(timer);
    }

    fn handle_focus(self: Handle<Self>, app: &mut App) {
        let focus_node = self.widget(app).focus_node;
        if !focus_node.has_focus(app) {
            if UndoManager::client(app) == Some(self.as_undo_manager_client()) {
                UndoManager::set_client(app, None);
            }

            return;
        }
        UndoManager::set_client(app, Some(self.as_undo_manager_client()));
        self.update_state(app);
    }

    /// Dart's `_throttledPush`: one pending call, latest arg wins.
    fn throttled_push(self: Handle<Self>, app: &mut App, current_arg: T) -> Timer {
        app.get_mut(self).throttle_arg = Some(current_arg);
        if let Some(timer) = app.get(self).throttle_timer
            && timer.is_active(app)
        {
            return timer;
        }
        Timer::new(
            app,
            K_THROTTLE_DURATION,
            Listener::handle_method(self, Self::throttled_push_fire),
        )
    }

    fn throttled_push_fire(self: Handle<Self>, app: &mut App) {
        let arg = app
            .get_mut(self)
            .throttle_arg
            .take()
            .expect("throttled push stores the arg");
        app.get_mut(self).throttle_timer = None;
        app.get_mut(self).stack.push(arg);
        self.update_state(app);
    }
}

impl<T: Clone + PartialEq + 'static, V: UndoHistorySource<T>> UndoManagerClient
    for UndoHistoryState<T, V>
{
    fn handle_platform_undo(self: Handle<Self>, app: &mut App, direction: UndoDirection) {
        match direction {
            UndoDirection::Undo => self.undo(app),
            UndoDirection::Redo => self.redo(app),
        }
    }

    fn undo(self: Handle<Self>, app: &mut App) {
        UndoHistoryState::undo(self, app);
    }

    fn redo(self: Handle<Self>, app: &mut App) {
        UndoHistoryState::redo(self, app);
    }

    fn can_undo(self: Handle<Self>, app: &App) -> bool {
        UndoHistoryState::can_undo(self, app)
    }

    fn can_redo(self: Handle<Self>, app: &App) -> bool {
        UndoHistoryState::can_redo(self, app)
    }
}

impl<T: Clone + PartialEq + 'static, V: UndoHistorySource<T>> State for UndoHistoryState<T, V> {
    type Widget = UndoHistory<T, V>;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        self.push(app);
        let value = self.widget(app).value;
        value.add_listener(app, Listener::handle_method(self, Self::push));
        self.handle_focus(app);
        let focus_node = self.widget(app).focus_node;
        focus_node.add_listener(app, Listener::handle_method(self, Self::handle_focus));
        let controller = self.effective_controller(app);
        controller
            .on_undo(app)
            .add_listener(app, Listener::handle_method(self, Self::undo));
        let controller = self.effective_controller(app);
        controller
            .on_redo(app)
            .add_listener(app, Listener::handle_method(self, Self::redo));
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &UndoHistory<T, V>) {
        let value = self.widget(app).value;
        if value != old_widget.value {
            app.get_mut(self).stack.clear();
            old_widget
                .value
                .remove_listener(app, &Listener::handle_method(self, Self::push));
            value.add_listener(app, Listener::handle_method(self, Self::push));
        }
        let focus_node = self.widget(app).focus_node;
        if focus_node != old_widget.focus_node {
            old_widget
                .focus_node
                .remove_listener(app, &Listener::handle_method(self, Self::handle_focus));
            focus_node.add_listener(app, Listener::handle_method(self, Self::handle_focus));
        }
        if self.widget(app).controller != old_widget.controller {
            let controller = self.effective_controller(app);
            controller
                .on_undo(app)
                .remove_listener(app, &Listener::handle_method(self, Self::undo));
            let controller = self.effective_controller(app);
            controller
                .on_redo(app)
                .remove_listener(app, &Listener::handle_method(self, Self::redo));
            if let Some(controller) = app.get(self).controller {
                controller.dispose(app);
            }
            app.get_mut(self).controller = None;
            let controller = self.effective_controller(app);
            controller
                .on_undo(app)
                .add_listener(app, Listener::handle_method(self, Self::undo));
            let controller = self.effective_controller(app);
            controller
                .on_redo(app)
                .add_listener(app, Listener::handle_method(self, Self::redo));
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if UndoManager::client(app) == Some(self.as_undo_manager_client()) {
            UndoManager::set_client(app, None);
        }

        let value = self.widget(app).value;
        value.remove_listener(app, &Listener::handle_method(self, Self::push));
        let focus_node = self.widget(app).focus_node;
        focus_node.remove_listener(app, &Listener::handle_method(self, Self::handle_focus));
        let controller = self.effective_controller(app);
        controller
            .on_undo(app)
            .remove_listener(app, &Listener::handle_method(self, Self::undo));
        let controller = self.effective_controller(app);
        controller
            .on_redo(app)
            .remove_listener(app, &Listener::handle_method(self, Self::redo));
        if let Some(controller) = app.get(self).controller {
            controller.dispose(app);
        }
        if let Some(timer) = app.get(self).throttle_timer {
            timer.cancel(app);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let child = self.widget(app).child.clone();
        let undo_default = CallbackAction::<UndoTextIntent>::new(
            app,
            Rc::new({
                let this = self;
                move |app, _intent| {
                    this.undo_from_intent(app);
                    None
                }
            }),
        );
        let redo_default = CallbackAction::<RedoTextIntent>::new(
            app,
            Rc::new({
                let this = self;
                move |app, _intent| {
                    this.redo_from_intent(app);
                    None
                }
            }),
        );
        let undo = AnyAction::overridable(app, Action::as_action(undo_default), context);
        let redo = AnyAction::overridable(app, Action::as_action(redo_default), context);
        Actions::new(
            HashMap::from([
                (TypeId::of::<UndoTextIntent>(), undo),
                (TypeId::of::<RedoTextIntent>(), redo),
            ]),
            child,
        )
        .into_widget()
    }
}

/// Represents whether the current undo stack can undo or redo.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct UndoHistoryValue {
    /// Whether the current undo stack can perform an undo operation.
    pub can_undo: bool,
    /// Whether the current undo stack can perform a redo operation.
    pub can_redo: bool,
}

impl UndoHistoryValue {
    /// A value corresponding to an undo stack that can neither undo nor redo.
    pub const EMPTY: UndoHistoryValue = UndoHistoryValue {
        can_undo: false,
        can_redo: false,
    };

    /// Creates a value for whether the current undo stack can undo or redo.
    ///
    /// The [`can_undo`](Self::can_undo) and [`can_redo`](Self::can_redo) arguments must have a
    /// value, but default to false.
    pub const fn new() -> UndoHistoryValue {
        UndoHistoryValue::EMPTY
    }

    /// Dart `UndoHistoryValue(canUndo:)`.
    pub const fn can_undo(mut self, can_undo: bool) -> UndoHistoryValue {
        self.can_undo = can_undo;
        self
    }

    /// Dart `UndoHistoryValue(canRedo:)`.
    pub const fn can_redo(mut self, can_redo: bool) -> UndoHistoryValue {
        self.can_redo = can_redo;
        self
    }
}

impl Default for UndoHistoryValue {
    fn default() -> UndoHistoryValue {
        UndoHistoryValue::EMPTY
    }
}

/// A controller for the undo history, for example for an editable text field.
///
/// Whenever a change happens to the underlying value that the `UndoHistory` widget tracks,
/// that widget updates the [`value`](Self::value) and the controller notifies its listeners.
/// Listeners can then read the [`can_undo`](UndoHistoryValue::can_undo) and
/// [`can_redo`](UndoHistoryValue::can_redo) properties of the value to discover whether
/// [`undo`](Self::undo) or [`redo`](Self::redo) are possible.
///
/// The controller also has [`undo`](Self::undo) and [`redo`](Self::redo) methods to modify
/// the undo history.
///
/// See also:
///
///  * `EditableText`, which uses the `UndoHistory` widget and allows control of the
///    underlying history using an [`UndoHistoryController`].
pub struct UndoHistoryController {
    change_notifier: ChangeNotifierData,
    value: UndoHistoryValue,
    on_undo: Handle<ChangeNotifierData>,
    on_redo: Handle<ChangeNotifierData>,
}

impl Debug for UndoHistoryController {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UndoHistoryController")
            .field("value", &self.value)
            .finish()
    }
}

impl UndoHistoryController {
    /// Creates a controller for an `UndoHistory` widget.
    pub fn new(app: &mut App) -> Handle<UndoHistoryController> {
        Self::from_value(app, UndoHistoryValue::EMPTY)
    }

    /// Dart `UndoHistoryController(value:)`.
    pub fn from_value(app: &mut App, value: UndoHistoryValue) -> Handle<UndoHistoryController> {
        let on_undo = app.create(ChangeNotifierData::new());
        let on_redo = app.create(ChangeNotifierData::new());
        app.create(UndoHistoryController {
            change_notifier: ChangeNotifierData::new(),
            value,
            on_undo,
            on_redo,
        })
    }

    /// The current undo/redo capability.
    pub fn value(self: Handle<Self>, app: &App) -> &UndoHistoryValue {
        &app.get(self).value
    }

    /// Setting this notifies listeners when the value differs.
    pub fn set_value(self: Handle<Self>, app: &mut App, new_value: UndoHistoryValue) {
        if app.get(self).value == new_value {
            return;
        }
        app.get_mut(self).value = new_value;
        self.notify_listeners(app);
    }

    /// Notifies listeners that [`undo`](Self::undo) has been called.
    pub fn on_undo(self: Handle<Self>, app: &App) -> Handle<ChangeNotifierData> {
        app.get(self).on_undo
    }

    /// Notifies listeners that [`redo`](Self::redo) has been called.
    pub fn on_redo(self: Handle<Self>, app: &App) -> Handle<ChangeNotifierData> {
        app.get(self).on_redo
    }

    /// Reverts the value on the stack to the previous value.
    pub fn undo(self: Handle<Self>, app: &mut App) {
        if !self.value(app).can_undo {
            return;
        }
        let on_undo = self.on_undo(app);
        on_undo.notify_listeners(app);
    }

    /// Updates the value on the stack to the next value.
    pub fn redo(self: Handle<Self>, app: &mut App) {
        if !self.value(app).can_redo {
            return;
        }
        let on_redo = self.on_redo(app);
        on_redo.notify_listeners(app);
    }

    /// Discards any resources used by the object.
    pub fn dispose(self: Handle<Self>, app: &mut App) {
        let on_undo = self.on_undo(app);
        let on_redo = self.on_redo(app);
        app.get_mut(on_undo).dispose();
        app.get_mut(on_redo).dispose();
        app.get_mut(self).change_notifier.dispose();
    }
}

impl ChangeNotifier for UndoHistoryController {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

/// A data structure representing a chronological list of states that can be undone and redone.
///
/// Dart's `_UndoStack`.
struct UndoStack<T> {
    list: Vec<T>,
    // The index of the current value, or -1 if the list is empty.
    index: i32,
}

// This duration was chosen as a best fit for the behavior of Mac, Linux,
// and Windows undo/redo state save durations, but it is not perfect for any
// of them.
const K_THROTTLE_DURATION: Duration = Duration::from_millis(500);

impl<T: PartialEq> UndoStack<T> {
    /// Creates an instance of [`UndoStack`].
    fn new() -> UndoStack<T> {
        UndoStack {
            list: Vec::new(),
            index: -1,
        }
    }

    /// Returns the current value of the stack.
    fn current_value(&self) -> Option<&T> {
        if self.list.is_empty() {
            None
        } else {
            Some(&self.list[self.index as usize])
        }
    }

    fn can_undo(&self) -> bool {
        !self.list.is_empty() && self.index > 0
    }

    fn can_redo(&self) -> bool {
        !self.list.is_empty() && (self.index as usize) < self.list.len() - 1
    }

    /// Add a new state change to the stack.
    ///
    /// Pushing identical objects will not create multiple entries.
    fn push(&mut self, value: T) {
        if self.list.is_empty() {
            self.index = 0;
            self.list.push(value);
            return;
        }

        debug_assert!((self.index as usize) < self.list.len() && self.index >= 0);

        if self.current_value() == Some(&value) {
            return;
        }

        // If anything has been undone in this stack, remove those irrelevant states
        // before adding the new one.
        if (self.index as usize) != self.list.len() - 1 {
            self.list.truncate((self.index as usize) + 1);
        }
        self.list.push(value);
        self.index = self.list.len() as i32 - 1;
    }

    /// Returns the current value after an undo operation.
    ///
    /// An undo operation moves the current value to the previously pushed value, if any.
    ///
    /// Iff the stack is completely empty, then returns null.
    fn undo(&mut self) -> Option<&T> {
        if self.list.is_empty() {
            return None;
        }

        debug_assert!((self.index as usize) < self.list.len() && self.index >= 0);

        if self.index != 0 {
            self.index -= 1;
        }

        self.current_value()
    }

    /// Returns the current value after a redo operation.
    ///
    /// A redo operation moves the current value to the value that was last undone, if any.
    ///
    /// Iff the stack is completely empty, then returns null.
    fn redo(&mut self) -> Option<&T> {
        if self.list.is_empty() {
            return None;
        }

        debug_assert!((self.index as usize) < self.list.len() && self.index >= 0);

        if (self.index as usize) < self.list.len() - 1 {
            self.index += 1;
        }

        self.current_value()
    }

    /// Remove everything from the stack.
    fn clear(&mut self) {
        self.list.clear();
        self.index = -1;
    }
}

impl<T: Debug> Debug for UndoStack<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "_UndoStack {:?}", self.list)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use inset_foundation::{AppCell, Listenable, Listener, ValueListenable};
    use std::cell::Cell;
    use std::rc::Rc;
    use std::time::Duration;

    use crate::framework::GlobalKey;
    use crate::test_harness::Harness;
    use crate::widgets::basic::SizedBox;

    #[test]
    fn controller_starts_empty() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let controller = UndoHistoryController::new(&mut app);
        assert_eq!(controller.value(&app), &UndoHistoryValue::EMPTY);
    }

    #[test]
    fn undo_notifies_on_undo_when_can_undo() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let controller =
            UndoHistoryController::from_value(&mut app, UndoHistoryValue::new().can_undo(true));
        let fired = Rc::new(Cell::new(false));
        let seen = Rc::clone(&fired);
        controller
            .on_undo(&app)
            .add_listener(&mut app, Listener::new(move |_app| seen.set(true)));
        controller.undo(&mut app);
        assert!(fired.get());
    }

    #[test]
    fn undo_is_a_no_op_when_empty() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let controller = UndoHistoryController::new(&mut app);
        let fired = Rc::new(Cell::new(false));
        let seen = Rc::clone(&fired);
        controller
            .on_undo(&app)
            .add_listener(&mut app, Listener::new(move |_app| seen.set(true)));
        controller.undo(&mut app);
        assert!(!fired.get());
    }

    #[test]
    fn stack_push_undo_redo() {
        let mut stack = UndoStack::new();
        stack.push("a");
        stack.push("b");
        stack.push("c");
        assert_eq!(stack.current_value(), Some(&"c"));
        assert!(stack.can_undo());
        assert!(!stack.can_redo());
        assert_eq!(stack.undo(), Some(&"b"));
        assert!(stack.can_redo());
        assert_eq!(stack.redo(), Some(&"c"));
        stack.undo();
        stack.push("d");
        assert_eq!(stack.current_value(), Some(&"d"));
        assert!(!stack.can_redo());
        assert_eq!(stack.undo(), Some(&"b"));
    }

    #[test]
    fn push_two_values_then_undo_restores() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let value = app.create(ValueNotifier::new(0i32));
        let focus_node = FocusNode::new(&mut app);
        let key = GlobalKey::new();
        let widget = UndoHistory::new(
            value,
            move |app, next| {
                value.set_value(app, next);
            },
            focus_node,
            SizedBox::shrink(),
        )
        .key(Rc::new(key.clone()));
        let harness = Harness::mount(&mut app, widget.into_widget());
        harness.pump(&mut app);
        drop(app);
        cell.elapse(Duration::from_millis(500));

        let mut app = cell.borrow_mut();
        value.set_value(&mut app, 1);
        drop(app);
        cell.elapse(Duration::from_millis(500));

        let mut app = cell.borrow_mut();
        let state = key
            .current_state::<UndoHistoryState<i32>>(&mut app)
            .expect("the history is mounted");
        assert!(state.can_undo(&app));
        assert!(!state.can_redo(&app));
        state.undo(&mut app);
        assert_eq!(*value.value(&app), 0);
        assert!(!state.can_undo(&app));
        assert!(state.can_redo(&app));
        state.redo(&mut app);
        assert_eq!(*value.value(&app), 1);
    }
}
