//! Flutter counterpart: `widgets/actions.dart`.
//!
//! The action system: [`Intent`] describes what should happen, [`Action`] performs it, the
//! [`Actions`] widget maps one to the other for its descendants, and [`ActionDispatcher`]
//! invokes the action that was found. [`FocusableActionDetector`] combines [`Actions`],
//! `Shortcuts`, `Focus` and `MouseRegion` into the detector a control is built from.

use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet};
use std::fmt::{self, Debug};
use std::rc::Rc;

use inset_foundation::{App, Handle, HandleId, Listener, ValueChanged};
use inset_scheduler::{FrameCallback, SchedulerBinding, SchedulerPhase};
use inset_services::{MouseCursor, MouseCursorRef};

use crate::framework::{
    BuildContext, GlobalKey, InheritedWidget, IntoWidget, KeyRef, State, StateData, StatefulWidget,
    WidgetRef,
};
use crate::widgets::basic::MouseRegion;
use crate::widgets::focus_manager::{
    AnyFocusNode, FocusHighlightMode, FocusManager, HighlightModeListener, KeyEventResult,
    primary_focus,
};
use crate::widgets::focus_scope::Focus;
use crate::widgets::media_query::{MediaQuery, NavigationMode};
use crate::widgets::shortcuts::{ShortcutMap, Shortcuts};

/// Returns the parent [`BuildContext`] of a given `context`.
///
/// [`BuildContext`] doesn't have a `parent` accessor, but the parent can be obtained using
/// [`BuildContext::visit_ancestor_elements`](crate::AnyElement::visit_ancestor_elements).
///
/// [`BuildContext::get_element_for_inherited_widget_of_exact_type`](crate::AnyElement::get_element_for_inherited_widget_of_exact_type)
/// returns the same [`BuildContext`] if it happens to be of the correct type. To obtain the
/// previous inherited widget, the search must therefore start from the parent; this is what
/// `get_parent` is used for.
///
/// `get_parent` is O(1), because it always stops at the first ancestor.
fn get_parent(app: &App, context: BuildContext) -> Option<BuildContext> {
    let mut parent = None;
    context.visit_ancestor_elements(app, &mut |ancestor| {
        parent = Some(ancestor);
        false
    });
    parent
}

// ---------------------------------------------------------------------------------------------
// Intent

/// A particular configuration of an [`Action`].
///
/// This trait is what the `Shortcuts::shortcuts` map has as values, and is used by an
/// [`ActionDispatcher`] to look up an action and invoke it, giving it this object to extract
/// configuration information from.
///
/// See also:
///
///  * `Shortcuts`, a widget used to bind key combinations to [`Intent`]s.
///  * [`Actions`], a widget used to map [`Intent`]s to [`Action`]s.
///  * [`Actions::invoke`], which invokes the action associated with a specified [`Intent`]
///    using the [`Actions`] widget that most tightly encloses the given [`BuildContext`].
pub trait Intent: Debug + 'static {
    /// The value behind the erased intent, for the downcast an [`Action`] does to recover its
    /// own intent type.
    fn as_any(&self) -> &dyn Any;

    /// Dart's `intent.runtimeType`: the key an [`Actions`] map is looked up with.
    fn intent_type(&self) -> TypeId {
        self.as_any().type_id()
    }
}

/// The erased [`Intent`] value: what a field or parameter Dart types as `Intent` becomes.
pub type IntentRef = Rc<dyn Intent>;

/// An intent that is mapped to a [`DoNothingAction`], which, as the name implies, does
/// nothing.
///
/// This intent is mapped to an action in the `WidgetsApp` that does nothing, so that it can be
/// bound to a key in a `Shortcuts` widget in order to disable a key binding made above it in
/// the hierarchy.
///
/// Dart's `Intent.doNothing`.
pub const DO_NOTHING: DoNothingIntent = DoNothingIntent::new();

/// The type argument of an [`Action`]: Dart's `T extends Intent` in `Action<T>`.
///
/// Every [`Intent`] implements it, and so does `dyn Intent`, which is Dart's `Action<Intent>` —
/// an action that handles every intent.
pub trait ActionIntent: 'static {
    /// Dart's `intent is T`.
    fn cast_intent(intent: &dyn Intent) -> Option<&Self>;

    /// This value back as the erased [`Intent`], for the lookups keyed by intent type.
    fn as_intent(&self) -> &dyn Intent;
}

impl<T: Intent> ActionIntent for T {
    fn cast_intent(intent: &dyn Intent) -> Option<&T> {
        intent.as_any().downcast_ref::<T>()
    }

    fn as_intent(&self) -> &dyn Intent {
        self
    }
}

impl ActionIntent for dyn Intent {
    fn cast_intent(intent: &dyn Intent) -> Option<&dyn Intent> {
        Some(intent)
    }

    fn as_intent(&self) -> &dyn Intent {
        self
    }
}

// ---------------------------------------------------------------------------------------------
// Action

/// The kind of callback that an [`Action`] uses to notify of changes to the action's state.
///
/// To register an action listener, call [`Action::add_action_listener`].
pub type ActionListenerCallback = Rc<dyn Fn(&mut App, AnyAction)>;

/// The fields Dart's `Action` declares; every action carries this bag under the field
/// `action`.
pub struct ActionData {
    listeners: Vec<ActionListenerCallback>,
    current_calling_action: Option<AnyAction>,
}

impl ActionData {
    /// The bag of a freshly created action.
    pub fn new() -> ActionData {
        ActionData {
            listeners: Vec::new(),
            current_calling_action: None,
        }
    }
}

impl Default for ActionData {
    fn default() -> ActionData {
        ActionData::new()
    }
}

/// The accessors [`Action`] asks for, for a struct whose bag is the field `action`.
#[macro_export]
macro_rules! action_accessors {
    () => {
        fn action_data(
            self: ::inset_foundation::Handle<Self>,
            app: &::inset_foundation::App,
        ) -> &$crate::ActionData {
            &app.get(self).action
        }

        fn action_data_mut(
            self: ::inset_foundation::Handle<Self>,
            app: &mut ::inset_foundation::App,
        ) -> &mut $crate::ActionData {
            &mut app.get_mut(self).action
        }
    };
}

/// The overrides a [`ContextAction`] leaf adds to its `impl Action`: the type-erased handle
/// that carries the context through, the two context-less entry points Dart gets from an optional
/// parameter, and the overridable form.
#[macro_export]
macro_rules! context_action_overrides {
    () => {
        fn as_action(self: ::inset_foundation::Handle<Self>) -> $crate::AnyAction {
            $crate::AnyAction::of_context::<Self>(self)
        }

        fn is_enabled(
            self: ::inset_foundation::Handle<Self>,
            app: &mut ::inset_foundation::App,
            intent: &Self::Intent,
        ) -> bool {
            $crate::ContextAction::is_enabled(self, app, intent, None)
        }

        fn invoke(
            self: ::inset_foundation::Handle<Self>,
            app: &mut ::inset_foundation::App,
            intent: &Self::Intent,
        ) -> Option<::std::rc::Rc<dyn ::std::any::Any>> {
            $crate::ContextAction::invoke(self, app, intent, None)
        }

        fn make_overridable_action(
            self: ::inset_foundation::Handle<Self>,
            app: &mut ::inset_foundation::App,
            context: $crate::BuildContext,
        ) -> $crate::AnyAction {
            let default_action = $crate::Action::as_action(self);
            let overridable =
                $crate::OverridableContextAction::<Self::Intent>::new(app, default_action, context);
            $crate::Action::as_action(overridable)
        }
    };
}

/// Base trait for an action or command to be performed.
///
/// [`Action`]s are typically invoked as a result of a user action. For example, the `Shortcuts`
/// widget will map a keyboard shortcut into an [`Intent`], which is given to an
/// [`ActionDispatcher`] to map the [`Intent`] to an [`Action`] and invoke it.
///
/// The [`ActionDispatcher`] can invoke an [`Action`] on the primary focus, or without regard
/// for focus.
///
/// ### Action Overriding
///
/// When using a leaf widget to build a more specialized widget, it's sometimes desirable to
/// change the default handling of an [`Intent`] defined in the leaf widget. For instance, a
/// text field's select-all intent by default selects the text it currently contains, but in a
/// US phone number widget that consists of 3 different text fields (area code, prefix and line
/// number), it should instead select the text within all 3 text fields.
///
/// An overridable [`Action`] is a special kind of [`Action`] created using
/// [`AnyAction::overridable`]. It has access to a default [`Action`], and an optional override
/// [`Action`]. It has the same behavior as its override if that exists, and mirrors the
/// behavior of its `default_action` otherwise.
///
/// See also:
///
///  * `Shortcuts`, which is a widget that contains a key map, in which it looks up key
///    combinations in order to invoke actions.
///  * [`Actions`], which is a widget that defines a map of [`Intent`] to [`Action`] and allows
///    redefining of actions for its descendants.
///  * [`ActionDispatcher`], a class that takes an [`Action`] and invokes it, passing a given
///    [`Intent`].
pub trait Action: Sized + 'static {
    /// Dart's type argument `T extends Intent`.
    ///
    /// `dyn Intent` is Dart's `Action<Intent>`: an action that handles every intent.
    type Intent: ActionIntent + ?Sized;

    /// Dart's `Action` fields, held under the field `action`
    /// ([`action_accessors!`](crate::action_accessors)).
    fn action_data(self: Handle<Self>, app: &App) -> &ActionData;

    /// See [`action_data`](Self::action_data).
    fn action_data_mut(self: Handle<Self>, app: &mut App) -> &mut ActionData;

    /// This action as the erased [`AnyAction`] — what to pass where a Dart API takes an
    /// `Action<Intent>`.
    ///
    /// A [`ContextAction`] overrides this with
    /// [`context_action_overrides!`](crate::context_action_overrides), so that the type-erased
    /// handle carries the invoking context through.
    fn as_action(self: Handle<Self>) -> AnyAction {
        AnyAction::of::<Self>(self)
    }

    /// Gets the type of intent this action responds to.
    fn intent_type() -> TypeId {
        TypeId::of::<Self::Intent>()
    }

    /// The [`Action`] overridden by this [`Action`].
    ///
    /// [`AnyAction::overridable`] creates an overridable [`Action`] that allows itself to be
    /// overridden by the closest ancestor [`Action`], and falls back to its own
    /// `default_action` when no overrides can be found. When an override is present, an
    /// overridable [`Action`] forwards all incoming method calls to the override, and allows
    /// the override to access the `default_action` via its
    /// [`calling_action`](Self::calling_action) property.
    ///
    /// Before forwarding the call to the override, the overridable [`Action`] is responsible
    /// for setting [`calling_action`](Self::calling_action) to its `default_action`, which is
    /// already taken care of by the overridable [`Action`] created using
    /// [`AnyAction::overridable`].
    ///
    /// This property is only set when this [`Action`] is an override of the calling action,
    /// and is currently being invoked from it.
    ///
    /// Invoking the calling action's methods, or accessing its properties, is allowed and does
    /// not introduce infinite loops or infinite recursions.
    fn calling_action(self: Handle<Self>, app: &App) -> Option<AnyAction> {
        self.action_data(app).current_calling_action
    }

    /// Sets [`calling_action`](Self::calling_action); overridden by the overridable actions so
    /// that the default action sees it too.
    fn update_calling_action(self: Handle<Self>, app: &mut App, value: Option<AnyAction>) {
        self.action_data_mut(app).current_calling_action = value;
    }

    /// Returns true if the action is enabled and is ready to be invoked.
    ///
    /// This will be called by the [`ActionDispatcher`] before attempting to invoke the action.
    ///
    /// If the action's enable state depends on a [`BuildContext`], implement [`ContextAction`]
    /// instead.
    fn is_enabled(self: Handle<Self>, app: &mut App, intent: &Self::Intent) -> bool {
        let _ = intent;
        self.is_action_enabled(app)
    }

    /// Whether this [`Action`] is inherently enabled.
    ///
    /// If [`is_action_enabled`](Self::is_action_enabled) is false, then this [`Action`] is
    /// disabled for any given [`Intent`].
    ///
    /// If the enabled state changes, overriding implementations must call
    /// [`notify_action_listeners`](Self::notify_action_listeners) to notify any listeners of
    /// the change.
    ///
    /// In the case of an overridable action, accessing this property creates a dependency on
    /// the overridable action's lookup context.
    fn is_action_enabled(self: Handle<Self>, app: &mut App) -> bool {
        let _ = app;
        true
    }

    /// Indicates whether this action should treat key events mapped to this action as being
    /// "handled" when it is invoked via the key event.
    ///
    /// If the key is handled, then no other key event handlers in the focus chain will receive
    /// the event.
    ///
    /// If the key event is not handled, it will be passed back to the engine, and continue to
    /// be processed there, allowing text fields and non-Flutter widgets to receive the key
    /// event.
    ///
    /// The default implementation returns true.
    fn consumes_key(self: Handle<Self>, app: &mut App, intent: &Self::Intent) -> bool {
        let _ = (app, intent);
        true
    }

    /// Converts the result of [`invoke`](Self::invoke) of this action to a [`KeyEventResult`].
    ///
    /// This is typically used when the action is invoked in response to a keyboard shortcut.
    ///
    /// The `invoke_result` argument is the value returned by [`invoke`](Self::invoke).
    ///
    /// By default, calls [`consumes_key`](Self::consumes_key) and converts the returned boolean
    /// to [`KeyEventResult::Handled`] if it's true, and
    /// [`KeyEventResult::SkipRemainingHandlers`] if it's false.
    fn to_key_event_result(
        self: Handle<Self>,
        app: &mut App,
        intent: &Self::Intent,
        invoke_result: Option<&Rc<dyn Any>>,
    ) -> KeyEventResult {
        let _ = invoke_result;
        if self.consumes_key(app, intent) {
            KeyEventResult::Handled
        } else {
            KeyEventResult::SkipRemainingHandlers
        }
    }

    /// Called when the action is to be performed.
    ///
    /// This is called by the [`ActionDispatcher`] when an action is invoked via
    /// [`Actions::invoke`], or when an action is invoked using
    /// [`ActionDispatcher::invoke_action`] directly.
    ///
    /// This method is only meant to be invoked by an [`ActionDispatcher`], and only when
    /// [`is_enabled`](Self::is_enabled) is true.
    ///
    /// To receive the result of invoking an action, it must be invoked using
    /// [`Actions::invoke`], or by invoking it using an [`ActionDispatcher`]. An action invoked
    /// via a `Shortcuts` widget will have its return value ignored.
    ///
    /// If the action's behavior depends on a [`BuildContext`], implement [`ContextAction`]
    /// instead.
    fn invoke(self: Handle<Self>, app: &mut App, intent: &Self::Intent) -> Option<Rc<dyn Any>>;

    /// Register a callback to listen for changes to the state of this action.
    ///
    /// If you call this, you must call
    /// [`remove_action_listener`](Self::remove_action_listener) a matching number of times, or
    /// memory leaks will occur. To help manage this and avoid memory leaks, use of the
    /// [`ActionListener`] widget to register and unregister your listener appropriately is
    /// highly recommended.
    fn add_action_listener(self: Handle<Self>, app: &mut App, listener: ActionListenerCallback) {
        self.action_data_mut(app).listeners.push(listener);
    }

    /// Remove a previously registered closure from the list of closures that are notified when
    /// the object changes.
    ///
    /// If the given listener is not registered, the call is ignored.
    fn remove_action_listener(
        self: Handle<Self>,
        app: &mut App,
        listener: &ActionListenerCallback,
    ) {
        let listeners = &mut self.action_data_mut(app).listeners;
        if let Some(index) = listeners
            .iter()
            .position(|registered| Rc::ptr_eq(registered, listener))
        {
            listeners.remove(index);
        }
    }

    /// Call all the registered listeners.
    ///
    /// Implementations should call this method whenever the object changes, to notify any
    /// clients the object may have changed. Listeners that are added during this iteration
    /// will not be visited. Listeners that are removed during this iteration will not be
    /// visited after they are removed.
    fn notify_action_listeners(self: Handle<Self>, app: &mut App) {
        if self.action_data(app).listeners.is_empty() {
            return;
        }
        // Make a local copy so that a listener can unregister while the list is being iterated
        // over.
        let local_listeners = self.action_data(app).listeners.clone();
        let this = self.as_action();
        for listener in local_listeners {
            let still_registered = self
                .action_data(app)
                .listeners
                .iter()
                .any(|registered| Rc::ptr_eq(registered, &listener));
            if still_registered {
                listener(app, this);
            }
        }
    }

    /// Dart's `Action._makeOverridableAction`: the overridable form of this action, which
    /// [`AnyAction::overridable`] hands back.
    fn make_overridable_action(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
    ) -> AnyAction {
        let default_action = self.as_action();
        let overridable = OverridableAction::<Self::Intent>::new(app, default_action, context);
        Action::as_action(overridable)
    }
}

/// An [`Action`] that adds an optional [`BuildContext`] to the
/// [`is_enabled`](Self::is_enabled) and [`invoke`](Self::invoke) methods to be able to provide
/// context to actions.
///
/// [`ActionDispatcher::invoke_action`] checks to see if the action it is invoking is a
/// [`ContextAction`], and if it is, supplies it with a context; the type-erased handle a
/// [`ContextAction`] hands out ([`context_action_overrides!`](crate::context_action_overrides))
/// is what answers that check.
pub trait ContextAction: Action {
    /// Returns true if the action is enabled and is ready to be invoked.
    ///
    /// This will be called by the [`ActionDispatcher`] before attempting to invoke the action.
    ///
    /// The optional `context` parameter is the context of the invocation of the action, and in
    /// the case of an action invoked by a `ShortcutManager`, via a `Shortcuts` widget, will be
    /// the context of the `Shortcuts` widget.
    fn is_enabled(
        self: Handle<Self>,
        app: &mut App,
        intent: &Self::Intent,
        context: Option<BuildContext>,
    ) -> bool {
        let _ = (intent, context);
        Action::is_action_enabled(self, app)
    }

    /// Called when the action is to be performed.
    ///
    /// This is called by the [`ActionDispatcher`] when an action is invoked via
    /// [`Actions::invoke`], or when an action is invoked using
    /// [`ActionDispatcher::invoke_action`] directly.
    ///
    /// This method is only meant to be invoked by an [`ActionDispatcher`], and only when
    /// [`is_enabled`](Self::is_enabled) is true.
    ///
    /// The optional `context` parameter is the context of the invocation of the action, and in
    /// the case of an action invoked by a `ShortcutManager`, via a `Shortcuts` widget, will be
    /// the context of the `Shortcuts` widget.
    fn invoke(
        self: Handle<Self>,
        app: &mut App,
        intent: &Self::Intent,
        context: Option<BuildContext>,
    ) -> Option<Rc<dyn Any>>;
}

/// The vtable's `Action.toKeyEventResult`, over the erased invoke result.
type ToKeyEventResultFn =
    fn(&mut App, HandleId, &dyn Intent, Option<&Rc<dyn Any>>) -> KeyEventResult;

/// The vtable's `Action._invoke`.
type InvokeFn = fn(&mut App, HandleId, &dyn Intent, Option<BuildContext>) -> Option<Rc<dyn Any>>;

/// The vtable of an erased [`AnyAction`]: one `&'static` table per leaf type, built by
/// [`ActionVTable::of`] or [`ActionVTable::of_context`].
struct ActionVTable {
    type_name: fn() -> &'static str,
    intent_type: fn() -> TypeId,
    calling_action: fn(&App, HandleId) -> Option<AnyAction>,
    update_calling_action: fn(&mut App, HandleId, Option<AnyAction>),
    is_enabled: fn(&mut App, HandleId, &dyn Intent, Option<BuildContext>) -> bool,
    is_action_enabled: fn(&mut App, HandleId) -> bool,
    consumes_key: fn(&mut App, HandleId, &dyn Intent) -> bool,
    to_key_event_result: ToKeyEventResultFn,
    invoke: InvokeFn,
    add_action_listener: fn(&mut App, HandleId, ActionListenerCallback),
    remove_action_listener: fn(&mut App, HandleId, &ActionListenerCallback),
    notify_action_listeners: fn(&mut App, HandleId),
    make_overridable_action: fn(&mut App, HandleId, BuildContext) -> AnyAction,
}

/// The typed handle for an erased id. Free: nothing is looked up; `get` checks the slot.
fn resolve<L: 'static>(id: HandleId) -> Handle<L> {
    Handle::from_id(id)
}

/// Dart's `_debugCanHandleIntent`, which has to succeed for the leaf to see its own intent.
fn cast_intent<L: Action>(intent: &dyn Intent) -> &L::Intent {
    <L::Intent as ActionIntent>::cast_intent(intent).unwrap_or_else(|| {
        panic!(
            "An Intent of type {:?} cannot be handled by {}: the Intent must be of a subtype \
             of {}.",
            intent,
            std::any::type_name::<L>(),
            std::any::type_name::<L::Intent>()
        )
    })
}

impl ActionVTable {
    /// The table for a plain [`Action`] leaf: Dart's `_isEnabled` / `_invoke` drop the context
    /// because the action is not a `ContextAction`.
    const fn of<L: Action>() -> ActionVTable {
        ActionVTable {
            type_name: std::any::type_name::<L>,
            intent_type: L::intent_type,
            calling_action: |app, id| L::calling_action(resolve(id), app),
            update_calling_action: |app, id, value| {
                L::update_calling_action(resolve(id), app, value)
            },
            is_enabled: |app, id, intent, _context| {
                L::is_enabled(resolve(id), app, cast_intent::<L>(intent))
            },
            is_action_enabled: |app, id| L::is_action_enabled(resolve(id), app),
            consumes_key: |app, id, intent| {
                L::consumes_key(resolve(id), app, cast_intent::<L>(intent))
            },
            to_key_event_result: |app, id, intent, invoke_result| {
                L::to_key_event_result(resolve(id), app, cast_intent::<L>(intent), invoke_result)
            },
            invoke: |app, id, intent, _context| {
                L::invoke(resolve(id), app, cast_intent::<L>(intent))
            },
            add_action_listener: |app, id, listener| {
                L::add_action_listener(resolve(id), app, listener)
            },
            remove_action_listener: |app, id, listener| {
                L::remove_action_listener(resolve(id), app, listener)
            },
            notify_action_listeners: |app, id| L::notify_action_listeners(resolve(id), app),
            make_overridable_action: |app, id, context| {
                L::make_overridable_action(resolve(id), app, context)
            },
        }
    }

    /// The table for a [`ContextAction`] leaf: Dart's `_isEnabled` / `_invoke` hand the context
    /// on.
    const fn of_context<L: ContextAction>() -> ActionVTable {
        ActionVTable {
            is_enabled: |app, id, intent, context| {
                ContextAction::is_enabled(resolve::<L>(id), app, cast_intent::<L>(intent), context)
            },
            invoke: |app, id, intent, context| {
                ContextAction::invoke(resolve::<L>(id), app, cast_intent::<L>(intent), context)
            },
            ..ActionVTable::of::<L>()
        }
    }
}

/// Erased `Action<Intent>`: one identity and a static vtable, the fat pointer rustc cannot
/// build for an arena id.
///
/// This is what a field or parameter Dart types as `Action<Intent>` becomes; the leaf stays in
/// the [`App`] under its own type, and [`downcast`](Self::downcast) gets the typed handle back.
///
/// Equality is Dart's `==` on an object reference: two handles are equal exactly when they
/// address the same action.
#[derive(Clone, Copy)]
pub struct AnyAction {
    id: HandleId,
    vtable: &'static ActionVTable,
}

impl PartialEq for AnyAction {
    fn eq(&self, other: &AnyAction) -> bool {
        self.id == other.id
    }
}

impl Eq for AnyAction {}

impl std::hash::Hash for AnyAction {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl fmt::Debug for AnyAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({:?})", (self.vtable.type_name)(), self.id)
    }
}

impl AnyAction {
    /// The type-erased handle of a plain [`Action`] leaf; [`Action::as_action`] calls this.
    pub fn of<L: Action>(action: Handle<L>) -> AnyAction {
        AnyAction {
            id: action.id(),
            vtable: const { &ActionVTable::of::<L>() },
        }
    }

    /// The type-erased handle of a [`ContextAction`] leaf;
    /// [`context_action_overrides!`](crate::context_action_overrides) calls this.
    pub fn of_context<L: ContextAction>(action: Handle<L>) -> AnyAction {
        AnyAction {
            id: action.id(),
            vtable: const { &ActionVTable::of_context::<L>() },
        }
    }

    /// Creates an [`Action`] that allows itself to be overridden by the closest ancestor
    /// [`Action`] in the given `context` that handles the same [`Intent`], if one exists.
    ///
    /// When invoked, the resulting [`Action`] tries to find the closest [`Action`] in the given
    /// `context` that handles the same type of [`Intent`] as the `default_action`, then calls
    /// its [`invoke`](Self::invoke) method. When no override [`Action`]s can be found, it
    /// invokes the `default_action`.
    ///
    /// An overridable action delegates everything to its override if one exists, and has the
    /// same behavior as its `default_action` otherwise. For this reason, the override has full
    /// control over whether and how an [`Intent`] should be handled, or a key event should be
    /// consumed. An override [`Action`]'s [`Action::calling_action`] property will be set to
    /// the [`Action`] it currently overrides, giving it access to the default behavior.
    ///
    /// The `context` argument is the [`BuildContext`] to find the override with. It is
    /// typically a [`BuildContext`] above the [`Actions`] widget that contains this overridable
    /// [`Action`].
    ///
    /// The `default_action` argument is the [`Action`] to be invoked where no ancestor
    /// [`Action`]s can be found in `context` that handle the same type of [`Intent`].
    ///
    /// Dart's `Action.overridable`.
    pub fn overridable(
        app: &mut App,
        default_action: AnyAction,
        context: BuildContext,
    ) -> AnyAction {
        default_action.make_overridable_action(app, context)
    }

    /// The arena id behind this handle.
    pub fn id(self) -> HandleId {
        self.id
    }

    /// Dart's `action as T`: the typed handle when this action is a `T`, else `None`.
    pub fn downcast<T: 'static>(self, app: &App) -> Option<Handle<T>> {
        app.handle::<T>(self.id)
    }

    /// See [`Action::intent_type`].
    pub fn intent_type(self) -> TypeId {
        (self.vtable.intent_type)()
    }

    /// See [`Action::calling_action`].
    pub fn calling_action(self, app: &App) -> Option<AnyAction> {
        (self.vtable.calling_action)(app, self.id)
    }

    /// See [`Action::update_calling_action`].
    pub fn update_calling_action(self, app: &mut App, value: Option<AnyAction>) {
        (self.vtable.update_calling_action)(app, self.id, value);
    }

    /// Dart's `Action._isEnabled`: [`ContextAction::is_enabled`] when the action is a
    /// [`ContextAction`], [`Action::is_enabled`] otherwise.
    pub fn is_enabled(
        self,
        app: &mut App,
        intent: &dyn Intent,
        context: Option<BuildContext>,
    ) -> bool {
        (self.vtable.is_enabled)(app, self.id, intent, context)
    }

    /// See [`Action::is_action_enabled`].
    pub fn is_action_enabled(self, app: &mut App) -> bool {
        (self.vtable.is_action_enabled)(app, self.id)
    }

    /// See [`Action::consumes_key`].
    pub fn consumes_key(self, app: &mut App, intent: &dyn Intent) -> bool {
        (self.vtable.consumes_key)(app, self.id, intent)
    }

    /// See [`Action::to_key_event_result`].
    pub fn to_key_event_result(
        self,
        app: &mut App,
        intent: &dyn Intent,
        invoke_result: Option<&Rc<dyn Any>>,
    ) -> KeyEventResult {
        (self.vtable.to_key_event_result)(app, self.id, intent, invoke_result)
    }

    /// Dart's `Action._invoke`: [`ContextAction::invoke`] when the action is a
    /// [`ContextAction`], [`Action::invoke`] otherwise.
    pub fn invoke(
        self,
        app: &mut App,
        intent: &dyn Intent,
        context: Option<BuildContext>,
    ) -> Option<Rc<dyn Any>> {
        (self.vtable.invoke)(app, self.id, intent, context)
    }

    /// See [`Action::add_action_listener`].
    pub fn add_action_listener(self, app: &mut App, listener: ActionListenerCallback) {
        (self.vtable.add_action_listener)(app, self.id, listener);
    }

    /// See [`Action::remove_action_listener`].
    pub fn remove_action_listener(self, app: &mut App, listener: &ActionListenerCallback) {
        (self.vtable.remove_action_listener)(app, self.id, listener);
    }

    /// See [`Action::notify_action_listeners`].
    pub fn notify_action_listeners(self, app: &mut App) {
        (self.vtable.notify_action_listeners)(app, self.id);
    }

    /// See [`Action::make_overridable_action`].
    fn make_overridable_action(self, app: &mut App, context: BuildContext) -> AnyAction {
        (self.vtable.make_overridable_action)(app, self.id, context)
    }
}

// ---------------------------------------------------------------------------------------------
// ActionListener

/// A helper widget for making sure that listeners on an action are removed properly.
///
/// Listeners on an [`Action`] must have their listener callbacks removed with
/// [`Action::remove_action_listener`] when the listener is disposed of. This widget helps with
/// that, by providing a lifetime for the connection between the
/// [`listener`](Self::listener) and the [`action`](Self::action), and by handling the adding
/// and removing of the [`listener`](Self::listener) at the right points in the widget
/// lifecycle.
///
/// If you listen to an [`Action`] in a widget hierarchy, you should use this widget. If you are
/// using an [`Action`] outside of a widget context, then you must call
/// [`Action::remove_action_listener`] yourself.
pub struct ActionListener {
    /// See [`Widget::key`](crate::Widget::key).
    pub key: Option<KeyRef>,

    /// The [`ActionListenerCallback`] callback to register with the [`action`](Self::action).
    pub listener: ActionListenerCallback,

    /// The [`Action`] that the callback will be registered with.
    pub action: AnyAction,

    /// The widget below this widget in the tree.
    pub child: WidgetRef,
}

impl ActionListener {
    /// Creates an [`ActionListener`].
    pub fn new<K>(
        listener: ActionListenerCallback,
        action: AnyAction,
        child: impl IntoWidget<K>,
    ) -> ActionListener {
        ActionListener {
            key: None,
            listener,
            action,
            child: child.into_widget(),
        }
    }

    /// Dart `ActionListener(key:)`.
    pub fn key(mut self, key: KeyRef) -> ActionListener {
        self.key = Some(key);
        self
    }
}

impl fmt::Debug for ActionListener {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ActionListener")
            .field("action", &self.action)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for ActionListener {
    type State = ActionListenerState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> ActionListenerState {
        ActionListenerState {
            state: StateData::new(),
        }
    }
}

/// Dart's `_ActionListenerState`.
pub struct ActionListenerState {
    state: StateData<ActionListener>,
}

impl State for ActionListenerState {
    type Widget = ActionListener;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let widget = self.widget(app);
        let (action, listener) = (widget.action, Rc::clone(&widget.listener));
        action.add_action_listener(app, listener);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &ActionListener) {
        let widget = self.widget(app);
        let (action, listener) = (widget.action, Rc::clone(&widget.listener));
        if old_widget.action == action && Rc::ptr_eq(&old_widget.listener, &listener) {
            return;
        }
        let (old_action, old_listener) = (old_widget.action, Rc::clone(&old_widget.listener));
        old_action.remove_action_listener(app, &old_listener);
        action.add_action_listener(app, listener);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let widget = self.widget(app);
        let (action, listener) = (widget.action, Rc::clone(&widget.listener));
        action.remove_action_listener(app, &listener);
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        self.widget(app).child.clone()
    }
}

// ---------------------------------------------------------------------------------------------
// CallbackAction

/// The signature of a callback accepted by `CallbackAction::on_invoke`.
///
/// Such callbacks are implementations of [`Action::invoke`]. The returned value is the return
/// value of [`Action::invoke`], the argument is the intent passed to [`Action::invoke`], and so
/// forth.
pub type OnInvokeCallback<T> = Rc<dyn Fn(&mut App, &T) -> Option<Rc<dyn Any>>>;

/// An [`Action`] that takes a callback in order to configure it without having to create an
/// explicit [`Action`] implementation just to call a callback.
///
/// See also:
///
///  * `Shortcuts`, which is a widget that contains a key map, in which it looks up key
///    combinations in order to invoke actions.
///  * [`Actions`], which is a widget that defines a map of [`Intent`] to [`Action`] and allows
///    redefining of actions for its descendants.
///  * [`ActionDispatcher`], a class that takes an [`Action`] and invokes it using a `FocusNode`
///    for context.
pub struct CallbackAction<T: Intent> {
    action: ActionData,

    /// The callback to be called when invoked.
    ///
    /// This is effectively the implementation of [`Action::invoke`].
    on_invoke: OnInvokeCallback<T>,
}

impl<T: Intent> CallbackAction<T> {
    /// A constructor for a [`CallbackAction`].
    ///
    /// The given callback is used as the implementation of [`Action::invoke`].
    pub fn new(app: &mut App, on_invoke: OnInvokeCallback<T>) -> Handle<CallbackAction<T>> {
        app.create(CallbackAction {
            action: ActionData::new(),
            on_invoke,
        })
    }
}

impl<T: Intent> Action for CallbackAction<T> {
    type Intent = T;
    crate::action_accessors!();

    fn invoke(self: Handle<Self>, app: &mut App, intent: &T) -> Option<Rc<dyn Any>> {
        let on_invoke = Rc::clone(&app.get(self).on_invoke);
        on_invoke(app, intent)
    }
}

// ---------------------------------------------------------------------------------------------
// ActionDispatcher

/// The erased [`ActionDispatcher`]: what `Actions::dispatcher` holds.
pub type ActionDispatcherRef = Rc<dyn ActionDispatcher>;

/// An action dispatcher that invokes the actions given to it.
///
/// [`invoke_action`](Self::invoke_action) directly calls [`Action::invoke`] on the [`Action`]
/// object.
///
/// For [`ContextAction`] actions, if no `context` is provided, the [`BuildContext`] of the
/// [`primary_focus`] is used instead.
///
/// Dart's instantiable `const ActionDispatcher()` is [`PlainActionDispatcher`].
///
/// See also:
///
///  * `ShortcutManager`, that uses this class to invoke actions.
///  * `Shortcuts`, which defines key mappings to [`Intent`]s.
///  * [`Actions`], which defines a mapping between an [`Intent`] type and an [`Action`].
pub trait ActionDispatcher: Debug {
    /// Invokes the given `action`, passing it the given `intent`.
    ///
    /// The action will be invoked with the given `context`, if given, but only if the action is
    /// a [`ContextAction`]. If no `context` is given, and the action is a [`ContextAction`],
    /// then the context from the [`primary_focus`] is used.
    ///
    /// Returns the object returned from [`Action::invoke`].
    ///
    /// The caller must receive a `true` result from [`AnyAction::is_enabled`] before calling
    /// this function. This function will assert if the action is not enabled when called.
    ///
    /// Consider using [`invoke_action_if_enabled`](Self::invoke_action_if_enabled) to invoke
    /// the action conditionally based on whether it is enabled or not, without having to check
    /// first.
    fn invoke_action(
        &self,
        app: &mut App,
        action: AnyAction,
        intent: &dyn Intent,
        context: Option<BuildContext>,
    ) -> Option<Rc<dyn Any>> {
        let target = context.or_else(|| primary_focus(app).and_then(|node| node.context(app)));
        debug_assert!(
            action.is_enabled(app, intent, target),
            "Action must be enabled when calling invokeAction"
        );
        action.invoke(app, intent, target)
    }

    /// Invokes the given `action`, passing it the given `intent`, but only if the action is
    /// enabled.
    ///
    /// The action will be invoked with the given `context`, if given, but only if the action is
    /// a [`ContextAction`]. If no `context` is given, and the action is a [`ContextAction`],
    /// then the context from the [`primary_focus`] is used.
    ///
    /// The return value has two components. The first is a boolean indicating if the action was
    /// enabled (as per [`AnyAction::is_enabled`]). If this is false, the second return value is
    /// `None`. Otherwise, the second return value is the object returned from
    /// [`Action::invoke`].
    ///
    /// Consider using [`invoke_action`](Self::invoke_action) if the enabled state of the action
    /// is not in question; this avoids calling [`AnyAction::is_enabled`] redundantly.
    fn invoke_action_if_enabled(
        &self,
        app: &mut App,
        action: AnyAction,
        intent: &dyn Intent,
        context: Option<BuildContext>,
    ) -> (bool, Option<Rc<dyn Any>>) {
        let target = context.or_else(|| primary_focus(app).and_then(|node| node.context(app)));
        if action.is_enabled(app, intent, target) {
            return (true, action.invoke(app, intent, target));
        }
        (false, None)
    }
}

/// Dart's instantiable `const ActionDispatcher()`: the dispatcher
/// [`Actions::of`] falls back to.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PlainActionDispatcher;

impl PlainActionDispatcher {
    /// Creates an action dispatcher that invokes actions directly.
    pub fn new() -> PlainActionDispatcher {
        PlainActionDispatcher
    }
}

impl ActionDispatcher for PlainActionDispatcher {}

// ---------------------------------------------------------------------------------------------
// Actions

/// A widget that maps [`Intent`]s to [`Action`]s to be used by its descendants when invoking an
/// [`Action`].
///
/// Actions are typically invoked using `Shortcuts`. They can also be invoked using
/// [`Actions::invoke`] on a context containing an ambient [`Actions`] widget.
///
/// See also:
///
///  * `Shortcuts`, a widget used to bind key combinations to [`Intent`]s.
///  * [`Intent`], a trait for values that contain configuration information for running an
///    [`Action`].
///  * [`Action`], a trait for containing and defining an invocation of a user action.
///  * [`ActionDispatcher`], the object that this widget uses to manage actions.
pub struct Actions {
    /// See [`Widget::key`](crate::Widget::key).
    pub key: Option<KeyRef>,

    /// The [`ActionDispatcher`] object that invokes actions.
    ///
    /// This is what is returned from [`Actions::of`], and used by [`Actions::invoke`].
    ///
    /// If this [`dispatcher`](Self::dispatcher) is `None`, then [`Actions::of`] and
    /// [`Actions::invoke`] will look up the tree until they find an [`Actions`] widget that has
    /// a dispatcher set. If no such widget is found, then they will return/use a
    /// [`PlainActionDispatcher`].
    pub dispatcher: Option<ActionDispatcherRef>,

    /// A map of [`Intent`] types to [`Action`] objects that defines which actions this widget
    /// knows about.
    ///
    /// For performance reasons, it is recommended that a pre-built map is passed in here
    /// instead of defining it inline in the build function.
    pub actions: HashMap<TypeId, AnyAction>,

    /// The widget below this widget in the tree.
    pub child: WidgetRef,
}

impl Actions {
    /// Creates an [`Actions`] widget.
    pub fn new<K>(actions: HashMap<TypeId, AnyAction>, child: impl IntoWidget<K>) -> Actions {
        Actions {
            key: None,
            dispatcher: None,
            actions,
            child: child.into_widget(),
        }
    }

    /// Dart `Actions(key:)`.
    pub fn key(mut self, key: KeyRef) -> Actions {
        self.key = Some(key);
        self
    }

    /// Dart `Actions(dispatcher:)`.
    pub fn dispatcher(mut self, dispatcher: ActionDispatcherRef) -> Actions {
        self.dispatcher = Some(dispatcher);
        self
    }

    /// Visits the [`Actions`] widget ancestors of the given element. Returns true if the
    /// visitor found what it was looking for.
    fn visit_actions_ancestors(
        app: &mut App,
        context: BuildContext,
        visitor: &mut dyn FnMut(&mut App, BuildContext) -> bool,
    ) -> bool {
        if !context.mounted(app) {
            return false;
        }
        let mut actions_element =
            context.get_element_for_inherited_widget_of_exact_type::<ActionsScope>(app);
        while let Some(element) = actions_element {
            if visitor(app, element) {
                break;
            }
            // `get_parent` is needed here because
            // `get_element_for_inherited_widget_of_exact_type` will return itself if it happens
            // to be of the correct type.
            actions_element = get_parent(app, element).and_then(|parent| {
                parent.get_element_for_inherited_widget_of_exact_type::<ActionsScope>(app)
            });
        }
        actions_element.is_some()
    }

    /// Finds the nearest valid [`ActionDispatcher`], or creates a new one if it doesn't find
    /// one.
    fn find_dispatcher(app: &mut App, context: BuildContext) -> ActionDispatcherRef {
        let mut dispatcher: Option<ActionDispatcherRef> = None;
        Actions::visit_actions_ancestors(app, context, &mut |app, element| {
            let found = Actions::scope_of(app, element).dispatcher.clone();
            match found {
                Some(found) => {
                    dispatcher = Some(found);
                    true
                }
                None => false,
            }
        });
        dispatcher.unwrap_or_else(|| Rc::new(PlainActionDispatcher::new()))
    }

    /// The `_ActionsScope` an ancestor element found by
    /// [`visit_actions_ancestors`](Self::visit_actions_ancestors) holds.
    fn scope_of(app: &App, element: BuildContext) -> &ActionsScope {
        crate::framework::downcast_widget::<ActionsScope>(&**element.widget(app))
            .expect("the element of an _ActionsScope holds one")
    }

    /// Returns a handler that invokes the bound action for the given `intent` if the action is
    /// enabled, and returns `None` if the action is not enabled, or no matching action is
    /// found.
    ///
    /// This is intended to be used in widgets which have something similar to an `on_tap`
    /// handler, which takes a `GestureTapCallback`, and can be set to the result of calling
    /// this function.
    ///
    /// Creates a dependency on the [`Actions`] widget that maps the bound action so that if the
    /// actions change, the context will be rebuilt and find the updated action.
    ///
    /// The value returned from [`Action::invoke`] is discarded when the returned callback is
    /// called. If the return value is needed, consider using [`Actions::invoke`] instead.
    pub fn handler(app: &mut App, context: BuildContext, intent: IntentRef) -> Option<Listener> {
        let action = Actions::maybe_find(app, context, intent.intent_type())?;
        if action.is_enabled(app, &*intent, Some(context)) {
            return Some(Listener::new(move |app| {
                // Could be that the action was enabled when the closure was created, but is now
                // no longer enabled, so check again.
                if action.is_enabled(app, &*intent, Some(context)) {
                    Actions::of(app, context).invoke_action(app, action, &*intent, Some(context));
                }
            }));
        }
        None
    }

    /// Finds the [`Action`] bound to the given `intent_type` in the given `context`.
    ///
    /// Creates a dependency on the [`Actions`] widget that maps the bound action so that if the
    /// actions change, the context will be rebuilt and find the updated action.
    ///
    /// If no [`Actions`] widget surrounds the given context, or no action is bound to
    /// `intent_type`, this function panics.
    ///
    /// See also:
    ///
    ///  * [`maybe_find`](Self::maybe_find), which is similar to this function, but will return
    ///    `None` if no [`Actions`] ancestor is found.
    pub fn find(app: &mut App, context: BuildContext, intent_type: TypeId) -> AnyAction {
        Actions::maybe_find(app, context, intent_type).expect(
            "Unable to find an action for the given intent type in an Actions widget in the \
             given context.\nActions.find() was called on a context that doesn't contain an \
             Actions widget with a mapping for the given intent type.",
        )
    }

    /// Finds the [`Action`] bound to the given `intent_type` in the given `context`.
    ///
    /// Creates a dependency on the [`Actions`] widget that maps the bound action so that if the
    /// actions change, the context will be rebuilt and find the updated action.
    ///
    /// If no [`Actions`] widget surrounds the given context, this function returns `None`.
    ///
    /// See also:
    ///
    ///  * [`find`](Self::find), which is similar to this function, but will panic if no
    ///    [`Actions`] ancestor is found.
    pub fn maybe_find(
        app: &mut App,
        context: BuildContext,
        intent_type: TypeId,
    ) -> Option<AnyAction> {
        let mut action: Option<AnyAction> = None;
        Actions::visit_actions_ancestors(app, context, &mut |app, element| {
            let result = Actions::get_action_for_intent(app, element, intent_type);
            match result {
                Some(result) => {
                    context.depend_on_inherited_element(app, element, None);
                    action = Some(result);
                    true
                }
                None => false,
            }
        });
        action
    }

    /// Dart's `Actions._maybeFindWithoutDependingOn`.
    fn maybe_find_without_depending_on(
        app: &mut App,
        context: BuildContext,
        intent_type: TypeId,
    ) -> Option<AnyAction> {
        let mut action: Option<AnyAction> = None;
        Actions::visit_actions_ancestors(app, context, &mut |app, element| {
            let result = Actions::get_action_for_intent(app, element, intent_type);
            match result {
                Some(result) => {
                    action = Some(result);
                    true
                }
                None => false,
            }
        });
        action
    }

    /// Dart's `Actions._getActionForIntent`.
    fn get_action_for_intent(
        app: &App,
        element: BuildContext,
        intent_type: TypeId,
    ) -> Option<AnyAction> {
        Actions::scope_of(app, element)
            .actions
            .get(&intent_type)
            .copied()
    }

    /// Returns the [`ActionDispatcher`] associated with the [`Actions`] widget that most tightly
    /// encloses the given [`BuildContext`].
    ///
    /// Will return a newly created [`PlainActionDispatcher`] if no ambient [`Actions`] widget is
    /// found.
    pub fn of(app: &mut App, context: BuildContext) -> ActionDispatcherRef {
        let marker = context
            .depend_on_inherited_widget_of_exact_type::<ActionsScope>(app)
            .and_then(|scope| scope.dispatcher.clone());
        match marker {
            Some(dispatcher) => dispatcher,
            None => Actions::find_dispatcher(app, context),
        }
    }

    /// Invokes the action associated with the given [`Intent`] using the [`Actions`] widget that
    /// most tightly encloses the given [`BuildContext`].
    ///
    /// This method returns the result of invoking the action's [`Action::invoke`] method.
    ///
    /// If the given `intent` doesn't map to an action, then it will look to the next ancestor
    /// [`Actions`] widget in the hierarchy until it reaches the root.
    ///
    /// This method panics if no ambient [`Actions`] widget is found.
    pub fn invoke(
        app: &mut App,
        context: BuildContext,
        intent: &dyn Intent,
    ) -> Option<Rc<dyn Any>> {
        let mut return_value: Option<Rc<dyn Any>> = None;
        let action_found = Actions::visit_actions_ancestors(app, context, &mut |app, element| {
            let result = Actions::get_action_for_intent(app, element, intent.intent_type());
            if let Some(result) = result
                && result.is_enabled(app, intent, Some(context))
            {
                // Invoke the action we found using the relevant dispatcher from the Actions
                // element we found.
                return_value = Actions::find_dispatcher(app, element).invoke_action(
                    app,
                    result,
                    intent,
                    Some(context),
                );
            }
            result.is_some()
        });
        debug_assert!(
            action_found,
            "Unable to find an action for an Intent with type {intent:?} in an Actions widget in \
             the given context."
        );
        return_value
    }

    /// Invokes the action associated with the given [`Intent`] using the [`Actions`] widget that
    /// most tightly encloses the given [`BuildContext`].
    ///
    /// This method returns the result of invoking the action's [`Action::invoke`] method. If no
    /// action mapping was found for the specified intent, or if the first action found was
    /// disabled, or the action itself returns `None` from [`Action::invoke`], then this method
    /// returns `None`.
    ///
    /// If the given `intent` doesn't map to an action, then it will look to the next ancestor
    /// [`Actions`] widget in the hierarchy until it reaches the root. If a suitable [`Action`]
    /// is found but its [`AnyAction::is_enabled`] returns false, the search will stop and this
    /// method will return `None`.
    pub fn maybe_invoke(
        app: &mut App,
        context: BuildContext,
        intent: &dyn Intent,
    ) -> Option<Rc<dyn Any>> {
        let mut return_value: Option<Rc<dyn Any>> = None;
        Actions::visit_actions_ancestors(app, context, &mut |app, element| {
            let result = Actions::get_action_for_intent(app, element, intent.intent_type());
            if let Some(result) = result
                && result.is_enabled(app, intent, Some(context))
            {
                return_value = Actions::find_dispatcher(app, element).invoke_action(
                    app,
                    result,
                    intent,
                    Some(context),
                );
            }
            result.is_some()
        });
        return_value
    }
}

impl fmt::Debug for Actions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Actions")
            .field("dispatcher", &self.dispatcher)
            .field("actions", &self.actions)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for Actions {
    type State = ActionsState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> ActionsState {
        ActionsState {
            state: StateData::new(),
            listened_actions: HashSet::new(),
            action_changed: None,
            rebuild_key: 0,
        }
    }
}

/// Dart's `_ActionsState`.
pub struct ActionsState {
    state: StateData<Actions>,
    /// The set of actions that this [`Actions`] widget is currently listening to.
    listened_actions: HashSet<AnyAction>,
    /// Dart's `_handleActionChanged` tear-off, kept so that it can be removed again.
    action_changed: Option<ActionListenerCallback>,
    /// Used to tell the marker to rebuild its dependencies when the state of an action in the
    /// map changes.
    rebuild_key: u64,
}

impl ActionsState {
    fn handle_action_changed(self: Handle<Self>, app: &mut App) {
        // Generate a new key so that the marker notifies dependents.
        self.set_state(app, |state| state.rebuild_key += 1);
    }

    fn update_action_listeners(self: Handle<Self>, app: &mut App) {
        let widget_actions: HashSet<AnyAction> =
            self.widget(app).actions.values().copied().collect();
        let listened = app.get(self).listened_actions.clone();
        let removed_actions: Vec<AnyAction> =
            listened.difference(&widget_actions).copied().collect();
        let added_actions: Vec<AnyAction> = widget_actions.difference(&listened).copied().collect();

        let handler = self.action_changed(app);
        for action in removed_actions {
            action.remove_action_listener(app, &handler);
        }
        for action in added_actions {
            action.add_action_listener(app, Rc::clone(&handler));
        }
        app.get_mut(self).listened_actions = widget_actions;
    }

    /// The `Rc` behind Dart's `_handleActionChanged` tear-off, minted once so that every
    /// add / remove pair matches.
    fn action_changed(self: Handle<Self>, app: &mut App) -> ActionListenerCallback {
        if let Some(handler) = app.get(self).action_changed.clone() {
            return handler;
        }
        let handler: ActionListenerCallback =
            Rc::new(move |app: &mut App, _action: AnyAction| self.handle_action_changed(app));
        app.get_mut(self).action_changed = Some(Rc::clone(&handler));
        handler
    }
}

impl State for ActionsState {
    type Widget = Actions;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        self.update_action_listeners(app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, _old_widget: &Actions) {
        self.update_action_listeners(app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let handler = self.action_changed(app);
        for action in std::mem::take(&mut app.get_mut(self).listened_actions) {
            action.remove_action_listener(app, &handler);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let rebuild_key = app.get(self).rebuild_key;
        let widget = self.widget(app);
        ActionsScope {
            dispatcher: widget.dispatcher.clone(),
            actions: widget.actions.clone(),
            rebuild_key,
            child: widget.child.clone(),
        }
        .into_widget()
    }
}

/// An inherited widget used by the [`Actions`] widget for fast lookup of the [`Actions`] widget
/// information.
#[derive(Debug)]
struct ActionsScope {
    dispatcher: Option<ActionDispatcherRef>,
    actions: HashMap<TypeId, AnyAction>,
    rebuild_key: u64,
    child: WidgetRef,
}

impl InheritedWidget for ActionsScope {
    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &ActionsScope) -> bool {
        self.rebuild_key != old_widget.rebuild_key
            || !same_dispatcher(&old_widget.dispatcher, &self.dispatcher)
            || old_widget.actions != self.actions
    }
}

/// Dart's `oldWidget.dispatcher != dispatcher` on a reference type.
fn same_dispatcher(a: &Option<ActionDispatcherRef>, b: &Option<ActionDispatcherRef>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => Rc::ptr_eq(a, b),
        _ => false,
    }
}

// ---------------------------------------------------------------------------------------------
// FocusableActionDetector

/// A widget that combines the functionality of [`Actions`], `Shortcuts`, `MouseRegion` and a
/// `Focus` widget to create a detector that defines actions and key bindings, and provides
/// callbacks for handling focus and hover highlights.
///
/// This widget can be used to give a control the required detection modes for focus and hover
/// handling. It is most often used when authoring a new control widget, and the new control
/// should be enabled for keyboard traversal and activation.
///
/// This widget doesn't have any visual representation, it is just a detector that provides
/// focus and hover capabilities.
///
/// It hosts its own `FocusNode` or uses [`focus_node`](Self::focus_node), if given.
pub struct FocusableActionDetector {
    /// See [`Widget::key`](crate::Widget::key).
    pub key: Option<KeyRef>,

    /// Is this widget enabled or not.
    ///
    /// If disabled, will not send any notifications needed to update highlight or focus state,
    /// and will not define or respond to any actions or shortcuts.
    ///
    /// When disabled, adds `Focus` to the widget tree, but sets `Focus::can_request_focus` to
    /// false.
    pub enabled: bool,

    /// An optional focus node to use as the focus node for this widget.
    pub focus_node: Option<AnyFocusNode>,

    /// True if this widget will be selected as the initial focus when no other node in its
    /// scope is currently focused.
    pub autofocus: bool,

    /// If false, will make this widget's descendants unfocusable.
    pub descendants_are_focusable: bool,

    /// If false, will make this widget's descendants untraversable.
    pub descendants_are_traversable: bool,

    /// A map of [`Intent`] types to [`Action`] objects that defines which actions this widget
    /// knows about.
    pub actions: Option<HashMap<TypeId, AnyAction>>,

    /// The map of shortcuts that describes the mapping between a key sequence defined by a
    /// `ShortcutActivator` and the [`Intent`] that will be emitted when that key sequence is
    /// pressed.
    pub shortcuts: Option<ShortcutMap>,

    /// A function that will be called when the focus highlight should be shown or hidden.
    ///
    /// This method is not triggered at the unmount of the widget.
    pub on_show_focus_highlight: Option<ValueChanged<bool>>,

    /// A function that will be called when the hover highlight should be shown or hidden.
    ///
    /// This method is not triggered at the unmount of the widget.
    pub on_show_hover_highlight: Option<ValueChanged<bool>>,

    /// A function that will be called when the focus changes.
    ///
    /// Called with true if the [`focus_node`](Self::focus_node) has primary focus.
    pub on_focus_change: Option<ValueChanged<bool>>,

    /// The cursor for a mouse pointer when it enters or is hovering over the widget.
    ///
    /// The [`mouse_cursor`](Self::mouse_cursor) defaults to `MouseCursor.defer`, deferring the
    /// choice of cursor to the next region behind it in hit-test order.
    pub mouse_cursor: MouseCursorRef,

    /// Whether to include semantics from `Focus`.
    ///
    /// Defaults to true. Nothing reads it yet: the `Semantics` wrapper waits with
    /// accessibility.
    pub include_focus_semantics: bool,

    /// The child widget for this [`FocusableActionDetector`] widget.
    pub child: WidgetRef,
}

impl FocusableActionDetector {
    /// Creates a [`FocusableActionDetector`].
    pub fn new<K>(child: impl IntoWidget<K>) -> FocusableActionDetector {
        FocusableActionDetector {
            key: None,
            enabled: true,
            focus_node: None,
            autofocus: false,
            descendants_are_focusable: true,
            descendants_are_traversable: true,
            actions: None,
            shortcuts: None,
            on_show_focus_highlight: None,
            on_show_hover_highlight: None,
            on_focus_change: None,
            mouse_cursor: <dyn MouseCursor>::defer(),
            include_focus_semantics: true,
            child: child.into_widget(),
        }
    }

    /// Dart `FocusableActionDetector(key:)`.
    pub fn key(mut self, key: KeyRef) -> FocusableActionDetector {
        self.key = Some(key);
        self
    }

    /// Dart `FocusableActionDetector(enabled:)`.
    pub fn enabled(mut self, enabled: bool) -> FocusableActionDetector {
        self.enabled = enabled;
        self
    }

    /// Dart `FocusableActionDetector(focusNode:)`.
    pub fn focus_node(mut self, focus_node: AnyFocusNode) -> FocusableActionDetector {
        self.focus_node = Some(focus_node);
        self
    }

    /// Dart `FocusableActionDetector(autofocus:)`.
    pub fn autofocus(mut self, autofocus: bool) -> FocusableActionDetector {
        self.autofocus = autofocus;
        self
    }

    /// Dart `FocusableActionDetector(descendantsAreFocusable:)`.
    pub fn descendants_are_focusable(
        mut self,
        descendants_are_focusable: bool,
    ) -> FocusableActionDetector {
        self.descendants_are_focusable = descendants_are_focusable;
        self
    }

    /// Dart `FocusableActionDetector(descendantsAreTraversable:)`.
    pub fn descendants_are_traversable(
        mut self,
        descendants_are_traversable: bool,
    ) -> FocusableActionDetector {
        self.descendants_are_traversable = descendants_are_traversable;
        self
    }

    /// Dart `FocusableActionDetector(actions:)`.
    pub fn actions(mut self, actions: HashMap<TypeId, AnyAction>) -> FocusableActionDetector {
        self.actions = Some(actions);
        self
    }

    /// Dart `FocusableActionDetector(shortcuts:)`.
    pub fn shortcuts(mut self, shortcuts: ShortcutMap) -> FocusableActionDetector {
        self.shortcuts = Some(shortcuts);
        self
    }

    /// Dart `FocusableActionDetector(onShowFocusHighlight:)`.
    pub fn on_show_focus_highlight(
        mut self,
        on_show_focus_highlight: impl Fn(&mut App, bool) + 'static,
    ) -> FocusableActionDetector {
        self.on_show_focus_highlight = Some(Rc::new(on_show_focus_highlight));
        self
    }

    /// Dart `FocusableActionDetector(onShowHoverHighlight:)`.
    pub fn on_show_hover_highlight(
        mut self,
        on_show_hover_highlight: impl Fn(&mut App, bool) + 'static,
    ) -> FocusableActionDetector {
        self.on_show_hover_highlight = Some(Rc::new(on_show_hover_highlight));
        self
    }

    /// Dart `FocusableActionDetector(onFocusChange:)`.
    pub fn on_focus_change(
        mut self,
        on_focus_change: impl Fn(&mut App, bool) + 'static,
    ) -> FocusableActionDetector {
        self.on_focus_change = Some(Rc::new(on_focus_change));
        self
    }

    /// Dart `FocusableActionDetector(mouseCursor:)`.
    pub fn mouse_cursor(mut self, mouse_cursor: MouseCursorRef) -> FocusableActionDetector {
        self.mouse_cursor = mouse_cursor;
        self
    }

    /// Dart `FocusableActionDetector(includeFocusSemantics:)`.
    pub fn include_focus_semantics(
        mut self,
        include_focus_semantics: bool,
    ) -> FocusableActionDetector {
        self.include_focus_semantics = include_focus_semantics;
        self
    }
}

impl fmt::Debug for FocusableActionDetector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FocusableActionDetector")
            .field("enabled", &self.enabled)
            .field("focusNode", &self.focus_node)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for FocusableActionDetector {
    type State = FocusableActionDetectorState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> FocusableActionDetectorState {
        FocusableActionDetectorState {
            state: StateData::new(),
            can_show_highlight: false,
            hovering: false,
            focused: false,
            highlight_mode_listener: None,
            // This global key is needed to keep only the necessary widgets in the tree while
            // maintaining the subtree's state.
            mouse_region_key: Rc::new(GlobalKey::new()),
        }
    }
}

/// Dart's `_FocusableActionDetectorState`.
pub struct FocusableActionDetectorState {
    state: StateData<FocusableActionDetector>,
    can_show_highlight: bool,
    hovering: bool,
    focused: bool,
    highlight_mode_listener: Option<HighlightModeListener>,
    mouse_region_key: KeyRef,
}

/// Dart's `VoidCallback? task` argument of `_mayTriggerCallback`, which mutates the state
/// between the two readings.
type HighlightTask<'a> = &'a dyn Fn(Handle<FocusableActionDetectorState>, &mut App);

/// The state Dart's `_mayTriggerCallback` records before and after its task.
#[derive(Clone, Copy)]
struct HighlightState {
    hover: bool,
    focus: bool,
}

impl FocusableActionDetectorState {
    fn update_highlight_mode(self: Handle<Self>, app: &mut App, _mode: FocusHighlightMode) {
        self.may_trigger_callback(
            app,
            None,
            Some(&|this, app| {
                let mode = FocusManager::instance(app).highlight_mode(app);
                app.get_mut(this).can_show_highlight = match mode {
                    FocusHighlightMode::Touch => false,
                    FocusHighlightMode::Traditional => true,
                };
            }),
        );
    }

    /// Have to have this separate from
    /// [`update_highlight_mode`](Self::update_highlight_mode) because it gets called in
    /// `init_state`, where things aren't mounted yet.
    ///
    /// Since this method is a highlight mode listener, it is only called immediately following
    /// pointer events.
    fn handle_focus_highlight_mode_change(
        self: Handle<Self>,
        app: &mut App,
        mode: FocusHighlightMode,
    ) {
        if !self.mounted(app) {
            return;
        }
        self.update_highlight_mode(app, mode);
    }

    fn handle_mouse_enter(self: Handle<Self>, app: &mut App) {
        if !app.get(self).hovering {
            self.may_trigger_callback(
                app,
                None,
                Some(&|this, app| {
                    app.get_mut(this).hovering = true;
                }),
            );
        }
    }

    fn handle_mouse_exit(self: Handle<Self>, app: &mut App) {
        if app.get(self).hovering {
            self.may_trigger_callback(
                app,
                None,
                Some(&|this, app| {
                    app.get_mut(this).hovering = false;
                }),
            );
        }
    }

    fn handle_focus_change(self: Handle<Self>, app: &mut App, focused: bool) {
        if app.get(self).focused != focused {
            self.may_trigger_callback(
                app,
                None,
                Some(&|this, app| {
                    app.get_mut(this).focused = focused;
                }),
            );
            if let Some(on_focus_change) = self.widget(app).on_focus_change.clone() {
                let focused = app.get(self).focused;
                on_focus_change(app, focused);
            }
        }
    }

    fn should_show_hover_highlight(self: Handle<Self>, app: &App, target_enabled: bool) -> bool {
        app.get(self).hovering && target_enabled && app.get(self).can_show_highlight
    }

    fn can_request_focus(self: Handle<Self>, app: &mut App, target_enabled: bool) -> bool {
        let context = self.context(app);
        match MediaQuery::maybe_navigation_mode_of(app, context) {
            None | Some(NavigationMode::Traditional) => target_enabled,
            Some(NavigationMode::Directional) => true,
        }
    }

    fn should_show_focus_highlight(
        self: Handle<Self>,
        app: &mut App,
        target_enabled: bool,
    ) -> bool {
        app.get(self).focused
            && app.get(self).can_show_highlight
            && self.can_request_focus(app, target_enabled)
    }

    fn highlight_state(self: Handle<Self>, app: &mut App, target_enabled: bool) -> HighlightState {
        HighlightState {
            hover: self.should_show_hover_highlight(app, target_enabled),
            focus: self.should_show_focus_highlight(app, target_enabled),
        }
    }

    /// Record old states, do `task` if given, then compare old states with the new states, and
    /// trigger callbacks if necessary.
    ///
    /// The old states are collected from `old_enabled` if it is provided (Dart passes the whole
    /// `oldWidget`, of which this is the only field read), or the current widget (before doing
    /// `task`) otherwise. The new states are always collected from the current widget.
    fn may_trigger_callback(
        self: Handle<Self>,
        app: &mut App,
        old_enabled: Option<bool>,
        task: Option<HighlightTask<'_>>,
    ) {
        debug_assert!(
            SchedulerBinding::scheduler_phase(app) != SchedulerPhase::PersistentCallbacks
        );
        let old_enabled = old_enabled.unwrap_or_else(|| self.widget(app).enabled);
        let did = self.highlight_state(app, old_enabled);
        if let Some(task) = task {
            task(self, app);
        }
        let enabled = self.widget(app).enabled;
        let do_show = self.highlight_state(app, enabled);
        if did.focus != do_show.focus
            && let Some(callback) = self.widget(app).on_show_focus_highlight.clone()
        {
            callback(app, do_show.focus);
        }
        if did.hover != do_show.hover
            && let Some(callback) = self.widget(app).on_show_hover_highlight.clone()
        {
            callback(app, do_show.hover);
        }
    }
}

impl State for FocusableActionDetectorState {
    type Widget = FocusableActionDetector;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        SchedulerBinding::add_post_frame_callback(
            app,
            FrameCallback::new(move |app, _duration| {
                let mode = FocusManager::instance(app).highlight_mode(app);
                self.update_highlight_mode(app, mode);
            }),
        );
        let listener: HighlightModeListener =
            Rc::new(move |app: &mut App, mode| self.handle_focus_highlight_mode_change(app, mode));
        app.get_mut(self).highlight_mode_listener = Some(Rc::clone(&listener));
        FocusManager::instance(app).add_highlight_mode_listener(app, listener);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(listener) = app.get_mut(self).highlight_mode_listener.take() {
            FocusManager::instance(app).remove_highlight_mode_listener(app, &listener);
        }
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &FocusableActionDetector) {
        if self.widget(app).enabled != old_widget.enabled {
            let old_enabled = old_widget.enabled;
            SchedulerBinding::add_post_frame_callback(
                app,
                FrameCallback::new(move |app, _duration| {
                    self.may_trigger_callback(app, Some(old_enabled), None);
                }),
            );
        }
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let can_request_focus = {
            let enabled = self.widget(app).enabled;
            self.can_request_focus(app, enabled)
        };
        let widget = self.widget(app);
        let (
            focus_node,
            autofocus,
            descendants_are_focusable,
            descendants_are_traversable,
            include_focus_semantics,
            mouse_cursor,
            child,
            enabled,
        ) = (
            widget.focus_node,
            widget.autofocus,
            widget.descendants_are_focusable,
            widget.descendants_are_traversable,
            widget.include_focus_semantics,
            Rc::clone(&widget.mouse_cursor),
            widget.child.clone(),
            widget.enabled,
        );
        let mouse_region_key = Rc::clone(&app.get(self).mouse_region_key);

        let mut focus = Focus::new(child)
            .autofocus(autofocus)
            .descendants_are_focusable(descendants_are_focusable)
            .descendants_are_traversable(descendants_are_traversable)
            .can_request_focus(can_request_focus)
            .on_focus_change(move |app, focused| self.handle_focus_change(app, focused))
            .include_semantics(include_focus_semantics);
        if let Some(focus_node) = focus_node {
            focus = focus.focus_node(focus_node);
        }

        let mut child: WidgetRef = MouseRegion::new()
            .key(mouse_region_key)
            .on_enter(Rc::new(move |app, _event| self.handle_mouse_enter(app)))
            .on_exit(Rc::new(move |app, _event| self.handle_mouse_exit(app)))
            .cursor(mouse_cursor)
            .child(focus)
            .into_widget();

        let actions = self.widget(app).actions.clone();
        if let Some(actions) = actions.filter(|actions| enabled && !actions.is_empty()) {
            child = Actions::new(actions, child).into_widget();
        }
        let shortcuts = self.widget(app).shortcuts.clone();
        if let Some(shortcuts) = shortcuts.filter(|shortcuts| enabled && !shortcuts.is_empty()) {
            child = Shortcuts::new(shortcuts, child).into_widget();
        }
        child
    }
}

// ---------------------------------------------------------------------------------------------
// The intents and actions this file declares

/// An [`Intent`] that keeps a callback to be invoked by a [`VoidCallbackAction`] when it
/// receives this intent.
pub struct VoidCallbackIntent {
    /// The callback that is to be called by the [`VoidCallbackAction`] that receives this
    /// intent.
    pub callback: Listener,
}

impl VoidCallbackIntent {
    /// Creates a [`VoidCallbackIntent`].
    pub fn new(callback: Listener) -> VoidCallbackIntent {
        VoidCallbackIntent { callback }
    }
}

impl fmt::Debug for VoidCallbackIntent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VoidCallbackIntent").finish()
    }
}

impl Intent for VoidCallbackIntent {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// An [`Action`] that invokes the callback given to it in the [`VoidCallbackIntent`] passed to
/// it when invoked.
///
/// See also:
///
///  * [`CallbackAction`], which is an action that will invoke a callback with the intent passed
///    to the action's invoke method. The callback is configured on the action, not the intent,
///    like this class.
pub struct VoidCallbackAction {
    action: ActionData,
}

impl VoidCallbackAction {
    /// Creates a [`VoidCallbackAction`].
    pub fn new(app: &mut App) -> Handle<VoidCallbackAction> {
        app.create(VoidCallbackAction {
            action: ActionData::new(),
        })
    }
}

impl Action for VoidCallbackAction {
    type Intent = VoidCallbackIntent;
    crate::action_accessors!();

    fn invoke(
        self: Handle<Self>,
        app: &mut App,
        intent: &VoidCallbackIntent,
    ) -> Option<Rc<dyn Any>> {
        intent.callback.call(app);
        None
    }
}

/// An [`Intent`] that is bound to a [`DoNothingAction`].
///
/// Attaching a [`DoNothingIntent`] to a `Shortcuts` mapping is one way to disable a keyboard
/// shortcut defined by a widget higher in the widget hierarchy and consume any key event that
/// triggers it via a shortcut.
///
/// See also:
///
///  * [`DoNothingAndStopPropagationIntent`], a similar intent that will not handle the key
///    event, but will still keep it from being passed to other key handlers in the focus chain.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DoNothingIntent;

impl DoNothingIntent {
    /// Creates a [`DoNothingIntent`].
    pub const fn new() -> DoNothingIntent {
        DoNothingIntent
    }
}

impl Intent for DoNothingIntent {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// An [`Intent`] that is bound to a [`DoNothingAction`], but, in addition to not performing an
/// action, also stops the propagation of the key event bound to this intent to other key event
/// handlers in the focus chain.
///
/// Attaching a [`DoNothingAndStopPropagationIntent`] to a `Shortcuts::shortcuts` mapping is one
/// way to disable a keyboard shortcut defined by a widget higher in the widget hierarchy. In
/// addition, the bound [`DoNothingAction`] will return false from
/// [`Action::consumes_key`], causing the key bound to this intent to be passed on to the
/// platform embedding as "not handled" without passing it to other key handlers in the focus
/// chain (e.g. parent `Shortcuts` widgets higher up in the chain).
///
/// See also:
///
///  * [`DoNothingIntent`], a similar intent that will handle the key event.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DoNothingAndStopPropagationIntent;

impl DoNothingAndStopPropagationIntent {
    /// Creates a [`DoNothingAndStopPropagationIntent`].
    pub const fn new() -> DoNothingAndStopPropagationIntent {
        DoNothingAndStopPropagationIntent
    }
}

impl Intent for DoNothingAndStopPropagationIntent {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// An [`Action`] that doesn't perform any action when invoked.
///
/// Attaching a [`DoNothingAction`] to an [`Actions::actions`] mapping is a way to disable an
/// action defined by a widget higher in the widget hierarchy.
///
/// If [`Action::consumes_key`] returns false, then not only will this action do nothing, but it
/// will stop the propagation of the key event used to trigger it to other widgets in the focus
/// chain and tell the embedding that the key wasn't handled, allowing text input fields or
/// other non-Flutter elements to receive that key event. The return value of
/// [`Action::consumes_key`] can be set via the `consumes_key` argument to the constructor.
///
/// This action can be bound to any [`Intent`].
///
/// See also:
///
///  * [`DoNothingIntent`], which is an intent that can be bound to a key set in a `Shortcuts`
///    widget to do nothing.
///  * [`DoNothingAndStopPropagationIntent`], which is an intent that can be bound to a key set
///    in a `Shortcuts` widget to do nothing and also stop key event propagation to other key
///    handlers in the focus chain.
pub struct DoNothingAction {
    action: ActionData,
    consumes_key: bool,
}

impl DoNothingAction {
    /// Creates a [`DoNothingAction`].
    ///
    /// Dart's optional `consumesKey` argument defaults to true.
    pub fn new(app: &mut App) -> Handle<DoNothingAction> {
        app.create(DoNothingAction {
            action: ActionData::new(),
            consumes_key: true,
        })
    }

    /// Dart `DoNothingAction(consumesKey:)`.
    pub fn set_consumes_key(self: Handle<Self>, app: &mut App, consumes_key: bool) {
        app.get_mut(self).consumes_key = consumes_key;
    }
}

impl Action for DoNothingAction {
    type Intent = dyn Intent;
    crate::action_accessors!();

    fn consumes_key(self: Handle<Self>, app: &mut App, _intent: &dyn Intent) -> bool {
        app.get(self).consumes_key
    }

    fn invoke(self: Handle<Self>, _app: &mut App, _intent: &dyn Intent) -> Option<Rc<dyn Any>> {
        None
    }
}

/// An [`Intent`] that activates the currently focused control.
///
/// This intent is bound by default to the space key on all platforms, and also to the enter key
/// on all platforms except the web, where enter doesn't toggle selection. On the web, enter is
/// bound to [`ButtonActivateIntent`] instead.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ActivateIntent;

impl ActivateIntent {
    /// Creates an intent that activates the currently focused control.
    pub const fn new() -> ActivateIntent {
        ActivateIntent
    }
}

impl Intent for ActivateIntent {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// An [`Intent`] that activates the currently focused button.
///
/// This intent is bound by default to the enter key on the web, where enter can be used to
/// activate buttons, but not toggle selection. All other platforms bind the enter key to
/// [`ActivateIntent`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ButtonActivateIntent;

impl ButtonActivateIntent {
    /// Creates an intent that activates the currently focused control, if it's a button.
    pub const fn new() -> ButtonActivateIntent {
        ButtonActivateIntent
    }
}

impl Intent for ButtonActivateIntent {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// An [`Action`] that activates the currently focused control.
///
/// This is a trait that serves as a base for actions that activate a control. By default, is
/// bound to the enter, game button A, and space keys in the default keyboard map in
/// `WidgetsApp`.
pub trait ActivateAction: Action<Intent = ActivateIntent> {}

/// An [`Intent`] that selects the currently focused control.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SelectIntent;

impl SelectIntent {
    /// Creates an intent that selects the currently focused control.
    pub const fn new() -> SelectIntent {
        SelectIntent
    }
}

impl Intent for SelectIntent {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// An action that selects the currently focused control.
///
/// This is a trait that serves as a base for actions that select something. It is not bound to
/// any key by default.
pub trait SelectAction: Action<Intent = SelectIntent> {}

/// An [`Intent`] that dismisses the currently focused widget.
///
/// `WidgetsApp::default_shortcuts` binds this intent to the escape and game button B keys.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DismissIntent;

impl DismissIntent {
    /// Creates an intent that dismisses the currently focused widget.
    pub const fn new() -> DismissIntent {
        DismissIntent
    }
}

impl Intent for DismissIntent {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// An [`Action`] that dismisses the focused widget.
///
/// This is a trait that serves as a base for dismiss actions.
pub trait DismissAction: Action<Intent = DismissIntent> {}

/// An [`Intent`] that evaluates a series of specified
/// [`ordered_intents`](Self::ordered_intents) for execution.
///
/// The first intent that matches an enabled action is used.
#[derive(Debug)]
pub struct PrioritizedIntents {
    /// List of intents to be evaluated in order for execution. When an
    /// [`AnyAction::is_enabled`] returns true, that action will be invoked and progression
    /// through the ordered intents stops.
    pub ordered_intents: Vec<IntentRef>,
}

impl PrioritizedIntents {
    /// Creates an intent that is used with [`PrioritizedAction`] to specify a list of intents,
    /// the first available of which will be used.
    pub fn new(ordered_intents: Vec<IntentRef>) -> PrioritizedIntents {
        PrioritizedIntents { ordered_intents }
    }
}

impl Intent for PrioritizedIntents {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// An [`Action`] that iterates through a list of [`Intent`]s, invoking the first that is
/// enabled.
///
/// [`ContextAction::is_enabled`] must be called before [`ContextAction::invoke`]. Calling it
/// configures the object by seeking the first intent with an enabled action. If the actions
/// have an opportunity to change enabled state, it must be called again before calling
/// [`ContextAction::invoke`].
pub struct PrioritizedAction {
    action: ActionData,
    selected_action: Option<AnyAction>,
    selected_intent: Option<IntentRef>,
}

impl PrioritizedAction {
    /// Creates a [`PrioritizedAction`].
    pub fn new(app: &mut App) -> Handle<PrioritizedAction> {
        app.create(PrioritizedAction {
            action: ActionData::new(),
            selected_action: None,
            selected_intent: None,
        })
    }
}

impl Action for PrioritizedAction {
    type Intent = PrioritizedIntents;
    crate::action_accessors!();
    crate::context_action_overrides!();
}

impl ContextAction for PrioritizedAction {
    fn is_enabled(
        self: Handle<Self>,
        app: &mut App,
        intent: &PrioritizedIntents,
        context: Option<BuildContext>,
    ) -> bool {
        let Some(focus) = primary_focus(app) else {
            return false;
        };
        let Some(focus_context) = focus.context(app) else {
            return false;
        };
        for candidate_intent in intent.ordered_intents.iter() {
            let candidate_action =
                Actions::maybe_find(app, focus_context, candidate_intent.intent_type());
            if let Some(candidate_action) = candidate_action
                && candidate_action.is_enabled(app, &**candidate_intent, context)
            {
                let this = app.get_mut(self);
                this.selected_action = Some(candidate_action);
                this.selected_intent = Some(Rc::clone(candidate_intent));
                return true;
            }
        }
        false
    }

    fn invoke(
        self: Handle<Self>,
        app: &mut App,
        _intent: &PrioritizedIntents,
        context: Option<BuildContext>,
    ) -> Option<Rc<dyn Any>> {
        let selected_action = app
            .get(self)
            .selected_action
            .expect("isEnabled selected an action");
        let selected_intent = app
            .get(self)
            .selected_intent
            .clone()
            .expect("isEnabled selected an intent");
        selected_action.invoke(app, &*selected_intent, context);
        None
    }
}

// ---------------------------------------------------------------------------------------------
// The overridable actions

/// The fields Dart's `_OverridableActionMixin` declares; both overridable actions carry this
/// bag under the field `overridable`.
struct OverridableActionData {
    /// The default action to invoke if an enabled override [`Action`] can't be found using
    /// [`lookup_context`](Self::lookup_context).
    default_action: AnyAction,
    /// The [`BuildContext`] used to find the override of this [`Action`].
    lookup_context: BuildContext,
    // When these are true, this action throws when the override calls this action's method and
    // the override is already being invoked from within it.
    debug_assert_mutually_recursive: bool,
    debug_assert_is_action_enabled_mutually_recursive: bool,
    debug_assert_is_enabled_mutually_recursive: bool,
    debug_assert_consume_key_mutually_recursive: bool,
}

impl OverridableActionData {
    fn new(default_action: AnyAction, lookup_context: BuildContext) -> OverridableActionData {
        OverridableActionData {
            default_action,
            lookup_context,
            debug_assert_mutually_recursive: false,
            debug_assert_is_action_enabled_mutually_recursive: false,
            debug_assert_is_enabled_mutually_recursive: false,
            debug_assert_consume_key_mutually_recursive: false,
        }
    }
}

/// Dart's `_OverridableActionMixin`: the shared bodies of the two overridable actions.
trait OverridableActionMixin: ContextAction {
    /// Dart's `_OverridableActionMixin` fields, held under the field `overridable`.
    fn overridable_data(self: Handle<Self>, app: &App) -> &OverridableActionData;

    /// See [`overridable_data`](Self::overridable_data).
    fn overridable_data_mut(self: Handle<Self>, app: &mut App) -> &mut OverridableActionData;

    /// How to invoke the default action, given the caller `from_action`.
    fn invoke_default_action(
        self: Handle<Self>,
        app: &mut App,
        intent: &Self::Intent,
        from_action: Option<AnyAction>,
        context: Option<BuildContext>,
    ) -> Option<Rc<dyn Any>>;

    /// Dart's `_getOverrideAction`.
    fn get_override_action(
        self: Handle<Self>,
        app: &mut App,
        intent_type: TypeId,
        declare_dependency: bool,
    ) -> Option<AnyAction> {
        let lookup_context = self.overridable_data(app).lookup_context;
        let override_action = if declare_dependency {
            Actions::maybe_find(app, lookup_context, intent_type)
        } else {
            Actions::maybe_find_without_depending_on(app, lookup_context, intent_type)
        };
        debug_assert!(override_action != Some(Action::as_action(self)));
        override_action
    }

    /// Dart's `_OverridableActionMixin._updateCallingAction`.
    fn update_calling_action(self: Handle<Self>, app: &mut App, value: Option<AnyAction>) {
        self.action_data_mut(app).current_calling_action = value;
        let default_action = self.overridable_data(app).default_action;
        default_action.update_calling_action(app, value);
    }

    /// Dart's `_OverridableActionMixin._invokeOverride`.
    fn invoke_override(
        self: Handle<Self>,
        app: &mut App,
        override_action: AnyAction,
        intent: &Self::Intent,
        context: Option<BuildContext>,
    ) -> Option<Rc<dyn Any>> {
        debug_assert!(!self.overridable_data(app).debug_assert_mutually_recursive);
        if cfg!(debug_assertions) {
            self.overridable_data_mut(app)
                .debug_assert_mutually_recursive = true;
        }
        let default_action = self.overridable_data(app).default_action;
        override_action.update_calling_action(app, Some(default_action));
        let return_value = override_action.invoke(app, intent.as_intent(), context);
        override_action.update_calling_action(app, None);
        if cfg!(debug_assertions) {
            self.overridable_data_mut(app)
                .debug_assert_mutually_recursive = false;
        }
        return_value
    }

    /// Dart's `_OverridableActionMixin.invoke`.
    fn invoke(
        self: Handle<Self>,
        app: &mut App,
        intent: &Self::Intent,
        context: Option<BuildContext>,
    ) -> Option<Rc<dyn Any>> {
        let intent_type = intent.as_intent().intent_type();
        let override_action = self.get_override_action(app, intent_type, false);
        match override_action {
            None => {
                let calling = self.action_data(app).current_calling_action;
                self.invoke_default_action(app, intent, calling, context)
            }
            Some(override_action) => self.invoke_override(app, override_action, intent, context),
        }
    }

    /// Dart's `_OverridableActionMixin._isOverrideActionEnabled`.
    fn is_override_action_enabled(
        self: Handle<Self>,
        app: &mut App,
        override_action: AnyAction,
    ) -> bool {
        debug_assert!(
            !self
                .overridable_data(app)
                .debug_assert_is_action_enabled_mutually_recursive
        );
        if cfg!(debug_assertions) {
            self.overridable_data_mut(app)
                .debug_assert_is_action_enabled_mutually_recursive = true;
        }
        let default_action = self.overridable_data(app).default_action;
        override_action.update_calling_action(app, Some(default_action));
        let is_override_enabled = override_action.is_action_enabled(app);
        override_action.update_calling_action(app, None);
        if cfg!(debug_assertions) {
            self.overridable_data_mut(app)
                .debug_assert_is_action_enabled_mutually_recursive = false;
        }
        is_override_enabled
    }

    /// Dart's `_OverridableActionMixin.isActionEnabled`.
    fn is_action_enabled(self: Handle<Self>, app: &mut App) -> bool {
        let intent_type = TypeId::of::<Self::Intent>();
        let override_action = self.get_override_action(app, intent_type, true);
        match override_action {
            Some(override_action) => self.is_override_action_enabled(app, override_action),
            None => self
                .overridable_data(app)
                .default_action
                .is_action_enabled(app),
        }
    }

    /// Dart's `_OverridableActionMixin.isEnabled`.
    fn is_enabled(
        self: Handle<Self>,
        app: &mut App,
        intent: &Self::Intent,
        context: Option<BuildContext>,
    ) -> bool {
        debug_assert!(
            !self
                .overridable_data(app)
                .debug_assert_is_enabled_mutually_recursive
        );
        if cfg!(debug_assertions) {
            self.overridable_data_mut(app)
                .debug_assert_is_enabled_mutually_recursive = true;
        }

        let erased = intent.as_intent();
        let override_action = self.get_override_action(app, erased.intent_type(), false);
        let default_action = self.overridable_data(app).default_action;
        if let Some(override_action) = override_action {
            override_action.update_calling_action(app, Some(default_action));
        }
        let return_value =
            override_action
                .unwrap_or(default_action)
                .is_enabled(app, intent.as_intent(), context);
        if let Some(override_action) = override_action {
            override_action.update_calling_action(app, None);
        }
        if cfg!(debug_assertions) {
            self.overridable_data_mut(app)
                .debug_assert_is_enabled_mutually_recursive = false;
        }
        return_value
    }

    /// Dart's `_OverridableActionMixin.consumesKey`.
    fn consumes_key(self: Handle<Self>, app: &mut App, intent: &Self::Intent) -> bool {
        debug_assert!(
            !self
                .overridable_data(app)
                .debug_assert_consume_key_mutually_recursive
        );
        if cfg!(debug_assertions) {
            self.overridable_data_mut(app)
                .debug_assert_consume_key_mutually_recursive = true;
        }
        let erased = intent.as_intent();
        let override_action = self.get_override_action(app, erased.intent_type(), false);
        let default_action = self.overridable_data(app).default_action;
        if let Some(override_action) = override_action {
            override_action.update_calling_action(app, Some(default_action));
        }
        let is_enabled = override_action
            .unwrap_or(default_action)
            .consumes_key(app, intent.as_intent());
        if let Some(override_action) = override_action {
            override_action.update_calling_action(app, None);
        }
        if cfg!(debug_assertions) {
            self.overridable_data_mut(app)
                .debug_assert_consume_key_mutually_recursive = false;
        }
        is_enabled
    }
}

/// Dart's `_OverridableAction`: the overridable form of a plain [`Action`].
pub struct OverridableAction<T: ActionIntent + ?Sized> {
    action: ActionData,
    overridable: OverridableActionData,
    intent: std::marker::PhantomData<fn(&T)>,
}

impl<T: ActionIntent + ?Sized> OverridableAction<T> {
    /// Dart's `_OverridableAction(defaultAction:, lookupContext:)`; reached through
    /// [`AnyAction::overridable`].
    pub fn new(
        app: &mut App,
        default_action: AnyAction,
        lookup_context: BuildContext,
    ) -> Handle<OverridableAction<T>> {
        app.create(OverridableAction::<T> {
            action: ActionData::new(),
            overridable: OverridableActionData::new(default_action, lookup_context),
            intent: std::marker::PhantomData,
        })
    }
}

impl<T: ActionIntent + ?Sized> Action for OverridableAction<T> {
    type Intent = T;
    crate::action_accessors!();

    fn as_action(self: Handle<Self>) -> AnyAction {
        AnyAction::of_context::<Self>(self)
    }

    fn is_enabled(self: Handle<Self>, app: &mut App, intent: &T) -> bool {
        ContextAction::is_enabled(self, app, intent, None)
    }

    fn invoke(self: Handle<Self>, app: &mut App, intent: &T) -> Option<Rc<dyn Any>> {
        ContextAction::invoke(self, app, intent, None)
    }

    fn make_overridable_action(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
    ) -> AnyAction {
        let default_action = self.overridable_data(app).default_action;
        let overridable = OverridableAction::<T>::new(app, default_action, context);
        Action::as_action(overridable)
    }

    fn update_calling_action(self: Handle<Self>, app: &mut App, value: Option<AnyAction>) {
        OverridableActionMixin::update_calling_action(self, app, value);
    }

    fn is_action_enabled(self: Handle<Self>, app: &mut App) -> bool {
        OverridableActionMixin::is_action_enabled(self, app)
    }

    fn consumes_key(self: Handle<Self>, app: &mut App, intent: &T) -> bool {
        OverridableActionMixin::consumes_key(self, app, intent)
    }
}

impl<T: ActionIntent + ?Sized> ContextAction for OverridableAction<T> {
    fn is_enabled(
        self: Handle<Self>,
        app: &mut App,
        intent: &T,
        context: Option<BuildContext>,
    ) -> bool {
        OverridableActionMixin::is_enabled(self, app, intent, context)
    }

    fn invoke(
        self: Handle<Self>,
        app: &mut App,
        intent: &T,
        context: Option<BuildContext>,
    ) -> Option<Rc<dyn Any>> {
        OverridableActionMixin::invoke(self, app, intent, context)
    }
}

impl<T: ActionIntent + ?Sized> OverridableActionMixin for OverridableAction<T> {
    fn overridable_data(self: Handle<Self>, app: &App) -> &OverridableActionData {
        &app.get(self).overridable
    }

    fn overridable_data_mut(self: Handle<Self>, app: &mut App) -> &mut OverridableActionData {
        &mut app.get_mut(self).overridable
    }

    fn invoke_default_action(
        self: Handle<Self>,
        app: &mut App,
        intent: &T,
        _from_action: Option<AnyAction>,
        _context: Option<BuildContext>,
    ) -> Option<Rc<dyn Any>> {
        let default_action = self.overridable_data(app).default_action;
        default_action.invoke(app, intent.as_intent(), None)
    }
}

/// Dart's `_OverridableContextAction`: the overridable form of a [`ContextAction`].
pub struct OverridableContextAction<T: ActionIntent + ?Sized> {
    action: ActionData,
    overridable: OverridableActionData,
    intent: std::marker::PhantomData<fn(&T)>,
}

impl<T: ActionIntent + ?Sized> OverridableContextAction<T> {
    /// Dart's `_OverridableContextAction(defaultAction:, lookupContext:)`; reached through
    /// [`AnyAction::overridable`].
    pub fn new(
        app: &mut App,
        default_action: AnyAction,
        lookup_context: BuildContext,
    ) -> Handle<OverridableContextAction<T>> {
        app.create(OverridableContextAction::<T> {
            action: ActionData::new(),
            overridable: OverridableActionData::new(default_action, lookup_context),
            intent: std::marker::PhantomData,
        })
    }
}

impl<T: ActionIntent + ?Sized> Action for OverridableContextAction<T> {
    type Intent = T;
    crate::action_accessors!();

    fn as_action(self: Handle<Self>) -> AnyAction {
        AnyAction::of_context::<Self>(self)
    }

    fn is_enabled(self: Handle<Self>, app: &mut App, intent: &T) -> bool {
        ContextAction::is_enabled(self, app, intent, None)
    }

    fn invoke(self: Handle<Self>, app: &mut App, intent: &T) -> Option<Rc<dyn Any>> {
        ContextAction::invoke(self, app, intent, None)
    }

    fn make_overridable_action(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
    ) -> AnyAction {
        let default_action = self.overridable_data(app).default_action;
        let overridable = OverridableContextAction::<T>::new(app, default_action, context);
        Action::as_action(overridable)
    }

    fn update_calling_action(self: Handle<Self>, app: &mut App, value: Option<AnyAction>) {
        OverridableActionMixin::update_calling_action(self, app, value);
    }

    fn is_action_enabled(self: Handle<Self>, app: &mut App) -> bool {
        OverridableActionMixin::is_action_enabled(self, app)
    }

    fn consumes_key(self: Handle<Self>, app: &mut App, intent: &T) -> bool {
        OverridableActionMixin::consumes_key(self, app, intent)
    }
}

impl<T: ActionIntent + ?Sized> ContextAction for OverridableContextAction<T> {
    fn is_enabled(
        self: Handle<Self>,
        app: &mut App,
        intent: &T,
        context: Option<BuildContext>,
    ) -> bool {
        OverridableActionMixin::is_enabled(self, app, intent, context)
    }

    fn invoke(
        self: Handle<Self>,
        app: &mut App,
        intent: &T,
        context: Option<BuildContext>,
    ) -> Option<Rc<dyn Any>> {
        OverridableActionMixin::invoke(self, app, intent, context)
    }
}

impl<T: ActionIntent + ?Sized> OverridableActionMixin for OverridableContextAction<T> {
    fn overridable_data(self: Handle<Self>, app: &App) -> &OverridableActionData {
        &app.get(self).overridable
    }

    fn overridable_data_mut(self: Handle<Self>, app: &mut App) -> &mut OverridableActionData {
        &mut app.get_mut(self).overridable
    }

    /// Wraps the default [`Action`] together with the calling context in case the override is
    /// not a [`ContextAction`] and thus has no access to the calling [`BuildContext`].
    fn invoke_override(
        self: Handle<Self>,
        app: &mut App,
        override_action: AnyAction,
        intent: &T,
        context: Option<BuildContext>,
    ) -> Option<Rc<dyn Any>> {
        debug_assert!(context.is_some());
        debug_assert!(!self.overridable_data(app).debug_assert_mutually_recursive);
        if cfg!(debug_assertions) {
            self.overridable_data_mut(app)
                .debug_assert_mutually_recursive = true;
        }

        let default_action = self.overridable_data(app).default_action;
        let invoke_context = context.expect("an overridable context action is invoked with one");
        let wrapped_default =
            ContextActionToActionAdapter::<T>::new(app, invoke_context, default_action);
        override_action.update_calling_action(app, Some(Action::as_action(wrapped_default)));
        let return_value = override_action.invoke(app, intent.as_intent(), context);
        override_action.update_calling_action(app, None);

        if cfg!(debug_assertions) {
            self.overridable_data_mut(app)
                .debug_assert_mutually_recursive = false;
        }
        return_value
    }

    fn invoke_default_action(
        self: Handle<Self>,
        app: &mut App,
        intent: &T,
        _from_action: Option<AnyAction>,
        context: Option<BuildContext>,
    ) -> Option<Rc<dyn Any>> {
        let default_action = self.overridable_data(app).default_action;
        default_action.invoke(app, intent.as_intent(), context)
    }
}

/// Dart's `_ContextActionToActionAdapter`: a plain [`Action`] that calls a [`ContextAction`]
/// with a fixed context.
struct ContextActionToActionAdapter<T: ActionIntent + ?Sized> {
    action: ActionData,
    invoke_context: BuildContext,
    wrapped: AnyAction,
    intent: std::marker::PhantomData<fn(&T)>,
}

impl<T: ActionIntent + ?Sized> ContextActionToActionAdapter<T> {
    fn new(
        app: &mut App,
        invoke_context: BuildContext,
        wrapped: AnyAction,
    ) -> Handle<ContextActionToActionAdapter<T>> {
        app.create(ContextActionToActionAdapter::<T> {
            action: ActionData::new(),
            invoke_context,
            wrapped,
            intent: std::marker::PhantomData,
        })
    }
}

impl<T: ActionIntent + ?Sized> Action for ContextActionToActionAdapter<T> {
    type Intent = T;
    crate::action_accessors!();

    fn update_calling_action(self: Handle<Self>, app: &mut App, value: Option<AnyAction>) {
        let wrapped = app.get(self).wrapped;
        wrapped.update_calling_action(app, value);
    }

    fn calling_action(self: Handle<Self>, app: &App) -> Option<AnyAction> {
        app.get(self).wrapped.calling_action(app)
    }

    fn is_enabled(self: Handle<Self>, app: &mut App, intent: &T) -> bool {
        let (wrapped, invoke_context) = {
            let this = app.get(self);
            (this.wrapped, this.invoke_context)
        };
        wrapped.is_enabled(app, intent.as_intent(), Some(invoke_context))
    }

    fn is_action_enabled(self: Handle<Self>, app: &mut App) -> bool {
        app.get(self).wrapped.is_action_enabled(app)
    }

    fn consumes_key(self: Handle<Self>, app: &mut App, intent: &T) -> bool {
        let wrapped = app.get(self).wrapped;
        wrapped.consumes_key(app, intent.as_intent())
    }

    fn add_action_listener(self: Handle<Self>, app: &mut App, listener: ActionListenerCallback) {
        self.action_data_mut(app)
            .listeners
            .push(Rc::clone(&listener));
        let wrapped = app.get(self).wrapped;
        wrapped.add_action_listener(app, listener);
    }

    fn remove_action_listener(
        self: Handle<Self>,
        app: &mut App,
        listener: &ActionListenerCallback,
    ) {
        let listeners = &mut self.action_data_mut(app).listeners;
        if let Some(index) = listeners
            .iter()
            .position(|registered| Rc::ptr_eq(registered, listener))
        {
            listeners.remove(index);
        }
        let wrapped = app.get(self).wrapped;
        wrapped.remove_action_listener(app, listener);
    }

    fn notify_action_listeners(self: Handle<Self>, app: &mut App) {
        app.get(self).wrapped.notify_action_listeners(app);
    }

    fn invoke(self: Handle<Self>, app: &mut App, intent: &T) -> Option<Rc<dyn Any>> {
        let (wrapped, invoke_context) = {
            let this = app.get(self);
            (this.wrapped, this.invoke_context)
        };
        wrapped.invoke(app, intent.as_intent(), Some(invoke_context))
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use inset_embedder::{Offset, PointerDeviceKind};
    use inset_gestures::{GestureBinding, PointerEvent, PointerHoverEvent};

    use super::*;
    use crate::widgets::basic::{Builder, SizedBox};
    use crate::widgets::focus_manager::tests::{app_with_view, mount, pump_frame};
    use crate::widgets::focus_manager::{FocusHighlightStrategy, FocusNodeLeaf};

    #[derive(Debug)]
    struct CountIntent {
        amount: i32,
    }

    impl Intent for CountIntent {
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    #[derive(Debug)]
    struct OtherIntent;

    impl Intent for OtherIntent {
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    /// An action that records that it ran and doubles the intent's amount.
    struct CountAction {
        action: ActionData,
        log: Rc<RefCell<Vec<String>>>,
        name: &'static str,
        enabled: bool,
    }

    impl CountAction {
        fn new(
            app: &mut App,
            log: &Rc<RefCell<Vec<String>>>,
            name: &'static str,
        ) -> Handle<CountAction> {
            app.create(CountAction {
                action: ActionData::new(),
                log: Rc::clone(log),
                name,
                enabled: true,
            })
        }

        fn set_enabled(self: Handle<Self>, app: &mut App, enabled: bool) {
            app.get_mut(self).enabled = enabled;
            self.notify_action_listeners(app);
        }
    }

    impl Action for CountAction {
        type Intent = CountIntent;
        crate::action_accessors!();

        fn is_action_enabled(self: Handle<Self>, app: &mut App) -> bool {
            app.get(self).enabled
        }

        fn invoke(self: Handle<Self>, app: &mut App, intent: &CountIntent) -> Option<Rc<dyn Any>> {
            let (log, name) = {
                let this = app.get(self);
                (Rc::clone(&this.log), this.name)
            };
            log.borrow_mut().push(format!("{name}:{}", intent.amount));
            Some(Rc::new(intent.amount * 2))
        }
    }

    fn map(intent_type: TypeId, action: AnyAction) -> HashMap<TypeId, AnyAction> {
        HashMap::from([(intent_type, action)])
    }

    /// A `Builder` that records its own context.
    fn probe(sink: &Rc<Cell<Option<BuildContext>>>) -> WidgetRef {
        let sink = Rc::clone(sink);
        Builder::new(move |_app, context| {
            sink.set(Some(context));
            SizedBox::shrink().into_widget()
        })
        .into_widget()
    }

    fn as_i32(result: Option<Rc<dyn Any>>) -> Option<i32> {
        result.and_then(|value| value.downcast_ref::<i32>().copied())
    }

    #[test]
    fn invoke_reaches_the_nearest_actions_widget_that_maps_the_intent() {
        let cell = app_with_view();
        let mut app = cell.borrow_mut();
        let log: Rc<RefCell<Vec<String>>> = Rc::default();
        let outer = CountAction::new(&mut app, &log, "outer");
        let inner = CountAction::new(&mut app, &log, "inner");
        let captured: Rc<Cell<Option<BuildContext>>> = Rc::default();
        drop(app);
        mount(
            &cell,
            Actions::new(
                map(TypeId::of::<CountIntent>(), Action::as_action(outer)),
                Actions::new(
                    map(TypeId::of::<CountIntent>(), Action::as_action(inner)),
                    probe(&captured),
                ),
            )
            .into_widget(),
        );
        let mut app = cell.borrow_mut();
        let context = captured.get().expect("the builder ran");

        let result = Actions::invoke(&mut app, context, &CountIntent { amount: 2 });
        assert_eq!(*log.borrow(), ["inner:2"]);
        assert_eq!(as_i32(result), Some(4));

        // A disabled action still stops the search, so nothing is invoked.
        inner.set_enabled(&mut app, false);
        log.borrow_mut().clear();
        let result = Actions::maybe_invoke(&mut app, context, &CountIntent { amount: 3 });
        assert!(log.borrow().is_empty());
        assert_eq!(as_i32(result), None);
    }

    #[test]
    fn invoke_walks_past_an_actions_widget_that_does_not_map_the_intent() {
        let cell = app_with_view();
        let mut app = cell.borrow_mut();
        let log: Rc<RefCell<Vec<String>>> = Rc::default();
        let outer = CountAction::new(&mut app, &log, "outer");
        let unrelated = DoNothingAction::new(&mut app);
        let captured: Rc<Cell<Option<BuildContext>>> = Rc::default();
        drop(app);
        mount(
            &cell,
            Actions::new(
                map(TypeId::of::<CountIntent>(), Action::as_action(outer)),
                Actions::new(
                    map(TypeId::of::<OtherIntent>(), Action::as_action(unrelated)),
                    probe(&captured),
                ),
            )
            .into_widget(),
        );
        let mut app = cell.borrow_mut();
        let context = captured.get().expect("the builder ran");

        let result = Actions::invoke(&mut app, context, &CountIntent { amount: 5 });
        assert_eq!(*log.borrow(), ["outer:5"]);
        assert_eq!(as_i32(result), Some(10));
    }

    #[test]
    fn maybe_find_returns_the_bound_action_and_handler_only_when_it_is_enabled() {
        let cell = app_with_view();
        let mut app = cell.borrow_mut();
        let log: Rc<RefCell<Vec<String>>> = Rc::default();
        let action = CountAction::new(&mut app, &log, "action");
        let captured: Rc<Cell<Option<BuildContext>>> = Rc::default();
        drop(app);
        mount(
            &cell,
            Actions::new(
                map(TypeId::of::<CountIntent>(), Action::as_action(action)),
                probe(&captured),
            )
            .into_widget(),
        );
        let mut app = cell.borrow_mut();
        let context = captured.get().expect("the builder ran");

        assert_eq!(
            Actions::maybe_find(&mut app, context, TypeId::of::<CountIntent>()),
            Some(Action::as_action(action))
        );
        assert_eq!(
            Actions::maybe_find(&mut app, context, TypeId::of::<OtherIntent>()),
            None
        );

        let intent: IntentRef = Rc::new(CountIntent { amount: 7 });
        let handler = Actions::handler(&mut app, context, Rc::clone(&intent))
            .expect("an enabled action has a handler");
        handler.call(&mut app);
        assert_eq!(*log.borrow(), ["action:7"]);

        action.set_enabled(&mut app, false);
        assert!(Actions::handler(&mut app, context, intent).is_none());
    }

    #[test]
    fn an_overridable_action_prefers_the_override_above_its_lookup_context() {
        let cell = app_with_view();
        let mut app = cell.borrow_mut();
        let log: Rc<RefCell<Vec<String>>> = Rc::default();
        let override_action = CountAction::new(&mut app, &log, "override");
        let default_action = CountAction::new(&mut app, &log, "default");
        let outer_context: Rc<Cell<Option<BuildContext>>> = Rc::default();
        let inner_context: Rc<Cell<Option<BuildContext>>> = Rc::default();
        let outer_sink = Rc::clone(&outer_context);
        let inner_sink = Rc::clone(&inner_context);
        drop(app);
        mount(
            &cell,
            Actions::new(
                map(
                    TypeId::of::<CountIntent>(),
                    Action::as_action(override_action),
                ),
                Builder::new(move |_app, context| {
                    outer_sink.set(Some(context));
                    probe(&inner_sink)
                }),
            )
            .into_widget(),
        );
        let mut app = cell.borrow_mut();
        let lookup_context = outer_context.get().expect("the builder ran");

        let overridable =
            AnyAction::overridable(&mut app, Action::as_action(default_action), lookup_context);
        let intent = CountIntent { amount: 1 };
        overridable.invoke(&mut app, &intent, Some(lookup_context));
        assert_eq!(*log.borrow(), ["override:1"]);

        // With no override above the lookup context, the default action runs.
        let cell = app_with_view();
        let mut app = cell.borrow_mut();
        let log: Rc<RefCell<Vec<String>>> = Rc::default();
        let default_action = CountAction::new(&mut app, &log, "default");
        let captured: Rc<Cell<Option<BuildContext>>> = Rc::default();
        drop(app);
        mount(&cell, probe(&captured));
        let mut app = cell.borrow_mut();
        let lookup_context = captured.get().expect("the builder ran");
        let overridable =
            AnyAction::overridable(&mut app, Action::as_action(default_action), lookup_context);
        overridable.invoke(&mut app, &intent, Some(lookup_context));
        assert_eq!(*log.borrow(), ["default:1"]);
    }

    #[test]
    fn the_detector_reports_the_focus_highlight() {
        let cell = app_with_view();
        let mut app = cell.borrow_mut();
        FocusManager::instance(&mut app)
            .set_highlight_strategy(&mut app, FocusHighlightStrategy::AlwaysTraditional);
        let focus_highlights: Rc<RefCell<Vec<bool>>> = Rc::default();
        let focused: Rc<RefCell<Vec<bool>>> = Rc::default();
        let highlight_sink = Rc::clone(&focus_highlights);
        let focus_sink = Rc::clone(&focused);
        drop(app);
        mount(
            &cell,
            FocusableActionDetector::new(SizedBox::expand())
                .autofocus(true)
                .on_show_focus_highlight(move |_app, show| highlight_sink.borrow_mut().push(show))
                .on_focus_change(move |_app, has_focus| focus_sink.borrow_mut().push(has_focus))
                .into_widget(),
        );
        let mut app = cell.borrow_mut();
        // The post-frame callback `init_state` registered decides whether a highlight may show.
        pump_frame(&mut app);

        assert_eq!(*focused.borrow(), [true]);
        assert_eq!(*focus_highlights.borrow(), [true]);
    }

    #[test]
    fn the_detector_reports_the_hover_highlight() {
        let cell = app_with_view();
        let mut app = cell.borrow_mut();
        FocusManager::instance(&mut app)
            .set_highlight_strategy(&mut app, FocusHighlightStrategy::AlwaysTraditional);
        let hover_highlights: Rc<RefCell<Vec<bool>>> = Rc::default();
        let sink = Rc::clone(&hover_highlights);
        drop(app);
        mount(
            &cell,
            FocusableActionDetector::new(SizedBox::expand())
                .on_show_hover_highlight(move |_app, show| sink.borrow_mut().push(show))
                .into_widget(),
        );
        let mut app = cell.borrow_mut();
        pump_frame(&mut app);

        let hover = |position: Offset| {
            PointerEvent::Hover(PointerHoverEvent {
                position,
                kind: PointerDeviceKind::Mouse,
                ..PointerHoverEvent::default()
            })
        };
        let binding = GestureBinding::instance(&mut app);
        binding.handle_pointer_event(&mut app, hover(Offset::new(10.0, 10.0)));
        assert_eq!(*hover_highlights.borrow(), [true]);

        binding.handle_pointer_event(&mut app, hover(Offset::new(10_000.0, 10_000.0)));
        assert_eq!(*hover_highlights.borrow(), [true, false]);
    }

    #[test]
    fn a_disabled_detector_shows_no_highlight() {
        let cell = app_with_view();
        let mut app = cell.borrow_mut();
        FocusManager::instance(&mut app)
            .set_highlight_strategy(&mut app, FocusHighlightStrategy::AlwaysTraditional);
        let focus_highlights: Rc<RefCell<Vec<bool>>> = Rc::default();
        let sink = Rc::clone(&focus_highlights);
        let node = crate::FocusNode::new(&mut app);
        let detector = |enabled: bool, sink: Rc<RefCell<Vec<bool>>>| {
            FocusableActionDetector::new(SizedBox::expand())
                .enabled(enabled)
                .focus_node(node.as_node())
                .autofocus(true)
                .on_show_focus_highlight(move |_app, show| sink.borrow_mut().push(show))
                .into_widget()
        };
        drop(app);
        mount(&cell, detector(false, Rc::clone(&sink)));
        let mut app = cell.borrow_mut();
        pump_frame(&mut app);
        // A disabled detector sets `Focus.canRequestFocus` to false, so nothing is focused.
        node.request_focus(&mut app, None);
        app.drain_microtasks();
        assert!(!node.has_primary_focus(&app));
        assert!(focus_highlights.borrow().is_empty());

        drop(app);
        mount(&cell, detector(true, Rc::clone(&sink)));
        let mut app = cell.borrow_mut();
        pump_frame(&mut app);
        node.request_focus(&mut app, None);
        app.drain_microtasks();
        assert!(node.has_primary_focus(&app));
        assert_eq!(*focus_highlights.borrow(), [true]);
    }
}
