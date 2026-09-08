//! Flutter counterpart: `widgets/autofill.dart`.

use std::collections::HashMap;

use reveal_foundation::{App, Handle};
use reveal_services::{AnyAutofillClient, AutofillScope, TextInput};

use crate::framework::{
    BuildContext, InheritedWidget, IntoWidget, KeyRef, State, StateData, StatefulWidget, WidgetRef,
};

pub use reveal_services::AutofillHints;

/// Predefined autofill context clean up actions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AutofillContextAction {
    /// Destroys the current autofill context after informing the platform to save
    /// the user input from it.
    ///
    /// Corresponds to calling [`TextInput::finish_autofill_context`] with
    /// `should_save == true`.
    Commit,

    /// Destroys the current autofill context without saving the user input.
    ///
    /// Corresponds to calling [`TextInput::finish_autofill_context`] with
    /// `should_save == false`.
    Cancel,
}

/// An [`AutofillScope`] widget that groups `AutofillClient`s together.
///
/// `AutofillClient`s that share the same closest [`AutofillGroup`] ancestor must
/// be built together, and they will be autofilled together.
///
/// The [`AutofillGroup`] widget only knows about `AutofillClient`s registered to
/// it using the [`AutofillGroupState::register`] API. Typically, [`AutofillGroup`]
/// will not pick up `AutofillClient`s that are not mounted, for example, an
/// `AutofillClient` within a `Scrollable` that has never been scrolled into the
/// viewport. To workaround this problem, ensure clients in the same
/// [`AutofillGroup`] are built together.
///
/// The topmost [`AutofillGroup`] widgets (the ones that are closest to the root
/// widget) can be used to clean up the current autofill context when the
/// current autofill context is no longer relevant.
///
/// By default, [`on_dispose_action`](Self::on_dispose_action) is set to
/// [`AutofillContextAction::Commit`], in which case when any of the topmost
/// [`AutofillGroup`]s is being disposed, the platform will be informed to save
/// the user input from the current autofill context, then the current autofill
/// context will be destroyed, to free resources. You can, for example, wrap a
/// route that contains a `Form` full of autofillable input fields in an
/// [`AutofillGroup`], so the user input of the `Form` can be saved for future
/// autofill by the platform.
///
/// See also:
///
/// * [`AutofillContextAction`], an enum that contains predefined autofill context
///   clean up actions to be run when a topmost [`AutofillGroup`] is disposed.
#[derive(Debug)]
pub struct AutofillGroup {
    /// See [`StatefulWidget::key`].
    pub key: Option<KeyRef>,
    /// The widget below this widget in the tree.
    pub child: WidgetRef,
    /// The [`AutofillContextAction`] to be run when this [`AutofillGroup`] is the
    /// topmost [`AutofillGroup`] and it's being disposed, in order to clean up the
    /// current autofill context.
    ///
    /// Defaults to [`AutofillContextAction::Commit`], which prompts the platform to
    /// save the user input and destroy the current autofill context.
    pub on_dispose_action: AutofillContextAction,
}

impl AutofillGroup {
    /// Creates a scope for autofillable input fields.
    pub fn new<K>(child: impl IntoWidget<K>) -> AutofillGroup {
        AutofillGroup {
            key: None,
            child: child.into_widget(),
            on_dispose_action: AutofillContextAction::Commit,
        }
    }

    /// Dart `AutofillGroup(key:)`.
    pub fn key(mut self, key: KeyRef) -> AutofillGroup {
        self.key = Some(key);
        self
    }

    /// Dart `AutofillGroup(onDisposeAction:)`.
    pub fn on_dispose_action(mut self, on_dispose_action: AutofillContextAction) -> AutofillGroup {
        self.on_dispose_action = on_dispose_action;
        self
    }

    /// Returns the [`AutofillGroupState`] of the closest [`AutofillGroup`] widget
    /// which encloses the given context, or null if one cannot be found.
    ///
    /// Calling this method will create a dependency on the closest
    /// [`AutofillGroup`] in the [`context`], if there is one.
    ///
    /// See also:
    ///
    /// * [`AutofillGroup::of`], which is similar to this method, but asserts if an
    ///   [`AutofillGroup`] cannot be found.
    /// * `EditableTextState`, where this method is used to retrieve the closest
    ///   [`AutofillGroupState`].
    pub fn maybe_of(app: &mut App, context: BuildContext) -> Option<Handle<AutofillGroupState>> {
        context
            .depend_on_inherited_widget_of_exact_type::<AutofillScopeInherited>(app)
            .and_then(|scope| scope.scope)
    }

    /// Returns the [`AutofillGroupState`] of the closest [`AutofillGroup`] widget
    /// which encloses the given context.
    ///
    /// If no instance is found, this method panics.
    ///
    /// Calling this method will create a dependency on the closest
    /// [`AutofillGroup`] in the [`context`].
    ///
    /// See also:
    ///
    /// * [`AutofillGroup::maybe_of`], which is similar to this method, but returns
    ///   null if an [`AutofillGroup`] cannot be found.
    /// * `EditableTextState`, where this method is used to retrieve the closest
    ///   [`AutofillGroupState`].
    pub fn of(app: &mut App, context: BuildContext) -> Handle<AutofillGroupState> {
        AutofillGroup::maybe_of(app, context).expect(
            "AutofillGroup.of() was called with a context that does not contain an \
             AutofillGroup widget.\n\
             No AutofillGroup widget ancestor could be found starting from the \
             context that was passed to AutofillGroup.of(). This can happen \
             because you are using a widget that looks for an AutofillGroup \
             ancestor, but no such ancestor exists.",
        )
    }
}

impl StatefulWidget for AutofillGroup {
    type State = AutofillGroupState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> AutofillGroupState {
        AutofillGroupState {
            state: StateData::new(),
            clients: HashMap::new(),
            is_topmost_autofill_group: false,
        }
    }
}

/// State associated with an [`AutofillGroup`] widget.
///
/// An [`AutofillGroupState`] can be used to register an `AutofillClient` when it
/// enters this [`AutofillGroup`] (for example, when an `EditableText` is mounted or
/// reparented onto the [`AutofillGroup`]'s subtree), and unregister an
/// `AutofillClient` when it exits (for example, when an `EditableText` gets
/// unmounted or reparented out of the [`AutofillGroup`]'s subtree).
///
/// The [`AutofillGroupState`] class also provides an [`AutofillScope::attach`]
/// method that can be called by `TextInputClient`s that support autofill,
/// instead of [`TextInput::attach`], to create a `TextInputConnection` to interact
/// with the platform's text input system.
///
/// Typically obtained using [`AutofillGroup::of`].
pub struct AutofillGroupState {
    state: StateData<AutofillGroup>,
    clients: HashMap<String, AnyAutofillClient>,
    // Whether this AutofillGroup widget is the topmost AutofillGroup (i.e., it
    // has no AutofillGroup ancestor). Each topmost AutofillGroup runs its
    // `AutofillGroup.onDisposeAction` when it gets disposed.
    is_topmost_autofill_group: bool,
}

impl AutofillGroupState {
    /// Adds the `AutofillClient` to this [`AutofillGroup`].
    ///
    /// Typically, this is called by `TextInputClient`s that support autofill (for
    /// example, `EditableTextState`) in [`State::did_change_dependencies`], when the
    /// input field should be registered to a new [`AutofillGroup`].
    ///
    /// See also:
    ///
    /// * `EditableTextState.didChangeDependencies`, where this method is called
    ///   to update the current [`AutofillScope`] when needed.
    pub fn register(self: Handle<Self>, app: &mut App, client: AnyAutofillClient) {
        let autofill_id = client.autofill_id(app);
        app.get_mut(self)
            .clients
            .entry(autofill_id)
            .or_insert(client);
    }

    /// Removes an `AutofillClient` with the given `autofill_id` from this
    /// [`AutofillGroup`].
    ///
    /// Typically, this should be called by a text field when it's being disposed,
    /// or before it's registered with a different [`AutofillGroup`].
    ///
    /// See also:
    ///
    /// * `EditableTextState.didChangeDependencies`, where this method is called
    ///   to unregister from the previous [`AutofillScope`].
    /// * `EditableTextState.dispose`, where this method is called to unregister
    ///   from the current [`AutofillScope`] when the widget is about to be removed
    ///   from the tree.
    pub fn unregister(self: Handle<Self>, app: &mut App, autofill_id: &str) {
        debug_assert!(app.get(self).clients.contains_key(autofill_id));
        app.get_mut(self).clients.remove(autofill_id);
    }
}

impl AutofillScope for AutofillGroupState {
    fn get_autofill_client(
        self: Handle<Self>,
        app: &App,
        autofill_id: &str,
    ) -> Option<AnyAutofillClient> {
        app.get(self).clients.get(autofill_id).copied()
    }

    fn autofill_clients(self: Handle<Self>, app: &App) -> Vec<AnyAutofillClient> {
        app.get(self)
            .clients
            .values()
            .copied()
            .filter(|client| {
                client
                    .text_input_configuration(app)
                    .autofill_configuration
                    .enabled
            })
            .collect()
    }
}

impl State for AutofillGroupState {
    type Widget = AutofillGroup;
    crate::state_accessors!();

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        let context = self.context(app);
        let is_topmost = AutofillGroup::maybe_of(app, context).is_none();
        app.get_mut(self).is_topmost_autofill_group = is_topmost;
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let child = self.widget(app).child.clone();
        AutofillScopeInherited {
            scope: Some(self),
            child,
        }
        .into_widget()
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if !app.get(self).is_topmost_autofill_group {
            return;
        }
        match self.widget(app).on_dispose_action {
            AutofillContextAction::Cancel => TextInput::finish_autofill_context(app, false),
            AutofillContextAction::Commit => TextInput::finish_autofill_context(app, true),
        }
    }
}

/// Dart's `_AutofillScope`.
#[derive(Debug)]
struct AutofillScopeInherited {
    scope: Option<Handle<AutofillGroupState>>,
    child: WidgetRef,
}

impl InheritedWidget for AutofillScopeInherited {
    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old: &AutofillScopeInherited) -> bool {
        self.scope != old.scope
    }
}

#[cfg(test)]
mod tests {
    use reveal_foundation::{AppCell, Handle};
    use reveal_services::{
        AutofillClient, AutofillConfiguration, TextEditingValue, TextInputConfiguration,
    };

    use super::*;
    use crate::framework::{AnyElement, Element, IntoWidget};
    use crate::test_harness::Harness;
    use crate::widgets::basic::SizedBox;

    struct TestClient {
        autofill_id: String,
        enabled: bool,
    }

    impl AutofillClient for TestClient {
        fn autofill_id(self: Handle<Self>, app: &App) -> String {
            app.get(self).autofill_id.clone()
        }

        fn text_input_configuration(self: Handle<Self>, app: &App) -> TextInputConfiguration {
            if !app.get(self).enabled {
                return TextInputConfiguration::new();
            }
            TextInputConfiguration::new().autofill_configuration(AutofillConfiguration::new(
                app.get(self).autofill_id.clone(),
                vec![AutofillHints::EMAIL.to_owned()],
                TextEditingValue::new(),
            ))
        }

        fn autofill(self: Handle<Self>, _app: &mut App, _new_editing_value: TextEditingValue) {}
    }

    fn descendant(harness: &Harness, app: &App, depth: usize) -> AnyElement {
        let mut element = harness.root.as_element();
        for _ in 0..=depth {
            element = element.children(app)[0];
        }
        element
    }

    #[test]
    fn maybe_of_is_none_without_an_ancestor() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let harness = Harness::mount(&mut app, SizedBox::shrink().into_widget());
        harness.pump(&mut app);
        let context = descendant(&harness, &app, 0);
        assert!(AutofillGroup::maybe_of(&mut app, context).is_none());
    }

    #[test]
    #[should_panic(expected = "AutofillGroup.of() was called with a context that does not contain")]
    fn of_panics_without_an_ancestor() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let harness = Harness::mount(&mut app, SizedBox::shrink().into_widget());
        harness.pump(&mut app);
        let context = descendant(&harness, &app, 0);
        let _ = AutofillGroup::of(&mut app, context);
    }

    #[test]
    fn maybe_of_finds_the_group() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let harness = Harness::mount(
            &mut app,
            AutofillGroup::new(SizedBox::shrink()).into_widget(),
        );
        harness.pump(&mut app);
        // TestRoot → AutofillGroup → _AutofillScope → SizedBox
        let child = descendant(&harness, &app, 2);
        assert!(AutofillGroup::maybe_of(&mut app, child).is_some());
    }

    #[test]
    fn register_and_unregister() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let harness = Harness::mount(
            &mut app,
            AutofillGroup::new(SizedBox::shrink()).into_widget(),
        );
        harness.pump(&mut app);
        let child = descendant(&harness, &app, 2);
        let group = AutofillGroup::of(&mut app, child);

        let enabled = app.create(TestClient {
            autofill_id: "email".into(),
            enabled: true,
        });
        let disabled = app.create(TestClient {
            autofill_id: "plain".into(),
            enabled: false,
        });

        group.register(&mut app, enabled.as_autofill_client());
        group.register(&mut app, disabled.as_autofill_client());

        assert_eq!(
            group
                .get_autofill_client(&app, "email")
                .map(|c| c.autofill_id(&app)),
            Some("email".into())
        );
        assert_eq!(
            group
                .autofill_clients(&app)
                .into_iter()
                .map(|c| c.autofill_id(&app))
                .collect::<Vec<_>>(),
            vec!["email".to_string()]
        );

        group.unregister(&mut app, "email");
        group.unregister(&mut app, "plain");
        assert!(group.get_autofill_client(&app, "email").is_none());
        assert!(group.autofill_clients(&app).is_empty());
    }
}
