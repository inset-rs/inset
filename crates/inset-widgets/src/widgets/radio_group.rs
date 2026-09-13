//! Flutter counterpart: `widgets/radio_group.dart`.
//!
//! A [`RadioGroup`] treats every [`RadioClient`] of the same value type below it as one group:
//! it holds the group value, hands a selection change back through
//! [`RadioGroup::on_changed`], and moves the selection with the arrow keys.

use std::fmt::{self, Debug};
use std::rc::Rc;

use inset_foundation::{
    App, ChangeNotifier, ChangeNotifierData, Handle, HandleId, Listener, ValueChanged,
};
use inset_services::{KeyEvent, LogicalKeyboardKey};

use crate::framework::{
    BuildContext, InheritedWidget, IntoWidget, KeyRef, State, StateData, StatefulWidget, WidgetRef,
};
use crate::widgets::actions::{IntentRef, VoidCallbackIntent};
use crate::widgets::focus_manager::{AnyFocusNode, KeyEventResult};
use crate::widgets::focus_traversal::{
    DirectionalFocusTraversalPolicyMixin, DirectionalFocusTraversalPolicyMixinData,
    FocusTraversalGroup, FocusTraversalPolicy, FocusTraversalPolicyData,
    ReadingOrderTraversalPolicy,
};
use crate::widgets::shortcuts::{
    ShortcutActivatorRef, ShortcutManagerBase, ShortcutManagerData, ShortcutManagerKeypress,
    ShortcutMap, Shortcuts, SingleActivator,
};

// ---------------------------------------------------------------------------------------------
// RadioGroupRegistry

/// The vtable of an erased [`AnyRadioGroupRegistry`]: one `&'static` table per leaf type.
struct RadioGroupRegistryVTable<T: Copy + PartialEq + 'static> {
    type_name: fn() -> &'static str,
    group_value: fn(&App, HandleId) -> Option<T>,
    register_client: fn(&mut App, HandleId, AnyRadioClient<T>),
    unregister_client: fn(&mut App, HandleId, AnyRadioClient<T>),
    on_changed: fn(&mut App, HandleId, Option<T>),
}

/// The typed handle for an erased id. Free: nothing is looked up; `get` checks the slot.
fn resolve<L: 'static>(id: HandleId) -> Handle<L> {
    Handle::from_id(id)
}

impl<T: Copy + PartialEq + 'static> RadioGroupRegistryVTable<T> {
    const fn of<L: RadioGroupRegistry<T>>() -> RadioGroupRegistryVTable<T> {
        RadioGroupRegistryVTable {
            type_name: std::any::type_name::<L>,
            group_value: |app, id| L::group_value(resolve(id), app),
            register_client: |app, id, radio| L::register_client(resolve(id), app, radio),
            unregister_client: |app, id, radio| L::unregister_client(resolve(id), app, radio),
            on_changed: |app, id, value| L::on_changed(resolve(id), app, value),
        }
    }
}

/// An abstract interface for registering a group of radios.
///
/// Use [`register_client`](Self::register_client) or
/// [`unregister_client`](Self::unregister_client) to handle registrations of radios.
///
/// The registry manages the group value for the radios. The radio needs to call
/// [`on_changed`](Self::on_changed) to notify the group value needs to be changed.
pub trait RadioGroupRegistry<T: Copy + PartialEq + 'static>: Sized + 'static {
    /// This registry as the erased [`AnyRadioGroupRegistry`] — what to pass where a Dart API
    /// takes a `RadioGroupRegistry<T>`.
    fn as_radio_group_registry(self: Handle<Self>) -> AnyRadioGroupRegistry<T> {
        AnyRadioGroupRegistry {
            id: self.id(),
            vtable: const { &RadioGroupRegistryVTable::<T>::of::<Self>() },
        }
    }

    /// The group value for the group.
    fn group_value(self: Handle<Self>, app: &App) -> Option<T>;

    /// Registers a radio client.
    ///
    /// The implementor provides additional features, such as keyboard navigation for the
    /// registered clients.
    fn register_client(self: Handle<Self>, app: &mut App, radio: AnyRadioClient<T>);

    /// Unregisters a radio client.
    fn unregister_client(self: Handle<Self>, app: &mut App, radio: AnyRadioClient<T>);

    /// Notifies the registry that a radio is selected or unselected.
    fn on_changed(self: Handle<Self>, app: &mut App, value: Option<T>);
}

/// Erased `RadioGroupRegistry<T>`: one identity and a static vtable.
///
/// This is what a field or parameter Dart types as `RadioGroupRegistry<T>` becomes.
pub struct AnyRadioGroupRegistry<T: Copy + PartialEq + 'static> {
    id: HandleId,
    vtable: &'static RadioGroupRegistryVTable<T>,
}

impl<T: Copy + PartialEq + 'static> Clone for AnyRadioGroupRegistry<T> {
    fn clone(&self) -> AnyRadioGroupRegistry<T> {
        *self
    }
}

impl<T: Copy + PartialEq + 'static> Copy for AnyRadioGroupRegistry<T> {}

impl<T: Copy + PartialEq + 'static> PartialEq for AnyRadioGroupRegistry<T> {
    fn eq(&self, other: &AnyRadioGroupRegistry<T>) -> bool {
        self.id == other.id
    }
}

impl<T: Copy + PartialEq + 'static> Eq for AnyRadioGroupRegistry<T> {}

impl<T: Copy + PartialEq + 'static> Debug for AnyRadioGroupRegistry<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({:?})", (self.vtable.type_name)(), self.id)
    }
}

impl<T: Copy + PartialEq + 'static> AnyRadioGroupRegistry<T> {
    /// The arena id behind this handle.
    pub fn id(self) -> HandleId {
        self.id
    }

    /// See [`RadioGroupRegistry::group_value`].
    pub fn group_value(self, app: &App) -> Option<T> {
        (self.vtable.group_value)(app, self.id)
    }

    /// See [`RadioGroupRegistry::register_client`].
    pub fn register_client(self, app: &mut App, radio: AnyRadioClient<T>) {
        (self.vtable.register_client)(app, self.id, radio);
    }

    /// See [`RadioGroupRegistry::unregister_client`].
    pub fn unregister_client(self, app: &mut App, radio: AnyRadioClient<T>) {
        (self.vtable.unregister_client)(app, self.id, radio);
    }

    /// See [`RadioGroupRegistry::on_changed`].
    pub fn on_changed(self, app: &mut App, value: Option<T>) {
        (self.vtable.on_changed)(app, self.id, value);
    }
}

// ---------------------------------------------------------------------------------------------
// RadioClient

/// The vtable of an erased [`AnyRadioClient`]: one `&'static` table per leaf type.
struct RadioClientVTable<T: Copy + PartialEq + 'static> {
    type_name: fn() -> &'static str,
    tristate: fn(&App, HandleId) -> bool,
    radio_value: fn(&App, HandleId) -> T,
    enabled: fn(&App, HandleId) -> bool,
    focus_node: fn(&App, HandleId) -> AnyFocusNode,
}

impl<T: Copy + PartialEq + 'static> RadioClientVTable<T> {
    const fn of<L: RadioClient<T>>() -> RadioClientVTable<T> {
        RadioClientVTable {
            type_name: std::any::type_name::<L>,
            tristate: |app, id| L::tristate(resolve(id), app),
            radio_value: |app, id| L::radio_value(resolve(id), app),
            enabled: |app, id| L::enabled(resolve(id), app),
            focus_node: |app, id| L::focus_node(resolve(id), app),
        }
    }
}

/// The field Dart's `RadioClient` mixin declares; a client carries this bag under the field
/// `radio_client` ([`radio_client_accessors!`](crate::radio_client_accessors)).
pub struct RadioClientData<T: Copy + PartialEq + 'static> {
    registry: Option<AnyRadioGroupRegistry<T>>,
}

impl<T: Copy + PartialEq + 'static> RadioClientData<T> {
    /// The bag of a freshly created client.
    pub fn new() -> RadioClientData<T> {
        RadioClientData { registry: None }
    }
}

impl<T: Copy + PartialEq + 'static> Default for RadioClientData<T> {
    fn default() -> RadioClientData<T> {
        RadioClientData::new()
    }
}

/// The accessors [`RadioClient`] asks for, for a struct whose bag is the field `radio_client`.
///
/// Takes the radio's value type, which the trait's own type parameter names.
#[macro_export]
macro_rules! radio_client_accessors {
    ($value:ty) => {
        fn radio_client_data(
            self: ::inset_foundation::Handle<Self>,
            app: &::inset_foundation::App,
        ) -> &$crate::RadioClientData<$value> {
            &app.get(self).radio_client
        }

        fn radio_client_data_mut(
            self: ::inset_foundation::Handle<Self>,
            app: &mut ::inset_foundation::App,
        ) -> &mut $crate::RadioClientData<$value> {
            &mut app.get_mut(self).radio_client
        }
    };
}

/// A client for a [`RadioGroupRegistry`].
///
/// This is typically mixed into a [`State`].
///
/// To register to a [`RadioGroupRegistry`], assign the registry with
/// [`set_registry`](Self::set_registry).
///
/// To unregister from the previous [`RadioGroupRegistry`], either assign a different value or
/// set it to `None`.
pub trait RadioClient<T: Copy + PartialEq + 'static>: Sized + 'static {
    /// Dart's `RadioClient` field, held under the field `radio_client`
    /// ([`radio_client_accessors!`](crate::radio_client_accessors)).
    fn radio_client_data(self: Handle<Self>, app: &App) -> &RadioClientData<T>;

    /// See [`radio_client_data`](Self::radio_client_data).
    fn radio_client_data_mut(self: Handle<Self>, app: &mut App) -> &mut RadioClientData<T>;

    /// This client as the erased [`AnyRadioClient`] — what to pass where a Dart API takes a
    /// `RadioClient<T>`.
    fn as_radio_client(self: Handle<Self>) -> AnyRadioClient<T> {
        AnyRadioClient {
            id: self.id(),
            vtable: const { &RadioClientVTable::<T>::of::<Self>() },
        }
    }

    /// Whether this radio supports toggles.
    ///
    /// Used by the registry to provide additional features such as keyboard support.
    fn tristate(self: Handle<Self>, app: &App) -> bool;

    /// The value this radio represents.
    ///
    /// Used by the registry to provide additional features such as keyboard support.
    fn radio_value(self: Handle<Self>, app: &App) -> T;

    /// Whether this radio is enabled.
    ///
    /// If false, the registry skips this client when handling keyboard navigation.
    fn enabled(self: Handle<Self>, app: &App) -> bool;

    /// Focus node for this radio.
    ///
    /// Used by the registry to provide additional features such as keyboard support.
    fn focus_node(self: Handle<Self>, app: &App) -> AnyFocusNode;

    /// The [`RadioGroupRegistry`] this client is registered to.
    fn registry(self: Handle<Self>, app: &App) -> Option<AnyRadioGroupRegistry<T>> {
        self.radio_client_data(app).registry
    }

    /// See [`registry`](Self::registry).
    ///
    /// Setting this property automatically registers with the new value and unregisters the old
    /// one. This should be set to `None` on dispose.
    fn set_registry(
        self: Handle<Self>,
        app: &mut App,
        new_registry: Option<AnyRadioGroupRegistry<T>>,
    ) {
        let registry = self.radio_client_data(app).registry;
        if registry != new_registry
            && let Some(registry) = registry
        {
            registry.unregister_client(app, self.as_radio_client());
        }
        self.radio_client_data_mut(app).registry = new_registry;
        if let Some(new_registry) = new_registry {
            new_registry.register_client(app, self.as_radio_client());
        }
    }
}

/// Erased `RadioClient<T>`: one identity and a static vtable.
///
/// This is what a field or parameter Dart types as `RadioClient<T>` becomes.
pub struct AnyRadioClient<T: Copy + PartialEq + 'static> {
    id: HandleId,
    vtable: &'static RadioClientVTable<T>,
}

impl<T: Copy + PartialEq + 'static> Clone for AnyRadioClient<T> {
    fn clone(&self) -> AnyRadioClient<T> {
        *self
    }
}

impl<T: Copy + PartialEq + 'static> Copy for AnyRadioClient<T> {}

impl<T: Copy + PartialEq + 'static> PartialEq for AnyRadioClient<T> {
    fn eq(&self, other: &AnyRadioClient<T>) -> bool {
        self.id == other.id
    }
}

impl<T: Copy + PartialEq + 'static> Eq for AnyRadioClient<T> {}

impl<T: Copy + PartialEq + 'static> Debug for AnyRadioClient<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({:?})", (self.vtable.type_name)(), self.id)
    }
}

impl<T: Copy + PartialEq + 'static> AnyRadioClient<T> {
    /// The arena id behind this handle.
    pub fn id(self) -> HandleId {
        self.id
    }

    /// See [`RadioClient::tristate`].
    pub fn tristate(self, app: &App) -> bool {
        (self.vtable.tristate)(app, self.id)
    }

    /// See [`RadioClient::radio_value`].
    pub fn radio_value(self, app: &App) -> T {
        (self.vtable.radio_value)(app, self.id)
    }

    /// See [`RadioClient::enabled`].
    pub fn enabled(self, app: &App) -> bool {
        (self.vtable.enabled)(app, self.id)
    }

    /// See [`RadioClient::focus_node`].
    pub fn focus_node(self, app: &App) -> AnyFocusNode {
        (self.vtable.focus_node)(app, self.id)
    }
}

// ---------------------------------------------------------------------------------------------
// RadioGroup

/// A group for radios.
///
/// This widget treats all radios in the subtree with the same type `T` as a group. Radios with
/// different types are not included in the group.
///
/// This widget handles the group value for the radios in the subtree with the same value type.
///
/// Using this widget also provides keyboard navigation for the radio buttons that matches
/// [APG](https://www.w3.org/WAI/ARIA/apg/patterns/radio/).
///
/// The keyboard behaviors are:
/// * Tab and Shift+Tab: moves focus into and out of the radio group. When focus moves into a
///   radio group and a radio button is selected, focus is set on the selected button.
///   Otherwise, it focuses the first radio button in reading order.
/// * Space: toggle the selection on the focused radio button.
/// * Right and down arrow key: move selection to the next radio button in the group in reading
///   order.
/// * Left and up arrow key: move selection to the previous radio button in the group in reading
///   order.
///
/// Arrow keys will wrap around if they reach the first or last radio in the group.
pub struct RadioGroup<T: Copy + PartialEq + 'static> {
    /// See [`Widget::key`](crate::Widget::key).
    pub key: Option<KeyRef>,

    /// The selected value under this radio group.
    ///
    /// A radio under this radio group whose value equals this value will be selected.
    pub group_value: Option<T>,

    /// Called when selection has changed.
    ///
    /// The value can be `None` when unselecting a radio with `tristate` set to true.
    pub on_changed: ValueChanged<Option<T>>,

    /// The widget below this widget in the tree.
    pub child: WidgetRef,
}

impl<T: Copy + PartialEq + 'static> RadioGroup<T> {
    /// Creates a radio group; Dart's optional `groupValue` argument is the setter.
    ///
    /// The `on_changed` callback is called when the selection has changed in the subtree
    /// radios.
    pub fn new<K>(on_changed: ValueChanged<Option<T>>, child: impl IntoWidget<K>) -> RadioGroup<T> {
        RadioGroup {
            key: None,
            group_value: None,
            on_changed,
            child: child.into_widget(),
        }
    }

    /// Dart `RadioGroup(key:)`.
    pub fn key(mut self, key: KeyRef) -> RadioGroup<T> {
        self.key = Some(key);
        self
    }

    /// Dart `RadioGroup(groupValue:)`.
    pub fn group_value(mut self, group_value: T) -> RadioGroup<T> {
        self.group_value = Some(group_value);
        self
    }

    /// Gets the [`RadioGroupRegistry`] from above the context.
    ///
    /// This registers a dependency on the context so that it rebuilds if the registry has
    /// changed or its [`RadioGroupRegistry::group_value`] has changed.
    pub fn maybe_of(app: &mut App, context: BuildContext) -> Option<AnyRadioGroupRegistry<T>> {
        context
            .depend_on_inherited_widget_of_exact_type::<RadioGroupStateScope<T>>(app)
            .map(|scope| scope.state)
            .map(|state| state.as_radio_group_registry())
    }
}

impl<T: Copy + PartialEq + 'static> Debug for RadioGroup<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RadioGroup").finish_non_exhaustive()
    }
}

impl<T: Copy + PartialEq + 'static> StatefulWidget for RadioGroup<T> {
    type State = RadioGroupState<T>;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> RadioGroupState<T> {
        RadioGroupState {
            state: StateData::new(),
            radio_group_shortcut_manager: None,
            policy: None,
            radios: Vec::new(),
        }
    }
}

/// Dart's `_RadioGroupState`.
pub struct RadioGroupState<T: Copy + PartialEq + 'static> {
    state: StateData<RadioGroup<T>>,
    radio_group_shortcut_manager: Option<Handle<RadioGroupShortcutManager<T>>>,
    policy: Option<Handle<SkipUnselectedRadioPolicy<T>>>,
    radios: Vec<AnyRadioClient<T>>,
}

impl<T: Copy + PartialEq + 'static> RadioGroupState<T> {
    /// Dart's `_radioGroupShortcuts`, built once the state has an `App`.
    fn radio_group_shortcuts(self: Handle<Self>) -> ShortcutMap {
        let select_previous = || {
            Rc::new(VoidCallbackIntent::new(Listener::handle_method(
                self,
                RadioGroupState::<T>::select_previous_radio,
            ))) as IntentRef
        };
        let select_next = || {
            Rc::new(VoidCallbackIntent::new(Listener::handle_method(
                self,
                RadioGroupState::<T>::select_next_radio,
            ))) as IntentRef
        };
        vec![
            (
                Rc::new(SingleActivator::new(LogicalKeyboardKey::ARROW_LEFT))
                    as ShortcutActivatorRef,
                select_previous(),
            ),
            (
                Rc::new(SingleActivator::new(LogicalKeyboardKey::ARROW_RIGHT))
                    as ShortcutActivatorRef,
                select_next(),
            ),
            (
                Rc::new(SingleActivator::new(LogicalKeyboardKey::ARROW_DOWN))
                    as ShortcutActivatorRef,
                select_next(),
            ),
            (
                Rc::new(SingleActivator::new(LogicalKeyboardKey::ARROW_UP)) as ShortcutActivatorRef,
                select_previous(),
            ),
            (
                Rc::new(SingleActivator::new(LogicalKeyboardKey::SPACE)) as ShortcutActivatorRef,
                Rc::new(VoidCallbackIntent::new(Listener::handle_method(
                    self,
                    RadioGroupState::<T>::toggle_focused_radio,
                ))) as IntentRef,
            ),
        ]
    }

    /// The radios registered with this group, in registration order.
    pub fn radios(self: Handle<Self>, app: &App) -> Vec<AnyRadioClient<T>> {
        app.get(self).radios.clone()
    }

    /// The first registered radio whose focus node has focus, if any.
    fn focused_radio(self: Handle<Self>, app: &mut App) -> Option<AnyRadioClient<T>> {
        self.radios(app)
            .into_iter()
            .find(|radio| radio.focus_node(app).has_focus(app))
    }

    fn toggle_focused_radio(self: Handle<Self>, app: &mut App) {
        let Some(radio) = self.focused_radio(app) else {
            return;
        };
        let value = radio.radio_value(app);
        if Some(value) != self.widget(app).group_value {
            self.on_changed(app, Some(value));
            return;
        }

        if radio.tristate(app) {
            self.on_changed(app, None);
        }
    }

    fn select_next_radio(self: Handle<Self>, app: &mut App) {
        self.select_radio_in_direction(app, true);
    }

    fn select_previous_radio(self: Handle<Self>, app: &mut App) {
        self.select_radio_in_direction(app, false);
    }

    fn select_radio_in_direction(self: Handle<Self>, app: &mut App, forward: bool) {
        if self.radios(app).len() < 2 {
            return;
        }
        let Some(current_focus) = self.focused_radio(app).map(|radio| radio.focus_node(app)) else {
            // The focused node is either a non-interactive radio or another control.
            return;
        };
        let enabled: Vec<AnyFocusNode> = self
            .radios(app)
            .into_iter()
            .filter(|radio| radio.enabled(app))
            .map(|radio| radio.focus_node(app))
            .collect();
        let sorted = ReadingOrderTraversalPolicy::sort(app, enabled);
        debug_assert!(!sorted.is_empty());
        let nodes_in_effective_order: Vec<AnyFocusNode> = if forward {
            sorted
        } else {
            sorted.into_iter().rev().collect()
        };

        let next_focus = nodes_in_effective_order
            .iter()
            .position(|node| *node == current_focus)
            .and_then(|index| nodes_in_effective_order.get(index + 1))
            .copied()
            // Current focus is at the end, the next focus should wrap around.
            .or_else(|| nodes_in_effective_order.first().copied())
            .expect("the sorted list is not empty");
        let radio_to_select = self
            .radios(app)
            .into_iter()
            .find(|radio| radio.focus_node(app) == next_focus)
            .expect("the next node belongs to a registered radio");
        let value = radio_to_select.radio_value(app);
        self.on_changed(app, Some(value));
        next_focus.request_focus(app, None);
    }
}

impl<T: Copy + PartialEq + 'static> RadioGroupRegistry<T> for RadioGroupState<T> {
    fn group_value(self: Handle<Self>, app: &App) -> Option<T> {
        self.widget(app).group_value
    }

    fn register_client(self: Handle<Self>, app: &mut App, radio: AnyRadioClient<T>) {
        let radios = &mut app.get_mut(self).radios;
        if !radios.contains(&radio) {
            radios.push(radio);
        }
    }

    fn unregister_client(self: Handle<Self>, app: &mut App, radio: AnyRadioClient<T>) {
        app.get_mut(self).radios.retain(|entry| *entry != radio);
    }

    fn on_changed(self: Handle<Self>, app: &mut App, value: Option<T>) {
        let on_changed = Rc::clone(&self.widget(app).on_changed);
        on_changed(app, value);
    }
}

impl<T: Copy + PartialEq + 'static> State for RadioGroupState<T> {
    type Widget = RadioGroup<T>;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let shortcuts = self.radio_group_shortcuts();
        let manager = RadioGroupShortcutManager::new(app, self);
        manager.set_shortcuts(app, shortcuts);
        let policy = SkipUnselectedRadioPolicy::new(app, self);
        let this = app.get_mut(self);
        this.radio_group_shortcut_manager = Some(manager);
        this.policy = Some(policy);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(manager) = app.get(self).radio_group_shortcut_manager {
            app.get_mut(manager).change_notifier.dispose();
        }
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let manager = app
            .get(self)
            .radio_group_shortcut_manager
            .expect("created in init_state");
        let policy = app.get(self).policy.expect("created in init_state");
        let group_value = self.widget(app).group_value;
        policy.set_group_value(app, group_value);
        let child = self.widget(app).child.clone();
        Shortcuts::manager(
            manager.as_shortcut_manager(),
            FocusTraversalGroup::new(RadioGroupStateScope {
                state: self,
                group_value,
                child,
            })
            .policy(policy.as_policy()),
        )
        .into_widget()
    }
}

/// Dart's `_RadioGroupShortcutManager`: a [`ShortcutManagerBase`] that ignores a key event
/// while no radio in its group has focus.
pub struct RadioGroupShortcutManager<T: Copy + PartialEq + 'static> {
    change_notifier: ChangeNotifierData,
    shortcut_manager: ShortcutManagerData,
    state: Handle<RadioGroupState<T>>,
}

impl<T: Copy + PartialEq + 'static> RadioGroupShortcutManager<T> {
    fn new(
        app: &mut App,
        state: Handle<RadioGroupState<T>>,
    ) -> Handle<RadioGroupShortcutManager<T>> {
        app.create(RadioGroupShortcutManager {
            change_notifier: ChangeNotifierData::new(),
            shortcut_manager: ShortcutManagerData::new(),
            state,
        })
    }
}

impl<T: Copy + PartialEq + 'static> ShortcutManagerBase for RadioGroupShortcutManager<T> {
    crate::shortcut_manager_accessors!();

    fn handle_keypress(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        event: &KeyEvent,
    ) -> KeyEventResult {
        let state = app.get(self).state;
        let radio_has_focus = state
            .radios(app)
            .into_iter()
            .any(|radio| radio.focus_node(app).has_focus(app));
        if !radio_has_focus {
            // Ignore the event if no radio is focused. This prevents this handler from
            // unintentionally consuming an event meant for a non-radio widget that currently
            // has focus.
            return KeyEventResult::Ignored;
        }
        ShortcutManagerKeypress::handle_keypress(self, app, context, event)
    }
}

impl<T: Copy + PartialEq + 'static> ChangeNotifier for RadioGroupShortcutManager<T> {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

/// Dart's `_RadioGroupStateScope`.
pub struct RadioGroupStateScope<T: Copy + PartialEq + 'static> {
    state: Handle<RadioGroupState<T>>,
    /// The group value is part of the scope so that dependents are notified when it changes.
    group_value: Option<T>,
    child: WidgetRef,
}

impl<T: Copy + PartialEq + 'static> Debug for RadioGroupStateScope<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RadioGroupStateScope")
            .field("state", &self.state)
            .finish_non_exhaustive()
    }
}

impl<T: Copy + PartialEq + 'static> InheritedWidget for RadioGroupStateScope<T> {
    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &RadioGroupStateScope<T>) -> bool {
        self.state != old_widget.state || self.group_value != old_widget.group_value
    }
}

/// A traversal policy that is the same as [`ReadingOrderTraversalPolicy`] except that it skips
/// the nodes of unselected radio buttons if there is a selected radio button.
///
/// If none of the radios is selected, this defaults to [`ReadingOrderTraversalPolicy`] for all
/// nodes.
///
/// This policy ensures that tabbing into a radio group only focuses the currently selected
/// radio button and prevents focus from reaching unselected ones.
pub struct SkipUnselectedRadioPolicy<T: Copy + PartialEq + 'static> {
    policy: FocusTraversalPolicyData,
    directional: DirectionalFocusTraversalPolicyMixinData,
    state: Handle<RadioGroupState<T>>,
    group_value: Option<T>,
}

impl<T: Copy + PartialEq + 'static> SkipUnselectedRadioPolicy<T> {
    fn new(
        app: &mut App,
        state: Handle<RadioGroupState<T>>,
    ) -> Handle<SkipUnselectedRadioPolicy<T>> {
        app.create(SkipUnselectedRadioPolicy {
            policy: FocusTraversalPolicyData::new(),
            directional: DirectionalFocusTraversalPolicyMixinData::new(),
            state,
            group_value: None,
        })
    }

    /// The group value the policy compares each radio against; Dart passes it to the
    /// constructor of the policy it builds every frame.
    fn set_group_value(self: Handle<Self>, app: &mut App, group_value: Option<T>) {
        app.get_mut(self).group_value = group_value;
    }

    fn radio_selected(self: Handle<Self>, app: &App, radio: AnyRadioClient<T>) -> bool {
        Some(radio.radio_value(app)) == app.get(self).group_value
    }
}

impl<T: Copy + PartialEq + 'static> FocusTraversalPolicy for SkipUnselectedRadioPolicy<T> {
    crate::focus_traversal_policy_accessors!();
    crate::directional_focus_traversal_policy_overrides!();

    fn sort_descendants(
        self: Handle<Self>,
        app: &mut App,
        descendants: Vec<AnyFocusNode>,
        current_node: AnyFocusNode,
    ) -> Vec<AnyFocusNode> {
        let nodes_in_read_order = ReadingOrderTraversalPolicy::sort(app, descendants.clone());
        let radios = app.get(self).state.radios(app);
        let mut selected = radios
            .iter()
            .copied()
            .find(|radio| self.radio_selected(app, *radio));

        if selected.is_none() {
            // None of the radios is selected. Select the first radio in read order.
            for node in &nodes_in_read_order {
                selected = radios
                    .iter()
                    .copied()
                    .find(|radio| radio.focus_node(app) == *node);
                if selected.is_some() {
                    break;
                }
            }
        }

        let Some(selected) = selected else {
            // None of the radios is selected or focusable, defaults to reading order.
            return nodes_in_read_order;
        };

        // Nodes that are not selected AND not currently focused, since we can't remove the
        // focused node from the sorted result.
        let node_to_skip: Vec<AnyFocusNode> = radios
            .iter()
            .copied()
            .filter(|radio| selected != *radio && radio.focus_node(app) != current_node)
            .map(|radio| radio.focus_node(app))
            .collect();
        let skips_non_selected: Vec<AnyFocusNode> = descendants
            .into_iter()
            .filter(|node| !node_to_skip.contains(node))
            .collect();
        ReadingOrderTraversalPolicy::sort(app, skips_non_selected)
    }
}

impl<T: Copy + PartialEq + 'static> DirectionalFocusTraversalPolicyMixin
    for SkipUnselectedRadioPolicy<T>
{
    fn directional_data(
        self: Handle<Self>,
        app: &App,
    ) -> &DirectionalFocusTraversalPolicyMixinData {
        &app.get(self).directional
    }

    fn directional_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut DirectionalFocusTraversalPolicyMixinData {
        &mut app.get_mut(self).directional
    }
}

#[cfg(test)]
mod tests {
    use inset_foundation::AppCell;
    use std::any::TypeId;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::time::Duration;

    use inset_embedder::{Rect, TextDirection};
    use inset_services::{HardwareKeyboard, KeyDownEvent, LogicalKeyboardKey, PhysicalKeyboardKey};

    use super::*;
    use crate::widgets::actions::{Action, Actions, VoidCallbackAction};
    use crate::widgets::basic::{Directionality, Positioned, SizedBox, Stack};
    use crate::widgets::focus_manager::tests::{app_with_view, mount};
    use crate::widgets::focus_manager::{FocusNode, FocusNodeLeaf, primary_focus};
    use crate::widgets::focus_scope::Focus;

    /// A radio that only registers with the group: enough to drive the registry and the arrow
    /// keys, standing in for `RawRadio`.
    #[derive(Debug)]
    struct TestRadio {
        value: u32,
        enabled: bool,
        rect: Rect,
    }

    impl StatefulWidget for TestRadio {
        type State = TestRadioState;

        fn create_state(&self) -> TestRadioState {
            TestRadioState {
                state: StateData::new(),
                radio_client: RadioClientData::new(),
                focus_node: None,
            }
        }
    }

    struct TestRadioState {
        state: StateData<TestRadio>,
        radio_client: RadioClientData<u32>,
        focus_node: Option<Handle<FocusNode>>,
    }

    impl TestRadioState {
        fn node(self: Handle<Self>, app: &App) -> Handle<FocusNode> {
            app.get(self).focus_node.expect("created in init_state")
        }
    }

    impl RadioClient<u32> for TestRadioState {
        crate::radio_client_accessors!(u32);

        fn tristate(self: Handle<Self>, _app: &App) -> bool {
            false
        }

        fn radio_value(self: Handle<Self>, app: &App) -> u32 {
            self.widget(app).value
        }

        fn enabled(self: Handle<Self>, app: &App) -> bool {
            self.widget(app).enabled
        }

        fn focus_node(self: Handle<Self>, app: &App) -> AnyFocusNode {
            self.node(app).as_node()
        }
    }

    impl State for TestRadioState {
        type Widget = TestRadio;
        crate::state_accessors!();

        fn init_state(self: Handle<Self>, app: &mut App) {
            let node = FocusNode::new(app);
            let label = format!("radio {}", self.widget(app).value);
            node.as_node().set_debug_label(app, Some(label));
            app.get_mut(self).focus_node = Some(node);
        }

        fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
            let context = self.context(app);
            let registry = if self.widget(app).enabled {
                RadioGroup::<u32>::maybe_of(app, context)
            } else {
                None
            };
            self.set_registry(app, registry);
        }

        fn dispose(self: Handle<Self>, app: &mut App) {
            self.set_registry(app, None);
            self.node(app).as_node().dispose(app);
        }

        fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
            let node = self.node(app).as_node();
            let enabled = self.widget(app).enabled;
            let rect = self.widget(app).rect;
            Positioned::from_rect(
                rect,
                Focus::new(SizedBox::expand())
                    .focus_node(node)
                    .can_request_focus(enabled),
            )
            .into_widget()
        }
    }

    /// The group's state, reached through a `GlobalKey`, plus what `on_changed` recorded.
    struct GroupHarness {
        cell: Rc<AppCell>,
        state: Handle<RadioGroupState<u32>>,
        changes: Rc<RefCell<Vec<Option<u32>>>>,
    }

    fn radio(value: u32, enabled: bool, rect: Rect) -> WidgetRef {
        TestRadio {
            value,
            enabled,
            rect,
        }
        .into_widget()
    }

    /// Three radios side by side in reading order, under a group with `group_value`.
    fn mount_group(group_value: Option<u32>, enabled: [bool; 3]) -> GroupHarness {
        let cell = app_with_view();
        let mut app = cell.borrow_mut();
        let changes: Rc<RefCell<Vec<Option<u32>>>> = Rc::default();
        let recorded = Rc::clone(&changes);
        let key = crate::framework::GlobalKey::new();
        let children = vec![
            radio(0, enabled[0], Rect::from_ltwh(0.0, 0.0, 40.0, 20.0)),
            radio(1, enabled[1], Rect::from_ltwh(50.0, 0.0, 40.0, 20.0)),
            radio(2, enabled[2], Rect::from_ltwh(100.0, 0.0, 40.0, 20.0)),
        ];
        let mut group = RadioGroup::<u32>::new(
            Rc::new(move |_app, value| recorded.borrow_mut().push(value)),
            Stack::new().children(children),
        )
        .key(Rc::new(key.clone()));
        if let Some(value) = group_value {
            group = group.group_value(value);
        }
        let void_callback_action = VoidCallbackAction::new(&mut app);
        drop(app);
        mount(
            &cell,
            Directionality::new(
                TextDirection::Ltr,
                Actions::new(
                    HashMap::from([(
                        TypeId::of::<VoidCallbackIntent>(),
                        Action::as_action(void_callback_action),
                    )]),
                    group,
                ),
            )
            .into_widget(),
        );
        let mut app = cell.borrow_mut();
        let state = key
            .current_state::<RadioGroupState<u32>>(&mut app)
            .expect("the group is mounted");
        drop(app);
        GroupHarness {
            cell,
            state,
            changes,
        }
    }

    fn arrow_down(app: &mut App) -> bool {
        let keyboard = HardwareKeyboard::instance(app);
        let event = KeyEvent::Down(KeyDownEvent::new(
            PhysicalKeyboardKey::ARROW_DOWN,
            LogicalKeyboardKey::ARROW_DOWN,
            Duration::ZERO,
        ));
        keyboard.handle_key_event(app, &event)
    }

    #[test]
    fn a_radio_group_registers_every_client_below_it() {
        let harness = mount_group(None, [true, true, true]);
        let values: Vec<u32> = harness
            .state
            .radios(&harness.cell.borrow())
            .into_iter()
            .map(|radio| radio.radio_value(&harness.cell.borrow()))
            .collect();
        assert_eq!(values, [0, 1, 2]);
        assert_eq!(harness.state.group_value(&harness.cell.borrow()), None);

        // A disabled radio does not register.
        let disabled = mount_group(Some(1), [true, false, true]);
        let values: Vec<u32> = disabled
            .state
            .radios(&disabled.cell.borrow())
            .into_iter()
            .map(|radio| radio.radio_value(&disabled.cell.borrow()))
            .collect();
        assert_eq!(values, [0, 2]);
        assert_eq!(disabled.state.group_value(&disabled.cell.borrow()), Some(1));

        // Reported selections reach the widget's callback.
        let registry = harness.state.as_radio_group_registry();
        registry.on_changed(&mut harness.cell.borrow_mut(), Some(2));
        assert_eq!(*harness.changes.borrow(), [Some(2)]);
        drop(disabled);
    }

    #[test]
    fn an_arrow_key_moves_the_selection_to_the_next_radio() {
        let harness = mount_group(Some(0), [true, true, true]);
        let first =
            harness.state.radios(&harness.cell.borrow())[0].focus_node(&harness.cell.borrow());
        first.request_focus(&mut harness.cell.borrow_mut(), None);
        harness.cell.borrow_mut().drain_microtasks();
        assert_eq!(primary_focus(&mut harness.cell.borrow_mut()), Some(first));

        assert!(
            arrow_down(&mut harness.cell.borrow_mut()),
            "the group handled the key"
        );
        harness.cell.borrow_mut().drain_microtasks();
        assert_eq!(*harness.changes.borrow(), [Some(1)]);
        let second =
            harness.state.radios(&harness.cell.borrow())[1].focus_node(&harness.cell.borrow());
        assert_eq!(primary_focus(&mut harness.cell.borrow_mut()), Some(second));

        // The last radio wraps around to the first.
        let third =
            harness.state.radios(&harness.cell.borrow())[2].focus_node(&harness.cell.borrow());
        third.request_focus(&mut harness.cell.borrow_mut(), None);
        harness.cell.borrow_mut().drain_microtasks();
        assert!(arrow_down(&mut harness.cell.borrow_mut()));
        harness.cell.borrow_mut().drain_microtasks();
        assert_eq!(*harness.changes.borrow(), [Some(1), Some(0)]);
    }

    #[test]
    fn an_arrow_key_is_ignored_while_no_radio_has_focus() {
        let harness = mount_group(Some(0), [true, true, true]);
        assert!(
            !arrow_down(&mut harness.cell.borrow_mut()),
            "the manager ignores the event when the focus is elsewhere"
        );
        assert!(harness.changes.borrow().is_empty());
    }

    #[test]
    fn the_traversal_policy_skips_the_unselected_radios() {
        let harness = mount_group(Some(1), [true, true, true]);
        let nodes: Vec<AnyFocusNode> = harness
            .state
            .radios(&harness.cell.borrow())
            .into_iter()
            .map(|radio| radio.focus_node(&harness.cell.borrow()))
            .collect();
        let policy = harness
            .cell
            .borrow()
            .get(harness.state)
            .policy
            .expect("created in init_state");
        let sorted =
            policy.sort_descendants(&mut harness.cell.borrow_mut(), nodes.clone(), nodes[0]);
        assert_eq!(
            sorted,
            vec![nodes[0], nodes[1]],
            "only the selected radio and the currently focused one stay"
        );
    }
}
